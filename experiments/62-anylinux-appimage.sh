#!/bin/sh
# THE QUESTION
#
#   Against the AppImage that is actually competitive -- not the vanilla one --
#   what is pgb still better at, and what is it not?
#
# -- WHY THIS EXISTS, AND WHAT IT IS CORRECTING ------------------------------
#
# ⛔ experiments/60- BUILT ITS AppImage ARM WITH VANILLA `appimagetool` AND
# SCORED IT 2 OF 11. That number is a fact about the wrong tool. A vanilla
# AppImage deliberately does NOT bundle glibc -- its documented practice is to
# build against the oldest glibc you intend to support -- so it cannot start on
# a musl host and breaks below its build glibc. Measuring that and reporting it
# as "AppImage" is measuring a strawman.
#
# ⭐ THE COMPETITIVE ONE IS pkgforge-dev/Anylinux-AppImages, which is what
# tmp/START.md named as required reading in the first place. Its position is
# the opposite of vanilla's, and stated bluntly in its own HOW-TO-MAKE-THESE.md
# under "The solution": *bundle every library the application needs and don't
# rely on the host libc*. Same page, on static linking: "Compile statically!
# Sure, that works, go and compile all of kdenlive statically and get back to
# me once you get it done." ⚠ That is a direct and fair criticism of this
# project's approach, and it belongs in the comparison rather than out of it.
#
# -- WHAT THAT STACK ACTUALLY IS ---------------------------------------------
#
# Built exactly as HOW-TO-MAKE-THESE.md prescribes, on Arch (the page says
# building anywhere else is a bad idea), with the application installed to
# /usr first (it says that too), through `quick-sharun.sh`:
#
#   sharun          bundles the closure, hardlinks itself per binary so
#                   /proc/self/exe is right, and runs the BUNDLED ld-linux
#                   with --library-path rather than LD_LIBRARY_PATH or rpath
#   anylinux.so     LD_PRELOAD that pins NSS with __nss_configure_lookup --
#                   ⭐ the same mechanism, over the same 14 databases, that
#                   tool/runtime/pgb-nssfix.c uses. This project took the idea
#                   from that file; docs/research/prior-art.md records it
#   bundled gconv   the whole /usr/lib/gconv tree plus its config, reached by
#                   GCONV_PATH. ⭐ THIS IS WHY THE onelf RESULT IN 60- DOES NOT
#                   GENERALISE: bundling a libc does not bundle gconv, but this
#                   stack bundles gconv ON PURPOSE, and its modules' DT_NEEDED
#                   libc.so.6 resolves to the BUNDLED libc, so no second libc
#                   enters. docs/AGENTS.md §14's refusal to bundle gconv is
#                   correct for a STATIC binary and does not apply here
#   uruntime        AppImage runtime with DWARFS instead of squashfs
#
# -- WHAT IS MEASURED --------------------------------------------------------
#
# The same three axes as everything else, so the rows compose with 60- and 61-:
# does it run and what did it load (11 environments), what does it cost to
# ship, and -- the axis 60- got wrong -- what is its steady-state throughput.
#
# ⛔ NOTHING IS ASSERTED ABOUT THE anylinux ARM. First measurement; an
# expectation would be the guess this exists to replace. pgb's own established
# results are asserted, as in 60-.
#
# Exit: 0 measured, 1 an assertion failed, 2 could not measure.

. "$(dirname "$0")/lib.sh"

exp_begin "62 - pgb against Anylinux-AppImages, the competitive AppImage"

ARCH_ROOT="$ROOTFS_DIR/archlinux-latest"
ENV_ROOT="$ROOTFS_DIR/${PGB_ENV_NAME:-pgb-env-debian12}"
RR="$REPO_DIR/scripts/common/rootfs-run.sh"
QS="$REPO_DIR/references/pkgforge-dev__Anylinux-AppImages/tree/useful-tools/quick-sharun.sh"
ANYLINUX_C="$REPO_DIR/references/pkgforge-dev__Anylinux-AppImages/tree/useful-tools/lib/anylinux.c"

