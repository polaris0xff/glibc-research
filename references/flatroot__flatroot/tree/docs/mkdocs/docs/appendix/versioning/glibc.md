---
tags:
  - appendix
  - glibc
  - versioning
---

# glibc Version Reference

This table maps Linux distributions to their shipped glibc versions and corresponding FlatRoot remotes. Choosing the right target suite is critical for binary compatibility.

## Version Table

| Distribution | Release Year | glibc Version | FlatRoot Remote |
|-------------|-------------|---------------|----------------|
| CentOS 7 / RHEL 7 | 2014 | 2.17 | `centos:7` |
| Debian 9 (Stretch) | 2017 | 2.24 | `debian:stretch` |
| CentOS 8 / RHEL 8 | 2019 | 2.28 | `centos:8` |
| AlmaLinux 8 / Rocky 8 | 2021 | 2.28 | `alma:8`, `rocky:8` |
| Debian 10 (Buster) | 2019 | 2.28 | `debian:buster` |
| Ubuntu 20.04 (Focal) | 2020 | 2.31 | `ubuntu:focal` |
| Fedora 33 | 2020 | 2.32 | `fedora:33` |
| Debian 11 (Bullseye) | 2021 | 2.31 | `debian:bullseye` |
| Fedora 35 | 2021 | 2.34 | `fedora:35` |
| Ubuntu 22.04 (Jammy) | 2022 | 2.35 | `ubuntu:jammy` |
| Debian 12 (Bookworm) | 2023 | 2.36 | `debian:bookworm` |
| Fedora 39 | 2023 | 2.38 | `fedora:39` |
| CentOS Stream 9 | 2024 | 2.34 | `centos:stream9` |
| AlmaLinux 9 / Rocky 9 | 2022 | 2.34 | `alma:9`, `rocky:9` |
| openSUSE Leap 15.6 | 2024 | 2.38 | `opensuse:15.6` |
| Ubuntu 24.04 (Noble) | 2024 | 2.39 | `ubuntu:noble` |
| Fedora 41 | 2024 | 2.40 | `fedora:41` |
| Debian 13 (Trixie) | 2025 | 2.41 | `debian:trixie` |
| Fedora 42 | 2025 | 2.41 | `fedora:42` |

## Why glibc Version Matters

The GNU C Library (glibc) uses **symbol versioning** to maintain backward compatibility. Each function in glibc is tagged with the version in which it was introduced or last changed (e.g., `GLIBC_2.17`, `GLIBC_2.34`). When a binary is compiled and linked against glibc, the linker records which symbol versions it requires.

At runtime, the dynamic linker checks whether the system's glibc provides all required symbol versions. If the binary was compiled against glibc 2.36 and uses a symbol introduced in that version, it will fail to run on a system with glibc 2.28:

```
./program: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.36' not found
```

This is a hard failure -- there is no workaround short of recompiling, providing a compatible glibc, or binary patching / compatibility tools.

## Targeting for Compatibility

To maximize the number of systems a binary can run on, build against the oldest glibc version that provides the features you need. The general principle: **targeting an older release gives broader compatibility at the cost of newer library features.**

### Common scenarios

**Target `centos:7` (glibc 2.17)** when:

- You need maximum compatibility back to 2014-era systems.
- Your target includes RHEL 7, Amazon Linux 2, or other long-lived enterprise systems.

**Target `debian:buster`, `alma:8`, or `rocky:8` (glibc 2.28)** when:

- You are building a portable CLI tool or library that must run on enterprise distros: RHEL 8, Ubuntu 18.04, Amazon Linux 2.
- You are shipping a self-contained application bundle and want broad compatibility.
- For RPM-based toolchains, `alma:8` or `rocky:8` provide actively maintained packages at the same glibc version.

**Target `debian:bullseye` (glibc 2.31)** when:

- You need newer package versions than Buster provides but still want broad compatibility.
- Your minimum supported platform is Ubuntu 20.04 or RHEL 9.

**Target `debian:bookworm` (glibc 2.36)** when:

- You do not need to support systems older than 2022-era distributions.
- You want the latest stable packages and security patches.

**Target `debian:stretch` (glibc 2.24)** for maximum compatibility back to 2017-era systems, though some modern software may not compile against a glibc this old.

## Verifying the installed glibc

After building your rootfs, verify which glibc is installed by executing the libc shared object directly — glibc's `.so` file prints version information when run as a program:

```bash
sudo chroot ./root /lib/x86_64-linux-gnu/libc.so.6
```

Expected output (for Buster):

```
GNU C Library (Debian GLIBC 2.28-10+deb10u...) stable release version 2.28.
...
```

See [chroot](../system/chroot.md) for details on running programs inside a rootfs.

## Checking a binary's glibc requirements

To inspect which glibc versions a binary requires, use `objdump` or `readelf`:

```bash
objdump -T ./my-binary | grep GLIBC_ | sed 's/.*GLIBC_/GLIBC_/' | sort -u -t. -k1,1 -k2,2n
```

This lists every `GLIBC_` symbol version the binary references. The highest version in the output determines the minimum glibc required at runtime.

--8<-- "_glossary.md"
