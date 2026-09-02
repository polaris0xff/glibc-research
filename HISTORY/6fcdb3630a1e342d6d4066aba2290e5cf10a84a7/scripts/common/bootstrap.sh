#!/bin/sh
# bootstrap.sh - get a FRESH machine ready, in parallel, in one command.
#
# ⛔ THE DEFECT THIS EXISTS TO CATCH, AND IT HAS COST TWO SESSIONS.
#
# Nothing this project needs survives the container. Every session starts with
# 0 of 11 rootfs, no build environment, no static libiconv and no nix, and the
# first part of the session goes on setup. ⚠ Both previous sessions did it
# SERIALLY and INTERACTIVELY -- each step waited for the last while the agent
# watched. Measured here on 2026-09-01c, wall clock:
#
#     nix install (pkgforge install_nix.sh)   ~7 min
#     pgb env create (chroot + libiconv)      ~8 min
#     fetch-rootfs.sh (11 environments)      ~10 min
#     ---------------------------------------------
#     serial                                 ~25 min
#
# ⭐ They are independent and mostly network-bound, so they run TOGETHER here
# and the caller reads the documentation while they do.
#
# ⭐ RESUMABLE. Each step is skipped when its artefact is already on disk --
# checked by looking at the disk, never at a marker this script wrote -- so a
# re-run after a failure costs only what actually failed.
#
# -- ⛔ THE DOCKER TRAP, WHICH IS WHY THIS SCRIPT TOUCHES DOCKER AT ALL -------
#
# `pick_engine` in tool/lib/common.sh prefers docker over chroot the moment
# `docker info` succeeds. So MERELY STARTING dockerd silently switches every
# `pgb build` and `pgb verify` onto an engine whose environment does not
# exist. Reproduced here, immediately after starting the daemon:
#
#     pgb: engine docker has no build environment.
#          chosen engine: docker
#          but these engines DO have one: chroot
#
# T-017's guard catches it with a good message, and a session that just
# started dockerd would still hit it on its first build. ⭐ So this script
# does not start dockerd without ALSO building the docker environment: after
# it runs, whichever engine `pick_engine` picks is one that works.
# `--no-docker` leaves the daemon alone entirely.
#
# Usage:
#   "./pgb" bootstrap              # everything, in parallel, wait
#   "./pgb" bootstrap --detach     # start and return; --check later
#   "./pgb" bootstrap --check      # what is present; exit 1 if not ready
#   "./pgb" bootstrap --selftest   # the logic, offline, no side effects
#   "./pgb" bootstrap --no-nix --no-bed --no-env --no-docker
#
# Exit: 0 ready, 1 something failed, 2 could not run.

set -u

SELF=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO=$(dirname "$(dirname "$SELF")")
LOGDIR="${PGB_BOOTSTRAP_LOGS:-/var/tmp/pgb-bootstrap}"
ROOTFS_DIR="${PGB_ROOTFS_DIR:-/var/lib/pgb-rootfs}"
ENV_NAME="${PGB_ENV_NAME:-pgb-env-debian12}"
IMAGES="$SELF/rootfs-images.txt"

# ⛔ THE FLOOR IS MEASURED, NOT GUESSED: the bed is 2.3 GiB, /nix reaches
# 1.4 GiB, the chroot environment about 1.5 GiB, and a POC build wants several
# more. Refusing early with the number beats failing halfway through a
# 2.3 GiB download with ENOSPC, which is the failure this catches.
MIN_FREE_GIB="${PGB_MIN_FREE_GIB:-10}"

DO_NIX=1; DO_BED=1; DO_ENV=1; DO_DOCKER=1; MODE=wait; CHECK=0; SELFTEST=0

while [ $# -gt 0 ]; do
  case "$1" in
    --check)     CHECK=1 ;;
    --selftest)  SELFTEST=1 ;;
    --no-nix)    DO_NIX=0 ;;
    --no-bed)    DO_BED=0 ;;
    --no-env)    DO_ENV=0 ;;
    --no-docker) DO_DOCKER=0 ;;
    --wait)      MODE=wait ;;
    --detach)    MODE=detach ;;
    -h|--help)   awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"; exit 0 ;;
    *)           printf 'bootstrap: unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
  shift
