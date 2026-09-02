// sonamescan_selftest.go — the fast soname scan must find exactly what the
// naive one finds.
//
// ⛔ WHY THIS FILE EXISTS. sonamesMentionedInObjects decides which libraries a
// bundle KEEPS: a soname spelled out inside an ELF is the fingerprint of a
// `dlopen` by name, and a library nothing else references survives only
// because this function found it. It was rewritten from "one bytes.Contains
// per needle per object" to a single pass, because on a kdenlive-sized bundle
// the first shape is a thousand objects re-read a thousand times.
//
// ⚠ A SPEEDUP THAT SILENTLY MISSED ONE NAME WOULD DELETE A LIBRARY THE
// APPLICATION LOADS — on somebody else's machine, with every DT_NEEDED still
// resolving and every gate in this tree green. That is precisely how
// libSDL3.so.0 got deleted and reached a run. So the two implementations are
// compared on fixtures built to hit the ways the fast path could differ.
//
// SPDX-License-Identifier: MIT
package bundle

import (
	"os"
	"path/filepath"
	"strings"

	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// SonameScanSelftest compares the fast scan against the naive one.
func SonameScanSelftest() *selftest.Report {
	r := selftest.New("bundle-soname-scan")

	dir, err := os.MkdirTemp("", "pgb-soname-scan-")
	if err != nil {
		r.Fail("tempdir", err.Error(), "created")
		return r
	}
	defer os.RemoveAll(dir)
	lib := filepath.Join(dir, "lib")
	if err := os.MkdirAll(lib, 0o755); err != nil {
		r.Fail("mkdir", err.Error(), "created")
		return r
	}

	// The bundle's index: the names that can be found. ⚠ Two of them are NOT
	// mentioned by anything, so a scan that returned "everything" would fail.
	index := map[string]string{}
	for _, n := range []string{
		"libplain.so.0",       // spelled out bare
		"libinpath.so.2",      // only ever inside an absolute path
		"libtight.so",         // adjacent to other name bytes on both sides
		"libinlist.so.1",      // inside a space-separated list
		"libself.so.3",        // ONLY mentioned by itself: must not be found
		"libunmentioned.so.4", // mentioned by nothing at all
		"notalib.txt",         // not a shared object: never a needle
	} {
		p := filepath.Join(lib, n)
		if err := os.WriteFile(p, []byte("placeholder"), 0o644); err != nil {
			r.Fail("write "+n, err.Error(), "created")
			return r
		}
		index[n] = p
	}

	// The objects, each written to exercise one way a soname can appear. NUL
	// bytes and non-printable bytes are deliberate: real ELF string tables are
	// NUL-separated, and the fast path splits on exactly the bytes that cannot
	// be part of a needle.
	objects := map[string]string{
		"caller-a.so.1": "\x00\x7f\x01ELF\x00libplain.so.0\x00some other string\x00",
		"caller-b.so.1": "\x00/nix/store/aaaa-x/lib/libinpath.so.2\x00",
		// ⛔ THE ADJACENCY CASE, AND THE PADDING BYTES ARE THE WHOLE POINT.
		// The needle must be a substring of a longer run of ALPHABET
		// characters, so a scan comparing whole tokens misses it while
		// `bytes.Contains` does not.
		//
		// ⚠ THE FIRST VERSION OF THIS FIXTURE PADDED WITH `xx`/`yy` AND
		// TESTED NOTHING. No needle here contains `x` or `y`, so those bytes
		// are not in the derived alphabet, so they SPLIT the run -- and what
		// the fast path actually saw was the bare `libtight.so`. Both
		// implementations agreed, the assertion went green, and a deliberate
		// whole-token regression planted to test this very guard passed all
		// seven cases. The padding is now `aa`/`bb`, which every `lib*` needle
		// puts in the alphabet.
		"caller-c.so.1": "\x00aalibtight.sobb\x00",
		"caller-d.so.1": "\x00libinlist.so.1 libplain.so.0\x00",
		// ⚠ Mentions its own name only: the rule is that an object naming
		// itself is not evidence anything loads it.
		"libself.so.3": "\x00libself.so.3\x00",
	}
	var paths []string
	for n, body := range objects {
		p := filepath.Join(lib, n)
		if err := os.WriteFile(p, []byte(body), 0o644); err != nil {
			r.Fail("write "+n, err.Error(), "created")
			return r
		}
		if _, ok := index[n]; !ok {
			index[n] = p
		}
		paths = append(paths, p)
	}

	fast := sonamesMentionedInObjects(dir, paths, index)
	slow := sonamesMentionedNaive(dir, paths, index)

	base := func(in []string) string {
		var out []string
		for _, p := range in {
			out = append(out, filepath.Base(p))
		}
		return strings.Join(out, " ")
	}
	// ⭐ THE ASSERTION THAT MATTERS: not "the fast one found the right things"
	// but "the two agree", so the control is the thing being trusted.
	r.Check("the fast scan agrees with the naive one, exactly",
		base(fast), base(slow))

	// And the fixtures' own expectations, so a change that broke BOTH the same
	// way is still caught.
	got := map[string]bool{}
	for _, p := range fast {
		got[filepath.Base(p)] = true
	}
	r.CheckBool("a bare soname is found", got["libplain.so.0"], true)
	r.CheckBool("a soname inside an absolute path is found", got["libinpath.so.2"], true)
	r.CheckBool("a soname with name bytes either side is found", got["libtight.so"], true)
	r.CheckBool("a soname inside a space-separated list is found", got["libinlist.so.1"], true)
	// ⛔ The two negatives.
	r.CheckBool("an object naming only ITSELF is not a root", got["libself.so.3"], false)
	r.CheckBool("a library nothing mentions is not a root", got["libunmentioned.so.4"], false)
	return r
}
