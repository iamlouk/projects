//go:build darwin && !no_vz

package main

// Register Lima's vz driver (Apple Virtualization) — Lima's own limactl does
// the same via side-effect imports in cmd/limactl/main_darwin.go.

import _ "github.com/lima-vm/lima/v2/pkg/driver/vz"
