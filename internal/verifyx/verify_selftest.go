// verify_selftest.go — the trace classifiers, asserted offline.
//
// ⛔ WHY THIS PACKAGE NEEDED ONE MOST. `pgb verify` decides `docs/AGENTS.md`
// §3 criterion 2 — *loads no host shared object* — and it decides it from the
// functions below. Everything else in `verifyx` shells out to a bed and needs
// half an hour and a network; these four are pure, they are where the verdict
// is actually made, and until now nothing carried in the binary asserted any
// of them. TODO T-062.
//
// ⭐ AND THE DEFECTS THEY HAVE ALREADY HAD ARE THE CASES. Every assertion here
// that carries a ⛔ is a mistake this tree made and paid for, written down as
// a test rather than as a comment:
//
//   - `/etc/ld.so.cache` counted as a loaded shared object, because the rule
//     matched the substring ".so". It is an INDEX, every glibc process that
//     reaches dlopen opens it, and it reached committed evidence.
//   - the trace attributed to the wrong pid, so coreutils `cp` — dynamic on
//     this host — charged libacl, libattr, libselinux and libc.so.6 to the
//     subject. Read naively that says "the portable binary loaded seven host
//     libraries", which is false.
//
// ⚠ NO BED IS INVOLVED and none is needed: these are string in, verdict out.
// Whether the tracer *attaches* is a different question and stays with the
// matrix run, which reports `unmeasured` rather than `none` when it cannot.
//
// SPDX-License-Identifier: MIT
package verifyx

