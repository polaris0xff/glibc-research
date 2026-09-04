#!/bin/sh
# THE QUESTION
#
#   docs/design/store-paths.md §3's comparison table has exactly ONE row marked
#   ⛔ NOT MEASURED: the interposer "works for a STATIC binary, or one issuing
#   RAW SYSCALLS -- no", because there is no PLT to win. Is that true, and is
#   it true for BOTH shapes or only one?
#
# ⛔ THAT ROW IS REASONING ABOUT A MECHANISM, NOT A RESULT, and store-paths.md
# says so itself: "Until somebody runs it, the honest statement is that the
# interposer has no PLT to win there and that nothing has confirmed the
# consequence." docs/research/app-corpus.md rung 2 is the same hole.
#
# -- ⭐ THE ROW NAMES TWO SHAPES AND THEY ARE DIFFERENT MECHANISMS -----------
#
# ⭐ "a static binary, OR one issuing raw syscalls" is written as one row and
# is two causes:
#
#   STATIC      there is no dynamic loader in the process at all, so LD_PRELOAD
#               never happens. Nothing of ours is mapped.
#   RAW SYSCALL the loader ran and our object IS mapped -- but the call leaves
#               through `syscall(SYS_openat, ...)` rather than through the PLT
#               entry we replaced, so the interposer is present and loses.
#
# ⛔ THOSE FAIL FOR DIFFERENT REASONS AND ONLY ONE OF THEM IS ABOUT LINKING.
# A subject can be dynamic and still defeat the interposer; a Go program is
# usually both at once, which is why measuring only Go would have conflated
# them. Each is a separate arm here.
#
# -- ⭐ TWO ARMS ------------------------------------------------------------
#
#   P  the PRELOAD arm. Planted probes against the real
#      tool/runtime/pgb-storefix.c and a real .storemap. No nix, no bed, no
#      display -- so it runs while a corpus holds the machine. It measures
#      the MECHANISM.
#   G  the GO arm. A real Go application out of a real nixpkgs closure,
#      bundled and run on all eleven. It measures the CONSEQUENCE.
#
#   sh scripts/common/run-experiment.sh 100        both arms
#   PGB_EXP100_ONLY=P sh experiments/100-static-payload.sh   the mechanism only
#
# -- ⛔ PRE-REGISTERED EXPECTATIONS, AND THE FAILURES ARE REGISTERED AS SUCH --
#
# ⛔ COMMITTED BEFORE THE RUN. PROGRESS.md delivery rule 1. app-corpus.md rung
# 2 says: "Expect it to FAIL, and pre-register that. The honest outcome is that
# the build REPORTS the compiled-in path and the program cannot resolve it. If
# it passes, the reasoning was wrong and the record says so."
#
#   S1  ⭐ THE POSITIVE CONTROL, AND EVERYTHING ELSE IS READ AGAINST IT: a
#       DYNAMIC probe that opens a compiled-in /nix/store path through fopen()
#       SUCCEEDS under the interposer. ⛔ If S1 fails, the interposer is not
#       working in this harness at all and S2/S3 are measuring nothing --
#       they would "confirm" the prediction for entirely the wrong reason.
#   S2  ⛔ PREDICTED TO FAIL: the SAME source built -static does NOT resolve
#       the path. No loader, so no LD_PRELOAD, so no interposer.
#   S3  ⛔ PREDICTED TO FAIL: a DYNAMIC probe issuing syscall(SYS_openat,...)
#       directly does NOT resolve it -- even though the interposer IS mapped
#       into that process. ⭐ This is the arm that separates the two causes.
#   S4  the interposer is genuinely LOADED in S1 and S3 and genuinely ABSENT
#       in S2, read from /proc/self/maps by the probe itself. ⛔ Without this
#       row, S2 and S3 are indistinguishable: both are "it did not work".
#   S5  ⭐ THE NEGATIVE CONTROL FOR THE HARNESS: the dynamic probe with NO
#       LD_PRELOAD fails too. That is what shows S1 passes BECAUSE of the
#       interposer rather than because the path happened to exist.
#   G1  a real Go subject (syncthing) BUILDS a bundle, and the build REPORTS
#       how many store paths are compiled in.
#   G2  ⚠ NOT PREDICTED EITHER WAY: whether it RUNS on 11 of 11. A Go program
#       having a store path in its .rodata does not mean it opens it -- it may
#       never need one at run time, in which case it runs perfectly and S2
#       still stands. ⛔ Recording a pass here as "the interposer worked"
#       would be wrong, and so would recording it as evidence against S2.
#   G3  the payload really is static / has no PT_INTERP, so G2 is about the
#       shape this experiment claims it is about.
#
# ⭐ WHAT A PASS ON S2 OR S3 WOULD MEAN: the reasoning in store-paths.md §3 was
# wrong and the row should be rewritten, not the run repeated.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "100 - the interposer against a STATIC and a RAW-SYSCALL payload"

