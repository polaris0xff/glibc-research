#!/bin/sh
# 91 - what moving the glibc pin would buy, and what it would cost.
#
# -- THE QUESTION -----------------------------------------------------------
#
# `internal/cfg/cfg.go` pins the build environment to debian:12, glibc 2.36.
# `docs/design/glibc-versions.md` establishes that the pin is a FLOOR and not
# a ceiling -- the output is static, `PT_INTERP=0 DT_NEEDED=0`, so the usual
# "build old so it runs on old hosts" pressure does not exist here at all --
# and that the floor is 2.34, set by glibc building the `files` and `dns` NSS
# services into libc.
#
# ⛔ SO THE PIN SITS TWO RELEASES ABOVE ITS OWN FLOOR FOR NO MEASURED REASON,
# AND SOMETHING ELSE IS PUSHING THE OTHER WAY. `experiments/73-`'s class B is
# what a HOST shared object imports at a GLIBC_ version NEWER than the pin --
# 20 symbols, 14 of them the `__isoc23_*` family at exactly GLIBC_2.38. ⭐ It
# is the only quantity in this project that gets worse purely by the passage
# of time: every glibc release the pin does not follow widens it.
#
# T-070 is therefore a MEASUREMENT and not a change:
#
#     what does a newer pin buy, and what does it cost?
#
# -- ⛔ THE COST IS THE INTERESTING HALF, AND IT HAS A NAME ------------------
#
# A static binary's only host requirement is the KERNEL. `file` reports our
# output as `for GNU/Linux 3.2.0`, and that number comes from the .note.ABI-tag
# glibc's own crt files put there -- so it is set by the BUILD glibc, not by
# anything this project chooses. ⛔ If a newer glibc declares a higher minimum
# kernel, moving the pin trades a real portability property for a
# symbol-coverage one, and that trade has to be stated rather than discovered
# by somebody whose kernel is too old.
#
# ⭐ SO THE VETO IS MEASURED FIRST AND CHEAPLY. Arm 2 needs a compiler and
# nothing else, runs in a throwaway container, and takes seconds. Only a
# candidate that survives it is worth a full build environment, which costs
# gigabytes and minutes. An experiment that spends the expensive resource
# before the cheap veto is answering the questions in the wrong order.
#
# -- ARMS -------------------------------------------------------------------
#
#   1  the candidates, resolved to manifest digests.
#      ⭐ WITH A CONTROL THAT CAN FAIL: the same method is run against
#      debian:12, whose digest is already pinned in two places in this tree.
#      If it does not reproduce that digest, the method is wrong and every
#      other digest it produced is worthless.
#
#   2  ⛔ THE KERNEL FLOOR -- the veto. A static binary built in each
#      candidate, read TWO ways: `readelf -n` on .note.ABI-tag, which is the
#      authoritative encoding, and `file`, which is what a user would run.
#      Two instruments that can disagree.
#
#   3  the NSS floor still holds. The `experiments/21-` probe, built in the
#      candidate and run on a target that really ships libnss_files.so.2.
#      ⛔ A pin that loses this is not a candidate at any price: it is the
#      reason the pin exists.
#
#   4  class B bought and class C paid, by re-running `experiments/73-`
#      against the candidate environment. ⭐ 73- already honours PGB_ENV_NAME,
#      so this arm runs the project's own instrument rather than a second
#      implementation of its classifier -- `docs/AGENTS.md` §14.
#
#   5  the ten POCs still build against the candidate. ⚠ EXPENSIVE, and gated:
#      it runs only for a candidate that survived arms 2 and 3, and it records
#      which POCs ran rather than claiming all of them.
#
# -- WHAT THIS CANNOT SETTLE ------------------------------------------------
#
# ⛔ It does not move the pin. The output is a row per candidate and a ruling
# written into the entry; changing `cfg.go` is a separate commit gated on the
# full POC matrix and CI.
#
# ⚠ Candidates are Debian- and Ubuntu-family only, because arms 2, 3 and 5
# install packages with apt. A candidate from another family would need its
# own installer line and is out of scope rather than impossible.
#
# ⚠ The kernel floor read here is the one the BUILD glibc's crt files declare.
# It is not a claim that the binary runs on that kernel -- no environment in
# this bed runs a kernel that old, and `docs/research/architecture.md` records
# the same limit for arm64.
#
# Exit: 0 the measurement ran and matched, 1 it ran and did not, 2 could not
#       run.
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "91 - what moving the glibc pin would buy, and what it would cost"

