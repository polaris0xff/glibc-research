#!/bin/sh
# 86 - our bundler against a hand-built Anylinux AppImage, same application.
#
# ⛔ WHAT THIS IS FOR ------------------------------------------------------
#
# TODO T-057 states the claim so it cannot drift: not *"as good as a
# hand-crafted AppImage"* but *"produced by one command from a package name,
# and within measurable distance of one"*. Nothing had measured the distance.
# `experiments/62-` compares **pgb's static output** against an anylinux
# AppImage; nothing compared **our bundle** against one. This does.
#
#   arm P   `sh tool/nix-appimage.sh <attr>` -- ONE command, from a package
#           name. nixpkgs resolves the closure, our script packs it with
#           uruntime + dwarfs + sharun.
#   arm A   the hand-built route: install the distribution's package on Arch,
#           run `quick-sharun.sh` on the binary, then `--make-appimage`.
#           This is `Anylinux-AppImages`' own documented flow, with its own
#           pinned tools, and it is what a person does today.
#
# -- ⛔ THE TWO ARMS ARE NOT THE SAME BUILD OF THE PROGRAM -----------------
#
# Arm P ships nixpkgs' build; arm A ships Arch's. Same upstream project,
# different compilers, flags and dependency versions. ⭐ That is inherent to
# the comparison -- a bundler is judged on what it can deliver, and neither
# route gets to choose the other's payload -- but it means a size difference
# of a few hundred KiB says as much about the two distributions as about the
# two bundlers. Both versions are printed below so the reader can see it.
#
# -- WHY A CLI APPLICATION, SAID PLAINLY ----------------------------------
#
# ⚠ `jq` is the subject because both arms have to be ASSERTED on all eleven,
# and a GUI application cannot be: none of the eleven has a display. The GL
# and GUI case is `experiments/85-`'s, which measures the stack initialising
# rather than a window appearing. What jq buys is a real functional test --
# a UTF-8 round trip and an oniguruma regex, the same work `poc/40-jq` uses --
# and a real shared-library dependency, which is what a bundle is for.
# `PGB_VS_APP` overrides it for anyone who wants a bigger tree.
#
# Exit: 0 matched, 1 did not, 2 could not run.

set -u
. "$(dirname "$0")/lib.sh"

exp_begin "86 - our bundler against a hand-built Anylinux AppImage"

RR="$REPO_DIR/scripts/common/rootfs-run.sh"
BUNDLER="$REPO_DIR/tool/nix-appimage.sh"
ARCH_ROOT="$ROOTFS_DIR/archlinux-latest"
QS="$REPO_DIR/references/pkgforge-dev__Anylinux-AppImages/tree/useful-tools/quick-sharun.sh"
ANYLINUX_C="$REPO_DIR/references/pkgforge-dev__Anylinux-AppImages/tree/useful-tools/lib/anylinux.c"
APP="${PGB_VS_APP:-jq}"
RUN_TIMEOUT="${PGB_VS_TIMEOUT:-90}"
WARM_RUNS="${PGB_VS_WARM_RUNS:-5}"

# ⛔ PINNED BY DIGEST, exactly as experiments/62- pins them and for the same
# reason: every one is published under a moving tag or a "latest" redirect,
# and two runs a week apart would otherwise measure different tooling with
# nothing saying so.
SHARUN_URL="https://github.com/pkgforge-dev/sharun/releases/download/2.3.0/sharun-x86_64"
SHARUN_SHA="826bb0da3824daca97d710e4120074fcbdde82550e98516e4f35c5e653611169"
AIT_URL="https://github.com/pkgforge-dev/appimagetool/releases/download/0.3.3/appimagetool-x86_64-linux"
AIT_SHA="0d01a4e2628efc897be9663f0faeed6a171e1851270b4e1ce4c22434a36ecebd"
URUNTIME_URL="https://github.com/VHSgunzo/uruntime/releases/download/v0.5.9/uruntime-appimage-dwarfs-lite-x86_64"
URUNTIME_SHA="cef962c299f2fa19b2b3cdf2fa1565ee8541796cc89b9a97a591f94041e8b083"
MKDWARFS_URL="https://github.com/mhx/dwarfs/releases/download/v0.15.6/dwarfs-universal-0.15.6-Linux-x86_64"
MKDWARFS_SHA="50891c38ba359db8271819a6cbf6aaa8068681523f0c4f2b8242007a45edaa28"

B="$EXP_OUT/build"
mkdir -p "$B" || exit 2
: > "$EXP_OUT/per-environment.txt"
: > "$B/build.log"

