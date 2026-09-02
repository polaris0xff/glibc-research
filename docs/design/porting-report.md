# Preliminary porting analysis: `glibc-research` / `pgb`

Date: 2026-09-02  
Repository: `https://github.com/polaris0xff/glibc-research`  
Analysed commit: `2e4c616933fa9379a4ba2eec0868893fb40f1d2f`

## Executive recommendation

**Use Go for a full tooling port.** It is the best balance of:

1. fast translation from the existing imperative POSIX shell and Python;
2. easy distribution as one libc-independent Linux executable;
3. enough native performance to remove Python parsing/crypto overhead and many shell process launches;
4. a standard library that directly covers the repository's core needs: process execution, streaming I/O, JSON, HTTP, tar, hashing, Ed25519, ELF inspection, and embedded assets.

**Rust is the strongest second choice** if maximum parser correctness, memory control, and long-term systems-level work outweigh porting speed. It is also the only compiled candidate already tested by this repository: static GNU and musl Rust helpers ran on all 12 tested environments.

**Nim is third** because its Python-like syntax makes mechanical translation attractive and it produces fast native binaries, but it has weaker project-specific evidence, a smaller ecosystem, and a less turnkey static/musl deployment story than Go or Rust.

This recommendation deliberately differs from the repository's current “shell driver plus compiled planner” decision. The repository kept shell mainly because orchestration is readable there, not because compiled delivery is impossible; experiment 70 disproved that bootstrap objection.

## What was cloned and inspected

The repository is a research-backed toolchain for producing ordinary static-glibc ELF binaries that run across glibc and musl distributions. The current tool consists of:

- a POSIX-shell command driver (`pgb`);
- sourced shell modules for environments, compiler wrappers, builds, verification, and Nix planning;
- Python helpers for Nix derivations/plans/indexes/NARs, ELF rewriting, wrapper decoding, and recipe generation;
- small C runtime/tracer components compiled into or alongside generated output;
- a large shell-based experiment and POC suite.

The project reports 11/11 target coverage for its supported class and nine POCs. The remaining known product limitation is loading host plugins.

### Porting surface

Production/distribution-oriented tooling:

| Language | Files | Lines | Bytes |
|---|---:|---:|---:|
| POSIX shell, including extensionless `pgb` | 15 | 6,428 | 307,232 |
| Python | 7 | 2,554 | 96,292 |
| **Total** | **22** | **8,982** | **403,524** |

Literal “all shell and Python” scope, excluding vendored references and committed evidence:

| Scope | Files | Lines |
|---|---:|---:|
| Product/tool/bootstrap code | 22 | 8,982 |
| Experiments, POCs, and TODO consistency checker | 37 | 10,359 |
| **Total** | **59** | **19,341** |

The second number is a poor first milestone. Experiments and POCs are the behavioral oracle for the port; rewriting them at the same time would remove the independent test harness. Keep them in shell until the product binary has parity.

## What “single binary” can and cannot mean

A single `pgb` executable can replace the checked-in shell/Python runtime and embed:

- C runtime/tracer sources;
- wrapper templates;
- the rootfs manifest;
- Nix fixtures and other small data files.

It cannot remove the intentional external build toolchain. `pgb` still orchestrates GCC/G++, binutils, make/CMake/Meson/autotools, Git/HTTP sources, and a chroot/container engine. The realistic goal is therefore **one distributable controller binary**, not an entire compiler and container stack packed into one executable.

For the fastest safe port, preserve the C runtime pieces as embedded, byte-identical source assets and compile/cache them exactly as today. Rewriting those measured mechanisms is a separate project and is not required to eliminate shell/Python as distribution dependencies.

## Ranked language comparison

| Rank | Language | Port speed | Runtime performance | Single-binary delivery | Fit for this repository |
|---:|---|---|---|---|---|
| **1** | **Go** | **Excellent** | Very good | **Excellent** with pure Go / `CGO_ENABLED=0` | **Best overall** |
| **2** | **Rust** | Fair | **Excellent** | **Excellent**, especially a musl target | Best for maximum rigor/performance |
| **3** | **Nim** | **Excellent for Python**, good for shell | Very good | Good, but needs more build/link validation | Fastest-looking translation, highest delivery risk |

### 1. Go — recommended

Benefits:

- Shell orchestration maps cleanly to `os/exec`, explicit argv arrays, environment maps, pipes, contexts, and process lifecycle handling.
- Python dictionaries/lists, JSON transforms, binary readers, and streaming loops map directly without ownership/lifetime friction.
- Pure-Go binaries are straightforward to cross-compile and avoid a target libc. Go's own documentation says the standard `gc` linker creates statically linked binaries by default; building this project with cgo disabled avoids accidentally acquiring C-library dependencies.
- `embed` can place the C sources, manifests, fixtures, and templates inside the executable.
- The standard library already contains unusually relevant packages: `debug/elf`, `archive/tar`, `encoding/json`, `crypto/ed25519`, `crypto/sha256`, `net/http`, and compression primitives.
- Goroutines are a natural replacement for `bootstrap.sh`'s explicitly parallel independent jobs.
- Build times and contributor onboarding are generally much lighter than Rust for an orchestration-heavy CLI.

