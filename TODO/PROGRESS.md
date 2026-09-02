# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log
and the entries.

    STATE     2026-09-02c
    COUNTS    45 entries, 20 open, 25 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, TEN POCs
              CI: GREEN; selftests 138 pass, 1 could not run (no zstd)
              throughput: glibc 4.53 ns/op vs musl 584.71 (malloc, 4 threads)
    NEW       ⭐ GATE 5 IS COMPLETE — ten of ten POCs and twenty-three
              experiments, every row measured. T-061's last gap is closed.

## ⛔ READ THIS FIRST: the toolchain is Go now, and the shell is the oracle

⭐ **`pgb` is one statically linked Go binary.** The driver, the compiler
wrappers, the planner, the verifier and the bundler are the same executable,
built `CGO_ENABLED=0`, carrying the C runtime sources it compiles. The shell
and Python it replaced are under `HISTORY/<commit>/`, unedited, and are the
oracle every gate is measured against.

**Read [`toolchain.md`](toolchain.md) §T-061 for what was required, then
[`../docs/design/toolchain.md`](../docs/design/toolchain.md) "Language and
structure" for the decision, the architecture and the six gates.** The
measurements are in `evidence/92-go-port/RESULT.txt`, per row, as they landed.

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
| 1. the builder | T-050 ✅, T-051, T-060, T-012 | ⭐ **T-050 CLOSED**: `experiments/88-` plans, fetches **and builds** a nixpkgs package with no nix and no root, 25 assertions. ⚠ It still needs a C toolchain on the host, which is T-051; **T-060** is the static-glibc nix that removes the last crutch and is **rung 1 of 3**, 31 of nix's dependencies built |
| 2. the bundler | T-057 ⚠started, T-052 ✅, T-053 ✅ | ⭐ **items 1, 3 and 4 landed**: debloating with a three-arm control (`experiments/89-`), wrapper environments lifted into `.env`, and the lib32 path. ⛔ **item 2, a 32-bit application, is still untried** |
| 3. kdenlive | T-054, T-055 | ⭐ **rung 2 climbed**: `poc/91-qt-xcb` — a static Qt 6 opening a **real xcb window**, 26 assertions, 11/11, zero host objects. ⛔ **T-055's bar is NOT met**: ours 395,294,317 B against 191,900,604 B |

## What this session did (2026-09-02c)

⭐ **THREE OF THE FOUR P0s ARE CLOSED. T-066 is significantly advanced and
stays open**, because 1.22× the field is not parity.

### 1. ⭐ T-064 — static glibc's `dlopen`, REALLY solved

`tool/runtime/pgb-elfload.c` is an ELF loader compiled INTO the binary;
`pgb build --host-dlopen` turns it on. It maps the object, walks `DT_NEEDED`,
relocates (`DT_RELA` **and** `DT_RELR`), honours symbol versioning, places
initial-exec TLS in glibc's own reserved surplus, runs the initialisers, and
binds every undefined symbol to the static glibc already in the executable. A
`DT_NEEDED` naming a library the image already contains is **answered, not
opened**.

`experiments/76-`, exit 0, four of four:

    TARGET               LIBC   CARRIED  NATIVE  CONTROL  HOST .so LOADED
    alpine-3.22          musl   ok       exit1   exit1    none
    alpine-3.20          musl   ok       exit1   exit1    none
    alpine-3.10          musl   ok       exit1   exit1    none
    voidlinux-musl       musl   ok       exit1   exit1    none
    debian-11            glibc  ok       ok      SIG6     none
    debian-12            glibc  ok       ok      SIG11    none
    ubuntu-20.04         glibc  ok       ok      SIG6     none
    rockylinux-8         glibc  ok       ok      SIG6     none
    opensuse-leap-15.6   glibc  ok       ok      SIG8     none
    fedora-42            glibc  ok       ok      SIG8     none
    archlinux-latest     glibc  ok       ok      SIG11    none

      carried: nine assertions, every environment  = 11 of 11
      carried: loaded no host shared object        = 11 of 11
      native:  a REAL host .so on every glibc row  =  7 of 7
      native:  refuses CLEANLY on musl, no signal  =  4 of 4
      control: ran                                 =  0 of 11

⭐ **On the four musl rows that is a GLIBC shared object being `dlopen`'d on a
machine that ships no glibc**, from one ordinary static ELF —
`PT_INTERP=0 DT_NEEDED=0` — with nothing beside it. ⭐ **1,093 code lines
against `pg83/solo`'s 2,332** for the loader alone; solo's other 5,948
translate glibc onto musl and a glibc host needs none of them.

⛔ **The musl refusal is the mechanism working, not a gap**: every object there
carries `DT_NEEDED libc.musl-x86_64.so.1`, and musl's libc IS its loader.

### 2. ⭐ T-065 — what a bundle may take from the HOST

[`../docs/design/host-fallback.md`](../docs/design/host-fallback.md) is the
write-up; `internal/bundle/hostpolicy.go` the mechanism; 29 offline selftest
cases the assertion. ⛔ **"Zero host objects" is right for a static ELF and
wrong for a bundle**, and the difference had never been written down. Four
classes, search order adopted from `Anylinux-sharun` rather than invented, and
**NVIDIA is host-always and not an opt-in** — its driver links a 10+ year old
glibc and it cannot be bundled at all.

### 3. ⚠ T-066 — 2.86× the field to 1.22×, on `jq`

