#!/bin/sh
# 76-host-dlopen.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# Can a static glibc binary load a shared object it did NOT link, on every
# environment, with its OWN loader, and without a second libc entering the
# process?
#
# This is docs/limitations.md §1 -- the project's one measured, unfixed
# failure -- and TODO T-064. Three prior measurements set it up:
#
#   experiments/50-  dlopen of a host object from a static binary is
#                    host-dependent, and SUCCESS IS THE WORSE OUTCOME: where
#                    it works, the host's ld.so and libc.so.6 are in the
#                    process.
#   experiments/72-  a better host loader cannot help. A static executable's
#                    dynamic symbol table is EMPTY (DYNSYM 0), so a
#                    host-loaded plugin has nothing to bind back to.
#   experiments/73-  90.8%-97.8% of every versioned import of 5,807 real host
#                    shared objects is already definable by the pinned static
#                    glibc, and the unexplained residue is ZERO.
#
# So the loader has to be ours. tool/runtime/pgb-elfload.c is that loader and
# `pgb build --host-dlopen` compiles it in. This measures whether it holds.
#
# -- ARMS, AND WHY THERE ARE TWO SUBJECTS -----------------------------------
#
#   carried   dlopen a .so built by the PINNED environment, carried to the
#             target. ⭐ This is the 11-of-11 claim, and on the four musl
#             rows it is a GLIBC shared object running on a machine that
#             ships no glibc.
#   native    dlopen a shared object that was already ON the target -- a real
#             host .so, chosen from the rootfs at run time.
#   control   the same source built WITHOUT --host-dlopen. It reaches the
#             host loader, so it restates 50-'s result on this subject.
#
# ⛔ THE NATIVE ARM IS NOT EXPECTED TO PASS ON MUSL, AND THAT IS THE CORRECT
# ANSWER, not a gap. A shared object on an Alpine box carries
# DT_NEEDED libc.musl-x86_64.so.1. This image's libc is glibc; mapping musl's
# libc into it would be the second-libc outcome this whole project exists to
# prevent. The loader refuses it by name. What is asserted on those rows is
# that the refusal is CLEAN -- an error through dlerror(), no signal, and
# still zero host shared objects opened.
#
# ⛔ AND THE SUBJECT ASSERTS NEGATIVES. A loader that returned a handle for
# everything would pass a naive "did dlopen work" test, so the subject also
# requires that a symbol the object does not export does NOT resolve, that a
# path that does not exist does NOT open, and that dlerror() is set in both
# cases. The loaded code must also RUN: a non-NULL dlsym pointer proves a
# table lookup, not a working object, so the subject calls through it and
# checks the value that comes back.
#
# Exit: 0 the measurement ran and matched, 1 it ran and did not, 2 could not run.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "76 - --host-dlopen: a static binary loading an object it did not link"

WORK="$EXP_OUT/build"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
RESULT="$EXP_OUT/RESULT.txt"
PGB="$REPO_DIR/pgb"

[ -x "$PGB" ] || { printf 'pgb is not built (run make)\n'; exit 2; }

# ---------------------------------------------------------------------------
# The carried object, and the program that loads it.
#
# ⚠ The plugin calls back into libc on purpose -- malloc, snprintf, strlen --
# because binding the NAMES is not the claim. If the provider table bound
# `snprintf` to the wrong definition, a lookup test would still pass and this
# would print the wrong string.
# ---------------------------------------------------------------------------
cat > "$WORK/demo.c" <<'EOF'
#include <stdlib.h>
#include <stdio.h>
#include <string.h>

/* A thread-local, so the loader's TLS path is exercised rather than assumed.
 * Its initialiser is non-zero: a loader that placed the block but never
 * copied the image would return 0 here and look like it worked. */
static __thread int demo_tls = 1234;

static int demo_ctor_ran;

__attribute__((constructor))
static void demo_ctor(void) { demo_ctor_ran = 1; }

int demo_answer(void) { return 42; }

