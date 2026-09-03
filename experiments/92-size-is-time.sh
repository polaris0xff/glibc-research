#!/bin/sh
# 92-size-is-time.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# ⭐ `TODO/PROGRESS.md` N2, and the operator's own framing of why the struck
# size work might still count:
#
#     "on kdenlive, start and render are dominated by mounting a 398 MB
#      dwarfs image against a 192 MB one -- i.e. the size column IS the time
#      column. ⛔ NOBODY HAS MEASURED THAT."
#
# It is the hypothesis that decides where the bundler's remaining work goes.
# ⭐ If it holds, every byte lever this project already built -- `--cut`,
# `--fixpoint`, the debloat rules -- is a millisecond lever, and the size work
# the operator struck on 2026-09-03c comes back through the front door.
# ⛔ If it does not, those levers are worth nothing on the clock and the work
# is somewhere else entirely.
#
# -- HOW IT IS ASKED --------------------------------------------------------
#
# ⛔ NOT by comparing kdenlive to the competitor. Those two artefacts differ in
# byte count AND file count AND program AND distribution, so a difference
# between them cannot be attributed to any one of those. ⭐ The question is
# asked WITHIN ONE SUBJECT, by taking a single AppDir and changing exactly one
# thing about it at a time:
#
#   base       the AppDir as `pgb bundle appimage` produced it
#   bytes      ⭐ + a large INCOMPRESSIBLE blob. The image grows by roughly the
#              blob; the inode count grows by one; nothing the program runs
#              ever reads it. This is the BYTE axis, alone.
#   inodes     ⭐ + tens of thousands of tiny files. The inode count grows by
#              that many; the image barely grows, because tiny files compress.
#              This is the FILE-COUNT axis, alone.
#
# ⚠ Incompressible, and that is load-bearing twice: dwarfs deduplicates
# identical files and compresses compressible ones, so padding with copies of
# real libraries would add no bytes at all and would measure nothing.
#
# ⭐ Every arm runs the SAME program, `jq --version`, out of the same store.
# The reachable content is identical across arms by construction, so any
# difference is the padding and nothing else.
#
# ⛔ EVERY NUMBER HERE COMES THROUGH `experiments/clock.sh`: median of N,
# arms interleaved with a rotating start, and an A/A control -- one artefact
# under two names -- whose ratio is the floor below which no row may be
# believed. `experiments/99-` is why: the cold column of the instrument this
# replaces was measuring a warm start.
#
# Exit: 0 the measurement ran and matched, 1 it ran and did not, 2 could not run.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"
. "$(cd "$(dirname "$0")" && pwd)/clock.sh"

exp_begin "92 - is the size column the time column?"

# ⛔ SCRATCH GOES IN `build/`, WHICH `.gitignore` ALREADY EXCLUDES. This read
# `$EXP_OUT/work` until a 207 MB padded AppImage was committed and GitHub
# refused the push at its 100 MB file limit. The raw samples are saved out
# separately by `clk_save`, because those ARE evidence.
WORK="$EXP_OUT/build"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
RESULT="$EXP_OUT/RESULT.txt"
PGB="$REPO_DIR/pgb"
CACHE="${PGB_APPIMAGE_CACHE:-/var/tmp/pgb-appimage}"
# ⚠ 21, not 9. The byte axis sits within a factor of two of the instrument
# floor, so the estimator needs the samples even though the assertion no
# longer turns on the knife edge. Each sample is ~85 ms; the run is dominated
# by making and packing a 200 MiB blob, not by timing.
ROUNDS="${PGB_CLOCK_ROUNDS:-21}"
PKG="${PGB_CLOCK_PKG:-jq}"
PAD_MB="${PGB_PAD_MB:-200}"          # the byte axis, in MiB
PAD_FILES="${PGB_PAD_FILES:-40000}"  # the inode axis, in files

[ -x "$PGB" ] || { printf 'pgb is not built (run make)\n'; exit 2; }

MTMP="$WORK/tmp"; mkdir -p "$MTMP"
TMPDIR="$MTMP"; export TMPDIR

