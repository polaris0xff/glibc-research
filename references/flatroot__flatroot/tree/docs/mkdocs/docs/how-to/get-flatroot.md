---
tags:
  - how-to
  - install
  - setup
---

# Get FlatRoot

This guide walks through installing the latest FlatRoot binary on Linux. FlatRoot ships as a single self-contained executable for each supported host architecture: `x86_64`, `aarch64`, `armv7l`, `i686`, and `riscv64`. Nothing else is needed on the host — no package manager, no runtime libraries, no setup.

## Download a release binary

Fetch the latest release for your host architecture:

```bash
curl -Lo flatroot {{ FLATROOT_RELEASE_URL }}/flatroot-linux-$(uname -m)
chmod +x flatroot
```

`$(uname -m)` expands to one of the architectures listed above. If your host reports something else, there is no pre-built binary for it today.

## Install to PATH

To invoke `flatroot` from anywhere, move it into a directory on your `$PATH`.

Per-user (no root required):

```bash
mkdir -p ~/.local/bin
mv flatroot ~/.local/bin/
```

Make sure `~/.local/bin` is on your `$PATH` — most modern distributions include it by default; check with `echo $PATH`.

System-wide:

```bash
sudo mv flatroot /usr/local/bin/
```

After either, `flatroot --version` should work from any directory.

## Verify the binary

Sanity-check the install:

```bash
./flatroot --version    # local
flatroot --version      # on PATH
```

You should see a version string. If the command is not found, re-check that the binary is executable (`chmod +x`) and — for the PATH install — that the target directory is actually on your `$PATH`.

--8<-- "_glossary.md"
