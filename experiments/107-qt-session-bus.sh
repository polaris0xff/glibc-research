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
# the artefact execs the environment's `/bin/sh`, which is a dynamic PIE on all
# eleven and therefore pulls in that environment's libc. That object is real,
# the classifier is right to count it, and it has nothing to do with the
# bundle: it is a shell the subject spawned and did not bring.
#
# -- ⛔ PRE-REGISTERED EXPECTATIONS ------------------------------------------
#
# ⚠ These are written before the run. ⭐ Q3(b) is the one that can fail for
# the right reason: if a dirty row spawned nothing, the residue is the
# bundle's after all and the whole reading here is wrong. ⛔ Q1 is the gate —
# a different number there means 107 and 65 are not measuring the same thing
# and nothing below it can be read.
#
#   Q1  ⭐ THE CORPUS NUMBER REPRODUCES. Arm A is the subject built exactly as
#       the corpus built it, and it must read clean on 4 of 11. ⛔ Without
#       this the other three say nothing — a different number here would mean
#       107 and 65 are not measuring the same thing.
#   Q2  ⛔ NOT A PREDICTION — A REFUTATION, AND IT IS THIS FILE'S OWN. The
#       first draft of 107- predicted the four clean rows would be the four
#       musl ones, "because Alpine and Void ship a STATIC busybox as /bin/sh,
#       so exec'ing it loads nothing". ⭐ THAT PREMISE IS FALSE, and asking
#       `readelf -l` before running said so:
#
#           alpine-3.22  /bin/sh -> /bin/busybox
#                        ELF 64-bit LSB pie, DYNAMICALLY LINKED,
#                        interpreter /lib/ld-musl-x86_64.so.1
#
#       All eleven have a dynamic `/bin/sh`, so exec'ing it loads that
#       environment's libc on every one of them and the split cannot be
#       musl-versus-glibc. ⚠ The `sh` column is therefore printed as
#       EVIDENCE for that refutation, and Q2 checks the only thing it can
#       honestly assert: that the split is not the one that was guessed.
#   Q3  ⭐ THE RESIDUE IS SPAWNED, NOT LOADED. Two halves, both checkable:
#       (a) on every dirty row NO host object is a Qt, X11, GL or GTK library
#           — ⛔ one of those would mean the bundle really is leaking and the
#           DBus story is a coincidence; and
#       (b) every dirty row shows an `execve` of a shell or a launcher, which
#           is the positive observation that something WAS spawned. ⛔ Without
#           (b), (a) is an absence and delivery rule 4 does not accept one.
#   Q4  ⛔ THE FIX IS A BUILD FLAG AND IT MUST CLOSE THE ROW. Arm B carries the
#       launcher out of its own closure — `--extra dbus --with-program
#       dbus-daemon dbus-launch` — so Qt finds a `dbus-launch`, never reaches
#       the shell fallback, and must read clean on EVERY row it ran on while
#       still opening its window on every one. ⚠ If Q4 fails, the kept traces
#       say which paths Qt actually tried, because "it is there now" is only a
#       fix if that is where it looks.
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
# ⛔ THE WINDOW BUDGET MUST MATCH THE CORPUS OR Q1 CANNOT REPRODUCE IT, AND
# THE FIRST RUN OF THIS FILE PROVED IT. `experiments/65-` waits
# RUN_TIMEOUT=150 and sets WIN_WAIT to the SAME value; 107- was written with
# 90 and 25. ⭐ Measured on the first two rows: `A win = no` on both, with the
# only window on the server a 3x3 `Qt Selection Owner for qalculate-qt` —
# qalculate-qt had STARTED and had not yet drawn. A 238 MiB Qt artefact needs
# to extract before it can draw, and 25 seconds does not cover it.
#
# ⚠ AND THE FAILURE WOULD HAVE LOOKED LIKE A RESULT: a row that never draws
# also loads zero host objects, so arm A would have read `clean 11/11` — the
# corpus number it exists to reproduce, arrived at by measuring nothing.
# ⭐ Q1 is the gate that caught it, which is what Q1 is for.
RUN_TIMEOUT="${PGB_EXP107_TIMEOUT:-150}"
WIN_WAIT="${PGB_EXP107_WINWAIT:-$RUN_TIMEOUT}"

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

