#!/bin/sh
# POC 91 -- Qt 6, static, with the REAL xcb platform plugin, on a real display.
#
# ⛔ WHY THIS EXISTS, AND IT IS AN OPERATOR RULING RATHER THAN A NEW IDEA.
# `poc/90-qt` built qtbase with `-no-xcb -no-opengl -no-network -no-sql` and an
# OFFSCREEN QPA, and the operator's judgement on 2026-09-01e was:
#
#   *"That is not a Qt application, it is a Qt library that links. A Qt POC
#    that never opens a window and never loads a real platform plugin has not
#    reached the rung T-054 names."*
#
# That is correct, and this POC is rung 2. What changes:
#
#   | | poc/90-qt (rung 1)        | poc/91-qt-xcb (rung 2)                |
#   |-|--------------------------|---------------------------------------|
#   |QPA| offscreen, compiled in  | ⭐ **xcb**, talking to a real X server |
#   |window| none                 | ⭐ a mapped, resized, exposed QWidget  |
#   |network| -no-network         | ⭐ Qt Network with **OpenSSL LINKED**  |
#   |sql| -no-sql                 | ⭐ QtSql with the SQLite driver        |
#   |deps| none outside the tarball| ⭐ nine X libraries + xkbcommon, all   |
#   | |                           | built static by pgb from nixpkgs plans |
#
# ⚠ WHAT IS STILL NOT HERE, said plainly rather than discovered later:
#   - **OpenGL.** `libglvnd` is a dispatch layer that exists to `dlopen` a
#     vendor, which is docs/limitations.md §1 from the GL side, and T-052's
#     answer for that is a BUNDLE (experiments/85-, 89-). A static Qt widget
#     application uses the raster paint engine and does not need it. So GL is
#     absent by design here and named in the record, not skipped quietly.
#   - **`--wrap-dlopen` on the QPA plugin.** A static Qt emits its plugins as
#     ARCHIVES (`libqxcb.a`), so `Q_IMPORT_PLUGIN` puts the platform plugin in
#     the link and `dlopen` is never called. There is nothing for
#     `--wrap-dlopen` to intercept. T-054's phrasing predates that measurement
#     (poc/90-qt made it); the rung it was reaching for -- a REAL platform
#     plugin rather than a stub -- is what this POC does.
#
# ⚠ THE DISPLAY IS A REAL X SERVER AND IT IS REACHED OVER TCP. `Xvfb -ac
# -listen tcp` runs on the build host; every target connects to
# `127.0.0.1:<n>`. The bed is `unshare --mount` + `chroot`, so the network
# namespace is shared and the loopback connection works from inside without
# binding a socket into eleven root filesystems. ⛔ A unix-socket display
# cannot be staged with `poc_stage_extras`, which copies files.
#
# Exit: 0 all assertions matched, 1 one did not, 2 could not run.
# SPDX-License-Identifier: MIT
. "$(dirname "$0")/../common.sh"

POC_WHY="a static Qt 6 program that OPENS A WINDOW through the real xcb plugin"
POC_URL="https://download.qt.io/archive/qt/6.11/6.11.1/submodules/qtbase-everywhere-src-6.11.1.tar.xz"
POC_VERSION="Qt 6.11.1 (qtbase) + libxcb 1.17 + xkbcommon 1.13 + OpenSSL 3"
POC_SHA256="d9594a31228aa23ad6b531719a29b45f0f3989fe6c136d45767ea179f233c1ac"
POC_NORMAL_BUILD="apt install qt6-base-dev && cmake && make"
POC_STRESSES="xcb QPA on a real X server, a mapped window, static OpenSSL linked into QtNetwork, QtSql/SQLite"

poc_begin

W="$WORK/91-qt-xcb"
SRC="$W/qtbase"
BLD="$W/build"
INST="$W/inst"
APP="$W/app"
LOG="$POC_OUT/build.log"
PREFIX="${NIX_PREFIX_QT:-$W/prefix}"
mkdir -p "$W" "$BLD" "$APP" "$PREFIX" || exit 2
: > "$LOG"

