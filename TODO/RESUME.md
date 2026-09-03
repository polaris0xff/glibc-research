# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-03c, refreshed as work landed (RULES.md §RESUME:
                   written at the START and whenever what is in flight changes)
    TREE           main, clean, everything pushed. Began at 44b6a1c8.
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
    CI             ⭐ GREEN on every completed run this session (checked with
                   the GitHub API, not assumed). Re-check per push.
    SELFTESTS      371 → ⭐ 540 pass, 1 could not run (no zstd)
    ACCEPTANCE     ⭐ the ten POCs, TWICE, both clean rebuilds:
                     R1 (pre-fix)             10/10, 167 assertions
                     post `-l` fix            10/10, 172 assertions
                   and a THIRD run with both toolchain fixes was in flight at
                   the last refresh — see "In flight".

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

## ⛔ THE ONE THING A FRESH SESSION MUST DO FIRST

    ⚠ ONLY IF the run described under "In flight" did not finish. Check
    `evidence/poc/*/RESULT.txt` dates and `git log` before spending an hour.

        make && sh poc/run-all.sh --rebuild

    ⛔ `--rebuild` IS NOT OPTIONAL after a wrapper change: every POC skips its
    build when the artefact exists, and only five of the ten honour
    POC_REBUILD, so a plain re-run reuses binaries the OLD toolchain produced
    and reports ten green rows that say nothing. `poc/run-all.sh` is new this
    session and exists because that trap had caught every previous "re-run the
    POCs" instruction.

    TWO hot-path fixes landed this session and the suite must cover both:
      1. `cxxRuntimeDemand` now resolves `-lNAME` against `-L` (66528e59)
      2. `proc.Cmd` now resolves argv[0] against the CHILD's PATH (6a04721d)
    ⭐ (2) is ALSO proved directly, which is its own test:
        pgb --engine chroot build -- cc -o t t.c
          rc=0  runs  pgb-runtime: YES  PT_INTERP 0  DT_NEEDED 0
      — where before the fix that exact line failed outright.

    ⛔ SMALL AND OWED: `poc/90-qt/run.sh` prints "building qtbase (this is the
    long pole: hours, not minutes)". Measured twice at 868 s and 888 s for the
    WHOLE POC including the eleven-environment matrix. The line is stale; it
    was not edited because the script was running.

## In flight right now

    ⏳ `sh poc/run-all.sh --rebuild` with BOTH toolchain fixes — RUNNING.
       log: scratchpad/runall2.log   (the previous run: runall.log)
       ⛔ NOTHING ELSE MAY TOUCH pgb-env-debian13 WHILE IT RUNS, and ⛔ DO NOT
       REBUILD ./pgb — a mid-run swap makes the result describe two binaries.
       ⭐ THE EXPECTED RESULT IS "NO CHANGE", pre-registered in PROGRESS.md
       before the run: the proc fix fires only when `cmd.Env` is non-nil AND
       argv[0] has no separator, and every POC reaches proc as
       `build -- /bin/sh -c "…"`. A green run confirms that reading; a red one
       means the reading is wrong, which is worth more than the run cost.

    ✅ THE PREVIOUS RUN (fix 1 only) WAS ALL TEN GREEN, clean rebuild:
         10:13  20:13  30:13  40:13  50:13  60:13
         70:21  80:22  90:21  91:28      ran=10 failed=0
       ⭐ 70, 80 and 91 gained a case each — `poc_check_built_by_env`, live for
       the first time.
       ⚠ Timings 3,327 s → 3,471 s are CONTAMINATED and are not a cost for the
       wider scan: nix plans, closure walks, Go builds and both gates ran on
       the same four cores throughout. The control is 60-leveldb 15 s → 5 s,
       which no scan change can produce.

    ✅ R3 IS ANSWERED, BOTH HALVES.
       ⛔ First run: the C++-archive fix did NOT reach postgres. `-with-icu`
       survived all 14 rounds, then the link died on `operator delete` out of
       libicuuc.a. Cause: `cxxRuntimeDemand` skipped every argument beginning
       with `-`, and postgres names ICU as `-L… -licuuc`. (66528e59)
       ⭐ Re-run with the fix: ICU errors 0, `src/backend/postgres` built,
       101,647,216 B, PT_INTERP 0, DT_NEEDED 0, 3,911 icu_78 symbols,
       PostgreSQL 18.6 answering ON ALPINE. It now stops in
       `src/interfaces/libpq` on postgres's OWN policy check against the
       SHARED libpq — a different rung, named in T-063. (b89138c3)

