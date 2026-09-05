# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: [`PROGRESS.md`](PROGRESS.md) holds those and is read
first anyway. This file exists only so a session that ends badly still hands
over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-05, refreshed at 03:50Z while the corpus runs.
    TREE           main. ⛔ EIGHTH session running.
    BRANCH         ⛔ main. The harness names a `claude/*` branch and THE
                   OPERATOR SAYS main.
    GATES          both green at every commit. CI green (444).
    STATE          ⛔⛔ THE CORPUS RE-RUN IS IN FLIGHT AND IT IS THE SESSION'S
                   MAIN DELIVERABLE. Two instances, started 03:14Z, ~28 min
                   per subject, 13 subjects each -> expect ~09:15Z.
                     A  pgid 8573  /var/tmp/evidence-65a  :99  rootfs
                        PGB_EXP65_ONLY='gtk3-*|x11-*|gl-*|vulkan-*|sdl-1'
                     B  pgid 8673  /var/tmp/evidence-65b  :98  rootfs2
                        PGB_EXP65_ONLY='sdl-2|sdl-3|qt-*|py-*|media-*|field-*'
                   ⭐ FIRST ROW IS GREEN AND IT IS THE C6 CONTROL:
                   gtk3-1 galculator 11/11 pass, 11/11 clean, 0 spawns.
                   ⛔ IT WAS KILLED AND RESTARTED TWICE before this, for two
                   real defects — C57 and C58. Both are fixed and committed.

## ⛔⛔ WHAT TO DO WHEN THE CORPUS FINISHES — IN THIS ORDER

    1. ⭐ A FINAL UNFILTERED READ-BACK PASS WRITES THE VERDICT. Both running
       instances are FILTERED, so neither may be quoted (C6's controls are
       split across them). The read-back recomputes C1/C2/C6/C8/C9 from the
       recorded rows in SECONDS -- it re-runs nothing:
           sh scripts/common/run-experiment.sh 65
    2. ⛔ `make` — the tree carries an UNINSTALLED Go change (atomic `--out`,
       commit 3b5fe7aa). `./pgb` does not have it yet. Forbidden until now
       because a rebuild mid-run mixes two tools in one table.
    3. `sh poc/run-all.sh --rebuild`   (C53)
       ⛔ AND IT DOES **NOT** EXERCISE THE ATOMIC `--out` OF STEP 2. Measured:
       **no POC bundles** (only `poc/92-miniflux`, which is in progress and
       not one of the ten), and **CI does not pack either** — its jobs are
       toolchain / matrix / build / run-matrix / verify-docker / probe-host,
       and `bundle appimage --selftest` is unit-level and never runs
       `mkdwarfs`. ⭐ **NINETEEN EXPERIMENTS call `bundle appimage`**, so the
       first real exercise of `pack()`'s rename is the next bundle
       experiment — `105-` or `108-` below. Watch for `built <path>` naming
       the FINAL name and no `.part` left behind.
    4. ⛔ `sh scripts/common/bed-fixtures.sh --install all` — THIS CONTAINER
       HAS NO FIXTURES AND A FRESH ONE NEVER WILL. Checked 2026-09-05:
       `theme=no` on 11 of 11, `locale=no` on all 7 glibc rows, `dbus=no` on
       10 of 11 (rockylinux-8 ships one natively). ⛔ `108-` NEEDS THE DBUS
       ONE — its own header says so — so running it first would reproduce a
       failure that is the BED's and score it against the bundle, which is
       C48's exact trap. `101-` and `106-` need theme and locale.
       ⚠ A fixture does not survive `pgb rootfs fetch`; re-install after one.
    5. `sh scripts/common/run-experiment.sh 108`   (needs `make` AND the
       dbus fixture)
    6. `105-`, then `103-` run 2.
    7. `10-`, `20-`, `50-` (bed) and `40-` (IDLE MACHINE) — T-096's four.