done

say()  { printf '%s\n' "$*"; }
warn() { printf 'bootstrap: %s\n' "$*" >&2; }

# ---------------------------------------------------------------------------
# What "present" means. ⛔ EVERY ANSWER IS A FILE ON DISK. A bootstrap that
# trusts a marker it wrote itself reports success after a step that half-ran.
# ---------------------------------------------------------------------------
have_nix() {
  for c in nix /nix/var/nix/profiles/default/bin/nix "$HOME/.nix-profile/bin/nix"; do
    command -v "$c" >/dev/null 2>&1 && return 0
    [ -x "$c" ] && return 0
  done
  return 1
}
have_env()        { [ -s "$ROOTFS_DIR/$ENV_NAME/.pgb-env-stamp" ]; }
have_dockerd()    { command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; }
have_docker_env() {
  have_dockerd || return 1
  docker image inspect "pgb-env:$(sed -n 's/^PGB_VERSION="\(.*\)"/\1/p' "$REPO/pgb" | head -1)" \
    >/dev/null 2>&1
}
bed_count() {
  _n=0
  while read -r ref name libc digest; do
    case "$ref" in ''|\#*) continue ;; esac
    [ -d "$ROOTFS_DIR/$name" ] && _n=$((_n + 1))
  done < "$IMAGES"
  printf '%s' "$_n"
}
bed_total() { grep -cvE '^[[:space:]]*(#|$)' "$IMAGES"; }

# ⛔ THE ONE GUARD THAT PROTECTS SOMEBODY ELSE'S DATA. pkgforge's installer
# opens with `sudo rm -rf /nix /etc/nix ~/.nix-profile ~/.nix-channels ...`.
# That is correct for a machine with no nix and DESTRUCTIVE for a machine with
# a half-installed one or a store another session is using. So: run it only
# when there is no /nix at all.
nix_install_is_safe() {
  have_nix && return 1          # already working: nothing to do, not "safe to wipe"
  [ -e /nix ] && return 1       # a store exists but nix does not run: DO NOT WIPE
  return 0
}

free_gib() {
  df -Pk "${1:-/}" 2>/dev/null | awk 'NR==2 { printf "%d", $4/1024/1024 }'
}

report() {
  _b=$(bed_count); _t=$(bed_total)
  printf '  %-22s %s\n' "nix"               "$(have_nix && echo present || echo absent)"
  printf '  %-22s %s\n' "build env (chroot)" "$(have_env && echo present || echo ABSENT)"
  printf '  %-22s %s\n' "test bed"          "$_b of $_t rootfs"
  printf '  %-22s %s\n' "dockerd"           "$(have_dockerd && echo running || echo 'not running')"
  printf '  %-22s %s\n' "build env (docker)" "$(have_docker_env && echo present || echo absent)"
  printf '  %-22s %s GiB\n' "free disk"     "$(free_gib /)"
  # ⚠ READY DOES NOT REQUIRE NIX. `pgb nix plan` has a nix-free route for a
  # package whose .drv is cached, and a committed plan needs none at all.
  # Demanding nix here would send a session installing something it may not
  # need. It DOES require that the engine pick_engine will choose has an
  # environment, which is the docker trap in the header.
  have_env || return 1
  [ "$_b" = "$_t" ] || return 1
  if have_dockerd && ! have_docker_env; then
    printf '\n'
    # ⛔ SINGLE QUOTES, AND THE REASON IS THAT THE FIRST VERSION HAD BACKTICKS
    # HERE. `warn "... every `pgb build` will refuse."` is a command
    # substitution: the shell ran `pgb build`, printed
    # `bootstrap.sh: 1: pgb: not found`, and spliced the empty result into the
    # middle of the sentence. It is the same defect the header of
    # mine-repo.sh documents -- a prose payload the shell reached into.
    warn '⛔ dockerd is RUNNING and the docker environment is MISSING.'
    warn '   pick_engine prefers docker, so every pgb build will refuse.'
    warn '   Fix it with either:'
    warn '     ./pgb env create --engine docker'
    warn '     ./pgb build --engine chroot ...   (per invocation)'
    return 1
  fi
  return 0
}

