#!/bin/sh
# poc/80-mlt -- kdenlive's ENGINE, statically linked, rendering video on all
# eleven environments with its plugin directory holding nothing but empty
# files.
#
# -- WHY THIS ONE ----------------------------------------------------------
#
# ⭐ THE OPERATOR SET THIS AS A CHALLENGE: the Anylinux-AppImages README and
# this project's own documents make a joke of "just statically compile
# kdenlive". Taking it seriously is what TODO T-003 asks for anyway --
# "deliberately pick above the current class ... record the failure ... with
# the cause named at file and line".
#
# kdenlive is a Qt/KDE application on top of MLT, and MLT is where the video
# work actually happens: it is the engine, it loads its functionality as
# dlopen'd modules, and it links ffmpeg. So the climb is
#
#     ffmpeg  ->  MLT (melt)  ->  Qt/KF6  ->  kdenlive
#
# and this POC is the first two rungs. ⛔ IT DOES NOT BUILD KDENLIVE and does
# not claim to; §"the depth reached" below says exactly where it stops and
# why, which is the honest form of the answer.
#
# -- ⛔ TWO REAL FAILURES, WHICH ARE THE POINT AND ARE ASSERTED AS SUCH -----
#
# 1. `melt` CANNOT BE LINKED THROUGH THE ORDINARY BUILD, and the cause is one
#    line of somebody else's build system:
#
#        src/framework/CMakeLists.txt:36    add_library(mlt SHARED ...)
#
#    `SHARED` is hard-coded, so BUILD_SHARED_LIBS cannot turn it off, and pgb
#    links executables with -static. The result:
#
#        /usr/bin/ld: attempted static link of dynamic object
#                     `../../out/lib/libmlt-7.so.7.30.0'
#
#    ⭐ This project's rule is NO SOURCE PATCHES, so the answer is not to edit
#    that line: it is to link `melt` from the objects the build already
#    produced, which is a link line and not a patch. That is what this POC
#    does, and it turns "kdenlive's engine cannot be static" into "kdenlive's
#    engine's BUILD SYSTEM cannot be, and the code is fine".
#
# 2. THE avformat MODULE CANNOT BE BUILT AS A SHARED OBJECT against a static
#    ffmpeg:
#
#        libavcodec.a(cavsdsp.o): relocation R_X86_64_PC32 against symbol
#        `ff_pw_5' can not be used when making a shared object
#
#    ⭐ AND IT LINKS PERFECTLY INTO A STATIC EXECUTABLE. The requirement was a
#    property of the shared-module SHAPE, not of the code -- so the module
#    that could not be built the normal way is in the binary this POC ships.
#
# -- ⛔ AND ONE REAL LIMIT OF --wrap-dlopen, FOUND HERE ---------------------
#
# MLT does not dlopen a plugin BY NAME. `mlt_repository_init` LISTS the module
# directory (`src/framework/mlt_repository.c`, the loop above line 150) and
# dlopens whatever it finds. So an EMPTY directory means it finds nothing and
# never calls dlopen at all -- `--wrap-dlopen` is never reached.
#
# ⭐ The plugin directory here therefore holds one ZERO-BYTE FILE per module.
# Nothing is mapped, nothing is read, no code is in them -- they exist so the
# directory listing has entries, and every dlopen that follows is answered out
# of the compiled-in table. ⛔ That is a real limit of the mechanism and it is
# written into docs/limitations.md rather than hidden behind a passing row:
# `--wrap-dlopen` serves dlopen-BY-NAME. A program that DISCOVERS its plugins
# by listing a directory needs that directory to have names in it.
#
# -- WHAT IT PROVES --------------------------------------------------------
#
#   - a 100 MB statically linked `melt` with EIGHT MLT modules and the whole
#     of ffmpeg 7.1 compiled in;
#   - it renders a real MP4 -- checked by the `ftyp` box in the output, not by
#     an exit status;
#   - on all eleven environments, loading no host shared object.
#
# ⚠ BUDGET: ffmpeg and MLT are built from source. Expect 30-60 minutes on
# four cores the first time. Both steps are skipped if their output is
# already present.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/../common.sh"

