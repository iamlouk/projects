package main

// Docker backend: drives any reachable Docker daemon through the pure-Go
// moby client SDK. Used natively on Linux, or on macOS when a Docker
// daemon (Docker Desktop, OrbStack, ...) is already running.

import (
	"context"
	"encoding/binary"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"github.com/moby/moby/api/types/container"
	mobyclient "github.com/moby/moby/client"
)

// errBackendUnavailable marks a backend that is not installed/reachable on
// this machine (not an error in itself during auto-selection).
var errBackendUnavailable = errors.New("backend unavailable")

// dockerBackend runs the sandbox in a long-lived Docker container and uses
// docker exec for each shell command.
type dockerBackend struct {
	cli         *mobyclient.Client
	containerID string
	name        string
}

// newDockerSandbox connects to a reachable Docker daemon (DOCKER_HOST, the
// default socket, or the Docker Desktop socket) and starts the sandbox
// container there.
func newDockerSandbox(ctx context.Context) (shellSandbox, func(), error) {
	host, err := reachableDockerHost(ctx)
	if err != nil {
		return nil, nil, err
	}
	return newDockerSandboxAt(ctx, host)
}

// newDockerClientAt returns a moby client for an explicit Docker host.
func newDockerClientAt(host string) (*mobyclient.Client, error) {
	return mobyclient.NewClientWithOpts(mobyclient.WithHost(host), mobyclient.WithAPIVersionNegotiation())
}

// newDockerSandboxAt creates the sandbox against an explicit Docker host
// (e.g. the socket forwarded from a Lima VM).
func newDockerSandboxAt(ctx context.Context, host string) (shellSandbox, func(), error) {
	cli, err := newDockerClientAt(host)
	if err != nil {
		return nil, nil, fmt.Errorf("docker client for %s: %w", host, err)
	}
	if _, err = cli.Ping(ctx, mobyclient.PingOptions{}); err != nil {
		return nil, nil, fmt.Errorf("docker daemon at %s not reachable: %w", host, err)
	}

	if err := cleanupStaleContainers(ctx, cli); err != nil {
		return nil, nil, fmt.Errorf("stale container cleanup failed: %w", err)
	}

	hostDir, err := hostDirForSandbox()
	if err != nil {
		return nil, nil, err
	}
	id, err := startContainer(ctx, cli, sandboxImage(), hostDir)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to start sandbox container: %w", err)
	}

	s := &dockerBackend{cli: cli, containerID: id, name: sandboxName()}
	cleanup := func() {
		stopCtx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		_, _ = cli.ContainerStop(stopCtx, id, mobyclient.ContainerStopOptions{})
		_, _ = cli.ContainerRemove(stopCtx, id, mobyclient.ContainerRemoveOptions{Force: true})
	}
	return s, cleanup, nil
}

// reachableDockerHost returns the first Docker endpoint that answers a ping:
// DOCKER_HOST if set, then the default sockets.
func reachableDockerHost(ctx context.Context) (string, error) {
	var candidates []string
	if h := os.Getenv("DOCKER_HOST"); h != "" {
		candidates = []string{h}
	} else {
		candidates = []string{
			"unix:///var/run/docker.sock",
			"unix://" + filepath.Join(homeDir(), ".docker", "run", "docker.sock"),
		}
	}
	for _, host := range candidates {
		cli, err := mobyclient.NewClientWithOpts(mobyclient.WithHost(host), mobyclient.WithAPIVersionNegotiation())
		if err != nil {
			continue
		}
		if _, err := cli.Ping(ctx, mobyclient.PingOptions{}); err == nil {
			return host, nil
		}
	}
	if len(candidates) == 1 {
		return "", fmt.Errorf("DOCKER_HOST (%s) not reachable: %w", candidates[0], errBackendUnavailable)
	}
	return "", fmt.Errorf("no docker daemon found (tried %s): %w", strings.Join(candidates, ", "), errBackendUnavailable)
}