Drawbacks:

- Go's garbage collector and runtime produce a larger baseline binary than C/Nim and give less deterministic memory behavior than Rust. This is unlikely to matter for a CLI dominated by subprocesses and streaming I/O.
- Low-level mount namespace, ptrace, process-group, and signal behavior needs careful Linux-specific code, probably through `golang.org/x/sys/unix`, or continued execution of `unshare`, `mount`, and `chroot` during the first port.
- Directly linking the existing C runtime into the controller with cgo would weaken the clean static deployment story. Embed/materialize the C assets instead.
- Go process execution deliberately does not apply shell word splitting, globbing, or substitutions. That is safer, but every existing pipeline must be translated consciously; places that truly require shell semantics should invoke `/bin/sh -c` explicitly.
- The repository has not yet run a static Go helper through its 12-target matrix. Add that arm before committing to the full rewrite.

Recommended release build shape:

```sh
CGO_ENABLED=0 GOOS=linux GOARCH=amd64 GOAMD64=v1 \
  go build -trimpath -ldflags='-s -w' -o pgb ./cmd/pgb
```

Do not treat `-s -w` as mandatory during development; retain symbols in debug and CI artifacts.

### 2. Rust — strongest technical alternative

Benefits:

- Best fit for trusted ELF/NAR parsers, archive extraction, cryptographic verification, and future in-process loader work.
- Strong types and ownership help prevent lifetime, aliasing, and accidental whole-buffer copies in the 400 MB JSON index and NAR streams.
- Predictable memory use and excellent native performance.
- `std::process::Command` provides literal argv and environment control; `include_bytes!` embeds assets.
- The repository already proved both static GNU (`+crt-static`) and musl Rust helpers on 12/12 environments. This project-specific evidence is stronger than generic portability claims.
- Mature crates exist for CLI parsing, ELF, compression, temporary files, error context, and serialization.

Drawbacks:

- It conflicts most strongly with “incredibly fast to port.” Shell pipeline translation, recursive archive code, heterogeneous JSON, path bytes, and error propagation will take materially longer than in Go or Nim.
- `OsStr`/byte-path correctness, ownership, and streaming lifetimes add up-front design work to code that is currently very dynamic.
- Slower clean builds and a larger dependency graph if common ecosystem crates are used.
- Static linkage is target-specific; Rust's own reference warns that unsupported targets may ignore CRT linkage flags, so resulting ELF files must be inspected in CI.

Choose Rust instead of Go only if the priority order is changed to **correctness and maximum low-level performance first, port speed second**.

### 3. Nim — rapid translation candidate

Benefits:

- Python-like indentation, sequences, tables, exceptions, iterators, and concise syntax make the Python helpers quick to translate.
- Produces native executables without a VM and offers configurable memory management.
- Its C backend and `{.compile.}`/link pragmas can integrate the existing C runtime code with little FFI ceremony.
- Expected performance is comfortably above CPython and shell for parsing and orchestration.

Drawbacks:

- Static, cross-libc delivery is less turnkey than pure Go or Rust's established musl targets and has not been tested by this repository.
- Smaller package ecosystem and contributor pool, especially for Nix/NAR, ELF, OCI, zstd, and Linux namespace/ptrace work.
- More likely to depend on C libraries for crypto/compression/network features, complicating the promised one portable binary.
- Generated C adds another debugging layer, and long-term maintenance/toolchain stability is a greater project risk.
- It is fast to write, but the extra portability validation can erase that initial advantage.

Nim is a reasonable spike, not the recommended foundation.

### Why Zig is not in the top three

Zig is excellent for static linking and direct reuse of the repository's C code. It loses on the user's dominant constraint: translating roughly 9,000 product lines of process orchestration, text manipulation, JSON, HTTP, archive, and crypto code quickly. Its standard-library/ecosystem fit for this particular workload is weaker than Go's, and it offers less mechanical similarity to Python than Nim. It is a strong choice for a future in-process ELF loader, not the fastest full-tooling port.

## Performance assessment

The performance case must be stated narrowly.

Likely meaningful wins:

- no Python interpreter requirement or startup for seven helpers;
- fewer `awk`/`sed`/`grep`/`head`/`cut` subprocesses and pipelines;
- faster streaming parse of the roughly 400 MB nixpkgs package index;
- native NAR serialization/extraction, hashing, and Ed25519 verification instead of the pure-Python fallback;
- cheaper graph traversal and plan construction;
- easier bounded concurrency for downloads, index work, and independent bootstrap steps.

Likely small end-to-end wins:

- `pgb build` spends most of its wall time in downloads, decompression, configure/CMake, GCC/Clang, linking, and test matrices;
- environment creation is recorded in minutes, and large POCs take tens of minutes;
- replacing the controller language will not make GCC compile Qt or ffmpeg faster.