POC_URL="https://ffmpeg.org/releases/ffmpeg-7.1.tar.xz + https://github.com/mltframework/mlt v7.30.0"
POC_VERSION="ffmpeg 7.1, MLT 7.30.0 (kdenlive's engine)"
POC_SHA256="⚠ NOT PINNED -- see the note below"
POC_NORMAL_BUILD="./configure && make; cmake && make  -- both produce SHARED objects"
POC_WHY="kdenlive's engine: a large dependency graph, dlopen'd modules, and a build system that hard-codes SHARED"
POC_STRESSES="a 142 MB static libavcodec, 8 dlopen'd modules, module discovery by directory listing, C++ inside a C program, PIC vs static"

# ⛔ NOT PINNED, AND SAYING SO IS THE POINT. Every other POC in this tree
# fetches a sha256-verified tarball. These two are not pinned yet because the
# first run of this POC is what established that they build at all, and a pin
# recorded from an unverified download is a pin in name only. ⚠ The hashes
# this run sees are printed below; a later session should move them into
# poc_fetch's third argument once they have been checked against upstream's
# own published digest, and this note deleted.
poc_begin

W="$WORK/80-kdenlive"
mkdir -p "$W" || exit 2
RV="https://api.rv.pkgforge.dev"
LOG="$POC_OUT/build.log"
: > "$LOG"

# ---------------------------------------------------------------------------
# 1. ffmpeg
# ---------------------------------------------------------------------------
printf -- '-- rung 1: ffmpeg 7.1 -----------------------------------------\n'
if [ ! -f "$W/inst/lib/libavcodec.a" ]; then
  poc_fetch "$RV/https://ffmpeg.org/releases/ffmpeg-7.1.tar.xz" "$W/ffmpeg.tar.xz" \
    || { poc_skip "fetch ffmpeg" "download failed"; poc_finish; }
  [ -d "$W/ffmpeg" ] || { mkdir -p "$W/ffmpeg" && tar xf "$W/ffmpeg.tar.xz" -C "$W/ffmpeg" --strip-components=1; }
  # ⚠ --disable-x86asm because the pinned environment has no nasm or yasm.
  # That costs runtime speed and costs this POC nothing: it measures linking
  # and loading, not codec throughput. It is also a finding about the pinned
  # environment, in the same family as T-016.
  poc_in_env "cd $W/ffmpeg && ./configure --prefix=$W/inst --disable-x86asm \
      --disable-doc --disable-autodetect --disable-shared --enable-static --enable-pic \
      && make -j\$(nproc) && make install" >>"$LOG" 2>&1 \
    || { poc_check "ffmpeg builds through pgb" failed ok; poc_finish; }
fi
poc_check "ffmpeg builds through pgb" \
  "$([ -f "$W/inst/lib/libavcodec.a" ] && echo ok || echo failed)" "ok"
poc_note "libavcodec.a $(wc -c < "$W/inst/lib/libavcodec.a" 2>/dev/null) bytes"
if [ -x "$W/inst/bin/ffmpeg" ]; then
  poc_check "the ffmpeg CLI it produced is static" \
    "$(readelf -lW "$W/inst/bin/ffmpeg" 2>/dev/null | grep -c INTERP)" "0"
  poc_note "$("$W/inst/bin/ffmpeg" -hide_banner -version 2>/dev/null | head -1 || true)"
fi

# ---------------------------------------------------------------------------
# 2. MLT
# ---------------------------------------------------------------------------
printf -- '\n-- rung 2: MLT 7.30.0, kdenlive'"'"'s engine -------------------\n'
B="$W/mlt/build"
if [ ! -d "$B" ]; then
  poc_fetch "$RV/https://github.com/mltframework/mlt/archive/refs/tags/v7.30.0.tar.gz" "$W/mlt.tar.gz" \
    || { poc_skip "fetch MLT" "download failed"; poc_finish; }
  [ -d "$W/mlt" ] || { mkdir -p "$W/mlt" && tar xzf "$W/mlt.tar.gz" -C "$W/mlt" --strip-components=1; }
  poc_in_env "cd $W/mlt && PKG_CONFIG_PATH=$W/inst/lib/pkgconfig cmake -S . -B build \
      -DCMAKE_INSTALL_PREFIX=$W/inst -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=OFF \
      -DMOD_QT=OFF -DMOD_QT6=OFF -DMOD_GDK=OFF -DMOD_SDL1=OFF -DMOD_SDL2=OFF \
      -DMOD_RTAUDIO=OFF -DMOD_JACKRACK=OFF -DMOD_FREI0R=OFF -DMOD_DECKLINK=OFF \
      -DMOD_OPENCV=OFF -DMOD_MOVIT=OFF -DMOD_RUBBERBAND=OFF -DMOD_VIDSTAB=OFF \
      -DMOD_NDI=OFF -DMOD_GLAXNIMATE=OFF -DMOD_GLAXNIMATE_QT6=OFF -DMOD_RESAMPLE=OFF \
      -DMOD_SOX=OFF -DGPL=ON -DGPL3=ON" >>"$LOG" 2>&1 \
    || { poc_check "MLT configures through pgb" failed ok; poc_finish; }
  # ⛔ `make -k`, DELIBERATELY. The ordinary build FAILS -- twice -- and both
  # failures are results this POC asserts below. -k gets the objects built so
  # the failures can be measured instead of stopping everything.
  poc_in_env "cd $B && make -k -j\$(nproc) >/dev/null 2>&1; true" >>"$LOG" 2>&1