// startContainer pulls the image and starts a long-lived sandbox container
// bind-mounting hostDir at /workdir, running as the host user's UID/GID.
func startContainer(ctx context.Context, cli *mobyclient.Client, image, hostDir string) (string, error) {
	pull, err := cli.ImagePull(ctx, image, mobyclient.ImagePullOptions{})
	if err != nil {
		return "", fmt.Errorf("pulling %s: %w", image, err)
	}
	if err := pull.Wait(ctx); err != nil {
		return "", fmt.Errorf("pulling %s: %w", image, err)
	}
	pull.Close()

	res, err := cli.ContainerCreate(ctx, mobyclient.ContainerCreateOptions{
		Name: sandboxName(),
		Config: &container.Config{
			Image:      image,
			User:       fmt.Sprintf("%d:%d", os.Getuid(), os.Getgid()),
			Cmd:        []string{"sleep", "infinity"}, // keep-alive; commands run via exec
			Env:        containerEnv(),
			WorkingDir: "/workdir",
		},
		HostConfig: &container.HostConfig{
			Binds: []string{hostDir + ":/workdir"},
		},
	})
	if err != nil {
		return "", fmt.Errorf("creating container: %w", err)
	}
	if _, err := cli.ContainerStart(ctx, res.ID, mobyclient.ContainerStartOptions{}); err != nil {
		return "", fmt.Errorf("starting container: %w", err)
	}
	return res.ID, nil
}

// cleanupStaleContainers removes sandbox containers left behind by crashed
// lllm sessions (owner pid no longer alive).
func cleanupStaleContainers(ctx context.Context, cli *mobyclient.Client) error {
	list, err := cli.ContainerList(ctx, mobyclient.ContainerListOptions{
		All:     true,
		Filters: mobyclient.Filters{"name": {containerPrefix: true}},
	})
	if err != nil {
		return err
	}
	for _, c := range list.Items {
		for _, n := range c.Names {
			name := strings.TrimPrefix(strings.TrimPrefix(n, "/"), containerPrefix)
			pid, err := strconv.Atoi(name)
			if err != nil || pidAlive(pid) {
				continue
			}
			rmCtx, cancel := context.WithTimeout(ctx, 10*time.Second)
			_, _ = cli.ContainerRemove(rmCtx, c.ID, mobyclient.ContainerRemoveOptions{Force: true})
			cancel()
		}
	}
	return nil
}

// RunShellCommand executes cmd inside the sandbox container via docker exec.
func (s *dockerBackend) RunShellCommand(ctx context.Context, cmd string, timeout time.Duration) (string, error) {
	execRes, err := s.cli.ExecCreate(ctx, s.containerID, mobyclient.ExecCreateOptions{
		Cmd:          []string{"/bin/sh", "-c", cmd},
		WorkingDir:   "/workdir",
		Env:          containerEnv(),
		AttachStdout: true,
		AttachStderr: true,
	})
	if err != nil {
		return "", fmt.Errorf("creating exec: %w", err)
	}

	attachCtx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	attach, err := s.cli.ExecAttach(attachCtx, execRes.ID, mobyclient.ExecAttachOptions{})
	if err != nil {
		return "", fmt.Errorf("starting exec: %w", err)
	}
	defer attach.Close()

	type result struct {
		out string
	}
	done := make(chan result, 1)
	go func() {
		stdout, stderr := demuxDockerStream(attach.Reader)
		done <- result{out: stdout.String() + stderr.String()}
	}()

	select {
	case r := <-done:
		if insp, ierr := s.cli.ExecInspect(ctx, execRes.ID, mobyclient.ExecInspectOptions{}); ierr == nil && insp.ExitCode != 0 {
			return r.out, fmt.Errorf("exit code %d", insp.ExitCode)
		}
		return r.out, nil
	case <-attachCtx.Done():
		return "", fmt.Errorf("command timed out after %s", timeout)
	}
}

// demuxDockerStream splits Docker's non-TTY multiplexed attach stream
// (8-byte header: stream byte + 4-byte big-endian payload size) into
// stdout and stderr builders.
func demuxDockerStream(r io.Reader) (stdout, stderr strings.Builder) {
	header := make([]byte, 8)
	for {
		if _, err := io.ReadFull(r, header); err != nil {
			return stdout, stderr
		}
		size := binary.BigEndian.Uint32(header[4:8])
		if size == 0 {
			continue
		}
		buf := make([]byte, size)
		if _, err := io.ReadFull(r, buf); err != nil {
			return stdout, stderr
		}
		switch header[0] {
		case 1: // stdout
			stdout.Write(buf)
		case 2: // stderr
			stderr.Write(buf)
		}
	}
}
