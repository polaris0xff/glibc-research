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
	"sort"
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
		// ⛔ THE SAME SHAPE AGAIN, BUT THE SECOND NAME IS A HARDLINK. See the
		// os.Link below: nix optimises its store by hardlinking identical
		// files, and an AppDir assembled with `cp -al` hardlinks everything, so
		// a bundle reaching this scan can carry a SONAME that is a hardlink
		// rather than a symlink. `filepath.EvalSymlinks` cannot see through
		// one — a hardlink is not a symlink, it IS the file — so the two names
		// land in different selfKeys groups and the library becomes a root of
		// itself, which is precisely the defect the symlink case above records.
		"libhard.so.9.0.1": "\x00libhard.so.9\x00",
		// ⛔ THE CASE THIS FIXTURE DID NOT HAVE, AND IT IS THE ORDINARY ONE.
		// `libself.so.3` is a file whose NAME equals its SONAME, which is not
		// what a real library looks like: `libunistring.so.5.2.1` carries
		// `DT_SONAME libunistring.so.5` and has that name beside it as a
		// symlink. The self-check compared the needle against
		// `filepath.Base(o)`, so for every versioned library it compared
		// "libversioned.so.7" against "libversioned.so.7.1.2", never matched,
		// and the library became a ROOT OF ITSELF through the SONAME in its
		// own .dynstr.
		//
		// ⭐ Measured on the real `jq` AppDir before the fix: roots 40, and 12
		// of them were this. TODO T-066.
		"libversioned.so.7.1.2": "\x00libversioned.so.7\x00",
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

	// ⭐ THE SONAME SYMLINK, which is what makes the case above realistic: the
	// index holds `libversioned.so.7` and it resolves to the versioned file.
	// Without this the two names are unrelated and the case tests nothing.
	if err := os.Symlink(filepath.Join(lib, "libversioned.so.7.1.2"),
		filepath.Join(lib, "libversioned.so.7")); err != nil {
		r.Fail("symlink libversioned.so.7", err.Error(), "created")
		return r
	}
	index["libversioned.so.7"] = filepath.Join(lib, "libversioned.so.7")

	// ⭐ THE SAME RELATIONSHIP, EXPRESSED AS A HARDLINK. `libhard.so.9` and
	// `libhard.so.9.0.1` are one inode under two names, and no symlink resolves
	// between them.
	if err := os.Link(filepath.Join(lib, "libhard.so.9.0.1"),
		filepath.Join(lib, "libhard.so.9")); err != nil {
		// ⚠ An absence is not a zero: a filesystem that refuses hardlinks
		// leaves the case unmeasured rather than passed.
		r.Skip("os.Link for the hardlinked SONAME: " + err.Error())
	} else {
		index["libhard.so.9"] = filepath.Join(lib, "libhard.so.9")
	}

	fast := sonamesMentionedInObjects(dir, paths, index, nil)
	slow := sonamesMentionedNaive(dir, paths, index, nil)

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
	// ⛔ THE SAME RULE FOR THE ORDINARY SHAPE, and this is the one that was
	// broken: the needle is the SONAME, the file is the versioned name, and
	// the two are never equal. Both names must be excluded, because both name
	// the same file. Fails before the selfKeys() fix, passes after.
	r.CheckBool("a VERSIONED object naming only its own SONAME is not a root",
		got["libversioned.so.7"], false)
	r.CheckBool("...nor under its own file name", got["libversioned.so.7.1.2"], false)
	r.CheckBool("a library nothing mentions is not a root", got["libunmentioned.so.4"], false)

	// ⭐ selfKeys() PINNED DIRECTLY, not through what it happens to make the
	// two scans do.
	//
	// ⛔ WHY, AND IT IS THE OTHER HALF OF R2. The assertion above is an
	// EQUIVALENCE, so it is only as strong as the independence of its two
	// sides — and for one commit there was none, because the selfKeys fix was
	// applied to the subject and to the control together. selfSetNaiveFor now
	// restores that independence; these cases are the belt to its braces, and
	// they are the ones that would still fire if both scans were changed the
	// same wrong way at once.
	group := func(name string) string {
		keys := selfSetFor(selfKeys(index), filepath.Join(lib, name))
		var out []string
		for k := range keys {
			out = append(out, k)
		}
		sort.Strings(out)
		return strings.Join(out, " ")
	}
	r.Check("selfKeys groups a versioned file with its SONAME symlink",
		group("libversioned.so.7.1.2"), "libversioned.so.7 libversioned.so.7.1.2")
	r.Check("...and reaches the same group from the symlink",
		group("libversioned.so.7"), "libversioned.so.7 libversioned.so.7.1.2")
	r.Check("selfKeys leaves an unrelated library in a group of its own",
		group("libself.so.3"), "libself.so.3")
	if index["libhard.so.9"] != "" {
		// ⛔ THE CASE THAT DISAGREED. Recorded as an assertion so it stays
		// disagreed-about if the grouping ever goes back to path strings.
		r.Check("selfKeys groups a versioned file with its SONAME HARDLINK",
			group("libhard.so.9.0.1"), "libhard.so.9 libhard.so.9.0.1")
	}
	return r
}
