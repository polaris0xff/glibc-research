# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log
and the entries.

    STATE     2026-09-02d
    COUNTS    45 entries, 20 open, 25 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, TEN POCs
              CI: GREEN; selftests 200 pass, 1 could not run (no zstd)
              throughput: glibc 4.53 ns/op vs musl 584.71 (malloc, 4 threads)
    NEW       ⭐ KDENLIVE RENDERS ON 11 OF 11, at `safe` AND at `aggressive`.
              Five runs in, the comparison the operator asked for exists.
              ⛔ And its bar is NOT met: 2.22x the size.

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
| 2. the bundler | T-057 ⚠, T-052 ✅, T-053 ✅, T-066, T-071 | ⭐ the sweep is proved on a plugin-heavy subject; ⛔ **the size gap is structural**, see below |
| 3. kdenlive | T-054, T-055 | ⭐ **the comparison exists and is honest**: 11/11 render, 11/11 zero host objects against 4/11. ⛔ **The bar is NOT met**: 2.22× the size |

## What this session did (2026-09-02d)

### 1. ⭐ kdenlive, validated — and the bar, still not met

⛔ **Runs 1–4 each failed for a different reason and run 4's cause was the
previous agent's own kill.** Run 5 (`safe`) and run 6 (`aggressive`) both
completed:

| | ours | kdenlive-AppImage-Enhanced |
|---|---|---|
| rendered a real MP4 | **11 of 11** | 11 of 11 |
| zero host shared objects | ⭐ **11 of 11** | 4 of 11; **10 on rockylinux-8** |
| size, `safe` | 471,033,944 B | 191,900,604 B — **2.45×** |
| size, `aggressive` | 426,528,098 B | — **2.22×** |

⛔ **THE THREE SWEEP FIXES ARE PROVED.** At `aggressive`, `DropUnreachable`
deleted **1,712 objects, 227.4 MiB**, and `melt` still rendered **4,149
bytes** — byte-for-byte what `safe` produced. MLT's modules and
`libSDL3.so.0` survived a sweep that removed 1,712 objects around them, which
is exactly what runs 1 and 3 died on.

⚠ **Run 6's render and startup MILLISECONDS are contaminated**, by this session
running builds and selftests during a wall-clock arm. The control that exposed
it: the *competitor's fixed artefact* moved 2,033 → 13,680 ms. `RULES.md` gains
the rule. ⛔ **A same-day `safe` vs `aggressive` timing comparison has NOT been
done.**

### 2. ⚠ T-070 — three of four costs measured at zero, and the pin has not moved

`experiments/91-glibc-pin-candidates.sh`, new. The cheap veto runs first so
nothing expensive is spent on a candidate that fails it.

    image           glibc  gcc      .note.ABI-tag  file(1)
    debian:12       2.36   12.2.0   3.2.0          for GNU/Linux 3.2.0
    debian:trixie   2.41   14.2.0   3.2.0          for GNU/Linux 3.2.0

    ENVIRONMENT            B@2.36   B@2.41   SERVED
    opensuse-leap-15.6     13       0        993  -> 1005
    fedora-42              15       0        961  -> 976
    archlinux-latest       20       5        1198 -> 1213
    debian-12              0        0        851  -> 849   ⚠ the one cost
    class B distinct   20 -> 5      class C  empty at BOTH pins, all 11 rows

⭐ The `__isoc23_*` family at `GLIBC_2.38` is gone; the five left are at
`GLIBC_2.42`/`2.43` on the one rolling distribution. The NSS floor holds at
2.41, with `experiments/21-`'s below-floor arm firing as the control.

⛔ **`cfg.go` is UNTOUCHED. The ten POCs at 2.41 are the row that stands
between "indicated" and "measured"**, and gcc goes 12.2.0 → 14.2.0 with the
pin, which is the larger of the two changes.

### 3. ⭐ T-066 — the gap is the direction the pipeline runs in

Read out of the competitor's own 89 lines: they `pacman -Syu` twelve
hand-picked packages, swap in **debloated rebuilds** of the heavy ones, remove
one explicitly, and `quick-sharun` walks the closure of ~20 **named** paths.
We start from nixpkgs' complete 2.53 GiB closure and subtract.

⛔ **Subtractive cannot win against additive**, because `sweep.go`'s own rule
is *"anything a rule cannot classify counts as REACHABLE"*.

⭐ **And the arithmetic agrees independently:** `aggressive` removes **319.6
MiB** of AppDir — 92.2 of extra debloat plus 227.4 of sweep — for **42.4 MiB of
artefact**. That is **7.5 to 1**, because dwarfs at `zstd:19` was already
compressing what got deleted. ⛔ Closing 426 MB → 192 MB by deletion alone
would need **~1.65 GiB** more of provably-dead AppDir, out of the ~1.85 GiB
that remains: **89% of what is left**.

⛔ **And the sweep was quadratic**: **838 s** on the real kdenlive AppDir,
against ~8 minutes for the whole of the rest of the build. ⭐ Replaced with a
single-pass scan: **7.07 s, and the two outputs are byte-for-byte identical**
on 1,633 real libraries and 2,586 real roots. ⚠ The naive arm carried
concurrent load, so "about 100×" is the honest claim and 118× the arithmetic.

### 4. ⚠ T-071 — items 1, 2 and 5 done

