# poc — harder applications, until something breaks

⭐ **This is priority one.** The way static glibc gets pushed further is by
building progressively harder software with `pgb`, watching it fail, and fixing
`pgb` or `tool/runtime/*.c` until it does not. Five projects pass today
(`docs/AGENTS.md` §9); none of them is hard enough to find the next defect.

---

## T-001 — Build a C++ project with a real dependency tree

**Source** operator, session of 2026-09-01.
**Category** poc · **Priority** P1 · **Effort** M · **Status** ✅ done

**Problem.** Every passing POC is C. C++ brings static initialisation order,
`libstdc++` exception tables across a static link, RTTI across translation
units, and `--wrap` interacting with mangled symbols. None is exercised.

**Premise.** ⚠ Read, not measured: `pgb`'s wrappers pass `-shared` through
untouched and inject only at executable links, so a C++ build *should* work
unchanged. No C++ project has been built.

**Approach.** Pick something with a dependency chain that autotools/CMake
resolves — a candidate list is in the entry, not here — build under
`pgb build`, run `pgb verify` on the matrix.

**Prove.** `sh poc/60-<name>/run.sh` exits 0 with the matrix table showing
`ok` and `none` on all 11.

**Closed with** `poc/60-leveldb/` — LevelDB 1.23 built with CMake, linked into
a C++ subject, `pass=12 fail=0 skip=0`, `ok` and `none` on all 11.
`evidence/poc/60-leveldb/RESULT.txt`.

⛔ **The premise was wrong and keeps its title.** It read "⚠ Read, not
measured: `pgb`'s wrappers pass `-shared` through untouched and inject only at
executable links, so a C++ build *should* work unchanged." Measured: **no C++
program linked at all**, and the reason is ordering, not `-shared`.

libstdc++ calls `iconv` itself from `std::__narrow_multibyte_chars`. `--wrap`
rewrites those references to `__wrap_iconv*` exactly as it does the
application's — and the wrappers append `pgb`'s flags to the end of the user's
argv, after which the compiler driver appends its own libraries. `gcc -###`
on this machine: `-lpgbruntime` at 178, `"-lstdc++"` at 180. An archive is
scanned where it appears, so:

```
undefined reference to `__wrap_iconv_open'
... in .text._ZSt24__narrow_multibyte_charsPKcP15__locale_struct
```

Fixed in `tool/lib/wrappers.sh` with `-Wl,-u,__wrap_iconv_open` for the **C++
drivers only** — the same forcing technique already used for
`pgb_runtime_anchor`. ⚠ The cost is stated rather than hidden: a C++ program
now links the iconv shim whether or not it calls `iconv`. The C property
`docs/AGENTS.md` §10 measures is kept — re-measured after the change, a C
program that never calls `iconv` is **1,008,152 bytes** and a C++ one is
**2,160,440**.

⭐ **Three more things the C++ arm found, none of them about C++:**

1. **The pinned environment had no CMake, meson or autoconf** — T-016. Every
   passing POC was an autotools tarball because that was the only build system
   the environment could run, and nothing said so.
2. ⛔ **`poc_matrix` passed with no functional test.** `poc_functional_test >
   script` writes an **empty file** when the function is undefined; `sh` on an
   empty script exits 0; every row read `ok` and the trace of that empty
   script found no objects. **Eleven green rows having executed nothing.**
   `poc_matrix` now refuses. ⚠ All five existing POCs define the function, so
   no committed result was affected — the harness could have certified a bad
   POC, and did not.
3. ⛔ **`poc_observe` had the same shape** — a missing probe printed `<none>`
   in every row, indistinguishable from eleven environments measured clean.
   It now says `NOT MEASURED` and counts a skip.

## T-002 — Build something that dlopens its own plugins at scale

**Source** operator · **Category** poc · **Priority** P1 · **Effort** M · **Status** ✅ done

**Problem.** POC 50 links CPython's 49 extension modules in by hand, using
CPython's own `Modules/Setup.local` mechanism. Nothing generalises that.

**Premise.** ⭐ Measured: a program loading its *own* plugins is servable —
POC 50 does it. What is unmeasured is whether it generalises without the
project cooperating.

**Approach.** A project whose plugin loading is not configurable at build time.
This is the entry that most likely produces T-010 (`--wrap-dlopen`).

**Prove.** `sh poc/<n>-<name>/run.sh` exits 0, and `evidence/.../RESULT.txt`
shows an empty plugin directory with the functionality still working.

**Closed with** `poc/70-sqlite-extensions/`, `pass=20 fail=0 skip=0`,
`evidence/poc/70-sqlite-extensions/RESULT.txt`.

⭐ **SQLite is the right subject and the reason is structural.** Its
loadable-extension interface is an OPEN ABI: `.load ./series` calls `dlopen()`
on a path the *user* names, derives an entry point from the FILENAME, and
calls it. There is no configure switch and no `Setup.local` equivalent — to
link an extension in and keep `.load` working you would have to edit
`sqlite3.c`. Measured rather than assumed: `sqlite3LoadExtension()` calls
`sqlite3OsDlOpen()` with **no stat and no access check**, and retries with
`.so` appended when the first call returns NULL, so both paths through the
wrapper are exercised on every load.

**Fifteen extensions, plugin directory created EMPTY on every target:**

```
  11 of 11 environments   functional test ok
  11 of 11 environments   host shared objects loaded: none
  18 assertions per environment, 3 of them NEGATIVE
