#!/bin/sh
# 77-uruntime-header.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# ⭐ `experiments/92-` measured that the bundler's cold start is NOT a function
# of what is inside the artefact: a 29.6x image and a 138x file count each move
# it by about 1.05x, and extrapolated across the whole 196 MiB separating our
# kdenlive bundle from the competitor's that is 4.8 ms of a gap never observed
# smaller than 129 ms. The cost is a FIXED price for standing the runtime up.
#
# ⛔ SO THE RUNTIME IS THE SUBJECT, AND OURS IS NOT THE ONE WE ARE MEASURED
# AGAINST. `internal/bundle/appimage.go` pins uruntime **v0.5.6**, the FULL
# build; `experiments/86-`, which stages the competitor's own toolchain from
# `references/pkgforge-dev__Anylinux-AppImages`, stages uruntime **v0.5.9
# LITE** and dwarfs **0.15.6** against our 0.14.1. The lite runtime is
# 1,487,344 bytes against the full one's 3,068,400 -- less than half.
#
# ⚠ Nothing in the record noticed. `docs/comparison.md` and T-066 attribute the
# jq cold-start gap to the bundle; two of the three tools in the delivery path
# are a different build from the competitor's.
#
# -- HOW IT IS ASKED --------------------------------------------------------
#
# ⭐ ONE AppDir, packed five ways, each differing from the one before in
# EXACTLY ONE component. Same closure, same program, same libraries, same
# mkdwarfs settings -- so a difference is attributable to the component that
# changed and to nothing else.
#
#   ours       v0.5.6 full header, dwarfs 0.14.1     <- what pgb ships today
#   v059full   v0.5.9 full header, dwarfs 0.14.1     <- the version bump alone
#   v059lite   v0.5.9 LITE header, dwarfs 0.14.1     <- lite vs full alone
#   field      v0.5.9 lite header, dwarfs 0.15.6     <- the competitor's stack
#   twin       a byte copy of `ours`                 <- ⭐ the A/A control
#
# ⛔ Every number comes through `experiments/clock.sh`: median of N, arms
# interleaved with a rotating start, and the A/A control in the same pass.
# `experiments/99-` is why -- the instrument this replaces reported a warm
# start in its cold column.
#
# Exit: 0 the measurement ran and matched, 1 it ran and did not, 2 could not run.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"
. "$(cd "$(dirname "$0")" && pwd)/clock.sh"

exp_begin "77 - the bundler's runtime, and it is not the one we are measured against"

WORK="$EXP_OUT/build"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
RESULT="$EXP_OUT/RESULT.txt"
PGB="$REPO_DIR/pgb"
CACHE="${PGB_APPIMAGE_CACHE:-/var/tmp/pgb-appimage}"
ROUNDS="${PGB_CLOCK_ROUNDS:-21}"
PKG="${PGB_CLOCK_PKG:-jq}"

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

# ---------------------------------------------------------------------------
# ⛔ EVERY TOOL PINNED BY SHA-256, IN THIS FILE.
# `methodology/experiments.md`: a fetched input that is not pinned makes the
# result describe whatever the tag pointed at that day. ⚠ The `ours` row is
# pinned here too even though `pgb` fetches it itself -- if the two ever
# disagree, this experiment is comparing something `pgb` does not ship, and
# the assertion below is what says so.
# ---------------------------------------------------------------------------
U056_URL="https://github.com/VHSgunzo/uruntime/releases/download/v0.5.6/uruntime-appimage-dwarfs-x86_64"
U056_SHA=6416a112fac1e9983b1c0738cd140f17dc1205f515b9bdb36b4607ef98ee2a70
U059_URL="https://github.com/VHSgunzo/uruntime/releases/download/v0.5.9/uruntime-appimage-dwarfs-x86_64"
U059_SHA=cb89770e27c692194d4ac5ae37ffba8c8986f763264333ef90e250d44f3c4c80
ULITE_URL="https://github.com/VHSgunzo/uruntime/releases/download/v0.5.9/uruntime-appimage-dwarfs-lite-x86_64"
ULITE_SHA=cef962c299f2fa19b2b3cdf2fa1565ee8541796cc89b9a97a591f94041e8b083
MKD156_URL="https://github.com/mhx/dwarfs/releases/download/v0.15.6/dwarfs-universal-0.15.6-Linux-x86_64"
MKD156_SHA=50891c38ba359db8271819a6cbf6aaa8068681523f0c4f2b8242007a45edaa28

