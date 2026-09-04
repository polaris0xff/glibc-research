#!/bin/sh
# THE QUESTION
#
#   `experiments/63-` found ONE axis where native musl beats both glibc
#   columns, 11-0: with no locale variable set at all, what codeset does the
#   process report? Can a glibc binary match musl there without lying about
#   anything else?
#
# -- WHAT WAS ALREADY MEASURED ----------------------------------------------
#
# ⛔ 63-'s Q3 WAS PRE-REGISTERED AND FALSIFIED ON BOTH HALVES. The prediction
# was that musl's minimal locale support would give it a poor codeset and that
# `pgb` would report UTF-8. The opposite happened:
#
#     setlocale(LC_ALL, "") then nl_langinfo(CODESET), no LANG set
#     glibc (vanilla AND pgb)   ANSI_X3.4-1968   11 of 11
#     native musl               UTF-8            11 of 11
#
# ⚠ musl's default charset IS UTF-8, unconditionally. Asked for `C.UTF-8` BY
# NAME, `pgb --embed-locale` answers UTF-8 on 11 of 11 and vanilla on 7 — so
# the mechanism works and this is a different question.
#
# ⛔ AND --embed-locale CANNOT MOVE IT, for a precise reason rather than an
# incidental one: that mechanism answers a REQUEST the host could not satisfy.
# Here the host satisfied the request — glibc returned "C" successfully.
# Nothing failed, so nothing fell back.
#
# -- WHAT IS BEING TESTED ----------------------------------------------------
#
# `--utf8-default`: when `setlocale(category, "")` is called AND LC_ALL,
# LC_CTYPE and LANG are all unset or empty, answer `C.UTF-8` instead of `C`.
#
# ⛔ THIS IS A CHANGE TO A DOCUMENTED DEFAULT, NOT A REPAIR, and the experiment
# is built to say so. POSIX leaves the choice to the implementation when the
# environment is silent; glibc chooses "C" and this chooses "C.UTF-8", which is
# what musl does unconditionally. A program that assumed a single-byte default
# sees a multibyte one. So it is opt-in, `pgb explain` prints it, and arm E
# below is the row that proves it does not go further than it claims.
#
# -- ⭐ PRE-REGISTERED EXPECTATION -------------------------------------------
#
# ⛔ COMMITTED BEFORE THE RUN. PROGRESS.md delivery rule 1.
#
#   U1  arm A (plain `gcc -static`) reports a NON-UTF-8 codeset on 11 of 11.
#       ⚠ This is the row that makes the rest non-vacuous.
#   U2  arm M (native `musl-gcc -static`) reports UTF-8 on 11 of 11 — the bar.
#   U3  arm B (`pgb build`, no flag) reports a NON-UTF-8 codeset on 11 of 11,
#       reproducing 63-'s finding with this instrument.
#   U4  ⭐ arm C (`pgb build --utf8-default`) reports UTF-8 on 11 of 11.
#   U5  ⛔ arm E — arm C's binary run with `LANG=C` EXPLICITLY set — reports a
#       NON-UTF-8 codeset on 11 of 11. The environment still decides when the
#       environment says anything, and a flag that overrode an explicit LANG
#       would be a bug wearing a feature's clothes.
#
# ⚠ U5 IS THE CRITERION THAT CAN FAIL FOR THE RIGHT REASON. Without it, a
# mechanism that simply forced UTF-8 unconditionally would score U4 green.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "67 - what an UNSET LANG means, and the one axis where native musl wins"

WORK="${PGB_EXP67_WORK:-/var/tmp/pgb-exp67}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { exp_note "no $CC on PATH"; exit 2; }

exp_conditions

# ⚠ IT PRINTS, IT DOES NOT ASSERT. Deciding whether a codeset is right belongs
# to this script, which knows what the environment was supposed to be.
cat > "$WORK/cs.c" <<'EOF'
#include <langinfo.h>
#include <locale.h>
#include <stdio.h>
int main(void) {
    const char *r = setlocale(LC_ALL, "");
    const char *cs = nl_langinfo(CODESET);
    printf("%s %s\n", (cs && *cs) ? cs : "EMPTY", r ? r : "NULL");
    return 0;
}
EOF

"$CC" -static -o "$WORK/csA" "$WORK/cs.c" >"$WORK/ccA.log" 2>&1 \
  || { exp_note "the control did not link: $(tail -1 "$WORK/ccA.log")"; exit 2; }

# ⭐ THE musl ARM IS THE BAR, AND ITS ABSENCE IS A SKIP RATHER THAN A GREEN
# RUN WITHOUT IT. A skip is not a pass — RULES.md.
MUSL=""
if command -v musl-gcc >/dev/null 2>&1; then
  if musl-gcc -static -o "$WORK/csM" "$WORK/cs.c" >"$WORK/ccM.log" 2>&1; then
    MUSL="$WORK/csM"
  else
    exp_note "musl-gcc is present but did not link: $(tail -1 "$WORK/ccM.log")"
  fi
fi

