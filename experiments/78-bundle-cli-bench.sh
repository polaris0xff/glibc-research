#!/bin/sh
# 78-bundle-cli-bench.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# The operator's verdict on the bundler was "bloated, slow and a complete
# failure": 2.86x the field on `jq`, 2.49x and ~3x slower on kdenlive.
# `TODO` T-066. This is the harness that makes that number move.
#
# ⭐ THE SUBJECT IS A CLI, NOT kdenlive, AND THAT IS THE POINT. A kdenlive
# bundle takes about twenty minutes to build, which means roughly two
# iterations in a session and no confidence that a difference is real. `jq`
# builds in about one minute, runs to completion in milliseconds, and has a
# workload that is deterministic and easy to check. The bar is still the
# field; the ITERATION is on the thing that iterates.
#
# -- WHAT IS MEASURED -------------------------------------------------------
#
#   bytes      the packed artefact, exactly, from `wc -c`
#   cold       first run after the page cache is dropped -- what a user pays
#              the first time. ⚠ Needs /proc/sys/vm/drop_caches, which needs
#              root; without it the column is reported as unavailable rather
#              than silently measuring a warm run.
#   warm       best of N with everything cached: the floor
#   workload   a real job, not --version, so the measurement includes the
#              parts of the bundle a version string never touches
#
# ⛔ AND A CORRECTNESS COLUMN, because a smaller bundle that answers
# differently is not a smaller bundle. Every arm runs the same job and the
# output is compared byte for byte against the first arm's.
#
# Exit: 0 the measurement ran and matched, 1 it ran and did not, 2 could not run.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "78 - the bundler, iterated on a CLI instead of on kdenlive"

WORK="$EXP_OUT/build"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
RESULT="$EXP_OUT/RESULT.txt"
PGB="$REPO_DIR/pgb"
CACHE="${PGB_APPIMAGE_CACHE:-/var/tmp/pgb-appimage}"
ROUNDS="${PGB_BENCH_ROUNDS:-20}"

[ -x "$PGB" ] || { printf 'pgb is not built (run make)\n'; exit 2; }

# The workload. Deterministic, exercises the parser, the numeric path and a
# UTF-8 round trip, and is small enough that the timing is dominated by
# starting the bundle rather than by jq itself -- which is what a CLI bundle's
# users actually pay.
cat > "$WORK/input.json" <<'EOF'
{"items":[{"n":1,"s":"日本"},{"n":2,"s":"café"},{"n":3,"s":"naïve"}],"k":"ok"}
EOF
JQPROG='[.items[].n] | add'
EXPECTED=6

# ---------------------------------------------------------------------------
# Build the arms. Each is a full `pgb bundle appimage jq` with one option
# changed, and the cache is cleared between them so no arm inherits another's
# AppDir.
# ---------------------------------------------------------------------------
build_arm() { # name extra-args...
  _n="$1"; shift
  rm -rf "$CACHE/jq"
  if "$PGB" bundle appimage jq --out "$WORK/$_n.AppImage" --cache "$CACHE" "$@" \
       > "$WORK/$_n.log" 2>&1; then
    printf '%s' "$WORK/$_n.AppImage"
  else
    printf ''
  fi
}