⭐ **AND THE TWO CORPUS RUNS ARE DIRECTLY COMPARABLE, which was checked rather
than assumed.** `docs/AGENTS.md` §8 says a result must state which bed it
describes, and this bed carries no fixtures. ⛔ Neither did the previous one:
`bed-fixtures.sh` landed at **2026-09-04 14:29** and the corpus it is being
compared against finished at **12:22** the same day. So no bed difference
confounds the before/after.

⚠ **AND ONE EDIT IS DRAFTED BUT NOT APPLIED**, because `65-` must never be
edited while it executes: **C1 and C2 compare against `MEASURED` while C5
pre-registers `xterm` as an exception**, so the corpus can never exit 0 and its
pass/fail bit carries no information — only the number moves. ⭐ The fix is the
pattern this tree already uses twice (`STALE-EVIDENCE.txt`, `criteria-audit.sh`):
a NAMED exception list, so an UNLISTED failure flips the bit. ⚠ Apply it AFTER
the run, then re-run step 1 — which costs seconds, because it reads the rows.

## ⛔⛔ THE PIVOT — OPERATOR INSTRUCTION, 2026-09-05, AND IT OUTRANKS THE WORK ORDER

⛔ **The operator stopped the session mid-run and called a REFACTOR.** The
measurement work below is not cancelled, but it is no longer what a new session
should start on. ⭐ **The full instruction is in the kickoff prompt the operator
holds; this is the durable copy so it cannot be lost.**

**The diagnosis, in the operator's own terms — seven problems:**

1. ⛔ **Code comments are history, changelog-in-a-manual and narrative lore.**
   An agent has to shift through the slop before it can read, understand or
   change anything. Measured 2026-09-05: **4,744 comment lines in 24,945 lines
   of Go (19%)**, and **142 non-doc files** carry the `⭐/⛔/⚠` markers.
2. ⛔ **The docs are the same** — bloated, full of narrative about problems
   already solved, and they MISGUIDE agents.
3. ⛔ **Scripts scattered everywhere** that should be proper functions or
   builtins.
4. ⛔ **The codebase only grows.** Instead of cleanly separating
   functions / features / utils into components and libraries, more is piled on
   top.
5. ⛔ **Reliance on third parties that are THEMSELVES forks**, so we patch forks
   or work around them.
6. ⛔ **AI commit attributions, `Co-Authored-By`, and excessive emoji** —
   *"anyone that takes a look at this project has a repulsive reaction and
   refuses to look or even consider its merits."* Measured: **476 of 488
   commits** carry the attribution trailer.
7. ⛔ **References mined and studied, then forgotten** — by the time an agent
   reaches `references/` its context is already polluted and exhausted by
   problems 1-4.

**The shape of the fix — six items, in order:**

0. The overhaul follows **`https://github.com/Azathothas/TEMPLATE`**, adopting
   its docs, conventions, rules and scripts where applicable. Relicense to
   **0BSD**. ⚠ *Already 0BSD as of 2026-09-05 — `LICENSE` is the BSD Zero
   Clause text; what is owed is consistency, not a change.* ⚠ TEMPLATE is
   **not** vendored under `references/`; only `docs/methodology/` came from it.
1. **Every comment in every code file and script** is extracted and read line
   by line, then cleared of history / changelog / narrative lore. ⭐ **Only
   genuine GOTCHAS and LESSONS are preserved.**
2. **Every doc** gets the same review.
3. **The whole `docs` tree is completely overhauled** into topic-by-topic
   files. All **current** facts (not narrative history, not hallucinated, not
   contradicting) rewritten so they cannot drift again. ⚠ The shape the
   operator gave, as an EXAMPLE and not as a literal path list — none of these
   exist yet, which is why they are not written as links: a *nix* overview
   README routing to a *bundler* README, which documents the bundler's current
   features, behaviours and limitations and links onward to a per-topic page
   such as a Qt one carrying all current Qt-relevant information. Every leaf is
   referenced by a sub-parent, which is referenced by a parent.
