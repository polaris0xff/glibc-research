#!/bin/sh
# poc/70-sqlite-extensions -- SQLite loading FIFTEEN of its own extensions
# out of an EMPTY directory, on all eleven environments.
#
# -- WHY THIS PROJECT, AND WHY IT IS THE RIGHT ONE FOR TODO T-002/T-030 -----
#
# POC 50 links CPython's 49 extension modules in by hand, through CPython's
# own Modules/Setup.local. ⛔ That is a mechanism CPython HAPPENS TO HAVE, and
# a POC built on it demonstrates two routes agreeing rather than the tool
# reaching something that was out of reach. T-030's acceptance was corrected
# for exactly that reason and asks instead for
#
#   "a project whose plugin loading is NOT configurable at build time -- i.e.
#    one with no Setup.local equivalent -- with its plugin directory emptied
#    and the functionality intact, on 11 of 11."
#
# ⭐ SQLite qualifies strictly. Its loadable-extension interface is an OPEN
# ABI: `.load ./series` calls dlopen() on a path the user names, derives an
# entry point from the FILENAME, and calls it. There is no configure switch,
# no Setup.local, no generated table -- to link an extension in and keep
# `.load` working you would have to edit sqlite3.c. Measured, not assumed:
# sqlite3LoadExtension() calls sqlite3OsDlOpen() with NO stat, no access
# check, and retries with ".so" appended if the first returns NULL.
#
# ⭐ AND IT IS AT SCALE. sqlite's own ext/misc/ ships dozens of these; fifteen
# are built here, every one third-party-shaped: a plain .c file, compiled
# separately, loaded by name.
#
# -- ⛔ WHAT BUILDING IT AT SCALE FOUND, WHICH ONE PLUGIN NEVER WOULD --------
#
# `--wrap-dlopen` puts the plugin objects in ONE executable, and SQLite's
# extension ABI requires every extension to define a file-scope, non-static
#
#     const sqlite3_api_routines *sqlite3_api;      (SQLITE_EXTENSION_INIT1)
#
# ⛔ ALL SIXTEEN of sqlite's ext/misc extensions define it, so ANY TWO of them
# collided at LINK time and the build stopped:
#
#     ld: uuid.o:(.bss+0x0): multiple definition of `sqlite3_api';
#         series.o:(.bss+0x0): first defined here
#
# ⛔ Worse, sqlite derives an entry point from the filename keeping only
# ALPHABETIC characters, so base64.c and base85.c both define
# `sqlite3_base_init` ON PURPOSE. Two plugins colliding on their entry point
# is a thing upstreams deliberately do.
#
# ⭐ Fixed in tool/lib/wrappers.sh by giving each plugin the namespace the
# loader would have given it: every symbol a plugin object defines is renamed
# with `objcopy --redefine-syms` to a per-plugin prefix, and the table maps
# the ORIGINAL name to the renamed one. That is RTLD_LOCAL, at link time.
# This POC keeps the collision as a live check -- see the last section.
#
# -- ARMS -------------------------------------------------------------------
#
#   wrapped   pgb --wrap-dlopen, one spec per extension. THE EXTENSION
#             DIRECTORY IS CREATED EMPTY on every target.
#   control   pgb WITHOUT --wrap-dlopen, with real .so files staged in.
#             Observed, never asserted: it reaches the host loader, which is
#             docs/limitations.md §1 and is host-dependent by nature.
#
# ⛔ THE FUNCTIONAL TEST ASSERTS VALUES, NOT LOADING. A dlopen that returns a
# handle proves a table lookup. Each extension is called and its result
# compared, so a plugin that loaded and did nothing fails.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/../common.sh"

POC_URL="https://www.sqlite.org/2024/sqlite-autoconf-3470000.tar.gz"
POC_VERSION="SQLite 3.47.0 + 15 extensions from sqlite/sqlite ext/misc @ version-3.47.0"
POC_SHA256="83eb21a6f6a649f506df8bd3aab85a08f7556ceed5dbd8dea743ea003fc3a957"
POC_NORMAL_BUILD="cc shell.c sqlite3.c -ldl -lm; each extension: cc -shared -fPIC ext.c"
POC_WHY="a program with an OPEN plugin ABI and no way to link a plugin in"
POC_STRESSES="dlopen by user-supplied path, dlsym by derived name, 15 plugins in one link, per-plugin symbol namespaces"

poc_begin