fetch_pinned() { # url sha dest
  [ -f "$3" ] && [ "$(sha256sum "$3" 2>/dev/null | cut -d' ' -f1)" = "$2" ] && return 0
  # ⚠ Direct first, then the mirror TODO/RULES.md names for a source that 401s
  # or 403s. Neither is trusted: the sha256 decides.
  curl -fsSL --max-time 300 -o "$3" "$1" 2>/dev/null \
    || curl -fsSL --max-time 300 -o "$3" "https://api.rv.pkgforge.dev/$1" 2>/dev/null \
    || return 1
  [ "$(sha256sum "$3" 2>/dev/null | cut -d' ' -f1)" = "$2" ] || return 1
}

printf -- '-- staging the runtimes, each pinned by sha256 ----------------\n'
command -v curl >/dev/null 2>&1 || { printf 'no curl\n'; exit 2; }
STAGED=yes
fetch_pinned "$U056_URL"   "$U056_SHA"   "$WORK/u056"   || STAGED=no
fetch_pinned "$U059_URL"   "$U059_SHA"   "$WORK/u059"   || STAGED=no
fetch_pinned "$ULITE_URL"  "$ULITE_SHA"  "$WORK/ulite"  || STAGED=no
# ⛔ THE NAME IS THE DISPATCH. `dwarfs-universal` picks its tool from
# basename(argv[0]) and prints its help when the name matches nothing -- so a
# copy called `mkd156` silently packs NOTHING and the pack step fails with a
# missing output rather than an error. ⚠ Same class as the onelf entrypoint
# trap experiments/90- documents: a silent fallback turns a naming mistake
# into a claim about somebody else's tool. It goes in its own directory under
# the name the tool answers to.
mkdir -p "$WORK/d156"
fetch_pinned "$MKD156_URL" "$MKD156_SHA" "$WORK/d156/mkdwarfs" || STAGED=no
if [ "$STAGED" != yes ]; then
  printf 'could not stage the pinned runtimes (network, or a moved release)\n'
  exit 2
fi
chmod +x "$WORK/d156/mkdwarfs"
for f in u056 u059 ulite d156/mkdwarfs; do
  printf '  %-16s %10s bytes\n' "$f" "$(wc -c < "$WORK/$f")"
done

# ---------------------------------------------------------------------------
# the AppDir, built the way pgb builds it
# ---------------------------------------------------------------------------
printf -- '\n-- the payload -----------------------------------------------\n'
SEED="$WORK/seed.AppImage"
rm -rf "$CACHE/$PKG"
if ! "$PGB" bundle appimage "$PKG" --out "$SEED" --cache "$CACHE" \
      --debloat safe > "$WORK/build.log" 2>&1; then
  printf 'the bundle did not build; see %s\n' "$WORK/build.log"
  tail -5 "$WORK/build.log"
  exit 2
fi
APPDIR="$CACHE/$PKG/AppDir"
MKD141="$CACHE/tools/mkdwarfs"
SHIPPED="$CACHE/tools/uruntime"
[ -d "$APPDIR" ] && [ -x "$MKD141" ] || { printf 'no AppDir or mkdwarfs\n'; exit 2; }

# ⭐ THE ASSERTION THAT MAKES THE `ours` ARM MEAN WHAT IT SAYS: the header this
# experiment packs as `ours` must be byte-identical to the one `pgb` just
# fetched for itself. Without this the row is about a runtime nobody ships.
SHIPPED_SHA=$(sha256sum "$SHIPPED" 2>/dev/null | cut -d' ' -f1)
printf '  %-28s %s\n' "pgb ships uruntime" "${SHIPPED_SHA:-<absent>}"
printf '  %-28s %s\n' "this experiment's 'ours'" "$U056_SHA"

# ---------------------------------------------------------------------------
# pack it five ways
# ---------------------------------------------------------------------------
printf -- '\n-- packing, one component changed at a time ------------------\n'
pack() { # mkdwarfs header out
  "$1" --force --set-owner 0 --set-group 0 \
    --no-history --no-create-timestamp \
    --header "$2" --input "$APPDIR" --output "$3" \
    -C zstd:level=19 -S26 >> "$WORK/mkdwarfs.log" 2>&1 || return 1
  chmod +x "$3"
}
pack "$MKD141"      "$WORK/u056"  "$WORK/ours.AppImage"     || { printf 'pack ours failed\n';     exit 2; }
pack "$MKD141"      "$WORK/u059"  "$WORK/v059full.AppImage" || { printf 'pack v059full failed\n'; exit 2; }
pack "$MKD141"      "$WORK/ulite" "$WORK/v059lite.AppImage" || { printf 'pack v059lite failed\n'; exit 2; }
pack "$WORK/d156/mkdwarfs" "$WORK/ulite" "$WORK/field.AppImage"    || { printf 'pack field failed\n';    exit 2; }
cp "$WORK/ours.AppImage" "$WORK/twin.AppImage"; chmod +x "$WORK/twin.AppImage"

