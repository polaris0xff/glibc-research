#!/bin/sh
# poc/90-qt -- a Qt 6 WIDGET program, statically linked against glibc, on all
# eleven pinned environments.
#
# -- ⛔ WHY THIS EXISTS, AND WHAT IT IS NOT --------------------------------
#
# `poc/80-mlt` climbed kdenlive's ladder as far as its ENGINE -- ffmpeg 7.1
# and MLT 7.30.0, a 105 MB static `melt` rendering a real MP4 on 11 of 11 --
# and then wrote, in its own output:
#
#     Qt 6 / KDE Frameworks    ⛔ NOT ATTEMPTED -- the next rung
#     kdenlive                 ⛔ NOT ATTEMPTED
#
# ⭐ NOT ATTEMPTED IS NOT A FAILURE. Nobody had run it. TODO T-054 says the
# evidence points the other way -- Qt supports `-static` upstream, and Qt's
# plugins (QPA, image formats, SQL drivers) are the application's OWN plugins,
# which is the class `--wrap-dlopen` already serves on 11 of 11 in POC 70.
# This POC is rung 1 of four: a Qt 6 *widget* program, static.
#
# ⛔ IT DOES NOT BUILD KDENLIVE and does not claim to. The remaining rungs --
# a QPA platform plugin under --wrap-dlopen, then KF6, then kdenlive -- are
# named in T-054 and in "the depth reached" below.
#
# -- WHERE THE SOURCE COMES FROM ------------------------------------------
#
# ⭐ nixpkgs is the planner, which is this project's route since `pgb nix`:
# `pgb nix plan qt6.qtbase` evaluates the nixpkgs derivation and `pgb nix
# fetch` pulls the source from cache.nixos.org, signed and NarHash-checked.
# What comes back is qtbase-everywhere-src-6.11.1.tar.xz, an ordinary upstream
# tarball.
#
# ⛔ NIXPKGS' ELEVEN PATCHES ARE DELIBERATELY NOT APPLIED, and that is a
# recorded decision rather than an oversight. They exist to make Qt behave
# inside the nix store -- `derive-plugin-load-path-from-PATH`,
# `allow-translations-outside-prefix`, `qmake-fix-includedir`,
# `find-qmlimportscanner` and so on. Every one of them is about finding things
# at RUN TIME on disk, which is exactly what a static build with compiled-in
# plugins does not do. Applying them would also cost this POC the property
# every other POC in this tree has: stock tarball, stock configure, no source
# patches. If a later rung needs one, apply it and say which.
#
# ⚠ nixpkgs' own qtbase is a SHARED, feature-complete build with gtk3,
# at-spi2, mariadb and libpq in its buildInputs. This POC does not build that
# graph: it configures the same source with `-static -force-bundled-libs` and
# the feature set a headless widget program needs. The full-featured static
# graph is a later rung and the deviation is named here so it is not read as
# "nixpkgs' qtbase builds static".
#
# -- WHAT IS ACTUALLY EXERCISED -------------------------------------------
#
# ⛔ NOT `--version`. poc/common.sh's rule. The probe drives, inside every
# target: the QPA platform plugin (statically imported, so `dlopen` is never
# reached), Qt's CLDR locale data compiled in, a QImage through the raster
# paint engine with a pixel read back, a QWidget rendered with QWidget::grab(),
# the event loop with QTimer, QFile I/O and a Latin-1 QStringConverter round
# trip. Its exit status is the result.
#
# -- IF A RUNG STOPS IT ----------------------------------------------------
#
# T-054 closes one of two ways and this script produces both: a static Qt
# program on the matrix, or `evidence/poc/90-qt/RUNG-FAILURE.txt` naming the
# rung, the error and the file it came from -- the shape
# `evidence/72-static-host-plugin-abi/CPYTHON-FAILURE.txt` already uses.

set -u
. "$(dirname "$0")/../common.sh"

POC_WHY="Qt 6 widgets, static glibc, on eleven environments -- T-054 rung 1"
POC_URL="nixpkgs qt6.qtbase -> qtbase-everywhere-src-6.11.1.tar.xz"
POC_VERSION="Qt 6.11.1 (qtbase)"
POC_SHA256="d9594a31228aa23ad6b531719a29b45f0f3989fe6c136d45767ea179f233c1ac"
POC_NORMAL_BUILD="./configure -prefix ... && cmake --build . && cmake --install ."
POC_STRESSES="a very large C++ build system, static plugin import, QPA, raster paint, CLDR data"

