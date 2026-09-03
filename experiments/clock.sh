#!/bin/sh
# clock.sh -- the wall-clock instrument, and the control that says whether to
#             believe it.
#
# ⛔ THE DEFECT THIS EXISTS TO FIX.  `docs/history/corrections.md` C23.
# `experiments/90-` took ONE SAMPLE per arm.  Four runs of the same comparison,
# the same two artefacts, the same machine, gave cold-start ratios of 2.52x,
# 3.48x, 4.92x and 5.02x -- and in two of the four, warm came out SLOWER than
# cold, which is not a load artefact, it is the instrument's cold/warm
# distinction collapsing.  On 2026-09-03c the operator made those milliseconds
# the entire acceptance bar for the bundler.  ⭐ An unpinned millisecond is
# worth less than no millisecond, because it reads as a measurement.
#
# ⛔ AND THE FIX IS NOT "TAKE MORE SAMPLES".  More samples of a biased design
# buy precision around the wrong number.  Three things are wrong with the
# one-sample design and this file fixes all three:
#
#   1. N=1            -> `clk_samples`, and the estimator is the MEDIAN, not
#                        the mean (one 10x outlier moves a mean of five by 2x
#                        and a median of five not at all) and not best-of
#                        (best-of reports a floor and silently hides spread).
#   2. SEQUENTIAL     -> `clk_interleave`.  Arms measured one after the other
#      ARMS              charge whatever the machine did in between to
#                        whichever arm was running.  Round-robin puts that
#                        drift on every arm instead of on the last one.
#   3. ⭐ NO CONTROL  -> `clk_aa`.  The same artefact, measured under two
#                        names, through the identical protocol.  The true
#                        ratio is 1.00 by construction, so whatever the
#                        instrument reports IS its resolution floor on this
#                        subject and this machine, today.  ⛔ A real arm-vs-arm
#                        ratio inside that band is not a small difference, it
#                        is no difference this instrument can see.
#
# ⚠ Sourced by an experiment AFTER lib.sh.  Everything is POSIX sh plus awk;
# `bc` is not assumed and is absent on some of the eleven.
#
# SPDX-License-Identifier: MIT

# ---------------------------------------------------------------------------
# raw timing
# ---------------------------------------------------------------------------

clk_now_ns() { date +%s%N; }

# clk_time_once CMD... -> nanoseconds on stdout, or -1 if the command failed.
# ⛔ A failed run is NOT timed.  Timing an error path is how `90-` came to
# report onelf "cannot run our payload" for a binary that answers in 0.4 s.
clk_time_once() {
  _clk_s=$(clk_now_ns)
  "$@" >/dev/null 2>&1 || { printf -- '-1'; return; }
  _clk_e=$(clk_now_ns)
  printf '%s' "$(( _clk_e - _clk_s ))"
}

# ---------------------------------------------------------------------------
# statistics over a newline-separated list of nanosecond samples on stdin
#
# ⚠ Every one of these ignores the -1 sentinel, so a partly-failing arm
# reports the samples it has and `clk_n` says how many that was.  An absence
# is not a zero -- docs/AGENTS.md §0b.
# ---------------------------------------------------------------------------

clk_n()      { awk '$1 != -1 && NF' | wc -l | tr -d ' '; }
clk_min()    { awk '$1!=-1&&NF{if(m==""||$1<m)m=$1}END{print (m==""?-1:m)}'; }
clk_max()    { awk '$1!=-1&&NF{if(m==""||$1>m)m=$1}END{print (m==""?-1:m)}'; }
clk_mean()   { awk '$1!=-1&&NF{s+=$1;n++}END{print (n?int(s/n):-1)}'; }

# ⭐ THE ESTIMATOR.  Median of the finite samples; the lower of the two middle
# values on an even count, which is deterministic and does not invent a
# reading that was never taken.
clk_median() {
  awk '$1!=-1&&NF{v[n++]=$1}
       END{ if(!n){print -1; exit}
            for(i=0;i<n;i++)for(j=i+1;j<n;j++)if(v[j]<v[i]){t=v[i];v[i]=v[j];v[j]=t}
            print v[int((n-1)/2)] }'
}

# Median absolute deviation, in the same units.  Reported as a PERCENTAGE of
# the median by clk_row, because that is the number that says whether two arms
# can be told apart.
clk_mad() {
  awk '$1!=-1&&NF{v[n++]=$1}
       END{ if(!n){print -1; exit}
            for(i=0;i<n;i++)for(j=i+1;j<n;j++)if(v[j]<v[i]){t=v[i];v[i]=v[j];v[j]=t}
            m=v[int((n-1)/2)]
            for(i=0;i<n;i++){d=v[i]-m; if(d<0)d=-d; a[i]=d}
            for(i=0;i<n;i++)for(j=i+1;j<n;j++)if(a[j]<a[i]){t=a[i];a[i]=a[j];a[j]=t}
            print a[int((n-1)/2)] }'
}

