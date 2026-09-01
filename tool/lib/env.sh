# tool/lib/env.sh -- part of `pgb`. Sourced by it, never executed.
#
# ⛔ SOURCED, NOT RUN. `pgb build` re-enters itself inside the build
# environment as `pgb __inner-build`, and `pgb verify` enters every target
# rootfs. Both depend on that being ONE process: a library executed as a child
# would put a shell between `pgb` and the thing it is measuring, and the
# PGB_OPT_* handoff in `../../pgb` exists precisely because that boundary is
# already where options got lost once. So: `. "$PGB_SELF/tool/lib/env.sh"`,
# no shebang, no `set -e`, no exec.
#
# ⚠ Every path here resolves from $PGB_SELF, which `../../pgb` sets from its
# own location. Nothing resolves from the caller's working directory.
#
# Holds: `pgb env create` and `pgb env info` -- the OCI pull, the chroot
#        install, the image build, and the static libiconv inside each.
#
# SPDX-License-Identifier: MIT

# ---------------------------------------------------------------------------
# env
# ---------------------------------------------------------------------------
env_root() { printf '%s/%s' "$PGB_ROOTFS_DIR" "$PGB_ENV_NAME"; }

# ---------------------------------------------------------------------------
# The environment stamp: what an environment was BUILT FROM, in one line.
#
# ⛔ THE DEFECT THIS EXISTS TO CATCH, measured twice in the session of
# 2026-09-01: `pgb env create` builds an environment for whichever engine
# `pick_engine` returns AT THAT MOMENT, and a later `pgb build` calls
# `pick_engine` AGAIN. The two engines keep independent environments and
# nothing compared them, so a chroot environment rebuilt with a new
# PGB_ENV_PACKAGES sat beside a docker image carrying the old set and the
# failure was whatever the missing tool happened to say:
#
#     /bin/sh: 1: cmake: not found
#
# ⚠ AND IT IS WORSE THAN A STALE ENVIRONMENT. `pick_engine` prefers podman,
# then docker, then chroot, so MERELY STARTING `dockerd` changes which
# environment every subsequent command uses. Reproduced on this machine:
# `pgb doctor` reported `chosen engine: chroot`, one `dockerd` later the same
# command reported `chosen engine: docker`, with no other change.
#
# ⛔ THE FIX IS DETECTION, NOT BUILDING EVERY ENGINE. Three environments to use
# one is worse than the problem and the chroot one alone is ~1 GiB.
#
# ⭐ ONE function produces the stamp and one consumes it, so the writer and the
# checker cannot drift apart -- which is the way this defect would come back.
# Packages are sorted, so reordering PGB_ENV_PACKAGES is not a difference.
env_stamp() {
  _pk=$(printf '%s\n' $PGB_ENV_PACKAGES | LC_ALL=C sort -u | tr '\n' ' ' | sed 's/  */ /g; s/^ //; s/ $//')
  printf '%s@%s iconv=%s packages=[%s]' \
    "$PGB_ENV_IMAGE" "$PGB_ENV_DIGEST" "$USE_ICONV" "$_pk"
}

# What the environment an engine actually holds was built from, or empty when
# there is none. ⛔ EMPTY MEANS "NO ENVIRONMENT", NEVER "MATCHES".
env_stamp_of() {  # engine -> the stamp, or nothing
  case "$1" in
    chroot)
      _r=$(env_root)
      [ -d "$_r" ] || return 0
      if [ -f "$_r/.pgb-env-stamp" ]; then
        cat "$_r/.pgb-env-stamp"
        return 0
      fi
      # ⚠ An environment created before this stamp existed is NOT unusable and
      # must not be treated as a mismatch: `.pgb-env` already records the
      # image, the digest and the package set, so reconstruct from it. Only
      # `iconv` is not recorded there, and the archive's presence answers that.
      [ -f "$_r/.pgb-env" ] || return 0
      _i=$(sed -n 's/^image: //p'    "$_r/.pgb-env" | head -1)
      _d=$(sed -n 's/^digest: //p'   "$_r/.pgb-env" | head -1)
      _p=$(sed -n 's/^packages: //p' "$_r/.pgb-env" | head -1)
      _ic=0; [ -f "$_r$PGB_LIBICONV_PREFIX/lib/libiconv.a" ] && _ic=1
      _p=$(printf '%s\n' $_p | LC_ALL=C sort -u | tr '\n' ' ' | sed 's/  */ /g; s/^ //; s/ $//')
      printf '%s@%s iconv=%s packages=[%s]' "$_i" "$_d" "$_ic" "$_p"
      ;;
    docker|podman)
      "$1" image inspect --format '{{index .Config.Labels "org.pgb.stamp"}}' \
        "pgb-env:$PGB_VERSION" 2>/dev/null | sed 's/^<no value>$//'
      ;;
  esac
}