poc_begin

W="$WORK/90-qt"
SRC="$W/qtbase"
# ⚠ Qt's ./configure takes the SOURCE tree from its own path and the BUILD
# tree from $PWD, so the two must differ. Running it from inside the source
# is how a first attempt gets an in-source build nobody asked for.
BLD="$W/build"
INST="$W/inst"
APP="$W/app"
LOG="$POC_OUT/build.log"
mkdir -p "$W" "$BLD" "$APP" || exit 2
: > "$LOG"

# ⛔ The rung recorder. T-054 may NOT close as "impossible"; it closes with the
# rung that stopped it, at file and line. Every failure path below calls this.
RUNGFILE="$POC_OUT/RUNG-FAILURE.txt"
rung_failed() {  # rung-name  one-line-summary  logfile
  {
    printf 'POC 90-qt -- THE RUNG THAT STOPPED IT\n'
    printf '=====================================\n\n'
    printf 'rung      : %s\n' "$1"
    printf 'summary   : %s\n' "$2"
    printf 'date (UTC): %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf 'source    : %s\n' "$POC_URL"
    printf 'configure : %s\n\n' "${QT_CONFIGURE_LINE:-<not reached>}"
    printf -- '-- the error, verbatim, with the file it came from --------------\n\n'
    # Lines that name a file, and the errors around them. ⚠ head, not tail:
    # the FIRST error is the cause; the later ones are usually its consequences.
    grep -nE "error:|Error|CMake Error|fatal error|undefined reference|cannot find" "$3" \
      2>/dev/null | head -60
    printf '\n-- the last 40 lines of the log --------------------------------\n\n'
    tail -40 "$3" 2>/dev/null
    printf '\nfull log: %s\n' "$3"
  } > "$RUNGFILE"
  printf '\n  ⛔ rung "%s" stopped. Recorded at %s\n' "$1" "$RUNGFILE"
}

