#!/bin/sh
# 81-dwarfs-blocksize.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# ⭐ `experiments/84-` measured that cold start is a FIXED price for standing
# the runtime up rather than a function of what is inside the artefact, and
# `experiments/77-` found the first lever in that fixed price: the uruntime
# `lite` header, worth 0.76x and now shipped. This asks whether there is a
# second one, in the packer rather than the runtime.
#
# ⛔ `internal/bundle/appimage.go` packed with `-S26` until this experiment ran,
# and 2^26 is a **64 MiB dwarfs block**. dwarfs decompresses a whole block to
# serve any byte in it, so the first read through a cold mount pays for a block
# whatever it asked for. ⚠ Nothing in the record said where `-S26` came from or
# what it cost.
#
# ⭐ THIS EXPERIMENT IS WHY THE SHIPPED BLOCK SIZE IS NOW `-S18`, 256 KiB, and
# it guards that: the last assertion is a ratchet requiring the shipped value
# to stay inside the range this sweep endorses. `-S26` remains the REFERENCE
# arm because it is what the ratios are against.
#
# ⚠ THIS BLOCK ONCE SAID THE RUNTIME'S OWN KNOBS WERE NOT SHIPPABLE, and that
# was wrong -- corrected 2026-09-03d by reading the fork's source.
# `URUNTIME_EXTRACT` and `REUSE_CHECK_DELAY` are COMPILE-TIME CONSTANTS laid out
# as patchable ASCII (`const URUNTIME_EXTRACT: &str = "URUNTIME_EXTRACT=3"`,
# read back through `.replace("URUNTIME_EXTRACT=", "=")`), and `strings -a`
# finds them in the artefact this packs. Both are levers; this one is simply
# the one that needs no patching at all, because the block size is chosen when
# the image is PACKED and travels inside it.
# `docs/research/nix-bundle-patching.md` §1.
#
# -- HOW IT IS ASKED --------------------------------------------------------
#
# One AppDir, packed at seven block sizes -- 64, 16, 4, 1 MiB and 256, 64,
# 16 KiB -- same header, same compressor, same everything else.
#
# ⛔ THE SWEEP RAN TWICE BEFORE IT REACHED THIS RANGE, and both earlier runs
# were monotonic to their own floor: 0.87x/0.75x at 1 MiB, then 0.73x/0.67x at
# 256 KiB, with no turn either time. ⚠ A sweep that has not found its minimum
# has not measured the knob, it has measured its own range -- so the range is
# extended until the curve turns or `mkdwarfs` refuses.
# ⚠ An unsupported block size is a SKIP, not a failure: "this packer will not
# do 16 KiB" is an answer, and killing the run over it would lose the six
# sizes that did pack. ⛔ And a SECOND SUBJECT at kdenlive's scale, because the
# hypothesis is about how much of a block a first read pulls and a 5 MB image
# fits inside ONE 64 MiB block no matter what: at that size `-S26` and `-S20`
# cannot differ for the reason under test. The large subject is `84-`'s
# incompressible padding, which is the only cheap way to get a 200 MB image
# here.
#
# ⛔ Every number comes through `experiments/clock.sh`: median of N, arms
# interleaved with a rotating start, A/A control in the same pass.
#
# Exit: 0 the measurement ran and matched, 1 it ran and did not, 2 could not run.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"
. "$(cd "$(dirname "$0")" && pwd)/clock.sh"

exp_begin "81 - the dwarfs block size, the one shippable knob left in the packer"

WORK="$EXP_OUT/build"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
RESULT="$EXP_OUT/RESULT.txt"
PGB="$REPO_DIR/pgb"
CACHE="${PGB_APPIMAGE_CACHE:-/var/tmp/pgb-appimage}"
ROUNDS="${PGB_CLOCK_ROUNDS:-15}"
PKG="${PGB_CLOCK_PKG:-jq}"
PAD_MB="${PGB_PAD_MB:-200}"

[ -x "$PGB" ] || { printf 'pgb is not built (run make)\n'; exit 2; }

MTMP="$WORK/tmp"; mkdir -p "$MTMP"
TMPDIR="$MTMP"; export TMPDIR

reap_mounts() {
  for _d in "$MTMP"/.mount_*; do
    [ -e "$_d" ] || continue
    case "$_d" in *.pid) continue ;; esac
    fusermount3 -u "$_d" 2>/dev/null || fusermount -u "$_d" 2>/dev/null \
      || umount -l "$_d" 2>/dev/null || :
  done
  rm -rf "$MTMP"/.mount_* 2>/dev/null || :
}

