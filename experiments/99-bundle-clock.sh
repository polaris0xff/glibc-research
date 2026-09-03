#!/bin/sh
# 99-bundle-clock.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# ⛔ On 2026-09-03c the operator made the bundler's milliseconds the entire
# acceptance bar -- *"acceptable as long as ours performs better"* -- and deep
# review 1 immediately found that the timing half of the record does not
# re-derive.  `docs/history/corrections.md` C23: four runs of the same
# comparison, the same two artefacts, the same machine, gave cold-start ratios
# of 2.52x, 3.48x, 4.92x and 5.02x, and in TWO of the four, warm came out
# SLOWER than cold.
#
# C23 recorded the spread and named the cause as "the instrument's cold/warm
# distinction collapsing on this subject".  ⛔ It did not say WHY it collapses,
# and PROGRESS.md N0 asks for the instrument to be fixed before any lever is
# measured with it.  ⭐ This experiment answers both: it identifies the
# mechanism, proves the old protocol measures the wrong thing, and stands up
# the replacement with a control that says whether to believe it.
#
# -- WHAT IS MEASURED -------------------------------------------------------
#
#   1  ⭐ THE REUSE WINDOW.  How long after a run does the next run of the SAME
#      artefact still find a live mount?  This is the hidden variable: the same
#      command is cold or warm depending only on how many seconds have passed.
#   2  ⛔ THE OLD PROTOCOL, PROVED WRONG.  `90-`'s `cold_of()` obtains "cold" by
#      running a FRESH COPY, on the stated reasoning that "uruntime keys its
#      mount on the image, so a fresh file is cold by construction".  ⚠ The
#      mount is keyed on CONTENT, not on path, so a byte-identical copy reuses
#      the live mount and the cold column reports a warm number.
#   3  ⭐ THE A/A CONTROL.  One artefact under two names, through the identical
#      protocol.  The true ratio is 1.00, so what the instrument reports is its
#      resolution floor here, today.  `experiments/clock.sh`.
#   4  cold and warm under the corrected protocol, median of N, interleaved.
#
# ⚠ The subject is a `jq` bundle: it builds in about fifteen seconds against a
# warm nix closure, so the instrument can be exercised many times.  The
# mechanism is uruntime's, not jq's, and applies to every artefact `pgb bundle
# appimage` produces including kdenlive.
#
# Exit: 0 the measurement ran and matched, 1 it ran and did not, 2 could not run.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"
. "$(cd "$(dirname "$0")" && pwd)/clock.sh"

exp_begin "99 - the bundler's clock: the reuse window, and the protocol that missed it"

# ⛔ SCRATCH GOES IN `build/`, WHICH `.gitignore` ALREADY EXCLUDES. This read
# `$EXP_OUT/work` until a 207 MB padded AppImage was committed and GitHub
# refused the push at its 100 MB file limit. The raw samples are saved out
# separately by `clk_save`, because those ARE evidence.
WORK="$EXP_OUT/build"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
RESULT="$EXP_OUT/RESULT.txt"
PGB="$REPO_DIR/pgb"
CACHE="${PGB_APPIMAGE_CACHE:-/var/tmp/pgb-appimage}"
ROUNDS="${PGB_CLOCK_ROUNDS:-9}"
PKG="${PGB_CLOCK_PKG:-jq}"

[ -x "$PGB" ] || { printf 'pgb is not built (run make)\n'; exit 2; }

# ⛔ THE MOUNTS LIVE UNDER $TMPDIR AND THE INSTRUMENT MUST OWN IT.  uruntime
# puts its mountpoint at $TMPDIR/.mount_<name><digits>; pointing TMPDIR at a
# directory of ours means the reap below can never touch a mount that belongs
# to something else on this machine.
MTMP="$WORK/tmp"; mkdir -p "$MTMP"
TMPDIR="$MTMP"; export TMPDIR

# ---------------------------------------------------------------------------
# ⭐ THE REAP -- the operation the old protocol never performed
#
# ⛔ NOT `pkill -f` and NOT a process-name match: docs/AGENTS.md §14. The
# artefact's path is in this runner's own command line, so a full-command-line
# match kills the experiment, and a name match misses the FUSE daemon a bundle
# leaves behind. ⭐ The handle that is neither is the MOUNTPOINT PATH, which
# uruntime publishes in $TMPDIR and which fusermount takes directly.
# ---------------------------------------------------------------------------
reap_mounts() {
  for _d in "$MTMP"/.mount_*; do
    [ -e "$_d" ] || continue
    case "$_d" in *.pid) continue ;; esac
    fusermount3 -u "$_d" 2>/dev/null || fusermount -u "$_d" 2>/dev/null \
      || umount -l "$_d" 2>/dev/null || :
  done
  rm -rf "$MTMP"/.mount_* 2>/dev/null || :
}