WORK="${PGB_EXP100_WORK:-/var/tmp/t100}"
ONLY="${PGB_EXP100_ONLY:-PG}"
RUN_TIMEOUT="${PGB_EXP100_TIMEOUT:-150}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
command -v cc >/dev/null 2>&1 || { exp_note "no cc on PATH"; exit 2; }

ARMS_RUN=""

# ===========================================================================
# ARM P -- the mechanism, against the real interposer
# ===========================================================================
case "$ONLY" in *P*)
  ARMS_RUN="$ARMS_RUN P"
  AD="$WORK/AppDir"
  mkdir -p "$AD/lib" "$AD/store/data-1.0" || exit 2

  # ⭐ THE PLANTED STORE PATH. It does NOT exist on this machine -- that is the
  # point. The only way a probe reads the file is if something rewrote the
  # path to the AppDir's own copy.
  FAKE="/nix/store/00000000000000000000000000000000-data-1.0"
  printf 'the-real-payload\n' > "$AD/store/data-1.0/hello.txt"
  # <base><TAB><dir>, the format load_map() parses.
  printf '%s\t%s\n' "$FAKE" "$AD/store/data-1.0" > "$AD/.storemap"

  SFX="$REPO_DIR/tool/runtime/pgb-storefix.c"
  [ -f "$SFX" ] || { exp_note "missing $SFX"; exit 2; }
  # ⚠ the same command internal/bundle/storefix.go uses, so this arm is
  # measuring the shipped object rather than a differently-built one.
  cc -O2 -fPIC -shared -o "$AD/lib/libpgb-storefix.so" "$SFX" 2>"$WORK/cc.log" \
    || { exp_note "could not build the interposer; see $WORK/cc.log"; exit 2; }

  # ⭐ THE PROBE REPORTS THREE THINGS, and the third is what makes S2 and S3
  # distinguishable: whether our object is mapped into THIS process. Without
  # it both arms just say "did not resolve" and the two causes collapse.
  cat > "$WORK/probe.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#ifdef RAW_SYSCALL
#include <sys/syscall.h>
#endif

static int interposer_mapped(void)
{
    char line[4096];
    FILE *m = fopen("/proc/self/maps", "r");
    int found = 0;
    if (!m) return -1;
    while (fgets(line, sizeof line, m))
        if (strstr(line, "libpgb-storefix.so")) { found = 1; break; }
    fclose(m);
    return found;
}

