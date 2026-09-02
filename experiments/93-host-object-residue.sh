#!/bin/sh
# 93 - every shared object on this host, through pgb's own ELF loader.
#
# -- THE QUESTION -----------------------------------------------------------
#
# T-064 closed with `--host-dlopen` loading a HOST shared object from a static
# glibc binary, on 11 of 11 environments. ⛔ It did not load EVERYTHING, and
# `TODO` T-068 exists so the remainder is carried as work rather than rounded
# off in a summary: 818 of 904 objects on the build host loaded, and this is
# the harness that says what the other 86 are.
#
# ⛔ WHY IT IS AN EXPERIMENT AND WAS NOT ONE. The 904-object figure quoted in
# `docs/limitations.md`, `docs/AGENTS.md` and three entries came from an
# ad-hoc sweep that was never committed. ⚠ That is a number with no command
# that reproduces it, which `docs/AGENTS.md` §0b forbids in a document — and
# T-068's own Prove asks for the sweep re-run, which nobody can do without the
# harness. This file is that harness.
#
# -- THE SHAPE, AND WHY ONE FORK PER OBJECT ---------------------------------
#
# ⛔ A crash in one object must leave the rest measurable. `experiments/50-`
# learned the same thing: a loader defect that takes SIGSEGV inside object 12
# would otherwise end the run and report 892 objects as untested while looking
# like a total failure. So each object is a separate `timeout`ed process, and
# the exit status is the classifier's first input.
#
#   0        the object loaded and its initialisers ran
#   1        dlopen returned NULL and dlerror said why  -> classified by message
#   >128     killed by a signal                          -> a CRASH
#   124      timeout                                     -> a HANG, not a crash
#
# ⭐ THE DISTINCTION THE ENTRY TURNS ON. A refusal with a message is the
# loader working: `el_refused_class()` declines NSS modules and allocator or
# sanitizer interposers BY NAME, because an interposer has to be present
# before libc initialises and in this image it already is. A signal is the
# loader failing. T-068's Prove is "the crash count at zero because each is a
# named refusal rather than a signal", so the two must never be summed.
#
# -- WHAT THIS CANNOT SETTLE ------------------------------------------------
#
# ⚠ It measures LOADING, not behaviour. `docs/limitations.md` §1 and
# `pg83/solo`'s own README say the same thing: loading is the floor, not the
# claim. An object that maps, relocates and runs its constructors may still
# misbehave when called, and nothing here calls it.
#
# ⚠ The population is THIS host's shared objects, so the count moves with the
# machine. The classes are the result; 904 is not a constant.
#
# Exit: 0 the sweep ran and every failure is a named refusal, 1 it ran and some
#       failure was a signal, 2 it could not run.
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "93 - every host shared object through pgb's own ELF loader"

PGB="$REPO_DIR/pgb"
WORK="${PGB_T068_WORK:-/var/tmp/pgb-exp93}"
RESULT="$EXP_OUT/RESULT.txt"
PER_OBJ="$EXP_OUT/per-object.txt"
TIMEOUT="${PGB_T068_TIMEOUT:-20}"

[ -x "$PGB" ] || { exp_skip "pgb" "not built; run make"; exp_finish; }
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2

# ---------------------------------------------------------------------------
# The probe
# ---------------------------------------------------------------------------
printf -- '-- the probe -----------------------------------------------------\n'
# ⚠ RTLD_NOW, so an undefined symbol is found at load time rather than at the
# first call. Binding lazily would report objects as loaded that cannot run.
cat > "$WORK/probe.c" <<'EOF'
#include <dlfcn.h>
#include <stdio.h>
int main(int argc, char **argv)
{
    void *h;
    const char *e;
    if (argc < 2) return 2;
    h = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (h != NULL)
        return 0;
    e = dlerror();
    fprintf(stderr, "%s\n", e ? e : "dlopen returned NULL and dlerror said nothing");
    return 1;
}
EOF

