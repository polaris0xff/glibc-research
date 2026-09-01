#!/bin/sh
# 75-terminfo.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# The last of the five host DATA dependencies in docs/limitations.md §3 that
# had no mechanism. ncurses reaches terminal descriptions by PATH, and a
# setupterm() probe linked against the same static ncursesw, measured in
# poc/20-nano:
#
#     OK                                     all 7 glibc
#     rc=-1 err=-1  no database at all       Alpine 3.10, 3.20, 3.22
#     rc=-1 err=0   database present, no
#                   xterm-256color entry     Void musl
#
# ⚠ THE TWO FAILURE MODES ARE DIFFERENT AND THE SECOND IS THE ONE A NAIVE
# MECHANISM MISSES. "No database" is obvious. "A database that does not
# describe THIS terminal" looks like success to anything that only checks
# whether the directory exists.
#
# ⛔ AND WHETHER A glibc PORTABILITY TOOL SHOULD OWN A TERMINAL DATABASE IS A
# REAL QUESTION. limitations.md §3 says the argument is weak and it is:
# terminfo is ncurses' data, not libc's. So `--embed-terminfo` is OPT-IN, it
# carries a HANDFUL of descriptions rather than a database, and ncurses still
# consults its own compiled-in path afterwards, so an entry this binary does
# not carry is still reachable.
#
# -- ARMS -------------------------------------------------------------------
#
#   control   pgb WITHOUT --embed-terminfo: what a program sees today.
#   embedded  pgb WITH it.
#
# ⛔ THE PROBE DOES NOT LINK ncurses. It reproduces ncurses' own search --
# $TERMINFO, then the compiled-in roots, <dir>/<first letter>/<name> -- and
# reports whether a description for $TERM is reachable. That keeps this
# experiment independent of which ncurses happens to be around, and poc/20's
# real setupterm() is the other half of the acceptance.
#
# Exit: 0 the measurement ran and matched, 1 it ran and did not, 2 could not run.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "75 - --embed-terminfo: a terminal description on every host"

WORK="$EXP_OUT/build"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
RESULT="$EXP_OUT/RESULT.txt"

cat > "$WORK/probe.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

/* ncurses' own order: $TERMINFO first, then the compiled-in roots. */
static const char *const roots[] = {
    "/usr/share/terminfo", "/lib/terminfo", "/usr/lib/terminfo",
    "/etc/terminfo", "/usr/share/lib/terminfo", NULL
};

static int entry_at(const char *root, const char *term, char *out, size_t n)
{
    struct stat st;
    if (!root || !*root || !term || !*term) return 0;
    if (snprintf(out, n, "%s/%c/%s", root, term[0], term) >= (int)n) return 0;
    return stat(out, &st) == 0 && S_ISREG(st.st_mode) && st.st_size > 0;
}

int main(void)
{
    const char *term = getenv("TERM");
    const char *ti = getenv("TERMINFO");
    char path[512];
    int i, host = 0, via_env = 0;

    if (!term) term = "";
    if (ti && entry_at(ti, term, path, sizeof path)) via_env = 1;
    for (i = 0; roots[i]; i++)
        if (entry_at(roots[i], term, path, sizeof path)) { host = 1; break; }

    printf("TERM=%s|TERMINFO=%s|host_has=%d|env_has=%d|reachable=%d\n",
           term, ti ? ti : "<unset>", host, via_env, (host || via_env));
    return 0;
}
EOF

printf -- '-- building --------------------------------------------------\n'
PGB="$REPO_DIR/pgb"
if ! sh "$PGB" --bind "$WORK" build -- /bin/sh -c \
      "\$CC -O2 -o $WORK/probe-plain $WORK/probe.c" >"$WORK/plain.log" 2>&1; then
  exp_skip "build the control arm" "see $WORK/plain.log"; exp_finish
fi
if ! sh "$PGB" --bind "$WORK" --embed-terminfo build -- /bin/sh -c \
      "\$CC -O2 -o $WORK/probe-ti $WORK/probe.c" >"$WORK/ti.log" 2>&1; then
  exp_skip "build the --embed-terminfo arm" "see $WORK/ti.log"; exp_finish
fi
sz_p=$(wc -c < "$WORK/probe-plain"); sz_t=$(wc -c < "$WORK/probe-ti")
exp_note "control  $sz_p bytes"
exp_note "embedded $sz_t bytes  (+$((sz_t - sz_p)), the descriptions)"
exp_check "the anchor forced the constructor in" \
  "$(nm "$WORK/probe-ti" 2>/dev/null | grep -c pgb_terminfo_anchor)" "1"