arm_path() {
  case "$1" in
    ours)     printf '%s' "$WORK/ours.AppImage" ;;
    v059full) printf '%s' "$WORK/v059full.AppImage" ;;
    v059lite) printf '%s' "$WORK/v059lite.AppImage" ;;
    field)    printf '%s' "$WORK/field.AppImage" ;;
    twin)     printf '%s' "$WORK/twin.AppImage" ;;
  esac
}
ARMS='ours v059full v059lite field twin'
for a in $ARMS; do
  printf '  %-10s %12s bytes\n' "$a" "$(wc -c < "$(arm_path "$a")")"
done

# ⛔ CORRECTNESS BEFORE TIMING. A runtime that starts faster and runs a
# different program is not a faster runtime.
printf -- '\n-- every arm answers the same --------------------------------\n'
AGREE=0; NARMS=0; FIRST=""
for a in $ARMS; do
  NARMS=$((NARMS + 1))
  o=$("$(arm_path "$a")" --version 2>/dev/null | tr -d ' \n')
  [ -z "$FIRST" ] && FIRST="$o"
  [ "$o" = "$FIRST" ] && [ -n "$o" ] && AGREE=$((AGREE + 1))
  printf '  %-10s -> %s\n' "$a" "${o:-<nothing>}"
done
reap_mounts

# ---------------------------------------------------------------------------
# the measurement
# ---------------------------------------------------------------------------
printf -- '\n-- cold start, one interleave, median of %s -----------------\n' "$ROUNDS"
clk_init "$WORK/clk" || exit 2
clk_prep() { reap_mounts; }
clk_run() { clk_time_once "$(arm_path "$1")" --version; }
clk_interleave "$ROUNDS" ours v059full twin v059lite field
reap_mounts
clk_table
printf '\n'

AA=$(clk_aa ours twin)
AA_FLOOR=$(clk_floor ours twin "$AA")
OURS_M=$(clk_stat ours median)
printf '  A/A control (ours vs twin)  : ratio %s, floor %s, resolves %s\n' \
  "$AA" "$AA_FLOOR" "$(clk_resolves "$AA" "$AA_FLOOR")"
printf '\n'
printf '  %-10s %10s %10s %10s  %s\n' ARM MEDIAN 'vs OURS' FLOOR RESOLVES
BEST=""; BEST_M=""
for a in v059full v059lite field; do
  m=$(clk_stat "$a" median)
  r=$(clk_ratio "$m" "$OURS_M")
  f=$(clk_floor "$a" ours "$AA")
  res=$(clk_resolves "$r" "$f")
  printf '  %-10s %8s ms %10s %10s  %s\n' "$a" "$(clk_ms "$m")" "${r}x" "$f" "$res"
  if [ "$res" = yes ] && [ "$m" -lt "$OURS_M" ] 2>/dev/null; then
    if [ -z "$BEST_M" ] || [ "$m" -lt "$BEST_M" ]; then BEST=$a; BEST_M=$m; fi
  fi
done
printf '\n'

LITE_M=$(clk_stat v059lite median)
FIELD_M=$(clk_stat field median)
LITE_R=$(clk_ratio "$LITE_M" "$OURS_M")
FIELD_R=$(clk_ratio "$FIELD_M" "$OURS_M")
FIELD_RES=$(clk_resolves "$FIELD_R" "$(clk_floor field ours "$AA")")

# ---------------------------------------------------------------------------
# assertions
# ---------------------------------------------------------------------------
printf -- '-- assertions ------------------------------------------------\n'
exp_check "every arm answers --version identically" "$AGREE" "$NARMS"
exp_check "the A/A pair is NOT resolvable (the control)" \
  "$(clk_resolves "$AA" "$AA_FLOOR")" "no"

# ⭐ THE FINDING THIS EXPERIMENT EXISTS FOR, asserted as a fact about the
# TREE rather than about the clock: pgb and the competitor's staged toolchain
# do not run the same runtime. ⛔ It is expected to hold until somebody
# changes cfg-equivalent constants in internal/bundle/appimage.go, at which
# point this assertion fires and the entry is closed.
exp_check "pgb ships the runtime this experiment calls 'ours'" \
  "$SHIPPED_SHA" "$U056_SHA"