int demo_ctor_seen(void) { return demo_ctor_ran; }

int demo_tls_value(void) { return demo_tls; }

/* Round-trips through the host image's libc: allocate, format, measure. */
const char *demo_libc_roundtrip(void)
{
    static char kept[64];
    char *p = malloc(64);
    if (!p) return "malloc-failed";
    snprintf(p, 64, "len=%zu", strlen("abcdefghij"));
    memcpy(kept, p, sizeof kept - 1);
    kept[sizeof kept - 1] = 0;
    free(p);
    return kept;
}

static int not_exported(void) { return 7; }
int demo_uses_static(void) { return not_exported(); }
EOF

cat > "$WORK/host.c" <<'EOF'
/* argv[1] is the object to load. Exit 0 only if every assertion holds. */
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>

/* ⛔ dlerror() CLEARS ON READ. Calling it twice in one expression -- once to
 * test, once to print -- returns the message and then NULL, so a real failure
 * prints "(null)" and reads as "no error was set". Read it exactly once. */
static const char *err_once(void)
{
    const char *e = dlerror();
    return e ? e : "(no error was set)";
}

int main(int argc, char **argv)
{
    const char *path = argc > 1 ? argv[1] : "libdemo.so";
    /* ⛔ argv[2] SELECTS THE ARM, and the first run of this experiment is why
     * it exists: without it the native arm asserted demo_answer() against the
     * target's own libz, which no host object could ever export. It read as
     * "the loader failed on 7 of 7 glibc rows" when the loader had in fact
     * mapped, relocated and initialised the object correctly on every one. */
    const char *want = argc > 2 ? argv[2] : NULL;
    int fails = 0;
    void *h, *bad;
    int (*answer)(void);
    int (*ctor)(void);
    int (*tlsv)(void);
    const char *(*trip)(void);

    h = dlopen(path, RTLD_NOW);
    if (!h) { printf("  FAIL dlopen(%s): %s\n", path, err_once()); return 1; }
    printf("  ok   dlopen returned a handle\n");

    if (want) {
        /* NATIVE arm: a shared object that was already on this machine. What
         * is claimed is that it MAPPED, RELOCATED and BOUND -- one of its own
         * exported symbols must resolve, and a name it cannot have must not. */
        if (dlsym(h, want) == NULL) {
            printf("  FAIL dlsym(%s): %s\n", want, err_once());
            return 1;
        }
        printf("  ok   dlsym resolved %s in a real host object\n", want);
        if (dlsym(h, "pgb_no_such_symbol_at_all") != NULL) {
            printf("  FAIL an absent symbol resolved\n");
            fails++;
        } else printf("  ok   an absent symbol did NOT resolve (%s)\n", err_once());
        if (dlclose(h) != 0) { printf("  FAIL dlclose\n"); fails++; }
        else printf("  ok   dlclose returned 0\n");
        return fails ? 1 : 0;
    }

    answer = (int (*)(void))dlsym(h, "demo_answer");
    ctor   = (int (*)(void))dlsym(h, "demo_ctor_seen");
    tlsv   = (int (*)(void))dlsym(h, "demo_tls_value");
    trip   = (const char *(*)(void))dlsym(h, "demo_libc_roundtrip");
    if (!answer || !ctor || !tlsv || !trip) {
        printf("  FAIL dlsym: %s\n", err_once());
        return 1;
    }
    printf("  ok   dlsym resolved four exported symbols\n");

    if (answer() != 42) { printf("  FAIL demo_answer() = %d\n", answer()); fails++; }
    else printf("  ok   demo_answer() = 42, the loaded code RAN\n");

    if (ctor() != 1) { printf("  FAIL the constructor did not run\n"); fails++; }
    else printf("  ok   DT_INIT_ARRAY constructor ran\n");

    if (tlsv() != 1234) { printf("  FAIL demo_tls_value() = %d, want 1234\n", tlsv()); fails++; }
    else printf("  ok   thread-local read back its initialiser (1234)\n");

    if (strcmp(trip(), "len=10") != 0) {
        printf("  FAIL libc round trip = %s, want len=10\n", trip());
        fails++;
    } else printf("  ok   the object called malloc/snprintf/strlen in OUR libc\n");

    /* ⛔ NEGATIVE: a symbol that is file-local must NOT resolve. */
    if (dlsym(h, "not_exported") != NULL) {
        printf("  FAIL a static function resolved through dlsym\n");
        fails++;
    } else printf("  ok   a file-local symbol did NOT resolve (%s)\n", err_once());

    /* ⛔ NEGATIVE: an object that is not there must NOT open. */
    bad = dlopen("/nonexistent/pgb-no-such-object.so", RTLD_NOW);
    if (bad != NULL) { printf("  FAIL a missing object returned a handle\n"); fails++; }
    else printf("  ok   a missing object did NOT open (%s)\n", err_once());

    if (dlclose(h) != 0) { printf("  FAIL dlclose\n"); fails++; }
    else printf("  ok   dlclose returned 0\n");

    return fails ? 1 : 0;
}
EOF

