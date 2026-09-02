#!/bin/sh
# THE QUESTION
#
#   What does portability cost? Binary size, startup time, peak memory and
#   build time, for the same source built three ways.
#
# -- WHY THIS EXISTS ---------------------------------------------------------
#
# The project's stated goal is "little to no runtime overhead", and until this
# script existed that was an argument from structure -- no extra process, no
# loader, no extraction -- with no number behind it. An argument from structure
# is not a measurement and docs/comparison.md said so with dashes.
#
#   arm N  native dynamic       what the distribution's own build produces
#   arm S  plain gcc -static    static, none of the portability mechanisms
#   arm P  pgb                  static + NSS override + static libiconv
#
# ⭐ ARM S IS THE CONTROL THAT MATTERS. The interesting question is not
# "static versus dynamic", which is well known; it is what pgb adds ON TOP of
# a static build, because that is the part this project is responsible for.
#
# -- HOW STARTUP IS MEASURED, AND WHY NOT WITH `time` ON ONE RUN -------------
#
# One run of a program that does almost nothing measures the scheduler. This
# takes ITERATIONS runs per round, ROUNDS rounds, and reports the MINIMUM
# round -- the minimum is the round least disturbed by everything else on the
# machine, and it is stable under noise that the mean is not.
#
# ⚠ THE NUMBER IS ONE MACHINE ON ONE DAY, and the conditions block above every
# run says which. It is not a benchmark of static linking in general.
#
# Exit: 0 measured, 1 an arm could not be built, 2 could not run.

. "$(dirname "$0")/lib.sh"

exp_begin "40 - what portability costs: size, startup, memory, build time"

ITERATIONS="${PGB_OVERHEAD_ITERS:-200}"
ROUNDS="${PGB_OVERHEAD_ROUNDS:-5}"

B="$EXP_OUT/build"
rm -rf "$B"; mkdir -p "$B" || exit 2

# The subject. Deliberately NOT a hello-world: it touches the three subsystems
# the portability work is about, so the constructor and the iconv shim are
# actually linked in rather than optimised away.
cat > "$B/subject.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <string.h>
#include <iconv.h>
#include <pwd.h>
#include <netdb.h>
#include <sys/socket.h>
int main(int argc, char **argv) {
    /* startup mode: do the minimum, so the measurement is dominated by
     * process setup rather than by the work. */
    if (argc > 1 && strcmp(argv[1], "--startup") == 0) return 0;
    struct passwd *pw = getpwuid(0);
    iconv_t cd = iconv_open("ISO-8859-1", "UTF-8");
    if (cd != (iconv_t)-1) iconv_close(cd);
    struct addrinfo h, *r; memset(&h, 0, sizeof h); h.ai_socktype = SOCK_STREAM;
    int s = getaddrinfo("localhost", "80", &h, &r);
    if (s == 0) freeaddrinfo(r);
    printf("%s %d\n", pw ? pw->pw_name : "?", s);
    return 0;
}
EOF

# --- build the three arms, timing each build -------------------------------
time_build() {  # label command...
  _lbl="$1"; shift
  _t0=$(date +%s%N)
  "$@" >>"$B/build.log" 2>&1 || return 1
  _t1=$(date +%s%N)
  echo $(( (_t1 - _t0) / 1000000 ))
}

BN=$(time_build native ${CC:-cc} -O2 -o "$B/subject-native" "$B/subject.c") \
  || { exp_skip "arm N" "native build failed"; BN=""; }
BS=$(time_build static ${CC:-cc} -O2 -static -o "$B/subject-static" "$B/subject.c") \
  || { exp_skip "arm S" "static build failed"; BS=""; }

# ⚠ arm P goes through `pgb build`, which on this machine enters a chroot.
# Its build time therefore includes entering that environment, and the row
# below says so rather than presenting it as compiler time.
BP=""
if [ -x "$REPO_DIR/pgb" ]; then
  _t0=$(date +%s%N)
  if "$REPO_DIR/pgb" --bind "$B" build -- /bin/sh -c \
       "\$CC -O2 -o '$B/subject-pgb' '$B/subject.c'" >>"$B/build.log" 2>&1; then
    _t1=$(date +%s%N); BP=$(( (_t1 - _t0) / 1000000 ))
  else
    exp_skip "arm P" "pgb build failed; see $B/build.log"
  fi
fi

