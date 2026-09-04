#!/bin/sh
# THE QUESTION
#
#   TODO/ci.md T-084 says the trace classifier is SIX HAND COPIES and that one
#   of them counts a FAILED open as a load (docs/history/corrections.md C25).
#   Its step 2 -- convert the six and re-run them -- is expensive: `90-` and
#   `86-` build kdenlive-scale bundles. ⭐ SO ASK THE CHEAP QUESTION FIRST:
#   WHERE DO THE COPIES ACTUALLY DISAGREE WITH THE SHARED CLASSIFIER, and does
#   the disagreement run only the one way the entry claims?
#
# ⛔ WHY THAT QUESTION IS NOT RHETORICAL. T-084's central claim is:
#
#     "THE ERROR ONLY RUNS ONE WAY. It can turn a clean row dirty and can
#      never turn a dirty row clean, so every committed ZERO stands."
#
#   That claim is about C25. It is not a claim about the copies, and nobody
#   had diffed them. This experiment diffs them, on fixtures, with no bundle
#   build at all -- so the expensive re-run is entered knowing what it can
#   move, instead of being the thing that finds out.
#
# -- ⚠ HOW THESE EXPECTATIONS WERE ARRIVED AT, STATED PLAINLY ---------------
#
# ⚠ NOT A BLIND PRE-REGISTRATION, AND SAYING SO IS THE POINT OF DELIVERY
# RULE 1. E1 and E2 below were measured before this file existed, by stripping
# comments from the six function bodies and hashing them. D1..D5 were then
# DERIVED BY READING the two distinct bodies against `exp_classify_trace` --
# derived, not run. ⭐ That is the interesting half: a prediction read out of
# source and then executed is falsifiable in a way a hand run is not.
#
# -- ⛔ EXPECTATIONS ---------------------------------------------------------
#
#   E1  ⛔ THIS PREDICTION WAS WRITTEN AS "SIX FILES, TWO IMPLEMENTATIONS"
#       AND IT WAS WRONG -- kept here as written, because a prediction edited
#       after its run is not one. It read THREE. The cause is that it was
#       measured by HASHING the comment-stripped bodies, and a hash measures
#       TEXT: `90-` is `62-`'s code with one rule reflowed across four lines,
#       so it hashes differently and behaves identically.
#       ⭐ THE NUMBER THE PREDICTION MEANT IS BEHAVIOURAL, so E1b measures it
#       behaviourally -- every copy's output over all five fixtures in both
#       modes, hashed -- and that reads TWO. Two texts that behave alike are
#       one implementation; only running them can say so.
#   E2  all six extract. ⛔ A copy that fails to extract must SKIP loudly --
#       silently comparing five would report agreement it never measured.
#
#   D1  ⭐ THE POSITIVE CONTROL. On a fixture with nothing pathological in it,
#       every copy agrees with the shared classifier in BOTH modes. Without
#       this row a disagreement below could be the harness mis-loading a copy
#       rather than the copy differing. 12 cells, all equal.
#   D2  ⛔ C25 -- the split failed open. All six count it, the shared one does
#       not, in both modes. This row is also the harness's own control in the
#       other direction: if it reported agreement, the copies are not loaded.
#   D3  ⛔ THE ROW THE ENTRY DOES NOT HAVE. When the artefact path is exec'd
#       a SECOND time, the two implementations disagree with the shared
#       classifier in OPPOSITE MODES:
#         - `62-`/`85-`/`86-`/`89-`/`90-` `delete cur` UNCONDITIONALLY on the
#           artefact's own execve, so in TREE mode they throw away everything
#           counted before it -- a DIRTY row turned CLEAN;
#         - `60-` never deletes there, so in PAYLOAD mode it keeps objects the
#           second exec unmapped -- a CLEAN row turned DIRTY.
#       ⛔ If D3 holds, T-084's "the error only runs one way" is TRUE OF C25
#       AND FALSE OF THE COPIES, and a committed ZERO from `62-` or `90-` --
#       the two that call TREE mode -- is not automatically safe.
#   D4  `/etc/ld.so.cache` is an index, not an object: zero from all seven.
#   D5  a SPLIT clone is followed by all seven -- the child pid is only on the
#       resumed line, and `62-` is where missing that cost a whole row.
#
#   R1  ⭐ DOES D3 EVER FIRE? A defect that no real trace can reach is a
#       latent hazard, not a wrong number, and the difference decides whether
#       the committed rows have to be re-run or merely re-labelled. Counted on
#       a REAL trace: how many `execve("<artefact>"` lines does one carry?
#       ⚠ SKIPS when no trace is on disk. A skip is not a pass.
#
# ⛔ WHAT THIS EXPERIMENT DOES NOT MEASURE. It does not re-run `60-`, `62-`,
# `85-`, `86-`, `89-` or `90-`, so it moves no committed number. It says which
# committed numbers CAN move, and in which direction. T-084 step 2 still owes
# the re-run.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "102 - the trace classifier: where the six hand copies disagree with the shared one"

