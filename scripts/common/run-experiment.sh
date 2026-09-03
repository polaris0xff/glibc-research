#!/bin/sh
# run-experiment.sh - run one experiment and leave its evidence behind.
#
# ⛔ THE DEFECT THIS EXISTS TO REMOVE, found by deep review 5 on 2026-09-03c.
# `TODO/RESUME.md` carried this as a machine note:
#
#     ⚠ An experiment writes its own `RESULT.txt`. Redirect stdout elsewhere.
#     ⚠ A POC does NOT -- redirect, or its RESULT.txt describes the previous run.
#
# ⛔ BOTH HALVES ARE WRONG, and measured:
#
#     experiments that write their own RESULT.txt   19  (70 71 72 73 74 75 76
#                                                        78 79 80 83 87 88 89
#                                                        90 91 93 94 96)
#     experiments that do NOT                       13  (10 20 21 30 40 50 60
#                                                        61 62 85 86 95 97)
#     POCs                                          all of them, via poc/common.sh
#
# So the rule is the opposite of the note for 19 of 32 experiments, right for
# 13 by accident, and there is no way to tell which group one is in without
# reading it. ⚠ IT COST A MEASUREMENT THE SAME DAY: `experiments/30-` was
# re-run to clear a stale-evidence finding, reported `pass=11 fail=0`, and
# refreshed NOTHING -- its committed evidence still carried a date two days
# earlier and a `Linux 6.18.44-fc-v22` kernel this machine no longer runs.
# A green run that updates nothing is the same shape as a green gate that
# cannot fail.
#
# ⭐ WHAT THIS DOES, AND WHY IT IS NOT JUST `> RESULT.txt`. The 19 that write
# their own produce a CURATED file -- a table, not the transcript -- and
# redirecting stdout over it would replace better evidence with worse. So:
#
#   * stdout is always tee'd to `evidence/<name>/run.log`;
#   * `RESULT.txt` is written from that transcript ONLY IF the experiment did
#     not write one itself during this run, decided by comparing the file's
#     modification time against a marker taken before the run rather than by a
#     list of names that would go stale exactly like the note did.
#
# Usage:  sh scripts/common/run-experiment.sh 30
#         sh scripts/common/run-experiment.sh experiments/30-gconv-and-locale.sh
#
# Exit: the experiment's own status. 0 measured and matched, 1 measured and did
# not, 2 could not run.
set -u
R=$(cd "$(dirname "$0")/../.." && pwd)
[ $# -ge 1 ] || { echo "usage: $0 <experiment number or path> [args...]" >&2; exit 2; }

spec=$1; shift
case "$spec" in
  */*) script=$spec ;;
  *)   script=$(ls "$R"/experiments/"$spec"-*.sh 2>/dev/null | head -1) ;;
esac
[ -n "${script:-}" ] && [ -r "$script" ] || { echo "run-experiment: no experiment for '$spec'" >&2; exit 2; }

name=$(basename "$script" .sh)
out="${PGB_EVIDENCE_DIR:-$R/evidence}/$name"
mkdir -p "$out" || exit 2
res="$out/RESULT.txt"

# ⛔ THE MARKER IS A FILE, NOT A TIMESTAMP READ FROM $res. Reading the mtime of
# a file that may not exist yet needs two code paths; a marker created now
# needs one, and `find -newer` answers the question directly.
marker=$(mktemp) || exit 2
trap 'rm -f "$marker"' EXIT INT TERM

sh "$script" "$@" 2>&1 | tee "$out/run.log"
# ⛔ THE PIPELINE'S STATUS IS tee'S. This tree has paid for that twice; the
# experiment's own status is recovered from the transcript's verdict line.
if grep -q '^VERDICT: matched expectation\.' "$out/run.log"; then
  st=0
elif grep -q '^VERDICT: the measurement ran' "$out/run.log"; then
  st=1
else
  st=2
fi

if [ -e "$res" ] && [ -n "$(find "$res" -newer "$marker" 2>/dev/null)" ]; then
  printf 'run-experiment: %s wrote its own RESULT.txt; transcript in %s\n' \
    "$name" "$out/run.log"
else
  cp "$out/run.log" "$res"
  printf 'run-experiment: wrote %s from the transcript\n' "$res"
fi
exit "$st"