Therefore, “the new language is faster” is not enough. Accept the port only if it passes workload-level gates such as:

1. `nix-index`: identical output from the real ~400 MB input, with wall time and peak RSS recorded;
2. `nix-nar`: identical hashes/extraction and signature decisions on fixtures plus real cache objects;
3. `pgb doctor`, `env info`, `nix plan`, and `verify`: output/exit-code parity;
4. wrapper hot path: compile/link behavior parity with no meaningful per-invocation regression;
5. complete 11-target matrix and all nine POCs;
6. controller startup and artifact size recorded, but not used as substitutes for the workload tests above.

## Recommended Go architecture

Use one multi-call executable:

```text
pgb
├── normal CLI dispatch: doctor/env/build/verify/nix/...
├── hidden re-entry: __inner-build, __inner-shell
├── compiler-wrapper mode selected by argv[0] or hidden subcommand
├── embedded assets: runtime C/H, manifests, fixtures, templates
└── internal packages
    ├── process       exact argv/env/pipes/status/signal behavior
    ├── engine        host/chroot/docker/podman boundaries
    ├── wrapper       compile/link classification and flag injection
    ├── nix           drv, plan, index, NAR, signatures, fetch
    ├── elf           DT_NEEDED and inspection
    ├── verify        tracer parsing and matrix execution
    └── bundle        current nix-appimage behavior
```

Compiler wrappers should eventually be symlinks/copies of the same binary, dispatching by `argv[0]`, rather than generated shell scripts. That preserves one-file distribution and eliminates a shell from the hot wrapper path. During the first parity phase, embedded wrapper templates are lower risk.

## Migration sequence

Avoid a big-bang rewrite.

1. **Freeze behavior and add performance baselines on Linux.** Run all existing self-tests, capture golden CLI outputs/exit codes, and benchmark the real index/NAR workloads.
2. **Add Go to experiment 70.** Prove a `CGO_ENABLED=0`, `GOAMD64=v1` helper on the same 12 targets where Rust passed.
3. **Build a vertical Go spike.** Implement `pgb doctor`, asset embedding, one hidden re-entry, and one compiler-wrapper path. This tests distribution, argv/env fidelity, and engine-boundary behavior early.
4. **Port Python helpers behind unchanged CLIs.** Start with `nix-drv.py`, `nix-plan.py`, `elf-needed.py`, `nix-wrapper.py`, and `onelf-recipe.py`; then port the streaming `nix-index.py` and security-sensitive `nix-nar.py` with golden/self-test parity and benchmarks.
5. **Port the `pgb` shell modules command by command.** Keep the old implementation selectable in CI until each command passes differential tests.
6. **Port `scripts/common` and `nix-appimage.sh`.** Preserve cleanup, atomic replacement, whiteout, mount namespace, and signature/hash behavior exactly.
7. **Convert compiler wrappers to multi-call binary mode.** Do this after link/compile classification parity exists.
8. **Keep experiments and POCs in shell.** They remain the independent acceptance harness. Port them only later if “zero shell files in the repository” is a separate, explicit goal.
9. **Remove Python/shell runtime checks from the product only after matrix parity.** Run the full 11-environment verification and nine POCs, inspect the produced ELF, and compare performance gates.

## Final verdict

**Go is the best language for this requested full port.** It is fast enough to justify replacing Python and shell in the code paths where the language matters, and it is much faster to implement than Rust for this orchestration-heavy repository. Rust remains the better choice for a narrower high-assurance planner/parser or a future in-process ELF loader, but it is not the best answer to the combined requirement of single-binary delivery, very rapid migration, and better performance.

The port's strongest justification is **distribution reliability plus native helper performance**, not a promise that whole-project builds will become dramatically faster. The existing test suite is good enough to support an incremental Go rewrite without sacrificing the evidence that makes this repository valuable.

## Sources

Repository evidence and design:

- `docs/design/toolchain.md` — current language/structure decision and experiment-70 correction.
- `TODO/toolchain.md`, T-011 and T-056 — language decision and existing Rust-helper port item.
- `evidence/70-carried-helper/RESULT.txt` — static Rust GNU and musl helpers passed 12/12.
- `internal/nixx/index.go` — the ~400 MB streaming JSON workload.
- `internal/nixx/nar.go` — NAR, hashing, compression, and Ed25519 verification.

Official language documentation:

- Go FAQ: https://go.dev/doc/faq#Why_is_my_trivial_program_such_a_large_binary
- Go `embed`: https://pkg.go.dev/embed
- Go `os/exec`: https://pkg.go.dev/os/exec
- Go `cgo`: https://pkg.go.dev/cmd/cgo
- Rust linkage and static CRT: https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes
- Rust `Command`: https://doc.rust-lang.org/std/process/struct.Command.html
- Rust `include_bytes!`: https://doc.rust-lang.org/std/macro.include_bytes.html
- Nim language overview: https://nim-lang.org/
- Nim backend/C integration: https://nim-lang.org/docs/backends.html