SRC="$WORK/70-sqlite"
mkdir -p "$SRC" || exit 2

# The extension set. ⚠ base85 is deliberately NOT here: it collides with
# base64 on `sqlite3_base_init` by upstream design, and the last section of
# this POC uses that collision as a check rather than avoiding it silently.
#
# ⛔ AND `percentile` IS NOT HERE EITHER, WITH THE CONTROL THAT SAYS WHY.
# It was in the first version of this POC and produced a SEGFAULT on all
# eleven environments, which reads exactly like a defect in --wrap-dlopen.
# It is not. `ext/misc/percentile.c` at version-3.47.0 segfaults AT LOAD
# TIME -- before any query runs, `.load ./percentile` + `SELECT 1;` is enough
# -- in an ORDINARY DYNAMICALLY LINKED sqlite3 built from this same
# amalgamation, loading a real .so through the real dlopen:
#
#   $ ./sq3 :memory: ".load ./percentile" "SELECT 1;"
#   Segmentation fault                                  exit 139
#
# Reproduced with two different build configurations (with and without
# SQLITE_THREADSAFE=0). ⭐ THE CONTROL IS THE POINT: without it this POC would
# have reported eleven segfaults against the mechanism under test. Replaced
# with `csv`, which is the same shape and works.
EXTS="base64 carray csv decimal ieee754 regexp rot13 series sha1 shathree spellfix totype uint uuid zorder"

ext_sha() {
  case "$1" in
    base64)     echo 53e9ae06be66476ad452178c331cfb2c01dfcbb001b6f506e107d4d32a0e55f1 ;;
    base85)     echo 892e83c7de3c3c877c5ea71e59b251e209846f2339eaa97b7f9ef0507be2b65f ;;
    carray)     echo 1f5a73ed200e66f472f2f62d8c7125dfd6035a247106ba6673b73f86eca1cd34 ;;
    decimal)    echo 8586c0225994b0c822eb2d7fc4aeb1efe1c6ed5499dc757d332ccc126940c993 ;;
    ieee754)    echo 7ed36b4e839880ac5beb6ec43d2623fd2108861146dc5de3f56037aaf4c526f5 ;;
    csv)        echo 86e321052107d1a58aafaf73220526784604822eec0943aa5c2c26c3303d9803 ;;
    regexp)     echo 383d259f876590869bb9c8f30e3594f1a999b8e1546660595da62abf3f3c9136 ;;
    rot13)      echo 2502067e568c60a0d7899850981a5864dc879bb6f01d280e97954709f4d1d333 ;;
    series)     echo 7c6da27e1a37d7f95ba187021d579db34964f7119e931463987541e2902f3578 ;;
    sha1)       echo a598b927e504a34faba57e52702f0c9285d192c93f53c0543cfdaf82602a39c2 ;;
    shathree)   echo 7bebbca59a2ebedcda3d9f0908df7b461ce06a09e20a5ad512321bfb4c698d1e ;;
    spellfix)   echo 41f1d5baf0dcb19ac7ea9d82e37faefc8d1cbe340370aae9aa20c08492307f33 ;;
    totype)     echo 75087ced4001102bfe841b27a36de25c5f1c9d87dd591e22a83ca5368768ff11 ;;
    uint)       echo 856161e14db6ab889ef446c30f7d52a1166638b82b9852d6c2008330d3cf4660 ;;
    uuid)       echo 8b90b44e4c3aebe75f9f9e1ab9f59b73e2590a621668636831f210690f9ae20c ;;
    zorder)     echo 007aef5527b137f23696722c2a23b4538389fb5055765ec1afd49231c4c1e50e ;;
  esac
}

# ⛔ RULES.md route 2: github.com answers this path with JSON, not a tarball,
# through this environment's proxy. Measured here: the direct fetch produced
# 378 bytes of JSON and the proxied one produced the 15 MiB archive. Every
# non-GitHub-API fetch goes through api.rv.pkgforge.dev with the ORIGINAL URL,
# scheme included.
RV="https://api.rv.pkgforge.dev"
SQL_BASE="https://raw.githubusercontent.com/sqlite/sqlite/version-3.47.0/ext/misc"

printf -- '-- fetching ---------------------------------------------------\n'
poc_fetch "$RV/$POC_URL" "$SRC/sqlite.tar.gz" "$POC_SHA256" \
  || { poc_skip "fetch sqlite" "download or checksum failed"; poc_finish; }
