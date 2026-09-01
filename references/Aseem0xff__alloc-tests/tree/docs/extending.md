# Extending the project

Adding an allocator, an application, a distribution or a benchmark
configuration. Each section names the files that change and the check that
proves the addition works.

⭐ **The rule that governs all of them: an addition that cannot be built is
still added.** It gets a recorded technical reason and the report prints it. A
configuration that silently vanishes is indistinguishable from one nobody
thought of.

---

## Adding an allocator

### 1. Register it

`allocators/allocators.toml`:

```toml
[[allocator]]
id           = "myalloc"
name         = "myalloc"
repo         = "https://github.com/example/myalloc"
license      = "MIT"
track        = "latest-release"   # or "latest-any", or "branch"
branch        = "main"
build        = "cmake"
prefix       = "my_"
integrations = ["rust-global", "libc-surgery", "link-override", "preload"]
summary      = """
One paragraph: what it is, and anything about it that changes how it is built.
"""

# For any mechanism NOT in `integrations`, say why. This text is what the
# report prints, so write it for a reader who is wondering where the row went.
[allocator.unsupported_notes]
libc-surgery = """
Upstream builds no static archive; the only artefact is a shared object whose
initialisation runs at load time.
"""
```

⛔ **A mechanism omitted with no `unsupported_notes` entry is a CI failure.** The
`configuration` job rejects any unsupported cell without a reason.

### 2. Pin it

```sh
alloc-bench update --write
```

Adds the resolved commit to `allocators/allocators.lock.json`. Commit that diff:
it is what makes every future result reproducible.

### 3. Write the recipe

`allocators/myalloc/build.sh`. The contract is in `allocators/lib.sh`:

```sh
#!/bin/sh
# One paragraph on what is unusual about this allocator's build.
. "$(dirname "$0")/../lib.sh"

prepare_out
B="$OUT/.build"

# MODE is `prefixed` (exports my_malloc, defines no malloc) or `override`
# (defines malloc itself). PIC is 1 for static-PIE. LIBC is musl or glibc.
cmake -S "$SRC" -B "$B" -DCMAKE_BUILD_TYPE=Release \
    -DMYALLOC_PREFIX="$([ "$MODE" = override ] && echo '' || echo my_)" \
    -DCMAKE_POSITION_INDEPENDENT_CODE="$([ "$PIC" = 1 ] && echo ON || echo OFF)"
cmake --build "$B" --parallel "$NPROC"

cp "$(find "$B" -name 'libmyalloc*.a' | head -1)" "$ARCHIVE"
write_meta "" "cmake Release MODE=$MODE PIC=$PIC"

# `finish` asserts the archive really defines these, and that `prefixed` mode
# does NOT define malloc. That check is what stops a broken build from being
# measured as the system allocator under your allocator's name.
if [ "$MODE" = override ]; then
    finish malloc free realloc calloc
else
    finish my_malloc my_free my_realloc
fi
```

Exit **3** with `UNSUPPORTED: <reason>` where a configuration cannot be built —
for example a C++20 requirement the image's compiler does not meet. That is a
result and the report prints it.

⚠ Where the archive needs a C++ runtime, set `ALLOC_LINK_CXX` (e.g.
`static=stdc++`) and `ALLOC_LINK_SEARCH` before `write_meta`. rustc does not
search gcc's own version directory; ask the compiler with
`$CXX -print-file-name=libstdc++.a`.

### 4. Teach the shim, for `rust-global`

`crates/rgalloc-shim/Cargo.toml` — add a feature named exactly the allocator id.
`crates/rgalloc-shim/src/lib.rs` — add a `#[cfg(feature = "myalloc")] mod
backend` implementing `RawAlloc`, and add the feature to the `not(any(...))`
list on the `system` backend.

⛔ **Read the alignment rules at the top of that file first.** Over-aligned
allocations must not go to plain `malloc`; `dealloc` must use the same family
the pointer came from; and `realloc` on an over-aligned block is done by hand
unless the allocator has a genuine aligned realloc. Getting this wrong shows up
as *the allocator* crashing, and gets written down as that allocator's fault.

```sh
cargo test --release -p rgalloc-shim   # runs against the host allocator
```

### 5. Teach the identity oracle

`crates/alloc-runner/src/ident.rs`, `SIGNATURES`: symbols that prove *this*
allocator's implementation is linked in.

