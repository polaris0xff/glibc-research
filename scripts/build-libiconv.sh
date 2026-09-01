#!/bin/sh
# build-libiconv.sh - build GNU libiconv as a static archive.
#
# WHY THIS IS HERE: glibc's iconv reaches its encodings through dlopen'd gconv
# modules, and every one of those carries DT_NEEDED libc.so.6. In a static
# binary that means a second libc, and on a musl host it means nothing loads at
# all. experiments/30-gconv-and-locale.sh measures both. GNU libiconv carries
# the same encodings as ordinary code in an archive, so a static link gets them
# with no dlopen and no data directory.
#
# Pinned. An unpinned dependency makes every number taken against it undated.
set -eu
VERSION="${LIBICONV_VERSION:-1.18}"
PREFIX="${PGB_LIBICONV_PREFIX:-/opt/pgb-libiconv}"
URL="https://ftp.gnu.org/pub/gnu/libiconv/libiconv-${VERSION}.tar.gz"
WORK="${TMPDIR:-/tmp}/pgb-libiconv-$$"

if [ -f "$PREFIX/lib/libiconv.a" ] && [ "${FORCE:-0}" != 1 ]; then
  printf 'libiconv already at %s (FORCE=1 to rebuild)\n' "$PREFIX"; exit 0
fi

mkdir -p "$WORK"; trap 'rm -rf "$WORK"' EXIT INT TERM
printf 'fetching %s\n' "$URL"
curl -sSfL --retry 3 -o "$WORK/src.tar.gz" "$URL"
tar xzf "$WORK/src.tar.gz" -C "$WORK"
cd "$WORK/libiconv-${VERSION}"
./configure --prefix="$PREFIX" --enable-static --disable-shared \
            --enable-extra-encodings CFLAGS="-O2 -fPIC" >"$WORK/configure.log" 2>&1
make -j"$(nproc 2>/dev/null || echo 2)" >"$WORK/make.log" 2>&1
make install >"$WORK/install.log" 2>&1
printf 'libiconv %s -> %s (%s bytes)\n' "$VERSION" "$PREFIX/lib/libiconv.a" \
       "$(wc -c < "$PREFIX/lib/libiconv.a")"
