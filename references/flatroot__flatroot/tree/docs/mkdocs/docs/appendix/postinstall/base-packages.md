---
tags:
  - appendix
  - post-install
  - dependencies
---

# Base Packages

Base packages are a fixed set of packages that FlatRoot always includes to ensure a functional rootfs. They are added as seed packages before dependency resolution starts.

## Per-distribution base packages

| Distribution | Base packages | Mechanism |
|-------------|--------------|-----------|
| Debian | _(dynamic from index)_ | Packages with `Essential: yes` are scanned from the index and included automatically |
| Ubuntu | _(dynamic from index)_ | Same as Debian — uses `Essential: yes` |
| Arch Linux | `filesystem`, `glibc`, `bash`, `coreutils` | Hardcoded list |
| CachyOS | `filesystem`, `glibc`, `bash`, `coreutils` | Hardcoded list |
| Alpine Linux | `musl`, `busybox`, `alpine-baselayout`, `busybox-binsh` | Hardcoded list |
| CentOS | `filesystem`, `basesystem`, `bash`, `coreutils`, `glibc`, `glibc-common` | Hardcoded list |
| Fedora | `filesystem`, `basesystem`, `bash`, `coreutils`, `glibc`, `glibc-common` | Hardcoded list |
| AlmaLinux | `filesystem`, `basesystem`, `bash`, `coreutils`, `glibc`, `glibc-common` | Hardcoded list |
| Rocky Linux | `filesystem`, `basesystem`, `bash`, `coreutils`, `glibc`, `glibc-common` | Hardcoded list |
| openSUSE | `filesystem`, `bash`, `coreutils`, `glibc` | Hardcoded list |

## Debian Essential packages

Debian and Ubuntu don't use a hardcoded base package list. Instead, certain packages in the repository are marked with `Essential: yes` in their index entry:

```
Package: dash
Essential: yes
```

FlatRoot scans the entire index for this field and adds all Essential packages to the seed list. This typically includes:

- `dash` — the default `/bin/sh`
- `coreutils` — basic file/text utilities
- `grep`, `sed`, `findutils` — text processing
- `base-passwd` — `/etc/passwd` and `/etc/group`
- `base-files` — filesystem structure
- `dpkg` — package manager (needed by postinst scripts)
- `libc-bin` — dynamic linker configuration

The advantage of this approach is that the base set evolves with the distribution — when Debian adds or removes an Essential package, FlatRoot picks up the change automatically without a code update.

## Why base packages matter

Without base packages, the rootfs would contain only the requested package and its hard dependencies. This typically works for the binaries themselves, but the rootfs would be missing:

- `/bin/sh` — required by postinst scripts and many applications
- `/etc/passwd`, `/etc/group` — required by applications that look up user/group names
- The C library (`glibc` or `musl`) — implicitly needed by every binary but often not declared as a dependency because it's assumed to be present
- Basic filesystem structure (`/tmp`, `/var`, `/etc`) — created by `filesystem` packages

--8<-- "_glossary.md"
