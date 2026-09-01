# tool/lib/common.sh -- part of `pgb`. Sourced by it, never executed.
#
# ⛔ SOURCED, NOT RUN. `pgb build` re-enters itself inside the build
# environment as `pgb __inner-build`, and `pgb verify` enters every target
# rootfs. Both depend on that being ONE process: a library executed as a child
# would put a shell between `pgb` and the thing it is measuring, and the
# PGB_OPT_* handoff in `../../pgb` exists precisely because that boundary is
# already where options got lost once. So: `. "$PGB_SELF/tool/lib/common.sh"`,
# no shebang, no `set -e`, no exec.
#
# ⚠ Every path here resolves from $PGB_SELF, which `../../pgb` sets from its
# own location. Nothing resolves from the caller's working directory.
#
# Holds: say/vsay/die/usage; bind-path and TLS-anchor resolution; engine
#        selection; `pgb doctor`.
#
# SPDX-License-Identifier: MIT

say()  { printf '%s\n' "$*"; }
vsay() { [ "$VERBOSE" = 1 ] && printf 'pgb: %s\n' "$*" >&2; return 0; }
# ⛔ "$1", NOT "$*". The second argument is the EXIT CODE, and `$*` printed it
# as part of the sentence: every `die "..." 2` in this tree ended
#     pgb: no build environment. run: pgb env create 2
# with a stray 2 that reads as part of the command the user is being told to
# run. Thirteen call sites, every one of them affected.
die()  { printf 'pgb: %s\n' "$1" >&2; exit "${2:-1}"; }
# ⚠ SAY IT AND CARRY ON. `warn` is for a thing that changed the build and did
# not stop it -- a dropped flag, a patch that would not apply -- and it goes to
# stderr so a caller capturing stdout still sees it. ⛔ It did not exist and
# `pgb nix` called it: the shell printed `pgb: 447: warn: not found` in the
# middle of a build and the real message was never printed at all.
warn() { printf 'pgb: %s\n' "$*" >&2; return 0; }
usage(){ awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"; }

# ⛔ A RELATIVE BIND SOURCE IS SILENTLY A NAMED VOLUME, NOT A DIRECTORY.
# `docker run -v relbind:/mnt` does not fail and does not mount `./relbind`:
# it creates an empty volume called `relbind` and mounts that, so the build
# sees an empty directory and reports whatever "the sources are not there"
# looks like in its own words. Reproduced on docker 29.3.1:
#
#   mkdir relbind && echo hi > relbind/marker.txt
#   docker run --rm -v relbind:/mnt alpine ls /mnt     # empty
#   docker run --rm -v ./relbind:/mnt alpine ls /mnt   # marker.txt
#
# The chroot engine has the same hazard by another route: rootfs-run.sh
# resolves the rootfs but not the bind sources. So every bind source is made
# absolute HERE, once, for every engine.
# references/Aseem0xff__alloc-tests/tree/docs/containers.md @ efc84ab5,
# "Bind-mount paths must be absolute".
abs_path() {
  case "$1" in
    /*) printf '%s' "$1" ;;
    *)  if [ -d "$1" ]; then (cd "$1" && pwd)
        else printf '%s/%s' "$(pwd)" "${1#./}"; fi ;;
  esac
}

# Split a bind spec SRC[:DEST] and re-emit it with both sides absolute.
abs_bindspec() {
  case "$1" in
    *:*) _bs=${1%%:*}; _bd=${1#*:} ;;
    *)   _bs=$1;       _bd=$1 ;;
  esac
  printf '%s:%s' "$(abs_path "$_bs")" "$(abs_path "$_bd")"
}

# ⚠ A TLS TRUST ANCHOR IS A NETWORK INTERFACE, NOT HOST USERLAND.
# scripts/common/rootfs-run.sh replicates it into the chroot and says why; the
# docker and podman engines did not, so they were asymmetric with the chroot
# engine on the one seam docs/history/corrections.md C3 had already paid for.
# Measured: the FIRST run of `pgb --engine docker env create` in an
# environment that terminates TLS died at
#   RUN sh /opt/pgb/build-libiconv.sh -> exit code 60
# which is curl's "SSL certificate problem: unable to get local issuer
# certificate". `apt-get` in the same image had just succeeded, because
# Debian's default sources are http, so the failure reads as "libiconv is
# broken" rather than "this container has no trust anchor".
#
# ⛔ ONLY the file the caller's own variables already name is carried in.
# Nothing else is trusted, and verification is never disabled --
# references/Aseem0xff__alloc-tests/tree/docs/containers.md @ efc84ab5,
# "The fix is never to disable verification. Supply the CA."
ca_anchor() {
  for _cav in CURL_CA_BUNDLE SSL_CERT_FILE GIT_SSL_CAINFO REQUESTS_CA_BUNDLE NODE_EXTRA_CA_CERTS; do
    eval "_cap=\${$_cav:-}"
    [ -n "${_cap:-}" ] && [ -f "$_cap" ] && { printf '%s' "$_cap"; return 0; }
  done
  printf ''
}

default_baseline() {
  case "$(uname -m)" in
    x86_64|amd64)  printf 'x86-64' ;;
    aarch64|arm64) printf 'armv8-a' ;;
    *)             printf '' ;;
  esac
}

# ---------------------------------------------------------------------------
# doctor
# ---------------------------------------------------------------------------
cmd_doctor() {
  rc=0
  say "pgb $PGB_VERSION -- host report"
  say ""
  printf '  %-34s %s\n' "kernel"        "$(uname -sr)"
  printf '  %-34s %s\n' "architecture"  "$(uname -m)"
  [ -r /etc/os-release ] && printf '  %-34s %s\n' "distribution" \
      "$(. /etc/os-release 2>/dev/null; printf '%s' "${PRETTY_NAME:-unknown}")"
  printf '  %-34s %s\n' "host libc"     "$(ldd --version 2>&1 | head -1)"
  printf '  %-34s %s\n' "cc"            "$({ ${CC:-cc} --version 2>/dev/null || echo none; } | head -1)"
  say ""

  chk() { # label ok-condition-result advice
    if [ "$2" = yes ]; then printf '  ok    %-32s\n' "$1"
    else printf '  MISS  %-32s %s\n' "$1" "$3"; rc=1; fi
  }
  chk "a C compiler"       "$(command -v ${CC:-cc} >/dev/null 2>&1 && echo yes || echo no)" "install gcc or clang"
  chk "static linking"     "$(printf 'int main(void){return 0;}\n' > /tmp/.pgb$$.c 2>/dev/null &&
                              ${CC:-cc} -static -o /tmp/.pgb$$ /tmp/.pgb$$.c 2>/dev/null && echo yes || echo no)" \
                           "install libc6-dev / glibc-static"
  rm -f /tmp/.pgb$$ /tmp/.pgb$$.c
  LIBC_A=$(${CC:-cc} -print-file-name=libc.a 2>/dev/null)
  chk "__nss_configure_lookup in libc.a" \
      "$([ -f "$LIBC_A" ] && nm -A "$LIBC_A" 2>/dev/null | grep -q 'T __nss_configure_lookup' && echo yes || echo no)" \
      "the NSS fix needs a glibc libc.a"
  # ⛔ THE ROWS ABOVE DESCRIBE THIS MACHINE, AND THAT IS ONLY THE RIGHT ANSWER
  # FOR THE `host` ENGINE. libiconv is built INSIDE the build environment by
  # `pgb env create`, because it has to be compiled by the environment's own
  # compiler against the environment's own glibc. Probing the host path while
  # the chosen engine is chroot reported
  #
  #     MISS  GNU libiconv (static)   run scripts/build-libiconv.sh
  #
  # on a machine where `pgb build` worked perfectly and the archive was sitting
  # in the chroot at $r/opt/pgb-libiconv/lib/libiconv.a. Measured on a freshly
  # bootstrapped machine, session of 2026-09-01b. ⚠ A MISS for something that
  # is not missing where it is used sends the reader to fix a working tool.
  _doc_eng=$(pick_engine)
  if [ "$_doc_eng" = chroot ] && [ -d "$(env_root)" ]; then
    chk "GNU libiconv (static), in the chroot environment" \
        "$([ -f "$(env_root)$PGB_LIBICONV_PREFIX/lib/libiconv.a" ] && echo yes || echo no)" \
        "run: pgb env create"
  elif [ "$_doc_eng" = docker ] || [ "$_doc_eng" = podman ]; then
    printf '  --    %-32s %s\n' "GNU libiconv (static)" \
           "inside the $_doc_eng image; pgb build checks it"
  else
    chk "GNU libiconv (static), on this machine" \
        "$([ -f "$PGB_LIBICONV_PREFIX/lib/libiconv.a" ] && echo yes || echo no)" \
        "run scripts/build-libiconv.sh"
  fi
  say ""
  say "  build environment engines:"
  for e in docker podman; do
    if command -v $e >/dev/null 2>&1 && $e info >/dev/null 2>&1; then
      printf '    ok    %-10s usable\n' "$e"
    elif command -v $e >/dev/null 2>&1; then
      printf '    --    %-10s present but no daemon\n' "$e"
    else
      printf '    --    %-10s absent\n' "$e"
    fi
  done
  if [ "$(id -u)" = 0 ] && command -v unshare >/dev/null 2>&1 &&
     unshare --mount --propagation private true 2>/dev/null; then
    printf '    ok    %-10s usable (root + CAP_SYS_ADMIN)\n' chroot
  else
    printf '    --    %-10s needs root and CAP_SYS_ADMIN\n' chroot
  fi
  printf '    ok    %-10s always available, but see the warning below\n' host
  say ""
  say "  chosen engine: $(pick_engine)"
  if [ "$(pick_engine)" = host ]; then
    say ""
    say "  ⚠ THE host ENGINE BUILDS AGAINST WHATEVER GLIBC THIS MACHINE HAS."
    say "    That is not a controlled environment: the binary inherits this"
    say "    host's glibc version, its headers and its CPU defaults. Use it to"
    say "    experiment, not to ship. \`pgb env create\` gives the pinned one."
  fi
  say ""
  say "  target root filesystems for \`pgb verify\`:"
  n=0
  if [ -f "$PGB_SELF/scripts/common/rootfs-images.txt" ]; then
    while read -r ref name libc digest; do
      case "$ref" in ''|\#*) continue ;; esac
      if [ -d "$PGB_ROOTFS_DIR/$name" ]; then n=$((n+1)); fi
    done < "$PGB_SELF/scripts/common/rootfs-images.txt"
  fi
  printf '    %s present under %s\n' "$n" "$PGB_ROOTFS_DIR"
  [ "$n" = 0 ] && say "    run: sh scripts/common/fetch-rootfs.sh"
  return $rc
}

pick_engine() {
  if [ -n "$ENGINE" ]; then printf '%s' "$ENGINE"; return; fi
  if command -v podman >/dev/null 2>&1 && podman info >/dev/null 2>&1; then printf 'podman'; return; fi
  if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then printf 'docker'; return; fi
  if [ "$(id -u)" = 0 ] && command -v unshare >/dev/null 2>&1; then printf 'chroot'; return; fi
  printf 'host'
}
