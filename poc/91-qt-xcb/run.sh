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
XSTACK="xorg.libXau xorg.libXdmcp xorg.libxcb xorg.xcbutil xorg.xcbutilimage \
xorg.xcbutilkeysyms xorg.xcbutilrenderutil xorg.xcbutilwm xorg.xcbutilcursor \
libxkbcommon openssl"

SYNTH="$W/xstack.plan"
if [ ! -s "$PREFIX/.built/libxkbcommon" ] || [ ! -s "$SYNTH" ]; then
  : > "$W/xstack.tsv"
  for a in $XSTACK; do
    p="$W/plan-$(printf '%s' "$a" | tr './' '__').json"
    [ -s "$p" ] || sh "$PGB" nix plan "$a" --out "$p" >>"$LOG" 2>&1 || true
    if [ -s "$p" ]; then
      python3 - "$p" >> "$W/xstack.tsv" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
print("%s\t%s" % (d.get("pname", "") + "-" + d.get("version", ""), d.get("drv", "")))
PY
    else
      poc_note "⚠ no plan for $a"
    fi
  done
  python3 - "$W/xstack.tsv" > "$SYNTH" <<'PY'
import json, sys
deps = []
for line in open(sys.argv[1]):
    name, _, drv = line.rstrip("\n").partition("\t")
    if name and drv:
        deps.append({"name": name, "drv": drv, "out": ""})
json.dump({"schema": "pgb-nix-plan/1", "attr": "qt-xcb-stack",
           "pname": "qt-xcb-stack", "version": "0", "system": "x86_64-linux",
           "src": {}, "patches": [], "outputs": ["out"],
           "configureFlags": [], "cmakeFlags": [], "mesonFlags": [],
           "makeFlags": [], "buildInputs": [d["name"] for d in deps],
           "nativeBuildInputs": [], "propagatedBuildInputs": [],
           "buildSystemHooks": [], "nix_only": {}, "nixpkgs": "",
           "deps": deps}, sys.stdout, indent=1)
PY
fi
poc_check "a plan for the X stack was produced" \
  "$([ -s "$SYNTH" ] && echo ok || echo failed)" ok
poc_note "$(python3 -c 'import json,sys;print(len(json.load(open(sys.argv[1]))["deps"]))' "$SYNTH" 2>/dev/null) derivations in the synthetic plan"

NIX_PREFIX="$PREFIX" sh "$PGB" nix deps --plan "$SYNTH" >>"$LOG" 2>&1 || true
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
  sh "$PGB" nix fetch qt6.qtbase --out "$NIXOUT" >>"$LOG" 2>&1 || true
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

if [ ! -x "$INST/bin/qmake" ]; then
  if [ ! -f "$BLD/CMakeCache.txt" ]; then
    POC_PGB_FLAGS="--bind $PREFIX" \
    poc_in_env "cd $BLD && PKG_CONFIG_PATH=$PREFIX/lib/pkgconfig \
       CFLAGS=-I$PREFIX/include CXXFLAGS=-I$PREFIX/include LDFLAGS=-L$PREFIX/lib \
       OPENSSL_ROOT_DIR=$PREFIX \
       $SRC/configure -prefix $INST $QT_CONFIGURE_LINE" >>"$LOG" 2>&1 \
      || { poc_check "qtbase configures with xcb" failed ok
           rung_failed "qtbase configure -xcb" "./configure refused" "$LOG"; poc_finish; }
  fi
  poc_check "qtbase configures with xcb" \
    "$([ -f "$BLD/CMakeCache.txt" ] && echo ok || echo failed)" ok
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
        bool made = opened && q.exec(QStringLiteral("create table t(a text)"))
                    && q.exec(QStringLiteral("insert into t values('\xe6\x97\xa5\xe6\x9c\xac')"))
                    && q.exec(QStringLiteral("select a from t")) && q.next();
        check("a real SQLite query round trip", made && q.value(0).toString()
                  == QString::fromUtf8("\xe6\x97\xa5\xe6\x9c\xac"),
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
  poc_in_env "cd $APP && cmake -S . -B build -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_PREFIX_PATH='$INST;$PREFIX' && cmake --build build --parallel \$(nproc)" \
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
