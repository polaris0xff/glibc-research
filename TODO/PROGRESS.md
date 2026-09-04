# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log,
the entries, and [`../HISTORY/`](../HISTORY/).

    STATE     2026-09-03e  ✅ COMPLETE
    COUNTS    65 entries, 25 open, 40 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, TEN POCs
              CI: green through the session (read after every push)
              musl-gcc INSTALLED — the blocker the last session recorded
              throughput: glibc 8.40 ns/op vs musl 704.79 (malloc, 4 threads)
    NEW       ⭐ THREE ENTRIES CLOSED, AND TWO OF THEM CAME OUT AGAINST US.
              T-078 the three-way matrix: level with or ahead of native
              musl everywhere EXCEPT the environment-default codeset,
              where musl wins 11-0 and a pre-registered prediction said
              the opposite. T-079 found an ELEVENTH glibc-static quirk
              (/etc/services) by a re-runnable search. T-080 proved GTK
              works out of a nix closure — real windows on 11 of 11 —
              and showed both remaining gaps are OUR tooling.
              ⛔ AND THE OPERATOR CORRECTED THE T-080 INSTRUMENT MID-RUN:
              "cannot open display" is not a result, because bundled apps
              print it on real hardware WITH a display. A real display
              turned 11 green rows into 0.

## ⛔ READ THIS FIRST

⭐ **`pgb` is one statically linked Go binary.** The driver, the compiler
wrappers, the planner, the verifier and the bundler are the same executable,
built `CGO_ENABLED=0`, carrying the C runtime sources it compiles. The shell
and Python it replaced are under [`../HISTORY/`](../HISTORY/), unedited, and
are the oracle every gate is measured against.

⭐ **Every millisecond in this tree goes through
[`../experiments/clock.sh`](../experiments/clock.sh)** — median of N, arms
interleaved with a rotating start, and an **A/A control** whose ratio is the
floor below which no row may be believed.

⛔ **AND A GUI CLAIM NEEDS A DISPLAY, NOT A LOG LINE.** Added 2026-09-03e
after the operator caught this session scoring a bundle green on
`Gtk-WARNING: cannot open display`. `experiments/64-` now starts `Xvfb`,
binds its socket into each rootfs, and asks the **X server** whether a window
exists. ⛔ A program's own output is not evidence that it drew something.

## ⭐ The operator's rulings in force

| when | ruling | where it is recorded |
|---|---|---|
| 2026-09-01b | *"replace with per part claim, also anylinux is a bundle, our primary goal is still a static glibc binary that has none of the issues"* | [`../docs/REQUIREMENTS.md`](../docs/REQUIREMENTS.md) |
| 2026-09-01b | nixpkgs IS the planner | [`../docs/design/nix-front-end.md`](../docs/design/nix-front-end.md) |
| 2026-09-02b | *"pgb bundle isn't good enough, it is bloated, slow and a complete failure"* | T-066 |
| 2026-09-03c | *"us having a bigger size than anylinux-appimages and onelf is acceptable as long as ours performs better and packaging is just one command"* | [`../docs/design/toolchain.md`](../docs/design/toolchain.md) |
| 2026-09-03d | *"Defer comparing speed/startup/performance with anylinux-appimages & onelf for now"* | this page |
| ⭐ **2026-09-03e** | *"you may take on T-081 as you go since it is required now to complete your own tasks"* | T-081, **unblocked** |
| ⭐ **2026-09-03e** | *"you may be measuring the wrong success criteria ... confirm it properly by feeding it a fake/emulated display"* | `experiments/64-`, and §"read this first" |

## ⭐ The operator's three goals

> *"1. Make the 'universal' builder true via pgb + nix. 2. make the 'universal'
> bundler true via a modern, updated, maintained 'nixappimage' descendant ...
> and also solving the opengl problem ... 3. poc a kdenlive static (exhaust all
> resources), if impossible, pivot to kdenlive.nixappimage, but it must be
> smaller, load faster, run faster than pkgforge-dev/kdenlive-AppImage-Enhanced."*

