package main

import (
	"os/exec"
	"strings"
	"testing"

	tea "github.com/charmbracelet/bubbletea"
)

func newTestModel() Model {
	// Tools are registered in main() for the real app; tests must do it themselves
	RegisterFSTools()
	RegisterChoicesTool()
	m := InitialModel()
	m.config = Config{APIEndpoint: "unused", APIKey: "unused", Model: "test", MaxToolChain: 10}
	return m
}

// nativeChoiceCall builds an assistant tool call for the choices tool
func nativeChoiceCall(id string) ToolCall {
	return ToolCall{
		ID:   id,
		Type: "function",
		Function: FunctionCall{
			Name: "choices",
			Arguments: `{"questions":[` +
				`{"question":"What is your favorite color?","options":["red","green","blue"]},` +
				`{"question":"What is your favorite pet?","options":["cat","dog"]}]}`,
		},
	}
}

func nativeToolCall(id, name, args string) ToolCall {
	return ToolCall{ID: id, Type: "function", Function: FunctionCall{Name: name, Arguments: args}}
}

// drive runs Update with a message and returns the new model.
func drive(t *testing.T, m Model, msg tea.Msg) Model {
	t.Helper()
	newModel, _ := m.Update(msg)
	return newModel.(Model)
}

func TestChoicesFlow(t *testing.T) {
	m := newTestModel()
	// Terminal size must be known before the list can render
	m = drive(t, m, tea.WindowSizeMsg{Width: 100, Height: 40})

	// 1. LLM returns an assistant message with a choices tool call
	m = drive(t, m, llmResult{toolCalls: []ToolCall{nativeChoiceCall("call_1")}})
	if m.mode != modeChoice {
		t.Fatalf("expected modeChoice, got %v", m.mode)
	}
	if m.choiceList.Title != "[1/2] What is your favorite color?" {
		t.Fatalf("wrong title: %q", m.choiceList.Title)
	}
	if len(m.toolChoicesForCurrent()) != 4 { // red, green, blue + Other
		t.Fatalf("expected 4 choices, got %v", m.toolChoicesForCurrent())
	}

	// The assistant message (with tool calls) must be in the history
	lastMsg := m.messages[len(m.messages)-1]
	if lastMsg.Role != RoleAssistant || len(lastMsg.ToolCalls) != 1 || lastMsg.ToolCalls[0].ID != "call_1" {
		t.Fatalf("assistant tool-call message not recorded: %+v", lastMsg)
	}

	// The rendered view should show the question, options and the Other option
	view := m.View()
	for _, want := range []string{"What is your favorite color?", "red", "Other: Type something"} {
		if !strings.Contains(view, want) {
			t.Errorf("view missing %q", want)
		}
	}

	// 2. Select the first option (red)
	m = drive(t, m, tea.KeyMsg{Type: tea.KeyEnter})
	if m.mode != modeChoice {
		t.Fatalf("expected to stay in choice mode for question 2, got %v", m.mode)
	}
	if m.choiceList.Title != "[2/2] What is your favorite pet?" {
		t.Fatalf("wrong title after answer 1: %q", m.choiceList.Title)
	}
	if len(m.choiceAnswers) != 1 || m.choiceAnswers[0] != "red" {
		t.Fatalf("answer 1 not recorded: %v", m.choiceAnswers)
	}

	// 3. Question 2: navigate down to "Other" (cat, dog, Other)
	m = drive(t, m, tea.KeyMsg{Type: tea.KeyDown}) // -> dog
	m = drive(t, m, tea.KeyMsg{Type: tea.KeyDown}) // -> Other
	m = drive(t, m, tea.KeyMsg{Type: tea.KeyEnter})
	if m.mode != modeOther {
		t.Fatalf("expected modeOther after selecting Other, got %v", m.mode)
	}

	// 4. Type a free-text answer
	m = drive(t, m, tea.KeyMsg{Type: tea.KeyRunes, Runes: []rune("parrot")})
	m = drive(t, m, tea.KeyMsg{Type: tea.KeyEnter})

	// 5. All questions answered -> tool result with the answers, LLM call pending
	if m.mode != modeInput {
		t.Fatalf("expected modeInput after finishing, got %v", m.mode)
	}
	if !m.waitingForLLM {
		t.Fatal("expected waitingForLLM after choice results sent")
	}
	lastMsg = m.messages[len(m.messages)-1]
	if lastMsg.Role != RoleTool || lastMsg.ToolCallID != "call_1" {
		t.Fatalf("choice results not recorded as tool message: %+v", lastMsg)
	}
	if lastMsg.Content == nil || !strings.Contains(*lastMsg.Content, "parrot") {
		t.Fatalf("answers missing from tool result: %+v", lastMsg.Content)
	}

	// Conversation shows both answers
	conv := strings.Join(m.conversation, "\n")
	if !strings.Contains(conv, "↳ red") || !strings.Contains(conv, "↳ parrot") {
		t.Errorf("answers not shown in conversation:\n%s", conv)
	}
}