# ⛔ PINNED BY DIGEST, like scripts/common/rootfs-images.txt and for the same
# reason: every one of these is published under a moving tag or a "latest"
# redirect. These are the exact artefacts every number below was taken
# against. A mismatch SKIPS the arm rather than quietly measuring a different
# stack.
SHARUN_URL="https://github.com/pkgforge-dev/sharun/releases/download/2.3.0/sharun-x86_64"
SHARUN_SHA="826bb0da3824daca97d710e4120074fcbdde82550e98516e4f35c5e653611169"
AIT_URL="https://github.com/pkgforge-dev/appimagetool/releases/download/0.3.3/appimagetool-x86_64-linux"
AIT_SHA="0d01a4e2628efc897be9663f0faeed6a171e1851270b4e1ce4c22434a36ecebd"
URUNTIME_URL="https://github.com/VHSgunzo/uruntime/releases/download/v0.5.9/uruntime-appimage-dwarfs-lite-x86_64"
URUNTIME_SHA="cef962c299f2fa19b2b3cdf2fa1565ee8541796cc89b9a97a591f94041e8b083"
MKDWARFS_URL="https://github.com/mhx/dwarfs/releases/download/v0.15.6/dwarfs-universal-0.15.6-Linux-x86_64"
MKDWARFS_SHA="50891c38ba359db8271819a6cbf6aaa8068681523f0c4f2b8242007a45edaa28"

RUN_TIMEOUT="${PGB_VS_TIMEOUT:-45}"
MATRIX_SCALE="${PGB_BENCH_MATRIX_SCALE:-1}"

B="$EXP_OUT/build"
mkdir -p "$B" || exit 2
: > "$EXP_OUT/per-environment.txt"

[ -d "$ARCH_ROOT" ] || { exp_note "archlinux-latest not fetched"; exit 2; }
[ -d "$ENV_ROOT" ]  || { exp_note "no build environment: sh pgb env create"; exit 2; }
[ -f "$QS" ]        || { exp_note "quick-sharun.sh missing from the reference corpus"; exit 2; }

