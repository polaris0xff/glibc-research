# SUMMARY.md — the session of 2026-09-04

⛔ **Overwritten every session.** The work order is
[`PROGRESS.md`](PROGRESS.md); the closed entries are
[`../HISTORY/entries/`](../HISTORY/entries/).

    SCOPE     T-081 first, with its acceptance test named BEFORE the work
              by the operator: experiments/64- arm G must go 0 of 11 ->
              11 of 11 WITHOUT the bind arm C used. Then T-080 reopened
              and re-measured with three applications per category. Plus
              two axes flagged as "also open, neither blocking":
              /etc/services and the environment-default codeset.
              ⭐ MID-SESSION the operator added a 40+ application
              battle-test list, four questions, and a tooling ask.
    RESULT    ⭐ T-081 CLOSED (twice), T-085 and T-086 CLOSED, the four
              questions answered from source, the app list classified.
              ⛔ T-080 IS STILL RUNNING and that is the honest state.
              THREE corrections, all about instruments.

## ⭐ What moved

| | before | after |
|---|---|---|
| a compiled-in `/nix/store` path | ⛔ the bundle drew **0 of 11** | ⭐ **11 of 11, no bind**, twice; `--no-storefix` still 0 of 11 |
| a Python GUI application | ⛔ **no artefact at all** — `resolveEntry` oscillated five hops | ⭐ `meld` draws on **11 of 11**, zero host objects |
| `/etc/services` | measured, **no mechanism** | ⭐ `--embed-netdb`, 8/11 → **11 of 11**, host file still wins |
| the environment-default codeset | ⛔ the one axis musl beat both glibc columns **11-0** | ⭐ `--utf8-default`, **11 of 11**, and `LANG=C` still obeyed |
| the eleven glibc-static issues | 9 closed, 2 open | **10 closed, 1 open** (host `dlopen`) |
| the 40+ application list | a list | ⭐ eight rungs ordered **by mechanism** — [`../docs/research/app-corpus.md`](../docs/research/app-corpus.md) |
| a long bundle run | died silently on ENOSPC or a stray FUSE mount | `scripts/common/watchdog.sh`, with a `--selftest` |

## ⛔ THREE CORRECTIONS, AND ALL THREE ARE ABOUT INSTRUMENTS

⭐ **None was found by reading the code that contained it.** Two came from a
disagreement, one from a build log.

**C26 — the corpus had no positive control.** `experiments/65-` scored
`galculator` **0 of 11**; `experiments/64-` had measured the same subject at
**11 of 11, twice**, two days earlier. The mechanism was a copied constant:
`64-` waits 25 s for a window in **mount** mode, where a bundle starts in about
two seconds, and **150 s** for the one arm it runs in **extract** mode — and
`65-` runs *every* subject in extract mode and kept the 25. ⭐ Measured: a
bundle puts its first toplevel on the X server at **t+21 s**, unpack included.
⛔ **The budget was the symptom.** The defect was that five pre-registered
expectations contained **no control**, so nothing in the experiment could tell a
broken subject from a broken instrument. C6 now asserts that `gtk3-1`, `gtk3-2`
and `py-1` — `64-`'s arms G, X and P — come back 11 of 11, and both of its
failure modes were planted before it was believed.

**C27 — our own regex had the boundary defect we accused the field of.**
T-081's write-up said the field's `[^ \"']*` *"does not stop at `<`"*.
`storeRefRe` was `[^" ']*`. In a binary the match ran through the terminating
NUL, and three of the six paths a bundle reported as unresolvable were the
scanner rather than the bundle. Re-measured on the same AppDir: **13 of 13**
distinct text-file store paths are in the closure and **0** are not — the one
that "was not" was `…-dejavu-fonts-minimal-2.37<`. ⭐ **The argument survives
and is stronger**: the field's regex *substitutes* on a mis-bounded match; this
route never substitutes on a match it cannot resolve, so the worst a bad
boundary could do was **report** a path it should have rewritten.

**C28 — a review's own hypothesis, falsified by the plant it wrote.** The
review predicted `StoreRefToBundle` was the dangerous pattern because it
substitutes without asking the closure. With the old class restored, both cases
written to prove corruption **passed**: the substitution is `"store/" + name`,
so an over-captured name is reproduced verbatim. ⭐ What the review found
instead is a coupling nothing checks — the name must be the farm directory
`buildStoreFarm` created, and **no check reads a `.env` value back against the
tree**. T-092.

## The measurements, each with its verdict line

| | verdict | runs |
|---|---|---|
| `experiments/64-` — T-081's acceptance test, four arms | `pass=11 fail=0 skip=0` | **two**, every cell identical |
| `experiments/66-` — `--embed-netdb` | `pass=12 fail=0 skip=0` | **two**, identical |
| `experiments/67-` — `--utf8-default` | `pass=7 fail=0 skip=0` | **two**, identical |
| `experiments/65-` — the T-080 corpus, 26 subjects | ⏳ **RUNNING** | restarted twice, both times because the tool changed under it |

⛔ **The corpus is the unfinished half of the session and the record says so.**
It is resumable — a recorded row is never re-measured — and
[`RESUME.md`](RESUME.md) carries the state.

## ⭐ The operator's four questions, answered

Full answers with their evidence are in [`PROGRESS.md`](PROGRESS.md).

1. **Is the glibc-static work complete?** ⛔ **No.** Ten of eleven closed, one
   open (host `dlopen`, host-dependent by nature); a boundary *inside* the
   eleventh (`getaddrinfo` with a service name, 8 of 11); four of the ten closed
   only behind an opt-in flag; and the list grew **nine → ten → eleven on three
   consecutive days**.
2. **Multi-binary applications, and does renaming the bundle work?** ⭐ **Yes,
   by construction** — a static `ARGV0`/`argv[0]`/`$1` selector and automatic
   enumeration of the entry `bin/` — ⛔ **and it has never been run.** T-088.
3. **Can the walker count entry points?** ⭐ **It already prints the number**:
   `programs <prog> + N more`.
4. **Feature parity with anylinux?** Level or ahead on everything measured here;
   ⛔ they are ahead on **the sandbox** (which this bed cannot measure) and on
   **breadth**.

## What the next session inherits

⭐ **The work order is now ordered BY MECHANISM**, on the operator's rule
*"what will auto fix/complete what, not easy first"*. Rungs 1–3 of
[`../docs/research/app-corpus.md`](../docs/research/app-corpus.md) decide about
twenty of the forty subjects.

    T-080  ⏳ finish the corpus. RESUMABLE.
    T-088  rung 1 — multi-entry dispatch, shipped and never run.
    T-089  rung 2 — the one row store-paths.md marks NOT MEASURED.
    T-087  rungs 3+ — the battle-test corpus. ⚠ RULE 3 SUSPENDED HERE.
    T-090  rung 5 is a BED problem, not a bundler problem.
    T-084  six hand copies of the trace classifier, and lib.sh's shared
           version needs the `mode` argument before any can be deleted.

⚠ **Delivery rules 7 and 8 are new and each was paid for by a discarded run**:
check that a criterion can *finish*, and carry a **positive control**.
