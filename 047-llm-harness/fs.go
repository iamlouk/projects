package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// validatePath ensures the path is within the current working directory
func validatePath(path string, cwd string) error {
	// Clean the path
	cleanPath := filepath.Clean(path)

	// Get absolute path
	absPath, err := filepath.Abs(cleanPath)
	if err != nil {
		return fmt.Errorf("invalid path: %v", err)
	}

	// Get absolute current working directory
	absCwd, err := filepath.Abs(cwd)
	if err != nil {
		return fmt.Errorf("invalid current directory: %v", err)
	}

	// Check if the path is within cwd
	// Use filepath.Rel to check if absPath is under absCwd
	rel, err := filepath.Rel(absCwd, absPath)
	if err != nil {
		return fmt.Errorf("path %s is outside the current directory", path)
	}
	if strings.HasPrefix(rel, "..") {
		return fmt.Errorf("path %s is outside the current directory", path)
	}

	return nil
}

// handleFSRead reads a file and returns its content
func handleFSRead(args map[string]any, cwd string) (string, error) {
	path, ok := args["path"].(string)
	if !ok || path == "" {
		return "", fmt.Errorf("path parameter is required")
	}

	// Validate path is within current directory
	if err := validatePath(path, cwd); err != nil {
		return "", err
	}

	// Get absolute path
	absPath, err := filepath.Abs(filepath.Join(cwd, path))
	if err != nil {
		return "", fmt.Errorf("failed to resolve path: %v", err)
	}

	// Check if file exists
	info, err := os.Stat(absPath)
	if err != nil {
		if os.IsNotExist(err) {
			return "", fmt.Errorf("file not found: %s", path)
		}
		return "", fmt.Errorf("failed to stat file: %v", err)
	}

	if info.IsDir() {
		return "", fmt.Errorf("path is a directory, not a file: %s", path)
	}

	// Read the file
	content, err := os.ReadFile(absPath)
	if err != nil {
		return "", fmt.Errorf("failed to read file: %v", err)
	}

	// Handle offset and limit
	offset, hasOffset := args["offset"].(float64)
	limit, hasLimit := args["limit"].(float64)

	lines := strings.Split(string(content), "\n")

	if hasOffset || hasLimit {
		start := 0
		if hasOffset {
			start = int(offset) - 1 // Convert to 0-indexed
			if start < 0 {
				start = 0
			}
		}

		end := len(lines)
		if hasLimit {
			end = start + int(limit)
			if end > len(lines) {
				end = len(lines)
			}
		}

		if start >= len(lines) {
			return "", nil
		}

		return strings.Join(lines[start:end], "\n"), nil
	}

	return string(content), nil
}

// handleFSTree lists directory contents as a tree
func handleFSTree(args map[string]any, cwd string) (string, error) {
	path, _ := args["path"].(string)
	maxDepth, _ := args["max_depth"].(float64)

	// Default max_depth to 1
	if maxDepth == 0 {
		maxDepth = 1
	}

	// Default path to current directory
	if path == "" {
		path = "."
	}

	// Validate path is within current directory
	if err := validatePath(path, cwd); err != nil {
		return "", err
	}

	// Get absolute path
	absPath, err := filepath.Abs(filepath.Join(cwd, path))
	if err != nil {
		return "", fmt.Errorf("failed to resolve path: %v", err)
	}

	// Check if path exists
	info, err := os.Stat(absPath)
	if err != nil {
		if os.IsNotExist(err) {
			return "", fmt.Errorf("path not found: %s", path)
		}
		return "", fmt.Errorf("failed to stat path: %v", err)
	}

	// Build tree
	var sb strings.Builder
	if info.IsDir() {
		tree(absPath, "", 0, int(maxDepth), &sb)
	} else {
		// If it's a file, just show it with its info
		sb.WriteString(formatEntry(info, filepath.Base(absPath), absPath))
	}

	return sb.String(), nil
}

// formatSize renders a byte count in human-readable form (B, K, M, ...)
func formatSize(b int64) string {
	const unit = 1024
	if b < unit {
		return fmt.Sprintf("%dB", b)
	}
	div, exp := int64(unit), 0
	for n := b / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}
	return fmt.Sprintf("%.1f%c", float64(b)/float64(div), "KMGTPE"[exp])
}

// symlinkInfo wraps a FileInfo with an overridden mode, so a symlink can
// show its target's size while still being reported as a symlink.
type symlinkInfo struct {
	os.FileInfo
	mode os.FileMode
}

func (s symlinkInfo) Mode() os.FileMode { return s.mode }

// formatEntry renders a name plus its type and size info, ls -lh style
func formatEntry(info os.FileInfo, name string, fullPath string) string {
	mode := info.Mode()
	var typ string
	switch {
	case mode.IsDir():
		typ = "dir"
	case mode&os.ModeSymlink != 0:
		typ = "symlink"
	case mode.IsRegular():
		typ = "file"
	case mode&os.ModeDevice != 0:
		typ = "device"
	case mode&os.ModeNamedPipe != 0:
		typ = "fifo"
	case mode&os.ModeSocket != 0:
		typ = "socket"
	default:
		typ = "other"
	}

	if mode.IsDir() {
		return fmt.Sprintf("%-24s [%s]", name+"/", typ)
	}
	if mode&os.ModeSymlink != 0 {
		if target, err := os.Readlink(fullPath); err == nil {
			return fmt.Sprintf("%-24s [%s -> %s]", name, typ, target)
		}
		return fmt.Sprintf("%-24s [%s]", name, typ)
	}
	return fmt.Sprintf("%-24s [%s, %s]", name, typ, formatSize(info.Size()))
}

