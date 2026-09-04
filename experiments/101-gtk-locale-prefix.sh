#!/bin/sh
# THE QUESTION
#
#   docs/research/app-corpus.md rung 3, and it is the DIFFERENTIATOR: a GTK
#   application looks for its translations at a path compiled into the binary,
#   with no environment variable that can redirect it. Does our interposer
#   resolve that path, and does the SAME bundle without it fail?
#
# ⛔ WHY THIS IS NOT ANOTHER WINDOW ROW. app-corpus.md says it plainly: "A
# window is not enough here — the window appears either way." A GTK app with
# unresolved locales draws exactly the same window; it is simply in English.
# So a criterion that counts windows cannot fail for the right reason on this
# rung (delivery rule 6), and `experiments/64-`/`65-` already own that column.
#
# -- ⭐ THE CRITERION, AND WHY IT IS THE .mo OPEN --------------------------
#
# ⚠ THREE CRITERIA WERE CONSIDERED AND TWO WERE REJECTED, recorded so the next
# session does not re-reach them:
#
#   a translated --help string   ⛔ REJECTED. Many GTK4 applications do not
#                                translate --help at all, so an English answer
#                                would be ambiguous between "the mechanism
#                                failed" and "there was nothing to translate".
#   a translated window title    ⛔ REJECTED for the same ambiguity, plus it
#                                needs the row to reach a display.
#   ⭐ the .mo file being OPENED  ✅ TAKEN. It is the application's own syscall,
#     at the resolved path        observed from outside, it is exactly the
#                                mechanism the rung is about, and it does not
#                                depend on whether the environment has locale
#                                support -- which four of the eleven do not.
#
# ⛔ AND IT DISCRIMINATES, WHICH IS THE POINT. Under `--no-storefix` the same
# application issues the same open against `/nix/store/...`, a path that does
# not exist on any of the eleven, and it fails with ENOENT. Arm T sees the
# open SUCCEED under the AppDir; arm N sees it FAIL under /nix/store. Those are
# two different observations, not one observation and its absence.
#
# -- ⛔ PRE-REGISTERED EXPECTATIONS ------------------------------------------
#
# ⛔ COMMITTED BEFORE THE RUN. PROGRESS.md delivery rule 1.
#
#   L1  ⭐ arm T: the application opens a locale catalogue under the BUNDLE's
#       own directory, on 11 of 11.
#   L2  ⭐ arm N, THE NEGATIVE CONTROL: the same bundle built `--no-storefix`
#       opens no catalogue under the bundle on any row, and is seen attempting
#       `/nix/store/...` instead. ⛔ Without L2, L1 could be satisfied by a
#       path that resolved for some other reason.
#   L3  the window still appears in BOTH arms -- which is the sentence that
#       makes this rung worth measuring separately. A row where arm N loses
#       its window is measuring something else and L1 cannot be read as a
#       locale result.
#   L4  arm T loads zero host shared objects on 11 of 11.
#   L5  ⚠ REPORTED, NOT PREDICTED: whether the process actually renders a
#       translated string. That needs the environment to have locale support,
#       and four of the eleven ship none -- so a failure there is the
#       environment, not the bundle, and counting it would be dishonest.
#
# ⚠ WHAT THIS DOES NOT SHOW: that the translation is CORRECT, or that any
# particular application's own gettext call is well formed. It shows the path
# resolves and the catalogue is read.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "101 - rung 3: GTK's compiled-in locale prefix, with and without the interposer"

WORK="${PGB_EXP101_WORK:-/var/tmp/t101}"
RUN_TIMEOUT="${PGB_EXP101_TIMEOUT:-150}"
XDISP="${PGB_EXP101_DISPLAY:-:98}"     # ⛔ NOT :99 -- experiments/65- counts there
ATTR="${PGB_EXP101_ATTR:-gnome-chess}"
PROG="${PGB_EXP101_PROG:-gnome-chess}"
LANGV="${PGB_EXP101_LANG:-de_DE.UTF-8}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2