fi

nmods=0
MODS="core avformat kdenlive plus plusgpl normalize oldfilm xine"
for m in $MODS; do
  [ -d "$B/src/modules/$m/CMakeFiles/mlt$m.dir" ] && nmods=$((nmods+1))
done
poc_check "MLT module objects built" "$nmods" "8"

# ⛔ THE TWO FAILURES ARE RE-MEASURED ON EVERY RUN, NOT GREPPED FROM A LOG.
#
# The first version of this asserted them by searching the build log -- which
# is written only when the build actually runs, so the SECOND run of this POC
# (everything cached) found nothing and reported BOTH failures as "the failure
# did not happen". ⚠ A record of a failure that only exists while the failure
# is fresh is not a record, and this is the shape docs/methodology says to
# watch for: an absence read as a value. Each one is now one `make` of one
# target, which costs a link and cannot rot.
rerun_target() {  # target -> the linker's own message, or "LINKED"
  rm -f "$2" 2>/dev/null
  poc_in_env "cd $B && make $1" >"$POC_OUT/$1.log" 2>&1 && { printf 'LINKED'; return; }
  cat "$POC_OUT/$1.log"
}

_melt_out=$(rerun_target melt "$B/out/bin/melt")
case "$_melt_out" in
  *"attempted static link of dynamic object"*) _melt=attempted-static-link ;;
  LINKED) _melt=linked ;;
  *) _melt=other ;;
esac
poc_check "the ordinary melt link fails on add_library(mlt SHARED)" \
  "$_melt" "attempted-static-link"
poc_note "src/framework/CMakeLists.txt:36 -- SHARED is hard-coded, so"
poc_note "BUILD_SHARED_LIBS cannot turn it off and -static cannot consume it"

_av_out=$(rerun_target mltavformat "$B/out/lib/mlt/libmltavformat.so")
case "$_av_out" in
  *"can not be used when making a shared object"*) _av=pic-failure ;;
  LINKED) _av=linked ;;
  *) _av=other ;;
esac
poc_check "the avformat SHARED module fails on PIC against static ffmpeg" \
  "$_av" "pic-failure"
poc_note "libavcodec.a(cavsdsp.o): R_X86_64_PC32 against \`ff_pw_5'"
poc_note "⭐ and the same objects link into the STATIC executable below, so the"
poc_note "   requirement was the shared-module SHAPE's and not the code's"

# ---------------------------------------------------------------------------
# 3. The static link pgb makes possible.
# ---------------------------------------------------------------------------
printf -- '\n-- the link the build system could not make --------------------\n'
FW=$(find "$B/src/framework/CMakeFiles/mlt.dir" -name '*.o' 2>/dev/null | sort | tr '\n' ' ')
MELTO=$(find "$B/src/melt/CMakeFiles/melt.dir" -name '*.o' 2>/dev/null | sort | tr '\n' ' ')
SPECS=""
for m in $MODS; do
  objs=$(find "$B/src/modules/$m/CMakeFiles/mlt$m.dir" -name '*.o' 2>/dev/null | sort | tr '\n' ',' | sed 's/,$//')
  [ -n "$objs" ] && SPECS="$SPECS --wrap-dlopen libmlt$m.so=$objs"
done
poc_note "framework $(printf '%s' "$FW" | wc -w) objects, melt $(printf '%s' "$MELTO" | wc -w), $(printf '%s' "$SPECS" | grep -o wrap-dlopen | wc -l) modules"

