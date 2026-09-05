#!/bin/sh
# criteria-audit.sh - which PRE-REGISTERED criteria never reach an assertion?
#
# ⛔ THE DEFECT CLASS THIS SERVES IS THIS TREE'S MOST EXPENSIVE ONE.
# `docs/history/corrections.md` C48, C50, C52, C54, C56 and C57 are all one
# shape: *a criterion that cannot fire*, hidden by a skip or by a zero. Neither
# gate can see one, because a skip is not a failure and a zero looks like a
# result.
#
# ⭐ ONE SUB-SHAPE IS MECHANICALLY FINDABLE, and this finds it: an experiment
# pre-registers `S2` in its header (delivery rule 1) and the code never asserts
# `S2` at all. Found exactly that in `experiments/108-` on 2026-09-05 — the
# header required a captured PNG whose IHDR "match the server's" and the
# counter accepted any PNG with w>0,h>0, so a placeholder or an icon would have
# scored a capture on the one criterion the experiment exists to test.
#
# ⛔⛔ THIS IS AN ADVISORY REVIEW INSTRUMENT AND IT IS DELIBERATELY NOT A GATE.
# Measured over all 54 experiments: it raised 15 files, and FOURTEEN were
# correct code. The header IDs in this tree carry at least four genres and only
# one of them is an assertion:
#
#   a criterion    `108-` S2, asserted            -> a real finding when absent
#   a REPORTED     `101-` L5, `103-` D4, `106-` T3, `66-` N4 — deliberately
#     observation  measured and printed rather than asserted, and each says so
#   a research     `80-` Q1 "Can a nixpkgs package be RESOLVED with no nix?"
#     QUESTION     — a question is not a check
#   a corrections  `102-`/`60-`/`62-` C25 and C38, `99-` C23 — references to
#     .md ID       docs/history/corrections.md, not criteria at all
#
# ⚠ AND ONE MORE, WHICH IS WHY A GATE WOULD BE WRONG RATHER THAN MERELY NOISY:
# `64-`'s E6 IS asserted, at line 431, as `exp_check "control: no target ships
# these programs"` — the assertion simply does not repeat the id in its label.
# A gate would demand that 54 experiments adopt a labelling convention they do
# not have, and the pressure would be to relabel rather than to think.
#
# ⭐ SO IT CARRIES ITS OWN TRIAGE. Everything below was read and accepted on
# 2026-09-05; the script reports only what is NOT in this list, so a NEW
# unasserted criterion stands out instead of arriving in a crowd of fifteen.
# ⚠ Delete a line when the experiment changes, the way STALE-EVIDENCE.txt
# works: an accepted entry must not silently outlive its reason.
#
# Usage:  sh scripts/common/criteria-audit.sh            report
#         sh scripts/common/criteria-audit.sh --selftest assert the reader
#
# Exit: 0 nothing new, 1 a criterion is pre-registered and never asserted,
#       2 it could not run.
set -u
R=$(cd "$(dirname "$0")/../.." && pwd)
CORR="$R/docs/history/corrections.md"

# <experiment basename> <id> <why it is not an assertion>
ACCEPTED=$(cat <<'EOF'
101-gtk-locale-prefix L5 REPORTED, NOT PREDICTED: four of eleven ship no locale support
103-gstreamer-decode D4 REPORTED, NOT PREDICTED: whether gst-plugin-scanner runs at all
106-host-integration T3 reported beside T1 so a drawing failure is visible as itself
66-netdb N4 the boundary, pre-registered as a FAILURE and measured rather than asserted
63-three-way-parity Q4 an expectation about a printed comparison table, not a check
63-three-way-parity Q7 the parity VERDICT, read off the table
64-python-gui-bundle E6 asserted at "control: no target ships these programs" without the id
80-nix-without-nix Q1 a research QUESTION, not a criterion
80-nix-without-nix Q2 a research QUESTION, not a criterion
82-host-data-search P1 an expectation about a printed classification, not a check
82-host-data-search P2 an expectation about a printed classification, not a check
82-host-data-search P3 an expectation about a printed classification, not a check
82-host-data-search P4 an expectation about a printed classification, not a check
82-host-data-search P5 an expectation about a printed classification, not a check
65-capability-corpus C3 a LIMIT, and the script says so in as many words
65-capability-corpus C4 an UNRESOLVED subject is a gap, not a result
65-capability-corpus C5 the xterm prediction; C2's number is where it shows up
108-flameshot-capture S3 REPORTED, NOT CHECKED: host objects can only be read once the capture works
EOF
)

