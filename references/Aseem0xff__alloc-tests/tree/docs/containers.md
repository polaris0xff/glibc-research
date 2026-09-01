# Containers: Docker and Podman

The container is not an implementation detail here — it is the environment the
question is about. This page covers checking whether a machine can run
containers at all, building the images, what is inside them, rebuilding after a
change, and the differences between the two runtimes.

---

## Can this machine run containers, and how do I start one

⭐ **Check in this order.** Each step distinguishes a different failure, and
skipping to the last one turns three separate problems into one confusing one.

```sh
# 1. Is a client installed at all?
command -v docker || command -v podman

# 2. Is there a DAEMON behind the client?  ⚠ THIS IS THE REAL TEST.
docker info                     # podman: `podman info`

# 3. What does the daemon say it can do?
docker version --format '{{.Server.Version}}'
docker info --format 'storage={{.Driver}} root={{.DockerRootDir}} cpus={{.NCPU}}'
```

⛔ **Never probe with `docker --version`.** A client binary with no daemon
answers it happily — it prints a version and exits 0 — and then every real
command fails. `alloc-bench doctor` uses `info` for exactly this reason, and so
should you.

**If `info` fails**, the message names which case you are in:

| what `docker info` says | what it means | what to do |
| --- | --- | --- |
| `command not found` | no client | install `docker`/`podman` |
| `Cannot connect to the Docker daemon at unix:///var/run/docker.sock` | client is there, daemon is not running | start it — see below |
| `permission denied ... /var/run/docker.sock` | daemon is up, your user is not in the `docker` group | `sudo usermod -aG docker "$USER"`, then re-login; or use `sudo`; or use rootless podman |
| it prints a report | ✅ you are fine | `alloc-bench doctor` |

**Starting the daemon.** With an init system, the one-liner is
`sudo systemctl start docker`. In a container, a CI runner or any environment
with no init — which is where this project was actually developed — start
`dockerd` directly and wait for the socket to appear rather than sleeping a
fixed number of seconds:

```sh
sudo dockerd >/tmp/dockerd.log 2>&1 &
for i in $(seq 1 30); do docker info >/dev/null 2>&1 && break; sleep 1; done
docker info >/dev/null || { echo "daemon did not come up"; tail -20 /tmp/dockerd.log; }
```

⚠ `dockerd` needs to create network bridges and mount cgroups, so it needs root
and a reasonably privileged environment. If it dies immediately, `/tmp/dockerd.log`
says why — usually cgroup v2 delegation or an unavailable `iptables`. **Podman
rootless is the better answer in a restricted environment**: no daemon, no
socket, no group membership. `alloc-bench --runtime podman` selects it.

⚠ **Nothing in this project needs `--privileged`** except cross-architecture
emulation. If a step seems to want it, that is a bug worth reporting rather than
a flag worth adding.

**If the daemon runs but builds cannot reach the network**, you are almost
certainly behind a TLS-terminating proxy — see *Networks that terminate TLS*
below, which is a supported configuration, not a workaround.

## What the images are

One per distribution, built from the upstream image:

| file | base | libc |
| --- | --- | --- |
| `images/alpine.Dockerfile` | `alpine:latest` | musl |
| `images/debian.Dockerfile` | `debian:latest` | glibc |
| `images/arch.Dockerfile` | `archlinux:latest` (x86_64) / `menci/archlinuxarm:base-devel` (aarch64) | glibc |

Each installs the distribution's own build toolchain **plus** one pinned Rust
toolchain and one pinned `zig`, both from `toolchains/pins.env`.

⭐ **Why a pinned Rust rather than the distribution's package.** The
distribution's Rust moves on its own schedule, so using it would make the Rust
version a hidden variable that differs per distribution — and a difference
between two rows would then be partly a difference between two compilers.

⭐ **Why `zig` is also installed.** It is the *control* for the distribution
axis. A static binary built with each distribution's own gcc embeds that gcc's
codegen; comparing three distributions compares three compilers as much as two
libcs. Selecting `TOOLCHAIN=zig` makes the C/C++ compiler identical everywhere.
⚠ zig is not required: if it cannot be installed the image still works and
records `zig_available=no`, and the zig cells report `unsupported` rather than
silently falling back to gcc.

Each image also records what it is, during the build, into
`/opt/alloc-tests/image-env.txt` — distribution, libc version, cc, cxx, ld, ar,
rustc, cargo, cmake, zig. The orchestrator reads that back rather than
re-deriving it, so the recorded values are the ones that were actually present.

⛔ **The instrument's selftest runs during the image build.** An image whose
instrument fails its own checks must not exist, because every number it later
produces comes from that instrument.

## Building them

Normally you do not: `alloc-bench run` builds what a plan needs and reuses it.
By hand:

```sh
docker build --platform linux/amd64 \
  --build-arg BASE_IMAGE=alpine:latest \
  -f images/alpine.Dockerfile -t alloc-tests/alpine-x86_64:local .
```

The build context is the repository root, because the image compiles the
instrument from `crates/`.

## The unit of reproduction

⭐ **One cell = one `docker run` of one script.** There is no CI-only step:

```sh
docker run --rm \
  -e CELL_ID=manual -e OUTDIR=/out -e CACHE=/cache \
  -e ALLOCATOR=mimalloc -e INTEGRATION=libc-surgery \
  -e PROFILE=static-pie -e TOOLCHAIN=distro -e LIBC=musl \
  -e TARGET_ARCH=x86_64 -e CORPUS_PROFILE=standard -e REPEAT=10 \
  -e ALLOC_REPO=https://github.com/microsoft/mimalloc \
  -e ALLOC_COMMIT=18b08671c9302247bfb682286e6bf3cc1773f801 \
  -e RG_COMMIT=e89fff89ac9af12e8d4ce9d5fd07beb408ca730f \
  -v "$PWD/out:/out" -v "$PWD/cache:/cache" \
  alloc-tests/alpine-x86_64:local \
  sh /opt/alloc-tests/scripts/build/run-cell.sh
```

⚠ **Bind-mount paths must be absolute.** A relative path is read by both
runtimes as a *named volume*, so the cell writes into a volume nobody reads and
every result comes back empty — which looks exactly like a build that produced
nothing. `alloc-bench` canonicalises them.

## Rebuilding an image with a different allocator

This is the `libc-surgery` mechanism, and it is the practical deliverable: an
image in which **every statically linked binary uses the new allocator, with no
build flags**.

```dockerfile
FROM alloc-tests/alpine-x86_64:local

RUN sh /opt/alloc-tests/scripts/build/fetch-source.sh \
      https://github.com/microsoft/mimalloc \
      18b08671c9302247bfb682286e6bf3cc1773f801 /work/mimalloc

RUN SRC=/work/mimalloc OUT=/opt/mimalloc MODE=override PIC=1 \
    LIBC=musl TARGET_ARCH=x86_64 CC=cc CXX=c++ \
    sh /opt/alloc-tests/allocators/mimalloc/build.sh

RUN sh /opt/alloc-tests/scripts/build/libc-surgery.sh \
      /opt/mimalloc/lib/liballocbench.a /usr/local/bin/alloc-runner
```

⛔ The surgery script **asserts** afterwards: exactly one archive member may
define `malloc` and exactly one `free`. It exits 1 otherwise, so a splice that
matched nothing cannot pass silently.

Verified on Alpine musl 1.2.6 (2026-09-01, `experiments/50-libc-surgery-verify.sh`):
13 members displaced from each of two `libc.a` copies — the distribution's and
Rust's self-contained one — with `malloc`, `free`, `calloc` and `realloc` each
defined exactly once afterwards, and `fork`, `printf` and `pthread_create` still
present.

⚠ **Do not skip Rust's copy.** Rust ships its own musl `libc.a` under
`$RUSTUP_HOME/toolchains/*/lib/rustlib/<target>/lib/self-contained/`. Patching
only `/usr/lib/libc.a` leaves every Rust musl build using the unpatched one.

## Podman

Everything above works with `podman` substituted for `docker`.
`alloc-bench --runtime podman` selects it, and with neither flag it takes
whichever answers `info`.

⚠ **The probe is `info`, not `--version`.** A client binary with no daemon
behind it answers `--version` happily and fails on the first real command.

Differences worth knowing:

| | Docker | Podman |
| --- | --- | --- |
| daemon | required | none (rootless by default) |
| `--platform` for another architecture | needs `binfmt_misc` (`docker run --privileged tonistiigi/binfmt --install all`) | needs `qemu-user-static` |
| bind-mount ownership | root-owned output | rootless maps to your uid — usually more convenient |
| `RepoDigests` on a locally built image | absent | absent |

Because a locally built image has no registry digest, `alloc-bench` records its
content `Id` instead and labels it `local:` — it pins the image for that run
without pretending to be a registry digest.

⚠ **No feature here needs `--privileged`, systemd, or host configuration**,
except cross-architecture emulation, which needs binfmt registered on the host.
Emulated runs are recorded and excluded from ranking.

## Networks that terminate TLS

Corporate egress proxies and some CI gateways re-sign TLS. Inside such a network
the first `apk`/`apt`/`pacman` call fails with `certificate verify failed`.

⛔ **The fix is never to disable verification.** Supply the CA:

```sh
cp /path/to/corporate-ca.pem images/extra-ca/proxy.crt
export ALLOC_TESTS_HTTPS_PROXY=http://proxy.internal:3128
alloc-bench run --suite smoke
```

Every image build appends any `.crt` in `images/extra-ca/` to its trust store
before any package manager runs. The directory is empty by default and the step
is a no-op without one.

⚠ **`ALLOC_TESTS_HTTPS_PROXY` is deliberately separate from the ambient
`HTTPS_PROXY`.** A host proxy is usually on `127.0.0.1`, which inside a
container is the container — inheriting it silently gives every network call a
connection refused that looks like an upstream outage.

## Disk

A full suite is tens of gigabytes: several images, eight allocator source trees,
a ripgrep checkout per cell and a 65 MB corpus. `alloc-bench doctor` warns below
20 GiB.

⚠ A run that dies on `ENOSPC` half way leaves a dataset that looks *partial*
rather than *failed*. The validator catches it — missing cells are an error —
but the cheaper fix is to check first.

```sh
docker system prune -af        # between full runs
rm -rf .cache/<distro>-<arch>  # forget cached allocator builds and sources
```

⚠ The allocator cache key includes allocator, commit, mode, PIC, libc,
architecture, toolchain and variant. If you change a *recipe* without changing
any of those, the stale archive is reused — delete the cache after editing a
recipe. Two of this project's early failures were exactly that.
