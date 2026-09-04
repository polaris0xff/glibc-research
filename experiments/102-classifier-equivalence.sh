#!/bin/sh
# THE QUESTION
#
#   TODO/ci.md T-084 said the trace classifier was SIX HAND COPIES carrying two
#   defects the shared `exp_classify_trace` does not have. ⭐ THE COPIES ARE
#   GONE — converted 2026-09-04c — so this experiment asks the two questions
#   that outlive them:
#
#     1. do they STAY gone, and does every converted call site still pass a
#        mode the shared classifier implements?
#     2. WHAT DID THE CONVERSION CHANGE? Not as a story: the copies are read
#        back out of git at the pinned pre-conversion commit and run against
#        the shared classifier on the same fixtures, so the before/after is
#        re-derivable from this tree rather than remembered.
#
# ⛔ WHY QUESTION 2 IS NOT DECORATION. T-084's Prove line asks for the
# before/after host count side by side, and five of the six are too expensive
# to re-run for that alone (`86-` and `90-` build kdenlive-scale bundles).
# ⭐ Running the OLD code from git on FIXTURES gives the direction and the size
# of the change for all six at no cost, and leaves only `90-` — the one C38
# actually reaches — needing the real re-run.
#
# -- ⛔ WHAT THE COPIES DID, AND IT IS ASSERTED BELOW, NOT RECITED -----------
#
#   C25  strace splits a long call across `openat(..., "path" <unfinished ...>`
#        and `<... openat resumed>) = -1 ENOENT`. The PATH is on the first line
#        and the RESULT on the second, so a filter that drops lines containing
#        ENOENT keeps the first half of a FAILED open. ⛔ CLEAN → DIRTY. All
#        six copies had it; the shared classifier does not.
#   C38  five of the six cleared their result set on the artefact's own execve
#        UNCONDITIONALLY, so in `tree` mode everything opened before the last
#        invocation vanished. ⛔ DIRTY → CLEAN. `60-` never cleared there, so
#        it errs the OTHER way, in `payload` mode. ⛔ "The error only runs one
#        way" was true of C25 and false of the copies.
#
# -- ⛔ EXPECTATIONS ---------------------------------------------------------
#
#   G1  ⭐ THE STANDING GUARD. No file under experiments/ defines its own
#       `classify_trace`. This is the row that keeps the conversion from being
#       undone by the next person who needs a classifier in a hurry.
#   G2  every one of the six converted experiments calls `exp_classify_trace`,
#       and the call sites match a recorded count — drift detection, so an
#       edit fails this row rather than making the table quietly stale.
#   G3  every mode passed at a call site is one the shared classifier
#       implements. An unknown mode is a LOUD error there and a silent zero
#       would be the worst outcome; this asserts the loudness.
#
#   H0  all six copies extract from the pinned commit. ⛔ A copy that fails to
#       extract must SKIP loudly — silently comparing five would report an
#       agreement it never measured.
#   H1  ⭐ distinct BEHAVIOURS among the six: 2, not 6 and not the 3 that
#       hashing their TEXT reads. `90-` is `62-` reflowed.
#   D1  ⭐ THE POSITIVE CONTROL. On a fixture with nothing pathological in it
#       every copy agrees with the shared classifier in BOTH modes — so a
#       disagreement below is the copy differing, not the harness mis-loading.
#   D2  ⛔ C25: all six count the split failed open, the shared one does not.
#   D3  ⛔ C38: with the artefact exec'd TWICE the two implementations differ
#       from the shared classifier in OPPOSITE modes.
#   D4  `/etc/ld.so.cache` is an index, not an object: zero from all seven.
#   D5  a SPLIT clone is followed by all seven.
#
#   R1  ⭐ DOES D3 EVER FIRE? Counted on a REAL trace: how many
#       `execve("<artefact>")` lines does one carry? ⚠ SKIPS when no trace is
#       on disk. A skip is not a pass.
#   S1  ⛔ which committed number C38 reaches, read off each experiment's mode
#       and its traced run's invocation count.
#
# ⛔ WHAT THIS EXPERIMENT DOES NOT MEASURE. It runs no bundle, so it moves no
# committed number by itself. It says which committed numbers the conversion
# CAN move, and in which direction. `90-`'s re-run is what moves one.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "102 - the trace classifier: the copies are gone, and what their removal changed"

WORK="${PGB_EXP102_WORK:-/var/tmp/t102}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
SRC="$(cd "$(dirname "$0")" && pwd)"

COPIES='60-versus-alternatives 62-anylinux-appimage 85-opengl
        86-bundler-vs-anylinux 89-debloat 90-kdenlive-vs-enhanced'

