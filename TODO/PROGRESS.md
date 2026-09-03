# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log
and the entries.

    STATE     2026-09-02f / 2026-09-03
    COUNTS    45 entries, 20 open, 25 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, TEN POCs
              CI: GREEN before this session; selftests 239 pass, 1 could not
              run (no zstd)
              throughput: glibc 4.53 ns/op vs musl 584.71 (malloc, 4 threads)
    NEW       ⭐ THE GLIBC PIN MOVED — 2.36 → 2.41, gcc 12.2.0 → 14.2.0, all
              four measured costs zero, class B 20 → 5 distinct symbols.
              ⭐ TWO REAL LOADER DEFECTS FOUND AND FIXED, and the first was
              HIDING the second.

## ⛔ READ THIS FIRST: the toolchain is Go now, and the shell is the oracle

⭐ **`pgb` is one statically linked Go binary.** The driver, the compiler
wrappers, the planner, the verifier and the bundler are the same executable,
built `CGO_ENABLED=0`, carrying the C runtime sources it compiles. The shell
and Python it replaced are under `HISTORY/<commit>/`, unedited, and are the
oracle every gate is measured against.

**Read [`toolchain.md`](toolchain.md) §T-061 for what was required, then
[`../docs/design/toolchain.md`](../docs/design/toolchain.md) "Language and
structure" for the decision, the architecture and the six gates.**

⛔ **The experiments and the POCs stay shell** and are the acceptance harness.
An entry below that proposes editing shell under `tool/` or `scripts/` is
describing files that no longer exist; check `HISTORY/` before acting on it.

## ⭐ The operator's three goals

Quoted, because the framing is load-bearing:

> *"1. Make the 'universal' builder true via pgb + nix. 2. make the 'universal'
> bundler true via a modern, updated, maintained 'nixappimage' descendant that
> uses or rather reimplements many of the anylinux tooling, iterating/improving
> them, and debloating nixappimages, correctly packing them, and also solving
> the opengl problem ... 3. poc a kdenlive static (exhaust all resources), if
> impossible, pivot to kdenlive.nixappimage, but it must be smaller, load
> faster, run faster than pkgforge-dev/kdenlive-AppImage-Enhanced."*

| goal | entries | where it stands |
|---|---|---|
| 1. the builder | T-050 ✅, T-051, T-060, T-012 | unchanged this session |
| 2. the bundler | T-057 ⚠, T-052 ✅, T-053 ✅, T-066, T-071 | unchanged this session; ⛔ **the size gap is structural** |
| 3. kdenlive | T-054, T-055 | unchanged this session. ⛔ **The bar is NOT met**: 2.22× the size |

⚠ **This session worked the top of the work order, which is glibc's remaining
quirks and future-proofing — not the three goals.** That is the ordering
`INDEX.md` argues for and it is deliberate.

## What this session did

### 1. ⭐ THE PIN MOVED, and the landing was three steps because the pin is nine constants

    DefaultEnvImage   debian:12  ->  debian:13
    DefaultEnvDigest  2f65600e…  ->  6788062a…
    DefaultEnvName    pgb-env-debian12 -> pgb-env-debian13

⛔ **Changing `cfg.go` alone would have changed nothing measurable.** The name
had **nine** copies in code and the digest **two** more in CI:

| where | what it would have done |
|---|---|
| `experiments/` 60- 61- 62- 70- 73- 80- 87- 88- | gone from disk they SKIP (exit 2, which nobody reads as a regression); still on disk they **measure the old glibc and say nothing** |
| ⛔ `.github/workflows/portability.yml`, twice | an `env.BUILD_IMAGE` **nothing read** (the `env` context is unavailable in a job's `container.image`) plus a retyped digest |

⭐ **And two of the nine copies could never have matched**: `experiments/80-`
looked for `$ROOTFS_DIR/debian12` and `$ROOTFS_DIR/alpine322`, and the local
names are `debian-12` and `alpine-3.22`.

`experiments/lib.sh` derives the name from `cfg.go`; the `matrix` CI job derives
the image; `TODO/check.sh` fails on a copy of any of the three, with a control
that puts one back.

### 2. ⭐ The matrices at 2.41, and 21- now follows the pin

`experiments/73-` **independently reproduced** what arm 4 predicted from a
candidate environment:

    ENVIRONMENT          SERVED @2.36 -> @2.41   class B
    opensuse-leap-15.6   993  -> 1005            13 -> 0
    fedora-42            961  ->  976            15 -> 0
    archlinux-latest    1198  -> 1213            20 -> 5
    debian-12            851  ->  849   ⚠ the one cost
    class C empty on all 11 at BOTH pins    class E empty, both pins

⭐ **`experiments/21-` gained a third arm that follows `cfg.go`.** It hardcoded
"2.31" and "2.36" as labels and built from two *target* rootfs, so it would
have gone on printing 2.36 forever. Every label is now `ldd --version` read out
of the environment that produced the arm.

⛔ **And a defect I introduced, caught by running it**: writing the probe to
`/tmp` inside a rootfs gave "compile failed" on all three arms with empty logs.
`pgb rootfs run` **mounts a fresh tmpfs over `/tmp`**.

### 3. ⭐ TWO REAL LOADER DEFECTS, and the first was hiding the second

**1. `--host-dlopen` could not load anything that used iconv.** The provider
table declares glibc symbols as **weak** undefined references; `--wrap` rewrote
`iconv_open` to `__wrap_iconv_open`; **a weak undefined reference does not pull
a member out of an archive**. Controlled, same machine, same 1527 objects:

    ok = 406  ->  620

**2. ⛔ The two halves of a general-dynamic TLS pair disagreed, silently.**
`DTPMOD64` searched every loaded object, `DTPOFF64` searched only one, so a
cross-module thread-local came out as *(the right module, offset 0)*. ⚠ **Not
primarily a crash — one module reading and WRITING at offset 0 of another
module's thread storage.**

⭐ **The seven LLVM-family objects went from SIGSEGV 7 of 7 to loaded 7 of 7.**

⛔ **And `experiments/93-`'s control had been passing at DIFFER=0 partly
BECAUSE of defect 1** — the ten objects it should have caught were failing
earlier, on iconv, and never reached the code that crashes them.

### 4. ⚠ T-072's motivating object is refused for a different reason

Every host `.so` was read for a `PT_TLS`; the 71 that have one were run at
reserve 0 and 65536. ⛔ **`liblsan.so.0.0.0` is the 56,248-byte object this
entry was opened on, and it is a sanitizer interposer the loader declines BY
NAME before TLS is considered.** Objects of 71 reporting "static TLS surplus
exhausted" at reserve 0: **zero**. The mechanism stays; its justification now
rests on the synthetic subject alone, and that is recorded.

### 5. ⛔ Findings, and not one came from reading code

1. **`main` came up 18 commits behind** and `git switch` said "up to date" —
   on a shallow clone, before the fetch.
2. **`PROGRESS.md` was 12 commits stale and `TODO/runtime.md` said
   "93- has not been RUN yet"** twelve commits after it was run twice.
3. **The stamp guard caught a stale docker environment** on the first real pin
   move — and its remedy line named the wrong engine's fix.
4. **`pgb rootfs run` mounts a tmpfs over `/tmp`.**
5. **`docs/research/solo.md`'s "5,807 objects" disagreed with the table
   printed beneath it**, by 200; and its "90.8%–97.8%" range missed
   opensuse's 97.9%, so it was wrong twice.
6. ⛔ **A `$?` after a pipeline is the pipeline's status.** A sweep reported
   `ok=1527 fail=0 crash=0` over a population containing 96 files that are not
   ELF. Caught because the number was implausible, not because anything
   checked it.
7. **`chmod 000` is not a control when you are root** — an unreadable-file arm
   passed until the file was moved away instead.

## ⭐ Work order

    ---- glibc's remaining quirks, and future-proofing ----

    T-070   ⚠ P0. Steps 1 and 2 LANDED; step 3 is nearly done — 73-, 21- and
            six POCs re-run at 2.41 and committed. ⛔ 80-mlt at 2.41 through
            the normal POC path is what is left.
    T-068   P1. ⭐ Two defects fixed. ⛔ experiments/93- re-run with BOTH is
            the row that closes it; the control must be read, not assumed.
    T-072   P1. --tls-reserve is implemented. ⛔ experiments/76- with a
            non-zero reserve on the eleven is still owed.

    ---- the bundle, and the one class that is all DATA ----

    T-071   ⚠ P0. Items 1, 2, 5 done. ⛔ experiments/85-'s new data-coherence
            arm is WRITTEN AND NOT RUN -- that is this entry's Prove.
    T-066   ⚠ P0. ⭐ The corpus IS MINED (34 trees). ⛔ An allowlist cannot
            reach a DT_NEEDED edge a `-mini` rebuild deletes. Measure its
            ceiling first (route A in the entry). Needs an AppDir.

    ---- then, unchanged in relative order ----

    T-063   the miniflux proof: arm S has a static postgres on Alpine;
            src/interfaces does not build
    T-062   ⭐ `internal/wrapper` NOW HAS ONE (24 cases). Seven packages left
    T-060   rungs 2 and 3, the static nix
    T-054   rungs 3 (KF6) and 4 (kdenlive static)
    T-057   item 2: a 32-bit application through the lib32 path
    T-051   the no-compiler host
    T-012   pgb build <url-or-package>
    then    P2 by category

⭐ **Two pieces of real work are NAMED and are not entries**, because each is
one clear fix inside T-063 arm S:

    the static link-order problem   AC_SEARCH_LIBS probes -lreadline alone and
                                    libreadline.a's ncurses references go
                                    unresolved. poc/91-qt-xcb answered the same
                                    class with -Wl,--start-group
    a C link that pulled a C++      libicuuc.a needs operator delete and the
    archive                         __cxxabiv1 vtables; LinkFlags already takes
                                    a `cxx bool` and does not notice this case

## Open questions for the operator

⭐ **None blocking.**

1. ⚠ **Two branches exist on the remote that no session created deliberately**
   — `claude/glibc-kdenlive-validation-2x7c3c` and
   `claude/glibc-research-foundations-7pjoqe`, both named by the harness.
   `RULES.md` §Git outranks the harness and every commit is on `main`. The git
   proxy refuses remote deletes, so they are left for a human to remove in the
   web UI.
2. ⚠ **A GPU** — **T-059**, not a question. Every GL row is `swrast`, and
   T-071 item 4's second half cannot be settled without one.
3. ⚠ **Docker Hub rate-limits anonymous pulls in this environment.**
   ⭐ `pgb rootfs pull` does the anonymous-token dance and succeeds where
   `docker pull` 429s; ⭐ `docker buildx imagetools inspect` reads metadata
   without pulling and answered for every tag this session.
4. ⚠ **`musl-gcc` is absent**, which is the one remaining blocker on
   `experiments/90-`'s arm O.
5. ⚠ **`--tls-reserve` now costs ~1.15 MB on EVERY `--host-dlopen` build**,
   because the iconv wrapper has to be forced out of its archive whether or not
   the program calls iconv. Measured: 4,575,160 → 5,725,864 bytes. A narrower
   fix would need the provider table to reference the wrapper strongly for
   those three names only; it is not obviously worth the machinery.
