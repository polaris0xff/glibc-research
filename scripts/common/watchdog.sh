#!/bin/sh
# watchdog.sh — the resource watch a long bundle run needs, because the two
# ways these runs die are silent.
#
# ⛔ WHY THIS EXISTS. A bundle run fetches multi-GB closures and mounts FUSE
# filesystems. Both failure modes kill the work without saying so:
#
#   ENOSPC       the writable allowance is a fixed per-session budget, so `df`
#                reads "Avail 0" with a low "Used" and every later write fails.
#                A run that dies this way has measured nothing and the log
#                blames whatever wrote last.
#   stray mounts a dwarfs FUSE daemon outlives its AppImage when the tracer is
#                killed. Each holds its extraction directory, so disk is not
#                reclaimed and `strace` on the NEXT row can deadlock against
#                it (docs/history/corrections.md, experiments/64- arm P).
#
# ⭐ IT REPORTS AND IT REAPS, AND THE TWO ARE SEPARATE. Reporting is always
# safe; reaping is not, so `--reap` is opt-in and never touches a mount whose
# daemon is younger than --min-age seconds — a live run's own mount must not be
# pulled out from under it.
#
# Usage:
#   sh scripts/common/watchdog.sh                     one report, then exit
#   sh scripts/common/watchdog.sh --watch             report every 60s
#   sh scripts/common/watchdog.sh --watch --reap      ...and reap stray mounts
#   sh scripts/common/watchdog.sh --watch --interval 30 --floor 5 \
#       --log /var/tmp/watchdog.log &
#
# Options:
#   --watch            loop instead of reporting once
#   --interval N       seconds between reports (default 60)
#   --floor N          GiB below which every line is prefixed ALERT (default 4)
#   --reap             unmount stray FUSE mounts and kill their daemons
#   --min-age N        seconds a mount must have been idle to be stray (900)
#   --log FILE         append there as well as to stdout
#   --selftest         assert the mountinfo parser against a fixture, then exit
#
# Exit: 0 always in --watch; without it, 1 when free space is under the floor,
# so a caller can gate on it.
#
# SPDX-License-Identifier: MIT
set -u

INTERVAL=60
FLOOR_GIB=4
WATCH=no
REAP=no
MIN_AGE=900
LOGF=""
SELFTEST=no

while [ $# -gt 0 ]; do
  case "$1" in
    --watch)     WATCH=yes ;;
    --reap)      REAP=yes ;;
    --interval)  INTERVAL="$2"; shift ;;
    --floor)     FLOOR_GIB="$2"; shift ;;
    --min-age)   MIN_AGE="$2"; shift ;;
    --log)       LOGF="$2"; shift ;;
    --selftest)  SELFTEST=yes ;;
    -h|--help)   sed -n '2,40p' "$0"; exit 0 ;;
    *) printf 'watchdog: unknown option %s\n' "$1" >&2; exit 2 ;;
  esac
  shift
done

say() {
  printf '%s\n' "$*"
  [ -n "$LOGF" ] && printf '%s\n' "$*" >> "$LOGF"
  return 0
}

# ⛔ df's "Avail" IS THE NUMBER, not "Size minus Used". On this harness the
# writable layer is a fixed allowance, so Used stays small while Avail reaches
# zero, and a check written against Used never fires.
free_gib() {
  df -P -BG / 2>/dev/null | awk 'NR==2 { sub(/G$/, "", $4); print $4+0 }'
}

# The directories a bundle run actually fills, largest first. ⚠ Named rather
# than discovered: a `du /` on this tree walks every rootfs and costs minutes.
BIG_DIRS="/root/.local/state/pgb /var/tmp/t065 /var/tmp/t080 /var/tmp/pgb-appimage
/var/tmp/pgb-poc /var/lib/pgb-rootfs /home/user/glibc-research/evidence"

report_disk() {
  _free=$(free_gib)
  _tag=""
  [ -n "$_free" ] && [ "$_free" -lt "$FLOOR_GIB" ] && _tag="ALERT "
  say "$(date -u +%H:%M:%S) ${_tag}free=${_free}GiB floor=${FLOOR_GIB}GiB"
  [ -n "$_tag" ] || return 0
  # Only when it matters: sizing every candidate costs seconds.
  for d in $BIG_DIRS; do
    [ -d "$d" ] || continue
    say "           $(du -sh "$d" 2>/dev/null | awk '{print $1}')  $d"
  done
  return 1
}

# ⛔ A MOUNT IS NOT STRAY BECAUSE IT EXISTS. It is stray when nothing is using
# it: no process has its mountpoint as cwd or root, and its daemon has been
# idle longer than --min-age. Anything younger belongs to a live run.
# ⛔ THE FSTYPE IS NOT FIELD 3 AND THE FIRST VERSION OF THIS FUNCTION SAID IT
# WAS. /proc/self/mountinfo is
#     ID PARENT MAJ:MIN ROOT MOUNTPOINT OPTIONS [OPTIONAL…] - FSTYPE SRC OPTS
# so field 3 is `major:minor` and the fstype is the field AFTER the `-`
# separator, whose position varies with the optional fields. `$3 ~ /fuse/`
# therefore matched nothing, ever: a mount check that reports "no stray
# mounts" on a machine full of them is worse than no check.
# Verified against a fixture in both directions — see --selftest.
list_fuse() {
  awk '{ for (i = 7; i <= NF; i++) if ($i == "-") { if ($(i+1) ~ /fuse/) print $5; break } }' \
    "${1:-/proc/self/mountinfo}" 2>/dev/null | sort -u
}

