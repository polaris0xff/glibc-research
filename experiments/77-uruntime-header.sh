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
# ⛔ SO THE RUNTIME IS THE SUBJECT -- AND UNTIL 2026-09-03d OURS WAS NOT THE
# ONE WE WERE MEASURED AGAINST. `internal/bundle/appimage.go` pinned uruntime
# **v0.5.6**, the FULL build, and dwarfs **0.14.1**; `experiments/86-`, which
# stages the competitor's own toolchain from
# `references/pkgforge-dev__Anylinux-AppImages`, stages uruntime **v0.5.9 LITE**
# and dwarfs **0.15.6**. The lite runtime is 1,487,344 bytes against the full
# one's 3,068,400 -- less than half.
#
# ⚠ Nothing in the record noticed. `docs/comparison.md` and T-066 attributed
# the jq cold-start gap to the bundle, while two of the three tools in the
# delivery path were a different build from the competitor's.
#
# ⭐ THIS EXPERIMENT IS WHY THE PIN MOVED, and it now guards the move: the
# final assertion is a ratchet requiring `pgb` to ship the FAST runtime.
#
# -- HOW IT IS ASKED --------------------------------------------------------
#
# ⭐ ONE AppDir, packed five ways, each differing from the one before in
# EXACTLY ONE component. Same closure, same program, same libraries, same
# mkdwarfs settings -- so a difference is attributable to the component that
# changed and to nothing else.
#
#   ours       v0.5.6 full header, dwarfs 0.14.1     <- what pgb shipped
#                                                    until 2026-09-03d
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
# ⛔ 0.14.1 IS PINNED HERE TOO, AND IT HAS TO BE. The first version borrowed
# `$CACHE/tools/mkdwarfs` for the three 0.14.1 arms on the reasoning that it
# was what `pgb` shipped. ⚠ The moment the pin in appimage.go moved to 0.15.6
# that path became 0.15.6, so three arms silently changed meaning and `field`
# stopped differing from `v059lite` by anything at all -- they came back 63.2
# and 63.6 ms, which reads as a measurement and was an identity.
MKD141_URL="https://github.com/mhx/dwarfs/releases/download/v0.14.1/dwarfs-universal-0.14.1-Linux-x86_64"
MKD141_SHA=f3a117fd6d5b7304944b199af7fdb8086a48c509ea2e9832255d8f9a54c98587

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
mkdir -p "$WORK/d156" "$WORK/d141"
fetch_pinned "$MKD156_URL" "$MKD156_SHA" "$WORK/d156/mkdwarfs" || STAGED=no
fetch_pinned "$MKD141_URL" "$MKD141_SHA" "$WORK/d141/mkdwarfs" || STAGED=no
if [ "$STAGED" != yes ]; then
  printf 'could not stage the pinned runtimes (network, or a moved release)\n'
  exit 2
fi
chmod +x "$WORK/d156/mkdwarfs" "$WORK/d141/mkdwarfs"
for f in u056 u059 ulite d141/mkdwarfs d156/mkdwarfs; do
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
MKD141="$WORK/d141/mkdwarfs"
SHIPPED="$CACHE/tools/uruntime"
[ -d "$APPDIR" ] || { printf 'no AppDir\n'; exit 2; }

