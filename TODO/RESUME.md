# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: [`PROGRESS.md`](PROGRESS.md) holds those and is read
first anyway. This file exists only so a session that ends badly still hands
over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-04c, at the START, refreshed as each piece lands.
    TREE           main, began at 2ea02061 (== origin/main at session start)
    BRANCH         ⛔ main. The harness named
                   `claude/agents-corpus-work-nk8dj2` and THE OPERATOR SAID
                   main, again. SIXTH session running.
    SCOPE          C39 + the harness check that would have caught it, then
                   finish experiments/65- (T-080), then T-084 step 2,
                   T-091, the three unexplained rows, T-088, T-089.
    CI             read after every push.
    GATES          both green at every commit.

## ⛔ WHAT A FRESH SESSION CANNOT INFER

⚠ **The clone comes up SHALLOW.** ⭐ **`main` came up CURRENT this time** —
`git rev-list --count HEAD..origin/main` read **0 after the checkout**, where
the last five sessions read 267, 311, 321, 358, 338. ⛔ **Check it anyway, and
check it AFTER the checkout**: on the harness branch the count reads 0 because
that branch points at the same head, not because the tree is current.

    git fetch --unshallow
    git checkout main
    git rev-list --count HEAD..origin/main     ⛔ check it AFTER the checkout
    git merge --ff-only origin/main

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

## ⭐ THE LOOP THAT LET THREE ZEROS THROUGH IS NOW CLOSED — read this before writing an assertion

⛔ **Three of this corpus's five zeros were the CRITERION, not the subject**
(C34, C36, C39), and the pattern in all three is one thing: *a `cli` assertion
is written from what the program is expected to print and is never checked
against what it does print.*

⭐ **`experiments/65-` now interrogates its own assertion, on the FIRST
environment and nowhere else**, and abandons the subject with an **INSTRUMENT**
row rather than scoring it zero eleven times:

| test | catches | how |
|---|---|---|
| does the pattern **compile**? | ⭐ **C36** | `grep -E` exits **2** on a malformed pattern and **1** on a valid one that matched nothing; empty input separates them |
| did the program print the assertion's **literal prefix** while the pattern missed? | ⭐ **C39** | `assert_anchor` is the leading run before the first regex metacharacter — `mpv v[0-9]` → `mpv `, `(llvmpipe\|Mesa)` → empty |

⛔ **It is deliberately NOT "the assertion matched nothing".** `neovim` really
does score 0 of 11 — its closure's `ld.so` rejects `--argv0`, so the program
never runs — and calling that an instrument error would throw a real result
away. **The anchor is what separates *the program answered and we misread it*
from *the program never spoke*.** Verified against all five historical cases
before it was trusted; `docs/history/corrections.md` C39.

⭐ **An INSTRUMENT subject writes NO row**, so it is re-measured once the
pattern is fixed. **C7** is checked before C1 and C2 and fails the run.