live_mounts() {  # how many mountpoints uruntime currently has open here
  _n=0
  for _d in "$MTMP"/.mount_*; do
    [ -d "$_d" ] || continue
    case "$_d" in *.pid) continue ;; esac
    _n=$((_n + 1))
  done
  printf '%s' "$_n"
}

# ---------------------------------------------------------------------------
# the subject
# ---------------------------------------------------------------------------
printf -- '-- building the subject --------------------------------------\n'
APP="$WORK/$PKG.AppImage"
rm -rf "$CACHE/$PKG"
if ! "$PGB" bundle appimage "$PKG" --out "$APP" --cache "$CACHE" \
      --debloat safe > "$WORK/build.log" 2>&1; then
  printf 'the bundle did not build; see %s\n' "$WORK/build.log"
  tail -5 "$WORK/build.log"
  exit 2
fi
[ -s "$APP" ] || { printf 'no artefact\n'; exit 2; }
BYTES=$(wc -c < "$APP")
printf '  %-28s %s bytes\n' "$PKG.AppImage" "$BYTES"

# ⭐ The A/A twin: byte-identical, different path, different name on disk.
# ⚠ To uruntime these are the SAME artefact, because the mount is keyed on
# content -- which is the very property this experiment is about.
TWIN_D="$WORK/twin"; mkdir -p "$TWIN_D"
TWIN="$TWIN_D/$PKG.AppImage"
cp "$APP" "$TWIN"; chmod +x "$TWIN"
printf '  %-28s %s bytes (A/A twin)\n' "twin/$PKG.AppImage" "$(wc -c < "$TWIN")"

run_ok() { "$1" --version >/dev/null 2>&1; }
if ! run_ok "$APP"; then
  printf 'the artefact does not run here\n'; exit 2
fi
reap_mounts
printf '\n'

# ---------------------------------------------------------------------------
# 1. ⭐ THE REUSE WINDOW -- the hidden variable
# ---------------------------------------------------------------------------
printf -- '-- 1. the reuse window ---------------------------------------\n'
printf '  ⭐ The SAME command, the SAME file. The only thing that changes is\n'
printf '  how many seconds have passed since the previous run.\n\n'
printf '  %8s %10s %8s\n' GAP_S 'MS' 'MOUNTS'
WINDOW_WARM=-1; WINDOW_COLD=-1; WINDOW_AT=""
: > "$WORK/window.txt"
for gap in 0 2 4 6 10; do
  run_ok "$APP" || continue                       # establish the mount
  [ "$gap" -gt 0 ] && sleep "$gap"
  v=$(clk_time_once "$APP" --version)
  printf '  %8s %10s %8s\n' "$gap" "$(clk_ms "$v")" "$(live_mounts)"
  printf '%s %s\n' "$gap" "$v" >> "$WORK/window.txt"
  # the first gap is the warm reference; the first reading far above it is the
  # window's far edge
  if [ "$gap" = 0 ]; then WINDOW_WARM=$v; fi
  if [ "$WINDOW_COLD" = -1 ] && [ "$WINDOW_WARM" != -1 ] && [ "$v" != -1 ] \
     && [ "$v" -gt $((WINDOW_WARM * 3)) ]; then
    WINDOW_COLD=$v; WINDOW_AT=$gap
  fi
done
reap_mounts
printf '\n'
if [ "$WINDOW_COLD" != -1 ]; then
  _lo=$((WINDOW_AT - 2)); [ "$_lo" -lt 0 ] && _lo=0
  printf '  ⭐ THE MOUNT IS TORN DOWN BETWEEN %ss AND %ss AFTER THE LAST RUN.\n' \
    "$_lo" "$WINDOW_AT"
  printf '  Across that boundary the same command costs %s ms instead of %s ms\n' \
    "$(clk_ms "$WINDOW_COLD")" "$(clk_ms "$WINDOW_WARM")"
  printf '  -- %sx, decided by nothing but elapsed time.\n' \
    "$(clk_ratio "$WINDOW_COLD" "$WINDOW_WARM")"
