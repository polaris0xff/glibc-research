# Runtime Behavior of Statically Linked glibc Binaries: NSS Dynamic Module Loading

## 1. Purpose and Scope

This document consolidates research into what `gcc -static` actually produces on a glibc-based Linux system, and what happens inside such a binary at runtime when it performs name resolution.

Two groups of engineers rely on static linking as a guarantee of a self-contained execution environment: builders of distroless containers, and developers of embedded or industrial control (OT/ICS) gateways. In the glibc ecosystem, that assumption is false: with the default configuration shipped by most distributions, the NSS (Name Service Switch) dispatcher silently loads the dynamic linker, third-party `.so` modules, and a second, full copy of `libc.so.6` into the "static" process at runtime — negating the isolation the build was meant to provide.

Scope of this document:

- How static linking is used as an isolation mechanism and why it is trusted.
- How the glibc NSS subsystem is architected, including its plugin model.
- The glibc version history of NSS module loading (pre- and post-2.34).
- Measured runtime behavior of `-static` glibc binaries across three distributions.
- Why this is an architectural property of the design rather than a memory-safety bug.
- The vendor's (Red Hat Product Security) official position and guidance.
- The reliable mitigation: static linking against musl libc instead of glibc.
- A build and verification toolchain for auditing your own binaries.

## 2. Background: Why Static Linking Is Chosen

### 2.1 Embedded systems have grown full operating systems

Microcontrollers have become powerful enough that embedded systems are no longer limited to bare metal and RTOS deployments. Running an industrial gateway or PLC on a full Debian installation with systemd in production is now commonplace, because engineers reuse off-the-shelf IT solutions to speed up time-to-market.

### 2.2 The cost of system redundancy

This redundancy carries a real cost. In enterprise IT, extra code merely consumes RAM and latency only degrades user experience. In industrial control, jitter breaks determinism: if background OS activity makes a controller pause for even half a second and miss a control cycle, a valve actuates late. On a steelmaking converter, that shows up as overheating or excess oxygen/gas consumption, and in practice can mean scrapping an entire batch of steel.

### 2.3 Inverted priorities

This inverts the classic security hierarchy. Enterprise IT orders its priorities Confidentiality → Integrity → Availability. Industrial control systems order them the opposite way: there, a denial of service is not merely unhappy users — it means a production shutdown or an industrial disaster.

### 2.4 Static linking as the isolation answer

To protect Availability and minimize exposure to the host environment, architects turn to static linking. The reasoning is sound on its face: pack every dependency inside the executable, cut off external shared libraries, and eliminate an entire class of environment-dependent failures. The standard tools appear to confirm success:

```bash
file ./gateway
# ELF 64-bit LSB executable, ARM aarch64, ... statically linked

ldd ./gateway
# not a dynamic executable
```

The binary looks fully autonomous. This is the assumption the rest of this document examines — and it is exactly where glibc's design intervenes.

## 3. The NSS (Name Service Switch) Architecture in glibc

### 3.1 Configuration-driven dispatch

glibc does not hardcode how hostnames, users, or services are resolved. Instead, it reads `/etc/nsswitch.conf`, whose `hosts:` line defines an ordered chain of resolver modules. A typical modern distribution default:

```text
hosts: files myhostname mdns4_minimal [NOTFOUND=return] resolve [!UNAVAIL=return] dns
```

Bracketed tokens such as `[NOTFOUND=return]` and `[!UNAVAIL=return]` are action modifiers that control whether the lookup chain continues to the next module based on each module's result status.

Note the ordering implication in some defaults (e.g. `resolve` placed before `files` with `[!UNAVAIL=return]`): if the resolution daemon is down, the whole lookup fails rather than falling through to local files.

### 3.2 Runtime module loading via dlopen()

When an application calls `getaddrinfo()`, glibc's NSS dispatcher walks this chain and loads each non-builtin module with `dlopen()`:

- `libnss_myhostname.so.2` — provided by systemd
- `libnss_mdns4_minimal.so.2` — provided by Avahi (mDNS)
- `libnss_resolve.so.2` — provided by systemd-resolved
- `libnss_dns.so.2` / `libnss_files.so.2` — on glibc older than 2.34

Because module loading goes through the dynamic loader's standard `dlopen()` machinery, which module objects actually enter the process is decided by the host environment at execution time, not at build time.

### 3.3 Module interface contract

