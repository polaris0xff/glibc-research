# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: [`PROGRESS.md`](PROGRESS.md) holds those and is read
first anyway. This file exists only so a session that ends badly still hands
over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-03f, at the START; refreshed when arm G came back
                   11 of 11, when the instrument deadlocked, and again on
                   2026-09-04 when T-081 CLOSED and 66-/67- landed.
                   Session IN PROGRESS.
    TREE           main, began at 820d899f (== origin/main at session start)
    BRANCH         ⛔ main. The harness named
                   `claude/t-081-bundle-capabilities-2c24c0` and THE OPERATOR
                   SAID main, again. Fourth session running.
    SCOPE          ⭐ T-081 ✅ CLOSED (arm G 0/11 -> 11/11, twice).
                   ⭐ T-085 ✅ /etc/services, T-086 ✅ the codeset axis.
                   ⏳ T-080 REOPENED and IN FLIGHT: every capability in
                   docs/research/bundle-capabilities.md re-measured with
                   THREE applications per category, simple -> complex.
    CI             ⭐ 323 success on 357c0346, the T-081 closure. 316-323 all
                   success. ⚠ 324-326 (c4fa93cf, ef6a55f9, 68c923e6) NOT YET
                   READ — READ THEM.
    GATES          both green at every commit so far.

## ⛔ WHAT A FRESH SESSION CANNOT INFER

⚠ **The clone comes up SHALLOW and `main` comes up BEHIND.** Measured a fourth
time; the number was **321** (267, then 311, then 321):

    git fetch --unshallow
    git checkout main
    git rev-list --count HEAD..origin/main     ⛔ check it AFTER the checkout
    git merge --ff-only origin/main

⛔ **On the harness branch the count reads 0.** It reads 0 because that branch
points at the same head, not because the tree is current.

⚠ **The container is fresh: nothing is bootstrapped.**

    make                                     builds ./pgb, ~15 s
    ./pgb bootstrap --detach                 nix + env + bed, parallel
    ./pgb bootstrap --check                  is it ready
    sh scripts/common/install-codegraph.sh   v1.6.0

⭐ **AND FOR ANY GUI WORK, TWO MORE PACKAGES**, without which the next session
will repeat the 2026-09-03e mistake:

    apt-get install -y musl-tools musl-dev   # T-078's musl column
    apt-get install -y xvfb x11-utils        # ⛔ T-080/T-081's ONLY honest
                                             # GUI criterion

⛔ **AND START THE WATCHDOG BEFORE ANY LONG BUNDLE RUN.** Both ways these runs
die are silent — a fixed writable allowance that makes `df` read `Avail 0` at a
low `Used`, and a `dwarfs` daemon that outlives its AppImage and holds its
extraction directory:

    nohup sh scripts/common/watchdog.sh --watch --interval 120 --floor 6 \
          --reap --log /var/tmp/watchdog.log >/dev/null 2>&1 &

It also reports any process in **state D**, which is the `strace`-on-FUSE
deadlock and cannot be killed. `docs/AGENTS.md` §6 has the rest, including why
`LD_DEBUG=libs` is the first instrument for a bundle and `strace` the second.

## ⛔ THE INSTRUMENT LESSON THAT COST ELEVEN GREEN ROWS

⛔ **`Gtk-WARNING: cannot open display` IS NOT A RESULT.** `experiments/64-`
scored a bundle 11 of 11 green on it, reasoning that the message comes from the
*bundled* libgtk-3 and so proves it loaded. The operator rejected it:

> *"previously nixappimage bundled apps showed the same error on real hw with
> display, confirm it properly by feeding it a fake/emulated display"*

⭐ **The message does not discriminate.** With a real `Xvfb` display and
`xwininfo` asking the **X server** for a window from outside the process, those
11 rows became **0**. ⛔ Before trusting any GUI row, ask what would make the
criterion fail *for the right reason*.

