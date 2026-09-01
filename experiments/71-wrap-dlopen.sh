#!/bin/sh
# 71-wrap-dlopen.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# Can a static binary load its OWN plugins on every environment, without a
# loader and without a second libc entering the process?
#
# experiments/50- established that dlopen of a shared object from a static
# glibc binary is host-dependent and that SUCCESS IS THE WORSE OUTCOME: where
# it works, the host's ld.so and libc.so.6 are now in the process. That is
# docs/limitations.md §1 and it is about HOST plugins.
#
# ⭐ This asks the other half of the question, which 50- did not separate out:
# a program loading plugins IT SHIPPED does not need a loader at all. The code
# is available at link time, so all dlopen is really providing is a name to
# function lookup. tool/runtime/pgb-dlopen.c answers that from a table `pgb`
# generates with `nm`, and this measures whether that holds up.
#
# -- ARMS -------------------------------------------------------------------
#
#   plain     pgb WITHOUT --wrap-dlopen. The control. It reaches the host
#             loader, so this is 50-'s result restated on this subject.
#   wrapped   pgb WITH --wrap-dlopen. The table answers.
#
# ⛔ BOTH ARMS RUN THE SAME SIX ASSERTIONS, and three of them are NEGATIVE.
# A wrapper that returned a non-NULL handle for everything would pass a naive
# "did dlopen work" test, so the subject also requires that a symbol the
# plugin did not export does NOT resolve, that a plugin not in the table does
# NOT open, and that dlerror() is set in both cases. docs/methodology/
# experiments.md: "an absence is not a zero ... distinguishable only by a
# positive control that the probe does find."
#
# ⚠ The plugin's code must actually RUN. Asserting that dlsym returned a
# non-NULL pointer proves a table lookup, not a working plugin, so the subject
# calls through the pointer and checks the value that comes back.
#
# Exit: 0 the measurement ran and matched, 1 it ran and did not, 2 could not run.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "71 - --wrap-dlopen: a static binary loading its own plugins"

WORK="$EXP_OUT/build"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
RESULT="$EXP_OUT/RESULT.txt"

# ---------------------------------------------------------------------------
# The plugin, and the program that loads it.
# ---------------------------------------------------------------------------
cat > "$WORK/plugin.c" <<'EOF'
/* The plugin: what a program would normally ship as a .so and dlopen. */
int plugin_answer(void) { return 42; }
const char *plugin_name(void) { return "demo-plugin"; }
/* ⛔ static: must NOT reach the generated table. `nm --extern-only` is what
 * keeps it out, and the subject asserts that it stayed out. */
static int not_exported(void) { return 7; }
int plugin_uses_static(void) { return not_exported(); }
EOF

cat > "$WORK/host.c" <<'EOF'
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>

/* ⛔ dlerror() CLEARS ON READ. Calling it twice in one expression -- once in
 * the test, once to print -- returns the message and then NULL, so the
 * failure prints "(null)" and reads as "no error was set" when one was. This
 * helper reads it exactly once. */
static const char *err_once(void) {
    const char *e = dlerror();
    return e ? e : "(no error was set)";
}

int main(void) {
    int fails = 0;
    void *h;
    int (*answer)(void);
    const char *(*nm)(void);

    h = dlopen("libdemo.so", RTLD_NOW);
    if (!h) { printf("  FAIL dlopen: %s\n", err_once()); return 1; }
    printf("  ok   dlopen returned a handle\n");

    answer = (int (*)(void))dlsym(h, "plugin_answer");
    nm     = (const char *(*)(void))dlsym(h, "plugin_name");
    if (!answer || !nm) { printf("  FAIL dlsym: %s\n", err_once()); return 1; }

    /* Call through, because a non-NULL pointer proves a lookup, not a plugin. */
    if (answer() != 42) { printf("  FAIL plugin_answer()=%d want 42\n", answer()); fails++; }
    else printf("  ok   plugin_answer() = 42   (the plugin's code ran)\n");

    if (strcmp(nm(), "demo-plugin") != 0) { printf("  FAIL plugin_name()=%s\n", nm()); fails++; }
    else printf("  ok   plugin_name() = demo-plugin\n");

    /* NEGATIVE: a file-local symbol must not be reachable. */
    if (dlsym(h, "not_exported") != NULL) {
        printf("  FAIL a static symbol leaked into the table\n"); fails++;
    } else {
        const char *e = err_once();
        if (strcmp(e, "(no error was set)") == 0) { printf("  FAIL dlsym(absent) set no error\n"); fails++; }
        else printf("  ok   dlsym(not_exported) refused, dlerror set\n");
    }

    /* NEGATIVE: a plugin that is not in the table must not open. */
    if (dlopen("libnope.so", RTLD_NOW) != NULL) {
        printf("  FAIL dlopen of an absent plugin succeeded\n"); fails++;
    } else {
        const char *e = err_once();
        if (strcmp(e, "(no error was set)") == 0) { printf("  FAIL dlopen(absent) set no error\n"); fails++; }
        else printf("  ok   dlopen(absent) refused, dlerror set\n");
    }

    if (dlclose(h) != 0) { printf("  FAIL dlclose\n"); fails++; }
    else printf("  ok   dlclose = 0\n");

    printf("%s: %d failure(s)\n", fails ? "FAILED" : "PASSED", fails);
    return fails ? 1 : 0;
}
EOF

