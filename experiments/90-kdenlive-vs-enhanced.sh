#!/bin/sh
# 90 - our kdenlive bundle against pkgforge-dev/kdenlive-AppImage-Enhanced.
#
# ⛔ THE BAR IS A COMPARISON, NOT A BUILD. T-055 quotes the operator:
# *"if impossible, pivot to kdenlive.nixappimage, but it must be smaller, load
# faster, run faster than pkgforge-dev/kdenlive-AppImage-Enhanced."* An
# AppImage that works is not this entry; three measured columns against a
# named competitor is.
#
#   size   bytes, both artefacts, same day
#   load   first-run and warm-run startup. ⚠ experiments/40-'s noise floor
#          applies: a difference at or under it is "no difference measurable"
#   run    ⭐ A REAL RENDER. `melt` -- kdenlive's engine, and the workload
#          poc/80-mlt already uses -- encodes a generated clip to an MP4 and
#          the wall clock is taken. `--version` would pass on a bundle that
#          cannot decode anything.
#
# ⭐ AND THE TWO SIDES SHIP THE SAME UPSTREAM RELEASE, which is luck rather
# than design and is worth stating either way: nixpkgs has kdenlive 26.08.0
# and the competitor's tag is 26.08.0-1.
#
# ⚠ WHAT THIS CANNOT SETTLE. Neither artefact is exercised as a GUI: none of
# the eleven environments has a display, and the build host's Xvfb is a
# software X server with no compositor. What the GUI column would need is
# T-059's hardware. So "load" is process start, and "run" is the render
# engine, and the window is only proved to OPEN (`poc/91-qt-xcb` does that on
# a static Qt).
#
# Exit: 0 matched, 1 did not, 2 could not run.
# SPDX-License-Identifier: MIT

set -u
. "$(dirname "$0")/lib.sh"

exp_begin "90 - our kdenlive bundle against kdenlive-AppImage-Enhanced"

RR="$REPO_DIR/scripts/common/rootfs-run.sh"
BUNDLER="$REPO_DIR/tool/nix-appimage.sh"
WORK="${PGB_KDEN_WORK:-/var/tmp/t055}"
CACHE="${PGB_KDEN_CACHE:-/var/tmp/pgb-appimage-kden}"
RUN_TIMEOUT="${PGB_KDEN_TIMEOUT:-300}"
WARM_RUNS="${PGB_KDEN_WARM:-3}"
mkdir -p "$WORK" || exit 2

# ⛔ PINNED. The competitor publishes under a moving `latest` and a dated tag;
# two runs a week apart would otherwise compare different artefacts with
# nothing saying so.
ENH_TAG="26.08.0-1@2026-09-01_1788277742"
ENH_URL="https://github.com/pkgforge-dev/kdenlive-AppImage-Enhanced/releases/download/$ENH_TAG/Kdenlive-26.08.0-1-anylinux-x86_64.AppImage"
ENH="$WORK/kdenlive-enhanced.AppImage"
OURS="$CACHE/kdenlive/kdenlive-anylinux-x86_64.AppImage"

B="$EXP_OUT/build"; mkdir -p "$B" || exit 2
: > "$EXP_OUT/per-environment.txt"