WORK="${PGB_EXP102_WORK:-/var/tmp/t102}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
SRC="$(cd "$(dirname "$0")" && pwd)"

COPIES='60-versus-alternatives 62-anylinux-appimage 85-opengl
        86-bundler-vs-anylinux 89-debloat 90-kdenlive-vs-enhanced'

# ---------------------------------------------------------------------------
# E: extract each hand copy into a file that can be sourced on its own.
#
# ⛔ THE EXTRACTION IS ITSELF A MEASUREMENT AND IS CHECKED. `awk` between the
# opening line and a bare `}` is only correct while every copy keeps that
# shape; a copy that grew a nested `}` at column 0 would extract short and
# then fail to parse. Each extracted body is therefore run once on the
# positive-control fixture before it is trusted, and a copy that cannot answer
# it is SKIPPED by name rather than silently dropped from the table.
# ---------------------------------------------------------------------------
printf -- '-- E: the copies -------------------------------------------------\n'
got=0
for f in $COPIES; do
  [ -f "$SRC/$f.sh" ] || continue
  awk '/^classify_trace\(\)/,/^}$/' "$SRC/$f.sh" > "$WORK/body.$f"
  [ -s "$WORK/body.$f" ] || continue
  # the sourced runner: $1 trace, $2 want, $3 mode
  { printf '#!/bin/sh\n'; cat "$WORK/body.$f"
    printf 'classify_trace "$1" "$2" "$3"\n'; } > "$WORK/run.$f"
  got=$((got+1))
  # comment-stripped body, for the identity count
  grep -v '^ *#' "$WORK/body.$f" | sed 's/[[:space:]]*$//' > "$WORK/bare.$f"
done
exp_check "E2  every named hand copy extracted" "$got" 6

# ⛔ How many distinct TEXTS, ignoring comments and trailing space. This is
# what the E1 prediction actually measured, and why it was wrong: `90-` is
# `62-` with one rule reflowed, which changes the hash and not the behaviour.
distinct=$(md5sum "$WORK"/bare.* 2>/dev/null | awk '{print $1}' | sort -u | wc -l)
exp_check "E1a ⛔ distinct TEXTS (the prediction said 2)" "$distinct" 3
for f in $COPIES; do
  [ -f "$WORK/bare.$f" ] || continue
  exp_note "$(printf '%-26s %s' "$f" "$(md5sum "$WORK/bare.$f" | cut -c1-8)")"
done

# ---------------------------------------------------------------------------
# The fixtures. Each is strace's real line shape; the artefact is `/subj`.
# ---------------------------------------------------------------------------
cat > "$WORK/F1" <<'TRACE'
100 execve("/subj", ["/subj"], 0x7ffd) = 0
100 openat(AT_FDCWD, "/usr/lib/x86_64-linux-gnu/libhost.so.6", O_RDONLY) = 3
100 clone(child_stack=NULL) = 200
200 openat(AT_FDCWD, "/tmp/mnt/lib/libbundled.so.1", O_RDONLY) = 4
TRACE

cat > "$WORK/F2" <<'TRACE'
100 execve("/subj", ["/subj"], 0x7ffd) = 0
100 openat(AT_FDCWD, "/usr/lib/libfailed.so.1", O_RDONLY <unfinished ...>
100 <... openat resumed>) = -1 ENOENT (No such file or directory)
TRACE

cat > "$WORK/F3" <<'TRACE'
100 execve("/subj", ["/subj"], 0x7ffd) = 0
100 openat(AT_FDCWD, "/usr/lib/libearly.so.1", O_RDONLY) = 3
100 execve("/subj", ["/subj", "--stage2"], 0x7ffd) = 0
100 openat(AT_FDCWD, "/usr/lib/liblate.so.2", O_RDONLY) = 4
TRACE

