#!/bin/sh
# THE QUESTION
#
#   Does a GUI application bundled out of a nixpkgs closure by
#   `pgb bundle appimage` actually DRAW A WINDOW on all eleven environments —
#   including the two shapes that could not, and which T-081 is about?
#
# ⛔ WHY THIS EXPERIMENT EXISTS, AND IT IS THE OPERATOR'S OWN COUNTER-EXAMPLE.
# `Anylinux-AppImages`' `HALL-OF-FAME.md` grades **Python "Utter garbage"** and
# **GTK "Garbage"**. The operator's answer, 2026-09-03d:
#
#   *"in nixappimage for instance, python is easy and works, choose any python
#    gui app and it works"*
#
# ⭐ AND THE GRADES ARE NOT OURS TO QUOTE. They were earned deploying **Arch
# packages** through `quick-sharun`, where a library's data files and plugin
# directories must be discovered by hand. A nix closure is the opposite: it is
# the exact set the derivation declared. ⛔ T-080's rule is that a row not run
# through `pgb bundle appimage` is a **HYPOTHESIS**. This experiment converts
# them into measurements, or fails trying.
#
# -- ⛔ THE SUCCESS CRITERION, AND THE FIRST ONE WAS WRONG --------------------
#
# ⛔ AN EARLIER VERSION OF THIS EXPERIMENT SCORED A ROW GREEN WHEN THE PROGRAM
# PRINTED `Gtk-WARNING **: cannot open display:`, on the reasoning that the
# message is emitted BY the bundled libgtk-3 and therefore proves it loaded.
#
# ⚠ THE OPERATOR REJECTED THAT, AND WAS RIGHT: *"previously nixappimage bundled
# apps showed the same error on real hw with display"*. The message is NOT
# discriminating. It looks identical when there is genuinely no display and
# when the bundle's own X client stack cannot connect, so it cannot tell a
# working bundle from a broken one — which is the only thing this experiment
# is for.
#
# ⭐ SO THE DISPLAY IS REAL. An `Xvfb` server is started on this host, its
# socket is bound into each rootfs, and the criterion is EXTERNAL:
#
#   1. the program must NOT print `cannot open display` — it connected;
#   2. ⭐ **a window must appear on the X server**, checked with `xwininfo
#      -root -tree` from OUTSIDE the process. A program's own stdout cannot
#      be the evidence that it drew something.
#
# -- ⭐ THE FOUR ARMS, AND WHY EACH ONE IS HERE ------------------------------
#
#   G  galculator  C + GTK 3, UI loaded from a FILE at a compiled-in absolute
#                  store path. ⛔ THE SUBJECT T-081 EXISTS FOR: it drew 0 of 11
#                  before the store-path mechanism.
#   X  mousepad    C + GTK 3, UI compiled IN as a GResource. The regression
#                  control: it drew 11 of 11 before and must still.
#   P  meld        Python 3 + GTK 3 through PyGObject. ⛔ IT DID NOT BUILD AT
#                  ALL before the script entry point resolved to
#                  interpreter + script.
#   N  galculator, built again with `--no-storefix`. ⭐ THE NEGATIVE CONTROL.
#
# ⛔ ARM N IS DELIVERY RULE 6 MADE INTO A FLAG. "Check that your success
# criterion can fail for the right reason" is not satisfied by a comment; arm N
# builds the SAME bundle with the one mechanism absent and nothing else
# changed. If arm N also drew windows, arm G's green would be measuring
# something other than the mechanism.
#
# ⚠ ARM C — the old positive control, which bound the AppDir at the store path
# with a mount namespace — IS RETIRED. It answered "is the store path the
# cause"; arm G answering 11 of 11 with no bind answers it better, and arm N is
# what keeps the instrument honest now. `history/corrections.md`.
#
# -- ⭐ PRE-REGISTERED EXPECTATION -------------------------------------------
#
# ⛔ COMMITTED BEFORE THE RUN. PROGRESS.md delivery rule 1.
#
#   E1  arm G draws a real window on 11 of 11, WITH NO BIND.
#   E2  arm G loads zero HOST shared objects on 11 of 11.
#   E3  ⭐ arm N draws 0 of 11 — the criterion still fails without the
#       mechanism, so E1 is measuring the mechanism.
#   E4  arm P (meld, Python GUI) produces an artefact AND draws on 11 of 11.
#       ⚠ Its window budget is PGB_EXP64_PY_WIN_WAIT (150s by default) rather
#       than the 25s the other arms use — see the deadlock note below. A
#       longer budget can only make a NEGATIVE result stronger, and arm N,
#       which must draw NOTHING, keeps the short one.
#   E5  arm X still draws 11 of 11 — no regression from the changes E1 needed.
#   E6  no target ships galculator, mousepad or meld of its own (the control).
#
# -- ⛔ AND THE INSTRUMENT DEADLOCKED ONCE, WHICH IS WHY THE ORDER MATTERS ----
#
# ⛔ Measured 2026-09-03f, on arm P's first row, after arms G, N and X had each
# completed 11 rows: `strace` sat in state **D** on `folio_wait_bit_common`
# for nineteen minutes. The dwarfs FUSE daemon it was reading through sat in
# `futex_do_wait`; the traced `python3` sat in `ptrace_stop`. strace was
# blocked on a page the FUSE daemon could only serve by making progress the
# ptrace-stopped process could not make.
#
# ⚠ `kill` CANNOT END A PROCESS IN D, so `wait` never returned and the run
# stopped dead. ⭐ The fix is an ORDERING: `reap_in_root` kills the FUSE
# daemon, the blocked read fails, and only then can strace be reaped. It now
# runs BEFORE `wait` instead of after.
#
# ⛔ AND THE ORDERING FIX WAS NOT ENOUGH — THE SECOND RUN DEADLOCKED AGAIN,
# with the daemon now in `ptrace_stop` rather than `futex_do_wait`, which names
# the cause exactly: **strace had STOPPED the FUSE daemon itself**. strace
# reads a path argument out of the tracee's address space; that page is backed
# by the dwarfs mount; the only process that can serve it is the daemon strace
# is holding stopped. ⚠ `reap_in_root` still recovers the row, but only after
# the whole window budget has expired with the subject frozen — so the row
# reads 0 and the reason is the instrument.
#
# ⭐ SO ARM P RUNS IN **EXTRACT** MODE (`APPIMAGE_EXTRACT_AND_RUN=1`), which
# removes the daemon from the picture: uruntime unpacks to a tmpfs and runs
# from there. Mount and extract are two DELIVERY modes of the same artefact and
# neither criterion depends on which is used — the window is the X server's
# fact, the host objects are the process tree's. ⚠ The arms that do not
# deadlock keep mount mode, so their rows are unchanged and comparable with
# every earlier run.
#
# ⚠ AND THE SUBJECT MATTERS FOR TIME TOO: a Python interpreter importing its
# whole stack through ptrace is orders of magnitude slower than a C program
# starting, which is why arm P's window budget is separate and long. A budget
# that is too short scores a working bundle as a broken one, which is this
# experiment's own original sin in a new place.
#
# -- THE CONTROL -------------------------------------------------------------
#
# ⭐ WITHOUT IT A GREEN ROW IS A PROGRAM PRINTING A STRING. The control is that
# the target has none of these programs of its own: `command -v <prog>` must
# fail on every one of the eleven, so the capability demonstrably came out of
# the bundle.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "64 - GTK and Python out of a nixpkgs closure, with the compiled-in store path resolved"

