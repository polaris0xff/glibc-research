# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: [`PROGRESS.md`](PROGRESS.md) holds those and is read
first anyway. This file exists only so a session that ends badly still hands
over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-03e, at the START and refreshed as work landed.
                   Session COMPLETE.
    TREE           main, began at 8cfbeacb (== origin/main at session start)
    BRANCH         ⛔ main. The harness named `claude/t-078-079-080-tasks-jirtgy`
                   and THE OPERATOR SAID main, again. Third session running.
    SCOPE          T-078, T-079, T-080 — ⭐ ALL THREE CLOSED and MOVED to
                   HISTORY/entries/. ⭐ T-081 UNBLOCKED mid-session by the
                   operator and is now the next work.
    CI             ⭐ green on every push this session (296, 297, 298
                   success; 299 checked). ⛔ Read it after every push
                   anyway — two local-gate holes produced six red runs
                   last session and the lesson outlives the fix.
    GATES          both green at every commit.

## ⭐ WHAT THIS SESSION ESTABLISHED, in one line each

    musl-gcc is INSTALLED (musl-tools 1.2.4-2). experiments/61- arm A and
      63- arm M now RUN; they had been SKIPPING.
    the three-way parity matrix is in docs/comparison.md, skip=0, two runs.
    the glibc-static quirk list is ELEVEN, not ten: /etc/services.
    GTK out of a nix closure DRAWS REAL WINDOWS on 11 of 11, zero host
      objects — and both remaining bundle gaps are OUR tooling, T-081.

## ⛔ WHAT A FRESH SESSION CANNOT INFER

⚠ **The clone comes up SHALLOW and `main` comes up BEHIND.** Measured a third
time; the number was **311**:

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
will repeat this one's mistake:

    apt-get install -y musl-tools musl-dev   # T-078's musl column
    apt-get install -y xvfb x11-utils        # ⛔ T-080/T-081's ONLY honest
                                             # GUI criterion

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

    ⭐ NOTHING. Everything is committed, pushed and gated.

    ⛔ THE NEXT WORK IS T-081, AND ITS ACCEPTANCE TEST ALREADY EXISTS:
       `experiments/64-` arm G must go 0 of 11 -> 11 of 11 WITHOUT the
       bind that arm C uses. Two blockers, both in T-081's entry:

       1. absolute store paths compiled into .rodata. A rewrite cannot
          LENGTHEN the string — but `/nix/store/` is 11 bytes and so is
          `/tmp/.pgbs/`, so a same-length prefix substitution needs no
          relocation and no patchelf. ⛔ ANSWER THE SECURITY QUESTION
          FIRST: a fixed, predictable path under a world-writable /tmp is
          a symlink-attack surface, and host-fallback.md's rule applies.
       2. script entry points. resolveEntry oscillates between a
          makeBinaryWrapper ELF and the Python script it targets, five
          hops, then `no entry point` (assemble.go:60). NO Python GUI app
          bundles at all. That is the standard nixpkgs shape.

    ⚠ ALSO OPEN, and neither is blocking:
       the ELEVENTH glibc-static quirk (/etc/services) has a measurement
         and no mechanism. The precedent is --embed-tzdata: look first,
         carry a fallback, never prefer the stale copy.
       the environment-default codeset is the one axis where native musl
         beats both glibc columns, 11-0. --embed-locale answers a REQUEST
         and does not change what an unset LANG means.

## ⛔ Machine notes (carried forward, re-verify)

- 4 cores, uid 0. Kernel `6.18.44-fc-v24`. ~14 GiB free at session end
  (two GTK bundles at ~160 MB each plus their closures).
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
  `/var/tmp/t080/*cache` (this session's bundles), `/var/tmp/pgb-poc/<one POC>`.
- ⛔ **Do not rebuild `./pgb` while the POC suite is running.**
- ⛔ **`$?` after a pipeline is the PIPELINE's status.**
- ⛔ **`chmod 000` is not a control when you are root.** Move the file away.
- ⛔ **Never edit a shell script while it is running.**
- ⛔ **USE `sh scripts/common/run-experiment.sh <NN>`**, not the script directly:
  19 experiments write their own `RESULT.txt` and 13 do not, and there is no
  way to tell which without reading them.
- ⚠ **`RESULT.txt` IS OVERWRITTEN BY EACH RUN.** If a document quotes two runs,
  only the second is re-derivable from the tree — say so, or quote the second.
- ⛔ **read the CI run; a local gate does not speak for it.**
- ⚠ **`scratchpad/` is NOT a path in the repo.**

## ⛔ THE RULE ABOUT THE SHARED RESOURCE

⭐ Counts and exit statuses need the **bed** idle; **milliseconds need the whole
machine** idle. ⭐ **And GUI rows need the DISPLAY idle** — a second program on
`:99` puts windows on the same server the observer is counting, which is a
false positive nothing else in the harness would catch.
