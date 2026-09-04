#!/bin/sh
# THE QUESTION
#
#   Is `pgb` actually better than the things that already exist? Same program,
#   same matrix, measured head to head: coverage, startup, memory, size, and
#   what each one needs the host to already have.
#
# -- WHY THIS EXISTS ---------------------------------------------------------
#
# docs/REQUIREMENTS.md is the operator's binding bar: *works everywhere, or
# strictly better than every existing format and technique*. Part 2 of it --
# "strictly better, measured head to head" -- had no measurement at all.
# docs/comparison.md carried a dash in every behaviour cell of every non-pgb
# row and said so plainly: nothing else was ever run. A dash is not a result,
# and "better than the alternatives" with no alternative measured is an
# assertion.
#
# ⛔ SO THE POINT OF THIS SCRIPT IS TO LET pgb LOSE. Every arm is built the way
# its own documentation says to build it, in the best configuration this
# machine can give it, and run on exactly the same 11 environments. If
# something else wins a column, the column says so.
#
# -- THE ARMS ----------------------------------------------------------------
#
#   N  native dynamic glibc     what a distribution's own build produces
#   S  plain `gcc -static`      the naive portable attempt, the §2 control
#   P  pgb                      the subject
#   M  static musl              the technique that already works, at the cost
#                               of not being glibc
#   A  AppImage (type 2)        packaging format, single file
#   O  onelf bundle             packaging format, single file, bundles glibc
#                               AND its loader -- the tier-2 shape of
#                               docs/design/tiers.md, already built by someone
#   F  Flatpak                  packaging format, needs a host runtime
#   K  snap                     packaging format, needs a host daemon
#
# ⭐ N, S, P, A, O, F AND K ALL CARRY THE SAME glibc. Every one is built inside
# the same pinned debian:12 environment, so no arm wins or loses a row because
# it was compiled against a different libc. That is what makes this a
# comparison rather than eight separate anecdotes. M is the exception and
# cannot not be: its whole identity is a different libc.
#
# -- WHAT COUNTS AS A HOST OBJECT, WHICH IS THE CRUX -------------------------
#
# ⛔ TWO INSTRUMENT DEFECTS HAD TO BE FIXED BEFORE THIS SCRIPT MEASURED
# ANYTHING, AND EITHER WOULD HAVE FLATTERED THE ANSWER:
#
#   1. `exp_trace_libs` attributes opens to the single pid that execve'd the
#      target. Right for one static ELF; wrong for every bundle format, which
#      forks, extracts, and execs a payload -- whose library loads then fall
#      outside the filter and score the format a clean "none". So the awk below
#      follows clone/fork/vfork OUT of the artefact's pid.
#   2. Counting every `*.so` open as a host object is worse than useless for a
#      bundling format: onelf's own bundled glibc is opened as
#      `/root/.cache/onelf/<id>/lib/libc.so.6`, whose BASENAME is
#      indistinguishable from the host's. Reduced to basenames, a bundle that
#      correctly avoided the host would have been recorded as loading it.
#
# ⭐ SO A HOST OBJECT IS DEFINED BY PATH, NOT BY NAME: an open under the
# TARGET DISTRIBUTION's own library directories -- /lib, /lib64, /usr/lib,
# /usr/lib64, /usr/local/lib and the multiarch subdirectories of those. An
# object the artefact brought with it lives under its own extraction or mount
# directory and is counted separately, as `bundled`. Both numbers are kept per
# environment in per-environment.txt so the classification can be audited
# rather than believed.
#
# -- ⛔ NOTHING ABOUT THE ALTERNATIVES IS ASSERTED ---------------------------
#
# Same rule as experiments/50-: this is the first measurement of these arms, so
# an expected value would be exactly the guess the experiment exists to
# replace. What IS asserted is (a) that each arm built, because a build that
# silently did not happen would read as a format that scored zero, and (b)
# pgb's own already-published result, so a regression in the subject fails this
# script instead of quietly re-baselining.
#
# Exit: 0 measured and matched, 1 an assertion failed, 2 could not measure.

. "$(dirname "$0")/lib.sh"

exp_begin "60 - head to head against the alternatives (REQUIREMENTS.md part 2)"

ITERATIONS="${PGB_VS_ITERS:-100}"
ROUNDS="${PGB_VS_ROUNDS:-3}"

# ENV_ROOT and ENV_NAME come from lib.sh, which reads the pinned build
# environment's name out of internal/cfg/cfg.go. T-070: this file used to
# carry its own `pgb-env-debian12` fallback and would not have followed a pin
# move.
MUSL_ROOT="$ROOTFS_DIR/alpine-3.22"
RR="$REPO_DIR/pgb"
# ⛔ PINNED, FOR THE SAME REASON scripts/common/rootfs-images.txt IS PINNED.
# "continuous" is a rolling tag: appimagetool publishes every build under it,
# so an unpinned fetch measures a different packer each week and says nothing
# about it. The digest below is the one every AppImage number here was taken
# against. If the fetch stops matching, the arm is SKIPPED rather than quietly
# describing a different tool -- update the digest deliberately and re-measure.
APPIMAGETOOL_URL="https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
APPIMAGETOOL_SHA256="a6d71e2b6cd66f8e8d16c37ad164658985e0cf5fcaa950c90a482890cb9d13e0"

