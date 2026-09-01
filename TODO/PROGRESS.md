# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log
and the entries.

    STATE     2026-09-01b, in progress
    COUNTS    22 entries, 9 open, 13 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, EIGHT POCs
              CI: GREEN, 15 jobs, and it asserts criterion 2
              chroot and docker engines produce BYTE-IDENTICAL binaries
              throughput: glibc 4.53 ns/op vs musl 584.71 (malloc, 4 threads)
              pgb over plain gcc -static, same workloads: 0.99x-1.05x

## ⭐ Two operator rulings, taken at the start of this session

⛔ **Both questions the previous session left in "Open questions" are ANSWERED
and are no longer open.** They are recorded in the pages they bind, not only
here: `docs/REQUIREMENTS.md`, `TODO/runtime.md` T-030,
`docs/history/corrections.md` C13 and C14.

1. **T-030's corrected acceptance is ACCEPTED as proposed.** The entry closes
   on *a project whose plugin loading is not configurable at build time, with
   its plugin directory emptied and the functionality intact, on 11 of 11* —
   not on rebuilding CPython, which `experiments/72-` showed cannot be built.
2. **`REQUIREMENTS.md` part 2 is REPLACED with the per-part claim**, with the
   operator's reason recorded verbatim beside it: ⭐ *"anylinux is a bundle,
   our primary goal is still a static glibc binary that has none of the
   issues."* Part 2 is now an enumerated list of nine issues a `gcc -static`
   glibc binary has — **six closed on 11 of 11, three open**, each with an
   entry. Part 1 is unchanged and still binding.

## What this session did

- **Bootstrapped from nothing.** ⚠ This machine started with **0 of 11 rootfs
  present and no static libiconv**: the bed is not cached between sessions and
  the first hour is `pgb env create` plus `fetch-rootfs.sh`. Baseline
  reproduced afterwards — `pgb build` + `pgb verify`, 11 ok, 11 none.
- ⭐ **Swept `pg83/solo` on the operator's instruction, and it opened a fourth
  route to the host-plugin class.** `docs/research/solo.md`; corpus tracked at
  `references/pg83__solo/`, commit `79451211`, MIT. solo compiles a `.so`
  loader **into** a static binary and never asks the host `ld.so` for
  anything. It does it on musl and pays 5,948 lines translating a guest's
  glibc imports onto a musl runtime — ⛔ **which is exactly what a static
  glibc host does not need.**
- ⭐ **`experiments/73-` measures whether that route is worth taking, before
  building it.** 5,807 real host shared objects across the seven glibc
  environments, parsed byte-wise with no binutils inside the target:
  **90.8%–97.8%** of every `GLIBC_`/`GCC_`-versioned import is already
  definable by the pinned static glibc, and the **unexplained residue is
  zero** — every remainder falls into a class decided by the target's own
  files. Two of those classes were not in the tree before:
  - **a version CEILING** (20 symbols, `__isoc23_*` and `strlcpy` at
    `GLIBC_2.38`), which points the opposite way to the floor `AGENTS.md` §14
    already carries. ⛔ No single pin satisfies both ends.
  - **a static/shared split** (49 symbols): `libc.so.6` exports `xdr_void`,
    `__xmknod`, `_sys_siglist` and `__malloc_initialize_hook`, and the *same
    glibc's* `libc.a` does not contain them. ⭐ The static libc is not a
    subset-by-version of the shared one; at one version it is a different
    symbol set, so no choice of pin reaches these.
- ⭐ **The same experiment settled the version-resolution rule in both
  directions**, and one half explains an existing result: rebuilding the object
  *named in a reference's `DT_VERNEED`* without versions makes glibc's loader
  **assert** (`dl-lookup.c:106`). That is what `experiments/50-` ported one
  function of. Route B is weakened; T-031 keeps its other two steps.
- **T-018 done, and it came out of the sweep's tracker rather than its code.**
  ⛔ GCC suppresses `--eh-frame-hdr` for **every** `-static` link, so a static
  executable has no `PT_GNU_EH_FRAME`. ⚠ Nothing was broken — GNU libgcc's
  `crtbeginT.o` registry answers instead, and POC 60 was passing for that
  reason and not by luck — but the fallback belongs to the GNU runtime, not to
  the format, and an unwinder that reads only the segment gets
  `std::terminate` at the first throw. Fixed, +16,512 bytes, 11 of 11
  unchanged.
- **T-033 opened** with route D, its four sub-parts, and ⛔ its unknowns named
  rather than hidden: symbol availability is not a working `dlopen`, and TLS is
  the one place "we are glibc, so it is simpler" is not obviously true.
- **T-017 done.** `env create` and `pgb build` can pick different engines, and
  ⚠ **merely starting `dockerd` changes which environment every command uses**
  — reproduced here before anything was written. `env create` now stamps what
  an environment was built from (a file for chroot, an image **label** for
  docker) and `pgb build` refuses with the difference **named** instead of
  failing inside somebody else's makefile. Six cases measured, including one
  the first version got wrong: `--no-iconv` is a *build* option and must not be
  refused against an environment that happens to have libiconv.
