#!/bin/sh
# POC 92 -- miniflux with an EMBEDDED PostgreSQL, against onelf's ~70 MB.
#
# ⛔ WHY THIS EXISTS. The operator, 2026-09-02, naming onelf's own walkthrough:
# *"prove pgb can build something as complex as this"*. The subject is
# `references/QaidVoid__onelf/tree/docs/guide/examples/miniflux.md`, which
# produces one ~70 MB artefact that starts a private postgres on a unix socket,
# initialises the cluster on first run, runs miniflux's migrations, seeds an
# admin user and serves HTTP on 127.0.0.1:8080.
#
# ⭐ WHAT MAKES IT A STRESS TEST IS WHAT IT IS NOT: one program. It is two
# programs plus five postgres helpers, a share tree found through PGSHAREDIR,
# and a set of extensions postgres dlopen()s through a `$libdir` it computes at
# RUN TIME from the build-time relationship between bindir and pkglibdir. Get
# that relationship wrong and initdb dies with
# `could not access file "dict_snowball"`.
#
# ⚠ AND THE HALVES ARE NOT EQUALLY HARD, which is the first thing this POC
# measured rather than assumed. `pgb nix plan miniflux` reports
# buildInputs: [] and nativeBuildInputs: [go, install-shell-files] -- miniflux
# is a pure Go program, so it is already a static ELF with no libc question to
# answer and NONE of pgb's mechanisms apply to it. The whole difficulty is
# PostgreSQL. This POC says so rather than presenting one artefact and implying
# both halves were hard.
#
# TWO ARMS, and the comparison is the deliverable:
#
#   arm S   static. `pgb nix build postgresql` and `miniflux`, one static ELF
#           per program, --wrap-dlopen for the extensions so the dlopen table
#           is compiled in and the $libdir path problem is REMOVED rather than
#           reproduced.
#   arm B   bundle. `pgb bundle appimage` over the same closure, for the case
#           where arm S cannot be reached.
#
# ⛔ ARM S IS ATTEMPTED FIRST AND ITS FAILURE IS A RESULT. If nixpkgs' postgres
# will not build statically, the deliverable is WHICH DEPENDENCY REFUSED, named
# and recorded, not a quiet fall through to arm B.
#
# Exit: 0 all assertions matched, 1 one did not, 2 could not run.
# SPDX-License-Identifier: MIT
. "$(dirname "$0")/../common.sh"

POC_WHY="miniflux plus an embedded PostgreSQL in one artefact, against onelf's ~70 MB"
POC_URL="https://github.com/QaidVoid/onelf/blob/main/docs/guide/examples/miniflux.md"
POC_VERSION="miniflux 2.3.3 (pure Go) + postgresql 18.6, both from nixpkgs"
POC_NORMAL_BUILD="apt install miniflux postgresql && systemctl enable both"
POC_STRESSES="two programs plus five postgres helpers, a dlopen'd extension set reached through a runtime-computed \$libdir, and a share tree"

poc_begin

W="$WORK/92-miniflux"
LOG="$POC_OUT/build.log"
RUNGFILE="$POC_OUT/RUNG-FAILURE.txt"
mkdir -p "$W" || exit 2
: > "$LOG"

# The rung that stopped it, recorded the way poc/90-qt and poc/91-qt-xcb do:
# the error verbatim with the file it came from, not a summary of it.
rung_failed() {  # rung-name summary logfile
  {
    printf 'POC 92-miniflux -- THE RUNG THAT STOPPED IT\n'
    printf '===========================================\n\n'
    printf 'rung      : %s\n' "$1"
    printf 'summary   : %s\n' "$2"
    printf 'date (UTC): %s\n\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    printf -- '-- the error, verbatim, with the file it came from --------------\n\n'
    grep -nE "error:|Error|fatal error|undefined reference|cannot find|No such file" "$3" \
      2>/dev/null | tail -60
    printf '\n-- the last 40 lines of the log --------------------------------\n\n'
    tail -40 "$3" 2>/dev/null
    printf '\nfull log: %s\n' "$3"
  } > "$RUNGFILE"
  printf '\n  ⛔ rung "%s" stopped. Recorded at %s\n' "$1" "$RUNGFILE"
}

# ---------------------------------------------------------------------------
# rung 1: the two plans, and the asymmetry between them
# ---------------------------------------------------------------------------
printf -- '\n-- rung 1: the plans, and which half is actually hard -----------\n'