# ---------------------------------------------------------------------------
# --selftest: the decisions, offline, with no side effects
# ---------------------------------------------------------------------------
if [ "$SELFTEST" = 1 ]; then
  bad=0
  chk() {
    if [ "$2" = "$3" ]; then printf '  ok    %-44s = %s\n' "$1" "$2"
    else printf '  FAIL  %-44s = %s, expected %s\n' "$1" "$2" "$3"; bad=1; fi
  }
  t=$(mktemp -d) || exit 2

  # bed_count must count only the rootfs that exist, and must not be fooled by
  # a comment line or a blank line in the image list.
  IMAGES="$t/images.txt"
  printf '# a comment\n\nrepo:tag  alpha  musl  sha256:x\nrepo:tag  beta  glibc  sha256:y\n' > "$IMAGES"
  ROOTFS_DIR="$t/roots"; mkdir -p "$ROOTFS_DIR/alpha"
  chk "bed_total ignores comments and blanks" "$(bed_total)" 2
  chk "bed_count counts what is on disk" "$(bed_count)" 1
  mkdir -p "$ROOTFS_DIR/beta"
  chk "bed_count after the second appears" "$(bed_count)" 2

  # have_env must want the STAMP, not merely the directory: an environment
  # that half-built has the directory and no stamp, and treating it as present
  # is how a session builds against a rootfs with no compiler in it.
  ENV_NAME=e; mkdir -p "$ROOTFS_DIR/e"
  chk "half-built environment is not 'present'" "$(have_env && echo yes || echo no)" no
  printf 'stamp\n' > "$ROOTFS_DIR/e/.pgb-env-stamp"
  chk "environment with a stamp is present" "$(have_env && echo yes || echo no)" yes

  # ⛔ THE DESTRUCTIVE-INSTALL GUARD, which is the only check here that
  # protects data rather than a result.
  chk "installer refused when /nix already exists" \
      "$( [ -e /nix ] && { nix_install_is_safe && echo run || echo refuse; } || echo 'n/a' )" \
      "$( [ -e /nix ] && echo refuse || echo 'n/a' )"

  rm -rf "$t"
  printf 'bootstrap --selftest: %s\n' \
    "$([ "$bad" = 0 ] && echo 'all checks pass.' || echo 'FAILURES above.')"
  exit "$bad"
fi

if [ "$CHECK" = 1 ]; then
  say "pgb bootstrap: what this machine has"
  if report; then say "  VERDICT: ready to build and verify."; exit 0
  else say "  VERDICT: NOT ready. Run: "./pgb" bootstrap"; exit 1; fi
fi

# ---------------------------------------------------------------------------
# Preflight. ⛔ EVERYTHING THAT WOULD FAIL LATER IS CHECKED HERE, because a
# missing tool found ten minutes into a 2.3 GiB download costs the download.
# ---------------------------------------------------------------------------
say "pgb bootstrap: preparing a fresh machine"
say ""
fatal=0
for t in curl python3 git tar xz; do
  command -v "$t" >/dev/null 2>&1 || { warn "missing: $t"; fatal=1; }
done
[ -r "$IMAGES" ] || { warn "missing: $IMAGES"; fatal=1; }
if [ "$DO_ENV" = 1 ] || [ "$DO_BED" = 1 ]; then
  [ "$(id -u)" = 0 ] || { warn "the chroot bed needs root (id -u = $(id -u))"; fatal=1; }
  command -v unshare >/dev/null 2>&1 || { warn "missing: unshare"; fatal=1; }
fi
_free=$(free_gib /)
if [ -n "$_free" ] && [ "$_free" -lt "$MIN_FREE_GIB" ]; then
  warn "only ${_free} GiB free on /; this needs about ${MIN_FREE_GIB}."
  warn "the bed is 2.3 GiB, /nix reaches 1.4, the chroot env about 1.5,"
  warn "and a POC build wants several more. Set PGB_MIN_FREE_GIB to override."
  fatal=1
