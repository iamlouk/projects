package main

import (
	"bytes"
	"fmt"
	"os/exec"
	"strings"

	"github.com/charmbracelet/bubbles/list"
	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

// otherOptionLabel is appended to every question's option list
const otherOptionLabel = "Other: Type something..."

// Message types for TUI communication
type (
	// InputMessage carries user-submitted input into the update loop
	InputMessage struct {
		Text string
	}
	// llmResult is the async result of an LLM API call
	llmResult struct {
		content   string // message text (may be empty when only tool calls)
		toolCalls []ToolCall
		err       error
	}
)

// choiceQuestion is one multiple-choice question from the choices tool
type choiceQuestion struct {
	Question string
	Options  []string
}

// Model holds the TUI state
type Model struct {
	mode           mode
	textInput      textinput.Model
	choiceList     list.Model
	conversation   []string
	config         Config
	cwd            string
	messages       []Message
	waitingForLLM  bool
	quitting       bool
	styles         *styles
	width          int
	height         int
	toolChainCount int // tool calls executed for the current user turn

	// Choice session state (choices tool with multiple questions)
	pendingQuestions []choiceQuestion
	currentQuestion  int
	choiceAnswers    []string // parallel to pendingQuestions
	choicesCallID    string   // tool call ID the answers must be sent to

	// Tool calls from the same assistant message that must run after the
	// choice session finishes
	pendingToolCalls []ToolCall
}

type mode int

const (
	modeInput  mode = iota
	modeChoice      // selecting an option from the list
	modeOther       // typing a free-text answer for "Other"
)

type styles struct {
	titleStyle        lipgloss.Style
	statusStyle       lipgloss.Style
	promptStyle       lipgloss.Style
	questionStyle     lipgloss.Style
	answerStyle       lipgloss.Style
	errorStyle        lipgloss.Style
	conversationStyle lipgloss.Style
	assistantStyle    lipgloss.Style
	toolStyle         lipgloss.Style
}

func newStyles() *styles {
	return &styles{
		titleStyle:        lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#006400")),
		statusStyle:       lipgloss.NewStyle().Foreground(lipgloss.Color("#006400")),
		promptStyle:       lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#006400")),
		questionStyle:     lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#FFD866")),
		answerStyle:       lipgloss.NewStyle().Foreground(lipgloss.Color("#A0FF80")),
		errorStyle:        lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("#FF5555")),
		conversationStyle: lipgloss.NewStyle().Foreground(lipgloss.Color("#E0E0E0")),
		assistantStyle:    lipgloss.NewStyle().Foreground(lipgloss.Color("#FFFFFF")),
		toolStyle: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#8BE9FD")).
			Border(lipgloss.RoundedBorder()).
			BorderForeground(lipgloss.Color("#44475A")).
			Padding(0, 1),
	}
}

func InitialModel() Model {
	ti := textinput.New()
	ti.Placeholder = "Type your message..."
	ti.Focus()
	ti.CharLimit = 0
	ti.Width = 80

	return Model{
		mode:       modeInput,
		textInput:  ti,
		choiceList: list.New([]list.Item{}, list.NewDefaultDelegate(), 0, 0),
		styles:     newStyles(),
		messages:   []Message{},
	}
}

func (m Model) Init() tea.Cmd {
	return textinput.Blink
}