# ---------------------------------------------------------------------------
# 1. the source, through nixpkgs
# ---------------------------------------------------------------------------
printf -- '-- rung 1: the source, planned by nixpkgs ----------------------\n'
NIXOUT="$WORK/qt6-src"
TARBALL=$(ls "$NIXOUT"/*qtbase-everywhere-src-*.tar.xz 2>/dev/null | head -1)
if [ -z "$TARBALL" ]; then
  mkdir -p "$NIXOUT"
  sh "$PGB" nix fetch qt6.qtbase --out "$NIXOUT" >>"$LOG" 2>&1 || true
  TARBALL=$(ls "$NIXOUT"/*qtbase-everywhere-src-*.tar.xz 2>/dev/null | head -1)
fi
# ⚠ The fallback is upstream, through the RV proxy, pinned to the same digest
# the nixpkgs plan named -- so a machine with no nix can still run this POC.
if [ -z "$TARBALL" ]; then
  TARBALL="$W/qtbase.tar.xz"
  poc_fetch "https://api.rv.pkgforge.dev/https://download.qt.io/archive/qt/6.11/6.11.1/submodules/qtbase-everywhere-src-6.11.1.tar.xz" \
    "$TARBALL" "$POC_SHA256" || { poc_skip "fetch qtbase" "no nix route and no upstream route"; poc_finish; }
fi
poc_check "qtbase source present" "$([ -s "$TARBALL" ] && echo ok || echo failed)" ok
poc_note "$(sha256sum "$TARBALL" | cut -c1-64)  $(basename "$TARBALL")"
poc_note "nixpkgs named 11 patches; NONE applied -- see the header for why"

if [ ! -f "$SRC/configure" ]; then
  mkdir -p "$SRC" && tar xf "$TARBALL" -C "$SRC" --strip-components=1 \
    || { poc_check "unpack qtbase" failed ok; poc_finish; }
fi
poc_check "stock tarball, unpatched" \
  "$([ -x "$SRC/configure" ] && echo ok || echo failed)" ok

# ---------------------------------------------------------------------------
# 2. qtbase, static, through pgb
# ---------------------------------------------------------------------------
printf -- '\n-- rung 2: qtbase 6.11.1, -static, through pgb -----------------\n'
#
# ⭐ `-force-bundled-libs` is what makes this tractable: Qt ships zlib, pcre2,
# libpng, libjpeg, freetype, harfbuzz, double-conversion and md4c in
# src/3rdparty, so the whole dependency tree is INSIDE the tarball and pgb
# links it statically along with everything else. Without it, configure
# autodetects the build environment's shared libraries and the static link
# fails on each one in turn.
#
# ⭐ `-qpa offscreen -default-qpa offscreen` is the choice that makes a
# headless matrix meaningful. offscreen is a real QPA plugin doing real
# raster work; it just has no display server behind it, which none of the
# eleven has either.
#
# ⚠ The disabled features are the ones that would reach OUT of the tarball:
# xcb, egl, opengl, dbus, glib, icu, openssl, cups, fontconfig, libudev and
# the input backends all need host development packages, and every one of
# them is a shared library this project would then have to keep out of the
# link. Network, sql and testlib are off because a widget program does not
# need them and they are build time this session does not have.
QT_CONFIGURE_LINE="-static -release -force-bundled-libs -no-dbus -no-opengl \
-no-egl -no-icu -no-glib -no-openssl -no-cups -no-fontconfig -no-xcb \
-no-libudev -no-evdev -no-libinput -no-mtdev -no-eglfs -no-gbm -no-kms \
-no-linuxfb -no-directfb -no-feature-network -no-feature-sql \
-no-feature-testlib -no-feature-printsupport -qpa offscreen \
-default-qpa offscreen -nomake examples -nomake tests"

if [ ! -f "$INST/lib/libQt6Widgets.a" ]; then
  printf '  building qtbase (this is the long pole: hours, not minutes)\n'
  # ⚠ Guarded on CMakeCache.txt, not on the install: a re-run after a build
  # that stopped half way should not pay four minutes of configure again.
  if [ ! -f "$BLD/CMakeCache.txt" ]; then
    poc_in_env "cd $BLD && $SRC/configure -prefix $INST $QT_CONFIGURE_LINE" \
      >>"$LOG" 2>&1 \
      || { poc_check "qtbase configures through pgb" failed ok
           rung_failed "qtbase configure" "./configure refused" "$LOG"; poc_finish; }
  fi
  poc_check "qtbase configures through pgb" \
    "$([ -f "$BLD/CMakeCache.txt" ] && echo ok || echo failed)" ok
  poc_in_env "cd $BLD && cmake --build . --parallel \$(nproc)" >>"$LOG" 2>&1 \
    || { poc_check "qtbase builds through pgb" failed ok
         rung_failed "qtbase build" "cmake --build failed" "$LOG"; poc_finish; }
  poc_in_env "cd $BLD && cmake --install ." >>"$LOG" 2>&1 \
    || { poc_check "qtbase installs" failed ok
         rung_failed "qtbase install" "cmake --install failed" "$LOG"; poc_finish; }
fi
poc_check "qtbase builds through pgb" \
  "$([ -f "$INST/lib/libQt6Widgets.a" ] && echo ok || echo failed)" ok
for a in Core Gui Widgets; do
  [ -f "$INST/lib/libQt6$a.a" ] && \
    poc_note "libQt6$a.a $(wc -c < "$INST/lib/libQt6$a.a") bytes"
done
# ⛔ THE PROPERTY THAT MATTERS AT THIS RUNG: no shared object came out. A Qt
# that built .so files would mean -static was not honoured, and the link
# below would have succeeded against them without anybody noticing.
poc_check "qtbase produced NO shared libraries" \
  "$(find "$INST" -name 'libQt6*.so*' 2>/dev/null | wc -l)" 0
poc_check "the offscreen QPA plugin is a static archive" \
  "$([ -f "$INST/lib/qt6/plugins/platforms/libqoffscreen.a" ] && echo ok || echo failed)" ok

# ---------------------------------------------------------------------------
# 3. the probe: a real Qt Widgets program
# ---------------------------------------------------------------------------
printf -- '\n-- rung 3: a Qt Widgets program, linked static -----------------\n'
cat > "$APP/CMakeLists.txt" <<'CMAKE'
cmake_minimum_required(VERSION 3.22)
project(qtprobe LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets)
# ⭐ qt_add_executable, not add_executable: on a STATIC Qt this is what
# generates the plugin-import translation unit, so Q_IMPORT_PLUGIN for the
# offscreen QPA plugin is emitted by Qt's own build system rather than written
# by hand. That is Qt reaching the same answer --wrap-dlopen reaches from
# pgb's side: the plugin is in the link, so dlopen is never called.
qt_add_executable(qtprobe main.cpp)
target_link_libraries(qtprobe PRIVATE Qt6::Core Qt6::Gui Qt6::Widgets)
CMAKE

cat > "$APP/main.cpp" <<'CPP'
// The functional test, and it is not --version. Everything here runs inside
// each of the eleven target environments.
#include <QtWidgets/QApplication>
#include <QtWidgets/QWidget>
#include <QtGui/QGuiApplication>
#include <QtGui/QImage>
#include <QtGui/QPainter>
#include <QtGui/QPixmap>
#include <QtCore/QByteArray>
#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QLocale>
#include <QtCore/QString>
#include <QtCore/QStringConverter>
#include <QtCore/QTimer>
#include <cstdio>

static int failures = 0;

static void check(const char *what, bool ok, const QString &got)
{
    std::printf("  %-40s %-5s %s\n", what, ok ? "ok" : "FAIL",
                got.toUtf8().constData());
    if (!ok)
        ++failures;
}

int main(int argc, char **argv)
{
    QApplication app(argc, argv);

    // 1. The QPA plugin. It is compiled in, so this answering "offscreen"
    //    means a plugin was found without dlopen ever being called.
    const QString qpa = QGuiApplication::platformName();
    check("QPA platform plugin", qpa == QLatin1String("offscreen"), qpa);

    // 2. Unicode through QString: CJK plus an astral-plane codepoint, so the
    //    surrogate pair path is exercised and not just Latin-1 in disguise.
    const QString u = QString::fromUtf8("\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e"
                                        "\xf0\x9f\x8e\xac");
    const QByteArray u8 = u.toUtf8();
    check("UTF-8 round trip", QString::fromUtf8(u8) == u && u.size() == 5,
          QString::number(u.size()) + QLatin1String(" utf16 units, ")
              + QString::number(u8.size()) + QLatin1String(" bytes"));

    // 3. Qt's own CLDR data, compiled in. ⭐ This is the locale result from
    //    Qt's side: the answer must be German whatever the host's locale is,
    //    and four of the eleven have no glibc locale data at all.
    const QString de = QLocale(QLocale::German, QLocale::Germany).toString(1234.5, 'f', 1);
    check("QLocale(de_DE) formats 1234.5", de == QString::fromUtf8("1.234,5"), de);

    // 4. The system locale, reported not asserted: it is host data, and
    //    poc/common.sh's rule is that host DATA is observed, never required.
    check("QLocale::system() answers", !QLocale::system().name().isEmpty(),
          QLocale::system().name());

    // 5. The raster paint engine, with the pixel read back. This is real
    //    QtGui work, not construction.
    QImage img(64, 48, QImage::Format_RGB32);
    img.fill(QColor(0, 0, 255));
    {
        QPainter p(&img);
        p.fillRect(16, 12, 32, 24, QColor(255, 0, 0));
    }
    const bool raster = img.pixelColor(0, 0) == QColor(0, 0, 255)
        && img.pixelColor(32, 24) == QColor(255, 0, 0);
    check("QPainter fills a QImage", raster,
          QString::number(img.width()) + QLatin1Char('x')
              + QString::number(img.height()));

    // 6. A real QWidget rendered through the widget stack.
    QWidget w;
    w.resize(120, 90);
    QPalette pal = w.palette();
    pal.setColor(QPalette::Window, QColor(0, 200, 0));
    w.setPalette(pal);
    w.setAutoFillBackground(true);
    const QPixmap shot = w.grab();
    const QImage shotimg = shot.toImage();
    check("QWidget::grab renders the widget",
          shot.size() == QSize(120, 90)
              && shotimg.pixelColor(60, 45) == QColor(0, 200, 0),
          QString::number(shot.width()) + QLatin1Char('x')
              + QString::number(shot.height()));

    // 7. QFile, in the target's own filesystem.
    const QString path = QDir::tempPath() + QLatin1String("/pgb-qtprobe.txt");
    bool io = false;
    {
        QFile f(path);
        if (f.open(QIODevice::WriteOnly)) {
            f.write(u8);
            f.close();
            QFile g(path);
            if (g.open(QIODevice::ReadOnly)) {
                io = (g.readAll() == u8);
                g.close();
            }
        }
    }
    QFile::remove(path);
    check("QFile write and read back", io, path);

    // 8. A QStringConverter that is not UTF-8. Qt 6 carries Latin-1 and the
    //    UTF family itself; with -no-icu there is no host codec path at all,
    //    which is the property being demonstrated.
    QStringEncoder enc(QStringConverter::Latin1);
    QStringDecoder dec(QStringConverter::Latin1);
    const QString cafe = QString::fromUtf8("caf\xc3\xa9");
    const QByteArray l1 = enc.encode(cafe);
    check("Latin-1 QStringConverter round trip",
          l1.size() == 4 && QString(dec.decode(l1)) == cafe,
          QString::number(l1.size()) + QLatin1String(" bytes"));

    // 9. The event loop and the Unix event dispatcher (-no-glib, so this is
    //    Qt's own, not glib's).
    QTimer::singleShot(0, &app, &QCoreApplication::quit);
    const int rc = app.exec();
    check("QTimer drives QApplication::exec", rc == 0, QString::number(rc));

    std::printf("QTPROBE %s (%d failed)\n", failures ? "FAILED" : "OK", failures);
    return failures ? 1 : 0;
}
CPP

if [ ! -x "$APP/build/qtprobe" ]; then
  poc_in_env "cd $APP && cmake -S . -B build -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_PREFIX_PATH=$INST && cmake --build build --parallel \$(nproc)" \
    >"$POC_OUT/app-build.log" 2>&1 \
    || { poc_check "the Qt program links static" failed ok
         rung_failed "linking a Qt Widgets program" \
           "cmake/link of the probe failed against the static Qt" \
           "$POC_OUT/app-build.log"; poc_finish; }
fi
poc_check "the Qt program links static" \
  "$([ -x "$APP/build/qtprobe" ] && echo ok || echo failed)" ok
[ -x "$APP/build/qtprobe" ] || poc_finish

poc_inspect "$APP/build/qtprobe"
poc_check "no PT_INTERP" \
  "$(readelf -lW "$APP/build/qtprobe" 2>/dev/null | grep -c INTERP)" 0
poc_check "no DT_NEEDED" \
  "$(readelf -dW "$APP/build/qtprobe" 2>/dev/null | grep -c NEEDED)" 0

# ---------------------------------------------------------------------------
# 4. the matrix
# ---------------------------------------------------------------------------
poc_functional_test() {
  cat <<'SH'
#!/bin/sh
# ⚠ No QT_QPA_PLATFORM is set on purpose: the binary was configured with
# -default-qpa offscreen, so if it reaches "offscreen" it did so from its own
# compiled-in default and not from an environment variable this script fed it.
# ⚠ HOME is unset in the bed; Qt writes nothing, but QStandardPaths warns.
HOME=/tmp; export HOME
/qtprobe
SH
}

# ⛔ AND THE CONTROL. Without it "no host object loaded" is a claim about a
# program that might simply never have tried. This asks Qt for a plugin it
# does NOT have compiled in, by a path in the target -- the one call that
# would reach the host loader if anything did.
poc_observation_probe() {
  cat <<'SH'
#!/bin/sh
HOME=/tmp; export HOME
QT_DEBUG_PLUGINS=1 QT_QPA_PLATFORM=xcb /qtprobe >/tmp/o 2>&1
printf 'xcb-requested:exit%s\n' "$?"
SH
}

poc_matrix "$APP/build/qtprobe"
poc_observe "$APP/build/qtprobe" "asking for a QPA plugin that is NOT compiled in"

# ---------------------------------------------------------------------------
# the depth reached -- ⛔ the honest part
# ---------------------------------------------------------------------------
printf '\n  the depth reached:\n'
printf '    %-24s %s\n' "Qt 6 QtCore/Gui/Widgets" "⭐ static, this POC"
printf '    %-24s %s\n' "QPA offscreen plugin"    "⭐ compiled in, no dlopen"
printf '    %-24s %s\n' "Qt with a real display"  "⛔ NOT ATTEMPTED -- needs xcb, T-054 rung 2"
printf '    %-24s %s\n' "KDE Frameworks 6"        "⛔ NOT ATTEMPTED -- T-054 rung 3"
printf '    %-24s %s\n' "kdenlive"                "⛔ NOT ATTEMPTED -- T-054 rung 4"
printf '    %-24s %s\n' "MLT + ffmpeg (engine)"   "⭐ done in poc/80-mlt, 11 of 11"

poc_finish
