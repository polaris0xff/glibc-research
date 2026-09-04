#!/bin/sh
# THE QUESTION
#
#   What does a `gcc -static` glibc binary lose when the host has no glibc
#   gconv modules and no glibc locale data -- which is every musl host -- and
#   can either be given back without a data directory beside the binary?
#
# NSS (experiment 20) is the famous one. gconv is the same shape and is worse
# in one specific way: it fails at the point of USE rather than at startup, so
# a binary that has been "tested" can still die the first time a user feeds it
# text in an encoding nobody tried.
#
# -- WHAT IS ACTUALLY BEING MEASURED -----------------------------------------
#
#   arm A  plain -static                       the baseline loss
#   arm B  -static + GNU libiconv, redirected  the candidate fix
#          at LINK time with -Wl,--wrap
#
# ⭐ ARM B CHANGES NO APPLICATION SOURCE. --wrap rewrites undefined references
# at the final link, so it catches calls from any object in the link including
# static archives that were compiled without seeing our headers. That is what
# puts this fix at tier 2 of the project's hierarchy (automatic toolchain
# change) rather than tier 4 (patch the application).
#
# -- WHAT ARM A DOES, WHICH IS TWO DIFFERENT FAILURES ------------------------
#
# Measured here, and the split is decided by ONE THING: whether the host keeps
# its gconv modules at the same path the BUILD glibc compiled in.
#
#   path matches (Debian 11, Debian 12, Ubuntu 20.04 all use
#   /usr/lib/<triplet>/gconv, and so does the Ubuntu build host)
#       -> the host modules are found and dlopen'd, each carrying
#          DT_NEEDED libc.so.6, so a second libc and the dynamic loader enter
#          the "static" process, and it DIES. SIGFPE or SIGABRT, 3 runs of 3.
#
#   path differs (Fedora and Rocky use /usr/lib64/gconv, Arch uses
#   /usr/lib/gconv, openSUSE Leap's minimal image ships none, no musl host has
#   any)
#       -> nothing is found and 11 of 12 encodings are SILENTLY unavailable.
#          iconv_open returns EINVAL and a program that does not check it
#          writes nothing, or writes mojibake.
#
# ⚠ THERE IS NO THIRD, WORKING CASE IN THE MATRIX. "It worked on my Debian
# box" is the first column with a build host whose path happened to match AND
# a program that never reached the crashing call.
#
# -- THE CONTROL THAT MAKES AN EMPTY TRACE MEAN SOMETHING ---------------------
#
# ⛔ Arm B opens no files at all on Alpine. On its own that is not evidence: a
# trace filter looking in the wrong place also prints nothing. Arm A is run
# through the SAME filter on the SAME root filesystem and DOES print the gconv
# lookups, which is what makes arm B's silence a measurement.
#
# Exit: 0 arm B restored what arm A lost, 1 it did not, 2 could not run.

. "$(dirname "$0")/lib.sh"

exp_begin "30 - gconv and locale: what a static glibc binary loses off a glibc host"

B="$EXP_OUT/build"
rm -rf "$B"; mkdir -p "$B" || exit 2

# ⛔ ARM B — THE CANDIDATE FIX, AND THE WHOLE POINT OF THIS EXPERIMENT — HAD
# BEEN SKIPPED ON EVERY RUN, AND A SKIP IS NOT A FAILURE SO BOTH GATES STAYED
# GREEN OVER IT.
#
# `pgb` builds GNU libiconv INSIDE the build environment, so the archive lives
# at `<env root>/opt/pgb-libiconv/lib/libiconv.a` — `cmd/pgb/doctor.go` looks
# for it exactly there. This file looked at `/opt/pgb-libiconv` on the HOST,
# where it has never been, and reported
#
#     SKIP  arm B (static libiconv)  (no libiconv.a at /opt/pgb-libiconv …)
#
# ⭐ Same family as corrections.md C48, C50 and C52: an instrument looking in
# a place the thing is never in, made invisible by a skip. 2026-09-04c.
#
# ⚠ BOTH are tried, host first, and the one actually used is REPORTED — a run
# on a machine that really does have it at the host path must stay valid.
ICONV_PREFIX="${PGB_LIBICONV_PREFIX:-}"
if [ -z "$ICONV_PREFIX" ]; then
  for _ip in /opt/pgb-libiconv "$ENV_ROOT/opt/pgb-libiconv"; do
    [ -f "$_ip/lib/libiconv.a" ] && { ICONV_PREFIX=$_ip; break; }
  done
  ICONV_PREFIX="${ICONV_PREFIX:-/opt/pgb-libiconv}"
fi

cat > "$B/gconv-probe.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <iconv.h>
#include <errno.h>
#include <string.h>
/* Encodings a real program actually meets: Latin-1 text, Windows CP1252 from
 * exported spreadsheets, UTF-16 from Windows tooling, and the CJK sets. */
