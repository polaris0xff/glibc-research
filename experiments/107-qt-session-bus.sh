#!/bin/sh
# THE QUESTION — and it is the last UNEXPLAINED row of the capability corpus.
#
# `experiments/65-` measures twenty-six subjects on eleven environments and
# scores each on two axes: does it WORK, and does it load a HOST shared object.
# Twenty-five rows agree with themselves. One does not:
#
#   qt-1  qalculate-qt   pass 11/11   clean 4/11
#
# ⛔ A SUBJECT THAT WORKS EVERYWHERE AND IS CLEAN ALMOST NOWHERE IS THE MOST
# SUSPICIOUS SHAPE IN THE TABLE, because the two numbers are measured off the
# SAME run: eleven windows opened, and seven of those eleven runs also touched
# something belonging to the host. Either the bundle is leaking on seven
# environments — which would be a real defect in the bundler — or the residue
# is not the bundle's and the corpus is attributing it to the wrong thing.
#
# ⭐ THE MECHANISM WAS TRACED BY HAND (2026-09-04c) AND IT IS THE SECOND ONE.
# Qt, finding no session bus, AUTOLAUNCHES one. That path in QtDBus does not
# start a daemon directly; it looks for `dbus-launch` at two ABSOLUTE paths:
#
#     execve("/sbin/dbus-launch", …)   ENOENT
#     execve("/bin/dbus-launch",  …)   ENOENT
#
# and having failed, falls back to running the launcher THROUGH A SHELL — so
# the artefact execs the environment's `/bin/sh`, and on a glibc rootfs that
# shell is dynamically linked and pulls in the HOST's `libc.so.6`. That object
# is real, the classifier is right to count it, and it has nothing to do with
# the bundle: it is a shell the subject spawned and did not bring.
#
# -- ⛔ PRE-REGISTERED EXPECTATIONS ------------------------------------------
#
# ⚠ These are written before the run, and Q2 is the one that can fail for the
# right reason: it is a PREDICTION about which four rows are clean, derived
# from the mechanism, and the mechanism survives Q2 failing only if Q3 does
# not also fail.
#
#   Q1  ⭐ THE CORPUS NUMBER REPRODUCES. Arm A is the subject built exactly as
#       the corpus built it, and it must read clean on 4 of 11. ⛔ Without
#       this the other three say nothing — a different number here would mean
#       107 and 65 are not measuring the same thing.
#   Q2  ⭐ THE FOUR CLEAN ROWS ARE THE FOUR musl ROWS. Alpine 3.22, 3.20, 3.10
#       and Void musl ship a STATIC busybox as `/bin/sh`: exec'ing it loads no
#       shared object at all, so the same failed autolaunch leaves no residue.
#       The seven glibc rows have a dynamic `/bin/sh` and must be dirty.
#   Q3  ⭐ AND THE RESIDUE IS THE SHELL'S, NOT Qt's. On every dirty row, each
#       host object counted must be a libc-family object — the shell's — and
#       NOT a Qt, X11 or GL library. ⛔ A host `libQt*` or `libX11*` would mean
#       the bundle really is leaking and the DBus story is a coincidence.
#   Q4  ⛔ THE FIX IS A BUILD FLAG AND IT MUST CLOSE THE ROW. Arm B carries the
#       launcher out of its own closure — `--with-program dbus-daemon
#       dbus-launch` — so Qt finds a `dbus-launch` on PATH, never reaches the
#       shell fallback, and must read clean on ALL eleven while still opening
#       its window on all eleven.
#
# ⚠ WHAT THE BED SUPPLIES AND WHAT IT DOES NOT. `dbus-daemon` insists on
# reading `/etc/dbus-1/session.conf`, an absolute /etc path with no search
# variable that the interposer does not reach by construction. That FILE is a
# bed fixture (`scripts/common/bed-fixtures.sh --install dbus`, DATA only) and
# this experiment checks it is installed rather than assuming it. ⛔ The
# DAEMON is not a fixture — it comes out of the bundle, which is the point.
#
# ⚠ WHAT THIS DOES NOT ESTABLISH. It explains ONE row. It does not say the
# corpus's other clean counts are right, and it does not say a bundle can
# never leak — it says THIS residue is a shell the subject spawned, and names
# the flag that stops it being spawned.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "107 - qalculate-qt's 4-of-11 clean: Qt autolaunches a bus, and execs the host shell"