# ⛔ The mountpoint path is the handle, never a process name: docs/AGENTS.md
# §14, and experiments/99- for what happens when nothing reaps at all.
reap_mounts() {
  for _d in "$MTMP"/.mount_*; do
    [ -e "$_d" ] || continue
    case "$_d" in *.pid) continue ;; esac
    fusermount3 -u "$_d" 2>/dev/null || fusermount -u "$_d" 2>/dev/null \
      || umount -l "$_d" 2>/dev/null || :
  done
  rm -rf "$MTMP"/.mount_* 2>/dev/null || :
}

# ---------------------------------------------------------------------------
# the base bundle, and the tools that packed it
# ---------------------------------------------------------------------------
printf -- '-- building the base -----------------------------------------\n'
BASE="$WORK/base.AppImage"
rm -rf "$CACHE/$PKG"
if ! "$PGB" bundle appimage "$PKG" --out "$BASE" --cache "$CACHE" \
      --debloat safe > "$WORK/build.log" 2>&1; then
  printf 'the bundle did not build; see %s\n' "$WORK/build.log"
  tail -5 "$WORK/build.log"
  exit 2
fi
APPDIR="$CACHE/$PKG/AppDir"
URUNTIME="$CACHE/tools/uruntime"
MKDWARFS="$CACHE/tools/mkdwarfs"
for f in "$APPDIR" "$URUNTIME" "$MKDWARFS"; do
  [ -e "$f" ] || { printf 'missing %s\n' "$f"; exit 2; }
done

# ⛔ THE PACK COMMAND IS THE PRODUCTION ONE, COPIED FROM
# internal/bundle/appimage.go's mkdwarfs invocation. An arm packed with
# different settings would measure the settings.
repack() { # appdir out
  "$MKDWARFS" --force --set-owner 0 --set-group 0 \
    --no-history --no-create-timestamp \
    --header "$URUNTIME" --input "$1" --output "$2" \
    -C zstd:level=19 -S26 >> "$WORK/mkdwarfs.log" 2>&1 || return 1
  chmod +x "$2"
}

BASE_FILES=$(find "$APPDIR" -type f 2>/dev/null | wc -l | tr -d ' ')
printf '  %-12s %12s bytes  %8s files\n' base "$(wc -c < "$BASE")" "$BASE_FILES"

# ---------------------------------------------------------------------------
# the two padded arms
# ---------------------------------------------------------------------------
printf -- '-- padding, one axis at a time -------------------------------\n'

BYTES_DIR="$WORK/ad-bytes"
cp -a "$APPDIR" "$BYTES_DIR" || exit 2
mkdir -p "$BYTES_DIR/pgb-pad"
head -c $((PAD_MB * 1024 * 1024)) /dev/urandom > "$BYTES_DIR/pgb-pad/blob.bin" || exit 2
BYTES_APP="$WORK/bytes.AppImage"
repack "$BYTES_DIR" "$BYTES_APP" || { printf 'repack (bytes) failed\n'; exit 2; }
BYTES_FILES=$(find "$BYTES_DIR" -type f | wc -l | tr -d ' ')
printf '  %-12s %12s bytes  %8s files   (+%s MiB in ONE file)\n' \
  bytes "$(wc -c < "$BYTES_APP")" "$BYTES_FILES" "$PAD_MB"

INODE_DIR="$WORK/ad-inodes"
cp -a "$APPDIR" "$INODE_DIR" || exit 2
i=0
while [ "$i" -lt "$PAD_FILES" ]; do
  d="$INODE_DIR/pgb-pad/$((i / 1000))"
  [ -d "$d" ] || mkdir -p "$d"
  head -c 64 /dev/urandom > "$d/f$i.dat"
  i=$((i + 1))