# ⛔ REAP BY WHAT A PROCESS IS CHROOTED INTO, NOT BY ITS NAME, AND THIS ARM IS
# WHY. An AppImage's uruntime leaves a DWARFS FUSE daemon running on purpose --
# a mount that outlives the program is what mount mode IS -- and that daemon's
# comm is `memfd:dwarfs`, not the artefact's name. A name-matched reaper
# therefore cleaned up nothing: a full pass of this experiment left 22 daemons
# alive, one per AppImage invocation, and they had to be killed by hand.
#
# ⚠ AND NOT `pkill -f` EITHER. The rootfs path appears in the runner's own
# command line, so a full-command-line match kills the experiment -- that
# mistake is already in docs/history/corrections.md and was made twice.
#
# /proc/PID/root is the chroot each process actually sits in, so this reaps
# every straggler of a cell whatever it is called, and can match nothing
# outside the test bed.
reap_rootfs() {  # rootfs-path
  for _d in /proc/[0-9]*; do
    _p=${_d#/proc/}
    _r=$(readlink "$_d/root" 2>/dev/null) || continue
    case "$_r" in "$1"|"$1"/*) kill -9 "$_p" 2>/dev/null ;; esac
  done
  return 0
}

# Belt and braces: an interrupted run must not leave the bed populated either.
reap_all() {
  while read -r _ref _name _libc _digest; do
    case "$_ref" in ''|\#*) continue ;; esac
    _r=$(exp_rootfs "$_name"); [ -n "$_r" ] && reap_rootfs "$_r"
  done < "$REPO_DIR/scripts/common/rootfs-images.txt"
  return 0
}
trap 'reap_all' EXIT INT TERM

# ⛔ EVERY ARTEFACT IS FETCHED ON THE HOST, NOT IN THE CHROOT, and the reason
# is worth recording rather than working around silently: this development
# environment routes HTTPS through a proxy whose CA the pinned Arch image does
# not carry, and `appimagetool` is a Rust binary using its own bundled trust
# store, so even installing the CA into the image does not reach it. Fetching
# outside and passing the paths in through the tool's OWN documented env vars
# (RUNTIME, DWARFS_CMD, APPIMAGETOOL) changes nothing about what is built.
fetch_pinned() {  # url sha dest -> 0 ok
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

# The two payloads: 60-'s functional subject and 61-'s throughput benchmark,
# reused verbatim so the rows compose with those experiments.
cp "$REPO_DIR/evidence/60-versus-alternatives/build/subject.c" "$B/subject.c" 2>/dev/null
cp "$REPO_DIR/evidence/61-libc-throughput/build/bench.c"       "$B/bench.c"   2>/dev/null
if [ ! -f "$B/subject.c" ] || [ ! -f "$B/bench.c" ]; then
  exp_note "run experiments/60- and 61- first: this reuses their sources verbatim"
  exp_finish
fi

mk_desktop() { printf '[Desktop Entry]\nType=Application\nName=%s\nExec=%s\nIcon=%s\nCategories=Utility;\nTerminal=true\n' "$1" "$1" "$1" > "$2"; }
printf '\211PNG\r\n\032\n\0\0\0\rIHDR\0\0\0\1\0\0\0\1\10\6\0\0\0\37\25\304\211\0\0\0\nIDATx\234c\370\17\0\1\1\1\0\30\335\215\260\0\0\0\0IEND\256B`\202' > "$B/icon.png"

# ⭐ BUILT ON ARCH, WITH THE APP INSTALLED TO /usr, BECAUSE THAT IS WHAT THE
# UPSTREAM PAGE REQUIRES. Every other arm in 60- was built in the pinned
# debian:12 so no arm won on compiler; this one cannot be, and the difference
# is deliberate rather than sloppy: anylinux bundles its own glibc, so its
# build host sets no floor on the target the way vanilla AppImage's does.
build_appimage() { # name source-file extra-cflags -> $B/<name>.AppImage
  _n="$1"; _src="$2"; _cf="$3"
  _ad="$B/AppDir-$_n"
  rm -rf "$_ad"; mkdir -p "$_ad/lib"
  cp "$B/sharun" "$_ad/sharun"; chmod +x "$_ad/sharun"
  mk_desktop "pgb-$_n" "$B/pgb-$_n.desktop"
  cp "$B/icon.png" "$B/pgb-$_n.png"
  # anylinux.so is pre-built into the AppDir on purpose: quick-sharun skips its
  # own build when the object is already there, which is its documented path
  # and avoids a fetch this environment cannot make.
  sh "$RR" "$ARCH_ROOT" --bind "$B:$B" --workdir "$B" -- /bin/sh -c "
    set -e
    gcc -O2 -o /usr/bin/pgb-$_n $_src $_cf
    cc -shared -fPIC -O2 $B/anylinux.c -o $_ad/lib/anylinux.so
    cd $B
    export APPDIR=$_ad OUTPATH=$B OUTNAME=$_n-x86_64.AppImage
    export DEPLOY_GLIBC=1 DESKTOP=$B/pgb-$_n.desktop ICON=$B/pgb-$_n.png
    export APPIMAGETOOL=$B/appimagetool RUNTIME=$B/uruntime DWARFS_CMD=$B/mkdwarfs
    sh ./quick-sharun.sh /usr/bin/pgb-$_n
    sh ./quick-sharun.sh --make-appimage" </dev/null >>"$B/build.log" 2>&1
  [ -f "$B/$_n-x86_64.AppImage" ]
}

ANY_SUBJ=no; ANY_BENCH=no
sh "$RR" "$ARCH_ROOT" -- /bin/sh -c \
  'pacman -Sy --noconfirm --needed base-devel wget curl file binutils patchelf >/dev/null 2>&1' \
  </dev/null >>"$B/build.log" 2>&1
build_appimage subject "$B/subject.c" ""                 && ANY_SUBJ=yes
build_appimage bench   "$B/bench.c"   "-lm -lpthread"    && ANY_BENCH=yes
exp_check "anylinux AppImage built (subject)" "$ANY_SUBJ" yes
exp_check "anylinux AppImage built (bench)"   "$ANY_BENCH" yes

# pgb's own two binaries, reused from 60- and 61- so nothing is rebuilt
# differently here.
PGB_SUBJ="$REPO_DIR/evidence/60-versus-alternatives/build/arm-P"
PGB_BENCH="$REPO_DIR/evidence/61-libc-throughput/build/b-pgb"
MUSL_BENCH="$REPO_DIR/evidence/61-libc-throughput/build/c-musl"
for f in "$PGB_SUBJ" "$PGB_BENCH" "$MUSL_BENCH"; do
  [ -x "$f" ] || { exp_note "missing $f -- run experiments/60- and 61- first"; exp_finish; }
done
printf '\n'

# ---------------------------------------------------------------------------
# Tracing: the same classifier as 60-, with the fork bug 60- had
# ---------------------------------------------------------------------------
# ⛔ 60-'s FORK-FOLLOWER MISSED HALF THE FORKS AND THIS ARM IS WHERE IT SHOWED.
# strace splits a call across `vfork( <unfinished ...>` and
# `<... vfork resumed>) = 1234`; the return value is only on the SECOND line,
# which does not contain `vfork(`. 60- matched on `vfork(` and a trailing
# `= N`, so no line ever satisfied both and every vfork child was dropped. The
# anylinux AppImage runs its payload in exactly such a child, so the first
# reading of this arm reported that it opened nothing at all -- not one object,
# bundled or host, which is impossible for a program that ran and passed.
#
# ⛔ AND A SECOND CORRECTION THE SAME ARM FORCED: `execve` REPLACES THE ADDRESS
# SPACE, so an object opened BEFORE the last exec is not in the running program.
# An AppImage AppRun is a shell script, so the sequence in one pid is
#
#     execve(AppImage) -> execve(AppRun) -> execve(/bin/sh) -> execve(payload)
#
# and on a distribution whose /bin/sh is dynamically linked the shell opens the
# host libc, readline and ncurses. Attributing those to the payload -- which
# the first version did, because they happen in the payload's pid -- reported
# the anylinux arm as "payload clean 4 of 11" when the program itself had none
# of them mapped. ⭐ In `payload` mode the set is therefore CLEARED at each
# successful exec, which is what "what is mapped in the process now" means. In
# `tree` mode nothing is cleared, because "what did the machine have to load to
# deliver this" is a different and also real question.
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

# ---------------------------------------------------------------------------
# Matrix: does it run, and what did it load
# ---------------------------------------------------------------------------
printf -- '-- runs correctly, and host objects loaded ------------------------\n'
printf '  %-19s %-6s %-14s %-14s %s\n' ENVIRONMENT LIBC 'anylinux' 'pgb' 'anylinux host .so'
A_RUNS=0; A_CLEAN=0; P_RUNS=0; P_CLEAN=0; ENVS=0
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  root=$(exp_rootfs "$name"); [ -n "$root" ] || { exp_skip "$name" "not fetched"; continue; }
  ENVS=$((ENVS+1))
  cells=""
  for arm in A P; do
    case $arm in
      A) src="$B/subject-x86_64.AppImage" ;;
      P) src="$PGB_SUBJ" ;;
    esac
    rm -f "$root/vs-arm"; cp "$src" "$root/vs-arm"; chmod +x "$root/vs-arm"
    timeout -k 10 "$RUN_TIMEOUT" sh "$RR" "$root" -- /vs-arm </dev/null >"$B/out.$name.$arm" 2>&1
    st=$?; reap_rootfs "$root"
    case $st in
      0) res=ok ;; 124) res=timeout ;;
      13[0-9]|1[4-6][0-9]) res="SIG$((st-128))" ;; *) res="exit$st" ;;
    esac
    timeout -k 10 "$RUN_TIMEOUT" strace -f \
      -e trace=openat,open,execve,clone,clone3,vfork,fork -o "$B/tr.$name.$arm" \
      sh "$RR" "$root" -- /vs-arm </dev/null >/dev/null 2>&1
    reap_rootfs "$root"
    pl=$(classify_trace "$B/tr.$name.$arm" /vs-arm payload)
    nh=$(printf '%s\n' "$pl" | grep '^host ' | count)
    nb=$(printf '%s\n' "$pl" | grep '^bundled ' | count)
    tr=$(classify_trace "$B/tr.$name.$arm" /vs-arm tree)
    th=$(printf '%s\n' "$tr" | sed -n 's/^host //p' | sed 's|.*/||' | tr '\n' ' ')
    if [ "$arm" = A ]; then
      [ "$res" = ok ] && A_RUNS=$((A_RUNS+1)); [ "$nh" = 0 ] && A_CLEAN=$((A_CLEAN+1)); A_TREE="$th"
    else
      [ "$res" = ok ] && P_RUNS=$((P_RUNS+1)); [ "$nh" = 0 ] && P_CLEAN=$((P_CLEAN+1))
    fi
    cells="$cells $(printf '%-14s' "$res/$nh")"
    {
      printf '== %s  arm %s\n' "$name" "$arm"
      printf '   status  : %s\n' "$res"
      printf '   payload : host=%s bundled=%s\n' "$nh" "$nb"
      printf '   objects : %s\n' "$(printf '%s\n' "$pl" | sed 's|/tmp/\.mount_[^/]*|<mount>|' | tr '\n' ' ')"
      printf '   tree    : %s\n' "${th:-none}"
    } >> "$EXP_OUT/per-environment.txt"
    rm -f "$root/vs-arm"
  done
  printf '  %-19s %-6s%s %s\n' "$name" "$libc" "$cells" "${A_TREE:-none}"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"
[ "$ENVS" -gt 0 ] || { exp_note "no environments fetched"; exit 2; }

printf '\n  cell is EXITSTATUS/H, H = HOST shared objects the payload process\n'
printf '  opened. Objects the artefact brought with it are counted separately\n'
printf '  and are in per-environment.txt -- for the anylinux arm they are its\n'
printf '  bundled ld-linux, libc, anylinux.so and gconv modules.\n'
printf '\n  ⚠ THE LAST COLUMN IS THE DELIVERY MECHANISM, NOT THE PROGRAM. An\n'
printf '    AppImage AppRun is a shell script, so on a distribution whose\n'
printf '    /bin/sh is dynamically linked the tree picks up that shell libc.\n'
printf '    The payload process itself stays clean; both numbers are real and\n'
printf '    they answer different questions.\n\n'

# ---------------------------------------------------------------------------
# Throughput on the matrix: does each delivery keep glibc's numbers?
# ---------------------------------------------------------------------------
printf -- '-- throughput, ns per operation, lower is better ------------------\n'
printf '  one round at scale %s; see experiments/61- for the multi-round figures\n\n' "$MATRIX_SCALE"
printf '  %-19s %-6s %8s %8s %8s   %8s %8s %8s\n' \
  ENVIRONMENT LIBC 'A mal4' 'P mal4' 'M mal4' 'A strop' 'P strop' 'M strop'
getv() { awk -v k="$1" '$1 == k { print $2; exit }'; }
if [ "$ANY_BENCH" = yes ]; then
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  root=$(exp_rootfs "$name"); [ -n "$root" ] || continue
  for arm in A P M; do
    case $arm in
      A) src="$B/bench-x86_64.AppImage" ;;
      P) src="$PGB_BENCH" ;;
      M) src="$MUSL_BENCH" ;;
    esac
    rm -f "$root/vs-arm"; cp "$src" "$root/vs-arm"; chmod +x "$root/vs-arm"
    timeout -k 10 600 sh "$RR" "$root" -- /vs-arm all "$MATRIX_SCALE" \
      </dev/null >"$B/bench.$name.$arm" 2>/dev/null
    reap_rootfs "$root"; rm -f "$root/vs-arm"
  done
  printf '  %-19s %-6s %8s %8s %8s   %8s %8s %8s\n' "$name" "$libc" \
    "$(getv malloc4 < "$B/bench.$name.A")" "$(getv malloc4 < "$B/bench.$name.P")" \
    "$(getv malloc4 < "$B/bench.$name.M")" \
    "$(getv strops  < "$B/bench.$name.A")" "$(getv strops  < "$B/bench.$name.P")" \
    "$(getv strops  < "$B/bench.$name.M")"
  {
    printf 'THROUGHPUT %s libc=%s\n' "$name" "$libc"
    for w in malloc1 malloc4 memcpy strops qsort snprintf math; do
      printf '  %-9s anylinux=%-10s pgb=%-10s musl=%s\n' "$w" \
        "$(getv "$w" < "$B/bench.$name.A")" "$(getv "$w" < "$B/bench.$name.P")" \
        "$(getv "$w" < "$B/bench.$name.M")"
    done
  } >> "$EXP_OUT/per-environment.txt"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"