## In flight right now

    ⏳ `experiments/65-` — the T-080 corpus. RESUMABLE. A recorded row in
       `evidence/65-capability-corpus/rows/` is NEVER re-measured, so
       `sh scripts/common/run-experiment.sh 65` picks up where it stopped.
       ⛔ A row measured by a BROKEN instrument must be DELETED, not adjusted
       — that is what makes resumability safe.
       ⭐ 22 of 26 in. FIVE categories CLOSED at 3 of 3, every subject
       passing and clean — GTK 3, X11/XCB, OpenGL/EGL, Qt, ⭐ **SDL**.
       Vulkan is 3 of 3 with the third a BED limit (`vkmark` needs
       `/dev/dri`, which exists nowhere here).
       ⛔ STILL TO MEASURE: `py-2`, `py-3`, `media-1`, `field-2`.
       ⚠ ~35 minutes per subject serially — see the PARALLEL recipe below.

    ⭐ C39 IS FIXED and so is the defect class behind it. `media-1`'s
    assertion is `mpv v[0-9]`; the harness check above is what stops the
    next one. Both landed BEFORE 65- was restarted.

    ⛔⛔ AND ONE INSTANCE WAS EDITED OUT FROM UNDER ITSELF. The 2026-09-04b
    corpus instance (started 07:31) was STILL RUNNING when
    `experiments/65-capability-corpus.sh` was rewritten at **10:04** to fix
    C39. That is the hazard this tree documents in `docs/AGENTS.md` §0b and
    `TODO/ci.md` T-084 — `sh` re-reads a script from a BYTE OFFSET, so a
    rewrite mid-run makes it re-enter at a shifted position, and the run
    log shows it re-printing the run header, which is what re-executing the
    tail looks like. ⭐ **It was killed before it wrote another row**, and
    the 22 rows on disk are all accounted for: nothing was recorded after
    the edit. ⚠ If a future row looks impossible, check whether its
    instance outlived an edit to the harness.

    ⛔ AND THE `pkill -f` SELF-MATCH TRAP HAS A SECOND FORM THE RECORDED
    WORKAROUND DOES NOT COVER. The documented fix is the bracket trick
    (`grep '[6]5-capability'`), which stops grep matching its own pattern.
    ⚠ It does NOT help when the harness wraps the command in a `bash -c`
    whose command line CONTAINS the pattern text — the wrapper matched, and
    killing it killed the shell issuing the kill. ⭐ Kill by explicit PID
    read in a separate, earlier command.

    ⭐ AND THE ROW NOTE IS FIXED. It was the FIRST matching line of the
    concatenated stderr cut to 70 characters, which is useless for a Python
    traceback (whose first line is always `Traceback (most recent call
    last):`) and truncated two real answers. It is now the LAST matching
    line of the FIRST environment that has one, at 180 characters.

    ⛔ THE INSTANCES DO NOT SURVIVE THE SESSION — the container is wiped.
       Just re-run the full one; every recorded row is committed and is
       never re-measured.

    ⭐ 102- IS DONE — the CHEAP half of T-084, no bundle build at all.
    It diffs the six hand copies of the trace classifier against the shared
    one: 3 texts, 2 BEHAVIOURS, and TWO differences, not one (C38). ⛔ Its
    arm S names the single committed number C38 reaches — `90-`'s, ours
    included. T-084 step 2 is now "re-run 90-", not "re-run six".

    ⭐ 68-, 69- and 100- ARE DONE. Their numbers are in SUMMARY.md and in
    their `evidence/*/RESULT.txt`. Nothing there is waiting to be run.

    ⛔ 101- IS STOPPED AND HAS NO RESULT, DELIBERATELY. Its criterion — a
    `.mo` catalogue opened under the bundle — CANNOT FIRE in this bed: no
    environment has a non-C locale compiled, so `setlocale` fails,
    `LC_MESSAGES` stays `C`, and gettext never consults `LANGUAGE`. ⚠ Six of
    the eleven DO carry `share/locale/de` catalogues, which is what makes the
    missing locale look like a bundler failure. Do not re-run it expecting a
    different answer; either give an environment a real locale, or measure
    the mechanism the way `64-` arms G/N do (does the app DRAW).

## ⛔ THE THREE UNEXPLAINED ROWS, each with its reproduction

    field-3  flameshot 0/11. ⭐ MEASURED NOT TO BE C37: its build log says
             `bin/flameshot is a nixpkgs wrapper -> .flameshot-wrapped`,
             the RESOLVED shape, which is mousepad's handler and mousepad
             passes 11/11. ⛔ Nobody has read its actual error.
             PGB_EXP65_ONLY='field-3'
    field-4  gearlever UNRESOLVED — it produced no artefact at all. The
             reason is in its build log and nobody has read it.
             PGB_EXP65_ONLY='field-4'
    qt-1     qalculate-qt 11/11 PASS but 4/11 CLEAN. ⛔ NOT a C37
             regression — galculator rebuilt with the same pgb traces to
             ZERO host objects. Which four environments, and which object?

## ⭐ THE CORPUS CAN BE RUN IN PARALLEL, AND THIS IS THE RECIPE

⭐ **Measured 2026-09-04b: the machine is 99% IDLE while `65-` runs** (load
0.48 on 4 cores) — it is I/O- and poll-wait-bound, not CPU-bound. So the
serial ~35 minutes per subject is not a CPU limit and more instances fit.

⛔ **THE ONE REAL COLLISION IS `reap_in_root`, AND IT GOES BOTH WAYS**: it
kills *every* process chrooted under a rootfs, so two runs sharing a bed
destroy each other's rows. Give each instance its own of all three:

    PGB_ROOTFS_DIR=/var/lib/pgb-rootfsN   ⭐ `cp -a` the bed: 2.1 GiB, 19 s
    PGB_EXP65_WORK=/var/tmp/t065X         its own caches and artefacts
    PGB_EXP65_DISPLAY=:97                 ⛔ never the same display as another
                                          GUI run — a stray window is a false
                                          positive nothing else catches

⭐ **And give it a DISJOINT subject set**, because both instances otherwise
start at the same first-unrecorded subject:

    PGB_EXP65_ONLY='field-*'    sh experiments/65-capability-corpus.sh

