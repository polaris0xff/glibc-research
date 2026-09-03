# Design: pgb is a toolchain

⭐ **`pgb` is not a delivery format and must not be compared as one.** AppImage,
Flatpak, snap and onelf are formats: they answer *how does this reach a user's
machine*. `pgb` answers *how does a developer get from source to a binary that
runs*, and its output is deliberately not a format at all — it is an ordinary
ELF.

That difference decides what to build next, so it is worth stating precisely.

---

## The shape

```sh
pgb build https://github.com/owner/project      # a URL
pgb build nano                                  # or a name
```

and `pgb`:

1. **resolves the spec** — clones the repository, or finds the package across
   distribution archives and fetches its source;
2. **finds the build instructions** — the project's own `configure`,
   `CMakeLists.txt`, `meson.build`, `Makefile`, or the distribution's build
   recipe;
3. **works out the dependencies** — what this needs, transitively;
4. **links statically everything it can**, building dependencies from source
   into the same static image where no static library exists;
5. **bundles only what is left**, and says exactly what and why.

⛔ **The developer supplies none of that.** No dependency list, no `.desktop`
file, no icon, no AppDir layout, no runtime selection, no environment
variables. Those are the tool's job.

## What this is measured against

The comparison that matters is not "which artefact runs in more places" —
[`../comparison.md`](../comparison.md) shows several tie there. It is **what a
developer has to know and assemble**, and ⚠ **nothing measures that yet**:
**T-013** is the open entry that would, and the experiment it reserves a
number for has not been written.

⭐ **Credit where it is due**: the `Anylinux-AppImages` stack automates
dependency *bundling* extremely well. `quick-sharun` finds a program's whole
library closure — including things reached only by `dlopen` — and deploys the
libc, loader, gconv tree and NSS modules without being told to. That part is
solved, well, and this project should learn from it rather than repeat it.

⚠ **The friction is in assembling the toolchain, not in the per-application
work.** Producing one AppImage of a 40-line C program pulls in five separately
versioned upstream binaries — `sharun`, a forked `appimagetool`, `uruntime`,
`mkdwarfs`, and `cross-libc-dlopen` — across four upstreams, plus a
121 KB driver script, plus a `.desktop` file and an icon the developer must
author, plus around nine environment variables, on a build host that the
upstream guidance says should specifically be Arch Linux. Every one of those is
a thing to learn, pin, and keep working.

⭐ **`pgb`'s target is that the equivalent is `pgb build <spec>`**, with the
pinned build environment created once. That is the axis on which it should be
judged, and the axis it should be developed along.

## Static first, bundle last

The brief's preference order is explicit, and this is where it bites:

1. no application changes
2. automatic build/linker/toolchain changes
3. a generic wrapper/runtime technique
4. automatic application patching
5. application-specific patches
6. **a new packaging/runtime format — last**

⭐ **Every dependency gets pushed as far up that list as it will go**, and the
tool reports where each one landed. A dependency with a static library is
linked. One without gets built from source into a static library. One that
resists — a plugin discovered at run time, a driver that must come from the
host — is the only kind that reaches step 6.

⛔ **And if `pgb` ever bundles, the bundle has to earn its existence.** There
are two good bundling implementations already. A `pgb` bundle that is merely
another one of those is not worth building. It has to be better on a stated,
measured axis — and the honest candidates are:

| axis | what "better" would have to mean |
|---|---|
| **shape** | still a single ordinary ELF: no mount, no extraction, nothing written to the target filesystem, no shell in the delivery path |
| ~~**size**~~ | ~~only what static linking genuinely could not absorb, rather than the whole closure~~ — ⛔ **struck by the operator, 2026-09-03c; see the amendment below** |
| ⭐ **speed** | **faster than the field**, on the two things a user feels: how long the artefact takes to start, and how long the work inside it takes |
| ⭐ **friction** | **one command.** Not a multiline shell script, not a `.desktop` file, not an icon, not a set of environment variables — one command, with no format-specific input from the developer |
| **honesty** | the tool names every component it could not link statically, and why |

## ⭐ AMENDMENT — size struck, speed and one-command promoted, operator ruling, 2026-09-03c

⛔ **Two rows of the table above changed and the change is binding.** The
ruling is quoted verbatim because it names both halves of the new bar:

> *"us having a bigger size than anylinux-appimages and onelf is acceptable as
> long as ours performs better and packaging is just one command not a
> multiline shell script"*

⭐ **What that changes.** Size was one of four candidate axes and it was the
one `pgb`'s bundler was **losing** on — 2.86× on `jq`, 2.45× on kdenlive,
3.05× in `experiments/86-`. It is no longer an axis. In its place the ruling
names two conditions, and they are conjunctive: **perform better**, *and*
**package in one command**.

⚠ **This is a harder bar, not a softer one.** Under the old table a bundle
that was merely a single ELF passed. Under the ruling it has to be *faster
than the field*, and that is the column `pgb` is furthest behind on:

| what the ruling now decides | ours | the field | |
|---|---|---|---|
| `jq` cold start — ⭐ **11 environments, mean of 5** (`experiments/86-`) | 139 ms | 67 ms | ⛔ **2.07×** |
| `jq` warm start, same method | 14.9 ms | 10.8 ms | ⛔ **1.38×** |
| kdenlive cold start (`TODO/toolchain.md` T-066) | 300 ms | 61 ms | ⛔ 4.92× ⚠ **one sample** |
| kdenlive render | 4,947 ms | 2,033 ms | ⛔ 2.43× ⚠ **one sample** |
| ~~artefact size~~ | ~~2.86×–3.05×~~ | | ⭐ **no longer counted** |

⛔ **Trust the first two rows and treat the kdenlive pair as a direction, not a
number** — deep review 1, 2026-09-03c. `86-` takes eleven environments and a
mean of five per arm; `90-` takes **one sample**, its numbers come from a
superseded version of the cited evidence file, and four runs of it give
cold-start ratios of 2.52×, 3.48×, 4.92× and 5.02× with warm above cold in two
of them. ⭐ **Every run agrees on the direction**: we are slower.
[`../history/corrections.md`](../history/corrections.md) C23.

⭐ **And the second condition is already met, decisively.** `pgb bundle
appimage kdenlive` is one command from a package name. The competitor's route
is five separately-versioned binaries plus a 121 KB driver script, a
`.desktop` file, an icon and about nine environment variables — that is the
"multiline shell script" the ruling names, and it is measured, not asserted
(`references/pkgforge-dev__Anylinux-AppImages`, and `TODO/research.md` T-057).
So of the two conditions the bundler owes, **one is a win it can already
publish and the other is the whole of the remaining work.**

⛔ **The consequence for planning:** every size lever measured to date —
`--cut`, `--fixpoint`, the debloat rules, route A and route B of T-066 — is
now worth only what it buys in **startup and run time**, which is not nothing
(fewer objects is less to mount, map and relocate) but is also not what any of
them was measured on. ⚠ **None of the size work is invalidated; all of it is
now un-scored until someone re-measures it on the clock.**

⚠ **If a design cannot beat `sharun` + `uruntime` on shape, speed, friction or
honesty, the right answer is to emit an anylinux AppImage and say so**, not to
ship a fifth mediocre bundler. That is a real option and it should stay on the
table.

## Language and structure

⭐ **`pgb` is one statically linked Go binary and nothing else.** The driver,
the compiler wrappers it puts on `PATH`, the nixpkgs planner, the verifier and
the bundler are all the same executable. It is built with `CGO_ENABLED=0`, so
it links no libc of its own, and it carries the C runtime sources it compiles
into a build inside itself. Nothing has to be cloned beside it and no
interpreter has to exist anywhere it runs.

```sh
CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -trimpath -o pgb ./cmd/pgb
```

### The constraint that used to decide this, and the measurement that settled it

⛔ **"The driver must be `sh` because `sh` is the only thing that exists in a
target rootfs" is measured false.** `experiments/70-carried-helper.sh` builds
the same helper — open a file on whatever filesystem it landed on, parse it,
report — several ways and **carries** each into the eleven pinned target
rootfs plus the pinned build environment. `sh`, plain `gcc -static`, C built
by `pgb`, Rust `+crt-static` and Rust musl all run on **12 of 12**. Nothing is
installed anywhere; the binary is copied in and executed.

⚠ **The constraint was never "does the language exist there", it was "can
something be *put* there"** — and putting a static binary somewhere and having
it run is this project's entire output. `evidence/70-carried-helper/RESULT.txt`
is the table.

⚠ **What experiment 70 does not show.** It measures whether a helper
**executes**, not whether it is correct on every libc path — which is why the
plain `gcc -static` arm scores 12 of 12 where `ci/probe.c` fails on 11 of 11.
A helper that resolved a hostname or converted an encoding would need `pgb`'s
mechanisms like anything else. Availability, and nothing more.

### Why Go

The work is orchestration — `git`, `curl`, `tar`, `chroot`, `gcc`, `ld` — over
data structures: dependency graphs, ELF tables, package metadata, a ~400 MB
JSON index, NAR streams and Ed25519 signatures. Shell is good at the first
half and bad at the second; Python was carrying the second half at the cost of
an interpreter dependency in the product.

| rank | language | port speed | runtime | single-binary delivery | fit |
|---:|---|---|---|---|---|
| **1** | **Go** | **excellent** | very good | **excellent**, `CGO_ENABLED=0` | **chosen** |
| 2 | Rust | fair | excellent | excellent, musl target | best for maximum rigour |
| 3 | Nim | excellent from Python | very good | needs validation | fastest-looking, highest delivery risk |

⭐ **Go wins on the combination, not on any single axis.** Imperative shell and
Python translate to it almost line for line; `os/exec` gives literal argv,
environment and pipes with no word splitting to re-derive; and the standard
library already contains what this toolchain needs — `debug/elf`,
`archive/tar`, `encoding/json`, `crypto/ed25519`, `crypto/sha256`, `net/http`,
the compression primitives, and `embed` for the carried assets. Goroutines
replace `bootstrap`'s hand-rolled parallel jobs.