fi
printf '\n  A anylinux AppImage   P pgb   M static musl\n\n'

# ---------------------------------------------------------------------------
printf -- '-- what each one is, and costs -----------------------------------\n'
printf '  %-22s %12s  %s\n' DELIVERY 'SHIP (B)' 'WHAT THE TARGET SEES'
printf '  %-22s %12s  %s\n' "anylinux AppImage" \
  "$(wc -c < "$B/subject-x86_64.AppImage" 2>/dev/null)" \
  "a mount or an extraction into /tmp, then a shell AppRun"
printf '  %-22s %12s  %s\n' "pgb" "$(wc -c < "$PGB_SUBJ")" \
  "one ELF, no interpreter, no mount, nothing written"
printf '\n'

printf -- '-- coverage, out of %s ---------------------------------------------\n' "$ENVS"
printf '  %-22s %10s %16s\n' DELIVERY RUNS 'PAYLOAD CLEAN'
printf '  %-22s %10s %16s\n' "anylinux AppImage" "$A_RUNS/$ENVS" "$A_CLEAN/$ENVS"
printf '  %-22s %10s %16s\n' "pgb"               "$P_RUNS/$ENVS" "$P_CLEAN/$ENVS"
printf '\n'

exp_check "pgb ran on every environment"     "$P_RUNS"  "$ENVS"
exp_check "pgb loaded no host shared object" "$P_CLEAN" "$ENVS"

