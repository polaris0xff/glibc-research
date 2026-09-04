---
tags:
  - appendix
  - formats
  - export
---

# SquashFS

[SquashFS](https://docs.kernel.org/filesystems/squashfs.html) is a compressed read-only filesystem built into the Linux kernel. It's the format behind live CDs, Ubuntu's `.snap` packages, OpenWRT firmware, compressed initramfs images, and most "ship a filesystem tree as a single file" use cases. flatroot supports it as an export target for exactly that reason — SquashFS output works anywhere a mainline Linux kernel can mount, which is almost everywhere.

## What SquashFS gives you

- **Built-in kernel support** — no FUSE driver, no external tool at mount time. `mount -t squashfs image.sqfs /mnt` works on any modern kernel.
- **Multiple compression backends** — gzip, lzo, xz, lz4, zstd. `mksquashfs` picks based on command-line flags.
- **Block-level random access** — individual files can be read without decompressing the whole image.

Compression ratio is competitive with `tar.xz` and better than `tar.gz`, though typically lower than [DwarFS](dwarfs.md) on rootfs-shaped inputs.

## External-tool requirement

Building a SquashFS image requires `mksquashfs` (part of the `squashfs-tools` package on most distros). flatroot shells out — it does not re-implement the format. At mount time, the kernel's built-in driver handles everything; no userspace tooling is needed.

## How flatroot uses it

```
flatroot export ./rootfs rootfs.sqfs
```

flatroot invokes:

```
mksquashfs <rootfs> <output> -noappend -e .flatroot
```

Flags explained:

- `-noappend` — forces a clean write. Without it, `mksquashfs` *appends* to an existing image rather than overwriting it, silently merging stale contents into the new archive; the flag exists to prevent that.
- `-e .flatroot` — excludes the `.flatroot/` metadata directory. `-e` paths are source-relative, so this matches only `<rootfs>/.flatroot` and leaves any user directories named `.flatroot` elsewhere in the tree untouched. See [Export exclusion](../../reference/metadata.md#export-exclusion) for the invariant.

Resulting images mount with:

```
sudo mount -t squashfs -o loop rootfs.sqfs /mnt
```

## When to choose SquashFS over DwarFS

- **Use SquashFS** when portability matters — the target system just needs a Linux kernel, no extra packages.
- **Use [DwarFS](dwarfs.md)** when archive size matters more than consumer-side simplicity.

## Further reading

- [SquashFS — kernel docs](https://docs.kernel.org/filesystems/squashfs.html)
- [squashfs-tools project](https://github.com/plougher/squashfs-tools)
- [CLI reference — export](../../reference/cli.md#export)
- [DwarFS](dwarfs.md) — the compression-first alternative.

--8<-- "_glossary.md"