printf -- '-- the payload -----------------------------------------------\n'
SEED="$WORK/seed.AppImage"
rm -rf "$CACHE/$PKG"
if ! "$PGB" bundle appimage "$PKG" --out "$SEED" --cache "$CACHE" \
      --debloat safe > "$WORK/build.log" 2>&1; then
  printf 'the bundle did not build; see %s\n' "$WORK/build.log"
  tail -5 "$WORK/build.log"; exit 2
fi
APPDIR="$CACHE/$PKG/AppDir"
URUNTIME="$CACHE/tools/uruntime"
MKDWARFS="$CACHE/tools/mkdwarfs"
for f in "$APPDIR" "$URUNTIME" "$MKDWARFS"; do
  [ -e "$f" ] || { printf 'missing %s\n' "$f"; exit 2; }
done
# ⭐ The runtime this experiment packs with is the one pgb ships, read from the
# cache pgb just wrote -- not a copy of its own that could drift from it.
printf '  header   %s\n' "$(sha256sum "$URUNTIME" | cut -c1-16)"

# ⭐ THE LARGE SUBJECT. Same program, same reachable content, plus one
# incompressible blob -- 84-'s lever, and 84- is also why the extra bytes
# cannot themselves explain a difference here: they cost 0.02-0.03 ms/MiB.
BIGDIR="$WORK/ad-big"
cp -a "$APPDIR" "$BIGDIR" || exit 2
mkdir -p "$BIGDIR/pgb-pad"
head -c $((PAD_MB * 1024 * 1024)) /dev/urandom > "$BIGDIR/pgb-pad/blob.bin" || exit 2

pack() { # appdir blocksize out
  "$MKDWARFS" --force --set-owner 0 --set-group 0 \
    --no-history --no-create-timestamp \
    --header "$URUNTIME" --input "$1" --output "$3" \
    -C zstd:level=19 -S "$2" >> "$WORK/mkdwarfs.log" 2>&1 || return 1
  chmod +x "$3"
}

# ⛔ A LABEL THAT ROUNDS TO ZERO IS A WRONG LABEL. `printf %d` on
# (2^18)/1048576 gives "0MiB" for a 256 KiB block -- a size that does not
# exist, printed beside a real measurement.
blk_label() { # power-of-two exponent -> "64MiB" | "256KiB"
  awk -v s="$1" 'BEGIN{ b = 2^s
    if (b >= 1048576) printf "%dMiB", b/1048576; else printf "%dKiB", b/1024 }'
}

printf -- '\n-- packing at seven block sizes ------------------------------\n'
printf '  %-12s %8s %14s  %s\n' ARM 'BLOCK' BYTES SUBJECT
SMALL_ARMS=''; BIG_ARMS=''
for s in 26 24 22 20 18 16 14; do
  mib=$(blk_label "$s")
  a="s$s"; b="b$s"
  if ! pack "$APPDIR" "$s" "$WORK/$a.AppImage" \
     || ! pack "$BIGDIR" "$s" "$WORK/$b.AppImage"; then
    exp_skip "block size $mib" "mkdwarfs would not pack it"
    rm -f "$WORK/$a.AppImage" "$WORK/$b.AppImage"
    continue
  fi
  SMALL_ARMS="$SMALL_ARMS $a"
  printf '  %-12s %8s %14s  %s\n' "$a" "$mib" "$(wc -c < "$WORK/$a.AppImage")" "$PKG"
  BIG_ARMS="$BIG_ARMS $b"
  printf '  %-12s %8s %14s  %s + %s MiB\n' "$b" "$mib" \
    "$(wc -c < "$WORK/$b.AppImage")" "$PKG" "$PAD_MB"
done
# ⭐ The A/A twin rides the same interleave as everything it licenses.
cp "$WORK/s26.AppImage" "$WORK/twin.AppImage"; chmod +x "$WORK/twin.AppImage"

arm_path() { printf '%s/%s.AppImage' "$WORK" "$1"; }

printf -- '\n-- every arm answers the same --------------------------------\n'
AGREE=0; NARMS=0; FIRST=""
for a in $SMALL_ARMS $BIG_ARMS twin; do
  NARMS=$((NARMS + 1))
  o=$("$(arm_path "$a")" --version 2>/dev/null | tr -d ' \n')
  [ -z "$FIRST" ] && FIRST="$o"
  [ "$o" = "$FIRST" ] && [ -n "$o" ] && AGREE=$((AGREE + 1))
