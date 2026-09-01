# Troubleshooting

Failures that have actually happened here, what they mean, and what to do. Every
entry was observed on a real run — none is hypothetical.

---

## Reading a failed cell

`alloc-bench` prints one line per cell with an outcome. They are not
interchangeable:

| outcome | meaning | is it a defect? |
| --- | --- | --- |
| `ok` | built, identified, passed correctness, measured | no |
| `unsupported` | **cannot exist**, with a technical reason | ⭐ no — this is a published result |
| `build_failed` | the build was attempted and failed | usually yes |
| `identity_failed` | it built, but is not the configuration it claims | ⛔ **always investigate** |
| `correctness_failed` | it built and is wrong | ⛔ always investigate |
| `measurement_failed` | no workload produced a usable sample | yes |

Everything a cell wrote is under `results/<run>/cells/<cell-id>/`:

```
steps.log          every step's output, in order
build.log          cargo's output
build.json         flags, tool versions, binary size
identity.json      what the ELF says is actually in the binary
correctness.json   each check with its expected and actual value
measure-*.json     raw samples
status, reason     the one-word outcome and why
```

Start with `reason`, then `steps.log`.

---

## `identity_failed`

⛔ **Never "fix" this by relaxing the check.** If the oracle rejects a cell, the
cell is wrong.

### `no symbol evidence of <allocator> in the binary`

The archive did not make it into the link. Check `build.json` for the
`rustflags` actually used, and that `alloc-meta.env` exists in the cell
directory. For `rust-global` this should be impossible — the shim binds to
prefixed symbols, so a missing archive fails the **link**. If you see it, the
shim probably fell back to the `system` backend because the cargo feature name
did not match the allocator id.

### `binary has no .symtab (stripped)`

⚠ **This is "the instrument could not look", not "the allocator is absent".**
Something set `strip`. `build-ripgrep.sh` sets
`CARGO_PROFILE_RELEASE_STRIP=none` precisely to prevent it; a `Cargo.toml`
profile or a `RUSTFLAGS` `-C strip` will override.

### `replacement build still contains the <libc> allocator implementation`

For `libc-surgery`/`link-override`, the displaced allocator must be gone. Two
causes, and they need opposite responses:

1. **The surgery really did not displace it.** Check `steps.log` for
   `surgery: displacing N member(s)`. If N is 0 the script exits 1 already.
2. **A false positive in the signature.** ⚠ This happened here on the first
   libc-surgery run: `__libc_malloc` was in the glibc signature, but mimalloc
   *also* defines it as a compatibility alias, so a correct musl replacement was
   reported as containing glibc. The fix was to key the negative control on
   symbols only the displaced implementation can have — `_int_malloc`,
   `tcache_init` — never on a name an allocator might alias.

### `baseline binary contains candidate allocator(s)`

⛔ The control is contaminated: an allocator leaked into the baseline image. The
usual cause is a `libc-surgery` cell's patched `libc.a` persisting into a later
cell. Cells run in `--rm` containers, so this should not happen; if it does, the
image itself was rebuilt with surgery applied.

---

## `build_failed`

### `multiple definition of __libc_malloc / calloc / free ...`

⭐ **This is the expected outcome of `link-override` on musl-static, and it is a
result, not a bug.** Pulling the allocator's whole archive in ahead of libc
brings its compatibility aliases, which collide with musl's own. It is exactly
why `libc-surgery` deletes the displaced members instead. Observed 2026-09-01;
evidence in `results/published/2026-09-01-mechanisms-x86_64/evidence/`.

⛔ **Do not add `--allow-multiple-definition`.** That makes link order decide
which allocator serves `malloc`, silently — the precise failure this project
exists to prevent.

### `could not find native static library 'stdc++'`

rustc does not search gcc's own version directory. The recipe must ask the
compiler (`$CXX -print-file-name=libstdc++.a`) and pass the directory through
`ALLOC_LINK_SEARCH`. snmalloc's recipe is the worked example.

### `multiple definition of 'operator delete(void*, unsigned long)'`

A C++ allocator's archive defines the operators and so does `libstdc++.a`.
snmalloc's recipe removes `new.cc.o`; jemalloc uses `--disable-cxx`;
hardened_malloc uses `CONFIG_CXX_ALLOCATOR=false`.

