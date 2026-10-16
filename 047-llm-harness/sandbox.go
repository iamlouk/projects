package main

// Sandboxed shell execution for lllm.
//
// Backends (platform-specific files, selected in NewSandbox):
//   - apple: apple/container on macOS (CLI-driven; its APIs are XPC-only)
//   - docker: any reachable Docker daemon via the pure-Go moby SDK
//     (native on Linux, Docker Desktop/OrbStack on macOS)
//   - lima: Lima VM with Docker inside (macOS fallback; pure-Go Lima APIs)
//
// All backends implement the same contract: a long-lived container named
// "lllm-sandbox-<pid>", the current directory bind-mounted at /workdir,
// running as the host user's UID/GID, with stale-container cleanup at
// startup and teardown at lllm exit.

import (
	"context"
	"errors"
	"fmt"
	"os"
	"strings"
	"sync"
	"syscall"
	"time"
)

// pidAlive reports whether a process with the given pid exists (signal 0;
// works on Linux and macOS).
func pidAlive(pid int) bool {
	return pid > 0 && syscall.Kill(pid, 0) == nil
}

const (
	// DefaultLimaInstance is the Lima VM instance name used for the sandbox.
	DefaultLimaInstance = "lllm-sandbox"
	// DefaultSandboxImage is the container image the shell tool runs in.
	DefaultSandboxImage = "alpine:latest"
	// containerPrefix is the sandbox container name prefix
	// ("lllm-sandbox-<pid>"), shared by all backends.
	containerPrefix = "lllm-sandbox-"
	// dockerStartupTimeout is how long we wait for Docker inside a freshly
	// started Lima VM to become reachable.
	dockerStartupTimeout = 120 * time.Second
)

// shellSandbox is the backend-agnostic contract for executing shell commands.
type shellSandbox interface {
	// RunShellCommand executes cmd (a shell snippet) and blocks until it
	// finishes or the timeout expires.
	// TODO: expose async shell command invocations
	RunShellCommand(ctx context.Context, cmd string, timeout time.Duration) (string, error)
}

// sandboxSingleton is shared by all shell tool calls within one lllm session.
var (
	sandboxSingleton shellSandbox
	sandboxOnce      sync.Once
	sandboxInitErr   error
)

// NewSandbox selects and initializes the sandbox backend. The backend can be
// forced with LLM_SANDBOX_BACKEND=apple|docker|lima; the default (auto) is:
// apple/container on macOS, then any reachable Docker daemon, then Lima.
func NewSandbox(ctx context.Context) (shellSandbox, func(), error) {
	switch strings.ToLower(os.Getenv("LLLM_SANDBOX_BACKEND")) {
	case "apple":
		return newAppleSandbox(ctx)
	case "docker":
		return newDockerSandbox(ctx)
	case "lima":
		return newLimaSandbox(ctx)
	}

	var errs []error
	// auto: apple/container first (macOS), then existing Docker daemons,
	// then the Lima VM as the last resort.
	if sb, cleanup, err := newAppleSandbox(ctx); err == nil {
		return sb, cleanup, nil
	} else if !errors.Is(err, errBackendUnavailable) {
		errs = append(errs, fmt.Errorf("apple/container: %w", err))
	}
	if sb, cleanup, err := newDockerSandbox(ctx); err == nil {
		return sb, cleanup, nil
	} else if !errors.Is(err, errBackendUnavailable) {
		errs = append(errs, fmt.Errorf("docker: %w", err))
	}
	if sb, cleanup, err := newLimaSandbox(ctx); err == nil {
		return sb, cleanup, nil
	} else {
		errs = append(errs, fmt.Errorf("lima: %w", err))
	}
	return nil, nil, fmt.Errorf("no sandbox backend available (tried apple/container, docker, lima): %w", errors.Join(errs...))
}

// GetSandbox returns the session-wide sandbox, initializing it on first use.
func GetSandbox(ctx context.Context) (shellSandbox, error) {
	sandboxOnce.Do(func() {
		var cleanup func()
		sandboxSingleton, cleanup, sandboxInitErr = NewSandbox(ctx)
		if cleanup != nil {
			gracefulCleanups = append(gracefulCleanups, cleanup)
		}
	})
	return sandboxSingleton, sandboxInitErr
}

// gracefulCleanups collect exit-time hooks (run from main).
var gracefulCleanups []func()

// RunGracefulCleanups runs registered cleanups (called on lllm exit).
func RunGracefulCleanups() {
	for _, fn := range gracefulCleanups {
		fn()
	}
}

// RegisterShellTool registers the "shell" tool for the LLM. The tool runs
// commands in the session's sandbox container.
func RegisterShellTool() {
	RegisterTool(Tool{
		Name: "shell",
		Description: "Execute a shell command in a sandboxed Linux container (image: " +
			sandboxImageDesc() + "). The current directory is mounted at /workdir inside the container. " +
			"Returns stdout and stderr.",
		Parameters: map[string]ToolParam{
			"command": {
				Type:        "string",
				Description: "Shell command to execute",
				Required:    true,
			},
			"timeout": {
				Type:        "number",
				Description: "Timeout in seconds (default: 10)",
				Required:    false,
				Default:     10,
			},
		},
		Handler: func(args map[string]any, _ string) (string, error) {
			cmd, _ := args["command"].(string)
			if cmd == "" {
				return "", fmt.Errorf("missing command")
			}
			timeout := 10 * time.Second
			if t, ok := args["timeout"].(float64); ok && t > 0 {
				timeout = time.Duration(t) * time.Second
			}
			sb, err := GetSandbox(context.Background())
			if err != nil {
				return "", err
			}
			return sb.RunShellCommand(context.Background(), cmd, timeout)
		},
	})
}

func sandboxImageDesc() string {
	if img := os.Getenv("LLLM_SANDBOX_IMAGE"); img != "" {
		return img
	}
	return DefaultSandboxImage
}

// sandboxName returns this session's container name (lllm-sandbox-<pid>).
func sandboxName() string {
	return fmt.Sprintf("%s%d", containerPrefix, os.Getpid())
}

// hostDirForSandbox returns the directory to bind-mount at /workdir.
func hostDirForSandbox() (string, error) {
	dir, err := os.Getwd()
	if err != nil {
		return "", fmt.Errorf("sandbox: cannot determine working directory: %w", err)
	}
	return dir, nil
}

// sandboxImage returns the configured container image.
func sandboxImage() string {
	if img := os.Getenv("LLLM_SANDBOX_IMAGE"); img != "" {
		return img
	}
	return DefaultSandboxImage
}

// containerEnv is the minimal host environment passed into the container.
func containerEnv() []string {
	env := []string{
		"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
		"HOME=" + homeDir(),
	}
	for _, kv := range os.Environ() {
		if strings.HasPrefix(kv, "LLLM_") {
			env = append(env, kv)
		}
	}
	return env
}

func homeDir() string {
	if h, err := os.UserHomeDir(); err == nil {
		return h
	}
	return "/root"
}
