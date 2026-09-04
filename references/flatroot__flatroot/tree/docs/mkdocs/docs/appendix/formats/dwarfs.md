---
tags:
  - appendix
  - formats
  - export
---

# DwarFS

[DwarFS](https://github.com/mhx/dwarfs) — *Deduplicating Warp-speed Advanced Read-only File System* — is a compressed read-only filesystem optimized for extreme compression while preserving fast random access. It's an alternative to [SquashFS](squashfs.md), favored in workloads where archive size matters most (large read-only datasets, rootfs images, CI caches) and where CPU cost to read individual files should stay low.

## What DwarFS does differently

Two design choices distinguish DwarFS from SquashFS:

- **Segmentation and deduplication** — DwarFS splits files into content-defined segments and deduplicates across the entire filesystem before compression. Multiple copies of identical file chunks (very common in distro rootfs: duplicated documentation, shared headers, icons at identical sizes) contribute once to the compressed image.
- **Per-section algorithm choice** — DwarFS uses zstd, LZMA, or Brotli per section based on which compresses better for that data, rather than a single algorithm for the whole image.

The combination produces smaller archives than SquashFS on typical rootfs inputs, often by a factor of 2–3×.

## External-tool requirement

DwarFS is not in the Linux kernel. flatroot does not re-implement it. Exporting requires:

- `mkdwarfs` — the builder binary. flatroot shells out to it.
- `dwarfs` — the FUSE driver, needed to mount the exported image at read time.

If `mkdwarfs` is not on `$PATH` when you run `flatroot export --format dwarfs`, the command fails with a clear error. Installation varies: `dwarfs`, `dwarfs-tools`, or build from source depending on your distro.

## How flatroot uses it

```
flatroot export ./rootfs rootfs.dwarfs
```

flatroot invokes:

```
mkdwarfs -i <rootfs> -o <output> --filter "- /.flatroot/"
```

The `--filter "- /.flatroot/"` rule excludes the top-level `.flatroot/` metadata directory. The leading `/` anchors the rule at the source root; the trailing `/` forces a directory match. Both are required — without them mkdwarfs matches only regular files named `.flatroot` at any depth and silently fails to exclude the directory. See [Export exclusion](../../reference/metadata.md#export-exclusion) for the full invariant.

## When to choose DwarFS over SquashFS

- **Use DwarFS** when archive size is the overriding concern and you control the reader side (can install `mkdwarfs` tooling or the FUSE driver on the target).
- **Use [SquashFS](squashfs.md)** when you need kernel-native mount support with no extra packages at read time.

## Further reading

- [DwarFS project](https://github.com/mhx/dwarfs)
- [CLI reference — export](../../reference/cli.md#export)
- [SquashFS](squashfs.md) — the portable alternative.

--8<-- "_glossary.md"