int main(void)
{
    char buf[128];
    ssize_t n;
    int fd;
    const char *p = PROBE_PATH;

    printf("MAPPED=%d\n", interposer_mapped());
#ifdef RAW_SYSCALL
    /* ⭐ straight to the kernel: the PLT entry the interposer replaced is
     * never reached, even though the object IS in this process. */
    fd = (int)syscall(SYS_openat, AT_FDCWD, p, O_RDONLY);
#else
    fd = open(p, O_RDONLY);
#endif
    if (fd < 0) { printf("OPEN=fail\n"); return 1; }
    n = read(fd, buf, sizeof buf - 1);
    close(fd);
    if (n <= 0) { printf("OPEN=empty\n"); return 1; }
    buf[n] = '\0';
    if (buf[n-1] == '\n') buf[n-1] = '\0';
    printf("OPEN=ok CONTENT=%s\n", buf);
    return 0;
}
EOF

  PP="\"$FAKE/hello.txt\""
  cc -O2 -DPROBE_PATH="$PP"                -o "$WORK/probe-dyn"    "$WORK/probe.c" 2>>"$WORK/cc.log" || exit 2
  cc -O2 -DPROBE_PATH="$PP" -static        -o "$WORK/probe-static" "$WORK/probe.c" 2>>"$WORK/cc.log" || exit 2
  cc -O2 -DPROBE_PATH="$PP" -DRAW_SYSCALL  -o "$WORK/probe-raw"    "$WORK/probe.c" 2>>"$WORK/cc.log" || exit 2

  # run <tag> <binary> [preload]
  runp() {
    _t=$1; _b=$2; _pre=${3:-yes}
    if [ "$_pre" = yes ]; then
      env APPDIR="$AD" LD_PRELOAD="$AD/lib/libpgb-storefix.so" "$_b" \
        > "$WORK/$_t.out" 2>"$WORK/$_t.err"
    else
      env APPDIR="$AD" "$_b" > "$WORK/$_t.out" 2>"$WORK/$_t.err"
    fi
    printf '%s' $? > "$WORK/$_t.code"
  }
  got() { grep -m1 -oE "$2=[^ ]*" "$WORK/$1.out" 2>/dev/null | cut -d= -f2- || true; }

  printf '\n-- arm P: the mechanism ------------------------------------------\n'
  exp_note "the planted path is $FAKE/hello.txt and it does NOT exist on this machine"

  runp dyn    "$WORK/probe-dyn"
  runp stat   "$WORK/probe-static"
  runp raw    "$WORK/probe-raw"
  runp nopre  "$WORK/probe-dyn" no

  printf '        %-24s MAPPED=%-3s OPEN=%s\n' "dynamic + preload"   "$(got dyn MAPPED)"   "$(got dyn OPEN)"
  printf '        %-24s MAPPED=%-3s OPEN=%s\n' "static + preload"    "$(got stat MAPPED)"  "$(got stat OPEN)"
  printf '        %-24s MAPPED=%-3s OPEN=%s\n' "raw syscall + preload" "$(got raw MAPPED)" "$(got raw OPEN)"
  printf '        %-24s MAPPED=%-3s OPEN=%s\n' "dynamic, NO preload" "$(got nopre MAPPED)" "$(got nopre OPEN)"

  # ⭐ S1 FIRST. If the interposer does not win the case it is designed for,
  # nothing below it can be read as a boundary -- it would be read as one and
  # would actually be a broken harness. Same shape as experiments/65- C6.
  exp_check "S1  ⭐ CONTROL: dynamic + preload RESOLVES the path" \
    "$(got dyn CONTENT)" "the-real-payload"
  exp_check "S5  ⭐ CONTROL: the same probe with NO preload fails" \
    "$(got nopre OPEN)" fail

  exp_check "S2  ⛔ PREDICTED FAIL: -static does not resolve"   "$(got stat OPEN)" fail
  exp_check "S3  ⛔ PREDICTED FAIL: raw syscall does not resolve" "$(got raw OPEN)" fail

  # ⛔ and the two failures must have DIFFERENT causes, or the row that says
  # so is wrong. This is the whole reason MAPPED is reported.
  exp_check "S4  the interposer IS mapped in the dynamic probe"  "$(got dyn MAPPED)" 1
  exp_check "S4  ⭐ ...is ABSENT in the static one (no loader)"   "$(got stat MAPPED)" 0
  exp_check "S4  ⭐ ...and IS mapped in the raw-syscall one"      "$(got raw MAPPED)" 1
  exp_note "⭐ S4 is what makes S2 and S3 two findings instead of one: the"
  exp_note "   static probe fails because our object is NOT THERE, the"
  exp_note "   raw-syscall probe fails WITH IT LOADED. Only the first is about"
  exp_note "   linking, and store-paths.md §3 writes them as one row."
  ;;
esac

# ===========================================================================
# ARM G -- a real Go application out of a real closure
# ===========================================================================
ATTR="${PGB_EXP100_ATTR:-syncthing}"
PROG="${PGB_EXP100_PROG:-syncthing}"
ASSERT="${PGB_EXP100_ASSERT:-syncthing v[0-9]}"

