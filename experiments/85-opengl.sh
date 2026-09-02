#!/bin/sh
# 85 - the bundled OpenGL stack, on all eleven environments.
#
# ⛔ WHAT THIS ANSWERS, AND WHAT IT CANNOT --------------------------------
#
# TODO T-052 opened on the operator's message: *"I also remember an infamous
# 'libGl' problem with the nixappimages created, which further killed the
# dream of a 'universal' builder as many graphical apps didn't work."* The
# previous session solved the mesa half ON THE BUILD HOST and said so; this
# experiment is the part T-052's own text calls "still open": **nothing has
# run on the eleven.**
#
# ⛔ AND THE CAVEAT IS PART OF THE RESULT, NOT A FOOTNOTE. **This machine has
# no GPU.** Every row below is software rasterisation. A green row says the
# bundle carries a complete, self-contained GL stack that initialises and
# reports its vendor on a distribution that has none of its own -- which is
# exactly the "libGL problem" as stated. It says NOTHING about anybody's
# iris, radeonsi, amdvlk or NVIDIA path, and T-052 stays open for that reason.
#
# -- THE TWO ARMS, AND WHY THE SECOND ONE IS THE POINT --------------------
#
#   A  bundled mesa      the bundle carries nixpkgs' mesa and points libglvnd
#                        at itself -- LIBGL_DRIVERS_PATH, GBM_BACKENDS_PATH,
#                        __EGL_VENDOR_LIBRARY_DIRS, and the ICD JSONs rewritten
#                        off their absolute /nix/store library_path.
#   B  no bundled mesa   the same closure with `--no-gl`, so libglvnd is there
#                        and no driver is. This is what a bundle of a GL
#                        program IS today.
#
# ⭐ Arm B is a control and it is the whole argument. Without it, arm A's
# "EGL vendor string: Mesa Project" is a program printing a string. With it,
# the pair says the vendor came from the bundle, because the identical bundle
# minus mesa cannot produce one on any of the eleven.
#
# ⚠ NOT MEASURED HERE, and named so nobody reads it in: NVIDIA proprietary
# (the userspace half must match the running kernel module -- nixGL FETCHES a
# matching driver at `nixGL.nix:69` and a bundle cannot), and any GL work on
# real hardware.
#
# Exit: 0 matched, 1 did not, 2 could not run.

set -u
. "$(dirname "$0")/lib.sh"

exp_begin "85 - the bundled OpenGL stack, on all eleven"

RR="$REPO_DIR/pgb"
BUNDLER="$REPO_DIR/pgb"
CACHE="${PGB_APPIMAGE_CACHE:-/var/tmp/pgb-appimage}"
RUN_TIMEOUT="${PGB_GL_TIMEOUT:-90}"

B="$EXP_OUT/build"
mkdir -p "$B" || exit 2
: > "$EXP_OUT/per-environment.txt"