## ✅ DONE AND PUSHED THIS SESSION (all CI-green where CI has reported)

    R1   ⭐ TEN OF TEN POCs, 167 assertions, fail=0 skip=0, 55 min. The debt
         is cleared: the link hot path change is validated on ten real
         projects across eleven environments.  516a6cf7
    R2   the soname scan's control shared code with its subject; separating
         them found a HARDLINKED root-of-itself. selfKeys keys on dev:ino now.
         ⚠ Corrected the same day: NOT reachable through pgb's own output
         (0 of 284 files hardlinked in a jq AppDir).  ac43d08c, fba4a3c8
    B1   ⭐ ROUTE B COSTED, and it OVERTURNS the argument against it: the whole
         -mini set forces 161 of 676 kdenlive closure paths (23.8%) from
         source, qtbase alone 78 (11.5%) -- not "the entire KDE/Qt subtree".
         experiments/95-.  1dc6e49e
    B3   ⭐ the fixpoint lever, as a MEASURING DEVICE (`pgb bundle sweep
         --fixpoint`), +978,576 B on jq. ⭐ The two levers are ADDITIVE.
         ⭐ And it inherits the baseline's risk rather than adding one --
         measured, 0 reachable objects mention the seven it drops.
         71b5e3b5, 489cb3f8, 49f9df01
    T-062 ✅ CLOSED. buildx, logx, proc covered; every package now has a
         suite. 375 -> 516 cases, each proved able to fail.  6f445c0a
    T-075 ✅ CLOSED. experiments/96-: LD_DEBUG prints NOTHING when ld.so
         arrives as a library, even at `all`, and even though ld.so IS in the
         process. Both remaining placements refused, with the reason.  5e447ebf
    T-057 ⚠ "no 32-bit path" was STALE -- lib32 is implemented; the
         measurement is what is missing. elfClass now covered.  761dbbfe
    POC harness: three of the ten never asserted which compiler built their
         binary. Split out as poc_check_built_by_env.  724c738d

## ⛔ WHAT IS LEFT — READ PROGRESS.md, IT IS THE WORK ORDER

    The debt (R1, R2, R3) is done or running. T-066 is still the last open P0.

    B1b ⛔ COST ROUTE B IN WALL CLOCK. B1 says 161 of 676 paths, which is
        affordable in COUNT; one of them is qtbase and Qt does not build in a
        minute. That number decides whether the bundle stays one command.
    B2  ⭐ THEN the allowlist, now bounded AND now worth more than it looked:
        route B is cheaper than the entry assumed, so the two compose.
    B3b ⛔ the fixpoint's control. ⚠ NOT one command -- PROGRESS.md carries
        the three things a planner needs first (it is not wireable into 89-
        as it stands, 89- builds three mesa-demos bundles, and its assertion
        is an EGL one that never touches what the fixpoint drops).
    then T-057 (a 32-bit application through lib32 -- the code EXISTS, the
        measurement does not), T-060, T-054, T-051, T-012.

    ⭐ FOUR OF THOSE HAD THEIR PREMISE MEASURED THIS SESSION, so none of them
    starts cold. PROGRESS.md carries each in full:
      T-057  "no 32-bit path" is STALE -- lib32 is implemented (EI_CLASS
             routing, a 32-bit loader, the shared/lib32 symlink). Only the
             measurement is missing. elfClass now has seven hermetic cases.
      T-060  the components are NOT index attributes and `nix` is an
             aggregator -- but the transitive .drv walk ALREADY EXISTS
             (`pgb nix cache closure`, 2,000 paths / 1,665 derivations, every
             named risk dependency in it). Blocked on BUILDING them.
      T-054  rung 4's direct demand is 13 buildInputs and rung 3 is TWO of
             them, not a framework set. qtbase 6.11.1 already proved at
             exactly the version kdenlive wants.
      T-012  all three pieces the entry says to "split before starting"
             already exist and R3 ran them end to end. What is left: the URL
             route (no code), the report, joining static-first to bundle-last,
             and the dotted-attribute gap that needs nix.
    ⛔ T-051 step 2 (a static nix against a store under $HOME) is the one that
    still needs the bed: proving "no /nix" requires a rootfs, and this build
    host has nix installed, so testing here would prove nothing.

## ⛔ Machine notes (carried forward, re-verify)

- 4 cores, uid 0, ~29 GiB free at session start. Kernel `6.18.44-fc-v24`.
- ⛔ **`make` depends on `tool/runtime/*.c`.** Rebuild after touching the loader.
- ⛔ **DISK IS BINDING, AND `poc/91-qt-xcb` IS WHERE IT BITES.** Measured
  2026-09-03c: a full `poc/run-all.sh --rebuild` took the machine from 18 GiB
  free to **4.8 GiB** while 91 was linking Qt, and it was still falling.
  ⭐ **What is safe to reclaim, in this order** (all rebuildable, none is
  evidence — every committed result lives under `evidence/`):

      /root/.local/state/pgb/nix-deps/<hash>   ⭐ the biggest, 4.6 GB for
          postgres's set alone. ⛔ ONE PER OPTION SET, so check which one the
          running build is using before deleting: `ls` it. postgres's holds
          krb5/icu4c/curl/audit; poc/91's holds libx11/libxcb/xorgproto.
      /root/.local/state/pgb/nix-build        a finished nix build tree
      /root/.local/state/pgb/nix-prefix       the static prefix it installed to
      /var/tmp/pgb-appimage-*                 AppDirs, ~10 min to rebuild
      /var/tmp/pgb-poc/<one POC>              ⚠ costs that POC a full rebuild

  ⚠ `ps aux | grep nix-deps` matches your own grep's command line — read the
  running build's log for the path instead, or `ls` the directory and see whose
  dependencies they are.
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
