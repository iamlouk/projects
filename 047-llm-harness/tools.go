package main

import (
	"encoding/json"
	"fmt"
	"sort"
	"strings"
)

// Tool represents a callable tool with name, description, and parameters
type Tool struct {
	Name        string                                                `json:"name"`
	Description string                                                `json:"description"`
	Parameters  map[string]ToolParam                                  `json:"parameters"`
	Handler     func(args map[string]any, cwd string) (string, error) `json:"-"`
}

// ToolParam represents a tool parameter
type ToolParam struct {
	Type        string `json:"type"`
	Description string `json:"description"`
	Required    bool   `json:"required,omitempty"`
	Default     any    `json:"default,omitempty"`
}

// ToolRegistry holds all available tools
var ToolRegistry = map[string]Tool{}

// RegisterTool adds a tool to the registry
func RegisterTool(tool Tool) {
	ToolRegistry[tool.Name] = tool
}

// GenerateToolInstructions returns a short system-prompt section listing the
// available tools. The full wire definitions are sent via the API's native
// function calling (see GenerateToolsPayload), so no call-syntax instructions
// are needed here.
func GenerateToolInstructions() string {
	if len(ToolRegistry) == 0 {
		return ""
	}
	names := make([]string, 0, len(ToolRegistry))
	for name := range ToolRegistry {
		names = append(names, name)
	}
	sort.Strings(names)
	return "\n\nAvailable tools: " + strings.Join(names, ", ") + "\n" +
		"Call tools via function calling when appropriate. Tool results come back to you as tool responses.\n"
}

// GenerateToolsPayload converts the tool registry into the API's native
// function-calling wire format (JSON Schema per tool).
func GenerateToolsPayload() []ToolDefinition {
	if len(ToolRegistry) == 0 {
		return nil
	}
	names := make([]string, 0, len(ToolRegistry))
	for name := range ToolRegistry {
		names = append(names, name)
	}
	sort.Strings(names)

	defs := make([]ToolDefinition, 0, len(names))
	for _, name := range names {
		tool := ToolRegistry[name]
		properties := make(map[string]any, len(tool.Parameters))
		var required []string
		for paramName, param := range tool.Parameters {
			prop := map[string]any{
				"type":        param.Type,
				"description": param.Description,
			}
			if param.Default != nil {
				prop["default"] = param.Default
			}
			properties[paramName] = prop
			if param.Required {
				required = append(required, paramName)
			}
		}
		schema := map[string]any{
			"type":       "object",
			"properties": properties,
		}
		if len(required) > 0 {
			sort.Strings(required)
			schema["required"] = required
		}
		defs = append(defs, ToolDefinition{
			Type: "function",
			Function: ToolFunction{
				Name:        tool.Name,
				Description: tool.Description,
				Parameters:  schema,
			},
		})
	}
	return defs
}

// ParseToolCallArguments decodes the JSON arguments string of a tool call
func ParseToolCallArguments(argsJSON string) (map[string]any, error) {
	if strings.TrimSpace(argsJSON) == "" {
		return map[string]any{}, nil
	}
	var args map[string]any
	if err := json.Unmarshal([]byte(argsJSON), &args); err != nil {
		return nil, fmt.Errorf("invalid tool arguments: %v", err)
	}
	return args, nil
}

// CallTool executes a tool by name with the given arguments
func CallTool(name string, args map[string]any, cwd string) (string, error) {
	tool, exists := ToolRegistry[name]
	if !exists {
		return "", fmt.Errorf("unknown tool: %s", name)
	}

	// Validate required parameters
	for paramName, param := range tool.Parameters {
		if param.Required {
			if _, exists := args[paramName]; !exists {
				return "", fmt.Errorf("missing required parameter: %s", paramName)
			}
		}
	}

	// Apply defaults
	for paramName, param := range tool.Parameters {
		if param.Default != nil {
			if _, exists := args[paramName]; !exists {
				args[paramName] = param.Default
			}
		}
	}

	return tool.Handler(args, cwd)
}