clk_ms() { # nanoseconds -> milliseconds, one decimal
  awk -v v="$1" 'BEGIN{ if(v<0){print "n/a"; exit} printf "%.1f", v/1000000 }'
}

clk_ratio() { # a b -> "a/b" to two decimals, or n/a
  awk -v a="$1" -v b="$2" 'BEGIN{ if(a<0||b<=0){print "n/a"; exit} printf "%.2f", a/b }'
}

clk_pct() { # part whole -> percent, one decimal
  awk -v a="$1" -v b="$2" 'BEGIN{ if(a<0||b<=0){print "n/a"; exit} printf "%.1f", 100*a/b }'
}

# ---------------------------------------------------------------------------
# the sample store
#
# One file per arm under $CLK_DIR.  Files, not shell variables, because POSIX
# sh has no arrays and because a sample list that survives the run is what
# lets the RESULT file carry the spread instead of just the estimator.
# ---------------------------------------------------------------------------

clk_init() { # dir
  CLK_DIR="$1"
  rm -rf "$CLK_DIR"; mkdir -p "$CLK_DIR" || return 2
  CLK_ARMS=""
  return 0
}

clk_arm_file() { printf '%s/samples.%s' "$CLK_DIR" "$1"; }

clk_add() { # tag ns
  printf '%s\n' "$2" >> "$(clk_arm_file "$1")"
}

# ---------------------------------------------------------------------------
# ⭐ THE INTERLEAVED PROTOCOL
#
# clk_interleave ROUNDS TAG...   with two caller-supplied hooks:
#
#   clk_prep  TAG   optional, NOT timed.  Whatever makes the next run of this
#                   arm mean what the column says -- drop the page cache, lay
#                   down a fresh copy so a FUSE mount is cold, empty a cache
#                   directory.  ⛔ It runs before EVERY sample, so "cold"
#                   stays cold on sample 5 as much as on sample 1.
#   clk_run   TAG   required.  Runs the arm exactly once.  Its wall time is
#                   the sample; a non-zero exit discards that sample.
#
# ⛔ THE ORDER ROTATES BY ROUND.  Round 1 runs the arms left to right, round 2
# starts from the second arm, and so on.  Interleaving alone still gives the
# first arm of every round a systematically different neighbourhood (it is the
# one that follows the inter-round gap); rotating removes that too.
# ---------------------------------------------------------------------------

clk_interleave() { # rounds tag...
  _clk_rounds="$1"; shift
  CLK_ARMS="$*"
  _clk_ntags=0
  for _t in $CLK_ARMS; do _clk_ntags=$((_clk_ntags + 1)); : "$_t"; done
  [ "$_clk_ntags" -gt 0 ] || return 2

  _clk_r=0
  while [ "$_clk_r" -lt "$_clk_rounds" ]; do
    # rotate the starting arm by the round number
    _clk_off=$(( _clk_r % _clk_ntags ))
    _clk_i=0
    _clk_order=""
    for _t in $CLK_ARMS; do
      if [ "$_clk_i" -ge "$_clk_off" ]; then _clk_order="$_clk_order $_t"; fi
      _clk_i=$((_clk_i + 1))
    done
    _clk_i=0
    for _t in $CLK_ARMS; do
      if [ "$_clk_i" -lt "$_clk_off" ]; then _clk_order="$_clk_order $_t"; fi
      _clk_i=$((_clk_i + 1))
    done

    for _t in $_clk_order; do
      if command -v clk_prep >/dev/null 2>&1; then clk_prep "$_t"; fi
      _clk_v=$(clk_run "$_t")
      clk_add "$_t" "$_clk_v"
    done
    _clk_r=$((_clk_r + 1))
  done
  return 0
}

# ---------------------------------------------------------------------------
# reporting
# ---------------------------------------------------------------------------

# clk_stat TAG WHAT -> one number in ns (median|min|max|mad|mean|n)
clk_stat() {
  _clk_f=$(clk_arm_file "$1")
  [ -f "$_clk_f" ] || { printf -- '-1'; return; }
  case "$2" in
    median) clk_median < "$_clk_f" ;;
    min)    clk_min    < "$_clk_f" ;;
    max)    clk_max    < "$_clk_f" ;;
    mad)    clk_mad    < "$_clk_f" ;;
    mean)   clk_mean   < "$_clk_f" ;;
    n)      clk_n      < "$_clk_f" ;;
    *)      printf -- '-1' ;;
  esac
}

clk_header() {
  printf '  %-14s %5s %10s %10s %10s %8s\n' ARM N 'MEDIAN' 'MIN' 'MAX' 'MAD%'
}

# ⛔ THE SPREAD IS PRINTED BESIDE THE ESTIMATOR, ALWAYS.  A median with no MAD
# beside it is the shape of number this instrument was built to stop producing.
clk_row() { # tag
  _clk_med=$(clk_stat "$1" median)
  printf '  %-14s %5s %10s %10s %10s %8s\n' \
    "$1" "$(clk_stat "$1" n)" \
    "$(clk_ms "$_clk_med")" "$(clk_ms "$(clk_stat "$1" min)")" \
    "$(clk_ms "$(clk_stat "$1" max)")" \
    "$(clk_pct "$(clk_stat "$1" mad)" "$_clk_med")"
}

