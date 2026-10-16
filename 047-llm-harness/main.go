package main

import (
	"bytes"
	"context"
	_ "embed"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"strconv"
	"time"
)

//go:embed system-prompt.txt
var systemPromptContent string

// FunctionCall names a tool and carries its JSON-encoded arguments string
type FunctionCall struct {
	Name      string `json:"name"`
	Arguments string `json:"arguments"`
}

// ToolCall is a tool invocation inside an assistant message
type ToolCall struct {
	ID       string       `json:"id"`
	Type     string       `json:"type"` // "function"
	Function FunctionCall `json:"function"`
}

// Message represents a chat message in the OpenAI-compatible wire format.
// Content is a pointer: assistant messages that only call tools carry null.
type Message struct {
	Role       string     `json:"role"`
	Content    *string    `json:"content"`
	ToolCalls  []ToolCall `json:"tool_calls,omitempty"`
	ToolCallID string     `json:"tool_call_id,omitempty"`
}

// TextMessage builds a plain text message
func TextMessage(role, content string) Message {
	return Message{Role: role, Content: &content}
}

// ToolResultMessage builds a tool response referencing a tool call by ID
func ToolResultMessage(toolCallID, content string) Message {
	return Message{Role: RoleTool, Content: &content, ToolCallID: toolCallID}
}

// Valid role values for OpenAI-compatible chat APIs (Mistral follows this)
const (
	RoleSystem    = "system"
	RoleUser      = "user"
	RoleAssistant = "assistant"
	RoleTool      = "tool"
)

// Config holds the environment configuration
type Config struct {
	APIEndpoint  string
	APIKey       string
	Model        string
	MaxToolChain int // max chained tool calls per user turn (0 = unlimited)
}

// ToolDefinition declares a tool for the API's native function calling
type ToolDefinition struct {
	Type     string       `json:"type"` // "function"
	Function ToolFunction `json:"function"`
}

// ToolFunction is the function payload of a ToolDefinition
type ToolFunction struct {
	Name        string         `json:"name"`
	Description string         `json:"description"`
	Parameters  map[string]any `json:"parameters"` // JSON Schema
}

// LLMRequest represents the request body for LLM chat APIs
type LLMRequest struct {
	Model       string           `json:"model"`
	Messages    []Message        `json:"messages"`
	Temperature float64          `json:"temperature,omitempty"`
	MaxTokens   int              `json:"max_tokens,omitempty"`
	Tools       []ToolDefinition `json:"tools,omitempty"`
	ToolChoice  string           `json:"tool_choice,omitempty"`
}

// LLMResponse represents the response from LLM chat APIs
type LLMResponse struct {
	Choices []struct {
		Message struct {
			Role      string     `json:"role"`
			Content   *string    `json:"content"`
			ToolCalls []ToolCall `json:"tool_calls"`
		} `json:"message"`
		FinishReason string `json:"finish_reason"`
	} `json:"choices"`
	Error struct {
		Message string `json:"message"`
		Type    string `json:"type"`
	} `json:"error,omitempty"`
}

// Shared HTTP client with gzip compression and keep-alive
var httpClient = &http.Client{
	Transport: &http.Transport{
		MaxIdleConns:       10,
		IdleConnTimeout:    90 * time.Second,
		DisableKeepAlives:  false,
		DisableCompression: false,
	},
	Timeout: 120 * time.Second,
}

func main() {
	// Load configuration from environment
	config := loadConfig()

	// Validate configuration
	if config.APIEndpoint == "" || config.APIKey == "" {
		fmt.Fprintln(os.Stderr, "Error: LLLM_API_ENDPOINT and LLLM_API_KEY must be set")
		os.Exit(1)
	}

	// Get current working directory
	cwd, err := os.Getwd()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error getting current directory: %v\n", err)
		os.Exit(1)
	}

	// Sandboxed shell execution is a hard dependency: the Lima VM and Docker
	// container must come up before the TUI starts. Cleaned up on exit.
	if _, err := GetSandbox(context.Background()); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
	defer RunGracefulCleanups()

	// Register all tools
	RegisterFSTools()
	RegisterChoicesTool()
	RegisterShellTool()

	// Generate a short tool overview for the system prompt; the actual tool
	// definitions are sent via the API's native function calling.
	fullSystemPrompt := systemPromptContent + GenerateToolInstructions()

	// Start TUI
	if err := RunTUI(config, fullSystemPrompt, cwd); err != nil {
		fmt.Fprintf(os.Stderr, "TUI error: %v\n", err)
		os.Exit(1)
	}
}

func loadConfig() Config {
	endpoint := os.Getenv("LLLM_API_ENDPOINT")
	key := os.Getenv("LLLM_API_KEY")
	model := os.Getenv("LLLM_MODEL")

	// Default to a more powerful model
	if model == "" {
		model = "mistral-small-latest"
	}

	// Max chained tool calls per user turn (default 10, 0 = unlimited)
	maxToolChain := 10
	if v := os.Getenv("LLLM_MAX_TOOL_CHAIN"); v != "" {
		if n, err := strconv.Atoi(v); err == nil {
			maxToolChain = n
		}
	}

	return Config{
		APIEndpoint:  endpoint,
		APIKey:       key,
		Model:        model,
		MaxToolChain: maxToolChain,
	}
}

func callLLMAPI(config Config, conversation []Message) (*LLMResponse, error) {
	// Build request
	request := LLMRequest{
		Model:       config.Model,
		Messages:    conversation,
		Temperature: 0.7,
		MaxTokens:   4096,
	}

	// Native function calling: declare tools in the request payload
	if tools := GenerateToolsPayload(); len(tools) > 0 {
		request.Tools = tools
		request.ToolChoice = "auto"
	}

	// Marshal to JSON
	jsonData, err := json.Marshal(request)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal request: %w", err)
	}

	// Create HTTP request
	req, err := http.NewRequest("POST", config.APIEndpoint, bytes.NewBuffer(jsonData))
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	// Set headers
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+config.APIKey)
	req.Header.Set("Accept", "application/json")

	// Send request using shared client with keep-alive and gzip
	resp, err := httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to send request: %w", err)
	}
	defer resp.Body.Close()

	// Check status code
	if resp.StatusCode != http.StatusOK {
		body, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("API returned status %d: %s", resp.StatusCode, string(body))
	}

	// Parse response
	var response LLMResponse
	if err := json.NewDecoder(resp.Body).Decode(&response); err != nil {
		return nil, fmt.Errorf("failed to decode response: %w", err)
	}

	return &response, nil
}
