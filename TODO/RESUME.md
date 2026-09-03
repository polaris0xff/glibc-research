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

    ⏳ R3 — T-063 arm S with `--without-icu` REMOVED. RUNNING on the bed.
         cmd   cd /var/tmp/pgb-r3 && NIX_MAX_ROUNDS=24 \
               /home/user/glibc-research/pgb nix build --plan /var/tmp/pgb-r3/pg.plan
         log   scratchpad/r3-postgres.log   (plan already produced: pg.plan)
         ⛔ NOTHING ELSE MAY TOUCH pgb-env-debian13 WHILE IT RUNS.
         ⭐ WHAT TO READ OFF IT: the plan carries `--with-icu`. The question is
            whether the adaptation loop KEEPS it. If postgres builds with
            `--with-icu` still on the final configure line, the C++-archive fix
            (elfx.NeedsCXXRuntime) is proved on the real subject. If the loop
            drops it, the fix did not reach this case and that is the finding.

    ⛔ OWED AS SOON AS THE BED IS FREE, and it is small:
       re-run poc/70-sqlite-extensions, poc/80-mlt and poc/91-qt-xcb. Their
       committed RESULT.txt files describe runs WITHOUT the new
       `poc_check_built_by_env` assertion. Build trees for 80 and 91 are still
       on disk, so the re-runs are minutes, not the full build.
       ⚠ 90-qt's build tree was DELETED to make room for R3 (3.6 GB). Its
       evidence is committed; a re-run costs ~14 minutes.

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