ONELF_SRC="$REPO_DIR/references/QaidVoid__onelf/tree"

B="$EXP_OUT/build"
rm -rf "$B"; mkdir -p "$B" || exit 2
: > "$EXP_OUT/per-environment.txt"

[ -d "$ENV_ROOT" ] || { exp_note "no build environment: ./pgb env create"; exit 2; }

# ---------------------------------------------------------------------------
# The subject. One source, every arm.
# ---------------------------------------------------------------------------
# ⭐ IT HAS TO TOUCH THE SUBSYSTEMS THE PROJECT IS ABOUT, or the comparison
# measures process startup and nothing else. NSS (getpwuid), the builtin-vs-
# dlopen'd gconv split (ISO-8859-1 is one of glibc's builtins, EUC-JP is not),
# and a byte-level assertion on the conversion rather than "it returned
# something". `--startup` is the do-nothing mode the timing loop uses.
cat > "$B/subject.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <string.h>
#include <iconv.h>
#include <pwd.h>

static int fails = 0;
static void chk(const char *what, int ok) {
    if (!ok) { fails++; fprintf(stderr, "FAIL %s\n", what); }
}

int main(int argc, char **argv)
{
    if (argc > 1 && strcmp(argv[1], "--startup") == 0) return 0;

    struct passwd *pw = getpwuid(0);
    chk("nss.getpwuid(0)==root", pw && strcmp(pw->pw_name, "root") == 0);

    /* ISO-8859-1 is one of the encodings glibc has builtin; EUC-JP is not, so
     * it needs a gconv module -- or, under pgb, static GNU libiconv. The pair
     * is the whole gconv finding in one probe. */
    const char *in = "h\xc3\xa9llo";
    char out[64];
    iconv_t cd = iconv_open("ISO-8859-1", "UTF-8");
    chk("iconv.open ISO-8859-1", cd != (iconv_t)-1);
    if (cd != (iconv_t)-1) {
        char *ip = (char *)in, *op = out;
        size_t il = strlen(in), ol = sizeof out;
        size_t r = iconv(cd, &ip, &il, &op, &ol);
        chk("iconv.latin1 roundtrip", r != (size_t)-1 && (size_t)(op-out) == 5
                                      && (unsigned char)out[1] == 0xe9);
        iconv_close(cd);
    }
    cd = iconv_open("EUC-JP", "UTF-8");
    chk("iconv.open EUC-JP", cd != (iconv_t)-1);
    if (cd != (iconv_t)-1) iconv_close(cd);

    if (argc > 1 && strcmp(argv[1], "--verbose") == 0) printf("fails=%d\n", fails);
    return fails == 0 ? 0 : 1;
}
EOF

in_env() {  # run a command inside the pinned build environment
  "$RR" rootfs run "$ENV_ROOT" --bind "$B:$B" --workdir "$B" -- /bin/sh -c "$1" </dev/null
}

arm_file() { printf '%s' "$B/arm-$1"; }
arm_built() { eval "printf '%s' \"\$BUILT_$1\""; }
arm_name() {
  case "$1" in
    N) printf 'native dynamic' ;; S) printf 'plain gcc -static' ;;
    P) printf 'pgb' ;;            M) printf 'static musl' ;;
    A) printf 'AppImage' ;;       O) printf 'onelf' ;;
    F) printf 'Flatpak' ;;        K) printf 'snap' ;;
  esac
}

# ---------------------------------------------------------------------------
# Arms N, S, P, M
# ---------------------------------------------------------------------------
BUILT_N=no; BUILT_S=no; BUILT_P=no; BUILT_M=no
BUILT_A=no; BUILT_O=no; BUILT_F=no; BUILT_K=no
WHY_M=; WHY_A=; WHY_O=; WHY_F=; WHY_K=
MUSL_VER=; FLATPAK_RUNTIME_BYTES=

in_env "gcc -O2 -o $B/arm-N $B/subject.c" >>"$B/build.log" 2>&1 && BUILT_N=yes
in_env "gcc -O2 -static -o $B/arm-S $B/subject.c" >>"$B/build.log" 2>&1 && BUILT_S=yes
( cd "$B" && "$REPO_DIR/pgb" --bind "$B" build -- /bin/sh -c \
    "\$CC -O2 -o $B/arm-P $B/subject.c" ) >>"$B/build.log" 2>&1 && BUILT_P=yes

# ⚠ THE ONE UNPINNED BUILD INPUT IN THIS SCRIPT. The musl arm is compiled by
# Alpine's own gcc, which is the faithful way to produce a static musl binary
# -- but `apk add` resolves against Alpine's live index, so its musl and gcc
# are whatever that says today, unlike every other arm here. The RESULT records
# the versions it actually got; a rerun that disagrees is a different
# toolchain, not a different finding about pgb.
if [ -d "$MUSL_ROOT" ]; then
  if "$RR" rootfs run "$MUSL_ROOT" --bind "$B:$B" --workdir "$B" -- /bin/sh -c \
       "apk add --no-cache gcc musl-dev >/dev/null 2>&1 && gcc -O2 -static -o $B/arm-M $B/subject.c" \
       </dev/null >>"$B/build.log" 2>&1 && [ -x "$B/arm-M" ]; then
    BUILT_M=yes
    MUSL_VER=$("$RR" rootfs run "$MUSL_ROOT" -- /bin/sh -c 'apk info musl 2>/dev/null | head -1' \
                 </dev/null 2>/dev/null | tr -d '\r')
  else
    WHY_M="alpine gcc/musl-dev could not be installed, or the link failed"
  fi