## In flight right now

    ⭐ DONE AND COMMITTED, both runs agreeing on every cell:

        64-  T-081's acceptance test.  pass=11 fail=0 skip=0, twice.
             G galculator  compiled-in store path   WINDOW 11/11, host .so 0
             N the SAME bundle, --no-storefix       WINDOW  0/11
             X mousepad    GResource control        WINDOW 11/11, host .so 0
             P meld        Python 3 + GTK 3         WINDOW 11/11, host .so 0
        66-  --embed-netdb.    pass=12 fail=0 skip=0, twice.  8/11 -> 11/11.
        67-  --utf8-default.   pass=7  fail=0 skip=0, twice.  0/11 -> 11/11,
             and LANG=C still obeyed 11/11 (the row that makes it mean
             something).

    ⛔ THE CORPUS'S FIRST RUN WAS STOPPED AFTER ONE ROW AND THE ROW WAS
    RETRACTED — docs/history/corrections.md C26. It scored `galculator`
    0 of 11, a subject `experiments/64-` had measured at 11 of 11 TWICE.
    ⭐ The cause was a 25-second window budget copied out of `64-`, which
    uses 25s only for its MOUNT-mode arms and 150s for the one EXTRACT-mode
    arm — and `65-` runs EVERY subject in extract mode. Measured: a bundle
    puts its first window on the X server at t+21s, unpack included.
    ⭐ THE REAL DEFECT WAS THE MISSING POSITIVE CONTROL, now C6: `gtk3-1`,
    `gtk3-2` and `py-1` are `64-`'s arms G, X and P and must come back
    11 of 11 or the instrument, not the capability, is the finding.

    ⏳ RUNNING NOW: `experiments/65-`, relaunched **02:28 UTC** by
       `scratchpad/chain3.sh` (session scheduling, not evidence) with the
       corrected instrument AND the corrected `storeRefRe` (C27) — it was
       started at 02:16 and stopped again nine minutes later, because
       C27 changed the bundler and rows built by two different tools may
       not sit in one table. 26 subjects × 11 environments. ⚠ HOURS.
       ⭐ RESUMABLE: a completed subject writes a TAB-separated row into
       `evidence/65-capability-corpus/rows/` and a recorded row is never
       re-measured, so `sh scripts/common/run-experiment.sh 65` picks up
       wherever it stopped. ⛔ A row measured by a broken instrument must be
       DELETED, not adjusted — that is what makes resumability safe.

    ⛔ WHILE 65- RUNS, THE MACHINE IS NOT FREE.
      - ⛔ Do not run another GUI experiment: 65- counts windows on `:99`,
        and a second program's window there is a false positive nothing
        else in the harness catches.
      - ⛔ Do not `make`: 65- compares each artefact against `./pgb`.
      - ⚠ Do not run bed-heavy experiments; the counts they take need the
        bed idle and 65- is using it continuously.

    ⛔ THE DEADLOCK THAT COST RUN 1 OF 64-, kept because it will recur:

        strace   D    folio_wait_bit_common
        dwarfs   Ssl  futex_do_wait        (the FUSE daemon for the mount)
        python3  t    ptrace_stop

    strace was blocked reading a page the FUSE daemon could only serve by
    making progress the ptrace-stopped process could not make. ⛔ `kill`
    cannot end a process in D, so `wait` never returned. ⭐ THE FIX IS AN
    ORDERING — `reap_in_root` kills the FUSE daemon and must run BEFORE
    `wait`, not after — plus `APPIMAGE_EXTRACT_AND_RUN=1`, which removes the
    FUSE mount from the picture entirely and is why every 65- subject sets
    it. The ordering fix also made every row ~10× faster, which means the
    slow version had been paying the same cost in a milder form all along.

## ⭐ WHAT LANDED THIS SESSION, IN ORDER (all pushed, gates green each time)

    024c550f  T-081 CLOSED: arm G 0/11 -> 11/11 without the bind
    357c0346  both runs of 64- agree, 11/11 on every arm
    c4fa93cf  ⭐ T-085 /etc/services 11/11, T-086 the codeset axis 11/11
    ef6a55f9  T-084 corrected: SIX hand copies, not nine; 77- has none
    68c923e6  ⛔ C26: the corpus had no positive control
    74d3f5e0  ⛔ C27: our own storeRefRe had the boundary defect we
              accused the field of. 585 selftest cases; three of four new
              ones fail under the planted old regex.
    (this)    the C27 re-measurement written into the record

⛔ **TWO CORRECTIONS IN ONE SESSION, BOTH ABOUT INSTRUMENTS, BOTH FOUND BY A
DISAGREEMENT RATHER THAN BY READING.** C26 came from a corpus row that
contradicted `experiments/64-`; C27 came from reading a build log. ⭐ Neither
was found by re-reading the code that contained it.

## ⛔ Machine notes (carried forward, re-verify)

- 4 cores, uid 0. Kernel `6.18.44-fc-v24`. ~23 GiB free after bootstrap.
- ⭐ **musl-gcc, Xvfb and x11-utils were installed this session** — a fresh
  container has none of them.
- ⛔ **`pgb rootfs run` MOUNTS A FRESH TMPFS OVER `/tmp`**, so an X socket must
  be bound in explicitly: `--bind /tmp/.X11-unix:/tmp/.X11-unix`.
- ⛔ **A GUI program that WORKS does not exit** — it enters its event loop. Run
  it in the background and look at the X server while it is alive; waiting for
  it to finish finds no windows either way and scores a working bundle exactly
  like a broken one.
- ⛔ **`make` depends on `tool/runtime/*.c`.** Rebuild after touching the loader.
- ⛔ **DISK IS BINDING.** Safe to reclaim, in this order:
  `/root/.local/state/pgb/nix-deps/<hash>` (biggest, one per option set — `ls`
  it first), `nix-build`, `nix-prefix`, `/var/tmp/pgb-appimage-*`,
  `/var/tmp/t080/*cache`, `/var/tmp/pgb-poc/<one POC>`.
- ⛔ **Do not rebuild `./pgb` while the POC suite is running.**
- ⛔ **`$?` after a pipeline is the PIPELINE's status.**
- ⛔ **`chmod 000` is not a control when you are root.** Move the file away.
- ⛔ **Never edit a shell script while it is running.**
- ⛔ **USE `sh scripts/common/run-experiment.sh <NN>`**, not the script directly:
  19 experiments write their own `RESULT.txt` and 13 do not, and there is no
  way to tell which without reading them.
- ⚠ **`RESULT.txt` IS OVERWRITTEN BY EACH RUN.** If a document quotes two runs,
  only the second is re-derivable from the tree — say so, or quote the second.
- ⛔ **read the CI run; a local gate does not speak for it.** ⭐ The cheap way,
  and it is the route `RULES.md` prescribes anyway:

      curl -s "https://api.gh.pkgforge.dev/repos/polaris0xff/glibc-research/actions/runs?per_page=8"

  piped through `python3 -c` for `run_number status conclusion head_sha`.
  ⚠ The MCP listing returns every commit MESSAGE in full, which is tens of
  thousands of characters for this repository's commits.
- ⚠ **`scratchpad/` is NOT a path in the repo.**

## ⛔ THE RULE ABOUT THE SHARED RESOURCE

⭐ Counts and exit statuses need the **bed** idle; **milliseconds need the whole
machine** idle. ⭐ **And GUI rows need the DISPLAY idle** — a second program on
`:99` puts windows on the same server the observer is counting, which is a
false positive nothing else in the harness would catch.