WORK="$EXP_OUT/build"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
RESULT="$EXP_OUT/RESULT.txt"

# The incumbent, from cfg.go. ⛔ Read out of the source rather than repeated
# here: a copy would drift the moment the pin moves, and this experiment
# exists precisely for the session that moves it.
CFG="$REPO_DIR/internal/cfg/cfg.go"
PIN_IMAGE=$(sed -n 's/.*DefaultEnvImage  *= *"\([^"]*\)".*/\1/p'  "$CFG" | head -1)
PIN_DIGEST=$(sed -n 's/.*DefaultEnvDigest *= *"\([^"]*\)".*/\1/p' "$CFG" | head -1)
PIN_NAME=$(sed -n 's/.*DefaultEnvName   *= *"\([^"]*\)".*/\1/p'   "$CFG" | head -1)
[ -n "$PIN_IMAGE" ] && [ -n "$PIN_DIGEST" ] || {
  exp_skip "the incumbent pin" "could not be read out of $CFG"; exp_finish; }
exp_note "incumbent pin: $PIN_IMAGE $PIN_DIGEST"
exp_note "               env $PIN_NAME"

# The candidates. The incumbent is first and is the control everywhere.
#
# ⚠ `debian:trixie` RATHER THAN `debian:13`, and the reason is recorded because
# it looks arbitrary: they name the same image, and on 2026-09-02 the registry
# answered `debian:13` with `429 Too Many Requests` while answering
# `debian:trixie` normally, repeatedly. The codename is the tag Debian's own
# documentation uses; the number is an alias. If `debian:13` resolves for you,
# nothing here depends on which of the two you pass.
CANDIDATES="${PGB_PIN_CANDIDATES:-$PIN_IMAGE debian:trixie ubuntu:24.04}"
exp_note "candidates: $CANDIDATES"
printf '\n'

command -v docker >/dev/null 2>&1 || { exp_skip "docker" "absent"; exp_finish; }
docker info >/dev/null 2>&1       || { exp_skip "dockerd" "not running"; exp_finish; }

tagof() { printf '%s' "$1" | tr ':/' '--'; }

# ===========================================================================
# ARM 1 -- the candidates, resolved to manifest digests
# ===========================================================================
printf -- '-- arm 1: the candidates, resolved to manifest digests -----------\n'

# ⛔ THE DIGEST THIS REPOSITORY PINS IS THE PER-PLATFORM MANIFEST DIGEST, NOT
# THE INDEX DIGEST, and one command prints both side by side. For debian:12 the
# OCI index is sha256:6ebd97fa… and the linux/amd64 manifest inside it is
# sha256:2f65600e… -- and 2f65600e is what `cfg.go` and `rootfs-images.txt`
# carry. `rootfs-images.txt` says so in its arm64 note: "the digests ... are
# amd64-specific."
#
# ⚠ SO THE OBVIOUS METHOD RETURNS THE WRONG ONE OF THE TWO. `docker pull` of a
# tag records the INDEX digest in RepoDigests, which is a real digest, resolves
# on any machine, and is not the number this tree pins. ⭐ The control below is
# what found that, and it is the reason it exists: a digest resolver with no
# control produces a plausible sha256 either way.
case "$(uname -m)" in
  x86_64)  DOCKER_ARCH=amd64 ;;
  aarch64) DOCKER_ARCH=arm64 ;;
  *)       DOCKER_ARCH=$(uname -m) ;;
esac
#
# ⛔ AND IT KEEPS ITS STDERR. Docker Hub rate-limits anonymous requests, and a
# resolver that redirected stderr to /dev/null reported `debian:13` as
# "unresolved" when what the registry actually said was
# `429 Too Many Requests`. ⚠ That is `docs/AGENTS.md` §0b's "an absence is not
# a zero" in one line: the two readings look identical in the table and mean
# opposite things -- one is a tag that does not exist, the other is a tag
# nobody asked for properly.
digest_of() {  # image errfile -> the linux/$DOCKER_ARCH manifest digest, no pull
  docker buildx imagetools inspect "$1" 2>"$2" \
    | awk -v want="linux/$DOCKER_ARCH" '
        /^ *Name: .*@sha256:/ { d = $2; sub(/.*@/, "", d) }
        /^ *Platform:/ && $2 == want && d != "" { print d; exit }'
}

