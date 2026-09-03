# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: [`PROGRESS.md`](PROGRESS.md) holds those and is read
first anyway. This file exists only so a session that ends badly still hands
over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-03e, at the START (RULES.md §RESUME). Refreshed as
                   work lands. Session IN PROGRESS.
    TREE           main, began at 8cfbeacb (== origin/main at session start)
    BRANCH         ⛔ main. The harness named `claude/t-078-079-080-tasks-jirtgy`
                   and THE OPERATOR SAID main, again. Same as last session.
    SCOPE          ⛔ THREE ENTRIES, operator 2026-09-03d:
                     T-078 the three-way parity matrix   (runtime P1 L)
                     T-079 enumerate the remainder, BY SEARCH (runtime P1 M)
                     T-080 the capability guarantee      (research P1 L)
                   ⛔ NOT T-081 / T-082 / T-083. Not started, not looked at.
                   ⛔ T-066 is the only open P0 and is NOT the work: its
                   remaining column is size, struck 2026-09-03c and deferred
                   2026-09-03d. PROGRESS.md's work order decides, not the
                   priority ordering.

## ⭐ RESOLVED AT SESSION START — the blocker the last session recorded

⭐ **`musl-gcc` IS NOW INSTALLED.** PROGRESS.md "Open questions" #2 said it was
absent and that it would bite T-078's musl column. It is Ubuntu 24.04 noble:

    apt-get install -y musl-tools musl-dev     # musl 1.2.4-2

⭐ **Verified, not assumed** — a `musl-gcc -static` hello builds, runs, and
`readelf -d` reports *"There is no dynamic section in this file"* with zero
`GLIBC_2` strings. ⛔ The toolchain that `experiments/60-`/`61-` skip their
musl arms without is present, so a skip in those runs now means something
other than a missing toolchain and must be read, not assumed.

⚠ **musl-gcc here is a SPEC WRAPPER around gcc 13.3.0**, not a separate
compiler — `musl-gcc --version` prints `x86_64-linux-gnu-gcc 13.3.0`. That is
the ordinary Debian/Ubuntu shape and it matters for one reason only: the musl
column's compiler is NOT the pinned 14.2.0 the `pgb` column uses, so a
throughput row confounds libc with compiler version unless the vanilla arm is
built with the same 13.3.0. ⛔ Say which, in the table.

---

# ⛔ WHAT A FRESH SESSION CANNOT INFER

⚠ **The clone comes up SHALLOW and `main` comes up BEHIND.** Measured a third
time this session, and the number was **311**:

    git fetch --unshallow          # reported "+ e32a50b9...8cfbeacb (forced update)"
    git checkout main              # "behind 'origin/main' by 311 commits"
    git rev-list --count HEAD..origin/main
    git merge --ff-only origin/main

⛔ **On the harness branch the count read 0.** It reads 0 because the harness
branch points at the same head, not because the tree is current. Check it
**after** `git checkout main`, never before.

⚠ **The container is fresh: nothing is bootstrapped.**

    make                                     builds ./pgb, ~15 s
    ./pgb bootstrap --detach                 nix + env + bed, parallel
    ./pgb bootstrap --check                  is it ready
    sh scripts/common/install-codegraph.sh   v1.6.0, 102 files / 1,918 nodes

## ⛔ THE RECORD MOVED ON 2026-09-03c — READ THIS BEFORE LOOKING FOR AN ENTRY

⭐ **`TODO/` carries ONLY open work.** The closed entries are
[`../HISTORY/entries/<category>.md`](../HISTORY/entries/); the long-form
findings behind the open ones are `<category>-open.md` beside them; the
session narratives are [`../HISTORY/sessions/`](../HISTORY/sessions/).
⛔ **An open entry in `TODO/` is deliberately short — go to its `📚 detail`
link before re-running anything**, because most of it has been run once.

⚠ `sh TODO/check.sh` enforces the split (4b: an entry is filed on the side its
status says; 4c: no id has two entries). Closing an entry means **moving** it.

## In flight right now

    ⭐ Session start. Bootstrap detached and running; nothing else in flight.

    ⛔ THE THREE TRAPS, one per entry, taken from the Prove lines:
      T-078  a SKIP is not a dash and not a PASS. 60-/61- skip arms they
             cannot build, so a green run can carry an EMPTY musl column.
             Read the skip count. PRE-REGISTER which cells differ BEFORE
             running. A row that comes out against us IS the deliverable.
      T-079  done means a reader RE-RUNS the search and gets the list.
             An absence is not a zero — say WHERE YOU LOOKED. "Still ten"
             is a result if shown and not one if asserted.
      T-080  overclaiming: "Vulkan works" is NOT supported by swrast +
             surfaceless. The supported sentence is "the closure produces a
             working EGL display offscreen". Underclaiming by borrowing:
             the field's grades were earned on ARCH PACKAGES through
             quick-sharun, a different pipeline. A row not run through
             `pgb bundle appimage` is a HYPOTHESIS and is labelled one.

## ⛔ Machine notes (carried forward, re-verify)

- 4 cores, uid 0. Kernel `6.18.44-fc-v24`. **29 GiB free at session start**
  (bootstrap preflight said so).
- ⭐ **musl-gcc present**, see above. Ubuntu 24.04 noble, `musl-tools` 1.2.4-2.
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
- ⛔ **read the CI run; a local gate does not speak for it.** Two local-gate
  holes produced SIX red runs last session; both are closed, and the lesson
  (the gate does not speak for CI) is not.
- ⚠ **`scratchpad/` is NOT a path in the repo.** It is the session's own
  directory outside the tree; a relative `scratchpad/x` silently reads nothing.

## ⛔ THE RULE ABOUT THE SHARED RESOURCE

⭐ Counts and exit statuses need the **bed** idle; **milliseconds need the whole
machine** idle. `RULES.md` §"the shared resource is sometimes the clock".
⛔ T-078 has throughput, startup and RSS rows, so those rows cannot share the
machine with a POC suite, a nix build or a bundle build.
