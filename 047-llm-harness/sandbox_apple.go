//go:build darwin

package main

// apple/container backend (macOS). apple/container runs Linux containers in
// lightweight vz microVMs with ~1s start times. Its 1.x APIs are XPC-only
// (no Go bindings, gRPC was removed), so this backend drives the official
// `container` CLI — the only stable interface.

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"strconv"
	"strings"
	"time"
)

// appleBackend runs the sandbox in a long-lived apple/container container
// and uses `container exec` for each shell command.
type appleBackend struct {
	name string
}

// newAppleSandbox starts the container system service and the sandbox
// container. Returns errBackendUnavailable if the `container` CLI is not
// installed.
func newAppleSandbox(ctx context.Context) (shellSandbox, func(), error) {
	cli, err := exec.LookPath("container")
	if err != nil {
		return nil, nil, fmt.Errorf("container CLI not found: %w", errBackendUnavailable)
	}

	// Idempotent: starts the system service if it is not already running.
	sctx, scancel := context.WithTimeout(ctx, 30*time.Second)
	defer scancel()
	if out, err := runCLI(sctx, cli, "system", "start"); err != nil {
		return nil, nil, fmt.Errorf("container system start failed: %w: %s", err, out)
	}

	if err := appleCleanupStale(ctx, cli); err != nil {
		return nil, nil, fmt.Errorf("stale container cleanup failed: %w", err)
	}

	hostDir, err := hostDirForSandbox()
	if err != nil {
		return nil, nil, err
	}

	name := sandboxName()
	args := []string{
		"run", "--detach",
		"--name", name,
		"--uid", strconv.Itoa(os.Getuid()),
		"--gid", strconv.Itoa(os.Getgid()),
		"--workdir", "/workdir",
		"--mount", "type=virtiofs,source=" + hostDir + ",target=/workdir",
	}
	for _, kv := range containerEnv() {
		args = append(args, "--env", kv)
	}
	args = append(args, sandboxImage(), "sleep", "infinity")

	rctx, rcancel := context.WithTimeout(ctx, 120*time.Second)
	defer rcancel()
	if out, err := runCLI(rctx, cli, args...); err != nil {
		return nil, nil, fmt.Errorf("container run failed: %w: %s", err, out)
	}

	s := &appleBackend{name: name}
	cleanup := func() {
		cctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		_, _ = runCLI(cctx, cli, "rm", "--force", name)
	}
	return s, cleanup, nil
}

// RunShellCommand executes cmd inside the sandbox container. The CLI's exit
// code is the process's exit code.
func (s *appleBackend) RunShellCommand(ctx context.Context, cmd string, timeout time.Duration) (string, error) {
	cli, err := exec.LookPath("container")
	if err != nil {
		return "", err
	}
	cctx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()
	out, err := runCLI(cctx, cli, "exec", s.name, "/bin/sh", "-c", cmd)
	if err != nil {
		if cctx.Err() != nil {
			return "", fmt.Errorf("command timed out after %s", timeout)
		}
		var exitErr *exec.ExitError
		if errors.As(err, &exitErr) {
			return out, fmt.Errorf("exit code %d", exitErr.ExitCode())
		}
		return out, fmt.Errorf("container exec failed: %w", err)
	}
	return out, nil
}

// appleCleanupStale removes sandbox containers whose owner pid is dead.
func appleCleanupStale(ctx context.Context, cli string) error {
	lctx, lcancel := context.WithTimeout(ctx, 30*time.Second)
	defer lcancel()
	out, err := runCLI(lctx, cli, "list", "--all", "--format", "json")
	if err != nil {
		return fmt.Errorf("container list failed: %w: %s", err, out)
	}
	var containers []struct {
		ID string `json:"id"`
	}
	if err := json.Unmarshal([]byte(out), &containers); err != nil {
		return fmt.Errorf("parsing container list: %w", err)
	}
	for _, c := range containers {
		name := strings.TrimPrefix(c.ID, containerPrefix)
		if name == c.ID { // no prefix match
			continue
		}
		pid, err := strconv.Atoi(name)
		if err != nil || pidAlive(pid) {
			continue
		}
		rmCtx, cancel := context.WithTimeout(ctx, 10*time.Second)
		_, _ = runCLI(rmCtx, cli, "rm", "--force", c.ID)
		cancel()
	}
	return nil
}

// runCLI runs the container CLI and returns combined output. The caller's
// context controls cancellation/timeout.
func runCLI(ctx context.Context, cli string, args ...string) (string, error) {
	cmd := exec.CommandContext(ctx, cli, args...)
	out, err := cmd.CombinedOutput()
	return string(out), err
}