RUNGFILE="$POC_OUT/RUNG-FAILURE.txt"
rung_failed() {  # rung-name summary logfile
  {
    printf 'POC 91-qt-xcb -- THE RUNG THAT STOPPED IT\n'
    printf '=========================================\n\n'
    printf 'rung      : %s\n' "$1"
    printf 'summary   : %s\n' "$2"
    printf 'date (UTC): %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'configure : %s\n\n' "${QT_CONFIGURE_LINE:-<not reached>}"
    printf -- '-- the error, verbatim, with the file it came from --------------\n\n'
    grep -nE "error:|Error|CMake Error|fatal error|undefined reference|cannot find" "$3" \
      2>/dev/null | head -60
    printf '\n-- the last 40 lines of the log --------------------------------\n\n'
    tail -40 "$3" 2>/dev/null
    printf '\nfull log: %s\n' "$3"
  } > "$RUNGFILE"
  printf '\n  ⛔ rung "%s" stopped. Recorded at %s\n' "$1" "$RUNGFILE"
}

# ---------------------------------------------------------------------------
# 1. the X libraries, static, from nixpkgs plans
#
# ⭐ nixpkgs KNOWS this graph and `pgb nix deps` builds it. The list is the xcb
# QPA plugin's own requirement set, which qtbase's configure checks one by one
# and reports as `xcb ... no` when any is missing -- a message that names the
# feature and not the library that was absent.
# ---------------------------------------------------------------------------
printf -- '-- rung 1: the X stack, static, planned by nixpkgs -------------\n'
# ⛔ THE ATTRIBUTE NAMES ARE NOT REACHABLE AND THE PLAN DOES NOT NEED THEM.
# `packages.json` carries top-level attributes; `xorg.libxcb` and its siblings
# are not among them, so `nix-fetch attr xorg.libxcb` answers "no attribute,
# pname or name". ⭐ But the qtbase plan already NAMES EVERY DEPENDENCY'S
# DERIVATION PATH -- that is what a plan is -- so the X stack is reachable
# with no index lookup at all.
#
# ⚠ AND MOST OF THAT LIST IS NOT WANTED. nixpkgs' qtbase pulls 61 build
# inputs: GTK, CUPS, MariaDB, PostgreSQL, ODBC, systemd, ICU, wayland, mesa,
# Vulkan. `-force-bundled-libs` covers freetype, harfbuzz, pcre2, zlib, libpng
# and libjpeg out of the tarball. So the KEEP list is written out and
# everything else is skipped -- the opposite of a skip list, because naming
# what is needed is checkable and naming what is not is a guess that grows.
QTPLAN="${PGB_QT_PLAN:-$W/qtbase.plan}"
if [ ! -s "$QTPLAN" ]; then
  "$PGB" nix plan qt6.qtbase --out "$QTPLAN" >>"$LOG" 2>&1 || true
fi
poc_check "a plan for qtbase was produced" \
  "$([ -s "$QTPLAN" ] && echo ok || echo failed)" ok
[ -s "$QTPLAN" ] || poc_finish

# What the xcb QPA plugin actually needs, plus TLS for QtNetwork.
# ⚠ libxml2 is on the list because libxkbcommon's meson REQUIRES it -- it
# parses xkeyboard-config's XML rulesets -- and leaving it off produced
# `Dependency "libxml-2.0" not found` on a package nothing else here needs.
QT_KEEP="libxcb libxcb-util libxcb-image libxcb-keysyms libxcb-render-util \
libxcb-wm libxcb-cursor libxdmcp libxau libx11 libxext libxrender libxi \
libxfixes libxkbcommon libxml2 openssl xorgproto xcb-proto"
SKIP=$(QT_KEEP="$QT_KEEP" QTPLAN="$QTPLAN" python3 "$POC_DIR/skiplist.py")
poc_note "keeping: $QT_KEEP"
poc_note "skipping $(printf '%s\n' $SKIP | grep -c .) of qtbase's build inputs"

NIX_PREFIX="$PREFIX" NIX_DEP_SKIP="$SKIP" \
  "$PGB" nix deps --plan "$QTPLAN" >>"$LOG" 2>&1 || true
BUILT=$(ls "$PREFIX/.built" 2>/dev/null | tr '\n' ' ')
poc_note "built into the static prefix: ${BUILT:-<none>}"
for want in libxcb libxkbcommon openssl; do
  poc_check "static $want in the prefix" \
    "$([ -e "$PREFIX/.built/$want" ] && echo ok || echo missing)" ok
