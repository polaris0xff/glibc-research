#!/bin/sh
# THE QUESTION — the third and last unexplained row of the capability corpus,
# and the only half of it still open.
#
# `flameshot` reads **0 of 11** in `experiments/65-` (`field-3`). That row has
# already been taken apart once and it was two separate things:
#
#   the SHAPE        it is a tray/daemon screenshot tool. `flameshot` with no
#                    subcommand draws nothing by design and puts a 3x3
#                    `Qt Selection Owner` on the server, so a `gui` row
#                    demanding a toplevel >=50x50 cannot pass on it however
#                    well the bundle works. ⭐ Not a bundler result.
#   the SESSION BUS  ⭐ FIXED. The bundle carries `dbus-daemon` out of its own
#                    closure (`--with-program`) and the bed carries the config
#                    file it insists on reading from an absolute /etc path
#                    (`bed-fixtures.sh --install dbus`). `Unable to connect via
#                    DBus` is gone.
#
# ⛔ WHAT REMAINS IS THE CAPTURE, AND IT IS THE ONLY PART THAT IS A CAPABILITY
# QUESTION. `flameshot full --path …` answers `Unable to capture screen` under
# Xvfb with `QT_QPA_PLATFORM=xcb` forced. Its closure carries `grim`, which is
# **Wayland-only**, and no `xdg-desktop-portal`.
#
# ⭐ THE RECORD NAMES A ROUTE RATHER THAN DECLARING A LIMIT — "pull
# `xdg-desktop-portal` and a backend in with `--extra` and run it beside the
# bus" — and has said **Untried** ever since. This experiment tries it.
#
# -- ⛔ PRE-REGISTERED EXPECTATIONS ------------------------------------------
#
#   S0  ⭐ THE BED CAN BE CAPTURED AT ALL, AND THIS IS CHECKED FIRST AND FROM
#       OUTSIDE. `xwd` on the RUNNER, against the same X server, must return a
#       non-empty dump. ⛔ Without this every row below is unreadable: "no
#       screenshot" is equally consistent with "Xvfb cannot be grabbed", and
#       charging that to the bundle is exactly the mistake `64-` made with
#       `cannot open display`. ⚠ If S0 fails the run stops at exit 2.
#   S1  ⛔ ARM A REPRODUCES THE FAILURE. flameshot as the corpus builds it,
#       plus the session bus, must still fail to capture. A green A would mean
#       the recorded failure was the missing bus all along and the rest of
#       this experiment is measuring nothing.
#   S2  ⭐ ARM B CAPTURES. Same closure plus `--extra xdg-desktop-portal` and a
#       backend, portal started beside the bus. The criterion is a FILE:
#       `/tmp/shot.png` must exist, begin with the PNG signature, and carry a
#       width and height read out of its IHDR that match the server's.
#       ⛔ NOT a log line, and not a zero-length file — a broken run also
#       prints "saved" and also creates a path.
#   S3  ⚠ REPORTED, NOT CHECKED: how many host shared objects each arm loads.
#       A portal is a D-Bus service the bundle now starts itself, so this
#       number can only be read once the capture works; scoring it before then
#       would be scoring a program that never ran.
#
# ⚠ WHAT THIS CANNOT ESTABLISH. Xvfb has no compositor and no real GPU, so a
# capture here says the PIPELINE works — a portal reached over the bundle's own
# bus, a grab performed, a file written — and says nothing about a Wayland
# session or hardware capture. ⛔ It also cannot say flameshot's INTERACTIVE
# mode works: `full` is the non-interactive subcommand and is the only one a
# headless bed can answer.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "108 - flameshot's capture: a portal and a bus the bundle brings itself"

WORK="${PGB_EXP108_WORK:-/var/tmp/t108}"
mkdir -p "$WORK" || exit 2
ATTR="${PGB_EXP108_ATTR:-flameshot}"
PROG=flameshot
RUN_TIMEOUT="${PGB_EXP108_TIMEOUT:-120}"
XDISP="${PGB_EXP108_DISPLAY:-:108}"
SCRW=1280; SCRH=800