4. ⭐ **`pgb` IS EVOLVING INTO A FAMILY**, with `pg-toolkit` as the single
   entry point that bundles everything:
   - **`pga`** — *Portable GLIBC AppImage*: the current nix bundler (nixappimage).
   - **`pgb`** — *Portable GLIBC Binary*: the current glibc STATIC builder.
   - **`pgc`** — *Portable GLIBC Container* (future): behaves like `runimage` or
     `flatimage`, packing a tiny container/distro itself. Probably required to
     make genuinely complex applications — `podman`, `docker` — portable.
   - **`pgd`** — *Portable GLIBC Distro* (final): a live, relocatable, full
     Linux distro that still behaves like a native AppImage or binary.
   ⭐ **In automode `pg-toolkit` tries `pgb` first**, with as much versatility
   as possible — prefer the host where it does not interfere, or be completely
   standalone, smartly — **then `pga`, then `pgc`**. ⛔ `pgd` should never be
   needed except for something like a portable Alpine that beats
   containers/chroot.
5. ⭐ **Vendor / patch / reimplement / iterate on everything we depend on** —
   `sharun`, the `uruntime`, and the rest. ⛔ The runtime and tooling must be
   adapted and capable for OUR needs; never wait on upstream or be subject to
   their whims. ⚠ The plausible exception is the `mkdwarfs` binaries, and those
   can be bundled anyway.

6. ⛔ **THE DEV ENVIRONMENT IS NO LONGER ASSUMED.** Work may happen on a
   remote Linux container like the one this was written on, **or on a Windows
   dev machine with WSL and podman**. ⭐ There is a sanctioned way to work and
   **the TEMPLATE will guide it**. ⚠ So nothing may hardcode this container's
   assumptions — uid 0, `chroot`, `/var/lib/pgb-rootfs`, `unshare --mount`,
   dockerd. Podman is currently marked **untested** in `docs/AGENTS.md` §9 and
   that becomes a first-class target rather than a footnote.

⛔ **All of the above must be turned into proper TODO entries**, not left as
prose.

## ⭐ THE OPERATOR'S FOUR DECISIONS, 2026-09-05 — asked and answered

| question | ruling |
|---|---|
| **git history** | ⭐ **REWRITE IT.** Strip the AI attribution and session trailers from all 488 commits. ⛔ Everything that is not needed — **after it has been combed and its useful parts mined into the core docs, scripts and tools** — is discarded straight into a dedicated directory under the docs history tree. **The TEMPLATE provides the further instructions.** ⚠ Consequence to plan for: `evidence/STALE-EVIDENCE.txt` pins commit PAIRS and `docs/history/corrections.md` quotes hashes; every one of them breaks on a rewrite and must be repaired or replaced. |
| **emoji and markers** | ⭐ **Any and all HUMAN-facing docs must be written for a human reader**: concise, containing exactly what is current and true, reading like a **technical manual**. ⛔ **There is no place for emoji in human-facing docs.** ⚠ **AGENT-facing docs are the exception** — the TEMPLATE allows a *minimal, not excessive* use. |
| **the corpus run** | ⭐ **KILLED**, 2026-09-05 04:20Z, at 4 of 26 subjects. See `evidence/65-capability-corpus/spawns/README.md` for exactly what landed and what it validated. |
| **`pg-toolkit`** | ⭐ **P0, and one of the FIRST tasks after the docs and repo refactor** — not during it. ⛔ It must be done **in steps**: vendoring, patching and reimplementing the dependencies FIRST, then the toolkit on top. |

## ⛔ WHAT A FRESH SESSION CANNOT INFER