# ⛔ Refuse a build against an environment that is not what the current
# settings describe, and NAME the difference. The whole point of the entry is
# that the old failure was a missing tool's own error message, arriving deep
# inside somebody else's build system.
env_require_current() {  # engine
  _eng="$1"
  case "$_eng" in host) return 0 ;; esac
  _want=$(env_stamp)
  _got=$(env_stamp_of "$_eng")
  if [ -z "$_got" ]; then
    printf 'pgb: engine %s has no build environment.\n' "$_eng" >&2
    printf '     chosen engine: %s\n' "$_eng" >&2
    _other=""
    for _e in chroot docker podman; do
      [ "$_e" = "$_eng" ] && continue
      [ -n "$(env_stamp_of "$_e" 2>/dev/null)" ] && _other="$_other $_e"
    done
    [ -n "$_other" ] && \
      printf '     but these engines DO have one:%s -- pgb build --engine <one of those>\n' "$_other" >&2
    die "run: pgb env create   (or pgb build --engine ...)" 2
  fi
  [ "$_want" = "$_got" ] && return 0

  # ⛔ A DIFFERENCE IS NOT AUTOMATICALLY A PROBLEM, and the first version of
  # this function got that wrong in a way that broke a documented flag.
  # `--no-iconv` is a BUILD option, not an environment property: an environment
  # that HAS static libiconv serves an iconv build and a --no-iconv build
  # equally well, and refusing the second is a false positive. Only the other
  # direction is fatal. Measured -- `pgb build --engine chroot --no-iconv --
  # true` was refused against a perfectly good environment.
  #
  # ⭐ So each field is compared with the rule that field actually has:
  #   image/digest  differ at all  -> fatal, it is a different glibc
  #   packages      wanted MISSING -> fatal;  extra -> a note, not a refusal
  #   iconv         want 1, have 0 -> fatal;  want 0, have 1 -> nothing
  _fatal=0; _notes=""
  _wi=${_want%% *}; _gi=${_got%% *}
  if [ "$_wi" != "$_gi" ]; then
    _fatal=1
    _notes="$_notes     image    wanted $_wi
              have   $_gi
"
  fi
  _wc=$(printf '%s' "$_want" | sed -n 's/.*iconv=\([01]\).*/\1/p')
  _gc=$(printf '%s' "$_got"  | sed -n 's/.*iconv=\([01]\).*/\1/p')
  if [ "$_wc" = 1 ] && [ "$_gc" = 0 ]; then
    _fatal=1
    _notes="$_notes     iconv    this build links static libiconv and the environment has none
"
  fi
  _wp=$(printf '%s' "$_want" | sed -n 's/.*packages=\[\(.*\)\]$/\1/p' | tr ' ' '\n' | LC_ALL=C sort -u)
  _gp=$(printf '%s' "$_got"  | sed -n 's/.*packages=\[\(.*\)\]$/\1/p' | tr ' ' '\n' | LC_ALL=C sort -u)
  # ⛔ NOT `grep -vxF -e "$other"`: when the other side is EMPTY that is the
  # empty pattern, which matches every line, so `-v` drops everything and a
  # completely different package set reports NO difference at all. The failure
  # reads as "the environments agree", which is the one answer this function
  # must never give wrongly.
  _pkg_only_in_first() {  # first second -> words in first and not in second
    awk -v other="$2" 'BEGIN { n = split(other, o, "\n"); for (i = 1; i <= n; i++) O[o[i]] = 1 }
      $0 != "" && !($0 in O) { printf "%s ", $0 }' <<EOF
$1
EOF
  }
  _add=$(_pkg_only_in_first "$_wp" "$_gp" | sed 's/ *$//')
  _rem=$(_pkg_only_in_first "$_gp" "$_wp" | sed 's/ *$//')
  if [ -n "$_add" ]; then
    _fatal=1
    _notes="$_notes     packages MISSING from the environment: $_add
