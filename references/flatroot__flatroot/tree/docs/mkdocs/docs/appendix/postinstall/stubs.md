---
tags:
  - appendix
  - post-install
  - sandbox
---

# Stub Commands

Stub commands are shell scripts installed at `.flatroot/bin/` inside the rootfs before post-install scripts run. This directory is prepended to `PATH`, so stubs shadow the real (missing or privileged) commands that scripts expect.

## Why stubs are needed

Post-install scripts are written for live systems with root privileges, a running init system, and a package database. In a rootfs context, these assumptions don't hold:

- There is no real root — ownership operations are meaningless
- There are no running services to restart
- There is no dpkg/rpm database to query
- There are no system users to create

Without stubs, scripts would fail on the first privileged command and abort. Stubs allow the harmless parts of scripts (cache regeneration, symlink creation) to execute while silently skipping the privileged parts.

## No-op stubs

These stubs exit 0 immediately, doing nothing. The set is **per-flavour** — each post-install flavour installs only the stand-ins its own family's scripts assume, so there is no single universal list.

**Debian flavour (also used by the RPM family):**

| Command | Why stubbed |
|---------|------------|
| `chown`, `chgrp`, `chmod` | Ownership and permissions are meaningless without root |
| `systemctl`, `invoke-rc.d`, `update-rc.d`, `start-stop-daemon` | No running services in a rootfs |
| `adduser`, `addgroup`, `deluser`, `delgroup`, `pam-auth-update` | User/group management requires `/etc/passwd` authority |
| `dpkg-trigger`, `dpkg-divert`, `dpkg-query`, `dpkg-statoverride`, `dpkg-maintscript-helper` | No dpkg infrastructure |
| `pycompile`, `py3compile`, `pyclean`, `py3clean` | Python bytecode caches regenerate on demand |
| `mount`, `umount` | Mounting would touch the host |

A `remove-shell` script is also installed as a plain no-op.

**Arch flavour:**

| Command | Why stubbed |
|---------|------------|
| `groupadd`, `groupmod`, `useradd`, `usermod` | User/group management requires `/etc/passwd` authority |
| `systemd-sysusers`, `systemd-tmpfiles` | No systemd running |
| `install-info`, `xdg-icon-resource` | Registration is non-essential |

The Arch set has **no** `chown` and **no** `systemctl` stub — Arch `.INSTALL` scripts do not call them.

**Alpine flavour:**

| Command | Why stubbed |
|---------|------------|
| `rc-update`, `rc-service`, `openrc` | No OpenRC init system |
| `apk` | No apk database |

## Functional stubs

Some stubs perform real work because scripts depend on their output:

### `dpkg --compare-versions`

Many Debian postinst scripts gate behavior on version comparisons:

```sh
if dpkg --compare-versions "$2" lt "1.0"; then
    # migrate from old format
fi
```

The stub implements this using `sort -V` (version sort) to compare version strings. This covers the vast majority of real-world usage. The same stub also answers `dpkg -s` / `--status` with `Status: install ok installed`, so a script that probes whether a package is already installed sees success rather than an error.

### `update-alternatives --install`

Wine, bash, and dash depend on the alternatives system to create their command symlinks. The stub parses `--install <link> <name> <path> <priority>` and any `--slave` entries to create the appropriate symlinks. Without this, Wine's command symlinks (`wine`, `wineserver`, etc.) would not exist.

### `add-shell`

Appends shell paths to `/etc/shells`. Used by bash's postinst to register `/bin/bash`.

### `vercmp` (Arch only)

Arch's version comparison tool from libalpm. Scripts use it to conditionally run migrations. The stub always returns 0 (equal); scripts read that as "already at this version", so their version-guarded upgrade/migration blocks are *skipped* — a safe default for fresh installs where there is no previous version to migrate from.

## Lua scriptlets

Some RPM packages (Fedora, CentOS, AlmaLinux, Rocky, openSUSE) use Lua instead of shell for their post-install scriptlets. These call RPM's embedded Lua runtime with `posix.*` and `rpm.*` extension libraries that exist only inside the `rpm` binary itself. FlatRoot detects Lua scriptlets (via the POSTINPROG header tag) and skips them. Coverage of the work skipped Lua scriptlets would have done is partial — [ldconfig](../system/ldconfig.md) and the content-based cache hooks (CA-trust, locale generation, gdk-pixbuf loaders, the MIME database, GIO modules, the icon cache, the desktop-file database, GTK immodules, and dconf) handle the common cases, but distro-specific Lua tasks such as gconv module registration are not replicated.

--8<-- "_glossary.md"