func TestToolCallsExecuteAndChain(t *testing.T) {
	m := newTestModel()
	m = drive(t, m, tea.WindowSizeMsg{Width: 100, Height: 40})

	// LLM calls fs-tree, then fs-read in the same message
	m = drive(t, m, llmResult{toolCalls: []ToolCall{
		nativeToolCall("call_1", "fs-tree", `{"path":"."}`),
		nativeToolCall("call_2", "fs-read", `{"path":"system-prompt.txt"}`),
	}})

	// Both tools executed, results recorded as tool messages, LLM called again
	if !m.waitingForLLM {
		t.Fatal("expected the chain to call the LLM again after tool results")
	}
	if len(m.messages) < 3 {
		t.Fatalf("expected assistant + 2 tool messages in history, got %d", len(m.messages))
	}
	var toolMsgs []Message
	for _, msg := range m.messages {
		if msg.Role == RoleTool {
			toolMsgs = append(toolMsgs, msg)
		}
	}
	if len(toolMsgs) != 2 {
		t.Fatalf("expected 2 tool messages, got %d", len(toolMsgs))
	}
	if toolMsgs[0].ToolCallID != "call_1" || toolMsgs[1].ToolCallID != "call_2" {
		t.Fatalf("tool messages not linked by ID: %+v", toolMsgs)
	}
	conv := strings.Join(m.conversation, "\n")
	if !strings.Contains(conv, "[Tool: fs-tree]") || !strings.Contains(conv, "[Tool: fs-read]") {
		t.Errorf("tool outputs not displayed:\n%s", conv)
	}
}

func TestUnknownToolCallFeedsErrorBack(t *testing.T) {
	m := newTestModel()
	m = drive(t, m, tea.WindowSizeMsg{Width: 100, Height: 40})

	m = drive(t, m, llmResult{toolCalls: []ToolCall{
		nativeToolCall("call_x", "does-not-exist", `{}`),
	}})

	// The error must be recorded as the tool result (not abort the chain)
	if !m.waitingForLLM {
		t.Fatal("chain should continue so the model can recover from the error")
	}
	lastMsg := m.messages[len(m.messages)-1]
	if lastMsg.Role != RoleTool || lastMsg.ToolCallID != "call_x" {
		t.Fatalf("error not recorded as tool message: %+v", lastMsg)
	}
	if lastMsg.Content == nil || !strings.Contains(*lastMsg.Content, "unknown tool") {
		t.Fatalf("error content missing: %+v", lastMsg.Content)
	}
}

func TestToolChainLimit(t *testing.T) {
	m := newTestModel()
	m.config.MaxToolChain = 2
	m = drive(t, m, tea.WindowSizeMsg{Width: 100, Height: 40})

	fsTreeCall := nativeToolCall("call_1", "fs-tree", `{"path":"."}`)

	// Tool call 1 and 2 execute and chain back to the LLM
	m = drive(t, m, llmResult{toolCalls: []ToolCall{fsTreeCall}})
	if m.waitingForLLM != true {
		t.Fatal("after tool 1, LLM should be called")
	}
	m.waitingForLLM = false // simulate LLM turn ending
	m = drive(t, m, llmResult{toolCalls: []ToolCall{nativeToolCall("call_2", "fs-tree", `{"path":"."}`)}})
	if m.waitingForLLM != true {
		t.Fatal("after tool 2, LLM should be called")
	}
	m.waitingForLLM = false

	// Tool call 3 exceeds the limit: refused, no LLM call, but the tool call
	// still gets a result message so the history stays valid
	m = drive(t, m, llmResult{toolCalls: []ToolCall{nativeToolCall("call_3", "fs-tree", `{"path":"."}`)}})
	if m.waitingForLLM {
		t.Fatal("tool call over the limit must not trigger an LLM call")
	}
	conv := strings.Join(m.conversation, "\n")
	if !strings.Contains(conv, "Tool chain limit (2) reached") {
		t.Fatalf("missing limit message in conversation: %s", conv)
	}
	lastMsg := m.messages[len(m.messages)-1]
	if lastMsg.Role != RoleTool || !strings.Contains(*lastMsg.Content, "not executed") {
		t.Fatalf("refused call must still get a tool result message: %+v", lastMsg)
	}

	// A new user turn resets the counter
	m = drive(t, m, InputMessage{Text: "continue"})
	m.waitingForLLM = false
	m = drive(t, m, llmResult{toolCalls: []ToolCall{nativeToolCall("call_4", "fs-tree", `{"path":"."}`)}})
	if m.waitingForLLM != true {
		t.Fatal("after a new user turn, tools must execute again")
	}
}