| goal | entries | where it stands |
|---|---|---|
| 1. the builder | T-051, T-060, T-012 | ⛔ Blocked on **building nix's own closure static**, not on a tool |
| 2. the bundler | T-057, T-066, T-081 | ⭐ **GTK PROVEN: real windows on 11 of 11, zero host objects.** ⛔ Blocked on T-081 — a hardcoded store path stops one GUI app drawing and a script entry point stops Python bundling at all |
| 3. kdenlive | T-054, T-055 | ⭐ cold start 0.74×, host objects 0 of 11 against 4 of 11. ⛔ *smaller* not met; *run faster* unresolved |

## ⭐ Work order

    ---- ⭐ WHAT THE NEXT SESSION SHOULD DO, AND IT IS ONE ENTRY FIRST ----

    ⭐ T-081 IS CLOSED. Its acceptance test was named by the operator
       before the work and is met, twice, pass=11 fail=0 skip=0:

         arm G  galculator, UI at a compiled-in store path  11 of 11 draw
         arm N  the SAME bundle, --no-storefix               0 of 11 draw
         arm X  mousepad, the regression control            11 of 11 draw
         arm P  meld, a PYTHON GUI application              11 of 11 draw

       ⭐ Arm N is why arm G means anything, and arm P is the operator's
       own counter-example reached: meld produced NO ARTEFACT before.

    ⛔ T-080 IS REOPENED AND IS NOW THE WORK. The operator: "every
       capability listed in docs/research/bundle-capabilities.md
       including ones already measured, must be remeasured with 3
       applications per category in order of simple to complex".

       `experiments/65-` is the corpus: 26 subjects, three per category,
       plus four of the field's own thirteen nixappimage recipes. It is
       RESUMABLE — a row in evidence/65-capability-corpus/rows is never
       re-measured — so run it, and keep running it until every row of
       §0 carries a count out of eleven and the subject that produced it.

       ⛔ TWO ROWS CANNOT BE CLOSED ON THIS MACHINE AND MUST SAY SO IN
       THE SENTENCE: every GL and Vulkan row here is a SOFTWARE
       rasteriser, and NVIDIA is not bundled by design. T-059.

    ---- then, in order. ⛔ THE ORDER IS BY MECHANISM, NOT BY EASE ----

    ⭐ OPERATOR, 2026-09-04, and it decides the four entries below:
       "sort the tasks for next session based on completing/fixing what
       will auto fix/complete what, not easy first". The classification
       is docs/research/app-corpus.md and its EIGHT RUNGS are the task
       list; rungs 1-3 decide about twenty of the forty subjects.

    T-088  ⛔ RUNG 1. Multi-entry dispatch is SHIPPED and has NEVER been
        run. pgb-apprun.c is a static ARGV0/argv[0]/$1 selector and
        assemble.go installs every program in the entry bin/, so
        rnote/rnote-cli works by construction -- and no experiment has
        ever run a second program out of a bundle. One day, and it turns
        a source reading into a result.
    T-089  ⛔ RUNG 2. store-paths.md §3 marks ONE row NOT MEASURED: a
        static or raw-syscall payload has no PLT for the interposer to
        win. `syncthing` is that subject. PRE-REGISTER THE FAILURE.
    T-087  ⭐ RUNG 3 ONWARD: the battle-test corpus, 40+ applications.
        ⚠ THE MEASURE-TWICE RULE IS SUSPENDED FOR THIS ENTRY, by the
        operator; every other delivery rule holds.
    T-090  ⛔ RUNG 5 is a BED problem: unshare(CLONE_NEWUSER) is EPERM
        here, so a browser row measures --no-sandbox unless the bed
        changes. Do not merge "carries a browser" with "the sandbox
        works".
    T-084  ⛔ the trace classifier is SIX hand copies and one counted
        FAILED opens as loads. lib.sh's shared version needs the `mode`
        argument first -- it is not the deletion the entry first said.
        The competitor's "4 of 11" in comparison.md may be inflated.
    T-091  GStreamer: four variables and a scanner; bakedOverride emits
        one. Also decides whether a media subject's host-object count is
        measuring the app or the plugin scanner.
    T-059  a real GPU. ⛔ Every GL row is still swrast and surfaceless.
    T-066  ⛔ still the only open P0, and its remaining column is SIZE,
        which the operator struck on 2026-09-03c and deferred on
        2026-09-03d. ⚠ The priority ordering points here; this work
        order does not. This page decides.
    T-082  vendor + patch + drift detection. XL -- start early, finish late.
    T-083  desktop integration. ⭐ Its two named gaps are already closed
        (X-AppImage-Version and a dangling Icon=), and the bundle now
        carries a usr/ tree; what is left is the managers themselves.

    ---- then the builder, by how foundational ----

    T-060  rungs 1→3, the static nix. THE TARGET: `nix-cli-static`.
    T-051  a published (musl) static nix serves "enough nix on a minimal host".
    T-012  items 2, 3 and 4. ⛔ ITEM 1, the git/URL route, is DEFERRED.

    ---- then kdenlive ----

    T-054  rung 3 then rung 4.    T-063  arm S: src/interfaces.

    ---- P2 by category ----

    T-013  T-015  T-021  T-031  T-041

