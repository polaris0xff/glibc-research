#!/bin/sh
# check-docs.sh - the gate for the DOCUMENTATION, as TODO/check.sh is for the
# record.
#
# ⛔ WHY A SECOND GATE, AND THE GAP IT WAS WRITTEN FOR. `TODO/check.sh` compares
# the record with itself and checks links **inside `TODO/`**. Nothing checked
# `docs/`. The documentation went thirteen commits without an edit while five
# entries changed state under it, and the way that was noticed was an operator
# saying so -- not a check. `docs/methodology/reviews.md`: *"Anything a check
# can assert should be asserted by a check, not by a reading."*
#
# What it asserts, all derived, none of it typed in:
#   1.  every relative markdown link this project WROTE resolves
#   1b. the vendored set's unresolved links are exactly the ones PROVENANCE.md
#       publishes -- so vendoring one more file is a one-line diff, not a
#       forgotten paragraph
#   2.  every repo path named in backticks resolves
#   3.  every `evidence/...` path a doc cites is TRACKED, or is a gitignored
#       build product -- never merely present on this machine
#   4.  every experiment and POC referenced by number exists
#   5.  the entry counts quoted in prose agree with TODO/INDEX.md
#   6.  every vendored methodology file named in PROVENANCE.md is on disk
#
# ⚠ (2) IS THE ONE THAT PAYS, AND ITS PATTERN IS WHAT KEEPS IT HONEST: a
# backticked token whose first segment is a real top-level directory of THIS
# repository. A nix store path, `/usr/lib`, `$HOME/...` and a bare command name
# never match, so no exemption list is needed -- and a long exemption list is a
# check that has been argued out of its job.
#
# ⚠ IT READS THE TRACKED SET, so a file that is written but not `git add`ed is
# invisible to it. That is deliberate -- an untracked file is not published --
# but it means "the check went green" and "the work is committed" are the same
# sentence here, and a new document has to be added before it is checked.
#
# Exit: 0 agrees, 1 disagrees.
set -u
R=$(cd "$(dirname "$0")/../.." && pwd)
cd "$R" || exit 2
fail=0
say() { printf '  %-6s %s\n' "$1" "$2"; }
bad() { say FAIL "$1"; fail=$((fail + 1)); }
ok()  { say ok   "$1"; }

# ⛔ WHAT IS AND IS NOT THIS PROJECT'S TO FIX. `references/` is mined
# third-party trees and `docs/methodology/` is vendored upstream: ⛔ vendored
# files are not edited (`docs/methodology/vendoring.md`), so reporting their
# internal links as defects is noise a reader learns to skip -- which is how a
# real one gets skipped too. They get their own check, (1b), which is the one
# that actually binds: the set of links they cannot resolve must equal the set
# `PROVENANCE.md` publishes.
#
# ⚠ Fenced code blocks are excluded everywhere. A `sed` script in an example is
# not a link, and treating it as one is how (1) first reported
# `PROVENANCE.md: dead link -> \(.*\`.
strip_fences() { awk '/^ *```/ {f = !f; next} !f' "$1"; }

# ⛔ `git ls-files '*.md'` IS NOT "THE MARKDOWN AT THE TOP". A git pathspec
# without `:(glob)` matches at ANY depth, so that one pattern pulled in every
# vendored tree under `references/` and the whole of `docs/methodology/` -- and
# the first run of this checker reported 60 defects in files it is forbidden to
# edit. The set is therefore derived and then FILTERED, and the filter is the
# assertion.
# ⚠ `tmp/START.md` is the OPERATOR'S BRIEF, quoted throughout this tree and not
# this project's to edit; its paths are the ones it asked for, not ones we owe.
# ⛔ TRACKED **AND** UNTRACKED, AND THE DIFFERENCE COST A RED CI RUN.
# This was `git ls-files '*.md'`, which lists only files git already knows
# about — so a BRAND NEW document was invisible to every check below until the
# commit that added it, which is precisely the commit the gate exists to block.
# ⚠ Measured on 2026-09-03d: `HISTORY/sessions/2026-09-03d.md` referenced
# `experiments/92`, the local gate said "the documentation agrees with the
# tree", and CI failed on the push. A gate that cannot see the file being added
# is not a gate, it is a delay.
# ⭐ `--others --exclude-standard` adds the untracked files that are NOT
# gitignored, which is exactly the set a `git add -A` is about to stage.
OURS=$(git ls-files --cached --others --exclude-standard '*.md' \
       | grep -vE '^(references|docs/methodology|tmp)/' | sort -u)
