# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log,
the entries, and [`../HISTORY/`](../HISTORY/).

    STATE     2026-09-03e  ✅ COMPLETE
    COUNTS    57 entries, 19 open, 38 done
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

    ⭐ T-081 IS NOW THE WORK, and it is no longer speculative — this
       session MEASURED what it costs, with a positive control:

         arm G  galculator, UI at a compiled-in store path   0 of 11 draw
         arm X  mousepad,  UI compiled in as a GResource    11 of 11 draw
         arm C  galculator AGAIN with the path resolving    11 of 11 draw

       ⭐ Arm C is the argument: identical artefact, one variable, and it
       draws. So "a hardcoded store path is what stops it" is a
       measurement rather than a reading of an error message.

       TWO BLOCKERS, BOTH IN T-081's ENTRY WITH A ROUTE EACH:
         1. absolute store paths compiled into .rodata. ⚠ A rewrite
            cannot LENGTHEN the string — but `/nix/store/` is 11 bytes
            and so is `/tmp/.pgbs/`. ⛔ Answer the security question
            before building it: a fixed path under a world-writable /tmp
            is a symlink-attack surface.
         2. script entry points. resolveEntry oscillates between a
            makeBinaryWrapper ELF and the Python script it targets, so
            NO Python GUI app bundles at all. That is the standard
            nixpkgs shape, not a meld quirk.

       ⛔ ITS ACCEPTANCE TEST ALREADY EXISTS: `experiments/64-` arm G
       must go 0 of 11 -> 11 of 11 WITHOUT the bind arm C uses.

    ---- then, in order ----

    T-079's residue  ⛔ THE ELEVENTH ROW HAS NO MECHANISM. /etc/services
        and /etc/protocols: getservbyname returns NULL on 3 of 11, all
        glibc. The precedent for a fix is --embed-tzdata: look first,
        carry a fallback, never prefer the stale copy.
    T-059  a real GPU. ⛔ Every GL row is still swrast and surfaceless,
        and T-080's guarantee says so in its own sentence rather than in
        a footnote. Vulkan and NVIDIA are NOT measured.
    T-066  ⛔ still the only open P0, and its remaining column is SIZE,
        which the operator struck on 2026-09-03c and deferred on
        2026-09-03d. ⚠ The priority ordering points here; this work
        order does not. This page decides.
    T-082  vendor + patch + drift detection. XL — start early, finish late.
    T-083  desktop integration. Depends on T-081.

    ---- then the builder, by how foundational ----

    T-060  rungs 1→3, the static nix. THE TARGET: `nix-cli-static`.
    T-051  a published (musl) static nix serves "enough nix on a minimal host".
    T-012  items 2, 3 and 4. ⛔ ITEM 1, the git/URL route, is DEFERRED.

    ---- then kdenlive ----

    T-054  rung 3 then rung 4.    T-063  arm S: src/interfaces.

    ---- P2 by category ----

    T-013  T-015  T-021  T-031  T-041

## ⭐ Delivery rules — kept, and one is NEW

⚠ **Five came from the session that wrote T-078/079/080. The sixth is from
this one, and it cost eleven green rows.**

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
    6. ⭐ **NEW — CHECK THAT YOUR SUCCESS CRITERION CAN FAIL FOR THE
       RIGHT REASON.** `Gtk-WARNING: cannot open display` is emitted by
       the bundled library, so it looked like proof the bundle worked.
       It is printed on real hardware WITH a display too. ⛔ An
       instrument that cannot tell a working subject from a broken one
       is worse than no instrument, because it reports green.

## Open questions for the operator

⭐ **None blocking.**

1. ⚠ **A GPU** — **T-059**. Every GL row is `swrast` and surfaceless.
   T-080's guarantee claims *"the closure produces a working EGL display
   offscreen"* and explicitly does **not** claim Vulkan or NVIDIA.
2. ⛔ **The `/tmp/.pgbs/` route for T-081 has a security question**, and it
   should be answered before it is built rather than after: a fixed,
   predictable path under a world-writable directory is a symlink-attack
   surface. The same-length property that makes it attractive
   (`/nix/store/` and `/tmp/.pgbs/` are both 11 bytes) does not make it safe.
3. ⚠ **kdenlive's warm row is 3.45× against us and unexplained.** First
   candidate: at 565 MB it is over uruntime's 350 MB `MAX_EXTRACT_SELF_SIZE`,
   so it **extracts** where `jq` **mounts**.
4. ⚠ **Docker Hub rate-limits anonymous pulls here.** `pgb rootfs pull` does
   the anonymous-token dance and succeeds where `docker pull` 429s.