ENVS=$(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt")

reap_in_root() {
  _rr=$1
  for _p in /proc/[0-9]*; do
    _pid=${_p#/proc/}
    _rt=$(readlink "/proc/$_pid/root" 2>/dev/null) || continue
    case "$_rt" in "$_rr"|"$_rr"/*) kill -9 "$_pid" 2>/dev/null ;; esac
  done
}

start_x() {
  command -v Xvfb >/dev/null 2>&1 || return 1
  DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1 && return 0
  Xvfb "$XDISP" -screen 0 "${SCRW}x${SCRH}x24" -nolisten tcp >/dev/null 2>&1 &
  _n=0
  while [ "$_n" -lt 20 ]; do
    sleep 1; _n=$((_n+1))
    DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1 && return 0
  done
  return 1
}

# ⭐ THE PNG IS READ, NOT TRUSTED. IHDR is the first chunk of every PNG and
# carries width and height as two big-endian 32-bit integers at offsets 16 and
# 20. ⛔ A zero-length file, an HTML error page and a truncated write all fail
# here; "the file exists" would pass on all three.
png_dims() {  # file -> "WxH", or empty
  [ -s "$1" ] || return 0
  head -c 8 "$1" 2>/dev/null | od -An -tx1 | tr -d ' \n' \
    | grep -q '^89504e470d0a1a0a$' || return 0
  od -An -tu1 -j16 -N8 "$1" 2>/dev/null | awk '{
      w = $1*16777216 + $2*65536 + $3*256 + $4
      h = $5*16777216 + $6*65536 + $7*256 + $8
      if (w > 0 && h > 0) printf "%dx%d", w, h
    }'
}

build() {  # out-image extra-flag...
  _img=$1; shift
  if [ ! -s "$_img" ]; then
    PGB_APPIMAGE_CACHE="$WORK/cache" "$REPO_DIR/pgb" bundle appimage "$ATTR" \
      --out "$_img" --name "$PROG" "$@" >"$_img.log" 2>&1 || true
  fi
  [ -s "$_img" ]
}

start_x || { exp_note "no X server on $XDISP"; exit 2; }

# -- S0: can this display be captured AT ALL? --------------------------------
printf -- '-- S0: the instrument control, from OUTSIDE ----------------------\n'
cap=no
if command -v xwd >/dev/null 2>&1; then
  DISPLAY="$XDISP" xwd -root -silent > "$WORK/root.xwd" 2>"$WORK/xwd.err" || true
  [ -s "$WORK/root.xwd" ] && cap=yes
fi
exp_check "S0  ⭐ the RUNNER can dump this display's root window" "$cap" yes
exp_note "$(printf '   xwd wrote %s byte(s); %s' \
    "$(wc -c < "$WORK/root.xwd" 2>/dev/null || echo 0)" \
    "$(head -c 100 "$WORK/xwd.err" 2>/dev/null | tr -d '\n')")"
if [ "$cap" != yes ]; then
  exp_note "⛔ THE BED CANNOT BE GRABBED, so nothing below could be read as a"
  exp_note "   flameshot result. This is an instrument gap, not a finding."
  exit 2
fi

# -- the two arms ------------------------------------------------------------
printf -- '\n-- building the two arms ------------------------------------------\n'
A="$WORK/flameshot-bus.AppImage"
B="$WORK/flameshot-portal.AppImage"

# ⛔ ARM A IS NOT THE CORPUS BUILD. It is the corpus build PLUS the session bus,
# because the bus half is already fixed and re-measuring it would only
# re-discover a known answer. What A isolates is the capture.
build "$A" --with-program dbus-daemon --with-program dbus-launch \
  || { exp_note "arm A did not build; see $A.log"; tail -5 "$A.log"; exit 2; }
exp_check "S0  arm A built (bus only)" "$([ -s "$A" ] && echo yes || echo no)" yes

# ⚠ THE BACKEND IS THE PART THAT MAY NOT RESOLVE. `xdg-desktop-portal` alone
# implements no Screenshot backend; a portal implementation must be there too.
# ⛔ If either attribute is not in nixpkgs under this name the build log says
# so and the row is UNRESOLVED, which is not a failure of the capability.
build "$B" --extra xdg-desktop-portal --extra xdg-desktop-portal-gtk \
          --with-program dbus-daemon --with-program dbus-launch \
          --with-program xdg-desktop-portal --with-program xdg-desktop-portal-gtk \
  || exp_note "arm B did not build; see $B.log"
exp_check "S0  ⭐ arm B built (portal + backend)" \
    "$([ -s "$B" ] && echo yes || echo no)" yes
miss=$(exp_count 'no such program in the closure\|nixpkgs has no attribute' "$B.log")
exp_check "S0  ⛔ arm B resolved BOTH portal pieces" "${miss:-0}" 0
[ "${miss:-0}" != 0 ] && exp_note "$(printf '   %s' \
    "$(grep -a 'no such program in the closure\|nixpkgs has no attribute' "$B.log" \
       | head -2 | tr '\n' ' ' | cut -c1-160)")"

printf -- '\n-- the eleven -----------------------------------------------------\n'
printf '  %-18s %-14s %-14s %s\n' ENVIRONMENT 'A capture' 'B capture' 'B host .so'

rows=0; a_cap=0; b_cap=0; b_rows=0
for name in $ENVS; do
  root=$(exp_rootfs "$name") || true
  [ -n "$root" ] || { exp_skip "$name" "rootfs not fetched"; continue; }

  rm -f "$root/subjA" "$root/subjB"
  if ! cp "$A" "$root/subjA" 2>"$WORK/cp.$name"; then
    exp_skip "$name" "could not stage arm A: $(tr -d '\n' < "$WORK/cp.$name" | cut -c1-80)"
    rm -f "$root/subjA" "$WORK/cp.$name"; continue
  fi
  chmod +x "$root/subjA"
  bhave=no
  if [ -s "$B" ] && cp "$B" "$root/subjB" 2>/dev/null; then chmod +x "$root/subjB"; bhave=yes; fi
  rows=$((rows+1))

  # ⭐ THE BUS AND THE PORTAL ARE STARTED BY THE ARTEFACT, INSIDE THE ROOTFS.
  # `dbus-run-session` is not used: it is not in every closure, and starting
  # the daemon by hand is what proves the bundle carries a working one.
  # ⛔ THE OUTPUT FILES ARE PER-ARM. They used to be per-ENVIRONMENT, so arm B
  # overwrote arm A and the "why did it not capture" note quoted whichever ran
  # last while naming the other. A note that names the wrong arm is worse than
  # no note.
  #
  # ⛔ AND `--appimage-extract` IS CHECKED RATHER THAN ASSUMED. If it fails, or
  # uruntime does not honour the flag, `squashfs-root/bin` does not exist, the
  # bus and the portal never start, and arm B silently BECOMES arm A — an
  # instrument failure that reads exactly like a capability result. ⭐ The
  # script prints EXTRACT=yes|no and the caller SKIPS the row when it is no.
  # ⚠ `cd /tmp` first so `squashfs-root` lands somewhere known: the flag
  # extracts into the CWD, and the CWD inside `pgb rootfs run` is not this
  # experiment's to assume.
  arm_run() {  # subject shot-path trace out-prefix -> "<dims|none> <nhost> <extract>"
    _sp=$1; _shot=$2; _tr=$3; _op=$4
    strace -f -e trace=openat,open,execve,clone,clone3,vfork -o "$_tr" \
      timeout "$RUN_TIMEOUT" "$REPO_DIR/pgb" rootfs run "$root" \
        --bind /tmp/.X11-unix:/tmp/.X11-unix -- \
        /bin/sh -c "
          cd /tmp || exit 9
          rm -rf squashfs-root
          export DISPLAY=$XDISP QT_QPA_PLATFORM=xcb APPIMAGE_EXTRACT_AND_RUN=1
          export XDG_RUNTIME_DIR=/tmp/xdgrt; mkdir -p \$XDG_RUNTIME_DIR
          $_sp --appimage-extract >/dev/null 2>&1 || true
          D=/tmp/squashfs-root
          if [ -d \$D/bin ]; then echo EXTRACT=yes; else echo EXTRACT=no; fi
          [ -x \$D/bin/dbus-daemon ] && {
            \$D/bin/dbus-daemon --session --print-address --fork \
              > /tmp/busaddr 2>/dev/null || true
            export DBUS_SESSION_BUS_ADDRESS=\$(cat /tmp/busaddr 2>/dev/null)
            echo BUS=\$DBUS_SESSION_BUS_ADDRESS
          }
          [ -x \$D/bin/xdg-desktop-portal ] && {
            echo PORTAL=starting
            \$D/bin/xdg-desktop-portal >/tmp/portal.log 2>&1 &
            [ -x \$D/bin/xdg-desktop-portal-gtk ] && \
              \$D/bin/xdg-desktop-portal-gtk >>/tmp/portal.log 2>&1 &
            sleep 3
          }
          $_sp full --path $_shot
          echo \"EXIT=\$?\"
        " >"$_op.out" 2>"$_op.err" &
    _pid=$!
    wait "$_pid" 2>/dev/null
    reap_in_root "$root"
    _d=$(png_dims "$root$_shot")
    [ -n "$_d" ] && printf '%s ' "$_d" || printf 'none '
    printf '%s ' "$(exp_classify_trace "$_tr" "$_sp" | grep -c '^host ' || true)"
    sed -n 's/^EXTRACT=//p' "$_op.out" 2>/dev/null | tail -1 | grep -q yes \
      && printf 'yes' || printf 'no'
  }

  set -- $(arm_run /subjA /tmp/shotA.png "$WORK/tr.A.$name" "$WORK/A.$name")
  adim=$1; aext=$3
  bdim=-; bhost=-; bext=no
  # ⛔ A ROW WHOSE ARM A COULD NOT EXTRACT IS NOT A ROW. Nothing below it was
  # measured: no bus, no portal, no subject.
  if [ "$aext" != yes ]; then
    exp_skip "$name" "the artefact did not --appimage-extract; nothing was measured"
    rm -f "$root/subjA" "$root/subjB"; rows=$((rows-1)); continue
  fi
  if [ "$bhave" = yes ]; then
    set -- $(arm_run /subjB /tmp/shotB.png "$WORK/tr.B.$name" "$WORK/B.$name")
    bdim=$1; bhost=$2; bext=$3
    if [ "$bext" = yes ]; then
      b_rows=$((b_rows+1))
    else
      exp_note "$(printf '   ⛔ %s arm B did not extract; NOT counted toward S2' "$name")"
      bdim=-; bhost=-
    fi
  fi
  rm -f "$root/subjA" "$root/subjB" "$root/tmp/shotA.png" "$root/tmp/shotB.png"

  printf '  %-18s %-14s %-14s %s\n' "$name" "$adim" "$bdim" "$bhost"
  [ "$adim" != none ] && a_cap=$((a_cap+1))
  [ "$bdim" != none ] && [ "$bdim" != - ] && b_cap=$((b_cap+1))
  # ⛔ AND THE REASON IS KEPT WHEN THERE IS NO CAPTURE, or S1/S2 are numbers
  # with nothing behind them.
  for _a in A B; do
    _f="$WORK/$_a.$name.err"
    [ -s "$_f" ] || continue
    _msg=$(grep -aiE 'unable|error|cannot|fail' "$_f" 2>/dev/null | head -1 | cut -c1-110)
    [ -n "$_msg" ] && exp_note "$(printf '   %s arm %s: %s' "$name" "$_a" "$_msg")"
  done
  [ "$rows" = 1 ] && { mv "$WORK/tr.A.$name" "$WORK/keep.tr.A.$name" 2>/dev/null || :
                       mv "$WORK/tr.B.$name" "$WORK/keep.tr.B.$name" 2>/dev/null || :; }
  rm -f "$WORK/tr.A.$name" "$WORK/tr.B.$name"
done

printf '\n'
exp_check "S1  ⛔ arm A (bus only) captures on NONE of $rows" "$a_cap" 0
if [ "$b_rows" = 0 ]; then
  exp_skip "S2  ⭐ arm B captures" "arm B never ran (not built or not staged)"
else
  exp_check "S2  ⭐ arm B (portal) captures on all $b_rows" "$b_cap" "$b_rows"
fi
exp_note "$(printf '⚠ S3 REPORTED: the server is %sx%s, so a capture that is not' "$SCRW" "$SCRH")"
exp_note "   that size is a capture of something else and the table shows it."
exp_note "⛔ WHAT THIS CANNOT SAY. Xvfb has no compositor and no GPU: a green"
exp_note "   S2 says the PIPELINE works — a portal reached over the bundle's"
exp_note "   OWN bus, a grab performed, a file written — and says nothing"
exp_note "   about a Wayland session, hardware capture, or flameshot's"
exp_note "   INTERACTIVE mode, which a headless bed cannot answer at all."

exp_finish
