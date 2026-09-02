#!/bin/sh
# rootfs-run.sh - run a command inside an unpacked root filesystem, in a
# private mount namespace, with nothing of the host's userland visible.
#
# -- THE QUESTION THIS EXISTS TO ANSWER --------------------------------------
#
# docs/methodology/experiments.md: "Measure from outside the thing you are
# measuring", and this project's own rule is that runtime behaviour beats
# `file`/`ldd`/`readelf` labels. Both mean the same thing here: the only
# evidence that a binary runs on Alpine is the binary running on Alpine.
#
# This is the runner half of that bed. scripts/common/oci-pull.sh puts a distro
# root filesystem on disk; this enters it.
#
# -- WHY chroot AND NOT A CONTAINER RUNTIME ----------------------------------
#
# There is no container daemon on this machine. There IS root and
# CAP_SYS_ADMIN, so `unshare --mount` plus `chroot` gives the one property the
# experiments actually need: the target distro's /lib, /usr/lib, /etc and its
# loader are the ONLY ones on the filesystem the process can see.
#
# ⚠ WHAT THIS IS NOT. It is not a security boundary and it is not a container:
# the PID, network, user and IPC namespaces are shared with the host unless
# --private-net is passed, and the kernel is the host kernel. That last one is
# a real limit on what these experiments can conclude and it is stated in the
# write-ups: a chroot can falsify "runs on musl", it cannot test behaviour that
# depends on the target distro's kernel version.
#
# -- THE CONTAMINATION RULE --------------------------------------------------
#
# ⛔ NOTHING FROM THE HOST IS MOUNTED IN BY DEFAULT except /proc, /sys, /dev and
# a resolv.conf, each of which is a kernel or network interface rather than a
# userland one. A test that needs the artefact under test inside the target
# passes --copy, which COPIES it in, so the copy is visibly part of the
# experiment instead of an invisible bind mount that could be resolving to host
# libraries.
#
# Usage:
#   "./pgb" rootfs run ROOTFS -- /bin/sh -c 'echo hi'
#   "./pgb" rootfs run ROOTFS --copy ./prog:/prog -- /prog
#   "./pgb" rootfs run ROOTFS --bind /src:/src -- make
#   "./pgb" rootfs run ROOTFS --no-net -- /prog
#   "./pgb" rootfs run --selftest        prove the isolation, offline
#
# Exit codes: the command's own, or 2 if the runner could not start it.
# ⛔ Read the exit code unpiped: it is the command's, and that is the point.

set -u

ROOTFS=""
COPIES=""
BINDS=""
NET=1
PRIVNET=0
DNS=""
WD="/"
SELFTEST=0

usage() { awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"; }

while [ $# -gt 0 ]; do
  case "$1" in
    --copy)     shift; COPIES="$COPIES ${1:-}" ;;
    --bind)     shift; BINDS="$BINDS ${1:-}" ;;
    --no-net)   NET=0 ;;
    --private-net) PRIVNET=1 ;;
    --dns)      shift; DNS="${1:-}" ;;
    --workdir)  shift; WD="${1:-/}" ;;
    --selftest) SELFTEST=1 ;;
    -h|--help)  usage; exit 0 ;;
    --)         shift; break ;;
    -*)         printf 'rootfs-run: unknown argument: %s\n' "$1" >&2; exit 2 ;;
    *)          ROOTFS="$1" ;;
  esac
  shift
done

die() { printf 'rootfs-run: %s\n' "$*" >&2; exit 2; }

if [ "$SELFTEST" = 1 ]; then
  # ⚠ A POSITIVE CONTROL, not just an absence. docs/methodology/experiments.md:
  # "an absence is not a zero. A probe that found nothing may have been looking
  # in the wrong place." So the fixture plants a file the host does NOT have at
  # that path and asserts the chroot sees it, THEN asserts a host path that
  # certainly exists is invisible. One without the other proves nothing.
  command -v unshare >/dev/null 2>&1 || die "unshare is required"
  [ "$(id -u)" = 0 ] || die "selftest needs root"
  T=$(mktemp -d) || exit 2
  fails=0
  chk() { if [ "$2" = "$3" ]; then printf '  ok    %s = %s\n' "$1" "$2"; else printf '  FAIL  %s = %s, wanted %s\n' "$1" "$2" "$3"; fails=$((fails+1)); fi; }

  mkdir -p "$T/rootfs/bin" "$T/rootfs/marker"
  : > "$T/rootfs/marker/only-inside"
  # A statically linked shell is needed: the fixture rootfs has no loader.
  cat > "$T/tiny.c" <<'EOF'
#include <unistd.h>
#include <sys/stat.h>
int main(int argc, char **argv){ struct stat st;
  if (argc<2) return 2;
  return stat(argv[1], &st)==0 ? 0 : 1; }
EOF
  if cc -static -O0 -o "$T/rootfs/bin/exists" "$T/tiny.c" 2>/dev/null; then
    IN=$(sh "$0" "$T/rootfs" -- /bin/exists /marker/only-inside >/dev/null 2>&1; echo $?)
    OUTP=$(sh "$0" "$T/rootfs" -- /bin/exists /etc/hostname >/dev/null 2>&1; echo $?)
    chk sees-rootfs-only-path "$IN" 0
    chk host-etc-invisible    "$OUTP" 1
    HOSTSEE=$(sh "$0" "$T/rootfs" --copy "$T/rootfs/bin/exists:/copied" -- /copied /copied >/dev/null 2>&1; echo $?)
    chk copy-lands-inside     "$HOSTSEE" 0
  else
    printf '  SKIP  no static cc available, isolation cases not run\n'
    fails=$((fails+1))
  fi
  rm -rf "$T"
  if [ "$fails" = 0 ]; then printf 'rootfs-run --selftest: 3 cases, all pass.\n'; exit 0; fi
  printf 'rootfs-run --selftest: %s case(s) FAILED or skipped.\n' "$fails"; exit 1
