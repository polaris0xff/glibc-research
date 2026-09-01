#!/bin/sh
# rpmalloc. Compiled directly rather than through its ninja generator.
#
# ⚠ Why not `python3 configure.py && ninja`: that also builds C++ test binaries,
# which is the failure reported in upstream mimalloc-bench issue 256 (a build
# host with clang but no libstdc++ headers fails on `#include <new>`). None of
# those targets are wanted here, and two source files do not need a generator.
#
# rpmalloc's override layer lives in rpmalloc/malloc.c behind ENABLE_OVERRIDE.
# In prefixed mode it is simply not compiled, which is why this recipe does not
# need a patch to keep malloc out of the archive.
. "$(dirname "$0")/../lib.sh"

prepare_out
B="$OUT/.build"
mkdir -p "$B"

[ -f "$SRC/rpmalloc/rpmalloc.c" ] || unsupported "rpmalloc/rpmalloc.c not found at this revision; the source layout changed"

pic_flag=$(pic_cflags)
CFLAGS="-O3 $pic_flag -std=c11 -D_GNU_SOURCE -I$SRC/rpmalloc -fno-builtin-malloc -fno-builtin-free"

# ENABLE_PRELOAD=0: this archive is linked into a static binary, never
# LD_PRELOADed, and the preload path adds an indirection that would be measured
# but never used.
"$CC" $CFLAGS -DENABLE_PRELOAD=0 -c "$SRC/rpmalloc/rpmalloc.c" -o "$B/rpmalloc.o"
set -- "$B/rpmalloc.o"

if [ "$MODE" = override ]; then
    [ -f "$SRC/rpmalloc/malloc.c" ] || unsupported "rpmalloc/malloc.c (the override layer) not found at this revision"
    "$CC" $CFLAGS -DENABLE_OVERRIDE=1 -DENABLE_PRELOAD=0 -c "$SRC/rpmalloc/malloc.c" -o "$B/malloc.o"
    set -- "$@" "$B/malloc.o"
fi

"$AR" rcs "$ARCHIVE" "$@"

# ⚠ rpmalloc.c defines the plain malloc names itself at this revision, not only
# through malloc.c, so -DENABLE_OVERRIDE=0 is not enough to keep them out of a
# prefixed archive. They are localised instead, and `finish` re-checks. Observed
# on 2026-09-01 as `archive defines malloc in prefixed mode`.
if [ "$MODE" = prefixed ] && "$NM" --defined-only "$ARCHIVE" 2>/dev/null | grep -qE '[[:space:]]T[[:space:]]+malloc$'; then
    command -v objcopy >/dev/null 2>&1 || \
        unsupported "rpmalloc defines malloc unconditionally at this revision and objcopy is unavailable to localise it"
    tmp="$B/localise"; mkdir -p "$tmp"
    ( cd "$tmp" && "$AR" x "$ARCHIVE" )
    for o in "$tmp"/*.o; do
        objcopy \
            --localize-symbol=malloc --localize-symbol=free \
            --localize-symbol=calloc --localize-symbol=realloc \
            --localize-symbol=aligned_alloc --localize-symbol=posix_memalign \
            --localize-symbol=memalign --localize-symbol=valloc \
            --localize-symbol=pvalloc --localize-symbol=cfree \
            --localize-symbol=malloc_usable_size --localize-symbol=reallocarray \
            "$o" 2>/dev/null || true
    done
    rm -f "$ARCHIVE"
    "$AR" rcs "$ARCHIVE" "$tmp"/*.o
fi
cp "$SRC/rpmalloc/rpmalloc.h" "$OUT/include/" 2>/dev/null || true

write_meta "" "direct $CC $CFLAGS override=$([ "$MODE" = override ] && echo 1 || echo 0)"

if [ "$MODE" = override ]; then
    finish malloc free realloc calloc
else
    finish rpmalloc rpfree rpaligned_alloc rpaligned_realloc rpaligned_calloc rpmalloc_initialize rpmalloc_thread_initialize
fi