: > "$WORK/candidates.txt"
printf '  %-16s %s\n' IMAGE DIGEST
for img in $CANDIDATES; do
  e="$WORK/digest-$(tagof "$img").err"
  d=$(digest_of "$img" "$e" || true)
  if [ -n "${d:-}" ]; then
    printf '  %-16s %s\n' "$img" "$d"
    printf '%s %s\n' "$img" "$d" >> "$WORK/candidates.txt"
  else
    # ⛔ The reason, not just the absence. A rate limit and a missing tag are
    # both "no digest" and only one of them is about the candidate.
    printf '  %-16s %s\n' "$img" "unresolved: $(head -1 "$e" 2>/dev/null)"
  fi
done

# ⭐ THE CONTROL. The method must reproduce the digest this repository already
# pins for the incumbent. If it does not, arm 1 measured something else and
# every digest it produced is worthless.
GOT_PIN=$(awk -v i="$PIN_IMAGE" '$1==i{print $2}' "$WORK/candidates.txt")
exp_check "the method reproduces the pinned digest for $PIN_IMAGE" \
  "${GOT_PIN:-none}" "$PIN_DIGEST"
printf '\n'

# ===========================================================================
# ARM 2 -- ⛔ the kernel floor: the veto
# ===========================================================================
printf -- '-- arm 2: the kernel floor a static binary declares (THE VETO) ---\n'

# The probe is the smallest program that still drags in glibc's crt files,
# which is where .note.ABI-tag comes from.
printf 'int main(void){return 0;}\n' > "$WORK/floor.c"

# ⚠ Everything the container needs is installed inside it, so the reading is
# the CANDIDATE's toolchain and not the host's. `binutils` for readelf, `file`
# for the second opinion.
floor_probe() {  # image outdir -> writes glibc.txt readelf.txt file.txt
  _img="$1"; _o="$2"
  mkdir -p "$_o"
  docker run --rm -i -v "$_o:/out" -v "$WORK/floor.c:/floor.c:ro" "$_img" \
    /bin/sh -c '
      export DEBIAN_FRONTEND=noninteractive
      apt-get update -qq >/dev/null 2>&1 &&
      apt-get install -y -qq --no-install-recommends \
        gcc libc6-dev binutils file >/dev/null 2>&1 || exit 3
      ldd --version 2>&1 | head -1        > /out/glibc.txt
      gcc -static -O2 -o /out/static /floor.c || exit 4
      readelf -n /out/static 2>/dev/null  > /out/readelf.txt
      file    /out/static    2>/dev/null  > /out/file.txt
      gcc -dumpmachine                    > /out/triple.txt 2>/dev/null
      gcc --version 2>&1 | head -1        > /out/gcc.txt
    ' >"$_o/docker.log" 2>&1
}

# ⭐ Two readings of the same fact. `readelf -n` prints the note glibc encoded;
# `file` prints its own interpretation of it. They are recorded separately so
# a disagreement is visible rather than averaged.
floor_readelf() { sed -n 's/.*OS: Linux, ABI: \([0-9.]*\).*/\1/p' "$1/readelf.txt" 2>/dev/null | head -1; }
floor_file()    { sed -n 's/.*for GNU\/Linux \([0-9.]*\).*/\1/p'  "$1/file.txt"    2>/dev/null | head -1; }

: > "$WORK/floors.txt"
printf '  %-16s %-10s %-12s %-12s %s\n' IMAGE GLIBC 'ABI-tag' 'file(1)' AGREE
while read -r img _d; do
  [ -n "$img" ] || continue
  o="$WORK/floor-$(tagof "$img")"
  floor_probe "$img" "$o"
  g=$(sed -n 's/.*) \([0-9][0-9.]*\)$/\1/p' "$o/glibc.txt" 2>/dev/null | head -1)
  [ -n "$g" ] || g=$(tr -dc '0-9.\n' < "$o/glibc.txt" 2>/dev/null | tail -1)
  fr=$(floor_readelf "$o"); ff=$(floor_file "$o")
  agree=no; [ -n "$fr" ] && [ "$fr" = "$ff" ] && agree=yes
  printf '  %-16s %-10s %-12s %-12s %s\n' \
    "$img" "${g:-?}" "${fr:-unread}" "${ff:-unread}" "$agree"
  printf '%s %s %s %s\n' "$img" "${g:-?}" "${fr:-unread}" "${ff:-unread}" >> "$WORK/floors.txt"