# ⛔ REAP BY WHAT A PROCESS IS CHROOTED INTO, NOT BY ITS NAME. An AppImage's
# uruntime leaves a DWARFS FUSE daemon running on purpose -- a mount that
# outlives the program is what mount mode IS -- and its comm is
# `memfd:dwarfs`, not the artefact's name. `pkill -f` is worse still: the
# rootfs path is in the RUNNER's own command line, so a full-command-line
# match kills the experiment. Both mistakes are already in
# docs/history/corrections.md. Same reaper as experiments/62-.
reap_rootfs() {  # rootfs-path
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
# The two artefacts
# ---------------------------------------------------------------------------
printf -- '-- the two bundles -----------------------------------------------\n'
# ⛔ BOTH ARMS ARE `--name eglinfo`, AND THE SECOND ONE GETS ITS OWN CACHE
# DIRECTORY RATHER THAN ITS OWN NAME. The first version of this file asked for
# `--name eglinfo-nogl` so the two artefacts would not collide under
# $PGB_APPIMAGE_CACHE -- and mesa-demos has no binary called `eglinfo-nogl`, so
# the bundler fell back to the first thing in bin/ and packed `quadstrip-flat`.
# The control arm would have been a different program. `pgb bundle appimage`
# refuses that now (and has a selftest for it); the collision is avoided the
# correct way, by moving the cache.
A_CACHE="$CACHE"
B_CACHE="${PGB_APPIMAGE_CACHE_NOGL:-${CACHE}-nogl}"
A_IMG="$A_CACHE/eglinfo/eglinfo-x86_64.AppImage"
B_IMG="$B_CACHE/eglinfo/eglinfo-nogl-x86_64.AppImage"

if [ ! -s "$A_IMG" ]; then
  exp_note "building arm A (bundled mesa) -- several minutes, ~400 MB of closure"
  PGB_APPIMAGE_CACHE="$A_CACHE" "$BUNDLER" bundle appimage mesa-demos \
    --out "$A_IMG" --name eglinfo >"$B/build-A.log" 2>&1 || true
fi
if [ ! -s "$B_IMG" ]; then
  exp_note "building arm B (--no-gl control)"
  PGB_APPIMAGE_CACHE="$B_CACHE" "$BUNDLER" bundle appimage mesa-demos --no-gl \
    --out "$B_IMG" --name eglinfo >"$B/build-B.log" 2>&1 || true
fi
[ -s "$A_IMG" ] || { exp_note "arm A did not build; see $B/build-A.log"; exit 2; }
[ -s "$B_IMG" ] || { exp_note "arm B did not build; see $B/build-B.log"; exit 2; }

exp_check "arm A built (bundled mesa)"  "$([ -s "$A_IMG" ] && echo yes)" yes
exp_check "arm B built (--no-gl)"       "$([ -s "$B_IMG" ] && echo yes)" yes
exp_note "A $(wc -c < "$A_IMG") bytes    B $(wc -c < "$B_IMG") bytes"
# ⭐ The size difference IS the bundled GL stack, and it is the cost column
# T-057's debloat step will have to argue with.
exp_note "the difference is the GL stack: $(( ($(wc -c < "$A_IMG") - $(wc -c < "$B_IMG")) / 1024 / 1024 )) MiB"

# ---------------------------------------------------------------------------
# ⛔ DATA COHERENCE -- T-071's Prove, and the arm that would have caught all
# four of its failures.
# ---------------------------------------------------------------------------
printf -- '\n-- the bundle'"'"'s DATA, not its DT_NEEDED graph -----------------\n'
# ⛔ WHY THIS IS A SEPARATE ARM AND NOT PART OF THE RUN BELOW. Every other
# check in this tree follows CODE: DT_NEEDED edges, traced opens, symbol
# tables. EGL is not reached that way -- libglvnd reads a JSON file and
# dlopens the string inside it -- so a bundle whose vendor JSON still names
# `/nix/store/…` has every DT_NEEDED resolving, every file present, every
# check green, and no EGL on the target. That is failure 3 of T-071's four,
# and it shipped.
#
# ⭐ The verdict comes from `pgb bundle manifests`, which is the SAME code the
# build runs -- internal/bundle.CheckManifests -- so this asserts on the
# mechanism rather than on a shell re-implementation of it.
A_APPDIR="$A_CACHE/eglinfo/AppDir"
if [ ! -d "$A_APPDIR" ]; then
  exp_skip "manifest coherence" "arm A's AppDir is not on disk at $A_APPDIR"
else
  "$BUNDLER" bundle manifests "$A_APPDIR" >"$B/manifests-A.log" 2>&1
  _mst=$?
  exp_check "arm A: every manifest names a library present in the bundle" \
    "$([ "$_mst" -eq 0 ] && echo yes || echo no)" yes
  sed 's/^/        /' "$B/manifests-A.log"

  # ⛔ THE NEGATIVE CONTROL, AND WITHOUT IT THE ARM ABOVE IS WORTHLESS. A
  # check that passes on a bundle where nothing is wrong has not shown it can
  # see anything. One vendor JSON is put back the way nixpkgs ships it -- an
  # absolute store path -- and the check must FAIL. Restored afterwards, and
  # the restore is verified, because a control that damages the subject and
  # leaves it damaged has broken every arm after it.
  _vj=$(ls "$A_APPDIR"/share/glvnd/egl_vendor.d/*.json 2>/dev/null | head -1)
  if [ -z "$_vj" ]; then
    exp_skip "manifest coherence: the negative control" \
      "arm A carries no EGL vendor JSON to un-rewrite"
  else
    cp "$_vj" "$B/vendor.json.orig"
    sed 's|"library_path"[[:space:]]*:[[:space:]]*"\([^"/]*\)"|"library_path": "/nix/store/00000000000000000000000000000000-mesa/lib/\1"|' \
      "$B/vendor.json.orig" > "$_vj"
    if cmp -s "$B/vendor.json.orig" "$_vj"; then
      exp_skip "manifest coherence: the negative control" \
        "the JSON could not be un-rewritten, so the control proves nothing"
    else
      "$BUNDLER" bundle manifests "$A_APPDIR" >"$B/manifests-A-broken.log" 2>&1
      _bst=$?
      exp_check "an un-rewritten manifest is CAUGHT (the control)" \
        "$([ "$_bst" -ne 0 ] && echo caught || echo missed)" caught
      grep -m1 '^OUTSIDE' "$B/manifests-A-broken.log" | sed 's/^/        /'
    fi
    cp "$B/vendor.json.orig" "$_vj"
    exp_check "the control restored the manifest it damaged" \
      "$(cmp -s "$B/vendor.json.orig" "$_vj" && echo yes || echo no)" yes
  fi
fi

# ---------------------------------------------------------------------------
# The trace classifier, taken verbatim from experiments/62- for the same two
# reasons its comment gives: strace splits a call across `vfork( <unfinished`
# and `<... vfork resumed>) = N`, so matching on the call name alone drops
# every forked child; and execve REPLACES the address space, so objects opened
# before the last exec are not in the running program.
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

# ⚠ EGL_PLATFORM=surfaceless, and it is not a convenience. None of the eleven
# has an X server, a wayland compositor or a GBM device, so the only EGL
# platform any of them can offer is the one that needs no display at all.
# Asking for the default would measure "there is no display here", which is
# true of the bed and not the question.
run_arm() {  # rootfs artefact tag -> writes $B/out.$tag, echoes exit status
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

# ⛔ SCOPED TO THE SURFACELESS SECTION, NOT THE FIRST MATCH IN THE FILE.
# `eglinfo` enumerates EVERY platform it was built for -- GBM, Wayland, X11,
# surfaceless -- and prints a section per platform. On this bed the first three
# print `eglinfo: eglInitialize failed` because there is no display and no DRM
# device, so a first-match read happens to land on the surfaceless section and
# happens to be right. On a machine where GBM initialised it would silently
# start reporting a different platform's vendor under the same column heading.
field() {  # file label -> the value under "Surfaceless platform:", or empty
  awk -v lab="$2" '
    /^Surfaceless platform:/ { insec = 1; next }
    insec && /^[A-Za-z].*platform:/ { insec = 0 }
    insec && index($0, lab ": ") == 1 { print substr($0, length(lab) + 3); exit }
  ' "$1" 2>/dev/null | sed 's/[[:space:]]*$//'
}

# ---------------------------------------------------------------------------
# The matrix
# ---------------------------------------------------------------------------
printf -- '\n-- eglinfo on the eleven, one row per environment -----------------\n'
printf '  %-19s %-6s | %-8s %-14s %-9s | %-8s %s\n' \
  ENVIRONMENT LIBC 'A:exit' 'A:EGL vendor' 'A:driver' 'B:exit' 'B:EGL vendor'
A_MESA=0; A_DRV=0; A_CLEAN=0; A_SAME=0; B_MESA=0; ENVS=0
# ⛔ EXIT STATUS IS NOT THE MEASUREMENT HERE, and pretending it was would have
# failed this experiment on a correct result. `eglinfo` walks EVERY platform
# it was built for and returns the number that failed to initialise, so on a
# machine with no display and no DRM node it exits 3 -- GBM, Wayland and X11 --
# with surfaceless working perfectly. ⭐ The build host produces exactly 3
# too, so the honest assertion is that every target AGREES WITH THE BUILD
# HOST, and that is what A_SAME counts. HOST_EXIT is measured, not assumed.
EGL_PLATFORM=surfaceless "$A_IMG" >"$B/out.HOST.A" 2>&1
HOST_EXIT=$?
HOST_VEND=$(field "$B/out.HOST.A" 'EGL vendor string')
exp_note "the build host, same artefact: exit $HOST_EXIT, vendor '${HOST_VEND:-<none>}'"
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  root=$(exp_rootfs "$name") || root=""
  [ -n "$root" ] || { exp_skip "$name" "not fetched"; continue; }
  ENVS=$((ENVS + 1))

  ast=$(run_arm "$root" "$A_IMG" "$name.A")
  avend=$(field "$B/out.$name.A" 'EGL vendor string')
  adrv=$(field "$B/out.$name.A" 'EGL driver name')
  aapis=$(field "$B/out.$name.A" 'EGL client APIs')
  aver=$(field "$B/out.$name.A" 'EGL version string')

  # The host-object question, asked only of arm A: it is the arm that claims
  # to be self-contained.
  timeout -k 10 "$RUN_TIMEOUT" strace -f \
    -e trace=openat,open,execve,clone,clone3,vfork,fork -o "$B/tr.$name.A" \
    "$RR" rootfs run "$root" -- /bin/sh /gl-run.sh </dev/null >/dev/null 2>&1
  reap_rootfs "$root"
  apl=$(classify_trace "$B/tr.$name.A" /gl-arm payload)
  anh=$(printf '%s\n' "$apl" | grep '^host ' | count)
  anb=$(printf '%s\n' "$apl" | grep '^bundled ' | count)

  bst=$(run_arm "$root" "$B_IMG" "$name.B")
  bvend=$(field "$B/out.$name.B" 'EGL vendor string')

  rm -f "$root/gl-arm" "$root/gl-run.sh"

  case "$ast" in 0) ares=ok ;; 124) ares=timeout ;;
    13[0-9]|1[4-6][0-9]) ares="SIG$((ast-128))" ;; *) ares="exit$ast" ;; esac
  case "$bst" in 0) bres=ok ;; 124) bres=timeout ;;
    13[0-9]|1[4-6][0-9]) bres="SIG$((bst-128))" ;; *) bres="exit$bst" ;; esac

  [ "$avend" = "Mesa Project" ] && A_MESA=$((A_MESA + 1))
  [ -n "$adrv" ] && A_DRV=$((A_DRV + 1))
  [ "$anh" = 0 ] && A_CLEAN=$((A_CLEAN + 1))
  [ "$ast" = "$HOST_EXIT" ] && A_SAME=$((A_SAME + 1))
  [ -n "$bvend" ] && B_MESA=$((B_MESA + 1))

  printf '  %-19s %-6s | %-8s %-14s %-9s | %-8s %s\n' \
    "$name" "$libc" "$ares" "${avend:--}" "${adrv:--}" "$bres" "${bvend:--}"
  {
    printf '== %s (%s)\n' "$name" "$libc"
    printf '   A exit      : %s\n' "$ares"
    printf '   A vendor    : %s\n' "${avend:-<none>}"
    printf '   A driver    : %s\n' "${adrv:-<none>}"
    printf '   A EGL ver   : %s\n' "${aver:-<none>}"
    printf '   A client API: %s\n' "${aapis:-<none>}"
    printf '   A objects   : host=%s bundled=%s\n' "$anh" "$anb"
    printf '   A host .so  : %s\n' "$(printf '%s\n' "$apl" | sed -n 's/^host //p' | tr '\n' ' ')"
    printf '   B exit      : %s\n' "$bres"
    printf '   B vendor    : %s\n' "${bvend:-<none>}"
    printf '   B first line: %s\n' "$(head -1 "$B/out.$name.B" 2>/dev/null)"
  } >> "$EXP_OUT/per-environment.txt"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"
[ "$ENVS" -gt 0 ] || { exp_note "no environments fetched"; exit 2; }

printf '\n  ⚠ A:exit is eglinfo'"'"'s own convention -- the NUMBER OF PLATFORMS\n'
printf '    THAT FAILED to initialise. %s here, on every row and on the build\n' "$HOST_EXIT"
printf '    host: GBM, Wayland and X11, none of which exists in a rootfs with\n'
printf '    no display and no DRM node. Surfaceless is the one that can work\n'
printf '    and it is the one the vendor and driver columns read.\n\n'
exp_check "arm A: surfaceless EGL reports Mesa on every environment" "$A_MESA"  "$ENVS"
exp_check "arm A: a driver is named on every environment"            "$A_DRV"   "$ENVS"
exp_check "arm A: every target agrees with the build host's exit"    "$A_SAME"  "$ENVS"
exp_check "arm A loaded no host shared object"                       "$A_CLEAN" "$ENVS"
# ⛔ THE CONTROL. If this is not zero, the bundle is not what produced the
# vendor string above and every conclusion in T-052 has to be re-read.
# ⭐ Measured: arm B's EGL CLIENT EXTENSIONS STRING IS EMPTY and it reports one
# "Default display platform: eglInitialize failed". libglvnd with no vendor
# does not know the surfaceless platform extension exists, so it cannot even
# enumerate the platform that works in arm A.
exp_check "arm B (no bundled mesa) reported NO vendor anywhere" "$B_MESA" 0

# ---------------------------------------------------------------------------
# What this does and does not establish
# ---------------------------------------------------------------------------
printf '\n-- ⛔ read this before quoting any row above ----------------------\n'
printf '  no GPU        this machine has none. Every row is software\n'
printf '                rasterisation (swrast). That is a real, complete GL\n'
printf '                implementation and it is NOT evidence about anybody'"'"'s\n'
printf '                iris, radeonsi, amdvlk or NVIDIA path.\n'
printf '  no display    none of the eleven has an X server, a wayland\n'
printf '                compositor or a GBM device, so EGL_PLATFORM is\n'
printf '                surfaceless. This measures the GL STACK loading and\n'
printf '                initialising, not anything being drawn to a screen.\n'
printf '  NVIDIA        untouched. The userspace half must match the running\n'
printf '                kernel module; nixGL fetches a matching driver\n'
printf '                (nixGL.nix:69) and a bundle cannot. T-052 stays open.\n'
printf '  size          nothing is debloated -- T-057 owns that.\n'

exp_finish
