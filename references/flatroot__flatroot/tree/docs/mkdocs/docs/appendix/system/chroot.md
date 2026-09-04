---
tags:
  - appendix
  - chroot
---

# chroot

`chroot(1)` changes the apparent root directory (`/`) for the running process and its descendants. Programs launched under a chroot see the given directory as `/` and cannot reach files above it through ordinary path lookups.

This makes chroot a natural companion to a rootfs built with flatroot: the rootfs directory becomes `/` for the program you run inside it, so system paths like `/etc/passwd` and `/bin/sh` resolve against the rootfs instead of the host.

## Why it needs root

The `chroot` system call is reserved for the root user. Without that restriction, an ordinary user could construct a directory containing a forged `/etc/passwd` and chroot a setuid program into it to escalate privilege. Linux therefore requires `CAP_SYS_CHROOT` to call it, which in practice means `sudo chroot`.

## What chroot does not isolate

chroot only changes the filesystem root. It does not create a new:

- **process namespace** — processes inside the chroot see, and can signal, processes on the host.
- **network namespace** — network interfaces, sockets, and ports are shared with the host.
- **mount namespace** — new mounts inside the chroot are visible on the host and vice versa.
- **user namespace** — user and group IDs are the same as the host's.

chroot is a filesystem fence, not a sandbox. A process with enough privilege inside a chroot can break out of it.

## Unprivileged alternatives

To run programs from a flatroot-built rootfs without root, use tools that combine filesystem isolation with a user namespace:

- **[bubblewrap](bubblewrap.md)** — the canonical rootless sandbox, used by Flatpak and rootless Podman.
- `unshare --user --mount --pid --fork chroot ...` — achieves similar isolation with the `util-linux` tools.

## Further reading

- [chroot(1) man page](https://man7.org/linux/man-pages/man1/chroot.1.html)
- [chroot(2) system call man page](https://man7.org/linux/man-pages/man2/chroot.2.html)

--8<-- "_glossary.md"