"
  fi

  if [ "$_fatal" = 0 ]; then
    # The environment is a superset of what this build needs. Say so once --
    # ⚠ silence would leave the operator unable to tell a checked build from
    # an unchecked one -- and carry on.
    [ -n "$_rem" ] && \
      vsay "environment has packages these settings do not name: $_rem"
    return 0
  fi

  printf 'pgb: the %s build environment cannot serve these settings.\n' "$_eng" >&2
  printf '%s' "$_notes" >&2
  [ -n "$_rem" ] && \
    printf '     (it also has, harmlessly: %s)\n' "$_rem" >&2
  # ⚠ Two different consequences, so two different sentences. A missing tool
  # makes the build FAIL, confusingly, inside somebody else's makefile. A
  # wrong image makes it SUCCEED against a glibc the pin does not describe,
  # which is worse and completely silent.
  if [ "$_wi" != "$_gi" ]; then
    printf '     The build would SUCCEED, against a glibc this pin does not\n' >&2
    printf '     describe -- which no output of it would ever show.\n' >&2
  else
    printf '     A build would fail inside your build system with whatever the\n' >&2
    printf '     missing tool says, so it is refused here instead.\n' >&2
  fi
  die "rebuild it: pgb env create   (chroot: delete $(env_root) first)" 2
}

cmd_env() {
  sub="${1:-info}"; shift 2>/dev/null || true
  case "$sub" in
    create) env_create ;;
    info)   env_info ;;
    *)      die "unknown: pgb env $sub" 2 ;;
  esac
}

env_info() {
  r=$(env_root)
  say "build environment"
  printf '  %-22s %s\n' "engine"        "$(pick_engine)"
  printf '  %-22s %s\n' "image"         "$PGB_ENV_IMAGE"
  printf '  %-22s %s\n' "digest"        "$PGB_ENV_DIGEST"
  printf '  %-22s %s\n' "packages"      "$PGB_ENV_PACKAGES"
  printf '  %-22s %s\n' "root"          "$r"
  if [ -d "$r" ]; then
    printf '  %-22s %s\n' "state" "created"
    if [ -f "$r/.pgb-env" ]; then sed 's/^/    /' "$r/.pgb-env"; fi
  else
    printf '  %-22s %s\n' "state" "NOT created -- run: pgb env create"
  fi
  say ""
  say "  why this image: glibc 2.36. At or above 2.34 the 'files' and 'dns'"
  say "  NSS services are implemented inside libc, which is what leaves the"
  say "  NSS override with nothing to dlopen. Below that floor it would move"
  say "  the dlopen rather than remove it -- see experiments/21."
}