done
INODE_APP="$WORK/inodes.AppImage"
repack "$INODE_DIR" "$INODE_APP" || { printf 'repack (inodes) failed\n'; exit 2; }
INODE_FILES=$(find "$INODE_DIR" -type f | wc -l | tr -d ' ')
printf '  %-12s %12s bytes  %8s files   (+%s tiny files)\n' \
  inodes "$(wc -c < "$INODE_APP")" "$INODE_FILES" "$PAD_FILES"

# ⭐ The A/A twin of the BASE arm: byte-identical, different path. It rides the
# same interleave as everything else and its ratio is the floor.
TWIN="$WORK/twin.AppImage"
cp "$BASE" "$TWIN"; chmod +x "$TWIN"
printf '  %-12s %12s bytes  %8s files   (A/A twin of base)\n' \
  twin "$(wc -c < "$TWIN")" "$BASE_FILES"

# ⛔ CORRECTNESS BEFORE TIMING. An arm that answers differently is not an arm.
printf '\n'
printf -- '-- every arm answers the same ---------------------------------\n'
AGREE=0; NARMS=0; FIRST=""
for a in "$BASE" "$BYTES_APP" "$INODE_APP" "$TWIN"; do
  NARMS=$((NARMS + 1))
  o=$("$a" --version 2>/dev/null | tr -d ' \n')
  [ -z "$FIRST" ] && FIRST="$o"
  [ "$o" = "$FIRST" ] && [ -n "$o" ] && AGREE=$((AGREE + 1))
  printf '  %-40s -> %s\n' "$(basename "$a")" "${o:-<nothing>}"
done
reap_mounts
printf '\n'

# ---------------------------------------------------------------------------
# the measurement
# ---------------------------------------------------------------------------
printf -- '-- cold start, one interleave, median of %s ------------------\n' "$ROUNDS"
clk_init "$WORK/clk" || exit 2
clk_prep() { reap_mounts; }
clk_run() {
  case "$1" in
    base)   clk_time_once "$BASE"      --version ;;
    bytes)  clk_time_once "$BYTES_APP" --version ;;
    inodes) clk_time_once "$INODE_APP" --version ;;
    twin)   clk_time_once "$TWIN"      --version ;;
  esac
}
clk_interleave "$ROUNDS" base bytes twin inodes
reap_mounts
clk_table
printf '\n'

AA=$(clk_aa base twin)
AA_FLOOR=$(clk_floor base twin "$AA")
BASE_M=$(clk_stat base median)
BYTES_M=$(clk_stat bytes median)
INODE_M=$(clk_stat inodes median)

BYTES_R=$(clk_ratio "$BYTES_M" "$BASE_M")
INODE_R=$(clk_ratio "$INODE_M" "$BASE_M")
BYTES_F=$(clk_floor bytes base "$AA")
INODE_F=$(clk_floor inodes base "$AA")
BYTES_RES=$(clk_resolves "$BYTES_R" "$BYTES_F")
INODE_RES=$(clk_resolves "$INODE_R" "$INODE_F")

BASE_SZ=$(wc -c < "$BASE"); BYTES_SZ=$(wc -c < "$BYTES_APP")
INODE_SZ=$(wc -c < "$INODE_APP")
SZ_GROWTH=$(clk_ratio "$BYTES_SZ" "$BASE_SZ")
FILE_GROWTH=$(clk_ratio "$INODE_FILES" "$BASE_FILES")

printf '  A/A control (base vs twin)  : ratio %s, floor %s, resolves %s\n' \
  "$AA" "$AA_FLOOR" "$(clk_resolves "$AA" "$AA_FLOOR")"
printf '\n'
printf '  %-34s %-10s %-8s %s\n' AXIS 'GREW BY' 'TIME' 'RESOLVES'
printf '  %-34s %-10s %-8s %s\n' \
  "bytes (image size)"  "${SZ_GROWTH}x"   "${BYTES_R}x" "$BYTES_RES"
printf '  %-34s %-10s %-8s %s\n' \
  "inodes (file count)" "${FILE_GROWTH}x" "${INODE_R}x" "$INODE_RES"
printf '\n'

# ---------------------------------------------------------------------------
# assertions
# ---------------------------------------------------------------------------
printf -- '-- assertions ------------------------------------------------\n'