# ---------------------------------------------------------------------------
# Build: the carried .so, the subject, and the control.
#
# ⛔ ONE pgb build AT A TIME on the bed. RULES.md.
# ---------------------------------------------------------------------------
printf -- '-- building --------------------------------------------------\n'

BUILDDIR=/var/tmp/pgb-exp76
rm -rf "$BUILDDIR"; mkdir -p "$BUILDDIR" || exit 2
cp "$WORK/demo.c" "$WORK/host.c" "$BUILDDIR/" || exit 2

# The carried object. `-shared` is passed through untouched by pgb's wrappers,
# which is what lets a configure script's shared-library probes still work, and
# is what makes this a genuine dynamic .so built by the PINNED glibc.
if ! "$PGB" --engine chroot build --bind "$BUILDDIR" -- \
        sh -c "cd $BUILDDIR && cc -shared -fPIC -o libdemo.so demo.c" \
        >"$WORK/build-so.log" 2>&1; then
  printf 'could not build the carried shared object\n'
  tail -5 "$WORK/build-so.log"
  exit 2
fi
printf '  built  libdemo.so       %s bytes\n' "$(wc -c < "$BUILDDIR/libdemo.so")"

# The subject.
if ! "$PGB" --engine chroot build --host-dlopen --bind "$BUILDDIR" -- \
        sh -c "cd $BUILDDIR && cc -o host-loader host.c" \
        >"$WORK/build-subject.log" 2>&1; then
  printf 'could not build the subject\n'
  tail -5 "$WORK/build-subject.log"
  exit 2
fi
printf '  built  host-loader      %s bytes  (--host-dlopen)\n' \
  "$(wc -c < "$BUILDDIR/host-loader")"

# The control: same source, no --host-dlopen, so dlopen reaches the host ld.so.
if ! "$PGB" --engine chroot build --bind "$BUILDDIR" -- \
        sh -c "cd $BUILDDIR && cc -o host-plain host.c" \
        >"$WORK/build-control.log" 2>&1; then
  printf '  note   the control did not build; its column will read not-built\n'
fi
[ -f "$BUILDDIR/host-plain" ] && \
  printf '  built  host-plain       %s bytes  (control, host loader)\n' \
    "$(wc -c < "$BUILDDIR/host-plain")"

cp "$BUILDDIR/libdemo.so" "$BUILDDIR/host-loader" "$WORK/" 2>/dev/null
[ -f "$BUILDDIR/host-plain" ] && cp "$BUILDDIR/host-plain" "$WORK/"

# ⛔ Assert the subject really is one ordinary static ELF. A binary that grew a
# PT_INTERP would pass every runtime row by quietly being dynamic.
interp=$(readelf -lW "$WORK/host-loader" 2>/dev/null | grep -c INTERP || true)
needed=$(readelf -dW "$WORK/host-loader" 2>/dev/null | grep -c NEEDED || true)
printf '  subject PT_INTERP=%s DT_NEEDED=%s\n' "$interp" "$needed"
printf '\n'