fi

[ -n "$ROOTFS" ] || { usage >&2; exit 2; }
[ -d "$ROOTFS" ] || die "$ROOTFS is not a directory"
[ $# -gt 0 ] || die "no command given (use -- CMD ...)"
[ "$(id -u)" = 0 ] || die "needs root for mount/chroot"
command -v unshare >/dev/null 2>&1 || die "unshare is required"

ROOTFS=$(cd "$ROOTFS" && pwd) || die "cannot resolve $ROOTFS"

# Copies happen OUTSIDE the namespace so they persist and can be inspected
# after the run: an experiment's inputs should still be on disk afterwards.
for spec in $COPIES; do
  src=${spec%%:*}; dst=${spec#*:}
  [ "$src" = "$spec" ] && dst="/$(basename "$src")"
  mkdir -p "$ROOTFS$(dirname "$dst")" || die "cannot create $(dirname "$dst") in rootfs"
  cp -a "$src" "$ROOTFS$dst" || die "cannot copy $src -> $dst"
done

if [ "$NET" = 1 ]; then
  mkdir -p "$ROOTFS/etc"
  if [ -n "$DNS" ]; then
    printf 'nameserver %s\n' "$DNS" > "$ROOTFS/etc/resolv.conf"
  elif [ ! -s "$ROOTFS/etc/resolv.conf" ] && [ -s /etc/resolv.conf ]; then
    # The resolver config is a network interface, not host userland: without it
    # a DNS experiment inside measures the absence of a nameserver rather than
    # the behaviour of the resolver.
    cp /etc/resolv.conf "$ROOTFS/etc/resolv.conf" 2>/dev/null || true
  fi

  # ⚠ A TLS TRUST ANCHOR IS A NETWORK INTERFACE, NOT HOST USERLAND, and it is
  # replicated for the same reason /etc/resolv.conf is. Where the environment
  # routes HTTPS through a proxy, the variables naming its CA bundle are
  # inherited by anything run inside, but the FILE they name is not there, so
  # every fetch fails with "error setting certificate file" -- which reads as
  # "the network is down" and is not. The file is copied to the same absolute
  # path so the inherited variable keeps resolving.
  #
  # ⛔ ONLY the file those variables already name is copied. This does not
  # bring in the host's certificate store, and it adds no trust the caller's
  # own environment did not already have.
  for _cav in CURL_CA_BUNDLE SSL_CERT_FILE GIT_SSL_CAINFO REQUESTS_CA_BUNDLE NODE_EXTRA_CA_CERTS; do
    eval "_cap=\${$_cav:-}"
    [ -n "$_cap" ] && [ -f "$_cap" ] || continue
    [ -f "$ROOTFS$_cap" ] && continue
    mkdir -p "$ROOTFS$(dirname "$_cap")" 2>/dev/null || continue
    cp "$_cap" "$ROOTFS$_cap" 2>/dev/null || true
  done
fi

BINDSPEC=""
for spec in $BINDS; do
  BINDSPEC="$BINDSPEC $spec"
done

UNSHARE_ARGS="--mount --propagation private"
[ "$PRIVNET" = 1 ] && UNSHARE_ARGS="$UNSHARE_ARGS --net"

# The inner script runs with the mount namespace already private, so every
# mount below is torn down automatically when it exits, including on a crash.
export PGB_ROOTFS="$ROOTFS" PGB_BINDS="$BINDSPEC" PGB_WD="$WD"
exec unshare $UNSHARE_ARGS /bin/sh -c '
  set -e
  R="$PGB_ROOTFS"
  mkdir -p "$R/proc" "$R/sys" "$R/dev" "$R/tmp"
  mount -t proc  none "$R/proc"    2>/dev/null || true
  mount -t sysfs none "$R/sys"     2>/dev/null || true
  mount --rbind /dev "$R/dev"      2>/dev/null || true
  mount --make-rslave "$R/dev"     2>/dev/null || true
  mount -t tmpfs none "$R/tmp"     2>/dev/null || true
  for spec in $PGB_BINDS; do
    src=${spec%%:*}; dst=${spec#*:}
    [ "$src" = "$spec" ] && dst="$src"
    mkdir -p "$R$dst" 2>/dev/null || true
    mount --bind "$src" "$R$dst"
  done
  # ⛔ DO NOT ROUTE THROUGH THE TARGET`S /bin/sh UNCONDITIONALLY. A root
  # filesystem under test may not have one -- a distroless image does not, and
  # neither does a fixture built to hold exactly one binary. Going through a
  # shell that is not there turns every such run into exit 127, which reads as
  # "the binary failed" when it means "the runner could not start it". Only the
  # --workdir case needs a shell, and it says so by failing there alone.
  if [ "$PGB_WD" = "/" ]; then
    exec chroot "$R" "$@"
  else
    exec chroot "$R" /bin/sh -c '"'"'cd "$0" || exit 2; exec "$@"'"'"' "$PGB_WD" "$@"
  fi
' sh "$@"