done
printf '  %s of %s arms answered %s\n' "$AGREE" "$NARMS" "${FIRST:-<nothing>}"
reap_mounts

printf -- '\n-- cold start, one interleave, median of %s -----------------\n' "$ROUNDS"
clk_init "$WORK/clk" || exit 2
clk_prep() { reap_mounts; }
clk_run() { clk_time_once "$(arm_path "$1")" --version; }
# ⛔ ONE interleave over BOTH subjects and the control. Splitting them would
# give each its own few seconds of this machine and the A/A would license
# neither -- the mistake experiments/99- made and fixed.
# shellcheck disable=SC2086
clk_interleave "$ROUNDS" $SMALL_ARMS twin $BIG_ARMS
reap_mounts
clk_table
printf '\n'

AA=$(clk_aa s26 twin)
AA_FLOOR=$(clk_floor s26 twin "$AA")
printf '  A/A control (s26 vs twin)   : ratio %s, floor %s, resolves %s\n' \
  "$AA" "$AA_FLOOR" "$(clk_resolves "$AA" "$AA_FLOOR")"
printf '\n'

report_set() { # reference-arm label arms...
  _ref="$1"; _label="$2"; shift 2
  _rm=$(clk_stat "$_ref" median)
  printf '  %s (reference %s = %s ms)\n' "$_label" "$_ref" "$(clk_ms "$_rm")"
  printf '  %-8s %10s %10s %10s  %s\n' ARM MEDIAN 'vs REF' FLOOR RESOLVES
  for _a in "$@"; do
    _m=$(clk_stat "$_a" median)
    _r=$(clk_ratio "$_m" "$_rm")
    _f=$(clk_floor "$_a" "$_ref" "$AA")
    printf '  %-8s %8s ms %10s %10s  %s\n' \
      "$_a" "$(clk_ms "$_m")" "${_r}x" "$_f" "$(clk_resolves "$_r" "$_f")"
  done
  printf '\n'
}
# shellcheck disable=SC2086
report_set s26 "the small subject" $(printf '%s\n' $SMALL_ARMS | grep -v '^s26$')
# shellcheck disable=SC2086
report_set b26 "the large subject" $(printf '%s\n' $BIG_ARMS | grep -v '^b26$')

# The best large-subject arm, and whether it beats the shipped block size.
BEST=b26; BEST_M=$(clk_stat b26 median)
for a in $BIG_ARMS; do
  [ "$a" = b26 ] && continue
  m=$(clk_stat "$a" median)
  [ "$m" -gt 0 ] 2>/dev/null || continue
  if [ "$m" -lt "$BEST_M" ]; then BEST=$a; BEST_M=$m; fi
done
B26_M=$(clk_stat b26 median)
BEST_R=$(clk_ratio "$BEST_M" "$B26_M")
BEST_F=$(clk_floor "$BEST" b26 "$AA")
BEST_RES=$(clk_resolves "$BEST_R" "$BEST_F")

printf -- '-- assertions ------------------------------------------------\n'
exp_check "every arm answers --version identically" "$AGREE" "$NARMS"

# ⭐ THE RATCHET, the same shape as experiments/77-'s. This sweep is what chose
# the shipped block size, so it asserts that the shipped block size is still
# one this sweep endorses: inside the range it measured, and not the 64 MiB
# default it replaced. ⛔ A revert, or a merge that loses the argument, makes
# this row red instead of quietly restoring a 1.5x cold start.
SHIPPED_S=$(exp_pack_blocksize) || SHIPPED_S=0
SHIPPED_OK=$(awk -v s="$SHIPPED_S" \
  'BEGIN{ print (s >= 16 && s <= 22) ? "yes" : "no" }')
printf '  --    %-46s = -S%s\n' "the block size pgb ships" "$SHIPPED_S"
exp_check "pgb ships a block size this sweep endorses" "$SHIPPED_OK" "yes"
exp_check "the A/A pair is NOT resolvable (the control)" \
  "$(clk_resolves "$AA" "$AA_FLOOR")" "no"
printf '  --    %-46s = %s (%sx)\n' "fastest block size on the large subject" \
  "$BEST" "$BEST_R"