### `cc: error: x86_64: linker input file not found`

⭐ **`TARGET_ARCH` is a GNU make built-in.** The implicit C rule is
`$(CC) $(CFLAGS) $(CPPFLAGS) $(TARGET_ARCH) -c`, so exporting
`TARGET_ARCH=x86_64` puts the bare word on the compiler line. Pass
`TARGET_ARCH=` on the make command line, as hardened_malloc's recipe does.

### `meta.env: line N: <word>: not found`

A shell-hostile value reached a `.`-sourced file. `meta.env` carries only
single-token values; free-form flag text belongs in `build-flags.txt`, which is
never sourced.

### `#include "h_malloc.h"` not found, or `CONFIG_*` defines missing

A command-line `make CPPFLAGS=...` overrode the whole variable **and** made
every later `+=` a no-op. Put the define in a variable you are already
overriding (`SHARED_FLAGS`).

### `fatal: unable to access ...: self-signed certificate in certificate chain`

The network terminates TLS and the image does not trust its CA. Put the PEM in
`images/extra-ca/*.crt` and rebuild. ⛔ **Never disable verification.**

### `apk/apt/pacman: no such package` for *everything*

The package index could not be fetched — almost always the same TLS or proxy
problem as above, one layer earlier. Set `ALLOC_TESTS_HTTPS_PROXY` to a
**container-reachable** address; `127.0.0.1` inside a container is the
container.

### The build succeeds but the change had no effect

⚠ **The allocator cache.** Its key covers allocator, commit, mode, PIC, libc,
architecture, toolchain and variant — **not the recipe's contents**. After
editing a recipe:

```sh
rm -rf .cache/<distro>-<arch>/alloc
```

Two of this project's early diagnoses were spent on stale cache entries.

---

## `correctness_failed`

### Every count is off by exactly one, and `nomatch` finds one match

⭐ The corpus manifest is inside the searched tree. It records the patterns, so
it *contains* the needles. The data belongs in `<corpus>/data/`. Fixed here, but
the signature is worth recognising: a **uniform** off-by-one across every check
at once is the instrument measuring its own notes.

### `-j1` and `-j4` disagree

⛔ **A real bug, and the gate exists for it.** A data race in a thread-caching
allocator shows up as a wrong or unstable count long before it shows up as a
crash. Do not retry until it passes.

### `icase` equals the case-sensitive count

The expectations differ on purpose. Equal counts mean `-i` was ignored, or the
corpus lost its lowercase plantings.

---

## Empty results, or a run that produced nothing

### Every cell `build_failed` with a docker error mentioning a volume

⚠ **Bind-mount paths must be absolute.** A relative path is read as a *named
volume*, so cells write into a volume nobody reads. `alloc-bench` canonicalises
them; a hand-written `docker run` must not use a relative `-v`.

### `only N cell(s) produced a usable result; a ranking needs at least a control and one candidate`

⭐ The validator refusing to rank. This is correct behaviour, not a bug — the
report is still written and carries the failures. Fix the cells.

---

## Suspicious numbers

### A cell is flagged `noisy`

Its relative MAD is above 5%. The number is still real; it just cannot carry a
fine comparison. ⚠ **A difference smaller than that cell's own spread is not
attributable to the allocator.** More repetitions, a larger corpus profile, or a
quieter machine.

### Everything is within noise of everything

Likely the `smoke` corpus, where a search takes milliseconds. Use `standard` or
`large`.

### A result looks too good

Check `identity.json` first. A binary that is not what it claims is the fastest
way to a wrong table — and it is the failure mode that this project found in the
prior art.

---

## Getting more detail

```sh
alloc-bench validate --run results/local/<id>       # every finding, with severity
alloc-runner identify --bin ./rg --expect-allocator mimalloc --expect-kind static-pie
alloc-runner archive-check --archive /usr/lib/libc.a --symbol malloc --expect-providers 1
alloc-runner ar-members --archive /usr/lib/libc.a --symbols malloc,free
alloc-runner selftest                               # is the instrument itself sound?
```

Running one cell by hand, with the container's output on your terminal, is in
`docs/containers.md`.