printf '  --    %-46s = %s\n' "the competitor's stack, vs ours" "${FIELD_R}x"
printf '  --    %-46s = %s\n' "does it resolve" "$FIELD_RES"

printf '\n'
if [ "$FIELD_RES" = yes ] && [ "$FIELD_M" -lt "$OURS_M" ] 2>/dev/null; then
  exp_note "⭐ THE RUNTIME IS A REAL LEVER AND IT IS FREE. The competitor's"
  exp_note "own stack -- uruntime v0.5.9 lite + dwarfs 0.15.6 -- starts the"
  exp_note "IDENTICAL AppDir in ${FIELD_R}x of what ours does. Nothing about"
  exp_note "the closure, the sweep or the debloat rules changed; only the"
  exp_note "three constants in internal/bundle/appimage.go."
elif [ "$FIELD_RES" = no ]; then
  exp_note "⛔ THE RUNTIME IS NOT THE LEVER. The competitor's own stack starts"
  exp_note "the identical AppDir at ${FIELD_R}x, which this instrument cannot"
  exp_note "tell from 1.00x. So the jq gap docs/comparison.md records is not"
  exp_note "in uruntime or dwarfs, and the next place to look is the closure"
  exp_note "the sweep produces -- or the chroot the comparison runs in."
else
  exp_note "⚠ The competitor's stack is SLOWER here (${FIELD_R}x), which is a"
  exp_note "result: the runtime pin is not costing us anything and the gap"
  exp_note "docs/comparison.md records is somewhere else entirely."
fi

# ---------------------------------------------------------------------------
{
  printf 'experiment 77 - the bundler'"'"'s runtime\n'
  printf 'date (UTC)   : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'host kernel  : %s\n' "$(uname -sr)"
  printf 'subject      : one `pgb bundle appimage %s --debloat safe` AppDir,\n' "$PKG"
  printf '               packed five ways. Only the runtime component changes.\n'
  printf 'instrument   : experiments/clock.sh -- median of %s, interleaved,\n' "$ROUNDS"
  printf '               A/A control in the same pass\n'
  printf '\n'
  printf '%-10s %14s %12s %10s %10s %s\n' ARM BYTES COLD_MEDIAN 'vs OURS' FLOOR RESOLVES
  for a in $ARMS; do
    m=$(clk_stat "$a" median)
    r=$(clk_ratio "$m" "$OURS_M")
    f=$(clk_floor "$a" ours "$AA")
    case "$a" in
      ours) note='(reference) v0.5.6 full + dwarfs 0.14.1' ;;
      twin) note='A/A control' ;;
      *)    note=$(clk_resolves "$r" "$f") ;;
    esac
    printf '%-10s %14s %12s %10s %10s %s\n' \
      "$a" "$(wc -c < "$(arm_path "$a")")" "$(clk_ms "$m") ms" "${r}x" "$f" "$note"
  done
  printf '\n'
  printf 'WHAT EACH ARM CHANGES\n'
  printf '  ours       uruntime v0.5.6 FULL  + mkdwarfs 0.14.1   <- pgb today\n'
  printf '  v059full   uruntime v0.5.9 FULL  + mkdwarfs 0.14.1   <- version only\n'
  printf '  v059lite   uruntime v0.5.9 LITE  + mkdwarfs 0.14.1   <- lite only\n'
  printf '  field      uruntime v0.5.9 LITE  + mkdwarfs 0.15.6   <- the stack\n'
  printf '                                                          experiments/86-\n'
  printf '                                                          stages for the\n'
  printf '                                                          COMPETITOR arm\n'
  printf '  twin       a byte copy of ours                       <- A/A control\n'
  printf '\n'
  printf 'uruntime v0.5.6 full  3,039,728 B\n'
  printf 'uruntime v0.5.9 full  3,068,400 B\n'
  printf 'uruntime v0.5.9 lite  1,487,344 B   <- less than half\n'
  printf '\n'
  printf 'A/A control  : ratio %s, floor %s, resolves %s (must be "no")\n' \
    "$AA" "$AA_FLOOR" "$(clk_resolves "$AA" "$AA_FLOOR")"
} > "$RESULT"

clk_save "$EXP_OUT/samples"
printf '\nfull table: %s\n' "$RESULT"

exp_finish