# ---------------------------------------------------------------------------
# The matrix.
# ---------------------------------------------------------------------------
{
  printf 'experiment 76 - --host-dlopen across the pinned bed\n'
  printf 'date (UTC)   : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'host kernel  : %s\n' "$(uname -sr)"
  printf 'subject      : %s bytes, PT_INTERP=%s DT_NEEDED=%s\n' \
    "$(wc -c < "$WORK/host-loader")" "$interp" "$needed"
  printf '\n'
  printf '%-22s %-6s %-9s %-9s %-9s %s\n' \
    TARGET LIBC CARRIED NATIVE CONTROL 'HOST .so LOADED BY SUBJECT'
} > "$RESULT"

printf -- '-- running ---------------------------------------------------\n'
printf '%-22s %-6s %-9s %-9s %-9s %s\n' \
  TARGET LIBC CARRIED NATIVE CONTROL 'HOST .so LOADED BY SUBJECT'

n_t=0; n_carried=0; n_clean=0; n_native_glibc=0; n_glibc=0
n_native_musl_clean=0; n_musl=0; n_control_ok=0

# Run one binary in a rootfs with an argument, echoing a cell.
cell() { # rootfs binary arg
  _r="$1"; _b="$2"; _a="$3"; _base=$(basename "$_b")
  cp "$_b" "$_r/$_base" 2>/dev/null || { printf 'copy-failed'; return; }
  "$PGB" rootfs run "$_r" -- "/$_base" "$_a" >/dev/null 2>&1
  _st=$?
  rm -f "$_r/$_base"
  case $_st in
    0) printf 'ok' ;;
    13[0-9]|1[4-6][0-9]) printf 'SIG%s' "$((_st-128))" ;;
    *) printf 'exit%s' "$_st" ;;
  esac
}

cell3() { # rootfs binary arg1 arg2
  _r="$1"; _b="$2"; _a="$3"; _a2="$4"; _base=$(basename "$_b")
  cp "$_b" "$_r/$_base" 2>/dev/null || { printf 'copy-failed'; return; }
  "$PGB" rootfs run "$_r" -- "/$_base" "$_a" "$_a2" >/dev/null 2>&1
  _st=$?
  rm -f "$_r/$_base"
  case $_st in
    0) printf 'ok' ;;
    13[0-9]|1[4-6][0-9]) printf 'SIG%s' "$((_st-128))" ;;
    *) printf 'exit%s' "$_st" ;;
  esac
}