done < "$WORK/candidates.txt"

PIN_FLOOR=$(awk -v i="$PIN_IMAGE" '$1==i{print $3}' "$WORK/floors.txt")
exp_check "the incumbent's floor was read at all" \
  "$([ -n "${PIN_FLOOR:-}" ] && [ "$PIN_FLOOR" != unread ] && echo yes || echo no)" yes

# ⛔ REPORTED, NOT ASSERTED. Whichever way a candidate's floor lands is the
# finding; asserting "no candidate raises it" would turn a measurement into a
# confirmation, and `docs/methodology/experiments.md` forbids that.
printf '\n'
RAISED=""
while read -r img g fr ff; do
  [ "$img" = "$PIN_IMAGE" ] && continue
  case "$fr" in unread|'') exp_note "$img: floor UNREAD -- treat as unmeasured, not as equal"; continue ;; esac
  if [ "$fr" = "$PIN_FLOOR" ]; then
    exp_note "$img (glibc $g): kernel floor $fr -- SAME as the incumbent, no cost here"
  else
    exp_note "⛔ $img (glibc $g): kernel floor $fr against the incumbent's $PIN_FLOOR"
    RAISED="$RAISED $img"
  fi
done < "$WORK/floors.txt"
[ -n "$RAISED" ] && exp_note "candidates that MOVE the floor:$RAISED"
printf '\n'

# ===========================================================================
# ARM 3 -- the NSS floor still holds
# ===========================================================================
printf -- '-- arm 3: does the NSS override still remove the dlopen ----------\n'

# ⭐ The probe is experiments/21-'s, unchanged, because the question is
# identical and a second copy of it would be a second thing to keep right.
cat > "$WORK/nss.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <string.h>
#include <netdb.h>
#include <pwd.h>
#include <sys/socket.h>
extern int __nss_configure_lookup(const char *db, const char *line);
__attribute__((constructor(101)))
static void nssfix(void) {
    static const char *const f[] = {"passwd","group","shadow","gshadow","aliases",
        "ethers","initgroups","netgroup","networks","protocols","publickey",
        "rpc","services", NULL};
    for (const char *const *d = f; *d; d++) __nss_configure_lookup(*d, "files");
    __nss_configure_lookup("hosts", "files dns");
}
int main(void) {
    struct addrinfo h, *r;
    memset(&h, 0, sizeof h); h.ai_socktype = SOCK_STREAM;
    int s = getaddrinfo("example.com", "80", &h, &r);
    if (s == 0) freeaddrinfo(r);
    struct passwd *pw = getpwuid(0);
    printf("getaddrinfo=%d getpwuid=%s\n", s, pw ? pw->pw_name : "(null)");
    return 0;
}
EOF

# ⛔ THE TARGET MUST REALLY SHIP libnss_files.so.2, or a binary that tried to
# open one would fail to find it and the trace would look deceptively clean.
# 21- uses debian-11 for exactly this reason.
TARGET=$(exp_rootfs debian-11)
if [ -z "$TARGET" ]; then
  exp_skip "arm 3" "the debian-11 target rootfs is absent; run: pgb rootfs fetch debian-11"
