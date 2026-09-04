---
tags:
  - appendix
  - sandbox
---

# bubblewrap

`bwrap` — the executable shipped by the [bubblewrap](https://github.com/containers/bubblewrap) project — runs a program inside a fresh set of Linux namespaces without requiring root privileges. It is the tool Flatpak uses to sandbox applications and the one you reach for when you want to run a program inside a flatroot-built rootfs without `sudo`.

Conceptually, `bwrap` is `chroot` plus the full set of Linux isolation primitives: a fresh filesystem root, a fresh mount namespace, a user namespace that makes the whole thing work unprivileged, and optionally fresh PID, network, IPC, and UTS namespaces.

## Why it doesn't need root

Unlike `chroot(1)`, `bwrap` does not gain authority by calling a privileged syscall. It first enters a **user namespace** — a Linux kernel feature that lets an ordinary user become "root" inside a new namespace without acquiring any authority outside of it. Inside that namespace, `bwrap` performs the operations normally reserved for root — creating mount namespaces, pivoting the filesystem root, remounting `/proc` — while the kernel continues to check every file access against the real user's credentials on the outside.

This is the same machinery rootless Podman, Flatpak, and `toolbx` use. It requires the `CONFIG_USER_NS=y` kernel option (enabled on every mainstream Linux distribution since around 2017) and is typically permitted for ordinary users without configuration.

## What bwrap isolates

A default `bwrap` invocation creates:

- A **mount namespace** — bind mounts inside the sandbox are invisible on the host and vice versa.
- A **user namespace** — uids inside the sandbox are mapped back to the caller's uid on the outside; no privileged operations on the host are possible.
- A **filesystem root** — pivoted to the directory you specify.

Optional flags add:

- `--unshare-pid` — fresh PID namespace (the sandboxed program sees no host processes).
- `--unshare-net` — fresh network namespace (no network access at all).
- `--unshare-ipc` — fresh IPC namespace (no shared System V IPC or POSIX message queues).
- `--unshare-uts` — fresh UTS namespace (independent hostname and domain name).

This makes `bwrap` a genuine sandbox, not just a filesystem fence. A process inside cannot signal host processes, access host mounts, or open host sockets unless the wrapper explicitly exposes them.

## Minimal invocation

To run a program inside a flatroot-built rootfs:

```
bwrap --bind ./rootfs / --proc /proc --dev /dev /usr/bin/whatever
```

The flags do three things:

- `--bind ./rootfs /` — make `./rootfs` the new root of the sandboxed filesystem.
- `--proc /proc` — mount a fresh `procfs` inside the sandbox (needed by most programs that read `/proc/self/...`).
- `--dev /dev` — provide a minimal synthetic `/dev` with the standard device nodes (`/dev/null`, `/dev/zero`, `/dev/random`, `/dev/tty`).

Without `--proc` and `--dev`, programs that expect them will fail on read. They are near-universal additions to any `bwrap` invocation targeting a full distribution rootfs.

## Relationship to chroot

|                              | `chroot` | `bwrap`  |
|------------------------------|----------|----------|
| Requires root                | yes      | no       |
| Changes filesystem root      | yes      | yes      |
| Isolates mounts              | no       | yes      |
| Isolates processes           | no       | optional |
| Isolates network             | no       | optional |
| Remaps user ids              | no       | yes      |

`chroot` is a single syscall with a narrow contract — change the filesystem root. `bwrap` is a composition of Linux namespace features that produces a full sandbox. See [chroot](chroot.md) for the older primitive and its trade-offs.

## Further reading

- [bwrap(1) man page](https://man7.org/linux/man-pages/man1/bwrap.1.html)
- [bubblewrap project on GitHub](https://github.com/containers/bubblewrap)
- [user_namespaces(7)](https://man7.org/linux/man-pages/man7/user_namespaces.7.html) — the kernel feature that makes rootless containers possible.

--8<-- "_glossary.md"
