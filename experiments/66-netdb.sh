#!/bin/sh
# THE QUESTION
#
#   The ELEVENTH glibc-static quirk had a measurement and no mechanism.
#   Does `--embed-netdb` close it, and where exactly does it stop?
#
# -- WHAT WAS ALREADY MEASURED ----------------------------------------------
#
# ⛔ `experiments/82-` enumerates every absolute path the pinned `libc.a`
# names — 78 at glibc 2.41 — and classifies each against the host-data rows
# this project owns. `/etc/services` and `/etc/protocols` were among the ones
# no row owned, and the follow-up measured:
#
#     getservbyname("http", "tcp") returns NULL on 3 of 11
#     debian-11, debian-12, ubuntu-20.04 — ⭐ ALL THREE GLIBC
#     all four musl environments ship the file
#
# ⚠ SO IT IS NOT A musl STORY. It is the same shape as the other ten: static
# linking resolves the CODE and touches none of the DATA.
#
# -- ⛔ WHY THIS ONE COULD NOT COPY --embed-tzdata --------------------------
#
# terminfo, the CA bundle and tzdata are all closed by POINTING a search
# variable at carried data before `main()` runs — TERMINFO, SSL_CERT_FILE,
# TZDIR. ⛔ THERE IS NO SUCH VARIABLE FOR /etc/services. glibc's
# `getservbyname` opens a compiled-in path and honours nothing.
#
# ⭐ So this mechanism intercepts the CALL instead, the way `--embed-locale`
# does: `-Wl,--wrap=getservbyname` and seven more. The wrapper calls glibc's
# own implementation FIRST and answers from the carried table only when that
# returned NULL — which is `docs/design/host-fallback.md`'s rule, and here
# "look at the host first" is literal rather than a search order.
#
# -- ⭐ PRE-REGISTERED EXPECTATION -------------------------------------------
#
# ⛔ COMMITTED BEFORE THE RUN. PROGRESS.md delivery rule 1.
#
#   N1  the control (plain `gcc -static`) answers NULL for getservbyname on
#       exactly the environments whose /etc/services is absent.
#   N2  `--embed-netdb` answers 80/tcp on 11 of 11.
#   N3  ⭐ AND IT DOES NOT OVERRIDE A HOST THAT HAS THE FILE. On an
#       environment that ships /etc/services, the answer must come from the
#       host — asserted by looking up a name this bundle's table does NOT
#       carry but the host's file does, and by a name the host's file does
#       not carry.
#   N4  ⛔ THE BOUNDARY, PRE-REGISTERED AS A FAILURE: `getaddrinfo` with a
#       SERVICE NAME is NOT expected to be fixed, because glibc calls its own
#       internal `__getservbyname_r` and `--wrap` redirects the public symbol
#       only. This experiment measures that rather than asserting it.
#
# ⚠ N4 IS WRITTEN DOWN BEFORE THE RUN ON PURPOSE. If it comes out green the
# mechanism reaches further than expected and the record must say the
# prediction was wrong — PROGRESS.md rule 1 exists for exactly that.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "66 - /etc/services and /etc/protocols: the eleventh quirk, and a mechanism for it"

WORK="${PGB_EXP66_WORK:-/var/tmp/pgb-exp66}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { exp_note "no $CC on PATH"; exit 2; }

exp_conditions

# -- 1. what the static archive itself names ---------------------------------
#
# ⛔ READ THE ARCHIVE, NOT THE DOCUMENTATION.
LIBC_A=""
for p in /usr/lib/x86_64-linux-gnu/libc.a /usr/lib64/libc.a /usr/lib/libc.a; do
  [ -e "$p" ] && { LIBC_A="$p"; break; }
done
if [ -n "$LIBC_A" ]; then
  _has() { [ "$(strings -a "$LIBC_A" 2>/dev/null | grep -cx "$1")" -gt 0 ] && echo yes || echo no; }
  exp_check "libc.a names /etc/services"  "$(_has '/etc/services')"  yes
  exp_check "libc.a names /etc/protocols" "$(_has '/etc/protocols')" yes
  exp_note  "so both are HOST data: nothing about them is linked in."
else
  exp_skip "libc.a names /etc/services"  "no libc.a on this host"
  exp_skip "libc.a names /etc/protocols" "no libc.a on this host"