fi
[ "$fatal" = 0 ] || { warn "refusing to start. Nothing was changed."; exit 2; }
say "  preflight ok: ${_free} GiB free, root, unshare, curl, python3"

mkdir -p "$LOGDIR" || { warn "cannot write $LOGDIR"; exit 2; }

# ---------------------------------------------------------------------------
# dockerd first: it takes seconds, and whether it comes up decides whether the
# docker environment is one of the parallel jobs.
# ---------------------------------------------------------------------------
if [ "$DO_DOCKER" = 1 ] && command -v docker >/dev/null 2>&1 && ! have_dockerd; then
  say "  starting dockerd  (log: $LOGDIR/dockerd.log)"
  ( dockerd >"$LOGDIR/dockerd.log" 2>&1 & ) 2>/dev/null
  _w=0
  while [ "$_w" -lt 30 ]; do
    have_dockerd && break
    _w=$((_w + 1)); sleep 1
  done
  if have_dockerd; then say "  dockerd up after ${_w}s"
  else
    # ⚠ NOT FATAL, and the reason is in the log. A container without the right
    # capabilities cannot run dockerd, and the chroot engine is the one every
    # committed number in this repository was measured through anyway.
    warn "dockerd did not come up in 30s -- continuing with the chroot engine"
    tail -3 "$LOGDIR/dockerd.log" 2>/dev/null | sed 's/^/      /' >&2
  fi
fi

# ---------------------------------------------------------------------------
# The parallel phase.
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
    -o "$d/install_nix.sh" || { rm -rf "$d"; return 1; }
  chmod +x "$d/install_nix.sh"
  bash "$d/install_nix.sh"
  _rc=$?
  rm -rf "$d"
  return $_rc
}

if [ "$DO_NIX" = 1 ]; then
  if have_nix; then
    say "  nix already present, skipping"
  elif nix_install_is_safe; then
    # ⚠ SAID OUT LOUD BEFORE IT RUNS. This is a third-party script fetched
    # over the network and run as root, and it is here only because the
    # operator authorised it by name with this URL in the session of
    # 2026-09-01c. ⛔ Do not add a second installer to this file on your own
    # authority.
    say "  starting nix     ⚠ third-party installer, run as ROOT:"
    say "                   pkgforge/devscripts Linux/install_nix.sh"
    start nix install_nix
  else
    warn "/nix exists but nix does not run: NOT running the installer."
    warn "  its first action is 'sudo rm -rf /nix', which would destroy a"
    warn "  store this script did not create. Remove /nix by hand if that is"
    warn "  really what you want, then re-run."
  fi
fi

if [ "$DO_ENV" = 1 ]; then
  if have_env; then say "  chroot environment already present, skipping"
  else start env "$REPO/pgb" env create; fi
fi

if [ "$DO_BED" = 1 ]; then
  if [ "$(bed_count)" = "$(bed_total)" ]; then say "  test bed already complete, skipping"
  else start bed sh "$SELF/fetch-rootfs.sh"; fi
fi

# ⛔ AND THE DOCKER ENVIRONMENT, whenever the daemon is up, because starting
# the daemon is what makes pick_engine choose docker. See the header.
if [ "$DO_DOCKER" = 1 ] && have_dockerd; then
  if have_docker_env; then say "  docker environment already present, skipping"
  else start env-docker "$REPO/pgb" env create --engine docker; fi
fi

if [ -z "$PIDS" ]; then
  say ""
  say "nothing to do -- this machine is already set up."
  if report; then exit 0; else exit 1; fi
fi

if [ "$MODE" = detach ]; then
  say ""
  say "started in the background. Read docs/AGENTS.md and TODO/PROGRESS.md,"
  say "then: "./pgb" bootstrap --check"
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
  say "  engine pgb will pick: $(have_dockerd && echo docker || echo chroot)"
else
  say "  VERDICT: NOT ready. The logs above say which step, and re-running"
  say "  this script repeats only what is missing."
  rc=1
fi
exit $rc
