#!/bin/sh
# jemalloc. Autotools; the git tree has no `configure`, so autogen.sh runs it.
#
# --with-jemalloc-prefix is the mode switch. With `je_` the archive exports
# je_mallocx and friends and defines no malloc; with an empty prefix it defines
# malloc itself, which is jemalloc's default on Linux.
#
# --disable-cxx is not optional here. jemalloc otherwise compiles
# src/jemalloc_cpp.cpp, which defines operator new/delete and pulls a C++
# runtime dependency into the archive. Splicing that into libc.a, or linking it
# into a Rust binary that has no C++ runtime, fails at link with undefined
# __cxa_* symbols.
#
# --disable-initial-exec-tls: the initial-exec TLS model needs a static TLS
# block reserved by the dynamic loader. In a *static* binary there is no loader
# to reserve it. Left enabled it is the difference between a working static
# jemalloc and one that aborts on first allocation in a thread.
. "$(dirname "$0")/../lib.sh"

prepare_out
B="$OUT/.build"
mkdir -p "$B"

prefix="je_"
[ "$MODE" = override ] && prefix=""

# Configure out of tree so a cached source directory is never mutated by one
# cell's configuration and then reused by the next with the wrong prefix.
if [ ! -x "$SRC/configure" ]; then
    ( cd "$SRC" && ./autogen.sh --help >/dev/null 2>&1 || true )
    if [ ! -x "$SRC/configure" ]; then
        ( cd "$SRC" && autoconf ) || { echo "could not generate jemalloc configure" >&2; exit 1; }
    fi
fi

pic_flag=$(pic_cflags)

cd "$B" || { echo "jemalloc: cannot enter build dir $B" >&2; exit 1; }
CC="$CC" CFLAGS="-O3 $pic_flag" "$SRC/configure" \
    --prefix="$OUT" \
    --with-jemalloc-prefix="$prefix" \
    --disable-cxx \
    --disable-doc \
    --disable-initial-exec-tls \
    --enable-static \
    --disable-shared \
    >/dev/null

make -j "$NPROC" build_lib_static >/dev/null

lib=$(find "$B" -name 'libjemalloc*.a' | head -1)
[ -n "$lib" ] || { echo "no libjemalloc*.a under $B" >&2; exit 1; }
cp "$lib" "$ARCHIVE"
cp -r "$B/include/." "$OUT/include/" 2>/dev/null || true

# jemalloc's static archive needs pthread and, on some targets, libatomic.
write_meta "" "autotools --with-jemalloc-prefix='$prefix' --disable-cxx --disable-initial-exec-tls CFLAGS='-O3 $pic_flag'"

if [ "$MODE" = override ]; then
    finish malloc free realloc calloc
else
    finish je_mallocx je_rallocx je_sdallocx je_malloc je_free
fi
