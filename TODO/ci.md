# ci

⚠ **Open entries only.** T-040, which ran CI for the first time, is
[`../HISTORY/entries/ci.md`](../HISTORY/entries/ci.md); the long-form detail
behind the entry below is
[`../HISTORY/entries/ci-open.md`](../HISTORY/entries/ci-open.md).

---

## T-041 — aarch64

**Source** [`../docs/AGENTS.md`](../docs/AGENTS.md) §13 · **Category** ci · **Priority** P2 · **Effort** M · **Status** open

**Problem.** Every number in this repository is x86_64, one machine, one day.
`--arch arm64` exists in `pgb rootfs pull` and `pgb rootfs fetch` and
re-resolves by tag, trading the digest pin away.

**Premise.** ⚠ Expect IFUNC and CPU-baseline questions x86_64 did not raise.
`experiments/61-` shows glibc's advantage is largely IFUNC-dispatched
routines, so the throughput result may not carry.

**Prove.** `experiments/61-` and `62-` run on an aarch64 runner with their
tables filled.

📚 [detail](../HISTORY/entries/ci-open.md)

## T-077 — ⛔ the head-to-head was measured on the RETIRED pin, and nobody re-ran it

**Source** ⭐ **deep review 6, 2026-09-03c**, by asking of `docs/AGENTS.md` §9's
*"all 32 experiments · every one measured"* the same question that found T-076:
measured **by which version of itself?**
**Category** ci · **Priority** P1 · **Effort** M · **Status** open

**Problem.** Seven experiments had committed evidence older than the last
non-comment change to their own script. Four of them changed in the same way,
and it is not cosmetic:

    -ENV_ROOT="$ROOTFS_DIR/${PGB_ENV_NAME:-pgb-env-debian12}"
    +# ENV_ROOT comes from lib.sh, out of internal/cfg/cfg.go. T-070.

⛔ **So the committed numbers were measured inside `pgb-env-debian12` —
glibc 2.36 — and the script now measures `pgb-env-debian13`, glibc 2.41.**
The build environment the whole experiment runs in changed underneath its
evidence.

⛔ **And 60-, 61- and 62- ARE THE HEAD-TO-HEAD.** `docs/comparison.md` and
`docs/REQUIREMENTS.md`'s table — artefact size, malloc throughput, the
eleven-row coverage against AppImage, Flatpak, snap, onelf and static musl —
all come from them.

⚠ **This is not a claim that the numbers are wrong.** T-070 measured the pin
move and found its four named costs at zero, so they may be unchanged. It is a
claim that **nobody has checked**, and that the record read as though somebody
had.

⭐ **Three of the seven were re-run on 2026-09-03c and are now current** —
`30-` (pass=11 fail=0 skip=1), `70-` (pass=1 fail=0 skip=2, and its table now
says `pgb-env-debian13` where it said `pgb-env-debian12`) and `80-` (pass=16
fail=0). ⛔ **Four are pinned in [`../evidence/STALE-EVIDENCE.txt`](../evidence/STALE-EVIDENCE.txt)**
because between them they build five delivery formats and run two benchmark
matrices: hours, and the clock rows need the machine to themselves.

**What is left.** Re-run `60-`, `61-`, `62-` and `88-` on the current pin, on
an idle machine, and delete their lines from the ledger. ⚠ **Compare, do not
overwrite blindly** — `corrections.md` C23 is what happens when a re-run
silently replaces the numbers an entry quotes.

⭐ **The class is gated now**, which is the durable half:
`scripts/common/check-docs.sh` **gate 10** fails when an experiment's committed
evidence predates a non-comment change to its script. Comment-only edits do not
count, and an exemption is pinned to **both** commits so it cannot outlive its
reason.

**Prove.** `evidence/STALE-EVIDENCE.txt` is empty of entries, and gate 10
reports `0 pinned stale`.

## T-084 — ⛔ the trace classifier is SIX hand copies, and one of them was wrong