// handleFSWrite writes content to a file within the current directory
func handleFSWrite(args map[string]any, cwd string) (string, error) {
	path, _ := args["path"].(string)
	content, _ := args["content"].(string)
	append := false
	if v, ok := args["append"].(bool); ok {
		append = v
	}
	createDirs := false
	if v, ok := args["create_dirs"].(bool); ok {
		createDirs = v
	}

	if path == "" {
		return "", fmt.Errorf("path parameter is required")
	}

	// Validate path is within current directory
	if err := validatePath(path, cwd); err != nil {
		return "", err
	}

	absPath, err := filepath.Abs(filepath.Join(cwd, path))
	if err != nil {
		return "", fmt.Errorf("failed to resolve path: %v", err)
	}

	// Parent directory must exist, unless create_dirs is set
	if _, err := os.Stat(filepath.Dir(absPath)); err != nil {
		if !createDirs {
			return "", fmt.Errorf("parent directory does not exist for: %s (set create_dirs=true to create it)", path)
		}
		if err := os.MkdirAll(filepath.Dir(absPath), 0o755); err != nil {
			return "", fmt.Errorf("failed to create parent directory: %v", err)
		}
	}

	// Existing files keep their permissions; new files get 0644
	mode := os.FileMode(0o644)
	if info, statErr := os.Stat(absPath); statErr == nil {
		mode = info.Mode().Perm()
	}

	var err2 error
	if append {
		f, openErr := os.OpenFile(absPath, os.O_WRONLY|os.O_APPEND|os.O_CREATE, mode)
		if openErr != nil {
			return "", fmt.Errorf("failed to write file: %v", openErr)
		}
		defer f.Close()
		_, err2 = f.WriteString(content)
	} else {
		err2 = os.WriteFile(absPath, []byte(content), mode)
	}
	if err2 != nil {
		return "", fmt.Errorf("failed to write file: %v", err2)
	}

	verb := "wrote"
	if append {
		verb = "appended"
	}
	return fmt.Sprintf("Successfully %s %d bytes to %s", verb, len(content), path), nil
}

// tree recursively builds a tree representation with type and size info
func tree(path string, prefix string, depth int, maxDepth int, sb *strings.Builder) {
	if depth >= maxDepth {
		return
	}

	entries, err := os.ReadDir(path)
	if err != nil {
		sb.WriteString(fmt.Sprintf("[error reading directory: %v]", err))
		return
	}

	for i, entry := range entries {
		isLast := i == len(entries)-1
		newPrefix := prefix
		if depth > 0 {
			if isLast {
				newPrefix += "    "
				sb.WriteString(prefix + "└── ")
			} else {
				newPrefix += "│   "
				sb.WriteString(prefix + "├── ")
			}
		}

		info, err := entry.Info()
		if err != nil {
			sb.WriteString(fmt.Sprintf("%s [unknown]\n", entry.Name()))
			continue
		}

		// For symlinks, stat the target so the size is meaningful
		displayInfo := info
		if info.Mode()&os.ModeSymlink != 0 {
			if targetInfo, err := os.Stat(filepath.Join(path, entry.Name())); err == nil {
				// Keep the symlink type from lstat, but show the target's size
				displayInfo = symlinkInfo{FileInfo: targetInfo, mode: info.Mode()}
			}
		}

		sb.WriteString(formatEntry(displayInfo, entry.Name(), filepath.Join(path, entry.Name())))
		sb.WriteString("\n")

		if entry.IsDir() {
			entryPath := filepath.Join(path, entry.Name())
			tree(entryPath, newPrefix, depth+1, maxDepth, sb)
		}
	}
}

// RegisterFSTools registers the filesystem tools
func RegisterFSTools() {
	RegisterTool(Tool{
		Name:        "fs-write",
		Description: "Write content to a file in the current directory or subdirectory (creates or overwrites; use append to add to an existing file). Path must be within the current working directory.",
		Parameters: map[string]ToolParam{
			"path": {
				Type:        "string",
				Description: "Path to the file, relative to current directory",
				Required:    true,
			},
			"content": {
				Type:        "string",
				Description: "The full content to write",
				Required:    true,
			},
			"append": {
				Type:        "boolean",
				Description: "Append to the file instead of overwriting it (optional)",
				Required:    false,
			},
			"create_dirs": {
				Type:        "boolean",
				Description: "Create missing parent directories automatically (optional)",
				Required:    false,
			},
		},
		Handler: handleFSWrite,
	})

	RegisterTool(Tool{
		Name:        "fs-read",
		Description: "Read a file from the current directory or subdirectory. Path must be within the current working directory.",
		Parameters: map[string]ToolParam{
			"path": {
				Type:        "string",
				Description: "Path to the file, relative to current directory",
				Required:    true,
			},
			"offset": {
				Type:        "integer",
				Description: "Starting line number (1-indexed, optional)",
				Required:    false,
			},
			"limit": {
				Type:        "integer",
				Description: "Number of lines to read (optional)",
				Required:    false,
			},
		},
		Handler: handleFSRead,
	})

	RegisterTool(Tool{
		Name:        "fs-tree",
		Description: "List directory contents as a tree, showing name, type (dir/file/symlink/...) and size for each entry. Path must be within the current working directory.",
		Parameters: map[string]ToolParam{
			"path": {
				Type:        "string",
				Description: "Path to the directory, relative to current directory (defaults to current directory)",
				Required:    false,
			},
			"max_depth": {
				Type:        "integer",
				Description: "Maximum depth to traverse (defaults to 1, which behaves like ls)",
				Required:    false,
				Default:     1,
			},
		},
		Handler: handleFSTree,
	})
}
