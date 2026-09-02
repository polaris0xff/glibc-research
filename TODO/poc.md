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

Fixed in `internal/wrapper/wrappers.go` with `-Wl,-u,__wrap_iconv_open` for the **C++
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
it**: `internal/wrapper/wrappers.go` renames every symbol a plugin object defines with
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

## T-054 — kdenlive, static: exhaust it

**Source** ⭐ **operator, 2026-09-01c**: *"is there something about static
kdenlive in our research, did we actually build/prove it was possible to build
it statically?"* — and then: *"poc a kdenlive static (exhaust all resources)"*.
**Category** poc · **Priority** P1 · **Effort** L · **Status** open

⭐ **The answer to the question, precisely, because the record has it.**
`poc/80-mlt` built **kdenlive's ENGINE** statically and proved it on the
matrix: ffmpeg 7.1 with a 142 MB `libavcodec.a`, MLT 7.30.0, and a **105 MB
static `melt`** with eight `dlopen`'d modules compiled in, **rendering a real
MP4 on 11 of 11 with zero host shared objects**
(`evidence/poc/80-mlt/RESULT.txt`). ⛔ **Qt 6 and KDE Frameworks were NOT
attempted, and kdenlive itself was not attempted** — that POC says so in its
own "depth reached" section. So: the engine is proved, the application is not,
and nobody has yet shown either that it can or that it cannot be done.

**Two failures worth carrying forward**, both from that POC and both about the
build system rather than the code:
- MLT hard-codes `add_library(mlt SHARED)` at `src/framework/CMakeLists.txt:36`,
  so `BUILD_SHARED_LIBS` cannot turn it off and the answer is a link line;
- its `avformat` module cannot be built as a shared object against a static
  ffmpeg (`R_X86_64_PC32 against ff_pw_5`) while **the same objects link into
  a static executable perfectly**.

## ⛔ "Qt/KF6 are impossible then" — no, and the record does not say that

⚠ **Asked by the operator, 2026-09-01c, and it is worth answering precisely
because the answer is the opposite.** `poc/80-mlt` writes, in its own output:

```
Qt 6 / KDE Frameworks    ⛔ NOT ATTEMPTED -- the next rung
kdenlive                 ⛔ NOT ATTEMPTED
```

**NOT ATTEMPTED is not a failure.** Nobody has run it. `grep -rn "Qt" poc/
experiments/` finds four lines and all four are in POC 80's own prose about
what comes next. There is no error, no log and no rung that stopped.

⭐ **And the evidence points the other way**, which is why this entry is L
rather than "closed as unreachable":

| | |
|---|---|
| Qt 6 supports `-static` upstream | `configure -static` is a supported, documented Qt configuration; this is not a hack somebody would have to invent |
| **Qt's plugins are the class this project has ALREADY SOLVED TWICE** | QPA (`libqxcb`), image formats and SQL drivers are *the application's own* plugins, `dlopen`'d by name. That is exactly what `--wrap-dlopen` serves: **POC 70 runs SQLite with fifteen of its own extensions out of an EMPTY directory on 11 of 11**, and POC 50 links 49 CPython modules in with `lib-dynload` empty |
| Qt says so itself | a static Qt build uses `Q_IMPORT_PLUGIN` to link plugins in, which is the same mechanism reached from Qt's side rather than pgb's |
| the engine below it is done | ffmpeg and MLT, static, 11 of 11, rendering a real MP4 |

⚠ **The honest risks, named rather than hidden:** Qt 6 is a very large build
(hours, not minutes, and it is the reason this entry is L); KF6 has far less
static-build practice behind it than Qt does; and MLT's own
`add_library(mlt SHARED)` shows that a project's BUILD SYSTEM can refuse
`-static` even when its code is fine. Any of those could stop it.

⛔ **What this entry may NOT do** is close as "impossible", per
`docs/AGENTS.md` §14. It closes one of two ways: a static Qt program running
on the matrix, or a written record of the exact rung that stopped it, with the
error and the file it came from — the shape
`evidence/72-static-host-plugin-abi/CPYTHON-FAILURE.txt` already uses.