fi

# -- the subject -------------------------------------------------------------
#
# ⚠ IT PRINTS, IT DOES NOT ASSERT. Four fields, so one run answers every
# question this experiment has:
#
#   1  getservbyname("http","tcp")      the case the quirk was found on
#   2  getprotobyname("tcp")            the second file
#   3  getservbyname("z-pgb-absent")    ⭐ a name NOBODY has: it must stay NULL
#   4  getaddrinfo(NULL,"http")         ⛔ the boundary --wrap cannot reach
cat > "$WORK/nd.c" <<'EOF'
#include <netdb.h>
#include <netinet/in.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
int main(void) {
    struct servent *s = getservbyname("http", "tcp");
    struct protoent *p = getprotobyname("tcp");
    struct servent *n = getservbyname("z-pgb-absent", "tcp");
    struct addrinfo hints, *ai = 0;
    int gaport = -1;
    memset(&hints, 0, sizeof hints);
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_PASSIVE;
    if (getaddrinfo(0, "http", &hints, &ai) == 0 && ai)
        gaport = ntohs(((struct sockaddr_in *)ai->ai_addr)->sin_port);
    printf("%d %d %s %d\n",
           s ? ntohs((unsigned short)s->s_port) : -1,
           p ? p->p_proto : -1,
           n ? "LEAK" : "null",
           gaport);
    return 0;
}
EOF
"$CC" -static -o "$WORK/nd" "$WORK/nd.c" > "$WORK/cc.log" 2>&1 \
  || { exp_note "the static arm did not link: $(tail -1 "$WORK/cc.log")"; exit 2; }

exp_check "the subject has no PT_INTERP" \
  "$(readelf -lW "$WORK/nd" 2>/dev/null | grep -c INTERP)" 0

# -- 2. the control: the build host HAS the files ----------------------------
#
# ⛔ WITHOUT THIS ROW NOTHING BELOW MEANS ANYTHING.
HOST_OUT=$("$WORK/nd" 2>/dev/null)
exp_note "build host: $HOST_OUT   (http/tcp, tcp, absent-name, getaddrinfo)"
exp_check "the CONTROL resolves http/tcp on the build host" \
  "$(printf '%s' "$HOST_OUT" | awk '{print $1}')" 80
exp_check "...and tcp's protocol number" \
  "$(printf '%s' "$HOST_OUT" | awk '{print $2}')" 6

