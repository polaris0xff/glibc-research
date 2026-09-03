#!/bin/sh
# 94 - the loader's own-symbol table, and the two directions it has to serve.
#
# -- THE QUESTION -----------------------------------------------------------
#
# `tool/runtime/pgb-elfload.c` answers a handful of names out of a table of
# its own, because glibc's `ld.so` defines them and `libc.a` does not. ⛔ The
# two entries in that table have OPPOSITE requirements:
#
#   __tls_get_addr             MUST WIN over every loaded object. The module
#                              ids it is handed were minted by THIS loader's
#                              R_X86_64_DTPMOD64 case; nothing else in the
#                              process can interpret them.
#
#   _dl_mcount_wrapper_check   MUST YIELD to any real definition. It is a
#                              stand-in whose entire content is `do nothing`,
#                              so answering when a real one is in scope
#                              SHADOWS it with a no-op.
#
# Until T-073 both lived in ONE table, checked first and unconditionally inside
# `el_provider()`. That was safe for one of the two names and wrong for the
# other, and it was safe for that one BY ACCIDENT: `el_resolve()` happens to
# call `el_provider()` last, and nothing anywhere asserted that ordering or
# recorded that either entry depended on it.
#
# ⭐ WHAT THIS FILE IS FOR: the ordering is now two tables consulted at two
# points, and this is what says so. Both directions, both names, plus the
# negative control that reproduces the single-table shape.
#
# -- THE DEFECT CLASS -------------------------------------------------------
#
# ⚠ A lookup that ANSWERS when it should DEFER. This tree has now paid for it
# four times: the weak `iconv_open` provider entry that held NULL and shadowed
# the real one; R_X86_64_DTPOFF64 answering offset 0 instead of deferring to
# the defining module (both TODO T-068); and upstream
# `pkgforge-dev/cross-libc-dlopen#28`, where a forwarding shim with no target
# kept 358 zero-returning stubs that shadowed a provider the process could
# still serve. ⛔ WE ARE NOT AFFECTED BY THAT BUG -- it is an LD_PRELOAD
# interposition defect, this tool ships no preload shim and its output has no
# PT_INTERP -- but the SHAPE is ours and this is the live instance of it.
#
# -- ⛔ WHY THE OBSERVABLE IS THE CALL COUNT AND NOT THE VALUE ---------------
#
# ⭐ THE ROUND TRIP PASSES UNDER THE DEFECT. Arm C stores 0x5eeded through a
# general-dynamic thread-local and reads it back. Under the reversal the decoy
# `__tls_get_addr` answers BOTH accesses and hands back its own slab both
# times, so the value still returns 0x5eeded and every naive assertion is
# green. What separates them is that the decoy was CALLED: `decoy_calls=0`
# fixed, `decoy_calls=2` reversed. An experiment written around the value would
# have measured nothing and said "pass".
#
# -- THE ARMS ---------------------------------------------------------------
#
#   A yield    a loaded object DOES define _dl_mcount_wrapper_check; it must
#              win, and the loader's no-op must stand aside
#   B standin  NOTHING defines it; the loader's own entry must answer, or the
#              247 host objects that reference it go back to being refused
#   C win      a loaded object defines __tls_get_addr; OURS must win anyway
#   D own      nothing defines it; ours must answer
#
# Arms A and B are the two halves of "yield", C and D of "win". ⚠ B and D are
# in here because a table that answers nothing would pass A and C: the fix
# must not be "stop answering".
#
# -- WHAT THIS CANNOT SETTLE ------------------------------------------------
#
# ⚠ It measures RESOLUTION ORDER inside our own loader on names we own. It says
# nothing about which definition ld.so would pick for the same set -- there is
# no ld.so in the subject to ask. The eleven-row matrix says the order does not
# vary with the target, not that the order is right; arms A-D say that.
#
# ⚠ LD_DEBUG=bindings, which is how upstream settled the same question in one
# command, CANNOT be used on the subject: it is interpreted by glibc's dynamic
# loader, and the subject is static with the loader compiled in. It is used on
# the CONTROL in experiments/93-, where the process really is dynamic.
#
# Exit: 0 both directions hold and the control reproduces the defect, 1 they
#       ran and did not, 2 they could not run.
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "94 - the own-symbol table: what must win, and what must yield"

