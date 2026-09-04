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
#
# ⛔ AN ENTRY LIVES IN ONE OF TWO PLACES AND ITS STATUS DECIDES WHICH, from the
# operator's instruction of 2026-09-03c: "strip away the fat, things that are
# already resolved and fixed and just send them straight into /HISTORY/*, the
# TODO/* must be lean and contain only what's left". An `open` entry belongs in
# TODO/; a `done` entry belongs in HISTORY/entries/. Check 4b is what stops
# that decaying back into one pile the moment somebody closes an entry.
H="$D/../HISTORY/entries"
for id in $rows; do
  f=$(grep -rl "^## $id — " "$D"/*.md "$H"/*.md 2>/dev/null | head -1)
  if [ -z "$f" ]; then bad "$id has a row but no entry"; continue; fi
  i_st=$(awk -F'|' -v id="$id" '/^\| T-[0-9]+ \|/ {gsub(/ /,"",$2); gsub(/ /,"",$5); if ($2==id) print $5}' "$D/INDEX.md")
  e_st=$(awk -v id="$id" '$0 ~ "^## "id" — " {f=1} f && /\*\*Status\*\*/ {print; exit}' "$f" \
         | grep -oE 'Status\*\* [^ ]+( [^ ]+)?' | sed 's/Status\*\* //; s/✅ //' | tr -d ' ')
  [ "$i_st" = "$e_st" ] || bad "$id: index says '$i_st', entry says '$e_st'"
  # 4b. the entry is filed where its status says it belongs
  case "$f" in
    "$H"/*) [ "$i_st" = done ] || bad "$id is '$i_st' but its entry is in HISTORY/entries; open work lives in TODO/" ;;
    *)      [ "$i_st" = open ] || bad "$id is '$i_st' but its entry is in TODO/; closed work lives in HISTORY/entries/" ;;
  esac
done
ok "every row has an entry, statuses compared, entries filed by status"
for f in "$D"/*.md "$H"/*.md; do
  case "$f" in */INDEX.md|*/PROGRESS.md|*/RULES.md) continue ;; esac
  grep -oE '^## (T-[0-9]+) — ' "$f" | awk '{print $2}' | while read -r eid; do
    grep -q "^| $eid |" "$D/INDEX.md" || bad "$eid has an entry but no row"
  done
