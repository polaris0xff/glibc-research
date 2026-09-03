#!/bin/sh
# 95 - T-066 route B, costed: how much of a closure does a `-mini` rebuild
# force from source?  Subject: $PGB_EXP95_ATTR, default kdenlive.
#
# ⛔ WHY THIS RUNS BEFORE THE ALLOWLIST. T-066 measured route A's ceiling on
# 2026-09-03 -- the sweep can prove 6.3% of a bundle's library tree dead, while
# two `-mini` rebuild edges are worth 23.3% -- and the gap to the field on
# kdenlive is 2.22x. An allowlist chooses which PATHS to carry; it cannot remove
# a dependency a library DECLARES, and only a rebuild does that. So the entry's
# work order was reordered to cost route B first, and this is that measurement.
#
# ⭐ THE QUESTION, in T-066's own words: *"how many store paths in kdenlive's
# closure are downstream of qtbase and mesa? That number is what a `-mini`
# derivation forces from source, and it is the whole argument against route B."*
#
# WHY THE NUMBER IS WHAT IT IS. A nixpkgs store path is the hash of its inputs.
# Changing qtbase's build options changes qtbase's hash, which changes the hash
# of everything built against it -- so every path DOWNSTREAM of qtbase leaves
# the binary cache and has to be built from source. That is the cost Arch does
# not pay: `qt6-base-mini` keeps the soname `libQt6Core.so.6`, so nothing
# downstream rebuilds at all. TODO T-066, "why their swap is cheap and ours is
# not".
#
# ⛔ WHAT THIS MEASURES, EXACTLY, AND WHAT IT DOES NOT.
# The graph here is the RUNTIME REFERENCE graph, read from each path's narinfo
# `References:` field. It is NOT the build-input graph: a package can build
# against qtbase without referencing it at run time, and nixpkgs propagates a
# rebuild along build inputs. Runtime references are a SUBSET of build inputs,
# so every number below is a LOWER BOUND on the rebuild set. ⭐ It is a lower
# bound on the right population, though: the bundle carries exactly this
# closure, so "how many of the paths I fetch would have to be built instead" is
# bounded below by what this counts.
#
# ⚠ NO REBUILD AND NO AppDir. The entry called this measurement cheap and this
# time it is: one hydra job, one closure walk, and the narinfos the closure
# walk already cached.
#
#   0  the graph was built, the controls held, the numbers are in the table
#   1  a control did not hold -- the graph is not oriented the way it must be
#   2  the closure could not be resolved or fetched here
#
# SPDX-License-Identifier: MIT
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "95 - T-066 route B costed: a closure's paths downstream of the -mini seeds"

PGB="$REPO_DIR/pgb"
[ -x "$PGB" ] || { exp_note "pgb is a build product: run make"; exit 2; }

# ⭐ THE SUBJECT IS A PARAMETER, because route A's ceiling and route B's cost
# were first measured on DIFFERENT closures — the ceiling on `mesa-demos`, the
# cost on `kdenlive` — and T-066 says plainly that two numbers from two
# closures must not be subtracted from one another. One subject, both routes,
# is the comparison worth having.
#
#   PGB_EXP95_ATTR=mesa-demos sh experiments/95-route-b-cost.sh
#
# ⚠ The RESULT file is named after the subject, the way `experiments/86-` names
# `RESULT.jq.txt` and `RESULT.mpv.txt`: one file per subject, so a second
# subject does not overwrite the first.
SUBJECT="${PGB_EXP95_ATTR:-kdenlive}"
WORK="${PGB_EXP95_WORK:-/var/tmp/pgb-exp95}/$SUBJECT"
mkdir -p "$WORK" || exit 2
exp_note "subject              $SUBJECT"

# ⛔ THE SUBJECT IS RESOLVED THE WAY THE BUNDLER RESOLVES IT, not by picking a
# name out of `nix cache resolve`. `internal/bundle/appimage.go:resolveTarget`
# asks hydra for the attribute's latest finished build and takes its `out`
# output; `pgb nix cache resolve kdenlive` lists TEN store paths for this
# package across outputs and evaluations, and choosing among them by hand would
# measure a closure the bundler never builds.
ATTR=$("$PGB" nix cache attr "$SUBJECT" 2>/dev/null | sed -n 's/^Attr: *//p')
[ -n "$ATTR" ] || { exp_note "pgb nix cache attr $SUBJECT answered nothing"; exit 2; }
exp_note "attribute            $ATTR"