## ⭐ Delivery rules — EIGHT, and rule 3 is suspended for T-087

⚠ **Five came from the session that wrote T-078/079/080. Rules 6, 7 and 8 were
each paid for by a discarded run.**

⛔ **RULE 3 IS SUSPENDED FOR T-087 ONLY**, by the operator, 2026-09-04:
*"Use cached/prebuilt fetches to make the install/build fast, and get rid of
the measure twice rule; we need to cover more cases for now, we can refine
later."* Every other rule holds everywhere, T-087 included.

    1. ⛔ PRE-REGISTER the expectation before the run, and COMMIT it
       before the run so the git log shows it was not written after.
       Done three times this session; TWO predictions were falsified and
       both are recorded rather than rewritten.
    2. ⛔ A SKIP IS NOT A PASS. Read the skip count on every run.
    3. ⛔ TWO RUNS OR IT IS NOT A NUMBER. This caught a counter that
       moved 0 -> 7 between two runs with no change to the subject.
    4. ⛔ AN ABSENCE IS NOT A ZERO. Say where you looked, AND where you
       could not: T-079's search cannot see runtime-assembled paths or
       other libraries' host data, and says so.
    5. ⛔ VERIFY YOUR OWN WRITE-UP AGAINST THE SOURCE. Two claims failed
       this check this session — "SIGABRT on 3" was SIGABRT on two and
       SIGFPE on one, and a quoted figure came from a run whose
       RESULT.txt the next run had overwritten.
    6. ⛔ CHECK THAT YOUR SUCCESS CRITERION CAN FAIL FOR THE RIGHT
       REASON. `Gtk-WARNING: cannot open display` is emitted by the
       BUNDLED library, so it looked like proof the bundle worked -- and
       it prints on real hardware WITH a display too. An instrument that
       cannot tell a working subject from a broken one is worse than no
       instrument, because it reports green.
    7. ⛔ AND CHECK THAT IT CAN FINISH. `strace` on a program reading
       through a FUSE mount deadlocks in state D; `kill` cannot end a
       process in D, so `wait` never returns. The fix was an ORDERING --
       reap the FUSE daemon BEFORE waiting -- and it also made every row
       ten times faster, which means the slow version had been paying
       the same cost in a milder form all along.
    8. ⭐ **NEW, 2026-09-04 — CARRY A POSITIVE CONTROL, AND NEVER CARRY A
       CONSTANT ACROSS A CHANGE OF MODE.** experiments/65- scored
       galculator 0 of 11 on a subject experiments/64- had measured at
       11 of 11 twice, because it copied 64-'s 25-second window budget
       while switching every subject from MOUNT to EXTRACT delivery --
       and 64- itself uses 150s for the one arm it extracts.
       ⛔ The budget was the symptom. THE DEFECT WAS THAT NOTHING IN THE
       EXPERIMENT COULD TELL A BROKEN SUBJECT FROM A BROKEN INSTRUMENT:
       five pre-registered expectations and not one control.
       corrections.md C26.

## ⭐ THE OPERATOR'S FOUR QUESTIONS, 2026-09-04 — ANSWERED HERE