func TestToolChainUnlimited(t *testing.T) {
	m := newTestModel()
	m.config.MaxToolChain = 0 // unlimited
	m = drive(t, m, tea.WindowSizeMsg{Width: 100, Height: 40})

	for i := 0; i < 15; i++ {
		m = drive(t, m, llmResult{toolCalls: []ToolCall{
			nativeToolCall("call", "fs-tree", `{"path":"."}`),
		}})
		if !m.waitingForLLM {
			t.Fatalf("unlimited config must not stop the chain (stopped after %d calls)", i+1)
		}
		m.waitingForLLM = false
	}
}

func TestChoicesEscCancel(t *testing.T) {
	m := newTestModel()
	m = drive(t, m, tea.WindowSizeMsg{Width: 100, Height: 40})
	m = drive(t, m, llmResult{toolCalls: []ToolCall{nativeChoiceCall("call_1")}})

	// Answer question 1, then cancel during question 2
	m = drive(t, m, tea.KeyMsg{Type: tea.KeyEnter})
	m = drive(t, m, tea.KeyMsg{Type: tea.KeyEsc})

	if m.mode != modeInput {
		t.Fatalf("expected modeInput after cancel, got %v", m.mode)
	}
	if len(m.pendingQuestions) != 0 || len(m.choiceAnswers) != 0 {
		t.Fatal("choice session state not cleared on cancel")
	}
	if m.waitingForLLM {
		t.Fatal("cancel must not trigger an LLM call")
	}
	// The cancelled call must still get a tool result so the history stays valid
	lastMsg := m.messages[len(m.messages)-1]
	if lastMsg.Role != RoleTool || lastMsg.ToolCallID != "call_1" || !strings.Contains(*lastMsg.Content, "cancelled") {
		t.Fatalf("cancelled choice call must have a tool result: %+v", lastMsg)
	}
}

func TestChoicesEscFromOtherBackToOptions(t *testing.T) {
	m := newTestModel()
	m = drive(t, m, tea.WindowSizeMsg{Width: 100, Height: 40})
	m = drive(t, m, llmResult{toolCalls: []ToolCall{nativeChoiceCall("call_1")}})

	// Select "Other" (last item: red, green, blue, Other)
	m = drive(t, m, tea.KeyMsg{Type: tea.KeyDown})
	m = drive(t, m, tea.KeyMsg{Type: tea.KeyDown})
	m = drive(t, m, tea.KeyMsg{Type: tea.KeyDown})
	m = drive(t, m, tea.KeyMsg{Type: tea.KeyEnter})
	if m.mode != modeOther {
		t.Fatalf("expected modeOther, got %v", m.mode)
	}

	// Esc goes back to the option list, not input mode
	m = drive(t, m, tea.KeyMsg{Type: tea.KeyEsc})
	if m.mode != modeChoice {
		t.Fatalf("expected modeChoice after Esc from Other, got %v", m.mode)
	}
	if m.choiceList.Title != "[1/2] What is your favorite color?" {
		t.Fatalf("wrong question after Esc from Other: %q", m.choiceList.Title)
	}
}

func TestParseChoiceQuestions(t *testing.T) {
	// Valid
	args, err := ParseToolCallArguments(nativeChoiceCall("x").Function.Arguments)
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	qs, err := parseChoiceQuestions(args)
	if err != nil {
		t.Fatalf("parse failed: %v", err)
	}
	if len(qs) != 2 || qs[0].Question != "What is your favorite color?" || len(qs[0].Options) != 3 {
		t.Fatalf("wrong questions: %+v", qs)
	}

	// Missing questions
	if _, err := parseChoiceQuestions(map[string]any{}); err == nil {
		t.Fatal("expected error for missing questions")
	}

	// Question without options
	if _, err := parseChoiceQuestions(map[string]any{
		"questions": []any{map[string]any{"question": "Q?"}},
	}); err == nil {
		t.Fatal("expected error for missing options")
	}
}

func TestHighlightWithPygmentize(t *testing.T) {
	// If pygmentize is not installed, highlight must return plain content
	content := "package main\n"
	out := highlightWithPygmentize(content, "main.go")
	if pygmentizeAvailable() && out == content {
		t.Log("pygmentize installed but no highlighting added (check lexer)")
	}
	if !pygmentizeAvailable() && out != content {
		t.Fatal("must fall back to plain content when pygmentize is missing")
	}
	t.Logf("highlighted output: %q", out[:minLen(60, len(out))])
}

func pygmentizeAvailable() bool {
	_, err := exec.LookPath("pygmentize")
	return err == nil
}

func minLen(a, b int) int {
	if a < b {
		return a
	}
	return b
}
