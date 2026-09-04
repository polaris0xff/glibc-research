---
tags:
  - how-to
  - install
  - dependencies
---

# Install Packages

This guide shows how to install one or more Linux packages into a local directory, producing a self-contained filesystem tree called a [rootfs](../appendix/system/rootfs.md). FlatRoot fetches each package directly from the distribution's servers and unpacks it into the directory you choose — no root privileges required, and nothing has to be set up on the host beforehand. The sections below start with a single package and work up to multi-architecture rootfses and incremental additions; for how FlatRoot decides what to pull in alongside what you asked for, see [Dependency Resolution](../explanation/dependency-resolution.md).

## Single package

This is the most basic form; it pulls curl and all its dependencies onto the `root` directory specified by the `--output` (1) option.
{ .annotate }

1. You can also use the abbreviated form, `-o`.

```bash
flatroot --from debian:bookworm install --output ./root curl
```

??? tip "Finding package names"

    Don't know the exact package name? Use [`flatroot search`](query-packages.md#search-packages) with a glob pattern (e.g. `'curl*'`) against the same `--from` source.

## Multiple packages

Dependencies are resolved across all packages together, avoiding duplicate downloads.

```bash
flatroot --from debian:bookworm install --output ./root python3 curl git
```

## Install by library or installed path

When you know the library a program failed to load or the concrete file you need on the rootfs — but not which package the distribution ships it in — name that instead with `--type`. The pattern resolves to the owning packages, the resolution is echoed on stderr, and the packages are installed with their full closure.

```bash
# The soname a program failed to load
flatroot --from debian:bookworm install -o ./root --type library 'libssl.so.3'

# A command, by its installed path (no leading slash)
flatroot --from debian:bookworm install -o ./root --type path 'bin/bash'

# An application, by the path of its main binary
flatroot --from debian:bookworm install -o ./root --type path 'usr/bin/gimp'
```

A pattern without wildcards matches exactly, an explicit wildcard installs every match, and a pattern matching nothing fails the run. `--type path` is unsupported on Alpine, which publishes no file lists; `--type library` works there through the `so:` provides.

### Pick one package when a path matches many

A path is usually shipped by several packages. On Debian, ten different mail servers all ship `usr/sbin/sendmail`, so installing by that path alone pulls in all ten:

```bash
flatroot --from debian:bookworm install -o ./root --type path usr/sbin/sendmail            # installs all 10 MTAs
```

To get just the one you want, add a second file that only that package ships and pass `--match all` — it keeps the package that ships *every* path you list:

```bash
flatroot --from debian:bookworm install -o ./root --type path --match all usr/sbin/sendmail usr/sbin/postfix   # installs postfix alone
```

The default (`--match any`) keeps every match; `--match all` is the opt-in for narrowing, and works the same for `--type library`. It is rejected for `--type package` — a package name already picks one package — and if no single package ships every path you list, the run fails and names the patterns.

## Supported Distributions

FlatRoot supports a variety of linux distributions depicted in the following snippet:

```bash
# Arch Linux
flatroot --from arch:rolling install --output ./arch firefox

# Alpine Linux
flatroot --from alpine:v3.21 install --output ./alpine curl

# CentOS 7
flatroot --from centos:7 install --output ./centos7 curl

# Fedora 42
flatroot --from fedora:42 install --output ./fedora42 firefox

# Ubuntu
flatroot --from ubuntu:noble install --output ./ubuntu-noble python3

# AlmaLinux 9
flatroot --from alma:9 install --output ./alma9 vim-enhanced

# Rocky Linux 9
flatroot --from rocky:9 install --output ./rocky9 git

# openSUSE Tumbleweed
flatroot --from opensuse:tumbleweed install --output ./suse curl

# CachyOS
flatroot --from cachyos:rolling install --output ./cachyos firefox
```

??? note "Distribution"

    The [glibc version table](../appendix/versioning/glibc.md) is a reference to help decide which release to target.

## Selecting the architecture

You can select a specific architecture with the `--arch` flag.

```bash
flatroot --from debian:bookworm --arch aarch64 install --output ./root bash
```

??? warning "Post-Installation on different architectures"

    Post-install runs the target architecture's own binaries (`ldconfig`, maintainer scripts) on the host, so they may not execute on a foreign architecture. Each phase is best-effort, though: a failure prints a warning and the install still completes. Pass `--postinstall=none` as an argument to `install` to skip the post-installation steps entirely.


## Include recommended packages

By default, FlatRoot only installs hard dependencies. Use `--with=recommends` to also install recommended packages (Debian `Recommends:` field):

```bash
flatroot --from debian:bookworm install --output ./root --with=recommends firefox-esr
```

