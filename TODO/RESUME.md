# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: [`PROGRESS.md`](PROGRESS.md) holds those and is read
first anyway. This file exists only so a session that ends badly still hands
over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-04c, at the END, as the session checkpointed.
    TREE           main. CI green through the session, read after every push.
    BRANCH         ⛔ main. The harness names a `claude/*` branch and THE
                   OPERATOR SAYS main. SEVENTH session running.
    GATES          both green at every commit.
    STATE          ⭐ The capability corpus is COMPLETE (26/26), all three of
                   its unexplained rows are explained, and `101-` closed
                   GREEN (pass=5 fail=0, eleven rows) — rung 3's locale
                   criterion fires at last. ⛔ The session's
                   yield was mostly INSTRUMENT defects: eight corrections,
                   two of which (C49, C54) can move a committed number and
                   one of which (C56) turned a claim that had never been
                   measured into a measurement.

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

⛔⛔ **AND START A LONG RUN WITH `setsid`, NOT `nohup … &`. THIS COST A RUN ON
2026-09-04c.** A job backgrounded from a tool call shares the harness's process
group, so when the harness tore down an unrelated background task the whole
group went with it — the experiment's parent shell was killed **47 minutes and
seven rows in**, leaving an orphaned `strace` still writing:


## ⭐⭐ THE WORK LIST, CONCRETE AND IN ORDER

⛔ **Read `docs/history/corrections.md` C49–C56 before trusting any
host-object count in this tree.** Two of them change what "clean" means.

### 1. ⛔ FINISH WHAT IS IN FLIGHT — these are runs, not investigations

| # | what | why it is owed | cost |
|---|---|---|---|
| 1 | `sh scripts/common/run-experiment.sh 108` | flameshot's capture. **Pre-registered and never run.** The last *Untried* in the record. | ~50 min |
| 2 | ⛔ `sh scripts/common/run-experiment.sh 65` | **ONE re-run clears TWO pinned debts**: **C49** (the host predicate missed `/usr/bin/ld.so` — the host loader) and **C54** (`clean` counted subjects that never started). Until it runs, every "clean" number in the tree means *clean under the old rules*. ⭐ Use the parallel recipe below. | hours |
| 3 | `sh poc/run-all.sh --rebuild` | `tool/runtime/pgb-storefix.c` changed (**C53**) and the POC suite has not run since. `./pgb` is rebuilt and carries it. | ~1 h |
| 4 | `sh scripts/common/run-experiment.sh 105` | pinned stale by the `0\n0` sweep; needs all eleven. | ~20 min |
| 5 | `sh scripts/common/run-experiment.sh 103` | run 2, owed since T-091, plus the C54 start-guard on D3. | ~30 min |

### 2. ⭐ THE OPEN QUESTIONS, SHARPEST FIRST

| entry | the question, and what to measure |
|---|---|
| ⭐ **T-094** (new, P1) | **An application that shells out to the host loads the host's libc through that shell, and no path rewriting prevents it.** Measured on `qalculate-qt`, which probes for `gnuplot` via `/bin/sh -c --`. ⛔ **Measure this first and it is cheap**: how many of the twenty-six corpus subjects spawn a host program at all? The trace already shows it. If it is one, this is a footnote; if it is ten, it is the next real piece of work. **That count does not exist.** |
| **T-093** (P2) | The **only** field objection left with no measurement: *"no more Vulkan layers like mangohud"*. A real layer is already in the mesa closure (`VkLayer_MESA_overlay`). ⚠ Runs on the RUNNER host, not the bed — a fixture may not add a shared object. |
| **T-090** (P1) | The sandbox rung. ⚠ `unshare -U` succeeds on the host and that is **not** the answer; measure it **inside the chroot bed** with `lsns -t user`. |
| **T-077** (P1) | The head-to-head was measured on the **retired** glibc pin and nobody re-ran it. |
| **T-095** (P2, new) | ⛔ **CI is fragile in a way that costs whole runs**: the libiconv fetch is ONE host (`ftp.gnu.org`) with no mirror and no retry, and it sits in the `build` job — so an upstream timeout skips `run-matrix` and `verify-docker` and turns a green tree red. Seen on a documentation-only commit (run 437). ⚠ A retry alone is not the fix: the failure was a **two-minute connect timeout**. |
| **T-059** (P1) | Real GPU. Every GL and Vulkan row here is `llvmpipe`/`lavapipe`. ⭐ Per the operator's fixture ruling, ask what a *seam* could answer before recording it as a limit. |

### 3. ⛔ WHAT IS **NOT** OWED, so nobody re-opens it

* **T-080** the corpus — 26/26, six categories closed. Retired.
* **T-084** the six classifier copies — gone, `102-` reads them back out of git. Retired.
* **T-088** `--with-program` — was being exercised by `90-` all along (**C45**). Retired.
* **T-089** the `-static` row — **answered by a refusal**: a fully static closure has no loader, the bundler refuses it, and there is no artefact to ask the question of. The raw binary is already 11/11 with 0 host objects. Retired. ⚠ Only a **mixed** closure would move it, and none is known.

## ⛔⛔ THE ONE PATTERN THAT PRODUCED MOST OF THIS SESSION'S FINDINGS

⭐ **A criterion that cannot fire, hidden by a SKIP or by a zero.** It happened
five times, and no gate could see any of them:

| | what looked fine | what was true |
|---|---|---|
| **C48** | rows read 0, "the bed has no locale" | the **artefact** had no catalogues — the build log said `kept: none` on every run |
| **C50** | `102-` R1 read `skip` | it looked for a trace in the one directory `65-` **deletes** traces from |
| **C52** | `101-` L2 read 0 of 11 | it demanded a syscall its own mechanism **prevents** |
| **C54** | `clean 11/11` | the subject **never started**; zero host objects is also what that reports |
| **C56** | `SKIP arm B` | the archive was in the **build env**, and the experiment looked on the **host** |

⛔ **So: of every SKIP and every zero, ask what would have to be TRUE for this
to fire.** A skip is not a failure, and neither gate can tell a structural one
from an environmental one.

⭐ **AND THE HABIT THAT CAUGHT THEM**: keep the trace. `101-` and `107-` now
retain the first environment's transcripts on purpose, because the question a
zero raises cannot be answered without them.

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