# ⭐ WHAT pgb ACTUALLY FETCHED FOR ITSELF, read from the cache it just wrote.
# The ratchet at the bottom compares it against the runtime measured fastest.
SHIPPED_SHA=$(sha256sum "$SHIPPED" 2>/dev/null | cut -d' ' -f1)
printf '  %-28s %s\n' "pgb ships uruntime" "${SHIPPED_SHA:-<absent>}"
printf '  %-28s %s\n' "the fast one (v0.5.9 lite)" "$ULITE_SHA"

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
# ⛔ THE ELEVEN -- a runtime that is 24% faster and does not start on Alpine
# is not a faster runtime, it is a regression with a good benchmark.
#
# ⚠ SCOPE, STATED. This arm asks ONE question: does the artefact still run.
# It does NOT re-measure §3 criterion 2 (zero host shared objects), because
# that is a property of the AppDir -- sharun plus the bundled loader -- and
# nothing here touches the AppDir: all five arms are packed from the same one.
# `experiments/86-` owns that column and has the instrument for it
# (`classify_trace`; docs/AGENTS.md §14 on why a bundle's trace must not be
# attributed to one pid).
# ---------------------------------------------------------------------------
printf -- '\n-- the eleven: does it still run -----------------------------\n'
printf '  %-19s %-6s | %-10s %-10s\n' ENVIRONMENT LIBC 'OURS' 'FIELD'
ENVS=0; OURS_RAN=0; FIELD_RAN=0
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  : "$ref" "$digest"
  root=$(exp_rootfs "$name") || root=""
  [ -n "$root" ] || { exp_skip "$name" "not fetched"; continue; }
  ENVS=$((ENVS + 1))
  cells=""
  for a in ours field; do
    rm -f "$root/probe.AppImage"
    cp "$(arm_path "$a")" "$root/probe.AppImage"; chmod +x "$root/probe.AppImage"
    out=$(timeout -k 5 120 "$PGB" rootfs run "$root" -- /probe.AppImage --version \
          </dev/null 2>/dev/null | tr -d ' \n')
    if [ "$out" = "$FIRST" ]; then
      res=ok
      case "$a" in ours) OURS_RAN=$((OURS_RAN + 1)) ;; *) FIELD_RAN=$((FIELD_RAN + 1)) ;; esac
    else
      res="${out:-no-answer}"
    fi
    cells="$cells$(printf ' %-10s' "$res")"
    rm -f "$root/probe.AppImage"
  done
  printf '  %-19s %-6s |%s\n' "$name" "$libc" "$cells"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"
printf '\n'

# ---------------------------------------------------------------------------
# assertions
# ---------------------------------------------------------------------------
printf -- '-- assertions ------------------------------------------------\n'
exp_check "every arm answers --version identically" "$AGREE" "$NARMS"
[ "$ENVS" -gt 0 ] || exp_skip "the eleven" "no rootfs fetched"
if [ "$ENVS" -gt 0 ]; then
  exp_check "ours runs on every environment fetched"  "$OURS_RAN"  "$ENVS"
  exp_check "the lite runtime does too"               "$FIELD_RAN" "$ENVS"
fi
exp_check "the A/A pair is NOT resolvable (the control)" \
  "$(clk_resolves "$AA" "$AA_FLOOR")" "no"

# ⭐ THE RATCHET. This asserted `SHIPPED_SHA = $U056_SHA` -- "pgb ships the
# runtime this experiment calls `ours`" -- while `ours` was still the full
# v0.5.6 build, and it FIRED the moment `internal/bundle/appimage.go` moved to
# lite, which is what it was written to do.
#
# ⛔ It now guards the other direction, and that is the version worth keeping:
# whatever else changes, pgb must ship the runtime this experiment measures as
# the FAST one. A revert, a merge that loses the pin, or a `download` that
# serves a stale cached tool all make this row red -- and the last of those is
# not hypothetical: `download` keyed its cache on the destination PATH, so the
# pin moved and every machine with a warm cache kept packing the old runtime.
exp_check "pgb ships the fast runtime, not the one 'ours' measures" \
  "$SHIPPED_SHA" "$ULITE_SHA"

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
      ours) note='(reference) v0.5.6 full + dwarfs 0.14.1, shipped until 2026-09-03d' ;;
      twin) note='A/A control' ;;
      *)    note=$(clk_resolves "$r" "$f") ;;
    esac
    printf '%-10s %14s %12s %10s %10s %s\n' \
      "$a" "$(wc -c < "$(arm_path "$a")")" "$(clk_ms "$m") ms" "${r}x" "$f" "$note"
  done
  printf '\n'
  printf 'WHAT EACH ARM CHANGES\n'
  printf '  ours       uruntime v0.5.6 FULL  + mkdwarfs 0.14.1   <- pgb until\n'
  printf '                                                          2026-09-03d\n'
  printf '  v059full   uruntime v0.5.9 FULL  + mkdwarfs 0.14.1   <- version only\n'
  printf '  v059lite   uruntime v0.5.9 LITE  + mkdwarfs 0.14.1   <- lite only\n'
  printf '  field      uruntime v0.5.9 LITE  + mkdwarfs 0.15.6   <- the stack\n'
  printf '                                                          experiments/86-\n'
  printf '                                                          stages for the\n'
  printf '                                                          COMPETITOR arm,\n'
  printf '                                                          and what pgb\n'
  printf '                                                          ships now\n'
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
