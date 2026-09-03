# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-03b, refreshed after the operator review landed
    TREE           main, clean, at 7c79dac5 + this
    BRANCH         ⛔ main. The harness named
                   `claude/cross-libc-dlopen-review-ukfukq`; RULES.md §Git
                   outranks it and THE OPERATOR SAID THE SAME ("Work on main,
                   never a claude/* branch"). Not created locally.
    CI             green as of 7c79dac5 (run 33699204833, 16 of 16).

---

# ⛔ WHAT A FRESH SESSION CANNOT INFER

⚠ **The clone comes up SHALLOW and `main` can come up BEHIND.** Measured again
this session: after `git fetch --unshallow`, `git rev-list --count
HEAD..origin/main` said **214**. Do this, in this order:

    git fetch --unshallow
    git rev-list --count HEAD..origin/main     ⛔ check it, do not assume
    git merge --ff-only origin/main

⚠ **The container is fresh: nothing is bootstrapped.** ~2 minutes.

    make                                     builds ./pgb, ~15 s
    ./pgb bootstrap --detach                 nix + env + bed, parallel
    ./pgb bootstrap --check                  is it ready
    sh scripts/common/install-codegraph.sh   v1.6.0, 96 files / 1,834 nodes

## In flight right now

    ⭐ NOTHING RUNNING. Everything is committed and pushed to main.
    CI green on every landed commit through 5d800353.

## ✅ THE OPERATOR REVIEW (cross-libc-dlopen #28 / PR 30) IS DONE, ALL FOUR

    1  T-073  el_own_syms[] was ONE table with TWO opposite requirements.
              __tls_get_addr must WIN; _dl_mcount_wrapper_check must YIELD.
              Split into el_own_syms_first / el_own_syms_last.
              experiments/94-, pass=16 fail=0, 11 of 11 on the bed.
              ⛔ THE VALUE HIDES IT: tls=0x5eeded is correct under the defect
              too, because the decoy is self-consistent. Only the call count
              separates them -- decoy_calls 0 fixed, 2 reversed.
    2  T-031  reference re-mined 1cecf50e -> 793f3f3f (PR 30's merge commit).
              mine-repo.sh now strips a third party's agent instruction file
              at fetch time and RECORDS the trim; the re-mine had silently
              put back the one docs/AGENTS.md §12 calls deliberately deleted.
              Two MORE were found across the 34 and removed.
              check-docs.sh gate 7 asserts none is vendored.
    3  T-074  the host-policy selftest's "is unset" assertions read the VALUE,
              and "" means BOTH absent and set-and-empty. Five of them could
              not fail on the dangerous state. present() now asks presence,
              and the instrument itself is asserted. Product unchanged.
    4  T-075  LD_DEBUG=bindings on 93-'s dynamic control, for the rows where
              the two loaders disagree. ⚠ It cannot see our own loader -- no
              PT_INTERP, no ld.so to read the variable -- and the comment
              says so. Two further placements stay open, each needing one
              measurement first.

## ⛔ WHAT IS LEFT, IN ORDER

    1  T-066 P0  ⛔ STILL THE LAST P0, premise significantly advanced.
                 ⭐ Route A's CEILING IS MEASURED: 218.5 MiB of a 938.8 MiB
                 mesa bundle (23.3%) is reachable ONLY through edges two
                 `-mini` recipes delete, against 6.3% the sweep can prove
                 dead. `pgb bundle sweep --cut FROM=>TO` is the instrument.
                 ⛔ LEFT: build the allowlist (bounded by that ceiling), and
                 cost route B. ⚠ The jq headline moved 1.22x -> 1.58x when a
                 stale pre-gating evidence file was re-run.
                 ⚠ A kdenlive AppDir still does not exist.
    2  T-062 P1  THREE packages left: buildx, logx, proc. verifyx and fail
                 landed; selftests 307 -> 359.
    3  T-063 P1  the miniflux proof. TWO NAMED FIXES in PROGRESS.md's work
                 order, and the second is offline-testable now that
                 `wrapper-flags` exists:
                   - AC_SEARCH_LIBS probes -lreadline alone and
                     libreadline.a's ncurses refs go unresolved
                     (poc/91-qt-xcb answered the same class with
                      -Wl,--start-group)
                   - a C link that pulls a C++ archive: libicuuc.a needs
                     operator delete and the __cxxabiv1 vtables, and
                     LinkFlags takes a `cxx bool` that does not notice
    4  T-054/T-055  kdenlive. ⛔ The bar is NOT met: 2.22x the size, and a
                 same-day safe vs aggressive timing comparison is owed.

## ⛔ Machine notes (carried forward, re-verify)

- 4 cores, uid 0, ~29 GiB free at session start. Kernel `6.18.44-fc-v24`.
- ⛔ **`make` depends on `tool/runtime/*.c`.** Rebuild after touching the loader.
- ⛔ **DISK IS BINDING.** Delete the previous build tree before the next.
- ⛔ **`pgb rootfs run` MOUNTS A FRESH TMPFS OVER `/tmp`.** Use `--bind`/`--copy`.
- ⛔ **`$?` after a pipeline is the PIPELINE's status.**
- ⛔ **`chmod 000` is not a control when you are root.** Move the file away.
- ⛔ **Never edit a shell script while it is running.**
- ⚠ **An experiment writes its own `RESULT.txt`.** Redirect stdout elsewhere.
  ⚠ **A POC does NOT** — redirect, or its RESULT.txt describes the previous run.
- ⛔ **read the CI run; a local gate does not speak for it.**

## ⛔ THE RULE ABOUT THE SHARED RESOURCE

⭐ Counts and exit statuses need the **bed** idle; **milliseconds need the whole
machine** idle. `RULES.md` §"the shared resource is sometimes the clock".