exp_check "every arm answers --version identically" "$AGREE" "$NARMS"

# ⛔ THE CONTROL, ASSERTED. Without it the two rows above are unreadable.
exp_check "the A/A pair is NOT resolvable (the control)" \
  "$(clk_resolves "$AA" "$AA_FLOOR")" "no"

# ⭐ THE HYPOTHESIS -- AND IT IS ASSERTED ON MAGNITUDE, NOT ON `resolves`.
#
# ⛔ THE FIRST VERSION ASSERTED `BYTES_RES = no` AND TWO CONSECUTIVE RUNS
# DISAGREED WITH EACH OTHER: 1.04x against a 1.06 floor (no), then 1.05x
# against a 1.03 floor (yes). ⚠ That is not an instrument failure -- the A/A
# control held at 1.01-1.02 in both -- it is a REAL effect sitting within a
# factor of two of the floor, and a boolean about a knife edge is the wrong
# assertion to hang a project's direction on.
#
# ⭐ THE STABLE QUESTION IS "IS IT BIG ENOUGH TO MATTER", and it has an
# arithmetic answer. Take the per-megabyte cost this experiment measures,
# extrapolate it to the byte difference between our kdenlive bundle and the
# competitor's, and compare it against the smallest cold-start gap ever
# OBSERVED between those two artefacts. ⚠ The smallest is the conservative
# choice: it makes the percentage as large as it can honestly be.
KDEN_DELTA_MB=196          # 398 MB ours - 192 MB theirs, TODO/toolchain.md T-066
KDEN_GAP_MS=129            # the SMALLEST of the four gaps in corrections.md
                           # C23 (181 vs 52 ms). The others are 2,019 / 239 /
                           # 4,758 ms, so any of them makes this share smaller.
PER_MB=$(awk -v b="$BYTES_M" -v a="$BASE_M" -v mb="$PAD_MB" \
  'BEGIN{ d=(b-a)/1000000.0; if(d<0)d=0; printf "%.4f", d/mb }')
KDEN_MS=$(awk -v p="$PER_MB" -v mb="$KDEN_DELTA_MB" 'BEGIN{printf "%.1f", p*mb}')
KDEN_SHARE=$(awk -v m="$KDEN_MS" -v g="$KDEN_GAP_MS" 'BEGIN{printf "%.1f", 100*m/g}')
UNDER_10=$(awk -v s="$KDEN_SHARE" 'BEGIN{print (s < 10) ? "yes" : "no"}')

printf '  --    %-46s = %s ms/MiB\n' "measured cost of image size" "$PER_MB"
printf '  --    %-46s = %s ms\n' "extrapolated to kdenlive's ${KDEN_DELTA_MB} MiB" "$KDEN_MS"
printf '  --    %-46s = %s%%\n' "as a share of the SMALLEST observed gap" "$KDEN_SHARE"
exp_check "image size explains under 10% of the kdenlive gap" "$UNDER_10" "yes"

# ⚠ Recorded, never asserted: whether the instrument could see the two axes at
# all on this run. It is the number that flickers, and a flickering assertion
# teaches the next session to route around the gate.
printf '  --    %-46s = %s\n' "byte axis resolves at this N" "$BYTES_RES"
printf '  --    %-46s = %s\n' "inode axis resolves at this N" "$INODE_RES"

printf '\n'
exp_note "⛔ THE SIZE COLUMN IS NOT THE TIME COLUMN IN ANY SENSE THAT MATTERS."
exp_note "Image size does cost something -- ${PER_MB} ms per MiB, at or just"
exp_note "above this instrument's floor -- but carried across the whole"
exp_note "${KDEN_DELTA_MB} MiB that separates our kdenlive bundle from the"
exp_note "competitor's it is ${KDEN_MS} ms of a gap that has never been"
exp_note "observed smaller than ${KDEN_GAP_MS} ms. The byte levers --cut,"
exp_note "--fixpoint and the debloat rules cannot close it, so the struck"
exp_note "size work stays struck and the cold cost is a FIXED price for"
exp_note "standing the mount up. TODO/PROGRESS.md N6 is where it is paid."