[ -d "$ARCH_ROOT" ] || { exp_note "archlinux-latest not fetched"; exit 2; }
[ -f "$QS" ]        || { exp_note "quick-sharun.sh missing from the reference corpus"; exit 2; }

# Same reaper as 62- and 85-: by what a process is chrooted into, never by
# name (the dwarfs daemon's comm is `memfd:dwarfs`) and never `pkill -f` (the
# rootfs path is in the runner's own command line).
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
# arm P -- one command from a package name
# ---------------------------------------------------------------------------
printf -- '-- arm P: our bundler, one command --------------------------------\n'
# ⛔ NO `--name`. It names the PROGRAM inside the closure's bin/, not the
# artefact, and asking for one that is not there is now refused outright
# (tool/nix-appimage.sh, and the selftest that keeps it refused). The artefact
# name comes from --out; the work directory is separated by its own cache.
P_IMG="${PGB_APPIMAGE_CACHE:-/var/tmp/pgb-appimage}/$APP/$APP-pgb-x86_64.AppImage"
if [ ! -s "$P_IMG" ]; then
  exp_note "sh tool/nix-appimage.sh $APP     <- the whole of arm P"
  sh "$BUNDLER" "$APP" --out "$P_IMG" >"$B/build-P.log" 2>&1 || true
fi
exp_check "arm P built from the package name alone" \
  "$([ -s "$P_IMG" ] && echo yes || echo no)" yes
[ -s "$P_IMG" ] || { exp_note "see $B/build-P.log"; exp_finish; }
P_PATHS=$(sed -n 's/^closure *\([0-9]*\) store paths.*/\1/p' "$B/build-P.log" 2>/dev/null | tail -1)

# ---------------------------------------------------------------------------
# arm A -- the hand-built Anylinux route
# ---------------------------------------------------------------------------
printf -- '\n-- arm A: the hand-built Anylinux route ---------------------------\n'
# ⛔ FETCHED ON THE HOST, NOT IN THE CHROOT, and 62- records why: this
# development environment routes HTTPS through a proxy whose CA the pinned
# Arch image does not carry, and appimagetool is a Rust binary with its own
# bundled trust store, so installing the CA into the image does not reach it.
# The tools' own documented env vars (RUNTIME, DWARFS_CMD, APPIMAGETOOL) take
# the paths, which changes nothing about what is built.
fetch_pinned() {  # url sha dest
  [ -f "$3" ] && [ "$(sha256sum "$3" 2>/dev/null | cut -d' ' -f1)" = "$2" ] && return 0
  curl -fsSL -o "$3" "$1" 2>>"$B/build.log" || return 1
  got=$(sha256sum "$3" 2>/dev/null | cut -d' ' -f1)
  [ "$got" = "$2" ] || { FETCH_WHY="$3 digest is $got, pinned $2"; return 1; }
  chmod +x "$3"
}
STAGED=no
if command -v curl >/dev/null 2>&1 \
   && fetch_pinned "$SHARUN_URL"   "$SHARUN_SHA"   "$B/sharun" \
   && fetch_pinned "$AIT_URL"      "$AIT_SHA"      "$B/appimagetool" \
   && fetch_pinned "$URUNTIME_URL" "$URUNTIME_SHA" "$B/uruntime" \
   && fetch_pinned "$MKDWARFS_URL" "$MKDWARFS_SHA" "$B/mkdwarfs"; then
  STAGED=yes
fi
exp_check "anylinux toolchain staged at its pinned digests" "$STAGED" yes
[ "$STAGED" = yes ] || { exp_note "${FETCH_WHY:-could not fetch the anylinux toolchain}"; exp_finish; }

cp "$ANYLINUX_C" "$B/anylinux.c"
cp "$QS" "$B/quick-sharun.sh"
printf '[Desktop Entry]\nType=Application\nName=%s\nExec=%s\nIcon=%s\nCategories=Utility;\nTerminal=true\n' \
  "$APP" "$APP" "$APP" > "$B/$APP.desktop"
printf '\211PNG\r\n\032\n\0\0\0\rIHDR\0\0\0\1\0\0\0\1\10\6\0\0\0\37\25\304\211\0\0\0\nIDATx\234c\370\17\0\1\1\1\0\30\335\215\260\0\0\0\0IEND\256B`\202' > "$B/$APP.png"