// clampMin ensures sizes never go below a usable minimum
func clampMin(v, minimum int) int {
	if v < minimum {
		return minimum
	}
	return v
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	if m.quitting {
		return m, tea.Quit
	}

	switch msg := msg.(type) {
	case tea.KeyMsg:
		return m.handleKeyMsg(msg)

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.textInput.Width = msg.Width - 4
		m.choiceList.SetSize(msg.Width, clampMin(msg.Height-10, 5))
		return m, nil

	case InputMessage:
		m.conversation = append(m.conversation, m.styles.promptStyle.Render("> ")+msg.Text)
		m.textInput.Reset()
		m.textInput.Placeholder = "Type your message..."
		m.waitingForLLM = true
		m.toolChainCount = 0 // new user turn resets the tool chain counter
		m.messages = append(m.messages, TextMessage(RoleUser, msg.Text))
		// tea.Cmd runs in a goroutine - the UI stays responsive
		return m, m.callLLMCmd()

	case llmResult:
		m.waitingForLLM = false
		if msg.err != nil {
			m.conversation = append(m.conversation, m.styles.errorStyle.Render("Error: "+msg.err.Error()))
			return m, nil
		}

		// Record the assistant message exactly as it came back (including
		// its tool calls) so the conversation history stays valid.
		assistantMsg := Message{Role: RoleAssistant, ToolCalls: msg.toolCalls}
		if msg.content != "" {
			content := msg.content
			assistantMsg.Content = &content
			m.conversation = append(m.conversation, m.styles.assistantStyle.Render(content))
		}
		m.messages = append(m.messages, assistantMsg)

		if len(msg.toolCalls) == 0 {
			return m, nil // plain response, turn is over
		}
		// Execute the requested tool calls (may chain back to the LLM)
		return m.executeToolCalls(msg.toolCalls)
	}

	return m, nil
}

// callLLMCmd builds a tea.Cmd that performs the LLM API call.
// Bubble Tea executes Cmds in goroutines, so this does not block the UI.
func (m Model) callLLMCmd() tea.Cmd {
	// Copy state for the closure - the model is a value type
	config := m.config
	messages := make([]Message, len(m.messages))
	copy(messages, m.messages)
	return func() tea.Msg {
		response, err := callLLMAPI(config, messages)
		if err != nil {
			return llmResult{err: err}
		}

		if len(response.Choices) > 0 {
			choice := response.Choices[0]
			result := llmResult{toolCalls: choice.Message.ToolCalls}
			if choice.Message.Content != nil {
				result.content = *choice.Message.Content
			}
			return result
		}

		if response.Error.Message != "" {
			return llmResult{err: fmt.Errorf("API Error: %s (%s)", response.Error.Message, response.Error.Type)}
		}

		return llmResult{err: fmt.Errorf("no response from LLM")}
	}
}

// executeToolCalls runs the tool calls of one assistant message in order,
// appends a tool result message for each, and hands control back to the LLM.
// The choices tool is special: it pauses for user input, so the remaining
// calls are stashed and executed once the user has answered.
func (m Model) executeToolCalls(calls []ToolCall) (tea.Model, tea.Cmd) {
	for len(calls) > 0 {
		call := calls[0]
		calls = calls[1:]

		// Enforce the tool chain limit (0 = unlimited)
		m.toolChainCount++
		if m.config.MaxToolChain > 0 && m.toolChainCount > m.config.MaxToolChain {
			m.conversation = append(m.conversation, m.styles.errorStyle.Render(
				fmt.Sprintf("Tool chain limit (%d) reached - stopping here. Send another message to continue.",
					m.config.MaxToolChain)))
			// Answer every unanswered tool call so the history stays valid
			m.messages = append(m.messages, ToolResultMessage(call.ID, "not executed: tool chain limit reached"))
			for _, c := range calls {
				m.messages = append(m.messages, ToolResultMessage(c.ID, "not executed: tool chain limit reached"))
			}
			return m, nil // hand control back to the user
		}

		if call.Function.Name == "choices" {
			args, err := ParseToolCallArguments(call.Function.Arguments)
			if err != nil {
				m.messages = append(m.messages, ToolResultMessage(call.ID, "error: "+err.Error()))
				continue
			}
			questions, err := parseChoiceQuestions(args)
			if err != nil {
				// Feed the error back to the model and continue with the rest
				m.messages = append(m.messages, ToolResultMessage(call.ID, "error: "+err.Error()))
				continue
			}

			// Pause the chain: stash remaining calls, start the choice session
			m.pendingToolCalls = calls
			m.choicesCallID = call.ID
			m.pendingQuestions = questions
			m.currentQuestion = 0
			m.choiceAnswers = nil
			return m.showCurrentQuestion(), nil
		}

		// Regular tool: execute and record the result
		args, argsErr := ParseToolCallArguments(call.Function.Arguments)
		var output string
		if argsErr != nil {
			output = "error: " + argsErr.Error()
		} else if result, err := CallTool(call.Function.Name, args, m.cwd); err != nil {
			output = "error: " + err.Error()
		} else {
			output = result
		}

		// Highlight file content for display if pygmentize is available;
		// the LLM still receives the plain text.
		displayOutput := output
		if call.Function.Name == "fs-read" {
			if path, ok := args["path"].(string); ok {
				displayOutput = highlightWithPygmentize(output, path)
			}
		}

		m.messages = append(m.messages, ToolResultMessage(call.ID, output))
		m.conversation = append(m.conversation,
			m.styles.toolStyle.Render(fmt.Sprintf("[Tool: %s]\n%s", call.Function.Name, displayOutput)))
	}

	// All tool results are in - let the LLM continue
	m.waitingForLLM = true
	return m, m.callLLMCmd()
}