Each NSS module is a shared object exporting functions named after the pattern `_nss_<module>_<service>_r`. For hostname resolution, the dispatcher looks up symbols of the following shape (shown for the `resolve`, `dns`, `myhostname`, `mdns4_minimal`, and `files` modules):

```text
_nss_<module>_gethostbyname4_r()
_nss_<module>_gethostbyname3_r()
_nss_<module>_gethostbyname2_r()
_nss_<module>_gethostbyname_r()
```

These functions return an NSS status code; the success status, `NSS_STATUS_SUCCESS`, is defined as `2`. A minimal reference module therefore consists of these exported stubs returning the appropriate status codes.

## 4. glibc Version History: Builtin vs. External Modules

### 4.1 Before glibc 2.34 ("ghosts of the past")

Prior to glibc 2.34, calling `getaddrinfo()` in a static binary still parsed `/etc/nsswitch.conf` and loaded the `files` and `dns` NSS modules — `libnss_files.so.2` and `libnss_dns.so.2` — at runtime. This was a well-known architectural behavior that broke container and chroot isolation for years. Even a minimal `hosts: files dns` configuration triggered the runtime loads.

### 4.2 glibc 2.34: the "Great Merge"

glibc 2.34 made a significant change: the `files` and `dns` plugins were built straight into libc itself. The release notes state the motivation directly — safe operation inside chroot environments and containers. After this merge, `dlopen()` is no longer needed for those two modules, and many engineers reasonably concluded the runtime-loading problem was solved and began mass-producing static builds for distroless containers and industrial gateways.

### 4.3 Only the defaults were merged

In the releases since, only the default `files` and `dns` modules were merged. The NSS ecosystem still relies on external modules, and modern distributions are tightly coupled to systemd. The default `/etc/nsswitch.conf` on Arch Linux and Fedora ships with `resolve`, `mymachines`, `myhostname`, and `mdns4` entries. All of these modules remain dynamic and are loaded at runtime through the very same `dlopen()` path — regardless of the glibc version in use.

## 5. The Cascade Effect

Tracing a supposedly self-contained static binary on a current Fedora illustrates the full consequence:

1. The binary tries to resolve a hostname; the request enters the NSS dispatcher.
2. The configured chain contains `resolve`, so glibc issues a `dlopen()` for `libnss_resolve.so.2`.
3. That module is itself a dynamic library — so loading it requires the dynamic linker `ld-linux-*.so`.
4. Right behind the dynamic linker comes a second, dynamic copy of `libc.so.6`, mapped into the address space of the "isolated" process.

The result: a process that `ldd` describes as "not a dynamic executable" ends up containing the dynamic loader, the shared libc, and multiple third-party shared objects — the static isolation guarantee is invalidated at its core.

## 6. The Toolchain Already Warns

When compiling with `-static`, the linker emits an explicit warning for any program using `getaddrinfo()`:

```text
warning: Using 'getaddrinfo' in statically linked applications requires at runtime
the shared libraries from the glibc version used for linking
```

In other words, the toolchain itself states that fully static linking of NSS-dependent code paths with glibc does not exist. The warning is easy to miss in build output — and is routinely ignored.

## 7. Test Program

The following minimal resolver client is sufficient to trigger and study the behavior. It performs a plain forward DNS resolution through the standard `<netdb.h>` API — no unusual APIs or special tricks are required, which is precisely the point: any static glibc program that resolves a name takes this path.

`src/main.c`:

```c
#include <stdio.h>
#include <string.h>
#include <netdb.h>
#include <sys/types.h>
#include <sys/socket.h>

int main(void) {
    struct addrinfo hints, *res;

    /* Zero-initialize hints so no stale stack fields influence the lookup. */
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;     /* Accept both IPv4 and IPv6 results. */
    hints.ai_socktype = SOCK_STREAM; /* TCP stream sockets. */

    printf("[+] Gateway started. Resolving example.com...\n");

    /* A fresh getaddrinfo() call forces glibc to consult the NSS chain
       rather than serve the answer from local caches such as nscd or
       systemd-resolved. */
    int status = getaddrinfo("example.com", "80", &hints, &res);

    if (status != 0) {
        printf("[-] Resolution failed: %s\n", gai_strerror(status));
        return 1;
    }

    printf("[+] DNS resolution finished.\n");
    freeaddrinfo(res);
    return 0;
}
```

## 8. Measured Behavior Across Tested Environments