```

⭐ **And the control says what that is worth.** The same sqlite3, built by
`pgb` without `--wrap-dlopen`, with the fifteen **real** `.so` files staged
in — observed, never asserted:

| | |
|---|---|
| loaded and worked | **2 of 11** (Debian 12, Arch) — and both pulled in the host `ld-linux-x86-64.so.2` **and** `libc.so.6` |
| SIGABRT | Debian 11, Ubuntu 20.04, Rocky 8 |
| SIGFPE | openSUSE Leap, Fedora 42 |
| refused, `libc.so.6: cannot open shared object file` | all 4 musl |

That is `docs/limitations.md` §1 reproduced on a second program, and the two
rows that "work" are the two-libc state the whole project exists to prevent.

### ⛔ What building at scale found, which one plugin never would

`--wrap-dlopen` puts the plugin objects in **one** executable, and SQLite's
extension ABI requires every extension to define a file-scope, non-static
`const sqlite3_api_routines *sqlite3_api` (`SQLITE_EXTENSION_INIT1`). **All
sixteen** of sqlite's `ext/misc` extensions define it, so **any two collided
at link time and the build stopped**:

```
ld: uuid.o:(.bss+0x0): multiple definition of `sqlite3_api';
    series.o:(.bss+0x0): first defined here
```

⛔ And worse than one ABI's habit: sqlite derives the entry point from the
filename keeping only *alphabetic* characters, so `base64.c` and `base85.c`
both define `sqlite3_base_init` **on purpose**. Two plugins colliding on their
entry point is a thing upstreams deliberately do.

⭐ **Fixed by giving each plugin the namespace the loader would have given
it**: `tool/lib/wrappers.sh` renames every symbol a plugin object defines with
`objcopy --redefine-syms` to a per-plugin prefix, and the generated table maps
the ORIGINAL name to the renamed one. `dlsym` still answers
`sqlite3_series_init`; nothing else in the link can see it. That is
`RTLD_LOCAL`, reproduced at link time. Checked on the artefact, not trusted:
15 `pgb_dl<N>_sqlite3_api` symbols in the binary and **0** unnamespaced ones,
and the POC keeps the raw collision as a live check that still fails.

⚠ **One extension was dropped, with the control that says why.**
`ext/misc/percentile.c` segfaulted on all eleven — which reads exactly like a
defect in `--wrap-dlopen`. It is not: it segfaults **at load time**, before
any query runs, in an ordinary dynamically linked sqlite3 built from the same
amalgamation loading a real `.so` through the real `dlopen`, in two different
build configurations. Replaced with `csv`. ⭐ Without that control this POC
would have reported eleven segfaults against the mechanism under test.

## T-003 — Build a project that fails, and write down why

**Source** operator · **Category** poc · **Priority** P1 · **Effort** S · **Status** ✅ done

**Problem.** Every POC in the tree passes. A tree of only-passing POCs is a
demo, not a test bed, and says nothing about where the edge is.

**Premise.** ⭐ Certain: something will fail. `docs/limitations.md` §4 already
records CPython's `nis` module failing to link and that finding came from a
failure, not a success.

**Approach.** Deliberately pick above the current class — a GTK or Qt
application. Record the failure per `docs/methodology/experiments.md`: what was
attempted, expected, actual, failure mode, suspected cause, whether it is
fundamental.

**Prove.** A committed `evidence/` record of a build that did not work, with
the cause named at file and line, and either a fix or a new entry.

### ✅ Closed with `poc/80-mlt/` — the kdenlive challenge, taken seriously

⭐ **The operator set this as a challenge**: the `Anylinux-AppImages` README
and this project's own documents make a joke of "just statically compile
kdenlive". kdenlive is a Qt/KDE application on top of **MLT**, and MLT is
where the video work happens — it is the engine, it loads its functionality
as `dlopen`'d modules, and it links ffmpeg. The climb is

    ffmpeg  ->  MLT (melt)  ->  Qt/KF6  ->  kdenlive

