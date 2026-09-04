---
tags:
  - tutorial
  - pinning
  - reproducibility
  - install
---

# Reproducible Builds

In this tutorial we will compile a small C program twice — each time against a pinned Debian toolchain installed into its own [rootfs](../appendix/system/rootfs.md) — and confirm the resulting binaries are byte-for-byte identical. By the end you will have performed a reproducible build: the same source, compiled against the same toolchain, produced the same bytes on disk.

## Before we start

Work in a fresh directory. If you don't have flatroot yet, follow [Get FlatRoot](../how-to/get-flatroot.md).

## Build the first toolchain

Install a pinned Debian toolchain — `gcc` and the C runtime headers — into a fresh rootfs:

```bash
./flatroot --from debian:bookworm@2024-06-15 install -o ./build-a gcc libc6-dev
```

The `@2024-06-15` suffix tells flatroot to use Debian's snapshot mirror for that exact date. flatroot fetches the package index as it was on that day, resolves dependencies, and extracts the resulting closure into `./build-a`.

## Build the second toolchain

Run the same command into a different directory:

```bash
./flatroot --from debian:bookworm@2024-06-15 install -o ./build-b gcc libc6-dev
```

This run is quicker — flatroot reuses the packages it already downloaded.

## Write the program

A trivial C program is enough to see the effect:

```bash
cat > hello.c <<'EOF'
int main() { return 0; }
EOF
```

## Compile it in the first environment

Copy the source into the first rootfs and compile it there:

```bash
cp hello.c ./build-a/hello.c
sudo chroot ./build-a /usr/bin/gcc -o /hello /hello.c
```

`sudo chroot` runs `gcc` with `./build-a` as the root of the filesystem. The output binary lands at `./build-a/hello`.

## Compile it in the second environment

Do the same thing in the second rootfs:

```bash
cp hello.c ./build-b/hello.c
sudo chroot ./build-b /usr/bin/gcc -o /hello /hello.c
```

## Compare the two binaries

Compute a SHA-256 for each compiled program:

```bash
sha256sum ./build-a/hello ./build-b/hello
```

You will see two lines with identical hashes. The two `hello` binaries — each compiled independently, in its own rootfs, by its own instance of `gcc` — are byte-for-byte the same.

This is reproducibility at the level of a build, not just provisioning. Pinning the Debian source to `@2024-06-15` gave us the exact same `gcc`, the exact same C runtime headers, and the exact same linker in both environments. Running that toolchain against the same source produced the same bytes. Anyone — you, a colleague, a build farm six months from now — running this sequence against the same pin will get the same binary.

## Clean up

Remove the two rootfs, the source, and the binary:

```bash
rm -rf ./build-a ./build-b ./flatroot ./hello.c
```

## What we did

- Built two identical toolchains by installing a pinned Debian source into two rootfs.
- Compiled the same C program against each toolchain, using `sudo chroot` to run `gcc` inside each rootfs.
- Verified the two resulting binaries were byte-identical via SHA-256.
- Observed how pinning enables reproducible builds: a deterministic toolchain is the missing ingredient that makes "same source → same binary" hold across time and machines.

flatroot's role here is narrow and important — it delivers a deterministic build environment on demand, from an upstream distribution's own archives, without a running package manager. The reproducibility of `hello` itself is a property of the toolchain; flatroot just makes sure you get the same toolchain every time.

--8<-- "_glossary.md"
