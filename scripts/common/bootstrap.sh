#!/bin/sh
# bootstrap.sh - get a FRESH machine ready, in parallel, in one command.
#
# ⛔ THE DEFECT THIS EXISTS TO CATCH, AND IT HAS COST TWO SESSIONS.
#
# Nothing this project needs survives the container. Every session starts with
# 0 of 11 rootfs, no build environment, no static libiconv and no nix, and the
# first hour goes on setup. ⚠ Both previous sessions did it SERIALLY, and
# worse, INTERACTIVELY -- each step waited for the last while the agent
# watched. Measured here on 2026-09-01c, wall clock:
#
#     nix install (pkgforge install_nix.sh)   ~7 min
#     pgb env create (chroot + libiconv)      ~8 min
#     fetch-rootfs.sh (11 environments)      ~10 min
#     ---------------------------------------------
#     serial                                 ~25 min
#
# ⭐ They are independent and mostly network-bound, so they run TOGETHER here,
# and the caller reads the documentation while they do. The wall clock becomes
# roughly the longest one.
#
# ⭐ AND IT IS RESUMABLE. Each step is skipped when its artefact is already
# there, so re-running after a failure costs only what actually failed.
#
# Usage:
#   sh scripts/common/bootstrap.sh              # everything, in parallel
#   sh scripts/common/bootstrap.sh --check      # what is present, exit 1 if not ready
#   sh scripts/common/bootstrap.sh --no-nix     # skip nix (planning falls back)
#   sh scripts/common/bootstrap.sh --no-bed     # skip the 11 rootfs (~2.3 GiB)
#   sh scripts/common/bootstrap.sh --wait       # run in parallel and WAIT (default)
#   sh scripts/common/bootstrap.sh --detach     # start and return; --check later
#
# Exit: 0 ready, 1 something failed, 2 could not run.

set -u

SELF=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO=$(dirname "$(dirname "$SELF")")
LOGDIR="${PGB_BOOTSTRAP_LOGS:-/var/tmp/pgb-bootstrap}"
ROOTFS_DIR="${PGB_ROOTFS_DIR:-/var/lib/pgb-rootfs}"
ENV_NAME="${PGB_ENV_NAME:-pgb-env-debian12}"
DO_NIX=1; DO_BED=1; DO_ENV=1; MODE=wait; CHECK=0

while [ $# -gt 0 ]; do
  case "$1" in
    --check)   CHECK=1 ;;
    --no-nix)  DO_NIX=0 ;;
    --no-bed)  DO_BED=0 ;;
    --no-env)  DO_ENV=0 ;;
    --wait)    MODE=wait ;;
    --detach)  MODE=detach ;;
    -h|--help) awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"; exit 0 ;;
    *)         printf 'bootstrap: unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
  shift
done

say() { printf '%s\n' "$*"; }

# ---------------------------------------------------------------------------
# What "ready" means, and each answer is a FILE ON DISK rather than a flag this
# script wrote. ⛔ A bootstrap that trusts its own marker reports success after
# a step that half-ran.
# ---------------------------------------------------------------------------
have_nix() {
  for c in nix /nix/var/nix/profiles/default/bin/nix "$HOME/.nix-profile/bin/nix"; do
    command -v "$c" >/dev/null 2>&1 && return 0
    [ -x "$c" ] && return 0
  done
  return 1
}
have_env() { [ -s "$ROOTFS_DIR/$ENV_NAME/.pgb-env-stamp" ]; }
bed_count() {
  _n=0
  while read -r ref name libc digest; do
    case "$ref" in ''|\#*) continue ;; esac
    [ -d "$ROOTFS_DIR/$name" ] && _n=$((_n + 1))
  done < "$SELF/rootfs-images.txt"
  printf '%s' "$_n"
}
bed_total() { grep -cvE '^\s*(#|$)' "$SELF/rootfs-images.txt"; }