done

# ---------------------------------------------------------------------------
# 2. the qtbase source (shared with poc/90-qt, fetched once)
# ---------------------------------------------------------------------------
printf -- '\n-- rung 2: the qtbase source ------------------------------------\n'
NIXOUT="$WORK/qt6-src"
TARBALL=$(ls "$NIXOUT"/*qtbase-everywhere-src-*.tar.xz 2>/dev/null | head -1)
if [ -z "$TARBALL" ]; then
  mkdir -p "$NIXOUT"
  "$PGB" nix fetch qt6.qtbase --out "$NIXOUT" >>"$LOG" 2>&1 || true
  TARBALL=$(ls "$NIXOUT"/*qtbase-everywhere-src-*.tar.xz 2>/dev/null | head -1)
fi
if [ -z "$TARBALL" ]; then
  TARBALL="$W/qtbase.tar.xz"
  poc_fetch "https://api.rv.pkgforge.dev/https://download.qt.io/archive/qt/6.11/6.11.1/submodules/qtbase-everywhere-src-6.11.1.tar.xz" \
    "$TARBALL" "$POC_SHA256" || { poc_skip "fetch qtbase" "no route to the source"; poc_finish; }
fi
poc_check "qtbase source present" "$([ -s "$TARBALL" ] && echo ok || echo failed)" ok
if [ ! -f "$SRC/configure" ]; then
  mkdir -p "$SRC"
  tar -xf "$TARBALL" -C "$SRC" --strip-components=1 >>"$LOG" 2>&1 \
    || { poc_check "qtbase source unpacks" failed ok; poc_finish; }
fi

# ---------------------------------------------------------------------------
# 3. configure and build Qt, static, WITH xcb
# ---------------------------------------------------------------------------
printf -- '\n-- rung 3: qtbase -static -xcb ---------------------------------\n'
# ⛔ `-openssl-linked` AND NOT `-openssl-runtime`. Qt's DEFAULT is to `dlopen`
# libssl at run time, which in a static binary is exactly the host-plugin
# failure docs/limitations.md §1 measures. `-openssl-linked` puts it in the
# link, which is the whole point of the exercise.
# ⚠ `-qt-sqlite` uses the copy in the tarball; the SQL driver is still built
# and registered, which is what `-no-sql` removed in poc/90-qt.
QT_CONFIGURE_LINE="-static -release -force-bundled-libs \
-xcb -xkbcommon -bundled-xcb-xinput -no-opengl -no-dbus -no-glib -no-icu \
-openssl-linked -sql-sqlite -qt-sqlite -no-cups -no-fontconfig -no-libudev \
-no-feature-testlib -qpa xcb -default-qpa xcb -nomake examples -nomake tests"

# ⛔ A STATIC ARCHIVE'S DEPENDENCIES HAVE TO COME AFTER IT, AND QT'S CONFIGURE
# TESTS DO NOT KNOW THAT. `libX11.a` calls into libxcb, so Qt's `xcb_syslibs`
# test failed with
#
#   libx11/.../src/xcb_io.c:272: undefined reference to `xcb_poll_for_event'
#
# and configure reported `TEST_xcb_syslibs = ""` -- a feature test failing for
# a link-order reason, on libraries that are all present. With shared
# libraries the loader resolves this and nobody notices; that is precisely the
# difference this project is about.
#
# ⭐ `CMAKE_C_STANDARD_LIBRARIES` is appended AFTER every other library on the
# link line, which is the one place a transitive static dependency can go.
# Repeating a library is safe: an archive member is pulled in once.
# ⛔ AND THE TAIL IS A GROUP. The xcb-util libraries call back into libxcb and
# libxcb calls into libXau/libXdmcp, so no single ordering resolves them all:
# the second attempt moved past libX11's xcb references and stopped at
# `libxcb-1.17.0/src/xcb_auth.c: undefined reference to XauGetBestAuthByAddr`.
# `--start-group` is the linker's own answer to a cycle between archives.
STATIC_TAIL="-Wl,--start-group -lxcb -lXau -lXdmcp -lxcb-xkb -lxcb-util -lxcb-image -lxcb-keysyms -lxcb-render-util -lxcb-icccm -lxcb-cursor -lxcb-render -lxcb-shm -lxcb-shape -lxcb-randr -lxcb-xfixes -lxcb-sync -lxcb-xinerama -lxcb-dri3 -lxcb-present -lxcb-glx -lxcb-xinput -lxkbcommon -lxkbcommon-x11 -Wl,--end-group -lpthread -ldl -lm"

# ⭐ QT'S CONFIG TEST IS ANSWERED WITH EVIDENCE, NOT ARGUED WITH.
#
# `TEST_xcb_syslibs` is a `try_compile` whose link line Qt composes itself,
# from an `XCB::XCB` target that carries no transitive information for a
# STATIC libxcb. It therefore fails on link order alone:
#
#   libxcb-1.17.0/src/xcb_auth.c: undefined reference to `XauGetBestAuthByAddr'
#   .../xcb_io.c: undefined reference to `xcb_poll_for_event'
#
# -- a cycle between libX11.a, libxcb.a and libXau.a, which is what
# `--start-group` exists for and which Qt's test cannot be told about through
# CMAKE_C_STANDARD_LIBRARIES (its try_compile does not use it).
#
# ⛔ SO THE OVERRIDE IS ONLY TAKEN IF THIS POC PROVES THE LINK ITSELF. The
# program below opens an xcb connection and uses xkbcommon-x11, linked exactly
# the way the Qt build will link it. If THAT fails, the override is not passed
# and the rung is recorded as stopped -- which is the point: an override
# nobody checked is a way of making a build succeed by lying to it.
printf -- '\n-- rung 2b: proving the static xcb link before overriding Qt ----\n'
cat > "$W/xcbprobe.c" <<'EOF'
#include <xcb/xcb.h>
#include <xcb/xkb.h>
#include <xkbcommon/xkbcommon-x11.h>
#include <X11/Xlib.h>
#include <stdio.h>
int main(void)
{
    int screen = 0;
    xcb_connection_t *c = xcb_connect(NULL, &screen);
    int err = c ? xcb_connection_has_error(c) : -1;
    /* referenced so the xkbcommon-x11 archive is really needed */
    int dev = c && !err ? xkb_x11_get_core_keyboard_device_id(c) : -1;
    if (c) xcb_disconnect(c);
    printf("xcb=%d xkb_device=%d\n", err, dev);
    return 0;
}
EOF
POC_PGB_FLAGS="--bind $PREFIX" \
poc_in_env "cd $W && \$CC -I$PREFIX/include -o xcbprobe xcbprobe.c \
   -L$PREFIX/lib $STATIC_TAIL" >>"$LOG" 2>&1
