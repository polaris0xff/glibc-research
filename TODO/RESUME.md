# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: [`PROGRESS.md`](PROGRESS.md) holds those and is read
first anyway. This file exists only so a session that ends badly still hands
over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-03d, at the START (RULES.md §RESUME), refreshed as
                   work lands
    TREE           main, began at 432e6413 (== origin/main at session start)
    BRANCH         ⛔ main. The harness names a `claude/*` branch and THE
                   OPERATOR SAID THE OPPOSITE. `git ls-remote --heads origin`
                   returns `main` and nothing else. ⛔ `git branch -r` is not
                   evidence about the remote; `ls-remote` is.
    CI             ⚠ green at 432e6413 per the last session. Re-check per push.
    SELFTESTS      546 pass, 1 could not run (no zstd) — carried, re-verify
    ACCEPTANCE     the ten POCs, four clean-rebuilt green runs last session

---

# ⛔ WHAT A FRESH SESSION CANNOT INFER

⚠ **The clone comes up SHALLOW and `main` can come up BEHIND.** Measured again
this session: `git fetch --unshallow` reported a forced update, and
`git rev-list --count HEAD..origin/main` read **0** — but only because the
harness branch pointed at the same head. ⛔ **`git checkout main` then said
"behind by 267 commits"**, which is the number that mattered.
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

⭐ **`TODO/` carries ONLY open work.** The closed entries are
[`../HISTORY/entries/<category>.md`](../HISTORY/entries/); the long-form
findings behind the open ones are `<category>-open.md` beside them; the
session narratives are [`../HISTORY/sessions/`](../HISTORY/sessions/).
⛔ **An open entry in `TODO/` is deliberately short — go to its `📚 detail`
link before re-running anything**, because most of it has been run once.

⚠ `sh TODO/check.sh` enforces the split (4b: an entry is filed on the side its
status says; 4c: no id has two entries). Closing an entry means **moving** it.

## ⛔ THE BAR IS THE CLOCK — and the instrument was the first thing to fix

> *"us having a bigger size than anylinux-appimages and onelf is acceptable as
> long as ours performs better and packaging is just one command not a
> multiline shell script"* — operator, 2026-09-03c

⛔ Size is struck. ⭐ One-command packaging is a win we already have. ⛔ Speed
is the failure. `corrections.md` C23 is why the old numbers cannot be quoted.

## In flight right now

    ⚠ SESSION 2026-09-03d. Scoped by the operator to PROGRESS.md N0–N6.
      ⭐ N0, N2 and N6 are DONE, and TWO LEVERS ARE SHIPPED.

    ⭐ THE HEADLINE: `jq` cold start went 2.07× → **1.00×** against the
       field, eleven environments, and the closure did not change. Two
       constants in internal/bundle/appimage.go did:
         uruntime v0.5.6 full → v0.5.9 LITE   (77-)  2.07× → 1.28×
         dwarfs block -S26 → -S18             (81-)  1.28× → 1.00×
       ⚠ Size went the other way to buy it: 1.44× → 1.70×.

    RUNNING   experiments/90- (kdenlive) with the CORRECTED cold protocol.
              It rebuilds the bundle (~7 GB cache, tens of minutes).
              ⛔ DO NOT EDIT ANY RUNNING SCRIPT. This session did, on 90-,
              and had to kill and restart the run: `sh` re-reads from a
              byte offset.

    ⛔ WHAT IS NOT DONE, IN ORDER:
       1. 86- has ONE run at the shipped configuration. The parity
          headline is N=1 at the config level, which is C23's own lesson.
          RE-RUN IT.
       2. 78-, 85-, 89- build bundles and their evidence describes the OLD
          runtime AND the old block size. NO GATE CAN SEE IT —
          check-docs.sh compares a script against its evidence and none of
          those scripts changed. C5's shape reached through the Go source.
          ⭐ `pgb bundle appimage` now PRINTS the two runtime filenames, so
          a run.log says which it describes. 78- and 89- are cheap.
       3. 86-'s WARM column subtracts a cold run that is not in the series
          it divides. Its cold column is sound. Carry clock.sh into it.

## ⛔ WHAT IS LEFT — READ PROGRESS.md, IT IS THE WORK ORDER

    N0  ✅ DONE. experiments/clock.sh + 99-. The cold column was measuring a
        WARM start: uruntime keys its mount on CONTENT, so 90-'s "fresh copy
        is cold by construction" reuses the live mount. corrections.md C24.
    N1  ⛔ PREMISE GONE. It existed because the byte levers might score under
        the new bar. N2 says they cannot.
    N2  ✅ ANSWERED, NO. experiments/84-: 0.024–0.031 ms per MiB, so the whole
        196 MiB between the two kdenlive bundles is ~5 ms of a gap never seen
        below 129 ms. A 138× file count does not resolve at all.
    N3  route B, costed and not built — untouched, and now lower value: it is
        a SIZE lever.
    N4  --fixpoint — same, a size lever.
    N5  ⛔ route A at path granularity is measured DEAD. Do not build it.
    N6  ✅ THE LEVER WAS THE RUNTIME AND IT IS SHIPPED. experiments/77-:
        uruntime v0.5.6 FULL → v0.5.9 LITE is 0.76× cold, 11 of 11, and the
        artefact loses 1.55 MB. The version bump alone buys nothing; it is
        `lite`. ⚠ pelf's extract-above-350 MB is NOT a lever we lack —
        uruntime exposes URUNTIME_EXTRACT and REUSE_CHECK_DELAY and we do
        not set them. Unmeasured with clock.sh; that is the next probe.

## ⛔ Machine notes (carried forward, re-verify)

- 4 cores, uid 0. Kernel `6.18.44-fc-v24`. **27–29 GiB free at session start.**
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
  ⚠ **19** experiments write their own `RESULT.txt`, **13** do not, and every
  POC does via `poc/common.sh`. There is no way to tell which group one is in
  without reading it. The wrapper tees the transcript to `run.log` always and
  writes `RESULT.txt` only when the experiment did not, decided by mtime.
- ⛔ **read the CI run; a local gate does not speak for it.**
- ⚠ **`scratchpad/` is NOT a path in the repo.** It is the session's own
  directory outside the tree; a relative `scratchpad/x` silently reads nothing.

## ⛔ THE RULE ABOUT THE SHARED RESOURCE

⭐ Counts and exit statuses need the **bed** idle; **milliseconds need the whole
machine** idle. `RULES.md` §"the shared resource is sometimes the clock".
⛔ **This matters more than it used to**: the bundler's bar is now milliseconds,
so N1's re-measurement cannot share the machine with a POC suite or a nix build.
