// diagnose.go — one pattern, one fix, and the message the build actually
// printed.
//
// Every rule here comes from a real failure and acts on text the build emitted.
// A diagnoser that guesses is worse than none: it turns a clear failure into a
// loop. A MANDATORY dependency is deliberately not in these tables — it is
// either built by the dependency walk or the build fails saying so.
//
// SPDX-License-Identifier: MIT
package nixx

import (
	"os"
	"regexp"
	"slices"
	"strings"
)

var (
	// meson names an option it does not have, which is the courtesy half of
	// the same behaviour: a component's own -Dcpuid=enabled is meaningless at
	// the top of a multi-subproject tree and meson says exactly that.
	mesonUnknownOption = regexp.MustCompile(`ERROR: Unknown option: "([A-Za-z0-9_-]+)"`)
	// meson says what to do, in the error, in its own words. Only
	// -Dname=false|disabled is taken: a suggestion to ENABLE something is a
	// different sentence and is not acted on.
	mesonDisableHint = regexp.MustCompile(`with (-D[A-Za-z0-9_-]+=(?:false|disabled))`)
	// autoconf's own error text very often names the flag that turns the
	// missing thing off. Taking the flag from the message is targeted; trying
	// flags until one works is the search this loop must not become.
	autoconfSuggestion = regexp.MustCompile("use `(--(?:disable|without)-[A-Za-z0-9_-]+)'")
	// The same courtesy without autoconf's quoting, which postgres writes as
	// `Use --without-readline to disable readline support.` Anchored on a
	// following " to " so a flag merely mentioned in prose is not taken.
	plainSuggestion = regexp.MustCompile(`(?i)use (--(?:disable|without)-[A-Za-z0-9_-]+) to `)
	// The error names the library and the flag that asked for it is spelled
	// from its name.
	configureMissing = regexp.MustCompile(`configure: error: ((?:lib)?[A-Za-z0-9_+-]+) (?:was not found|not found)`)
	// ⛔ The flag is not always spellable from the thing reported missing.
	// postgres reports `llvm-config not found` for the flag `--with-llvm`, so
	// spelling it gives --with-llvm-config and the drop misses. When the fatal
	// line NAMES the option, that name is the answer and no spelling is needed.
	configureNamesFlag = regexp.MustCompile(`configure: error:[^\n]*?(--(?:with|enable)-[A-Za-z0-9_+-]+)`)
	// PKG_CHECK_MODULES names the pkg-config module and no flag at all:
	// `configure: error: Package requirements (liburing) were not met`. The
	// module list can carry version constraints, so only the first token of
	// each entry is a name.
	pkgConfigMissing = regexp.MustCompile(`configure: error: Package requirements \(([^)]+)\) were not met`)
	// A third spelling of the same fact, quoting the library instead:
	// `configure: error: library 'libnuma' is required for NUMA support`.
	configureRequiresLib = regexp.MustCompile(`configure: error: library '([^']+)' is required`)
	// ⛔ A DIFFERENT KIND OF REFUSAL, and worth separating from the three
	// above: this one is not a missing package but a static build being asked
	// for a SHARED object that cannot exist in it. postgres' language bindings
	// say it as `could not find shared library for Python`. The flag still has
	// to come off, but the reason is structural rather than an absent input.
	configureWantsShared = regexp.MustCompile(`configure: error: could not find shared library for ([A-Za-z0-9_+-]+)`)
	// The feature is named at the END rather than the input: postgres writes
	// `could not find function 'gss_store_cred_into' required for GSSAPI`.
	configureRequiredFor = regexp.MustCompile(`configure: error:[^\n]*? required for ([A-Za-z0-9_+-]+)`)
	// Last resort, and only after every rule above has declined: in a fatal
	// line ending "not found", the FIRST word is usually the feature rather
	// than the token immediately before it — postgres writes `Tcl shell not
	// found` for --with-tcl. It is safe as a last rule because flagForName
	// answers "" unless the build actually carries the flag.
	configureFirstWord = regexp.MustCompile(`configure: error: ((?:lib)?[A-Za-z0-9_+-]+)[^\n]*?not found`)
)

// flagForName returns the drop directive for the option that asked for a
// dependency named in an error, trying the conventional spellings with and
// without a lib prefix. It answers "" when the build carries none of them,
// which keeps this targeted rather than a search over the flag list.
//
// ⚠ An option can carry a value — postgres asks for uuid as `--with-uuid=e2fs`
// — so a candidate matches either exactly or as the part before an `=`, and
// what is dropped is the flag as the build actually spells it.
func flagForName(name string, flags []string) string {
	cands := []string{
		"--with-" + name, "--enable-" + name,
		"--with-" + strings.TrimPrefix(name, "lib"),
		"--enable-" + strings.TrimPrefix(name, "lib"),
	}
	for _, cand := range cands {
		for _, f := range flags {
			if f == cand || strings.HasPrefix(f, cand+"=") {
				return "drop:" + f
			}
		}
	}
	return ""
}

// optionalDep maps a fatal configure message to the flag nixpkgs used to turn
// that optional dependency on. autoconf says it more than one way, so the
// library name is matched wherever it appears in the fatal line.
var optionalDep = []struct {
	Pattern *regexp.Regexp
	Flag    string
}{
	{regexp.MustCompile(`(?i)configure: error.*(libsensors|lm_sensors|sensors_init)`), "--enable-sensors"},
	{regexp.MustCompile(`(?i)configure: error.*(libcap|cap_init|sys/capability\.h)`), "--enable-capabilities"},
	{regexp.MustCompile(`(?i)configure: error.*(libnl-3|libnl/socket\.h|netlink/attr\.h)`), "--enable-delayacct"},
	{regexp.MustCompile(`(?i)configure: error.*(systemd|libsystemd)`), "--enable-systemd"},
	{regexp.MustCompile(`(?i)configure: error.*(utempter)`), "--enable-utempter"},
	{regexp.MustCompile(`(?i)configure: error.*(utf8proc)`), "--enable-utf8proc"},
	{regexp.MustCompile(`(?i)configure: error.*(sixel)`), "--enable-sixel"},
}