case "$ONLY" in *G*)
  ARMS_RUN="$ARMS_RUN G"
  printf '\n-- arm G: a real Go subject --------------------------------------\n'
  IMG="$WORK/go.AppImage"; BLOG="$WORK/build-go.log"
  PGB_APPIMAGE_CACHE="$WORK/cache" "$REPO_DIR/pgb" bundle appimage "$ATTR" \
    --out "$IMG" --name "$PROG" >"$BLOG" 2>&1 || true

  if [ ! -s "$IMG" ]; then
    why=$(grep -aoE "nixpkgs has no attribute [^ ]*|no entry point in [^ ]*|could not fetch the closure[^\"]*|--name [^ ]* names no program" "$BLOG" 2>/dev/null | head -1)
    exp_note "⛔ UNRESOLVED: ${why:-see $BLOG}"
    exp_note "   Arm G did not run. A gap in this measurement, not evidence."
    ARMS_RUN=$(printf '%s' "$ARMS_RUN" | sed 's/ G//')
  else
    paths=$(grep -a -m1 '^store paths' "$BLOG" | sed 's/^store paths *//')
    exp_note "build reports store paths: ${paths:-(no line)}"
    exp_check "G1  the build REPORTS compiled-in store paths" \
      "$([ -n "$paths" ] && echo yes || echo no)" yes

    # G3 -- is the payload actually the shape this experiment is about?
    AD_G=$(grep -a -m1 -oE '/var/tmp/[^ ]*/AppDir' "$BLOG" | head -1)
    interp=unknown
    if [ -n "$AD_G" ] && [ -x "$AD_G/shared/bin/$PROG" ]; then
      if head -c 4096 "$AD_G/shared/bin/$PROG" | grep -qa 'ld-linux\|ld-musl'; then
        interp=dynamic
      else
        interp=static
      fi
    fi
    exp_note "the payload's shape reads as: $interp"
    exp_check "G3  the payload has no host interpreter path" "$interp" static

    ENVS=$(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt")
    ok=0; clean=0; rows=0
    for name in $ENVS; do
      root=$(exp_rootfs "$name") || true
      [ -n "$root" ] || { exp_skip "G/$name" "rootfs not fetched"; continue; }
      rows=$((rows+1))
      rm -f "$root/gosubj"; cp "$IMG" "$root/gosubj" 2>/dev/null; chmod +x "$root/gosubj"
      tr="$WORK/tr.g.$name"
      strace -f -e trace=openat,open,execve,clone,clone3,vfork -o "$tr" \
        timeout "$RUN_TIMEOUT" "$REPO_DIR/pgb" rootfs run "$root" -- \
          /bin/sh -c "APPIMAGE_EXTRACT_AND_RUN=1 /gosubj --version" \
        >"$WORK/out.g.$name" 2>"$WORK/err.g.$name"
      st=$?
      # ⚠ reap BEFORE reading: delivery rule 7, the FUSE/strace deadlock.
      for _p in /proc/[0-9]*; do
        _pid=${_p#/proc/}; _rt=$(readlink "/proc/$_pid/root" 2>/dev/null) || continue
        case "$_rt" in "$root"|"$root"/*) kill -9 "$_pid" 2>/dev/null ;; esac
      done
      all=$(cat "$WORK/out.g.$name" "$WORK/err.g.$name" 2>/dev/null)
      [ "$st" = 0 ] && printf '%s' "$all" | grep -qE "$ASSERT" && ok=$((ok+1))
      nh=$(exp_classify_trace "$tr" /gosubj | grep -c '^host ' || true)
      [ "$nh" = 0 ] && clean=$((clean+1))
      rm -f "$tr" "$root/gosubj"
    done
    # ⛔ G2 IS REPORTED, NOT PREDICTED. A Go program with a store path in its
    # .rodata need not ever OPEN it, so a pass here is not evidence against
    # S2 and a failure is not evidence for it. The number is the finding.
    exp_note "⭐ G2 (REPORTED, NOT PREDICTED): $ATTR ran on $ok of $rows"
    exp_note "   host-object-clean on $clean of $rows"
    exp_check "G2  the arm produced a number for every environment" "$rows" 11
    rm -f "$IMG"; rm -rf "$WORK/cache"
  fi
  ;;
esac

printf '\n'
exp_check "both arms ran (P = the mechanism, G = a real Go subject)" \
  "$(printf '%s' "$ARMS_RUN" | tr -d ' ')" "PG"
exp_note "⚠ Arm P measures ONE interposer against THREE call shapes on the"
exp_note "  build host. It says nothing about other architectures, and nothing"
exp_note "  about a payload that mixes shapes -- which a real Go binary does."

exp_finish