XCBLINK=$([ -x "$W/xcbprobe" ] && echo ok || echo failed)
poc_check "a static xcb + xkbcommon-x11 program links" "$XCBLINK" ok
if [ "$XCBLINK" != ok ]; then
  rung_failed "static xcb link" "the xcb archives do not link even in a group" "$LOG"
  poc_finish
fi
poc_check "and it is static (no PT_INTERP)" \
  "$(readelf -l "$W/xcbprobe" 2>/dev/null | grep -c INTERP)" "0"
QT_TEST_OVERRIDES="-DTEST_xcb_syslibs=ON"
poc_note "so Qt is told TEST_xcb_syslibs=ON, with the link above as the evidence"

if [ ! -x "$INST/bin/qmake" ]; then
  # ⛔ `CMakeCache.txt` IS NOT A CONFIGURE-SUCCEEDED MARKER, and treating it as
  # one cost a whole run. A configure that dies part way still leaves one, so a
  # rerun SKIPPED configure, went straight to `cmake --build .` and reported
  # `ninja: error: loading 'build.ninja': No such file or directory` -- an
  # error about a missing file that is really a stale directory. `build.ninja`
  # is written last, so it is the marker.
  if [ ! -f "$BLD/build.ninja" ]; then
    rm -rf "$BLD"; mkdir -p "$BLD"
    POC_PGB_FLAGS="--bind $PREFIX" \
    # ⛔ QT FINDS XCB THROUGH CMake, NOT THROUGH LDFLAGS, and the first attempt
    # gave it only PKG_CONFIG_PATH and -L. What it printed was
    #
    #   ERROR: Feature "xcb": Forcing to "ON" breaks its condition:
    #       ... AND TARGET XCB::XCB AND TEST_xcb_syslibs AND ...
    #       TARGET XCB::XCB not found
    #
    # -- a feature error about a library that is built and installed. Qt's
    # configure passes everything after `--` to cmake, so CMAKE_PREFIX_PATH is
    # how the static prefix becomes visible to `find_package`. ⚠ And
    # `share/pkgconfig` belongs on PKG_CONFIG_PATH for the same reason it does
    # inside `pgb nix`: xcb-proto and xorgproto put their .pc files there.
    poc_in_env "cd $BLD && PKG_CONFIG_PATH=$PREFIX/lib/pkgconfig:$PREFIX/share/pkgconfig \
       PKG_CONFIG='pkg-config --static' \
       CFLAGS=-I$PREFIX/include CXXFLAGS=-I$PREFIX/include LDFLAGS=-L$PREFIX/lib \
       OPENSSL_ROOT_DIR=$PREFIX \
       $SRC/configure -prefix $INST $QT_CONFIGURE_LINE \
       -- -DCMAKE_PREFIX_PATH=$PREFIX $QT_TEST_OVERRIDES \
       -DCMAKE_C_STANDARD_LIBRARIES='$STATIC_TAIL' \
       -DCMAKE_CXX_STANDARD_LIBRARIES='$STATIC_TAIL'" >>"$LOG" 2>&1 \
      || { poc_check "qtbase configures with xcb" failed ok
           rung_failed "qtbase configure -xcb" "./configure refused" "$LOG"; poc_finish; }
  fi
  poc_check "qtbase configures with xcb" \
    "$([ -f "$BLD/build.ninja" ] && echo ok || echo failed)" ok
  POC_PGB_FLAGS="--bind $PREFIX" \
  poc_in_env "cd $BLD && cmake --build . --parallel \$(nproc)" >>"$LOG" 2>&1 \
    || { poc_check "qtbase builds" failed ok
         rung_failed "qtbase build" "cmake --build failed" "$LOG"; poc_finish; }
  POC_PGB_FLAGS="--bind $PREFIX" \
  poc_in_env "cd $BLD && cmake --install ." >>"$LOG" 2>&1 \
    || { poc_check "qtbase installs" failed ok
         rung_failed "qtbase install" "cmake --install failed" "$LOG"; poc_finish; }
