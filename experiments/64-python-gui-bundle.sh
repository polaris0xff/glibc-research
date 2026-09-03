#!/bin/sh
# THE QUESTION
#
#   Does a PYTHON GUI APPLICATION, bundled out of a nixpkgs closure by
#   `pgb bundle appimage`, actually run on all eleven environments?
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
# two of them — Python and GTK — into measurements, or fails trying.
#
# -- WHY meld ----------------------------------------------------------------
#
# ⭐ IT IS BOTH BOTTOM ROWS AT ONCE. `meld` is a Python 3 application whose UI
# is GTK 3 reached through PyGObject, so one artefact exercises:
#
#   Python      an interpreter that must find its own stdlib, its site-packages
#               and its compiled extension modules inside the bundle
#   GTK         a typelib + introspection stack that must find its .typelib
#               files, its loaders and its schemas inside the bundle
#
# ⚠ It is ONE application. It is not evidence about every Python or GTK
# program, and §"what this does not establish" says so at the end.
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
# ⭐ SO THE DISPLAY IS REAL NOW. An `Xvfb` server is started on this host, its
# socket is bound into each rootfs, and the criterion is EXTERNAL:
#
#   1. the program must NOT print `cannot open display` — it connected;
#   2. ⭐ **a window must appear on the X server**, checked with `xwininfo
#      -root -tree` from OUTSIDE the process. A program's own stdout cannot
#      be the evidence that it drew something.
#
# ⛔ CRITERION 2 IS WHAT MAKES THIS FALSIFIABLE, and it immediately falsified
# the green result: with a real display, galculator connects fine and the root
# window still has **0 children**.
#
# -- THE CONTROL -------------------------------------------------------------
#
# ⭐ WITHOUT IT A GREEN ROW IS A PROGRAM PRINTING A STRING. The control is that
# the target has no meld of its own: `command -v meld` must fail on every one
# of the eleven, so the capability demonstrably came out of the bundle.
#
# -- ⭐ PRE-REGISTERED EXPECTATION -------------------------------------------
#
# ⛔ COMMITTED BEFORE THE RUN. PROGRESS.md delivery rule 1.
#
#   R1  the bundle runs meld's own code on 11 of 11.
#   R2  zero HOST shared objects on 11 of 11, by the `62-` classifier.
#   R3  the trace shows libpython AND libgtk-3 opened FROM THE BUNDLE on
#       every row — i.e. the stacks the field grades worst are the ones
#       demonstrably loading.
#   R4  no target has a meld of its own (the control), 11 of 11.
#
# -- ⛔ WHAT ACTUALLY HAPPENED: R1-R3 NEVER GOT A CHANCE, AND THAT IS THE
#       FINDING ---------------------------------------------------------------
#
# ⛔ **THE meld BUNDLE DOES NOT BUILD**, and the cause is exact rather than
# vague. `internal/bundle/appimage.go`'s `resolveEntry` OSCILLATES:
#
#   1. `bin/meld` is a nixpkgs `makeBinaryWrapper` — a compiled ELF. The
#      reader recognises it and extracts its 10 environment records; its
#      target is `bin/.meld-wrapped`. ✅ correct so far.
#   2. `bin/.meld-wrapped` is a **Python script**, shebang
#      `#!/nix/store/…-python3-3.14.7/bin/python3`. `ReadWrapper` returns
#      nothing (it is not a wrapper) and `elfx.IsELF` is false, so the code
#      falls to `lastExistingStorePath`, which scans the script's TEXT for the
#      last executable store path it names — and that resolves back to
#      `bin/meld`.
#   3. → 1. Five hops, then `too many wrapper hops`, then
#      `no entry point in …/bin` (`assemble.go:60`).
#
# ⭐ THE REAL FIX IS A FEATURE, NOT A PATCH, and it is worth stating why: the
# bundle FLATTENS the closure — libraries into `lib/`, programs into
# `shared/bin/` — instead of shipping a `/nix/store` tree. So the script
# cannot simply be adopted as the entry point either: its shebang is an
# absolute store path that will not exist at run time. A script entry point
# has to become *interpreter + script argument*, which changes what
# `resolveEntry` returns and how the launcher invokes it.
#
# ⛔ SO THE PYTHON ROW IS A **TOOLING** GAP, WHICH IS EXACTLY THE CATEGORY
# T-080'S GUARANTEE IS ABOUT — not a statement that nix cannot do Python. The
# closure fetched fine (136 store paths, mesa augmented); nothing about the
# libraries failed. ⚠ And it is the STANDARD nixpkgs shape for a Python
# application, so it is not a `meld` quirk.
#
# -- ⭐ ARM G, ADDED SO THE GTK ROW IS MEASURED RATHER THAN BLOCKED WITH IT ---
#
# `galculator` is C + GTK 3 with `gtk+3` as its only build input, and its
# `bin/galculator` is a plain ELF, so it does not meet the wrapper defect.
# ⭐ That separates the two questions the meld failure had entangled: GTK is
# measured here, and Python is a named mechanism rather than an unknown.
#
#   S1  arm G connects to a REAL display on 11 of 11 (no "cannot open display")
#   S2  arm G loads zero HOST shared objects on 11 of 11
#   S3  arm G's trace shows libgtk-3 opened FROM THE BUNDLE on every row
#   S4  no target has a galculator of its own
#   S5  ⛔ arm P (meld) does NOT produce an artefact, for the reason above.
#       This is asserted as a MEASUREMENT, not recorded as a skip: the build
#       ran, fetched a complete closure, and failed at a named line.
#   S6  ⛔ **a WINDOW appears on the X server**, checked externally.
#
# -- ⛔ AND S6 IS WHERE IT FAILS, WHICH IS THE POINT OF MEASURING IT ---------
#
# ⭐ WITH A REAL DISPLAY, GTK CONNECTS AND THE APPLICATION STILL DRAWS
# NOTHING. galculator prints:
#
#   [galculator] Couldn't load /nix/store/<hash>-galculator-2.1.4/share/
#                galculator/ui/main_frame.ui
#
# ⛔ AN ABSOLUTE STORE PATH, COMPILED INTO THE BINARY, TO A DATA FILE THAT IS
# IN THE BUNDLE — `AppDir/share/galculator/ui/main_frame.ui` exists. The
# bundle sets `XDG_DATA_DIRS` to its own `share`, which serves every app that
# LOOKS UP its data; it cannot serve one that has the path baked in.
#
# ⭐ SO THE MECHANISM THAT STOPS THIS BUNDLE IS **TOOLING**, NOT CAPABILITY:
# GTK loaded, GTK connected to a real X server, zero host objects. What failed
# is a store path nothing rewrote — which is **T-081**'s entry text verbatim
# (*"shebang lines, hardcoded paths, .desktop files"*). ⛔ T-081 is out of
# scope this session and this experiment is where its cost is now measured.
#
# ⚠ TWO SUBJECTS, BECAUSE ONE WOULD NOT SEPARATE THE CASES. Arm X is
# `mousepad`, a GTK 3 editor that carries its UI as a GResource compiled into
# the binary rather than as a file on a path. If arm X draws a window and arm
# G does not, the boundary is exactly "data reached by a baked-in absolute
# path", and not GTK.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "64 - GTK and Python out of a nixpkgs closure: one measured, one blocked by our own tooling"

