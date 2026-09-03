# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: [`PROGRESS.md`](PROGRESS.md) holds those and is read
first anyway. This file exists only so a session that ends badly still hands
over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-03c, refreshed as work landed (RULES.md §RESUME:
                   written at the START and whenever what is in flight changes)
    TREE           main, everything pushed. Began at 44b6a1c8.
    BRANCH         ⛔ main. The harness names a `claude/*` branch and THE
                   OPERATOR SAID THE OPPOSITE. `git ls-remote --heads origin`
                   returns `main` and nothing else. ⛔ `git branch -r` is not
                   evidence about the remote; `ls-remote` is.
    CI             ⭐ GREEN on every completed run this session (checked with
                   the GitHub API, not assumed). Re-check per push.
    SELFTESTS      540 pass, 1 could not run (no zstd)
    ACCEPTANCE     ⭐ the ten POCs, twice clean-rebuilt and green; a THIRD run
                   with both toolchain fixes — see "In flight".

---

# ⛔ WHAT A FRESH SESSION CANNOT INFER

⚠ **The clone comes up SHALLOW and `main` can come up BEHIND.** Measured again
this session: `git fetch --unshallow` reported a forced update and the working
checkout was **228 commits behind** — `git rev-list --count HEAD..origin/main`
read 0 only because the harness branch already pointed at the old head.
⛔ **Check both**: the count from the branch you are on AND `git status` after
`git checkout main`.

    git fetch --unshallow
    git checkout main
    git rev-list --count HEAD..origin/main     ⛔ check it, do not assume
    git merge --ff-only origin/main

⚠ **The container is fresh: nothing is bootstrapped.** ~2 minutes to start.

    make                                     builds ./pgb, ~15 s
    ./pgb bootstrap --detach                 nix + env + bed, parallel
    ./pgb bootstrap --check                  is it ready
    sh scripts/common/install-codegraph.sh   v1.6.0

## ⛔ THE RECORD MOVED ON 2026-09-03c — READ THIS BEFORE LOOKING FOR AN ENTRY

⭐ **`TODO/` now carries ONLY open work.** The 34 closed entries are
[`../HISTORY/entries/<category>.md`](../HISTORY/entries/); the long-form
findings behind the 14 open ones are `<category>-open.md` beside them; the
session narrative is
[`../HISTORY/sessions/2026-09-03.md`](../HISTORY/sessions/2026-09-03.md).
⛔ **An open entry in `TODO/` is deliberately short — go to its `📚 detail`
link before re-running anything**, because most of it has been run once.

⚠ `sh TODO/check.sh` enforces the split (4b: an entry is filed on the side its
status says; 4c: no id has two entries). Closing an entry means **moving** it.

## ⛔ THE BAR MOVED TOO — the bundler is scored on the CLOCK now

> *"us having a bigger size than anylinux-appimages and onelf is acceptable as
> long as ours performs better and packaging is just one command not a
> multiline shell script"* — operator, 2026-09-03c

⛔ Size is struck. ⭐ One-command packaging is a win we already have. ⛔ Speed
is the failure: kdenlive cold start **300 ms** against **61 ms**, render
**4,947** against **2,033 ms**. ⚠ **Every byte lever is now un-scored** until
re-measured in milliseconds. `docs/design/toolchain.md` carries the amendment.

## In flight right now

    ⏳ `sh poc/run-all.sh --rebuild` with BOTH toolchain fixes — RUNNING.
       log: <scratchpad>/runall2.log
       ⛔ NOTHING ELSE MAY TOUCH pgb-env-debian13 WHILE IT RUNS, and ⛔ DO NOT
       REBUILD ./pgb — a mid-run swap makes the result describe two binaries.
       ⭐ THE EXPECTED RESULT IS "NO CHANGE", pre-registered before the run:
       the proc fix fires only when `cmd.Env` is non-nil AND argv[0] has no
       separator, and every POC reaches proc as `build -- /bin/sh -c "…"`.
       ⭐ NINE OF TEN GREEN at the last check (91-qt-xcb still running):
         10:13  20:13  30:13  40:13  50:13  60:13  70:21  80:22  90:21

    ⛔ SMALL AND OWED: `poc/90-qt/run.sh` prints "building qtbase (this is the
    long pole: hours, not minutes)". Measured three times now at 868/888/873 s
    for the WHOLE POC including the eleven-environment matrix. The line is
    stale; it must not be edited while the script is running.

## ⛔ WHAT IS LEFT — READ PROGRESS.md, IT IS THE WORK ORDER

The operator scoped the remaining session and the next one:

> *"reorder the leftover tasks and carry out 4 deep reviews, we can leave the
> build from git/url deferred for now, and fix all remaining GLIBC quirks if
> there still are some, else focus the next session entirely on optimizing the
> nix bundler as much as possible"*

    THIS SESSION   ✅ rulings recorded  ✅ TODO stripped  ⚠ four deep reviews
                   ⚠ the remaining glibc quirks (REQUIREMENTS.md: 8 of 9
                   closed, 1 open — and the question is whether the list of
                   NINE is still complete, not whether the eight are green)
    NEXT SESSION   ⛔ THE BUNDLER, ON THE CLOCK. PROGRESS.md N1–N6.
                   ⭐ N1 is not optional and N2 is the hypothesis to test
                   first: is the size column the time column, or not?

⭐ **Four references were named by the operator on 2026-09-03c, all mined that
day and ALL UNREAD.** Reading is owed under `docs/methodology/references.md` —
three passes each, the two write-up files, ⛔ **not delegated to a sub-agent**:

    references/DavHau__nix-portable          91122e3d  static nix
    references/nixie-dev__nixie              d14c6c37  static nix
    references/containerbase__nix-prebuild   9302079d  static nix
    references/xplshn__pelf                  d3cb5c7b  the bundler

⚠ **And the operator's framing for them**: *"we will have multiple backends,
nix just being one of them"*, and a static nix is something we **publish**
ourselves from a mix of existing techniques — not something to go looking for.
Measured 2026-09-03c: nixpkgs ships none.

## ⛔ Machine notes (carried forward, re-verify)

- 4 cores, uid 0. Kernel `6.18.44-fc-v24`.
- ⛔ **`make` depends on `tool/runtime/*.c`.** Rebuild after touching the loader.
- ⛔ **DISK IS BINDING, AND `poc/91-qt-xcb` IS WHERE IT BITES.** A full
  `poc/run-all.sh --rebuild` took the machine from 18 GiB free to **4.8 GiB**
  while 91 was linking Qt, and it was still falling.
  ⭐ **Safe to reclaim, in this order** (all rebuildable; every committed
  result lives under `evidence/`):

      /root/.local/state/pgb/nix-deps/<hash>   ⭐ the biggest, 4.6 GB for
          postgres's set alone. ⛔ ONE PER OPTION SET, so `ls` it and see whose
          dependencies they are before deleting.
      /root/.local/state/pgb/nix-build        a finished nix build tree
      /root/.local/state/pgb/nix-prefix       the static prefix it installed to
      /var/tmp/pgb-appimage-*                 AppDirs, ~10 min to rebuild
      /var/tmp/pgb-poc/<one POC>              ⚠ costs that POC a full rebuild

  ⚠ `ps aux | grep nix-deps` matches your own grep's command line — read the
  running build's log instead.
- ⛔ **Do not rebuild `./pgb` while the POC suite is running.**
- ⛔ **`pgb rootfs run` MOUNTS A FRESH TMPFS OVER `/tmp`.** Use `--bind`/`--copy`.
- ⛔ **`$?` after a pipeline is the PIPELINE's status.**
- ⛔ **`chmod 000` is not a control when you are root.** Move the file away.
- ⛔ **Never edit a shell script while it is running.**
- ⛔ **USE `sh scripts/common/run-experiment.sh <NN>` — NOT `sh experiments/NN-*.sh`.**
  ⚠ **This note used to say "an experiment writes its own `RESULT.txt`, redirect
  stdout elsewhere; a POC does NOT". BOTH HALVES WERE WRONG**, measured
  2026-09-03c: **19** experiments write their own, **13** do not, and every POC
  does via `poc/common.sh`. There is no way to tell which group one is in
  without reading it. ⛔ It cost a measurement the same day — `experiments/30-`
  was re-run, reported `pass=11 fail=0`, and refreshed **nothing**. The wrapper
  tees the transcript to `run.log` always and writes `RESULT.txt` only when the
  experiment did not, decided by mtime rather than by a list of names.
- ⛔ **read the CI run; a local gate does not speak for it.**
- ⚠ **`scratchpad/` is NOT a path in the repo.** It is the session's own
  directory outside the tree; a relative `scratchpad/x` silently reads nothing.

## ⛔ THE RULE ABOUT THE SHARED RESOURCE

⭐ Counts and exit statuses need the **bed** idle; **milliseconds need the whole
machine** idle. `RULES.md` §"the shared resource is sometimes the clock".
⛔ **This matters more than it used to**: the bundler's bar is now milliseconds,
so N1's re-measurement cannot share the machine with a POC suite or a nix build.
