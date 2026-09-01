# shellcheck shell=sh
# Shared helpers for the per-allocator build recipes.
#
# THE CONTRACT every allocators/<id>/build.sh honours
#
#   IN, as environment:
#     SRC          the allocator source tree, already at the pinned commit
#     OUT          install prefix to create; the archive goes in $OUT/lib
#     MODE         prefixed | override
#                    prefixed -> exports mi_malloc / je_mallocx / sn_malloc ...
#                                and does NOT define malloc. Used by the
#                                rust-global mechanism.
#                    override -> defines malloc/free/realloc/... themselves.
#                                Used by libc-surgery and link-override.
#     PIC          1 for a position-independent archive (required for
#                  static-PIE), 0 otherwise
#     LIBC         musl | glibc
#     TARGET_ARCH  x86_64 | aarch64
#     NPROC, CC, CXX, AR
#
#   OUT, on success:
#     $OUT/lib/liballocbench.a    the archive, under one name for every
#                                 allocator so the consumer never has to know
#                                 which one it linked
#     $OUT/meta.env               ALLOC_LINK_CXX=... and the flags used
#
#   EXIT CODES
#     0  built
#     3  UNSUPPORTED in this configuration -- prints `UNSUPPORTED: <reason>`
#        on stdout. ⭐ This is a RESULT, not an error: the orchestrator records
#        the reason and the report prints the cell as unsupported rather than
#        dropping it. A configuration that silently disappears from a matrix is
#        indistinguishable from one nobody thought of.
#     other  the build was attempted and failed

set -eu

: "${SRC:?SRC is required}"
: "${OUT:?OUT is required}"
: "${MODE:=prefixed}"
: "${PIC:=1}"
: "${LIBC:=musl}"
: "${TARGET_ARCH:=x86_64}"
: "${NPROC:=$(nproc 2>/dev/null || echo 2)}"
: "${CC:=cc}"
: "${CXX:=c++}"
: "${AR:=ar}"

ARCHIVE="$OUT/lib/liballocbench.a"

unsupported() {
    printf 'UNSUPPORTED: %s\n' "$1"
    exit 3
}

prepare_out() {
    rm -rf "$OUT"
    mkdir -p "$OUT/lib" "$OUT/include"
}

# ⭐ Every recipe ends here. Producing a file called liballocbench.a is not the
# same as producing a usable allocator, and the difference is exactly the class
# of failure this project exists to avoid reporting as a number: an archive
# whose symbols are missing links against libc's malloc instead and measures the
# baseline under another name.
#
# So the archive is checked for the symbols the mechanism actually needs before
# the recipe claims success.
finish() {
    # $@ = symbols that MUST be defined in the archive
    [ -f "$ARCHIVE" ] || { echo "build did not produce $ARCHIVE" >&2; exit 1; }

    missing=''
    for sym in "$@"; do
        if ! "$NM" --defined-only "$ARCHIVE" 2>/dev/null | grep -qE "[[:space:]][TtWwDdBb][[:space:]]+$sym\$"; then
            missing="$missing $sym"
        fi
    done
    if [ -n "$missing" ]; then
        echo "archive $ARCHIVE is missing required symbol(s):$missing" >&2
        echo "--- symbols the archive does define (first 40) ---" >&2
        "$NM" --defined-only "$ARCHIVE" 2>/dev/null | head -40 >&2
        exit 1
    fi

    # In `prefixed` mode the archive must NOT define malloc. If it did, linking
    # it for the rust-global experiment would also replace libc's allocator,
    # and the two mechanisms this project reports separately would silently be
    # the same experiment.
    if [ "$MODE" = prefixed ]; then
        if "$NM" --defined-only "$ARCHIVE" 2>/dev/null | grep -qE '[[:space:]]T[[:space:]]+malloc$'; then
            echo "archive defines malloc in prefixed mode: the override layer was not disabled" >&2
            exit 1
        fi
    else
        if ! "$NM" --defined-only "$ARCHIVE" 2>/dev/null | grep -qE '[[:space:]][TW][[:space:]]+malloc$'; then
            echo "archive does not define malloc in override mode: nothing would be replaced" >&2
            exit 1
        fi
    fi

    size=$(wc -c < "$ARCHIVE")
    echo "built $ARCHIVE ($size bytes, mode=$MODE pic=$PIC libc=$LIBC arch=$TARGET_ARCH)"
}

# `nm` is not in Alpine's base image and the LTO-aware variants differ by
# compiler. Resolved once, here, so every recipe agrees.
NM="${NM:-}"
if [ -z "$NM" ]; then
    for c in llvm-nm gcc-nm nm; do
        if command -v "$c" >/dev/null 2>&1; then NM="$c"; break; fi
    done
fi
: "${NM:?no nm found; install binutils or llvm}"

pic_cflags() {
    if [ "$PIC" = 1 ]; then printf -- '-fPIC'; else printf -- '-fno-PIC'; fi
}

# $1 = comma-separated link libs (may be empty), $2 = the flag description
#
# ⛔ TWO FILES, AND THE SPLIT IS THE POINT.
#
# meta.env is `.`-sourced by the consumer, so everything in it must be
# shell-safe. The flag description is not: it is free text that legitimately
# contains spaces, single quotes and `=` (jemalloc's is
# `CFLAGS='-O3 -fPIC'`). Putting it in a sourced file needs escaping that has
# already gone wrong twice here on 2026-09-01 --
#
#   unquoted, the shell RAN the second word: `meta.env: line 2: Release: not
#   found`, surfacing as a ripgrep build exiting 127 with no mention of the
#   allocator;
#   then quoted but unquoted at the printf CALL, so word splitting handed
#   printf several arguments and it repeated its format once per word,
#   producing five bogus ALLOC_BUILD_FLAGS lines that in turn produced invalid
#   JSON downstream and a silently missing binary-size column.
#
# ⭐ So the free text is not put in a sourced file at all. meta.env carries only
# single-token values; build-flags.txt carries the prose and is only ever read.
write_meta() {
    {
        printf "ALLOC_LINK_CXX='%s'\n" "$1"
        printf "ALLOC_LINK_SEARCH='%s'\n" "${ALLOC_LINK_SEARCH:-}"
        printf "ALLOC_MODE='%s'\n" "$MODE"
        printf "ALLOC_PIC='%s'\n" "$PIC"
        printf "ALLOC_LIBC='%s'\n" "$LIBC"
        printf "ALLOC_ARCH='%s'\n" "$TARGET_ARCH"
    } > "$OUT/meta.env"
    printf '%s\n' "$2" > "$OUT/build-flags.txt"
}