**Source** ⭐ **found by a disagreement, 2026-09-03f**: `experiments/64-`
reported **2 host shared objects** for a bundle running on `alpine-3.22` — a
musl image with no `/usr/lib/x86_64-linux-gnu` at all.
**Category** ci · **Priority** P1 · **Effort** M · **Status** open

**Problem.** `strace` splits a long call across two lines:

    openat(AT_FDCWD, "/usr/lib/…/libGLX.so.1", O_RDONLY <unfinished ...>
    <... openat resumed>)                     = -1 ENOENT

The **path** is on the first line and the **result** on the second. Every copy
of the classifier filters with `!/ENOENT|= -1/`, which the first line
satisfies — so a FAILED open is recorded as a loaded object.
[`../docs/history/corrections.md`](../docs/history/corrections.md) C25 has the
mechanism and the measurement.

⭐ **THE C25 ERROR ONLY RUNS ONE WAY.** It can turn a clean row dirty and can
never turn a dirty row clean. ⛔ A committed **non-zero** may be inflated, and
the one that matters is named: the competitor's *"4 of 11"* in
`docs/comparison.md` and `docs/AGENTS.md` §9, from `experiments/90-`.

⛔ **BUT THAT IS A CLAIM ABOUT C25, NOT ABOUT THE COPIES, AND THIS ENTRY READ
IT AS BOTH.** `experiments/102-` diffed them and found a **second**
difference that runs the **other** way — see C38. A committed **zero** from
`62-` or `90-` is therefore not automatically safe either.

**What is left.** ⭐ **The corrected implementation already exists and is
shared**: `experiments/lib.sh`'s `exp_classify_trace`, used by `64-` and `65-`.

⛔ **THIS PARAGRAPH SAID "NINE HAND COPIES" AND NAMED SEVEN FILES. BOTH NUMBERS
WERE WRONG AND THE ROUTE WAS WRONG TOO** — corrected 2026-09-04 by counting
`classify_trace()` definitions in the tree instead of trusting the sentence
that opened this entry:

- **SIX** experiments carry a hand copy — `60-`, `62-`, `85-`, `86-`, `89-`,
  `90-`. ⛔ **"Seven implementations" was wrong too**, and `experiments/102-`
  measured it rather than counting files: **three distinct texts** and, over
  five fixtures in both modes, **TWO distinct behaviours** — `60-` is one and
  the other five are the other. ⭐ That is what bounds step 2: converting the
  six is fixing **two** things, not six.
- ⛔ **`77-` was named and has no classifier and no `strace` at all.** It is a
  packing experiment (uruntime `full` → `lite`, the dwarfs block size). It was
  in the list because the list was written from memory.
- ⛔ **It is NOT "a deletion, not a rewrite".** Every one of the six takes a
  third argument the shared classifier does not implement: `mode`, either
  `payload` or `tree`. In `payload` mode the copy counts only opens in the pid
  that last `execve`d and clears its set at each exec, because an object opened
  before the last exec is not mapped in the running program; in `tree` mode it
  counts the whole process set. `exp_classify_trace` implements neither
  explicitly — it is `tree` without the clear.