WORK="${PGB_EXP107_WORK:-/var/tmp/t107}"
mkdir -p "$WORK" || exit 2
ATTR="${PGB_EXP107_ATTR:-qalculate-qt}"
PROG=qalculate-qt
RUN_TIMEOUT="${PGB_EXP107_TIMEOUT:-90}"
WIN_WAIT="${PGB_EXP107_WINWAIT:-25}"

ENVS=$(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt")
NENV=$(printf '%s\n' "$ENVS" | wc -l | tr -d ' ')

reap_in_root() {
  _rr=$1
  for _p in /proc/[0-9]*; do
    _pid=${_p#/proc/}
    _rt=$(readlink "/proc/$_pid/root" 2>/dev/null) || continue
    case "$_rt" in "$_rr"|"$_rr"/*) kill -9 "$_pid" 2>/dev/null ;; esac
  done
}

# ⭐ musl OR glibc COMES FROM lib.sh's `exp_rootfs_libc`, NOT FROM A SECOND
# COPY HERE. The first draft of this file hand-rolled the test, which is the
# shape TODO/ci.md T-084 spent a session undoing: six hand copies of the trace
# classifier that drifted apart and carried the same defect five times.
# ⚠ It takes the rootfs NAME, not its path.

# ⭐ IS THE ROOTFS'S OWN /bin/sh DYNAMIC? This is the actual discriminator Q2
# rests on, so it is MEASURED rather than inferred from the libc name: a
# static shell cannot load a host object however the autolaunch fails.
sh_dynamic() {
  _p=$1/bin/sh
  [ -e "$_p" ] || _p=$1/usr/bin/sh
  [ -e "$_p" ] || { echo unknown; return; }
  if head -c 4096 "$_p" 2>/dev/null | grep -aq 'ld-musl\|ld-linux'; then
    echo yes
  else
    echo no
  fi
}

XDISP="${PGB_EXP107_DISPLAY:-:107}"
start_x() {
  command -v Xvfb >/dev/null 2>&1 || return 1
  pgrep -f "Xvfb $XDISP" >/dev/null 2>&1 && return 0
  Xvfb "$XDISP" -screen 0 1280x800x24 -nolisten tcp >/dev/null 2>&1 &
  _n=0
  while [ "$_n" -lt 20 ]; do
    sleep 1; _n=$((_n+1))
    DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1 && return 0
  done
  return 1
}

# ⛔ SEEN FROM OUTSIDE, WITH A SIZE. The operator's bar: a toplevel >=50x50 on
# a real X server read by `xwininfo`, not a log line the program printed about
# itself.
windows_real() {
  DISPLAY="$XDISP" xwininfo -root -children 2>/dev/null \
    | awk '/^ +0x/ {
        if (match($0, /[0-9]+x[0-9]+\+/)) {
          split(substr($0, RSTART, RLENGTH - 1), d, "x")
          if (d[1] >= 50 && d[2] >= 50) n++
        }
      } END { print n+0 }'
}

build() {  # out-image extra-flag...
  _img=$1; shift
  if [ ! -s "$_img" ]; then
    PGB_APPIMAGE_CACHE="$WORK/cache" "$REPO_DIR/pgb" bundle appimage "$ATTR" \
      --out "$_img" --name "$PROG" "$@" >"$_img.log" 2>&1 || true
  fi
  [ -s "$_img" ]
}

printf -- '-- the bed --------------------------------------------------------\n'
# ⛔ CHECKED, NOT ASSUMED. A run that silently measured a bed without the
# fixture would blame the bundle for a file the bed was missing.
sh "$REPO_DIR/scripts/common/bed-fixtures.sh" --install dbus >"$WORK/fixture.log" 2>&1 || true
nfix=$(grep -c 'session.conf' "$WORK/fixture.log" 2>/dev/null || echo 0)
have=0
for name in $ENVS; do
  root=$(exp_rootfs "$name" 2>/dev/null) || continue
  [ -n "$root" ] && [ -s "$root/etc/dbus-1/session.conf" ] && have=$((have+1))
done
exp_note "$(printf '   /etc/dbus-1/session.conf present on %s rootfs' "$have")"

printf -- '\n-- building the two arms ------------------------------------------\n'
A="$WORK/qalc.AppImage"
B="$WORK/qalc-bus.AppImage"
build "$A" || { exp_note "arm A did not build; see $A.log"; tail -5 "$A.log"; exit 2; }
exp_check "Q0  arm A built (stock, as the corpus built it)" \
    "$([ -s "$A" ] && echo yes || echo no)" yes
# ⛔ `--extra dbus` IS NOT DECORATION. `--with-program` can only install a
# program that is already IN the closure, and warns `no such program in the
# closure` otherwise — Qt links libdbus but nothing in qalculate-qt's runtime
# closure needs the `dbus-launch` BINARY, so without the extra the flag is a
# no-op and arm B would silently be arm A under another name.
build "$B" --extra dbus --with-program dbus-daemon --with-program dbus-launch \
  || exp_note "arm B did not build; see $B.log"
exp_check "Q0  ⭐ arm B built (--extra dbus --with-program dbus-daemon dbus-launch)" \
    "$([ -s "$B" ] && echo yes || echo no)" yes
exp_note "$(printf '   arm B programs: %s' \
    "$(grep -a -m1 '^programs' "$B.log" 2>/dev/null | cut -c1-100)")"
# ⛔ AND THE WARNING IS AN INSTRUMENT ERROR, NOT A ROW. If either program is
# missing, arm B is arm A and Q4 would be measuring nothing.
miss=$(grep -ac 'no such program in the closure' "$B.log" 2>/dev/null || echo 0)
exp_check "Q0  ⛔ arm B installed BOTH programs (no 'no such program')" "${miss:-0}" 0
[ "${miss:-0}" != 0 ] && exp_note "$(printf '   %s' \
    "$(grep -a 'no such program in the closure' "$B.log" | head -2 | tr '\n' ' ')")"

start_x || { exp_note "no X server on $XDISP"; exit 2; }

printf -- '\n-- the eleven -----------------------------------------------------\n'
printf '  %-18s %-6s %-7s %-9s %-9s %-9s %s\n' \
    ENVIRONMENT LIBC 'sh dyn' 'A win' 'A host' 'B win' 'B host'

rows=0; a_clean=0; b_clean=0; a_win=0; b_win=0
musl_clean=0; musl_rows=0; glibc_dirty=0; glibc_rows=0; qtleak=0
for name in $ENVS; do
  root=$(exp_rootfs "$name") || true
  [ -n "$root" ] || { exp_skip "$name" "rootfs not fetched"; continue; }

  libc=$(exp_rootfs_libc "$name")
  shd=$(sh_dynamic "$root")

  # ⛔ STAGE FIRST, AND A FAILED COPY IS A SKIP, NOT A ZERO. corrections.md C44.
  rm -f "$root/subjA" "$root/subjB"
  if ! cp "$A" "$root/subjA" 2>"$WORK/cp.$name"; then
    exp_skip "$name" "could not stage arm A: $(tr -d '\n' < "$WORK/cp.$name" | cut -c1-80)"
    rm -f "$root/subjA" "$WORK/cp.$name"; continue
  fi
  chmod +x "$root/subjA"
  bhave=no
  if [ -s "$B" ] && cp "$B" "$root/subjB" 2>/dev/null; then
    chmod +x "$root/subjB"; bhave=yes
  fi
  rows=$((rows+1))
  [ "$libc" = musl ] && musl_rows=$((musl_rows+1)) || glibc_rows=$((glibc_rows+1))

  arm_run() {  # subject-path trace-out out-prefix -> prints "<win> <nhost>"
    _sp=$1; _tr=$2; _op=$3
    _q=0
    while [ "$_q" -lt 10 ] && [ "$(windows_real)" != 0 ]; do sleep 1; _q=$((_q+1)); done
    _base=$(windows_real)
    strace -f -e trace=openat,open,execve,clone,clone3,vfork -o "$_tr" \
      timeout "$RUN_TIMEOUT" "$REPO_DIR/pgb" rootfs run "$root" \
        --bind /tmp/.X11-unix:/tmp/.X11-unix -- \
        /bin/sh -c "DISPLAY=$XDISP APPIMAGE_EXTRACT_AND_RUN=1 $_sp" \
      >"$_op.out" 2>"$_op.err" &
    _pid=$!
    _w=0; _n=0
    while [ "$_n" -lt "$WIN_WAIT" ]; do
      sleep 1; _n=$((_n+1))
      _w=$(windows_real)
      [ "$_w" -gt "$_base" ] && break
      # ⭐ A GUI PROGRAM THAT WORKS DOES NOT EXIT, so this is the branch that
      # ends a FAILING row early rather than the one that ends a passing one.
      kill -0 "$_pid" 2>/dev/null || break
    done
    kill "$_pid" 2>/dev/null
    wait "$_pid" 2>/dev/null
    reap_in_root "$root"
    [ "$_w" -gt "$_base" ] && printf 'yes ' || printf 'no '
    exp_classify_trace "$_tr" "$_sp" | grep -c '^host ' || true
  }

  set -- $(arm_run /subjA "$WORK/tr.A.$name" "$WORK/A.$name")
  awin=$1; ahost=$2
  bwin=-; bhost=-
  if [ "$bhave" = yes ]; then
    set -- $(arm_run /subjB "$WORK/tr.B.$name" "$WORK/B.$name")
    bwin=$1; bhost=$2
  fi
  rm -f "$root/subjA" "$root/subjB"

  printf '  %-18s %-6s %-7s %-9s %-9s %-9s %s\n' \
      "$name" "$libc" "$shd" "$awin" "$ahost" "$bwin" "$bhost"

  [ "$awin" = yes ] && a_win=$((a_win+1))
  [ "$bwin" = yes ] && b_win=$((b_win+1))
  if [ "$ahost" = 0 ]; then
    a_clean=$((a_clean+1))
    [ "$libc" = musl ] && musl_clean=$((musl_clean+1))
  else
    [ "$libc" != musl ] && glibc_dirty=$((glibc_dirty+1))
    # ⭐ Q3: WHAT the host objects ARE, not how many. A libc-family object is
    # the shell's; a Qt/X11/GL one would be the bundle leaking, which is a
    # different and much worse finding.
    objs=$(exp_classify_trace "$WORK/tr.A.$name" /subjA | sed -n 's/^host  *//p' \
           | sed 's|.*/||' | sort -u | tr '\n' ' ')
    exp_note "$(printf '   %s arm A host objects: %s' "$name" \
        "$(printf '%s' "$objs" | cut -c1-140)")"
    printf '%s' "$objs" | grep -qE 'libQt|libX11|libGL|libEGL|libgtk' && qtleak=$((qtleak+1))
  fi
  [ "$bhost" != - ] && [ "$bhost" = 0 ] && b_clean=$((b_clean+1))
  # ⛔ AND WHEN ARM B IS STILL DIRTY, SAY WITH WHAT — a Q4 failure with no
  # object named is a number nobody can act on.
  if [ "$bhost" != - ] && [ "$bhost" != 0 ]; then
    exp_note "$(printf '   %s arm B STILL dirty: %s' "$name" \
        "$(exp_classify_trace "$WORK/tr.B.$name" /subjB | sed -n 's/^host  *//p' \
           | sed 's|.*/||' | sort -u | tr '\n' ' ' | cut -c1-140)")"
  fi
  # ⭐ THE MECHANISM ITSELF, READ OFF THE TRACE AND KEPT. ⛔ A count would not
  # be actionable: if Q4 fails, the next session needs the PATHS Qt actually
  # tried, because "it is on PATH now" is only a fix if PATH is what it
  # consults. So the execve lines naming dbus-launch and every shell exec are
  # written out per arm, and the FIRST environment's full traces are kept.
  for arm in A B; do
    _t="$WORK/tr.$arm.$name"
    [ -s "$_t" ] || continue
    grep -aE 'execve\("[^"]*(dbus-launch|dbus-daemon|/sh|/bash|/dash)"' "$_t" \
      | sed 's/^[0-9]* *//' | cut -c1-160 | sort -u > "$WORK/exec.$arm.$name" || true
    n=$(grep -ac . "$WORK/exec.$arm.$name" 2>/dev/null || echo 0)
    [ "$n" != 0 ] && exp_note "$(printf '   %s arm %s: %s launcher/shell exec(s), e.g. %s' \
        "$name" "$arm" "$n" "$(head -1 "$WORK/exec.$arm.$name" | cut -c1-100)")"
  done
  # ⚠ Traces are ~100 MiB each and disk is the binding constraint, so only the
  # first environment's survive -- enough to re-read the mechanism by hand.
  if [ "$rows" = 1 ]; then
    mv "$WORK/tr.A.$name" "$WORK/keep.tr.A.$name" 2>/dev/null || :
    mv "$WORK/tr.B.$name" "$WORK/keep.tr.B.$name" 2>/dev/null || :
  fi
  rm -f "$WORK/tr.A.$name" "$WORK/tr.B.$name"
done

printf '\n'
exp_check "Q1  ⭐ arm A reproduces the corpus: clean on 4 of $rows" "$a_clean" 4
exp_check "Q1  arm A opens a window on all $rows"                  "$a_win"   "$rows"
exp_check "Q2  ⭐ every musl row is clean ($musl_rows musl)"        "$musl_clean" "$musl_rows"
exp_check "Q2  ⭐ every glibc row is dirty ($glibc_rows glibc)"     "$glibc_dirty" "$glibc_rows"
exp_check "Q3  ⛔ no row leaks a Qt/X11/GL host object"             "$qtleak"  0
exp_check "Q4  ⭐ arm B is clean on all $rows"                      "$b_clean" "$rows"
exp_check "Q4  arm B still opens a window on all $rows"             "$b_win"   "$rows"

exp_note "⭐ WHAT THE ROW MEANS ONCE THIS IS GREEN. qalculate-qt's 4-of-11 is"
exp_note "   not the bundle leaking. Qt autolaunches a session bus, cannot find"
exp_note "   dbus-launch at /sbin or /bin, falls back through a SHELL, and on"
exp_note "   the seven glibc rows that shell is dynamic and loads the host"
exp_note "   libc. The four clean rows are the musl ones, where /bin/sh is a"
exp_note "   static busybox and the same failure costs nothing."
exp_note "⛔ AND IT IS A BUILD FLAG, NOT A PATCH: --with-program dbus-daemon"
exp_note "   dbus-launch puts the launcher in the artefact, the shell fallback"
exp_note "   is never reached, and the row closes at 11/11."
exp_note "⚠ ONE subject. It explains this row; it does not revalidate the"
exp_note "   other twenty-five, and it does not say a bundle can never leak."

exp_finish