HYDRA="$WORK/hydra.json"
if [ ! -s "$HYDRA" ]; then
  curl -sSfL -H 'Accept: application/json' --max-time 180 \
    "https://hydra.nixos.org/job/nixpkgs/trunk/$ATTR.x86_64-linux/latest-finished" \
    -o "$HYDRA" || { exp_note "hydra is not reachable from here"; exit 2; }
fi
# ⭐ Parsed by pgb, not by a hand-rolled JSON reader: `pgb nix hydra FILE` is the
# same code path `HydraJob` feeds. docs/AGENTS.md §14 -- do not write a second
# fetcher or a second parser.
OUTPATH=$("$PGB" nix hydra "$HYDRA" 2>/dev/null | sed -n 's/^Out\.out: *//p')
case "$OUTPATH" in
  /nix/store/*) ;;
  *) exp_note "hydra gave no out path for $ATTR"; exit 2 ;;
esac
exp_note "store path           $OUTPATH"

CLOSURE="$WORK/closure.txt"
if [ ! -s "$CLOSURE" ]; then
  "$PGB" nix cache closure "$OUTPATH" > "$CLOSURE.part" 2>"$WORK/closure.err" \
    || { exp_note "closure walk failed: $(tail -1 "$WORK/closure.err")"; exit 2; }
  mv "$CLOSURE.part" "$CLOSURE"
fi
NPATHS=$(grep -c . "$CLOSURE")
[ "$NPATHS" -gt 1 ] || { exp_note "closure came back with $NPATHS paths"; exit 2; }

# The narinfo cache the closure walk just filled. Each file carries the
# `References:` line this graph is made of and the `NarSize:` the bytes column
# comes from.
NARINFO="${PGB_NIX_CACHE:-/var/tmp/pgb-nix-cache}/narinfo"
[ -d "$NARINFO" ] || { exp_note "no narinfo cache at $NARINFO"; exit 2; }

EDGES="$WORK/edges.tsv"; SIZES="$WORK/sizes.tsv"; MISSING="$WORK/missing.txt"
: > "$EDGES"; : > "$SIZES"; : > "$MISSING"
while read -r p; do
  [ -n "$p" ] || continue
  b=${p#/nix/store/}; h=${b%%-*}
  f="$NARINFO/$h.narinfo"
  # ⛔ A MISSING NARINFO IS COUNTED, NOT SKIPPED. A path whose references
  # nothing read is a path with no outgoing edges, which makes everything under
  # it look unreachable and every downstream count too small -- silently.
  [ -f "$f" ] || { printf '%s\n' "$b" >> "$MISSING"; continue; }
  printf '%s\t%s\n' "$b" "$(sed -n 's/^NarSize: *//p' "$f")" >> "$SIZES"
  sed -n 's/^References: *//p' "$f" | tr ' ' '\n' | while read -r r; do
    [ -n "$r" ] && printf '%s\t%s\n' "$b" "$r" >> "$EDGES"
  done
done < "$CLOSURE"

# ⚠ `wc -l`, not `grep -c .`: grep exits 1 on an empty file, so
# `$(grep -c . f || echo 0)` prints BOTH its own 0 and the fallback 0, and the
# comparison then reads "0\n0" against "0" and fails on a clean run.
NMISSING=$(wc -l < "$MISSING" | tr -d ' ')
exp_check "every closure path had a narinfo" "$NMISSING" 0
exp_note "closure paths        $NPATHS"
exp_note "reference edges      $(grep -c . "$EDGES")"

# ---------------------------------------------------------------------------
# Reverse reachability. `downstream(S)` is the set of paths that transitively
# reference something in S, S included -- what a rebuild of S invalidates.
# ---------------------------------------------------------------------------
cat > "$WORK/downstream.awk" <<'AWK'
BEGIN { FS = "\t"; n = split(seeds, want, ",") }
FILENAME ~ /edges/ { rev[$2] = rev[$2] " " $1; next }
FILENAME ~ /sizes/ { size[$1] = $2; all[$1] = 1; next }
END {
  for (p in all)
    for (i = 1; i <= n; i++)
      if (want[i] != "" && index(p, want[i])) { seed[p] = 1; nseed++ }
  for (p in seed) { queue[++tail] = p; seen[p] = 1 }
  while (head < tail) {
    cur = queue[++head]
    m = split(rev[cur], ups, " ")
    for (i = 1; i <= m; i++)
      if (ups[i] != "" && !seen[ups[i]]) { seen[ups[i]] = 1; queue[++tail] = ups[i] }
  }
  for (p in seen) { count++; bytes += size[p] }
  for (p in all)  { total++; totalbytes += size[p] }
  printf "%d %d %d %d %d\n", nseed, count, bytes, total, totalbytes
  if (list == "1") for (p in seen) print "PATH " p > listfile
}
AWK

