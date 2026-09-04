---
tags:
  - appendix
  - formats
---

# cpio (RPM payload)

`cpio` is a Unix archive format like tar — a stream of `(header, filename, content)` triples with no central index. It is older than tar, less widespread today, and still used in one important place: RPM packages. Every `.rpm` file contains a cpio archive as its payload.

## Format variants

cpio has multiple on-disk encodings. flatroot parses exactly one: **SVR4/newc**, the format RPM uses. Each entry begins with a 110-byte header:

| Field | Size | Encoding |
|-------|------|----------|
| Magic | 6 | `070701` (ASCII, for newc) or `070702` (CRC variant) |
| inode, mode, uid, gid | 4 × 8 | 8-digit ASCII hex |
| nlink, mtime | 2 × 8 | 8-digit ASCII hex |
| filesize | 8 | 8-digit ASCII hex |
| devmajor, devminor, rdevmajor, rdevminor | 4 × 8 | 8-digit ASCII hex |
| namesize | 8 | 8-digit ASCII hex |
| check | 8 | 8-digit ASCII hex (zero for newc, CRC for crc variant) |

The header is immediately followed by the filename (length = `namesize`), null-terminated, padded to a 4-byte boundary. Then the file content, also padded to 4 bytes. The next entry begins at the next aligned offset.

The archive ends with a distinguished entry named `TRAILER!!!`.

## How .rpm uses it

An RPM file is a custom binary format whose last section is the **payload** — a cpio archive, compressed. The outer RPM headers carry all the package metadata (name, version, scriptlets, dependencies); the cpio archive carries only the files to extract into the rootfs.

Supported payload compressions: xz (most common in modern RPMs), gzip (older packages), zstd (newer Fedora / CentOS Stream). flatroot detects the payload compression from magic bytes, decompresses into a stream, and parses the cpio entries one by one. There is no tar layer anywhere in the pipeline.

## Why cpio and not tar

RPM predates tar's dominance, and changing formats now would break every existing mirror and RPM tool. The format has held up reasonably: it streams, supports symlinks and device nodes, and carries enough metadata for extraction. The main pain point is the 8-hex-digit filesize field, which limits individual files to 4 GiB; in practice this is not a limitation for any real package.

## Further reading

- [cpio(5) man page](https://man7.org/linux/man-pages/man5/cpio.5.html)
- [RPM file format reference](https://rpm-software-management.github.io/rpm/manual/format.html)
- [CentOS/RHEL distribution notes](../../explanation/the-distro-abstraction.md)

--8<-- "_glossary.md"
