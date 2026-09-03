# HISTORY/entries/poc-open.md — retired DETAIL of poc entries that are STILL OPEN

⚠ **These entries are open. This file is not the entry** — the entry is in
[`../../TODO/poc.md`](../../TODO/poc.md) and is deliberately short. What is
here is the long-form record each one accumulated: the measurements, the
corrections, the routes costed and the routes killed.

⛔ **Read the TODO entry first.** Come here when you need to know WHY it says
what it says, or before re-running something to check whether it was already
run. ⭐ A number quoted in the TODO entry was derived here.

⚠ The headings below deliberately do NOT use the `## T-NNN — ` form, because
that form is what `sh TODO/check.sh` treats as *the* entry, and there must be
exactly one of those per id.

---

## T-054 · retired detail — kdenlive, static: exhaust it

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

### ⭐ 2026-09-03c — RUNG 4's DIRECT DEMAND, MEASURED, AND IT IS 13 INPUTS

⭐ **`pgb nix plan kdePackages.kdenlive` — kdenlive 26.08.0, 13 buildInputs and
8 nativeBuildInputs.** The entry's rungs are ordered KF6 then kdenlive; this
says what rung 4 actually asks for, and it is narrower than "KDE Frameworks":

| group | inputs | where this project stands |
|---|---|---|
| **Qt** | `qtbase-6.11.1`, `qtsvg`, `qtmultimedia`, `qtnetworkauth`, `qtimageformats` | ⭐ **qtbase 6.11.1 is DONE at exactly this version** — `poc/90-qt` and `poc/91-qt-xcb`. Four further modules, none attempted |
| **media** | `ffmpeg-full-9.0`, `mlt-7.40.0`, `ffmpegthumbs` | ⭐ **ffmpeg and MLT are done at OLDER versions** — `poc/80-mlt` built ffmpeg 7.1 and MLT 7.30.0. ⚠ The version drift is real work, not a formality |
| **KDE** | `kio-extras`, `qqc2-desktop-style` | ⛔ **rung 3, and it is TWO direct inputs rather than a framework set.** The sprawl is transitive: `kio-extras` pulls `kio`, which pulls much of KF6 |
| **other** | `KDDockWidgets-2.4.1`, `v4l-utils`, `opentimelineio-0.18.1` | untouched |

    nativeBuildInputs  cmake-4.3.4  ninja-1.13.2  wrap-qt6-apps-hook
                       kf6-move-outputs-hook  qmllint-validate-hook
                       pkg-config-wrapper  shared-mime-info  separate-debug-info.sh

⭐ **So rung 3 is two direct inputs, not a wall**, and rung 4's Qt half already
has its largest piece proved at the version kdenlive wants. ⚠ That is a
narrowing of the work, not a claim it is easy: the transitive KF6 closure under
`kio-extras` is where the count goes, and `poc/80-mlt`'s two recorded build-system
failures below are exactly the shape to expect more of.

⛔ **AND THIS PLAN NEEDED NIX.** `pgb nix plan kdePackages.kdenlive` reported
`no nix-free route resolved … falling back to evaluation` and
`channel pin agrees: no` — the nix-free index/hydra route reaches `kdenlive`
(it did for `experiments/95-`) but not the dotted `kdePackages.` attribute with
its buildInputs. ⚠ **So this measurement is not reproducible on a host with no
nix**, which is precisely the gap T-060 exists to close, and it is recorded here
rather than left to surprise someone.

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



## T-055 · retired detail — If static will not reach it, a kdenlive bundle that BEATS the field

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


## T-063 · retired detail — miniflux with an embedded PostgreSQL, against onelf's ~70 MB

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

## ⭐ ARM S IS NOT "NO" — a static postgres exists and runs on Alpine

Session of 2026-09-02b. Full record: `../evidence/poc/92-miniflux/ARM-S-FINDINGS.txt`.

    src/backend/postgres   63,889,168 B, statically linked
    PT_INTERP absent, DT_NEEDED 0
    ./postgres --version                                  -> PostgreSQL 18.6
    pgb rootfs run alpine-3.22 -- /postgres --version      -> PostgreSQL 18.6

⭐ **PostgreSQL's server, built against glibc, answering on a distribution that
ships no glibc.** 15 dependencies build static without argument.

⚠ **The half that is NOT done:** `make` then fails in `src/interfaces` (libpq,
ecpg — the CLIENT libraries), so the `make install` that would give `initdb`,
`pg_ctl`, `psql`, `createdb` and `createuser` did not complete. ⛔ Nothing yet
claims the miniflux stack runs.

**What it took.** The plan enables fifteen optional features a static build
cannot have, and the adaptation loop removed thirteen of them one per round —
after **five defects in the loop** were fixed this session, each with a carried
selftest (`pgb selftest nix-diagnose`). ⛔ **And the default round budget is 8**,
which reported `gave up after 8 rounds` with nine flags still to go;
`NIX_MAX_ROUNDS=24` is what finished it. A budget below the number of optional
features a distro plan enables reads as "this package cannot be built".