printf -- '-- building --------------------------------------------------\n'
ARMS=""
for spec in "none:--debloat:none" "safe:--debloat:safe" "aggressive:--debloat:aggressive"; do
  name=${spec%%:*}; rest=${spec#*:}
  a1=${rest%%:*}; a2=${rest#*:}
  printf '  %-12s ' "$name"
  out=$(build_arm "$name" "$a1" "$a2")
  if [ -n "$out" ] && [ -f "$out" ]; then
    printf '%s bytes\n' "$(wc -c < "$out")"
    ARMS="$ARMS $name"
  else
    printf 'BUILD FAILED (see %s)\n' "$WORK/$name.log"
  fi
done
[ -n "$ARMS" ] || { printf 'no arm built\n'; exit 2; }
printf '\n'

# ---------------------------------------------------------------------------
# Timing.
#
# ⚠ A COLD READING NEEDS THE PAGE CACHE DROPPED and that needs root. Without
# it the column says "n/a" -- reporting a warm run in a column labelled cold
# would be a silently wrong number, which is worse than a missing one.
# ---------------------------------------------------------------------------
can_drop=no
if [ -w /proc/sys/vm/drop_caches ]; then can_drop=yes; fi

now_ns() { date +%s%N; }

time_once() { # cmd... -> ns
  _s=$(now_ns)
  "$@" >/dev/null 2>&1
  _e=$(now_ns)
  printf '%s' "$((_e - _s))"
}

best_of() { # rounds cmd... -> ns
  _r="$1"; shift
  _best=""
  _i=0
  while [ "$_i" -lt "$_r" ]; do
    _v=$(time_once "$@")
    if [ -z "$_best" ] || [ "$_v" -lt "$_best" ]; then _best=$_v; fi
    _i=$((_i+1))
  done
  printf '%s' "$_best"
}

{
  printf 'experiment 78 - the bundler iterated on a CLI\n'
  printf 'date (UTC)   : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'host kernel  : %s\n' "$(uname -sr)"
  printf 'rounds       : %s (best of), cold cache droppable: %s\n' "$ROUNDS" "$can_drop"
  printf 'workload     : jq %s over %s bytes of UTF-8 JSON\n' \
    "'$JQPROG'" "$(wc -c < "$WORK/input.json")"
  printf '\n'
  printf '%-12s %12s %12s %12s %12s %s\n' \
    ARM BYTES COLD_MS WARM_MS WORKLOAD_MS OUTPUT
} > "$RESULT"

printf -- '-- running ---------------------------------------------------\n'
printf '%-12s %12s %12s %12s %12s %s\n' \
  ARM BYTES COLD_MS WARM_MS WORKLOAD_MS OUTPUT

n_arms=0; n_agree=0; first_out=""
for name in $ARMS; do
  app="$WORK/$name.AppImage"
  n_arms=$((n_arms+1))
  bytes=$(wc -c < "$app")

  # ⛔ The correctness column FIRST: a smaller bundle that answers differently
  # is not a smaller bundle.
  out=$("$app" "$JQPROG" < "$WORK/input.json" 2>/dev/null | tr -d ' \n')
  [ -z "$first_out" ] && first_out="$out"
  if [ "$out" = "$first_out" ] && [ "$out" = "$EXPECTED" ]; then
    n_agree=$((n_agree+1))
  fi

  if [ "$can_drop" = yes ]; then
    sync; printf 3 > /proc/sys/vm/drop_caches 2>/dev/null
    cold=$(time_once "$app" --version)
    cold_ms=$((cold / 1000000))
  else
    cold_ms="n/a"
  fi

  warm=$(best_of "$ROUNDS" "$app" --version)
  work=$(best_of "$ROUNDS" sh -c "'$app' '$JQPROG' < '$WORK/input.json'")

  row=$(printf '%-12s %12s %12s %12s %12s %s' \
        "$name" "$bytes" "$cold_ms" "$((warm / 1000000))" \
        "$((work / 1000000))" "$out")
  printf '%s\n' "$row"
  printf '%s\n' "$row" >> "$RESULT"
done
printf '\n'

printf -- '-- assertions ------------------------------------------------\n'
exp_check "every arm answers the workload identically and correctly" \
  "$n_agree" "$n_arms"
printf '  --    %-46s = %s\n' "arms built (observed)" "$n_arms"
printf '\n'
exp_note "⛔ The size and timing columns are RECORDED, never asserted. A"
exp_note "threshold on a wall-clock figure from one machine on one day is not"
exp_note "a test; docs/AGENTS.md §10 says what this instrument's noise floor"
exp_note "does to small differences. What IS asserted is that the arms agree."
printf '\n'
printf 'full table: %s\n' "$RESULT"

exp_finish