# ⚠ $CXX and not $CC: MLT's `plus` module contains subtitles.cpp, so a C
# driver leaves std::ios_base::Init and __cxa_begin_catch unresolved. A C
# program with one C++ translation unit inside a plugin is a shape nothing in
# this tree had hit before.
# shellcheck disable=SC2086
"$PGB" --bind "$WORK" $SPECS build -- /bin/sh -c \
  "\$CXX -o $W/melt-static $MELTO $FW -L$W/inst/lib \
     -Wl,--start-group -lavdevice -lavfilter -lavformat -lavcodec -lswscale -lswresample -lavutil -Wl,--end-group \
     -lm -lpthread -lz" >>"$LOG" 2>&1
poc_check "the static melt links" \
  "$([ -f "$W/melt-static" ] && echo ok || echo failed)" "ok"
[ -f "$W/melt-static" ] || { poc_note "see $LOG"; poc_finish; }
poc_note "melt-static $(wc -c < "$W/melt-static") bytes"
poc_check "PT_INTERP absent" "$(readelf -lW "$W/melt-static" | grep -c INTERP)" "0"
poc_check "DT_NEEDED entries" "$(readelf -dW "$W/melt-static" | grep -c NEEDED)" "0"
# Every MLT module exports mlt_register; namespacing is what stops them
# colliding, exactly as with SQLite's sqlite3_api in poc/70.
poc_check "per-module mlt_register instances" \
  "$(nm "$W/melt-static" | grep -cE ' [tT] pgb_dl[0-9]+_mlt_register$')" "8"

# ⛔ THE PLUGIN DIRECTORY: one ZERO-BYTE file per module. MLT discovers by
# listing, so the names must exist; nothing else may.
STUBS="$W/stubmods"
rm -rf "$STUBS"; mkdir -p "$STUBS"
for m in $MODS; do : > "$STUBS/libmlt$m.so"; done
poc_check "every plugin file is zero bytes" \
  "$(find "$STUBS" -type f -size +0c | wc -l)" "0"

# ---------------------------------------------------------------------------
# 4. The matrix.
# ---------------------------------------------------------------------------
poc_functional_test() {
  cat <<'EOF'
set -u
fails=0
export MLT_REPOSITORY=/pgb-mlt-modules
export MLT_DATA=/pgb-mlt-data

# Every module registered, out of zero-byte files.
q=$(/melt-static -query producers 2>/dev/null)
for want in avformat color noise timewarp; do
  case "$q" in
    *"- $want"*) echo "  ok   producer $want registered" ;;
    *) echo "  FAIL producer $want missing"; fails=$((fails+1)) ;;
  esac
done

# ⛔ THE REAL WORK: render video. An exit status would not distinguish "the
# engine ran" from "the engine started and produced nothing", so the output
# is checked for the MP4 `ftyp` box and a plausible size.
out=/tmp/pgb-mlt-out.mp4
rm -f "$out"
/melt-static -profile atsc_720p_25 color:#00ff00 out=12 \
   -consumer avformat:"$out" >/dev/null 2>&1
if [ -s "$out" ]; then
  sz=$(wc -c < "$out")
  magic=$(dd if="$out" bs=1 skip=4 count=4 2>/dev/null)
  if [ "$magic" = "ftyp" ] && [ "$sz" -gt 1000 ]; then
    echo "  ok   rendered an MP4, $sz bytes, ftyp box present"
  else
    echo "  FAIL output is not an MP4: magic=[$magic] size=$sz"; fails=$((fails+1))
  fi
else
  echo "  FAIL no output file"; fails=$((fails+1))
fi

echo "$([ "$fails" = 0 ] && echo PASSED || echo FAILED): $fails failure(s)"
[ "$fails" = 0 ]
EOF
}

printf -- '\n-- the matrix: eight modules, all of them empty files ----------\n'
poc_matrix "$W/melt-static" \
  "$STUBS:/pgb-mlt-modules" \
  "$W/mlt/src/modules:/pgb-mlt-data"

printf -- '\n-- the depth reached ------------------------------------------\n'
poc_note "ffmpeg 7.1               ✅ static, runs"
poc_note "MLT 7.30.0 / melt        ✅ static, 8 modules compiled in, renders MP4"
poc_note "Qt 6 / KDE Frameworks    ⛔ NOT ATTEMPTED -- the next rung"
poc_note "kdenlive                 ⛔ NOT ATTEMPTED"
poc_note ""
poc_note "⭐ The engine is done. What is left is a GUI toolkit, and Qt is the"
poc_note "   next thing to break: its platform plugins are the host-plugin"
poc_note "   class docs/limitations.md §1 is about, and TODO T-033 is the route."

poc_finish