# ⭐ A CHECK THAT CANNOT FAIL IS THE ONE THING THIS TREE WILL NOT SHIP, so the
# parser is asserted against a fixture carrying both shapes: a fuse mount with
# optional fields present, and a non-fuse mount whose SOURCE contains "fuse".
selftest() {
  _t=$(mktemp) || exit 2
  cat > "$_t" <<'FIXTURE'
22 27 0:21 / /proc rw,relatime - proc proc rw
99 27 0:55 / /tmp/.mount_abcdef rw,nosuid,nodev,relatime shared:9 - fuse.dwarfs dwarfs rw
98 27 0:56 / /mnt/plain rw,relatime - ext4 /dev/fuse-disk rw
97 27 0:57 / /tmp/.mount_two rw,relatime - fuse squashfuse rw
FIXTURE
  _got=$(list_fuse "$_t" | tr '\n' ' ')
  rm -f "$_t"
  if [ "$_got" = "/tmp/.mount_abcdef /tmp/.mount_two " ]; then
    printf 'ok    mountinfo: the fstype is read after the "-" separator = %s\n' "$_got"
    printf 'ok    mountinfo: a non-fuse mount whose SOURCE says fuse is not counted\n'
    return 0
  fi
  printf 'FAIL  mountinfo: got [%s], wanted [/tmp/.mount_abcdef /tmp/.mount_two ]\n' "$_got"
  return 1
}

daemon_pids_for() {   # mountpoint
  _mp=$1
  for _p in /proc/[0-9]*; do
    _pid=${_p#/proc/}
    _cl=$(tr '\0' ' ' < "/proc/$_pid/cmdline" 2>/dev/null) || continue
    case "$_cl" in *dwarfs*|*fusermount*|*squashfuse*)
      case "$_cl" in *"$_mp"*) printf '%s\n' "$_pid" ;; esac ;;
    esac
  done
}

report_mounts() {
  _n=0
  for mp in $(list_fuse); do
    _n=$((_n+1))
    _users=$(fuser -m "$mp" 2>/dev/null | tr -s ' ' | sed 's/^ *//')
    _age="?"
    for pid in $(daemon_pids_for "$mp"); do
      _st=$(awk '{print $22}' "/proc/$pid/stat" 2>/dev/null)
      _hz=$(getconf CLK_TCK 2>/dev/null || echo 100)
      _up=$(awk '{print int($1)}' /proc/uptime 2>/dev/null)
      [ -n "$_st" ] && [ -n "$_up" ] && _age=$(( _up - _st / _hz ))
      break
    done
    say "           fuse mount age=${_age}s users=[${_users:-none}] $mp"
    [ "$REAP" = yes ] || continue
    [ "$_age" = "?" ] && continue
    [ "$_age" -lt "$MIN_AGE" ] && continue
    [ -n "$_users" ] && continue
    say "           REAPING $mp (idle ${_age}s, no users)"
    fusermount -u "$mp" 2>/dev/null || umount -l "$mp" 2>/dev/null
    for pid in $(daemon_pids_for "$mp"); do kill -9 "$pid" 2>/dev/null; done
  done
  [ "$_n" = 0 ] || say "           $_n fuse mount(s)"
  return 0
}

# ⚠ A PROCESS IN STATE D CANNOT BE KILLED AND IS THE SIGNATURE OF THE
# strace-on-FUSE DEADLOCK. It is reported and never acted on: there is nothing
# to do to it, and the point is that the log says why a run stopped moving.
report_stuck() {
  for _p in /proc/[0-9]*; do
    _pid=${_p#/proc/}
    _st=$(awk '{print $3}' "/proc/$_pid/stat" 2>/dev/null) || continue
    [ "$_st" = D ] || continue
    _cm=$(cat "/proc/$_pid/comm" 2>/dev/null)
    _wc=$(cat "/proc/$_pid/wchan" 2>/dev/null)
    say "           ⛔ pid $_pid ($_cm) is in state D at ${_wc:-?} — uninterruptible, kill will not work"
  done
  return 0
}

once() {
  report_disk; _rc=$?
  report_mounts
  report_stuck
  return $_rc
}

[ "$SELFTEST" = yes ] && { selftest; exit $?; }

if [ "$WATCH" = yes ]; then
  say "watchdog: every ${INTERVAL}s, floor ${FLOOR_GIB}GiB, reap=${REAP} min-age=${MIN_AGE}s"
  while :; do once; sleep "$INTERVAL"; done
fi
once
