#!/bin/sh
# poc/run-all.sh — run the acceptance suite: the ten POCs, in order.
#
# ⛔ WHY THIS EXISTS. `PROGRESS.md` has said "RUN THE TEN POCs" after every
# change to the toolchain's hot path, and there was no command in the tree that
# did it. Each session wrote its own loop, and the loops differed in the three
# ways that decide whether the run means anything:
#
#   1. ⛔ WHICH POCs ARE IN THE SUITE. `poc/92-miniflux` is in the tree and is
#      NOT in the count — `docs/AGENTS.md` §9 says so and T-063 owns it. A loop
#      over `poc/*/run.sh` runs eleven and reports a failure for the one that
#      is known to be incomplete.
#   2. ⛔ WHETHER A CACHED BUILD TREE IS REUSED. Every POC skips its build when
#      the artefact is already on disk, and only FIVE of the ten honour
#      `POC_REBUILD`. ⚠ So a re-run after a TOOLCHAIN change silently reuses
#      binaries the OLD toolchain produced and reports ten green rows that say
#      nothing about the change. `--rebuild` is the answer and it is what makes
#      this an acceptance run rather than a re-print.
#   3. ⚠ WHERE THE OUTPUT GOES. `poc/common.sh` says it plainly: a POC does not
#      write its own `RESULT.txt`, so a run without the redirect leaves
#      `RESULT.txt` describing the PREVIOUS run beside per-environment logs
#      describing this one.
#
# ⛔ THE ENGINE IS NAMED. Every committed `evidence/poc/*/RESULT.txt` reads
# `build env: /var/lib/pgb-rootfs/pgb-env-debian13`, so a docker-engine run is
# not comparable to the baseline. `PGB_ENGINE` is the environment's form of
# `--engine` and exists because `poc_in_env`'s only flag slot is the POC's.
#
# Usage:
#   sh poc/run-all.sh                 reuse cached build trees (a quick re-check)
#   sh poc/run-all.sh --rebuild       ⛔ delete the work tree first. REQUIRED
#                                     after any change to the wrapper's compile
#                                     or link path
#   PGB_ENGINE=docker sh poc/run-all.sh    a different engine, deliberately
#
# Exit: 0 every POC met its expectation, 1 one or more did not, 2 none ran.
#
# SPDX-License-Identifier: MIT
set -u
POC_ROOT=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$POC_ROOT/.." && pwd)
WORK="${PGB_POC_WORK:-/var/tmp/pgb-poc}"
EVIDENCE="${PGB_EVIDENCE_DIR:-$REPO_ROOT/evidence}/poc"

# ⭐ THE SUITE, WRITTEN OUT. Not a glob: see (1) above.
SUITE="10-gawk 20-nano 30-curl 40-jq 50-python 60-leveldb 70-sqlite-extensions 80-mlt 90-qt 91-qt-xcb"

REBUILD=0
for a in "$@"; do
  case "$a" in
    --rebuild) REBUILD=1 ;;
    -h|--help) sed -n '2,40p' "$0"; exit 0 ;;
    *) printf 'poc/run-all.sh: unknown argument: %s\n' "$a" >&2; exit 2 ;;
  esac
done

[ -x "$REPO_ROOT/pgb" ] || {
  printf 'poc/run-all.sh: pgb is a build product; run make\n' >&2; exit 2; }

: "${PGB_ENGINE:=chroot}"
export PGB_ENGINE

STAGE="${PGB_POC_STAGE:-$WORK/.run-all}"
mkdir -p "$STAGE" || exit 2

if [ "$REBUILD" = 1 ]; then
  # ⛔ THE WHOLE TREE, because the POCs share it: `qt6-src` is used by two, and
  # a per-POC delete would leave a source tree the old toolchain configured.
  # ⚠ Nothing here is evidence — every committed result is under evidence/poc.
  printf 'poc/run-all.sh: --rebuild, deleting %s\n' "$WORK"
  find "$WORK" -mindepth 1 -maxdepth 1 ! -name '.run-all' -exec rm -rf {} + 2>/dev/null
  POC_REBUILD=1
  export POC_REBUILD
fi

printf 'poc/run-all.sh: engine=%s rebuild=%s work=%s\n' "$PGB_ENGINE" "$REBUILD" "$WORK"
printf '%-24s %-10s %-8s %s\n' POC RESULT ELAPSED COUNTS

fail=0; ran=0
for p in $SUITE; do
  [ -f "$POC_ROOT/$p/run.sh" ] || { printf '%-24s %-10s\n' "$p" "MISSING"; fail=$((fail+1)); continue; }
  _t0=$(date +%s)
  # ⚠ Staged first, copied into evidence only after the POC exits — an
  # interrupted run cannot leave a truncated RESULT.txt where a complete one was.
  sh "$POC_ROOT/$p/run.sh" > "$STAGE/$p.out" 2>&1
  rc=$?
  _el=$(( $(date +%s) - _t0 ))
  mkdir -p "$EVIDENCE/$p"
  cp "$STAGE/$p.out" "$EVIDENCE/$p/RESULT.txt"
  ran=$((ran+1))
  case $rc in
    0) _r=ok ;;
    1) _r=FAILED; fail=$((fail+1)) ;;
    *) _r="could-not-run($rc)"; fail=$((fail+1)) ;;
  esac
  printf '%-24s %-10s %5ss   %s\n' "$p" "$_r" "$_el" \
    "$(grep -E '^pass=' "$STAGE/$p.out" | tail -1)"
done

printf -- '---------------------------------------------------------------\n'
printf 'ran=%s failed=%s   evidence: %s\n' "$ran" "$fail" "$EVIDENCE"
[ "$ran" = 0 ] && { printf 'VERDICT: nothing ran.\n'; exit 2; }
[ "$fail" = 0 ] || { printf 'VERDICT: %s POC(s) did not meet expectation.\n' "$fail"; exit 1; }
printf 'VERDICT: all %s POCs met expectation.\n' "$ran"
exit 0
