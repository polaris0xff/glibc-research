#!/bin/sh
# THE QUESTION
#
#   Does a `gcc -static` glibc binary carry its own timezone database, and if
#   not, what does it do on the environments that ship none?
#
# ⛔ WHY THIS EXPERIMENT EXISTS, AND IT IS NOT A HUNCH. `docs/REQUIREMENTS.md`
# enumerates NINE ways static glibc is not self-contained and says of them:
# *"there is no unenumerated remainder"*. That sentence is the whole reason
# part 2 of the operator's bar is countable, so it is worth attacking rather
# than believing. The operator asked on 2026-09-03c to *"fix all remaining
# GLIBC quirks if there still are some"* -- which is a question about
# completeness, not about the eight that are closed.
#
# ⭐ THE LIST OF NINE HAS NO TIMEZONE ROW, and `grep -rn zoneinfo` over the
# whole tree found nothing: not in docs/, not in TODO/, not in an experiment,
# not in the runtime. Nobody had looked.
#
# -- WHAT IS ACTUALLY BEING MEASURED -----------------------------------------
#
# Three facts, in the order that makes the third one mean something.
#
#   1. static libc.a hardcodes /etc/localtime, /usr/share/zoneinfo and TZDIR.
#      A `strings` question, answered on the archive itself.
#   2. how many of the eleven ship a timezone database at all.
#      A directory question, answered on the rootfs trees.
#   3. what ONE static binary asking for TZ=Europe/Berlin prints on each of
#      them -- which is the only one of the three that shows the CONSEQUENCE.
#
# ⛔ THE FAILURE MODE IS THE POINT, AND IT IS WORSE THAN "IT RETURNS UTC".
# MEASURED, not predicted: with no database, glibc does not report an error and
# does not print `UTC`. It re-reads `TZ=Europe/Berlin` as a POSIX zone
# specification -- a bare abbreviation with no offset -- and prints
#
#     Europe +0000 00
#
# the zone name you ASKED FOR, with a UTC offset. So the field that looks like
# a confirmation is an echo of the input, and the only field carrying the
# defect is the offset. A log line reading "Europe" beside a timestamp two
# hours off is the shape of this bug in production.
#
# That is the same class as gconv (experiment 30): a silent wrong answer at the
# point of USE, long after any startup check would have passed.
#
# ⚠ AND THE HONEST SCOPE. A DYNAMIC binary on the same host cannot resolve the
# zone either -- the database is genuinely absent. What makes this a row of the
# nine rather than a fact about Alpine is the same thing that made terminfo and
# the CA bundle rows of the nine: the promise is one ordinary ELF that works
# everywhere, and this is data glibc needs, does not carry, and fails silently
# without. `docs/REQUIREMENTS.md` set that precedent for `--embed-terminfo`
# and `--embed-cacert`; this measures whether a third one is owed.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "97 - the timezone database: a tenth way static glibc is not self-contained"

WORK="${PGB_EXP97_WORK:-/var/tmp/pgb-exp97}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { exp_note "no $CC on PATH"; exit 2; }

exp_conditions

# -- 1. what the static archive itself names ---------------------------------
#
# ⛔ READ THE ARCHIVE, NOT THE DOCUMENTATION. The question is whether the code
# that will be linked INTO the binary carries a compiled-in path, and the only
# thing that can answer it is the archive.
LIBC_A=""
for p in /usr/lib/x86_64-linux-gnu/libc.a /usr/lib64/libc.a /usr/lib/libc.a; do
  [ -e "$p" ] && { LIBC_A="$p"; break; }
done
if [ -n "$LIBC_A" ]; then
  # ⚠ PRESENCE, NOT A COUNT. The first version asserted "= 1" and went red at
  # 2: glibc names /etc/localtime in more than one translation unit, which is
  # a fact about glibc's source layout and not about this question.
  _has() { [ "$(strings -a "$LIBC_A" 2>/dev/null | grep -cx "$1")" -gt 0 ] && echo yes || echo no; }
  exp_check "libc.a names /usr/share/zoneinfo"      "$(_has '/usr/share/zoneinfo')" yes
  exp_check "libc.a names /etc/localtime"           "$(_has '/etc/localtime')"      yes
  exp_check "libc.a honours TZDIR"                  "$(_has 'TZDIR')"               yes
  exp_note  "so the database is HOST data: nothing about it is linked in."
else
  exp_skip "libc.a names the zoneinfo paths" "no libc.a on this host"
  exp_skip "libc.a names /etc/localtime"     "no libc.a on this host"
  exp_skip "libc.a honours TZDIR"            "no libc.a on this host"
fi

