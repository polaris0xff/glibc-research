# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: [`PROGRESS.md`](PROGRESS.md) holds those and is read
first anyway. This file exists only so a session that ends badly still hands
over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-04b, at the START; refreshed as each piece landed.
                   Session IN PROGRESS.
    TREE           main, began at 4f652df4 (== origin/main at session start)
    BRANCH         ⛔ main. The harness named
                   `claude/app-corpus-research-34c2el` and THE OPERATOR SAID
                   main, again. FIFTH session running.
    SCOPE          ⏳ T-080 finish experiments/65- (RESUMABLE). Then the arms
                   below that need the bed.
    CI             ⭐ 335-341 all success. Read after every push.
    GATES          both green at every commit.

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

    ⭐ **`./pgb` HAS BEEN REBUILT AND CARRIES C37.** The corpus was stopped a
    third time to do it, five rows measured against the old tool were
    DELETED (`gl-3`, `x11-3`, `py-2`, `py-3`, `field-2`), and both instances
    restarted. ⛔ Anything measured before that point is not evidence about
    the bundler.

    ⏳ `experiments/65-` — the T-080 corpus. RESUMABLE. A recorded row in
       `evidence/65-capability-corpus/rows/` is NEVER re-measured, so
       `sh scripts/common/run-experiment.sh 65` picks up where it stopped.
       ⛔ A row measured by a BROKEN instrument must be DELETED, not adjusted
       — that is what makes resumability safe.
       ⭐ 9 of 26 in. GTK 3 and X11/XCB are CLOSED at 3 of 3 each.
       ⚠ ~35 minutes per subject serially — see the PARALLEL recipe above.

    ⭐ AT THE TIME OF WRITING TWO INSTANCES ARE RUNNING:
       the FULL one (`/var/lib/pgb-rootfs`, `:99`, `/var/tmp/t065`) and a
       `field-*` one (`/var/lib/pgb-rootfs3`, `:97`, `/var/tmp/t065b`).
       ⛔ If they are gone, just re-run the full one; the rows survive.

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

## ⭐ A PRE-REGISTERED PREDICTION FOR THE NEXT PASS — write the result down

⛔ **Committed before the run that settles it.** Every corpus row at **11/11**
is a subject whose entry point is a **plain ELF**. Of the zeros, four —
`xterm`, `pdfarranger`, `virt-manager`, `neovim` — have a **script or
shell-wrapper** entry point.

⭐ **C37 fixed one of those** (`xterm`, which now draws in 2 s) by installing
the dot-named wrapper target and pointing the farm's `bin` at the sharun
hardlinks. ⭐ **The fix IS in `./pgb` now** — rebuilt once the corpus was
stopped, and the five affected rows deleted so they are re-measured against it.

| after `make`, re-measured | predicted |
|---|---|
| `x11-3` `xterm` | ⭐ **passes** — measured by hand at 1 window in 2 s |
| `gl-3` `glmark2` | ⭐ **passes** — also a shell wrapper; by hand it draws in 2 s and benchmarks at **360 FPS**. ⚠ Added after the table was first written, and said so |
| `py-2` `pdfarranger` | ⚠ **still fails.** It dies with a Python **Traceback**, so it STARTED — C37's exec failure is not its problem |
| `py-3` `virt-manager` | ⛔ **not predicted.** Nobody has read its error |
| `field-2` `neovim` | ⛔ **still fails**, and not ours: the closure carries **glibc 2.26**, and `ld.so` learned `--argv0` in 2.33 |