# ⛔ THE LAST COMMIT THAT CARRIED THE SIX HAND COPIES. Pinned, so this
# experiment reads the same bytes on every machine and every day. ⚠ If a
# rewritten history ever loses it, H0 SKIPS loudly rather than comparing
# nothing and reporting agreement.
PRE='b4e53f31'

# ---------------------------------------------------------------------------
# G: ⭐ THE STANDING GUARD — the copies are gone and they stay gone.
# ---------------------------------------------------------------------------
printf -- '-- G: ⭐ the copies are gone --------------------------------------\n'
dupes=$(grep -l '^classify_trace()' "$SRC"/*.sh 2>/dev/null | grep -c . || true)
exp_check "G1  ⭐ files under experiments/ defining their own classifier" "$dupes" 0

# name : artefact : modes it calls : exp_classify_trace call sites
G_TABLE='60-versus-alternatives:/pgb-vs-arm:payload+tree:2
62-anylinux-appimage:/vs-arm:payload+tree:2
85-opengl:/gl-arm:payload:1
86-bundler-vs-anylinux:/vs-arm:payload:1
89-debloat:/gl-arm:payload:1
90-kdenlive-vs-enhanced:/kd-arm:tree:2'

g_drift=0; g_badmode=0
printf '        %-26s %-14s %-6s %s\n' EXPERIMENT MODE CALLS 'MODES SEEN'
for row in $G_TABLE; do
  f=$(printf '%s' "$row" | cut -d: -f1)
  mds=$(printf '%s' "$row" | cut -d: -f3)
  want=$(printf '%s' "$row" | cut -d: -f4)
  # ⛔ COMMENT LINES ARE NOT CALL SITES. The first version of this counted them
  # and read 4 where there are 2 — and its mode list picked `[payload` out of
  # the block comment that documents the signature.
  calls=$(grep 'exp_classify_trace ' "$SRC/$f.sh" | grep -v '^[[:space:]]*#' || true)
  got=$(printf '%s\n' "$calls" | grep -c . || true)
  [ "$got" = "$want" ] || g_drift=$((g_drift+1))
  # ⛔ EVERY MODE ARGUMENT AT A CALL SITE MUST BE ONE THE SHARED CLASSIFIER
  # IMPLEMENTS. The third word after the call is the mode; a call with only two
  # defaults to `tree`, which is what the pre-conversion behaviour was.
  seen=$(printf '%s\n' "$calls" | sed 's/.*exp_classify_trace //' \
         | awk '{ m=$3; sub(/[)|].*$/,"",m); if (m=="") m="(default)"; print m }' \
         | tr -d '"' | sort -u | tr '\n' ' ')
  for m in $seen; do
    case "$m" in payload|tree|'(default)') ;; *) g_badmode=$((g_badmode+1)) ;; esac
  done
  printf '        %-26s %-14s %-6s %s\n' "$f" "$mds" "$got" "$seen"
done
exp_check "G2  no experiment drifted from its recorded call-site count" "$g_drift" 0
exp_check "G3  every mode passed is one the classifier implements" "$g_badmode" 0

# ⭐ AND THE LOUDNESS IS ASSERTED, NOT ASSUMED. An unknown mode must be an
# error with a message, never a silent fallback to `tree` — the fallback would
# make a mistyped mode read as today's answer and hide itself forever.
: > "$WORK/empty"
bad_out=$(exp_classify_trace "$WORK/empty" /subj nonsense 2>&1); bad_st=$?
exp_check "G3  ...and an unknown mode is a LOUD error (status)" "$bad_st" 2
exp_check "G3  ...with a message naming the modes" \
    "$(printf '%s' "$bad_out" | grep -c 'payload|tree' || true)" 1

# ---------------------------------------------------------------------------
# H: read the six copies back out of git and run them.
#
# ⛔ THE EXTRACTION IS ITSELF A MEASUREMENT AND IS CHECKED. `awk` between the
# opening line and a bare `}` is only correct while every copy kept that shape.
# Each extracted body is run once on the positive-control fixture before it is
# trusted, and a copy that cannot answer it is SKIPPED by name.
# ---------------------------------------------------------------------------
printf -- '\n-- H: ⭐ the copies as they were, read back out of git -------------\n'
have_git=no
git -C "$REPO_DIR" cat-file -e "$PRE^{commit}" 2>/dev/null && have_git=yes
if [ "$have_git" = no ]; then
  exp_skip "H0  the six copies extract from $PRE" "commit $PRE not in this clone"
else
  got=0
  for f in $COPIES; do
    git -C "$REPO_DIR" show "$PRE:experiments/$f.sh" 2>/dev/null \
      | awk '/^classify_trace\(\)/,/^}$/' > "$WORK/body.$f"
    [ -s "$WORK/body.$f" ] || continue
    { printf '#!/bin/sh\n'; cat "$WORK/body.$f"
      printf 'classify_trace "$1" "$2" "$3"\n'; } > "$WORK/run.$f"
    got=$((got+1))
  done
  exp_check "H0  the six copies extract from $PRE" "$got" 6
fi

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

ncopies=$(ls "$WORK"/run.* 2>/dev/null | grep -c . || true)
if [ "$ncopies" -ne 6 ]; then
  exp_skip "H1/D1..D5  the copies against the shared classifier" \
           "$ncopies of 6 copies loaded"
else

# ⭐ H1: THE COUNT THAT BOUNDS THE CONVERSION. A copy's signature is its output
# over every fixture in both modes; two copies with one signature are one
# implementation however differently they are written. ⛔ Hashing their TEXT
# reads THREE — `90-` is `62-` with one rule reflowed — and only running them
# says two.
printf '\n-- H1: ⭐ distinct BEHAVIOURS, not distinct text -------------------\n'
for c in $COPIES; do
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
  exp_note "$(printf '%-26s behaviour %s' "$c" "$(md5sum "$WORK/sig.$c" | cut -c1-8)")"
done
exp_check "H1  ⭐ distinct BEHAVIOURS among the six" "$behaviours" 2

printf '\n-- D1: ⭐ THE POSITIVE CONTROL, nothing pathological ---------------\n'
exp_note "$(printf 'reference tree   : %s' "$(exp_classify_trace "$WORK/F1" /subj tree | tr '\n' ' ')")"
exp_note "$(printf 'reference payload: %s' "$(exp_classify_trace "$WORK/F1" /subj payload | tr '\n' ' ')")"
exp_check "D1  all six agree on a clean fixture (tree)"    "$(agree_count F1 tree)"    6
exp_check "D1  all six agree on a clean fixture (payload)" "$(agree_count F1 payload)" 6

printf '\n-- D2: ⛔ C25, the SPLIT FAILED open --------------------------------\n'
exp_check "D2  the shared classifier does NOT count it"   "$(ref_matching F2 tree libfailed)" 0
exp_check "D2  ⛔ every copy DID count it (tree)"         "$(copies_matching F2 tree libfailed)"    6
exp_check "D2  ⛔ every copy DID count it (payload)"      "$(copies_matching F2 payload libfailed)" 6
exp_note "⭐ D2 is also the harness's control: agreement here would mean the"
exp_note "   copies are not being loaded at all."

printf '\n-- D3: ⛔ C38, THE ARTEFACT EXECD TWICE ----------------------------\n'
exp_note "$(printf 'reference tree   : %s' "$(exp_classify_trace "$WORK/F3" /subj tree | tr '\n' ' ')")"
exp_note "$(printf 'reference payload: %s' "$(exp_classify_trace "$WORK/F3" /subj payload | tr '\n' ' ')")"
for c in $COPIES; do
  exp_note "$(printf '%-26s tree=[%s] payload=[%s]' "$c" \
      "$(sh "$WORK/run.$c" "$WORK/F3" /subj tree    | tr '\n' ' ')" \
      "$(sh "$WORK/run.$c" "$WORK/F3" /subj payload | tr '\n' ' ')")"
done
# ⛔ In TREE mode the reference keeps BOTH objects; the five that delete
# unconditionally keep only the late one -- a DIRTY row turned CLEAN.
exp_check "D3  ⛔ copies agreeing in TREE mode (5 did not)"      "$(agree_count F3 tree)"    1
# ⛔ In PAYLOAD mode the reference keeps only the late one; `60-`, which never
# deletes on the artefact's own execve, keeps both -- a CLEAN row turned DIRTY.
exp_check "D3  ⛔ copies agreeing in PAYLOAD mode (60- did not)" "$(agree_count F3 payload)" 5

printf '\n-- D4/D5: the two defects that were already fixed everywhere -------\n'
exp_check "D4  ld.so.cache is an index: reference counts 0" \
    "$(exp_classify_trace "$WORK/F4" /subj tree | grep -c . || true)" 0
exp_check "D4  ...and no copy counted it either" "$(copies_matching F4 tree 'ld.so.cache')" 0
exp_check "D5  a SPLIT clone is followed (tree)"    "$(agree_count F5 tree)"    6
exp_check "D5  ...and the child is not the payload" "$(agree_count F5 payload)" 6

fi

# ---------------------------------------------------------------------------
# R1: does D3 describe a real trace, or a hazard nothing reaches?
#
# ⛔ THE DIFFERENCE DECIDES WHAT THE RE-RUN OWES. If a real delivery execs the
# artefact path exactly once, the unconditional clear never fired and the
# committed numbers stand as recorded. If it execs twice, `62-` and `90-` --
# the two that call TREE mode -- have committed numbers that are too LOW.
# ---------------------------------------------------------------------------
printf '\n-- R1: ⭐ how many times does a REAL trace exec the artefact? ------\n'
TR="${PGB_EXP102_TRACE:-}"
if [ -z "$TR" ]; then
  # ⛔ THIS USED TO NAME /var/tmp/t065 AND NOTHING ELSE, AND SO IT ALWAYS
  # SKIPPED. `experiments/65-` DELETES each trace the moment it has counted it
  # -- disk is that experiment's binding constraint -- so the one directory R1
  # looked in is empty by construction and R1 could only ever report "no trace
  # on disk". ⭐ A check that cannot fire is not a check.
  # ⚠ Any experiment work directory will do: the artefact name is taken from
  # the trace itself below, not assumed, so a `/subjA` from 107- reads the same
  # as a `/subj65` from 65-. `keep.tr.*` is 107-'s deliberately retained pair.
  TR=$(ls -1t /var/tmp/t*/keep.tr.* /var/tmp/t*/tr.* 2>/dev/null | head -1)
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
    exp_note "⭐ ONE. The unconditional delete had nothing to delete there, so"
    exp_note "   that shape's committed numbers stand -- as a MEASUREMENT."
    exp_note "⛔ It is one delivery shape. 90-'s test script invokes the"
    exp_note "   artefact TWICE, which is the shape that fires."
  else
    exp_skip "R1  ⭐ execve(\"<artefact>\") lines in a real trace" "no /subj* exec in $TR"
  fi
