---
tags:
  - explanation
  - hooks
  - post-install
  - cache
---

# Cache regeneration

After every package is extracted and every post-install script has run, the rootfs contains the right files in the right places. But some of those files are cache files — precomputed indexes that applications read at startup to avoid scanning the filesystem — and the caches are stale. The font cache lists fonts that were installed by packages extracted earlier but not fonts from packages extracted later. The MIME database is missing entries for packages that registered MIME types after the database was last regenerated. The icon cache covers the icon themes as they were mid-extract, not as they are at the end.

On a live system, the package manager regenerates these caches incrementally. Dpkg runs triggers after each package is unpacked: the fontconfig trigger runs `fc-cache`, the MIME trigger runs `update-mime-database`, the icon trigger runs `gtk-update-icon-cache`. Arch uses pacman hooks that fire when files are installed into specific directories. Alpine uses `.trigger` scripts. RPM uses `%posttrans` scriptlets and `%transfiletriggerin` file triggers.

None of this infrastructure exists in FlatRoot's sandbox. Dpkg triggers are no-oped by the stub commands. Pacman is not involved in the installation. Alpine's trigger mechanism is not executed. RPM's `%posttrans` scriptlets are not parsed. The caches must be regenerated some other way, or applications that read them will behave as though the rootfs is incomplete.

This page develops why FlatRoot uses a content-based approach — checking which binaries exist in the rootfs and running them — rather than reimplementing each distro's trigger system.

## The trigger problem

Reimplementing each distro's trigger system would mean reimplementing four different mechanisms. Dpkg's trigger system is declarative: packages declare interest in trigger names, other packages activate those triggers, dpkg runs the interested packages' postinst scripts in the right order. Pacman hooks are files in `/usr/share/libalpm/hooks/` that declare which files they watch and what command to run. Alpine's `.trigger` scripts fire on directory changes. RPM's file triggers use a pattern-matching language to select files and a shell command to run.

Each of these systems is Turing-complete in its own way. Reimplementing them would mean maintaining four mini-resolvers that track which triggers fired, in what order, against which packages, with which dependencies — effectively rebuilding each distro's package manager for the trigger subsystem alone.

FlatRoot does not do this. The trigger systems exist to answer one question: "after this set of packages has been installed, which caches need regeneration?" FlatRoot answers the question directly by listing the caches and checking whether the binary that regenerates each one is present in the rootfs.

## The content-based approach

FlatRoot maintains a table of cache regeneration hooks. Each hook names a cache, the binary that regenerates it, and the command to run. After all scripts complete, FlatRoot iterates the table. For each hook, it checks whether the required binary exists anywhere in the rootfs — in `usr/bin`, `usr/sbin`, `usr/lib`, and the library subdirectories where distros sometimes hide loader utilities. If the binary is found, the hook runs inside the sandbox. If not, the hook is skipped.

The approach works across all distros without configuration. On a Debian rootfs, `fc-cache` is present, so the font cache is regenerated. On an Alpine rootfs, `fc-cache` is also present (Alpine packages fontconfig), so the font cache is regenerated there too. On a minimal rootfs with no fonts, `fc-cache` is absent, and the hook is silently skipped. The presence of the binary is the only signal FlatRoot needs.

The hooks are best-effort. A hook whose binary exists but exits non-zero prints a warning and continues. A failing hook does not abort the build. The reasoning is that a stale cache is better than no rootfs — applications that find a stale cache will typically regenerate it on first run, albeit more slowly. A rootfs that failed to build is useless.

## The hook table

The hooks cover the caches that every distro's trigger system would regenerate:

- **Font cache** (`fc-cache`) — so applications can find installed fonts.
- **Icon cache** (`gtk-update-icon-cache`) — so GTK applications can find themed icons.
- **MIME database** (`update-mime-database`) — so file managers and browsers can identify file types.
- **GSettings schemas** (`glib-compile-schemas`) — so GNOME applications can read their settings.
- **GDK pixbuf loaders** (`gdk-pixbuf-query-loaders`) — so GTK can load image formats.
- **GTK input methods** (`gtk-query-immodules-3.0`) — so GTK can find input method modules.
- **GIO modules** (`gio-querymodules`) — so GLib can find I/O extension points.
- **Desktop database** (`update-desktop-database`) — so application launchers can find `.desktop` files.
- **CA certificates** (`update-ca-trust`, `update-ca-certificates`) — so TLS connections can verify certificates.
- **Locale data** (`locale-gen`) — so programs can format dates and numbers in the user's locale.
- **dconf database** (`dconf update`) — so dconf can find its settings.

## Why the hooks run after scripts, not during extraction

A natural alternative would be to run each hook after the package that provides its binary is extracted — regenerate the font cache as soon as `fc-cache` appears, the MIME database as soon as `update-mime-database` appears. This would mimic the incremental approach of the distros' own trigger systems.

The incremental approach has a problem that the bulk approach avoids: a cache regenerated mid-extract sees an incomplete filesystem. If the font cache runs before all font packages are extracted, some fonts are missing from the cache. If a later font package's postinst also triggers the font cache, the cache is regenerated again — but only if that package's trigger system is active, which in FlatRoot's sandbox it is not. The bulk approach runs each hook exactly once, after every file is in place, and the cache reflects the complete rootfs.

The cost is that the hooks cannot run in parallel with extraction. They extend the total install time by the sum of the cache regeneration times. In practice, this is seconds — `fc-cache` on a typical desktop font set takes under a second, and the slowest hook (`gtk-update-icon-cache` on a full icon theme) takes a few seconds. The time is paid once per install, not once per package, and the simplicity of the bulk approach outweighs the minor time cost.

--8<-- "_glossary.md"