cat > "$WORK/F4" <<'TRACE'
100 execve("/subj", ["/subj"], 0x7ffd) = 0
100 openat(AT_FDCWD, "/etc/ld.so.cache", O_RDONLY) = 3
TRACE

cat > "$WORK/F5" <<'TRACE'
100 execve("/subj", ["/subj"], 0x7ffd) = 0
100 clone(child_stack=NULL <unfinished ...>
100 <... clone resumed>) = 200
200 openat(AT_FDCWD, "/usr/lib/libchild.so.1", O_RDONLY) = 3
TRACE

# agree_count FIXTURE MODE -> how many of the extracted copies produce exactly
# what exp_classify_trace produces. The comparison is the full sorted output,
# not a count, so two different object sets of the same size do not read equal.
agree_count() {  # fixture mode
  _ref=$(exp_classify_trace "$WORK/$1" /subj "$2")
  _n=0
  for _c in $COPIES; do
    [ -x /bin/sh ] || return 2
    [ -f "$WORK/run.$_c" ] || continue
    _got=$(sh "$WORK/run.$_c" "$WORK/$1" /subj "$2" 2>/dev/null)
    [ "$_got" = "$_ref" ] && _n=$((_n+1))
  done
  printf '%s' "$_n"
}

# copies_matching FIXTURE MODE PATTERN -> how many copies print PATTERN
copies_matching() {  # fixture mode pattern
  _n=0
  for _c in $COPIES; do
    [ -f "$WORK/run.$_c" ] || continue
    sh "$WORK/run.$_c" "$WORK/$1" /subj "$2" 2>/dev/null \
      | grep -q "$3" && _n=$((_n+1))
  done
  printf '%s' "$_n"
}

ref_matching() { exp_classify_trace "$WORK/$1" /subj "$2" | grep -c "$3" || true; }

# ⭐ E1b: THE SAME QUESTION ASKED BEHAVIOURALLY. A copy's signature is its
# output over every fixture in both modes; two copies with one signature are
# one implementation however differently they are written. ⛔ This is the
# number E1 meant, and it is the only one of the two that bounds how much
# work step 2 is.
printf '\n-- E1b: ⭐ distinct BEHAVIOURS, not distinct text -------------------\n'
for c in $COPIES; do
  [ -f "$WORK/run.$c" ] || continue
  : > "$WORK/sig.$c"
  for fx in F1 F2 F3 F4 F5; do
    for md in tree payload; do
      sh "$WORK/run.$c" "$WORK/$fx" /subj "$md" >> "$WORK/sig.$c" 2>/dev/null
      printf -- '--\n' >> "$WORK/sig.$c"
    done
  done
done
behaviours=$(md5sum "$WORK"/sig.* 2>/dev/null | awk '{print $1}' | sort -u | wc -l)
for c in $COPIES; do
  [ -f "$WORK/sig.$c" ] || continue
  exp_note "$(printf '%-26s behaviour %s' "$c" "$(md5sum "$WORK/sig.$c" | cut -c1-8)")"
done
exp_check "E1b ⭐ distinct BEHAVIOURS among the six" "$behaviours" 2

printf '\n-- D1: ⭐ THE POSITIVE CONTROL, nothing pathological ---------------\n'
exp_note "$(printf 'reference tree   : %s' "$(exp_classify_trace "$WORK/F1" /subj tree | tr '\n' ' ')")"
exp_note "$(printf 'reference payload: %s' "$(exp_classify_trace "$WORK/F1" /subj payload | tr '\n' ' ')")"
exp_check "D1  all six agree on a clean fixture (tree)"    "$(agree_count F1 tree)"    6
exp_check "D1  all six agree on a clean fixture (payload)" "$(agree_count F1 payload)" 6

printf '\n-- D2: ⛔ C25, the SPLIT FAILED open --------------------------------\n'
exp_check "D2  the shared classifier does NOT count it"   "$(ref_matching F2 tree libfailed)" 0
exp_check "D2  ⛔ every copy DOES count it (tree)"        "$(copies_matching F2 tree libfailed)"    6
exp_check "D2  ⛔ every copy DOES count it (payload)"     "$(copies_matching F2 payload libfailed)" 6
exp_note "⭐ D2 is also the harness's control: agreement here would mean the"
exp_note "   copies are not being loaded at all."

