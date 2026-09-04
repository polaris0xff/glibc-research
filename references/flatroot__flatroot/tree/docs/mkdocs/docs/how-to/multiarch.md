---
tags:
  - how-to
  - multiarch
  - install
---

# Multiple Architectures

This guide shows how to install packages for more than one CPU architecture into the same [rootfs](../appendix/system/rootfs.md). Some applications need that: Wine and Steam pull 32-bit libraries alongside 64-bit ones, and cross-compilation toolchains may need a second architecture to produce binaries for the target platform. Pass a comma-separated list to `--arch` and FlatRoot runs the install pipeline once per architecture, dropping each set of libraries into its own architecture-specific path so they do not conflict.

## Install for multiple architectures

Pass a comma-separated list to `--arch`:

```bash
flatroot --from debian:bookworm --arch x86_64,i686 install --output ./root wine
```

??? warning "Comma-separated, no spaces"

    The `--arch` list is split on commas; any whitespace becomes part of the architecture name. `--arch x86_64,i686` works; `--arch "x86_64, i686"` produces an entry ` i686` (with leading space) that's rejected as an unknown architecture.

FlatRoot runs a separate resolve-download-extract pass for each architecture, all into the same rootfs. Libraries are placed in architecture-specific paths so they don't conflict:

```
./root/usr/lib/x86_64-linux-gnu/   # x86_64 libraries
./root/usr/lib/i386-linux-gnu/      # i686 libraries
```

??? warning "Missing packages abort the install"

    If a user-requested package doesn't exist for one of the architectures in `--arch`, flatroot aborts with `Package 'X' not found in the index`. Architectures are processed sequentially, so later passes are never attempted after a failure, and files extracted by earlier passes remain on disk (no rollback). Before running a multiarch install, confirm the package exists for every target: `flatroot --arch <each-arch> search '<pattern>'`.

## Other use cases

```bash
# Cross-compilation targeting aarch64
flatroot --from debian:bookworm --arch x86_64 install --output ./cross gcc-aarch64-linux-gnu libc6-dev

# Pure 32-bit environment
flatroot --from debian:bookworm --arch i686 install --output ./root32 python3
```

## Architecture library paths

| Architecture | Debian library path |
|-------------|------|
| `x86_64` | `/usr/lib/x86_64-linux-gnu/` |
| `i686` | `/usr/lib/i386-linux-gnu/` |
| `aarch64` | `/usr/lib/aarch64-linux-gnu/` |
| `armv7l` | `/usr/lib/arm-linux-gnueabihf/` |
| `riscv64` | `/usr/lib/riscv64-linux-gnu/` |

??? tip "Picking compatible distributions"

    Mixing architectures within one rootfs means picking a distribution release whose C library supports all of them. The [glibc version table](../appendix/versioning/glibc.md) lists which flatroot remotes ship which glibc version and which ABIs each one serves.

??? note "Running a cross-arch rootfs"

    A cross-arch rootfs holds foreign-ELF binaries; the host kernel cannot execute them directly. To run them: (a) register qemu-user handlers with `binfmt_misc` so the host dispatches foreign binaries through qemu transparently — `bwrap` and `chroot` then just work — or (b) invoke binaries explicitly through the user-mode emulator, e.g. `qemu-aarch64 ./rootfs/bin/sh` with `QEMU_LD_PREFIX=./rootfs`. See [bubblewrap](../appendix/system/bubblewrap.md) and [chroot](../appendix/system/chroot.md) for the wrapper side.

--8<-- "_glossary.md"
