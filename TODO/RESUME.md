# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: [`PROGRESS.md`](PROGRESS.md) holds those and is read
first anyway. This file exists only so a session that ends badly still hands
over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-03f, at the START; refreshed when arm G came
                   back 11 of 11. Session IN PROGRESS.
    TREE           main, began at 820d899f (== origin/main at session start)
    BRANCH         ⛔ main. The harness named
                   `claude/t-081-bundle-capabilities-2c24c0` and THE OPERATOR
                   SAID main, again. Fourth session running.
    SCOPE          ⭐ T-081 FIRST (the debloater/patcher), then REOPEN T-080
                   and re-measure EVERY capability in
                   docs/research/bundle-capabilities.md with THREE
                   applications per category, simple -> complex. Then
                   T-079's residue (/etc/services) and the codeset axis.
    CI             302 success; 303 and 304 in flight, READ THEM.
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

    ⭐ T-081's ACCEPTANCE TEST IS MET ON ARM G, run 1 of 2:

        experiments/64- arm G  galculator, UI at a compiled-in store path
        WINDOW on a real X server   11 of 11
        HOST shared objects          0 of 11
        ⛔ AND WITH NO BIND. The 2026-09-03e arm C used a mount namespace
        and root; this is the mechanism.

    ⏳ RUNNING: arms N (--no-storefix, the NEGATIVE CONTROL, must draw
       0 of 11), X (mousepad, the regression control) and P (meld, a
       PYTHON GUI application, which did not build at all before).
       ⛔ TWO RUNS OR IT IS NOT A NUMBER: run 2 is chained and starts
       automatically when run 1 exits. ⚠ Both runs use the pgb built at
       commit 2e66b0ce; the tool has changed since (reporting, the icon
       policy, usr/, --embed-netdb, --utf8-default) and none of those
       changes touch what arms G/N/X/P measure.

    ⏳ WRITTEN, PRE-REGISTERED, NOT YET RUN — in this order:
       experiments/66-  --embed-netdb, the ELEVENTH glibc-static quirk
       experiments/67-  --utf8-default, the one axis native musl wins
       experiments/65-  the T-080 REDO: 26 subjects, 3 per capability,
                        simple -> complex, plus 4 of the field's own 13
                        nixappimage recipes. ⚠ HOURS. It is RESUMABLE:
                        rows land in evidence/65-capability-corpus/rows
                        and a recorded row is never re-measured.

    ⛔ DO NOT `make` WHILE experiments/64- IS RUNNING. Its `stale()` check
    compares each artefact against ./pgb, so a rebuild mid-run makes later
    arms describe a different tool than earlier ones. That happened once
    this session and the run was discarded.

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
