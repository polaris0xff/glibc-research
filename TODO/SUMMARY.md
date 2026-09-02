# SUMMARY.md — the session of 2026-09-02

⛔ **Saved as well as printed**, per
[`../docs/methodology/sessions.md`](../docs/methodology/sessions.md), so it
survives the chat scrolling away. Overwritten each session.

⭐ **Every cell is grounded in something that can be pointed at**, and where a
thing was not measured this says so rather than giving a number.

| row | before | after |
|---|---|---|
| **Elapsed** | 2026-09-02T02:20Z | 2026-09-02T03:5xZ — **≈1h35m**, ⚠ **interrupted twice by the operator**, so this is not a planned session's shape |
| **Commits** | `2e4c6169` | `79c7e054` — **3 commits**, every one on `main`, pushed as they landed |
| **Work** | measurement work in flight | ⚠ **0 entries closed, 1 opened (T-061, P0)**, 2 defects fixed, 6 documentation defects fixed. ⛔ **The assigned measurement work was stopped, not finished** |
| **Changes** | — | **22 files**, 2,082 insertions(+), 231 deletions(-) |
| **Size** | — | **32,552 lines** of shell, Python, C and markdown, excluding `references/` and `evidence/` |
| **Checks** | `sh TODO/check.sh` green | green. ⭐ **Plus a new second gate**, `sh scripts/common/check-docs.sh`, also green |
| **Cost** | — | ⚠ **not metered.** What can be pointed at: two small reference files fetched (`tss/main.rs`, `stamp.ps1`), one onelf control package packed (46 MB), 31 nixpkgs dependencies built for T-060. Disk 7–9 GiB free throughout. No paid service used |
| **Health** | 33 entries, 15 open | **34 entries, 16 open.** ⛔ **One new P0.** Tree clean, `main` pushed, no `ephemeral-*` branches, no branch debt |

## What actually happened

⛔ **Two operator interrupts, and the session's shape is theirs, not a plan.**

1. **Measurement work** (the assigned task): diagnosed the boost failure in
   T-060's nix closure and the onelf arm of `experiments/90-`. Both turned out
   to be defects in **our own** code, not in the things being measured.
2. **Interrupt 1 — the documentation review.** `docs/` had gone thirteen
   commits without an edit while five entries changed state.
3. **Interrupt 2 — T-061.** The operator read the porting report and made the
   Go port a P0 that pre-empts everything.

## The two defects, because both were misattributed first

| what it looked like | what it was |
|---|---|
| `pgb: 1: .built: not found`, at the exact moment boost's round 1 began — read for an hour as a broken boost build | a COMMENT inside a double-quoted `_cmd="..."` assignment named a file in backticks. Backticks in double quotes are command substitution; the composing shell ran `.built`. **boost was never failing.** ⭐ This defect is the entire argument for T-061 |
| `experiments/90-`'s onelf arm: `Aborted`, no output, three runs — recorded as "onelf cannot run our payload" | onelf dispatches on **argv[0]'s basename** and falls back to the package default **silently**. The symlink was named `melt-onelf`, matched nothing, and ran **kdenlive** — which needs a display and died in `QMessageLogger::fatal`. Through a symlink named `melt` the same bundle answers in 0.4 s |

⭐ **The onelf control is the part that makes it a finding rather than a
guess**: a 141 MB, 188-library onelf package of the same nixpkgs ffmpeg, same
`[bundle] skip`, same compression level, runs on this machine. So the abort was
never about the payload or the host.

## The documentation review

⭐ **The mechanical half is a script now**, per `reviews.md`:
`scripts/common/check-docs.sh` — dead links, backticked repo paths, cited
evidence, referenced experiment numbers, quoted entry counts, and the vendored
set's own unresolved-link list. Six real defects on the way to green:

| # | defect |
|---|---|
| 1 | `docs/research/solo.md` said their CI *"has six jobs"* and named six. It has **nine**; three were dropped with nothing saying so. ⭐ The conclusion survives and is **re-derived beside the correction** |
| 2 | three files cited upstream's own limits document as though the path were ours |
| 3 | `docs/comparison.md` and `docs/design/toolchain.md` cited experiment 63 as if it existed — it is a number T-013 reserves and nothing more |
| 4 | `docs/history/corrections.md` and `docs/research/prior-art.md` cited onelf's guide by its upstream path, not the vendored one a reader can open |
| 5 | `gate.md` and `reviews.md` were required reading that **this tree did not have**, for a whole session |
| 6 | `TODO/PROGRESS.md` described the session before last |

## What was NOT done, and it is most of the assigned work

- ⛔ **`experiments/90-` is fixed and NOT re-run.** The recorded onelf row is
  the wrong one. **T-055 stays open with the wrong number in its evidence.**
- ⛔ **T-060 rung 1 did not finish.** 31 dependencies, boost in flight, on
  ephemeral `/var/tmp`. Rungs 2 and 3 untouched.
- ⛔ **The kdenlive bundle was not shrunk.** The 488,934,276-byte unreachable
  figure was measured and the sweep that produced it **was not committed**.
- ⛔ **The three deep reviews were not run as three separate passes.** The
  documentation review covered lens 1 (the door sweep, over `docs/`) and lens 3
  (the claim audit, which found the solo.md count). ⚠ **Lens 2, the guard
  mutation, was run only on the new checker** — its rules were planted and seen
  to fire while it was being written — **and not on anything else.** Saying it
  covered all three would be the fabrication `reviews.md` warns about.
- **Anything on a GPU**, **KF6**, **kdenlive static**, **a 32-bit
  application** — all untouched, all carried as open entries.