# ids_of FILE -> the criterion ids its header pre-registers
ids_of() {
  awk '/^set -u/{exit} {print}' "$1" \
    | sed -n 's/^#[[:space:]]\{1,\}\([A-Z][0-9][0-9]*[a-z]\?\)[[:space:]].*/\1/p' \
    | sort -u
}

# asserted FILE ID -> 0 when the body asserts it, directly or by a sub-id
# ⚠ `Q3` counts as asserted by `Q3a`/`Q3b`: splitting one criterion into two
# named halves is how this tree writes them, and demanding the bare id back
# would report `107-` every run.
asserted() {
  awk '/^set -u/,0' "$1" | grep -qE "exp_(check|skip) \"$2[a-z]?[ \"]"
}

audit() {   # -> lines "<file> <id>"
  for f in "$R"/experiments/[0-9]*.sh; do
    [ -e "$f" ] || continue
    b=$(basename "$f" .sh)
    for id in $(ids_of "$f"); do
      # a corrections.md heading with this id is a REFERENCE, not a criterion
      grep -qE "^## $id — " "$CORR" 2>/dev/null && continue
      asserted "$f" "$id" && continue
      printf '%s %s\n' "$b" "$id"
    done
  done
}

if [ "${1:-}" = "--selftest" ]; then
  # ⛔ ASSERT THE READER IN BOTH DIRECTIONS. A reporter that has only been
  # checked on the case it flags is half an instrument -- docs/AGENTS.md §0b.
  d=$(mktemp -d) || exit 2
  mkdir -p "$d/experiments" "$d/docs/history"
  : > "$d/docs/history/corrections.md"
  cat > "$d/experiments/1-x.sh" <<'FIX'
#   A1  a criterion that IS asserted
#   A2  a criterion that is NOT asserted
#   A3  a criterion asserted only by a sub-id
set -u
exp_check "A1 the first" "$x" 1
exp_check "A3a the third, first half" "$y" 1
FIX
  p=0; fl=0
  _ck() { if [ "$2" = "$3" ]; then printf '  ok    %-46s = %s\n' "$1" "$2"; p=$((p+1))
          else printf '  FAIL  %-46s = %s, expected %s\n' "$1" "$2" "$3"; fl=$((fl+1)); fi; }
  printf '\n-- criteria-audit --selftest -------------------------------------\n'
  out=$(R=$d; CORR="$d/docs/history/corrections.md"; audit)
  _ck "an ASSERTED criterion is not reported"      "$(printf '%s\n' "$out" | grep -c ' A1$')" 0
  _ck "⭐ an UNASSERTED criterion IS reported"     "$(printf '%s\n' "$out" | grep -c ' A2$')" 1
  _ck "⭐ a sub-id assertion counts (A3 by A3a)"   "$(printf '%s\n' "$out" | grep -c ' A3$')" 0
  _ck "⛔ and it reports exactly one, not three"   "$(printf '%s\n' "$out" | grep -c .)"      1
  rm -rf "$d"
  printf '\ncriteria-audit --selftest: %d pass, %d fail\n' "$p" "$fl"
  [ "$fl" = 0 ] || exit 1
  exit 0
fi

new=0
audit | while read -r b id; do
  printf '%s\n' "$ACCEPTED" | grep -qE "^$b $id " && continue
  printf '  ⛔ %-34s %s is pre-registered and never asserted\n' "$b" "$id"
done > /tmp/criteria-audit.$$
new=$(grep -c . /tmp/criteria-audit.$$ 2>/dev/null) || new=0
cat /tmp/criteria-audit.$$
rm -f /tmp/criteria-audit.$$
naccept=$(printf '%s\n' "$ACCEPTED" | grep -c .)
if [ "${new:-0}" = 0 ]; then
  printf '  ok     every pre-registered criterion is asserted (%s accepted, read and listed)\n' "$naccept"
  exit 0
fi
printf '\n⛔ %s criterion(s) above are pre-registered and never asserted.\n' "$new"
printf '   Assert it, or add it to this script ACCEPTED list WITH THE REASON.\n'
exit 1