⚠ **What it costs.** A garbage-collected runtime makes a larger baseline
binary than C or Rust and gives less deterministic memory behaviour — which
does not matter for a CLI dominated by subprocesses and streaming I/O. Mount
namespaces, `ptrace` and process groups need deliberate Linux-specific code
rather than borrowed shell. And Go refuses to guess: every pipeline that
genuinely wanted shell semantics now says `sh -c` out loud.

⛔ **Rust is the better answer to a different question.** If the priority order
were correctness and low-level performance first and port speed second — a
high-assurance NAR/ELF parser, or a future in-process loader — Rust wins, and
it is the one compiled candidate this repository had already measured. It
loses here on translation cost across ~9,000 lines of orchestration. Nim is a
reasonable spike and not a foundation: its static cross-libc delivery is
untested by this project and it is likelier to pull in C libraries, which is
the one thing the port exists to remove. Zig links statically beautifully and
reuses the C sources directly, but it is furthest from the dominant
constraint, which was translating a large body of imperative code quickly.

### What "one binary" can and cannot mean

`pgb` replaces the checked-in shell and Python runtime and embeds the C
runtime sources, the rootfs manifest and its fixtures. ⛔ **It does not
replace the build toolchain, and was never going to.** `pgb` still orchestrates
GCC, binutils, make/CMake/Meson/autotools, git and HTTP sources, and a
chroot or container engine. The claim is **one distributable controller**, not
a compiler and a container stack packed into an executable.

⚠ **The C runtime pieces stay C.** They are the measured mechanisms — NSS
override, the iconv wrap, the embedded locale, the dlopen table, the tracer —
and they are carried as byte-identical embedded sources, compiled and cached
exactly as before. Rewriting them is a separate project and is not part of
removing shell and Python from the product.

### The structure

```text
pgb
├── CLI dispatch: doctor/env/build/verify/nix/bundle/elf/rootfs/selftest
├── hidden re-entry: __inner-build, __inner-shell, __rootfs-inner
├── compiler-wrapper mode, selected by argv[0]
├── embedded assets: the runtime C and headers, the rootfs manifest
└── internal/
    ├── proc      exact argv, environment, pipes, status and signals
    ├── cfg       configuration, the engine boundary, paths
    ├── logx      levels, per-subsystem debug, timestamped output
    ├── fail      the exit-code contract: 0 ok, 1 ran and failed, 2 could not
    ├── wrapper   compile/link classification, flag injection, the runtime objects
    ├── buildx    build, shell, the re-entry inside the environment
    ├── envx      environment creation and its stamp; the libiconv build
    ├── rootfs    the private mount namespace, mounts and chroot
    ├── ociimg    the registry client and the whiteout merge
    ├── nixx      drv, plan, index, NAR, signatures, fetch
    ├── elfx      DT_NEEDED reading and rewriting, symbol tables
    ├── verifyx   static inspection, the tracer, the matrix run
    └── bundle    the reachability sweep and the AppImage assembly
```

⭐ **The wrappers are the binary under another name.** `cc`, `gcc`, `c++`,
`g++` and `cpp` are symlinks to `pgb`, which dispatches on `argv[0]` and reads
one JSON manifest written when the wrapper directory was created. A build
system that calls the compiler ten thousand times starts no shell at all.

⛔ **The namespace work is native.** `pgb rootfs run` re-execs itself with
`CLONE_NEWNS` and does its own mounts and `chroot`, so there is no `unshare`,
no `mount` and no `sh` in the path into a target. It keeps the `chroot`
convention on the way out: **127** when the command does not exist, **126**
when it exists and cannot be executed.

### How the port was accepted

⛔ **"The new language is faster" was never the bar.** `pgb build` spends its
wall time in downloads, `configure`, GCC and linking; replacing the controller
does not make GCC compile Qt faster. The port was accepted against workload
gates, each measured against the retired implementation rather than against a
claim, and recorded in `evidence/92-go-port/RESULT.txt`:

| gate | what it compares |
|---|---|
| 1 | `nix index`: identical output from the real ~400 MB nixpkgs input, with wall time and peak RSS |
| 2 | `nix nar`: identical archives, hashes, extraction **and signature decisions**, on fixtures and on a real `cache.nixos.org` object |
| 3 | `doctor`, `env info`, `nix plan`, `verify`: output and exit-code parity |
| 4 | the wrapper hot path: the same source through both toolchains, byte for byte |
| 5 | the complete 11-target matrix and all ten POCs |
| 6 | the artefact itself: statically linked, no `PT_INTERP`, no `DT_NEEDED` |

⚠ **The retired shell and Python tree is the oracle, and it is kept.** It sits
under `HISTORY/<commit>/` unedited, so any gate can be re-measured rather than
re-argued. `HISTORY/README.md` says what it is.

⭐ **The experiments and the POCs stay in shell.** They are the independent
acceptance harness: rewriting them in the same language as the subject would
remove the thing that measured the subject. They call `pgb` and nothing else.
