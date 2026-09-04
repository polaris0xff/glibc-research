# HISTORY/entries/ci.md — the CLOSED ci entries

⛔ **Nothing here is work.** Every entry below is `done`. They were moved
out of `TODO/ci.md` on 2026-09-03c so that `TODO/` carries only what is
left, at the operator's instruction:

> *"strip away the fat, things that are already resolved and fixed and just
> send them straight into /HISTORY/\*, the TODO/\* must be lean and contain
> only what's left"*

⭐ **They keep their `T-` ids and their rows in [`../../TODO/INDEX.md`](../../TODO/INDEX.md)**,
which is what stops any of this being rediscovered. `sh TODO/check.sh`
checks this file against those rows exactly as it checked `TODO/`.

⛔ Do not reopen an entry here. A defect that still matters is a NEW entry.

---

## T-040 — Run CI once

**Source** `docs/AGENTS.md` §9 · **Category** ci · **Priority** P1 · **Effort** S · **Status** ✅ done

⛔ **The title's premise was false and the title keeps it**, per
`../docs/methodology/authoring.md`. CI had not "never run": it had run **ten
times and been red ten times** before this entry was written. The entry is now
*get it green*.

**Problem.** Runs 1–10 (`79bbfa33` … `b77e0333`) were all red, on the same two
rows, and on neither did a probe ever execute:

```
voidlinux    exec /__e/node24/bin/node: no such file or directory
alpine-3.10  Error relocating /__e/node24_alpine/bin/node:
               pthread_getname_np: symbol not found
               secure_getenv: symbol not found
```

A job using GitHub's `container:` has the runner inject its own dynamically
linked Node.js to execute JavaScript actions. It cannot start on Void's musl
(the runner picks the glibc build unless `ID=alpine`) or on Alpine 3.10's musl
1.1.22. `actions/download-artifact` died before the binary was fetched.

**Premise.** ⭐ **Measured, not predicted, and it holds.** The nine other rows
were green every run — `probe-portable` printed `PASSED: 0 failure(s)` — and
the plain control segfaulted on Arch, which is the positive control. This
session then ran the *whole* matrix locally under `docker run --entrypoint`
against the digest-pinned images: **11 of 11 portable ok, 11 of 11 plain
failed**, including both rows CI could not reach. The chroot bed agreed:
11 of 11 ok, zero host shared objects.

**Approach.** Done in this session; what remains is the green run itself.

1. every job on the `ubuntu-latest` host; targets entered with
   `docker run --entrypoint`, so the only process in the target image is the
   probe — no shell, no Node, no runner;
2. the matrix **generated** from `scripts/common/rootfs-images.txt` and
   asserted to be 11 rows. ⛔ Runs 1–10 hand-wrote tags (`archlinux:latest` is
   rolling), so CI and the local bed were two different beds reporting as one.
   Measured consequence: CI's Arch killed the control with **SIGSEGV**; the
   digest-pinned Arch kills it with **SIGFPE**;
3. an assertion that the two arms are different binaries, because every other
   assertion is made against the pgb arm and a no-op `pgb` would otherwise go
   green;
4. `TODO/check.sh` and `sh -n` over every script, as CI steps.

`../docs/history/corrections.md` C8.

**Prove.** A green run on a runner, with its URL recorded in this entry.

**Closed with** run 11, the first green run this workflow has ever had:
<https://github.com/polaris0xff/glibc-research/actions/runs/33506148035>
(`a1d30d3`, 2026-09-01). ⛔ **The rollup is not the evidence** — a run can be
green because it did less. 14 jobs, all `success`, and the matrix job names
carry the digest each row resolved to:

```
matrix                                          success   parsed 11 targets
build                                           success   incl. "Assert the two arms are actually different binaries"
probe-host                                      success   incl. TODO/check.sh and sh -n over every script
run-matrix (alpine-3.22,  musl,  alpine@sha256:7c8cb692…)          success
run-matrix (alpine-3.20,  musl,  alpine@sha256:c64c687c…)          success
run-matrix (alpine-3.10,  musl,  alpine@sha256:e515aad2…)          success   <- red in runs 1-10
run-matrix (voidlinux-musl, musl, voidlinux/…@sha256:d5c970d0…)    success   <- red in runs 1-10
run-matrix (debian-11,    glibc, debian@sha256:c0a2ad73…)          success
run-matrix (debian-12,    glibc, debian@sha256:2f65600e…)          success
run-matrix (ubuntu-20.04, glibc, ubuntu@sha256:c664f8f8…)          success
run-matrix (rockylinux-8, glibc, rockylinux@sha256:2d05a926…)      success
run-matrix (opensuse-leap-15.6, glibc, opensuse/leap@sha256:ca2942f9…) success
run-matrix (fedora-42,    glibc, fedora@sha256:7c63468d…)          success
run-matrix (archlinux-latest, glibc, archlinux@sha256:818793c8…)   success
```

⭐ **Both rows that had never executed a binary now execute one and it passes.**