exp_check "the control has no terminfo code" \
  "$(nm "$WORK/probe-plain" 2>/dev/null | grep -c pgb_terminfo_anchor)" "0"
printf '\n'

printf -- '-- running, TERM=xterm-256color -------------------------------\n'
printf '  %-20s %-6s %-9s %-9s %s\n' TARGET LIBC CONTROL EMBEDDED 'TERMINFO SET TO'
field() { printf '%s' "$1" | tr '|' '\n' | sed -n "s/^$2=//p"; }

: > "$WORK/rows.txt"
NROWS=0; C_OK=0; E_OK=0; NOOVERRIDE=0
while read -r image name libc rest; do
  case "$image" in ''|\#*) continue;; esac
  [ -n "$name" ] || continue
  r=$(exp_rootfs "$name") || true
  [ -n "$r" ] || { exp_skip "$name" "rootfs absent"; continue; }

  oc=$(sh "$REPO_DIR/scripts/common/rootfs-run.sh" "$r" --copy "$WORK/probe-plain:/probe" \
        -- /bin/sh -c 'unset TERMINFO TERMINFO_DIRS; TERM=xterm-256color /probe' 2>/dev/null | tail -1)
  oe=$(sh "$REPO_DIR/scripts/common/rootfs-run.sh" "$r" --copy "$WORK/probe-ti:/probe" \
        -- /bin/sh -c 'unset TERMINFO TERMINFO_DIRS; TERM=xterm-256color /probe' 2>/dev/null | tail -1)
  # ⛔ The caller's own TERMINFO must survive, even pointing nowhere useful.
  oo=$(sh "$REPO_DIR/scripts/common/rootfs-run.sh" "$r" --copy "$WORK/probe-ti:/probe" \
        -- /bin/sh -c 'TERMINFO=/operator/choice TERM=xterm-256color /probe' 2>/dev/null | tail -1)

  c=$(field "$oc" reachable); c=${c:-0}
  e=$(field "$oe" reachable); e=${e:-0}
  ti=$(field "$oe" TERMINFO)
  oti=$(field "$oo" TERMINFO)

  printf '  %-20s %-6s %-9s %-9s %s\n' "$name" "$libc" \
    "$([ "$c" = 1 ] && echo found || echo NONE)" \
    "$([ "$e" = 1 ] && echo found || echo NONE)" "$ti"
  printf '%s %s %s %s %s\n' "$name" "$libc" \
    "$([ "$c" = 1 ] && echo found || echo NONE)" \
    "$([ "$e" = 1 ] && echo found || echo NONE)" "$ti" >> "$WORK/rows.txt"

  NROWS=$((NROWS+1))
  [ "$c" = 1 ] && C_OK=$((C_OK+1))
  [ "$e" = 1 ] && E_OK=$((E_OK+1))
  [ "$oti" = "/operator/choice" ] && NOOVERRIDE=$((NOOVERRIDE+1))
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

printf '\n'
printf -- '-- assertions ------------------------------------------------\n'
[ "$NROWS" -gt 0 ] || { exp_skip "every assertion below" "no environment was reachable"; exp_finish; }
exp_check "a description for TERM reachable, every environment" "$E_OK" "$NROWS"
exp_check "never overrode a TERMINFO the caller set" "$NOOVERRIDE" "$NROWS"
exp_note "control (the host's own database alone) = $C_OK of $NROWS"

{
  printf 'experiment 75 - --embed-terminfo\n\n'
  printf 'control %s bytes, embedded %s bytes (+%s)\n' "$sz_p" "$sz_t" "$((sz_t - sz_p))"
  printf 'TERM=xterm-256color, TERMINFO and TERMINFO_DIRS unset\n\n'
  printf '%-20s %-6s %-9s %-9s %s\n' TARGET LIBC CONTROL EMBEDDED 'TERMINFO SET TO'
  cat "$WORK/rows.txt"
  printf '\nreachable with the mechanism   %s of %s\n' "$E_OK" "$NROWS"
  printf 'reachable without it           %s of %s\n' "$C_OK" "$NROWS"
  printf 'never overrode the caller      %s of %s\n' "$NOOVERRIDE" "$NROWS"
} > "$RESULT"

exp_note "written: $RESULT"
exp_finish