[ -d "$SRC/sqlite" ] || { mkdir -p "$SRC/sqlite" && tar xzf "$SRC/sqlite.tar.gz" -C "$SRC/sqlite" --strip-components=1; } \
  || { poc_skip "unpack sqlite" "tar failed"; poc_finish; }

mkdir -p "$SRC/ext"
nfetched=0
for e in $EXTS base85; do
  poc_fetch "$RV/$SQL_BASE/$e.c" "$SRC/ext/$e.c" "$(ext_sha "$e")" \
    || { poc_skip "fetch $e.c" "download or checksum failed"; poc_finish; }
  nfetched=$((nfetched+1))
done
poc_check "extension sources fetched and verified" "$nfetched" "16"

# ---------------------------------------------------------------------------
# Build, inside the pinned environment.
# ---------------------------------------------------------------------------
printf -- '\n-- building ---------------------------------------------------\n'

# ⚠ The amalgamation, not autotools: shell.c + sqlite3.c is what the upstream
# tarball's own configure ends up compiling, and it keeps this POC's build
# short enough that the interesting part stays visible.
SQFLAGS="-O2 -I$SRC/sqlite -DSQLITE_ENABLE_LOAD_EXTENSION=1 -DSQLITE_THREADSAFE=0 -DSQLITE_OMIT_POPEN=1"

build_log="$POC_OUT/build.log"
: > "$build_log"

# 1. the extension objects (for the wrapped arm) and .so files (for control).
wrapspecs=""
nobj=0
for e in $EXTS; do
  poc_in_env "\$CC $SQFLAGS -fPIC -c -o $SRC/ext/$e.o $SRC/ext/$e.c" >>"$build_log" 2>&1 \
    || { poc_check "compile $e.o" failed ok; continue; }
  poc_in_env "\$CC $SQFLAGS -fPIC -shared -o $SRC/ext/$e.so $SRC/ext/$e.c" >>"$build_log" 2>&1 \
    || { poc_check "compile $e.so" failed ok; continue; }
  wrapspecs="$wrapspecs --wrap-dlopen $e.so=$SRC/ext/$e.o"
  nobj=$((nobj+1))
done
poc_check "extensions built (object and shared object)" "$nobj" "15"

# 2. the two arms.
#
# ⚠ `--wrap-dlopen` is repeatable and the specs must reach `pgb` as separate
# arguments, so this is one place where the spec list is intentionally
# unquoted. Every element is a shell-safe token by construction (an extension
# name and a path this script built).
# shellcheck disable=SC2086
"$PGB" --bind "$WORK" $wrapspecs build -- /bin/sh -c \
  "\$CC $SQFLAGS -o $SRC/sqlite3-wrapped $SRC/sqlite/shell.c $SRC/sqlite/sqlite3.c -lm" \
  >>"$build_log" 2>&1 || poc_check "link the wrapped arm" failed ok
"$PGB" --bind "$WORK" build -- /bin/sh -c \
  "\$CC $SQFLAGS -o $SRC/sqlite3-control $SRC/sqlite/shell.c $SRC/sqlite/sqlite3.c -lm" \
  >>"$build_log" 2>&1 || poc_check "link the control arm" failed ok

[ -f "$SRC/sqlite3-wrapped" ] || { poc_note "see $build_log"; poc_check "wrapped binary exists" no yes; poc_finish; }
poc_check "wrapped binary exists" yes yes
[ -f "$SRC/sqlite3-control" ] && poc_note "control  $(wc -c < "$SRC/sqlite3-control") bytes"
poc_note "wrapped  $(wc -c < "$SRC/sqlite3-wrapped") bytes, $nobj plugins compiled in"

printf -- '\n-- inspection -------------------------------------------------\n'
poc_check "PT_INTERP absent" \
  "$(readelf -lW "$SRC/sqlite3-wrapped" 2>/dev/null | grep -c INTERP)" "0"
poc_check "DT_NEEDED entries" \
  "$(readelf -dW "$SRC/sqlite3-wrapped" 2>/dev/null | grep -c NEEDED)" "0"
# ⭐ The namespacing, checked on the artefact rather than trusted: exactly one
# `sqlite3_api` would mean fifteen plugins sharing one, which is the bug.
# Fifteen prefixed ones means each has its own, which is what a .so would get.
poc_check "per-plugin sqlite3_api instances in the binary" \
  "$(nm "$SRC/sqlite3-wrapped" 2>/dev/null | grep -cE ' [bBdD] pgb_dl[0-9]+_sqlite3_api$')" "15"