and `poc/80-mlt` is the first two rungs. `pass=21 fail=0 skip=0`,
`evidence/poc/80-mlt/RESULT.txt`.

| rung | |
|---|---|
| **ffmpeg 7.1** | ✅ builds through `pgb`. 142 MB `libavcodec.a`, and the `ffmpeg` CLI it produces is static and runs |
| **MLT 7.30.0 / `melt`** | ✅ **105 MB static binary, eight modules compiled in, renders a real MP4 on 11 of 11 with zero host shared objects** |
| Qt 6 / KDE Frameworks | ⛔ not attempted — the next rung |
| kdenlive | ⛔ not attempted |

⛔ **Two real failures, and they are what this entry asked for.** Both are
asserted on every run — one `make` of one target each — rather than grepped
out of a build log, because the first version did grep a log and the second
run of the POC (everything cached) then reported both failures as *not having
happened*.

1. ⛔ **`melt` cannot be linked through its own build system**, and the cause
   is one line of somebody else's CMake:

   ```
   src/framework/CMakeLists.txt:36    add_library(mlt SHARED ...)
   /usr/bin/ld: attempted static link of dynamic object
                `../../out/lib/libmlt-7.so.7.30.0'
   ```

   `SHARED` is hard-coded, so `BUILD_SHARED_LIBS` cannot turn it off.
   ⭐ **This project's rule is no source patches, so the answer is a link
   line rather than an edit**: `melt` is linked from the objects the build
   already produced. That turns *"kdenlive's engine cannot be static"* into
   *"kdenlive's engine's BUILD SYSTEM cannot be, and the code is fine"* —
   which is a different and much more useful sentence.

2. ⛔ **The `avformat` module cannot be built as a shared object against a
   static ffmpeg**: `libavcodec.a(cavsdsp.o): relocation R_X86_64_PC32
   against symbol 'ff_pw_5' can not be used when making a shared object`.
   ⭐ **And the same objects link into the static executable perfectly.** The
   requirement was a property of the shared-module *shape*, not of the code,
   so the module that could not be built the normal way is in the binary.

### ⛔ One real limit of `--wrap-dlopen`, found here

MLT does not `dlopen` a plugin **by name**. `mlt_repository_init` **lists**
the module directory (`src/framework/mlt_repository.c`) and `dlopen`s whatever
it finds, so an **empty** directory means it finds nothing and never calls
`dlopen` at all — the wrapper is never reached.

⭐ So the plugin directory in this POC holds one **zero-byte file** per
module, asserted to be zero-byte. Nothing is mapped and no code is in them:
they exist so the *listing* has entries. ⛔ `--wrap-dlopen` serves
`dlopen`-by-name; a program that DISCOVERS its plugins by listing a directory
needs that directory to have names in it. `docs/limitations.md` §1.

### Landed in `pgb` while closing this

⛔ **Two link-ordering defects, both of which the caller could not have worked
around**, because the caller does not control where `pgb` puts its objects:

| | |
|---|---|
| plugin objects appended **after** the caller's `-l` | a plugin calling `pow()` got `undefined reference to 'pow'` against a `-lm` the caller had already spent. The wrapper now **re-emits the caller's own `-l`/`-L` after them** — repeating a `-l` is safe, and nothing is invented, so a plugin needing a library the program never named still fails loudly |
| that fix then put caller archives **after** `-lpgbruntime` | ffmpeg's `libavformat` calls `iconv`, `--wrap` rewrites it, and the archive defining `__wrap_iconv_open` was already behind the linker. Fixed with the same `-Wl,-u` forcing the C++ drivers already use — now applied whenever `--wrap-dlopen` is in play, for C too |

⚠ **And one shape nothing in this tree had hit**: MLT's `plus` module contains
`subtitles.cpp`, so a **C** program's plugin pulled in `std::ios_base::Init`
and `__cxa_begin_catch`. The link needs the C++ driver even though the program
is C.

⚠ **Partly served already, and this was true before the above.** `evidence/72-static-host-plugin-abi/CPYTHON-FAILURE.txt`
is a committed record of a build that did not work — CPython 3.12.7 with a
static interpreter and shared stdlib modules — with the cause named at
`Makefile:1125` and diagnosed to a structural fact (`.dynsym` entries: 0), and
it produced both a fix in `--wrap-dlopen` and a new finding in
`docs/limitations.md` §0. ⛔ But this entry asked for a failure found by
building **above the current class**, and that is the different question
`poc/80-mlt` answers above: what breaks when the *dependency graph* gets
large, rather than what breaks when a known-good project is deliberately
configured the hard way.
