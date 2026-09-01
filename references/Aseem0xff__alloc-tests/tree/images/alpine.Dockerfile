# Alpine benchmark image: musl libc, the primary target of this project.
#
# The base is passed in BY DIGEST by the orchestrator, never as `:latest`. A tag
# moves; a digest does not, and every result records the digest that produced it.
ARG BASE_IMAGE=alpine:latest
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

RUN apk add --no-cache \
        build-base cmake samurai git curl bash coreutils findutils grep sed tar xz \
        autoconf automake libtool binutils python3 linux-headers perl pkgconf file \
    && ln -sf /usr/bin/samu /usr/bin/ninja

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

# zig is a control, not a requirement: exit 2 means "unavailable here" and the
# image still works with the distribution's own gcc. Recorded either way.
RUN chmod +x scripts/build/*.sh \
    && ( sh scripts/build/install-zig.sh ./toolchains/pins.env > /opt/zig-version.txt 2>&1 \
         && echo "zig_available=yes" >> /opt/zig-version.txt ) \
    || echo "zig_available=no" >> /opt/zig-version.txt

# ⛔ Cargo.lock IS COPIED, and `--locked` enforces it.
#
# Without the lock the image re-resolves every dependency version at build
# time, so two images built a week apart embed different code in the
# instrument that takes every measurement. For a project whose entire point is
# pinning what produced a number, that was a hole.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY allocators ./allocators

# ⛔ The instrument's selftest RUNS during the image build. An image whose
# instrument fails its own checks must not exist: every number it later produces
# would come from that instrument.
RUN cargo build --release --locked -p alloc-runner \
    && install -m 0755 target/release/alloc-runner /usr/local/bin/alloc-runner \
    && chmod +x allocators/*/build.sh \
    && alloc-runner selftest

RUN { \
      echo "distro=alpine"; \
      echo "distro_version=$(cat /etc/alpine-release 2>/dev/null)"; \
      echo "libc=musl"; \
      echo "libc_version=$( (apk info -d musl 2>/dev/null | head -1) || echo unknown)"; \
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
    } > /opt/alloc-tests/image-env.txt && cat /opt/alloc-tests/image-env.txt

ENV ALLOC_TESTS_LIBC=musl \
    ALLOC_TESTS_DISTRO=alpine