else
  exp_check "the target ships libnss_files.so.2" \
    "$(ls "$TARGET"/lib/*/libnss_files.so.2 >/dev/null 2>&1 && echo yes || echo no)" yes
  printf '\n  %-16s %-6s %s\n' 'BUILD glibc' EXIT 'HOST NSS MODULES OPENED'
  while read -r img _g _fr _ff; do
    o="$WORK/nss-$(tagof "$img")"; mkdir -p "$o"
    docker run --rm -v "$o:/out" -v "$WORK/nss.c:/nss.c:ro" "$img" \
      /bin/sh -c '
        export DEBIAN_FRONTEND=noninteractive
        apt-get update -qq >/dev/null 2>&1 &&
        apt-get install -y -qq --no-install-recommends gcc libc6-dev >/dev/null 2>&1 || exit 3
        gcc -static -O2 -o /out/nssfix /nss.c
      ' >"$o/docker.log" 2>&1
    if [ ! -s "$o/nssfix" ]; then
      exp_skip "nss probe for $img" "it did not build; see $o/docker.log"
      continue
    fi
    cp "$o/nssfix" "$TARGET/exp91-nss"
    "$REPO_DIR/pgb" rootfs run "$TARGET" -- /exp91-nss >/dev/null 2>&1
    st=$?
    mods=$(exp_trace_opens "$TARGET" /exp91-nss "$o/trace.txt" \
           | grep -oE 'libnss_[a-z0-9_]*\.so[^"]*' | sort -u | tr '\n' ',' | sed 's/,$//')
    rm -f "$TARGET/exp91-nss"
    printf '  %-16s %-6s %s\n' "$img" "$st" "${mods:-none}"
    printf '%s %s\n' "$img" "${mods:-none}" >> "$WORK/nss.txt"
    # ⛔ THIS ONE IS ASSERTED. The pin exists for this property; a candidate
    # that loses it is not a candidate at any price.
    exp_check "$img + override loads no NSS module" "${mods:-none}" none
  done < "$WORK/floors.txt"
fi
printf '\n'

# ===========================================================================
# ARM 4 -- class B bought, class C paid
# ===========================================================================
printf -- '-- arm 4: what the move buys (class B) and costs (class C) -------\n'
# ⭐ RUNS 73- RATHER THAN REIMPLEMENTING ITS CLASSIFIER. 73- reads
# PGB_ENV_NAME, so a candidate environment is measured by the project's own
# instrument, with its own controls, on its own evidence path.
#
# ⚠ The environment has to exist first, and creating one is the expensive
# step this experiment deliberately defers until a candidate has survived the
# veto. PGB_PIN_ENVS names the candidates to build environments for; empty
# means "only report what is already on disk".
ENVS="${PGB_PIN_ENVS:-}"
if [ -z "$ENVS" ]; then
  exp_note "PGB_PIN_ENVS is empty: no build environment is created here."
  exp_note "arm 4 reports the environments already on disk and nothing else."
fi
for img in $ENVS; do
  en="pgb-env-$(tagof "$img")"
  [ -d "$ROOTFS_DIR/$en" ] && { exp_note "$en already exists"; continue; }
  d=$(awk -v i="$img" '$1==i{print $2}' "$WORK/candidates.txt")
  exp_note "creating the build environment $en for $img (this is the expensive step)"
  PGB_ENV_IMAGE="$img" PGB_ENV_DIGEST="$d" PGB_ENV_NAME="$en" \
    "$REPO_DIR/pgb" env create >"$WORK/env-$en.log" 2>&1 \
    || exp_note "⚠ $en did not create; see $WORK/env-$en.log"
done

: > "$WORK/classes.txt"
printf '  %-22s %-8s %-8s %-8s %-8s\n' ENVIRONMENT GLIBC 'CLASS B' 'CLASS C' 'CLASS S'
for img in $CANDIDATES; do
  en="pgb-env-$(tagof "$img")"
  [ "$img" = "$PIN_IMAGE" ] && en="$PIN_NAME"
  if [ ! -d "$ROOTFS_DIR/$en" ]; then
    printf '  %-22s %-8s %-8s %-8s %-8s\n' "$en" - 'not run' 'not run' 'not run'
    continue
  fi
  # 73- writes its own RESULT.txt under evidence/73-.../; it is moved aside per
  # environment so two runs cannot overwrite one another.
  PGB_ENV_NAME="$en" sh "$REPO_DIR/experiments/73-host-dso-abi-demand.sh" \
    >"$WORK/73-$en.log" 2>&1
  cp -f "$OUT_DIR/73-host-dso-abi-demand/RESULT.txt" "$WORK/73-$en.RESULT.txt" 2>/dev/null
  g=$(sed -n 's/^pinned glibc *: *\([0-9.]*\).*/\1/p' "$WORK/73-$en.log" | head -1)
  [ -n "$g" ] || g=$(sed -n 's/.*pinned glibc: \([0-9.]*\).*/\1/p' "$WORK/73-$en.log" | head -1)
  cb=$(awk '/CLASS B/{f=1;next} /CLASS [CSE]/{f=0} f&&/^ /{n++} END{print n+0}' "$WORK/73-$en.RESULT.txt" 2>/dev/null)
  cc=$(awk '/CLASS C/{f=1;next} /CLASS [SE]/{f=0} f&&/^ /{n++} END{print n+0}' "$WORK/73-$en.RESULT.txt" 2>/dev/null)
  cs=$(awk '/CLASS S/{f=1;next} /CLASS E/{f=0} f&&/^ /{n++} END{print n+0}' "$WORK/73-$en.RESULT.txt" 2>/dev/null)
  printf '  %-22s %-8s %-8s %-8s %-8s\n' "$en" "${g:-?}" "${cb:-?}" "${cc:-?}" "${cs:-?}"
  printf '%s %s %s %s %s\n' "$en" "${g:-?}" "${cb:-?}" "${cc:-?}" "${cs:-?}" >> "$WORK/classes.txt"
