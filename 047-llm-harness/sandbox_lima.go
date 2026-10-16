package main

// Lima backend: ensures a Lima VM (with Docker inside) exists and is running,
// then delegates to the docker backend against the Docker daemon inside the
// VM. Pure-Go: Lima's library APIs create/start the instance (the library
// spawns its hostagent itself; we only locate the limactl binary for it).

import (
	"context"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"time"

	_ "github.com/lima-vm/lima/v2/pkg/driver/qemu" // register qemu driver (fallback)
	"github.com/lima-vm/lima/v2/pkg/instance"
	"github.com/lima-vm/lima/v2/pkg/limatmpl"
	"github.com/lima-vm/lima/v2/pkg/limatype"
	"github.com/lima-vm/lima/v2/pkg/store"
	mobyclient "github.com/moby/moby/client"
)

// newLimaSandbox boots the Lima VM (creating it on first use) and runs the
// sandbox container in the Docker daemon inside the VM.
func newLimaSandbox(ctx context.Context) (shellSandbox, func(), error) {
	instName := os.Getenv("LLLM_LIMA_INSTANCE")
	if instName == "" {
		instName = DefaultLimaInstance
	}

	inst, err := ensureLimaInstance(ctx, instName)
	if err != nil {
		return nil, nil, fmt.Errorf("Lima instance %q unavailable: %w (install Lima from https://lima-vm.io)", instName, err)
	}

	dockerHost := os.Getenv("DOCKER_HOST")
	if dockerHost == "" {
		dockerHost = "unix://" + filepath.Join(inst.Dir, "sock", "docker.sock")
	}

	// Wait for the forwarded Docker socket to become usable (VM boot,
	// first-time docker.service start, etc.).
	if err := waitDockerHost(ctx, dockerHost, dockerStartupTimeout); err != nil {
		return nil, nil, fmt.Errorf("Docker inside %q not reachable: %w", instName, err)
	}

	return newDockerSandboxAt(ctx, dockerHost)
}

// waitDockerHost polls until the Docker daemon at host answers a ping or the
// timeout expires.
func waitDockerHost(ctx context.Context, host string, timeout time.Duration) error {
	deadline := time.Now().Add(timeout)
	for {
		cli, err := newDockerClientAt(host)
		if err == nil {
			if _, perr := cli.Ping(ctx, mobyclient.PingOptions{}); perr == nil {
				return nil
			}
		}
		if time.Now().After(deadline) {
			return fmt.Errorf("docker daemon at %s not reachable", host)
		}
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-time.After(2 * time.Second):
		}
	}
}

// ensureLimaInstance returns a running *limatype.Instance, creating and
// starting it from the built-in docker template when it does not exist yet.
func ensureLimaInstance(ctx context.Context, name string) (*limatype.Instance, error) {
	// Lima's template lookup searches relative to the running executable, so
	// from lllm it would miss Homebrew's templates. Point it at the templates
	// directory that ships next to the installed limactl binary.
	if os.Getenv("LIMA_TEMPLATES_PATH") == "" {
		if limactl, lerr := exec.LookPath("limactl"); lerr == nil {
			tplDir := filepath.Join(filepath.Dir(filepath.Dir(limactl)), "share", "lima", "templates")
			if _, serr := os.Stat(tplDir); serr == nil {
				os.Setenv("LIMA_TEMPLATES_PATH", tplDir)
			}
		}
	}

	inst, err := store.Inspect(ctx, name)
	if errors.Is(err, os.ErrNotExist) {
		// Create from Lima's built-in docker template (pure API, no limactl CLI).
		tmpl, terr := limatmpl.Read(ctx, name, "template:docker")
		if terr != nil {
			return nil, fmt.Errorf("reading docker template: %w", terr)
		}
		if terr := tmpl.Embed(ctx, true, true); terr != nil {
			return nil, fmt.Errorf("embedding template: %w", terr)
		}
		inst, terr = instance.Create(ctx, name, tmpl.Bytes, false)
		if terr != nil {
			return nil, fmt.Errorf("creating instance: %w", terr)
		}
	} else if err != nil {
		return nil, err
	}

	if inst.Status != limatype.StatusRunning {
		// instance.StartWithPaths spawns Lima's hostagent; it re-execs the
		// limactl binary for that, so hand it the installed path (we never
		// run it ourselves — without it Start would re-exec lllm itself).
		limactl, lerr := exec.LookPath("limactl")
		if lerr != nil {
			return nil, fmt.Errorf("limactl binary not found in PATH: %w", lerr)
		}
		// Homebrew's guestagent lives next to limactl; point Start at it.
		guestAgent, gerr := findGuestAgent(limactl, inst)
		if gerr != nil {
			return nil, gerr
		}
		serr := instance.StartWithPaths(ctx, inst, false, false, limactl, guestAgent)
		if serr != nil {
			return nil, fmt.Errorf("starting instance: %w", serr)
		}
	}
	return inst, nil
}

// findGuestAgent locates the lima-guestagent binary shipped with the Lima
// installation (next to limactl). Prepared uncompressed binaries win over
// gzipped ones; mirrors lima's own usrlocal.GuestAgentBinary search order.
func findGuestAgent(limactl string, inst *limatype.Instance) (string, error) {
	shareDir := filepath.Join(filepath.Dir(filepath.Dir(limactl)), "share", "lima")
	base := filepath.Join(shareDir, fmt.Sprintf("lima-guestagent.%s-%s", *inst.Config.OS, *inst.Config.Arch))
	for _, cand := range []string{base, base + ".gz"} {
		if _, err := os.Stat(cand); err == nil {
			return cand, nil
		}
	}
	return "", fmt.Errorf("lima-guestagent.%s-%s not found in %s", *inst.Config.OS, *inst.Config.Arch, shareDir)
}
