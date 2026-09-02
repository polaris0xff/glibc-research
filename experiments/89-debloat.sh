#!/bin/sh
# 89 - debloating the bundle, with the control that says nothing was lost.
#
# ⛔ THE NUMBER TO BEAT IS THIS PROJECT'S OWN. `experiments/85-` measured the
# bundled GL stack at **95 MiB of a 163 MB bundle** and `TODO` T-057 item 1
# reads, in full, *"No debloating at all."*
#
# ⭐ WHAT THE 95 MiB ACTUALLY IS, measured on `mesa-26.2.1` rather than
# assumed. Its `lib` is 273 MB uncompressed and the GL driver is a minority
# of it:
#
#   libgallium-26.2.1.so      58.2 MiB   the actual GL/gallium driver
#   libvulkan_intel.so        27.2 MiB   \
#   libvulkan_nouveau.so      21.4 MiB    | twelve Vulkan ICDs, ~194 MiB,
#   libvulkan_radeon.so       20.0 MiB    | each dlopen'd through a JSON in
#   libvulkan_panfrost.so     17.7 MiB    | share/vulkan/icd.d
#   libvulkan_freedreno.so    16.3 MiB    |
#   libvulkan_asahi.so        15.8 MiB    /
#   libteflon.so              12.1 MiB   an NPU delegate, not a GPU driver
#
# ⛔ SEVEN OF THE TWELVE ARE FOR GPUs THAT CANNOT EXIST ON x86_64 LINUX:
# panfrost is ARM Mali, freedreno is Adreno, broadcom is a Raspberry Pi,
# asahi is Apple silicon, powervr is Imagination, dzn is Direct3D 12 on
# Windows, gfxstream is an Android emulator transport. Removing them is not a
# size/function trade -- there is no function to lose here.
#
# ⚠ WHICH IS AN ARGUMENT, AND AN ARGUMENT IS NOT A MEASUREMENT. This
# experiment runs BOTH bundles on all eleven environments and requires the
# debloated one to reach the same EGL vendor and driver on every row. A
# smaller bundle that stopped working would be the obvious way to get a good
# size number, so the size column is only reported next to that check.
#
# Arms:
#   N  --debloat none    byte-for-byte what 85- and 86- measured
#   S  --debloat safe    the default: docs, build metadata, locales, and the
#                        drivers for hardware this architecture does not have
#   A  --debloat aggressive
#                        also the Vulkan ICDs for GPUs this architecture DOES
#                        have. ⚠ THIS ONE IS A REAL TRADE and the control is
#                        what makes it reportable: an OpenGL program does not
#                        touch them, a Vulkan program does. Arm A is measured
#                        with the same eglinfo assertions as the others, so
#                        the row says whether the trade cost anything HERE --
#                        it does not say a Vulkan application would survive
#                        it, and that is stated rather than implied.
#
# Exit: 0 matched, 1 did not, 2 could not run.
# SPDX-License-Identifier: MIT

set -u
. "$(dirname "$0")/lib.sh"

exp_begin "89 - debloating, and the control that says nothing was lost"

RR="$REPO_DIR/pgb"
BUNDLER="$REPO_DIR/tool/nix-appimage.sh"
RUN_TIMEOUT="${PGB_GL_TIMEOUT:-120}"

B="$EXP_OUT/build"
mkdir -p "$B" || exit 2
: > "$EXP_OUT/per-environment.txt"

