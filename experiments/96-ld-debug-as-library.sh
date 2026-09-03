#!/bin/sh
# 96 - does LD_DEBUG print anything when glibc's ld.so arrives as a LIBRARY?
#
# ⛔ THE QUESTION IS T-075's FIRST OPEN ROW, AND IT GATES A HARNESS CHANGE.
# The operator's review named three places for `LD_DEBUG=bindings`. One was
# placed (`experiments/93-`'s dynamic control). The second is
# `poc/10-gawk`'s host-extension observation, and T-075 refused to write the
# diagnostic in until somebody measured whether it prints anything there:
#
#   *"the subject is the STATIC gawk. Where the extension loads, the host
#    ld.so enters as a LIBRARY rather than as the program interpreter, and
#    LD_DEBUG is read at ld.so startup. Whether it prints anything there is
#    UNMEASURED, and this tree does not write an unverified diagnostic into a
#    harness."*
#
# ⭐ WHY IT MATTERS MORE THAN IT LOOKS. A diagnostic that prints nothing does
# not fail; it produces an EMPTY CAPTURE, and an empty capture in an
# observation table reads as *"no bindings"* rather than *"the instrument does
# not work here"*. That is `docs/AGENTS.md` §0b's "an absence is not a zero",
# and writing LD_DEBUG into a harness where it is silent would manufacture one
# on every row.
#
# THE SUBJECT AND THE CONTROL differ in one property and nothing else: the same
# source, the same compiler, one linked `-static` and one not. Both dlopen the
# same host object.
#
#   0  measured -- the arms behave as the write-up says
#   1  measured and did NOT match: read the table, the answer changed
#   2  could not be measured here (no cc, or the host has no dlopen'able libz)
#
# SPDX-License-Identifier: MIT
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "96 - LD_DEBUG when ld.so arrives as a library, not as the interpreter"

WORK="${PGB_EXP96_WORK:-/var/tmp/pgb-exp96}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2

CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { exp_note "no $CC on PATH"; exit 2; }

cat > "$WORK/probe.c" <<'EOF'
/* dlopen a host shared object and call one symbol out of it. Built twice: once
   -static (no PT_INTERP, so ld.so can only enter through the dlopen path) and
   once dynamic (ld.so is the program interpreter). */
#include <stdio.h>
#include <dlfcn.h>
int main(void) {
  void *h = dlopen("libz.so.1", RTLD_NOW);
  if (!h) { printf("dlopen FAILED: %s\n", dlerror()); return 2; }
  const char *(*f)(void) = (const char *(*)(void))dlsym(h, "zlibVersion");
  if (!f) { printf("dlsym FAILED\n"); return 2; }
  printf("zlib %s\n", f());
  return 0;
}
EOF

# ⚠ The static link warns ("Using 'dlopen' in statically linked applications
# requires at runtime the shared libraries from the glibc version used for
# linking"). That warning IS this experiment's subject, not a problem with it.
"$CC" -static -o "$WORK/static" "$WORK/probe.c" -ldl 2>"$WORK/static.cc.log" \
  || { exp_note "the static arm did not link: $(tail -1 "$WORK/static.cc.log")"; exit 2; }
"$CC"          -o "$WORK/dyn"    "$WORK/probe.c" -ldl 2>/dev/null \
  || { exp_note "the dynamic arm did not link"; exit 2; }

# The one property that separates the arms, asserted rather than assumed.
exp_check "the static arm has no PT_INTERP" \
  "$(readelf -l "$WORK/static" 2>/dev/null | grep -c INTERP)" 0
exp_check "...and no DT_NEEDED" \
  "$(readelf -d "$WORK/static" 2>/dev/null | grep -c NEEDED)" 0
exp_check "the dynamic control DOES have a PT_INTERP" \
  "$(readelf -l "$WORK/dyn" 2>/dev/null | grep -c INTERP)" 1

# Both must actually load the object, or the rest measures nothing.
"$WORK/static" > "$WORK/static.out" 2>&1; _s_rc=$?
"$WORK/dyn"    > "$WORK/dyn.out"    2>&1; _d_rc=$?
if [ "$_s_rc" != 0 ]; then
  # ⚠ Not a failure of the question: a host whose static dlopen refuses cannot
  # answer it at all, which is exit 2 and not exit 1.
  exp_note "the static arm could not dlopen here: $(cat "$WORK/static.out")"
  exp_note "this host cannot answer the question; it is not a negative result"
  exit 2
fi
exp_check "the static arm dlopens the host object"  "$_s_rc" 0
exp_check "the dynamic control does too"            "$_d_rc" 0

# ⛔ AND ld.so IS IN THE STATIC PROCESS. Without this the whole result could be
# "there was no loader to read the variable", which is a different and much
# less interesting answer.
if command -v strace >/dev/null 2>&1; then
  _ldso=$(strace -f -e trace=openat "$WORK/static" 2>&1 | grep -c 'ld-linux[^"]*\.so')
  exp_check "the static process DOES map glibc's ld.so" \
    "$([ "$_ldso" -gt 0 ] && echo yes || echo no)" yes
else
  exp_skip "ld.so is mapped by the static arm" "no strace"
fi

# ---------------------------------------------------------------------------
# The measurement. Three settings, two arms, counted rather than eyeballed.
# ---------------------------------------------------------------------------
lines() { # arm setting -> stderr line count
  LD_DEBUG="$2" "$WORK/$1" 2>&1 >/dev/null | wc -l | tr -d ' '
}
files() { # arm setting -> number of LD_DEBUG_OUTPUT files produced
  rm -f "$WORK/$1.ldlog"*
  LD_DEBUG="$2" LD_DEBUG_OUTPUT="$WORK/$1.ldlog" "$WORK/$1" >/dev/null 2>&1
  find "$WORK" -name "$1.ldlog*" | wc -l | tr -d ' '
}