The behavior was verified on three distributions spanning both the pre-merge and post-merge glibc eras.

### 8.1 Fedora 44 — glibc 2.43, ARM64 (aarch64)

- NSS chain: `files myhostname mdns4_minimal [NOTFOUND=return] resolve [!UNAVAIL=return] dns`
- `files` and `dns` are builtin at this glibc version; `myhostname`, `mdns4_minimal`, and `resolve` are external and runtime-loaded. The glibc being one release behind current at test time is irrelevant here — external modules load via `dlopen()` regardless of version.
- Static binary size: 4.1 MB (glibc) vs. 491 KB (musl).
- Runtime trace of the `-static` glibc binary:

```text
openat(AT_FDCWD, "/lib64/libnss_myhostname.so.2", O_RDONLY|O_CLOEXEC) = 3
openat(AT_FDCWD, "/lib64/libnss_mdns4_minimal.so.2", O_RDONLY|O_CLOEXEC) = 3
openat(AT_FDCWD, "/lib64/libnss_resolve.so.2", O_RDONLY|O_CLOEXEC) = 3
openat(AT_FDCWD, "/lib64/libc.so.6", O_RDONLY|O_CLOEXEC) = 3          # cascaded dynamic libc
openat(AT_FDCWD, "/lib/ld-linux-aarch64.so.1", O_RDONLY|O_CLOEXEC) = 3  # cascaded dynamic linker
```

- Despite loading three external NSS modules plus the dynamic libc and dynamic linker at runtime, `file` reports "statically linked" and `ldd` reports "not a dynamic executable".

### 8.2 Debian 10 Buster — glibc 2.28, ARM64 (aarch64)

- NSS chain: `files mdns4_minimal [NOTFOUND=return] dns`
- At this glibc version neither `files` nor `dns` is builtin, so all three configured modules are runtime-loaded.
- Static binary size: 653 KB (glibc) vs. 90 KB (musl).
- Runtime trace of the `-static` glibc binary:

```text
openat(AT_FDCWD, "/lib/aarch64-linux-gnu/libnss_dns.so.2", O_RDONLY|O_CLOEXEC) = 3
openat(AT_FDCWD, "/lib/aarch64-linux-gnu/libnss_files.so.2", O_RDONLY|O_CLOEXEC) = 3
openat(AT_FDCWD, "/lib/aarch64-linux-gnu/libnss_mdns4_minimal.so.2", O_RDONLY|O_CLOEXEC) = 3
openat(AT_FDCWD, "/lib/aarch64-linux-gnu/libc.so.6", O_RDONLY|O_CLOEXEC) = 3          # cascaded dynamic libc
openat(AT_FDCWD, "/lib/aarch64-linux-gnu/ld-linux-aarch64.so.1", O_RDONLY|O_CLOEXEC) = 3  # cascaded dynamic linker
```

- On legacy glibc, even the `files` module is dynamic, and during a single name resolution `libnss_files.so.2` can be opened hundreds of times.

### 8.3 Arch Linux — glibc 2.44, x86_64

- NSS chain: `mymachines resolve [!UNAVAIL=return] files myhostname dns`
- `files` and `dns` are builtin; `mymachines`, `resolve`, and `myhostname` are external and runtime-loaded.
- Static binary size: 1.1 MB (glibc) vs. 77 KB (musl).
- Runtime trace of the `-static` glibc binary:

```text
openat(AT_FDCWD, "/usr/lib/libnss_mymachines.so.2", O_RDONLY|O_CLOEXEC) = 3
openat(AT_FDCWD, "/usr/lib/libnss_resolve.so.2", O_RDONLY|O_CLOEXEC) = 3
openat(AT_FDCWD, "/usr/lib/libc.so.6", O_RDONLY|O_CLOEXEC) = 3          # cascaded dynamic libc
openat(AT_FDCWD, "/usr/lib/ld-linux-x86-64.so.2", O_RDONLY|O_CLOEXEC) = 3  # cascaded dynamic linker
```

### 8.4 Module loading mode matrix

Which modules enter the process at runtime depends on the glibc version and the distribution's default `/etc/nsswitch.conf`:

| glibc version | Module | Loading mode | Notes |
| :--- | :--- | :--- | :--- |
| < 2.34 (e.g. Debian 10) | `dns`, `files` | External, via `dlopen()` | Neither module is builtin; even minimal `hosts: files dns` triggers runtime loading |
| ≥ 2.34 (e.g. Fedora 44, Arch) | `dns`, `files` | Builtin | No `dlopen()` for these two modules |
| Any (systemd distros) | `resolve` | External, via `dlopen()` | Runtime-loaded regardless of glibc version |
| Any (systemd distros) | `myhostname` | External, via `dlopen()` | Runtime-loaded regardless of glibc version |
| Any (systemd distros) | `mymachines` | External, via `dlopen()` | Runtime-loaded regardless of glibc version |
| Any (Avahi installed) | `mdns4_minimal` | External, via `dlopen()` | Runtime-loaded regardless of glibc version |

An immediate consequence of this matrix: on glibc ≥ 2.34, a `hosts:` chain restricted to the builtin `files` and `dns` modules involves no runtime library loading, while any additional entry re-introduces it.

## 9. Architectural Property, Not a Memory-Safety Bug

This behavior is an architectural limitation of glibc's NSS dispatcher, not a memory corruption issue:

- No memory corruption of any kind is involved — the loads go through a legitimate runtime facility working exactly as designed.
- Compile-time hardening options (ASLR, NX, stack canaries, RELRO) have no bearing on this behavior, since it is not a memory-safety problem to begin with.
- The `dlopen()` call is issued by the NSS dispatcher itself, as designed.
- The behavior persists across glibc versions by migrating between modules: `dns`/`files` before 2.34, `resolve`/`mdns`/`myhostname`/`mymachines` after.

This is also why targeted patches have historically been powerless against the model: fixing one module's loading path does not change the plugin architecture that the dispatcher is built on.

## 10. Vendor Position: Red Hat Product Security

The behavior was reviewed with Red Hat Product Security, which issued an official assessment of it. Their conclusions, paraphrased:

1. **Mechanism confirmed as intended design.** Paraphrasing Red Hat's assessment: when NSS APIs are exercised, a `-static` build on a glibc-based system is not an artifact that is fully isolated from its environment — this follows from how glibc's NSS model is architected. Any `getaddrinfo()` call makes the NSS dispatcher execute `dlopen()` on external `.so` modules. The vendor also draws the execution-context boundary: outside the kernel's secure-execution mode (AT_SECURE), the dynamic loader honors its environment-controlled settings during these runtime loads.
2. **Static glibc is not a supported isolation model.** Red Hat's stance is that linking statically against the C/C++ runtime is outside their supported application models, and `-static` holds no value as a hardening control. Genuine isolation from shared-object loading calls for a libc designed without NSS plugins — musl being the vendor's example.
3. **Documentation and upstream direction.** Red Hat is reviewing its documentation to explicitly detail the limitations of `-static` in official guidance, and confirmed openness to upstream discussions with the glibc team on technical solutions such as static NSS archives or build flags to completely disable `dlopen()` in NSS. The vendor also acknowledged that glibc's current behavior strays from industry security expectations.

## 11. Mitigation: musl libc

The reliable way to achieve true static isolation on Linux is to replace glibc with musl, which implements name resolution natively and has no NSS plugin system. The musl documentation states this plainly: "musl does not have (or want) NSS."

A statically linked musl binary performs resolution entirely internally. Runtime tracing confirms it opens only standard configuration files — no shared libraries at all:

```text
$ strace ./gateway_musl 2>&1 | grep -E "openat.*\.so"
# (no output — zero shared libraries loaded)

$ strace ./gateway_musl 2>&1 | grep -E "openat.*/etc/"
openat(AT_FDCWD, "/etc/hosts", O_RDONLY|O_LARGEFILE|O_CLOEXEC) = 3
openat(AT_FDCWD, "/etc/resolv.conf", O_RDONLY|O_LARGEFILE|O_CLOEXEC) = 3
```

Toolchain availability:

- Debian/Ubuntu: `sudo apt install musl-tools`
- Fedora/RHEL: `sudo dnf install musl-gcc`
- Alpine: `apk add musl-dev` (native)

As a side observation from the measured builds, the musl static binaries are also an order of magnitude smaller than their glibc counterparts (491 KB vs. 4.1 MB on Fedora; 90 KB vs. 653 KB on Debian 10; 77 KB vs. 1.1 MB on Arch).

## 12. Security by Subtraction

