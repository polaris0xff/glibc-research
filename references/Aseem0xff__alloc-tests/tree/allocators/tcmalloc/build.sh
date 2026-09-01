#!/bin/sh
# Google tcmalloc. Bazel; used by the `preload` mechanism only.
#
# ⚠ Bazel is pinned to 8.6.0 through bazelisk. Bazel 9 breaks this build --
# recorded in upstream mimalloc-bench issue 258
# (references/daanx__mimalloc-bench, api/issues.json at commit
# 3ad2732048312b0cc472b60302ff120f02ee9558), and re-checked here by
# experiments/40-allocator-build-matrix.sh rather than taken on trust.
#
# ⛔ Upstream states musl is not a supported platform, and the Bazel `tcmalloc`
# target is a C++ library whose link closure pulls in Abseil and libstdc++. Both
# are why the static mechanisms report this allocator as unsupported rather than
# attempting a splice that would put a C++ runtime inside libc.
. "$(dirname "$0")/../lib.sh"

prepare_out

case "$MODE" in
    prefixed)
        unsupported "tcmalloc exports no prefixed C allocation API; its supported interface is the malloc/operator-new replacement plus the C++ MallocExtension, so a #[global_allocator] shim has no C symbol to bind to."
        ;;
esac

if [ "$LIBC" = musl ]; then
    unsupported "upstream does not support musl; the Bazel build requires glibc-specific interfaces."
fi

command -v bazelisk >/dev/null 2>&1 || command -v bazel >/dev/null 2>&1 || \
    unsupported "neither bazelisk nor bazel is present in this image"

BAZEL=$(command -v bazelisk 2>/dev/null || command -v bazel)
export USE_BAZEL_VERSION="${TCMALLOC_BAZEL_VERSION:-8.6.0}"

cd "$SRC" || { echo "tcmalloc: cannot enter source dir $SRC" >&2; exit 1; }
# --nohome_rc so a runner's stray ~/.bazelrc cannot change what is built.
"$BAZEL" --nohome_rc build -c opt //tcmalloc:tcmalloc_deprecated_perthread \
    >"$OUT/bazel.log" 2>&1 \
  || "$BAZEL" --nohome_rc build -c opt //tcmalloc \
    >>"$OUT/bazel.log" 2>&1 \
  || unsupported "bazel build failed with USE_BAZEL_VERSION=$USE_BAZEL_VERSION: $(tail -5 "$OUT/bazel.log" | tr '\n' ' ')"

so=$(find "$SRC/bazel-bin" -name 'libtcmalloc*.so' 2>/dev/null | head -1)
if [ -n "$so" ]; then
    cp "$so" "$OUT/lib/libtcmalloc.so"
    write_meta "" "bazel -c opt, USE_BAZEL_VERSION=$USE_BAZEL_VERSION"
    echo "built $OUT/lib/libtcmalloc.so for the preload mechanism"
    exit 0
fi

unsupported "the bazel build produced no shared object usable for LD_PRELOAD under bazel-bin"
