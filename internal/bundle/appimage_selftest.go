// appimage_selftest.go — the entry-point resolver's cases, offline.
//
// The resolver decides which program a bundle ships, and getting it wrong is
// silent: a name pgb DERIVED from a store path that misses may fall back, and
// a name the CALLER asked for must not — substituting a different program for
// one that was named has shipped the wrong binary.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"os"
	"path/filepath"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// AppImageSelftest exercises resolveEntry against a fixture closure.
func AppImageSelftest(c *cfg.Config) *selftest.Report {
	r := selftest.New("bundle-appimage")

	dir, err := os.MkdirTemp("", "pgb-appimage-selftest-")
	if err != nil {
		r.Fail("tempdir", err.Error(), "created")
		return r
	}
	defer os.RemoveAll(dir)

	realDir := filepath.Join(dir, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real")
	pkgDir := filepath.Join(dir, "pkg")
	for _, d := range []string{filepath.Join(realDir, "bin"), filepath.Join(pkgDir, "bin")} {
		if err := os.MkdirAll(d, 0o755); err != nil {
			r.Fail("mkdir", err.Error(), "created")
			return r
		}
	}
	// A file that begins with the ELF magic is enough: the resolver asks what
	// the file IS, not whether it links.
	_ = os.WriteFile(filepath.Join(realDir, "bin", "prog"), []byte("\x7fELF fake"), 0o755)
	_ = os.WriteFile(filepath.Join(pkgDir, "bin", "prog"),
		[]byte("#!/bin/sh\nexec /nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real/bin/prog \"$@\"\n"), 0o755)

	b := &Builder{C: c, Root: dir}
	got, err := b.resolveEntry(pkgDir, "prog", false)
	r.Check("a wrapper script is followed to its ELF",
		relOr(got, dir, err), "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real/bin/prog")

	_ = os.WriteFile(filepath.Join(pkgDir, "bin", "prog"),
		[]byte("#!/bin/sh\nexec /nix/store/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb-gone/bin/x\n"), 0o755)
	_, err = b.resolveEntry(pkgDir, "prog", false)
	r.CheckBool("a wrapper pointing nowhere is refused", err != nil, true)

	// Two binaries, neither named what was asked for. With a caller-supplied
	// name it must REFUSE; without one it may fall back, and the two must not
	// be the same answer.
	_ = os.Remove(filepath.Join(pkgDir, "bin", "prog"))
	_ = os.WriteFile(filepath.Join(pkgDir, "bin", "aardvark"), []byte("\x7fELF one"), 0o755)
	_ = os.WriteFile(filepath.Join(pkgDir, "bin", "zebra"), []byte("\x7fELF two"), 0o755)

	_, err = b.resolveEntry(pkgDir, "nosuchprog", true)
	r.CheckBool("--name naming no program is refused, not substituted", err != nil, true)

	got, err = b.resolveEntry(pkgDir, "nosuchprog", false)
	r.CheckBool("a DERIVED name that misses still falls back", err == nil && got != "", true)

	// The wrapper reader, on both shapes.
	shell := filepath.Join(dir, "shellwrap")
	_ = os.WriteFile(shell, []byte("#!/bin/sh\nexport QT_PLUGIN_PATH='/nix/store/cccccccccccccccccccccccccccccccc-qt/plugins':$QT_PLUGIN_PATH\n"+
		"exec /nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real/bin/prog \"$@\"\n"), 0o755)
	recs := ReadWrapper(shell)
	r.Check("a shell wrapper's target is read", WrapperTarget(recs),
		"/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real/bin/prog")
	found := ""
	for _, rec := range recs {
		if rec.Var == "QT_PLUGIN_PATH" {
			found = string(rec.Op)
		}
	}
	r.Check("and its prefix assignment is a prefix", found, "prefix")

	// The .env folder, which is what keeps a multi-program bundle from asking
	// Qt to scan the same directories once per program.
	envFile := filepath.Join(dir, ".env")
	_ = os.WriteFile(envFile, []byte(
		"QT_PLUGIN_PATH=/a:${QT_PLUGIN_PATH}\n"+
			"QT_PLUGIN_PATH=/a:/b:${QT_PLUGIN_PATH}\n"+
			"XDG_DATA_DIRS=/x:${XDG_DATA_DIRS}\n"), 0o644)
	keys, before, after, ferr := FoldEnv(envFile)
	if ferr != nil {
		r.Fail("fold the env file", ferr.Error(), "no error")
		return r
	}
	r.Check("folding keeps one line per key", itoa(keys), "2")
	r.CheckBool("and the folded file is smaller", after < before, true)
	folded, _ := os.ReadFile(envFile)
	r.Check("a repeated path component is dropped, first position kept",
		firstLineFor(string(folded), "QT_PLUGIN_PATH"),
		"QT_PLUGIN_PATH=/a:/b:${QT_PLUGIN_PATH}")
	return r
}

func relOr(p, root string, err error) string {
	if err != nil {
		return "error: " + err.Error()
	}
	rel, rerr := filepath.Rel(root, p)
	if rerr != nil {
		return p
	}
	return rel
}

func firstLineFor(text, key string) string {
	for _, line := range splitLines(text) {
		if len(line) > len(key) && line[:len(key)+1] == key+"=" {
			return line
		}
	}
	return ""
}

func splitLines(s string) []string {
	var out []string
	start := 0
	for i := 0; i < len(s); i++ {
		if s[i] == '\n' {
			out = append(out, s[start:i])
			start = i + 1
		}
	}
	if start < len(s) {
		out = append(out, s[start:])
	}
	return out
}