build_pgb() { # outname [pgb build flags...]
  _o=$1; shift
  _d="$WORK/$_o"; mkdir -p "$_d"; cp "$WORK/cs.c" "$_d/cs.c"
  if (cd "$_d" && "$REPO_DIR/pgb" build "$@" -- cc -o cs cs.c) >"$WORK/build-$_o.log" 2>&1 \
     && [ -x "$_d/cs" ]; then
    printf '%s' "$_d/cs"
  fi
}
BIN_B=$(build_pgb armB --embed-locale)
BIN_C=$(build_pgb armC --utf8-default)
[ -n "$BIN_B" ] || exp_note "arm B did not build; see $WORK/build-armB.log"
[ -n "$BIN_C" ] || exp_note "arm C did not build; see $WORK/build-armC.log"

# ⛔ THE ENVIRONMENT IS SCRUBBED, because the question is what happens when it
# says NOTHING. `env -u` on the three variables glibc reads, and on LANGUAGE
# which gettext reads, so a value inherited from the harness cannot decide the
# answer instead of the mechanism.
row() { # rootfs binary extra-env -> codeset
  _r=$1; _b=$2; _e=${3:-}
  "$REPO_DIR/pgb" rootfs run "$_r" --copy "$_b:/cs" -- \
    /bin/sh -c "unset LC_ALL LC_CTYPE LANG LANGUAGE; ${_e} /cs" 2>/dev/null \
    | tr -d '\r' | awk 'NR==1{print $1}'
}
is_utf8() { case "$1" in UTF-8|utf8|UTF8) echo yes ;; *) echo no ;; esac; }

printf '\n'
printf '  %-20s %-6s %-16s %-16s %-16s %-16s %s\n' \
  ENVIRONMENT LIBC 'A gcc -static' 'M musl -static' 'B pgb' 'C --utf8-default' 'E C, LANG=C'
A_NON=0; M_UTF=0; B_NON=0; C_UTF=0; E_NON=0; ROWS=0; MROWS=0
for name in $(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt"); do
  r=$(exp_rootfs "$name") || true
  [ -n "$r" ] || { exp_skip "$name" "not fetched"; continue; }
  ROWS=$((ROWS+1))
  libc=$(exp_rootfs_libc "$name")
  a=$(row "$r" "$WORK/csA")
  m="-"; if [ -n "$MUSL" ]; then m=$(row "$r" "$MUSL"); MROWS=$((MROWS+1)); fi
  b="-"; [ -n "$BIN_B" ] && b=$(row "$r" "$BIN_B")
  c="-"; [ -n "$BIN_C" ] && c=$(row "$r" "$BIN_C")
  e="-"; [ -n "$BIN_C" ] && e=$(row "$r" "$BIN_C" "LANG=C")
  [ "$(is_utf8 "${a:-EMPTY}")" = no ] && A_NON=$((A_NON+1))
  [ "$(is_utf8 "${m:-EMPTY}")" = yes ] && M_UTF=$((M_UTF+1))
  [ -n "$BIN_B" ] && [ "$(is_utf8 "${b:-EMPTY}")" = no ] && B_NON=$((B_NON+1))
  [ -n "$BIN_C" ] && [ "$(is_utf8 "${c:-EMPTY}")" = yes ] && C_UTF=$((C_UTF+1))
  [ -n "$BIN_C" ] && [ "$(is_utf8 "${e:-EMPTY}")" = no ] && E_NON=$((E_NON+1))
  printf '  %-20s %-6s %-16s %-16s %-16s %-16s %s\n' \
    "$name" "$libc" "${a:-<none>}" "${m:-<none>}" "${b:-<none>}" "${c:-<none>}" "${e:-<none>}"
done

printf '\n'
exp_check "U1  A: a NON-UTF-8 codeset on every row"     "$A_NON" "$ROWS"
if [ -n "$MUSL" ]; then
  exp_check "U2  M: native musl reports UTF-8"          "$M_UTF" "$MROWS"
else
  exp_skip "U2  M: native musl reports UTF-8" "no musl-gcc here: apt-get install musl-tools"
fi
if [ -n "$BIN_B" ]; then
  exp_check "U3  B: --embed-locale alone does NOT move it" "$B_NON" "$ROWS"
else
  exp_skip "U3  B: --embed-locale alone does NOT move it" "arm B did not build"
fi
if [ -n "$BIN_C" ]; then
  exp_check "U4  ⭐ C: --utf8-default reports UTF-8"     "$C_UTF" "$ROWS"
  exp_check "U5  ⛔ E: an EXPLICIT LANG=C is still obeyed" "$E_NON" "$ROWS"
else
  exp_skip "U4  ⭐ C: --utf8-default reports UTF-8"      "arm C did not build"
  exp_skip "U5  ⛔ E: an EXPLICIT LANG=C is still obeyed" "arm C did not build"
fi

exp_note "⭐ WHAT U4 AND U5 SAY TOGETHER. U4 alone would also be scored green"
exp_note "   by a mechanism that forced UTF-8 unconditionally, which would be"
exp_note "   a bug: a program run under LANG=C asked for a single-byte locale"
exp_note "   and must get one. U5 is the row that can fail for that reason."
exp_note "⚠ AND THE AXIS IS ONE AXIS. This closes the environment-default"
exp_note "   codeset and says nothing about collation, message catalogues or"
exp_note "   any other locale category: C.UTF-8 is the C locale with a UTF-8"
exp_note "   charset, not a full locale."

exp_finish
