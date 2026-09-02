// flags_selftest.go — the link line pgb composes, and the one rule about it
// that is not obvious from reading it.
//
// ⛔ THE DEFECT THIS EXISTS TO CATCH, measured 2026-09-02f on real host
// objects. --host-dlopen generates a provider table that declares every glibc
// symbol as
//
//	extern char iconv_open[] __attribute__((weak));
//
// which is an UNDEFINED reference — so `-Wl,--wrap=iconv_open` rewrites it to
// `__wrap_iconv_open`, and a WEAK undefined reference does not pull a member
// out of an archive. `__wrap_iconv_open` lives in libpgbruntime.a, so it stayed
// unresolved, the table entry held NULL, and every host object importing iconv
// failed to load naming the UNWRAPPED symbol:
//
//	pgb-elfload: libstdc++.so.6: undefined symbol: iconv_open@GLIBC_2.2.5
//
// Of 71 host objects carrying a PT_TLS, 36 loaded before the fix and 50 after —
// and the fixed build's outcome is IDENTICAL to a --no-iconv control, which is
// what says the wrap no longer decides anything.
//
// ⭐ SO THE RULE IS ABOUT WHERE A WRAPPER LIVES, NOT ABOUT iconv. A wrapper in
// an object file named on the command line is always linked; a wrapper inside
// an archive is reachable only from a STRONG reference or a -u. The cases below
// assert that for every symbol pgb wraps, in both directions, so a predicate
// stuck at either value fails one of them.
//
// Offline: it composes flags and reads no disk beyond the paths it prints.
//
// SPDX-License-Identifier: MIT
package wrapper

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/cfg"
	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// archiveWrapped are the symbols pgb wraps whose wrapper is compiled into
// libpgbruntime.a rather than into an object file on the link line. ⛔ These
// are the ones a weak undefined reference cannot reach.
var archiveWrapped = []string{"iconv_open", "iconv", "iconv_close"}

// objectWrapped are the symbols pgb wraps whose wrapper is in a .o that is
// named on the link line, so a weak reference resolves without help. Kept
// beside the list above because the distinction is the whole point.
var objectWrapped = []string{"dlopen", "dlsym", "dlclose", "dlerror", "setlocale"}

// FlagsSelftest asserts the composition of the injected link line.
func FlagsSelftest() *selftest.Report {
	r := selftest.New("wrapper-flags")

	// ⛔ THE RUNTIME DIRECTORY HAS TO LOOK BUILT, or the --host-dlopen branch
	// is skipped entirely and every case about it passes for the wrong
	// reason. hostDlopenLinkFlags() stats pgb-provider-table.o and returns nil
	// without it — which the first version of this selftest walked straight
	// into: it asserted pgb-dlopen.o was on the line and got nothing, because
	// the whole block had been elided. Nothing is compiled here; the file is
	// empty and only its existence is read.
	rd, err := os.MkdirTemp("", "pgb-flags-selftest")
	if err != nil {
		r.Skip("cannot create a temporary runtime directory: " + err.Error())
		return r
	}
	defer os.RemoveAll(rd)
	if err := os.WriteFile(filepath.Join(rd, "pgb-provider-table.o"), nil, 0o644); err != nil {
		r.Skip("cannot write into the temporary runtime directory: " + err.Error())
		return r
	}

	joined := func(c *cfg.Config, cxx bool) string {
		return strings.Join(LinkFlags(c, rd, cxx), " ")
	}
	base := func() *cfg.Config {
		c := cfg.Load("/nonexistent")
		// ⚠ Load() reads the environment, and a session that exported
		// PGB_OPT_* would otherwise make this selftest measure that session.
		c.UseIconv, c.EmbedLocale, c.EmbedCacert, c.EmbedTerminfo = true, false, false, false
		c.HostDlopen, c.WrapDlopen, c.TLSReserve = false, nil, 0
		return c
	}

	// --- the wrap itself, both directions --------------------------------
	on := joined(base(), false)
	off := func() string { c := base(); c.UseIconv = false; return joined(c, false) }()
	r.CheckBool("--wrap=iconv_open is injected when iconv is on",
		strings.Contains(on, "--wrap=iconv_open"), true)
	r.CheckBool("and NOT when --no-iconv",
		strings.Contains(off, "--wrap=iconv"), false)

	// --- ⛔ the defect: an archive wrapper under --host-dlopen ------------
	hd := func() string { c := base(); c.HostDlopen = true; return joined(c, false) }()
	for _, s := range archiveWrapped {
		r.CheckBool("--host-dlopen forces __wrap_"+s+" out of the archive",
			strings.Contains(hd, "-Wl,-u,__wrap_"+s), true)
	}
	// The two cases that already forced it, kept so a change that narrowed the
	// condition to --host-dlopen alone is caught.
	cxx := joined(base(), true)
	wd := func() string { c := base(); c.WrapDlopen = []string{"p=x.o"}; return joined(c, false) }()
	for _, s := range archiveWrapped {
		r.CheckBool("a C++ link forces __wrap_"+s,
			strings.Contains(cxx, "-Wl,-u,__wrap_"+s), true)
		r.CheckBool("--wrap-dlopen forces __wrap_"+s,
			strings.Contains(wd, "-Wl,-u,__wrap_"+s), true)
	}
	// ⛔ AND THE NEGATIVE DIRECTION, or every case above would pass against a
	// build that forced them unconditionally and this would assert nothing.
	// A plain C link with no host loader has no weak reference to serve and
	// pays no libiconv for one.
	for _, s := range archiveWrapped {
		r.CheckBool("a plain C link does NOT force __wrap_"+s,
			strings.Contains(on, "-Wl,-u,__wrap_"+s), false)
	}

	// --- the other half of the rule: wrappers that live in a .o ----------
	// ⚠ These need no -u, and asserting that is what keeps the fix from being
	// copy-pasted into a blanket "force every wrapper", which would link
	// mechanisms nobody asked for.
	both := func() string {
		c := base()
		c.HostDlopen, c.EmbedLocale = true, true
		return joined(c, false)
	}()
	for _, s := range objectWrapped {
		r.CheckBool("__wrap_"+s+" is not forced; its object is on the line",
			strings.Contains(both, "-Wl,-u,__wrap_"+s), false)
	}
	r.CheckBool("--embed-locale puts pgb-locale.o on the line",
		strings.Contains(both, "pgb-locale.o"), true)
	r.CheckBool("--host-dlopen puts pgb-dlopen.o on the line",
		strings.Contains(both, "pgb-dlopen.o"), true)

	// --- the flags that are not conditional at all -----------------------
	r.CheckBool("every link is -static", strings.Contains(on, "-static"), true)
	r.CheckBool("every link keeps --eh-frame-hdr",
		strings.Contains(on, "--eh-frame-hdr"), true)
	r.CheckBool("every link anchors the NSS constructor",
		strings.Contains(on, "-Wl,-u,pgb_runtime_anchor"), true)

	return r
}