**1. Has the GLIBC-STATIC work really fully completed?** ⛔ **No, and the
number is countable.** `REQUIREMENTS.md` enumerates **eleven** issues; **ten
are closed** on all eleven environments and **one is open** — `dlopen` of a
HOST shared object, which stays open because the row says *host-dependent* and
it still is. ⚠ Three further reasons not to call it complete:
  - closing the eleventh exposed a **boundary inside it**: `getaddrinfo` with a
    service name is 8 of 11 and no flag closes it (T-085);
  - four of the ten are closed by an **opt-in flag** a developer must pass, not
    by the default build;
  - ⛔ the list went **nine → ten → eleven on three consecutive days**, each
    time because somebody searched rather than assuming. "Ten of eleven" is a
    snapshot of a search, never a distance to done.

**2. Multiple binaries, busybox-style — and does renaming or symlinking the
bundle work?** ⭐ **Yes, by construction, and it is unmeasured.** `assemble.go`
installs the entry point and then **every other non-dot program in the same
store path's `bin/`**; `--with-program NAME` adds one found anywhere in the
closure. `tool/runtime/pgb-apprun.c` is a **static** selector — no shell, no
host interpreter — dispatching on `ARGV0` (what uruntime sets), then `argv[0]`'s
basename, then `$1`, then the default. So a symlink or a rename selects the
program exactly as an AppImage does, and `./app.AppImage rnote-cli` works too.
⛔ **No experiment has ever run a second program out of a bundle.** T-088.

**3. Can the walker say how many entry points an app has?** ⭐ **Yes, and it
already prints it**: the build log line `programs <prog> + N more`, and
`--name X` failing lists what IS in `bin/`. That count is the multi-call
dispatch table — the same set the selector chooses from. ⚠ It counts the entry
store path's `bin/` only, so a helper in a dependency needs `--with-program`
and is not in the number. T-088 puts the count in the corpus table.

**4. Feature parity with anylinux AppImages?** ⚠ **No — close on mechanism,
behind on features, and the list is theirs not mine.** Level or ahead:
coverage 11/11 both, payload clean 11/11 both, a static delivery path (no shell
in ours), nothing written to the filesystem, `dlopen` of our own plugins, the
compiled-in store path (T-081), a Python GUI (T-081), the seven host-data
dependencies. ⭐ **And one row that is ours outright**: they run `strace` to
guess a dependency set (`STRACE_MODE=1`); we are handed the exact one the
derivation declared. ⛔ **Where they are ahead**, read off their own
`HOW-TO-MAKE-THESE.md` variable list rather than inferred:
  - **the sandbox** — a namespaces hook and shipped browsers; we cannot even
    measure it in this bed (T-090);
  - ⭐ **`OPTIMIZE_LAUNCH`, a DWARFS *profile* image** — PGO for the mount. We
    have `lite` and `-S18`, which are different levers. A named cold-start
    lever we have never tried (T-066);
  - **four capability-check hooks** (`x86-64-v3`/`v4`, `vulkan`, `wayland`)
    that print a message where ours crashes silently;
  - `GTK_CLASS_FIX` (the taskbar groups the window with its icon),
    `self-updater`, `udev-installer`, and `QUICK_SHARUN_SKIP_DEPS_FOR`
    (skip a dependency subtree **by name**, where `--debloat` is coarser);
  - **breadth** — hundreds of applications and a graded per-toolkit record
    against our 26-subject corpus.
  ⚠ Not parity questions: size and speed, which the operator has deferred.
  ⭐ The full table is [`../docs/research/app-corpus.md`](../docs/research/app-corpus.md).

## Open questions for the operator

⭐ **None blocking.**

1. ⚠ **A GPU** — **T-059**. Every GL row is `swrast` and surfaceless.
   T-080's guarantee claims *"the closure produces a working EGL display
   offscreen"* and explicitly does **not** claim Vulkan or NVIDIA.
2. ⚠ **kdenlive's warm row is 3.45× against us and unexplained.** First
   candidate: at 565 MB it is over uruntime's 350 MB `MAX_EXTRACT_SELF_SIZE`,
   so it **extracts** where `jq` **mounts**.
3. ⚠ **Docker Hub rate-limits anonymous pulls here.** `pgb rootfs pull` does
   the anonymous-token dance and succeeds where `docker pull` 429s.