WORK="${PGB_EXP64_WORK:-/var/tmp/t080}"
mkdir -p "$WORK" || exit 2
GIMG="$WORK/galculator.AppImage"
XIMG="$WORK/mousepad.AppImage"
NIMG="$WORK/galculator-nofix.AppImage"
PYIMG="$WORK/meld.AppImage"
RUN_TIMEOUT="${PGB_EXP64_TIMEOUT:-120}"
WIN_WAIT="${PGB_EXP64_WIN_WAIT:-25}"   # seconds to wait for a window to appear

command -v strace >/dev/null 2>&1 || { exp_note "no strace on PATH"; exit 2; }

# ⛔ AN ARTEFACT OLDER THAN THE TOOL IS NOT A RESULT ABOUT THE TOOL, and this
# is docs/AGENTS.md §6's `--rebuild` lesson reached from the other side: the
# POC suite once reported ten green rows about a toolchain that had changed
# under it. Here the check is derived rather than remembered.
stale() { # image -> 0 when it must be rebuilt
  [ ! -s "$1" ] && return 0
  [ "$REPO_DIR/pgb" -nt "$1" ] && return 0
  return 1
}

build_arm() { # image attribute name [extra pgb args...]
  _img=$1; _attr=$2; _name=$3; shift 3
  if stale "$_img"; then
    exp_note "building $_name — several minutes"
    rm -f "$_img"
    PGB_APPIMAGE_CACHE="$WORK/${_name}cache" "$REPO_DIR/pgb" bundle appimage "$_attr" \
      --out "$_img" --name "$_attr" "$@" >"$WORK/build-$_name.log" 2>&1 || true
  fi
}