**What "exhaust" means here.** Climb the rungs in order and record each:
a Qt 6 *widget* program static (this is `poc/90-qt` and it is the next
session's first required POC), then a Qt program with a QPA platform plugin
under `--wrap-dlopen`, then KF6, then kdenlive.

**Prove.** `evidence/poc/90-kdenlive/RESULT.txt`, or a written record of the
rung that stopped it with the error and the file it came from — the shape
`evidence/72-static-host-plugin-abi/CPYTHON-FAILURE.txt` already uses.

## ⭐ RUNG 1 IS CLOSED: a static Qt 6 widget program runs on 11 of 11

`evidence/poc/90-qt/RESULT.txt`, 2026-09-01d. **19 assertions, 0 failures,
0 skips.** ⛔ The entry stays **open** — rungs 2, 3 and 4 are untouched — but
"Qt/KF6 are impossible then" is now answered with a binary rather than an
argument.

| | |
|---|---|
| source | `pgb nix plan/fetch qt6.qtbase` → `qtbase-everywhere-src-6.11.1.tar.xz` from cache.nixos.org, signature and NarHash checked. **Stock tarball; none of nixpkgs' eleven patches applied** — every one of them is about finding things on disk at run time, which a static build with compiled-in plugins does not do |
| configure | stock `./configure -static -release -force-bundled-libs -qpa offscreen -default-qpa offscreen`, plus the features that would reach outside the tarball turned off (xcb, egl, opengl, dbus, glib, icu, openssl, cups, fontconfig, libudev, network, sql, testlib, printsupport) |
| built | `libQt6Core.a` 23.0 MB, `libQt6Gui.a` 19.6 MB, `libQt6Widgets.a` 22.3 MB, and ⭐ **NOT ONE SHARED OBJECT** anywhere under the prefix |
| plugins | **five static plugin archives**: `libqoffscreen.a`, `libqminimal.a`, `libqjpeg.a`, `libqico.a`, `libqgif.a`. `qt_add_executable` emits the `Q_IMPORT_PLUGIN` translation unit, so the QPA plugin is in the link and `dlopen` is never called |
| the binary | 28,123,352 bytes, no `PT_INTERP`, 0 `DT_NEEDED`, pgb runtime linked |
| matrix | **11 of 11 pass, host shared objects loaded: none, on every row** |

⭐ **The functional test is not `--version`.** Inside every target: the QPA
plugin resolves to `offscreen` from its own compiled-in default; a UTF-8 round
trip over CJK plus an astral-plane codepoint; `QLocale(de_DE)` formatting
1234.5 as `1.234,5` out of Qt's compiled-in CLDR; `QPainter` filling a
`QImage` with the pixel read back; `QWidget::grab()` rendering a 120×90 widget
and the pixel read back; `QFile` write and read; a Latin-1 `QStringConverter`
round trip; and `QTimer` driving `QApplication::exec()` to a clean return.

⭐ **And the negative control.** Asking for a QPA plugin that is **not**
compiled in (`QT_QPA_PLATFORM=xcb`, `QT_DEBUG_PLUGINS=1`) aborts on all
eleven — and **pulls in no host shared object on any of them**. A static Qt
does not fall back to the host's plugin directory, which is the property that
makes the "no host objects" row above mean something.

⚠ **Two observations worth carrying to rung 2.** Qt prints
`Detected locale "C" ... Qt depends on a UTF-8 locale, but has failed to
switch to one` on the four musl rows and switches to `C.UTF-8` on the glibc
ones — the same locale gap `--embed-locale` exists for, arriving from Qt's
side. Nothing failed because of it, because Qt's internals are UTF-8
regardless, but a Qt application that formats user-facing text would want the
flag. And `getpwuid_r` produced the usual static-glibc link warning, which is
exactly what `pgb-nssfix.c` answers.

⛔ **One pgb defect had to be fixed before configure would even run**, and it
was not a Qt problem: `docs/history/corrections.md` C15. pgb appended its
`-march=x86-64` **after** the caller's argv, so Qt's `-march=cannonlake`
intrinsics probe was silently downgraded and configure stopped with
*"x86 intrinsics support missing. Check your compiler settings."* ⭐ Building
above the current class is what found it, which is what `TODO/INDEX.md`'s
ordering argument says POCs are for.

**What is left on this entry**, unchanged and untouched:
rung 2 a Qt program against a real display (xcb, and its plugin under
`--wrap-dlopen`); rung 3 KDE Frameworks 6; rung 4 kdenlive.

## ⭐ RUNG 2 IS CLOSED: A STATIC Qt 6 APPLICATION OPENS A REAL WINDOW

⛔ **The operator's ruling on `poc/90-qt`, 2026-09-01e, and it was right:**
*"That is not a Qt application, it is a Qt library that links. A Qt POC that
never opens a window and never loads a real platform plugin has not reached the
rung T-054 names."*

`poc/91-qt-xcb`, **26 assertions, 0 failures, 0 skips**,
`evidence/poc/91-qt-xcb/RESULT.txt`.

| | `poc/90-qt` (rung 1) | ⭐ `poc/91-qt-xcb` (rung 2) |
|---|---|---|
| QPA | offscreen | **xcb, talking to a real X server** |
| window | none | **a mapped, resized, EXPOSED QWidget** |
| network | `-no-network` | **QtNetwork with OpenSSL 3.6.3 LINKED IN** |
| sql | `-no-sql` | **QtSql, QSQLITE, a real query round trip** |
| dependencies | none outside the tarball | **20 X libraries built static by pgb from the nixpkgs plan** |

    libqxcb.a          17,004 bytes -- the REAL platform plugin, as an archive
    the probe          47,188,344 bytes, no PT_INTERP, 0 DT_NEEDED
    the matrix         11 of 11 pass, host shared objects loaded: NONE

⭐ **What the probe asserts, inside every one of the eleven**, against an
`Xvfb` reached over loopback TCP (the bed shares the host's network namespace,
so no socket has to be staged into eleven root filesystems):

    QPA platform plugin              xcb          <- not offscreen
    primary screen from the X server 1024x768     <- the server answered
    QWidget::show() produced a QWindow, with a native id
    the X server EXPOSED the window               <- an offscreen QPA cannot
    QWidget::grab() of the mapped window          320x240
    QSslSocket::supportsSsl()        OpenSSL 3.6.3 9 Jun 2026
    QSQLITE driver registered, and a UTF-8 round trip through a real query
    QApplication::exec() returned 0

⛔ **Qt's own config test had to be answered with evidence rather than argued
with.** `TEST_xcb_syslibs` is a `try_compile` whose link line Qt composes from
an `XCB::XCB` target that carries no transitive information for a static
libxcb, so it failed on link order alone — a cycle between `libX11.a`,
`libxcb.a` and `libXau.a`. ⭐ **The POC therefore links its own static xcb +
xkbcommon-x11 program first, in a `--start-group`, and only passes
`-DTEST_xcb_syslibs=ON` if that link succeeded.** An override nobody checked is
a way of making a build succeed by lying to it.

⚠ **What rung 2 does NOT include, stated rather than discovered later:**

- **OpenGL.** `libglvnd` is a dispatch layer whose whole purpose is to `dlopen`
  a vendor — `docs/limitations.md` §1 from the GL side — and T-052's answer for
  that is a **bundle** (`experiments/85-`, `89-`). A Qt *widget* application
  uses the raster paint engine and does not need it.
- **`--wrap-dlopen` on the QPA plugin.** A static Qt emits its plugins as
  **archives**, so `Q_IMPORT_PLUGIN` puts the platform plugin in the link and
  `dlopen` is never called. There is nothing to intercept. T-054's phrasing
  predates that measurement; the rung it was reaching for — a REAL platform
  plugin rather than a stub — is what this POC does.
- **Fonts.** With `-no-fontconfig` Qt warns `Cannot find font directory` and
  renders with no system font. Nothing asserted here needs a glyph; a Qt
  application that displays text would want `--embed`-style font carriage or
  fontconfig, and that is the next thing to try.

⛔ **Rungs 3 (KF6) and 4 (kdenlive) are still untouched**, and the entry stays
open for them. What rung 2 removes is the doubt: a static Qt **application**,
not a library that links.


## T-055 — If static will not reach it, a kdenlive bundle that BEATS the field

**Source** operator, 2026-09-01c: *"if impossible, pivot to
kdenlive.nixappimage, but it must be smaller, load faster, run faster than
pkgforge-dev/kdenlive-AppImage-Enhanced"*.
**Category** poc · **Priority** P1 · **Effort** L · **Status** open

⛔ **THE BAR IS A COMPARISON, NOT A BUILD.** An AppImage that works is not
this entry; three measured columns against a named competitor is. And
`docs/AGENTS.md` §14 forbids writing "strictly better" without the
measurement, so the columns come first:

| column | how, and the instrument that already exists |
|---|---|
| size | bytes, both artefacts, same day |
| load | first-run and warm-run startup, `experiments/40-`'s method, and ⚠ its noise floor applies — a difference at or under it is "no difference measurable" |
| run | a real render, `poc/80-mlt`'s MP4 workload, wall clock |

⚠ **And the honest risk, named now:** the competitor is hand-crafted per
application by people who do this full time, and `Anylinux-AppImages`'
README lists dozens built that way. Beating it by automation is the claim
worth making and it is not the same claim as beating it at all.

**Blocked on** T-054 answering first, and on T-052, because a video editor is
exactly the case where the OpenGL question decides whether the bundle runs.

## ⭐ THE COMPARISON EXISTS — `experiments/90-kdenlive-vs-enhanced.sh`

⛔ **AND THE BAR IS NOT MET. Ours is bigger, starts slower and renders
slower**, on all three of the columns the operator named. That is the result,
stated first, because `docs/AGENTS.md` §14 forbids "strictly better" without a
measurement and equally forbids hiding one.

Subject: **kdenlive 26.08.0** on both sides — nixpkgs has that exact release
and the competitor's tag is `26.08.0-1`, which is luck rather than design.

| column | P — ours, one command | E — `kdenlive-AppImage-Enhanced` | |
|---|---|---|---|
| **size** | **397,903,295 B** | **191,900,604 B** | ⛔ **2.07×** |
| **render** (melt → a real MP4) | **3,625 ms** | 2,001 ms | ⛔ **1.81×** |
| **start** cold | 3,344 ms | 1,325 ms | ⛔ 2.52× |
| **start** warm | 139 ms | 34 ms | ⛔ 4.1× |
| runs on the eleven | **11 of 11** | **11 of 11** | equal |
| the MP4 | 4,149 B, 48 frames, libx264 | 4,162 B | equal |

⭐ **What IS established, and it is the half T-054 could not reach**: a
kdenlive that renders, produced by **one command from a package name**, on
eleven distributions including four musl ones. The engine, the Qt stack, KF6,
MLT and ffmpeg all arrive from the nixpkgs closure with no nix installed.

### ⛔ Two defects this comparison found in our own bundle

- **`melt` started, answered `-version`, and could not render.** MLT bakes its
  module directory into libmlt at build time and nixpkgs does not wrap it, so
  the bundle printed `mlt_repository_init: no plugins found in
  "/nix/store/…-mlt-7.40.0/lib/mlt-7"`. ⭐ A bundle can pass every check the
  bundler already makes and still not render; §5d now scans the packed
  binaries for compiled-in store paths (**735 strings, 532 naming a real
  directory**) and redirects the ones a documented variable can reach.
- **The store shard copied whole packages when a variable named one
  directory.** `QT_PLUGIN_PATH=…/qtdeclarative-6.11.1/lib/qt-6/plugins` pulled
  in all **190 MB** of qtdeclarative, whose shared objects were already
  flattened into `lib/`. The shard was **947 MB of a 1.2 GB `lib/`**. Copying
  only the named subdirectory took the artefact from **539 MB to 398 MB**.

### ⛔ And one in the measurement, which is the interesting one

**A multi-program bundle needs a selector, a selector is a shell script, and a
script is run by the HOST's `/bin/sh` — which loads the host's libc.** Ours
opened 1–4 host shared objects on every glibc row and none on the four musl
ones. ⭐ **The competitor pays exactly the same price** — its `AppRun.sh` is a
shell script too, and on Rocky 8 it opened **10** where ours opened 3. So the
column is asserted as a comparison rather than an absolute, and
`internal/bundle/appimage.go` now takes the shell **only when there is more than one
program**, leaving 85-, 86- and 89-'s zero-host-object rows untouched.

### ⭐ The route to the bar, in the order the numbers say

1. **`--debloat aggressive`** is untried on this artefact and removes ~90 MiB
   of Vulkan ICDs for GPUs this architecture has; `experiments/89-` measured
   it as 0.78× on a GL bundle.
2. **`share/` is 368 MB**, most of it `breeze-icons` (108 MB) — an icon theme
   ships every size and colour of every icon, and a debloat rule for unused
   icon sizes is the biggest single lever left.
3. **The `store/` shard is still 405 MB.** The wrapper env names 226 paths;
   copying only the named subdirectory helped, and de-duplicating what is
   already in `lib/` would help again.
4. **Start and render** are dominated by mounting a 398 MB dwarfs image
   against a 192 MB one: the size column IS the time column here, so 1–3 move
   both.

⛔ **The entry stays open.** It closes when the three columns are measured
again after 1–3, whichever way they come out.

---

## T-063 — miniflux with an embedded PostgreSQL, against onelf's ~70 MB

**Source** operator, 2026-09-02: *"prove pgb can build something as complex as
this"*, naming
`../references/QaidVoid__onelf/tree/docs/guide/examples/miniflux.md`.
**Category** poc · **Priority** P1 · **Effort** L · **Status** open

⛔ **THE SUBJECT IS CHOSEN FOR WHAT IT IS NOT: ONE PROGRAM.** onelf's
walkthrough produces a single ~70 MB artefact that starts a private postgres on
a unix socket, initialises the cluster on first run, runs miniflux's
migrations, seeds an admin user, serves HTTP on `127.0.0.1:8080` and shuts
postgres down cleanly. Two programs, five postgres helpers, a share tree and a
set of `dlopen`'d extensions.

**What each part asks of pgb**

| part | what it demands |
|---|---|
| two programs + `initdb`, `pg_ctl`, `pg_isready`, `psql`, `createdb`, `createuser` | the bundler already carries several programs; the entry point is the open question |
| ⛔ **`$libdir` is computed at RUN TIME** from the build-time relationship between `bindir` and `pkglibdir` | get it wrong and initdb dies with `could not access file "dict_snowball"`. ⭐ This is what `--wrap-dlopen` is for: compiling the extension table in REMOVES the path problem rather than reproducing it |
| a share tree found through `PGSHAREDIR` | data, not code — the `--embed-*` question |
| symlink farms | onelf's guide insists on `cp -L`; a nix closure does not have this problem, and that is a point in pgb's favour worth stating |
| ⛔ a **shell orchestrator** as the entry point | which is the thing pgb's whole argument is against. ⭐ The interesting question is whether pgb can do it with NO SHELL in the delivery path, and the honest answer may be "not yet" |

**Premise, measured before either arm was attempted.** ⭐ **The two halves are
not equally hard, and the POC says so rather than implying otherwise.**
`pgb nix plan miniflux` reports `buildInputs: []` and
`nativeBuildInputs: [go-1.26.5, install-shell-files]` — miniflux 2.3.3 is a
**pure Go program**, already a static ELF, and none of pgb's four mechanisms
apply to it. `pgb nix plan postgresql` reports **16 buildInputs**, among them
`icu4c`, `libxml2`, `llvm` (for the JIT output) and the configure flag
`--with-systemd` — which asks a static binary to link a library whose purpose
is `dlopen`, and which `internal/nixx`'s dep-skip list refuses for that reason.
⛔ **The whole difficulty is PostgreSQL.**

**Two arms.**

- **arm S — static.** `pgb nix build postgresql` and `miniflux`, one static ELF
  per program, `--wrap-dlopen` for the extensions. ⛔ Attempted FIRST, and if
  nixpkgs' postgres will not build statically the deliverable is **which
  dependency refused**, named and recorded — not a quiet fall through to arm B.
- **arm B — bundle.** `pgb bundle appimage` over the same closure.

**Prove.** The artefact starts postgres, runs the migrations, answers an HTTP
request on `127.0.0.1:8080` and shuts down cleanly, measured on at least
Debian 12 and Alpine 3.22 through `pgb rootfs run`, plus a table comparing
size, first-launch and second-launch time against onelf's stated ~70 MB.
`poc/92-miniflux/`, exit-code contract 0/1/2 like every other POC.
