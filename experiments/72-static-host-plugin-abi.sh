#!/bin/sh
# 72-static-host-plugin-abi.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# Can a STATIC program host a SHARED plugin that calls back into it?
#
# ⭐ This is not the same question as experiments/50-, and confusing the two
# has cost this project a wrong plan. 50- asked whether a static binary can
# `dlopen` a host object at all, and found the failure is in glibc's loader.
# This asks something prior and much simpler: even where the loader works,
# can the plugin's references to the HOST PROGRAM'S OWN SYMBOLS be resolved?
#
# -- WHY IT WAS ASKED -------------------------------------------------------
#
# T-030's acceptance was written as "POC 50's CPython rebuilt with
# --wrap-dlopen instead of hand-written Modules/Setup.local". Building the
# subject that acceptance needs -- a static interpreter with SHARED stdlib
# modules -- fails, and not in the plugin-loading code:
#
#   ImportError: build/lib.linux-x86_64-3.12/math.cpython-312-x86_64-linux-gnu.so:
#                undefined symbol: PyLong_AsLongLongAndOverflow
#   make: *** [Makefile:1125: checksharedmods] Error 1
#
# Measured on that build: the interpreter is `statically linked`, PT_INTERP 0,
# DT_NEEDED 0, and ⛔ **.dynsym entries: 0**. `math.so` has 27 undefined
# `Py*` symbols. The interpreter DOES define the missing one -- `nm ./python`
# finds it -- but only in its static symbol table, which a dynamic loader
# never consults.
#
# -- WHAT THIS SCRIPT IS ----------------------------------------------------
#
# The same finding in a subject that builds in a second instead of half an
# hour, so it is a check this project keeps rather than a story about one
# afternoon. Three arms, one axis:
#
#   dynamic-host   an ordinary dynamic host, -rdynamic. The POSITIVE CONTROL:
#                  the plugin must load and call back, or the experiment is
#                  measuring its own subject being broken.
#   static-host    the same host and plugin, host linked -static.
#   static+wrap    the same host, plugin LINKED IN and reached through
#                  pgb --wrap-dlopen.
#
# ⛔ THE POSITIVE CONTROL IS THE POINT. Without it, "the static host could not
# load the plugin" is indistinguishable from "the plugin was broken all
# along" -- docs/methodology/experiments.md, "an absence is not a zero".
#
# Exit: 0 the measurement ran and matched, 1 it ran and did not, 2 could not run.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "72 - can a static program host a shared plugin that calls back?"

WORK="$EXP_OUT/build"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
RESULT="$EXP_OUT/RESULT.txt"

# ---------------------------------------------------------------------------
# The host exports a function. The plugin CALLS IT -- which is the whole
# point: a plugin that only receives data would not need the host's symbols
# and would not exercise the thing being measured.
# ---------------------------------------------------------------------------
cat > "$WORK/host.c" <<'EOF'
#include <dlfcn.h>
#include <stdio.h>

/* The host's own API, exactly the shape CPython's Py* functions have: the
 * plugin is compiled against a declaration of this and resolves it at LOAD
 * time from the host executable. */
int host_api_add(int a, int b) { return a + b; }

int main(int argc, char **argv) {
    const char *path = argc > 1 ? argv[1] : "libplug.so";
    void *h = dlopen(path, RTLD_NOW);
    int (*run)(void);
    const char *e;
    if (!h) {
        e = dlerror();
        printf("FAIL dlopen: %s\n", e ? e : "(no error set)");
        return 1;
    }
    run = (int (*)(void))dlsym(h, "plugin_run");
    if (!run) {
        e = dlerror();
        printf("FAIL dlsym: %s\n", e ? e : "(no error set)");
        return 1;
    }
    /* ⛔ CALL THROUGH. A non-NULL pointer proves a lookup, not a plugin that
     * can run -- and the failure being measured happens at LOAD time, so a
     * probe that stopped at dlsym could still be fooled. */
    if (run() != 42) { printf("FAIL plugin_run() = %d, want 42\n", run()); return 1; }
    printf("PASSED plugin_run() = 42, and it called back into the host\n");
    return 0;
}
EOF

cat > "$WORK/plug.c" <<'EOF'
/* Defined by the HOST, not here. This is the reference that has to be
 * resolved against the host executable when the plugin is loaded. */
extern int host_api_add(int a, int b);
int plugin_run(void) { return host_api_add(40, 2); }
EOF

printf -- '-- building --------------------------------------------------\n'
cc -O2 -fPIC -shared -o "$WORK/libplug.so" "$WORK/plug.c" || exit 2
printf '  plugin undefined host symbols: %s\n' \
  "$(nm -u "$WORK/libplug.so" | grep -c host_api_add)"