poc_check "unnamespaced sqlite3_api in the binary" \
  "$(nm "$SRC/sqlite3-wrapped" 2>/dev/null | grep -cE ' [bBdD] sqlite3_api$')" "0"

# ---------------------------------------------------------------------------
# The functional test. Runs INSIDE each target, against an EMPTY /pgb-ext.
# ---------------------------------------------------------------------------
poc_functional_test() {
  cat <<'EOF'
set -u
# ⛔ THE POINT OF THIS POC: the directory the extensions are loaded from is
# created EMPTY. Nothing here is on disk. If anything below works, it worked
# out of the compiled-in table.
rm -rf /pgb-ext && mkdir -p /pgb-ext
if [ -n "$(ls -A /pgb-ext 2>/dev/null)" ]; then
  echo "FAIL: /pgb-ext is not empty"; exit 1
fi

run() { /sqlite3-wrapped :memory: "$@" 2>&1; }
fails=0
want() {  # label expected actual
  if [ "$2" = "$3" ]; then echo "  ok   $1"
  else echo "  FAIL $1: got [$3] want [$2]"; fails=$((fails+1)); fi
}

# ⚠ .load takes a path with no extension; sqlite retries with ".so" appended
# when the first dlopen returns NULL, so both paths through the wrapper get
# exercised on every single load.
LOAD='.load /pgb-ext/'

# series: a table-valued function. Assert the VALUES, not that it loaded.
want "series generate_series(1,5)" "1|2|3|4|5" \
  "$(run "${LOAD}series" "SELECT group_concat(value,'|') FROM generate_series(1,5);")"
want "rot13" "uryyb" "$(run "${LOAD}rot13" "SELECT rot13('hello');")"
want "uuid is 36 chars" "36" "$(run "${LOAD}uuid" "SELECT length(uuid());")"
want "sha1" "a9993e364706816aba3e25717850c26c9cd0d89d" \
  "$(run "${LOAD}sha1" "SELECT sha1('abc');")"
want "decimal add" "3.30" "$(run "${LOAD}decimal" "SELECT decimal_add('1.10','2.20');")"
# ieee754(M,E) builds a double from mantissa and exponent: 3 x 2^-1 = 1.5.
want "ieee754 from mantissa/exponent" "1.5" "$(run "${LOAD}ieee754" "SELECT ieee754(3,-1);")"
want "regexp" "1" "$(run "${LOAD}regexp" "SELECT 'abc123' REGEXP '[a-c]+[0-9]+';")"
want "base64 round trip" "hello" \
  "$(run "${LOAD}base64" "SELECT CAST(base64(base64(CAST('hello' AS BLOB))) AS TEXT);")"
want "uint collation orders numerically" "a2|a10" \
  "$(run "${LOAD}uint" "SELECT group_concat(x,'|') FROM (SELECT 'a10' AS x UNION SELECT 'a2' ORDER BY x COLLATE uint);")"
want "csv virtual table" "alpha|beta" \
  "$(run "${LOAD}csv" "CREATE VIRTUAL TABLE temp.t USING csv(data='1,alpha
2,beta'); SELECT group_concat(c1,'|') FROM t;")"
want "totype" "1" "$(run "${LOAD}totype" "SELECT tointeger('42') = 42;")"
want "zorder" "3" "$(run "${LOAD}zorder" "SELECT zorder(1,1);")"
want "shathree" "1" "$(run "${LOAD}shathree" "SELECT length(sha3('abc'))=32;")"
want "spellfix editdist3" "1" \
  "$(run "${LOAD}spellfix" "SELECT editdist3('kitten','sitting') > 0;")"
want "carray registers" "1" \
  "$(run "${LOAD}carray" "SELECT count(*) >= 0 FROM pragma_function_list WHERE name='carray';")"

# ⛔ THREE NEGATIVE ASSERTIONS. A wrapper that answered everything would pass
# every line above. It must NOT answer a plugin that is not in the table, and
# a real dlopen would not either -- the file is genuinely absent.
out=$(run ".load /pgb-ext/not_a_plugin" "SELECT 1;")
case "$out" in
  *rror*|*not*|*fail*|*cannot*) echo "  ok   an absent plugin is refused" ;;
  *) echo "  FAIL an absent plugin was NOT refused: [$out]"; fails=$((fails+1)) ;;