The NSS behavior is a practical illustration of a broader design principle for critical and embedded systems: resilience is not achieved by stacking "defense-in-depth" controls (security theater), but by reducing the exposed surface baked in at design time — eliminating everything unnecessary rather than adding protection around it.

In critical infrastructure, shortcomings cannot be addressed piecemeal; security must be designed holistically from the architectural phase, covering the whole stack from OS and kernel down to compiler and runtime. A concrete five-layer subtraction blueprint for an isolation-critical build:

| Layer | Subtraction applied |
| :--- | :--- |
| OS & storage | No general-purpose OS; SquashFS + dm-verity; W^X (Write XOR Execute) memory policy |
| Kernel & isolation | Monolithic kernel with loadable modules disabled (`CONFIG_MODULES=n`); seccomp-bpf + Landlock |
| Runtime & libc | Drop glibc and migrate to musl |
| Compiler & CFI | Dead-code elimination (`--gc-sections`); LTO; `-fno-exceptions`; control-flow integrity |
| Supply chain | Reproducible builds; minimal dependencies |

Choosing a libc without a runtime plugin system is the runtime-layer instance of this principle; static linking itself is the library-layer instance.

## 13. Build and Audit Toolchain

### 13.1 Makefile

Two build variants: the glibc static binary whose runtime behavior is under study, and the musl static binary serving as the isolation reference.

```make
CC ?= gcc
MUSL_CC ?= musl-gcc
CFLAGS = -Wall -Wextra

.PHONY: all clean
all: gateway gateway_musl

# Statically linked glibc build — subject of the runtime-loading study.
gateway: src/main.c
	$(CC) $(CFLAGS) -static -o $@ $<

# Statically linked musl build — reference for full static isolation.
gateway_musl: src/main.c
	@if command -v $(MUSL_CC) >/dev/null 2>&1; then \
		$(MUSL_CC) $(CFLAGS) -static -o $@ $<; \
		echo "[+] gateway_musl compiled successfully."; \
	else \
		echo "[-] musl-gcc not found. Skipping gateway_musl build."; \
	fi

clean:
	rm -f gateway gateway_musl
```

### 13.2 Isolation audit script

Condensed audit harness: detects the local environment, classifies every configured NSS module by loading mode, builds both variants, and verifies each with `file`, `ldd`, and `strace`.

```bash
#!/bin/bash
# audit.sh — isolation audit for statically linked binaries.
# Detects the local libc/NSS environment, builds glibc and musl static
# variants, and measures which shared objects each one loads at runtime.

set -uo pipefail

IS_MODERN_GLIBC=0
IS_MUSL_SYSTEM=0
GLIBC_VERSION="Unknown"

command_exists() { command -v "$1" >/dev/null 2>&1; }

detect_environment() {
    if ! command_exists ldd; then return; fi
    local ldd_out
    ldd_out=$(ldd --version 2>&1 | head -n 1)

    if echo "$ldd_out" | grep -iq "musl"; then
        IS_MUSL_SYSTEM=1
        GLIBC_VERSION="musl"
        return
    fi

    local ver
    ver=$(echo "$ldd_out" | grep -oE '[0-9]+\.[0-9]+')
    if [[ -n "$ver" ]]; then
        GLIBC_VERSION="$ver"
        local minor
        minor=$(echo "$ver" | cut -d. -f2)
        # glibc 2.34+ has builtin dns and files
        if [[ "$minor" -ge 34 ]]; then
            IS_MODERN_GLIBC=1
        fi
    fi
}

audit_environment() {
    echo "== C standard library =="
    if [[ "$IS_MUSL_SYSTEM" -eq 1 ]]; then
        echo "  musl libc (no NSS plugin system)"
    else
        echo "  glibc version: $GLIBC_VERSION"
        if [[ "$IS_MODERN_GLIBC" -eq 1 ]]; then
            echo "  'dns' and 'files' are builtin"
        else
            echo "  'dns' and 'files' are external (loaded via dlopen())"
        fi
    fi

    echo "== NSS configuration (/etc/nsswitch.conf) =="
    if [[ -f /etc/nsswitch.conf ]]; then
        local hosts_line
        hosts_line=$(grep -E "^hosts:" /etc/nsswitch.conf 2>/dev/null || echo "")
        if [[ -n "$hosts_line" ]]; then
            echo "  $hosts_line"
            echo "  Module loading analysis:"
            for module in files dns myhostname mdns4_minimal resolve mymachines; do
                if echo "$hosts_line" | grep -qw "$module"; then
                    case "$module" in
                        files|dns)
                            if [[ "$IS_MODERN_GLIBC" -eq 1 ]]; then
                                echo "    $module — builtin (no runtime loading on glibc >= 2.34)"
                            else
                                echo "    libnss_${module}.so.2 — external, loaded via dlopen()"
                            fi
                            ;;
                        resolve|myhostname|mdns4_minimal|mymachines)
                            echo "    libnss_${module}.so.2 — external, loaded via dlopen()"
                            ;;
                    esac
                fi
            done
        else
            echo "  No 'hosts:' directive found"
        fi
    fi
}

audit_binary() {
    local bin="$1"
    echo "== $bin: static verification =="
    echo "  file: $(file "$bin" | cut -d: -f2 | xargs)"
    echo "  ldd:  $(ldd "$bin" 2>&1 | head -n1)"

    echo "== $bin: runtime shared-object loads (strace) =="
    local trace_output
    trace_output=$(strace -e trace=open,openat "./$bin" 2>&1 || true)
    local libs
    libs=$(echo "$trace_output" | grep -oE '"[^"]*\.so[^"]*"' | tr -d '"' | sort -u)
    if [[ -n "$libs" ]]; then
        echo "$libs" | sed 's/^/  LOADED: /'
    else
        echo "  ISOLATION VERIFIED: no external shared libraries loaded."
    fi
}

detect_environment
audit_environment

echo "== Building =="
make

[[ -f ./gateway ]]      && audit_binary gateway
[[ -f ./gateway_musl ]] && audit_binary gateway_musl
```

