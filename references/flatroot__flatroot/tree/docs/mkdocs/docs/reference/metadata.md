---
tags:
  - reference
  - metadata
  - reproducibility
---

# Metadata Reference

Every rootfs FlatRoot builds contains a `.flatroot/` directory at its root. This page documents the on-disk format, field specifications, checksum algorithms, and reinstallation behaviour. For the design rationale (why metadata is recorded at install time, why it is excluded from exports, why the format is plain text), see [The metadata layer](../explanation/the-metadata-layer.md).

## Location

```
<root>/.flatroot/
  manifest                    # top-level rootfs identity
  packages                    # per-package records
  files/
    <name>:<arch>             # one file per installed package
  scripts/
    <name>:<arch>/            # staged post-install scripts (persisted, never pruned)
  bin/                        # stub programs placed on PATH during post-install (persisted)
```

All text files use UTF-8 encoding, `\n` line endings, and carry no trailing newline at end-of-file. Record ordering is byte-ascending (equivalent to `LC_ALL=C` sort).

## Manifest

### Path

`<root>/.flatroot/manifest`

### Format

A single dpkg-style record (`Key: Value`) with fixed key order, written with no trailing newline — consistent with the end-of-file rule above.

### Fields

| Key | Description |
|-----|-------------|
| `FlatrootVersion` | Version of the FlatRoot binary that wrote this file, from `CARGO_PKG_VERSION`. |
| `Sources` | Deduplicated union of distro source strings across all installed packages. Comma-space separated, byte-ascending. |
| `Architectures` | Deduplicated union of architecture strings across all installed packages. Comma-space separated, byte-ascending. |
| `PackageCount` | Number of records in the `packages` file. |

### Example

```
FlatrootVersion: 0.1.0
Sources: debian:bookworm@2026-04-21
Architectures: i686, x86_64
PackageCount: 247
```

---

## Packages

### Path

`<root>/.flatroot/packages`

### Format

Dpkg-style records (`Key: Value`), one per installed `(name, architecture)` pair. Records are separated by exactly one blank line. Sorted by `(name, architecture)` byte-ascending. Every record has the same set of keys in fixed order.

### Fields