1. ⛔ **`mode` MUST BE THE LAST ARGUMENT WITH A DEFAULT, NOT THE FIRST**, and
   this instruction said the opposite. Measured 2026-09-04b — the reasoning is
   about `experiments/65-`, which is **resumable**:

   - a resumed `65-` **re-sources** `lib.sh`, so it gets the NEW function;
   - if it still calls the OLD two-argument way, `mode` takes the *tracefile*
     and `want` takes nothing;
   - `want` empty means no `execve` line ever matches, so `inset` stays empty,
     so **every row reports zero host shared objects**.

   ⭐ **A silent, total false clean on the exact number the corpus exists to
   measure.** With `mode` last and defaulting to `tree` — which is what
   `exp_classify_trace` already does — an un-updated caller keeps today's
   behaviour instead. ⛔ Anything not in `{payload, tree}` must be a loud
   error, not a fallback.

   ⭐ **DONE, 2026-09-04b.** `exp_classify_trace <trace> <want> [mode]`, with
   `mode` last and defaulting to `tree`, an unknown mode a **loud error**
   rather than a fallback, and `sh experiments/lib.sh --selftest` — **9 cases,
   both modes**, guarded on `$0` so a sourced experiment cannot trip it. ⛔
   Verified against a planted C25 defect: **three** cases fail under it.
   ⭐ It runs in CI, beside the other shell selftests, because a selftest
   nothing runs is what let six copies drift in the first place.

   ⚠ **The running `65-` was measured unaffected**: it calls the two-argument
   form, which is `tree`, which is what the function did before.

   ⛔ **Step 2 — converting the six — is NOT done, and the reason is not
   timidity.** Changing an experiment without re-running it makes its
   committed evidence stale, which the documentation gate correctly reports;
   and `90-` and `86-` build kdenlive-scale bundles, which is the expensive
   half this entry is `M` for. Convert and re-run together.

   ⭐ **THE CHEAP HALF OF STEP 2 IS DONE, 2026-09-04b, AND IT CHANGES WHAT THE
   EXPENSIVE HALF IS FOR.** `experiments/102-classifier-equivalence.sh` runs
   all six copies and the shared classifier over five `strace`-shaped
   fixtures in both modes — **no bundle build** — and reports where they
   differ. `pass=15 fail=0`, two runs identical.

   | | |
   |---|---|
   | distinct **texts** | 3 |
   | ⭐ distinct **behaviours** | **2** — `60-` is one, the other five are the other |
   | positive control (a clean fixture) | all six agree with the shared one, both modes |
   | ⛔ C25, the split failed open | all six count it; the shared one does not |
   | ⛔ the artefact exec'd **twice** | **C38** — they disagree in *opposite* modes |
   | ⭐ a real trace's `execve("<artefact>")` count | **1**, so C38 is latent in this shape |

   ⛔ **So step 2's re-run is no longer a fishing trip.** It has one number to
   watch (`90-`'s competitor count, which is `tree` mode and therefore C38's
   direction) and a bound on the rest.

   ⛔ **The edit still waits for `65-` to finish**, and the reason is NOT the
   one that was written here. Measured, both directions:

   | | |
   |---|---|
   | editing a **sourced** library while a script that sourced it runs | ✅ **safe** — the function is already in memory, and a re-read never happens |
   | editing a script that is **being executed** | ⛔ **catastrophic**, and worse than "it changes": the shell re-entered the rewritten file at a shifted byte offset, ran a garbage line, and then **executed the tail a second time** |

   ⭐ So `lib.sh` itself was never the hazard. `65-` is, because it is
   resumable *and* it is the file that would need its call sites changed;
2. re-run all six, and compare the host counts before and after. ⚠ `90-` and
   `86-` build kdenlive-scale bundles, so this is the expensive half and it is
   why the entry is M rather than S;
3. if the competitor's count moves, `docs/comparison.md` and `docs/AGENTS.md`
   §9 change with it.

⛔ **THE ONE-WAY ARGUMENT DOES NOT COVER THE COPIES.** The split-`openat`
defect can only turn a clean row dirty, and that much still holds. ⭐ But
`102-` found a second difference — C38 — that turns a **dirty row clean** in
`tree` mode for five of the six, and a **clean row dirty** in `payload` mode
for `60-`. ⚠ What keeps the committed numbers standing is not the argument, it
is the measurement: a real trace execs the artefact **once**, so the clear has
nothing to clear.

**Prove.** Every one of the seven re-run against the shared classifier, with
the before/after host count for each printed side by side.

⚠ **Do not "fix" this by widening the ENOENT filter.** The result is genuinely
not on the line the path is on; pairing by pid is the only correct read, and a
cleverer regular expression would be the same guess in a new place.
