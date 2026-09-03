#!/bin/sh
# THE QUESTION
#
#   What ELSE does glibc read from the host that a static link does not
#   absorb? Not "are the ten closed" — is TEN the number?
#
# ⛔ WHY THIS EXPERIMENT EXISTS, AND IT IS THE SECOND TIME THIS QUESTION HAS
# BEEN ASKED. `docs/REQUIREMENTS.md` enumerated NINE ways static glibc is not
# self-contained and said *"there is no unenumerated remainder"*. A TENTH — the
# timezone database — was found the next day by somebody asking about
# COMPLETENESS rather than about the nine. `grep -rn zoneinfo` over the whole
# tree returned nothing; nobody had looked. `experiments/97-`, T-076.
#
# ⭐ THE LIST IS NOW TEN AND NINE ARE CLOSED. ⛔ That is not evidence that ten
# is the number, and T-079 exists to say so with a SEARCH rather than a
# sentence.
#
# -- WHY A SEARCH AND NOT A LIST --------------------------------------------
#
# ⭐ THE TENTH WAS FOUND BY `strings` ON `libc.a`. That is the generalisable
# move, and this experiment is that move applied to the WHOLE archive instead
# of to one guessed keyword: enumerate every absolute path the PINNED `libc.a`
# names, classify each against the ten, and print the RESIDUE.
#
# ⛔ THE ARCHIVE IS THE RIGHT ORACLE because it is what gets linked IN. A
# document can be out of date, a `grep` over this tree can only find what
# somebody already thought of, and the host's own libc is the wrong version.
# The pinned archive is the code that will be in the binary.
#
# ⚠ AND AN ABSENCE HERE IS NOT A ZERO. This search sees:
#     - absolute-path STRING LITERALS in the pinned libc.a, and
#     - the environment-variable NAMES it carries.
#   It does NOT see:
#     - paths assembled at run time from `%s/%s` and a variable — the archive
#       carries exactly one such literal and it is the generic join, so a
#       path with no literal ROOT is invisible here. ⭐ This is not
#       hypothetical: it is why `/usr/share/zoneinfo` was findable and is
#       the reason to keep asking.
#     - host data belonging to OTHER static libraries. ⛔ TWO OF THE TEN —
#       terminfo (ncurses) and the CA bundle (OpenSSL) — are invisible to
#       this search BY CONSTRUCTION, because they are not glibc's. A clean
#       residue here says nothing about them. §4 names where to look instead.
#     - anything reached through a host DAEMON rather than a file.
#
# -- WHAT IS MEASURED, in the three-fact order `experiments/97-` established --
#
#   1. THE SEARCH. Every absolute path in the pinned libc.a, classified
#      against the ten. A `strings` question, answered on the archive.
#   2. PRESENCE. Which of the eleven environments actually ship each residue
#      path. A directory question, answered on the rootfs trees.
#   3. THE CONSEQUENCE. What one static binary calling the libc functions
#      that read those files PRINTS on each of the eleven — the only one of
#      the three that shows whether any of it matters.
#
# -- ⭐ PRE-REGISTERED EXPECTATION -------------------------------------------
#
# ⛔ WRITTEN BEFORE THE RUN AND COMMITTED BEFORE THE RUN, so that `git log`
# can show it was not written afterwards. `TODO/PROGRESS.md` delivery rule 1.
#
#   P1  The search finds ~78 absolute paths in the pinned libc.a.
#   P2  After classification, the residue — host DATA files that are not
#       covered by any of the ten and are not kernel/device interfaces —
#       is NON-EMPTY. I expect `/etc/services`, `/etc/protocols` and
#       `/etc/rpc` to be in it.
#   P3  ⭐ THE CONSEQUENCE ROW, AND THIS IS THE FALSIFIABLE ONE:
#       `getservbyname("http","tcp")` returns 80 where `/etc/services`
#       exists and NULL where it does not, and it does NOT exist on
#       ⛔ **debian-11, debian-12 and ubuntu-20.04 — 3 of 11, ALL GLIBC**.
#       So I predict FAIL on exactly 3 rows, and that all three are glibc.
#   P4  `getprotobyname("tcp")` returns 6 and fails on the SAME three.
#   P5  ⚠ I expect this to be a genuine ELEVENTH row rather than a
#       restatement of NSS. NSS is closed by `__nss_configure_lookup`
#       pinning the DISPATCH to `files`; that fix cannot conjure a `files`
#       backing store that is not on the host. Dispatch and data are two
#       different failures and only one of them is closed.
#
# ⚠ P3 IS THE INVERSE OF THE INTUITIVE DIRECTION and that is why it is worth
# running: the four musl environments all ship `/etc/services` and three
# glibc ones do not. If the run contradicts it, the run wins.
#
# -- ⚠ THE HONEST SCOPE, stated the way the timezone row had to state it -----
#
# A DYNAMIC binary on the same host cannot read `/etc/services` either — the
# file is genuinely absent. What makes that a row of the ten rather than a
# fact about Debian is the precedent `--embed-terminfo`, `--embed-cacert` and
# `--embed-tzdata` set: the promise is one ordinary ELF that works
# everywhere, and this is data glibc needs, does not carry, and fails without.
# ⛔ Whether it fails SILENTLY is measured below, not assumed.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "82 - what else does glibc read from the host: the search, not the list"

