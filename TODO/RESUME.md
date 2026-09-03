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
    SELFTESTS      546 pass, 1 could not run (no zstd)
    ACCEPTANCE     ⭐ the ten POCs, FOUR clean-rebuilt green runs this session,
                   the last against every toolchain change in it.

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
is the failure — and ⚠ **deep review 1 found the timing half of the record does
not re-derive**: `90-` takes ONE SAMPLE per arm, its published numbers come
from a superseded version of the evidence file it cites, four runs give
cold-start ratios of 2.52×, 3.48×, 4.92× and 5.02×, and warm exceeds cold in
two of them. ⭐ **Every run agrees on the DIRECTION — we are slower.** The one
figure that re-derives is `jq`: **139 vs 67 ms cold (2.07×)**, eleven
environments, mean of five. `corrections.md` C23; fix the instrument (N0)
before measuring anything with it.

## In flight right now

    ⭐ NOTHING. Everything is committed, pushed, and gated.

    ✅ THE FOURTH ACCEPTANCE RUN IS DONE AND GREEN — `poc/run-all.sh --rebuild`
       against deep review 4's separated-`-l` fix and the --embed-tzdata
       objects. ran=10 failed=0, 170 assertions, fail=0 skip=0:
         10:13 20:13 30:13 40:13 50:13 60:13 70:21 80:22 90:21 91:28
       ⭐ The expectation was "no change" and it was pre-registered BEFORE the
       run, not after. It held.
    ⚠ FOR THE NEXT RUN: ⛔ NOTHING ELSE MAY TOUCH pgb-env-debian13 while the
       suite runs, and ⛔ DO NOT REBUILD ./pgb mid-run — the result would
       describe two binaries.

## ⛔ WHAT IS LEFT — READ PROGRESS.md, IT IS THE WORK ORDER

The operator scoped the remaining session and the next one:

> *"reorder the leftover tasks and carry out 4 deep reviews, we can leave the
> build from git/url deferred for now, and fix all remaining GLIBC quirks if
> there still are some, else focus the next session entirely on optimizing the
> nix bundler as much as possible"*

    THIS SESSION   ✅ rulings recorded  ✅ TODO stripped  ✅ SIX deep reviews
                   ✅ THE GLIBC QUIRKS: the list of nine was NOT complete. A
                   TENTH — the timezone database — was found and CLOSED the
                   same day (`--embed-tzdata`, 11 of 11, 193 KB for 20 zones).
                   REQUIREMENTS.md now reads NINE OF TEN closed, ONE open.
    NEXT SESSION   ⛔ THE BUNDLER, ON THE CLOCK. PROGRESS.md N1–N6.
                   ⭐ N1 is not optional and N2 is the hypothesis to test
                   first: is the size column the time column, or not?

⭐ **The four references the operator named on 2026-09-03c were mined AND READ
that day.** [`../docs/research/portable-nix.md`](../docs/research/portable-nix.md)
carries the findings; `portable-nix-mechanisms.md` beside it carries the usable
half at file and line. ⛔ **Nothing in that sweep was RUN**, and it names three
probes that would settle its riskiest claims — start there, not with a re-read.

⚠ **Two of its findings correct things this project had written down:**
`nix-cli-static` is an attribute of the **nix flake**, not of nixpkgs, so
"there is no static nix to fetch" was answering the wrong question; and
`nix --store` under `$HOME` — T-051's step 2 — is `nix-portable`'s
**first-choice** runtime and **fails on Arch, Debian 11 and Debian 12**, with a
probe that passes first.

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