⭐ The rewrite now iterates the **sweep's own** `manifestGlobs`, so the two
rules cannot disagree about which files matter. `manifestIntegrity()` is the
**first check in this tree that reads DATA rather than DT_NEEDED**, and it
passed on the real kdenlive bundle: `manifests 8 name only libraries present`.
`pgb bundle manifests` exposes it with a non-zero exit so an experiment can
fail on it.

⛔ **`__EGL_VENDOR_LIBRARY_FILENAMES` REPLACES `_DIRS` rather than adding to
it** — read out of libglvnd's `LoadVendors()`. A host that exports it made a
bundle setting only `_DIRS` load the **host's** vendors. ⚠ And setting it empty
would have been worse: `getenv` returns a non-NULL empty string, so the branch
is taken and the bundle gets **no EGL at all**.

### 5. ⚠ T-072 — route B refuted, route D opened

    no pad     : size=3264  used=96    headroom=3168
    64 KiB pad : size=68864 used=65648 headroom=3216   pad at tp-65616

Padding the binary's own `PT_TLS` raises size **and** used together: +48 bytes,
alignment noise. ⭐ But the pad is allocated in every thread at a stable
offset, so a loader handing out slices of **its own** `__thread` array gets
what it reserved. ⛔ Not implemented.

### 6. ⚠ T-068 — the harness exists; the numbers it was built on do not

`experiments/93-host-object-residue.sh`, new. ⛔ Writing it exposed that the
904-object sweep quoted in four places was **ad-hoc and never committed** — a
number with no command that reproduces it, and the reason T-068's own Prove
could not be carried out. ⚠ **93- has not been run.**

### 7. ⛔ Ten findings, and not one came from reading code

1. **The artefact cache ignored the build options**, so the `aggressive` run
   would have re-measured the `safe` bytes to the digit. Run 2's defect in a
   new costume.
2. **The soname scan was quadratic** — found by watching `/proc/<pid>/io`.
3. **`__EGL_VENDOR_LIBRARY_FILENAMES` replaces `_DIRS`**, and empty is worse
   than unset.
4. **The pinned digest is the per-platform manifest digest, not the index
   digest** — caught by a control requiring the method to reproduce a digest
   this tree already pins.
5. **A registry 429 was being reported as "unresolved"** — indistinguishable
   from a tag that does not exist.
6. **An unquoted shell variable made a measurement print `none` for every
   arm** — caught because `experiments/21-` supplies an arm that must fail.
7. **Run 6's clock was contaminated by this session** — caught because the
   competitor's fixed artefact moved 6.7×.
8. **`_dl_tls_static_size` was being quoted as the surplus** in four places.
9. **T-068's 904-object sweep was never committed.**
10. **A `while read` loop whose child reads stdin truncates itself** —
    demonstrated at 1 of 5, not asserted.

## ⭐ Work order

⛔ **Unchanged in shape from 2026-09-02c.** What moved is how much of each is
measured.

    ---- glibc's remaining quirks, and future-proofing ----

    T-070   ⚠ P0. Three of four costs are ZERO. ⛔ ONE ROW LEFT: the ten POCs
            against pgb-env-debian-trixie (already built, glibc 2.41, full
            package list). Then the ruling, then cfg.go.
    T-068   P1. experiments/93- is written and NOT RUN. Run it first; it needs
            no bed, only ~900 forks.
    T-072   P1. Route D designed and costed, not implemented. It also closes
            T-068's "static TLS surplus exhausted" row.

    ---- the bundle, and the one class that is all DATA ----

    T-071   ⚠ P0. Items 1, 2, 5 done. ⛔ experiments/85-'s new data-coherence
            arm is WRITTEN AND NOT RUN -- that is this entry's Prove.
            Item 3 folds into T-066; item 4's other half is T-059's.
    T-066   ⚠ P0. ⭐ The corpus IS NOW MINED (34 trees), and reading it moved
            the lever: a "debloated package" is a REBUILD with different build
            options, so what it removes is a DT_NEEDED EDGE. ⛔ An allowlist
            cannot reach that -- it chooses paths, not what a library
            declares. Build the allowlist anyway, but measure its ceiling
            first (route A in the entry): the closure paths reachable ONLY
            through an edge a `-mini` rebuild would delete. Needs an AppDir,
            and the 7 GB one did not survive the container.

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

1. ⚠ **A branch exists on the remote that this session did not create.** The
   harness named `claude/glibc-kdenlive-validation-2x7c3c`; `RULES.md` §Git
   outranks it and every commit is on `main`. It was already on the remote at
   `main`'s commit when this session started, and the git proxy refuses remote
   deletes, so it is left for a human to remove in the web UI.
2. ⚠ **A GPU** — **T-059**, not a question. Every GL row is `swrast`, and
   T-071 item 4's second half cannot be settled without one.
3. ⚠ **Docker Hub rate-limits anonymous pulls in this environment**, which cost
   `ubuntu:24.04` its row in `experiments/91-`. ⭐ `pgb rootfs pull` does the
   anonymous-token dance and succeeds where `docker pull` 429s; that is the
   route to use.
4. ⚠ **`musl-gcc` is absent**, which is the one remaining blocker on
   `experiments/90-`'s arm O. The rust `x86_64-unknown-linux-musl` target was
   the first and is now installed.