// highlightWithPygmentize pipes content through pygmentize (if installed) to
// produce ANSI-highlighted output. Falls back to plain content when
// pygmentize is missing, the lexer is unknown, or anything fails.
func highlightWithPygmentize(content, filename string) string {
	pygmentize, err := exec.LookPath("pygmentize")
	if err != nil {
		return content // not installed
	}

	// Look up the lexer for this filename (-N prints the lexer name)
	lexerOut, err := exec.Command(pygmentize, "-N", filename).Output()
	if err != nil {
		return content
	}
	lexer := strings.TrimSpace(string(lexerOut))
	if lexer == "" || lexer == "text" {
		return content // no meaningful highlighting
	}

	cmd := exec.Command(pygmentize, "-f", "terminal256", "-O", "style=monokai", "-l", lexer)
	cmd.Stdin = strings.NewReader(content)
	var out bytes.Buffer
	cmd.Stdout = &out
	if err := cmd.Run(); err != nil {
		return content
	}
	if out.Len() == 0 {
		return content
	}
	return out.String()
}

func (m Model) handleKeyMsg(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "ctrl+c", "ctrl+d":
		m.quitting = true
		return m, tea.Quit

	case "enter":
		switch m.mode {
		case modeInput:
			input := m.textInput.Value()
			if input != "" {
				return m, func() tea.Msg { return InputMessage{Text: input} }
			}
			return m, nil

		case modeChoice:
			if m.choiceList.SelectedItem() == nil {
				return m, nil
			}
			index := m.choiceList.Index()
			// The "Other" option is always the last one
			if index == len(m.toolChoicesForCurrent())-1 {
				m.mode = modeOther
				m.textInput.Reset()
				m.textInput.Placeholder = "Type your answer..."
				m.textInput.Focus()
				return m, nil
			}
			return m.recordChoiceAnswer(m.choiceList.SelectedItem().(ChoiceItem).choice)

		case modeOther:
			input := m.textInput.Value()
			if input != "" {
				return m.recordChoiceAnswer(input)
			}
			return m, nil
		}
		return m, nil

	case "esc":
		switch m.mode {
		case modeChoice:
			// Cancel the whole choice session; report it to the model so the
			// history stays valid, but do not continue the chain.
			m.conversation = append(m.conversation, m.styles.errorStyle.Render("[Choice session cancelled]"))
			m.messages = append(m.messages, ToolResultMessage(m.choicesCallID, "cancelled by user"))
			return m.cancelChoiceSession()
		case modeOther:
			// Go back to the option list of the current question
			m.mode = modeChoice
			m.textInput.Reset()
			m.textInput.Placeholder = "Type your message..."
			return m, nil
		}
		return m, nil

	case "up", "down":
		if m.mode == modeChoice {
			var cmd tea.Cmd
			m.choiceList, cmd = m.choiceList.Update(msg)
			return m, cmd
		}
		return m, nil
	}

	// All other keys: forward to the text input so typing shows up
	if m.mode == modeInput || m.mode == modeOther {
		if m.mode == modeInput && m.waitingForLLM {
			return m, nil
		}
		var cmd tea.Cmd
		m.textInput, cmd = m.textInput.Update(msg)
		return m, cmd
	}

	return m, nil
}