A_IMG="$B/$APP-any-x86_64.AppImage"
if [ ! -s "$A_IMG" ]; then
  AD="$B/AppDir-any"
  rm -rf "$AD"; mkdir -p "$AD/lib"
  cp "$B/sharun" "$AD/sharun"; chmod +x "$AD/sharun"
  sh "$RR" "$ARCH_ROOT" --bind "$B:$B" --workdir "$B" -- /bin/sh -c "
    set -e
    pacman -Sy --noconfirm --needed base-devel binutils patchelf file $APP >/dev/null 2>&1
    cc -shared -fPIC -O2 $B/anylinux.c -o $AD/lib/anylinux.so
    cd $B
    export APPDIR=$AD OUTPATH=$B OUTNAME=$APP-any-x86_64.AppImage
    export DEPLOY_GLIBC=1 DESKTOP=$B/$APP.desktop ICON=$B/$APP.png
    export APPIMAGETOOL=$B/appimagetool RUNTIME=$B/uruntime DWARFS_CMD=$B/mkdwarfs
    sh ./quick-sharun.sh \$(command -v $APP)
    sh ./quick-sharun.sh --make-appimage" </dev/null >>"$B/build.log" 2>&1
  reap_rootfs "$ARCH_ROOT"
fi
exp_check "arm A built by the hand-built route" \
  "$([ -s "$A_IMG" ] && echo yes || echo no)" yes
[ -s "$A_IMG" ] || { exp_note "see $B/build.log"; exp_finish; }
A_LIBS=$(find "$B/AppDir-any/shared/lib" "$B/AppDir-any/lib" -name '*.so*' 2>/dev/null | wc -l)

# ---------------------------------------------------------------------------
# the SIZE column
# ---------------------------------------------------------------------------
P_SZ=$(wc -c < "$P_IMG"); A_SZ=$(wc -c < "$A_IMG")
printf '\n-- size ----------------------------------------------------------\n'
printf '  %-28s %12s  %s\n' ARTEFACT BYTES NOTE
printf '  %-28s %12s  %s\n' "P  ours (nixpkgs closure)" "$P_SZ" "${P_PATHS:-?} store paths, nothing debloated"
printf '  %-28s %12s  %s\n' "A  hand-built (Arch)"      "$A_SZ" "$A_LIBS libraries deployed"
printf '  %-28s %12s  %s\n' "   ratio P/A" \
  "$(awk -v p="$P_SZ" -v a="$A_SZ" 'BEGIN{printf "%.2fx", p/a}')" \
  "⛔ ours is bigger and T-057 item 1 is why: no debloating at all"

# ---------------------------------------------------------------------------
# the trace classifier, verbatim from experiments/62-
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

# ⛔ THE FUNCTIONAL TEST, AND IT IS NOT --version. A UTF-8 round trip through
# jq's own parser and an oniguruma regex: the same work poc/40-jq asserts on.
# A bundle that starts and then cannot reach its own libonig is exactly the
# failure this experiment is looking for.
write_test() {  # rootfs
  cat > "$1/vs-test.sh" <<'SH'
#!/bin/sh
HOME=/tmp; export HOME
TMPDIR=/tmp; export TMPDIR
out=$(printf '{"k":"\346\227\245\346\234\254\350\252\236"}' | /vs-arm -r .k 2>/dev/null) || exit 20
[ "$out" = "$(printf '\346\227\245\346\234\254\350\252\236')" ] || exit 21
re=$(/vs-arm -rn '"abc" | test("^a.c$")' 2>/dev/null) || exit 22
[ "$re" = true ] || exit 23
exit 0
SH
}

now_ns() { date +%s%N; }

# ---------------------------------------------------------------------------
# the matrix: runs, host objects, startup
# ---------------------------------------------------------------------------
printf '\n-- the eleven: does it run, what did it load, how fast does it start --\n'
printf '  %-19s %-6s | %-8s %-5s %-8s %-8s | %-8s %-5s %-8s %-8s\n' \
  ENVIRONMENT LIBC 'P:res' 'P:h' 'P:cold' 'P:warm' 'A:res' 'A:h' 'A:cold' 'A:warm'
