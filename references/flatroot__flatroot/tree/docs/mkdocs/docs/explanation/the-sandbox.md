---
tags:
  - explanation
  - sandbox
  - post-install
---

# The sandbox

Every supported distribution ships post-install scripts inside its packages. A Debian `.deb` contains a `postinst` shell script that runs after the package's files are unpacked. An Arch `.pkg.tar.zst` contains an `.INSTALL` file with shell functions. An Alpine `.apk` contains a `.post-install` script. An RPM carries a `%post` scriptlet in its header.

These scripts assume they are running as root on a live system. They call `chown` to set file ownership, `mknod` to create device nodes, `systemctl restart` to reload services, `update-initramfs` to rebuild the initrd, `ldconfig` to refresh the shared library cache. On a system where the installer is root and the target is the running system, every one of these calls succeeds and does the right thing.

FlatRoot is not root. It runs as an unprivileged user, writing into a directory that is not the running system. If the scripts ran on the host directly, `chown` would fail with `EPERM`, `mknod` would fail with `EPERM`, `systemctl restart` would restart the host's services — not the rootfs's — and `update-initramfs` would overwrite the host's initrd with one built from the rootfs's kernel modules. The scripts must run, because some create essential symlinks and register alternatives, but they must run in an environment where the dangerous operations are neutralised.

This page develops why FlatRoot uses a namespace sandbox with stub commands rather than skipping scripts or running them as root, and why the stub approach works across every supported distro with a single shared set of shell scripts.

## Why scripts must run

The simplest approach would be to skip post-install scripts entirely. Extract the files, ignore the scripts, declare success. Many packages would work — their scripts only call `chown` and `ldconfig`, and FlatRoot runs `ldconfig` separately after extraction anyway.

Some packages would break in ways that are hard to debug. `wine`'s postinst calls `update-alternatives --install` to register its command symlinks. Without that call, `wine` is installed but not executable at the expected paths. Debian's `dash` postinst calls `add-shell` to register itself in `/etc/shells`. Without that call, `chsh` does not offer `dash` as a login shell. The failures are silent — no error message, just a rootfs where some packages do not work, and the user has no way to know that a skipped script is the cause.

Running the scripts catches these cases. The scripts that do nothing important run and exit harmlessly. The scripts that create symlinks and register alternatives do their work. The scripts that call privileged operations hit stubs that either no-op safely or perform the equivalent action in userspace.

## Why a namespace sandbox

Running the scripts on the host directly — even with stubs — would give them access to the host's filesystem. A script that calls `rm -rf /var/cache` would delete the host's cache, not the rootfs's. A script that reads `/etc/passwd` would see the host's users, not the rootfs's. The scripts need to see the rootfs as their root directory, not the host.

FlatRoot uses Linux user namespaces to create a sandbox where the rootfs is the root directory and the host filesystem is inaccessible. The mechanism is `unshare(CLONE_NEWUSER | CLONE_NEWNS)` followed by `pivot_root` into the rootfs. Inside the sandbox, the process sees the rootfs as `/` and has no path back to the host. The user namespace maps the calling user's UID to 0 inside the sandbox, so the scripts see themselves running as root — `chown` calls succeed (they are no-ops in a user namespace, but they do not fail with `EPERM`), and the scripts do not abort on permission errors.

The sandbox is the same family of primitives `bubblewrap` builds on: fresh user and mount namespaces, `pivot_root` onto the target tree, host-supplied `/dev` and `/proc` bind-mounted in, a private `tmpfs` on `/tmp`. `/sys` is intentionally not mounted — the supported distros' post-install scripts do not read it, and bind-mounting the host's `/sys` would expose information about the host kernel/hardware the sandboxed scripts have no business seeing.

## Stub commands: neutralising privileged operations

Inside the sandbox, the scripts run with `PATH` pointing at a stub directory before the rootfs's own `/usr/bin` and `/bin`. The stub directory contains shell scripts that shadow every privileged command the scripts might call. Each stub either does nothing or performs a userspace equivalent.

**No-op stubs.** Commands that require real root or a running system are neutralised to exit successfully with no side effects. The Debian family stub set (which the RPM-family runners reuse) covers ownership/permission commands (`chown`, `chgrp`, `chmod`), service managers (`systemctl`, `invoke-rc.d`, `update-rc.d`, `start-stop-daemon`), user/group management (`adduser`, `addgroup`, `deluser`, `delgroup`, `pam-auth-update`), dpkg infrastructure (`dpkg-trigger`, `dpkg-divert`, `dpkg-query`, `dpkg-statoverride`, `dpkg-maintscript-helper`), Python bytecode helpers (`pycompile`, `py3compile`, `pyclean`, `py3clean`), and mount calls that would otherwise touch the host (`mount`, `umount`). The Arch and Alpine runners install their *own* independent stub sets rather than extending the Debian one: Arch's is `groupadd`, `useradd`, `usermod`, `groupmod`, `systemd-sysusers`, `systemd-tmpfiles`, `install-info`, and `xdg-icon-resource` (note: no `chown` and no `systemctl`), and Alpine's is the OpenRC/apk set `rc-update`, `rc-service`, `openrc`, `apk`.

`ldconfig` is not stubbed — FlatRoot runs the real `ldconfig` (from inside the rootfs, in a separate sandboxed pass) before scripts execute, so the cache is fresh by the time any script invokes it.

**Functional stubs.** A few commands can be implemented in userspace and are too important to no-op:

- `update-alternatives --install` creates a real symlink. `wine` and other packages depend on this to register their command names.
- `dpkg --compare-versions` performs actual version comparison using `sort -V`. Scripts use this to decide whether to run migration steps.
- `add-shell` appends to `/etc/shells`. The `dash` package uses this.
- `vercmp` (Arch's version comparison tool) returns `0` (equal). Because scripts read `0` as "already at this version", their version-guarded upgrade/migration blocks are *skipped* — the right outcome for a fresh install, where there is no prior version to migrate from.

**Environment.** `DEBIAN_FRONTEND=noninteractive` is set to suppress debconf prompts. Stdin is closed to prevent scripts from blocking on input.

## Why stubs, not a seccomp filter

An alternative to stub commands is a seccomp filter that blocks the dangerous syscalls: `chown`, `mknod`, `reboot`. The scripts would call `chown`, the kernel would return `EPERM`, and the script would abort with an error.

The stub approach is better for two reasons. First, it lets the script succeed. A script that calls `chown` and continues is harmless; a script that calls `chown` and aborts leaves its package half-configured. The stubs turn a class of errors into successes.

Second, seccomp cannot handle `systemctl restart`. That is not a syscall; it is a command that talks to systemd over D-Bus. Blocking the `connect` syscall would also block legitimate socket operations. The stub approach handles both syscall-level and command-level operations uniformly: replace the command with a no-op, let the script continue.

## Per-flavour stub sets

The stub set is chosen by the post-install flavour, not shared across every distro. The Debian flavour (which the RPM family reuses) installs the broad set above — `chown`, `systemctl`, the `dpkg-*` helpers, and the rest. The Arch flavour installs a narrower set with no `chown` and no `systemctl` at all, because Arch `.INSTALL` scripts do not call them; the Alpine flavour installs only its OpenRC/apk stubs. Each flavour ships exactly the stand-ins its own family's scripts assume.

What *is* shared is the mechanism: every flavour stages its stubs into `.flatroot/bin` and puts that directory first on `PATH`, so whichever stubs were installed shadow the rootfs's real binaries. The post-install *runner* and the stub set are both per-backend; only the staging mechanism is universal.

--8<-- "_glossary.md"