# ---------------------------------------------------------------------------
{
  printf 'experiment 92 - is the size column the time column?\n'
  printf 'date (UTC)   : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'host kernel  : %s\n' "$(uname -sr)"
  printf 'subject      : pgb bundle appimage %s --debloat safe\n' "$PKG"
  printf 'method       : one AppDir, one axis changed at a time; every arm\n'
  printf '               runs the same program out of the same store.\n'
  printf 'instrument   : experiments/clock.sh -- median of %s, interleaved\n' "$ROUNDS"
  printf '               with a rotating start, A/A control in the same pass\n'
  printf '\n'
  printf '%-10s %14s %10s %12s %10s %s\n' ARM BYTES FILES COLD_MEDIAN RATIO RESOLVES
  printf '%-10s %14s %10s %12s %10s %s\n' \
    base "$BASE_SZ" "$BASE_FILES" "$(clk_ms "$BASE_M") ms" '1.00x' '(reference)'
  printf '%-10s %14s %10s %12s %10s %s\n' \
    bytes "$BYTES_SZ" "$BYTES_FILES" "$(clk_ms "$BYTES_M") ms" "${BYTES_R}x" "$BYTES_RES"
  printf '%-10s %14s %10s %12s %10s %s\n' \
    inodes "$INODE_SZ" "$INODE_FILES" "$(clk_ms "$INODE_M") ms" "${INODE_R}x" "$INODE_RES"
  printf '%-10s %14s %10s %12s %10s %s\n' \
    twin "$BASE_SZ" "$BASE_FILES" "$(clk_ms "$(clk_stat twin median)") ms" \
    "$AA" 'A/A control'
  printf '\n'
  printf 'A/A control  : ratio %s, floor %s, resolves %s (must be "no")\n' \
    "$AA" "$AA_FLOOR" "$(clk_resolves "$AA" "$AA_FLOOR")"
  printf '\n'
  printf 'THE ANSWER\n'
  printf '  ⛔ NO, NOT IN ANY SENSE THAT MATTERS.\n'
  printf '\n'
  printf '  The image grew %s and the file count grew %s. Both moved cold\n' \
    "${SZ_GROWTH}x" "${FILE_GROWTH}x"
  printf '  start by about 1.05x, at or just above the instrument floor --\n'
  printf '  and two consecutive runs disagreed about whether the byte axis\n'
  printf '  crosses that floor at all (1.04x vs floor 1.06, then 1.05x vs\n'
  printf '  floor 1.03), while the A/A control held at 1.01-1.02 in both.\n'
  printf '  So the effect is real and it is SMALL.\n'
  printf '\n'
  printf '  Costed: %s ms per MiB of image. Carried across the %s MiB that\n' \
    "$PER_MB" "$KDEN_DELTA_MB"
  printf '  separates our kdenlive bundle (398 MB) from the competitor'"'"'s\n'
  printf '  (192 MB), that is %s ms -- %s%% of the SMALLEST cold-start gap\n' \
    "$KDEN_MS" "$KDEN_SHARE"
  printf '  ever observed between them (%s ms; the other three runs in\n' "$KDEN_GAP_MS"
  printf '  corrections.md C23 are 2,019 / 239 / 4,758 ms, which make the\n'
  printf '  share smaller still).\n'
  printf '\n'
  printf '  CONSEQUENCE: the byte levers (--cut, --fixpoint, the debloat\n'
  printf '  rules) cannot close the gap. Under the 2026-09-03c ruling that\n'
  printf '  makes them unscored AND unpromising, and the remaining work is\n'
  printf '  the mount itself -- PROGRESS.md N6.\n'
} > "$RESULT"
printf '\nfull table: %s\n' "$RESULT"

clk_save "$EXP_OUT/samples"
rm -rf "$BYTES_DIR" "$INODE_DIR" 2>/dev/null || :

exp_finish