done
# 4c. exactly one entry per id, across both halves
dupes=$(grep -hoE '^## T-[0-9]+ — ' "$D"/*.md "$H"/*.md 2>/dev/null | awk '{print $2}' \
        | sort | uniq -d)
if [ -n "$dupes" ]; then
  printf '%s\n' "$dupes" | while IFS= read -r d; do bad "$d has more than one entry"; done
else
  ok "no id carries two entries"
fi

# 4d. ⛔ AN OPEN ENTRY WHOSE DETAIL WAS RETIRED MUST SAY SO.
#
# The strip left every open entry short and put its measurements in
# HISTORY/entries/<category>-open.md. An entry that does not link its own
# detail is worse than one that never had any: the reader cannot tell that
# the route in front of them was already costed. Deep review 2 found FOUR
# entries in exactly that state within an hour of the move, which is how fast
# an unchecked convention decays.
n4d=0
for id in $rows; do
  det=$(grep -l "^## $id · retired detail" "$H"/*-open.md 2>/dev/null | head -1)
  [ -n "$det" ] || continue
  f=$(grep -rl "^## $id — " "$D"/*.md 2>/dev/null | head -1)
  [ -n "$f" ] || continue          # closed entries are whole; 4b owns them
  body=$(awk -v id="$id" '$0 ~ "^## "id" — " {f=1; next} /^## T-[0-9]+ — / {f=0} f' "$f")
  case "$body" in
    *"$(basename "$det")"*) n4d=$((n4d + 1)) ;;
    *) bad "$id is open and its detail was retired to $(basename "$det"), but the entry does not link it" ;;
  esac
done
ok "open entries link their retired detail ($n4d checked)"

# 5. PROGRESS counts
p_line=$(awk '/^ *COUNTS/ {print}' "$D/PROGRESS.md")
case "$p_line" in
  *"$n_rows entries, $n_open open, $n_done done"*) ok "PROGRESS COUNTS agrees" ;;
  *) bad "PROGRESS COUNTS is '$p_line'; rows say $n_rows entries, $n_open open, $n_done done" ;;
esac

# 6. links, in both halves of the record. ⚠ The HISTORY half is checked because
# its files were MOVED there and every relative link in them changed depth; a
# move that leaves ../docs/ pointing at HISTORY/docs/ breaks silently.
for f in "$D"/*.md "$H"/*.md; do
  b=$(dirname "$f")
  grep -oE '\]\(([^)]+)\)' "$f" 2>/dev/null | sed 's/](\(.*\))/\1/' | while read -r l; do
    case "$l" in http*|\#*) continue ;; esac
    [ -e "$b/${l%%#*}" ] || bad "$(basename "$f") -> $l does not resolve"
  done
done
ok "links checked, TODO/ and HISTORY/entries/"

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

# 9. ⛔ A STORE PATH'S NAME INSIDE THE BUNDLE HAS ONE RULE, AND IT IS
# farmDirName. Two existed: buildStoreFarm fell back to the full
# `<hash>-<name>` when two store paths shared a short name, and carryBakedPaths
# sliced `base[33:]` inline with no fallback -- so on a closure carrying two
# builds of one package the `.env` named a directory the farm never created,
# and nothing read a `.env` value back to notice. T-092, corrections.md C28.
#
# ⛔ WHY A GREP AND NOT A SELFTEST. The selftest can prove farmDirName is right
# and that both sides build the same string; it CANNOT see a third caller that
# slices the name itself and never calls farmDirName. That is the shape the
# defect actually had, so it needs a structural check -- the same argument as
# check 8 above.
#
# ⚠ 33 is the length of a store hash plus its `-`. Whole-line comments are
# exempt for the same reason as check 8: the block saying what a file no longer
# does has to be able to say it.
# ⚠ THE EXEMPTION IS shortStoreName'S BODY, FOUND BY CONTEXT. The first version
# exempted a LINE RANGE in storefix.go and was wrong within the same commit --
# inserting farmDirName above it moved the function. A gate whose exemption
# drifts is worse than no gate.
sn_body=$(awk '/^func shortStoreName\(/ {f=1} f {print FILENAME":"FNR} f && /^}/ {f=0}' \
          "$D/../internal/bundle/storefix.go" 2>/dev/null)
slices=$(grep -rnE '\[33:\]|> 33' "$D/../internal/bundle" 2>/dev/null \
         | grep -vE ':[0-9]+:[[:space:]]*(//|\*|/\*)' || true)
for e in $sn_body; do
  slices=$(printf '%s\n' "$slices" | grep -vF "$e:" || true)
done
if [ -n "$slices" ]; then
  printf '%s\n' "$slices" | while IFS= read -r c; do
    [ -n "$c" ] || continue
    bad "slices a store name by hand: ${c#"$D"/../} (call shortStoreName, or farmDirName for a farm directory)"
  done
else
  ok "the store-name slice lives only in shortStoreName"
fi

# 10. ⛔ EVERY tool/runtime/*.c MUST AT LEAST PARSE, and the reason is a silent
# fallback rather than a build failure.
#
# These files are EMBEDDED as strings and compiled by `cc` at BUILD or BUNDLE
# time, not by `make` -- so `go build` succeeds on a C file that cannot
# compile, and nothing here noticed. Worse, `buildStaticAppRun` catches a
# compile failure and falls back to writing a SHELL AppRun, which runs under
# the HOST's interpreter and loads the HOST's libc. That is the exact thing
# pgb-apprun.c exists to avoid, and it would have shipped as a warning in a
# build log.
#
# ⭐ IT REALLY HAPPENED, in a COMMENT. A revision quoting Anylinux's AppRun
# rule verbatim contained a shell parameter expansion whose `*/` ended the C
# block comment. experiments/68- arm S caught it; this is what catches it
# without running an experiment.
#
# ⚠ -fsyntax-only, not a link: these have wildly different link requirements
# (one is -shared, one -static, one needs -Wl,--wrap) and parsing is the
# property that was broken.
if command -v cc >/dev/null 2>&1; then
  nc=0; badc=0
  for f in "$D"/../tool/runtime/*.c; do
    [ -f "$f" ] || continue
    nc=$((nc + 1))
    if ! cc -fsyntax-only -D_GNU_SOURCE -DPGB_APPRUN_DEFAULT='"x"' "$f" 2>/dev/null; then
      bad "tool/runtime/$(basename "$f") does not parse (cc -fsyntax-only)"
      badc=$((badc + 1))
    fi
  done
  [ "$badc" = 0 ] && ok "every tool/runtime/*.c parses ($nc checked)"
else
  note "no cc; tool/runtime/*.c not syntax-checked"
fi

fail=$(grep -c . "$FAILS")
printf '\n'
if [ "$fail" -gt 0 ]; then printf 'VERDICT: the record disagrees with itself (%s).\n' "$fail"; exit 1; fi
printf 'VERDICT: the record agrees with itself.\n'