else
  WHY_M="alpine-3.22 not fetched"
fi

# ---------------------------------------------------------------------------
# Arm A -- AppImage, type 2
# ---------------------------------------------------------------------------
# ⭐ GIVEN ITS BEST CONFIGURATION ON PURPOSE. AppRun is a SYMLINK to the
# payload, not the shell script appimagetool's examples use: a shell AppRun
# would exec the target distribution's /bin/sh, and every library that shell
# loads would land in this arm's host-object column for a reason that has
# nothing to do with AppImage's design. The symlink form is documented, it is
# all this payload needs, and it keeps the measurement about the format.
#
# ⚠ NO glibc IS BUNDLED, and that is AppImage's documented practice rather than
# an oversight: its guidance is to build against the oldest glibc you intend to
# support and let forward compatibility do the rest. Building here in the
# pinned debian:12 therefore sets this arm's floor at glibc 2.36, and an older
# build host would raise its glibc coverage. ⛔ It would not change a single
# musl row: no glibc build of any age supplies ld-linux to a musl host.
build_appimage() {
  command -v curl >/dev/null 2>&1 || { WHY_A="curl absent"; return 1; }
  curl -fsSL -o "$B/appimagetool" "$APPIMAGETOOL_URL" 2>>"$B/build.log" \
    || { WHY_A="appimagetool could not be fetched"; return 1; }
  got=$(sha256sum "$B/appimagetool" 2>/dev/null | cut -d' ' -f1)
  if [ "$got" != "$APPIMAGETOOL_SHA256" ]; then
    WHY_A="appimagetool digest is $got, pinned $APPIMAGETOOL_SHA256 -- update the pin deliberately and re-measure"
    return 1
  fi
  chmod +x "$B/appimagetool"
  AD="$B/AppDir"; rm -rf "$AD"; mkdir -p "$AD/usr/bin"
  cp "$B/arm-N" "$AD/usr/bin/subject"
  ln -s usr/bin/subject "$AD/AppRun"
  cat > "$AD/subject.desktop" <<'DESK'
[Desktop Entry]
Type=Application
Name=subject
Exec=subject
Icon=subject
Categories=Utility;
Terminal=true
DESK
  # the smallest thing appimagetool accepts as an icon: a 1x1 PNG
  printf '\211PNG\r\n\032\n\0\0\0\rIHDR\0\0\0\1\0\0\0\1\10\6\0\0\0\37\25\304\211\0\0\0\nIDATx\234c\370\17\0\1\1\1\0\30\335\215\260\0\0\0\0IEND\256B`\202' > "$AD/subject.png"
  ( cd "$B" && APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 "$B/appimagetool" "$AD" "$B/arm-A" ) \
    </dev/null >>"$B/build.log" 2>&1 || { WHY_A="appimagetool failed; see build.log"; return 1; }
  [ -f "$B/arm-A" ] || { WHY_A="appimagetool produced nothing"; return 1; }
  chmod +x "$B/arm-A"
}

# ---------------------------------------------------------------------------
# Arm O -- onelf: a bundled glibc AND its loader, in one file
# ---------------------------------------------------------------------------
# ⭐ THE MOST INTERESTING ARM HERE, because it is tier 2 of
# docs/design/tiers.md already built by someone else. If a bundled glibc plus
# its own loader in a single file already works everywhere, the tier plan is
# mostly integration; if it does not, the reason is worth more to this project
# than the comparison is.
#
# ⛔ THE BUNDLED LIBRARIES COME OUT OF THE PINNED ENVIRONMENT, NOT THE HOST.
# `onelf bundle-libs` resolves DT_NEEDED against the machine it runs on, which
# here would have wrapped the HOST's glibc around a binary compiled against the
# pinned one -- a mismatch invented by the measurement rather than found by it.
# --search-path points it at the same debian:12 rootfs every other arm used.
build_onelf() {
  [ -d "$ONELF_SRC" ] || { WHY_O="reference corpus missing"; return 1; }
  command -v cargo >/dev/null 2>&1 || { WHY_O="cargo absent; onelf is a rust workspace"; return 1; }
  ONELF_BIN="$ONELF_SRC/target/release/onelf"
  if [ ! -x "$ONELF_BIN" ]; then
    ( cd "$ONELF_SRC" && cargo build --release ) </dev/null >>"$B/build.log" 2>&1 || {
      WHY_O="cargo build failed; onelf needs musl-gcc and the x86_64-unknown-linux-musl rust target"
      return 1; }
  fi
  [ -x "$ONELF_BIN" ] || { WHY_O="onelf did not build"; return 1; }
  D="$B/onelfdir"; rm -rf "$D"; mkdir -p "$D/bin"; cp "$B/arm-N" "$D/bin/subject"
  "$ONELF_BIN" bundle-libs "$D" --strip \
      --search-path "$ENV_ROOT/usr/lib/x86_64-linux-gnu" \
      --search-path "$ENV_ROOT/lib/x86_64-linux-gnu" \
      </dev/null >>"$B/build.log" 2>&1 || { WHY_O="onelf bundle-libs failed"; return 1; }
  "$ONELF_BIN" pack "$D" -o "$B/arm-O" --command bin/subject \
      </dev/null >>"$B/build.log" 2>&1 || { WHY_O="onelf pack failed"; return 1; }
  [ -f "$B/arm-O" ] || { WHY_O="onelf produced nothing"; return 1; }
  chmod +x "$B/arm-O"
}