printf -- '-- building --------------------------------------------------\n'
cc -O2 -c -o "$WORK/plugin.o" "$WORK/plugin.c" || exit 2
printf '  plugin.o exports: %s\n' \
  "$(nm --defined-only --extern-only "$WORK/plugin.o" | awk '{print $3}' | tr '\n' ' ')"

PGB="$REPO_DIR/pgb"
if ! sh "$PGB" --engine host build -- \
      sh -c "\$CC -O2 -o $WORK/host-plain $WORK/host.c" >"$WORK/plain.log" 2>&1; then
  exp_skip "build the plain arm" "see $WORK/plain.log"
fi
if ! sh "$PGB" --engine host --wrap-dlopen "libdemo.so=$WORK/plugin.o" build -- \
      sh -c "\$CC -O2 -o $WORK/host-wrap $WORK/host.c" >"$WORK/wrap.log" 2>&1; then
  exp_skip "build the wrapped arm" "see $WORK/wrap.log"
fi
[ -f "$WORK/host-plain" ] && printf '  built  plain    %s bytes\n' "$(wc -c < "$WORK/host-plain")"
[ -f "$WORK/host-wrap" ]  && printf '  built  wrapped  %s bytes\n' "$(wc -c < "$WORK/host-wrap")"
[ -f "$WORK/host-wrap" ] || { printf 'the wrapped arm did not build\n'; exit 2; }
printf '\n'

# ---------------------------------------------------------------------------
# Run both arms on every fetched environment, and record what each loaded.
# ---------------------------------------------------------------------------
{
  printf 'experiment 71 - --wrap-dlopen against a compiled-in table\n'
  printf 'date (UTC)   : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'host kernel  : %s\n' "$(uname -sr)"
  printf 'cc           : %s\n' "$({ ${CC:-cc} --version 2>/dev/null || echo none; } | head -1)"
  printf '\n'
  printf '%-22s %-6s %-10s %-10s %s\n' TARGET LIBC PLAIN WRAPPED 'HOST .so LOADED BY WRAPPED'
} > "$RESULT"

printf -- '-- running ---------------------------------------------------\n'
printf '%-22s %-6s %-10s %-10s %s\n' TARGET LIBC PLAIN WRAPPED 'HOST .so LOADED BY WRAPPED'

n_targets=0; n_wrap_ok=0; n_plain_ok=0; n_wrap_clean=0
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  root=$(exp_rootfs "$name")
  [ -n "$root" ] || continue
  n_targets=$((n_targets+1))

  code() { # rootfs binary -> cell
    _r="$1"; _b="$2"; _base=$(basename "$_b")
    cp "$_b" "$_r/$_base" 2>/dev/null || { printf 'copy-failed'; return; }
    sh "$REPO_DIR/scripts/common/rootfs-run.sh" "$_r" -- "/$_base" >/dev/null 2>&1
    _st=$?
    rm -f "$_r/$_base"
    case $_st in
      0) printf 'ok' ;;
      13[0-9]|1[4-6][0-9]) printf 'SIG%s' "$((_st-128))" ;;
      *) printf 'exit%s' "$_st" ;;
    esac
  }

  cp="$( [ -f "$WORK/host-plain" ] && code "$root" "$WORK/host-plain" || printf 'not-built' )"
  cw="$(code "$root" "$WORK/host-wrap")"

  # ⛔ Criterion 2 of docs/AGENTS.md §3: what did the WRAPPED binary load?
  # A mechanism that made dlopen work by quietly reaching the host loader
  # would show up here and nowhere else.
  cp "$WORK/host-wrap" "$root/host-wrap" 2>/dev/null
  libs=$(exp_trace_libs "$root" "/host-wrap" "$WORK/t.$name" | grep -E '\.so(\.[0-9]+)*$' | tr '\n' ' ')
  rm -f "$root/host-wrap"

  [ "$cp" = ok ] && n_plain_ok=$((n_plain_ok+1))
  [ "$cw" = ok ] && n_wrap_ok=$((n_wrap_ok+1))
  [ -z "$libs" ] && n_wrap_clean=$((n_wrap_clean+1))

  row=$(printf '%-22s %-6s %-10s %-10s %s' "$name" "$libc" "$cp" "$cw" "${libs:-none}")
  printf '%s\n' "$row"
  printf '%s\n' "$row" >> "$RESULT"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

[ "$n_targets" -gt 0 ] || { printf 'no rootfs fetched\n'; exit 2; }
printf '\n'

printf -- '-- assertions ------------------------------------------------\n'
exp_check "wrapped: all six assertions pass, every environment" "$n_wrap_ok" "$n_targets"
exp_check "wrapped: loaded no host shared object, every environment" "$n_wrap_clean" "$n_targets"
printf '  --    %-46s = %s of %s\n' "plain arm ran (observed, not asserted)" "$n_plain_ok" "$n_targets"
printf '\n'
exp_note "The plain arm is the control and is EXPECTED to fail: it reaches the"
exp_note "host loader, which is experiments/50-'s result. Its count is recorded,"
exp_note "never asserted, because what it does is host-dependent by nature."
printf '\n'
printf 'full table: %s\n' "$RESULT"

exp_finish