PGB="$REPO_DIR/pgb"
WORK="${PGB_T073_WORK:-/var/tmp/pgb-exp94}"
RESULT="$EXP_OUT/RESULT.txt"

[ -x "$PGB" ] || { printf 'pgb is not built: run make\n'; exit 2; }
[ -d "$ENV_ROOT" ] || {
  printf 'no build environment at %s\n' "$ENV_ROOT"
  printf 'run: ./pgb --engine chroot env create\n'
  exit 2
}

rm -rf "$WORK"; mkdir -p "$WORK" || exit 2

# ---------------------------------------------------------------------------
# The four probe objects. Each is the smallest thing that can hold one end of
# one direction.
# ---------------------------------------------------------------------------
cat > "$WORK/mcprovider.c" <<'EOF'
/* A REAL definition of _dl_mcount_wrapper_check. The loader's no-op stand-in
 * must yield to this one. */
static int calls;
void _dl_mcount_wrapper_check(void *selfpc) { (void)selfpc; calls++; }
int provider_calls(void) { return calls; }
EOF

cat > "$WORK/mccaller.c" <<'EOF'
/* Imports the name. Whoever answers gets the call, and provider_calls() is
 * how the subject finds out which. */
extern void _dl_mcount_wrapper_check(void *);
int caller_fire(void) { _dl_mcount_wrapper_check((void *)0x1234); return 1; }
EOF

cat > "$WORK/tlsdecoy.c" <<'EOF'
/* A decoy __tls_get_addr. It returns its own slab, which is why the VALUE a
 * thread-local round trip produces cannot tell the two loaders apart -- the
 * decoy is self-consistent. The counter is the observable. */
struct tls_index { unsigned long ti_module, ti_offset; };
static int hits;
static char slab[8192];
void *__tls_get_addr(struct tls_index *ti) { (void)ti; hits++; return slab; }
int decoy_calls(void) { return hits; }
EOF

cat > "$WORK/tlsuser.c" <<'EOF'
/* General-dynamic TLS: each access compiles to a __tls_get_addr call. */
__thread unsigned long tls_slot;
void tls_set(unsigned long v) { tls_slot = v; }
unsigned long tls_get(void) { return tls_slot; }
EOF

cat > "$WORK/subject.c" <<'EOF'
/* The subject. Each arm prints one machine-readable line; the experiment reads
 * that line, never the exit status alone -- a wrong binding here does not
 * crash, it answers. */
#include <stdio.h>
#include <string.h>
#include <dlfcn.h>

static void *must_open(const char *p)
{
    void *h = dlopen(p, RTLD_NOW | RTLD_GLOBAL);
    if (!h)
        printf("open-failed %s: %s\n", p, dlerror());
    return h;
}

int main(int argc, char **argv)
{
    const char *arm = argc > 1 ? argv[1] : "";

    if (strcmp(arm, "yield") == 0 && argc == 4) {
        void *prov = must_open(argv[2]);
        void *call = must_open(argv[3]);
        int (*fire)(void); int (*calls)(void);
        if (!prov || !call) return 2;
        fire  = (int (*)(void))dlsym(call, "caller_fire");
        calls = (int (*)(void))dlsym(prov, "provider_calls");
        if (!fire || !calls) { printf("dlsym-failed\n"); return 2; }
        fire();
        printf("provider_calls=%d\n", calls());
        return 0;
    }
    if (strcmp(arm, "standin") == 0 && argc == 3) {
        void *call = must_open(argv[2]);
        int (*fire)(void);
        if (!call) return 2;
        fire = (int (*)(void))dlsym(call, "caller_fire");
        if (!fire) { printf("dlsym-failed\n"); return 2; }
        printf("standin_fire=%d\n", fire());
        return 0;
    }
    if (strcmp(arm, "win") == 0 && argc == 4) {
        void *decoy = must_open(argv[2]);
        void *user  = must_open(argv[3]);
        void (*set)(unsigned long); unsigned long (*get)(void); int (*hits)(void);
        if (!decoy || !user) return 2;
        set  = (void (*)(unsigned long))dlsym(user, "tls_set");
        get  = (unsigned long (*)(void))dlsym(user, "tls_get");
        hits = (int (*)(void))dlsym(decoy, "decoy_calls");
        if (!set || !get || !hits) { printf("dlsym-failed\n"); return 2; }
        set(0x5eeded);
        printf("decoy_calls=%d tls=0x%lx\n", hits(), get());
        return 0;
    }
    if (strcmp(arm, "own") == 0 && argc == 3) {
        void *user = must_open(argv[2]);
        void (*set)(unsigned long); unsigned long (*get)(void);
        if (!user) return 2;
        set = (void (*)(unsigned long))dlsym(user, "tls_set");
        get = (unsigned long (*)(void))dlsym(user, "tls_get");
        if (!set || !get) { printf("dlsym-failed\n"); return 2; }
        set(0x5eeded);
        printf("tls=0x%lx\n", get());
        return 0;
    }
    printf("usage: subject yield|standin|win|own <paths...>\n");
    return 2;
}
EOF

