---
tags:
  - appendix
  - rootfs
---

# Rootfs

A **rootfs** (root filesystem) is the directory tree that appears as `/` to a running Linux process. It holds the minimum set of files needed for programs to run: the executables in `/bin` and `/usr/bin`, the shared libraries in `/lib` and `/usr/lib`, the system-wide configuration in `/etc`, and the standard companions — `/var`, `/tmp`, `/root`, `/home`, and so on. The shape of this tree is codified by the [Filesystem Hierarchy Standard](https://refspecs.linuxfoundation.org/FHS_3.0/fhs/index.html) and is shared across every mainstream Linux distribution.

When you boot a Linux system, its rootfs is mounted at `/` by the kernel early in startup. When you build one with flatroot, you get exactly the same kind of tree — but inside a directory you own, built from upstream packages, without touching your host's own `/`.

## Why build a rootfs as a directory

A rootfs-as-directory is a portable building block. Once you have one, you can:

- **Run programs inside it** with [chroot](chroot.md) or [bubblewrap](bubblewrap.md) as if it were the system root, keeping the program isolated from your host.
- **Package it** as an [OCI container image](../formats/oci.md), a tarball, a [SquashFS](../formats/squashfs.md) image, or a [DwarFS](../formats/dwarfs.md) image via `flatroot export`.
- **Use it as a build environment** — a pinned, deterministic toolchain for compiling software that must match a specific distribution.
- **Hand it to another tool** that expects a distribution-shaped directory — most container runtimes, many sandboxing frameworks, some installers.

Building the rootfs is a one-shot operation; what you do with it afterwards is a separate concern.

## What a rootfs is not

A rootfs is a **filesystem tree**, not a **running system**. In particular, a rootfs on its own:

- Is not bootable. There is no kernel, no init, no boot loader — only userspace files.
- Is not isolated. Until a process enters it (via `chroot`, `bwrap`, or a container runtime), it is just files on your host.
- Does not carry `/proc`, `/sys`, or `/dev`. Those are kernel-provided virtual filesystems that runtime tools mount into the rootfs on demand.

flatroot's job is to produce the tree. The kernel's job is to run it. Something in between — `chroot`, `bwrap`, Docker, Podman — connects the two.

## Why unprivileged construction matters

Traditional rootfs tools (`debootstrap`, `pacstrap`, `dnf --installroot`) run as root because they assume the target tree will eventually become a live system, where on-disk ownership and setuid bits matter during boot. flatroot does not assume that. Because the expected destinations are sandboxes, container images, or build environments that re-interpret ownership on use, flatroot extracts as the unprivileged user who invoked it. No root, no sudo, no background daemon.

The trade-off: files in the rootfs are owned by you, not by `root`. Tools that re-ship the rootfs handle this normally — containers assign their own uid mapping, OCI image builders rewrite ownership at export time, user namespaces remap on entry. If you need a rootfs with literal `root:root` ownership on disk, that is a different tool.

--8<-- "_glossary.md"