reap_rootfs() {  # rootfs-path -- by /proc/PID/root, never by name. See 62-.
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
# the two bundles
# ---------------------------------------------------------------------------
printf -- '-- the two bundles -----------------------------------------------\n'
N_CACHE="${PGB_DEBLOAT_CACHE_N:-/var/tmp/pgb-appimage-dbnone}"
S_CACHE="${PGB_DEBLOAT_CACHE_S:-/var/tmp/pgb-appimage-dbsafe}"
A_CACHE="${PGB_DEBLOAT_CACHE_A:-/var/tmp/pgb-appimage-dbaggr}"
N_IMG="$N_CACHE/eglinfo/eglinfo-none-x86_64.AppImage"
S_IMG="$S_CACHE/eglinfo/eglinfo-safe-x86_64.AppImage"
A_IMG="$A_CACHE/eglinfo/eglinfo-aggr-x86_64.AppImage"

# ⭐ The closure is identical for all three arms, so it is fetched once and
# hard-linked across. Re-fetching 400 MB twice to measure a debloat would be
# the experiment paying for its own arms.
seed_cache() {  # from to
  [ -d "$2/eglinfo/store" ] && return 0
  mkdir -p "$2/eglinfo" 2>/dev/null || return 0
  cp -al "$1/eglinfo/store" "$2/eglinfo/store" 2>/dev/null \
    || cp -a "$1/eglinfo/store" "$2/eglinfo/store" 2>/dev/null || true
  [ -d "$1/tools" ] && { mkdir -p "$2"; cp -a "$1/tools" "$2/" 2>/dev/null || true; }
  return 0
}

if [ ! -s "$N_IMG" ]; then
  exp_note "building arm N (--debloat none) -- several minutes, ~400 MB of closure"
  PGB_APPIMAGE_CACHE="$N_CACHE" "$BUNDLER" mesa-demos --debloat none \
    --out "$N_IMG" --name eglinfo >"$B/build-N.log" 2>&1 || true
fi
if [ ! -s "$S_IMG" ]; then
  exp_note "building arm S (--debloat safe)"
  PGB_APPIMAGE_CACHE="$S_CACHE" "$BUNDLER" mesa-demos --debloat safe \
    --out "$S_IMG" --name eglinfo >"$B/build-S.log" 2>&1 || true
fi
if [ ! -s "$A_IMG" ]; then
  exp_note "building arm A (--debloat aggressive)"
  seed_cache "$N_CACHE" "$A_CACHE"
  PGB_APPIMAGE_CACHE="$A_CACHE" "$BUNDLER" mesa-demos --debloat aggressive \
    --out "$A_IMG" --name eglinfo >"$B/build-A.log" 2>&1 || true
fi
[ -s "$N_IMG" ] || { exp_note "arm N did not build; see $B/build-N.log"; exit 2; }
[ -s "$S_IMG" ] || { exp_note "arm S did not build; see $B/build-S.log"; exit 2; }
[ -s "$A_IMG" ] || { exp_note "arm A did not build; see $B/build-A.log"; exit 2; }

N_SZ=$(wc -c < "$N_IMG"); S_SZ=$(wc -c < "$S_IMG"); A_SZ=$(wc -c < "$A_IMG")
N_DIR=$(du -sb "$N_CACHE/eglinfo/AppDir" 2>/dev/null | cut -f1)
S_DIR=$(du -sb "$S_CACHE/eglinfo/AppDir" 2>/dev/null | cut -f1)
A_DIR=$(du -sb "$A_CACHE/eglinfo/AppDir" 2>/dev/null | cut -f1)
exp_check "arm N built (--debloat none)" "$([ -s "$N_IMG" ] && echo yes)" yes
exp_check "arm S built (--debloat safe)" "$([ -s "$S_IMG" ] && echo yes)" yes
exp_check "arm A built (--debloat aggressive)" "$([ -s "$A_IMG" ] && echo yes)" yes
printf '  %-26s %14s %14s\n' '' 'AppDir bytes' 'AppImage bytes'
printf '  %-26s %14s %14s\n' 'N  --debloat none' "${N_DIR:-?}" "$N_SZ"
printf '  %-26s %14s %14s\n' 'S  --debloat safe' "${S_DIR:-?}" "$S_SZ"
printf '  %-26s %14s %14s\n' 'A  --debloat aggressive' "${A_DIR:-?}" "$A_SZ"
SAVED=$((N_SZ - S_SZ))
printf '  %-26s %14s %14s\n' '   saved' \
  "$(awk -v a="${N_DIR:-0}" -v b="${S_DIR:-0}" 'BEGIN{printf "%.1f MiB", (a-b)/1048576}')" \
  "$(awk -v s="$SAVED" 'BEGIN{printf "%.1f MiB", s/1048576}')"
printf '  %-26s %14s %14s\n' '   ratio S/N' \
  "$(awk -v a="${N_DIR:-1}" -v b="${S_DIR:-1}" 'BEGIN{printf "%.2fx", b/a}')" \
  "$(awk -v a="$N_SZ" -v b="$S_SZ" 'BEGIN{printf "%.2fx", b/a}')"
printf '  %-26s %14s %14s\n' '   ratio A/N' \
  "$(awk -v a="${N_DIR:-1}" -v b="${A_DIR:-1}" 'BEGIN{printf "%.2fx", b/a}')" \
  "$(awk -v a="$N_SZ" -v b="$A_SZ" 'BEGIN{printf "%.2fx", b/a}')"
# The rules that fired, straight out of the build log, so a reader can see
# what was removed and why rather than only the total.
printf '\n  rules that fired:\n'
grep '  debloat  ' "$B/build-S.log" 2>/dev/null | sed 's/^/  /' || true

exp_check "the debloated AppImage is smaller" \
          "$([ "$S_SZ" -lt "$N_SZ" ] && echo yes || echo no)" "yes"

# ---------------------------------------------------------------------------
# the classifier and the field reader, both verbatim from 85-
# ---------------------------------------------------------------------------
classify_trace() {  # tracefile /artefact payload|tree
  awk -v want="$2" -v mode="$3" '
    { pid = $1 }
    $0 ~ ("execve\\(\"" want "\"") { inset[pid] = 1; payload = pid; delete cur; next }
    ($0 ~ /(clone|clone3|vfork|fork)\(/ || $0 ~ /<\.\.\. (clone|clone3|vfork|fork) resumed>/) \
      && /= [0-9]+$/ { if (inset[pid]) inset[$NF] = 1; next }
    inset[pid] && /execve\(/ && !/ENOENT|= -1/ {
      payload = pid
      if (mode != "tree") delete cur
      next
    }
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

field() {  # file label -> the value under "Surfaceless platform:"
  awk -v lab="$2" '
    /^Surfaceless platform:/ { insec = 1; next }
    insec && /^[A-Za-z].*platform:/ { insec = 0 }
    insec && index($0, lab ": ") == 1 { print substr($0, length(lab) + 3); exit }
  ' "$1" 2>/dev/null | sed 's/[[:space:]]*$//'
}

run_arm() {  # rootfs artefact tag -> exit status
  _r="$1"; _img="$2"; _tag="$3"
  rm -f "$_r/gl-arm"; cp "$_img" "$_r/gl-arm"; chmod +x "$_r/gl-arm"
  cat > "$_r/gl-run.sh" <<'SH'
#!/bin/sh
EGL_PLATFORM=surfaceless; export EGL_PLATFORM
HOME=/tmp; export HOME
TMPDIR=/tmp; export TMPDIR
exec /gl-arm
SH
  timeout -k 10 "$RUN_TIMEOUT" "$RR" rootfs run "$_r" -- /bin/sh /gl-run.sh \
    </dev/null >"$B/out.$_tag" 2>&1
  _st=$?
  reap_rootfs "$_r"
  printf '%s' "$_st"
}

# ---------------------------------------------------------------------------
# the matrix
# ---------------------------------------------------------------------------
printf -- '\n-- eglinfo on the eleven, both arms -------------------------------\n'
printf '  %-19s %-6s | %-8s %-14s %-9s | %-8s %-14s %-9s\n' \
  ENVIRONMENT LIBC 'N:exit' 'N:vendor' 'N:driver' 'S:exit' 'S:vendor' 'S:driver'
EGL_PLATFORM=surfaceless "$N_IMG" >"$B/out.HOST.N" 2>&1; HOST_EXIT=$?
HOST_VEND=$(field "$B/out.HOST.N" 'EGL vendor string')
exp_note "the build host, arm N: exit $HOST_EXIT, vendor '${HOST_VEND:-<none>}'"

ENVS=0; S_MESA=0; S_DRV=0; S_CLEAN=0; AGREE=0; AGREE_A=0
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  root=$(exp_rootfs "$name") || root=""
  [ -n "$root" ] || { exp_skip "$name" "not fetched"; continue; }
  ENVS=$((ENVS + 1))

  nst=$(run_arm "$root" "$N_IMG" "$name.N")
  nvend=$(field "$B/out.$name.N" 'EGL vendor string')
  ndrv=$(field "$B/out.$name.N" 'EGL driver name')

  sst=$(run_arm "$root" "$S_IMG" "$name.S")
  svend=$(field "$B/out.$name.S" 'EGL vendor string')
  sdrv=$(field "$B/out.$name.S" 'EGL driver name')

  ast=$(run_arm "$root" "$A_IMG" "$name.A")
  avend=$(field "$B/out.$name.A" 'EGL vendor string')
  adrv=$(field "$B/out.$name.A" 'EGL driver name')
  [ "$avend" = "$nvend" ] && [ "$adrv" = "$ndrv" ] && AGREE_A=$((AGREE_A + 1))

  # The host-object question, asked of the DEBLOATED arm: it is the one whose
  # claim to be self-contained a removed file could have broken.
  timeout -k 10 "$RUN_TIMEOUT" strace -f \
    -e trace=openat,open,execve,clone,clone3,vfork,fork -o "$B/tr.$name.S" \
    "$RR" rootfs run "$root" -- /bin/sh /gl-run.sh </dev/null >/dev/null 2>&1
  reap_rootfs "$root"
  spl=$(classify_trace "$B/tr.$name.S" /gl-arm payload)
  snh=$(printf '%s\n' "$spl" | grep '^host ' | count)

  rm -f "$root/gl-arm" "$root/gl-run.sh"

  [ "$svend" = "Mesa Project" ] && S_MESA=$((S_MESA + 1))
  [ "$sdrv" = "swrast" ] && S_DRV=$((S_DRV + 1))
  [ "${snh:-1}" = 0 ] && S_CLEAN=$((S_CLEAN + 1))
  # ⭐ THE CONTROL: the debloated arm must agree with the undebloated one on
  # BOTH strings, on every row. Not "arm S works" -- "arm S does what arm N
  # did", which is the only statement a size number is allowed to sit next to.
  [ "$svend" = "$nvend" ] && [ "$sdrv" = "$ndrv" ] && AGREE=$((AGREE + 1))

  printf '  %-19s %-6s | %-8s %-14s %-9s | %-8s %-14s %-9s | %-8s %-14s\n' \
    "$name" "$libc" "$nst" "${nvend:-<none>}" "${ndrv:-<none>}" \
    "$sst" "${svend:-<none>}" "${sdrv:-<none>}" \
    "$ast" "${avend:-<none>}"
  printf '%-19s %-6s N exit=%s vendor=%s driver=%s | S exit=%s vendor=%s driver=%s host_objects=%s\n' \
    "$name" "$libc" "$nst" "${nvend:-<none>}" "${ndrv:-<none>}" \
    "$sst" "${svend:-<none>}" "${sdrv:-<none>}" "${snh:-?}" \
    >> "$EXP_OUT/per-environment.txt"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

printf '\n'
exp_check "environments measured"                       "$ENVS"    "11"
exp_check "debloated arm reports Mesa Project"          "$S_MESA"  "$ENVS"
exp_check "debloated arm reports the swrast driver"     "$S_DRV"   "$ENVS"
exp_check "debloated arm loaded no host shared object"  "$S_CLEAN" "$ENVS"
exp_check "debloated arm AGREES with the undebloated one" "$AGREE" "$ENVS"
exp_check "the AGGRESSIVE arm agrees too, on this OpenGL subject" "$AGREE_A" "$ENVS"
exp_note "⚠ arm A drops the Vulkan ICDs for THIS architecture as well. eglinfo"
exp_note "  is an OpenGL/EGL program and does not touch them, so these eleven"
exp_note "  rows say the trade cost nothing HERE. They say nothing about a"
exp_note "  Vulkan application, which is why aggressive is not the default."

{
  printf '89 - debloating the bundle, and the control that says nothing was lost\n\n'
  printf 'subject: mesa-demos eglinfo, the same subject experiments/85- used\n\n'
  printf '  %-26s %14s %14s\n' '' 'AppDir bytes' 'AppImage bytes'
  printf '  %-26s %14s %14s\n' 'N  --debloat none' "${N_DIR:-?}" "$N_SZ"
  printf '  %-26s %14s %14s\n' 'S  --debloat safe' "${S_DIR:-?}" "$S_SZ"
  printf '  %-26s %14s %14s\n' 'A  --debloat aggressive' "${A_DIR:-?}" "$A_SZ"
  printf '  %-26s %14s %14s\n' '   ratio A/N' \
    "$(awk -v a="${N_DIR:-1}" -v b="${A_DIR:-1}" 'BEGIN{printf "%.2fx", b/a}')" \
    "$(awk -v a="$N_SZ" -v b="$A_SZ" 'BEGIN{printf "%.2fx", b/a}')"
  printf '  %-26s %14s %14s\n' '   ratio S/N' \
    "$(awk -v a="${N_DIR:-1}" -v b="${S_DIR:-1}" 'BEGIN{printf "%.2fx", b/a}')" \
    "$(awk -v a="$N_SZ" -v b="$S_SZ" 'BEGIN{printf "%.2fx", b/a}')"
  printf '\nrules that fired:\n'
  grep '  debloat  ' "$B/build-S.log" 2>/dev/null || printf '  (none)\n'
  printf '\non the eleven: Mesa Project %s/%s, swrast %s/%s, zero host objects %s/%s,\n' \
    "$S_MESA" "$ENVS" "$S_DRV" "$ENVS" "$S_CLEAN" "$ENVS"
  printf 'and identical to the undebloated arm on %s/%s rows.\n' "$AGREE" "$ENVS"
  printf 'the aggressive arm is identical to the undebloated one on %s/%s rows.\n' "$AGREE_A" "$ENVS"
  printf '\nconditions: %s, %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(uname -sr)"
} > "$EXP_OUT/RESULT.txt"

exp_finish