# ---------------------------------------------------------------------------
# Arm F -- Flatpak
# ---------------------------------------------------------------------------
# Built from the prebuilt payload rather than through flatpak-builder's
# manifest machinery, because every other arm here also packages an
# already-compiled binary: putting one arm through a source build would measure
# flatpak-builder, not the format.
build_flatpak() {
  command -v flatpak >/dev/null 2>&1 || { WHY_F="flatpak absent on this machine"; return 1; }
  flatpak info org.freedesktop.Platform//24.08 >/dev/null 2>&1 \
    || { WHY_F="org.freedesktop.Platform//24.08 not installed"; return 1; }
  rm -rf "$B/fp" "$B/fprepo" "$B/arm-F"
  flatpak build-init "$B/fp" org.pgb.Subject org.freedesktop.Sdk \
      org.freedesktop.Platform 24.08 >>"$B/build.log" 2>&1 \
    || { WHY_F="flatpak build-init failed"; return 1; }
  mkdir -p "$B/fp/files/bin"; cp "$B/arm-N" "$B/fp/files/bin/subject"
  flatpak build-finish "$B/fp" --command=subject >>"$B/build.log" 2>&1 \
    || { WHY_F="flatpak build-finish failed"; return 1; }
  flatpak build-export "$B/fprepo" "$B/fp" >>"$B/build.log" 2>&1 \
    || { WHY_F="flatpak build-export failed"; return 1; }
  flatpak build-bundle "$B/fprepo" "$B/arm-F" org.pgb.Subject >>"$B/build.log" 2>&1 \
    || { WHY_F="flatpak build-bundle failed"; return 1; }
  [ -f "$B/arm-F" ] || { WHY_F="flatpak build-bundle produced nothing"; return 1; }
  for p in /var/lib/flatpak "$HOME/.local/share/flatpak"; do
    [ -d "$p/runtime/org.freedesktop.Platform" ] || continue
    FLATPAK_RUNTIME_BYTES=$(du -sb "$p/runtime/org.freedesktop.Platform" 2>/dev/null | cut -f1)
    break
  done
}

# ---------------------------------------------------------------------------
# Arm K -- snap
# ---------------------------------------------------------------------------
# ⚠ BUILT BY HAND WITH mksquashfs, NOT snapcraft, and the difference is worth
# stating: a .snap IS a squashfs carrying meta/snap.yaml, so this is a real one
# byte for byte -- but snapcraft is distributed as a snap, installing it needs
# a running snapd, and snapd needs systemd, which this container does not have.
# What is measured from this artefact is its size and what a target must
# already have to run it. Neither of those comes from snapcraft.
build_snap() {
  command -v mksquashfs >/dev/null 2>&1 || { WHY_K="mksquashfs absent"; return 1; }
  D="$B/snapdir"; rm -rf "$D" "$B/arm-K"; mkdir -p "$D/meta" "$D/bin"
  cp "$B/arm-N" "$D/bin/subject"
  cat > "$D/meta/snap.yaml" <<'SNAP'
name: pgb-subject
version: '1.0'
summary: head-to-head comparison subject
description: The same subject program packaged as a snap.
base: core24
confinement: strict
grade: stable
architectures: [amd64]
apps:
  pgb-subject:
    command: bin/subject
SNAP
  mksquashfs "$D" "$B/arm-K" -noappend -comp xz -no-fragments -no-progress \
    >>"$B/build.log" 2>&1 || { WHY_K="mksquashfs failed"; return 1; }
  [ -f "$B/arm-K" ] || { WHY_K="mksquashfs produced nothing"; return 1; }
}

if [ "$BUILT_N" = yes ]; then
  build_appimage && BUILT_A=yes
  build_onelf    && BUILT_O=yes
  build_flatpak  && BUILT_F=yes
  build_snap     && BUILT_K=yes
else
  WHY_A="arm N did not build"; WHY_O="$WHY_A"; WHY_F="$WHY_A"; WHY_K="$WHY_A"
fi

# ---------------------------------------------------------------------------
# ⛔ A BUILD THAT DID NOT HAPPEN MUST NOT READ AS A FORMAT THAT SCORED ZERO.
# ---------------------------------------------------------------------------
exp_check "arm N built (native dynamic)"    "$BUILT_N" yes
exp_check "arm S built (plain gcc -static)" "$BUILT_S" yes
exp_check "arm P built (pgb)"               "$BUILT_P" yes
for a in M A O F K; do
  ok=$(arm_built "$a"); eval "why=\$WHY_$a"
  if [ "$ok" = yes ]; then exp_check "arm $a built ($(arm_name "$a"))" built built
  else exp_skip "arm $a ($(arm_name "$a"))" "${why:-not built}"; fi