⚠ **The clone comes up SHALLOW.** ⭐ `main` came up CURRENT again on
2026-09-05 — `git rev-list --count HEAD..origin/main` read **0 after the
checkout**. ⛔ **Check it anyway, and check it AFTER the checkout**: on the
harness branch the count reads 0 because that branch points at the same head,
not because the tree is current.

    git fetch --unshallow
    git checkout main
    git rev-list --count HEAD..origin/main     ⛔ check it AFTER the checkout
    git merge --ff-only origin/main

⚠ **The container is fresh: nothing is bootstrapped.** Timed 2026-09-05 on a
4-core / 15 GiB machine: `make` 13 s, `bootstrap --detach` **~6 minutes wall**
for nix + chroot env + bed + docker env in parallel.

    make                                     builds ./pgb
    ./pgb bootstrap --detach                 nix + env + bed, parallel
    ./pgb bootstrap --check                  is it ready
    sh scripts/common/install-codegraph.sh   v1.6.0

⭐ **AND FOR ANY GUI WORK, THREE PACKAGES, NOT TWO.**

    apt-get install -y musl-tools musl-dev   # the musl arm of 61-/63-/67-
    apt-get install -y xvfb x11-utils        # Xvfb, xwininfo, xdpyinfo
    apt-get update && apt-get install -y x11-apps   # ⛔ xwd — see below

⛔ **`xwd` IS IN `x11-apps`, NOT `x11-utils`, and `experiments/108-` needs it**
as its instrument control. ⚠ And `apt-get install x11-apps` fails **404** on a
container whose package lists predate the last Ubuntu point release —
`apt-get update` first, always.

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
seven rows in**, leaving an orphaned `strace` still writing. ⛔ Confirm a run is
dead by **PID and log mtime**, never by `pgrep -f` (it matches the watching
shell).

## ⭐⭐ THE WORK LIST, CONCRETE AND IN ORDER

⛔ **Read `docs/history/corrections.md` C49–C56 before trusting any
host-object count in this tree.** Two of them change what "clean" means.

### 1. ⛔ FINISH WHAT IS IN FLIGHT — these are runs, not investigations

| # | what | why it is owed | cost |
|---|---|---|---|
| 1 | ⛔ `sh scripts/common/run-experiment.sh 65` | **ONE re-run clears TWO pinned debts**: **C49** (the host predicate missed `/usr/bin/ld.so` — the host loader) and **C54** (`clean` counted subjects that never started). ⭐ **AND SINCE 2026-09-05 IT ALSO ANSWERS T-094's COUNT** — the spawn instrument is in the same run. Use the parallel recipe below. | hours |
| 2 | `sh scripts/common/run-experiment.sh 108` | flameshot's capture. **Pre-registered and never run.** The last *Untried* in the record. | ~50 min |
| 3 | `sh poc/run-all.sh --rebuild` | `tool/runtime/pgb-storefix.c` changed (**C53**) and the POC suite has not run since. | ~1 h |
| 4 | `sh scripts/common/run-experiment.sh 105` | pinned stale by the `0\n0` sweep; needs all eleven. | ~20 min |
| 5 | `sh scripts/common/run-experiment.sh 103` | run 2, owed since T-091, plus the C54 start-guard on D3. | ~30 min |

### 2. ⭐ THE OPEN QUESTIONS, SHARPEST FIRST