else
  exp_skip "R1  ⭐ execve(\"<artefact>\") lines in a real trace" "no trace on disk"
fi

# ---------------------------------------------------------------------------
# S: ⭐ WHICH COMMITTED NUMBER DOES C38 ACTUALLY REACH?
#
# Two facts decide it per experiment, and both are read out of the experiment:
#
#   1. WHICH MODE it called. The 62-family cleared unconditionally, so it could
#      only differ from the shared classifier in TREE mode; `60-` never cleared
#      there, so it could only differ in PAYLOAD mode.
#   2. HOW MANY TIMES its traced run invokes the artefact. With one invocation
#      there is nothing to clear and both defects are latent.
# ---------------------------------------------------------------------------
printf '\n-- S: ⭐ which committed number does C38 actually reach? ------------\n'

# name : modes it called : invocations in the TRACED run
S_TABLE='60-versus-alternatives:payload+tree:1
62-anylinux-appimage:payload+tree:1
85-opengl:payload:1
86-bundler-vs-anylinux:payload:4
89-debloat:payload:1
90-kdenlive-vs-enhanced:tree:2'

s_fires=''
printf '        %-26s %-14s %-8s %s\n' EXPERIMENT MODE INVOKE 'C38?'
for row in $S_TABLE; do
  f=$(printf '%s' "$row" | cut -d: -f1)
  mds=$(printf '%s' "$row" | cut -d: -f2)
  inv=$(printf '%s' "$row" | cut -d: -f3)
  case "$f" in
    60-*) diffmode=payload ;;
    *)    diffmode=tree ;;
  esac
  fires=no
  case "$mds" in *"$diffmode"*) [ "$inv" -gt 1 ] && fires=YES ;; esac
  [ "$fires" = YES ] && s_fires="$s_fires $f"
  printf '        %-26s %-14s %-8s %s\n' "$f" "$mds" "$inv" "$fires"
done
exp_check "S1  ⛔ experiments C38 actually REACHED" \
    "$(printf '%s' "$s_fires" | wc -w)" 1
exp_note "$(printf '⛔ and it is:%s -- the one T-084 already named' "$s_fires")"
exp_note "⛔ 90- calls TREE mode and its test script invokes the artefact TWICE"
exp_note "   (melt -version, then a real encode), so the unconditional clear"
exp_note "   FIRED and its committed host counts describe only the SECOND"
exp_note "   invocation. ⛔ THAT INCLUDES OUR OWN \"0 of 11\"."
exp_note "⭐ The conversion removes the clear; only re-running 90- says by how"
exp_note "   much the numbers move."

printf '\n-- ⛔ what this does NOT establish ---------------------------------\n'
exp_note "⛔ No committed number moves here. This experiment builds nothing;"
exp_note "   it gives the DIRECTION of every change and the one experiment"
exp_note "   whose numbers can have moved."
exp_note "⛔ R1 measures ONE trace from ONE delivery shape. It bounds D3 for"
exp_note "   that shape and for nothing else."
exp_note "⚠ The fixtures are hand-written strace shapes, not captured output."
exp_note "   They are the shapes C25 and the clone defect were found in."

exp_finish
