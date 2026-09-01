# Static, static-PIE, LTO and ASLR

The build techniques this project uses, why each flag is there, and **how to
check the binary really is what you asked for**. The verification commands
matter as much as the flags: `file` calls a static PIE a "shared object", and
a build that quietly produced the wrong thing is the failure mode here.

---

## The build profiles

| profile | RUSTFLAGS | ELF result | ASLR on the executable |
| --- | --- | --- | --- |
| `static` | `-C target-feature=+crt-static -C relocation-model=static` | `ET_EXEC`, no `PT_INTERP` | **no** — fixed load address |
| `static-pie` | `-C target-feature=+crt-static -C relocation-model=pic` | `ET_DYN`, no `PT_INTERP` | **yes** |
| `static-lto` | as `static`, plus `CARGO_PROFILE_RELEASE_LTO=fat` | as `static` | no |
| `static-pie-lto` | as `static-pie`, plus `LTO=fat` | as `static-pie` | yes |
| `dynamic` | `-C target-feature=-crt-static` | `ET_DYN` **with** `PT_INTERP` | yes |

⭐ **`static` vs `static-pie` is the ASLR comparison.** A non-PIE executable is
loaded at a fixed address; a PIE is relocated by the kernel on every exec. That
is the practical cost/benefit the `profiles` suite measures.

### Flags applied to every static cell

```
-C link-arg=-Wl,-z,stack-size=8388608
```

⚠ musl's default thread stack is small enough that a regex engine can overflow
it on a pathological pattern. It is applied to **every** static cell **including
the baseline**, so it is a constant of the experiment rather than an advantage
given to one row.

### Cargo profile

Set through the environment, not by editing `Cargo.toml`:

```sh
CARGO_PROFILE_RELEASE_LTO=false|fat
CARGO_PROFILE_RELEASE_OPT_LEVEL=3
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
CARGO_PROFILE_RELEASE_PANIC=abort
CARGO_PROFILE_RELEASE_DEBUG=0
CARGO_PROFILE_RELEASE_STRIP=none
```

⛔ **`strip=none` is required, not cosmetic.** A stripped binary has no
`.symtab`, so the identity oracle cannot see which allocator is in it and has to
report UNPROVEN. ripgrep's own `release-lto` profile sets `strip="symbols"`,
which is exactly why that profile is not used here.

⚠ **Why not `--profile release-lto`?** It differs from `release` in *four* ways
at once — `lto`, `panic`, `strip` and `debug`. Using it for the LTO cells would
mean the LTO comparison also varied panic strategy and symbol retention. Setting
one key changes one thing.

⚠ **Binary sizes in this project's reports are therefore unstripped.** A shipped
build would strip and be smaller. The report says so under its size chart.

---

## Verifying the result

⭐ **Read the artefact, not the build log.**

### Is it static? Is it a PIE?

```sh
readelf -h target/.../rg | grep Type      # EXEC or DYN
readelf -l target/.../rg | grep INTERP    # nothing = no interpreter
```

| `Type` | `INTERP` | it is |
| --- | --- | --- |
| `EXEC` | absent | a classic static binary |
| `DYN` | **absent** | ⭐ a **static PIE** |
| `DYN` | present | dynamically linked |

⚠ **`file` gets this wrong in a way that matters.** It calls a static PIE a
"shared object", and people conclude the build failed. `ldd` is no better: on a
static PIE it may print "statically linked" or fail outright.

The instrument does this without binutils, because Alpine's base image has
neither:

```sh
alloc-runner identify --bin ./rg --expect-kind static-pie --expect-allocator mimalloc
```

It returns a JSON document with the link kind, the interpreter, the allocators
detected, the libc allocator detected, and a verdict with reasons. Exit 1 means
the binary is not the configuration it claims.

### Is ASLR actually on?

⚠ **Do not infer it from `ET_DYN`.** That is a claim about the kernel derived
from a property of the file, and it is wrong under `setarch -R`, with
`randomize_va_space=0`, or in some container configurations.

```sh
alloc-runner aslr-probe --bin ./rg --corpus /path/to/corpus --runs 6 --expect randomised
```

It starts the binary on a workload long enough to still be alive when the parent
looks, reads the load address from `/proc/<pid>/maps`, and reports how many
distinct bases it saw. ⛔ One run cannot answer this; the count out of N is what
is reported, and `--expect` turns it into an assertion.

Measured 2026-09-01 on alpine/x86_64/static-pie-lto: **6 distinct bases in 6
samples**, for every allocator.

### Which allocator is in it?

```sh
nm --defined-only ./rg | grep -E ' (T|t) (mi_malloc|je_mallocx|sn_malloc|rpmalloc|h_malloc)$'
```

or, without binutils and with a verdict rather than a list, `alloc-runner
identify` as above.

### Did libc surgery actually displace anything?

```sh
alloc-runner archive-check --archive /usr/lib/libc.a --symbol malloc --expect-providers 1
alloc-runner ar-members   --archive /usr/lib/libc.a --symbols malloc,free,calloc,realloc
```

⛔ **Two providers of `malloc` means link order decides which allocator runs**,
and nothing will fail. This is the check that makes the technique safe to use.

---

## Doing it by hand

The whole per-cell pipeline is one script, so a reader can run exactly what CI
runs:

```sh
docker run --rm -it \
  -e CELL_ID=manual -e OUTDIR=/out -e CACHE=/cache \
  -e ALLOCATOR=mimalloc -e INTEGRATION=rust-global \
  -e PROFILE=static-pie-lto -e TOOLCHAIN=distro -e LIBC=musl \
  -e TARGET_ARCH=x86_64 -e CORPUS_PROFILE=standard -e REPEAT=10 \
  -e ALLOC_REPO=https://github.com/microsoft/mimalloc \
  -e ALLOC_COMMIT=18b08671c9302247bfb682286e6bf3cc1773f801 \
  -e RG_COMMIT=e89fff89ac9af12e8d4ce9d5fd07beb408ca730f \
  -v "$PWD/out:/out" -v "$PWD/cache:/cache" \
  alloc-tests/alpine-x86_64:local \
  sh /opt/alloc-tests/scripts/build/run-cell.sh
```

The commits come from `allocators/allocators.lock.json`. `out/` then holds the
binary, the build metadata, the identity document, the correctness result, the
ASLR observation and one measurement file per workload.

To build only the binary, without measuring:

```sh
RG_SRC=/work/ripgrep OUT=/out RUNNER=/usr/local/bin/alloc-runner \
ALLOCATOR=mimalloc INTEGRATION=rust-global PROFILE=static-pie-lto \
TARGET=x86_64-unknown-linux-musl ALLOC_PREFIX=/cache/alloc/... \
  sh /opt/alloc-tests/scripts/build/build-ripgrep.sh
```

---

## Architecture notes

Both supported architectures are 64-bit little-endian, which is why the ELF
reader parses only that and reports anything else as such rather than guessing.

| | x86_64 | aarch64 |
| --- | --- | --- |
| rust target (musl) | `x86_64-unknown-linux-musl` | `aarch64-unknown-linux-musl` |
| rust target (glibc) | `x86_64-unknown-linux-gnu` | `aarch64-unknown-linux-gnu` |
| `-fcf-protection` | accepted | **rejected by GCC** — probed, not assumed |

⚠ **Nothing on aarch64 has been run yet.** See `docs/AGENTS.md` §13.

⚠ **`-C target-cpu` is deliberately left at the default.** Tuning to the
builder's CPU would make the binary non-portable and the result
non-reproducible across runners. This is the same reason hardened_malloc's
`CONFIG_NATIVE` is forced off.