done
printf '\n'

# ---------------------------------------------------------------------------
# Tracing that survives a fork, and tells a bundled object from a host one
# ---------------------------------------------------------------------------
# See the header for both defects this replaces. Reads a trace file already on
# disk and prints `host <path>` / `bundled <path>` lines.
#
#   exp_classify_trace TRACEFILE /artefact payload|tree
#
# payload = only the pid running the innermost ELF, which is the process the
#           two-libc hazard actually lives in;
# tree    = every pid descended from the artefact, which is everything the
#           machine was made to load in order to deliver it.
#
# ⭐ THE CLASSIFIER IS `experiments/lib.sh`'s `exp_classify_trace`, NOT A COPY.
#
# ⛔ SIX EXPERIMENTS CARRIED THE SAME awk BY HAND AND THEY COULD NOT BE
# CORRECTED TOGETHER. Two defects, found a day apart, running in opposite
# directions:
#
#   C25  strace splits a long call across `openat(..., "path" <unfinished ...>`
#        and `<... openat resumed>) = -1 ENOENT`. The PATH is on the first line
#        and the RESULT on the second, so a filter that drops lines containing
#        ENOENT keeps the first half of a FAILED open and counts it as a load.
#        ⛔ Turns a CLEAN row DIRTY.
#   C38  five of the six cleared their result set on the artefact's own execve
#        UNCONDITIONALLY, so in `tree` mode everything opened before the last
#        invocation vanished. ⛔ Turns a DIRTY row CLEAN.
#
# ⭐ `exp_classify_trace <tracefile> <artefact> [payload|tree]` — `mode` LAST
# with a default, so an un-updated caller keeps `tree` rather than silently
# reporting zero. `sh experiments/lib.sh --selftest` asserts both defects.
# TODO T-084; docs/history/corrections.md C25, C38.

# ⛔ EVERY RUN IS TIME-LIMITED, AND THE AppImage ARM IS WHY. `strace -f` does
# not return until every process it traced has exited, and the AppImage runtime
# forks a helper that does not exit on the same schedule as the payload. The
# traced run therefore never finished: the experiment stopped dead on the first
# AppImage row, and the tracee was left in ptrace-stop where SIGKILL to strace
# alone did not reap it. Without -f the same run completes in under a second.
#
# ⭐ SO -f IS ATTEMPTED AND THEN GIVEN UP ON, PER CELL. A cell that times out
# under -f is re-traced without it. That still measures the PAYLOAD correctly
# -- every arm here execs its payload in the same pid, which is exactly what -f
# is not needed for -- but it cannot see other processes, so the TREE column
# for that cell is recorded as unmeasured. ⛔ Unmeasured is not clean, and the
# tree counters below count only cells where -f actually completed.
RUN_TIMEOUT="${PGB_VS_TIMEOUT:-25}"