WORK="${PGB_EXP82_WORK:-/var/tmp/pgb-exp82}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
CC="${CC:-cc}"
command -v "$CC" >/dev/null 2>&1 || { exp_note "no $CC on PATH"; exit 2; }

# ---------------------------------------------------------------------------
# 1. THE SEARCH — every absolute path the PINNED libc.a names.
#
# ⛔ THE PINNED ARCHIVE, NOT THE HOST'S. The host here is glibc 2.39 and the
# pin is 2.41; a search over the host's archive would describe a libc this
# project does not link. lib.sh resolves ENV_ROOT from cfg.go for exactly
# this reason.
# ---------------------------------------------------------------------------
printf -- '-- 1. the search: absolute paths in the pinned libc.a ---------------\n'

LIBC_A=""
for p in "$ENV_ROOT"/usr/lib/x86_64-linux-gnu/libc.a "$ENV_ROOT"/usr/lib64/libc.a \
         "$ENV_ROOT"/usr/lib/libc.a; do
  [ -e "$p" ] && { LIBC_A="$p"; break; }
done
if [ -z "$LIBC_A" ]; then
  exp_note "no libc.a under $ENV_ROOT — the pinned environment is not built"
  exp_note "run: ./pgb bootstrap --detach"
  exit 2
fi
exp_note "archive : $LIBC_A"
exp_note "glibc   : $(strings -a "$LIBC_A" | grep -oE 'stable release version [0-9]+\.[0-9]+' | head -1)"

# ⚠ THE PATTERN IS DELIBERATELY OVER-INCLUSIVE. It matches any absolute path
# under a top-level directory that exists on a Linux system, and the
# classification below is what narrows it. An under-inclusive pattern would
# reproduce the original failure — finding only what was already thought of.
ALL="$WORK/paths.txt"
strings -a "$LIBC_A" \
  | grep -oE '(^|[^a-zA-Z0-9_./-])(/(etc|usr|var|lib|lib64|proc|sys|dev|run|tmp|bin|sbin|opt)(/[A-Za-z0-9_.+-]+)*)' \
  | sed 's/^[^/]*//' | sort -u > "$ALL"
NPATHS=$(wc -l < "$ALL" | tr -d ' ')
exp_note "absolute paths found: $NPATHS"

