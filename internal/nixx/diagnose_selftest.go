// diagnose_selftest.go — the adaptation loop's one decision, exercised offline.
//
// `diagnose` reads a failed build's log and returns one directive. Everything
// else in `pgb nix build` is a network and a compiler; this function is pure,
// and it is the part that decides whether a package builds at all.
//
// The subjects are real messages, quoted from builds in this tree, because a
// regular expression tested against a message someone invented for it tests
// the invention.
//
// SPDX-License-Identifier: MIT
package nixx

import (
	"os"
	"path/filepath"

	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// DiagnoseSelftest exercises the fix directives and, as importantly, the
// silences: a log this loop has nothing to say about must produce "".
func DiagnoseSelftest() *selftest.Report {
	r := selftest.New("nix-diagnose")

	dir, err := os.MkdirTemp("", "pgb-diagnose-")
	if err != nil {
		r.Skip("no writable temp directory: " + err.Error())
		return r
	}
	defer os.RemoveAll(dir)

	say := func(name, text string, flags []string) string {
		p := filepath.Join(dir, name+".log")
		if err := os.WriteFile(p, []byte(text), 0o644); err != nil {
			return "write failed: " + err.Error()
		}
		return diagnose(p, flags)
	}

	// ⛔ THE ONE THAT STOPPED poc/92-miniflux. postgres names the TOOL it could
	// not find, and the flag that wanted it is spelled differently: reading the
	// flag out of "llvm-config" gives --with-llvm-config, which is not in the
	// list, and the drop misses while the message plainly says --with-llvm.
	pgFlags := []string{"--with-openssl", "--with-libxml", "--with-icu", "--with-llvm", "--with-pam"}
	r.Check("postgres' llvm-config error drops the flag the MESSAGE names",
		say("pg", "configure: error: llvm-config not found, but required when "+
			"compiling --with-llvm, specify with LLVM_CONFIG=\n", pgFlags),
		"drop:--with-llvm")

	// The older route still works: the message names a library and nothing
	// else, so the flag is spelled from the name.
	r.Check("a bare 'not found' still spells the flag from the library",
		say("bare", "configure: error: libsomething was not found\n",
			[]string{"--with-something", "--with-icu"}),
		"drop:--with-something")

	// ⛔ AND THE SILENCES, which are the half a pattern change breaks quietly.
	r.Check("a flag the message names but the build does not carry is not dropped",
		say("absent", "configure: error: llvm-config not found, but required when "+
			"compiling --with-llvm\n", []string{"--with-openssl"}),
		"")
	r.Check("a log with no fatal configure line says nothing",
		say("quiet", "checking for gawk... no\nchecking for mawk... mawk\n", pgFlags),
		"")
	r.Check("a missing log file says nothing",
		diagnose(filepath.Join(dir, "does-not-exist.log"), pgFlags), "")

	// autoconf's own suggestion is taken as an ADD, and only for a disable.
	r.Check("autoconf's `use --without-x' suggestion is added, not dropped",
		say("suggest", "configure: error: something\nuse `--without-tcl' to disable\n",
			[]string{"--with-openssl"}),
		"add:--without-tcl")

	return r
}