# ---------------------------------------------------------------------------
# Build. ⛔ ONE pgb build AT A TIME on the bed. RULES.md.
#
# The probe objects are built by the PINNED environment, not by the host
# compiler, so the versioned undefined references they carry are the pin's.
# ---------------------------------------------------------------------------
printf -- '-- building --------------------------------------------------\n'

BUILDDIR="$WORK"
if ! "$PGB" --engine chroot build --bind "$BUILDDIR" -- \
        sh -c "cd $BUILDDIR && cc -shared -fPIC -o mcprovider.so mcprovider.c \
               && cc -shared -fPIC -o mccaller.so mccaller.c \
               && cc -shared -fPIC -o tlsdecoy.so tlsdecoy.c \
               && cc -shared -fPIC -ftls-model=global-dynamic -o tlsuser.so tlsuser.c" \
        >"$WORK/build-so.log" 2>&1; then
  printf 'could not build the probe objects\n'; tail -5 "$WORK/build-so.log"; exit 2
fi
for o in mcprovider mccaller tlsdecoy tlsuser; do
  printf '  built  %-14s %s bytes\n' "$o.so" "$(wc -c < "$WORK/$o.so")"
done

if ! "$PGB" --engine chroot build --host-dlopen --bind "$BUILDDIR" -- \
        sh -c "cd $BUILDDIR && cc -o subject subject.c" \
        >"$WORK/build-subject.log" 2>&1; then
  printf 'could not build the subject\n'; tail -5 "$WORK/build-subject.log"; exit 2
fi
printf '  built  %-14s %s bytes  (--host-dlopen)\n' subject "$(wc -c < "$WORK/subject")"

# ⭐ THE NEGATIVE CONTROL, built through the builder knob rather than by
# patching the tree. It merges the two tables back into el_provider(), which
# is the pre-T-073 shape exactly.
if ! PGB_T073_OWNSYMS_UNORDERED=1 "$PGB" --engine chroot build --host-dlopen \
        --bind "$BUILDDIR" -- \
        sh -c "cd $BUILDDIR && cc -o subject-unordered subject.c" \
        >"$WORK/build-control.log" 2>&1; then
  printf '  note   the reversal control did not build; its rows will read not-built\n'
fi
[ -f "$WORK/subject-unordered" ] && \
  printf '  built  %-14s %s bytes  (PGB_T073_OWNSYMS_UNORDERED=1)\n' \
    subject-unordered "$(wc -c < "$WORK/subject-unordered")"

# ⛔ The subject must be one ordinary static ELF. A binary that grew a
# PT_INTERP would pass every row below by quietly being dynamic and letting
# ld.so answer the very question this file is asking.
interp=$(readelf -lW "$WORK/subject" 2>/dev/null | grep -c INTERP || true)
exp_check "the subject has no PT_INTERP" "$interp" "0"

# ⚠ AN ABSENCE IS NOT A ZERO. If the compiler relaxed the thread-local access
# to initial-exec there is no __tls_get_addr call in tlsuser.so at all, and
# arms C and D would pass while measuring nothing.
gd=$(readelf --dyn-syms -W "$WORK/tlsuser.so" 2>/dev/null \
     | grep -c 'UND *__tls_get_addr' || true)
exp_check "tlsuser.so really imports __tls_get_addr" "$gd" "1"
dd=$(readelf --dyn-syms -W "$WORK/tlsdecoy.so" 2>/dev/null \
     | grep '__tls_get_addr' | grep -vc UND || true)