VENDORED=$(git ls-files 'docs/methodology/*.md')
[ -n "$OURS" ] || { echo "check-docs: no markdown found (not a git tree?)" >&2; exit 2; }

# ---------------------------------------------------------------------------
# 1. relative markdown links resolve, in the files this project writes
# ---------------------------------------------------------------------------
n=0; broken=0
for f in $OURS; do
  d=$(dirname "$f")
  # ⚠ The link target is everything up to the first '#' or whitespace: an
  # anchor is not part of the path and a title in quotes is not either.
  for l in $(strip_fences "$f" | grep -oE '\]\([^)]+\)' | sed 's/^](//; s/)$//' | sed 's/[#"].*$//' | sed 's/ .*$//'); do
    case "$l" in ''|http*|mailto:*) continue ;; esac
    n=$((n + 1))
    [ -e "$d/$l" ] && continue
    bad "$f: dead link -> $l"
    broken=$((broken + 1))
  done
done
[ "$broken" -eq 0 ] && ok "markdown links resolve ($n checked)"

# ---------------------------------------------------------------------------
# 1b. the vendored set's unresolved links are exactly the ones PROVENANCE lists
# ---------------------------------------------------------------------------
# ⭐ THIS IS THE CHECK PROVENANCE.md ASKED FOR IN PROSE. It publishes the
# derived list of links that do not resolve here and the command that derives
# it. Deriving it and never comparing it is a list that rots; comparing it
# turns "vendor one more file" into a one-line diff nobody can forget.
actual=$(for f in $VENDORED; do
  case "$f" in */PROVENANCE.md) continue ;; esac
  d=$(dirname "$f")
  strip_fences "$f" | grep -oE '\]\([^)]+\)' | sed 's/^](//; s/)$//' | sed 's/[#"].*$//' | sed 's/ .*$//' |
  while read -r l; do
    case "$l" in ''|http*|mailto:*) continue ;; esac
    [ -e "$d/$l" ] || printf '%s\n' "$l"
  done