printf '\n-- D3: ⛔ THE ARTEFACT EXECD TWICE -- the row T-084 does not have ---\n'
exp_note "$(printf 'reference tree   : %s' "$(exp_classify_trace "$WORK/F3" /subj tree | tr '\n' ' ')")"
exp_note "$(printf 'reference payload: %s' "$(exp_classify_trace "$WORK/F3" /subj payload | tr '\n' ' ')")"
for c in $COPIES; do
  [ -f "$WORK/run.$c" ] || continue
  exp_note "$(printf '%-26s tree=[%s] payload=[%s]' "$c" \
      "$(sh "$WORK/run.$c" "$WORK/F3" /subj tree    | tr '\n' ' ')" \
      "$(sh "$WORK/run.$c" "$WORK/F3" /subj payload | tr '\n' ' ')")"
done
# ⛔ In TREE mode the reference keeps BOTH objects; the five that delete
# unconditionally keep only the late one -- a DIRTY row turned CLEAN.
exp_check "D3  ⛔ copies agreeing in TREE mode (5 must not)"    "$(agree_count F3 tree)"    1
# ⛔ In PAYLOAD mode the reference keeps only the late one; `60-`, which never
# deletes on the artefact's own execve, keeps both -- a CLEAN row turned DIRTY.
exp_check "D3  ⛔ copies agreeing in PAYLOAD mode (60- must not)" "$(agree_count F3 payload)" 5

printf '\n-- D4/D5: the two defects that were already fixed everywhere -------\n'
exp_check "D4  ld.so.cache is an index: reference counts 0" \
    "$(exp_classify_trace "$WORK/F4" /subj tree | grep -c . || true)" 0
exp_check "D4  ...and no copy counts it either" "$(copies_matching F4 tree 'ld.so.cache')" 0
exp_check "D5  a SPLIT clone is followed (tree)"    "$(agree_count F5 tree)"    6
exp_check "D5  ...and the child is not the payload" "$(agree_count F5 payload)" 6

# ---------------------------------------------------------------------------
# R1: does D3 describe a real trace, or a hazard nothing reaches?
#
# ⛔ THE DIFFERENCE DECIDES WHAT STEP 2 OWES. If a real delivery execs the
# artefact path exactly once, the unconditional `delete cur` never fires and
# every committed number stands as recorded -- the copies are still wrong and
# still must be converted, but nothing has to be re-measured for THAT reason.
# If it execs twice, `62-` and `90-` -- the two that call TREE mode -- have
# committed numbers that are too LOW, and a zero from them is not evidence.
# ---------------------------------------------------------------------------
printf '\n-- R1: ⭐ how many times does a REAL trace exec the artefact? ------\n'
TR="${PGB_EXP102_TRACE:-}"
if [ -z "$TR" ]; then
  TR=$(ls -1t /var/tmp/t065/tr.* /var/tmp/t065b/tr.* 2>/dev/null | head -1)
fi
if [ -n "$TR" ] && [ -f "$TR" ]; then
  # the corpus plants its subject at /subj65; take the artefact from the trace
  # itself rather than assuming the name.
  wants=$(grep -o 'execve("/subj[^"]*"' "$TR" | sort -u | head -1 \
          | sed 's/execve("//; s/"$//')
  if [ -n "$wants" ]; then
    n=$(grep -c "execve(\"$wants\"" "$TR" || true)
    exp_note "$(printf 'trace   : %s' "$TR")"
    exp_note "$(printf 'artefact: %s' "$wants")"
    exp_check "R1  ⭐ execve(\"<artefact>\") lines in a real trace" "$n" 1
    exp_note "⭐ ONE. The unconditional delete has nothing to delete, so the"
    exp_note "   committed numbers stand -- as a MEASUREMENT, not an argument."
    exp_note "⛔ It is one delivery shape. A wrapper that re-execs the artefact"
    exp_note "   under its own name would make D3 fire, and nothing prevents one."
  else
    exp_skip "R1  ⭐ execve(\"<artefact>\") lines in a real trace" "no /subj* exec in $TR"
  fi
else
  exp_skip "R1  ⭐ execve(\"<artefact>\") lines in a real trace" "no trace on disk"
fi

