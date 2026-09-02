#!/bin/sh
# TODO/check.sh - the gate. Asserts the record agrees with itself.
#
# ⛔ THE DEFECT THIS EXISTS TO CATCH, from docs/methodology/work-todo.md: a
# session closed two entries, wrote it into the entries, the index and the
# record, pushed, then rewrote a fourth file and never pushed again. The
# published state said those entries were open beside entries saying done, for
# a whole session. Nothing was wrong with any single file. What was missing was
# anything that compared two of them.
#
# Checks, all derived from the rows and never from the prose:
#   1. the TOTAL/OPEN/DONE line agrees with the row count
#   2. the priority table agrees with the rows
#   3. every row has an entry with that id, and every entry has a row
#   4. status in the index matches status in the entry
#   5. PROGRESS.md's COUNTS line agrees
#   6. every markdown link in TODO/ resolves
#   7. the codegraph index is current, when codegraph is installed
#
# ⛔ THE FAILURE COUNT LIVES IN A FILE, NOT A VARIABLE. Checks 3 and 6 feed a
# `while` loop from a pipe, which POSIX sh runs in a subshell, so a variable
# incremented inside one is discarded when it exits. Both printed FAIL lines and
# the gate still exited 0 under them.
#
# Exit: 0 agrees, 1 disagrees.
set -u
D=$(cd "$(dirname "$0")" && pwd)
FAILS=$(mktemp) || exit 2
trap 'rm -f "$FAILS"' EXIT INT TERM
say() { printf '  %-6s %s\n' "$1" "$2"; }
bad() { say FAIL "$1"; printf '%s\n' "$1" >> "$FAILS"; }
ok()  { say ok   "$1"; }
note() { say '--'  "$1"; }

rows=$(awk -F'|' '/^\| T-[0-9]+ \|/ {gsub(/ /,"",$2); print $2}' "$D/INDEX.md")
n_rows=$(printf '%s\n' "$rows" | grep -c .)
n_done=$(awk -F'|' '/^\| T-[0-9]+ \|/ {gsub(/ /,"",$5); if ($5=="done") c++} END{print c+0}' "$D/INDEX.md")
n_open=$((n_rows - n_done))

# 1. the counts line
read_t=$(awk '/^ *TOTAL [0-9]+ +OPEN/ {print $2}' "$D/INDEX.md")
read_o=$(awk '/^ *TOTAL [0-9]+ +OPEN/ {print $4}' "$D/INDEX.md")
read_d=$(awk '/^ *TOTAL [0-9]+ +OPEN/ {print $6}' "$D/INDEX.md")
if [ "$read_t" = "$n_rows" ] && [ "$read_o" = "$n_open" ] && [ "$read_d" = "$n_done" ]; then
  ok "INDEX counts agree with rows ($n_rows/$n_open/$n_done)"
else
  bad "INDEX says TOTAL $read_t OPEN $read_o DONE $read_d; rows say $n_rows/$n_open/$n_done"
fi

# 2. the priority table
for p in P0 P1 P2 P3; do
  rt=$(awk -F'|' -v p="$p" '/^\| T-[0-9]+ \|/ {gsub(/ /,"",$3); if ($3==p) c++} END{print c+0}' "$D/INDEX.md")
  rd=$(awk -F'|' -v p="$p" '/^\| T-[0-9]+ \|/ {gsub(/ /,"",$3); gsub(/ /,"",$5); if ($3==p && $5=="done") c++} END{print c+0}' "$D/INDEX.md")
  ro=$((rt - rd))
  line=$(awk -F'|' -v p="$p" '$2 ~ ("^ *" p " *$") && NF>6 {gsub(/ /,"",$4);gsub(/ /,"",$5);gsub(/ /,"",$6); print $4":"$5":"$6; exit}' "$D/INDEX.md")
  if [ "$line" = "$rt:$ro:$rd" ]; then ok "$p table row agrees ($rt/$ro/$rd)"
  else bad "$p table says ${line:-<missing>}; rows say $rt:$ro:$rd"; fi
done

# 3 + 4. rows <-> entries, and status agreement
for id in $rows; do
  f=$(grep -rl "^## $id — " "$D"/*.md 2>/dev/null | head -1)
  if [ -z "$f" ]; then bad "$id has a row but no entry"; continue; fi
  i_st=$(awk -F'|' -v id="$id" '/^\| T-[0-9]+ \|/ {gsub(/ /,"",$2); gsub(/ /,"",$5); if ($2==id) print $5}' "$D/INDEX.md")
  e_st=$(awk -v id="$id" '$0 ~ "^## "id" — " {f=1} f && /\*\*Status\*\*/ {print; exit}' "$f" \
         | grep -oE 'Status\*\* [^ ]+( [^ ]+)?' | sed 's/Status\*\* //; s/✅ //' | tr -d ' ')
  [ "$i_st" = "$e_st" ] || bad "$id: index says '$i_st', entry says '$e_st'"
done
ok "every row has an entry, statuses compared"
for f in "$D"/*.md; do
  case "$f" in */INDEX.md|*/PROGRESS.md|*/RULES.md) continue ;; esac
  grep -oE '^## (T-[0-9]+) — ' "$f" | awk '{print $2}' | while read -r eid; do
    grep -q "^| $eid |" "$D/INDEX.md" || bad "$eid has an entry but no row"
  done