done | sort -u)
# ⛔ THE LIST IS PARSED BY SHAPE, NOT BY NAME. A first version matched the six
# file names it knew about, so vendoring `gate.md` -- which brought
# `../conventions/forbidden-patterns.md` in with it -- made the check report a
# stale list that was in fact correct. A check with the answer written into it
# is not a check.
listed=$(awk '/^    [^ ]/ {
                ok = 1
                for (i = 1; i <= NF; i++) if ($i !~ /(\/|\.md)$|\//) ok = 0
                if (ok) for (i = 1; i <= NF; i++) print $i
              }' docs/methodology/PROVENANCE.md | sort -u)
if [ "$actual" = "$listed" ]; then
  ok "PROVENANCE.md's unresolved-link list matches the vendored tree ($(printf '%s\n' "$actual" | grep -c .) entries)"
else
  # ⚠ Temporary files, not `<(...)`: this is /bin/sh and process substitution
  # is a bashism. `docs/methodology/` links to a shell convention document this
  # tree does not vendor, and the rule it carries is the same one.
  _t=$(mktemp -d) || exit 2
  printf '%s\n' "$actual" > "$_t/a"; printf '%s\n' "$listed" > "$_t/b"
  bad "PROVENANCE.md's unresolved-link list is stale"
  printf '         only in the tree: %s\n' "$(comm -23 "$_t/a" "$_t/b" | tr '\n' ' ')"
  printf '         only in the list: %s\n' "$(comm -13 "$_t/a" "$_t/b" | tr '\n' ' ')"
  rm -rf "$_t"
fi

# ---------------------------------------------------------------------------
# 2. repo paths named in backticks resolve
# ---------------------------------------------------------------------------
# ⛔ THE PATTERN IS DELIBERATELY NARROW: a backticked token containing a '/'
# whose first segment is a real top-level directory of this repository. That is
# what makes an exemption list unnecessary for everything else -- a nix store
# path, /usr/lib, $HOME/... and bare command names never match in the first
# place.
#
# ⭐ AND AN OPEN ENTRY IS ALLOWED TO NAME WHAT IT WILL PRODUCE. `authoring.md`
# says an entry carries the `Prove` command that WOULD close it, so
# `experiments/63-developer-friction.sh` inside the open T-013 is the entry
# working correctly, not a stale path. ⛔ Inside a DONE entry the same string is
# a defect: the thing was supposed to have been written. So the severity is
# taken from the status of the entry the line sits in, read from `INDEX.md` --
# never from a list in this script.
# ⚠ `experiments/<n>` and `poc/<n>` with no name are how this tree refers to
# one, and check (4) below resolves them by prefix. They are skipped here so
# the two checks do not disagree about the same string.
tops=$(git ls-files | awk -F/ 'NF>1 {print $1}' | sort -u | tr '\n' '|' | sed 's/|$//')
scan_paths() {  # file -> "strict <path>" or "plan <path>", one per line
  awk -v idx="$R/TODO/INDEX.md" -v tops="$tops" '
    BEGIN {
      split(tops, t, "|"); for (i in t) ours[t[i]] = 1
      while ((getline l < idx) > 0)
        if (l ~ /^\| T-[0-9]+ \|/) {
          split(l, a, "|"); id = a[2]; st = a[5]
          gsub(/ /, "", id); gsub(/ /, "", st); status[id] = st
        }
      strict = 1
    }
    /^## T-[0-9]+ / { strict = (status[$2] == "done") }
    # ⛔ A LINE ABOUT ANOTHER TREE IS NOT A CLAIM ABOUT OURS.
    # `docs/limitations.md` reads *"Upstream own `docs/limits.md` scopes the
    # question further"* -- a correct sentence about
    # `pkgforge-dev/cross-libc-dlopen`, whose repository really does have that
    # file. Four markers say the line is about another project, and each is a
    # phrase this tree already uses for exactly that: a `references/` path, the
    # word "upstream", the word "their", or a backticked `owner/repo` whose
    # first segment is not a directory of ours.
    # ⚠ THE LAST CONDITION NEEDS THAT QUALIFIER. Without it `docs/AGENTS.md` --
    # two segments, one slash -- reads as a foreign repository and every line
    # citing it is skipped, which is the check quietly switching itself off.
    /references\// || /[Uu]pstream/ || /[Tt]heir/ { foreign = 1 }
    {
      line = $0
      probe = $0
      while (match(probe, /`[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+`/)) {
        cand = substr(probe, RSTART + 1, RLENGTH - 2)
        split(cand, seg, "/")
        if (!(seg[1] in ours)) foreign = 1
        probe = substr(probe, RSTART + RLENGTH)
      }
      while (match(line, /`[a-zA-Z0-9_.-]+\/[a-zA-Z0-9_./-]*`/)) {
        if (!foreign)
          print (strict ? "strict " : "plan ") substr(line, RSTART + 1, RLENGTH - 2)
        line = substr(line, RSTART + RLENGTH)
      }
      foreign = 0
    }' "$1"
}
# ⭐ One severity walker, three checks. (2), (3) and (4) all need "which entry
# is this line inside, and is that entry done" -- writing it three times is
# three chances for them to disagree about the same file.
scan_severity() {  # file regex -> "<file>\t<strict|plan>\t<match>"
  awk -v idx="$R/TODO/INDEX.md" -v f="$1" -v re="$2" '
    BEGIN {
      while ((getline l < idx) > 0)
        if (l ~ /^\| T-[0-9]+ \|/) {
          split(l, a, "|"); id = a[2]; st = a[5]
          gsub(/ /, "", id); gsub(/ /, "", st); status[id] = st
        }
      strict = 1
    }
    /^## T-[0-9]+ / { strict = (status[$2] == "done") }
    {
      line = $0
      while (match(line, re)) {
        printf "%s\t%s\t%s\n", f, (strict ? "strict" : "plan"),
               substr(line, RSTART, RLENGTH)
        line = substr(line, RSTART + RLENGTH)
      }
    }' "$1" | sort -u
}

n=0; broken=0; planned=0
for f in $OURS; do
  scan_paths "$f" | while read -r sev p; do printf '%s\t%s\t%s\n' "$f" "$sev" "$p"; done
done > "${TMPDIR:-/tmp}/check-docs.$$" || exit 2
while IFS="$(printf '\t')" read -r f sev p; do
  case "$p" in */) p=${p%/} ;; esac
  case "$p" in *'*'*|*'...'*) continue ;; esac          # a glob or an ellipsis
  case "$p" in experiments/[0-9]*|poc/[0-9]*) continue ;; esac   # check (4)
  printf '%s' "$p" | grep -qE "^($tops)/" || continue
  n=$((n + 1))
  # ⛔ EXISTING IS NOT ENOUGH WHEN THE PATH IS GITIGNORED, and this cost five
  # red CI runs on 2026-09-03d. A document cited
  # `evidence/77-uruntime-header/build/field.AppImage`; `.gitignore` excludes
  # `evidence/*/build/`, so the file was right there on the machine that wrote
  # the sentence and absent in every fresh clone. ⚠ The local gate said the
  # documentation agreed with the tree, five times, and CI disagreed five
  # times -- `docs/AGENTS.md` §0b's "read the CI run; a local gate does not
  # speak for it", earned again.
  # ⭐ Scoped to `evidence/` on purpose: `../pgb` is a build product too and
  # documents name it deliberately. Evidence is the thing that must be
  # committed to be evidence.
  if [ -e "$p" ]; then
    case "$p" in
      evidence/*)
        if git check-ignore -q "$p" 2>/dev/null; then
          bad "$f: names a gitignored build product as evidence -> $p (it exists here and in no fresh clone)"
          broken=$((broken + 1))
        fi
        ;;
    esac
    continue
  fi
  if [ "$sev" = strict ]; then
    bad "$f: names a path that does not exist -> $p"
    broken=$((broken + 1))
  else
    planned=$((planned + 1))
  fi
done < "${TMPDIR:-/tmp}/check-docs.$$"
rm -f "${TMPDIR:-/tmp}/check-docs.$$"
[ "$broken" -eq 0 ] && ok "backticked repo paths resolve ($n checked, $planned named by open entries)"

# ---------------------------------------------------------------------------
# 3. cited evidence files exist
# ---------------------------------------------------------------------------
# ⭐ The same open-entry rule as (2), and for the same reason: an open entry
# names the evidence file its `Prove` command WILL write.
#
# ⛔ AND THE TEST IS THE REPOSITORY, NOT THE DISK, WHICH IS NOT THE SAME THING.
# This check used to be `[ -e "$p" ]`, and that made it INSTRUMENT-DEPENDENT in
# the one way that matters: a gitignored build product -- a POC's binary, an
# extension it built -- is present on the machine that just ran the POC and
# absent from CI's fresh clone. So the gate passed for whoever wrote the
# document and failed for everyone else, which is the worst possible place to
# put a disagreement. Measured 2026-09-03: docs/limitations.md cited
# `evidence/poc/10-gawk/gawk` in a reproduction command, check-docs.sh was
# green on the machine that produced it, and CI's probe-host job was red.
#
# Three outcomes now, and a build product is a legitimate citation:
#
#   tracked by git      the evidence is in the repository            ok
#   gitignored          a BUILD PRODUCT; the document has to say how
#                       to produce it, and citing one is how it does  ok, counted
#   neither             nobody who clones this can see it            FAIL
n=0; broken=0; planned=0; product=0
for f in $OURS; do
  scan_severity "$f" 'evidence/[a-zA-Z0-9_./-]+'
done > "${TMPDIR:-/tmp}/check-docs.$$"
while IFS="$(printf '\t')" read -r f sev p; do
  case "$p" in *'*'*|*'...'*|*.) continue ;; esac
  n=$((n + 1))
  if git ls-files --error-unmatch "$p" >/dev/null 2>&1; then
    continue
  fi
  if git check-ignore -q "$p" 2>/dev/null; then
    product=$((product + 1)); continue
  fi
  if [ "$sev" = strict ]; then
    bad "$f: cites evidence that is neither tracked nor gitignored -> $p"
    broken=$((broken + 1))
  else
    # ⛔ "NAMED BY AN OPEN ENTRY" IS THE RIGHT EXEMPTION FOR WORK NOT YET DONE
    # AND THE WRONG ONE FOR WORK DONE UNDER ANOTHER NAME, and the two are
    # distinguishable: if the evidence DIRECTORY exists and git tracks
    # something in it, the measurement was taken and the citation is simply
    # wrong.
    #
    # ⚠ Measured 2026-09-03c, which is why this branch exists: T-057's Prove
    # line cited `evidence/86-bundler-vs-anylinux/RESULT.txt` for months.
    # `86-` runs against two subjects and writes `RESULT.jq.txt` and
    # `RESULT.mpv.txt`; both were tracked, the directory was right there, and
    # the exemption passed it every time because T-057 is open.
    _ce_dir=$(dirname "$p")
    if [ -d "$_ce_dir" ] && [ -n "$(git ls-files "$_ce_dir" 2>/dev/null | head -1)" ]; then
      bad "$f: cites $p, but $_ce_dir EXISTS and is tracked -- the measurement was taken under another name:"
      git ls-files "$_ce_dir" 2>/dev/null | sed 's|^|           |' | head -6
      broken=$((broken + 1))
    else
      planned=$((planned + 1))
    fi
  fi
done < "${TMPDIR:-/tmp}/check-docs.$$"
rm -f "${TMPDIR:-/tmp}/check-docs.$$"
[ "$broken" -eq 0 ] && ok "cited evidence is in the repository ($n checked, $planned named by open entries, $product build products)"

# ---------------------------------------------------------------------------
# 4. experiments and POCs referenced by number exist
# ---------------------------------------------------------------------------
# ⚠ `experiments/86-` with no name is how this tree refers to one, so the check
# is on the NUMBER's directory or script, not on a full file name.
# ⛔ AND A NUMBER IS NOT REUSABLE. Two entries naming `experiments/89-` for
# different experiments is the collision this resolves by existing: the second
# one to be written silently overwrites, or does not, and the record points at
# whichever won.
n=0; broken=0; planned=0
for f in $OURS; do
  scan_severity "$f" '(experiments|poc)/[0-9]+'
done > "${TMPDIR:-/tmp}/check-docs.$$"
while IFS="$(printf '\t')" read -r f sev p; do
  n=$((n + 1))
  ls -d "$p"* >/dev/null 2>&1 && continue
  if [ "$sev" = strict ]; then
    bad "$f: references $p, which is not in the tree"; broken=$((broken + 1))
  else planned=$((planned + 1)); fi
done < "${TMPDIR:-/tmp}/check-docs.$$"
rm -f "${TMPDIR:-/tmp}/check-docs.$$"
[ "$broken" -eq 0 ] && ok "referenced experiments and POCs exist ($n checked, $planned planned)"

# ---------------------------------------------------------------------------
# 5. entry counts quoted in prose agree with the index
# ---------------------------------------------------------------------------
# ⛔ THE INDEX IS THE ONLY SOURCE. Prose that says "33 entries, 15 open" is a
# copy, and a copy is what goes stale; this is the check that makes the copy
# safe to keep.
i_tot=$(awk '/^ *TOTAL [0-9]+ +OPEN/ {print $2}' TODO/INDEX.md)
i_open=$(awk '/^ *TOTAL [0-9]+ +OPEN/ {print $4}' TODO/INDEX.md)
i_done=$(awk '/^ *TOTAL [0-9]+ +OPEN/ {print $6}' TODO/INDEX.md)
n=0; broken=0
for f in $OURS; do
  [ "$f" = TODO/INDEX.md ] && continue
  while IFS= read -r line; do
    # ⛔ NOT `.*\([0-9][0-9]*\)`. A greedy `.*` eats every digit but the last,
    # so "33 entries" was reported as "3 entries" and the failure message named
    # a number that appears nowhere. The boundary is explicit.
    t=$(printf '%s' "$line" | sed -n 's/.*[^0-9]\([0-9][0-9]*\) entries, \([0-9][0-9]*\) open, \([0-9][0-9]*\) done.*/\1 \2 \3/p')
    [ -n "$t" ] || continue
    n=$((n + 1))
    set -- $t
    [ "$1" = "$i_tot" ] && [ "$2" = "$i_open" ] && [ "$3" = "$i_done" ] && continue
    bad "$f: says '$1 entries, $2 open, $3 done'; INDEX.md says $i_tot/$i_open/$i_done"
    broken=$((broken + 1))
  done < "$f"
