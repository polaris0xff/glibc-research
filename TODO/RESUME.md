# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-03c, at the START of the session (RULES.md §RESUME)
    TREE           main, clean, at 44b6a1c8 when this session began
    BRANCH         ⛔ main. The harness named
                   `claude/glibc-pocs-routing-wniv9m` and THE OPERATOR SAID
                   THE OPPOSITE ("Work on main, never a claude/* branch").
                   The local copy was deleted with `git branch -d` after
                   `git checkout main`; it was fully merged, nothing lost.
                   ⭐ **The remote never had it.** `git ls-remote --heads
                   origin` returns `main` and nothing else — the
                   `origin/claude/...` ref was a STALE REMOTE-TRACKING REF the
                   harness's clone wrote locally, and `git remote prune origin`
                   removed it. ⛔ `git branch -r` is not evidence about the
                   remote; `ls-remote` is. Last session paid a paragraph
                   worrying about an undeletable remote branch that is not there.
    CI             green on 44b6a1c8 (last session's report). Re-check per push.

---

# ⛔ WHAT A FRESH SESSION CANNOT INFER

⚠ **The clone comes up SHALLOW and `main` can come up BEHIND.** Measured again
this session: `git fetch --unshallow` reported `+ e32a50b9...44b6a1c8 main ->
origin/main (forced update)`, and the working checkout was **228 commits
behind** — `git rev-list --count HEAD..origin/main` read 0 only because the
harness branch already pointed at 44b6a1c8. ⛔ **Check both**: the count from
the branch you are on AND `git status` after `git checkout main`.

    git fetch --unshallow
    git checkout main
    git rev-list --count HEAD..origin/main     ⛔ check it, do not assume
    git merge --ff-only origin/main

⚠ **The container is fresh: nothing is bootstrapped.** ~2 minutes to start.

    make                                     builds ./pgb, ~15 s
    ./pgb bootstrap --detach                 nix + env + bed, parallel
    ./pgb bootstrap --check                  is it ready
    sh scripts/common/install-codegraph.sh   v1.6.0, 98 files / 1,864 nodes

## In flight right now

    ⏳ R1, THE TEN POCs — 8 of 10 green (10,20,30,40,50,60,70,80), 90-qt
       building, 91-qt-xcb after it. PGB_ENGINE=chroot, matching the engine
       every committed RESULT.txt names.
         runner  scratchpad/run-pocs.sh, staging in scratchpad/poc-run/
         ⛔ DO NOT REBUILD ./pgb UNTIL IT FINISHES — a POC invokes ./pgb many
            times and a mid-run swap makes the result describe two binaries.
         Each POC's stdout is copied to evidence/poc/<name>/RESULT.txt as it
         completes, so a death here loses at most the running POC.

    ✅ R2 DONE and pushed (ac43d08c, CI green).
    ✅ B1 DONE and pushed (1dc6e49e, CI in progress at the time of writing).

    NEXT, in order:
      R3  T-063 arm S with --without-icu REMOVED  ⛔ needs the build rootfs,
          so it CANNOT start until R1 finishes — both use pgb-env-debian13.
            pgb nix plan postgresql --out pg.plan
            NIX_MAX_ROUNDS=24 pgb nix build --plan pg.plan   (no --without-icu)
      T-062  buildx, logx, proc selftests — pure Go, no bed, can overlap
      then T-075's two placements, T-057, T-060, T-054, T-051, T-012

## ⛔ WHAT IS LEFT — READ PROGRESS.md, IT IS THE WORK ORDER

    ---- 0. ⛔ THE DEBT THE LAST SESSION TOOK ON. FIRST. ----
    R1  ⛔ RUN THE TEN POCs. The wrapper's link hot path changed (every link
        now scans its .a/.o inputs via elfx.NeedsCXXRuntime) and nothing ran
        the acceptance harness against it.
    R2  ⚠ `bundle-soname-scan`'s oracle now SHARES selfKeys() with the subject,
        so their equivalence cannot catch a defect inside it.
    R3  ⚠ T-063 arm S with `--without-icu` REMOVED -- the C++ fix is proved on
        a synthetic subject, not on postgres.

    ---- 1. T-066 P0, and the ROUTE ORDER CHANGED ----
    ⭐ Route A's ceiling is MEASURED at 23.3% and the gap is 2.22x, so the
    allowlist cannot finish the job. ⛔ COST ROUTE B FIRST: how many store
    paths in kdenlive's closure are downstream of qtbase and mesa? That needs
    only the closure, no rebuild, no AppDir.

    ---- 2. then T-062 (buildx/logx/proc), T-075's two placements, T-057,
            T-060, T-054, T-051, T-012 ----

## ⛔ Machine notes (carried forward, re-verify)

- 4 cores, uid 0, ~29 GiB free at session start. Kernel `6.18.44-fc-v24`.
- ⛔ **`make` depends on `tool/runtime/*.c`.** Rebuild after touching the loader.
- ⛔ **DISK IS BINDING.** Delete the previous build tree before the next.
- ⛔ **Do not rebuild `./pgb` while the POC suite is running** — a POC invokes
  `./pgb` many times and a mid-run swap makes the acceptance result describe
  two different binaries.
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