exp_check "tlsdecoy.so really defines __tls_get_addr" "$dd" "1"
md=$(readelf --dyn-syms -W "$WORK/mcprovider.so" 2>/dev/null \
     | grep '_dl_mcount_wrapper_check' | grep -vc UND || true)
exp_check "mcprovider.so really defines _dl_mcount_wrapper_check" "$md" "1"

# ⚠ AND THE TWO BINARIES MUST DIFFER. They are the same size, so `cmp` is the
# check: an unchanged control would make every reversal row below vacuous.
if [ -f "$WORK/subject-unordered" ]; then
  exp_check "the reversal control is a different binary" \
    "$(cmp -s "$WORK/subject" "$WORK/subject-unordered" && echo same || echo differs)" \
    "differs"
fi

# ---------------------------------------------------------------------------
# ARMS A-D on the build host, against the fixed loader.
# ---------------------------------------------------------------------------
printf -- '\n-- arms A-D: the two directions, fixed loader -----------------\n'
cd "$WORK" || exit 2

a_yield=$(./subject yield ./mcprovider.so ./mccaller.so 2>&1); a_rc=$?
printf '  A yield   rc=%s  %s\n' "$a_rc" "$a_yield"
exp_check "A: a loaded object's definition WINS" "$a_yield" "provider_calls=1"

b_standin=$(./subject standin ./mccaller.so 2>&1); b_rc=$?
printf '  B standin rc=%s  %s\n' "$b_rc" "$b_standin"
exp_check "B: with nothing else, the stand-in ANSWERS" "$b_standin" "standin_fire=1"

c_win=$(./subject win ./tlsdecoy.so ./tlsuser.so 2>&1); c_rc=$?
printf '  C win     rc=%s  %s\n' "$c_rc" "$c_win"
exp_check "C: our __tls_get_addr WINS over a loaded definition" \
          "$c_win" "decoy_calls=0 tls=0x5eeded"

d_own=$(./subject own ./tlsuser.so 2>&1); d_rc=$?
printf '  D own     rc=%s  %s\n' "$d_rc" "$d_own"
exp_check "D: with nothing else, ours ANSWERS" "$d_own" "tls=0x5eeded"

# ---------------------------------------------------------------------------
# ⭐ THE NEGATIVE CONTROL. The single-table shape, and what it gets wrong.
# ---------------------------------------------------------------------------
printf -- '\n-- the reversal: one table, checked first, called last --------\n'
if [ ! -f "$WORK/subject-unordered" ]; then
  exp_skip "the reversal control" "it did not build"
  r_yield=not-built; r_standin=not-built; r_win=not-built; r_own=not-built
else
  r_yield=$(./subject-unordered yield ./mcprovider.so ./mccaller.so 2>&1)
  r_standin=$(./subject-unordered standin ./mccaller.so 2>&1)
  r_win=$(./subject-unordered win ./tlsdecoy.so ./tlsuser.so 2>&1)
  r_own=$(./subject-unordered own ./tlsuser.so 2>&1)
  printf '  A yield   %s\n  B standin %s\n  C win     %s\n  D own     %s\n' \
    "$r_yield" "$r_standin" "$r_win" "$r_own"

  # The yield direction was already right under one table -- by accident, but
  # right -- so the control must NOT move A or B. A control that broke
  # everything would not locate anything.
  exp_check "reversal: A is unchanged (yield was already correct)" \
            "$r_yield" "provider_calls=1"
  exp_check "reversal: B is unchanged" "$r_standin" "standin_fire=1"
  exp_check "reversal: D is unchanged" "$r_own" "tls=0x5eeded"

  # ⭐ AND C IS THE DEFECT. The decoy answered, twice.
  exp_check "reversal: C DIFFERS from the fixed loader" \
            "$([ "$r_win" != "$c_win" ] && echo differs || echo same)" "differs"
  exp_check "reversal: the decoy answered both accesses" \
            "$r_win" "decoy_calls=2 tls=0x5eeded"
  exp_note "⛔ the VALUE is 0x5eeded in BOTH -- the decoy is self-consistent,"
  exp_note "   so a round-trip assertion passes while the binding is wrong."
  exp_note "   Only the call count separates them. That is the whole finding."
fi