# ---------------------------------------------------------------------------
# ⭐ A REAL DISPLAY, BECAUSE "cannot open display" IS NOT A RESULT.
#
# ⛔ Xvfb IS REQUIRED, NOT OPTIONAL, AND ITS ABSENCE IS exit 2 RATHER THAN A
# GREEN RUN WITHOUT IT. Without a display this experiment can only observe
# that GTK loaded, which is precisely the non-discriminating measurement the
# operator rejected. `-ac` disables access control so a chrooted client with
# no Xauthority can connect.
# ---------------------------------------------------------------------------
XDISP="${PGB_EXP64_DISPLAY:-:99}"
for t in Xvfb xwininfo; do
  command -v "$t" >/dev/null 2>&1 || {
    exp_note "no $t on PATH — Debian/Ubuntu: apt-get install xvfb x11-utils"
    exp_note "⛔ this experiment CANNOT run without a real display: the whole"
    exp_note "   point is that 'cannot open display' does not discriminate."
    exit 2; }
done
XVFB_PID=""
if ! DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1; then
  Xvfb "$XDISP" -ac -screen 0 1024x768x24 >"$WORK/xvfb.log" 2>&1 &
  XVFB_PID=$!
  _w=0
  while [ "$_w" -lt 20 ]; do
    DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1 && break
    _w=$((_w+1)); sleep 1
  done
fi
DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1 || {
  exp_note "Xvfb did not come up on $XDISP; see $WORK/xvfb.log"; exit 2; }
trap '[ -n "$XVFB_PID" ] && kill "$XVFB_PID" 2>/dev/null' EXIT INT TERM
exp_note "display: $XDISP ($(DISPLAY=$XDISP xdpyinfo | sed -n 's/^vendor string: *//p' | head -1))"

# ⭐ THE EXTERNAL OBSERVER. A program's own output cannot be the evidence that
# it drew something; the X server's window tree can.
windows_named() {  # name -> count of matching toplevel windows
  DISPLAY="$XDISP" xwininfo -root -tree 2>/dev/null \
    | grep -ci "$1" || true
}

# ⛔ THE DISPLAY MUST BE IDLE BEFORE A ROW IS SCORED. A window another arm left
# behind is counted by the observer and nothing else in this harness would
# catch it — RESUME.md carries this as a standing rule about the shared
# resource, and here it is asserted instead of remembered.
display_idle() { # name -> 0 when no window of that name is on the server
  [ "$(windows_named "$1")" = 0 ]
}