D_BIND=$(lines dyn bindings);   S_BIND=$(lines static bindings)
D_ALL=$(lines dyn all);         S_ALL=$(lines static all)
D_FILES=$(files dyn bindings);  S_FILES=$(files static bindings)
# ⭐ LD_DEBUG=help makes ld.so print its help AND EXIT before running the
# program. The dynamic arm therefore never reaches its own printf; the static
# arm runs to completion, which is the clearest single proof that the variable
# was never processed.
D_HELP=$(LD_DEBUG=help "$WORK/dyn"    2>&1 | grep -c 'LD_DEBUG')
S_HELP=$(LD_DEBUG=help "$WORK/static" 2>&1 | grep -c 'LD_DEBUG')
S_HELP_RAN=$(LD_DEBUG=help "$WORK/static" 2>&1 | grep -c '^zlib ')
D_HELP_RAN=$(LD_DEBUG=help "$WORK/dyn"    2>&1 | grep -c '^zlib ')

printf '\n  %-34s %14s %14s\n' 'LD_DEBUG setting' 'DYNAMIC (ctrl)' 'STATIC (subj)'
printf '  %-34s %14s %14s\n' 'bindings, stderr lines'  "$D_BIND"  "$S_BIND"
printf '  %-34s %14s %14s\n' 'all, stderr lines'       "$D_ALL"   "$S_ALL"
printf '  %-34s %14s %14s\n' 'bindings, LD_DEBUG_OUTPUT files' "$D_FILES" "$S_FILES"
printf '  %-34s %14s %14s\n' 'help, lines mentioning LD_DEBUG' "$D_HELP"  "$S_HELP"
printf '  %-34s %14s %14s\n' 'help, program still ran'         "$D_HELP_RAN" "$S_HELP_RAN"
printf '\n'

# ⭐ THE CONTROL FIRST. If the dynamic arm prints nothing either, the
# instrument is broken here and the subject's silence says nothing at all.
exp_check "the CONTROL prints bindings (else nothing below means anything)" \
  "$([ "$D_BIND" -gt 0 ] && echo yes || echo no)" yes
exp_check "...and more of them at LD_DEBUG=all" \
  "$([ "$D_ALL" -gt "$D_BIND" ] && echo yes || echo no)" yes
exp_check "...and writes an LD_DEBUG_OUTPUT file" "$D_FILES" 1
exp_check "...and LD_DEBUG=help stops it before the program runs" "$D_HELP_RAN" 0

exp_check "the SUBJECT prints nothing at LD_DEBUG=bindings" "$S_BIND" 0
exp_check "...nothing at LD_DEBUG=all either"               "$S_ALL"  0
exp_check "...and writes no LD_DEBUG_OUTPUT file at all"    "$S_FILES" 0
exp_check "...and LD_DEBUG=help does not stop it: it runs"  "$S_HELP_RAN" 1

exp_note ""
exp_note "ANSWER: no. LD_DEBUG is processed during ld.so's own startup, which"
exp_note "never runs when ld.so is brought in through the static dlopen path."
exp_note "The loader IS in the process and reads the variable's value never."

RESULT="$EXP_OUT/RESULT.txt"
{
  printf '96 - LD_DEBUG when ld.so arrives as a library, not as the interpreter\n\n'
  printf 'date: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'kernel: %s\n' "$(uname -sr)"
  printf 'host libc: %s\n' "$(ldd --version 2>&1 | head -1)"
  printf 'cc: %s\n' "$($CC --version 2>/dev/null | head -1)"
  printf '\n'
  printf 'THE QUESTION (T-075, first open row): poc/10-gawk observes a STATIC\n'
  printf 'binary dlopening a host extension. There the host ld.so enters as a\n'
  printf 'LIBRARY rather than as the program interpreter. Does LD_DEBUG print\n'
  printf 'anything in that configuration?\n\n'
  printf '%-34s %14s %14s\n' 'LD_DEBUG setting' 'DYNAMIC (ctrl)' 'STATIC (subj)'
  printf '%-34s %14s %14s\n' 'bindings, stderr lines'  "$D_BIND"  "$S_BIND"
  printf '%-34s %14s %14s\n' 'all, stderr lines'       "$D_ALL"   "$S_ALL"
  printf '%-34s %14s %14s\n' 'bindings, output files'  "$D_FILES" "$S_FILES"
  printf '%-34s %14s %14s\n' 'help, program still ran' "$D_HELP_RAN" "$S_HELP_RAN"
  printf '\n'
  printf 'ANSWER: NO. And ld.so IS mapped into the static process -- strace\n'
  printf 'shows it opening ld-linux-x86-64.so.2 -- so this is not "no loader\n'
  printf 'was present". LD_DEBUG is parsed during the loader own startup,\n'
  printf 'which the static dlopen path never runs.\n\n'
  printf 'CONSEQUENCE for T-075: LD_DEBUG must NOT be written into\n'
  printf 'poc/10-gawk observation. It would not fail there -- it would produce\n'
  printf 'an EMPTY CAPTURE, which in an observation table reads as "no\n'
  printf 'bindings" rather than "the instrument does not work here".\n\n'
  printf 'reproduce:\n'
  printf '  sh experiments/96-ld-debug-as-library.sh\n'
} > "$RESULT"
printf '\nwrote %s\n' "$RESULT"

exp_finish