while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  root=$(exp_rootfs "$name")
  [ -n "$root" ] || continue
  n_t=$((n_t+1))
  [ "$libc" = glibc ] && n_glibc=$((n_glibc+1))
  [ "$libc" = musl ] && n_musl=$((n_musl+1))

  cp "$WORK/libdemo.so" "$root/libdemo.so" 2>/dev/null

  # -- carried: the .so the pinned environment built ------------------------
  c_carried=$(cell "$root" "$WORK/host-loader" "/libdemo.so")

  # -- native: a real shared object that was already on this target ---------
  # Picked from the rootfs itself rather than named, because the set differs
  # per distribution. Preference order is "small and widely present".
  # ⚠ Each candidate is paired with a symbol IT exports. Asserting a symbol the
  # object could not have is what made the first run of this experiment report
  # a loader failure that had not happened.
  native=""; nsym=""
  for pair in "libz.so.1 zlibVersion" "libbz2.so.1 BZ2_bzlibVersion" \
              "libexpat.so.1 XML_ParserCreate" "libffi.so.8 ffi_prep_cif" \
              "libpcre2-8.so.0 pcre2_compile_8" "liblzma.so.5 lzma_version_string"; do
    cand=${pair%% *}; sym=${pair##* }
    for d in lib/x86_64-linux-gnu usr/lib/x86_64-linux-gnu lib64 usr/lib64 lib usr/lib; do
      if [ -f "$root/$d/$cand" ]; then native="/$d/$cand"; nsym="$sym"; break 2; fi
    done
  done
  if [ -n "$native" ]; then
    c_native=$(cell3 "$root" "$WORK/host-loader" "$native" "$nsym")
  else
    c_native='no-cand'
  fi

  # -- control: the same source with no --host-dlopen -----------------------
  if [ -f "$WORK/host-plain" ]; then
    c_control=$(cell "$root" "$WORK/host-plain" "/libdemo.so")
  else
    c_control='not-built'
  fi

  # ⛔ Criterion 2 of docs/AGENTS.md §3, on the CARRIED arm: what did the
  # subject open? A loader that quietly reached the host's ld.so would show
  # up here and nowhere else.
  cp "$WORK/host-loader" "$root/host-loader" 2>/dev/null
  libs=$(exp_trace_libs "$root" "/host-loader" "$WORK/t.$name" \
         | grep -E '\.so(\.[0-9]+)*$' | grep -v '/libdemo\.so$' | tr '\n' ' ')
  rm -f "$root/host-loader" "$root/libdemo.so"

  [ "$c_carried" = ok ] && n_carried=$((n_carried+1))
  [ -z "$libs" ] && n_clean=$((n_clean+1))
  [ "$c_control" = ok ] && n_control_ok=$((n_control_ok+1))
  if [ "$libc" = glibc ] && [ "$c_native" = ok ]; then
    n_native_glibc=$((n_native_glibc+1))
  fi
  # On musl the refusal must be CLEAN: the subject's own exit status, never a
  # signal. exit1 is the subject reporting dlopen failed through dlerror().
  case "$libc:$c_native" in
    musl:exit*) n_native_musl_clean=$((n_native_musl_clean+1)) ;;
    musl:no-cand) n_native_musl_clean=$((n_native_musl_clean+1)) ;;
  esac

  row=$(printf '%-22s %-6s %-9s %-9s %-9s %s' \
        "$name" "$libc" "$c_carried" "$c_native" "$c_control" "${libs:-none}")
  printf '%s\n' "$row"
  printf '%s\n' "$row" >> "$RESULT"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

[ "$n_t" -gt 0 ] || { printf 'no rootfs fetched\n'; exit 2; }
printf '\n'

printf -- '-- assertions ------------------------------------------------\n'
exp_check "carried: nine assertions pass, every environment" "$n_carried" "$n_t"
exp_check "carried: loaded no host shared object, every environment" "$n_clean" "$n_t"
exp_check "native: loads a real host object on every glibc row" "$n_native_glibc" "$n_glibc"
exp_check "native: refuses CLEANLY on every musl row, no signal" "$n_native_musl_clean" "$n_musl"
printf '  --    %-46s = %s of %s\n' \
  "control arm ran (observed, not asserted)" "$n_control_ok" "$n_t"
printf '\n'
exp_note "The control is EXPECTED to fail: with no --host-dlopen its dlopen"
exp_note "reaches the host loader, which is experiments/50-'s result. Its count"
exp_note "is recorded, never asserted, because what it does is host-dependent."
exp_note ""
exp_note "The musl rows of the NATIVE arm are a refusal by design: a host object"
exp_note "there needs musl's libc, and mapping it into a glibc image is the"
exp_note "second-libc outcome docs/limitations.md §1 calls worse than failing."
printf '\n'

{
  printf '\n'
  printf 'carried  = dlopen a .so built by the PINNED glibc environment.\n'
  printf '           On the four musl rows this is a GLIBC shared object\n'
  printf '           running on a machine that ships no glibc.\n'
  printf 'native   = dlopen a shared object already present on the target.\n'
  printf '           Refused by design on musl: it needs musl.\n'
  printf 'control  = the same source with no --host-dlopen. Reaches ld.so.\n'
} >> "$RESULT"

printf 'full table: %s\n' "$RESULT"
rm -rf "$BUILDDIR"

exp_finish
