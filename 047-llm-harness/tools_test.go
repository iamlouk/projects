package main

import (
	"encoding/json"
	"testing"
)

func TestGenerateToolsPayload(t *testing.T) {
	RegisterFSTools()
	RegisterChoicesTool()

	payload := GenerateToolsPayload()
	if len(payload) != 4 { // fs-read, fs-tree, fs-write, choices
		t.Fatalf("expected 4 tool definitions, got %d", len(payload))
	}

	// Wire format checks
	data, err := json.Marshal(payload)
	if err != nil {
		t.Fatalf("payload not JSON-serializable: %v", err)
	}
	var raw []map[string]any
	if err := json.Unmarshal(data, &raw); err != nil {
		t.Fatalf("invalid JSON: %v", err)
	}
	for _, def := range raw {
		if def["type"] != "function" {
			t.Errorf("tool def must have type=function: %v", def)
		}
		fn, _ := def["function"].(map[string]any)
		if fn == nil || fn["name"] == nil || fn["description"] == nil {
			t.Errorf("tool def missing name/description: %v", def)
		}
		params, _ := fn["parameters"].(map[string]any)
		if params == nil || params["type"] != "object" || params["properties"] == nil {
			t.Errorf("parameters must be a JSON schema object: %v", params)
		}
	}

	// fs-write must declare its params including create_dirs
	var writeFn map[string]any
	for _, def := range raw {
		fn, _ := def["function"].(map[string]any)
		if fn["name"] == "fs-write" {
			writeFn = fn
		}
	}
	if writeFn == nil {
		t.Fatal("fs-write missing from payload")
	}
	props := writeFn["parameters"].(map[string]any)["properties"].(map[string]any)
	for _, want := range []string{"path", "content", "append", "create_dirs"} {
		if _, ok := props[want]; !ok {
			t.Errorf("fs-write schema missing property %q", want)
		}
	}
	// path and content are required
	reqList := writeFn["parameters"].(map[string]any)["required"].([]any)
	reqNames := map[string]bool{}
	for _, r := range reqList {
		reqNames[r.(string)] = true
	}
	if !reqNames["path"] || !reqNames["content"] {
		t.Errorf("fs-write required params wrong: %v", reqList)
	}
}

func TestParseToolCallArguments(t *testing.T) {
	// Empty arguments
	args, err := ParseToolCallArguments("")
	if err != nil || len(args) != 0 {
		t.Fatalf("empty args: %v %v", args, err)
	}
	// Valid JSON
	args, err = ParseToolCallArguments(`{"path": "x.txt"}`)
	if err != nil || args["path"] != "x.txt" {
		t.Fatalf("valid args: %v %v", args, err)
	}
	// Invalid JSON errors
	if _, err := ParseToolCallArguments(`{invalid`); err == nil {
		t.Fatal("invalid JSON must error")
	}
}

func TestGenerateToolInstructions(t *testing.T) {
	RegisterFSTools()
	RegisterChoicesTool()
	instr := GenerateToolInstructions()
	for _, want := range []string{"fs-read", "fs-tree", "fs-write", "choices"} {
		if !contains(instr, want) {
			t.Errorf("instructions missing tool %q", want)
		}
	}
}

func contains(s, sub string) bool {
	return len(s) >= len(sub) && (s == sub || indexOf(s, sub) >= 0)
}

func indexOf(s, sub string) int {
	for i := 0; i+len(sub) <= len(s); i++ {
		if s[i:i+len(sub)] == sub {
			return i
		}
	}
	return -1
}