fi
poc_check "qtbase installed" "$([ -x "$INST/bin/qmake" ] && echo ok || echo failed)" ok

# ⭐ THE PLUGIN THAT MATTERS. A static Qt emits plugins as ARCHIVES; if
# libqxcb.a is not there, the platform plugin was not built and every window
# assertion below would fail for a reason that has nothing to do with X.
XCBA=$(find "$INST" -name 'libqxcb.a' 2>/dev/null | head -1)
poc_check "the xcb platform plugin was built (as an archive)" \
  "$([ -n "$XCBA" ] && echo ok || echo missing)" ok
[ -n "$XCBA" ] && poc_note "$(ls -l "$XCBA" | awk '{print $5}') bytes  $XCBA"
NSO=$(find "$INST" -name '*.so' -o -name '*.so.*' 2>/dev/null | grep -c . || true)
poc_check "and NOT ONE shared object under the prefix" "${NSO:-0}" "0"

# ---------------------------------------------------------------------------
# 4. the application: it opens a window
# ---------------------------------------------------------------------------
printf -- '\n-- rung 4: an application that opens a window -------------------\n'
mkdir -p "$APP"
cat > "$APP/CMakeLists.txt" <<'EOF'
cmake_minimum_required(VERSION 3.22)
project(qtxcbprobe LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets Network Sql)
qt_standard_project_setup()
qt_add_executable(qtxcbprobe main.cpp)
target_link_libraries(qtxcbprobe PRIVATE Qt6::Core Qt6::Gui Qt6::Widgets Qt6::Network Qt6::Sql)
EOF
cat > "$APP/main.cpp" <<'EOF'
// The probe. ⛔ Not --version: every check below needs the X server to have
// answered, the window to have been mapped, and the paint to have happened.
#include <QApplication>
#include <QGuiApplication>
#include <QScreen>
#include <QWidget>
#include <QLabel>
#include <QPixmap>
#include <QImage>
#include <QPainter>
#include <QTimer>
#include <QWindow>
#include <QSslSocket>
#include <QSqlDatabase>
#include <QSqlQuery>
#include <QStringList>
#include <QTextStream>
#include <QThread>
#include <cstdio>