# ---------------------------------------------------------------------------
# S: ⭐ WHICH COMMITTED NUMBER DOES C38 ACTUALLY REACH?
#
# D3 says the copies differ from the shared classifier when the artefact is
# exec'd a SECOND time. R1 says the corpus's shape execs it once. ⛔ Neither
# says anything about the six experiments that CARRY a copy -- and that is the
# only question with a committed number behind it.
#
# Two facts decide it per experiment, and both are read out of the experiment:
#
#   1. WHICH MODE it calls. The 62-family clears unconditionally, so it can
#      only differ from the shared classifier in TREE mode; `60-` never clears
#      there, so it can only differ in PAYLOAD mode.
#   2. HOW MANY TIMES its traced run invokes the artefact. With one invocation
#      there is nothing to clear and both defects are latent.
#
# ⚠ The site count is a grep and it is an UPPER BOUND on invocations -- a site
# inside a loop is one line and many runs. It is asserted anyway, because its
# job here is DRIFT DETECTION: if someone edits one of these six, the count
# moves and this row fails rather than the verdict below going quietly stale.
# ---------------------------------------------------------------------------
printf '\n-- S: ⭐ which committed number does C38 actually reach? ------------\n'

# name : artefact : modes it calls : call sites : invocations in the TRACED run
S_TABLE='60-versus-alternatives:/pgb-vs-arm:payload+tree:2:1
62-anylinux-appimage:/vs-arm:payload+tree:3:1
85-opengl:/gl-arm:payload:1:1
86-bundler-vs-anylinux:/vs-arm:payload:4:4
89-debloat:/gl-arm:payload:1:1
90-kdenlive-vs-enhanced:/kd-arm:tree:2:2'

sites_of() {  # file artefact -> non-comment, non-plumbing mentions
  grep -n -- "$2" "$SRC/$1.sh" \
    | grep -v 'classify_trace\|cp "\|rm -f\|chmod\|^[0-9]*:#\|^[0-9]*: *#' \
    | grep -c . || true
}

s_drift=0; s_fires=''
printf '        %-26s %-12s %-6s %-6s %s\n' EXPERIMENT MODE SITES INVOKE 'C38?'
for row in $S_TABLE; do
  f=$(printf '%s' "$row" | cut -d: -f1)
  art=$(printf '%s' "$row" | cut -d: -f2)
  mds=$(printf '%s' "$row" | cut -d: -f3)
  exp_sites=$(printf '%s' "$row" | cut -d: -f4)
  inv=$(printf '%s' "$row" | cut -d: -f5)
  got=$(sites_of "$f" "$art")
  [ "$got" = "$exp_sites" ] || s_drift=$((s_drift+1))
  # C38 reaches an experiment only if it calls the mode its copy differs in
  # AND its traced run execs the artefact more than once.
  case "$f" in
    60-*) diffmode=payload ;;
    *)    diffmode=tree ;;
  esac
  fires=no
  case "$mds" in *"$diffmode"*) [ "$inv" -gt 1 ] && fires=YES ;; esac
  [ "$fires" = YES ] && s_fires="$s_fires $f"
  printf '        %-26s %-12s %-6s %-6s %s\n' "$f" "$mds" "$got" "$inv" "$fires"
done
exp_check "S1  no experiment drifted from its recorded call sites" "$s_drift" 0
exp_check "S2  ⛔ experiments C38 actually REACHES" \
    "$(printf '%s' "$s_fires" | wc -w)" 1
exp_note "$(printf '⛔ and it is:%s -- the one T-084 already named' "$s_fires")"
exp_note "⛔ 90- calls TREE mode and its test script invokes the artefact TWICE"
exp_note "   (melt -version, then a real encode), so the unconditional clear"
exp_note "   FIRES and its host counts describe only the SECOND invocation."
exp_note "⛔ THAT INCLUDES OUR OWN \"0 of 11\", not just the competitor's."
exp_note "⚠ How much it moves is NOT measured: the second invocation plausibly"
exp_note "   loads a superset of the first. Plausibly is not measured."

printf '\n-- ⛔ what this does NOT establish ---------------------------------\n'
exp_note "⛔ No committed number moved. Six experiments still carry a copy;"
exp_note "   T-084 step 2 owes the conversion AND the re-run."
exp_note "⛔ R1 measures ONE trace from ONE delivery shape. It bounds D3 for"
exp_note "   that shape and for nothing else."
exp_note "⚠ The fixtures are hand-written strace shapes, not captured output."
exp_note "   They are the shapes C25 and the clone defect were found in."

exp_finish