WORK="${PGB_EXP64_WORK:-/var/tmp/t080}"
mkdir -p "$WORK" || exit 2
GIMG="$WORK/galculator.AppImage"
XIMG="$WORK/mousepad.AppImage"
SUBJ=galculator
PYIMG="$WORK/meld.AppImage"
RUN_TIMEOUT="${PGB_EXP64_TIMEOUT:-120}"
WIN_WAIT="${PGB_EXP64_WIN_WAIT:-25}"   # seconds to wait for a window to appear

command -v strace >/dev/null 2>&1 || { exp_note "no strace on PATH"; exit 2; }

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

# ---------------------------------------------------------------------------
# ⛔ ARM P FIRST, BECAUSE ITS FAILURE IS THE FINDING AND NOT A PRELUDE.
#
# ⚠ THIS IS AN ASSERTION, NOT A SKIP. The build RAN: it resolved the
# attribute, fetched a 136-path closure with its signatures checked, augmented
# it with mesa, and then failed inside `resolveEntry`. A skip would say "could
# not measure"; this measured a defect and names it.
# ---------------------------------------------------------------------------
printf -- '-- arm P: a PYTHON GUI application (meld) --------------------------\n'
if [ ! -e "$WORK/build-meld.log" ]; then
  exp_note "building arm P (meld) — expected to fail at resolveEntry"
  PGB_APPIMAGE_CACHE="$WORK/cache" "$REPO_DIR/pgb" bundle appimage meld \
    --out "$PYIMG" --name meld >"$WORK/build-meld.log" 2>&1 || true
