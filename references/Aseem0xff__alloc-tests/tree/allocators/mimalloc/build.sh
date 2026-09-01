#!/bin/sh
# mimalloc. CMake, C only, both modes from one flag.
#
# MI_OVERRIDE controls whether the archive defines malloc. That single switch is
# the whole difference between the two mechanisms this project reports
# separately, which is why mimalloc is the allocator both are demonstrated with.
#
# MI_LIBC_MUSL matters: without it mimalloc assumes glibc-only interfaces and
# the musl build compiles but behaves differently around thread teardown.
#
# ⚠ MI_SKIP_COLLECT_ON_EXIT is deliberately NOT set here, although the prior art
# at references/haskell-wasm__rust-alpine-mimalloc sets it. It makes process
# exit cheaper by skipping a final collect, which would flatter mimalloc on the
# `startup` workload specifically -- the one workload where process teardown is
# a measurable share of the run. Leaving upstream's default in place keeps the
# comparison honest.
. "$(dirname "$0")/../lib.sh"

prepare_out
B="$OUT/.build"

override=OFF
[ "$MODE" = override ] && override=ON

musl=OFF
[ "$LIBC" = musl ] && musl=ON

pic=ON
[ "$PIC" = 1 ] || pic=OFF

cmake -S "$SRC" -B "$B" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_C_COMPILER="$CC" \
    -DMI_BUILD_SHARED=OFF \
    -DMI_BUILD_OBJECT=OFF \
    -DMI_BUILD_TESTS=OFF \
    -DMI_OVERRIDE="$override" \
    -DMI_LIBC_MUSL="$musl" \
    -DCMAKE_POSITION_INDEPENDENT_CODE="$pic" \
    >/dev/null

cmake --build "$B" --parallel "$NPROC" >/dev/null

# The output name carries a suffix in some configurations, so it is found
# rather than assumed. Assuming it is how a build "succeeds" and installs
# nothing.
lib=$(find "$B" -name 'libmimalloc*.a' | head -1)
[ -n "$lib" ] || { echo "no libmimalloc*.a under $B" >&2; exit 1; }
cp "$lib" "$ARCHIVE"
cp -r "$SRC/include/." "$OUT/include/" 2>/dev/null || true

write_meta "" "cmake Release MI_OVERRIDE=$override MI_LIBC_MUSL=$musl PIC=$pic"

if [ "$MODE" = override ]; then
    finish malloc free realloc calloc
else
    finish mi_malloc mi_free mi_realloc mi_malloc_aligned mi_zalloc_aligned mi_realloc_aligned
fi