else
  printf '  ⚠ No teardown observed within 10 s on this machine.\n'
fi
printf '\n'

# ---------------------------------------------------------------------------
# 2. ⛔ THE OLD PROTOCOL, THE NEW ONE, AND THE CONTROL -- ALL IN ONE INTERLEAVE
#
# Four arms for the column labelled "cold", plus the warm reference:
#
#   fresh    `90-`'s cold_of(): a byte-identical copy in a fresh directory,
#            with XDG_CACHE_HOME pointed at an empty directory. NO REAP.
#   cold_A   ⭐ the corrected protocol: the live mount is torn down first.
#   cold_B   ⭐ THE A/A TWIN -- the same bytes under a different path, through
#            the identical corrected protocol. Its true ratio to cold_A is
#            1.00 by construction, so what the instrument reports for the pair
#            IS its resolution floor.
#   warm     a mount established immediately before, then timed.
#
# ⛔ ONE `clk_init`, ONE `clk_interleave`, ON PURPOSE. A control measured in a
# separate pass licenses nothing: it saw a different few seconds of this
# machine. The first version of this experiment ran the A/A block after the
# protocol block and the second `clk_init` discarded the first block's
# samples, so two assertions came back `unknown` -- which is the loud version
# of the quiet failure that separation invites.
# ---------------------------------------------------------------------------
printf -- '-- 2. four protocols for one column, one interleave ----------\n'
clk_init "$WORK/proto" || exit 2

FRESH_N=0
clk_prep() {
  case "$1" in
    fresh)
      # `90-`'s reasoning, reproduced exactly: a fresh copy under a fresh name
      # in a fresh directory, with a fresh cache. ⛔ And no reap, because the
      # old protocol did not know there was anything to reap.
      run_ok "$APP" || :                       # something has run recently,
                                               # which is the realistic case
      FRESH_N=$((FRESH_N + 1))
      _fd="$WORK/fresh.$FRESH_N"; rm -rf "$_fd"; mkdir -p "$_fd/cache"
      cp -L "$APP" "$_fd/$PKG.AppImage"; chmod +x "$_fd/$PKG.AppImage"
      FRESH_PATH="$_fd/$PKG.AppImage"; FRESH_CACHE="$_fd/cache"
      ;;
    cold_A|cold_B) reap_mounts ;;
    warm)          run_ok "$APP" || : ;;       # guarantee a live mount
  esac
}
clk_run() {
  case "$1" in
    fresh)  XDG_CACHE_HOME="$FRESH_CACHE" clk_time_once "$FRESH_PATH" --version ;;
    cold_A) clk_time_once "$APP"  --version ;;
    cold_B) clk_time_once "$TWIN" --version ;;
    warm)   clk_time_once "$APP"  --version ;;
  esac
}
clk_interleave "$ROUNDS" fresh cold_A warm cold_B
rm -rf "$WORK"/fresh.* 2>/dev/null || :
reap_mounts
clk_table
printf '\n'

FRESH_M=$(clk_stat fresh median)
REAP_M=$(clk_stat cold_A median)
WARM_M=$(clk_stat warm median)

# ---------------------------------------------------------------------------
# 3. ⭐ THE A/A CONTROL, out of the arms just measured
# ---------------------------------------------------------------------------
printf -- '-- 3. the A/A control ----------------------------------------\n'
printf '  cold_A and cold_B are one artefact under two names, through the\n'
printf '  identical protocol. The true ratio is 1.00; what comes back is the\n'
printf '  floor below which no other row in this file may be believed.\n\n'
AA=$(clk_aa cold_A cold_B)
AA_FLOOR=$(clk_floor cold_A cold_B "$AA")
printf '  A/A ratio                   : %s\n' "$AA"
printf '  resolution floor            : %s   (the larger of the A/A ratio and\n' "$AA_FLOOR"
printf '                                       the two arms'"'"' combined MAD)\n'
printf '  does the A/A pair resolve?  : %s   ⛔ MUST be "no"\n' \
  "$(clk_resolves "$AA" "$AA_FLOOR")"
printf '\n'
printf '  fresh-copy "cold" vs warm   : %sx\n' "$(clk_ratio "$FRESH_M" "$WARM_M")"
printf '  reaped cold vs warm         : %sx\n' "$(clk_ratio "$REAP_M" "$WARM_M")"
printf '\n'

# ---------------------------------------------------------------------------
# assertions
# ---------------------------------------------------------------------------
printf -- '-- assertions ------------------------------------------------\n'

