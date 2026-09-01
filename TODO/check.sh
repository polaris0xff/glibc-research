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
#
# Exit: 0 agrees, 1 disagrees.
set -u
D=$(cd "$(dirname "$0")" && pwd)
fail=0
say() { printf '  %-6s %s\n' "$1" "$2"; }
bad() { say FAIL "$1"; fail=$((fail+1)); }
ok()  { say ok   "$1"; }

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
    grep -q "^| $eid |" "$D/INDEX.md" || echo "  FAIL   $eid has an entry but no row"
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
    [ -e "$D/${l%%#*}" ] || echo "  FAIL   $(basename "$f") -> $l does not resolve"
  done
done
ok "links checked"

printf '\n'
if [ "$fail" -gt 0 ]; then printf 'VERDICT: the record disagrees with itself (%s).\n' "$fail"; exit 1; fi
printf 'VERDICT: the record agrees with itself.\n'