# ⭐ THE CLASSIFICATION IS THE ARGUMENT, so it is written out rather than
# hidden in a grep. Each bucket names the row of the ten that owns it, or
# says why the path is not host DATA at all.
#
#   covered-N   owned by row N of the ten in docs/REQUIREMENTS.md
#   kernel      /proc, /sys, /dev — a kernel interface, present on any Linux
#               host by construction, not host DATA that can be absent
#   toolchain   a path only the dynamic loader or a build tool uses
#   RESIDUE     ⛔ none of the above: host data, unowned
classify() {  # path -> bucket
  case "$1" in
    /etc/nsswitch.conf|/etc/passwd|/etc/group|/etc/shadow|/etc/gshadow|\
/etc/hosts|/etc/netgroup|/etc/.pwd.lock|/etc/hosts.equiv)
        printf 'covered-NSS' ;;
    /usr/lib/*gconv*|/usr/lib/x86_64-linux-gnu/gconv*)
        printf 'covered-gconv' ;;
    /usr/lib/locale|/usr/lib/locale/*|/usr/share/locale)
        printf 'covered-locale' ;;
    /etc/resolv.conf|/etc/host.conf|/etc/gai.conf)
        printf 'covered-dns' ;;
    /etc/localtime|/usr/share/zoneinfo)
        printf 'covered-tz' ;;
    /etc/ld.so.cache|/lib|/lib/*|/usr/lib|/usr/bin|/bin|/tmp|/dev|/usr/lib/x86_64-linux-gnu)
        printf 'toolchain' ;;
    /proc/*|/sys/*|/dev/*)
        printf 'kernel' ;;
    *)  printf 'RESIDUE' ;;
  esac
}

RESIDUE="$WORK/residue.txt"; : > "$RESIDUE"
printf '\n  %-34s %s\n' PATH BUCKET
while IFS= read -r p; do
  b=$(classify "$p")
  [ "$b" = RESIDUE ] && printf '%s\n' "$p" >> "$RESIDUE"
  printf '  %-34s %s\n' "$p" "$b"
done < "$ALL"

NRES=$(wc -l < "$RESIDUE" | tr -d ' ')
printf '\n'
exp_note "⛔ RESIDUE: $NRES paths owned by NO row of the ten"

# ⛔ AN EMPTY RESIDUE WOULD BE A RESULT, NOT A PASS. Assert it is non-empty so
# that a classification bug which swallowed everything cannot read as "the
# list is complete" — the exact shape of the failure this experiment exists
# to prevent.
exp_check "the search found paths at all"          "$([ "$NPATHS" -gt 40 ] && echo yes || echo no)" yes
exp_check "the residue is non-empty (P2)"          "$([ "$NRES" -gt 0 ] && echo yes || echo no)" yes

# ---------------------------------------------------------------------------
# 2. PRESENCE — which of the eleven ship each residue path.
# ---------------------------------------------------------------------------
printf '\n'
printf -- '-- 2. presence of each residue path across the eleven ---------------\n'

ENVS=$(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt")
NENV=0; for n in $ENVS; do [ -n "$(exp_rootfs "$n")" ] && NENV=$((NENV+1)); done
exp_note "environments fetched: $NENV"

printf '\n  %-24s %s\n' PATH 'PRESENT ON'
SERVICES_MISSING=0; PROTOCOLS_MISSING=0
while IFS= read -r p; do
  have=0; miss=""
  for n in $ENVS; do
    r=$(exp_rootfs "$n"); [ -n "$r" ] || continue
    if [ -e "$r$p" ]; then have=$((have+1)); else miss="$miss $n"; fi
  done
  [ "$p" = /etc/services ]  && SERVICES_MISSING=$((NENV-have))
  [ "$p" = /etc/protocols ] && PROTOCOLS_MISSING=$((NENV-have))
  printf '  %-24s %2s of %-3s%s\n' "$p" "$have" "$NENV" \
    "$([ -n "$miss" ] && printf '  absent:%s' "$miss")"
done < "$RESIDUE"

# ---------------------------------------------------------------------------
# 3. THE CONSEQUENCE — one static binary, the eleven, and what it prints.
#
# ⭐ THIS IS THE ONLY PART THAT SHOWS WHETHER ANY OF IT MATTERS. Sections 1
# and 2 are a file listing; this is a program failing.
#
# ⛔ IT ALSO ANSWERS "does it fail SILENTLY", which is what made gconv and
# timezone worse than a missing feature. The probe prints the RETURNED VALUE,
# so a wrong answer and a refusal are distinguishable in the output.
# ---------------------------------------------------------------------------
printf '\n'
printf -- '-- 3. the consequence: getservbyname / getprotobyname on the eleven --\n'

cat > "$WORK/svc.c" <<'EOF'
/* Ask glibc the three questions that read the residue files, and print what
   it answered rather than whether it succeeded — a silent wrong answer and a
   refusal must be distinguishable. */
