# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: [`PROGRESS.md`](PROGRESS.md) holds those and is read
first anyway. This file exists only so a session that ends badly still hands
over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-04b, at the START. Session IN PROGRESS.
    TREE           main, began at 4f652df4 (== origin/main at session start)
    BRANCH         ⛔ main. The harness named
                   `claude/app-corpus-research-34c2el` and THE OPERATOR SAID
                   main, again. FIFTH session running.
    SCOPE          ⏳ T-080 finish experiments/65- (RESUMABLE, 1 of 26 rows in).
                   Then, BY MECHANISM: T-088 rung 1, T-089 rung 2, T-087
                   rungs 3+, T-090 rung 5, then T-084 / T-091 / T-092.
    CI             ⚠ runs after 4f652df4 NOT YET READ — READ THEM.
    GATES          not yet run this session.

## ⛔ WHAT A FRESH SESSION CANNOT INFER

⚠ **The clone comes up SHALLOW and `main` comes up BEHIND.** Measured a fifth
time; the number was **338** (267, 311, 321, 358, 338):

    git fetch --unshallow
    git checkout main
    git rev-list --count HEAD..origin/main     ⛔ check it AFTER the checkout
    git merge --ff-only origin/main

⛔ **On the harness branch the count reads 0.** It reads 0 because that branch
points at the same head, not because the tree is current.

⚠ **The container is fresh: nothing is bootstrapped.**

    make                                     builds ./pgb, ~13 s
    ./pgb bootstrap --detach                 nix + env + bed, parallel
    ./pgb bootstrap --check                  is it ready
    sh scripts/common/install-codegraph.sh   v1.6.0

⭐ **AND FOR ANY GUI WORK, TWO MORE PACKAGES**, without which the session will
repeat the 2026-09-03e mistake:

    apt-get install -y musl-tools musl-dev   # the musl arm of 61-/63-/67-
    apt-get install -y xvfb x11-utils        # ⛔ the ONLY honest GUI criterion

⛔ **AND START THE WATCHDOG BEFORE ANY LONG BUNDLE RUN.** Both ways these runs
die are silent — a fixed writable allowance that makes `df` read `Avail 0` at a
low `Used`, and a `dwarfs` daemon that outlives its AppImage and holds its
extraction directory:

    sh scripts/common/watchdog.sh --selftest          # ⛔ first, always
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

    ⏳ `experiments/65-` — the T-080 corpus. RESUMABLE and 1 of 26 rows in
       (`gtk3-1`, the C6 control, 11/11 pass 11/11 clean). A recorded row in
       `evidence/65-capability-corpus/rows/` is NEVER re-measured, so
       `sh scripts/common/run-experiment.sh 65` picks up where it stopped.
       ⛔ A row measured by a BROKEN instrument must be DELETED, not adjusted
       — that is what makes resumability safe.

    ⛔ WHILE 65- RUNS, THE MACHINE IS NOT FREE.
      - ⛔ Do not `make`: each subject's bundle is built by `$REPO_DIR/pgb`, so
        a rebuild mid-run puts rows from TWO tools in one table. It forced two
        restarts and NO GATE CATCHES IT. Typecheck with
        `go build -o /tmp/x ./cmd/pgb` instead.
      - ⛔ Do not run another GUI experiment on `:99`: 65- counts windows
        there, and a second program's window is a false positive nothing else
        in the harness catches. A different display (`:98`) is safe.
      - ⚠ Do not run bed-heavy experiments; the counts they take need the bed
        idle and 65- uses it continuously.

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

## ⛔ Machine notes (carried forward, re-verify)

- 4 cores, uid 0, 15 GiB RAM. Kernel `6.18.44-fc-v24`. ~24 GiB free after
  bootstrap.
- ⭐ **musl-gcc, Xvfb and x11-utils were installed this session** — a fresh
  container has none of them.
- ⚠ **`unshare -U` SUCCEEDS on the HOST here** (`/proc/sys/user/max_user_namespaces`
  = 64230). The `EPERM` T-090 is about is **inside the chroot bed**, which is a
  different question — measure it there with `lsns -t user`, do not carry the
  host answer across.
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
  `/var/tmp/t065/*cache`, `/var/tmp/pgb-poc/<one POC>`.
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