# ⭐ IS THE ROOTFS'S OWN /bin/sh DYNAMIC? Measured rather than inferred from
# the libc name, because a STATIC shell would cost nothing to exec and would
# explain a clean row — which is exactly the story this measurement kills.
#
# ⛔ AND THE SYMLINK MUST BE RESOLVED INSIDE THE ROOTFS, WHICH IS THE MISTAKE
# THIS TREE HAS NOW MADE FOUR TIMES (corrections.md C42, C43, C47). Alpine's
# `/bin/sh` is a symlink to `/bin/busybox` — an ABSOLUTE target, which `[ -e ]`
# and `head` both resolve against the HOST. On a host without busybox that
# reads "unknown"; on a host WITH one it would read the host's shell and say
# nothing. So each link is followed by hand, re-rooted at every hop.
# ⛔ AND IT IS READ WITH `readelf -l`, NOT BY GREPPING THE FIRST FEW KiB. The
# first draft sniffed `head -c 4096 | grep ld-musl` and called fedora-42's bash
# STATIC and alpine's busybox DYNAMIC on no better evidence than where the
# string happened to land. ⭐ PT_INTERP is a program header; asking for it is
# exact, and asking for it is what refuted this experiment's own first
# prediction before it cost an hour of bed time.
sh_dynamic() {
  _root=$1; _p=/bin/sh; _hop=0
  while [ "$_hop" -lt 10 ]; do
    _hop=$((_hop+1))
    [ -e "$_root$_p" ] || [ -L "$_root$_p" ] || { _p=/usr/bin/sh; [ "$_hop" = 1 ] && continue; echo unknown; return; }
    _t=$(readlink "$_root$_p" 2>/dev/null) || _t=""
    [ -n "$_t" ] || break
    case "$_t" in
      /*) _p=$_t ;;
      *)  _p=$(dirname "$_p")/$_t ;;
    esac
  done
  [ -f "$_root$_p" ] || { echo unknown; return; }
  if readelf -l "$_root$_p" 2>/dev/null | grep -q 'program interpreter'; then
    echo dynamic
  else
    echo static
  fi
}

XDISP="${PGB_EXP107_DISPLAY:-:107}"
start_x() {
  command -v Xvfb >/dev/null 2>&1 || return 1
  # ⛔ ASK THE SERVER, DO NOT `pgrep -f`. A `pgrep -f "Xvfb :107"` matches this
  # script's OWN command line as readily as a server, so it can report a
  # display that is not there and every row then fails for a reason the table
  # does not show. ⚠ Same family as the `pkill -f` trap TODO/RESUME.md records,
  # which kills the harness shell instead of the subject.
  DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1 && return 0
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
nfix=$(exp_count 'session.conf' "$WORK/fixture.log")
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
miss=$(exp_count 'no such program in the closure' "$B.log")
exp_check "Q0  ⛔ arm B installed BOTH programs (no 'no such program')" "${miss:-0}" 0
[ "${miss:-0}" != 0 ] && exp_note "$(printf '   %s' \
    "$(grep -a 'no such program in the closure' "$B.log" | head -2 | tr '\n' ' ')")"

start_x || { exp_note "no X server on $XDISP"; exit 2; }

printf -- '\n-- the eleven -----------------------------------------------------\n'
printf '  %-18s %-6s %-7s %-9s %-9s %-9s %s\n' \
    ENVIRONMENT LIBC 'sh dyn' 'A win' 'A host' 'B win' 'B host'

rows=0; b_rows=0; a_clean=0; b_clean=0; a_win=0; b_win=0
musl_rows=0; glibc_rows=0; qtleak=0; dyn_sh=0; dirty=0; dirty_spawn=0
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
  [ "$shd" = dynamic ] && dyn_sh=$((dyn_sh+1))

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
    b_rows=$((b_rows+1))
  fi
  rm -f "$root/subjA" "$root/subjB"

  printf '  %-18s %-6s %-7s %-9s %-9s %-9s %s\n' \
      "$name" "$libc" "$shd" "$awin" "$ahost" "$bwin" "$bhost"

  [ "$awin" = yes ] && a_win=$((a_win+1))
  [ "$bwin" = yes ] && b_win=$((b_win+1))
  if [ "$ahost" = 0 ]; then
    a_clean=$((a_clean+1))
  else
    dirty=$((dirty+1))
    # ⭐ Q3(a): WHAT the host objects ARE, not how many. A libc-family object
    # is a spawned process's; a Qt/X11/GL one would be the bundle leaking,
    # which is a different and much worse finding.
    objs=$(exp_classify_trace "$WORK/tr.A.$name" /subjA | sed -n 's/^host  *//p' \
           | sed 's|.*/||' | sort -u | tr '\n' ' ')
    exp_note "$(printf '   %s arm A host objects: %s' "$name" \
        "$(printf '%s' "$objs" | cut -c1-140)")"
    printf '%s' "$objs" | grep -qE 'libQt|libX11|libGL|libEGL|libgtk' && qtleak=$((qtleak+1))
    # ⭐ Q3(b): THE POSITIVE OBSERVATION. Something was spawned -- a shell or a
    # launcher -- and the exec is in the trace. ⛔ Delivery rule 4: "no Qt
    # library was loaded" is an absence, and an absence needs a presence
    # beside it or it is equally consistent with the subject never starting.
    if grep -aqE 'execve\("[^"]*(dbus-launch|dbus-daemon|/sh|/bash|/dash|/busybox)"' \
         "$WORK/tr.A.$name" 2>/dev/null; then
      dirty_spawn=$((dirty_spawn+1))
    else
      exp_note "$(printf '   ⛔ %s is dirty but spawned NO shell or launcher' "$name")"
    fi
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
    n=$(exp_count . "$WORK/exec.$arm.$name")
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
exp_check "Q2  ⛔ /bin/sh is DYNAMIC on all $rows (refutes the musl story)" \
                                                                   "$dyn_sh"  "$rows"
exp_check "Q3a ⛔ no row leaks a Qt/X11/GL host object"             "$qtleak"  0
exp_check "Q3b ⭐ every dirty row spawned a shell or launcher"      "$dirty_spawn" "$dirty"
# ⛔ Q4 IS CHECKED AGAINST THE ROWS ARM B ACTUALLY RAN ON, and when it ran on
# NONE it is a SKIP rather than a zero. A control that was never staged and a
# control that was staged and stayed dirty are different results, and scoring
# the first as a failure of the second is the shape delivery rule 4 forbids.
if [ "$b_rows" = 0 ]; then
  exp_skip "Q4  ⭐ arm B is clean on all rows" "arm B never ran (not built or not staged)"
  exp_skip "Q4  arm B still opens a window"    "arm B never ran"
else
  exp_check "Q4  ⭐ arm B is clean on all $b_rows"          "$b_clean" "$b_rows"
  exp_check "Q4  arm B still opens a window on all $b_rows" "$b_win"   "$b_rows"
fi

exp_note "⭐ MEASURED 2026-09-04c: Q1/Q2/Q3 green, ⛔ Q4 RED, and the red one is"
exp_note "   the finding. qalculate-qt is clean on exactly the four musl rows;"
exp_note "   the seven glibc rows load 1 to 4 host objects and EVERY one of"
exp_note "   them belongs to the SHELL the environment ships:"
exp_note "     debian-11/12, ubuntu-20.04   1   dash  -> libc"
exp_note "     fedora-42                    2"
exp_note "     rockylinux-8, arch           3   bash  -> libc, libdl, libtinfo"
exp_note "     opensuse-leap-15.6           4"
exp_note "   ⭐ So the count tracks WHICH SHELL the host has, not the bundle."
exp_note "⛔ AND THE MECHANISM IN THIS FILE'S HEADER WAS WRONG. It is NOT the"
exp_note "   DBus autolaunch: arm B carries dbus-daemon AND dbus-launch and is"
exp_note "   clean on the SAME four rows. The trace names the real one --"
exp_note "     execve(\"/bin/sh\", [\"sh\", \"-c\", \"--\","
exp_note "             \"/nix/store/b5c8ki47…-gnuplot-6.0.5/…\"])"
exp_note "   qalculate-qt probes for GNUPLOT through a shell."
exp_note "⭐ WHY THE FOUR musl ROWS ARE CLEAN, and it is not static busybox:"
exp_note "   on musl that exec never completes -- the child goes straight to"
exp_note "   +++ exited with 127 +++ having loaded NOTHING. On glibc the shell"
exp_note "   RUNS, loads the host libc, and only then fails to find the path."
exp_note "⭐ SO THE ROW IS EXPLAINED AND THE BUNDLE IS NOT LEAKING (Q3a: zero"
exp_note "   Qt/X11/GL host objects on every row). The residue is a HOST SHELL"
exp_note "   the application asked for and the bundle never carried."
exp_note "⚠ WHAT WOULD ACTUALLY CLOSE IT is not a path fix: an application that"
exp_note "   shells out to the host loads the host libc through that shell, and"
exp_note "   no rewriting prevents it. The bundle would have to CARRY a shell"
exp_note "   and be found first. Not implemented, not measured."
exp_note "⚠ ONE subject. It explains this row; it does not revalidate the"
exp_note "   other twenty-five, and it does not say a bundle can never leak."

exp_finish