done

# 5. PROGRESS counts
p_line=$(awk '/^ *COUNTS/ {print}' "$D/PROGRESS.md")
case "$p_line" in
  *"$n_rows entries, $n_open open, $n_done done"*) ok "PROGRESS COUNTS agrees" ;;
  *) bad "PROGRESS COUNTS is '$p_line'; rows say $n_rows entries, $n_open open, $n_done done" ;;
esac

# 6. links
for f in "$D"/*.md; do
  grep -oE '\]\(([^)]+)\)' "$f" 2>/dev/null | sed 's/](\(.*\))/\1/' | while read -r l; do
    case "$l" in http*|\#*) continue ;; esac
    [ -e "$D/${l%%#*}" ] || bad "$(basename "$f") -> $l does not resolve"
  done
done
ok "links checked"

# 7. the codegraph index, which is what an agent reads existing code with.
# ⚠ A missing codegraph is NOT a failure: the tool is a machine convenience and
# the index is gitignored, so a gate that failed on its absence would teach
# people to skip the gate. A STALE index is a failure -- it answers questions
# wrongly, and `codegraph sync` costs under a second.
if command -v codegraph >/dev/null 2>&1 && [ -d "$D/../.codegraph" ]; then
  if codegraph status "$D/.." 2>/dev/null | grep -q 'Index is up to date'; then
    ok "codegraph index is current"
  else
    bad "the codegraph index is stale; run: codegraph sync"
  fi
else
  note "codegraph absent or not indexed; run: sh scripts/common/install-codegraph.sh"
fi

# 8. ⛔ THE PINNED BUILD ENVIRONMENT HAS ONE SOURCE, AND IT IS cfg.go.
#
# T-070 measured the glibc pin move and found that changing `cfg.go` alone
# would leave EIGHT experiments and TWO CI jobs pinned to the old one: gone
# from disk they skip (exit 2, which nobody reads as a regression), still on
# disk they measure the old glibc and say nothing. `experiments/lib.sh` reads
# the name out of `cfg.go` and the `matrix` CI job reads the image and digest
# out of it; this is what stops the next copy being typed.
#
# ⚠ Prose is exempt: TODO/ and docs/ quote these values as evidence of what
# was measured, and rewriting history is not the fix. Only code is checked.
#
# ⚠ AND SO ARE WHOLE-LINE COMMENTS INSIDE CODE, which is not a loophole but the
# thing that makes the check usable: the block explaining WHY a file no longer
# hardcodes a value has to be allowed to say what it no longer hardcodes. A
# value on the right of live code is still caught — the filter is anchored.
#
# ⚠ AND scripts/common/rootfs-images.txt IS EXEMPT FOR THE IMAGE AND DIGEST,
# because a Debian release can be BOTH the build environment and one of the
# eleven TARGETS, and that file is the target list. It is not exempt for the
# name: an environment name has no business there.
cfg_const() {
  sed -n 's/^[[:space:]]*'"$1"'[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' \
    "$D/../internal/cfg/cfg.go" 2>/dev/null | head -1
}
for k in DefaultEnvName DefaultEnvImage DefaultEnvDigest; do
  v=$(cfg_const "$k")
  if [ -z "$v" ]; then
    bad "internal/cfg/cfg.go defines no $k"
    continue
  fi
  exempt='/internal/cfg/cfg\.go:'
  case "$k" in
    DefaultEnvImage|DefaultEnvDigest) exempt="$exempt|/scripts/common/rootfs-images\.txt:" ;;
  esac
  copies=$(grep -rnF "$v" \
             "$D/../experiments" "$D/../poc" "$D/../scripts" "$D/../internal" \
             "$D/../cmd" "$D/../.github" 2>/dev/null \
           | grep -vE "$exempt" \
           | grep -vE ':[0-9]+:[[:space:]]*(#|//|\*|/\*)' || true)
  if [ -n "$copies" ]; then
    printf '%s\n' "$copies" | while IFS= read -r c; do
      bad "hardcodes $k '$v': ${c#"$D"/../} (derive it from cfg.go)"
    done
  else
    ok "$k '$v' is defined once, in cfg.go"
  fi
done

fail=$(grep -c . "$FAILS")
printf '\n'
if [ "$fail" -gt 0 ]; then printf 'VERDICT: the record disagrees with itself (%s).\n' "$fail"; exit 1; fi
printf 'VERDICT: the record agrees with itself.\n'