⚠ **Choose implementation internals, not compatibility aliases.** Several
allocators define `__libc_malloc` and friends as glibc-compat aliases — that is
why `__libc_malloc` is deliberately absent from the glibc signature here, after
it produced a false positive on a correct libc-surgery build.

### 6. Add it to a suite and run it

```sh
alloc-bench plan --suite core | grep myalloc
alloc-bench run  --suite core --arch x86_64 --allocator myalloc
```

The control is always kept when `--allocator` filters, because a table of
absolute times from one machine is not the deliverable.

---

## Adding a benchmark configuration

`benchmarks/matrix.toml`. Each suite must carry a `why` explaining the question
it answers:

```toml
[[suite]]
id  = "my-question"
why = """
The question this suite exists to answer, and what is held constant so that the
answer means something.
"""
distros      = ["alpine"]
arches       = ["x86_64", "aarch64"]
allocators   = ["*"]              # or an explicit list
integrations = ["rust-global"]
profiles     = ["static-pie-lto"]
toolchains   = ["distro"]         # or "zig"
corpus       = "standard"         # smoke | standard | large
repeat       = 12
```

⚠ **Vary one thing.** A suite that varies allocator *and* profile *and*
distribution produces a table whose rows cannot be compared with each other, and
`rank.rs` will split them into groups that each contain one row.

## Adding a workload

`crates/alloc-runner/src/main.rs`, `WORKLOADS`. Each carries a `why`:

```rust
Workload {
    name: "my-workload",
    why: "What allocation behaviour this exercises that the others do not.",
    args: &["--no-ignore", "--hidden", "--no-messages", "PATTERN", "{corpus}"],
    accept: &[0],
},
```

`{corpus}` and `{onefile}` are substituted. `accept` lists the exit codes that
mean success — the `nomatch` workload accepts `1` by design.

⚠ Adding a workload does not change the ranking: the primary workload is
`validate::PRIMARY_WORKLOAD`. New workloads appear in the "Every workload" table
so a reader can see whether the ordering holds across shapes of work.

## Adding a distribution

1. `images/<name>.Dockerfile`, following an existing one. It must install the
   pinned Rust and zig from `toolchains/pins.env`, build and **selftest** the
   instrument, and write `/opt/alloc-tests/image-env.txt`.
2. `crates/alloc-bench/src/run.rs`: `dockerfile_for` and `base_image`.
3. `crates/alloc-bench/src/plan.rs`: `libc_for`, and `effective_distro` if the
   distribution's upstream image does not cover both architectures.

⛔ **If upstream publishes no image for an architecture, rename the
distribution — do not substitute silently.** Arch is the worked example: on
aarch64 the image comes from Arch Linux ARM, a separate project with its own
package set, and the planner labels it `archlinuxarm` so no table can merge the
two under one name.

## Adding an application

⚠ **This is the largest addition and the framework does not fully generalise
yet.** ripgrep-specific logic lives in three places, and honest scoping matters
more than a plan that sounds tidy:

| what | where | how ripgrep-specific |
| --- | --- | --- |
| source preparation | `alloc-runner patch-rg` | **very** — it knows ripgrep's `crates/core/main.rs` and its `tikv-jemallocator` dependency |
| build | `scripts/build/build-ripgrep.sh` | mostly cargo-generic; the binary path is ripgrep's |
| correctness + workloads | `verify.rs`, `WORKLOADS` | **entirely** — the expectations are ripgrep's semantics over this corpus |

A second application needs its own equivalents of all three. The parts that
already generalise are the allocator recipes, the mechanisms, the measurement,
the validation, the ranking and the report — which is most of the machinery.

⭐ The design constraint to preserve: **the correctness gate must have an oracle
independent of the program under test.** For ripgrep that is a generated corpus
with planted, counted matches. Any new application needs the equivalent, or its
performance numbers cannot be trusted.

## Before you commit

```sh
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
cargo test --release -p rgalloc-shim
./target/release/alloc-runner selftest
./target/release/alloc-bench plan --suite all >/dev/null
alloc-bench run --suite smoke --arch x86_64 --strict
```

⚠ **Delete `.cache/<distro>-<arch>` after editing a recipe.** The cache key
covers the allocator, its commit, mode, PIC, libc, architecture, toolchain and
variant — **not the recipe's contents** — so a stale archive is otherwise
reused. Two of this project's early failures were exactly that.
