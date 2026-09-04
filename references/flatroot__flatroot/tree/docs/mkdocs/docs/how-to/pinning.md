---
tags:
  - how-to
  - pinning
  - reproducibility
  - install
---

# Pin a build to a snapshot date

This guide shows how to pin a FlatRoot install to a specific point in time, so repeated runs produce the same rootfs regardless of when or where the command is executed. Without pinning, FlatRoot fetches from the distribution's current servers (its *mirrors*), and those change over time — the `bash` available today is not quite the `bash` available a month from now. Pinning fixes the source at a chosen date by redirecting FlatRoot to a *snapshot service* — a historical archive of the distribution — instead of the current mirrors. For how each supported distribution provides that archive, see [Snapshot Pinning](../appendix/versioning/snapshot-pinning.md).

## Syntax

```bash
# Debian
flatroot --from debian:bookworm@2024-06-15 install --output ./root bash

# Ubuntu
flatroot --from ubuntu:noble@2024-06-15 install --output ./root python3

# Arch Linux
flatroot --from arch:rolling@2024-06-15 install --output ./root firefox
```

Each command fetches the package index as it was on June 15, 2024, then downloads packages at the exact versions available on that date.

??? note "`arch:rolling@<date>`"

    `rolling` is Arch's main release (the release list also offers `multilib`, the 32-bit-enabled variant), and pairing a rolling release with a date may look contradictory. What you're actually pinning is a point in the rolling archive — the state of the Arch mirror on that specific day. The result is just as reproducible as a Debian snapshot.

## Supported distributions

Snapshot pinning is supported for:

- **Debian** via `snapshot.debian.org`
- **Ubuntu** via `snapshot.ubuntu.com` (any date from March 2023 onward)
- **Arch Linux** via `archive.archlinux.org`

Alpine, CentOS, Fedora, AlmaLinux, Rocky, openSUSE, and CachyOS do not have public snapshot services. CentOS 7 and 8 are already frozen on `vault.centos.org`, so every run produces the same output without pinning.

??? note "First snapshot fetch may be slow"

    `snapshot.debian.org` in particular is a single-server archive with rate limiting — the initial index fetch from a snapshot mirror can take minutes where the live mirror would take seconds. The index caches (the raw index and the SQLite database alike) expire after one hour, so a repeat run outside that window re-pays the slow snapshot index fetch; only the downloaded package archives are cached indefinitely.

## Choosing a date

- **Date of a distribution point release** (e.g., `2024-02-10` for Debian Bookworm 12.5) for a well-tested, complete set of packages.
- **Today's date** to freeze the current state and prevent future drift.
- **The date your team agreed on** so that every developer and CI runner produces identical rootfs outputs.

Browse available dates:

- Debian: [snapshot.debian.org/archive/debian/](https://snapshot.debian.org/archive/debian/)
- Arch: [archive.archlinux.org/repos/](https://archive.archlinux.org/repos/)
- Ubuntu: any date from March 2023 onward

??? warning "Pinning is part of the rootfs identity"

    The source string — including any `@date` suffix — is recorded in the rootfs's `.flatroot/manifest` and locked in for its lifetime. Later installs into the same rootfs must pass the exact same `--from` value; a different `@date`, a missing `@date`, a different release, or a different distro are all rejected before any network access. To switch to a different source, point `--output` at a fresh directory or remove the existing `.flatroot/` subdirectory.

??? tip "See also"

    [Reproducible Builds](../tutorials/reproducible-builds.md) — tutorial walk-through of building twice with the same pin and observing that the outputs are byte-identical.

--8<-- "_glossary.md"