import (
	"strings"

	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// Selftest checks the pure classification surface of `pgb verify`.
func Selftest() *selftest.Report {
	r := selftest.New("verify-classify")

	// ---- resultOf: the cell every matrix row prints ------------------------
	r.Check("resultOf: 0 is ok", resultOf(0), "ok")
	r.Check("resultOf: 1 is exit1", resultOf(1), "exit1")
	r.Check("resultOf: 139 is SIG11", resultOf(139), "SIG11")
	r.Check("resultOf: 134 is SIG6", resultOf(134), "SIG6")
	// ⚠ THE BOUNDARIES, because the range is written as a literal. 128 is not
	// a signal (128+0), and 170 is past the highest signal this maps.
	r.Check("resultOf: 128 is NOT a signal", resultOf(128), "exit128")
	r.Check("resultOf: 129 is SIG1", resultOf(129), "SIG1")
	r.Check("resultOf: 169 is SIG41", resultOf(169), "SIG41")
	r.Check("resultOf: 170 is NOT a signal", resultOf(170), "exit170")

	// ---- isSharedObject: the rule that decides criterion 2 -----------------
	//
	// ⛔ THE FIRST CASE IS A REAL DEFECT THIS TREE SHIPPED. `/etc/ld.so.cache`
	// contains ".so" and is not an object; it reached
	// `evidence/poc/10-gawk/RESULT.txt` on all seven glibc rows. No verdict
	// was wrong there because a real object sat beside it on every such row —
	// but the same expression decides pass or fail in `pgb verify`, and a
	// binary that opened only the cache would have been failed for it.
	r.CheckBool("isSharedObject: /etc/ld.so.cache is NOT one",
		isSharedObject("/etc/ld.so.cache"), false)
	r.CheckBool("isSharedObject: libz.so", isSharedObject("/usr/lib/libz.so"), true)
	r.CheckBool("isSharedObject: libz.so.1", isSharedObject("/usr/lib/libz.so.1"), true)
	r.CheckBool("isSharedObject: libz.so.1.2.13",
		isSharedObject("/usr/lib/libz.so.1.2.13"), true)
	// ⚠ A directory whose name merely starts with the same letters.
	r.CheckBool("isSharedObject: a .sources file is NOT one",
		isSharedObject("/etc/yum.repos.d/fedora.sources"), false)
	r.CheckBool("isSharedObject: a bare name with no suffix is NOT one",
		isSharedObject("/usr/bin/jq"), false)

	// ---- parseStrace: pid attribution, and what it must ignore -------------
	//
	// A miniature trace in strace's own shape. pid 100 is the runner, pid 200
	// is the subject; the runner opens real shared objects BEFORE and AFTER
	// the subject's execve, and none of them may be charged to the subject.
	trace := strings.Join([]string{
		`100   execve("/bin/cp", ["cp"], 0x7ffd) = 0`,
		`100   openat(AT_FDCWD, "/lib/x86_64-linux-gnu/libacl.so.1", O_RDONLY) = 3`,
		`100   openat(AT_FDCWD, "/lib/x86_64-linux-gnu/libselinux.so.1", O_RDONLY) = 3`,
		`200   execve("/subject", ["/subject"], 0x7ffd) = 0`,
		`200   openat(AT_FDCWD, "/etc/ld.so.cache", O_RDONLY|O_CLOEXEC) = 3`,
		`200   openat(AT_FDCWD, "/usr/lib/libdemo.so.2", O_RDONLY) = 4`,
		`200   openat(AT_FDCWD, "/usr/lib/libmissing.so.9", O_RDONLY) = -1 ENOENT`,
		`200   openat(AT_FDCWD, "/etc/nsswitch.conf", O_RDONLY) = 5`,
		`100   openat(AT_FDCWD, "/lib/x86_64-linux-gnu/libattr.so.1", O_RDONLY) = 3`,
	}, "\n")

	libs, data, ok := parseStrace(trace, "/subject")
	r.CheckBool("parseStrace: the subject's execve was seen", ok, true)
	// ⛔ THE ATTRIBUTION CASE. Exactly one object, and it is the subject's.
	r.Check("parseStrace: only the subject's own opens are charged to it",
		strings.Join(libs, " "), "/usr/lib/libdemo.so.2")
	r.CheckBool("parseStrace: the runner's libacl is NOT charged",
		strings.Contains(strings.Join(libs, " "), "libacl"), false)
	r.CheckBool("parseStrace: an open AFTER the subject, by another pid, is not either",
		strings.Contains(strings.Join(libs, " "), "libattr"), false)
	// ⛔ ld.so.cache again, this time through the whole parser rather than the
	// predicate alone — the two must agree.
	r.CheckBool("parseStrace: /etc/ld.so.cache is not counted as an object",
		strings.Contains(strings.Join(libs, " "), "ld.so.cache"), false)
	// ⚠ A FAILED open is not a load. The binary asked and was told no.
	r.CheckBool("parseStrace: an ENOENT open is not a load",
		strings.Contains(strings.Join(libs, " "), "libmissing"), false)
	r.Check("parseStrace: host DATA is reported in its own column",
		strings.Join(data, " "), "/etc/nsswitch.conf")

	// ⛔ AND THE CASE THAT MUST NOT READ AS "CLEAN". A trace in which the
	// subject never exec'd is `ok=false` — "unmeasured" — and NOT an empty
	// library list, which is what "loaded nothing" looks like. `pgb verify`
	// reports `unmeasured`, never `none`, when it cannot attach, and this is
	// the half of that promise that is decidable offline.
	_, _, ok2 := parseStrace(trace, "/never-exec'd")
	r.CheckBool("parseStrace: a subject that never exec'd is UNMEASURED, not clean",
		ok2, false)

	// ---- classifyTracerOutput: the carried tracer's own format -------------
	//
	// ⭐ THE STRONGEST ASSERTION HERE: two independent instruments, one
	// answer. `pgb verify --engine chroot` reads strace's output and
	// `--engine docker` reads the carried tracer's, and `docs/AGENTS.md` §9
	// records that both arms agree on all 11 rows. That agreement is a
	// property of these two functions and is decidable without either engine.
	tracer := strings.Join([]string{
		`open /etc/ld.so.cache`,
		`open /usr/lib/libdemo.so.2`,
		`open /etc/nsswitch.conf`,
		`something else entirely`,
	}, "\n")
	tlibs, tdata := classifyTracerOutput(tracer)
	r.Check("classifyTracerOutput: the same objects strace's reader found",
		strings.Join(tlibs, " "), strings.Join(libs, " "))
	r.Check("classifyTracerOutput: the same data column",
		strings.Join(tdata, " "), strings.Join(data, " "))
	r.CheckBool("classifyTracerOutput: a line that is not an open is ignored",
		strings.Contains(strings.Join(tlibs, " ")+strings.Join(tdata, " "), "something"),
		false)

	// ---- classifyData: the buckets the data column reports -----------------
	//
	// ⚠ These are COLLAPSED on purpose. A binary opening forty gconv modules
	// is one fact, not forty, and §3 says data reads are reported rather than
	// asserted.
	r.Check("classifyData: a gconv path collapses to one bucket",
		classifyData("/usr/lib/x86_64-linux-gnu/gconv/gconv-modules.cache"), "gconv-cfg")
	r.Check("classifyData: a locale path collapses",
		classifyData("/usr/lib/locale/locale-archive"), "/usr/lib/locale/*")
	r.Check("classifyData: a terminfo path collapses",
		classifyData("/usr/share/terminfo/x/xterm-256color"), "terminfo")
	r.Check("classifyData: anything else is reported verbatim",
		classifyData("/etc/nsswitch.conf"), "/etc/nsswitch.conf")

	return r
}