| Key | Description |
|-----|-------------|
| `Package` | Package name as published by the upstream index. |
| `Version` | Upstream version string, verbatim. Interpretation requires knowing the `Source:` (for its version-comparison algorithm). |
| `Architecture` | Target architecture in Linux kernel naming (`x86_64`, `i686`, `aarch64`, etc.). |
| `Source` | Full source string from `--from`, including any `@date` suffix. |
| `Url` | Fully qualified URL of the archive on the upstream mirror. |
| `Checksum` | Typed hash in `<algo>:<hex>` form. See [Checksum algorithms](#checksum-algorithms). |
| `Size` | Archive size in bytes. `0` means the index did not publish a size. |
| `Depends` | Flattened hard dependency list in dpkg syntax. May be empty, but the key is always present. |

### Example

```
Package: libc6
Version: 2.36-9+deb12u7
Architecture: x86_64
Source: debian:bookworm@2026-04-21
Url: https://snapshot.debian.org/archive/debian/20260421T000000Z/pool/main/g/glibc/libc6_2.36-9+deb12u7_amd64.deb
Checksum: sha256:a4f8e6c1f0b4c7d3e9a1b8c4f7e0d3a6b9c2e5f8a1d4b7e0c3f6a9d2b5e8c1f4
Size: 2700000
Depends: libgcc-s1, libcrypt1 (>= 1:4.1.0)
```

### Dependency Syntax

The `Depends` field flattens FlatRoot's in-memory dependency representation to dpkg syntax, regardless of the source distribution:

- Groups (hard dependencies) are separated by `,` and a space.
- Alternatives within a group are separated by ` | `.
- Version constraints are written as ` (<constraint>)` after the name.

For distros whose native indices carry information that does not map cleanly to package names — RPM file-path deps like `/bin/sh`, apk sonames like `so:libc.musl-x86_64.so.1` — the raw token is preserved verbatim.

### Checksum Algorithms

| Distros | Algorithm | Prefix | Scope |
|---------|-----------|--------|-------|
| Debian, Ubuntu, Arch, CachyOS, CentOS, Fedora, AlmaLinux, Rocky | SHA-256 | `sha256:` | Full archive |
| openSUSE | SHA-512 | `sha256:` | Full archive |
| Alpine | Q1+base64 SHA-1 | `q1-sha1:` | Control section only |

The Alpine checksum is not interchangeable with a plain `sha1:` hash — it is a base64-encoded SHA-1 of the package's control section, prefixed with `Q1`.

The openSUSE prefix is `sha256:` even though the recorded digest is a SHA-512 hash: the prefix is taken from the package format's declared scheme (shared by every RPM-family distro), while download verification infers the true algorithm from the digest's length.

---

## Filelists

### Path

`<root>/.flatroot/files/<name>:<arch>`

### Format

Plain text, one absolute path per line, sorted byte-ascending. Includes regular files, directories, and symlinks written during extraction. Excludes entries under `.flatroot/` — those are FlatRoot metadata, not package payload.

### Example

`files/bash:x86_64`:

```
/bin/bash
/etc/bash.bashrc
/etc/skel/.bash_logout
/etc/skel/.bashrc
/etc/skel/.profile
/usr/share/doc/bash/copyright
/usr/share/man/man1/bash.1.gz
```

---

## Cross-Distro Protection

On every `flatroot install` against an existing rootfs, FlatRoot reads the existing `manifest` file and compares every entry in `Sources` against the incoming `--from` string byte-for-byte. Any mismatch aborts the install before any network access. The check is strict: a rootfs is scoped to one exact source string for its lifetime.

### Error Shape

```
Error: cannot install from 'ubuntu:noble' into rootfs at /tmp/rootfs
  existing source: debian:bookworm@2026-04-21
  a rootfs is scoped to one exact source — different distros, releases, snapshot dates, and pinned vs unpinned variants are all refused
  to start fresh, remove /tmp/rootfs/.flatroot/ or use a different directory
```

### Allowed Operations

| Operation | Allowed? | Reason |
|-----------|----------|--------|
| Reinstallation (same `--from`) | Yes | Source strings match exactly |
| Multi-arch within one `--arch` invocation | Yes | One `--from` produces one source string across every per-arch record |
| Additional architecture via a second invocation with same `--from` | Yes | Source strings match exactly; only `--arch` differs |
| Different release, same distro (`debian:bookworm` → `debian:trixie`) | **No** | Source strings differ; would mix package universes across release generations |
| Different `@date`, same release (`debian:bookworm@2025-01-01` → `@2025-06-01`) | **No** | Source strings differ; would mix snapshot dates |
| Pinned vs unpinned of same release (`debian:bookworm` → `debian:bookworm@2024-06-15`) | **No** | Source strings differ; pinning is part of the rootfs identity |
| Different distro prefix (`debian:*` → `ubuntu:*`) | **No** | Incompatible version comparison, checksum, and ABI |

---

## Reinstallation

Running `flatroot install` against an existing rootfs compares each newly resolved package against the existing `packages` record by `(name, architecture)` key:

| Condition | Behaviour |
|-----------|-----------|
| Version and checksum match | Package skipped — no download or extraction, and its own setup script is not re-staged. The prior record is carried forward verbatim, including its `Source`, `Url`, and `files` list (only `Depends`, version, checksum, and size are re-read). If any *other* package was extracted this run, post-install still replays every script under `scripts/`, the skipped package's included. |
| Version or checksum differs | Package re-downloaded and re-extracted. Old record replaced. One line printed to stderr: `replacing <name>:<arch>: <old> (<chk>) -> <new> (<chk>), source <old> -> <new>` (ASCII `->`). |
| Every package matches | Entire post-install phase skipped. Prints `Nothing to do. All N packages already current in <path>.` |
| Package only in new install | Downloaded and extracted normally. |
| Package only in existing rootfs | Record retained unchanged. |

The reinstallation key is `(name, architecture)` — there is never more than one record per pair.

---

## Export Exclusion

The top-level `<root>/.flatroot/` directory is excluded from every export format (OCI images, tar archives, SquashFS, DwarFS). The metadata is specific to the build and has no meaning inside a container image or compressed filesystem. There is no `--include-metadata` flag.

---

## Atomic Writes

All metadata files are written to a `<path>.tmp` sibling and atomically renamed into place. A crash partway through leaves either the old file (rename not executed) or the new file (rename completed). A partially-written `.tmp` is never visible to readers.

--8<-- "_glossary.md"