static const char *pairs[][2] = {
  {"UTF-8","ASCII"}, {"UTF-8","ISO-8859-1"}, {"UTF-8","UTF-16LE"},
  {"UTF-8","ISO-8859-15"}, {"UTF-8","CP1252"}, {"UTF-8","EUC-JP"},
  {"UTF-8","SHIFT_JIS"}, {"UTF-8","GB18030"}, {"UTF-8","KOI8-R"},
  {"UTF-8","BIG5"}, {"UTF-8","ISO-8859-2"}, {"UTF-8","UTF-32"}, {NULL,NULL}
};
int main(void){
  int opened = 0, failed = 0;
  for (int i=0; pairs[i][0]; i++){
    iconv_t c = iconv_open(pairs[i][1], pairs[i][0]);
    if (c == (iconv_t)-1) failed++;
    else { opened++; iconv_close(c); }
  }
  /* ⭐ A ROUND TRIP, NOT JUST AN OPEN. iconv_open succeeding says the module
   * was found; it does not say the conversion produces the right bytes. */
  int roundtrip = 0;
  iconv_t c = iconv_open("ISO-8859-1", "UTF-8");
  if (c != (iconv_t)-1) {
    char in[] = "caf\xc3\xa9";          /* "café" in UTF-8 */
    char out[16]; char *ip=in, *op=out; size_t il=strlen(in), ol=sizeof out;
    if (iconv(c, &ip, &il, &op, &ol) != (size_t)-1 &&
        (size_t)(op-out) == 4 && (unsigned char)out[3] == 0xE9)
      roundtrip = 1;                    /* Latin-1 e-acute is 0xE9 */
    iconv_close(c);
  }
  printf("opened=%d failed=%d roundtrip=%d\n", opened, failed, roundtrip);
  return failed ? 1 : 0;
}
EOF

cat > "$B/iconv-shim.c" <<'EOF'
/* --wrap targets. The application calls iconv_open(); the linker turns that
 * undefined reference into __wrap_iconv_open() and this answers it. */
#include <stddef.h>
typedef void *iconv_t;
extern iconv_t libiconv_open(const char *tocode, const char *fromcode);
extern size_t  libiconv(iconv_t cd, char **inbuf, size_t *inbytesleft,
                        char **outbuf, size_t *outbytesleft);
extern int     libiconv_close(iconv_t cd);
iconv_t __wrap_iconv_open(const char *t, const char *f) { return libiconv_open(t, f); }
size_t  __wrap_iconv(iconv_t cd, char **ib, size_t *il, char **ob, size_t *ol)
        { return libiconv(cd, ib, il, ob, ol); }
int     __wrap_iconv_close(iconv_t cd) { return libiconv_close(cd); }
EOF

${CC:-cc} -static -O2 -o "$B/gconv-plain" "$B/gconv-probe.c" 2>/dev/null || exit 2

HAVE_ICONV=0
if [ -f "$ICONV_PREFIX/lib/libiconv.a" ]; then
  if ${CC:-cc} -static -O2 -o "$B/gconv-shim" "$B/gconv-probe.c" "$B/iconv-shim.c" \
       -Wl,--wrap=iconv_open,--wrap=iconv,--wrap=iconv_close \
       -L"$ICONV_PREFIX/lib" -liconv 2>"$B/shim-link.log"; then
    HAVE_ICONV=1
  fi
fi
if [ "$HAVE_ICONV" = 0 ]; then
  # ⛔ AND THE TWO REASONS ARE DIFFERENT. "The archive is not there" is an
  # environment gap; "it is there and the link failed" is a FINDING, and the
  # old message could not tell them apart.
  if [ -f "$ICONV_PREFIX/lib/libiconv.a" ]; then
    exp_skip "arm B (static libiconv)" \
      "libiconv.a IS at $ICONV_PREFIX but the link failed: $(tr -d '\n' < "$B/shim-link.log" 2>/dev/null | cut -c1-110)"
  else
    exp_skip "arm B (static libiconv)" \
      "no libiconv.a at /opt/pgb-libiconv nor $ENV_ROOT/opt/pgb-libiconv -- run pgb env create"
  fi
else
  exp_note "$(printf 'arm B linked against %s/lib/libiconv.a' "$ICONV_PREFIX")"
fi

exp_note "arm A size: $(wc -c < "$B/gconv-plain") bytes"
[ "$HAVE_ICONV" = 1 ] && exp_note "arm B size: $(wc -c < "$B/gconv-shim") bytes"

# ---------------------------------------------------------------------------
printf '\n  iconv, 12 encodings, opened/failed and one byte-exact round trip:\n'
printf '    %-20s %-6s %-26s %s\n' ENVIRONMENT LIBC 'A plain -static' 'B +static libiconv'

