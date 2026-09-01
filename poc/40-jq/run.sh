#!/bin/sh
# POC: jq
#
# WHY THIS PROJECT
#   jq is the Unicode case, and it is the one where a portability failure is
#   least likely to announce itself.
#
#   unicode  jq parses and re-emits JSON, which is Unicode by specification.
#            It decodes \uXXXX escapes, encodes surrogate pairs, and must
#            round-trip arbitrary UTF-8 through its own string machinery. A
#            locale that has collapsed to ASCII does not stop jq; it changes
#            what it produces. That is silent corruption, not a crash, and it
#            is the failure mode this POC is here to catch.
#   iconv    jq's @base64d and its string handling reach libc's multibyte
#            machinery.
#   oniguruma an external regex engine, optional at build time, that makes
#            test/match/capture work. Built static here, so this POC also
#            exercises an OPTIONAL dependency being detected by configure --
#            a build that silently drops it would still pass a naive test.
#
# NORMAL BUILD    ./configure --with-oniguruma && make
# WHY STATIC GLIBC IS HARD HERE
#   Nothing about jq crashes. Everything about it can go subtly wrong: the
#   regex engine can be silently absent, and the Unicode paths can silently
#   change behaviour when the locale is not what the build assumed.

. "$(dirname "$0")/../common.sh"

POC_URL="https://github.com/jqlang/jq/releases/download/jq-1.7.1/jq-1.7.1.tar.gz"
POC_VERSION="1.7.1"
POC_SHA256="478c9ca129fd2e3443fe27314b455e211e0d8c60bc8ff7df703873deeee580c2"
POC_NORMAL_BUILD="./configure --with-oniguruma && make"
POC_STRESSES="Unicode round-trip, surrogate pairs, optional static dependency detection"
POC_WHY="Unicode correctness, where the failure is silent rather than fatal"

ONIG_URL="https://github.com/kkos/oniguruma/releases/download/v6.9.9/onig-6.9.9.tar.gz"
ONIG_VERSION="6.9.9"
ONIG_SHA256="60162bd3b9fc6f4886d4c7a07925ffd374167732f55dce8c491bfd9cd818a6cf"

poc_begin
poc_note "dependency: oniguruma $ONIG_VERSION, built static in the same environment"

PREFIX="$WORK/prefix-jq"
BIN="$POC_OUT/jq"

poc_functional_test() {
cat <<'TEST'
set -u
fail=0
t() { if [ "$2" = "$3" ]; then printf '  ok   %-30s %s\n' "$1" "$2"
      else printf '  FAIL %-30s got [%s] want [%s]\n' "$1" "$2" "$3"; fail=1; fi; }

# 1. the basic filter
t identity "$(echo '{"a":1}' | /jq -c '.')" '{"a":1}'

# 2. arithmetic and construction
t arithmetic "$(echo '{"a":2,"b":3}' | /jq -c '{s:(.a+.b)}')" '{"s":5}'

# 3. ⭐ UTF-8 ROUND TRIP. Bytes in must be bytes out. A locale collapsed to
#    ASCII is the case that damages this rather than failing it.
t utf8-roundtrip "$(printf '{"k":"caf\303\251 na\303\257ve"}' | /jq -r '.k')" "café naïve"

# 4. ⭐ \u ESCAPE DECODING, including a character outside the BMP expressed as
#    a surrogate pair. This is jq's own Unicode code, exercised end to end.
t u-escape "$(echo '{"k":"é"}' | /jq -r '.k')" "é"
t surrogate-pair "$(echo '{"k":"😀"}' | /jq -r '.k' | wc -c | tr -d ' ')" "5"

# 5. ⭐ CHARACTER LENGTH, NOT BYTE LENGTH. jq must count codepoints. "café"
#    is 4 characters and 5 bytes, so this single number distinguishes the two.
t codepoint-length "$(printf '{"k":"caf\303\251"}' | /jq '.k|length')" "4"

# 6. ⚠ -a MEANS "ASCII OUTPUT", so escaping to \u00e9 is the CORRECT answer.
#    The first version of this check expected the unescaped form and failed on
#    all 11 environments -- a wrong expectation, not a portability defect, and
#    a reminder that a red matrix is a claim about the test as much as about
#    the binary. Both directions are asserted now.
t ascii-escape "$(printf '{"k":"caf\303\251"}' | /jq -ac '.k')" '"caf\u00e9"'
t utf8-passthrough "$(printf '{"k":"caf\303\251"}' | /jq -c '.k')" '"café"'

# 7. ⛔ THE OPTIONAL DEPENDENCY, ASSERTED. If oniguruma was not detected at
#    configure time jq still builds, still passes every check above, and
#    silently has no regex support at all. Without this line the POC would
#    report success on a jq missing a headline feature.
t regex-match "$(echo '"hello world"' | /jq -r 'test("^h.*d$")')" "true"
t regex-capture "$(echo '"2024-05-06"' | /jq -r '[match("([0-9]+)-([0-9]+)").captures[].string]|join(",")')" "2024,05"

# 8. base64, which goes through the byte/character boundary
t base64 "$(printf '{"k":"caf\303\251"}' | /jq -r '.k|@base64')" "Y2Fmw6k="
t base64d "$(echo '"Y2Fmw6k="' | /jq -r '@base64d')" "café"

# 9. sorting and grouping over a real document
t sort "$(echo '[3,1,2]' | /jq -c 'sort')" '[1,2,3]'
t group "$(echo '[{"k":"a"},{"k":"b"},{"k":"a"}]' | /jq -c 'group_by(.k)|map(length)')" '[2,1]'

# 10. numeric precision, a known-fragile area across libc printf implementations
t bignum "$(echo '100000000000000000000' | /jq -c '.')" "100000000000000000000"
exit $fail
TEST
}