done
[ "$broken" -eq 0 ] && ok "quoted entry counts agree with INDEX.md ($n checked)"

# ---------------------------------------------------------------------------
# 5b. quoted experiment and POC counts agree with the tree
# ---------------------------------------------------------------------------
# ⛔ THE SAME DEFECT AS 5, AGAINST A DIFFERENT SOURCE, AND IT HAD ALREADY
# HAPPENED. `docs/AGENTS.md` §9 read "all 24 experiments | every one measured"
# while `ls experiments/*.sh | grep -v lib.sh | wc -l` said 31 -- seven
# experiments written, run and committed without the orientation page moving.
# ⚠ Nothing pointed at it because every OTHER check passes: each experiment
# exists, each has evidence, each is referenced correctly. Only the COUNT was
# wrong, and a count with no source is exactly what gate 5 exists to refuse.
#
# ⚠ The POC count is deliberately the ones in the SUITE. `docs/AGENTS.md` §9
# says "all 10 POCs" and `poc/92-miniflux` is in the tree and explicitly not in
# the count (T-063 owns it), so the number compared against is the count of
# poc/*/run.sh MINUS the ones the same page marks as in progress.
#
# ⛔ SCOPED TO docs/, AND NOT TO TODO/, ON PURPOSE — the first version was not,
# and it fired on `TODO/toolchain.md`'s "all 11 environments, all 9 POCs" in
# T-061's record of what its gate 5 covered. That sentence is CORRECT: there
# were nine POCs then. `docs/AGENTS.md` §14 draws exactly this line — the pages
# in §11 state the CURRENT answer, superseded findings live in the entries and
# in history/corrections.md — so a current-state count check belongs on one
# side of it and not the other. `docs/history/` is excluded for the same
# reason: it exists to hold what was true before.
# ⛔ COUNTED BY THE NAMING RULE, NOT BY AN EXCLUSION LIST. This read
# `ls experiments/*.sh | grep -v lib.sh` until 2026-09-03d, when a SECOND
# library — `experiments/clock.sh`, the wall-clock instrument — was added
# beside `lib.sh` and was counted as an experiment. ⚠ An exclusion list grows
# once per library and is wrong in the window before somebody remembers to
# extend it; `docs/AGENTS.md` §5 already defines an experiment as
# `experiments/NN-*.sh`, so the check now asks that question directly.
x_have=$(ls experiments/[0-9]*-*.sh 2>/dev/null | wc -l | tr -d ' ')
p_all=$(ls -d poc/*/ 2>/dev/null | wc -l | tr -d ' ')
# ⛔ NOT `grep -c … || echo 0`: grep -c PRINTS 0 and EXITS 1 when nothing
# matches, so the fallback fires too and the value becomes "0\n0". This gate
# does not source experiments/lib.sh, so the same thing is done with awk.
p_wip=$(awk '/^⚠ `poc\/[0-9]/ {n++} END {print n+0}' docs/AGENTS.md 2>/dev/null)
p_wip=${p_wip:-0}
p_have=$((p_all - p_wip))
n=0; broken=0
for f in $OURS; do
  case "$f" in
    docs/history/*) continue ;;
    docs/*) ;;
    *) continue ;;
  esac
  while IFS= read -r line; do
    # ⛔ IT MATCHES THE CLAIM FORM, `all N experiments`, AND NOTHING ELSE.
    # Two earlier versions were wrong in opposite directions and both were
    # caught by planting a wrong count:
    #
    #   `\([0-9]*\) experiments`         could not fail on the line it was
    #                                    written for. These pages write
    #                                    `all **31** experiments`, and the
    #                                    digits do not touch the space -- `**`
    #                                    is between. It reported "1 checked",
    #                                    went green, and a planted 24 passed.
    #                                    T-074 is this tree's record of an
    #                                    assertion that cannot fail on the
    #                                    state it was written to catch.
    #   `\([0-9]*\)[^a-zA-Z0-9]*experi…` fired on FIVE false positives at once,
    #                                    because a date reads the same way:
    #                                    `2026-09-05` then a space then the
    #                                    word is digits-punctuation-word.
    #
    # ⚠ Requiring `all` is narrow on purpose. It is the form every count claim
    # in these pages actually uses, and a check with no false positives is one
    # a later session will keep rather than route around.
    q=$(printf '%s' "$line" | sed -n 's/.*[Aa]ll[^a-zA-Z0-9]*\([0-9][0-9]*\)[^a-zA-Z0-9]*experiments.*/\1/p')
    if [ -n "$q" ]; then
      n=$((n + 1))
      if [ "$q" != "$x_have" ]; then
        bad "$f: says '$q experiments'; the tree has $x_have (ls experiments/[0-9]*-*.sh)"
        broken=$((broken + 1))
      fi
    fi
    q=$(printf '%s' "$line" | sed -n 's/.*[Aa]ll[^a-zA-Z0-9]*\([0-9][0-9]*\)[^a-zA-Z0-9]*POCs.*/\1/p')
    if [ -n "$q" ]; then
      n=$((n + 1))
      if [ "$q" != "$p_have" ]; then
        bad "$f: says '$q POCs'; the tree has $p_all under poc/ minus $p_wip marked in progress = $p_have"
        broken=$((broken + 1))
      fi
    fi
  done < "$f"
done
[ "$broken" -eq 0 ] && ok "quoted experiment and POC counts agree with the tree ($n checked; $x_have experiments, $p_have POCs)"

# ---------------------------------------------------------------------------
# 6. every vendored methodology file PROVENANCE.md names is on disk
# ---------------------------------------------------------------------------
n=0; broken=0
for m in $(awk -F'|' '/^\| `[a-z-]+\.md` \|/ {gsub(/[ `]/,"",$2); print $2}' docs/methodology/PROVENANCE.md); do
  n=$((n + 1))
  [ -e "docs/methodology/$m" ] && continue
  bad "PROVENANCE.md lists $m as vendored; it is not on disk"
  broken=$((broken + 1))
done
[ "$n" -gt 0 ] || bad "PROVENANCE.md's vendored table parsed as empty -- the format moved"
[ "$broken" -eq 0 ] && [ "$n" -gt 0 ] && ok "vendored methodology files are on disk ($n checked)"

# ---------------------------------------------------------------------------
# 7. ⛔ no third party's agent instruction file is vendored under references/
#
# ⚠ THIS IS A GATE BECAUSE THE RULE WAS ALREADY BEING BROKEN. docs/AGENTS.md
# §12 recorded "one deliberate deletion" -- a `tree/docs/AGENTS.md` removed by
# hand from one reference on 2026-09-01. On 2026-09-03 a sweep of all 34 found
# TWO MORE that had never been noticed, and re-mining the first put its copy
# straight back. A rule applied to whichever repository somebody happened to
# open is not a rule.
#
# ⭐ mine-repo.sh's trim_tree() now strips them at fetch time and records the
# trim in PROVENANCE.md. This is the half that says so if that ever stops
# working: the fetcher makes it not happen, the gate makes it visible.
found=$(git ls-files -- 'references/*' \
        | grep -E '(^|/)(AGENTS\.md|CLAUDE\.md|GEMINI\.md|\.cursorrules|\.clinerules|\.windsurfrules)$' \
        || true)
if [ -n "$found" ]; then
  for f in $found; do
    bad "a third party's agent instruction file is vendored: $f"
  done
else
  ok "no third-party agent instruction file under references/"
fi

# ---------------------------------------------------------------------------
# 10. ⛔ AN EXPERIMENT'S COMMITTED EVIDENCE MUST NOT PREDATE THE EXPERIMENT.
#
# ⛔ THE DEFECT THIS EXISTS TO CATCH, found by deep review 6 on 2026-09-03c and
# not by reading: `docs/AGENTS.md` §9 says "all 32 experiments | every one
# measured". True -- and SEVEN of them had evidence written before the last
# change to the script that produces it, so what is committed describes a
# different instrument. The sharpest instance was
# `evidence/70-carried-helper/RESULT.txt`, whose table named `pgb-env-debian12`
# -- the environment T-070 RETIRED. The tree carried a glibc 2.36 measurement
# as current while the pin was 2.41, and nothing said so.
#
# ⚠ `TODO/check.sh` gate 8 cannot see this: it forbids a hardcoded environment
# name in CODE and explicitly exempts prose, and `evidence/` is neither.
# `check-docs.sh` gate 3 cannot see it either: the cited file EXISTS. This is
# corrections.md C5 and C23's class -- the file is current, the RUN is not.
#
# ⚠ COMMENT-ONLY EDITS DO NOT COUNT. A script whose only change since its
# evidence is a comment still measures the same thing, and a gate that fired on
# those would be turned off within a week.
#
# ⭐ THE EXEMPTION IS PINNED TO BOTH COMMITS. `evidence/STALE-EVIDENCE.txt`
# names pairs that are known-stale and too expensive to re-run right now. An
# entry stops matching the moment EITHER side moves, so it cannot silently
# outlive its reason.
n=0; stale=0; exempt=0
EXEMPT_FILE=evidence/STALE-EVIDENCE.txt
for s in experiments/[0-9]*.sh; do
  [ -e "$s" ] || continue
  # ⛔ MATCH THE EVIDENCE BY THE SCRIPT'S OWN SLUG, NOT BY ITS NUMBER.
  # This read `ls -d evidence/<num>-* | head -1` until 2026-09-03d, when a new
  # `experiments/92-size-is-time.sh` was checked against `evidence/92-go-port`
  # -- a legacy evidence directory under a number NO experiment uses, which
  # sorts first. ⚠ The gate then compared a script to somebody else's run and
  # reported it stale, which is the same shape of wrong answer it exists to
  # catch. The numeric fallback is kept for the directories that legitimately
  # differ in slug, and it is used ONLY when it is unambiguous.
  base=$(basename "$s" .sh)
  if [ -d "evidence/$base" ]; then
    d="evidence/$base"
  else
    num=$(basename "$s" | sed 's/-.*//')
    set -- evidence/"$num"-*
    if [ "$#" -eq 1 ] && [ -d "$1" ]; then d="$1"; else d=""; fi
  fi
  [ -n "$d" ] || continue
  n=$((n + 1))
  sc=$(git log -1 --format=%H -- "$s" 2>/dev/null)
  ec=$(git log -1 --format=%H -- "$d" 2>/dev/null)
  [ -n "$sc" ] && [ -n "$ec" ] || continue
  st=$(git log -1 --format=%ct -- "$s" 2>/dev/null)
  et=$(git log -1 --format=%ct -- "$d" 2>/dev/null)
  [ "${st:-0}" -gt "${et:-0}" ] || continue
  # ⛔ EVIDENCE BEING RE-RUN RIGHT NOW IS NOT STALE, and the first version of
  # this gate could not be satisfied before the commit it was demanding: you
  # re-run the experiment, the working tree holds the fresh result, `git log`
  # still points at the old commit, the gate fails, and the gate has to be
  # green BEFORE you may commit. A check that can only pass after the commit it
  # blocks is a check people delete. An uncommitted change under the evidence
  # directory means the answer is being replaced in this very commit.
  [ -z "$(git status --porcelain -- "$d" 2>/dev/null)" ] || continue
  # Did the script change in a way that could change what it measures?
  real=$(git diff "$ec".."$sc" -- "$s" 2>/dev/null \
         | grep -E '^[+-]' | grep -vE '^(\+\+\+|---)' \
         | sed 's/^[+-][[:space:]]*//' | grep -vE '^(#|$)' | grep -c . || true)
  [ "${real:-0}" -gt 0 ] || continue
  if [ -e "$EXEMPT_FILE" ] \
     && grep -qE "^$(basename "$s")[[:space:]]+${sc}[[:space:]]+${ec}([[:space:]]|$)" "$EXEMPT_FILE"; then
    exempt=$((exempt + 1))
    continue
  fi
  bad "$(basename "$s") changed after its evidence was written ($real non-comment line(s)); re-run it, or pin it in $EXEMPT_FILE as: $(basename "$s") $sc $ec <reason>"
  stale=$((stale + 1))
done
if [ "$stale" -eq 0 ]; then
  ok "committed evidence is no older than its experiment ($n checked, $exempt pinned stale)"
fi

echo
if [ "$fail" -eq 0 ]; then
  echo "VERDICT: the documentation agrees with the tree."
else
  echo "VERDICT: $fail disagreement(s)."
fi
exit $([ "$fail" -eq 0 ] && echo 0 || echo 1)