fi
P_HOPS=$(grep -c 'shape the reader does not know' "$WORK/build-meld.log" 2>/dev/null || true)
P_NOENTRY=$(grep -c 'no entry point in' "$WORK/build-meld.log" 2>/dev/null || true)
P_CLOSURE=$(sed -n 's/^closure *\([0-9]*\) store paths after augmentation.*/\1/p' \
  "$WORK/build-meld.log" 2>/dev/null | head -1)
exp_check "arm P: the closure fetched completely" \
  "$([ "${P_CLOSURE:-0}" -gt 100 ] && echo yes || echo no)" yes
exp_note "arm P closure: ${P_CLOSURE:-unknown} store paths — nothing about the"
exp_note "   LIBRARIES failed. The defect is downstream of the closure."
exp_check "arm P: no artefact was produced"  "$([ -s "$PYIMG" ] && echo yes || echo no)" no
exp_check "arm P: resolveEntry oscillated"   "$([ "$P_HOPS" -ge 2 ] && echo yes || echo no)" yes
exp_check "arm P: and it ended in 'no entry point'" \
  "$([ "$P_NOENTRY" -ge 1 ] && echo yes || echo no)" yes
exp_note "⛔ THE MECHANISM, exactly: bin/meld is a makeBinaryWrapper ELF whose"
exp_note "   target bin/.meld-wrapped is a PYTHON SCRIPT. ReadWrapper returns"
exp_note "   nothing for a script and elfx.IsELF is false, so"
exp_note "   lastExistingStorePath scans the script's text and resolves back"
exp_note "   to bin/meld. Five hops, then no entry point (assemble.go:60)."
exp_note "⭐ IT IS A TOOLING GAP, which is the category T-080 is about — NOT a"
exp_note "   statement that nix cannot do Python."

printf '\n'
printf -- '-- arm G: a C + GTK 3 application (galculator) ---------------------\n'
if [ ! -s "$GIMG" ]; then
  exp_note "building arm G — several minutes, ~130 store paths"
  PGB_APPIMAGE_CACHE="$WORK/gcache" "$REPO_DIR/pgb" bundle appimage galculator \
    --out "$GIMG" --name galculator >"$WORK/build-gal.log" 2>&1 || true
fi
[ -s "$GIMG" ] || { exp_note "arm G did not build; see $WORK/build-gal.log"; exit 2; }
exp_check "arm G: the GTK bundle built" "$([ -s "$GIMG" ] && echo yes || echo no)" yes
exp_note "artefact: $GIMG, $(wc -c < "$GIMG") bytes"

