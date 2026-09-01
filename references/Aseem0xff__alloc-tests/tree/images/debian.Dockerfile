# Debian benchmark image: glibc.
#
# glibc is the interesting contrast to Alpine, and not only because it is a
# different allocator. Statically linking glibc AND replacing its malloc is a
# materially harder problem than on musl -- glibc's malloc object also defines
# symbols the rest of libc needs, so deleting it from libc.a breaks the archive.
# The project runs the experiment anyway and records what happens; see
# docs/allocator-integration.md.
ARG BASE_IMAGE=debian:latest
FROM ${BASE_IMAGE}

# Optional trust anchor for networks that terminate TLS on the way out.
# A no-op when images/extra-ca/ holds no .crt -- see that directory's README.
COPY images/extra-ca /tmp/extra-ca
RUN set -eu; \
    for c in /tmp/extra-ca/*.crt; do \
      [ -e "$c" ] || continue; \
      echo "trusting extra CA: $c"; \
      cat "$c" >> /etc/ssl/certs/ca-certificates.crt; \
      mkdir -p /usr/local/share/ca-certificates; \
      cp "$c" /usr/local/share/ca-certificates/; \
    done; \
    rm -rf /tmp/extra-ca

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential cmake ninja-build git curl ca-certificates \
        autoconf automake libtool binutils python3 perl pkg-config \
        xz-utils file unzip \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/usr/local/zig:$PATH

WORKDIR /opt/alloc-tests
COPY toolchains ./toolchains
COPY scripts ./scripts

RUN . ./toolchains/pins.env \
    && curl --proto '=https' --tlsv1.2 -sSf --retry 5 https://sh.rustup.rs \
       | sh -s -- -y --no-modify-path --profile minimal --default-toolchain "${RUST_VERSION}" \
    && rustc --version && cargo --version

RUN chmod +x scripts/build/*.sh \
    && ( sh scripts/build/install-zig.sh ./toolchains/pins.env > /opt/zig-version.txt 2>&1 \
         && echo "zig_available=yes" >> /opt/zig-version.txt ) \
    || echo "zig_available=no" >> /opt/zig-version.txt

# bazelisk, for google/tcmalloc only. Pinned Bazel version comes from pins.env;
# Bazel 9 breaks that build.
RUN set -eux; \
    . ./toolchains/pins.env; \
    arch="$(uname -m)"; \
    case "$arch" in x86_64) b=amd64 ;; aarch64) b=arm64 ;; *) b="" ;; esac; \
    if [ -n "$b" ] && curl -fsSL --retry 5 -o /usr/local/bin/bazelisk \
        "https://github.com/bazelbuild/bazelisk/releases/latest/download/bazelisk-linux-$b"; then \
      chmod +x /usr/local/bin/bazelisk; \
      echo "USE_BAZEL_VERSION=$BAZEL_VERSION" > /opt/bazel-version.txt; \
    else \
      echo "bazelisk unavailable for $arch" > /opt/bazel-version.txt; \
    fi

# ⛔ Cargo.lock IS COPIED, and `--locked` enforces it.
#
# Without the lock the image re-resolves every dependency version at build
# time, so two images built a week apart embed different code in the
# instrument that takes every measurement. For a project whose entire point is
# pinning what produced a number, that was a hole.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY allocators ./allocators

RUN cargo build --release --locked -p alloc-runner \
    && install -m 0755 target/release/alloc-runner /usr/local/bin/alloc-runner \
    && chmod +x allocators/*/build.sh \
    && alloc-runner selftest

RUN { \
      echo "distro=debian"; \
      echo "distro_version=$(cat /etc/debian_version 2>/dev/null)"; \
      echo "libc=glibc"; \
      echo "libc_version=$(ldd --version 2>&1 | head -1)"; \
      echo "arch=$(uname -m)"; \
      echo "cc=$(cc --version | head -1)"; \
      echo "cxx=$(c++ --version | head -1)"; \
      echo "ld=$(ld --version | head -1)"; \
      echo "ar=$(ar --version | head -1)"; \
      echo "rustc=$(rustc --version)"; \
      echo "cargo=$(cargo --version)"; \
      echo "cmake=$(cmake --version | head -1)"; \
      echo "zig=$(head -1 /opt/zig-version.txt 2>/dev/null)"; \
      echo "zig_available=$(grep -o 'zig_available=.*' /opt/zig-version.txt | tail -1 | cut -d= -f2)"; \
      echo "bazel=$(cat /opt/bazel-version.txt 2>/dev/null | head -1)"; \
    } > /opt/alloc-tests/image-env.txt && cat /opt/alloc-tests/image-env.txt

ENV ALLOC_TESTS_LIBC=glibc \
    ALLOC_TESTS_DISTRO=debian
