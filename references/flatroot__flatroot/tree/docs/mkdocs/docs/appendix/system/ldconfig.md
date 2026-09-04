---
tags:
  - appendix
  - post-install
  - glibc
---

# ldconfig

`ldconfig(8)` is the glibc utility that builds `/etc/ld.so.cache` — the lookup index used by the dynamic linker to find shared libraries at program startup. Without a current cache, programs that link against libraries outside the hardcoded standard paths (`/lib`, `/usr/lib`) fail to start with "cannot open shared object file: No such file or directory", even when the library is physically present in the rootfs.

## Why flatroot runs it

After package extraction, the rootfs contains shared libraries scattered across `/lib`, `/usr/lib`, architecture-specific paths (`/usr/lib/x86_64-linux-gnu/`), and distribution-specific locations (e.g., `/usr/lib64/` on RPM distros). None of these are known to the dynamic linker until ldconfig scans them and writes the cache.

flatroot runs ldconfig once per install, immediately after extraction finishes and before any post-install script executes. This guarantees that post-install scripts can invoke binaries linked against newly-installed libraries without hitting linker errors mid-script.

## Why it's glibc-only

musl libc (used by Alpine) does not ship an `ldconfig`. musl uses a different algorithm for locating shared libraries that doesn't require a precomputed cache. flatroot probes the rootfs for an `ldconfig` binary in the standard executable directories (`usr/bin`, `usr/sbin`, `usr/local/bin`, `usr/local/sbin`, `bin`, `sbin`) and prints `ldconfig not found, skipping` to stderr when none is present. This is the correct behavior for Alpine and for any rootfs intentionally built without glibc.

## Invocation

ldconfig is invoked inside FlatRoot's user-namespace sandbox — the same sandbox used for post-install scripts. The implementation is a custom `unshare(CLONE_NEWUSER | CLONE_NEWNS)` + `pivot_root` in `src/sandbox.rs`, conceptually equivalent to [bubblewrap](bubblewrap.md). The rootfs is bind-mounted as `/`, and the rootfs-native `sbin/ldconfig` is executed — never the host's ldconfig. The host binary could read a different cache format (different glibc version) and produce a corrupt cache.

## Further reading

- [ldconfig(8) man page](https://man7.org/linux/man-pages/man8/ldconfig.8.html)
- [Shared-library sonames](soname.md) — the naming convention ldconfig indexes.
- [Stub Commands](../postinstall/stubs.md) — what runs alongside ldconfig in the post-install pipeline.

--8<-- "_glossary.md"
