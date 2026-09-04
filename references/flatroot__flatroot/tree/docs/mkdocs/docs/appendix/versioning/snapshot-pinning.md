---
tags:
  - appendix
  - pinning
  - reproducibility
---

# Snapshot Pinning

Snapshot pinning freezes the package index to a specific date, producing identical output regardless of when or where the command runs. Append `@YYYY-MM-DD` to the remote string:

```bash
flatroot --from debian:bookworm@2024-06-15 install --output ./root bash
```

## Supported distributions

| Distribution | Snapshot service | Date range |
|-------------|-----------------|------------|
| Debian | `snapshot.debian.org` | 2005 -- present |
| Ubuntu | `snapshot.ubuntu.com` | March 2023 -- present |
| Arch Linux | `archive.archlinux.org` | 2013 -- present |
| Alpine | Not available | — |
| CentOS | Not available (vault is already frozen) | — |
| Fedora | Not available | — |
| AlmaLinux | Not available | — |
| Rocky Linux | Not available | — |
| openSUSE | Not available | — |
| CachyOS | Not available | — |

## How it works

When a date is specified, FlatRoot constructs the mirror URL to point at the snapshot service instead of the live mirror:

| Distribution | Live mirror | Snapshot URL |
|-------------|------------|-------------|
| Debian | `deb.debian.org/debian` | `snapshot.debian.org/archive/debian/20240615T000000Z` |
| Ubuntu | `archive.ubuntu.com/ubuntu` | `snapshot.ubuntu.com/ubuntu/20240615T000000Z` |
| Arch | `geo.mirror.pkgbuild.com` | `archive.archlinux.org/repos/2024/06/15` |

The package index fetched from the snapshot URL contains the exact packages that were available on that date. All subsequent downloads use the same snapshot mirror, so every package is the version that was available at that time.

## CentOS and frozen mirrors

CentOS 7 and 8 are end-of-life and their packages are served from `vault.centos.org`, which is already a frozen archive. The packages don't change, so snapshot pinning isn't needed — every run produces the same output.

## Browse available dates

- Debian: [snapshot.debian.org/archive/debian/](https://snapshot.debian.org/archive/debian/)
- Ubuntu: any date from March 2023 onward works
- Arch: [archive.archlinux.org/repos/](https://archive.archlinux.org/repos/)

--8<-- "_glossary.md"
