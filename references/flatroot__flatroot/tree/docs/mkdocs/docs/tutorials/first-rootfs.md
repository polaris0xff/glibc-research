---
tags:
  - tutorial
  - install
  - rootfs
  - chroot
---

# Build your first rootfs

In this tutorial we will build a small Alpine Linux [rootfs](../appendix/system/rootfs.md) and confirm it works by entering it with `chroot`. By the end you will have a directory containing a miniature Linux environment that you assembled yourself, without a running package manager on your host.

## Before we start

Work in a fresh directory. If you don't have flatroot yet, follow [Get FlatRoot](../how-to/get-flatroot.md).

## Find a package

Before installing, we can ask flatroot what is available. Search Alpine 3.21 for packages whose names start with `busybox`:

```bash
./flatroot --from alpine:v3.21 search 'busybox*'
```

You will see several matching packages with their version numbers — `busybox`, `busybox-binsh`, `busybox-extras`, and more. The first run fetches the Alpine package index into a local cache; subsequent searches are instant.

See [Query packages](../how-to/query-packages.md) for other glob patterns and for running SQL directly against the index.

## Build the rootfs

Now install `busybox` into a new rootfs:

```bash
./flatroot --from alpine:v3.21 install -o ./hello-alpine busybox
```

You will see flatroot resolve dependencies, download the packages in parallel, extract them, and run post-install scripts. When it finishes, the prompt returns. See [Install packages](../how-to/install-packages.md) for installing multiple packages, adding to an existing rootfs, or targeting other distributions.

## Look at what we built

List the top of the new rootfs:

```bash
ls ./hello-alpine
```

You will see a familiar Linux directory tree — `bin`, `etc`, `lib`, `sbin`, `usr`, and the standard companions. This is a real root filesystem, identical in shape to an Alpine system.

Find the `busybox` binary we asked for:

```bash
ls -l ./hello-alpine/bin/busybox
```

The file is there, it is executable, and it belongs to the rootfs you just built.

## Enter the rootfs

To run a program inside the rootfs we will use `chroot`, the standard Linux tool for changing the apparent root directory of a process. Because `chroot` itself is a privileged operation, this step needs `sudo`:

```bash
sudo chroot ./hello-alpine /bin/busybox ls /
```

You will see the contents of the rootfs's own `/` — the directories you just inspected from outside. The `ls` that just ran was the one baked into `busybox` inside the rootfs, not the one from your host.

For a deeper look at what `chroot` does, why it needs root, and when to reach for unprivileged alternatives, see [chroot](../appendix/system/chroot.md).

## Clean up

When you are done exploring, remove the rootfs and the binary:

```bash
rm -rf ./hello-alpine ./flatroot
```

## What we did

- Downloaded the flatroot binary into the working directory.
- Searched the Alpine package index for available `busybox*` packages.
- Built a rootfs from a pinned Alpine release.
- Entered that rootfs with `chroot` and ran a program from inside.

--8<-- "_glossary.md"