P_RUNS=0; A_RUNS=0; P_CLEAN=0; A_CLEAN=0; ENVS=0
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  root=$(exp_rootfs "$name") || root=""
  [ -n "$root" ] || { exp_skip "$name" "not fetched"; continue; }
  ENVS=$((ENVS + 1))
  write_test "$root"
  cells=""
  for arm in P A; do
    case $arm in P) src="$P_IMG" ;; A) src="$A_IMG" ;; esac
    rm -f "$root/vs-arm"; cp "$src" "$root/vs-arm"; chmod +x "$root/vs-arm"

    # ⚠ COLD IS THE FIRST RUN OF THIS ARTEFACT IN THIS ROOTFS, and for an
    # AppImage that is a real cost rather than a page-cache artefact: uruntime
    # has to set up the dwarfs mount before the payload exists at all.
    t0=$(now_ns)
    timeout -k 10 "$RUN_TIMEOUT" sh "$RR" "$root" -- /bin/sh /vs-test.sh \
      </dev/null >"$B/out.$name.$arm" 2>&1
    st=$?; t1=$(now_ns); reap_rootfs "$root"
    cold=$(( (t1 - t0) / 1000000 ))

    warm=-1
    if [ "$st" = 0 ]; then
      best=""
      i=0
      while [ "$i" -lt "$WARM_RUNS" ]; do
        w0=$(now_ns)
        timeout -k 10 "$RUN_TIMEOUT" sh "$RR" "$root" -- /bin/sh /vs-test.sh \
          </dev/null >/dev/null 2>&1
        w1=$(now_ns); reap_rootfs "$root"
        ms=$(( (w1 - w0) / 1000000 ))
        [ -z "$best" ] && best=$ms
        [ "$ms" -lt "$best" ] && best=$ms
        i=$((i + 1))
      done
      warm=$best
    fi

    timeout -k 10 "$RUN_TIMEOUT" strace -f \
      -e trace=openat,open,execve,clone,clone3,vfork,fork -o "$B/tr.$name.$arm" \
      sh "$RR" "$root" -- /bin/sh /vs-test.sh </dev/null >/dev/null 2>&1
    reap_rootfs "$root"
    pl=$(classify_trace "$B/tr.$name.$arm" /vs-arm payload)
    nh=$(printf '%s\n' "$pl" | grep '^host ' | count)
    nb=$(printf '%s\n' "$pl" | grep '^bundled ' | count)

    case "$st" in 0) res=ok ;; 124) res=timeout ;;
      2[0-3]) res="assert$st" ;;
      13[0-9]|1[4-6][0-9]) res="SIG$((st-128))" ;; *) res="exit$st" ;; esac
    if [ "$arm" = P ]; then
      [ "$res" = ok ] && P_RUNS=$((P_RUNS + 1)); [ "$nh" = 0 ] && P_CLEAN=$((P_CLEAN + 1))
    else
      [ "$res" = ok ] && A_RUNS=$((A_RUNS + 1)); [ "$nh" = 0 ] && A_CLEAN=$((A_CLEAN + 1))
    fi
    cells="$cells$(printf ' %-8s %-5s %-8s %-8s |' "$res" "$nh" "${cold}ms" "${warm}ms")"
    {
      printf '== %s (%s) arm %s\n' "$name" "$libc" "$arm"
      printf '   status  : %s\n' "$res"
      printf '   startup : cold %s ms, best-of-%s warm %s ms\n' "$cold" "$WARM_RUNS" "$warm"
      printf '   objects : host=%s bundled=%s\n' "$nh" "$nb"
      printf '   host .so: %s\n' "$(printf '%s\n' "$pl" | sed -n 's/^host //p' | tr '\n' ' ')"
    } >> "$EXP_OUT/per-environment.txt"
    rm -f "$root/vs-arm"
  done
  printf '  %-19s %-6s |%s\n' "$name" "$libc" "${cells% |}"
  rm -f "$root/vs-test.sh"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"
[ "$ENVS" -gt 0 ] || { exp_note "no environments fetched"; exit 2; }

printf '\n  h = HOST shared objects the payload process opened. Objects the\n'
printf '  artefact brought with it are counted separately and are in\n'
printf '  per-environment.txt -- for both arms those are the bundled ld.so,\n'
printf '  libc and the application'"'"'s own libraries.\n'
printf '\n  ⚠ cold and warm both include the test harness (a chroot enter and\n'
printf '  a /bin/sh) which is IDENTICAL for the two arms, so the difference\n'
printf '  between the columns is the artefact and the absolute number is not\n'
printf '  a startup time. experiments/40-'"'"'s noise floor applies: a\n'
printf '  difference at or under it is "no difference measurable".\n\n'

exp_check "arm P ran on every environment"        "$P_RUNS"  "$ENVS"
exp_check "arm A ran on every environment"        "$A_RUNS"  "$ENVS"
exp_check "arm P loaded no host shared object"    "$P_CLEAN" "$ENVS"
exp_check "arm A loaded no host shared object"    "$A_CLEAN" "$ENVS"

printf '\n-- ⛔ what this does NOT establish --------------------------------\n'
printf '  not the same build   arm P ships nixpkgs'"'"' %s, arm A ships Arch'"'"'s.\n' "$APP"
printf '                       Both versions are in per-environment.txt.\n'
printf '  not a GUI            none of the eleven has a display; the GL case\n'
printf '                       is experiments/85-.\n'
printf '  not debloated        T-057 item 1 is untouched, and the size row\n'
printf '                       above is what that costs.\n'
printf '  one machine, one day\n'

exp_finish