downstream() { # seeds [listfile] -> "nseed ndown bytes ntotal totalbytes"
  if [ -n "${2:-}" ]; then
    awk -v seeds="$1" -v list=1 -v listfile="$2" -f "$WORK/downstream.awk" "$EDGES" "$SIZES"
  else
    awk -v seeds="$1" -f "$WORK/downstream.awk" "$EDGES" "$SIZES"
  fi
}

# ---------------------------------------------------------------------------
# ⛔ THE CONTROLS COME FIRST, because reverse reachability computed the wrong
# way round produces a perfectly plausible table. These two say which way the
# edges point, and they would SWAP if the graph were inverted.
# ---------------------------------------------------------------------------
# ⛔ THE TOP OF THE CLOSURE IS THE STORE PATH'S OWN NAME, not the subject
# string. ⚠ Measured the hard way: this line seeded on the literal "kdenlive"
# and the control FAILED the first time the experiment was run against another
# subject -- 0 seeds, 0 downstream, "expected 1". That is the control catching
# an incomplete parameterisation, which is what it is for.
TOPNAME=${OUTPATH#/nix/store/}; TOPNAME=${TOPNAME#*-}
set -- $(downstream "$TOPNAME")
TOP_DOWN=$2; TOTAL=$4; TOTAL_BYTES=$5
exp_check "control: nothing is downstream of the top but itself" "$TOP_DOWN" 1

set -- $(downstream "-glibc-")
GLIBC_DOWN=$2
# ⚠ Not an exact number: nixpkgs moves and this must not go red for that. The
# structural claim is that libc is under nearly everything, and "more than half
# the closure" is the weakest statement that still fails on an inverted graph.
if [ "$GLIBC_DOWN" -gt $((TOTAL / 2)) ]; then
  exp_check "control: most of the closure is downstream of glibc" over-half over-half
else
  exp_check "control: most of the closure is downstream of glibc" \
    "$GLIBC_DOWN of $TOTAL" "over half"
fi

printf '\n  what a `-mini` rebuild of each seed forces from source:\n'
printf '    %-30s %6s %8s %16s %8s\n' SEED SEEDS PATHS BYTES 'OF CLOSURE'
report() { # label seeds
  # ⛔ THE LABEL IS SAVED FIRST. `set -- $(...)` replaces the positional
  # parameters, so reading "$1" after it gives the first FIELD OF THE OUTPUT,
  # not the label -- and the first field is the seed count, so every row of
  # this table printed a number where its own name should be.
  _lab="$1"
  set -- $(downstream "$2")
  _ns=$1; _nd=$2; _nb=$3
  if [ "$_ns" = 0 ] || [ -z "$_ns" ]; then
    # ⛔ AN ABSENCE IS NOT A ZERO. A seed matching no path in this closure means
    # the package is not here, which is not the same as costing nothing --
    # and 0.0% in a cost table reads exactly like "free".
    printf '    %-30s %6s %8s %16s %8s\n' "$_lab" 0 '-' '-' 'NOT IN CLOSURE'
    return
  fi
  printf '    %-30s %6s %8s %16s %7.1f%%\n' "$_lab" "$_ns" "$_nd" "$_nb" \
    "$(awk -v d="$_nd" -v t="$TOTAL" 'BEGIN{print 100*d/t}')"
}
report qtbase                qtbase
report "mesa"                mesa
report "qtbase + mesa"       "qtbase,mesa"
report icu                   icu
report libxml2               libxml2
report opus                  opus
report gtk3                  gtk3
report gtk4                  gtk4
report glycin                glycin
report "the whole -mini set" "qtbase,mesa,icu,libxml2,opus,gtk3,gtk4,glycin"

# ⭐ The headline pair, kept as named variables so the write-up quotes the run.
set -- $(downstream "qtbase")             ; QT_N=$2;   QT_B=$3
set -- $(downstream "mesa")               ; MESA_N=$2; MESA_B=$3
set -- $(downstream "qtbase,mesa")        ; BOTH_N=$2; BOTH_B=$3
set -- $(downstream "qtbase,mesa,icu,libxml2,opus,gtk3,gtk4,glycin") ; ALL_N=$2; ALL_B=$3

printf '\n  the answer T-066 asked for:\n'
exp_note "closure                       $TOTAL paths, $TOTAL_BYTES B"
exp_note "downstream of qtbase          $QT_N paths, $QT_B B"
exp_note "downstream of mesa            $MESA_N paths, $MESA_B B"
exp_note "downstream of both            $BOTH_N paths, $BOTH_B B"
exp_note "downstream of the -mini set   $ALL_N paths, $ALL_B B"

# ⭐ A STRUCTURAL OBSERVATION, MEASURED RATHER THAN ASSERTED: if every path
# downstream of qtbase is also downstream of mesa, then cutting qtbase buys no
# rebuild that cutting mesa does not already force, and the two recipes cost
# almost the same together as mesa does alone.
# ⚠ A seed that matches nothing writes no list file, so the comparison is only
# meaningful when both seeds are present. Saying "n/a" is not the same as
# saying 0, and a closure without qtbase must not report "qtbase costs nothing
# mesa does not already cost".
: > "$WORK/ds-qtbase.txt"; : > "$WORK/ds-mesa.txt"
downstream "qtbase" "$WORK/ds-qtbase.txt" >/dev/null
downstream "mesa"   "$WORK/ds-mesa.txt"   >/dev/null
sort "$WORK/ds-qtbase.txt" -o "$WORK/ds-qtbase.txt"
sort "$WORK/ds-mesa.txt"   -o "$WORK/ds-mesa.txt"
if [ -s "$WORK/ds-qtbase.txt" ] && [ -s "$WORK/ds-mesa.txt" ]; then
  QT_ONLY=$(comm -23 "$WORK/ds-qtbase.txt" "$WORK/ds-mesa.txt" | grep -c . || true)
else
  QT_ONLY="n/a (one of the two seeds is not in this closure)"
fi
exp_note "downstream of qtbase but NOT of mesa: $QT_ONLY"

pct() { awk -v d="$1" -v t="$TOTAL" 'BEGIN{printf "%.1f", 100*d/t}'; }

RESULT="$EXP_OUT/RESULT.$SUBJECT.txt"
{
  printf '95 - T-066 route B costed: %s downstream of the -mini seeds\\n\\n' "$SUBJECT"
  printf 'date: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'kernel: %s\n' "$(uname -sr)"
  printf 'attribute: %s\n' "$ATTR"
  printf 'store path: %s\n' "$OUTPATH"
  printf '\n'
  printf 'THE QUESTION (T-066): how many store paths in this closure are\n'
  printf 'downstream of qtbase and mesa? A nixpkgs store path is the hash of\n'
  printf 'its inputs, so a -mini rebuild of a seed invalidates every path above\n'
  printf 'it and those must be built from source.\n\n'
  printf 'closure: %s paths, %s B\n\n' "$TOTAL" "$TOTAL_BYTES"
  printf '%-30s %8s %16s %10s\n' SEED PATHS BYTES 'OF CLOSURE'
  printf '%-30s %8s %16s %9s%%\n' 'qtbase'              "$QT_N"   "$QT_B"   "$(pct "$QT_N")"
  printf '%-30s %8s %16s %9s%%\n' 'mesa'                "$MESA_N" "$MESA_B" "$(pct "$MESA_N")"
  printf '%-30s %8s %16s %9s%%\n' 'qtbase + mesa'       "$BOTH_N" "$BOTH_B" "$(pct "$BOTH_N")"
  printf '%-30s %8s %16s %9s%%\n' 'the whole -mini set' "$ALL_N"  "$ALL_B"  "$(pct "$ALL_N")"
  printf '\ncontrols (they SWAP on an inverted graph):\n'
  printf '  downstream of %s itself   %s   (expected 1)\n' "$TOPNAME" "$TOP_DOWN"
  printf '  downstream of glibc             %s of %s\n' "$GLIBC_DOWN" "$TOTAL"
  printf '\ndownstream of qtbase but NOT of mesa: %s\n' "$QT_ONLY"
  printf '\nLOWER BOUND, and it is stated because the graph is the RUNTIME\n'
  printf 'reference graph read from narinfo References. nixpkgs propagates a\n'
  printf 'rebuild along BUILD inputs, of which runtime references are a subset.\n'
  printf 'Every count above is therefore a floor on the rebuild set.\n'
  printf '\nreproduce:\n'
  printf '  PGB_EXP95_ATTR=%s sh experiments/95-route-b-cost.sh\\n' "$SUBJECT"
} > "$RESULT"
printf '\nwrote %s\n' "$RESULT"

exp_finish