if ! "$PGB" --engine chroot build --host-dlopen --bind "$WORK" -- \
        sh -c "cd $WORK && cc -o probe probe.c" >"$WORK/build.log" 2>&1; then
  exp_skip "the probe" "it did not build; see $WORK/build.log"
  exp_finish
fi
exp_check "the probe built with --host-dlopen" \
  "$([ -x "$WORK/probe" ] && echo yes || echo no)" yes
# ⛔ AND IT MUST BE THE STATIC SHAPE, OR THIS MEASURES THE HOST'S ld.so AND
# SAYS NOTHING ABOUT OUR LOADER. A probe that kept its interpreter would load
# most of these objects perfectly well, print a fine table, and be a
# measurement of glibc. Same assertion experiments/76- makes on its subject.
exp_check "the probe has no interpreter" \
  "$("$PGB" elf info "$WORK/probe" 2>/dev/null | awk '$1=="interpreter"{print $2}')" "(none)"
exp_check "the probe has no DT_NEEDED" \
  "$("$PGB" elf info "$WORK/probe" 2>/dev/null | awk '$1=="needed"{print $2}')" "(none)"
exp_note "probe: $(wc -c < "$WORK/probe") bytes"

# ---------------------------------------------------------------------------
# The population
# ---------------------------------------------------------------------------
printf -- '\n-- the population ------------------------------------------------\n'
# ⚠ `/lib` and `/usr/lib` are the same tree on a merged-usr host, so the same
# file is reached twice. It is NOT deduplicated: the entry's own count says
# "each counted twice, /lib and /usr/lib being the same file", and changing
# that silently would make this sweep incomparable with the one it reproduces.
: > "$WORK/objects.txt"
for d in /lib /lib64 /usr/lib /usr/lib64 /usr/local/lib; do
  [ -d "$d" ] || continue
  find "$d" -type f -name '*.so*' 2>/dev/null >> "$WORK/objects.txt"
done
sort -u -o "$WORK/objects.txt" "$WORK/objects.txt"
NOBJ=$(wc -l < "$WORK/objects.txt")
exp_check "objects found to sweep" "$([ "$NOBJ" -gt 100 ] && echo many || echo "$NOBJ")" many
exp_note "$NOBJ shared objects under /lib /lib64 /usr/lib /usr/lib64 /usr/local/lib"

# ---------------------------------------------------------------------------
# The sweep
# ---------------------------------------------------------------------------
printf -- '\n-- the sweep -----------------------------------------------------\n'
# ⛔ THE CLASSES ARE READ OUT OF THE LOADER'S OWN MESSAGES, not guessed from
# the exit status. Each string below is emitted by exactly one el_err() call in
# tool/runtime/pgb-elfload.c, so a message this classifier does not recognise
# lands in `other` and is PRINTED rather than absorbed -- an unclassified
# failure is a finding, not a rounding error.
classify() {  # message -> class
  case "$1" in
    *"undefined symbol:"*)            printf 'undefined' ;;
    *"TLSDESC relocation"*)           printf 'tlsdesc' ;;
    *"static TLS surplus exhausted"*) printf 'tls-surplus' ;;
    *"an NSS module"*)                printf 'refused-nss' ;;
    *"interposer"*)                   printf 'refused-interposer' ;;
    *"a C library this image does not"*) printf 'foreign-libc' ;;
    *"cannot find"*)                  printf 'missing-dep' ;;
    *"is already served by this image"*) printf 'served-by-image' ;;
    *"not an x86-64 ELF shared object"*) printf 'not-an-object' ;;
    *"unhandled relocation type"*)    printf 'unhandled-reloc' ;;
    *"PT_TLS"*|*"static-TLS bookkeeping"*) printf 'tls-other' ;;
    *)                                printf 'other' ;;
  esac
}