⚠ **If `pdfarranger` passes, the script-entry class was broken in a way C37
describes and this table did not** — the more interesting outcome, which is
why it is written down rather than left as an expectation.

    ⛔ FOUR NAMED UNKNOWNS THE CORPUS HAS ALREADY PRODUCED, each with its
    reproduction (`PGB_EXP65_ONLY='<id>'`):
        field-2  ⭐ SOLVED. The closure carries glibc 2.26; sharun passes
                 `--argv0` to the loader and ld.so learned that in 2.33, so
                 the loader takes it as the program. The SAME old glibc is
                 why the interposer warned about dladdr/dlsym (they were in
                 libdl.so until 2.34). `checkLoaderOptions` now says so at
                 build time. ⛔ The bundle still cannot run — that is a
                 nixpkgs-closure fact, not a bundler one.
        field-1  helix 0/11. ⚠ A named limit of the farm and ⛔ NOT
                 established as the cause: its closure carries ~200 bare
                 TOP-LEVEL `.so` grammars, and mergedFor maps eight names,
                 none of which covers a bare file. ⭐ The route is short --
                 copyLibraries already flattened those `.so` into lib/, so a
                 top-level `.so` could point there -- but helix also finds
                 grammars via HELIX_RUNTIME, so it may never consult the
                 compiled-in path. A fixed mergedFor that left the row at 0
                 would be the useful result.
        vkmark   ⭐ SOLVED, AND IT IS THE BED. The full error ends
                 `[/dev/dri]` -- vkmark enumerates DRM devices before it asks
                 Vulkan anything, and /dev/dri exists NOWHERE here. Not a
                 bundler failure. ⚠ The guess recorded earlier -- a
                 compiled-in data directory, the T-081 shape -- was WRONG.
        glmark2  0/11, and no error line matched at all. ⛔ The vkmark guess
                 was offered for this too and was wrong there; do not carry
                 it over. Read its full error first.
        gl-1     ⭐ SOLVED -- TWO instrument defects, and the second was
        vulkan-1 hiding behind the first. C34: the cli criterion was
                 `exit 0 AND the assertion` and eglinfo exits 3 headless.
                 FIXED -- and gl-1 STILL read 0/11, which forced the real
                 search. C36: the corpus was `|`-SEPARATED and these two
                 assertions ALTERNATE, so `cut -d'|' -f6` gave grep an
                 unmatched `(`, handed --extra the wrong word, and passed
                 the rest as arguments. Separator is `;` now; both rows
                 deleted and re-measured. ⭐ Both capabilities WORK, measured
                 by hand: eglinfo matches its real assertion 30 times, and
                 vulkaninfo exits 0 with GPU0 = llvmpipe 1.4.354.
        x11-3    ⭐ SOLVED AND FIXED; ⏳ re-measurement RUNNING. xterm's
                 nixpkgs `bin/xterm` is a SHELL wrapper that execs
                 `.xterm-wrapped` by store path. Two defects: the dot-named
                 target was never installed, AND the farm's `bin` resolved to
                 `shared/bin` (raw payloads) whose PT_INTERP names a
                 /nix/store loader the bundle lacks -- so execve returned
                 ENOENT FOR THE INTERPRETER and the shell printed it against
                 the program. Both fixed; xterm now DRAWS in 2 s, with
                 galculator as an unchanged regression control.
                 ⛔ THE FIX IS NOT IN ./pgb: it was not rebuilt while 65- ran.
                 AFTER `make`, DELETE the x11-3 row and re-measure.
                 corrections.md C37.
        py-2     pdfarranger 0/11. Its entry is a SCRIPT (python3.14 + the
                 static trampoline) and it dies with a Python Traceback --
                 ⛔ NOT the C37 exec failure. ⚠ The row's note is USELESS
                 here: it takes the FIRST matching line, which for a
                 traceback is "Traceback (most recent call last):". Fix the
                 note to prefer the LAST line and keep >70 chars, then
                 re-measure: PGB_EXP65_ONLY='py-2'.
        vulkan-1 ⛔ 0/11 AND THE CAPABILITY DEMONSTRABLY WORKS. Run by hand
                 out of the vulkan-2 artefact: `vulkaninfo --summary` exits
                 0 and enumerates GPU0 = llvmpipe, apiVersion 1.4.354 --
                 twice, with and without DISPLAY. ⛔ NOT the C34 defect and
                 NOT explained. The obvious unexamined difference: vulkan-1
                 was built `--name vulkaninfo` and vulkan-2 `--name vkcube`,
                 so the selector's DEFAULT differs. Delete the row and
                 re-measure.

    ⛔ AND ONE THING IS BLOCKED ON 65- FOR A DIFFERENT REASON THAN THE ONE
    THAT WAS WRITTEN HERE. T-084 changes `exp_classify_trace`'s signature.
    Editing the sourced `lib.sh` is SAFE while 65- runs (measured — the
    function is in memory). What is not safe is that 65- is **resumable**: a
    resumed run re-sources the new `lib.sh` and, calling it the old way,
    would report **every row zero host objects**. So the call sites must
    change with it, and one of them is 65- itself, which is executing.
    ⭐ T-084 step 1 now says `mode` goes LAST with a default, not first,
    precisely so that cannot happen.

## ⭐ THE CORPUS CAN BE RUN IN PARALLEL, AND THIS IS THE RECIPE

⭐ **Measured 2026-09-04b: the machine is 99% IDLE while `65-` runs** (load
0.48 on 4 cores) — it is I/O- and poll-wait-bound, not CPU-bound. So the
serial ~35 minutes per subject is not a CPU limit and more instances fit.

⛔ **THE ONE REAL COLLISION IS `reap_in_root`, AND IT GOES BOTH WAYS**: it
kills *every* process chrooted under a rootfs, so two runs sharing a bed
destroy each other's rows. Give each instance its own of all three:

    PGB_ROOTFS_DIR=/var/lib/pgb-rootfsN   ⭐ `cp -a` the bed: 2.1 GiB, 4 s
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
one's.

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
  `/var/tmp/t065/*cache`, `/var/tmp/pgb-poc/<one POC>`.
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