### 13.3 Manual verification commands

```bash
make

# Confirm the binary reports as statically linked
file ./gateway
ldd ./gateway          # "not a dynamic executable"

# Observe what it actually loads at runtime
strace -e trace=open,openat ./gateway 2>&1 | grep -E "openat.*\.so"

# Compare with the musl variant
strace -e trace=open,openat ./gateway_musl 2>&1 | grep -E "openat.*\.so"   # empty
```

## 14. Practical Guidance

1. **Do not treat `-static` + glibc as an isolation guarantee.** The vendor's own position is that this combination is unsupported as a hardening model, and that runtime NSS module loading is intended behavior.
2. **Audit with `strace`, not `ldd`.** `ldd`/`file` report the link-time view; only syscall tracing reveals the runtime loads. Grep the trace for `.so` opens.
3. **Read the linker warning.** "Using 'getaddrinfo' in statically linked applications requires at runtime the shared libraries from the glibc version used for linking" is the toolchain telling you the build is not truly self-contained.
4. **Check the `hosts:` chain on target systems.** On glibc ≥ 2.34, only the builtin `files` and `dns` modules avoid runtime loading; every systemd/Avahi module (`resolve`, `myhostname`, `mymachines`, `mdns4_minimal`) re-introduces it.
5. **For workloads that must be genuinely self-contained, build against musl.** Static musl binaries load zero shared objects at runtime, read `/etc/hosts` and `/etc/resolv.conf` directly, and resolve internally — verified by syscall tracing on all tested systems.
6. **Apply subtraction at design time.** Prefer components without runtime plugin systems for isolation-critical deployments; treat security as architectural elimination of unnecessary surface, not as added controls.

## 15. Tested Environments Summary

| Environment | Kernel | Architecture | glibc | NSS `hosts:` chain | Runtime-loaded modules observed |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Fedora 44 | 7.0.12-201.fc44 | aarch64 | 2.43 | `files myhostname mdns4_minimal [NOTFOUND=return] resolve [!UNAVAIL=return] dns` | `myhostname`, `mdns4_minimal`, `resolve` (+ dynamic libc, dynamic linker) |
| Debian 10 Buster | 4.19.0-9-arm64 | aarch64 | 2.28 | `files mdns4_minimal [NOTFOUND=return] dns` | `files`, `dns`, `mdns4_minimal` (+ dynamic libc, dynamic linker) |
| Arch Linux | 7.1.5-zen1-1-zen | x86_64 | 2.44 | `mymachines resolve [!UNAVAIL=return] files myhostname dns` | `mymachines`, `resolve`, `myhostname` (+ dynamic libc, dynamic linker) |

In all three environments, the musl static variant loaded zero shared libraries, opening only `/etc/hosts` and `/etc/resolv.conf`.