# ⛔ A LEAK MUST FAIL THE EXPERIMENT, NOT BE LEFT FOR THE OPERATOR TO NOTICE.
# The first version of this script left 22 FUSE daemons running and said
# nothing; the person running it had to find and kill them by hand.
strays=0
while read -r _ref _name _libc _digest; do
  case "$_ref" in ''|\#*) continue ;; esac
  _r=$(exp_rootfs "$_name"); [ -n "$_r" ] || continue
  for _d in /proc/[0-9]*; do
    _rr=$(readlink "$_d/root" 2>/dev/null) || continue
    case "$_rr" in "$_r"|"$_r"/*) strays=$((strays+1)) ;; esac
  done
done < "$REPO_DIR/scripts/common/rootfs-images.txt"
exp_check "no processes left running in the test bed" "$strays" 0

exp_note "⛔ THE HONEST READING, IF THE ROWS COME OUT AS THEY DID FIRST TIME."
exp_note "  Both deliver glibc everywhere and both keep glibc's throughput on a"
exp_note "  musl host. They are not distinguished by portability or by speed."
exp_note "  What separates them is SHAPE: pgb hands you one ordinary ELF with"
exp_note "  no interpreter that writes nothing and mounts nothing; the AppImage"
exp_note "  hands you a runtime that mounts or extracts and then runs a shell."
exp_note "  ⭐ And the AppImage stack reaches a class pgb does not: it bundles"
exp_note "  arbitrary shared libraries, so it packages software that cannot be"
exp_note "  statically linked at all -- which is precisely the criticism its"
exp_note "  own HOW-TO makes of the static approach."
exp_note ""
exp_note "⚠ One machine, one day. See the conditions block above."
exp_finish