- ⭐ **T-002 and T-030 done, by one build**, exactly as the previous session's
  work order predicted. `poc/70-sqlite-extensions`: SQLite with **fifteen** of
  its own `ext/misc` extensions, **plugin directory created empty**, 11 of 11
  with zero host shared objects and values asserted rather than exit status.
  SQLite is the strict subject T-030's corrected acceptance asks for — `.load`
  calls `dlopen()` on a path the user names and there is no configure switch
  that links an extension in.
  ⭐ **And the control is what makes it mean something:** the same program
  without `--wrap-dlopen`, with the fifteen real `.so` files present, "loaded
  and worked" on **2 of 11** — and both of those pulled in the host
  `ld-linux-x86-64.so.2` and `libc.so.6`. The rest took SIGABRT, SIGFPE, or
  refused.
- ⛔ **Building at scale broke the mechanism, which is what T-002 is for.**
  Every SQLite extension defines a non-static `sqlite3_api`, so **any two
  collided at link time**; two of them define the same entry point by upstream
  design. Fixed by giving each plugin the namespace the loader would have
  given it — `objcopy --redefine-syms` to a per-plugin prefix, table mapping
  the original name to the renamed one. That is `RTLD_LOCAL` at link time, and
  the collision is kept as a live check.
- ⭐ **T-003 done, as the operator's kdenlive challenge.** `poc/80-mlt`:
  **ffmpeg 7.1 and MLT 7.30.0 — kdenlive's engine — built through `pgb`, and
  a 105 MB static `melt` with eight modules compiled in renders a real MP4 on
  11 of 11 with zero host shared objects.** The climb stops at Qt/KF6, which
  is named as not attempted rather than glossed.
  ⛔ **Two real failures, both asserted on every run rather than grepped from
  a log:** MLT hard-codes `add_library(mlt SHARED)` at
  `src/framework/CMakeLists.txt:36`, so `-static` cannot consume it — the
  answer is a link line, not a patch, and that turns "kdenlive's engine cannot
  be static" into "its build system cannot be, and the code is fine"; and the
  `avformat` module cannot be built as a shared object against a static ffmpeg
  (`R_X86_64_PC32 against ff_pw_5`) while **the same objects link into the
  static executable perfectly**.
- ⛔ **A real limit of `--wrap-dlopen`, found by that POC.** MLT does not
  `dlopen` by name — it **lists** its module directory and opens what it
  finds, so an empty directory means the wrapper is never reached. The POC's
  plugin directory therefore holds one **zero-byte file** per module, asserted
  zero-byte. The mechanism serves `dlopen`-by-name; discovery-by-listing needs
  names present.
- ⛔ **Two link-ordering defects in `pgb`, neither of which a caller could
  work around**, because the caller does not control where `pgb` puts its
  objects: plugin objects landed after the caller's `-l` (a plugin calling
  `pow()` failed against a `-lm` already spent), and the fix for that put
  caller archives after `-lpgbruntime` (ffmpeg's `libavformat` calls `iconv`).
  Both fixed, and the second with the same `-Wl,-u` forcing the C++ drivers
  already used.
- ⛔ **T-019: every build option silently did nothing under docker or podman.**
  A container does not inherit the caller's environment and the branch passed
  only `-e PGB_INNER=1`, so `--wrap-dlopen`, `--embed-locale`, `--no-iconv`,
  `--arch-baseline` and `-v` were dropped at the boundary. ⚠ **It hid behind a
  real result**: T-010's byte-identical measurement was taken on a build with
  no options, the one case where dropping them all changes nothing. Fixed, and
  the two engines are now byte-identical **with** options. `pgb shell` had no
  docker branch at all and handed the caller a **host** shell; fixed with it.

⭐ **Three defects were found in this session's own instrument, and every one
had already produced a plausible result**: `libm.a` on Debian 12 is a GNU ld
script rather than an archive, so `nm` returned nothing silently and two dozen
math functions were reported as undefinable; `rootfs-images.txt` was read with
its columns in the wrong order, so all eleven rows skipped and "unexplained
gaps = 0" **passed against nothing**; and the first version of the control
measured the one case glibc deliberately guards, reporting "no" for a rule that
does hold where it matters.

## In progress

Nothing half-written. See `RESUME.md`.

## Work order

    T-032                      the CA bundle and terminfo -- two of the three
                               open rows of REQUIREMENTS part 2
    T-012                      pgb build <spec> -- split it first, it is XL
    T-033                      route D, and it is L -- read solo.md first
    T-041                      aarch64
    then P2 by category

⭐ **Only THREE P1 entries are open**, and one of them (T-012) is the project
itself. T-032 is promoted above it because the operator's framing names the
gaps it closes: *"no networking/iconv/gconv/nss/locale/**cert**/etc issues"*.
The CA bundle and terminfo are two of the three open rows of `REQUIREMENTS.md`
part 2 and both have a proven mechanism waiting rather than an unknown.

## Open questions for the operator

⭐ **None blocking.** The two that were here are ruled on, above.

1. **Is a nixpkgs front end (T-022) in scope**, or does depending on nix defeat
   the point? T-020 argues the graph is worth taking and the store layout is
   not.
2. **T-015 changes what the bed is.** Applying an image's `Env` would make the
   chroot bed match `docker run`, and would also change what every
   locale-sensitive result describes. The entry says it lands with those
   experiments re-run or behind a flag; which one is a judgement about how much
   the existing numbers are worth.
3. ⚠ **`RULES.md` says work on `main`; this host's `main` is three commits of
   file uploads** and every commit of real work is on
   `claude/glibc-research-session-17ku6v`, which the harness designates and
   forbids leaving. The working branch is the trunk here. Worth a ruling on
   whether `main` should be fast-forwarded to it.