exp_check "arm N built" "$([ -x "$B/subject-native" ] && echo yes || echo no)" yes
exp_check "arm S built" "$([ -x "$B/subject-static" ] && echo yes || echo no)" yes
exp_check "arm P built" "$([ -x "$B/subject-pgb" ] && echo yes || echo no)" yes

# --- startup ---------------------------------------------------------------
# Minimum of ROUNDS rounds of ITERATIONS execs, in milliseconds per round.
startup_ms() {  # binary -> best round total, ms
  _bin="$1"; _best=""
  _r=0
  while [ "$_r" -lt "$ROUNDS" ]; do
    _t0=$(date +%s%N)
    _i=0
    while [ "$_i" -lt "$ITERATIONS" ]; do "$_bin" --startup; _i=$((_i+1)); done
    _t1=$(date +%s%N)
    _ms=$(( (_t1 - _t0) / 1000000 ))
    if [ -z "$_best" ] || [ "$_ms" -lt "$_best" ]; then _best=$_ms; fi
    _r=$((_r+1))
  done
  printf '%s' "$_best"
}

# --- peak RSS --------------------------------------------------------------
# ⚠ Measured from OUTSIDE the process, never by asking it.
# docs/methodology/experiments.md: "a subject's self-report is not a
# measurement".
#
# ⛔ getrusage(RUSAGE_CHILDREN).ru_maxrss IS THE WRONG CALL AND IT LOOKS RIGHT.
# It is a high-water mark across EVERY child the caller has ever reaped, so
# subtracting successive readings does not give per-process figures: the first
# arm measured "10300 KiB", the second "128" and the third "0", which is an
# artefact of the ordering and not three measurements.
#
# os.wait4() returns the rusage of THAT child, which is the per-process number.
# The minimum of several runs is reported, for the same reason as startup.
peak_rss_kb() { # binary -> KiB, or a dash when python3 is absent
  python3 -c '' 2>/dev/null || { printf -- "-"; return; }
  python3 - "$1" <<'PYRSS'
import os, sys
prog = sys.argv[1]
best = None
for _ in range(5):
    pid = os.fork()
    if pid == 0:
        try: os.execv(prog, [prog, "--startup"])
        except Exception: os._exit(127)
    _, _, ru = os.wait4(pid, 0)
    best = ru.ru_maxrss if best is None else min(best, ru.ru_maxrss)
print(best if best is not None else "-", end="")
PYRSS
}

printf '\n  conditions: %s iterations x %s rounds, best round reported\n' "$ITERATIONS" "$ROUNDS"
printf '  %-22s %12s %14s %13s %11s %11s\n' ARM 'SIZE (B)' 'STARTUP (ms)' 'PER EXEC (us)' 'RSS (KiB)' 'BUILD (ms)'

row() { # label binary buildms
  [ -x "$2" ] || { printf '  %-22s %12s %14s %13s %11s %11s\n' "$1" - - - - -; return; }
  _ms=$(startup_ms "$2")
  _us=$(( _ms * 1000 / ITERATIONS ))
  printf '  %-22s %12s %14s %13s %11s %11s\n' \
    "$1" "$(wc -c < "$2")" "$_ms" "$_us" "$(peak_rss_kb "$2")" "${3:--}"
}

row "N native dynamic"   "$B/subject-native" "$BN"
row "S plain gcc -static" "$B/subject-static" "$BS"
row "P pgb"              "$B/subject-pgb"    "$BP"

printf '\n'
exp_note "STARTUP is the total for $ITERATIONS execs, so per-exec cost is that"
exp_note "  number divided by $ITERATIONS. Arm P's extra work at startup is one"
exp_note "  constructor making 14 __nss_configure_lookup calls."
exp_note "BUILD (ms) for arm P includes entering the chroot build environment,"
exp_note "  which arms N and S do not pay. It is not compiler time."
exp_note "⚠ One machine, one day. See the conditions block above."

{
  printf 'iterations=%s rounds=%s\n' "$ITERATIONS" "$ROUNDS"
  printf 'native  size=%s build_ms=%s\n' "$(wc -c < "$B/subject-native" 2>/dev/null)" "$BN"
  printf 'static  size=%s build_ms=%s\n' "$(wc -c < "$B/subject-static" 2>/dev/null)" "$BS"
  printf 'pgb     size=%s build_ms=%s\n' "$(wc -c < "$B/subject-pgb" 2>/dev/null)" "$BP"
} > "$EXP_OUT/overhead.txt"

exp_finish
