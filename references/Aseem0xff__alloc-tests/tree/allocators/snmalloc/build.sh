#!/bin/sh
# snmalloc. CMake, C++; SNMALLOC_STATIC_LIBRARY_PREFIX is the mode switch.
#
# ⚠ SNMALLOC_CLEANUP=CXX11_DESTRUCTORS on musl. Upstream mimalloc-bench records
# this at references/daanx__mimalloc-bench tree/build-bench-env.sh, commit
# 3ad2732048312b0cc472b60302ff120f02ee9558: the default cleanup strategy is
# broken on Alpine. The failure is at thread teardown, so it does not appear at
# link time and it does not appear in a single-threaded smoke test.
#
# The archive references the C++ runtime, so meta.env names it and the consumer
# links it statically -- a static binary cannot pick up libstdc++.so later.
. "$(dirname "$0")/../lib.sh"

prepare_out
B="$OUT/.build"

prefix="sn_"
[ "$MODE" = override ] && prefix=""

pic=ON
[ "$PIC" = 1 ] || pic=OFF

set -- \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_C_COMPILER="$CC" \
    -DCMAKE_CXX_COMPILER="$CXX" \
    -DSNMALLOC_BUILD_TESTING=OFF \
    -DSNMALLOC_STATIC_LIBRARY=ON \
    -DSNMALLOC_STATIC_LIBRARY_PREFIX="$prefix" \
    -DCMAKE_POSITION_INDEPENDENT_CODE="$pic" \
    -DSNMALLOC_IPO=OFF

if [ "$LIBC" = musl ]; then
    set -- "$@" -DSNMALLOC_CLEANUP=CXX11_DESTRUCTORS
fi

cmake -S "$SRC" -B "$B" "$@" >/dev/null 2>"$OUT/cmake.log" || {
    # snmalloc requires a C++20 compiler by default. Where the image's compiler
    # is older, that is a real limitation of that image and is reported as one
    # rather than papered over.
    if grep -qiE 'c\+\+2|CMAKE_CXX_STANDARD|std=c\+\+' "$OUT/cmake.log" 2>/dev/null; then
        unsupported "snmalloc's CMake refused this toolchain (C++20 required). $(tail -3 "$OUT/cmake.log" | tr '\n' ' ')"
    fi
    cat "$OUT/cmake.log" >&2
    exit 1
}

cmake --build "$B" --parallel "$NPROC" --target snmallocshim-static >/dev/null

lib=$(find "$B" -name 'libsnmallocshim-static.a' -o -name 'libsnmallocshim*.a' | head -1)
[ -n "$lib" ] || { echo "no libsnmallocshim*.a under $B" >&2; exit 1; }
cp "$lib" "$ARCHIVE"

# ⛔ new.cc.o IS REMOVED, and this is required rather than tidy.
#
# snmalloc's shim archive defines the C++ operators as well as the C API.
# libstdc++.a defines them too, and a static link therefore ends in:
#
#   multiple definition of `operator delete(void*, unsigned long)';
#   del_ops.cc:(.text._ZdlPvm+0x0): first defined here
#
# observed here on 2026-09-01. ripgrep is Rust and allocates through the
# #[global_allocator], so the C++ operators are never called; and for the
# libc-surgery mechanism, putting an operator new inside libc.a would change
# the behaviour of every C++ program built in the image, which is a much larger
# claim than this project is making. The C allocation API is what is kept.
if "$AR" t "$ARCHIVE" 2>/dev/null | grep -q '^new\.cc\.o$'; then
    "$AR" d "$ARCHIVE" new.cc.o
    echo "snmalloc: removed new.cc.o (C++ operator new/delete) to avoid colliding with the C++ runtime"
fi

# Which C++ runtime depends on the compiler driver, so it is detected rather
# than guessed: a wrong guess shows up as an undefined _ZdlPv at the very end
# of the ripgrep link, which is an unhelpful place to learn it.
cxxlib="static=stdc++"
if "$CXX" --version 2>/dev/null | grep -qi clang && ! command -v g++ >/dev/null 2>&1; then
    cxxlib="static=c++,static=c++abi"
fi
# ⚠ Where the C++ runtime archive actually is. rustc's `-l static=stdc++`
# searches only the directories it was given, and libstdc++.a lives under
# gcc's own version directory, which is not one of them:
#   error: could not find native static library `stdc++`
# Observed on 2026-09-01. The compiler is asked rather than guessed, because
# the path contains the gcc version and the target triple.
cxxpath=$("$CXX" -print-file-name=libstdc++.a 2>/dev/null || true)
if [ -n "$cxxpath" ] && [ -f "$cxxpath" ]; then
    ALLOC_LINK_SEARCH=$(dirname "$cxxpath")
else
    ALLOC_LINK_SEARCH=""
    echo "snmalloc: warning: could not locate libstdc++.a; the link may fail" >&2
fi
export ALLOC_LINK_SEARCH

write_meta "$cxxlib" "cmake Release PREFIX='$prefix' PIC=$pic CLEANUP=$([ "$LIBC" = musl ] && echo CXX11_DESTRUCTORS || echo default)"

if [ "$MODE" = override ]; then
    finish malloc free realloc calloc
else
    finish sn_malloc sn_free sn_realloc sn_calloc sn_aligned_alloc
fi