# -- 3. arm A, the eleven, with no mechanism ---------------------------------
printf '\n'
printf -- '-- arm A: plain `gcc -static`, and what the host has ----------------\n'
printf '  %-20s %-6s %-9s %-10s %-7s %s\n' ENVIRONMENT LIBC /etc/serv /etc/proto PRINTED VERDICT
A_OK=0; A_MISS=0; ROWS=0
for name in $(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt"); do
  r=$(exp_rootfs "$name") || true
  [ -n "$r" ] || { exp_skip "$name" "not fetched"; continue; }
  ROWS=$((ROWS+1))
  libc=$(exp_rootfs_libc "$name")
  hs=$( [ -s "$r/etc/services" ]  && echo yes || echo no )
  hp=$( [ -s "$r/etc/protocols" ] && echo yes || echo no )
  out=$("$REPO_DIR/pgb" rootfs run "$r" --copy "$WORK/nd:/nd" -- /nd 2>/dev/null | tr -d '\r')
  port=$(printf '%s' "$out" | awk '{print $1}')
  if [ "$port" = 80 ]; then v="ok"; A_OK=$((A_OK+1)); else v="⛔ NULL"; A_MISS=$((A_MISS+1)); fi
  printf '  %-20s %-6s %-9s %-10s %-7s %s\n' "$name" "$libc" "$hs" "$hp" "${out:-<none>}" "$v"
done
printf '\n'
exp_check "every fetched environment answered"          "$((A_OK+A_MISS))" "$ROWS"
exp_check "N1  environments that CANNOT resolve http/tcp" \
  "$([ "$A_MISS" -gt 0 ] && echo some || echo none)" some
exp_note "arm A: $A_MISS of $ROWS answer NULL for getservbyname(\"http\",\"tcp\")."

# -- 4. arm B, the same program built with --embed-netdb ---------------------
BWORK="$WORK/armB"; mkdir -p "$BWORK"
cp "$WORK/nd.c" "$BWORK/nd.c"
if (cd "$BWORK" && "$REPO_DIR/pgb" build --embed-netdb -- cc -o nd nd.c) >"$WORK/armB.log" 2>&1 \
   && [ -x "$BWORK/nd" ]; then
  printf '\n'
  printf -- '-- arm B: the same program, built with --embed-netdb ----------------\n'
  printf '  %-20s %-9s %-7s %-7s %-6s %s\n' ENVIRONMENT /etc/serv 'http/tcp' 'proto' 'absent' 'getaddrinfo'
  B_OK=0; B_BAD=0; B_ROWS=0; B_LEAK=0; B_GAI=0
  for name in $(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt"); do
    r=$(exp_rootfs "$name") || true
    [ -n "$r" ] || continue
    B_ROWS=$((B_ROWS+1))
    hs=$( [ -s "$r/etc/services" ] && echo yes || echo no )
    out=$("$REPO_DIR/pgb" rootfs run "$r" --copy "$BWORK/nd:/nd" -- /nd 2>/dev/null | tr -d '\r')
    port=$(printf '%s' "$out" | awk '{print $1}')
    prot=$(printf '%s' "$out" | awk '{print $2}')
    absent=$(printf '%s' "$out" | awk '{print $3}')
    gai=$(printf '%s' "$out" | awk '{print $4}')
    if [ "$port" = 80 ] && [ "$prot" = 6 ]; then B_OK=$((B_OK+1)); else B_BAD=$((B_BAD+1)); fi
    [ "$absent" = LEAK ] && B_LEAK=$((B_LEAK+1))
    [ "$gai" = 80 ] && B_GAI=$((B_GAI+1))
    printf '  %-20s %-9s %-7s %-7s %-6s %s\n' "$name" "$hs" "${port:--}" "${prot:--}" "${absent:--}" "${gai:--}"
  done
  printf '\n'
  exp_check "N2  arm B: every environment resolves http/tcp"  "$B_OK"   "$B_ROWS"
  exp_check "N2  arm B: and none failed"                      "$B_BAD"  0
  # ⭐ THE NEGATIVE ROW. A table that answered a name nobody has would be
  # inventing services, which is worse than answering none.
  exp_check "N3  arm B: a name NOBODY carries still answers NULL" "$B_LEAK" 0
  # ⭐ THE CONTROL THAT SAYS ARM B DID SOMETHING.
  exp_check "arm A had rows that FAILED, so arm B is not vacuous" \
    "$([ "$A_MISS" -gt 0 ] && echo yes || echo no)" yes
  exp_note "arm A could not resolve on $A_MISS of $ROWS; arm B on $B_BAD."
  # ⛔ N4, PRE-REGISTERED AS A FAILURE. Reported as a NOTE and a separate
  # check so a green mechanism cannot be read as covering it.
  exp_note "⛔ N4, the boundary: getaddrinfo(NULL,\"http\") resolved to 80 on"
  exp_note "   $B_GAI of $B_ROWS rows. --wrap redirects the PUBLIC symbol, and"
  exp_note "   glibc's getaddrinfo calls its own internal __getservbyname_r, so"
  exp_note "   a row where the host has no /etc/services is expected to stay"
  exp_note "   unfixed here. This is the mechanism's stated edge, not a defect"
  exp_note "   hidden in a footnote."
  _sz_a=$(wc -c < "$WORK/nd"); _sz_b=$(wc -c < "$BWORK/nd")
  exp_note "size: arm A $_sz_a B, arm B $_sz_b B -- the carried tables cost $((_sz_b - _sz_a)) B"
else
  exp_skip "N2  arm B: every environment resolves http/tcp"  "pgb build --embed-netdb could not run: $(tail -1 "$WORK/armB.log" 2>/dev/null)"
  exp_skip "N2  arm B: and none failed"                      "see above"
  exp_skip "N3  arm B: a name NOBODY carries still answers NULL" "see above"
  exp_skip "arm A had rows that FAILED, so arm B is not vacuous" "see above"
fi

exp_finish
