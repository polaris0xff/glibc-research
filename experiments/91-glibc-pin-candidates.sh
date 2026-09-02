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
    # ⭐ AND A RATE LIMIT MUST NOT SILENTLY DROP A CANDIDATE. Docker Hub's
    # anonymous limit refuses a different tag on each run -- `debian:trixie`
    # resolved and `ubuntu:24.04` 429'd one run, and the reverse the next -- so
    # which candidates a run measures becomes a coin toss, and a candidate that
    # vanished looks the same as one nobody asked for. An environment already
    # built for this candidate recorded the digest it was built FROM, which is
    # a better answer than none: it is the digest this machine actually holds.
    # ⚠ Labelled `on-disk`, never presented as a fresh resolution.
    _d=$(sed -n 's/^digest: *//p' "$ROOTFS_DIR/pgb-env-$(tagof "$img")/.pgb-env" 2>/dev/null)
    if [ -n "${_d:-}" ]; then
      exp_note "⚠ $img: taking the digest from the environment on disk, NOT re-resolved: $_d"
      printf '  %-16s %s (on-disk)\n' "$img" "$_d"
      printf '%s %s\n' "$img" "$_d" >> "$WORK/candidates.txt"
    fi
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
  # ⛔ `</dev/null` IS NOT OPTIONAL, and leaving it off cost this arm its second
  # row. `docker run -i` attaches the container's stdin to THIS process's fd 0,
  # and the caller below is `while read ... done < candidates.txt` -- so docker
  # drained the candidate list and the loop ended after one iteration, having
  # measured the incumbent and nothing else. It does not look like a failure:
  # arm 2 printed one tidy row and its assertion (the incumbent's floor was
  # read) passed. Same defect experiments/93- carries the same guard against.
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
    ' >"$_o/docker.log" 2>&1 </dev/null
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
  # ⛔ 73- WRITES ITS OWN RESULT.txt, and evidence/73-host-dso-abi-demand/ is
  # COMMITTED and describes the PINNED environment. Running it for a candidate
  # replaced that file with the candidate's -- so the tree then carried a
  # committed result headed `pinned build glibc : 2.41` under the incumbent's
  # name, and nothing in it said a candidate produced it. PGB_EVIDENCE_DIR
  # sends each candidate's run to its own tree instead.
  PGB_EVIDENCE_DIR="$WORK/evidence-73-$en" PGB_ENV_NAME="$en" \
    sh "$REPO_DIR/experiments/73-host-dso-abi-demand.sh" \
    >"$WORK/73-$en.log" 2>&1
  cp -f "$WORK/evidence-73-$en/73-host-dso-abi-demand/RESULT.txt" \
        "$WORK/73-$en.RESULT.txt" 2>/dev/null
  g=$(sed -n 's/^pinned glibc *: *\([0-9.]*\).*/\1/p' "$WORK/73-$en.log" | head -1)
  [ -n "$g" ] || g=$(sed -n 's/.*pinned glibc: \([0-9.]*\).*/\1/p' "$WORK/73-$en.log" | head -1)
  # ⛔ COUNT SYMBOLS, NOT LINES. This counted every indented line in the
  # section, so it counted the column header, the "... N distinct symbols"
  # footer, and -- worst -- the sentence "empty on every environment.", which
  # made an EMPTY class C report as **1**. That is a cost this pin does not
  # have, printed in the one column the ruling turns on. A symbol row is
  # `  SYMBOL  GLIBC_x.yz  OBJS  ENVS`: four fields whose second is a version.
  # ⚠ AND THE SECTION IS TRUNCATED AT TEN ROWS, so counting rows undercounts
  # any class with more: 73- prints the top ten and then `... N distinct
  # symbols`. That footer is the exact figure, so it wins where it appears and
  # the row count is the fallback for a class that never reached ten.
  class_count() {  # result-file class-letter -> distinct symbols
    awk -v want="CLASS $2 " '
      index($0, want) == 3 { f = 1; n = 0; total = -1; next }
      /^  CLASS /          { f = 0 }
      f && $1 == "..." && $3 == "distinct" { total = $2 }
      f && $2 ~ /^GLIBC_/  { n++ }
      END { print (total >= 0 ? total : n + 0) }
    ' "$1" 2>/dev/null
  }
  cb=$(class_count "$WORK/73-$en.RESULT.txt" B)
  cc=$(class_count "$WORK/73-$en.RESULT.txt" C)
  cs=$(class_count "$WORK/73-$en.RESULT.txt" S)
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
  # ⛔ PGB_ENV_NAME ALONE DOES NOT SELECT A GLIBC, and believing it did made
  # this arm measure the incumbent while reporting the candidate. Three
  # separate things have to agree:
  #
  #   PGB_ENGINE      the chroot engine is the only one that READS a name; on a
  #                   machine running dockerd, detection picks docker, which
  #                   builds from the image pgb-env:<version> and ignores it
  #   PGB_ENV_IMAGE   the stamp check compares IMAGES. With only the name set,
  #   PGB_ENV_DIGEST  the wanted image is still the default, so the guard sees
  #                   a match and the build proceeds against the wrong glibc
  #
  # Both are read from the environment's own `.pgb-env`, so this cannot drift
  # from what `pgb env create` actually built.
  POC_ENV_ROOT="$ROOTFS_DIR/$POC_ENV"
  if [ ! -f "$POC_ENV_ROOT/.pgb-env" ]; then
    exp_skip "arm 5" "no $POC_ENV_ROOT/.pgb-env -- create the environment first"
  else
  POC_IMG=$(sed -n 's/^image: *//p'  "$POC_ENV_ROOT/.pgb-env")
  POC_DIG=$(sed -n 's/^digest: *//p' "$POC_ENV_ROOT/.pgb-env")
  # The gcc the candidate carries, as the version alone: `gcc (Debian
  # 14.2.0-19) 14.2.0` -> `14.2.0`. It is what every produced binary must say.
  POC_GCC=$(sed -n 's/^gcc: *//p' "$POC_ENV_ROOT/.pgb-env" | awk '{print $NF}')
  exp_note "$POC_ENV: $POC_IMG $POC_DIG"
  exp_note "$POC_ENV: gcc $POC_GCC -- every binary below must carry it"

  # ⛔ THE CANDIDATE'S POC EVIDENCE GOES SOMEWHERE ELSE. evidence/poc/<name>/
  # is committed and describes the PINNED environment; a candidate run writing
  # there would silently replace the incumbent's record with the candidate's,
  # and nothing in either file says which pin produced it.
  POC_EV="$WORK/evidence-$POC_ENV"
  mkdir -p "$POC_EV"

  printf '  %-14s %-8s %s\n' POC OUTCOME 'GCC IN .comment'
  : > "$WORK/pocgcc.txt"
  for p in $POCS; do
    if [ ! -f "$REPO_DIR/poc/$p/run.sh" ]; then
      printf '  %-14s %s\n' "$p" "absent"
      continue
    fi
    if PGB_ENGINE=chroot PGB_ENV_NAME="$POC_ENV" \
       PGB_ENV_IMAGE="$POC_IMG" PGB_ENV_DIGEST="$POC_DIG" \
       PGB_EVIDENCE_DIR="$POC_EV" \
       sh "$REPO_DIR/poc/$p/run.sh" >"$WORK/poc-$p.log" 2>&1; then
      st=ok
      printf '%s ok\n' "$p" >> "$WORK/pocs.txt"
    else
      st="exit $?"
      printf '%s failed\n' "$p" >> "$WORK/pocs.txt"
    fi
    # ⭐ THE CHECK THAT CAN DISAGREE WITH THE WHOLE ARM. A POC that built in the
    # wrong environment still runs, still passes its own 11-row matrix, and
    # says nothing about which glibc it used. `.comment` is what the compiler
    # wrote into the binary, so it is the one field the POC cannot fake.
    #
    # ⛔ THREE OUTCOMES, NOT TWO, AND THE THIRD IS WHY. A POC is not required
    # to leave its binary in its evidence directory: 90-qt and 91-qt-xcb leave
    # none at all, and the first version of this check called that a MISMATCH
    # and failed two POCs that had just passed 20 and 27 assertions across
    # eleven environments. `docs/AGENTS.md` §0b: an absence is not a zero, and
    # a skip is neither a pass nor a failure. Recursive, because some POCs put
    # what they built in a subdirectory.
    seen=""
    for b in $(find "$POC_EV/poc/$p" -type f -perm -u+x 2>/dev/null); do
      v=$(readelf -p .comment "$b" 2>/dev/null | sed -n 's/.*GCC: (.*) *\([0-9][0-9.]*\).*/\1/p' | head -1)
      [ -n "$v" ] && { seen="$v"; break; }
    done
    case "${seen:-}" in
      "")        gcc_state=unmeasured ;;
      "$POC_GCC") gcc_state="$seen" ;;
      *)         gcc_state="⛔ $seen" ;;
    esac
    printf '  %-20s %-8s %s\n' "$p" "$st" "$gcc_state"
    printf '%s %s\n' "$p" "${seen:-unmeasured}" >> "$WORK/pocgcc.txt"
    # ⚠ THE POC'S VERDICT IS ITS STDOUT. poc_finish() PRINTS pass/fail and
    # exits; it writes no RESULT.txt -- poc/common.sh says so in as many words,
    # "RESULT.txt IS A CONVENTION, NOT AN AUTOMATION ... saved by hand". The
    # first version of this copied a RESULT.txt that never exists, silently,
    # and left the candidate run with no committed record at all.
    cp -f "$WORK/poc-$p.log" "$EXP_OUT/poc-$POC_ENV-$p.txt" 2>/dev/null || true
  done

  # ⛔ NOT `grep -c ... || echo 0`. grep -c PRINTS the count and then EXITS 1
  # when the count is zero, so the fallback fires as well and the value becomes
  # two lines, "0\n0" -- which never equals "0" and fails the arm at exactly the
  # moment every POC passed.
  exp_check "every POC asked for built against $POC_ENV" \
    "$(awk '$2=="failed"{n++} END{print n+0}' "$WORK/pocs.txt" 2>/dev/null)" 0
  # ⛔ ASSERTED ON THE MISMATCH, not on the absence. A POC whose binary carries
  # another gcc was built somewhere else and its 11-of-11 table describes the
  # wrong environment -- that is a failure. A POC that leaves no binary to read
  # is UNMEASURED, and calling that a failure fails POCs that passed.
  _mis=$(awk -v w="$POC_GCC" '$2!="unmeasured" && $2!=w {n++} END{print n+0}' "$WORK/pocgcc.txt")
  _ver=$(awk -v w="$POC_GCC" '$2==w {n++} END{print n+0}' "$WORK/pocgcc.txt")
  _unm=$(awk '$2=="unmeasured" {n++} END{print n+0}' "$WORK/pocgcc.txt")
  exp_check "no POC binary carries a gcc other than the candidate's ($POC_GCC)" "$_mis" 0
  # ⭐ AND THE VERIFIED COUNT IS PRINTED, because "0 mismatches" is also what a
  # run that read nothing at all would report. The two numbers together are the
  # measurement; either alone can be satisfied by an instrument that is blind.
  exp_note "gcc verified on $_ver POC binaries, $_unm left none to read, $_mis mismatched"
  if [ "$_unm" -gt 0 ]; then
    exp_note "⚠ unmeasured (no executable in the POC's evidence directory): \
$(awk '$2=="unmeasured"{printf "%s ", $1}' "$WORK/pocgcc.txt")"
  fi
  fi
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
  # ⛔ THE GCC COLUMN GOES IN THE RECORD, and leaving it out was the third
  # defect in this arm. The committed file carried "10-gawk ok" and nothing
  # about WHICH environment produced it -- which is exactly the claim the arm
  # exists to support, and exactly what was wrong before the arm asserted it.
  printf '\nARM 5 -- POCs (name, outcome, gcc read from the binary .comment)\n'
  if [ -s "$WORK/pocs.txt" ]; then
    awk 'NR==FNR{g[$1]=$2; next} {printf "  %-22s %-8s %s\n", $1, $2, (g[$1]?g[$1]:"unmeasured")}' \
      "$WORK/pocgcc.txt" "$WORK/pocs.txt" 2>/dev/null
    printf '  environment : %s %s\n' "${POC_ENV:-?}" "${POC_IMG:-?}"
    printf '  its gcc     : %s\n' "${POC_GCC:-?}"
  else
    printf '  (not run)\n'
  fi
} > "$RESULT"
exp_note "written: $RESULT"

exp_finish