# ⛔ REAP BY WHAT A PROCESS IS CHROOTED INTO, NOT BY ITS NAME. uruntime leaves
# a dwarfs FUSE daemon behind on purpose — a mount that outlives the program is
# what mount mode IS — and its comm is `memfd:dwarfs`, not the artefact's.
# `pkill -f` is worse: the rootfs path is in the RUNNER's own command line, so
# a full-command-line match kills this script. docs/AGENTS.md §14.
reap_in_root() { # rootfs-path
  _rr=$1
  for _p in /proc/[0-9]*; do
    _pid=${_p#/proc/}
    _rt=$(readlink "/proc/$_pid/root" 2>/dev/null) || continue
    case "$_rt" in "$_rr"|"$_rr"/*) kill -9 "$_pid" 2>/dev/null ;; esac
  done
}

# ⭐ THE CLASSIFIER IS experiments/lib.sh's `exp_classify_trace`, NOT A COPY.
# ⛔ Nine experiments carried the same awk by hand and they could not be
# corrected together: 2026-09-03f found that a split `openat( ... <unfinished
# ...>` carries the PATH and no result, so a filter dropping lines that contain
# ENOENT keeps the first half of a FAILED open and counts it as a load. A
# galculator bundle read 2 host shared objects on alpine-3.22 and both were
# ENOENT probes. One implementation is what stops that from having to be found
# nine times. docs/history/corrections.md.

ENVS=$(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt")

# ⭐ ONE MATRIX, RUN PER SUBJECT. Every application goes through exactly the
# same instrument so the only thing that differs between arms is the arm.
# ⚠ THE WINDOW BUDGET IS PER ARM, and that is a statement rather than a
# convenience. The criterion is "a window appeared"; the budget is how long we
# waited for one. A LONGER budget can only make a NEGATIVE result stronger, so
# arm N — which must draw nothing — keeps the short one, and arm P, whose
# subject is a Python interpreter importing its whole stack THROUGH ptrace,
# gets a long one. Arms G and X draw in about two seconds and the loop breaks
# early, so their budget is not what their rows cost.
run_matrix() {  # window-name image-path program-name [window-budget] [extract]
  SUBJ="$1"; IMG="$2"; PROG="${3:-$1}"; WAIT_FOR="${4:-$WIN_WAIT}"
  # ⛔ EXTRACT MODE IS NOT A PREFERENCE, IT IS THE ONLY WAY TO TRACE THIS ONE.
  # See the deadlock note above: `strace` reads a path argument out of the
  # tracee's address space, that page is backed by the dwarfs FUSE mount, and
  # strace has ptrace-STOPPED the FUSE daemon that would have to serve it.
  # ⭐ `APPIMAGE_EXTRACT_AND_RUN=1` removes the daemon from the picture
  # entirely — uruntime unpacks to a tmpfs and runs from there — so there is no
  # process for strace to stop and then wait on.
  # ⚠ Mount and extract are two DELIVERY modes of the same artefact. Neither
  # criterion here depends on which is used: the window is the X server's fact
  # and the host objects are the process tree's. The arms that do NOT deadlock
  # keep mount mode so their rows are unchanged.
  EXTRACT="${5:-}"
  RAN=0; CLEAN=0; GTK=0; NOHOST=0; ROWS=0; WIN=0

printf '\n'
printf '  %-20s %-6s %-5s %-6s %-8s %-7s %s\n' \
  ENVIRONMENT LIBC CONN 'WINDOW' 'HOST.so' 'libgtk3' 'BUNDLED .so'
for name in $ENVS; do
  root=$(exp_rootfs "$name") || true
  [ -n "$root" ] || { exp_skip "$name" "not fetched"; continue; }
  ROWS=$((ROWS+1))
  libc=$(exp_rootfs_libc "$name")

  # ⭐ THE CONTROL: the target must not have this program of its own, or a
  # green row says nothing about the bundle.
  own=$("$REPO_DIR/pgb" rootfs run "$root" -- /bin/sh -c "command -v $PROG" 2>/dev/null | head -1)
  [ -z "$own" ] && NOHOST=$((NOHOST+1))

  # ⛔ AND THE SERVER MUST BE EMPTY BEFORE THE SUBJECT STARTS.
  _q=0
  while [ "$_q" -lt 10 ] && ! display_idle "$SUBJ"; do sleep 1; _q=$((_q+1)); done
  display_idle "$SUBJ" || exp_note "⚠ $name: a $SUBJ window was ALREADY on $XDISP"

  rm -f "$root/subj64"; cp "$IMG" "$root/subj64" 2>/dev/null; chmod +x "$root/subj64"
  tr="$WORK/tr.$name"

  # ⛔ THE X SOCKET MUST BE BOUND IN. `pgb rootfs run` mounts a FRESH TMPFS
  # over /tmp, so /tmp/.X11-unix vanishes unless it is bound explicitly.
  # RESUME.md carries this as a standing machine note.
  #
  # ⚠ RUN IT IN THE BACKGROUND AND LOOK AT THE X SERVER WHILE IT IS ALIVE. A
  # GUI program that succeeds does NOT exit — it enters its event loop — so
  # waiting for it to finish and then asking about windows would find none
  # either way, and would score a working bundle exactly like a broken one.
  strace -f -e trace=openat,open,execve,clone,clone3,vfork -o "$tr" \
    timeout "$RUN_TIMEOUT" "$REPO_DIR/pgb" rootfs run "$root" \
      --bind /tmp/.X11-unix:/tmp/.X11-unix -- \
      /bin/sh -c "DISPLAY=$XDISP ${EXTRACT:+APPIMAGE_EXTRACT_AND_RUN=1 }/subj64" \
    >"$WORK/out.$name" 2>"$WORK/err.$name" &
  _sp=$!
  _n=0; win=0
  while [ "$_n" -lt "$WAIT_FOR" ]; do
    sleep 1; _n=$((_n+1))
    win=$(windows_named "$SUBJ")
    [ "$win" -gt 0 ] && break
    kill -0 "$_sp" 2>/dev/null || break
  done
  # ⛔ REAP BEFORE `wait`, AND THE ORDER IS THE WHOLE OF A DEADLOCK.
  #
  # Measured 2026-09-03f on arm P: `strace` sat in state D on
  # `folio_wait_bit_common` for nineteen minutes, the dwarfs FUSE daemon it
  # was reading through sat in `futex_do_wait`, and the traced `python3` sat
  # in `ptrace_stop`. strace was blocked reading a page the FUSE daemon could
  # only serve by making progress the ptrace-stopped process could not make.
  # ⛔ `kill` cannot end a process in D, so `wait` never returned and the
  # experiment hung — after arms G, N and X had each run 11 rows without it.
  #
  # ⭐ `reap_in_root` kills the FUSE daemon (it is chrooted into the same
  # rootfs, which is why it reaps by /proc/PID/root rather than by name), the
  # blocked read fails, and strace becomes reapable. Doing that BEFORE `wait`
  # is the fix; doing it after is where the run stopped.
  kill "$_sp" 2>/dev/null
  reap_in_root "$root"
  wait "$_sp" 2>/dev/null
  st=$?
  rm -f "$root/subj64"

  cls=$(exp_classify_trace "$tr" /subj64)
  nhost=$(printf '%s\n' "$cls" | grep -c '^host ' || true)
  nbund=$(printf '%s\n' "$cls" | grep -c '^bundled ' || true)
  hasgtk=$(printf '%s\n' "$cls" | grep -c '^bundled .*libgtk-3' || true)

  # ⛔ TWO DIFFERENT QUESTIONS, AND CONFLATING THEM IS THE ERROR THIS
  # EXPERIMENT WAS CORRECTED FOR.
  #   CONNECTED  the bundled GTK reached the X server — no "cannot open
  #              display". This is necessary and NOWHERE NEAR sufficient.
  #   WINDOW     the X server actually has a window, observed from outside.
  # ⛔ `tr -d '\r' < f1 f2` REDIRECTS f1 AND PASSES f2 AS AN ARGUMENT, which
  # tr rejects — so this read nothing and every row printed "<none>" while the
  # trace beside it plainly showed GTK loading. `cat` both, THEN filter.
  all=$(cat "$WORK/err.$name" "$WORK/out.$name" 2>/dev/null | tr -d '\r')
  if printf '%s' "$all" | grep -q 'cannot open display'; then
    runs=no
  else
    runs=yes; RAN=$((RAN+1))
  fi
  [ "$win" -gt 0 ] && WIN=$((WIN+1))
  out=$(printf '%s' "$all" | grep -m1 "Couldn't load\|cannot open display\|Traceback\|error\|Error" || printf '%s' "$all" | head -1)
  [ "$nhost" = 0 ] && CLEAN=$((CLEAN+1))
  [ "$hasgtk" -gt 0 ] && GTK=$((GTK+1))

  printf '  %-20s %-6s %-5s %-8s %-8s %-8s %s\n' \
    "$name" "$libc" "$runs" "$win" "$nhost" \
    "$([ "$hasgtk" -gt 0 ] && echo yes || echo no)" "$nbund"
  printf '      out: %s  [exit %s]\n' "${out:-<none>}" "$st"
done

printf '\n'
}

# ---------------------------------------------------------------------------
# ARM G — galculator: GTK 3, and its UI is a FILE reached by a compiled-in path
# ---------------------------------------------------------------------------
printf '\n-- arm G: galculator (UI loaded from a file at a compiled-in path) --\n'
build_arm "$GIMG" galculator gal
[ -s "$GIMG" ] || { exp_note "arm G did not build; see $WORK/build-gal.log"; exit 2; }
exp_note "artefact: $GIMG, $(wc -c < "$GIMG") bytes"
G_STOREMAP=$(tr -d '\000' < "$WORK/build-gal.log" 2>/dev/null \
  | sed -n 's/^store map *\([0-9]*\) .*/\1/p' | head -1)
exp_note "store map: ${G_STOREMAP:-unknown} store paths resolve inside the bundle"
run_matrix galculator "$GIMG"
G_ROWS=$ROWS; G_CONN=$RAN; G_WIN=$WIN; G_CLEAN=$CLEAN; G_GTK=$GTK; G_NOHOST=$NOHOST

# ---------------------------------------------------------------------------
# ⭐ ARM N — THE NEGATIVE CONTROL. Same subject, same bundler, ONE mechanism
# removed by a shipped flag.
# ---------------------------------------------------------------------------
printf '\n-- arm N: galculator with --no-storefix (the NEGATIVE CONTROL) -----\n'
build_arm "$NIMG" galculator nofix --no-storefix
if [ -s "$NIMG" ]; then
  run_matrix galculator "$NIMG"
  N_ROWS=$ROWS; N_WIN=$WIN
else
  exp_skip "arm N (--no-storefix)" "the bundle did not build; see $WORK/build-nofix.log"
  N_ROWS=0; N_WIN=-1
fi

# ---------------------------------------------------------------------------
# ARM X — mousepad: GTK 3, and its UI is a GResource COMPILED INTO the binary
# ---------------------------------------------------------------------------
printf '\n-- arm X: mousepad (UI compiled into the binary as a GResource) ----\n'
build_arm "$XIMG" mousepad mousepad
if [ -s "$XIMG" ]; then
  run_matrix mousepad "$XIMG"
  X_ROWS=$ROWS; X_CONN=$RAN; X_WIN=$WIN; X_CLEAN=$CLEAN; X_GTK=$GTK; X_NOHOST=$NOHOST
else
  exp_skip "arm X (mousepad)" "the bundle did not build; see $WORK/build-mousepad.log"
  X_ROWS=0; X_CONN=0; X_WIN=0; X_CLEAN=0; X_GTK=0; X_NOHOST=0
fi

# ---------------------------------------------------------------------------
# ARM P — meld: a PYTHON 3 + GTK 3 application. ⛔ It did not build at all
# before a script entry point resolved to interpreter + script.
# ---------------------------------------------------------------------------
printf '\n-- arm P: a PYTHON GUI application (meld) --------------------------\n'
build_arm "$PYIMG" meld meld
# ⚠ `-a`/`-a`-equivalent: a build log carries bytes `grep` and `sed` call
# binary, and without it both of these printed nothing while the log plainly
# said otherwise. It cost two notes, not a measurement — but a note that says
# "<not a script entry>" about a bundle whose log says
# "script meld = python3 + ... (static trampoline)" is worse than no note.
P_CLOSURE=$(tr -d '\000' < "$WORK/build-meld.log" 2>/dev/null \
  | sed -n 's/^closure *\([0-9]*\) store paths after augmentation.*/\1/p' | head -1)
exp_note "arm P closure: ${P_CLOSURE:-unknown} store paths"
exp_check "arm P: an artefact was produced" "$([ -s "$PYIMG" ] && echo yes || echo no)" yes
if [ -s "$PYIMG" ]; then
  P_ENTRY=$(grep -a -m1 'static trampoline' "$WORK/build-meld.log" 2>/dev/null || true)
  exp_note "arm P entry: ${P_ENTRY:-<not a script entry>}"
  run_matrix meld "$PYIMG" meld "${PGB_EXP64_PY_WIN_WAIT:-150}" extract
  P_ROWS=$ROWS; P_CONN=$RAN; P_WIN=$WIN; P_CLEAN=$CLEAN; P_NOHOST=$NOHOST
else
  exp_skip "arm P (meld)" "the bundle did not build; see $WORK/build-meld.log"
  P_ROWS=0; P_CONN=0; P_WIN=0; P_CLEAN=0; P_NOHOST=0
fi

printf '\n'
printf -- '-- summary ---------------------------------------------------------\n'
printf '  %-30s %10s %10s %10s\n' AXIS 'G galc' 'X mousep' 'P meld'
printf '  %-30s %10s %10s %10s\n' 'rows measured'               "$G_ROWS"  "$X_ROWS"  "$P_ROWS"
printf '  %-30s %10s %10s %10s\n' 'connected to a real X'       "$G_CONN"  "$X_CONN"  "$P_CONN"
printf '  %-30s %10s %10s %10s\n' 'zero HOST shared objects'    "$G_CLEAN" "$X_CLEAN" "$P_CLEAN"
printf '  %-30s %10s %10s %10s\n' '⭐ A WINDOW ON THE X SERVER'  "$G_WIN"   "$X_WIN"   "$P_WIN"
printf '\n  %-30s %10s\n' '⭐ arm N: --no-storefix windows' "$N_WIN of $N_ROWS"

printf '\n'
exp_check "control: no target ships these programs" \
  "$((G_NOHOST + X_NOHOST + P_NOHOST))" "$((G_ROWS + X_ROWS + P_ROWS))"
exp_check "G: GTK connected to a real display"           "$G_CONN"  "$G_ROWS"
exp_check "G: libgtk-3 loaded FROM THE BUNDLE"           "$G_GTK"   "$G_ROWS"
exp_check "E2  G: host shared objects, rows with zero"   "$G_CLEAN" "$G_ROWS"
exp_check "E1  ⭐ G: a REAL WINDOW, with NO bind"         "$G_WIN"   "$G_ROWS"
exp_check "E3  ⭐ N: --no-storefix still draws NOTHING"   "$N_WIN"   0
exp_check "E5  X: a REAL WINDOW on the X server"         "$X_WIN"   "$X_ROWS"
exp_check "E5  X: host shared objects, rows with zero"   "$X_CLEAN" "$X_ROWS"
exp_check "E4  ⭐ P: a PYTHON GUI drew a REAL WINDOW"     "$P_WIN"   "$P_ROWS"
exp_check "E4  P: host shared objects, rows with zero"   "$P_CLEAN" "$P_ROWS"

exp_note "⭐ WHAT E1 AND E3 SAY TOGETHER. Arm G is the artefact whose UI is a"
exp_note "   file at /nix/store/<hash>-galculator-2.1.4/share/... — a path that"
exp_note "   does not exist on any of the eleven. Arm N is the SAME bundle with"
exp_note "   --no-storefix. If G draws and N does not, the mechanism is what"
exp_note "   drew it, and no reading of an error message is involved."
exp_note "⛔ THE MECHANISM IS NOT A /tmp STORE. docs/design/store-paths.md §2"
exp_note "   answers the security question the operator required first: a"
exp_note "   fixed path under a world-writable directory is squattable, and the"
exp_note "   tree it would serve is loadable code. It is an interposer, and the"
exp_note "   rewrite is exact-match against the closure pgb already computes."
exp_note "⚠ NOT ESTABLISHED HERE: anything about a GPU (every GL row is swrast,"
exp_note "   T-059 owns hardware), and anything about a STATICALLY linked or"
exp_note "   raw-syscall subject, which has no PLT for an interposer to win."

exp_finish
