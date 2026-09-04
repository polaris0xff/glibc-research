---
tags:
  - explanation
  - metadata
  - reproducibility
---

# The metadata layer

Every rootfs FlatRoot builds contains a `.flatroot/` directory at its root. The directory is not part of the distribution — no upstream package ships it, no post-install script writes to it. It is FlatRoot's private record of what was installed, where it came from, and which files each package wrote.

The metadata serves three purposes. It enables **reinstallation** — FlatRoot can detect which packages are already current and skip them. It enables **cross-distro protection** — FlatRoot can detect that an existing rootfs was built from a different distro and refuse to mix them. And it enables **downstream tooling** — SBOM generators, audit scripts, and package queries can read the metadata without invoking FlatRoot.

This page develops what FlatRoot records, why it records it at install time rather than inferring it later, and why the `.flatroot/` directory is excluded from every export format.

## What is recorded

The `.flatroot/` directory holds three records that describe the install — a manifest, a package list, and a file list — plus two working directories post-install leaves in place and never prunes: `scripts/` (the staged maintainer scripts) and `bin/` (the stub-command PATH).

**The manifest** (`manifest`) is a single record describing the rootfs as a whole: the FlatRoot version that built it, the distro sources it was built from, the architectures it contains, and the total package count. It answers the question "what is this rootfs and where did it come from?" in a small header record.

**The package list** (`packages`) is one record per installed `(name, architecture)` pair. Each record carries the upstream version, the distro source, the download URL, the archive checksum with its algorithm, the archive size, and the flattened dependency list. It answers the question "what exactly is installed in this rootfs?" at package granularity.

**The file list** (`files/<name>:<arch>`) is one text file per installed package, listing every absolute path the package wrote into the rootfs. It answers the question "which files did this package contribute?" — the question a downstream SBOM generator or audit script needs to answer.

All three use deterministic ordering: records are sorted byte-ascending by their primary key. Two runs of the same install command against the same index produce byte-identical metadata files.

## Why metadata is recorded at install time

The information FlatRoot records — which version of which package was installed, from which mirror, with which checksum — is knowable at install time and not recoverable later. An extracted rootfs contains files, but files do not carry their provenance. A `/usr/bin/bash` binary does not record that it came from `bash_5.2.15-2+b10_amd64.deb` at `sha256:a4f8...`. The metadata captures this at the moment of installation, when the download URL, checksum, and version are all in hand.

The alternative — inferring provenance from the extracted files — would require either storing the provenance inside the files themselves (which would modify upstream packages and break checksum verification) or re-deriving it by comparing the rootfs against the index (which is fragile: the index might have changed between install and audit, and file-level matching across package boundaries is error-prone). Recording at install time is the only reliable approach.

## Reinstallation and idempotency

Running `flatroot install` against an existing rootfs is supported. The `.flatroot/packages` file is the source of truth for what is already installed. Before downloading or extracting anything, FlatRoot reads the existing package list and compares each newly resolved package against its existing record.

A package whose `(name, architecture, version, checksum)` matches the existing record is already current. FlatRoot skips downloading and extracting that package. The prior record is carried forward verbatim — the file list, source, and URL all reused unchanged; only the version, checksum, size, and dependency list are re-read. No warning is emitted — a match is the expected outcome for an unmodified rootfs.

A package whose version or checksum differs from the existing record is an upgrade or downgrade. FlatRoot downloads the new archive, extracts it, and replaces the old record. A single line is printed naming the package, the old and new versions, and the old and new sources.

When every resolved package is already current, FlatRoot skips the entire post-install phase and prints `Nothing to do.` This makes `flatroot install` idempotent: running it twice against the same `--from` source produces the same rootfs and does no work on the second run.

## Cross-distro protection

The manifest's `Sources` field records the full source string the rootfs was built from (e.g., `debian:bookworm@2026-04-21`). Before any new install, FlatRoot reads the existing manifest and compares the recorded source against the incoming `--from` string. If the strings are not identical — a different distro, a different release, a different snapshot date, or pinned vs unpinned of the same release — the install aborts.

The reason is developed in [Cross-distro boundaries](cross-distro-boundaries.md). In brief: any source change between installs would interleave package versions from different points in time, with the same version-comparison, checksum, and ABI risks as a cross-distro switch. The only safe response is to refuse.

## Why `.flatroot/` is excluded from exports

Exports — OCI images, tar archives, SquashFS filesystems — ship the rootfs as an opaque artifact for downstream consumption. The `.flatroot/` directory is specific to the build and has no meaning inside a container image or a compressed filesystem. It is excluded from every export format.

The exclusion is not configurable. There is no `--include-metadata` flag. The reasoning is that the metadata describes the build, not the payload, and shipping it inside the artifact would confuse consumers that find a `.flatroot/` directory and do not know what it is. Downstream tooling that needs the metadata should read it from the build directory, not from the exported artifact.

## The metadata as a stable interface

The `.flatroot/` layout is versioned implicitly by the `FlatrootVersion` field in the manifest. A future version of FlatRoot that changes the metadata format will write a different version string; consumers can inspect the string to decide which parser to use.

The format itself is deliberately simple: dpkg-style `Key: Value` records for the manifest and package list, plain text paths for the file lists. A consumer that can read RFC 822-style records — a format that has been stable since Debian's first release in 1993 — can read FlatRoot's metadata. No binary formats, no SQLite, no protobuf. The simplicity is a commitment: the metadata layer is an interface between FlatRoot and downstream tools, and interfaces should be parseable with standard library functions in every language.

--8<-- "_glossary.md"