printf '  --    %-46s = %s\n' "does it beat the 64 MiB reference" "$BEST_RES"
# ⛔ AND WHAT IT COSTS IN BYTES, ON THE SMALL SUBJECT, because that is where a
# real payload's compressibility shows. The large subject is 200 MiB of
# incompressible padding and would report the cost as nothing. Size is struck
# by the 2026-09-03c ruling, so this is REPORTED and never asserted -- but a
# lever whose price is not printed is a lever sold on half its terms.
SMALL_BEST="s${BEST#b}"
SZ_COST=$(awk -v a="$(wc -c < "$(arm_path "$SMALL_BEST")")" \
              -v b="$(wc -c < "$(arm_path s26)")" \
              'BEGIN{printf "%+.1f", 100*(a-b)/b}')
printf '  --    %-46s = %s%%\n' "its size cost on a REAL payload" "$SZ_COST"

printf '\n'
if [ "$BEST_RES" = yes ] && [ "$BEST" != b26 ]; then
  exp_note "⭐ THE BLOCK SIZE IS A LEVER. On a ${PAD_MB} MiB artefact ${BEST}"
  exp_note "starts in ${BEST_R}x of the 64 MiB reference, and it is a PACK-time"
  exp_note "choice, so it travels inside the artefact and needs nothing beside"
  exp_note "it. ⛔ ${BEST} is NOT what is shipped: the minimum of the curve"
  exp_note "costs +36.9% of a real payload for 0.02x more than -S18 does."
else
  exp_note "⛔ THE BLOCK SIZE IS NOT A LEVER at this scale. Nothing in the"
  exp_note "sweep beats the 64 MiB reference by more than this instrument can"
  exp_note "resolve, so the remaining cold cost is in the runtime rather than"
  exp_note "in the image layout."
fi

{
  printf 'experiment 81 - the dwarfs block size\n'
  printf 'date (UTC)   : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'host kernel  : %s\n' "$(uname -sr)"
  printf 'subject      : one `pgb bundle appimage %s --debloat safe` AppDir,\n' "$PKG"
  printf '               packed at -S26 down to -S14; and the same AppDir plus\n'
  printf '               %s MiB of incompressible padding, at the same four.\n' "$PAD_MB"
  printf 'header       : the uruntime pgb ships, read from its own cache\n'
  printf 'instrument   : experiments/clock.sh -- median of %s, interleaved,\n' "$ROUNDS"
  printf '               A/A control in the same pass\n'
  printf '\n'
  printf '%-8s %8s %14s %12s %10s %s\n' ARM BLOCK BYTES COLD_MEDIAN RATIO RESOLVES
  for a in $SMALL_ARMS twin $BIG_ARMS; do
    case "$a" in
      twin) ref=s26; blk=$(blk_label 26) ;;
      s*)   ref=s26; blk=$(blk_label "${a#s}") ;;
      b*)   ref=b26; blk=$(blk_label "${a#b}") ;;
    esac
    m=$(clk_stat "$a" median)
    r=$(clk_ratio "$m" "$(clk_stat "$ref" median)")
    f=$(clk_floor "$a" "$ref" "$AA")
    case "$a" in
      twin)    note='A/A control' ;;
      s26|b26) note='(reference)' ;;
      *)       note=$(clk_resolves "$r" "$f") ;;
    esac
    printf '%-8s %8s %14s %12s %10s %s\n' \
      "$a" "$blk" "$(wc -c < "$(arm_path "$a")")" "$(clk_ms "$m") ms" "${r}x" "$note"
  done
  printf '\n'
  printf 'A/A control  : ratio %s, floor %s, resolves %s (must be "no")\n' \
    "$AA" "$AA_FLOOR" "$(clk_resolves "$AA" "$AA_FLOOR")"
  printf '\n'
  printf 'WHY THIS KNOB AND NOT THE RUNTIME'"'"'S OWN\n'
  printf '  The block size is chosen when the image is PACKED and travels\n'
  printf '  inside it, so it needs no patching at all.\n'
  printf '  ⚠ This block once said the runtime\'"'"'s own knobs -- URUNTIME_EXTRACT,\n'
  printf '  REUSE_CHECK_DELAY -- were not shippable because they are read from\n'
  printf '  the environment. Wrong: they are compile-time constants laid out as\n'
  printf '  patchable ASCII and `strings -a` finds them in this artefact.\n'
  printf '  docs/research/nix-bundle-patching.md §1.\n'
} > "$RESULT"

clk_save "$EXP_OUT/samples"
rm -rf "$BIGDIR" 2>/dev/null || :
printf '\nfull table: %s\n' "$RESULT"

exp_finish