done
exp_check "at least the incumbent environment was classified" \
  "$([ -s "$WORK/classes.txt" ] && echo yes || echo no)" yes
printf '\n'

# ===========================================================================
# ARM 5 -- the POCs still build
# ===========================================================================
printf -- '-- arm 5: the POCs against a candidate environment ---------------\n'
# ⚠ GATED AND ACCOUNTED. PGB_PIN_POCS names which POCs to build; empty means
# none ran, and "none ran" is printed rather than passed over. ⛔ A newer glibc
# deprecates as well as adds, so this arm is what stands between a measurement
# and a pin move.
POCS="${PGB_PIN_POCS:-}"
POC_ENV="${PGB_PIN_POC_ENV:-}"
if [ -z "$POCS" ] || [ -z "$POC_ENV" ]; then
  exp_note "not run: PGB_PIN_POCS and PGB_PIN_POC_ENV are what turn this arm on."
  exp_note "⛔ that is 'could not run', not 'passed' -- the ruling says so."
else
  printf '  %-14s %s\n' POC OUTCOME
  for p in $POCS; do
    if [ ! -x "$REPO_DIR/poc/$p/run.sh" ] && [ ! -f "$REPO_DIR/poc/$p/run.sh" ]; then
      printf '  %-14s %s\n' "$p" "absent"
      continue
    fi
    if PGB_ENV_NAME="$POC_ENV" sh "$REPO_DIR/poc/$p/run.sh" >"$WORK/poc-$p.log" 2>&1; then
      printf '  %-14s %s\n' "$p" "ok"
      printf '%s ok\n' "$p" >> "$WORK/pocs.txt"
    else
      printf '  %-14s %s\n' "$p" "exit $? -- see $WORK/poc-$p.log"
      printf '%s failed\n' "$p" >> "$WORK/pocs.txt"
    fi
  done
  exp_check "every POC asked for built against $POC_ENV" \
    "$(grep -c ' failed$' "$WORK/pocs.txt" 2>/dev/null || echo 0)" 0
fi
printf '\n'

# ===========================================================================
# RESULT
# ===========================================================================
{
  printf '91 - glibc pin candidates\n'
  printf 'date        : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'incumbent   : %s %s\n\n' "$PIN_IMAGE" "$PIN_DIGEST"
  printf 'ARM 1 -- digests\n'
  sed 's/^/  /' "$WORK/candidates.txt" 2>/dev/null
  printf '\nARM 2 -- kernel floor (image, glibc, ABI-tag, file(1))\n'
  sed 's/^/  /' "$WORK/floors.txt" 2>/dev/null
  printf '\nARM 3 -- NSS modules opened with the override on\n'
  sed 's/^/  /' "$WORK/nss.txt" 2>/dev/null || printf '  (not run)\n'
  printf '\nARM 4 -- class residue (env, glibc, B, C, S)\n'
  sed 's/^/  /' "$WORK/classes.txt" 2>/dev/null || printf '  (not run)\n'
  printf '\nARM 5 -- POCs\n'
  sed 's/^/  /' "$WORK/pocs.txt" 2>/dev/null || printf '  (not run)\n'
} > "$RESULT"
exp_note "written: $RESULT"

exp_finish
