#!/bin/sh
# THE QUESTION
#
#   `pkgforge-dev/Anylinux-AppImages`' FAQ dismisses the route this project
#   took — a loader compiled into a static binary — and one of its three
#   objections is a specific, testable claim about symbol binding:
#
#     "Also it seems none of the solutions implement `dlmopen`, so you are
#      likely to run into a lot of symbol collisions with host libraries
#      depending on what you end up building."
#
#   ⭐ It is aimed at `solo` and `detour`; `tool/runtime/pgb-elfload.c` is our
#   own loader and nobody has asked it this question. `docs/research/
#   bundle-capabilities.md` answers it from the DESIGN — each loaded object's
#   undefined symbols are resolved by us, against the static glibc already
#   linked in, and nothing is added to a global search scope — ⛔ and reading
#   a design is not measuring it.
#
# -- ⭐ THE PROBE, AND WHY IT DISCRIMINATES ---------------------------------
#
#   liba.so   defines  int pgb_which(void) { return 1; }
#   libb.so   defines  int pgb_which(void) { return 2; }
#             and      int b_calls_which(void) { return pgb_which(); }
#
#   ⭐ A CALL TO A DEFAULT-VISIBILITY FUNCTION INSIDE ONE SHARED OBJECT GOES
#   THROUGH THE PLT AND IS INTERPOSABLE. That is the whole mechanism the
#   objection is about: if `liba.so`'s symbols reach a scope `libb.so` is
#   searched against, `b_calls_which()` returns 1 — B calling A's definition
#   of its own function name. If they do not, it returns 2.
#
#   ⛔ So the number IS the answer, and a wrong answer is not a crash: it is a
#   program quietly running the wrong code, which is exactly the failure the
#   FAQ is warning about.
#
# -- ⛔ PRE-REGISTERED EXPECTATIONS -----------------------------------------
#
#   N1  ⭐ THE HAZARD IS REAL AND THE PROBE SEES IT. A DYNAMIC control, glibc's
#       own `ld.so`, `dlopen(liba, RTLD_NOW|RTLD_GLOBAL)` then
#       `dlopen(libb, RTLD_NOW)`:  b_calls_which() == 1.
#       ⛔ Without this row a "2" from the subject proves nothing — it could
#       mean the probe cannot detect a collision at all. This is the positive
#       control for the INSTRUMENT.
#   N2  the same dynamic control with RTLD_LOCAL on liba:  == 2.
#       ⚠ So the probe is not rigged: it reads 1 only when a global scope
#       actually exists.
#   N3  ⭐ THE SUBJECT — the same source built `pgb build --host-dlopen`,
#       whose `dlopen` is `pgb-elfload.c` and not glibc's:  == 2 predicted,
#       and the prediction is read off the loader's structure rather than
#       hoped for. ⛔ A 1 here would be a real defect and the FAQ's objection
#       landing on us.
#   N4  and both objects actually loaded — a `dlopen` that failed would make
#       N3 unreadable. Each handle is checked and `pgb_which` is resolved
#       from each handle directly, which must give 1 from A and 2 from B.
#
# ⛔ WHAT THIS DOES NOT MEASURE, and the limit is stated rather than found
# later. It runs on the BUILD HOST only: the binding decision is made by code
# compiled INTO the subject and is not a property of the machine it runs on,
# so the eleven would re-measure one answer eleven times. ⚠ It is two objects
# and one symbol; it says nothing about how a large host library's whole
# symbol table interacts, and nothing about `dlmopen`'s other guarantee —
# separate copies of a library's DATA.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "104 - does our compiled-in loader leak symbols between objects?"

WORK="${PGB_EXP104_WORK:-/var/tmp/t104}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
PGB="$REPO_DIR/pgb"
[ -x "$PGB" ] || { exp_note "no ./pgb — run make"; exit 2; }

BUILDDIR=/var/tmp/pgb-exp104
rm -rf "$BUILDDIR"; mkdir -p "$BUILDDIR" || exit 2

cat > "$BUILDDIR/a.c" <<'C'
int pgb_which(void) { return 1; }
C

cat > "$BUILDDIR/b.c" <<'C'
int pgb_which(void) { return 2; }
/* ⭐ The call that decides it. Inside one object, a call to a
 * default-visibility function still goes through the PLT, so a global scope
 * containing another definition of `pgb_which` wins here. */
int b_calls_which(void) { return pgb_which(); }
C

cat > "$BUILDDIR/probe.c" <<'C'
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>

/* argv[1] = liba.so, argv[2] = libb.so, argv[3] = "global" | "local" */
int main(int argc, char **argv)
{
    int flags = RTLD_NOW;
    void *ha, *hb;
    int (*a_which)(void), (*b_which)(void), (*b_calls)(void);

    if (argc < 4) { printf("usage: probe A B global|local\n"); return 2; }
    if (strcmp(argv[3], "global") == 0)
        flags |= RTLD_GLOBAL;

    ha = dlopen(argv[1], flags);
    if (!ha) { printf("LOAD_A fail %s\n", dlerror()); return 3; }
    hb = dlopen(argv[2], RTLD_NOW);
    if (!hb) { printf("LOAD_B fail %s\n", dlerror()); return 4; }

    a_which = (int (*)(void))dlsym(ha, "pgb_which");
    b_which = (int (*)(void))dlsym(hb, "pgb_which");
    b_calls = (int (*)(void))dlsym(hb, "b_calls_which");
    if (!a_which || !b_which || !b_calls) { printf("SYM fail\n"); return 5; }

    printf("A_WHICH=%d\n", a_which());
    printf("B_WHICH=%d\n", b_which());
    printf("B_CALLS=%d\n", b_calls());
    return 0;
}
C