# ⛔ TWO REAPERS WERE WRONG BEFORE THIS ONE.
#
#   `pkill -f` matched the runner's OWN command line -- `pgb rootfs run ... --
#   /pgb-vs-arm` -- so it killed the experiment along with the leftovers.
#
#   `pkill -x pgb-vs-arm` matched the artefact and nothing else, which is not
#   what a delivery format leaves behind: an AppImage's uruntime leaves a
#   DWARFS FUSE daemon whose comm is `memfd:dwarfs`, by design, because a mount
#   that outlives the program is what mount mode is. A full pass of
#   experiments/62- left 22 of them running and they had to be killed by hand.
#
# ⭐ /proc/PID/root IS THE CHROOT A PROCESS IS ACTUALLY IN, so matching on it
# reaps every straggler of a cell whatever it is called, and cannot match
# anything outside the test bed. docs/history/corrections.md.
reap_rootfs() {  # rootfs-path
  for _d in /proc/[0-9]*; do
    _p=${_d#/proc/}
    _r=$(readlink "$_d/root" 2>/dev/null) || continue
    case "$_r" in "$1"|"$1"/*) kill -9 "$_p" 2>/dev/null ;; esac
  done
  return 0
}

# An interrupted run must not leave the bed populated either.
reap_all() {
  while read -r _ref _name _libc _digest; do
    case "$_ref" in ''|\#*) continue ;; esac
    _r=$(exp_rootfs "$_name"); [ -n "$_r" ] && reap_rootfs "$_r"
  done < "$REPO_DIR/scripts/common/rootfs-images.txt"
  return 0
}
trap 'reap_all' EXIT INT TERM

# echoes `full` when -f completed, `nofork` when it had to be dropped
trace_run() {  # rootfs /artefact tracefile
  timeout -k 10 "$RUN_TIMEOUT" \
    strace -f -e trace=openat,open,execve,clone,clone3,vfork,fork -o "$3" \
      "$RR" rootfs run "$1" -- "$2" </dev/null >/dev/null 2>&1
  _rc=$?
  reap_rootfs "$1"
  if [ "$_rc" = 124 ] || [ "$_rc" = 137 ]; then
    timeout -k 10 "$RUN_TIMEOUT" \
      strace -e trace=openat,open,execve -o "$3" \
        "$RR" rootfs run "$1" -- "$2" </dev/null >/dev/null 2>&1
    reap_rootfs "$1"
    printf 'nofork'
  else
    printf 'full'
  fi
}

count() { n=$(grep -c . 2>/dev/null) || n=0; printf '%s' "$n"; }

# ---------------------------------------------------------------------------
# The matrix
# ---------------------------------------------------------------------------
ARMS="N S P M A O"
for a in $ARMS; do eval "RUNS_$a=0; CLEAN_$a=0; TREECLEAN_$a=0; TREEMEAS_$a=0; TESTED_$a=0"; done
ENVS=0

printf -- '-- per environment ------------------------------------------------\n'
printf '  %-19s %-6s' ENVIRONMENT LIBC
for a in $ARMS; do printf ' %-10s' "$a"; done
printf '\n'

while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  root=$(exp_rootfs "$name")
  if [ -z "$root" ]; then exp_skip "$name" "not fetched"; continue; fi
  ENVS=$((ENVS+1))
  printf '  %-19s %-6s' "$name" "$libc"
  for a in $ARMS; do
    if [ "$(arm_built "$a")" != yes ]; then printf ' %-10s' 'not-built'; continue; fi
    rm -f "$root/pgb-vs-arm"
    cp "$(arm_file "$a")" "$root/pgb-vs-arm" 2>/dev/null \
      || { printf ' %-10s' 'copy-fail'; continue; }
    chmod +x "$root/pgb-vs-arm"
    # ⛔ UNPIPED. lib.sh: the status has to be the command's own.
    timeout -k 10 "$RUN_TIMEOUT" "$RR" rootfs run "$root" -- /pgb-vs-arm \
      </dev/null >"$B/out.$name.$a" 2>&1
    st=$?
    reap_rootfs "$root"
    eval "TESTED_$a=\$((TESTED_$a+1))"
    case $st in
      0)   res=ok; eval "RUNS_$a=\$((RUNS_$a+1))" ;;
      124) res=timeout ;;
      13[0-9]|1[4-6][0-9]) res="SIG$((st-128))" ;;
      *)   res="exit$st" ;;
    esac
    # one trace, read twice
    tmode=$(trace_run "$root" /pgb-vs-arm "$B/tr.$name.$a")
    pl=$(exp_classify_trace "$B/tr.$name.$a" /pgb-vs-arm payload)
    nph=$(printf '%s\n' "$pl" | grep '^host ' | count)
    npb=$(printf '%s\n' "$pl" | grep '^bundled ' | count)
    [ "$nph" = 0 ] && eval "CLEAN_$a=\$((CLEAN_$a+1))"
    if [ "$tmode" = full ]; then
      tr=$(exp_classify_trace "$B/tr.$name.$a" /pgb-vs-arm tree)
      nth=$(printf '%s\n' "$tr" | grep '^host ' | count)
      eval "TREEMEAS_$a=\$((TREEMEAS_$a+1))"
      [ "$nth" = 0 ] && eval "TREECLEAN_$a=\$((TREECLEAN_$a+1))"
      treetxt=$(printf '%s\n' "$tr" | sed -n 's/^host //p' | tr '\n' ' ')
    else
      nth=-; treetxt="not measured: -f had to be dropped for this cell"
    fi
    printf ' %-10s' "$res/$nph"
    {
      printf '== %s  arm %s (%s)\n' "$name" "$a" "$(arm_name "$a")"
      printf '   status        : %s\n' "$res"
      printf '   trace         : %s\n' "$tmode"
      printf '   host, payload : %s\n' "$(printf '%s\n' "$pl" | sed -n 's/^host //p' | tr '\n' ' ')"
      printf '   host, tree    : %s\n' "$treetxt"
      printf '   bundled       : %s (%s objects)\n' \
        "$(printf '%s\n' "$pl" | sed -n 's/^bundled //p' | sed 's|.*/||' | tr '\n' ' ')" "$npb"
      printf '   output        : %s\n' "$(head -4 "$B/out.$name.$a" | tr '\n' ' ' | cut -c1-200)"
    } >> "$EXP_OUT/per-environment.txt"
    rm -f "$root/pgb-vs-arm"
  done
  printf '\n'
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

[ "$ENVS" -gt 0 ] || { exp_note "no environments fetched"; exit 2; }
printf '\n  cell is EXITSTATUS/H, H = host shared objects the payload process\n'
printf '  itself opened. ok/0 is the success criterion of docs/AGENTS.md §3.\n\n'

# ---------------------------------------------------------------------------
# What each format needs the target to already have
# ---------------------------------------------------------------------------
# ⭐ THIS IS A COVERAGE MEASUREMENT, NOT A FOOTNOTE. "Copy one file and run it"
# is the property pgb exists to provide, so what an alternative additionally
# demands of an untouched target IS the head-to-head question. Probed on the
# pinned images as they ship, not asserted.
printf -- '-- host prerequisites, probed on the unmodified images -------------\n'
printf '  %-19s %-9s %-9s %-12s %s\n' ENVIRONMENT flatpak snap fusermount /dev/fuse
HAVE_FLATPAK=0; HAVE_SNAP=0; HAVE_FUSE=0
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  root=$(exp_rootfs "$name"); [ -n "$root" ] || continue
  has() {
    for p in "$root/usr/bin/$1" "$root/bin/$1" "$root/usr/sbin/$1" "$root/sbin/$1"; do
      [ -e "$p" ] && { printf yes; return; }
    done
    printf no
  }
  f=$(has flatpak); s=$(has snap)
  u=$(has fusermount); [ "$u" = no ] && u=$(has fusermount3)
  d=no; [ -e "$root/dev/fuse" ] && d=yes
  [ "$f" = yes ] && HAVE_FLATPAK=$((HAVE_FLATPAK+1))
  [ "$s" = yes ] && HAVE_SNAP=$((HAVE_SNAP+1))
  [ "$u" = yes ] && HAVE_FUSE=$((HAVE_FUSE+1))
  printf '  %-19s %-9s %-9s %-12s %s\n' "$name" "$f" "$s" "$u" "$d"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"
printf '\n'

# ---------------------------------------------------------------------------
# Host-side cost
# ---------------------------------------------------------------------------
startup_ms() {  # binary -> best round total, ms
  _bin="$1"; _best=; _r=0
  while [ "$_r" -lt "$ROUNDS" ]; do
    _t0=$(date +%s%N); _i=0
    while [ "$_i" -lt "$ITERATIONS" ]; do "$_bin" --startup >/dev/null 2>&1; _i=$((_i+1)); done
    _t1=$(date +%s%N); _ms=$(( (_t1 - _t0) / 1000000 ))
    if [ -z "$_best" ] || [ "$_ms" -lt "$_best" ]; then _best=$_ms; fi
    _r=$((_r+1))
  done
  printf '%s' "$_best"
}

# ⛔ getrusage(RUSAGE_CHILDREN) IS THE WRONG CALL HERE and experiments/40-
# already paid to find that out: it is a high-water mark across every child
# ever reaped, so successive arms read as an artefact of their ordering.
# os.wait4() returns the rusage of THAT child.
peak_rss_kb() {
  python3 -c '' 2>/dev/null || { printf -- '-'; return; }
  python3 - "$1" <<'PYRSS'
import os, sys
prog = sys.argv[1]; best = None
for _ in range(5):
    pid = os.fork()
    if pid == 0:
        try:
            os.close(2); os.execv(prog, [prog, "--startup"])
        except Exception: os._exit(127)
    _, _, ru = os.wait4(pid, 0)
    best = ru.ru_maxrss if best is None else min(best, ru.ru_maxrss)
print(best if best is not None else "-", end="")
PYRSS
}

printf -- '-- what it costs on this machine ----------------------------------\n'
printf '  conditions: %s execs x %s rounds, best round; ⚠ one machine, one day\n\n' \
  "$ITERATIONS" "$ROUNDS"
printf '  %-3s %-18s %11s %14s %11s  %s\n' \
  ARM ARTEFACT 'SHIP (B)' 'PER EXEC (us)' 'RSS (KiB)' 'ALSO NEEDED ON TARGET'
for a in N S P M A O F K; do
  case "$a" in
    F) need="flatpak + org.freedesktop.Platform//24.08 (${FLATPAK_RUNTIME_BYTES:-?} B)" ;;
    K) need="snapd, which needs systemd, + the core24 base snap" ;;
    A) need="nothing; FUSE only for the mount mode, else it extracts" ;;
    *) need="nothing" ;;
  esac
  if [ "$(arm_built "$a")" != yes ]; then
    printf '  %-3s %-18s %11s %14s %11s  %s\n' "$a" "$(arm_name "$a")" - - - "$need"
    continue
  fi
  size=$(wc -c < "$(arm_file "$a")")
  # ⚠ F and K cannot be executed on this machine, and the table must not imply
  # a number was taken. Their blockers are in the notes below.
  #
  # ⛔ THE TIMING LOOP IS NOT GUARDED PER EXEC ON PURPOSE -- wrapping each of
  # ITERATIONS execs in `timeout` would put a second process in every sample
  # and measure that instead. One guarded probe first decides whether the arm
  # can be timed at all; if it cannot, the row says so rather than hanging.
  case "$a" in
    F|K) us=-; rss=- ;;
    *)   if timeout -k 5 20 "$(arm_file "$a")" --startup >/dev/null 2>&1; then
           ms=$(startup_ms "$(arm_file "$a")"); us=$(( ms * 1000 / ITERATIONS ))
           rss=$(peak_rss_kb "$(arm_file "$a")")
         else
           us=n/a; rss=n/a
         fi ;;
  esac
  printf '  %-3s %-18s %11s %14s %11s  %s\n' "$a" "$(arm_name "$a")" "$size" "$us" "$rss" "$need"