report() {
  _b=$(bed_count); _t=$(bed_total)
  printf '  %-22s %s\n' "nix"            "$(have_nix && echo present || echo ABSENT)"
  printf '  %-22s %s\n' "build environment" "$(have_env && echo present || echo ABSENT)"
  printf '  %-22s %s\n' "test bed"       "$_b of $_t rootfs"
  [ -d /var/tmp/pgb-nix-cache ] &&
    printf '  %-22s %s\n' "nix fetch cache" "$(du -sh /var/tmp/pgb-nix-cache 2>/dev/null | cut -f1)"
  # ⚠ READY DOES NOT REQUIRE NIX. `pgb nix plan` has a nix-free route for a
  # package whose .drv is cached (47% of named packages, experiments/83-), and
  # a committed plan needs none at all. Saying "not ready" without nix would
  # be wrong and would send a session installing something it may not need.
  have_env && [ "$_b" = "$_t" ]
}

if [ "$CHECK" = 1 ]; then
  say "pgb bootstrap: what this machine has"
  if report; then say "  VERDICT: ready to build and verify."; exit 0
  else say "  VERDICT: NOT ready. Run: sh scripts/common/bootstrap.sh"; exit 1; fi
fi

mkdir -p "$LOGDIR" || { printf 'bootstrap: cannot write %s\n' "$LOGDIR" >&2; exit 2; }

# ---------------------------------------------------------------------------
# The three steps, started together.
#
# ⚠ nix's installer is a THIRD PARTY SCRIPT RUN AS ROOT and it is here only
# because the operator authorised it by name, with the URL, in the session of
# 2026-09-01c. ⛔ Do not add another installer to this file on your own
# authority.
# ---------------------------------------------------------------------------
PIDS=""; NAMES=""

start() {   # name command...
  _n="$1"; shift
  say "  starting $_n  (log: $LOGDIR/$_n.log)"
  ( "$@" >"$LOGDIR/$_n.log" 2>&1; printf '%s' "$?" > "$LOGDIR/$_n.rc" ) &
  PIDS="$PIDS $!"; NAMES="$NAMES $_n"
}

install_nix() {
  d=$(mktemp -d) || return 2
  curl -qfsSL "https://raw.githubusercontent.com/pkgforge/devscripts/refs/heads/main/Linux/install_nix.sh" \
    -o "$d/install_nix.sh" || return 1
  chmod +x "$d/install_nix.sh"
  bash "$d/install_nix.sh"
  _rc=$?
  rm -rf "$d"
  return $_rc
}

say "pgb bootstrap: preparing a fresh machine"
say ""
if [ "$DO_NIX" = 1 ] && ! have_nix; then start nix install_nix
elif [ "$DO_NIX" = 1 ]; then say "  nix already present, skipping"
fi
if [ "$DO_ENV" = 1 ] && ! have_env; then start env sh "$REPO/pgb" env create
elif [ "$DO_ENV" = 1 ]; then say "  build environment already present, skipping"
fi
if [ "$DO_BED" = 1 ] && [ "$(bed_count)" != "$(bed_total)" ]; then
  start bed sh "$SELF/fetch-rootfs.sh"
elif [ "$DO_BED" = 1 ]; then say "  test bed already complete, skipping"
fi

if [ -z "$PIDS" ]; then
  say ""
  say "nothing to do -- this machine is already set up."
  report >/dev/null; exit 0
fi

if [ "$MODE" = detach ]; then
  say ""
  say "started in the background. Read docs/AGENTS.md and TODO/PROGRESS.md,"
  say "then: sh scripts/common/bootstrap.sh --check"
  exit 0
fi

say ""
say "  running in parallel. ⭐ Read docs/AGENTS.md while this happens --"
say "  that is the point of doing it this way."
say ""
rc=0
for p in $PIDS; do wait "$p" || rc=1; done
for n in $NAMES; do
  _rc=$(cat "$LOGDIR/$n.rc" 2>/dev/null || echo '?')
  if [ "$_rc" = 0 ]; then
    printf '  %-22s ok\n' "$n"
  else
    printf '  %-22s FAILED (rc=%s) -- last lines:\n' "$n" "$_rc"
    tail -8 "$LOGDIR/$n.log" 2>/dev/null | sed 's/^/      /'
    rc=1
  fi
done

say ""
if report; then
  say "  VERDICT: ready to build and verify."
else
  say "  VERDICT: NOT ready. The logs above say which step, and re-running"
  say "  this script repeats only what is missing."
  rc=1
fi
exit $rc