# -- the subject -------------------------------------------------------------
#
# ⚠ IT PRINTS, IT DOES NOT ASSERT. The program's job is to report what glibc
# gave it; deciding whether that is right belongs to this script, which knows
# what the environment was supposed to have.
cat > "$WORK/tz.c" <<'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
int main(void) {
    time_t t = 1751328000;             /* 2025-07-01 00:00:00 UTC, DST season */
    struct tm tm;
    char z[64], o[64];
    setenv("TZ", "Europe/Berlin", 1);
    tzset();
    if (!localtime_r(&t, &tm)) { puts("NOLOCALTIME"); return 2; }
    strftime(z, sizeof z, "%Z", &tm);
    strftime(o, sizeof o, "%z", &tm);
    printf("%s %s %02d\n", z, o, tm.tm_hour);
    return 0;
}
EOF
"$CC" -static -o "$WORK/tz" "$WORK/tz.c" > "$WORK/cc.log" 2>&1 \
  || { exp_note "the static arm did not link: $(tail -1 "$WORK/cc.log")"; exit 2; }

exp_check "the subject has no PT_INTERP" \
  "$(readelf -lW "$WORK/tz" 2>/dev/null | grep -c INTERP)" 0
exp_check "...and no DT_NEEDED" \
  "$(readelf -dW "$WORK/tz" 2>/dev/null | grep -c NEEDED)" 0

# -- 2. the control: the build host HAS a database ---------------------------
#
# ⛔ WITHOUT THIS ROW NOTHING BELOW MEANS ANYTHING. A binary that printed
# `UTC +0000` because it was built wrong would look exactly like one that
# printed it because the host had no database.
HOST_OUT=$("$WORK/tz" 2>/dev/null)
exp_note "build host: $HOST_OUT"
exp_check "the CONTROL resolves Europe/Berlin on the build host" \
  "$(printf '%s' "$HOST_OUT" | awk '{print $2}')" "+0200"
exp_check "...and the hour is the summer offset, not UTC" \
  "$(printf '%s' "$HOST_OUT" | awk '{print $3}')" "02"

# -- 3. the eleven -----------------------------------------------------------
printf '\n'
printf -- '-- the eleven: does the host carry a zone database, and what does the\n'
printf -- '   SAME static binary print when it asks for Europe/Berlin -----------\n'
printf '  %-20s %-6s %8s %-8s %-7s %s\n' ENVIRONMENT LIBC ZONEFILES /etc/lt PRINTED VERDICT
HAVE=0; MISSING=0; WRONG=0; ROWS=0
for name in $(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt"); do
  r=$(exp_rootfs "$name") || true
  [ -n "$r" ] || continue
  ROWS=$((ROWS+1))
  libc=$(exp_rootfs_libc "$name")
  n=$(find "$r/usr/share/zoneinfo" -type f 2>/dev/null | wc -l | tr -d ' ')
  lt=$( [ -e "$r/etc/localtime" ] && echo yes || echo no )
  # ⛔ `rootfs run` TAKES A PATH, NOT A NAME. The first version passed the
  # name and every row printed nothing, which read as "the binary produced no
  # output on all eleven" -- a far more alarming result than the truth.
  out=$("$REPO_DIR/pgb" rootfs run "$r" --copy "$WORK/tz:/tz" -- /tz 2>/dev/null | tr -d '\r')
  off=$(printf '%s' "$out" | awk '{print $2}')
  case "$n:$off" in
    0:*)      v="⛔ NO DATABASE"; MISSING=$((MISSING+1)) ;;
    *:+0200)  v="ok";             HAVE=$((HAVE+1)) ;;
    *)        v="⛔ HAS DATA, WRONG ANSWER"; WRONG=$((WRONG+1)) ;;
  esac
  printf '  %-20s %-6s %8s %-8s %-7s %s\n' "$name" "$libc" "$n" "$lt" "${out:-<none>}" "$v"
done

printf '\n'
exp_check "every fetched environment answered"        "$((HAVE+MISSING+WRONG))" "$ROWS"
exp_check "environments that CANNOT resolve the zone" "$MISSING" 4
exp_check "environments with a database that answer correctly" "$WRONG" 0
exp_note "⛔ the $MISSING that cannot do not SAY so: they print \`Europe +0000\`,"
exp_note "   the zone name ASKED FOR with a UTC offset. The %Z field echoes"
exp_note "   the input, so only the offset carries the defect."
exp_note "⚠ AND IT IS NOT A MUSL STORY: ubuntu-20.04 is glibc and is one of the"
exp_note "   four. Three of the four are Alpine; the fourth is a Debian family"
exp_note "   image that simply does not install tzdata."

exp_finish