esac
# base85 was built but deliberately left OUT of the table, so it is a plugin
# that EXISTS as a source file and still must not resolve.
out=$(run ".load /pgb-ext/base85" "SELECT 1;")
case "$out" in
  *rror*|*not*|*fail*|*cannot*) echo "  ok   a plugin outside the table is refused" ;;
  *) echo "  FAIL base85 resolved and must not have: [$out]"; fails=$((fails+1)) ;;
esac
# ⭐ And the database itself must still work, so a binary that answered dlopen
# by breaking sqlite would not pass.
want "sqlite still works" "6" \
  "$(run 'CREATE TABLE t(a);INSERT INTO t VALUES(1),(2),(3);SELECT sum(a) FROM t;')"

echo "$([ "$fails" = 0 ] && echo PASSED || echo FAILED): $fails failure(s)"
[ "$fails" = 0 ]
EOF
}

printf -- '\n-- the matrix: fifteen plugins, empty plugin directory ---------\n'
poc_matrix "$SRC/sqlite3-wrapped"

# ---------------------------------------------------------------------------
# The collision check. ⭐ KEPT AS A LIVE CHECK, not written up and deleted.
# ---------------------------------------------------------------------------
# ---------------------------------------------------------------------------
# The control, OBSERVED and never asserted.
#
# ⛔ The same sqlite3, built by pgb WITHOUT --wrap-dlopen, with the fifteen
# REAL .so files staged in. What it does is host-dependent by nature --
# docs/limitations.md §1 -- so an expectation here would be a guess. It is
# recorded because it is the answer to "what does this project's current class
# do with the same program", and without it the eleven green rows above have
# nothing to be green against.
# ---------------------------------------------------------------------------
poc_observation_probe() {
  cat <<'EOF'
set -u
# The control gets the plugins FOR REAL, on disk, and the ordinary dlopen.
out=$(/sqlite3-control :memory: ".load /pgb-ext/series" \
        "SELECT group_concat(value,'|') FROM generate_series(1,3);" 2>&1)
st=$?
if [ "$st" = 0 ] && [ "$out" = "1|2|3" ]; then echo "loaded and worked"
elif [ "$st" -gt 128 ]; then echo "SIG$((st-128))"
else echo "refused: $(printf '%s' "$out" | head -1 | cut -c1-46)"
fi
EOF
}
if [ -f "$SRC/sqlite3-control" ]; then
  extras=""
  for e in $EXTS; do extras="$extras $SRC/ext/$e.so:/pgb-ext/$e.so"; done
  # shellcheck disable=SC2086
  poc_observe "$SRC/sqlite3-control" "no --wrap-dlopen, the fifteen real .so files present" $extras
fi

printf -- '\n-- the collision this POC found, still checked -----------------\n'
poc_note "sqlite derives an entry point from the filename keeping only letters,"
poc_note "so base64.c and base85.c both define sqlite3_base_init, and every"
poc_note "extension defines sqlite3_api. Two plugins in one link collide."

if poc_in_env "\$CC $SQFLAGS -fPIC -c -o $SRC/ext/base85.o $SRC/ext/base85.c" >>"$build_log" 2>&1; then
  # Without namespacing: link the two objects directly. MUST fail.
  if poc_in_env "\$CC -o $SRC/collide $SRC/ext/base64.o $SRC/ext/base85.o -shared" >>"$build_log" 2>&1; then
    poc_check "raw link of base64.o + base85.o" "linked" "collides"
  else
    poc_check "raw link of base64.o + base85.o" "collides" "collides"
  fi
  # With namespacing, through pgb: MUST succeed.
  if "$PGB" --bind "$WORK" \
       --wrap-dlopen "base64.so=$SRC/ext/base64.o" \
       --wrap-dlopen "base85.so=$SRC/ext/base85.o" \
       build -- /bin/sh -c \
       "\$CC $SQFLAGS -o $SRC/sqlite3-both $SRC/sqlite/shell.c $SRC/sqlite/sqlite3.c -lm" \
       >>"$build_log" 2>&1; then
    poc_check "the same two through --wrap-dlopen" "linked" "linked"
  else
    poc_note "see $build_log"
    poc_check "the same two through --wrap-dlopen" "collides" "linked"
  fi
else
  poc_skip "the collision check" "base85.o did not compile"
fi

poc_finish