# Arm 1: the positive control.
cc -O2 -rdynamic -o "$WORK/host-dynamic" "$WORK/host.c" -ldl 2>/dev/null || exit 2
# Arm 2: the same thing, static.
cc -O2 -static  -o "$WORK/host-static"  "$WORK/host.c" 2>/dev/null || exit 2

printf '  host-dynamic .dynsym entries: %s\n' \
  "$(readelf --dyn-syms "$WORK/host-dynamic" 2>/dev/null | grep -c ' FUNC \| OBJECT ')"
printf '  host-static  .dynsym entries: %s\n' \
  "$(readelf --dyn-syms "$WORK/host-static" 2>/dev/null | grep -c ' FUNC \| OBJECT ')"

# Arm 3: static, plugin linked in, reached through the compiled-in table.
cc -O2 -c -o "$WORK/plug-static.o" "$WORK/plug.c" || exit 2
if "$REPO_DIR/pgb" --engine host --wrap-dlopen "libplug.so=$WORK/plug-static.o" \
     build -- sh -c "\$CC -O2 -o $WORK/host-wrap $WORK/host.c" >"$WORK/wrap.log" 2>&1 \
   && [ -x "$WORK/host-wrap" ]; then
  printf '  host-wrap built\n'
else
  exp_skip "build the static+wrap arm" "see $WORK/wrap.log"
fi
printf '\n'

# ---------------------------------------------------------------------------
run_arm() { # binary -> prints the one-line outcome
  ( cd "$WORK" && ./"$1" ./libplug.so 2>&1 | tail -1 )
}

printf -- '-- running ---------------------------------------------------\n'
d_out=$(run_arm host-dynamic); d_rc=$?
s_out=$(run_arm host-static);  s_rc=$?
if [ -x "$WORK/host-wrap" ]; then w_out=$(run_arm host-wrap); else w_out="(not built)"; fi

{
  printf 'experiment 72 - can a static program host a shared plugin that calls back?\n'
  printf 'date (UTC)   : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'host kernel  : %s\n' "$(uname -sr)"
  printf 'cc           : %s\n' "$({ ${CC:-cc} --version 2>/dev/null || echo none; } | head -1)"
  printf '\n'
  printf '%-16s %-8s %s\n' ARM DYNSYM OUTCOME
  printf '%-16s %-8s %s\n' dynamic-host \
    "$(readelf --dyn-syms "$WORK/host-dynamic" 2>/dev/null | grep -c ' FUNC \| OBJECT ')" "$d_out"
  printf '%-16s %-8s %s\n' static-host \
    "$(readelf --dyn-syms "$WORK/host-static" 2>/dev/null | grep -c ' FUNC \| OBJECT ')" "$s_out"
  printf '%-16s %-8s %s\n' static+wrap \
    "$(readelf --dyn-syms "$WORK/host-wrap" 2>/dev/null | grep -c ' FUNC \| OBJECT ')" "$w_out"
} > "$RESULT"
cat "$RESULT" | tail -5
printf '\n'

printf -- '-- assertions ------------------------------------------------\n'
case "$d_out" in
  PASSED*) exp_check "POSITIVE CONTROL: dynamic host loads it and it calls back" ok ok ;;
  *)       exp_check "POSITIVE CONTROL: dynamic host loads it and it calls back" "$d_out" ok ;;
esac
case "$s_out" in
  FAIL*)   exp_check "static host CANNOT resolve the callback" refused refused ;;
  *)       exp_check "static host CANNOT resolve the callback" "$s_out" refused ;;
esac
if [ -x "$WORK/host-wrap" ]; then
  case "$w_out" in
    PASSED*) exp_check "static + --wrap-dlopen: it works" ok ok ;;
    *)       exp_check "static + --wrap-dlopen: it works" "$w_out" ok ;;
  esac
fi
printf '\n'
exp_note "⛔ THE FINDING, and it is structural rather than a defect:"
exp_note "a statically linked executable has an EMPTY dynamic symbol table, so a"
exp_note "shared object loaded into it can never resolve a reference back to the"
exp_note "host program. -rdynamic exports through .dynsym and there is no .dynsym"
exp_note "to export through. This is prior to experiments/50-'s loader failures:"
exp_note "even a loader that worked perfectly would have nowhere to look."
exp_note ""
exp_note "⭐ So for a plugin that calls back into its host, linking it in is not a"
exp_note "workaround for dlopen -- it is the only thing that can work, and"
exp_note "--wrap-dlopen is what makes the program's existing dlopen calls find it."
printf '\n'
printf 'full table: %s\n' "$RESULT"

exp_finish