// toolChoicesForCurrent returns the options of the current question plus the "Other" option
func (m Model) toolChoicesForCurrent() []string {
	if m.currentQuestion >= len(m.pendingQuestions) {
		return nil
	}
	opts := append([]string{}, m.pendingQuestions[m.currentQuestion].Options...)
	return append(opts, otherOptionLabel)
}

// showCurrentQuestion switches to choice mode and displays the current question
func (m Model) showCurrentQuestion() tea.Model {
	q := m.pendingQuestions[m.currentQuestion]
	choices := m.toolChoicesForCurrent()
	items := make([]list.Item, len(choices))
	for i, choice := range choices {
		items[i] = ChoiceItem{choice: choice}
	}
	m.choiceList.SetItems(items)
	m.choiceList.Select(0)
	m.choiceList.Title = fmt.Sprintf("[%d/%d] %s", m.currentQuestion+1, len(m.pendingQuestions), q.Question)
	// Use the terminal size if known, otherwise a sensible fallback
	w, h := m.width, m.height
	if w <= 0 {
		w = 80
	}
	if h <= 0 {
		h = 24
	}
	m.choiceList.SetSize(w, clampMin(h-10, 5))
	m.mode = modeChoice
	m.textInput.Blur()
	return m
}

// recordChoiceAnswer stores the answer and advances to the next question or finishes
func (m Model) recordChoiceAnswer(answer string) (tea.Model, tea.Cmd) {
	m.choiceAnswers = append(m.choiceAnswers, answer)
	m.currentQuestion++

	// Show the answer in the conversation
	q := m.pendingQuestions[m.currentQuestion-1]
	m.conversation = append(m.conversation,
		m.styles.questionStyle.Render(q.Question),
		m.styles.answerStyle.Render("  ↳ "+answer))

	if m.currentQuestion < len(m.pendingQuestions) {
		return m.showCurrentQuestion(), nil
	}

	// All questions answered - report the results as the tool response
	var sb strings.Builder
	sb.WriteString("Choices results:\n")
	for i, pq := range m.pendingQuestions {
		sb.WriteString(fmt.Sprintf("%s: %s\n", pq.Question, m.choiceAnswers[i]))
	}
	result := sb.String()

	m.conversation = append(m.conversation, m.styles.toolStyle.Render(result))
	m.messages = append(m.messages, ToolResultMessage(m.choicesCallID, result))

	// Resume the chain: run any tool calls that were queued after the choices call
	pending := m.pendingToolCalls
	m.choicesCallID = ""
	m.pendingToolCalls = nil
	m.pendingQuestions = nil
	m.currentQuestion = 0
	m.choiceAnswers = nil
	m.mode = modeInput
	m.textInput.Reset()
	m.textInput.Placeholder = "Type your message..."
	m.textInput.Focus()
	return m.executeToolCalls(pending)
}

// cancelChoiceSession aborts the choice session and returns to input mode.
// The tool result (cancelled) has already been recorded by the caller.
func (m Model) cancelChoiceSession() (tea.Model, tea.Cmd) {
	pending := m.pendingToolCalls
	m.choicesCallID = ""
	m.pendingToolCalls = nil
	m.pendingQuestions = nil
	m.currentQuestion = 0
	m.choiceAnswers = nil
	m.mode = modeInput
	m.textInput.Reset()
	m.textInput.Placeholder = "Type your message..."
	m.textInput.Focus()

	// Tool calls queued after the cancelled choices call still get their
	// (cancelled) result so the history stays valid.
	for _, c := range pending {
		m.messages = append(m.messages, ToolResultMessage(c.ID, "not executed: choices cancelled by user"))
	}
	return m, nil
}