done
printf '\n'

# ---------------------------------------------------------------------------
# Coverage summary
# ---------------------------------------------------------------------------
printf -- '-- coverage, out of %s environments ---------------------------------\n' "$ENVS"
printf '  %-3s %-18s %9s %14s %12s\n' ARM ARTEFACT RUNS 'PAYLOAD CLEAN' 'TREE CLEAN'
for a in $ARMS; do
  eval "t=\$TESTED_$a; r=\$RUNS_$a; c=\$CLEAN_$a; tc=\$TREECLEAN_$a; tm=\$TREEMEAS_$a"
  if [ "$(arm_built "$a")" != yes ]; then
    printf '  %-3s %-18s %9s %14s %12s\n' "$a" "$(arm_name "$a")" 'not built' - -
  else
    # ⛔ THE TREE COLUMN IS OUT OF THE CELLS WHERE IT COULD BE MEASURED, not
    # out of all of them. A cell that lost -f is missing, never clean.
    printf '  %-3s %-18s %9s %14s %12s\n' "$a" "$(arm_name "$a")" "$r/$t" "$c/$t" "$tc/$tm"
  fi
done
for a in F K; do
  if [ "$(arm_built "$a")" = yes ]; then
    printf '  %-3s %-18s %9s %14s %12s\n' "$a" "$(arm_name "$a")" "0/$ENVS*" - -
  else
    printf '  %-3s %-18s %9s %14s %12s\n' "$a" "$(arm_name "$a")" 'not built' - -
  fi
