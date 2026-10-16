//go:build !darwin

package main

// apple/container only exists on macOS; on other platforms the apple
// backend reports itself as unavailable so auto-selection skips it.

import (
	"context"
	"errors"
)

func newAppleSandbox(context.Context) (shellSandbox, func(), error) {
	return nil, nil, errors.New("apple/container backend requires macOS: " + errBackendUnavailable.Error())
}
