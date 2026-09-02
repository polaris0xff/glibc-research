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
	pgFlags2 := []string{"--with-openssl", "--with-liburing", "--with-libcurl"}
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

	// ⛔ THE ONE THAT STOPPED IT NEXT. PKG_CHECK_MODULES names the pkg-config
	// module in parentheses and no flag at all, so neither of the two rules
	// above can see it.
	r.Check("a pkg-config module maps to the flag that asked for it",
		say("pkg", "configure: error: Package requirements (liburing) were not met:\n"+
			"No package 'liburing' found\n", pgFlags2),
		"drop:--with-liburing")
	// The same message with a version constraint and more than one module: the
	// name is the first token of an entry, not the whole entry.
	r.Check("a version constraint is not part of the module name",
		say("pkgver", "configure: error: Package requirements (libzstd >= 1.4.0, liburing) were not met:\n",
			[]string{"--with-openssl", "--with-zstd"}),
		"drop:--with-zstd")

	// A third spelling of the same fact, quoting the library instead.
	r.Check("a quoted 'library X is required' maps to its flag",
		say("reqlib", "configure: error: library 'libnuma' is required for NUMA support\n",
			[]string{"--with-openssl", "--with-libnuma"}),
		"drop:--with-libnuma")

	// ⛔ A STATIC BUILD ASKED FOR A SHARED OBJECT. Not a missing package: a
	// structural refusal, and the capitalised language name has to be folded
	// to match the flag.
	r.Check("'could not find shared library for Python' drops --with-python",
		say("shared", "configure: error: could not find shared library for Python\n",
			[]string{"--with-openssl", "--with-python"}),
		"drop:--with-python")

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
	// ⛔ The unquoted suggestion is anchored on a following " to ", so a flag
	// merely named in prose is not mistaken for advice.
	r.Check("a flag mentioned in prose is not taken as a suggestion",
		say("prose", "configure: error: something failed\n"+
			"see the manual for --without-readline and other options\n", pgFlags),
		"")

	// autoconf's own suggestion is taken as an ADD, and only for a disable.
	r.Check("autoconf's `use --without-x' suggestion is added, not dropped",
		say("suggest", "configure: error: something\nuse `--without-tcl' to disable\n",
			[]string{"--with-openssl"}),
		"add:--without-tcl")

	// ⛔ postgres writes the same courtesy unquoted, and this is the rung that
	// stopped arm S at readline: libreadline.a and libncursesw.a were BOTH in
	// the prefix, and AC_SEARCH_LIBS probes -lreadline alone, so the archive's
	// ncurses references go unresolved and configure calls the library absent.
	r.Check("postgres' unquoted 'Use --without-readline to disable' is added",
		say("plain", "configure: error: readline library not found\n"+
			"Use --without-readline to disable readline support.\n",
			[]string{"--with-openssl"}),
		"add:--without-readline")

	return r
}
