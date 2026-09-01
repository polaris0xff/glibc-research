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
      cat > "$d/Dockerfile" <<EOF
FROM ${PGB_ENV_IMAGE}@${PGB_ENV_DIGEST}
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