var readlineMissing = regexp.MustCompile(
	`cannot find -lreadline|readline/readline\.h: No such file|WARNING: could not find a version of the installed readline`)

// diagnose reads a build log and returns a fix directive — "drop:FLAG",
// "add:FLAG" or "env:NAME=VALUE" — or "" when it has nothing to say.
func diagnose(logFile string, flags []string) string {
	b, err := os.ReadFile(logFile)
	if err != nil {
		return ""
	}
	text := string(b)
	has := func(f string) bool {
		return slices.Contains(flags, f)
	}

	if m := mesonUnknownOption.FindStringSubmatch(text); m != nil {
		for _, f := range flags {
			if strings.HasPrefix(f, "-D"+m[1]+"=") {
				return "drop:" + f
			}
		}
	}
	if m := mesonDisableHint.FindStringSubmatch(text); m != nil && !has(m[1]) {
		return "add:" + m[1]
	}

	// nixpkgs passes --with-installed-readline because it builds against
	// nixpkgs' readline; there is no static readline in the pgb environment and
	// bash ships its own, so dropping the flag builds the bundled copy.
	if readlineMissing.MatchString(text) && has("--with-installed-readline") {
		return "drop:--with-installed-readline"
	}

	for _, rule := range optionalDep {
		if rule.Pattern.MatchString(text) && has(rule.Flag) {
			return "drop:" + rule.Flag
		}
	}

	// Before spelling a flag from a library name, take one the message states.
	if m := configureNamesFlag.FindStringSubmatch(text); m != nil && has(m[1]) {
		return "drop:" + m[1]
	}

	// A pkg-config module maps to the flag that asked for it by name.
	if m := pkgConfigMissing.FindStringSubmatch(text); m != nil {
		for _, entry := range strings.Split(m[1], ",") {
			fields := strings.Fields(entry)
			if len(fields) == 0 {
				continue
			}
			if d := flagForName(fields[0], flags); d != "" {
				return d
			}
		}
	}

	if m := configureRequiresLib.FindStringSubmatch(text); m != nil {
		if d := flagForName(m[1], flags); d != "" {
			return d
		}
	}

	if m := configureWantsShared.FindStringSubmatch(text); m != nil {
		if d := flagForName(strings.ToLower(m[1]), flags); d != "" {
			return d
		}
	}

	if m := configureRequiredFor.FindStringSubmatch(text); m != nil {
		if d := flagForName(strings.ToLower(m[1]), flags); d != "" {
			return d
		}
	}

	if m := configureMissing.FindStringSubmatch(text); m != nil {
		name := m[1]
		bare := strings.TrimPrefix(name, "lib")
		for _, cand := range []string{"--with-" + name, "--with-" + bare,
			"--enable-" + name, "--enable-" + bare} {
			if has(cand) {
				return "drop:" + cand
			}
		}
	}

	if m := autoconfSuggestion.FindStringSubmatch(text); m != nil && !has(m[1]) {
		return "add:" + m[1]
	}
	if m := plainSuggestion.FindStringSubmatch(text); m != nil && !has(m[1]) {
		return "add:" + m[1]
	}

	// Last resort: the first word of a fatal "not found" line.
	if m := configureFirstWord.FindStringSubmatch(text); m != nil {
		if d := flagForName(strings.ToLower(m[1]), flags); d != "" {
			return d
		}
	}

	// nixpkgs pins the autoconf cache variable cf_cv_type_of_bool for its own
	// compiler; with another gcc that produces a typedef in a translation unit
	// with no <stdbool.h>. Dropping the override lets configure work it out.
	if strings.Contains(text, "unknown type name 'bool'") && has("cf_cv_type_of_bool=bool") {
		return "drop:cf_cv_type_of_bool=bool"
	}

	// glibc folds libdl into libc from 2.34, but a configure script from an
	// older tarball can still emit a link line without it.
	if strings.Contains(text, "undefined reference to `dlopen'") {
		return "env:LIBS=-ldl"
	}
	return ""
}

// quirksFor is measured knowledge this repository already paid for. Every
// entry cites the POC or experiment that measured the failure it prevents; a
// package that merely fails to build gets a rule in diagnose instead, which
// acts on the message the build printed.
func quirksFor(pname string) []string {
	if strings.HasPrefix(pname, "ncurses") {
		// ncurses compiles its terminfo SEARCH PATH in at configure time and
		// derives it from --prefix, so one built into a private prefix
		// produces binaries that look for terminal descriptions under the
		// build machine's prefix and nowhere else. poc/20-nano measured
		// setupterm() returning "no database" on all eleven environments,
		// including the seven that ship a perfectly good /usr/share/terminfo.
		return []string{
			"--without-shared", "--with-normal", "--enable-widec", "--without-debug",
			"--without-ada", "--without-manpages", "--without-tests", "--enable-overwrite",
			"--with-default-terminfo-dir=/usr/share/terminfo",
			"--with-terminfo-dirs=/usr/share/terminfo:/lib/terminfo:/etc/terminfo:/usr/lib/terminfo:/usr/share/lib/terminfo",
		}
	}
	return nil
}