static int failures = 0;
static void check(const char *what, bool ok, const QString &saw)
{
    std::printf("  %-46s %s   %s\n", what, ok ? "ok  " : "FAIL",
                saw.toUtf8().constData());
    if (!ok) ++failures;
}

int main(int argc, char **argv)
{
    QApplication app(argc, argv);

    // 1. the platform plugin actually in use
    const QString qpa = QGuiApplication::platformName();
    check("QPA platform plugin", qpa == QLatin1String("xcb"), qpa);

    // 2. the X server answered: a screen with a real geometry
    QScreen *scr = QGuiApplication::primaryScreen();
    const QSize sz = scr ? scr->size() : QSize();
    check("primary screen from the X server", scr && sz.width() >= 640 && sz.height() >= 480,
          QString("%1x%2").arg(sz.width()).arg(sz.height()));

    // 3. a real, mapped window
    QWidget w;
    w.setWindowTitle(QStringLiteral("pgb xcb probe"));
    w.resize(320, 240);
    QLabel *lbl = new QLabel(QStringLiteral("pgb"), &w);
    lbl->setGeometry(0, 0, 320, 240);
    w.show();
    app.processEvents();
    QWindow *win = w.windowHandle();
    check("QWidget::show() produced a QWindow", win != nullptr,
          win ? QStringLiteral("winId=%1").arg(w.winId()) : QStringLiteral("<none>"));
    check("the window handle has a native id", win && w.winId() != 0,
          QString::number(static_cast<qulonglong>(w.winId())));
    // ⭐ THE MAPPED CHECK. isExposed() is true only once the X server has told
    // us the window is on screen -- an offscreen QPA can never satisfy it.
    for (int i = 0; i < 200 && win && !win->isExposed(); ++i) {
        app.processEvents();
        QThread::msleep(5);
    }
    check("the X server exposed the window", win && win->isExposed(),
          win && win->isExposed() ? QStringLiteral("exposed") : QStringLiteral("not exposed"));

    // 4. render it and read a pixel back
    QPixmap shot = w.grab();
    check("QWidget::grab() of the mapped window", !shot.isNull() && shot.width() == 320,
          QString("%1x%2").arg(shot.width()).arg(shot.height()));

    // 5. Qt Network with OpenSSL LINKED IN, not dlopen'd
    check("QSslSocket::supportsSsl()", QSslSocket::supportsSsl(),
          QSslSocket::sslLibraryBuildVersionString());
    check("the linked TLS backend names OpenSSL",
          QSslSocket::sslLibraryVersionString().contains(QLatin1String("OpenSSL")),
          QSslSocket::sslLibraryVersionString());

    // 6. QtSql, with the SQLite driver registered and a real query
    check("QSQLITE driver is registered",
          QSqlDatabase::drivers().contains(QLatin1String("QSQLITE")),
          QSqlDatabase::drivers().join(QLatin1Char(',')));
    {
        QSqlDatabase db = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"));
        db.setDatabaseName(QStringLiteral(":memory:"));
        bool opened = db.open();
        QSqlQuery q(db);
        // ⛔ QStringLiteral TREATS ITS BYTES AS LATIN-1, so a UTF-8 escape in
        // the SQL text went in as two mojibake characters and came back as
        // `\u00e6\u00a5\u00e6\u00ac` -- a FAILURE that was the test's, not
        // the database's. Both sides are built with fromUtf8 from the same
        // constant now, so the assertion is about the round trip.
        const QString jp = QString::fromUtf8("\xe6\x97\xa5\xe6\x9c\xac");
        bool made = opened && q.exec(QStringLiteral("create table t(a text)"))
                    && q.exec(QStringLiteral("insert into t values('") + jp + QStringLiteral("')"))
                    && q.exec(QStringLiteral("select a from t")) && q.next();
        check("a real SQLite query round trip", made && q.value(0).toString() == jp,
              made ? q.value(0).toString() : QStringLiteral("<no row>"));
    }

    // 7. the event loop still returns cleanly
    QTimer::singleShot(0, &app, &QCoreApplication::quit);
    const int rc = app.exec();
    check("QApplication::exec() returned 0", rc == 0, QString::number(rc));

    std::printf("failures: %d\n", failures);
    return failures == 0 ? 0 : 1;
}
EOF

