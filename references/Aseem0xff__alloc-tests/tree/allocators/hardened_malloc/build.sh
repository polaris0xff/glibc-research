#!/bin/sh
# hardened_malloc. Upstream builds a shared object; this needs a static
# archive, so the Makefile is used to produce the OBJECTS and `ar` archives
# them. Reusing upstream's own rule keeps the compile flags upstream's rather
# than a reimplementation that drifts.
#
# THREE DELIBERATE DEVIATIONS FROM upstream's defaults, each recorded in
# meta.env and each with a reason:
#
#   CONFIG_NATIVE=false      upstream defaults to true, which adds -march=native.
#                            A binary tuned to the builder's CPU is not
#                            reproducible and not comparable between runners,
#                            and no other allocator here is built that way.
#
#   CONFIG_CXX_ALLOCATOR=false
#                            drops new.cc, whose operator new/delete would pull
#                            a C++ runtime into a Rust binary that has none.
#
#   -flto removed            ⭐ every allocator here is built WITHOUT internal
#                            LTO, so that dimension is held constant across the
#                            comparison. hardened_malloc is the only one that
#                            forces it on. Leaving it would also make the
#                            archive's symbol index depend on an LTO-aware `ar`,
#                            and the objects unreadable to the identity oracle.
#                            LTO is measured as an APPLICATION build profile
#                            instead, where it applies to every cell equally.
#
#   -fvisibility=default     upstream hides symbols for the shared object. In an
#     (replacing hidden)     archive spliced into libc.a the public names have
#                            to remain visible or nothing can resolve them.
#
# ⭐ THE MODE SWITCH IS -DH_MALLOC_PREFIX, and it is not obvious from the
# header. include/h_malloc.h reads:
#
#     #ifndef H_MALLOC_PREFIX
#     #define h_malloc malloc
#     #define h_free   free
#     ...
#
# so the `h_`-prefixed API a caller reads in the header IS the plain malloc API
# unless that macro is defined. Building without it and then hunting for
# `h_malloc` in the archive finds nothing -- observed here on 2026-09-01 -- and
# the objcopy-based workaround that was written first was solving a problem
# upstream already has a flag for.
. "$(dirname "$0")/../lib.sh"

prepare_out
B="$OUT/.build"
mkdir -p "$B"

VARIANT="${HM_VARIANT:-default}"
[ -f "$SRC/config/$VARIANT.mk" ] || unsupported "hardened_malloc has no config/$VARIANT.mk at this revision"
[ -f "$SRC/h_malloc.c" ] || unsupported "h_malloc.c not found; the source layout changed"

pic_flag=$(pic_cflags)

# Upstream's SHARED_FLAGS with -flto and -march=native left out, and visibility
# opened. -fcf-protection and -fstack-clash-protection are not accepted by every
# compiler/architecture pair, so each is probed rather than assumed: on aarch64
# GCC, -fcf-protection is rejected outright.
SF="-pipe -O3 $pic_flag -fvisibility=default -fno-plt -Wall -Wextra"
# ⚠ The define goes in SHARED_FLAGS, NOT in CPPFLAGS.
#
# The Makefile builds CPPFLAGS in two stages:
#     CPPFLAGS := $(CPPFLAGS) -D_GNU_SOURCE -I include
#     ... later ...  CPPFLAGS += -DCONFIG_SEAL_METADATA=... (and ~15 more)
#
# A command-line `make CPPFLAGS=...` overrides the whole variable AND makes
# every later `+=` a no-op, so it drops `-I include` (the build then fails on
# `#include "h_malloc.h"`, observed 2026-09-01) and silently drops every
# CONFIG_* define, which would have built a differently-configured allocator
# than the one the result claims. SHARED_FLAGS is already overridden here and
# flows into CFLAGS, so the define rides along without disturbing either.
PREFIX_DEF=""
if [ "$MODE" = prefixed ]; then
    # An `[ ... ] && x=y` one-liner would return 1 in override mode and abort
    # the whole recipe, because lib.sh sets -e.
    PREFIX_DEF="-DH_MALLOC_PREFIX"
    SF="$SF $PREFIX_DEF"
fi
for f in -fstack-clash-protection -fcf-protection -fstack-protector-strong; do
    if echo 'int main(void){return 0;}' | "$CC" $f -x c - -o /dev/null 2>/dev/null; then
        SF="$SF $f"
    fi
done

objs=""
cd "$SRC" || { echo "hardened_malloc: cannot enter source dir $SRC" >&2; exit 1; }
for s in chacha.c h_malloc.c memory.c pages.c random.c util.c; do
    [ -f "$s" ] || continue
    o="$B/$(basename "$s" .c).o"
    # ⛔ TARGET_ARCH= IS LOAD-BEARING. GNU make's built-in C rule is
    #   $(CC) $(CFLAGS) $(CPPFLAGS) $(TARGET_ARCH) -c
    # and TARGET_ARCH is a make BUILT-IN meant for flags like -m32. This
    # project's contract also calls its architecture variable TARGET_ARCH, and
    # exporting it puts the bare word `x86_64` on the compiler command line:
    #   cc: error: x86_64: linker input file not found
    # Observed here on 2026-09-01. Clearing it on the make command line is the
    # fix; the recipe still has the value in $TARGET_ARCH for its own use.
    make -s OUT="$B" \
        TARGET_ARCH= \
        VARIANT="$VARIANT" \
        CONFIG_NATIVE=false \
        CONFIG_CXX_ALLOCATOR=false \
        CONFIG_WERROR=false \
        SHARED_FLAGS="$SF" \
        CC="$CC" \
        "$o" || { echo "hardened_malloc: make failed for $s" >&2; exit 1; }
    objs="$objs $o"
done
[ -n "$objs" ] || { echo "hardened_malloc: no objects built" >&2; exit 1; }

"$AR" rcs "$ARCHIVE" $objs
cp -r "$SRC/include/." "$OUT/include/" 2>/dev/null || true

write_meta "" "make VARIANT=$VARIANT CONFIG_NATIVE=false CONFIG_CXX_ALLOCATOR=false CPPFLAGS='$PREFIX_DEF' SHARED_FLAGS='$SF'"

if [ "$MODE" = prefixed ]; then
    finish h_malloc h_free h_realloc h_calloc h_aligned_alloc
else
    finish malloc free realloc calloc
fi
