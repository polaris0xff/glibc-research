// Package assets carries the files a distributed `pgb` needs but cannot fetch:
// the C runtime sources it compiles into every build, the pinned target list,
// and the small fixtures its selftests run against.
//
// It lives at the repository root because go:embed can only reach files under
// the embedding package's own directory, and the sources it carries have to
// stay where the documentation and the experiments refer to them.
//
// Materialise() writes them to a directory so the compiler in a build
// environment can read them; the copy is content-addressed, so a materialised
// tree that already matches is left alone.
//
// SPDX-License-Identifier: MIT
package assets

import (
	"crypto/sha256"
	"embed"
	"encoding/hex"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"
)

//go:embed tool/runtime/*.c tool/runtime/*.h
var runtimeFS embed.FS

//go:embed scripts/common/rootfs-images.txt
var dataFS embed.FS

// RuntimeFS exposes the C runtime sources, named as they are in the tree.
func RuntimeFS() fs.FS {
	sub, err := fs.Sub(runtimeFS, "tool/runtime")
	if err != nil {
		panic(err) // the embed pattern is a compile-time constant
	}
	return sub
}

// RuntimeFile returns one embedded C source or header by base name.
func RuntimeFile(name string) ([]byte, error) {
	return runtimeFS.ReadFile("tool/runtime/" + name)
}

// RuntimeNames lists the embedded runtime sources, sorted.
func RuntimeNames() []string {
	entries, err := runtimeFS.ReadDir("tool/runtime")
	if err != nil {
		return nil
	}
	out := make([]string, 0, len(entries))
	for _, e := range entries {
		out = append(out, e.Name())
	}
	sort.Strings(out)
	return out
}

// RootfsImages is the digest-pinned list of target environments.
func RootfsImages() []byte {
	b, err := dataFS.ReadFile("scripts/common/rootfs-images.txt")
	if err != nil {
		panic(err)
	}
	return b
}

// Materialise writes the runtime sources into dir, creating it if needed. A
// file whose contents already match is not rewritten, so timestamps stay
// stable and a rebuild is not triggered by the copy itself.
func Materialise(dir string) error {
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}
	for _, name := range RuntimeNames() {
		want, err := RuntimeFile(name)
		if err != nil {
			return err
		}
		dst := filepath.Join(dir, name)
		if got, err := os.ReadFile(dst); err == nil && sameBytes(got, want) {
			continue
		}
		tmp := dst + ".tmp"
		if err := os.WriteFile(tmp, want, 0o644); err != nil {
			return err
		}
		if err := os.Rename(tmp, dst); err != nil {
			return err
		}
	}
	return nil
}

// Digest is a stable identifier for the whole embedded runtime, used as part
// of a cache key so a pgb carrying different sources does not reuse objects
// compiled from the old ones.
func Digest() string {
	h := sha256.New()
	for _, name := range RuntimeNames() {
		b, err := RuntimeFile(name)
		if err != nil {
			continue
		}
		fmt.Fprintf(h, "%s\n%d\n", name, len(b))
		h.Write(b)
	}
	return hex.EncodeToString(h.Sum(nil))[:16]
}

// EmbeddedManifest renders name and size for each carried file, for
// `pgb doctor`.
func EmbeddedManifest() string {
	var b strings.Builder
	for _, name := range RuntimeNames() {
		d, _ := RuntimeFile(name)
		fmt.Fprintf(&b, "  %-22s %7d bytes\n", name, len(d))
	}
	fmt.Fprintf(&b, "  %-22s %7d bytes\n", "rootfs-images.txt", len(RootfsImages()))
	return b.String()
}

func sameBytes(a, b []byte) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}