#include <stdio.h>
#include <netdb.h>
int main(void) {
  struct servent  *s = getservbyname("http", "tcp");
  struct protoent *p = getprotobyname("tcp");
  printf("http/tcp=%s proto-tcp=%s\n",
         s ? "80" : "NULL",
         p ? "6"  : "NULL");
  if (s && ntohs((unsigned short)s->s_port) != 80) printf("WRONG-PORT\n");
  if (p && p->p_proto != 6)                        printf("WRONG-PROTO\n");
  return 0;
}
EOF

# ⚠ Built with plain `cc -static`, because the question is what a VANILLA
# static glibc binary does. `pgb`'s mechanisms are measured against it in
# `experiments/63-`; here the baseline is the subject.
if ! "$CC" -static -O2 -o "$WORK/svc" "$WORK/svc.c" 2>"$WORK/cc.log"; then
  exp_skip "the consequence probe built" "$(tail -1 "$WORK/cc.log")"
  exp_finish
fi

printf '\n  %-20s %-6s %-9s %-9s %s\n' ENVIRONMENT LIBC SERVICES PRINTED VERDICT
S_OK=0; S_BAD=0; S_ROWS=0; S_BAD_GLIBC=0
for name in $ENVS; do
  r=$(exp_rootfs "$name") || true
  [ -n "$r" ] || continue
  S_ROWS=$((S_ROWS+1))
  libc=$(exp_rootfs_libc "$name")
  has=$( [ -e "$r/etc/services" ] && echo yes || echo no )
  out=$("$REPO_DIR/pgb" rootfs run "$r" --copy "$WORK/svc:/svc" -- /svc 2>/dev/null | tr -d '\r' | head -1)
  case "$out" in
    *"http/tcp=80"*) v="ok"; S_OK=$((S_OK+1)) ;;
    *"http/tcp=NULL"*) v="⛔ CANNOT RESOLVE"; S_BAD=$((S_BAD+1))
                       [ "$libc" = glibc ] && S_BAD_GLIBC=$((S_BAD_GLIBC+1)) ;;
    *) v="⛔ NO OUTPUT" ;;
  esac
  printf '  %-20s %-6s %-9s %-9s %s\n' "$name" "$libc" "$has" "${out:-<none>}" "$v"
done

printf '\n'
exp_check "every fetched environment answered"        "$((S_OK+S_BAD))" "$S_ROWS"
exp_check "environments that CANNOT resolve http/tcp (P3)" "$S_BAD" 3
exp_check "and all of them are GLIBC rows (P3)"       "$S_BAD_GLIBC" "$S_BAD"
exp_check "the file listing agrees with the run"      "$SERVICES_MISSING" "$S_BAD"

exp_note "⛔ THE ELEVENTH ROW. /etc/services is host data glibc reads, does"
exp_note "   not carry, and none of the ten covers. NSS is closed for"
exp_note "   DISPATCH — __nss_configure_lookup pins the services database to"
exp_note "   \`files\` — and \`files\` still means /etc/services, which $S_BAD of"
exp_note "   $S_ROWS environments do not have."
exp_note "⚠ AND IT IS NOT A MUSL STORY, it is the reverse: all four musl"
exp_note "   environments SHIP the file and $S_BAD_GLIBC glibc ones do not."
exp_note "⚠ The failure is a NULL return rather than a wrong number, so it is"
exp_note "   louder than gconv's and timezone's. A caller that checks the"
exp_note "   return value sees it; one that does not dereferences NULL."

exp_finish