| entry | the question, and what to measure |
|---|---|
| ⭐ **T-094** (P1) | **An application that shells out to the host loads the host's libc through that shell, and no path rewriting prevents it.** ⭐ **THE CHEAP HALF IS NOW INSTRUMENTED**: `exp_host_spawns` in `experiments/lib.sh` counts, per subject, every host program the artefact's own process set `execve`s, **by name**. `65-` records it to `evidence/65-capability-corpus/spawns/<id>.tsv`. ⛔ Until that run lands the count does not exist — an old row carries **no** spawns file and is reported `-` (not measured), never 0. |
| **T-093** (P2) | The **only** field objection left with no measurement: *"no more Vulkan layers like mangohud"*. A real layer is already in the mesa closure (`VkLayer_MESA_overlay`). ⚠ Runs on the RUNNER host, not the bed — a fixture may not add a shared object. |
| **T-090** (P1) | The sandbox rung. ⚠ `unshare -U` succeeds on the host and that is **not** the answer; measure it **inside the chroot bed** with `lsns -t user`. |
| **T-077** (P1) | The head-to-head was measured on the **retired** glibc pin and nobody re-ran it. |
| **T-095** (P2) | ⛔ **CI is fragile in a way that costs whole runs**: the libiconv fetch is ONE host (`ftp.gnu.org`) with no mirror and no retry, in the `build` job. ⚠ A retry alone is not the fix: the failure was a **two-minute connect timeout**. |
| **T-059** (P1) | Real GPU. Every GL and Vulkan row here is `llvmpipe`/`lavapipe`. |

### 3. ⛔ WHAT IS **NOT** OWED, so nobody re-opens it

* **T-080** the corpus — 26/26, six categories closed. Retired.
* **T-084** the six classifier copies — gone, `102-` reads them back out of git. Retired.
* **T-088** `--with-program` — was being exercised by `90-` all along (**C45**). Retired.
* **T-089** the `-static` row — **answered by a refusal**: a fully static closure has no loader, the bundler refuses it, and there is no artefact to ask the question of. Retired.

## ⛔⛔ THE ONE PATTERN THAT PRODUCED MOST OF LAST SESSION'S FINDINGS

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

⭐ **AND THE HABIT THAT CAUGHT THEM**: keep the trace. `101-` and `107-` retain
the first environment's transcripts on purpose, because the question a zero
raises cannot be answered without them.

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
expansion is a single pattern. ⭐ **THE FIX SHIPPED 2026-09-05**: the matcher
now splits on `|` and tries each pattern, so `'qt-*|py-*'` works and is
covered by `lib.sh --selftest`. ⚠ A space-separated list works too.

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

- 4 cores, uid 0, 15 GiB RAM. Kernel `6.18.44-fc-v24`. ~23 GiB free after
  bootstrap, before any bed copy.
- ⭐ **musl-gcc, Xvfb, x11-utils and x11-apps were installed this session** — a
  fresh container has none of them.
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
  compile still builds a green `./pgb`. `TODO/check.sh` check 10 catches that.
- ⚠ **`codegraph status` reports the index STALE right after a Go edit**, and
  the record gate fails on it. Run `codegraph sync .` before the gate.
- ⛔ **DISK IS BINDING.** Safe to reclaim, in this order:
  `/root/.local/state/pgb/nix-deps/<hash>` (biggest, one per option set — `ls`
  it first), `nix-build`, `nix-prefix`, `/var/tmp/pgb-appimage-*`,
  `/var/tmp/t065*/*cache`, `/var/tmp/pgb-poc/<one POC>`.
- ⛔ **Do not rebuild `./pgb` while the POC suite is running.**
- ⛔ **`$?` after a pipeline is the PIPELINE's status.**
- ⛔ **`chmod 000` is not a control when you are root.** Move the file away.
- ⛔ **Never edit a shell script while it is running** — the shell re-enters the
  rewritten file at a **shifted byte offset**, executes a garbage line, and
  then **runs the tail a second time**.
- ⭐ **But editing a SOURCED library is SAFE** — the function is in memory once
  `.` has read it. ⛔ The hazard for `experiments/lib.sh` is a different one and
  it is real — a **resumable** experiment re-sources it, so a changed signature
  makes an un-updated caller silently wrong.
- ⛔ **USE `sh scripts/common/run-experiment.sh <NN>`**, not the script directly:
  19 experiments write their own `RESULT.txt` and 13 do not.
- ⚠ **`RESULT.txt` IS OVERWRITTEN BY EACH RUN.** If a document quotes two runs,
  only the second is re-derivable from the tree.
- ⛔ **read the CI run; a local gate does not speak for it.** ⭐ The cheap way:

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