MF_PLAN="$W/miniflux.plan"
PG_PLAN="$W/postgresql.plan"

"$PGB" nix plan miniflux --out "$MF_PLAN" >>"$LOG" 2>&1
poc_check "a plan for miniflux was produced" \
  "$([ -s "$MF_PLAN" ] && echo ok || echo failed)" ok

"$PGB" nix plan postgresql --out "$PG_PLAN" >>"$LOG" 2>&1
poc_check "a plan for postgresql was produced" \
  "$([ -s "$PG_PLAN" ] && echo ok || echo failed)" ok

[ -s "$MF_PLAN" ] && [ -s "$PG_PLAN" ] || {
  rung_failed "nixpkgs plan" "an attribute did not resolve" "$LOG"; poc_finish; }

# ⭐ MEASURED, NOT ASSERTED: miniflux carries no buildInputs at all, which is
# what makes it a different kind of problem from postgres.
MF_BI=$(sed -n 's/.*"buildInputs":\[\([^]]*\)\].*/\1/p' "$MF_PLAN" | tr -cd ',' | wc -c)
PG_BI=$(sed -n 's/.*"buildInputs":\[\([^]]*\)\].*/\1/p' "$PG_PLAN" | tr -cd ',' | wc -c)
poc_note "miniflux buildInputs: $MF_BI   postgresql buildInputs: $((PG_BI + 1))"
poc_note "miniflux is pure Go -- static by construction, no pgb mechanism applies"

# ---------------------------------------------------------------------------
# rung 2: arm S -- can nixpkgs' postgres be built statically at all
# ---------------------------------------------------------------------------
printf -- '\n-- rung 2: arm S, a static postgresql ---------------------------\n'
# ⚠ THE FLAGS THAT DECIDE THIS ARM, read out of the plan rather than guessed:
# `--with-systemd` asks a static binary to link a library whose whole purpose
# is dlopen, and `internal/nixx`'s dep-skip list refuses systemd for exactly
# that reason -- so the adaptation loop must drop the flag or the arm stops.
# `llvm` is present for the JIT output, which is a dlopen consumer too.
grep -o -- '--with-systemd\|--with-llvm\|--with-icu' "$PG_PLAN" 2>/dev/null | sort -u \
  | while read -r f; do poc_note "plan carries $f"; done

PG_OUT="$W/pg-static"
"$PGB" nix build --plan "$PG_PLAN" --out "$PG_OUT" >>"$LOG" 2>&1
PG_RC=$?
PG_BIN="$PG_OUT/out/bin/postgres"

if [ -x "$PG_BIN" ]; then
  poc_check "arm S: postgresql built" ok ok
  poc_inspect "$PG_BIN"
  ARM_S=ok
else
  # ⛔ NOT A SKIP. The arm ran and did not reach its rung, which is a result,
  # and the dependency that refused is the deliverable.
  poc_check "arm S: postgresql built" "failed(rc=$PG_RC)" ok
  FAILED_DEP=$(grep -oE "dep (build|FAILED) +[a-z0-9_.+-]+" "$LOG" | tail -1)
  rung_failed "arm S: static postgresql" \
    "nixpkgs postgresql did not build static; last dependency: ${FAILED_DEP:-unknown}" "$LOG"
  poc_note "arm S stopped -- arm B is what the comparison then rests on"
  ARM_S=stopped
fi

# ---------------------------------------------------------------------------
# rung 3: arm B -- the bundle
# ---------------------------------------------------------------------------
printf -- '\n-- rung 3: arm B, the bundle over the same closure --------------\n'
BUNDLE="$W/miniflux.AppImage"
if [ ! -s "$BUNDLE" ]; then
  "$PGB" bundle appimage miniflux --with-program postgres --name miniflux \
    --out "$BUNDLE" >>"$LOG" 2>&1 \
    || poc_note "bundle returned non-zero; see $LOG"
fi
poc_check "arm B: a bundle was produced" \
  "$([ -s "$BUNDLE" ] && echo ok || echo failed)" ok

[ -s "$BUNDLE" ] && {
  B_SZ=$(wc -c < "$BUNDLE")
  poc_note "arm B size: $B_SZ bytes, against onelf's stated ~70 MB (73400320)"
}

poc_finish
