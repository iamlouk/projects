package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestFSWrite(t *testing.T) {
	RegisterFSTools()
	cwd, _ := os.Getwd()
	abs := func(p string) string { return filepath.Join(cwd, p) }
	defer os.Remove(abs("write_test.txt"))

	// Write a new file
	out, err := CallTool("fs-write", map[string]any{"path": "write_test.txt", "content": "hello"}, cwd)
	if err != nil {
		t.Fatalf("write failed: %v (%s)", err, out)
	}
	got, _ := os.ReadFile(abs("write_test.txt"))
	if string(got) != "hello" {
		t.Fatalf("wrong content: %q", got)
	}

	// Overwrite
	_, err = CallTool("fs-write", map[string]any{"path": "write_test.txt", "content": "world"}, cwd)
	if err != nil {
		t.Fatalf("overwrite failed: %v", err)
	}
	got, _ = os.ReadFile(abs("write_test.txt"))
	if string(got) != "world" {
		t.Fatalf("wrong content after overwrite: %q", got)
	}

	// Append
	_, err = CallTool("fs-write", map[string]any{"path": "write_test.txt", "content": "!", "append": true}, cwd)
	if err != nil {
		t.Fatalf("append failed: %v", err)
	}
	got, _ = os.ReadFile(abs("write_test.txt"))
	if string(got) != "world!" {
		t.Fatalf("wrong content after append: %q", got)
	}

	// Escape attempt must fail
	if _, err := CallTool("fs-write", map[string]any{"path": "../escape.txt", "content": "x"}, cwd); err == nil {
		t.Fatal("write outside cwd must fail")
		os.Remove("../escape.txt")
	}

	// Missing parent dir must fail without create_dirs
	if _, err := CallTool("fs-write", map[string]any{"path": "nonexistent_dir/x.txt", "content": "x"}, cwd); err == nil {
		t.Fatal("write into missing parent dir must fail")
	}

	// ... but succeeds with create_dirs=true
	defer func() { os.RemoveAll(abs("created_dir")) }()
	out, err = CallTool("fs-write", map[string]any{"path": "created_dir/nested/x.txt", "content": "deep", "create_dirs": true}, cwd)
	if err != nil {
		t.Fatalf("write with create_dirs failed: %v (%s)", err, out)
	}
	got, _ = os.ReadFile(abs("created_dir/nested/x.txt"))
	if string(got) != "deep" {
		t.Fatalf("wrong content after create_dirs write: %q", got)
	}
}