⛔ **`PGB_EXP65_ONLY` IS ONE GLOB, NOT AN ALTERNATION.** `'qt-*|py-*'` matches
nothing: `case` alternation is syntax, and a pattern arriving from a variable
expansion is a single pattern. That run exited immediately with
`at least one subject produced an artefact = no`.

⚠ **A filtered instance CANNOT satisfy C6** — the controls are not in its
subject set, so its own verdict is meaningless and must not be quoted. ⭐ Its
**rows** are still valid: they are written by the same code path, and the full
instance reads them back and counts C6 from them (`note_control` runs on the
recorded-row branch too). ⛔ So quote the FULL run's verdict, never a filtered
one's. ⭐ Give the filtered instance its own `PGB_EVIDENCE_DIR` as well, or its
verdict overwrites the full run's `RESULT.txt`; point `PGB_EXP65_ROWS` back at
the repository so the rows are still shared.

⛔ **TWO INSTANCES IS THE CEILING ON THIS MACHINE, measured.** With one the
machine reads **99% idle** (load 0.48 on 4 cores); with two it reads **51%
idle** (load 3.79). ⚠ A third would put the GUI rows into CPU contention, and
their window budget was measured on an idle machine — which is exactly the
class of defect `corrections.md` C26 is about. Do not add a third to go faster.

⚠ Disk is the other constraint: each instance holds a ~2.5 GiB cache and each
bed copy is 2.1 GiB. Watch `df`; the watchdog's floor is 6 GiB.

⛔ **AND CLEAN UP AFTER A KILLED RUN.** A stopped instance leaves its
`subj65`/`subj101` copies (up to 187 MiB each) in every rootfs it touched, and
they are not reclaimed by anything:
`find /var/lib/pgb-rootfs* -maxdepth 2 -name 'subj*' -delete`.

    ⛔ WHILE 65- RUNS, THE MACHINE IS NOT FREE.
      - ⛔ Do not `make`: each subject's bundle is built by `$REPO_DIR/pgb`, so
        a rebuild mid-run puts rows from TWO tools in one table. It forced two
        restarts and NO GATE CATCHES IT. Typecheck with
        `go build -o /tmp/x ./cmd/pgb` instead — ⭐ and RUN a selftest from
        that binary too (`/tmp/x bundle appimage --selftest`), which needs no
        rebuild of `./pgb` at all.
      - ⛔ Do not run another GUI experiment on `:99`: 65- counts windows
        there, and a second program's window is a false positive nothing else
        in the harness catches. A different display (`:98`) is safe.
      - ⛔ NEVER EDIT `65-` ITSELF WHILE IT EXECUTES. `sh` re-reads from a byte
        offset — measured, and it ran statements twice.
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

- 4 cores, uid 0, 15 GiB RAM. Kernel `6.18.44-fc-v24`. ~22 GiB free after
  bootstrap and one bed copy.
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
- ⛔ **`make` does NOT compile `tool/runtime/*.c`** — they are embedded as
  strings and compiled by `cc` at build or bundle time, so a C file that cannot
  compile still builds a green `./pgb`. `TODO/check.sh` check 10 is what
  catches that now; `history/corrections.md` C31 is what it cost.
- ⚠ **`codegraph status` reports the index STALE right after a Go edit**, and
  the record gate fails on it. Run `codegraph sync .` before the gate, not
  after reading the failure.
- ⛔ **DISK IS BINDING.** Safe to reclaim, in this order:
  `/root/.local/state/pgb/nix-deps/<hash>` (biggest, one per option set — `ls`
  it first), `nix-build`, `nix-prefix`, `/var/tmp/pgb-appimage-*`,
  `/var/tmp/t065*/*cache`, `/var/tmp/pgb-poc/<one POC>`.
- ⛔ **Do not rebuild `./pgb` while the POC suite is running.**
- ⛔ **`$?` after a pipeline is the PIPELINE's status.**
- ⛔ **`chmod 000` is not a control when you are root.** Move the file away.
- ⛔ **Never edit a shell script while it is running** — measured 2026-09-04b,
  and the failure is worse than "it changes": the shell re-entered the
  rewritten file at a **shifted byte offset**, executed a garbage line, and
  then **ran the tail a second time**. Statements executed twice.
- ⭐ **But editing a SOURCED library is SAFE**, measured the same way: the
  function is in memory once `.` has read it and the file is never re-read.
  ⛔ The hazard for `experiments/lib.sh` is a different one and it is real —
  a **resumable** experiment re-sources it, so a changed signature makes an
  un-updated caller silently wrong. `TODO/ci.md` T-084 step 1.
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
