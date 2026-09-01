#!/bin/sh
# Mesh. Shared object only; used by the `preload` mechanism.
#
# ⛔ Mesh has no static-archive target and no prefixed C API, so it cannot
# participate in the static-binary mechanisms this project is mainly about.
# That is recorded in allocators/allocators.toml under unsupported_notes and
# checked, not assumed: this recipe attempts the build and reports what it
# actually found, so a future upstream that adds a static target turns this from
# `unsupported` into a build without anyone editing prose.
. "$(dirname "$0")/../lib.sh"

prepare_out
B="$OUT/.build"

case "$MODE" in
    prefixed)
        unsupported "Mesh exports no prefixed C allocation API at this revision, so a #[global_allocator] shim has no symbol to bind to. Checked: no mesh_malloc / mz_malloc in include/."
        ;;
esac

# Mesh's own CMake produces libmesh.so. A static archive is what this recipe
# would need for link-override or libc-surgery, so its absence is checked here
# rather than inferred from the documentation.
cmake -S "$SRC" -B "$B" -DCMAKE_BUILD_TYPE=Release -DCMAKE_C_COMPILER="$CC" -DCMAKE_CXX_COMPILER="$CXX" \
    >"$OUT/cmake.log" 2>&1 || {
    unsupported "Mesh's CMake configure failed in this image: $(tail -3 "$OUT/cmake.log" | tr '\n' ' ')"
}

# ⚠ Upstream Mesh issue 96: a parallel build races. Serial, deliberately.
cmake --build "$B" >"$OUT/build.log" 2>&1 || {
    unsupported "Mesh failed to build in this image: $(tail -5 "$OUT/build.log" | tr '\n' ' ')"
}

so=$(find "$B" -name 'libmesh*.so*' | head -1)
a=$(find "$B" -name 'libmesh*.a' | head -1)

if [ -n "$a" ]; then
    cp "$a" "$ARCHIVE"
    write_meta "static=stdc++" "cmake Release (static archive found at this revision)"
    finish malloc free realloc calloc
    exit 0
fi

if [ -n "$so" ]; then
    mkdir -p "$OUT/lib"
    cp "$so" "$OUT/lib/libmesh.so"
    write_meta "" "cmake Release (shared object only)"
    echo "built $OUT/lib/libmesh.so for the preload mechanism"
    # A shared object is a valid product for `preload` and a non-product for
    # every static mechanism. Exit 0: the build succeeded at what it can do.
    exit 0
fi

unsupported "Mesh's build produced neither a static archive nor a shared object under $B"