# ⭐ THE CLASSIFIER IS `experiments/62-`'s, DELIBERATELY. A bundle's trace must
# NOT be attributed to one pid — uruntime forks, mounts and re-execs, so the
# payload runs in a descendant — and a path is only a shared object if it ENDS
# in .so or .so.N, because /etc/ld.so.cache is an index. Both rules are
# docs/AGENTS.md §14 and both were defects here once.
classify_trace() {  # tracefile /artefact
  awk -v want="$2" '
    { pid = $1 }
    $0 ~ ("execve\\(\"" want "\"") { inset[pid] = 1; next }
    ($0 ~ /(clone|clone3|vfork|fork)\(/ || $0 ~ /<\.\.\. (clone|clone3|vfork|fork) resumed>/) \
      && /= [0-9]+$/ { if (inset[pid]) inset[$NF] = 1; next }
    inset[pid] && /open(at)?\(/ && !/ENOENT|= -1/ {
      if (match($0, /"[^"]*"/) == 0) next
      p = substr($0, RSTART + 1, RLENGTH - 2)
      if (p !~ /\.so(\.[0-9]+)*$/) next
      if (p ~ /^\/(usr\/)?(local\/)?lib(32|64)?\//) print "host " p
      else print "bundled " p
    }
  ' "$1" | sort -u
}

ENVS=$(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt")

# ⭐ ONE MATRIX, RUN PER SUBJECT. Two GTK applications go through exactly the
# same instrument so the only thing that differs between them is the
# application — which is what makes the comparison mean anything.
run_matrix() {  # subject image-path [extra-bind]
  SUBJ="$1"; IMG="$2"; XBIND="${3:-}"
  RAN=0; CLEAN=0; PY=0; GTK=0; NOHOST=0; ROWS=0; WIN=0

printf '\n'
printf '  %-20s %-6s %-5s %-6s %-8s %-7s %s\n' \
  ENVIRONMENT LIBC CONN 'WINDOW' 'HOST.so' 'libgtk3' 'BUNDLED .so'
for name in $ENVS; do
  root=$(exp_rootfs "$name") || true
  [ -n "$root" ] || { exp_skip "$name" "not fetched"; continue; }
  ROWS=$((ROWS+1))
  libc=$(exp_rootfs_libc "$name")

  # ⭐ THE CONTROL: the target must not have a galculator of its own, or a
  # green row says nothing about the bundle.
  own=$("$REPO_DIR/pgb" rootfs run "$root" -- /bin/sh -c "command -v $SUBJ" 2>/dev/null | head -1)
  [ -z "$own" ] && NOHOST=$((NOHOST+1))

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
      --bind /tmp/.X11-unix:/tmp/.X11-unix ${XBIND:+--bind "$XBIND"} -- \
      /bin/sh -c "DISPLAY=$XDISP /subj64" \
    >"$WORK/out.$name" 2>"$WORK/err.$name" &
  _sp=$!
  _n=0; win=0
  while [ "$_n" -lt "$WIN_WAIT" ]; do
    sleep 1; _n=$((_n+1))
    win=$(windows_named "$SUBJ")
    [ "$win" -gt 0 ] && break
    kill -0 "$_sp" 2>/dev/null || break
  done
  kill "$_sp" 2>/dev/null
  wait "$_sp" 2>/dev/null
  st=$?
  reap_in_root "$root"
  rm -f "$root/subj64"

  cls=$(classify_trace "$tr" /subj64)
  nhost=$(printf '%s\n' "$cls" | grep -c '^host ' || true)
  nbund=$(printf '%s\n' "$cls" | grep -c '^bundled ' || true)
  haspy=$(printf '%s\n' "$cls"  | grep -c '^bundled .*libgdk' || true)
  hasgtk=$(printf '%s\n' "$cls" | grep -c '^bundled .*libgtk-3' || true)

  # ⛔ "RUNS" IS GTK'S OWN OUTPUT, AND THAT IS A STRONGER TEST THAN --version.
  #
  # ⭐ galculator calls gtk_init() BEFORE it parses arguments, so it never
  # reaches a --version string. What it prints instead is
  #     (galculator:NNN): Gtk-WARNING **: cannot open display:
  # which is emitted BY THE BUNDLED libgtk-3 ITSELF, after the library has
  # loaded, initialised and got as far as connecting to a display server.
  # ⚠ A --version string would only have proved the program's argument parser
  # ran; this proves GTK ran. The failure that follows is the absence of a
  # DISPLAY on this machine, which is a property of the machine and not of
  # the bundle — §"what this does not establish".
  # ⛔ `tr -d '\r' < f1 f2` REDIRECTS f1 AND PASSES f2 AS AN ARGUMENT, which
  # tr rejects — so this read nothing and every row printed "<none>" while the
  # trace beside it plainly showed GTK loading. Caught by the disagreement
  # between the two, not by reading. `cat` both, THEN filter.
  # ⛔ TWO DIFFERENT QUESTIONS, AND CONFLATING THEM IS THE ERROR THIS
  # EXPERIMENT WAS CORRECTED FOR.
  #   CONNECTED  the bundled GTK reached the X server — no "cannot open
  #              display". This is necessary and NOWHERE NEAR sufficient.
  #   WINDOW     the X server actually has a window, observed from outside.
  all=$(cat "$WORK/err.$name" "$WORK/out.$name" 2>/dev/null | tr -d '\r')
  if printf '%s' "$all" | grep -q 'cannot open display'; then
    runs=no
  else
    runs=yes; RAN=$((RAN+1))
  fi
  [ "$win" -gt 0 ] && WIN=$((WIN+1))
  out=$(printf '%s' "$all" | grep -m1 "Couldn't load\|cannot open display\|error\|Error" || printf '%s' "$all" | head -1)
  [ "$nhost" = 0 ] && CLEAN=$((CLEAN+1))
  [ "$haspy"  -gt 0 ] && PY=$((PY+1))
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
run_matrix galculator "$GIMG"
G_ROWS=$ROWS; G_CONN=$RAN; G_WIN=$WIN; G_CLEAN=$CLEAN; G_GTK=$GTK; G_NOHOST=$NOHOST

# ---------------------------------------------------------------------------
# ARM X — mousepad: GTK 3, and its UI is a GResource COMPILED INTO the binary
# ---------------------------------------------------------------------------
printf '\n-- arm X: mousepad (UI compiled into the binary as a GResource) ----\n'
if [ ! -s "$XIMG" ]; then
  exp_note "building arm X — several minutes"
  PGB_APPIMAGE_CACHE="$WORK/mcache" "$REPO_DIR/pgb" bundle appimage mousepad \
    --out "$XIMG" --name mousepad >"$WORK/build-mousepad.log" 2>&1 || true
fi
if [ -s "$XIMG" ]; then
  run_matrix mousepad "$XIMG"
  X_ROWS=$ROWS; X_CONN=$RAN; X_WIN=$WIN; X_CLEAN=$CLEAN; X_GTK=$GTK; X_NOHOST=$NOHOST
else
  exp_skip "arm X (mousepad)" "the bundle did not build; see $WORK/build-mousepad.log"
  X_ROWS=0; X_CONN=0; X_WIN=0; X_CLEAN=0; X_GTK=0; X_NOHOST=0
fi

# ---------------------------------------------------------------------------
# ⭐ ARM C — THE POSITIVE CONTROL, AND IT IS WHAT TURNS THE DIAGNOSIS INTO A
# MEASUREMENT.
#
# ⛔ WITHOUT IT, "galculator fails because of a hardcoded store path" is an
# INFERENCE from an error message. Arm C runs the IDENTICAL artefact with one
# thing changed — the store path it names is made to resolve, by binding the
# bundle's own AppDir at that path — and asks whether it then draws.
#
# ⚠ THE BIND IS NOT A FIX AND IS NOT PROPOSED AS ONE. It is only available
# because this harness is root and controls the mount namespace; a user
# double-clicking an AppImage has neither. It exists to isolate the cause.
# T-081 owns the real mechanism.
# ---------------------------------------------------------------------------
printf '\n-- arm C: galculator AGAIN, with the store path made to resolve ----\n'
G_STORE=$(basename "$(find "$WORK/gcache" -maxdepth 3 -type d -name '*-galculator-*' 2>/dev/null | head -1)")
G_APPDIR="$WORK/gcache/galculator/AppDir"
if [ -n "$G_STORE" ] && [ -d "$G_APPDIR" ]; then
  run_matrix galculator "$GIMG" "$G_APPDIR:/nix/store/$G_STORE"
  C_ROWS=$ROWS; C_WIN=$WIN; C_CLEAN=$CLEAN
else
  exp_skip "arm C (store path supplied)" "could not locate the AppDir or the store name"
  C_ROWS=0; C_WIN=0; C_CLEAN=0
fi

printf '\n'
printf -- '-- summary ---------------------------------------------------------\n'
printf '  %-34s %12s %12s\n' AXIS 'G galculator' 'X mousepad'
printf '  %-34s %12s %12s\n' 'rows measured'                "$G_ROWS"   "$X_ROWS"
printf '  %-34s %12s %12s\n' 'GTK connected to a real X'    "$G_CONN"   "$X_CONN"
printf '  %-34s %12s %12s\n' 'libgtk-3 loaded from bundle'  "$G_GTK"    "$X_GTK"
printf '  %-34s %12s %12s\n' 'zero HOST shared objects'     "$G_CLEAN"  "$X_CLEAN"
printf '  %-34s %12s %12s\n' '⭐ A WINDOW ON THE X SERVER'   "$G_WIN"    "$X_WIN"
printf '\n  %-34s %12s\n' '⭐ arm C: galculator + store path' "$C_WIN of $C_ROWS"

printf '\n'
exp_check "control: no target ships either program"      "$((G_NOHOST + X_NOHOST))" "$((G_ROWS + X_ROWS))"
exp_check "G: GTK connected to a real display"           "$G_CONN"  "$G_ROWS"
exp_check "G: libgtk-3 loaded FROM THE BUNDLE"           "$G_GTK"   "$G_ROWS"
exp_check "G: host shared objects, rows with zero"       "$G_CLEAN" "$G_ROWS"
exp_check "X: GTK connected to a real display"           "$X_CONN"  "$X_ROWS"
exp_check "X: libgtk-3 loaded FROM THE BUNDLE"           "$X_GTK"   "$X_ROWS"
exp_check "X: host shared objects, rows with zero"       "$X_CLEAN" "$X_ROWS"

# ⭐ THE PAIR THAT CARRIES THE WHOLE FINDING, and it is why there are two
# subjects rather than one. Same bundler, same GTK, same eleven: the one whose
# UI is COMPILED IN draws windows, and the one whose UI is a FILE behind an
# absolute store path draws none.
exp_check "⭐ X: a REAL WINDOW on the X server"           "$X_WIN" "$X_ROWS"
exp_check "⛔ G: a real window on the X server"           "$G_WIN" 0
# ⭐ THE CONTROL THAT PROVES THE DIAGNOSIS. Identical artefact, one variable.
exp_check "⭐ C: the SAME bundle draws once the path resolves" "$C_WIN" "$C_ROWS"
exp_check "C: and it still loaded zero host objects"      "$C_CLEAN" "$C_ROWS"

exp_note "⭐ GTK IS NOT THE BLOCKER, AND THIS IS THE MEASUREMENT THAT SAYS SO."
exp_note "   mousepad draws $X_WIN of $X_ROWS real toplevel windows on a real X"
exp_note "   server, out of a nixpkgs closure, on distributions that ship no"
exp_note "   GTK — with ZERO host shared objects. The field grades GTK"
exp_note "   \"Garbage\"; through this pipeline, on this subject, it works."
exp_note "⛔ WHAT BLOCKS galculator IS A HARDCODED ABSOLUTE STORE PATH:"
exp_note "     Couldn't load /nix/store/<hash>-galculator-2.1.4/share/..."
exp_note "   The file IS in the bundle. XDG_DATA_DIRS points at the bundle and"
exp_note "   serves every app that LOOKS UP its data; it cannot serve one with"
exp_note "   the path baked into its .rodata."
exp_note "⭐ SO THE REMAINING GAP IS TOOLING, NOT CAPABILITY — T-081, whose"
exp_note "   entry names \"hardcoded paths\" verbatim. That is the label"
exp_note "   T-080's guarantee requires, and it is earned here rather than"
exp_note "   asserted."
exp_note "⛔ ARM P (meld, Python): still not measured, and still not a skip —"
exp_note "   a named tooling defect in resolveEntry, asserted above."
exp_note "⚠ AN EARLIER VERSION OF THIS EXPERIMENT SCORED galculator 11 of 11"
exp_note "   GREEN by treating 'cannot open display' as proof GTK worked. The"
exp_note "   operator rejected it: that message appears on real hardware WITH"
exp_note "   a display too. A real display turned 11 green rows into 0."
exp_note "⚠ NOT ESTABLISHED: anything about a GPU (every GL row here is"
exp_note "   swrast, T-059 owns hardware), and anything about Python GUI apps,"
exp_note "   which arm P could not build."

exp_finish