⚠ **What this run does NOT establish**, so it is not read as more than it is:
the eleven rows assert the program's own exit status. They do **not** assert
"loaded no host shared object" — that is criterion 2 of `docs/AGENTS.md` §3
and it needs the trace instrument, which needs `pgb verify` to have a docker
engine. Carried as **T-014**, and until it lands CI is a weaker check than the
local bed. `podman` is still unexercised.


## T-084 — ⛔ the trace classifier is SIX hand copies, and one of them was wrong

**Source** ⭐ **found by a disagreement, 2026-09-03f**: `experiments/64-`
reported **2 host shared objects** for a bundle running on `alpine-3.22` — a
musl image with no `/usr/lib/x86_64-linux-gnu` at all.
**Category** ci · **Priority** P1 · **Effort** M · **Status** ✅ done

**Problem.** `strace` splits a long call across two lines:

    openat(AT_FDCWD, "/usr/lib/…/libGLX.so.1", O_RDONLY <unfinished ...>
    <... openat resumed>)                     = -1 ENOENT

The **path** is on the first line and the **result** on the second. Every copy
of the classifier filters with `!/ENOENT|= -1/`, which the first line
satisfies — so a FAILED open is recorded as a loaded object.
[`../docs/history/corrections.md`](../../docs/history/corrections.md) C25 has the
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
   | ⭐ a real trace's `execve("<artefact>")` count | **1**, so C38 is latent in that shape |
   | ⛔ **which of the six C38 REACHES** (arm S) | ⛔ **exactly one — `90-`** |

   ⛔ **AND `90-` IS THE ONE THIS ENTRY ALREADY NAMED.** It calls `tree` mode
   *and* its test script invokes the artefact **twice** (`melt -version`, then
   a real encode), and all eleven of its rows recorded `P=ok E=ok`, so both
   invocations ran everywhere. ⛔ Its host counts describe only the **second**
   invocation — **ours as well as the competitor's**.

   ⭐ **So step 2's re-run is no longer a fishing trip: it is `90-`.** The
   other five are latent — `60-` and `62-` invoke the artefact once, `85-` and
   `89-` once, and `86-` invokes it four times but calls only `payload`, the
   mode its copy agrees in. They still need converting; they do not need
   re-running to defend a committed number.

   ⛔ **The edit still waits for `65-` to finish**, and the reason is NOT the
   one that was written here. Measured, both directions:

   | | |
   |---|---|
   | editing a **sourced** library while a script that sourced it runs | ✅ **safe** — the function is already in memory, and a re-read never happens |
   | editing a script that is **being executed** | ⛔ **catastrophic**, and worse than "it changes": the shell re-entered the rewritten file at a shifted byte offset, ran a garbage line, and then **executed the tail a second time** |

   ⭐ So `lib.sh` itself was never the hazard. `65-` is, because it is
   resumable *and* it is the file that would need its call sites changed;

   ⭐ **THE CONVERSION IS DONE — 2026-09-04c.** All six hand copies are
   deleted and every call site calls `exp_classify_trace`. ⛔ **`65-` was not
   touched**: it already called the two-argument form, which is `tree`, which
   is what the function does by default — the ordering decision in step 1 is
   what made that safe while the corpus was executing.

   | | before | after |
   |---|---|---|
   | files defining `classify_trace` | 6 | ⭐ **0**, and `experiments/102-` G1 is the standing guard |
   | call sites | 10 | 10, all `exp_classify_trace`, modes unchanged |
   | awk lines carried by hand | 151 | ⭐ **0** |

   ⭐ **AND `102-` WAS REWRITTEN RATHER THAN DELETED, so the before/after the
   Prove line asks for survives the copies it measured.** It reads the six
   bodies back out of git at the pinned pre-conversion commit and runs them
   against the shared classifier on the same five fixtures: `pass=20 fail=0
   skip=0`, two runs identical. It also now asserts G1 (no copy comes back),
   G2 (call-site drift), and G3 (every mode passed is one the classifier
   implements, *and* that an unknown mode is a loud error rather than a
   silent `tree`).
2. re-run the ones whose numbers can have moved. ⛔ **`102-` arm S says that
   is exactly ONE — `90-`** — because C38 fires only where the copy's
   differing mode is called *and* the traced run invokes the artefact more
   than once. The other five are latent and pinned in
   `evidence/STALE-EVIDENCE.txt` with that reason; ⚠ **pinned is owed, not
   settled.**
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

⭐ **CLOSED 2026-09-04c.** The six hand copies of the trace classifier are gone; every call site uses `exp_classify_trace` from `experiments/lib.sh`. `experiments/102-` was rewritten to read the six copies back **out of git** at a pinned commit and re-run D1–D5/H1 against them, so the before/after outlives the code. ⭐ `90-` — the one experiment C38 actually reached — was re-run and **both committed numbers stand** (ours 11/11 clean, the competitor 4/11), with every per-row count now describing both invocations. ⚠ `102-`'s R1 could not fire until **C50**; it now does.