done
printf '\n'
printf '  * F and K were BUILT here and could not be RUN on any target: %s of %s\n' \
  "$HAVE_FLATPAK" "$ENVS"
printf '    images ship flatpak and %s of %s ship snap, so an untouched target\n' \
  "$HAVE_SNAP" "$ENVS"
printf '    has nothing to execute them with. That is a measurement of the\n'
printf '    images -- see the prerequisite table above -- not an estimate.\n\n'

# ---------------------------------------------------------------------------
# ⛔ The only assertions. See the header for why the alternatives have none.
# ---------------------------------------------------------------------------
eval "P_RUNS=\$RUNS_P; P_CLEAN=\$CLEAN_P; P_T=\$TESTED_P"
exp_check "pgb ran on every environment"     "$P_RUNS"  "$P_T"
exp_check "pgb loaded no host shared object" "$P_CLEAN" "$P_T"

{
  printf 'environments=%s iterations=%s rounds=%s\n' "$ENVS" "$ITERATIONS" "$ROUNDS"
  printf 'musl toolchain=%s\n' "${MUSL_VER:-unknown}"
  printf 'appimagetool sha256=%s\n' "$APPIMAGETOOL_SHA256"
  printf 'flatpak runtime bytes=%s\n' "${FLATPAK_RUNTIME_BYTES:--}"
  printf 'targets shipping flatpak=%s snap=%s fusermount=%s\n' \
    "$HAVE_FLATPAK" "$HAVE_SNAP" "$HAVE_FUSE"
  for a in N S P M A O F K; do
    eval "b=\$BUILT_$a; r=\${RUNS_$a:--}; t=\${TESTED_$a:--}; c=\${CLEAN_$a:--}"
    s=$(wc -c < "$(arm_file "$a")" 2>/dev/null) || s=-
    printf 'arm %s built=%s size=%s runs=%s/%s payload_clean=%s\n' "$a" "$b" "$s" "$r" "$t" "$c"
  done
} > "$EXP_OUT/summary.txt"

# ⛔ A LEAK MUST FAIL THE EXPERIMENT, NOT BE LEFT FOR THE OPERATOR TO NOTICE.
# The first version of this script left 22 FUSE daemons running and said
# nothing; the person running it had to find and kill them by hand.
strays=0
while read -r _ref _name _libc _digest; do
  case "$_ref" in ''|\#*) continue ;; esac
  _r=$(exp_rootfs "$_name"); [ -n "$_r" ] || continue
  for _d in /proc/[0-9]*; do
    _rr=$(readlink "$_d/root" 2>/dev/null) || continue
    case "$_rr" in "$_r"|"$_r"/*) strays=$((strays+1)) ;; esac
  done
done < "$REPO_DIR/scripts/common/rootfs-images.txt"
exp_check "no processes left running in the test bed" "$strays" 0

exp_note "READ THE TWO CLEAN COLUMNS SEPARATELY. 'payload clean' is the §3"
exp_note "  criterion: no host object in the process the program runs in."
exp_note "  'tree clean' also counts what the delivery mechanism loaded to get"
exp_note "  there. A format can be clean by the first and not the second."
exp_note ""
exp_note "⚠ THE BED CANNOT GIVE onelf ITS PREFERRED MODES. Its fuse and tmpfs"
exp_note "  modes both call unshare(CLONE_NEWUSER|CLONE_NEWNS), which returns"
exp_note "  EPERM inside this chroot, so every onelf row here is its LAST"
exp_note "  fallback -- cache mode, which extracts to disk. Outside the bed, on"
exp_note "  this same host, fuse and tmpfs both work. ⛔ Do not read the onelf"
exp_note "  rows as onelf failing to mount; read them as onelf's worst mode."
exp_note ""
exp_note "⚠ F and K were built and never run. flatpak run needs a D-Bus session"
exp_note "  bus and dbus-daemon cannot start in this container (cap_sys_resource"
exp_note "  is dropped, so it cannot raise its fd limit); snapd needs systemd."
exp_note "  Neither blocker touches the coverage row, which is decided by the"
exp_note "  targets shipping no flatpak and no snap at all."
exp_note ""
exp_note "⚠ One machine, one day. See the conditions block at the top."
exp_finish