clk_table() { clk_header; for _t in $CLK_ARMS; do clk_row "$_t"; done; }

# ⭐ THE RAW SAMPLES ARE EVIDENCE AND MUST SURVIVE THE RUN.
# `docs/history/corrections.md` C23 could only be written because somebody
# went back through git history for superseded versions of a RESULT file. The
# samples behind an estimator are what makes a median re-derivable rather than
# quotable, so they are copied out of the scratch directory -- which is
# gitignored, correctly, because it also holds a 200 MB AppImage -- and into
# the committed evidence beside RESULT.txt.
clk_save() { # dest-dir  (one file per arm: samples.<tag>.txt, ns per line)
  mkdir -p "$1" || return 1
  for _t in $CLK_ARMS; do
    _f=$(clk_arm_file "$_t")
    [ -f "$_f" ] || continue
    {
      printf '# %s -- one wall-clock sample per line, nanoseconds, in the\n' "$_t"
      printf '# order taken. -1 means the run failed and was not timed.\n'
      cat "$_f"
    } > "$1/samples.$_t.txt"
  done
}

# ---------------------------------------------------------------------------
# ⭐ THE A/A CONTROL -- the reason to believe any row above
#
# clk_aa TAG_A TAG_B  ->  echoes the ratio of two arms that are the SAME
# artefact under two names.  The true value is 1.00.  What comes back is the
# floor this instrument can resolve on this subject, on this machine, today.
#
# ⛔ It is not a constant and must not be carried between runs.  It is
# measured in the same run as the arms it licenses, through the same
# `clk_interleave`, or it says nothing about them.
# ---------------------------------------------------------------------------

clk_aa() { # tag_a tag_b -> ratio as a decimal string
  _clk_a=$(clk_stat "$1" median); _clk_b=$(clk_stat "$2" median)
  if [ "$_clk_a" -lt 0 ] 2>/dev/null || [ "$_clk_b" -lt 0 ] 2>/dev/null; then
    printf 'n/a'; return
  fi
  # report it the way a reader checks it: always >= 1, so the direction of the
  # accident does not change how big the floor looks
  awk -v a="$_clk_a" -v b="$_clk_b" \
    'BEGIN{ r = (a>b) ? a/b : b/a; printf "%.2f", r }'
}

# clk_floor TAG_A TAG_B [AA_RATIO] -> the smallest ratio worth believing when
# comparing these two arms.
#
# ⛔ THE A/A RATIO ALONE IS NOT ENOUGH, and the smoke test that built this
# said so: two runs of /bin/true came out 1.01 apart while each arm's own MAD
# was 4-7% of its median.  A single pair of medians is one draw from a noisy
# quantity; believing a 1.07 difference because one A/A draw happened to land
# at 1.01 is the same N=1 mistake one level up.
#
# ⭐ So the floor is the LARGER of two independent estimates:
#   - the empirical A/A ratio, which catches SYSTEMATIC bias between two
#     positions in the interleave, and
#   - 1 + MAD_a/median_a + MAD_b/median_b, the combined dispersion of the two
#     arms actually being compared, which catches plain SPREAD.
# Neither subsumes the other: an A/A pair can be accidentally tight, and two
# tight arms can still sit either side of a drift the A/A never saw.
clk_floor() { # tag_a tag_b [aa_ratio]
  _clk_ma=$(clk_stat "$1" median); _clk_mb=$(clk_stat "$2" median)
  _clk_da=$(clk_stat "$1" mad);    _clk_db=$(clk_stat "$2" mad)
  awk -v ma="$_clk_ma" -v mb="$_clk_mb" -v da="$_clk_da" -v db="$_clk_db" \
      -v aa="${3:-1.00}" 'BEGIN{
    if (ma<=0 || mb<=0) { print "n/a"; exit }
    spread = 1 + da/ma + db/mb
    if (aa == "n/a") aa = 1.0
    printf "%.2f", (spread > aa+0) ? spread : aa+0 }'
}

# clk_resolves RATIO FLOOR -> yes|no|unknown.  Both are "x-fold" numbers >= 1.
# ⛔ THE DECISION RULE, WRITTEN DOWN ONCE.  A measured ratio counts as a real
# difference only when it is further from 1 than the floor is.  ⚠ "no" does
# NOT mean the arms are equal -- it means this instrument, at this N, cannot
# tell them apart, which is the sentence docs/AGENTS.md §10 already requires
# for the overhead table and which the bundler's record has never carried.
clk_resolves() { # ratio floor
  awk -v r="$1" -v f="$2" 'BEGIN{
    if (r == "n/a" || f == "n/a") { print "unknown"; exit }
    rr = (r < 1) ? 1/r : r
    print (rr > f) ? "yes" : "no" }'
}