`experiments/78-` is the harness and the subject is a CLI, which is what made
four measured iterations fit where one kdenlive build goes. Two levers, both
structural:

    the reachability sweep NOTHING consumed   277 objects, 12.0 MiB
    share/i18n, glibc's locale SOURCES        15.0 MiB of a 22 MiB bundle

Debloat went from 12.7% off to 86.9% off; the bundle from 11,471,610 B to
4,890,913 B against the field's 4,006,916.

### 4. ⭐ T-067 — C is adequate, and the defect log is the argument

[`../docs/design/runtime-language.md`](../docs/design/runtime-language.md).
0 UBSan findings running the loader over **904 real host shared objects**; the
5 gcc warnings are one false positive (`-Waddress` does not model weak
linkage). zig is **not packaged** in the pinned `debian:12` and would be a
53,733,924 B fetch and a *second* toolchain. ⭐ And the seven defects below are
the real argument: **not one is a C-language defect.**

### 5. ⛔ Seven defects, every one found by something disagreeing

1. **`libm.a` is a GNU ld script, not an archive.** Read as `ar` it yields zero
   symbols in silence; the provider table had 4,891 names instead of 7,216.
   ⚠ Second time this trap has fired in this tree.
2. **`__tls_get_addr` is in no archive** — `ld.so` exports it. 398 of 492
   undefined-symbol failures were that one name.
3. ⛔ **`DT_RELR` was ignored.** Fedora and Arch pack relative relocations into
   a bitmap, so the loader "succeeded" and left pointers unrelocated — a
   **silent wrong answer**, caught only because a constructor was called
   through one: `init_array[0] 0x670`.
4. ⛔ **`make` did not depend on the `go:embed`'d C**, so editing the loader
   printed "Nothing to be done" and the next build used the PREVIOUS loader.
   It cost a full eleven-environment run. Fixed in the `Makefile`.
5. ⛔ **My own benchmark forked per sample** and reported the loader 10×
   slower than `ld.so`; that was copy-on-write faults on a 4.4 MB static image.
6. ⛔ **The reachability sweep had no consumer** — `codegraph callers Sweep`.
7. ⛔ **The sweep then ran before `.env` existed** and deleted kdenlive's MLT
   modules. ⭐ `jq` did not catch this and COULD NOT: a CLI with no plugin
   directories has nothing at risk. The fast subject is for iterating; the
   plugin-heavy one is the control.

### 6. T-068 opened, so the residue is carried rather than rounded off

86 of 904 host objects do not load. 30 crash and almost all are objects no
static image should load — NSS modules, sanitizer and allocator interposers.
`../docs/limitations.md` §1 classifies every one.

## ⭐ Work order

⛔ **REORDERED 2026-09-02c on the operator's instruction**: *"focus on solving
glibc's remaining quirks, ensure future version won't break our tooling or
binary built by your tooling"*, plus a dedicated EGL/nix entry.

⭐ **The reordering has an argument, not just a preference.** Three of the four
2026-09-02b P0s are closed. What remains splits into two kinds of work, and one
of them **degrades on its own**:

| | |
|---|---|
| ⛔ **time-sensitive** | the glibc **class B ceiling widens with every glibc release the pin does not follow**. Nothing else here gets worse by being left alone. That is why it goes first |
| everything else | as expensive next year as this year |

    ---- glibc's remaining quirks, and future-proofing ----

    T-070   ⛔ THE PIN. It is a FLOOR set to 2.36 when the floor is 2.34, and
            because the output is STATIC there is no upward pressure at all --
            the usual "build old for compatibility" reason does not apply
            here and docs/design/glibc-versions.md is why. Meanwhile class B
            is 20 symbols, 14 of them __isoc23_* at exactly GLIBC_2.38, and
            it grows. ⛔ Measure the cost FIRST: the kernel floor a newer
            glibc declares is the thing a move can take away.
    T-068   the --host-dlopen residue, now 86 of 904 with the crashes down
            from 30 to 10. ⭐ The 10 are ONE family -- large C++ libraries
            with hundreds of static constructors -- and libLLVM is the
            specimen: it maps and relocates cleanly and dies in the 605th.
    T-072   the static TLS surplus: 3,176 bytes of headroom, and one real
            library wants 56,248. A glibc quirk with a named tunable and
            three untried routes.

    ---- the bundle, and the one class that is all DATA ----

    T-071   ⛔ EGL from a nixpkgs closure. FOUR distinct failures so far and
            every one of them was in DATA rather than code -- a missing
            package, a flattened directory, an absolute store path inside a
            third-party JSON, and a vendor library the reachability sweep
            could not see. ⚠ The fourth was caught before it shipped on
            2026-09-02c; the first three each cost a run.
    T-066   ⚠ the bundler, still open. 2.86x -> 1.22x on jq. ⛔ kdenlive is
            the outstanding row. The remaining gap is WHERE THE CLOSURE
            COMES FROM, not another debloat rule.

    ---- then, unchanged in relative order ----

    T-063   the miniflux proof: arm S has a static postgres running on
            Alpine; src/interfaces does not build
    T-062   eight packages carry no selftest
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

1. ⚠ **One branch exists on the remote and this session did not create it.**
   The harness named `claude/glibc-pgb-recovery-6dleai`; `RULES.md` §Git
   outranks it and every commit is on `main`. The branch was already on the
   remote at `main`'s commit when this session started and the git proxy
   refuses remote deletes, so it is left for a human to remove in the web UI.
2. ⚠ **A GPU** — **T-059**, not a question. Every GL row is `swrast`.
3. ⭐ **The porting report is gone, as the operator asked.** Its content is
   in `docs/design/toolchain.md` "Language and structure" and T-061 deleted
   the file.
