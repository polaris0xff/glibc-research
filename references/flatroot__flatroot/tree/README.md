<p align="center">
  <img src="docs/img/logo.svg" width="128" height="128" alt="FlatRoot logo">
</p>

<h1 align="center">FlatRoot</h1>

<p align="center">
  Build Linux root filesystem directories from official distribution packages — without root privileges or a running package manager.
</p>

---

[Abstract](#abstract) · [Quickstart](#quickstart) · [Introduction](#introduction) · [Related work](#related-work) · [Supported distributions](#supported-distributions) · [Usage](#usage) · [How it works](#how-it-works) · [Limitations](#limitations) · [License](#license)

## Abstract

Building a Linux root filesystem normally requires root privileges and the target distribution's own package manager, which couples every build to a matching host and makes results hard to reproduce or automate. FlatRoot removes both requirements: a single static CLI resolves a package set's dependency closure from the distribution's official mirrors, downloads and verifies the archives, extracts them in dependency order, and replays post-install scripts inside an unprivileged user-namespace sandbox. One binary covers ten distributions across the deb, rpm, pacman, and apk package formats. Debian, Ubuntu, and Arch builds can be pinned to historical archive snapshots, so the same command reproduces the same rootfs months later, and finished trees export to OCI images, tar archives, or compressed filesystems. FlatRoot is aimed at developers building application bundles, containers, and test environments who need rootfs trees from arbitrary distributions without root or a matching host system.

## Quickstart

Download the release binary for your architecture (`uname -m`; `x86_64`, `aarch64`, `armv7l`, `i686`, and `riscv64` are published). The binary is fully static and has no runtime dependencies; any Linux host works.

```bash
curl -Lo flatroot https://github.com/flatroot/flatroot/releases/latest/download/flatroot-linux-$(uname -m)
chmod +x flatroot
```

Build a minimal rootfs containing bash and its dependency closure:

```bash
./flatroot --from alpine:v3.21 install -o ./root bash
```

Expected output (package counts vary as the index updates):

```
Fetching package index for alpine:v3.21 (x86_64)...
  fetching https://dl-cdn.alpinelinux.org/alpine/v3.21/main/x86_64/APKINDEX.tar.gz
  fetching https://dl-cdn.alpinelinux.org/alpine/v3.21/community/x86_64/APKINDEX.tar.gz
Loaded 25391 packages
Resolved 9 packages
Downloaded 9 packages (0 already current)
Extracted 9 packages to ./root
ldconfig not found, skipping
Post-install scripts completed
Cache hooks completed
Done. 9 packages installed to ./root
```

The result is an ordinary directory: `./root/bin/bash` exists and the tree is ready to enter with any sandboxing tool (bwrap, chroot, a container runtime). See the [Your First Rootfs](https://flatroot.github.io/docs/master/tutorials/first-rootfs) tutorial and the [Get FlatRoot](https://flatroot.github.io/docs/master/how-to/get-flatroot) guide.

To build from source: `cargo build --release` (Rust with edition-2024 support).

## Introduction

Every Linux distribution publishes what a root filesystem is made of: versioned packages with dependency metadata, signed indices, and worldwide mirrors. But the standard way to consume that infrastructure is the distribution's own package manager, which brings terms of its own: it runs as root, it must exist on the host — tying every build to a host of the same package family — and it installs whatever the mirrors serve today, so the same command builds a different tree next month. The usual escape is to run that package manager inside a matching-distribution container, which only trades the host coupling for a container runtime and an image to pull.

FlatRoot consumes the same infrastructure directly. It parses the indices, resolves the dependency closure, and extracts the archives itself, so one static binary on any Linux host — no privileges, no package manager, no container runtime — turns a package list into an ordinary rootfs. Extraction alone is not enough — packages expect their maintainer scripts to run and their caches to exist — so FlatRoot replays those scripts inside an unprivileged user-namespace sandbox where they see the new tree as the whole system, and regenerates the runtime caches (fonts, icons, MIME, GSettings, CA bundles) that make the tree behave like an installed system rather than unpacked archives.

This project provides:

1. **One resolver over four package formats.** A breadth-first dependency resolver handles virtual packages, alternatives, version constraints, RPM rich (boolean) dependencies, and Alpine install-if triggers, on top of distro-agnostic parsers for deb, rpm, pacman, and apk archives.
2. **Unprivileged end-to-end operation.** Extraction needs no privileges, and post-install scripts run inside a user+mount-namespace sandbox, so maintainer scripts that assume root see the rootfs as the whole system while the host stays untouched.
3. **Snapshot-pinned reproducible builds.** A `@<date>` suffix on the source redirects index and package fetches to the distribution's historical archive for Debian, Ubuntu, and Arch.
4. **Declared-versus-linker dependency analysis.** The analyzer mode walks both the metadata dependency graph and the `DT_NEEDED` graph extracted from each package's ELF binaries, then merges them and flags packages whose binaries link against libraries the index never declared.
5. **Portable exports.** A finished tree repackages as an OCI image loadable by `docker load`/`podman load`, a tar.gz archive, or a DwarFS/SquashFS compressed filesystem.

## Related work

What sets FlatRoot apart is convergence: it brings together, in one static binary, capabilities that are otherwise spread across several distinct tool families — unprivileged operation, dependency resolution that needs no host package manager, coverage of ten distributions, snapshot-pinned reproducibility, and linker-level dependency analysis. Each family below covers part of that ground, and within its own niche several of them remain the better choice.

**Distro-native bootstrappers.** [debootstrap](https://wiki.debian.org/Debootstrap), [pacstrap](https://gitlab.archlinux.org/archlinux/arch-install-scripts), [`dnf --installroot`](https://dnf.readthedocs.io/en/latest/command_ref.html), [`zypper --root`](https://github.com/openSUSE/zypper), and [`apk --root`](https://gitlab.alpinelinux.org/alpine/apk-tools) assemble a rootfs with the distribution's own packaging stack, so each covers a single package family and most require the matching package manager on the host. The strongest of this family is [mmdebstrap](https://gitlab.mister-muffin.de/josch/mmdebstrap): it has a genuinely rootless user-namespace mode, supports multiple mirrors simultaneously, and writes directory, tar, squashfs, ext2, and ext4 outputs directly, making it the most mature choice for Debian-family trees.

**Multi-distro builders that wrap host package managers.** [mkosi](https://github.com/systemd/mkosi) covers more than a dozen distributions, runs unprivileged for many image types, pins snapshots for deb/rpm/pacman distros via its `Snapshot=` setting, and emits disk images, UKIs, and OCI layouts — but it is, by its own description, a wrapper around `dnf --installroot`, apt, pacman, and zypper: the distribution's package manager still performs the resolution and must be available to the build. [distrobuilder](https://github.com/lxc/distrobuilder) (LXC/Incus) reaches similar distro breadth through per-distro wrappers but requires root. FlatRoot's defining difference from both is that it resolves and fetches packages itself: the host needs nothing beyond the static binary.

**Package-manager-free reproducible builders.** These are the closest relatives in philosophy. [apko](https://github.com/chainguard-dev/apko) builds bit-for-bit reproducible OCI images directly from APK packages through a pure-Go reimplementation of apk — no Dockerfile, no container runtime, SBOM included — but it covers only APK-based ecosystems (Alpine, Wolfi) and emits only OCI/tar. [rules_distroless](https://github.com/bazel-contrib/rules_distroless) parses Debian's package index itself, fetches snapshot-pinned `.deb` files without apt or dpkg, and records a lockfile for reproducibility, but it is deb-only and bound to Bazel. [debuerreotype](https://github.com/debuerreotype/debuerreotype), builds rootfs tarballs against `snapshot.debian.org` timestamps and is the strongest precedent for FlatRoot's `@<date>` pinning, but it is Debian-only and built on debootstrap. FlatRoot generalizes what this family does for one format — independent resolution, snapshot pinning, reproducible output — across ten distributions and four package formats.

**Pinned environments and prebuilt-image runners.** [repro-env](https://github.com/kpcyrd/repro-env) locks exact package URLs and hashes across Debian, Arch, and Alpine, but it works inside a container and installs with each distro's own package manager. [proot-distro](https://github.com/termux/proot-distro), [Distrobox](https://github.com/89luca89/distrobox)/[Toolbox](https://github.com/containers/toolbox), and plain `docker pull` + `docker export` obtain a multi-distro tree rootlessly by downloading prebuilt images rather than resolving packages — exactly right when a vendor-maintained base image is what you want, but you take the image as published instead of naming the packages yourself. [Nix](https://nixos.org) and [Guix](https://guix.gnu.org) can also operate without root — Guix's build daemon runs unprivileged through Linux user namespaces, and rootless Nix mounts a user-owned store through a user-namespace chroot — and both are reproducible by design, yet they build their own package universe rather than rootfs trees from distribution mirrors.

**Dependency analysis.** What `analyze trace` does exists elsewhere only in pieces. [dpkg-shlibdeps](https://manpages.debian.org/dpkg-shlibdeps) and [RPM's elfdeps](https://rpm-software-management.github.io/rpm/man/rpm-dependency-generators.7) generate declared dependencies from `DT_NEEDED` at package build time; [adequate](https://manpages.debian.org/adequate) verifies that installed Debian binaries still resolve at install time; [lddtree](https://github.com/gentoo/pax-utils), [libtree](https://github.com/haampie/libtree), and [scanelf](https://github.com/gentoo/pax-utils) walk the ELF graph with no package metadata at all.

## Supported distributions

| Distribution | Source format | Example |
|-------------|--------------|---------|
| Debian | `debian:<release>[@<date>]` | `debian:bookworm`, `debian:buster@2023-01-01` |
| Ubuntu | `ubuntu:<release>[@<date>]` | `ubuntu:noble`, `ubuntu:focal@2024-06-15` |
| Arch Linux | `arch:<release>[@<date>]` | `arch:rolling`, `arch:rolling@2024-06-15` |
| CachyOS | `cachyos:rolling` | `cachyos:rolling` |
| Alpine Linux | `alpine:<version>` | `alpine:v3.21`, `alpine:edge` |
| CentOS/RHEL | `centos:<version>` | `centos:7`, `centos:stream9` |
| Fedora | `fedora:<release>` | `fedora:42`, `fedora:rawhide` |
| AlmaLinux | `alma:<version>` | `alma:8`, `alma:9` |
| Rocky Linux | `rocky:<version>` | `rocky:8`, `rocky:9` |
| openSUSE | `opensuse:<release>` | `opensuse:tumbleweed`, `opensuse:15.6` |

Target architectures use Linux kernel names (`uname -m`); the host architecture is detected automatically and `--arch` accepts a comma-separated list for multiarch builds.

| `--arch` | Debian | Ubuntu | Arch | CachyOS | Alpine | CentOS | Fedora | Alma | Rocky | openSUSE |
|----------|--------|--------|------|---------|--------|--------|--------|------|-------|----------|
| `x86_64` | Y | Y | Y | Y | Y | Y | Y | Y | Y | Y |
| `i686` | Y | Y | | | Y | | Y | | | |
| `aarch64` | Y | Y | | | Y | Y | Y | Y | Y | Y |
| `armv7l` | Y | Y | | | Y | | | | | |
| `riscv64` | Y | Y | | | Y | | | | | |

## Usage

### Install

```bash
# Multiple packages
flatroot --from fedora:42 install -o ./devel gcc gcc-c++ make

# Wine with 32-bit multiarch support, yields the binary `wine-stable` on the rootfs:
flatroot --from debian:bookworm --arch x86_64,i686 install -o ./wine wine

# CentOS 7 (glibc 2.17 for broad binary compatibility)
flatroot --from centos:7 install -o ./compat gcc make glibc-devel

# Add a single package to an existing rootfs without re-resolving dependencies
flatroot --from debian:bookworm install -o ./root --no-deps htop

# Install by library or installed path when the package name is unknown
flatroot --from debian:bookworm install -o ./root --type library 'libssl.so.3'
flatroot --from debian:bookworm install -o ./root --type path 'usr/bin/gimp'

# A path can be shipped by many packages: `usr/sbin/sendmail` is provided by every
# MTA (postfix, exim4, nullmailer, …). By default the owners are unioned, so this
# installs all of them:
flatroot --from debian:bookworm install -o ./root --type path usr/sbin/sendmail            # → all 10 MTAs

# `--match all` intersects the owners instead — keeping only the package that ships
# *every* listed path. Add a second path unique to the one you want, and it resolves
# to that package alone:
flatroot --from debian:bookworm install -o ./root --type path --match all usr/sbin/sendmail usr/sbin/postfix   # → postfix only
```

See [Install Packages](https://flatroot.github.io/docs/master/how-to/install-packages/) for soft dependencies (`--with recommends,suggests`), post-install phase selection (`--postinstall`), exclusions, and parallel downloads.

### Reproducible builds

Append a date to the source to pin the package index to a historical snapshot — the same command then produces the same rootfs regardless of when or where it runs. Pinning is supported for Debian, Ubuntu, and Arch Linux.

```bash
flatroot --from debian:bookworm@2024-06-15 install -o ./pinned bash
flatroot --from ubuntu:noble@2024-06-15 install -o ./pinned python3
flatroot --from arch:rolling@2024-06-15 install -o ./pinned firefox
```

Browse available dates: [snapshot.debian.org](https://snapshot.debian.org/archive/debian/), [archive.archlinux.org](https://archive.archlinux.org/repos/); Ubuntu serves any date from March 2023 onward. See the [tutorial](https://flatroot.github.io/docs/master/tutorials/reproducible-builds) and the [pinning guide](https://flatroot.github.io/docs/master/how-to/pinning).

### Query the package index

```bash
# List supported distribution backends / available releases
flatroot remote list
flatroot --from debian release list

# Search packages, libraries, or installed paths by glob pattern
flatroot --from debian:bookworm search 'firefox*'
flatroot --from debian:bookworm search --type library 'libssl.so*'
flatroot --from debian:bookworm search --type path 'bin/bash'

# `--match all` previews the disambiguation install uses: when several packages
# ship a path, it keeps only the one that ships every path you list
flatroot --from debian:bookworm search --type path --match all usr/sbin/sendmail usr/sbin/postfix   # → postfix

# Run SQL against the package index (from a file or stdin)
echo "SELECT name, version FROM packages WHERE essential = 1" | flatroot --from debian:bookworm query
```

See the [query guide](https://flatroot.github.io/docs/master/how-to/query-packages).

### Analyze dependency closures

`analyze trace` inspects a package before installing it: it walks the resolver's declared dependency graph and the linker's `DT_NEEDED` graph from the package's ELF binaries, merges the two, and flags packages whose binaries link against something the index does not declare.

```bash
$ flatroot --from debian:bookworm analyze trace bash
Analyzing bash 5.2.15-2+b10 from debian:bookworm (x86_64)
analyze.trace.1.depends_on.0=base-files
analyze.trace.1.depends_on.2=libc6
analyze.trace.1.name=bash
analyze.trace.1.reason=target
analyze.trace.1.sonames_consumed.0.binary=bin/bash
analyze.trace.1.sonames_consumed.0.provider=libtinfo6
analyze.trace.1.sonames_consumed.0.soname=libtinfo.so.6
...
```

`--type library` traces from a shared library glob back to whichever packages own it, and `--match all` — the same disambiguation flag `install` and `search` use — seeds the trace from only the package that owns every path or library you list:

```bash
flatroot --from debian:bookworm analyze trace --type path --match all usr/sbin/sendmail usr/sbin/postfix   # traces postfix only
```

See [CLI reference — analyze trace](https://flatroot.github.io/docs/master/reference/cli/#analyze-trace).

### Export

OCI and tar.gz are built in; DwarFS and SquashFS require `mkdwarfs` and `mksquashfs` on the host. The format is inferred from the output extension when unambiguous.

```bash
flatroot export --format oci -t myapp:v1.0 ./root root.tar   # docker load / podman load
flatroot export ./root root.tar.gz
flatroot export ./root root.dwarfs
flatroot export ./root root.sqfs
```

## How it works

1. **Fetch** — download and parse the distribution's package index into a SQLite database.
2. **Resolve** — walk the dependency graph breadth-first from the seed packages, handling virtuals, alternatives, version constraints, and conditional dependencies.
3. **Download** — check the local cache, fetch missing packages in parallel, verify checksums.
4. **Extract** — unpack archives into the rootfs in dependency order.
5. **Post-install** — run ldconfig, replay distro maintainer scripts inside the namespace sandbox, regenerate runtime caches (fonts, icons, MIME, GSettings, CA bundles).

Each rootfs carries a private `.flatroot/` metadata directory recording what was installed, so installs into an existing tree are incremental: packages whose version and checksum are already current are skipped, packages the index has since updated are re-extracted, and cross-distro installs into the same tree are refused. The directory is excluded from every export. See the [explanation](https://flatroot.github.io/docs/master/explanation/) section of the docs for the full architecture.

Package and index fetches go to the distributions' official mirrors, with automatic fallback to archive mirrors for EOL releases:

| Distribution | Primary mirror | Fallback | Snapshot |
|-------------|---------------|----------|----------|
| Debian | `deb.debian.org` | `archive.debian.org` | `snapshot.debian.org` |
| Ubuntu | `archive.ubuntu.com` | `old-releases.ubuntu.com` | `snapshot.ubuntu.com` |
| Arch Linux | `geo.mirror.pkgbuild.com` | — | `archive.archlinux.org` |
| CachyOS | `mirror.cachyos.org` + Arch repos | — | — |
| Alpine Linux | `dl-cdn.alpinelinux.org` | — | — |
| CentOS 7/8 | `vault.centos.org` | — | — |
| CentOS Stream 9 | `mirror.stream.centos.org` | — | — |
| Fedora | `dl.fedoraproject.org` | `archives.fedoraproject.org` | — |
| AlmaLinux | `repo.almalinux.org` | — | — |
| Rocky Linux | `dl.rockylinux.org` | — | — |
| openSUSE | `download.opensuse.org` | — | — |

## License

Apache License 2.0 — see [LICENSE](LICENSE).