reap_rootfs() {
  for _d in /proc/[0-9]*; do
    _p=${_d#/proc/}
    _r=$(readlink "$_d/root" 2>/dev/null) || continue
    case "$_r" in "$1"|"$1"/*) kill -9 "$_p" 2>/dev/null ;; esac
  done
  return 0
}
reap_all() {
  while read -r _ref _name _libc _digest; do
    case "$_ref" in ''|\#*) continue ;; esac
    _r=$(exp_rootfs "$_name"); [ -n "$_r" ] && reap_rootfs "$_r"
  done < "$REPO_DIR/scripts/common/rootfs-images.txt"
  return 0
}
trap 'reap_all' EXIT INT TERM

# ---------------------------------------------------------------------------
# the two artefacts
# ---------------------------------------------------------------------------
printf -- '-- the two artefacts ---------------------------------------------\n'
if [ ! -s "$ENH" ]; then
  exp_note "fetching the competitor, pinned at $ENH_TAG"
  curl -fsSL -o "$ENH" "$ENH_URL" \
    || curl -fsSL -o "$ENH" "https://api.rv.pkgforge.dev/$ENH_URL" \
    || { exp_note "could not fetch $ENH_URL"; exit 2; }
  chmod +x "$ENH"
fi
if [ ! -s "$OURS" ]; then
  exp_note "building ours: sh tool/nix-appimage.sh kdenlive --with-program melt"
  PGB_APPIMAGE_CACHE="$CACHE" sh "$BUNDLER" kdenlive \
    --with-program melt --with-program ffmpeg >"$B/build-ours.log" 2>&1 || true
fi
[ -s "$OURS" ] || { exp_note "ours did not build; see $B/build-ours.log"; exit 2; }
chmod +x "$OURS"

P_SZ=$(wc -c < "$OURS"); E_SZ=$(wc -c < "$ENH")
exp_check "ours exists"           "$([ -s "$OURS" ] && echo yes)" yes
exp_check "the competitor exists" "$([ -s "$ENH" ] && echo yes)" yes
printf '\n  %-34s %14s\n' ARTEFACT BYTES
printf '  %-34s %14s\n' "P  ours (one command, nixpkgs)" "$P_SZ"
printf '  %-34s %14s\n' "E  kdenlive-AppImage-Enhanced"  "$E_SZ"
printf '  %-34s %14s\n' "   ratio P/E" \
  "$(awk -v p="$P_SZ" -v e="$E_SZ" 'BEGIN{printf "%.2fx", p/e}')"
# ⛔ THE SIZE COLUMN IS THE ONE THE OPERATOR NAMED FIRST, and it is reported
# whichever way it comes out. `docs/AGENTS.md` §14 forbids "strictly better"
# without a measurement; it equally forbids hiding one.
exp_note "operator's bar: smaller, loads faster, runs faster than E"

# ---------------------------------------------------------------------------
# the render: melt, both artefacts, on the build host
# ---------------------------------------------------------------------------
printf -- '\n-- the render (melt, the engine kdenlive drives) ------------------\n'
# ⭐ A generated clip encoded to a real MP4. No input file, so the two arms
# cannot differ by what they read; the work is entirely inside their own
# ffmpeg and MLT.
MELT_ARGS="color:blue out=48 -consumer avformat:%OUT% vcodec=libx264 preset=ultrafast an=1"
render() {  # artefact tag -> ms, or -1
  _a="$1"; _t="$2"
  rm -f "$WORK/$_t.mp4"
  _s=$(date +%s%N)
  # shellcheck disable=SC2086
  timeout -k 10 "$RUN_TIMEOUT" "$_a" melt $(printf '%s' "$MELT_ARGS" | sed "s|%OUT%|$WORK/$_t.mp4|") \
    >"$B/render.$_t.log" 2>&1 || { printf '%s' -1; return; }
  _e=$(date +%s%N)
  [ -s "$WORK/$_t.mp4" ] || { printf '%s' -1; return; }
  printf '%s' $(( (_e - _s) / 1000000 ))
}
P_R=$(render "$OURS" ours)
E_R=$(render "$ENH" enh)
P_MP4=$( [ -f "$WORK/ours.mp4" ] && wc -c < "$WORK/ours.mp4" || echo 0)
E_MP4=$( [ -f "$WORK/enh.mp4" ] && wc -c < "$WORK/enh.mp4" || echo 0)
printf '  %-34s %10s ms   %10s bytes of MP4\n' "P  ours" "$P_R" "$P_MP4"
printf '  %-34s %10s ms   %10s bytes of MP4\n' "E  enhanced" "$E_R" "$E_MP4"
exp_check "ours rendered a real MP4"           "$([ "${P_MP4:-0}" -gt 1000 ] && echo yes || echo no)" yes
exp_check "the competitor rendered a real MP4" "$([ "${E_MP4:-0}" -gt 1000 ] && echo yes || echo no)" yes

# ---------------------------------------------------------------------------
# startup, on the build host, with the mount left alive
# ---------------------------------------------------------------------------
printf -- '\n-- startup (the instrument experiments/86- ended up with) ---------\n'
# ⚠ `melt -version` rather than `kdenlive --version`: kdenlive needs a display
# and the competitor's own AppRun refuses `--version` without one, so timing it
# would time an error path. melt starts the same runtime.
start_ms() {  # artefact n -> total ms for n invocations
  _a="$1"; _n="$2"; _i=0
  _s=$(date +%s%N)
  while [ "$_i" -lt "$_n" ]; do
    "$_a" melt -version >/dev/null 2>&1 || { printf '%s' -1; return; }
    _i=$((_i + 1))
  done
  _e=$(date +%s%N)
  printf '%s' $(( (_e - _s) / 1000000 ))
}
cold_of() {  # artefact -> ms, with the mount reaped first
  pkill -f "$(basename "$1")" >/dev/null 2>&1
  sleep 1
  start_ms "$1" 1
}
P_COLD=$(cold_of "$OURS"); P_TOT=$(start_ms "$OURS" $((WARM_RUNS + 1)))
E_COLD=$(cold_of "$ENH");  E_TOT=$(start_ms "$ENH"  $((WARM_RUNS + 1)))
warm_of() { # total cold n -> ms
  [ "$1" = -1 ] && { printf '%s' -1; return; }
  printf '%s' $(( ($1 - $2) / $3 ))
}
P_WARM=$(warm_of "$P_TOT" "$P_COLD" "$WARM_RUNS")
E_WARM=$(warm_of "$E_TOT" "$E_COLD" "$WARM_RUNS")
printf '  %-34s %10s ms cold   %8s ms warm\n' "P  ours"     "$P_COLD" "$P_WARM"
printf '  %-34s %10s ms cold   %8s ms warm\n' "E  enhanced" "$E_COLD" "$E_WARM"

# ---------------------------------------------------------------------------
# the eleven
# ---------------------------------------------------------------------------
printf -- '\n-- melt on the eleven, both artefacts -----------------------------\n'
classify_trace() {  # tracefile /artefact payload|tree
  awk -v want="$2" -v mode="$3" '
    { pid = $1 }
    $0 ~ ("execve\\(\"" want "\"") { inset[pid] = 1; payload = pid; delete cur; next }
    ($0 ~ /(clone|clone3|vfork|fork)\(/ || $0 ~ /<\.\.\. (clone|clone3|vfork|fork) resumed>/) \
      && /= [0-9]+$/ { if (inset[pid]) inset[$NF] = 1; next }
    inset[pid] && /execve\(/ && !/ENOENT|= -1/ { payload = pid; if (mode != "tree") delete cur; next }
    inset[pid] && /open(at)?\(/ && !/ENOENT|= -1/ {
      if (mode != "tree" && pid != payload) next
      if (match($0, /"[^"]*"/) == 0) next
      p = substr($0, RSTART + 1, RLENGTH - 2)
      if (p !~ /\.so(\.[0-9]+)*$/) next
      if (p ~ /^\/(usr\/)?(local\/)?lib(32|64)?\//) cur["host " p] = 1
      else cur["bundled " p] = 1
    }
    END { for (k in cur) print k }
  ' "$1" | sort -u
}
count() { n=$(grep -c . 2>/dev/null) || n=0; printf '%s' "$n"; }

write_test() {  # rootfs
  cat > "$1/kd-test.sh" <<'SH'
#!/bin/sh
HOME=/tmp; export HOME
TMPDIR=/tmp; export TMPDIR
out=$(/kd-arm melt -version 2>&1) || exit 20
case "$out" in *melt*) ;; *) exit 21 ;; esac
/kd-arm melt color:blue out=12 -consumer avformat:/tmp/kd.mp4 vcodec=libx264 preset=ultrafast an=1 \
  >/dev/null 2>&1 || exit 22
[ -s /tmp/kd.mp4 ] || exit 23
exit 0
SH
}

run_arm() {  # rootfs artefact tag -> exit status
  _r="$1"; _a="$2"; _t="$3"
  rm -f "$_r/kd-arm"; cp "$_a" "$_r/kd-arm"; chmod +x "$_r/kd-arm"
  write_test "$_r"
  timeout -k 10 "$RUN_TIMEOUT" sh "$RR" "$_r" -- /bin/sh /kd-test.sh \
    </dev/null >"$B/out.$_t" 2>&1
  _st=$?
  printf '%s' "$_st"
}

printf '  %-19s %-6s | %-8s %-6s | %-8s %-6s\n' ENVIRONMENT LIBC 'P:res' 'P:host' 'E:res' 'E:host'
ENVS=0; P_RUNS=0; E_RUNS=0; P_CLEAN=0; E_CLEAN=0
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  root=$(exp_rootfs "$name") || root=""
  [ -n "$root" ] || { exp_skip "$name" "not fetched"; continue; }
  ENVS=$((ENVS + 1))

  pst=$(run_arm "$root" "$OURS" "$name.P")
  timeout -k 10 "$RUN_TIMEOUT" strace -f \
    -e trace=openat,open,execve,clone,clone3,vfork,fork -o "$B/tr.$name.P" \
    sh "$RR" "$root" -- /bin/sh /kd-test.sh </dev/null >/dev/null 2>&1
  reap_rootfs "$root"
  pnh=$(classify_trace "$B/tr.$name.P" /kd-arm tree | grep '^host ' | count)

  est=$(run_arm "$root" "$ENH" "$name.E")
  timeout -k 10 "$RUN_TIMEOUT" strace -f \
    -e trace=openat,open,execve,clone,clone3,vfork,fork -o "$B/tr.$name.E" \
    sh "$RR" "$root" -- /bin/sh /kd-test.sh </dev/null >/dev/null 2>&1
  reap_rootfs "$root"
  enh_=$(classify_trace "$B/tr.$name.E" /kd-arm tree | grep '^host ' | count)

  rm -f "$root/kd-arm" "$root/kd-test.sh" "$root/tmp/kd.mp4"
  case "$pst" in 0) pres=ok ;; 124) pres=timeout ;; 13[0-9]|1[4-6][0-9]) pres="SIG$((pst-128))" ;; *) pres="exit$pst" ;; esac
  case "$est" in 0) eres=ok ;; 124) eres=timeout ;; 13[0-9]|1[4-6][0-9]) eres="SIG$((est-128))" ;; *) eres="exit$est" ;; esac
  [ "$pres" = ok ] && P_RUNS=$((P_RUNS + 1))
  [ "$eres" = ok ] && E_RUNS=$((E_RUNS + 1))
  [ "${pnh:-1}" = 0 ] && P_CLEAN=$((P_CLEAN + 1))
  [ "${enh_:-1}" = 0 ] && E_CLEAN=$((E_CLEAN + 1))
  printf '  %-19s %-6s | %-8s %-6s | %-8s %-6s\n' "$name" "$libc" "$pres" "${pnh:-?}" "$eres" "${enh_:-?}"
  printf '%-19s %-6s P=%s host=%s | E=%s host=%s\n' "$name" "$libc" "$pres" "${pnh:-?}" "$eres" "${enh_:-?}" \
    >> "$EXP_OUT/per-environment.txt"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

printf '\n'
exp_check "environments measured"                  "$ENVS"    "11"
exp_check "ours rendered on every environment"     "$P_RUNS"  "$ENVS"
exp_check "the competitor did too"                 "$E_RUNS"  "$ENVS"
exp_check "ours loaded no host shared object"      "$P_CLEAN" "$ENVS"

{
  printf '90 - our kdenlive bundle against pkgforge-dev/kdenlive-AppImage-Enhanced\n\n'
  printf 'competitor pinned at %s\n\n' "$ENH_TAG"
  printf '  %-34s %14s\n' ARTEFACT BYTES
  printf '  %-34s %14s\n' 'P  ours (one command, nixpkgs)' "$P_SZ"
  printf '  %-34s %14s\n' 'E  kdenlive-AppImage-Enhanced' "$E_SZ"
  printf '  %-34s %14s\n' '   ratio P/E' \
    "$(awk -v p="$P_SZ" -v e="$E_SZ" 'BEGIN{printf "%.2fx", p/e}')"
  printf '\nrender (melt, a real MP4):\n'
  printf '  P %s ms, %s bytes of MP4\n' "$P_R" "$P_MP4"
  printf '  E %s ms, %s bytes of MP4\n' "$E_R" "$E_MP4"
  printf '\nstartup (melt -version):\n'
  printf '  P %s ms cold, %s ms warm\n' "$P_COLD" "$P_WARM"
  printf '  E %s ms cold, %s ms warm\n' "$E_COLD" "$E_WARM"
  printf '\non the eleven: ours %s/%s rendered, %s/%s with zero host objects;\n' \
    "$P_RUNS" "$ENVS" "$P_CLEAN" "$ENVS"
  printf '               the competitor %s/%s rendered, %s/%s clean.\n' \
    "$E_RUNS" "$ENVS" "$E_CLEAN" "$ENVS"
  printf '\nconditions: %s, %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(uname -sr)"
} > "$EXP_OUT/RESULT.txt"

exp_finish
