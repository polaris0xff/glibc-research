---
tags:
  - explanation
  - distro
  - metadata
---

# Cross-distro boundaries

FlatRoot refuses to install packages from a different source into a rootfs that already has packages installed. The check is strict: any change to the `--from` string — a different distro, a different release, a different `@date` snapshot pin, or pinned vs unpinned of the same release — aborts the install before any network access, any download, any extraction.

The refusal is not a missing feature. It is the only correct response to a structural incompatibility that no amount of format engineering can fix. This page develops why same-named packages across distros are not interchangeable, why the same risks apply to subtler source changes within one distro, and why the guardrail is a full-source-string equality check rather than a per-package one.

## Same name, different package

A package called `bash` exists in Debian, Ubuntu, Fedora, Arch, and Alpine. The name is the same. Nothing else is.

The Debian `bash` package is version `5.2.15-2+b10`. The `-2+b10` suffix is a Debian revision: the second packaging revision of upstream `5.2.15`, with a binary rebuild (`b10`). The Fedora `bash` package is version `5.2.37-1.fc42`. The `-1.fc42` suffix is a Fedora revision: the first packaging of upstream `5.2.37` for Fedora 42. The two version strings look similar — both have an upstream part and a revision part — but they are incomparable under any single version algorithm. The dpkg algorithm that compares Debian versions would misinterpret the Fedora string. The RPM algorithm that compares Fedora versions would misinterpret the Debian string.

Even if the versions could be compared, the packages' contents differ. The Debian `bash` package applies Debian-specific patches to the upstream source. The Fedora `bash` package applies Fedora-specific patches. The two packages ship different `bash` binaries, different configuration files, different maintainer scripts. A rootfs with Debian's `bash` binary and Fedora's `bash` configuration files would have a `bash` that reads `/etc/bashrc` from a path Fedora uses but Debian does not, or that depends on a shared library Fedora packages under a different soname.

## Version comparison across distro families

The version comparison problem is the most concrete incompatibility. The two algorithms — dpkg and RPM — produce different results for the same input strings, and the differences are not edge cases.

The dpkg algorithm assigns sort weights to non-alphanumeric characters. The tilde (`~`) sorts before everything, so `1.0~beta < 1.0`. The plus (`+`) sorts after everything, so `1.0 < 1.0+dfsg`. The colon introduces an epoch that overrides all other comparisons: `2:1.0 > 1:999.0`.

The RPM algorithm skips separators between alphanumeric segments and compares the segments. The tilde is the exception — like dpkg, FlatRoot's RPM comparator gives it pre-release meaning and sorts it before everything (so `1.0~rc1 < 1.0`); it is never discarded. An epoch is stored as a separate integer, not embedded in the version string. Two version strings that compare correctly under dpkg can compare incorrectly under RPM, and a dependency constraint that is satisfiable under one algorithm can be unsatisfiable under the other.

Mixing distros means picking one algorithm for all version comparisons in the combined rootfs. Whichever algorithm is chosen, some packages' version constraints will be evaluated incorrectly, and the resolver will either reject satisfiable dependencies or accept unsatisfiable ones. Neither outcome is acceptable.

## Checksum algorithms differ

The archive checksums FlatRoot verifies after download use different algorithms depending on the source distro. Debian publishes SHA-256 hashes. openSUSE publishes SHA-512 hashes. Alpine publishes a base64-encoded SHA-1 of the package's control section, prefixed with `Q1`.

The `.flatroot/packages` file records each package's checksum with an algorithm prefix (`sha256:` for the deb, rpm, and pacman families — openSUSE's SHA-512 digest is carried under that same `sha256:` label — and `q1-sha1:` for Alpine). If two distros were mixed in one rootfs, the checksum prefixes would be inconsistent across packages. More importantly, a reinstall that compares checksums to decide whether to re-download would need to know which algorithm to use for each package — information that the package's source distro encodes in its index format, not in the package record itself. The metadata layer would need a new field to track the algorithm per package, and every consumer of the metadata would need to handle per-package algorithm variance. The complexity is not worth the use case.

## Why the guardrail is on the full source string, not at the package level

A finer-grained guardrail would check each package's source individually and refuse only the packages that conflict. A rootfs with Debian `bash` and Fedora `firefox` would be allowed if neither package conflicts with the other.

The finer-grained check fails because packages are not independent. Debian `bash` depends on Debian `libc6`, which depends on Debian `gcc-12-base`, which depends on Debian `libgcc-s1`. The transitive closure of a single Debian package is entirely Debian packages. Mixing sources at the package level would mean mixing them at the dependency level, and the same version-comparison and checksum-algorithm problems would appear across the dependency graph, not just at the top level.

The full-source-string guardrail — checking that the incoming `--from` matches every entry in the existing `Sources` set byte-for-byte — catches the problem at the cheapest possible boundary. The error message names the conflict, the existing sources, and the path to the manifest. It tells the user what went wrong and how to start fresh.

## Why the same check applies to date pins and release changes

A naive guardrail would compare only the distro prefix, allowing `debian:bookworm@2026-04-21` to install into a rootfs built with `debian:bookworm@2025-01-15` — same prefix, same release, just a date pin change. FlatRoot does not allow that.

The reasoning is the same as for cross-distro installs, scaled down to a single distro. Two different snapshot dates of the same Debian release are two different package universes. `libc6` at `2.36-9+deb12u7` from 2025-01-15 and `libc6` at `2.36-9+deb12u13` from 2026-04-21 are not the same archive, do not share a checksum, and may differ in any patched symbol the security team backported between those dates. A rootfs whose manifest says it is pinned to one date but contains files from another is not reproducible, not auditable, and not safe to redistribute as the pinned artefact it claims to be.

The same argument covers release changes (`debian:bookworm` → `debian:trixie`) and the pinned-vs-unpinned toggle (`debian:bookworm` → `debian:bookworm@2024-06-15`). In each case the source string differs and the package universe differs, so the install is refused.

## What is allowed

The guardrail permits two operations:

- **Reinstallation.** Installing the same `--from` source into an existing rootfs is the idempotent case described in [The metadata layer](the-metadata-layer.md). The guardrail passes because the source strings match exactly; per-package checksum comparison then skips already-current packages.

- **Multi-architecture within one invocation.** `--arch x86_64,i686` from one `flatroot install` runs the per-architecture loop with a single `--from`, so every package record carries the same source string. The guardrail is unaffected. (Adding a second architecture via a separate invocation would require the same `--from` as the first; that too is allowed.)

--8<-- "_glossary.md"
