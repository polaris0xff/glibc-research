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
# ⛔ ONE LIST, because the two places that use it drifted and the drift was
# invisible. `export_options` sets these for the re-entry, and the docker and
# podman branches of `cmd_build`/`cmd_shell` have to hand them across a
# CONTAINER boundary, which -- unlike chroot -- does not inherit the caller's
# environment. Deriving both from this variable is what stops one of them
# being extended and the other not.
PGB_OPT_VARS="PGB_OPT_VERBOSE PGB_OPT_EMBED_LOCALE PGB_OPT_EMBED_CACERT \
PGB_OPT_EMBED_TERMINFO \
PGB_OPT_USE_ICONV PGB_OPT_BASELINE PGB_OPT_BINDS PGB_OPT_WRAP_DLOPEN \
PGB_STATE PGB_LIBICONV_PREFIX"

export_options() {
  export PGB_OPT_VERBOSE="$VERBOSE" PGB_OPT_EMBED_LOCALE="$EMBED_LOCALE" \
         PGB_OPT_EMBED_CACERT="$EMBED_CACERT" \
         PGB_OPT_EMBED_TERMINFO="$EMBED_TERMINFO" \
         PGB_OPT_USE_ICONV="$USE_ICONV" PGB_OPT_BASELINE="$ARCH_BASELINE" \
         PGB_OPT_BINDS="$EXTRA_BINDS" PGB_OPT_WRAP_DLOPEN="$WRAP_DLOPEN" \
         PGB_STATE="$PGB_STATE" PGB_LIBICONV_PREFIX="$PGB_LIBICONV_PREFIX"
}

# -- ⛔ THE DEFECT THIS EXISTS TO FIX ---------------------------------------
#
# `chroot` inherits the caller's environment; a CONTAINER does not. The docker
# and podman branches passed exactly `-e PGB_INNER=1`, so every PGB_OPT_* was
# dropped at the boundary and EVERY BUILD OPTION SILENTLY DID NOTHING under
# those engines: --wrap-dlopen, --embed-locale, --no-iconv, --arch-baseline,
# and -v. Measured, same source, same command, engine the only variable:
#
#   chroot   __wrap_dlopen=1  pgb_dlopen_libs=1  size=2,453,656
#   docker   __wrap_dlopen=0  pgb_dlopen_libs=0  size=2,444,440
#
# ⚠ AND IT HID BEHIND A REAL RESULT. The two engines were measured
# BYTE-IDENTICAL and that measurement stands -- it was taken on a build with
# NO OPTIONS, which is the one case where dropping them all changes nothing.
#
# ⭐ `-e NAME` without `=VALUE` takes the value from the caller's environment,
# so a value containing spaces -- which PGB_OPT_WRAP_DLOPEN always has with
# more than one plugin -- cannot be torn apart by word splitting on the way.
# ⛔ Do NOT rewrite this as `-e NAME=$VALUE`; that is the same class of defect
# as the flattened argv this file already carries a warning about.
opt_env_args() {
  for _v in $PGB_OPT_VARS; do printf -- '-e %s ' "$_v"; done
}

cmd_build() {
  [ $# -gt 0 ] || die "pgb build needs a command, e.g. pgb build -- make" 2
  export_options
  eng=$(pick_engine)
  # ⛔ BEFORE anything is bind-mounted or any container starts. T-017: the two
  # engines keep independent environments, `pick_engine` may return a different
  # one than `pgb env create` built for, and the old failure was the missing
  # tool's own message from inside somebody else's build system.
  env_require_current "$eng"
  case "$eng" in
    chroot)
      r=$(env_root)
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
      # shellcheck disable=SC2046  # opt_env_args emits separate -e arguments
      exec $eng run --rm -v "$wrk:$wrk" -v "$PGB_SELF:$PGB_SELF" $dockerbinds $caargs -w "$wrk" \
        -e PGB_INNER=1 $(opt_env_args) "pgb-env:$PGB_VERSION" \
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
    docker|podman)
      # ⛔ THIS BRANCH DID NOT EXIST, and `pgb shell` fell through to
      # `inner_build` on the HOST. `pgb help` says "an interactive shell
      # inside it" -- inside the build environment -- and under the docker
      # engine it handed the caller a shell on this machine with the wrappers
      # on PATH, which is a different thing wearing the same name. Same defect
      # class as T-014: a documented capability quietly doing something else.
      env_require_current "$eng"
      export_options
      wrk=$(pwd)
      dockerbinds=""
      for b in $EXTRA_BINDS; do
        dockerbinds="$dockerbinds -v $(abs_bindspec "$b")"
      done
      # shellcheck disable=SC2046  # opt_env_args emits separate -e arguments
      exec $eng run --rm -it -v "$wrk:$wrk" -v "$PGB_SELF:$PGB_SELF" $dockerbinds -w "$wrk" \
        -e PGB_INNER=1 $(opt_env_args) "pgb-env:$PGB_VERSION" \
        /bin/sh -c 'PGB_INNER=1 "$0" __inner-shell' "$PGB_SELF/pgb"
      ;;
    *) inner_build "${SHELL:-/bin/sh}" ;;
  esac
}