printf -- '-- building the two objects and the two probes --------------------\n'

# ⛔ THE .so FILES ARE BUILT BY THE PINNED ENVIRONMENT, not by the host, for
# the same reason experiments/76- does it: `-shared` is passed through by pgb's
# wrappers untouched, so these are genuine dynamic objects from the pinned
# glibc rather than whatever the runner happens to have.
if ! "$PGB" --engine chroot build --bind "$BUILDDIR" -- \
        sh -c "cd $BUILDDIR && cc -shared -fPIC -o liba.so a.c \
               && cc -shared -fPIC -o libb.so b.c" \
        >"$WORK/build-so.log" 2>&1; then
  exp_note "could not build the two objects; see $WORK/build-so.log"
  tail -5 "$WORK/build-so.log"; exit 2
fi
exp_check "N0  both objects built" \
  "$([ -s "$BUILDDIR/liba.so" ] && [ -s "$BUILDDIR/libb.so" ] && echo yes || echo no)" yes

# The DYNAMIC control: glibc's own loader, ordinary dlopen.
if ! "$PGB" --engine chroot build --bind "$BUILDDIR" -- \
        sh -c "cd $BUILDDIR && cc -o probe-dyn probe.c -ldl" \
        >"$WORK/build-dyn.log" 2>&1; then
  exp_note "could not build the dynamic control; see $WORK/build-dyn.log"
  tail -5 "$WORK/build-dyn.log"; exit 2
fi

# ⭐ THE SUBJECT: the same source, whose dlopen is pgb-elfload.c.
if ! "$PGB" --engine chroot build --host-dlopen --bind "$BUILDDIR" -- \
        sh -c "cd $BUILDDIR && cc -o probe-static probe.c" \
        >"$WORK/build-static.log" 2>&1; then
  exp_note "could not build the subject; see $WORK/build-static.log"
  tail -5 "$WORK/build-static.log"; exit 2
fi
exp_check "N0  the subject is static (no PT_INTERP)" \
  "$(readelf -l "$BUILDDIR/probe-static" 2>/dev/null | grep -c INTERP || true)" 0

run_probe() {  # binary scope -> prints A_WHICH B_WHICH B_CALLS or "-"
  ( cd "$BUILDDIR" && ./"$1" ./liba.so ./libb.so "$2" 2>&1 ) > "$WORK/out.$1.$2" || true
  _a=$(sed -n 's/^A_WHICH=//p' "$WORK/out.$1.$2")
  _b=$(sed -n 's/^B_WHICH=//p' "$WORK/out.$1.$2")
  _c=$(sed -n 's/^B_CALLS=//p' "$WORK/out.$1.$2")
  printf '%s %s %s' "${_a:--}" "${_b:--}" "${_c:--}"
}

printf -- '\n-- the three rows -------------------------------------------------\n'
printf '  %-28s %-8s %-8s %s\n' PROBE A_WHICH B_WHICH B_CALLS
set -- $(run_probe probe-dyn global);    DG_A=$1; DG_B=$2; DG_C=$3
printf '  %-28s %-8s %-8s %s\n' 'dynamic, liba RTLD_GLOBAL' "$DG_A" "$DG_B" "$DG_C"
set -- $(run_probe probe-dyn local);     DL_A=$1; DL_B=$2; DL_C=$3
printf '  %-28s %-8s %-8s %s\n' 'dynamic, liba RTLD_LOCAL'  "$DL_A" "$DL_B" "$DL_C"
set -- $(run_probe probe-static global); SG_A=$1; SG_B=$2; SG_C=$3
printf '  %-28s %-8s %-8s %s\n' '⭐ OURS, --host-dlopen'    "$SG_A" "$SG_B" "$SG_C"

printf '\n'
exp_check "N4  each object answers from its OWN handle (dynamic)" \
    "$DG_A/$DG_B" "1/2"
exp_check "N4  each object answers from its OWN handle (ours)" \
    "$SG_A/$SG_B" "1/2"
exp_check "N1  ⭐ THE HAZARD IS REAL: dynamic + RTLD_GLOBAL, B calls A's" \
    "$DG_C" 1
exp_check "N2  ⚠ and the probe is not rigged: RTLD_LOCAL, B calls its own" \
    "$DL_C" 2
exp_check "N3  ⭐ OURS: B calls its OWN definition" "$SG_C" 2

if [ "$SG_C" = 2 ] && [ "$DG_C" = 1 ]; then
  exp_note "⭐ SO THE FAQ'S OBJECTION DOES NOT LAND ON THIS LOADER, and the"
  exp_note "   row above is why that sentence is allowed: the SAME probe reads"
  exp_note "   1 under glibc's global scope, so it can see a collision."
  exp_note "⛔ pgb-elfload.c resolves each object's undefined symbols itself,"
  exp_note "   against the static glibc already in the executable, and adds"
  exp_note "   nothing to a scope a later object is searched against."
fi
exp_note "⛔ BUILD HOST ONLY, and deliberately: the binding decision is made by"
exp_note "   code compiled INTO the subject, so the eleven would re-measure one"
exp_note "   answer eleven times. ⚠ Two objects and ONE symbol; it says nothing"
exp_note "   about a large library's whole symbol table, and nothing about"
exp_note "   dlmopen's other guarantee — separate copies of a library's DATA."

exp_finish