// parseChoiceQuestions extracts the questions from the choices tool arguments
func parseChoiceQuestions(args map[string]any) ([]choiceQuestion, error) {
	rawList, ok := args["questions"].([]any)
	if !ok || len(rawList) == 0 {
		return nil, fmt.Errorf("missing or empty 'questions' parameter")
	}

	questions := make([]choiceQuestion, 0, len(rawList))
	for _, rq := range rawList {
		m, ok := rq.(map[string]any)
		if !ok {
			return nil, fmt.Errorf("each question must be an object with 'question' and 'options'")
		}
		qText, _ := m["question"].(string)
		if qText == "" {
			return nil, fmt.Errorf("each question must have a non-empty 'question' field")
		}
		q := choiceQuestion{Question: qText}
		if rawOpts, ok := m["options"].([]any); ok {
			for _, o := range rawOpts {
				if s, ok := o.(string); ok {
					q.Options = append(q.Options, s)
				}
			}
		}
		if len(q.Options) == 0 {
			return nil, fmt.Errorf("question %q has no options", qText)
		}
		questions = append(questions, q)
	}
	return questions, nil
}

func (m Model) View() string {
	var sb strings.Builder

	sb.WriteString(m.styles.titleStyle.Render("lllm - Lou's Large Language Model"))
	sb.WriteString("\n")
	sb.WriteString(m.styles.statusStyle.Render("Type your message. Press Ctrl+C to quit."))
	sb.WriteString("\n\n")

	for _, msg := range m.conversation {
		sb.WriteString(msg + "\n")
	}

	if m.waitingForLLM {
		sb.WriteString(m.styles.statusStyle.Render("Waiting for LLM..."))
		sb.WriteString("\n")
	}

	switch m.mode {
	case modeInput:
		sb.WriteString(m.styles.promptStyle.Render("> ") + m.textInput.View())
	case modeChoice:
		sb.WriteString(m.choiceList.View() + "\n")
		sb.WriteString(m.styles.statusStyle.Render("↑/↓ navigate · Enter select · Esc cancel"))
	case modeOther:
		q := m.pendingQuestions[m.currentQuestion]
		sb.WriteString(m.styles.questionStyle.Render(q.Question) + "\n")
		sb.WriteString(m.styles.promptStyle.Render("answer> ") + m.textInput.View() + "\n")
		sb.WriteString(m.styles.statusStyle.Render("Enter submit · Esc back to options"))
	}

	return sb.String()
}

// ChoiceItem implements list.Item for choice selection
type ChoiceItem struct {
	choice string
}

func (i ChoiceItem) FilterValue() string { return i.choice }
func (i ChoiceItem) Title() string       { return i.choice }
func (i ChoiceItem) Description() string { return "" }

// RegisterChoicesTool registers the choices tool
func RegisterChoicesTool() {
	RegisterTool(Tool{
		Name:        "choices",
		Description: "Present multiple-choice questions to the user via an interactive TUI. Use this whenever you need input or decisions from the user. Each question's option list gets an 'Other: Type something...' free-text option automatically.",
		Parameters: map[string]ToolParam{
			"questions": {
				Type:        "array",
				Description: "List of question objects, each with 'question' (string) and 'options' (array of strings)",
				Required:    true,
			},
		},
		Handler: handleChoices,
	})
}

func handleChoices(args map[string]any, cwd string) (string, error) {
	return "", fmt.Errorf("choices tool must be used through the TUI")
}

// RunTUI starts the Bubble Tea TUI
func RunTUI(config Config, systemPrompt string, cwd string) error {
	m := InitialModel()
	m.config = config
	m.cwd = cwd
	m.messages = append(m.messages, TextMessage(RoleSystem, systemPrompt))

	p := tea.NewProgram(m)
	_, err := p.Run()
	return err
}