if [ ! -x "$APP/build/qtxcbprobe" ]; then
  POC_PGB_FLAGS="--bind $PREFIX" \
  # ⛔ THE APPLICATION LINK NEEDS THE SAME TAIL AS THE CONFIG TEST DID, and for
  # the same reason one layer out: Qt's own CMake config for a STATIC build
  # records `XCB::XCB` but not libxcb's transitive libXau/libXdmcp, so the
  # probe failed with `undefined reference to XauGetBestAuthByAddr` after
  # qtbase itself had built and installed perfectly.
  poc_in_env "cd $APP && cmake -S . -B build -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_PREFIX_PATH='$INST;$PREFIX' \
      -DCMAKE_CXX_STANDARD_LIBRARIES='-L$PREFIX/lib $STATIC_TAIL' \
      -DCMAKE_C_STANDARD_LIBRARIES='-L$PREFIX/lib $STATIC_TAIL' \
      && cmake --build build --parallel \$(nproc)" \
    >>"$LOG" 2>&1 \
    || { poc_check "the probe builds against static Qt" failed ok
         rung_failed "probe build" "cmake failed for the application" "$LOG"; poc_finish; }
fi
BIN="$APP/build/qtxcbprobe"
poc_check "the probe builds against static Qt" \
  "$([ -x "$BIN" ] && echo ok || echo failed)" ok
[ -x "$BIN" ] || poc_finish
poc_note "$(wc -c < "$BIN") bytes"
poc_check "no PT_INTERP" \
  "$(readelf -l "$BIN" 2>/dev/null | grep -c INTERP)" "0"
poc_check "no DT_NEEDED" \
  "$(readelf -d "$BIN" 2>/dev/null | grep -c NEEDED)" "0"

# ---------------------------------------------------------------------------
# 5. the X server, and the matrix
# ---------------------------------------------------------------------------
printf -- '\n-- rung 5: the eleven, against a real X server ------------------\n'
XDISP="${PGB_QT_XDISPLAY:-:91}"
XPORT=$((6000 + ${XDISP#:}))
if ! command -v Xvfb >/dev/null 2>&1; then
  poc_skip "the eleven-environment matrix" "no Xvfb on the build host"
  poc_finish
fi
# ⛔ `-ac` and `-listen tcp`: the targets reach the server over loopback
# because the bed shares the host's network namespace. A unix socket cannot be
# staged into eleven root filesystems by copying.
Xvfb "$XDISP" -screen 0 1024x768x24 -ac -listen tcp >"$POC_OUT/xvfb.log" 2>&1 &
XVFB_PID=$!
trap 'kill $XVFB_PID 2>/dev/null' EXIT INT TERM
i=0
while [ $i -lt 50 ]; do
  if command -v xdpyinfo >/dev/null 2>&1; then
    DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1 && break
  else
    # No xdpyinfo: wait for the port to answer instead.
    (exec 3<>/dev/tcp/127.0.0.1/$XPORT) 2>/dev/null && break
  fi
  sleep 0.2; i=$((i + 1))
done
poc_check "Xvfb is answering on 127.0.0.1:${XDISP#:}" \
  "$(kill -0 $XVFB_PID 2>/dev/null && echo ok || echo failed)" ok

# ⭐ The probe runs against the host's X server FIRST, so a failure inside a
# target can be told apart from a probe that never worked anywhere.
DISPLAY="$XDISP" "$BIN" > "$POC_OUT/host-run.txt" 2>&1
HOSTRC=$?
poc_check "the probe passes on the build host" "$HOSTRC" "0"
sed 's/^/        /' "$POC_OUT/host-run.txt"

poc_functional_test() {
  cat <<SH
#!/bin/sh
HOME=/tmp; export HOME
TMPDIR=/tmp; export TMPDIR
DISPLAY=127.0.0.1${XDISP}; export DISPLAY
QT_QPA_PLATFORM=xcb; export QT_QPA_PLATFORM
exec /qtxcbprobe
SH
}

poc_matrix "$BIN"
poc_finish
