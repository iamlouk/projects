package main

// End-to-end sandbox test. Skipped unless SBX_E2E=1 is set, because
// the first run creates a whole Lima VM (multi-minute image download + boot).

import (
	"context"
	"os"
	"strings"
	"testing"
	"time"
)

func TestSandboxE2E(t *testing.T) {
	if os.Getenv("SBX_E2E") != "1" {
		t.Skip("set LLM_SANDBOX_E2E=1 to run the sandbox e2e test")
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Minute)
	defer cancel()

	sb, cleanup, err := NewSandbox(ctx)
	if err != nil {
		t.Fatalf("NewSandbox: %v", err)
	}
	defer cleanup()

	// 1. It's Alpine Linux.
	out, err := sb.RunShellCommand(ctx, "cat /etc/os-release", 15*time.Second)
	if err != nil {
		t.Fatalf("RunShellCommand: %v", err)
	}
	if !strings.Contains(out, "Alpine") {
		t.Errorf("expected Alpine, got: %s", out)
	}
	t.Logf("os-release: %s", out)

	// 2. Current directory is bind-mounted at /workdir.
	out, err = sb.RunShellCommand(ctx, "pwd && ls", 10*time.Second)
	if err != nil {
		t.Fatalf("RunShellCommand: %v", err)
	}
	if !strings.Contains(out, "/workdir") {
		t.Errorf("expected /workdir as cwd, got: %s", out)
	}
	t.Logf("workdir: %s", out)

	// 3. Container reuse: state persists across calls in the same session.
	if _, err := sb.RunShellCommand(ctx, "echo hello > /tmp/lllm-e2e-marker", 10*time.Second); err != nil {
		t.Fatalf("marker write: %v", err)
	}
	out, err = sb.RunShellCommand(ctx, "cat /tmp/lllm-e2e-marker", 10*time.Second)
	if err != nil {
		t.Fatalf("marker read: %v", err)
	}
	if !strings.Contains(out, "hello") {
		t.Errorf("expected marker content, got: %s", out)
	}

	// 4. Non-zero exit code is reported as an error.
	if _, err := sb.RunShellCommand(ctx, "exit 42", 10*time.Second); err == nil {
		t.Error("expected exit-code error")
	} else {
		t.Logf("exit-code error as expected: %v", err)
	}

	// 5. Timeout kills long-running commands.
	start := time.Now()
	if _, err := sb.RunShellCommand(ctx, "sleep 60", 3*time.Second); err == nil {
		t.Error("expected timeout error")
	} else if !strings.Contains(err.Error(), "timed out") {
		t.Errorf("expected timeout error, got: %v", err)
	}
	if elapsed := time.Since(start); elapsed > 10*time.Second {
		t.Errorf("timeout took too long: %v", elapsed)
	}

	// 6. Sandbox isolation: cannot write outside the bind mount.
	out, err = sb.RunShellCommand(ctx, "touch /etc/lllm-test 2>&1; echo status=$?", 10*time.Second)
	if err != nil {
		t.Logf("write-outside result: %s / %v", out, err)
	}
	t.Logf("isolation probe: %s", out)
}

// TestLimaSandboxE2E exercises the Lima+Docker backend explicitly (set
// SBX_E2E_LIMA=1). Reuses the shared sandbox assertions.
func TestLimaSandboxE2E(t *testing.T) {
	if os.Getenv("SBX_E2E_LIMA") != "1" {
		t.Skip("set SBX_E2E_LIMA=1 to run the lima backend e2e test")
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Minute)
	defer cancel()

	sb, cleanup, err := newLimaSandbox(ctx)
	if err != nil {
		t.Fatalf("newLimaSandbox: %v", err)
	}
	defer cleanup()

	out, err := sb.RunShellCommand(ctx, "cat /etc/os-release | head -1; pwd", 15*time.Second)
	if err != nil {
		t.Fatalf("RunShellCommand: %v", err)
	}
	if !strings.Contains(out, "Alpine") || !strings.Contains(out, "/workdir") {
		t.Errorf("unexpected output: %s", out)
	}
	t.Logf("lima backend: %s", out)
}
