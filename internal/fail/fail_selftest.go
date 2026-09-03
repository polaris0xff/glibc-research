// fail_selftest.go — the exit-code contract, asserted.
//
// ⛔ WHY THIS ONE MATTERS OUT OF PROPORTION TO ITS SIZE. `docs/AGENTS.md` §0b
// makes the contract a rule of the project, not an implementation detail:
//
//	0 ok, 1 it ran and failed, 2 it could not run.
//	⛔ A skip is neither a pass nor a failure. A check that quietly runs
//	nothing and reports success is the worst answer this codebase can give.
//
// Every experiment, every POC and every gate reads those three numbers, and
// until now nothing carried in the binary asserted that `pgb` produces them.
// TODO T-062.
//
// SPDX-License-Identifier: MIT
package fail

import (
	"errors"
	"fmt"
	"strconv"

	"github.com/polaris0xff/glibc-research/internal/selftest"
)

// Selftest checks the exit-code contract.
func Selftest() *selftest.Report {
	r := selftest.New("fail-codes")

	code := func(err error) string { return strconv.Itoa(Code(err)) }

	// ---- the three numbers -------------------------------------------------
	r.Check("nil is 0", code(nil), "0")
	r.Check("Ran is 1 -- it ran and failed", code(Ran("boom")), "1")
	r.Check("Cannot is 2 -- it could not run", code(Cannot("no strace")), "2")
	// ⚠ AN ERROR FROM ANYWHERE ELSE defaults to 1. Defaulting to 2 would let
	// a real failure read as "could not run", which is the reading that makes
	// a red matrix row look like an absent one.
	r.Check("a plain error defaults to 1", code(errors.New("x")), "1")

	// ---- the wrapping promise, which is the doc comment's own claim --------
	//
	// ⛔ "Wrapping an error preserves the code of the innermost fail.Error, so
	// a helper's 'could not run' does not become a caller's 'ran and failed'."
	// That is the sentence at the top of fail.go and this is what checks it.
	// `internal/logx`, `internal/wrapper` and `internal/nixx` all wrap with
	// `fmt.Errorf("...: %w", err)`, so the property is load-bearing in three
	// packages.
	wrapped := fmt.Errorf("reading the manifest: %w", Cannot("no such file"))
	r.Check("a wrapped Cannot is still 2", code(wrapped), "2")
	r.Check("a doubly wrapped Cannot is still 2",
		code(fmt.Errorf("outer: %w", wrapped)), "2")
	r.Check("a wrapped Ran is still 1",
		code(fmt.Errorf("outer: %w", Ran("boom"))), "1")

	// ⚠ AND THE DIRECTION THE SENTENCE DOES NOT COVER, recorded because it is
	// surprising and nothing else says it. `errors.As` walks the chain from
	// the OUTSIDE IN and stops at the first match, so when one fail.Error
	// wraps another the OUTER one decides. "Innermost" in the doc comment
	// means "the innermost fail.Error under ordinary fmt.Errorf wrapping" --
	// which is every call site in this tree today, checked with grep: no
	// caller passes a %w verb to Ran or Cannot.
	//
	// ⛔ If one ever does, this case is what will tell them the code inverted.
	outerRan := Ran("outer: %w", Cannot("inner"))
	r.Check("⚠ an outer Ran over an inner Cannot resolves to the OUTER, 1",
		code(outerRan), "1")

	// ---- Silent: a status with no message ---------------------------------
	//
	// `main` asks IsSilent BEFORE Code, so a Silent error's status is used
	// whole. ⚠ Code() does not know about Silent and answers 1 for one; that
	// is only safe because of the order in run(), so both halves are pinned
	// here rather than left to be rediscovered.
	c, silent := IsSilent(Exit(3))
	r.CheckBool("Exit is silent", silent, true)
	r.Check("Exit carries its own status", strconv.Itoa(c), "3")
	_, notSilent := IsSilent(Ran("boom"))
	r.CheckBool("an ordinary error is NOT silent", notSilent, false)
	_, nilSilent := IsSilent(nil)
	r.CheckBool("nil is not silent", nilSilent, false)
	r.Check("⚠ Code() alone does NOT see Exit's status -- main asks IsSilent first",
		code(Exit(3)), "1")

	// ---- the message survives ---------------------------------------------
	//
	// ⚠ A code with no message is a status nobody can act on. `pgb` prints
	// `pgb: %s` from this string.
	r.Check("Ran formats its message", Ran("could not build %s", "jq").Error(),
		"could not build jq")
	r.Check("Cannot formats its message", Cannot("%d GiB free, need %d", 3, 10).Error(),
		"3 GiB free, need 10")
	r.Check("a wrapped error still reads whole", wrapped.Error(),
		"reading the manifest: no such file")

	return r
}
