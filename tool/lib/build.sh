# tool/lib/build.sh -- part of `pgb`. Sourced by it, never executed.
#
# ⛔ SOURCED, NOT RUN. `pgb build` re-enters itself inside the build
# environment as `pgb __inner-build`, and `pgb verify` enters every target
# rootfs. Both depend on that being ONE process: a library executed as a child
# would put a shell between `pgb` and the thing it is measuring, and the
# PGB_OPT_* handoff in `../../pgb` exists precisely because that boundary is
# already where options got lost once. So: `. "$PGB_SELF/tool/lib/build.sh"`,
# no shebang, no `set -e`, no exec.
#
# ⚠ Every path here resolves from $PGB_SELF, which `../../pgb` sets from its
# own location. Nothing resolves from the caller's working directory.
#
# Holds: `pgb build`, `pgb shell`, the __inner-* re-entry points, and the
#        PGB_OPT_* export that carries options across the engine boundary.
#
# SPDX-License-Identifier: MIT

# ---------------------------------------------------------------------------
# build / shell
# ---------------------------------------------------------------------------
export_options() {
  export PGB_OPT_VERBOSE="$VERBOSE" PGB_OPT_EMBED_LOCALE="$EMBED_LOCALE" \
         PGB_OPT_USE_ICONV="$USE_ICONV" PGB_OPT_BASELINE="$ARCH_BASELINE" \
         PGB_OPT_BINDS="$EXTRA_BINDS" PGB_OPT_WRAP_DLOPEN="$WRAP_DLOPEN" \
         PGB_STATE="$PGB_STATE" PGB_LIBICONV_PREFIX="$PGB_LIBICONV_PREFIX"
}

cmd_build() {
  [ $# -gt 0 ] || die "pgb build needs a command, e.g. pgb build -- make" 2
  export_options
  eng=$(pick_engine)
  case "$eng" in
    chroot)
      r=$(env_root)
      [ -d "$r" ] || die "no build environment. run: pgb env create" 2
      wrk=$(pwd)
      bindargs=""
      for b in $EXTRA_BINDS; do
        bindargs="$bindargs --bind $(abs_bindspec "$b")"
      done
      # ⭐ The source tree is bind-mounted at the SAME path inside, so every
      # absolute path a build system bakes into a Makefile still resolves.
      exec sh "$PGB_SELF/scripts/common/rootfs-run.sh" "$r" \
           --bind "$wrk:$wrk" --bind "$PGB_SELF:$PGB_SELF" \
           --bind "$PGB_STATE:$PGB_STATE" $bindargs --workdir "$wrk" \
           -- /bin/sh -c 'PGB_INNER=1 "$0" __inner-build "$@"' "$PGB_SELF/pgb" "$@"
      ;;
    host)
      inner_build "$@"
      ;;
    docker|podman)
      $eng image inspect "pgb-env:$PGB_VERSION" >/dev/null 2>&1 || \
        die "no build environment image. run: pgb env create" 2
      wrk=$(pwd)
      dockerbinds=""
      for b in $EXTRA_BINDS; do
        dockerbinds="$dockerbinds -v $(abs_bindspec "$b")"
      done
      # The anchor again, at the same absolute path, so a variable the caller
      # exported keeps resolving inside. ca_anchor() carries the reasoning.
      _ca=$(ca_anchor); caargs=""
      [ -n "$_ca" ] && caargs="-v $_ca:$_ca:ro"
      # ⛔ ARGV IS PASSED AS ARGV, NEVER FLATTENED INTO A STRING.
      # This branch used to end `/bin/sh -c "$PGB_SELF/pgb __inner-build $*"`.
      # `$*` joins the arguments with spaces and the inner `sh -c` re-splits
      # them, so a single argument that CONTAINS spaces -- which is every
      # ordinary use, e.g.
      #     pgb build -- sh -c '$CC -O2 -o out/x x.c'
      # -- was torn into separate words. Measured: the container printed
      # `sh: 0: Illegal option -O` and produced no output file. The chroot
      # branch above never had this because it passes "$@" through.
      exec $eng run --rm -v "$wrk:$wrk" -v "$PGB_SELF:$PGB_SELF" $dockerbinds $caargs -w "$wrk" \
        -e PGB_INNER=1 "pgb-env:$PGB_VERSION" \
        /bin/sh -c 'PGB_INNER=1 "$0" __inner-build "$@"' "$PGB_SELF/pgb" "$@"
      ;;
  esac
}

inner_build() {
  rd=$(build_runtime)
  wd=$(make_wrappers)
  export PATH="$wd:$PATH"
  export CC="$wd/cc" CXX="$wd/c++"
  [ "$VERBOSE" = 1 ] && export PGB_VERBOSE=1
  vsay "wrappers: $wd"
  vsay "runtime:  $rd"
  "$@"
}

cmd_shell() {
  eng=$(pick_engine)
  r=$(env_root)
  case "$eng" in
    chroot)
      [ -d "$r" ] || die "no build environment. run: pgb env create" 2
      export_options
      wrk=$(pwd)
      exec sh "$PGB_SELF/scripts/common/rootfs-run.sh" "$r" \
           --bind "$wrk:$wrk" --bind "$PGB_SELF:$PGB_SELF" \
           --bind "$PGB_STATE:$PGB_STATE" --workdir "$wrk" \
           -- /bin/sh -c 'PGB_INNER=1 "$0" __inner-shell' "$PGB_SELF/pgb"
      ;;
    *) inner_build "${SHELL:-/bin/sh}" ;;
  esac
}