# ---------------------------------------------------------------------------
# The eleven. ⚠ This says the order does not vary with the target; arms A-D
# above are what say the order is right.
# ---------------------------------------------------------------------------
printf -- '\n-- the eleven ------------------------------------------------\n'

cell() { # rootfs arm arg...
  _r="$1"; shift
  cp "$WORK/subject" "$_r/subject" 2>/dev/null || { printf 'copy-failed'; return; }
  for f in mcprovider.so mccaller.so tlsdecoy.so tlsuser.so; do
    cp "$WORK/$f" "$_r/$f" 2>/dev/null
  done
  _out=$("$PGB" rootfs run "$_r" -- /subject "$@" 2>/dev/null)
  _st=$?
  rm -f "$_r/subject" "$_r"/mcprovider.so "$_r"/mccaller.so \
        "$_r"/tlsdecoy.so "$_r"/tlsuser.so
  if [ "$_st" -ne 0 ]; then
    case $_st in
      13[0-9]|1[4-6][0-9]) printf 'SIG%s' "$((_st-128))" ;;
      *) printf 'exit%s' "$_st" ;;
    esac
    return
  fi
  printf '%s' "$_out"
}

rows=0; ok_yield=0; ok_win=0
printf '  %-22s %-6s %-18s %s\n' TARGET LIBC 'A yield' 'C win'
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  root=$(exp_rootfs "$name")
  [ -n "$root" ] || { printf '  %-22s %-6s %s\n' "$name" "$libc" '(absent)'; continue; }
  rows=$((rows+1))
  cy=$(cell "$root" yield /mcprovider.so /mccaller.so)
  cw=$(cell "$root" win /tlsdecoy.so /tlsuser.so)
  [ "$cy" = "provider_calls=1" ] && ok_yield=$((ok_yield+1))
  [ "$cw" = "decoy_calls=0 tls=0x5eeded" ] && ok_win=$((ok_win+1))
  printf '  %-22s %-6s %-18s %s\n' "$name" "$libc" "$cy" "$cw"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

if [ "$rows" -eq 0 ]; then
  exp_skip "the eleven" "no rootfs on disk: run ./pgb rootfs fetch"
else
  exp_check "yield holds on every row present" "$ok_yield" "$rows"
  exp_check "win holds on every row present"   "$ok_win"   "$rows"
fi

# ---------------------------------------------------------------------------
{
  printf 'experiment 94 - the own-symbol table: what must win, what must yield\n'
  printf 'date: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'kernel: %s\n' "$(uname -sr)"
  printf 'environment: %s\n' "$ENV_NAME"
  printf '\n'
  printf 'THE QUESTION: el_own_syms[] was ONE table checked first and\n'
  printf 'unconditionally inside el_provider(). Its two entries have opposite\n'
  printf 'requirements and nothing distinguished them or asserted the ordering\n'
  printf 'that made one of them safe.\n\n'
  printf 'arm                                   fixed loader              reversal (pre-T-073)\n'
  printf 'A yield  a loaded object defines it    %-25s %s\n' "$a_yield" "$r_yield"
  printf 'B standin nothing else defines it      %-25s %s\n' "$b_standin" "$r_standin"
  printf 'C win    a loaded object defines it    %-25s %s\n' "$c_win" "$r_win"
  printf 'D own    nothing else defines it       %-25s %s\n' "$d_own" "$r_own"
  printf '\n'
  printf 'C IS THE DEFECT, and the value column is why it was never noticed:\n'
  printf 'tls=0x5eeded is correct in BOTH. The decoy returns its own slab to\n'
  printf 'every caller, so a round trip through it is self-consistent. Only\n'
  printf 'decoy_calls -- 0 fixed, 2 reversed -- says which loader answered.\n\n'
  printf 'the eleven: rows=%s yield-ok=%s win-ok=%s\n' "$rows" "$ok_yield" "$ok_win"
  printf '\n'
  printf 'reproduce:\n'
  printf '  sh experiments/94-own-symbol-order.sh\n'
  printf 'the reversal alone:\n'
  printf '  PGB_T073_OWNSYMS_UNORDERED=1 ./pgb --engine chroot build \\\n'
  printf '      --host-dlopen -- cc -o subject subject.c\n'
} > "$RESULT"
printf '\nwrote %s\n' "$RESULT"

exp_finish