# ⛔ THE INSTRUMENT'S OWN NEGATIVE CONTROL, AND IT IS ASSERTED, NOT REPORTED.
# If two copies of one artefact come out apart by more than the floor, every
# other number in this file is unreadable and the run must fail.
exp_check "the A/A pair is NOT resolvable (the control)" \
  "$(clk_resolves "$AA" "$AA_FLOOR")" "no"

# ⭐ THE FINDING: the old protocol's cold column is not cold. It is asserted
# as "indistinguishable from warm", which is the claim that matters -- not
# "equal", which no clock can establish.
FRESH_R=$(clk_ratio "$FRESH_M" "$WARM_M")
FRESH_F=$(clk_floor fresh warm "$AA")
exp_check "90-'s fresh-copy 'cold' is indistinguishable from warm" \
  "$(clk_resolves "$FRESH_R" "$FRESH_F")" "no"

# ⭐ And the corrected protocol does separate them, which is what makes it a
# protocol rather than a second way of measuring the same state.
REAP_R=$(clk_ratio "$REAP_M" "$WARM_M")
REAP_F=$(clk_floor cold_A warm "$AA")
exp_check "the reaped protocol DOES separate cold from warm" \
  "$(clk_resolves "$REAP_R" "$REAP_F")" "yes"

exp_check "every arm produced a full sample set" \
  "$(clk_stat cold_B n)" "$ROUNDS"

printf '\n'
exp_note "⚠ The reuse window is a property of uruntime, not of jq. Every"
exp_note "artefact pgb bundle appimage produces inherits it, kdenlive"
exp_note "included, and NOTHING in the record knew it was there."

# ---------------------------------------------------------------------------
{
  printf 'experiment 99 - the bundler'"'"'s clock\n'
  printf 'date (UTC)   : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'host kernel  : %s\n' "$(uname -sr)"
  printf 'subject      : pgb bundle appimage %s --debloat safe, %s bytes\n' "$PKG" "$BYTES"
  printf 'rounds       : %s per arm, interleaved with a rotating start\n' "$ROUNDS"
  printf 'estimator    : median; spread reported as MAD %% of median\n'
  printf '\n'
  printf 'THE REUSE WINDOW (same file, same command, only elapsed time differs)\n'
  printf '  %8s %10s\n' GAP_S MS
  while read -r g v; do printf '  %8s %10s\n' "$g" "$(clk_ms "$v")"; done < "$WORK/window.txt"
  printf '\n'
  printf 'THREE PROTOCOLS FOR THE COLUMN LABELLED "COLD"\n'
  printf '  %-14s %12s  %s\n' 'fresh copy'  "$(clk_ms "$FRESH_M") ms" \
    "90-'s cold_of(); ratio to warm $FRESH_R -- resolves: $(clk_resolves "$FRESH_R" "$FRESH_F")"
  printf '  %-14s %12s  %s\n' 'reaped'      "$(clk_ms "$REAP_M") ms" \
    "the corrected protocol; ratio to warm $REAP_R -- resolves: $(clk_resolves "$REAP_R" "$REAP_F")"
  printf '  %-14s %12s  %s\n' 'warm'        "$(clk_ms "$WARM_M") ms" 'a live mount, by construction'
  printf '\n'
  printf 'THE A/A CONTROL\n'
  printf '  one artefact, two names, identical protocol\n'
  printf '  ratio %s, floor %s, resolves %s (must be "no")\n' \
    "$AA" "$AA_FLOOR" "$(clk_resolves "$AA" "$AA_FLOOR")"
  printf '\n'
  printf 'WHY IT MATTERS\n'
  printf '  uruntime keys its mountpoint on the artefact'"'"'s CONTENT and leaves it\n'
  printf '  alive for a few seconds after the process exits. A byte-identical\n'
  printf '  copy therefore reuses it. experiments/90-'"'"'s cold column obtained\n'
  printf '  "cold" by copying the file, and so reported a warm number whenever\n'
  printf '  anything had run the same content within the window -- which is why\n'
  printf '  docs/history/corrections.md C23 saw warm above cold in two of four\n'
  printf '  runs and a 20x spread in the absolute figure.\n'
} > "$RESULT"
cp "$WORK/window.txt" "$EXP_OUT/reuse-window.txt" 2>/dev/null || :
clk_save "$EXP_OUT/samples"

printf 'full table: %s\n' "$RESULT"

exp_finish