**Two rungs left that are real work rather than patterns:**

1. ⛔ **readline is not missing.** `libreadline.a` and `libncursesw.a` are both
   in the prefix; `AC_SEARCH_LIBS` probes `-lreadline` alone and the archive's
   ncurses references go unresolved. Dropping readline is a workaround for a
   defect that has a real fix.

   ⚠ **CORRECTED 2026-09-03, BY MEASURING IT: `-Wl,--start-group` IS NOT THAT
   FIX, and this row said it was.** Two archives, `libupper.a` calling into
   `liblower.a`, built by the pinned environment:

   | arm | link line | result |
   |---|---|---|
   | A | `cc m.c liblower.a libupper.a` — wrong order, no group | ⛔ **rc=1**, undefined reference |
   | B | the same wrong order inside `--start-group` | ⭐ **rc=0** |
   | C | `cc m.c -Wl,--start-group libupper.a -Wl,--end-group` — `liblower.a` **not on the line** | ⛔ **rc=1**, ``undefined reference to `lower_helper'`` |

   ⭐ **Grouping fixes ORDER. It cannot fix ABSENCE**, and `AC_SEARCH_LIBS
   -lreadline` is absence: `libncursesw.a` is never named on the probe's link
   line, so no amount of grouping can resolve anything out of it.
   `poc/91-qt-xcb`'s `--start-group` answered arm B's problem, which is a
   different one.

   ⭐ **AND THE REAL FIX IS THE SHAPE ITEM 2 ALREADY USES.** `libicuuc.a`
   needing `operator delete` and `libreadline.a` needing `tputs` are the same
   problem — *an archive on the link line has an undefined reference to
   something no archive on the line defines* — and item 2's answer is to READ
   the archives and append what they need. `elfx.NeedsCXXRuntime` is that
   mechanism specialised to one target; generalising it needs a symbol index
   over the candidate archives in the prefix, which is what
   `elfx.DefinedExternalSymbols` already builds for `--wrap-dlopen`.
   ⛔ Not built, and named here rather than guessed at: nothing has measured
   what that costs on a real `configure` run.
2. ✅ **ICU is C++ and postgres is C — FIXED 2026-09-03.** `libicuuc.a` needs
   `operator delete` and the `__cxxabiv1` vtables; a shared libicuuc carries
   `DT_NEEDED libstdc++.so.6` and an archive carries nothing. `internal/wrapper`
   took a `cxx bool` in `LinkFlags` decided **by `argv[0]`**, which is right
   about the SOURCES and wrong about the ARCHIVES.

   ⭐ **It notices now, by reading rather than by a list of names.**
   `elfx.NeedsCXXRuntime` walks an object or every member of an archive and
   looks for an **undefined** reference to a symbol only the C++ runtime
   defines — `_Znwm`, `_ZdlPv`, `__cxa_throw`, the `__cxxabiv1` type-info
   vtables. On a C link that finds one, the wrapper appends `-lstdc++ -lm`
   **after** the link flags, because a single-pass linker resolves an archive
   where it appears.

   ⚠ `__gxx_personality_v0` is deliberately NOT a marker: it appears in
   anything built with exceptions enabled, including C, so it would fire on
   links that need nothing. And `-nostdlib`/`-nodefaultlibs`/`-nostartfiles`
   suppress the whole scan — a caller who says it supplies its own runtime has
   said so deliberately.

   ⭐ **FAILS BEFORE, PASSES AFTER**, on a C++ archive with a C entry point —
   the shape `libicuuc.a` has:

       cc -o prog main.c libcxxthing.a
       pre-fix   rc=1  undefined reference to `operator delete(void*, unsigned long)'
                       undefined reference to `operator new(unsigned long)'
                       undefined reference to `vtable for __cxxabiv1::__class_type_info'
       post-fix  rc=0  ./prog prints 42, PT_INTERP=0 DT_NEEDED=0

   ⚠ **Both other paths were checked rather than assumed**: a plain C link with
   no C++ archive is byte-for-byte unchanged in shape and 978,624 bytes against
   the C++-archive link's 1,064,880; and a real `c++` driver link with
   exceptions and `std::string` still builds, runs and is static.

   **Carried:** `cxx-runtime` in `pgb selftest` builds the fixtures with a real
   compiler and skips visibly without one — including ⛔ **the negative
   control, that an ordinary C object does NOT demand a C++ runtime** — and
   `wrapper-flags` asserts the argument-filtering half offline. 359 → **371
   cases**.

   ⚠ **This does not by itself build postgres with ICU.** It removes the named
   blocker; `--without-icu` can come off when arm S is next run, and that run
   is what would say whether anything else is in the way.

   ### ⛔ 2026-09-03c — ARM S RE-RUN WITH `--without-icu` REMOVED, AND IT FOUND THE FIX DID NOT REACH THE REAL SUBJECT

   ⭐ **This is why R3 was on the work order**, and the answer is the one a
   synthetic subject could not give.

       pgb nix plan postgresql --out pg.plan
       NIX_MAX_ROUNDS=24 pgb nix build --plan pg.plan      (no --configure)

   **`--with-icu` survived all 14 adaptation rounds.** The loop dropped
   `--with-llvm`, `--with-liburing`, `--with-libcurl`, `--with-libnuma`,
   `--with-zstd`, `--with-python`, `--with-gssapi`, `--with-uuid`,
   `--with-systemd`, `--with-tcl`, `--with-perl` and added `--without-readline`
   — and ICU's configure check passed every time. ⛔ **Then the BUILD failed at
   the link**, on exactly the symbols `elfx.NeedsCXXRuntime` exists to
   anticipate:

       libicuuc.a(unifilt.ao): undefined reference to `operator delete(void*, unsigned long)'
       libicuuc.a(lstmbe.ao):  undefined reference to `vtable for __cxxabiv1::__si_class_type_info'
       collect2: error: ld returned 1 exit status

   ⛔ **THE CAUSE, at file and line: `internal/wrapper/dispatch.go`'s
   `cxxRuntimeDemand` skipped every argument beginning with `-`.** It only ever
   saw archives named as literal paths. ⚠ Real builds do not name them that
   way — postgres's own generated `src/Makefile.global` says:

       ICU_LIBS = -L/…/nix-prefix/lib -licui18n -licuuc -licudata -lpthread -lm

   Every one starts with `-`, so `libicuuc.a` was never opened. ⭐ **That is
   precisely why the fix passed a synthetic subject and failed the real one**:
   the `cxx-runtime` fixture links `cc -o prog main.c libcxxthing.a`, a literal
   path, and `wrapper-flags`' `cxx-demand` cases used deliberately non-existent
   paths — so "considered" and "skipped" both answered "no" and nothing could
   tell them apart. ⛔ One case even read *"a flag is never opened as an
   input"*: the defect written down as the intent.

   ⭐ **FIXED.** `-lNAME` and `-l:NAME` now resolve against the `-L`
   directories, `.a` only (a shared library carries its own `DT_NEEDED` on
   libstdc++), and the system directories are deliberately not searched so a
   link does not pay for scanning `/usr/lib`. Proved on the shape that failed,
   through `pgb build`, against a real C++ archive:

       BEFORE  cc -o prog main.c -Llib -lthing   rc=1, 2 C++ undefined refs
       AFTER   the same line                     rc=0, prints 42,
                                                 PT_INTERP 0, DT_NEEDED 0

   **Carried:** `cxxCandidates` is split out so what the scan WOULD open is
   observable without a filesystem, and ten cases in `wrapper-flags` pin the
   rule — `-lNAME` against one `-L` and against several in order, a separated
   `-L dir`, `-l:libfoo.a`, and the negatives (`-lm` with no `-L` resolves to
   nothing; `-o`'s and `-L`'s own values are not inputs; ordinary flags and
   source files name nothing).

   ### ⭐ AND THE RE-RUN WITH THE FIX: A STATIC POSTGRESQL **WITH ICU**, ON ALPINE

   Same command, same plan, the fixed wrapper. ⛔ **ICU link errors: 0.** The
   backend built, and the build advanced past `src/backend` to the failure this
   entry already records:

   | | before the `-l` fix | ⭐ after |
   |---|---|---|
   | where it stops | ⛔ `src/backend`, linking | ⭐ `src/interfaces/libpq` |
   | `operator delete` / `__cxxabiv1` errors | **many** | ⭐ **0** |
   | `src/backend/postgres` | not produced | ⭐ **101,647,216 B** |

       file        ELF 64-bit LSB executable, x86-64, statically linked
       PT_INTERP   0            DT_NEEDED   0
       icu_78 symbols in it     ⭐ 3,911
       ./postgres --version                              -> PostgreSQL 18.6
       pgb rootfs run alpine-3.22 -- /postgres --version -> PostgreSQL 18.6

   ⭐ **101.6 MB against the 63.9 MB this entry recorded for the `--without-icu`
   build** — the difference is ICU, and `nm` finds 3,911 of its symbols in the
   image. **PostgreSQL's server, with ICU collation, built against glibc,
   answering on a distribution that ships no glibc.**

   ⚠ **The stopping point moved but did not go away**, and the new one is a
   different KIND of problem — worth stating because this entry's remaining
   rung 1 is about static link order and this is not that:

       libpq.so.5.18:  U pthread_exit@GLIBC_2.2.5
       libpq must not be calling any function which invokes exit
       make[3]: *** [Makefile:147: libpq-refs-stamp] Error 1

   ⛔ That is **postgres's own policy check** (`libpq-refs-stamp`), not a
   linker error: it forbids `libpq.so` from referencing anything that can
   `exit()`. It is checking the SHARED libpq, which a static build has no use
   for. ⭐ So the next rung is not "make it link" — it is *stop building the
   shared client library at all*, and the flag for that is postgres's own
   (`--disable-shared` is not one of its options; the client libraries' shared
   half is built unconditionally by `src/interfaces/*/Makefile`). ⚠ Not
   attempted; named so the next session starts from the right question.