while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  r=$(exp_rootfs "$name")
  [ -n "$r" ] || { exp_skip "$name" "not fetched"; continue; }

  cp "$B/gconv-plain" "$r/pgb-gconv-plain"
  ra=$("$REPO_DIR/pgb" rootfs run "$r" -- /pgb-gconv-plain 2>/dev/null | tr -d '\n')
  rb="(skipped)"
  if [ "$HAVE_ICONV" = 1 ]; then
    cp "$B/gconv-shim" "$r/pgb-gconv-shim"
    rb=$("$REPO_DIR/pgb" rootfs run "$r" -- /pgb-gconv-shim 2>/dev/null | tr -d '\n')
  fi
  printf '    %-20s %-6s %-26s %s\n' "$name" "$libc" "${ra:-<no output>}" "${rb:-<no output>}"

  if [ "$HAVE_ICONV" = 1 ]; then
    # ⭐ THE GATE. Every environment must open all 12 and convert correctly.
    # Asserted on arm B only: arm A's losses are the finding being recorded.
    exp_check "$name: B opens all 12 encodings + round trip" "$rb" "opened=12 failed=0 roundtrip=1"
  fi
  rm -f "$r/pgb-gconv-plain" "$r/pgb-gconv-shim"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

# ---------------------------------------------------------------------------
# The trace pair. Arm A is the positive control for arm B's silence.
# ---------------------------------------------------------------------------
ALP=$(exp_rootfs alpine-3.22)
if [ -n "$ALP" ] && [ "$HAVE_ICONV" = 1 ]; then
  printf '\n  what each arm touches on Alpine 3.22 (musl):\n'
  cp "$B/gconv-plain" "$ALP/pgb-gconv-plain"; cp "$B/gconv-shim" "$ALP/pgb-gconv-shim"
  na=$(exp_trace_opens "$ALP" /pgb-gconv-plain "$B/tr-a.txt" | grep -c gconv)
  nb=$(exp_trace_opens "$ALP" /pgb-gconv-shim  "$B/tr-b.txt" | wc -l)
  exp_check "A: gconv lookups (the positive control)" "$([ "$na" -gt 0 ] && echo some || echo none)" some
  exp_note  "A tried $na gconv paths, every one ENOENT -- this is the loss"
  exp_check "B: files opened by the process at all" "$nb" 0
  rm -f "$ALP/pgb-gconv-plain" "$ALP/pgb-gconv-shim"
fi

# ---------------------------------------------------------------------------
# LOCALE. A separate data dependency from gconv, and it is NOT fixed by the
# iconv work: glibc's C.UTF-8 is a set of files under /usr/lib/locale, not
# something compiled into libc, so a musl host has nothing to give.
# ---------------------------------------------------------------------------
cat > "$B/locale-probe.c" <<'EOF'
#include <stdio.h>
#include <locale.h>
#include <langinfo.h>
#include <stdlib.h>
int main(void){
#ifdef LOCPATH_DIR
  setenv("LOCPATH", LOCPATH_DIR, 1);
#endif
  char *r = setlocale(LC_ALL, "C.UTF-8");
  if (!r) r = setlocale(LC_ALL, "C.utf8");
  printf("locale=%s codeset=%s\n", r ? r : "NULL", nl_langinfo(CODESET));
  return 0;
}
EOF
${CC:-cc} -static -O2 -o "$B/locale-plain" "$B/locale-probe.c" 2>/dev/null
${CC:-cc} -static -O2 -DLOCPATH_DIR='"/opt/pgb-locale"' -o "$B/locale-locpath" "$B/locale-probe.c" 2>/dev/null

HOST_LOCALE_SRC=""
for c in /usr/lib/locale/C.utf8 /usr/lib/locale/C.UTF-8; do
  [ -d "$c" ] && { HOST_LOCALE_SRC="$c"; break; }
done

printf '\n  locale: does setlocale(C.UTF-8) give a UTF-8 codeset?\n'
printf '    %-20s %-6s %-34s %s\n' ENVIRONMENT LIBC 'plain -static' 'with bundled locale + LOCPATH'
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  r=$(exp_rootfs "$name")
  [ -n "$r" ] || continue
  cp "$B/locale-plain" "$r/pgb-locale-plain"
  la=$("$REPO_DIR/pgb" rootfs run "$r" -- /pgb-locale-plain 2>/dev/null | tr -d '\n')
  lb="(no host C.utf8 to bundle)"
  if [ -n "$HOST_LOCALE_SRC" ]; then
    mkdir -p "$r/opt/pgb-locale"
    cp -a "$HOST_LOCALE_SRC" "$r/opt/pgb-locale/C.utf8" 2>/dev/null
    cp "$B/locale-locpath" "$r/pgb-locale-locpath"
    lb=$("$REPO_DIR/pgb" rootfs run "$r" -- /pgb-locale-locpath 2>/dev/null | tr -d '\n')
  fi
  printf '    %-20s %-6s %-34s %s\n' "$name" "$libc" "${la:-<none>}" "${lb:-<none>}"
  case "$lb" in *"codeset=UTF-8"*) exp_check "$name: bundled locale yields UTF-8" yes yes ;;
                *"(no host"*)      : ;;
                *)                 exp_check "$name: bundled locale yields UTF-8" no yes ;;
  esac
  rm -rf "$r/pgb-locale-plain" "$r/pgb-locale-locpath" "$r/opt/pgb-locale"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

if [ -n "$HOST_LOCALE_SRC" ]; then
  exp_note "C.utf8 costs $(du -sk "$HOST_LOCALE_SRC" | cut -f1) KiB, of which LC_CTYPE is nearly all"
fi

exp_finish