## Include suggested packages

Use `--with=suggests` to also install suggested packages (Debian `Suggests:`, Arch `%OPTDEPENDS%`). Typically combined with `recommends`:

```bash
flatroot --from debian:bookworm install --output ./root --with=recommends,suggests firefox-esr
```

??? warning "Some sources do not honour `--with`"

    The RPM-family `primary.xml` parser reads `<provides>`, `<requires>`, `<conflicts>`, and `<obsoletes>` sections only — `<rpm:recommends>`, `<rpm:suggests>`, `<rpm:supplements>`, and `<rpm:enhances>` are not parsed and not stored in the index. The flag therefore has no effect against the RPM family (`--from fedora:*`, `centos:*`, `alma:*`, `rocky:*`, `opensuse:*`), nor against Alpine (`alpine:*`), whose `APKINDEX` carries no recommends/suggests at all. It works as documented for Debian, Ubuntu, Arch, and CachyOS.

## Skip post-install steps

`--postinstall <PHASE>` selects which post-install steps to run. The default is `ldconfig,scripts,hooks` (all three). The sentinel value `none` skips every phase, producing a faster but potentially incomplete rootfs — shared library cache, fonts, icons, and MIME types won't be configured:

```bash
flatroot --from arch:rolling install --output ./minimal --postinstall=none bash
```

To run a subset, list only the phases you want:

```bash
flatroot --from arch:rolling install --output ./minimal --postinstall=ldconfig bash
```

## Add a single package without dependencies

Use `--no-deps` to install only the named packages, skipping dependency resolution and base packages. Useful for adding a package to an existing rootfs:

```bash
flatroot --from debian:bookworm install --output ./root --no-deps htop
```

??? tip "Build a rootfs with only what you name"

    `--no-deps` installs exactly the packages you list — no dependency resolution, no base packages, no Essential set. Useful for minimal environments (`--no-deps libc6` lays down only glibc and its data files) or when you want every transitive dependency pinned by name rather than resolved. A hand-picked Debian set such as `libc6 libgcc-s1 libcrypt1 libtinfo6 ncurses-base base-files base-passwd bash dash` builds a runnable bash rootfs.

## Exclude specific packages

Use `--exclude` with a comma-separated list to drop packages from dependency resolution. Excluded packages and any dependencies unique to them are silently skipped. Useful for trimming transitive dependencies you know you don't need.

```bash
flatroot --from debian:bookworm install --output ./root --exclude perl curl
```

Against a stock Debian bookworm, this drops the `perl` chain (and any packages reachable only through it — `perl-modules-*`, `libperl*`, `libgdbm*`) from what a plain `install curl` would pull in via the Essential set. The resulting rootfs can still run `curl` but cannot execute package-manager postinst scripts that need `perl`.

## Parallel downloads

Use `-p` to control the number of concurrent downloads (default 4):

```bash
flatroot --from fedora:42 install --output ./root -p 8 firefox
```

## Retry attempts

Use `--http-retries` to set the maximum number of retry attempts for failed HTTP requests (default 3). Applies to both index fetching and package downloads:

```bash
flatroot --from centos:7 --http-retries 5 install --output ./root curl
```

## Adding to an existing rootfs

Installing into the same rootfs multiple times is additive. Before each install, flatroot reads the rootfs's per-package record (`.flatroot/packages`) and skips any package already recorded there at the same version and checksum — it is neither re-downloaded nor re-extracted. New packages introduced by the new invocation are fetched and extracted; and a recorded package whose version or checksum has moved (routine against rolling or unpinned sources, whose index refreshes hourly) is re-downloaded and re-extracted over its earlier files.

```bash
flatroot --from debian:bookworm install --output ./root bash
flatroot --from debian:bookworm install --output ./root python3
```

??? warning "Source changes are refused"

    Once a rootfs is built from a given source (e.g., `debian:bookworm@2024-06-15`), FlatRoot refuses further installs whose `--from` string differs in any way — a different distro (`ubuntu:noble`), a different release (`debian:trixie`), a different snapshot date (`debian:bookworm@2025-01-01`), or pinned vs unpinned of the same release (`debian:bookworm`). Mixing sources would interleave package versions from different points in time, with the same ABI, version-comparison, and checksum risks as a cross-distro switch. To start fresh, delete the target directory (or its `.flatroot/` subdirectory) and install again. See [Rootfs Metadata](../reference/metadata.md#cross-distro-protection) for the full rule.

## Cached downloads

Downloaded packages and indices are cached in `~/.cache/flatroot/`. Repeated installs of the same packages skip the download. Indices are cached for 1 hour. See [Cache reference](../reference/cache.md) for details.

--8<-- "_glossary.md"