poc_observation_probe() {
cat <<'PROBE'
# What does jq report about its own locale-dependent view of the world?
l=$(/jq -rn 'env.LANG // "unset"' 2>/dev/null)
n=$(printf '{"k":"caf\303\251"}' | /jq '.k|length' 2>/dev/null)
echo "LANG=$l codepoint-length-of-cafe=$n"
PROBE
}

# ---------------------------------------------------------------------------
if [ ! -x "$BIN" ] || [ "${POC_REBUILD:-0}" = 1 ]; then
  poc_fetch "$ONIG_URL" "$WORK/onig-$ONIG_VERSION.tar.gz" "$ONIG_SHA256" || exit 2
  poc_fetch "$POC_URL" "$WORK/jq-$POC_VERSION.tar.gz" "$POC_SHA256" || exit 2
  rm -rf "$WORK/onig-$ONIG_VERSION" "$WORK/jq-$POC_VERSION" "$PREFIX"
  tar xzf "$WORK/onig-$ONIG_VERSION.tar.gz" -C "$WORK" || exit 2
  tar xzf "$WORK/jq-$POC_VERSION.tar.gz" -C "$WORK" || exit 2

  poc_in_env "cd '$WORK/onig-$ONIG_VERSION' && \
      ./configure --prefix='$PREFIX' --disable-shared --enable-static \
      >'$POC_OUT/onig-configure.log' 2>&1 && \
      make -j\$(nproc) >'$POC_OUT/onig-make.log' 2>&1 && \
      make install >>'$POC_OUT/onig-make.log' 2>&1" \
    || { poc_note "oniguruma build failed"; tail -15 "$POC_OUT/onig-make.log" 2>/dev/null; exit 1; }

  poc_in_env "cd '$WORK/jq-$POC_VERSION' && \
      PKG_CONFIG_PATH='$PREFIX/lib/pkgconfig' \
      ./configure --with-oniguruma='$PREFIX' --disable-shared --enable-static \
        --disable-docs --disable-valgrind \
      >'$POC_OUT/configure.log' 2>&1 && \
      make -j\$(nproc) >'$POC_OUT/make.log' 2>&1" \
    || { poc_note "jq build failed"; tail -25 "$POC_OUT/make.log" 2>/dev/null; exit 1; }

  cp "$WORK/jq-$POC_VERSION/jq" "$BIN" 2>/dev/null || cp "$WORK/jq-$POC_VERSION/.libs/jq" "$BIN" || exit 2
fi

poc_check "built" "$([ -x "$BIN" ] && echo yes || echo no)" yes
poc_inspect "$BIN"
poc_matrix "$BIN"
poc_observe "$BIN" "the locale jq sees, and whether it still counts codepoints"
poc_finish