env_create() {
  eng=$(pick_engine)
  r=$(env_root)
  case "$eng" in
    chroot)
      [ "$(id -u)" = 0 ] || die "the chroot engine needs root" 2
      if [ -d "$r" ] && [ -f "$r/.pgb-env" ]; then
        say "environment already at $r (delete it to rebuild)"; return 0
      fi
      say "creating $PGB_ENV_NAME from $PGB_ENV_IMAGE ($PGB_ENV_DIGEST)"
      sh "$PGB_SELF/scripts/common/oci-pull.sh" "$PGB_ENV_IMAGE" \
         --digest "$PGB_ENV_DIGEST" --out "$r" || die "image pull failed"
      say "installing: $PGB_ENV_PACKAGES"
      sh "$PGB_SELF/scripts/common/rootfs-run.sh" "$r" -- /bin/sh -c \
        "export DEBIAN_FRONTEND=noninteractive; apt-get update -qq && \
         apt-get install -y -qq --no-install-recommends $PGB_ENV_PACKAGES" \
        >"$r.install.log" 2>&1 || { tail -20 "$r.install.log" >&2; die "package install failed"; }
      # libiconv must be built by the ENVIRONMENT's compiler against the
      # ENVIRONMENT's glibc. An archive built on the host would link fine and
      # then carry the host's ABI assumptions into the pinned build.
      if [ "$USE_ICONV" = 1 ]; then
        say "building GNU libiconv inside the environment"
        mkdir -p "$r$PGB_LIBICONV_PREFIX"
        # ⚠ NOT /tmp: rootfs-run.sh mounts a tmpfs over it, so a file copied
        # to $r/tmp is invisible to the very command that needs it.
        mkdir -p "$r/opt"
        cp "$PGB_SELF/scripts/build-libiconv.sh" "$r/opt/pgb-build-libiconv.sh"
        sh "$PGB_SELF/scripts/common/rootfs-run.sh" "$r" -- /bin/sh -c \
          "export DEBIAN_FRONTEND=noninteractive; apt-get install -y -qq --no-install-recommends curl ca-certificates >/dev/null 2>&1; \
           PGB_LIBICONV_PREFIX=$PGB_LIBICONV_PREFIX sh /opt/pgb-build-libiconv.sh" \
          >>"$r.install.log" 2>&1 || { tail -20 "$r.install.log" >&2; die "libiconv build failed"; }
      fi
      {
        printf 'image: %s\n' "$PGB_ENV_IMAGE"
        printf 'digest: %s\n' "$PGB_ENV_DIGEST"
        printf 'packages: %s\n' "$PGB_ENV_PACKAGES"
        printf 'created: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        printf 'gcc: %s\n' "$(sh "$PGB_SELF/scripts/common/rootfs-run.sh" "$r" -- /usr/bin/gcc --version 2>/dev/null | head -1)"
        printf 'glibc: %s\n' "$(sh "$PGB_SELF/scripts/common/rootfs-run.sh" "$r" -- /bin/sh -c 'ldd --version' 2>/dev/null | head -1)"
      } > "$r/.pgb-env"
      # ⭐ The machine-readable half, written LAST so a half-built environment
      # has no stamp and is refused rather than trusted.
      env_stamp > "$r/.pgb-env-stamp"
      say "created. $(sed -n 's/^gcc: //p' "$r/.pgb-env")"
      ;;
    docker|podman)
      say "building image pgb-env:$PGB_VERSION with $eng"
      d="$PGB_STATE/docker"; mkdir -p "$d"
      rm -rf "$d/runtime" "$d/ca"; mkdir -p "$d/runtime" "$d/ca"
      cp "$PGB_SELF"/tool/runtime/* "$d/runtime/" 2>/dev/null
      cp "$PGB_SELF"/scripts/build-libiconv.sh "$d/" 2>/dev/null

      # The trust anchor, if the caller's environment names one. ca_anchor()
      # says why this is here and why it copies only that one file. The COPY
      # and the update-ca-certificates run are emitted ONLY when there is
      # something to copy, so the image is byte-identical to the old one on a
      # machine that does not terminate TLS.
      _ca=$(ca_anchor); _castep=""
      if [ -n "$_ca" ]; then
        cp "$_ca" "$d/ca/pgb-proxy-ca.crt" 2>/dev/null || die "cannot read CA anchor $_ca"
        say "carrying the trust anchor named by your environment: $_ca"
        _castep='COPY ca/pgb-proxy-ca.crt /usr/local/share/ca-certificates/pgb-proxy-ca.crt
RUN update-ca-certificates'
      else
        : > "$d/ca/.keep"
      fi

      # ⚠ ca-certificates has to be installed BEFORE the anchor is registered,
      # because update-ca-certificates ships with it. Both come before any
      # step that fetches over HTTPS.
      # ⭐ The same stamp, as a label, so `docker image inspect` answers the
      # question `.pgb-env-stamp` answers for the chroot arm. Written by the
      # same function, which is what keeps the two arms from drifting.
      cat > "$d/Dockerfile" <<EOF
FROM ${PGB_ENV_IMAGE}@${PGB_ENV_DIGEST}
LABEL org.pgb.stamp="$(env_stamp)"
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends \\
      ${PGB_ENV_PACKAGES} curl ca-certificates && rm -rf /var/lib/apt/lists/*
${_castep}
COPY runtime /opt/pgb/runtime
COPY build-libiconv.sh /opt/pgb/build-libiconv.sh
RUN sh /opt/pgb/build-libiconv.sh
EOF
      $eng build -t "pgb-env:$PGB_VERSION" "$d" || die "image build failed"
      say "built pgb-env:$PGB_VERSION"
      ;;
    host)
      say "engine 'host': nothing to create. ⚠ builds use THIS machine's glibc."
      ;;
  esac
}