: > "$PER_OBJ"
OK=0; REFUSED=0; FAILED=0; CRASH=0; HANG=0
: > "$WORK/crashes.txt"
: > "$WORK/other.txt"
while read -r obj; do
  [ -n "$obj" ] || continue
  # ⛔ `</dev/null` IS NOT OPTIONAL. This loop reads its object list on stdin,
  # and a child that reads stdin consumes the rest of it — the loop then ends
  # early having swept a fraction of the population and reported a clean table
  # for it. The probe does not read stdin today; the redirect is here so that
  # a future one cannot silently truncate the measurement.
  msg=$(timeout -k 5 "$TIMEOUT" "$WORK/probe" "$obj" 2>&1 >/dev/null </dev/null)
  st=$?
  case "$st" in
    0)   cls=ok; OK=$((OK + 1)) ;;
    124) cls=hang; HANG=$((HANG + 1)); printf '%s\thang\n' "$obj" >> "$WORK/crashes.txt" ;;
    1)   cls=$(classify "$msg")
         case "$cls" in
           refused-nss|refused-interposer|served-by-image|not-an-object)
             REFUSED=$((REFUSED + 1)) ;;
           other)
             FAILED=$((FAILED + 1)); printf '%s\t%s\n' "$obj" "$msg" >> "$WORK/other.txt" ;;
           *)
             FAILED=$((FAILED + 1)) ;;
         esac ;;
    *)   # ⛔ Anything else is a SIGNAL. 128+n from the shell, or a raw code.
         cls=crash; CRASH=$((CRASH + 1))
         printf '%s\tstatus=%s\t%s\n' "$obj" "$st" "$msg" >> "$WORK/crashes.txt" ;;
  esac
  printf '%s\t%s\t%s\n' "$obj" "$cls" "$msg" >> "$PER_OBJ"
done < "$WORK/objects.txt"

printf '  %-22s %s\n' 'CLASS' 'COUNT'
printf '  %-22s %s\n' 'ok (loaded)' "$OK"
printf '  %-22s %s\n' 'refused by name' "$REFUSED"
printf '  %-22s %s\n' 'failed with a reason' "$FAILED"
printf '  %-22s %s\n' 'CRASHED (a signal)' "$CRASH"
printf '  %-22s %s\n' 'hung' "$HANG"
printf '\n  by reason:\n'
awk -F'\t' '$2!="ok"{n[$2]++} END{for (k in n) printf "  %-22s %s\n", k, n[k]}' \
  "$PER_OBJ" | sort -k2 -rn

# ⛔ THE ASSERTION T-068's PROVE NAMES. A refusal with a message is the loader
# working; a signal is the loader failing. They are never summed.
exp_check "no host object crashes the loader" "$CRASH" 0
exp_check "no host object hangs the loader"   "$HANG"  0

if [ -s "$WORK/crashes.txt" ]; then
  printf '\n  ⛔ the objects that crashed or hung:\n'
  sed 's/^/    /' "$WORK/crashes.txt"
fi
if [ -s "$WORK/other.txt" ]; then
  printf '\n  ⚠ failures this classifier does not recognise -- read them:\n'
  sed 's/^/    /' "$WORK/other.txt" | head -20
fi

{
  printf '93 - host shared objects through pgb-elfload\n'
  printf 'date        : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'host        : %s\n' "$(uname -sr)"
  printf 'objects     : %s\n\n' "$NOBJ"
  printf 'ok=%s refused=%s failed=%s crash=%s hang=%s\n\n' \
    "$OK" "$REFUSED" "$FAILED" "$CRASH" "$HANG"
  printf 'by reason:\n'
  awk -F'\t' '$2!="ok"{n[$2]++} END{for (k in n) printf "  %-22s %s\n", k, n[k]}' \
    "$PER_OBJ" | sort -k2 -rn
  printf '\ncrashed or hung:\n'
  if [ -s "$WORK/crashes.txt" ]; then sed 's/^/  /' "$WORK/crashes.txt"; else printf '  none\n'; fi
} > "$RESULT"
exp_note "written: $RESULT"
exp_note "per-object: $PER_OBJ"

exp_finish