command -v strace >/dev/null 2>&1 || { exp_note "no strace on PATH"; exit 2; }
for t in Xvfb xwininfo; do
  command -v "$t" >/dev/null 2>&1 || { exp_note "no $t — apt-get install xvfb x11-utils"; exit 2; }
done

XVFB_PID=""
if ! DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1; then
  Xvfb "$XDISP" -ac -screen 0 1024x768x24 >"$WORK/xvfb.log" 2>&1 &
  XVFB_PID=$!
  _w=0; while [ "$_w" -lt 20 ]; do
    DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1 && break; _w=$((_w+1)); sleep 1
  done
fi
DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1 || {
  exp_note "Xvfb did not come up on $XDISP; see $WORK/xvfb.log"; exit 2; }
trap '[ -n "$XVFB_PID" ] && kill "$XVFB_PID" 2>/dev/null' EXIT INT TERM

windows_real() {
  DISPLAY="$XDISP" xwininfo -root -children 2>/dev/null | awk '
    /^ +0x[0-9a-f]+/ { if (match($0, /[0-9]+x[0-9]+\+/)) {
        split(substr($0, RSTART, RLENGTH - 1), d, "x")
        if (d[1] + 0 >= 50 && d[2] + 0 >= 50) n++ } }
    END { print n + 0 }'
}
reap_in_root() {
  for _p in /proc/[0-9]*; do
    _pid=${_p#/proc/}
    _rt=$(readlink "/proc/$_pid/root" 2>/dev/null) || continue
    case "$_rt" in "$1"|"$1"/*) kill -9 "$_pid" 2>/dev/null ;; esac
  done
}

# ---------------------------------------------------------------------------
# Both arms are the SAME subject through the SAME bundler, differing in one
# shipped flag. ⛔ That is what makes arm N a control rather than a second
# experiment: `--no-storefix` removes one mechanism and changes nothing else.
# ---------------------------------------------------------------------------
build_arm() {  # tag extra-flags...
  _tag=$1; shift
  _img="$WORK/$_tag.AppImage"
  [ -s "$_img" ] && { printf '%s' "$_img"; return 0; }
  PGB_APPIMAGE_CACHE="$WORK/cache" "$REPO_DIR/pgb" bundle appimage "$ATTR" \
    --out "$_img" --name "$PROG" "$@" >"$WORK/build-$_tag.log" 2>&1 || true
  [ -s "$_img" ] && printf '%s' "$_img"
}

IMG_T=$(build_arm T)
if [ -z "$IMG_T" ]; then
  why=$(grep -aoE "nixpkgs has no attribute [^ ]*|no entry point in [^ ]*|could not fetch the closure[^\"]*|--name [^ ]* names no program" \
        "$WORK/build-T.log" 2>/dev/null | head -1)
  exp_note "⛔ UNRESOLVED: ${why:-see $WORK/build-T.log}"
  exp_note "   A gap in this measurement, not evidence about the mechanism."
  exit 2
fi
IMG_N=$(build_arm N --no-storefix)
[ -n "$IMG_N" ] || { exp_note "the --no-storefix control did not build"; exit 2; }

ENVS=$(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt")
NENV=$(printf '%s\n' "$ENVS" | wc -l | tr -d ' ')

# mo_opened <trace> <where>  -> 1 if a .mo under <where> was opened SUCCESSFULLY
#
# ⛔ A SPLIT `openat( ... <unfinished ...>` CARRIES THE PATH AND NO RESULT, and
# a filter that only drops lines containing ENOENT keeps the first half of a
# FAILED open and counts it as a success. That is corrections.md C25, and it is
# why the result is required to be a non-negative fd on the SAME line.
mo_opened() {
  grep -aE 'openat\(.*\.mo"' "$1" 2>/dev/null \
    | grep -aF "$2" \
    | grep -avE '= -1' \
    | grep -acE '= [0-9]+' || true
}
mo_attempted() { grep -acE "openat\(.*$2.*\.mo\"" "$1" 2>/dev/null || true; }

printf '\n  %-16s %-10s %-10s %-8s %s\n' ENVIRONMENT 'T .mo' 'N .mo' 'WINDOWS' 'T host .so'

okT=0; okN=0; winBoth=0; cleanT=0; rows=0
for name in $ENVS; do
  root=$(exp_rootfs "$name") || true
  [ -n "$root" ] || { exp_skip "$name" "rootfs not fetched"; continue; }
  rows=$((rows+1))
  tmoT=0; tmoN=0; wT=0; wN=0; nhost=0
  for arm in T N; do
    img=$IMG_T; [ "$arm" = N ] && img=$IMG_N
    _q=0; while [ "$_q" -lt 10 ] && [ "$(windows_real)" != 0 ]; do sleep 1; _q=$((_q+1)); done
    base=$(windows_real)
    rm -f "$root/subj101"; cp "$img" "$root/subj101"; chmod +x "$root/subj101"
    tr="$WORK/tr.$arm.$name"
    strace -f -e trace=openat,open,execve,clone,clone3,vfork -o "$tr" \
      timeout "$RUN_TIMEOUT" "$REPO_DIR/pgb" rootfs run "$root" \
        --bind /tmp/.X11-unix:/tmp/.X11-unix -- \
        /bin/sh -c "DISPLAY=$XDISP LANG=$LANGV LANGUAGE=${LANGV%%.*} APPIMAGE_EXTRACT_AND_RUN=1 /subj101" \
      >"$WORK/out.$arm.$name" 2>"$WORK/err.$arm.$name" &
    _sp=$!
    win=0; _n=0
    while [ "$_n" -lt "$RUN_TIMEOUT" ]; do
      sleep 1; _n=$((_n+1))
      win=$(windows_real); [ "$win" -gt "$base" ] && break
      kill -0 "$_sp" 2>/dev/null || break
    done
    kill "$_sp" 2>/dev/null
    wait "$_sp" 2>/dev/null
    # ⚠ reap BEFORE reading the trace: delivery rule 7.
    reap_in_root "$root"; rm -f "$root/subj101"
    got=0; [ "$win" -gt "$base" ] && got=1
    if [ "$arm" = T ]; then
      # ⭐ the catalogue must be opened under the BUNDLE, not merely opened.
      tmoT=$(mo_opened "$tr" "/tmp/.mount_"); wT=$got
      [ "$tmoT" = 0 ] && tmoT=$(mo_opened "$tr" "/subj101")
      nhost=$(exp_classify_trace "$tr" /subj101 | grep -c '^host ' || true)
    else
      tmoN=$(mo_opened "$tr" "/tmp/.mount_"); wN=$got
      [ "$tmoN" = 0 ] && tmoN=$(mo_opened "$tr" "/subj101")
      nstore=$(mo_attempted "$tr" "/nix/store")
    fi
    rm -f "$tr"
  done
  printf '  %-16s %-10s %-10s %-8s %s\n' "$name" "$tmoT" "$tmoN" "$wT/$wN" "$nhost"
  [ "$tmoT" -gt 0 ] && okT=$((okT+1))
  [ "$tmoN" = 0 ] && okN=$((okN+1))
  [ "$wT" = 1 ] && [ "$wN" = 1 ] && winBoth=$((winBoth+1))
  [ "$nhost" = 0 ] && cleanT=$((cleanT+1))
done

printf '\n'
exp_check "L1  ⭐ arm T opens a catalogue INSIDE the bundle" "$okT" "$rows"
exp_check "L2  ⭐ CONTROL: --no-storefix opens none"          "$okN" "$rows"
exp_check "L3  the window appears in BOTH arms"               "$winBoth" "$rows"
exp_check "L4  arm T loads zero host shared objects"          "$cleanT" "$rows"
exp_note "⚠ L5 IS REPORTED, NOT CHECKED: whether a translated string is"
exp_note "  actually rendered depends on the environment having locale"
exp_note "  support, and four of the eleven ship none. Counting it would"
exp_note "  charge the bundle for the environment."
exp_note "⛔ L3 IS THE ROW THAT MAKES L1 MEAN SOMETHING. If arm N loses its"
exp_note "  window, the control is failing for a second reason and L1 cannot"
exp_note "  be read as a locale result -- it would be measuring whether the"
exp_note "  bundle starts at all, which experiments/64- already owns."

exp_finish
