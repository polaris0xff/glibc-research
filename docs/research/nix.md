# The nix sweep: using nixpkgs as the planner, and what it costs

Ten references, mined and kept. The write-up opens with what it did **not**
establish, because a reader skimming for the answer will not reach an appendix.

---

## ⛔ What this did NOT establish

| | |
|---|---|
| **never tested** | aarch64, and any machine but this one. Every number below is x86_64, one kernel (6.18.44), one day (2026-09-01). |
| **never tested** | a machine with a GPU. The OpenGL question — `TODO` T-052 — is stated here and **not measured anywhere**, and a video editor is exactly where it decides the outcome. |
| **never tested** | the bundler against a hand-built `Anylinux-AppImages` artefact. `experiments/62-` compares `pgb` against one; nothing yet compares **our bundle** against one. |
| **not fetched** | discussions, for every one of the ten references. GraphQL only, no credential-free route; each `PROVENANCE.md` records it. |
| **read at README/tracker depth only** | `yasunori0418/nput`, `grigio/docker-nixuser`, `NixOS/patchelf`, `nix-community/patsh`, `pkgforge-dev/kdenlive-AppImage-Enhanced`. Named here rather than implied. |

⭐ **How many claims a previous revision got wrong: this is revision 1, and it
already carries two corrections of its own.** Both are in the "known-weak"
section below, and **assume more remain**.

## Route the reader

| you have | read |
|---|---|
| two minutes | "The three findings", below |
| ten minutes | that, then "Known-weak claims" |
| the implementation to do | "The mechanisms", then `internal/nixx/build.go` |
| a reason to distrust this | `experiments/80-` and `83-`, then the selftests |

---

## The three findings

### 1. ⛔ nixpkgs' `pkgsStatic` is MUSL, so nixpkgs has no glibc-static answer

Measured, from nix itself:

```
pkgsStatic.stdenv.hostPlatform.libc  =  "musl"
stdenv.hostPlatform.libc             =  "glibc"
```

and the artefact `nix-build '<nixpkgs>' -A pkgsStatic.bash` produces is named
`bash-interactive-static-x86_64-unknown-linux-musl-5.3p15`.

⭐ **This reframes the whole relationship.** The operator pointed at
`pkgforge/soarpkgs`' `binaries/bash/static.nixpkgs.stable.yaml` (pinned commit
`55c774a5e24d9f17af69911a4d70884dfb566626`) as the reference recipe, and its
entire build step is that one `nix-build`. So nixpkgs and `pgb` are not two
answers to one question: **nixpkgs plans and fetches; `pgb` is the glibc half
nixpkgs does not have**, and `experiments/61-` already measured what the
difference costs (glibc 4.53 ns/op against musl 584.71 on a 4-thread allocator
workload).

### 2. ⭐ A nixpkgs package can be resolved, planned AND fetched with no nix

Three plain HTTPS endpoints, every hop signed and hash-checked:

```
channels.nixos.org/<channel>/store-paths.xz   302 -> a URL that NAMES THE
                                              REVISION, so one redirect is a
                                              pin; the body is every store
                                              path the channel built
                                              (308,271 for nixpkgs-unstable)
cache.nixos.org/<hash>.narinfo                signed metadata, and its
                                              `Deriver:` names the .drv
cache.nixos.org/<the narinfo's URL>           the NAR, or the .drv itself
```

⛔ **And the `.drv` is a store path like any other**, so the derivation graph —
source, patches, configure flags, dependency list — is reachable the same way.
`experiments/83-` shows the ATerm reader and `nix derivation show` agreeing on
gawk **field by field**, and a plan built with nix removed from `PATH`.

⚠ **With a limit that is not optional to state.** Deriver `.drv`
availability, by population:

| population | sampled | has .drv | |
|---|---|---|---|
| any path in the channel index, by stride | 60 | 2 | **3%** |
| paths with no output suffix | 60 | 1 | **1%** |
| twenty packages a person would name | 19 | 9 | **47%** |

⭐ And the packages that have one are not the *popular* ones, they are the ones
that are **inputs to other builds**: zlib, gawk, gnugrep and coreutils do; jq,
nano, htop, sqlite, ncurses and less do not.

## ⭐ REVISION 2: THE FALLBACK IS NOT AN EVALUATOR, AND THE CEILING IS GONE

⛔ **Superseded, 2026-09-01e.** The sentence that stood here said the fallback
to evaluation was mandatory. It is not, and `experiments/88-` measures the
replacement: **hydra built the channel**, so it holds the derivation for every
job it ran.

    hydra.nixos.org/job/<project>/<jobset>/<attr>.<system>/latest-finished
      -> drvpath, system, and every output's store path

That is an index of **builds**, not a field somebody happened to upload beside
a NAR, so `Deriver:` availability does not bound it. On **83-'s own twenty
packages with 83-'s own predicate**:

| route | resolved | not |
|---|---|---|
| ⭐ hydra | **19** | 1 |
| narinfo `Deriver:` | 9 | 11 |

and the one miss is a name that is not a nixpkgs attribute (`grep`; the
attribute is `gnugrep`). ⭐ **The control**: for jq, gawk, zlib and openssl the
drvpath hydra returns is **byte-identical** to what a local `nix-instantiate`
computes, and the two routes' plans agree on **19 of 19** comparable fields.

⚠ **It pins differently and that is stated rather than hidden.** hydra answers
for its latest FINISHED eval; the channel is an older tested revision. The
`drv` subcommand reports the revision and cross-checks the outputs against the
channel index, and `ChannelPinAgrees: no` is usual. It matters for fetching a
prebuilt binary and not for planning, because a plan is source URLs and
configure flags and sources are fixed-output paths that do not move.

### ⭐ And a second index answers finding 3b

`releases.nixos.org/nixpkgs/<pin>/packages.json.br` sits beside
`store-paths.xz`, is served with `Content-Encoding: br` so `curl --compressed`
decodes it with **no brotli library on the host**, and carries per attribute
the derivation `name` (**`bash` → `bash-interactive-5.3p15`**, which 3b says no
name match can know), the default `outputName` (**`jq` → `bin`**), the `pname`,
and the **`system`**.

### ⛔ THE DEFECT BOTH INDEXES FIX, WHICH HAD BEEN THERE FROM THE START

**`store-paths.xz` is every system the channel built.** Resolving `nix-2.35.2`
by name in this tree returned an **aarch64-darwin** build — fetched, signature
verified, NarHash checked, and a **Mach-O arm64 executable**. Nothing in the
route could tell. What gave it away was its closure: 57 paths **with no glibc
in it**, and two Apple-only libraries. The index holds **three** store paths
named `nix-2.35.2`.

⚠ **A `pname`-only match is reported as one**: `sed` reaches **`freebsd.sed`**,
a real package for the wrong userland, and the answer says `Matched: pname`.

⛔ **An index lookup is not an evaluation, and `experiments/83-` arm 3b shows
it two ways.** nixpkgs' `bash` attribute is `bash-interactive`, which no name
match can know; and the channel index is one snapshot while a local nix is
another, so the same package resolves to two store paths. An override, an
overlay or `pkgsStatic.*` is out of reach by the index route entirely.

### 3. ⭐ A nixpkgs binary is location-locked, and that is the whole reason bundlers exist

`experiments/80-`, arm 5: a fetched `bash` has

```
PT_INTERP = /nix/store/n51dhmdbik1kfrsm62j5knavmigwrl1a-glibc-2.42-84/lib/ld-linux-x86-64.so.2
```

In a rootfs with no `/nix` it exits **127**, with a positive control on the
same runner so the refusal is the binary's and not the harness's. Handed the
loader fetched beside it, the same binary runs.

⛔ That is why `nix bundle`'s AppImage ships a store and a `bwrap` AppRun to
bind-mount it — and `ralismark/nix-appimage` issue #10 has the maintainer
saying the container is *forced* by the absolute-path store.

---

## The mechanisms, at file and line

### `simonfxr/nix-download` — **adopt**, and it is now reimplemented

Commit `095cc446e7bf9fe1ccc9147599a9c7256684d1a2`, MIT, Go, 450 lines. It is the
complete binary-cache protocol and it reads as a specification:

| what | where |
|---|---|
| narinfo fetch and field parse | `main.go:164-244` |
| ed25519 verification, and the **fingerprint the signature covers** | `main.go:246-288` — `1;<StorePath>;<NarHash>;<NarSize>;<refs joined by comma, each /nix/store/-prefixed>` |
| nix-base32, **not RFC 4648**, bits consumed from the END | `main.go:427-450` |
| the NAR grammar | `narextract/narextract.go`, 265 lines |

⭐ **Reimplemented rather than carried**, in `internal/nixx/nar.go`, because
this project's tool is POSIX sh and C and a Go binary is a bigger dependency
than 400 lines. 28-check selftest, RFC 8032 vectors, and two real narinfo
bodies committed as fixtures under `scripts/common/fixtures/nix/`.

⚠ **A dead branch in the original, found by reading it:**
`narextract.go:150` tests `err == io.EOF`, but `readString` wraps every error
through `fmt.Errorf("...: %w", err)`, so that comparison can never be true. It
is harmless — a well-formed NAR never reaches EOF mid-stream — and it is the
kind of thing only reading finds.

### `pkgforge-dev/Anylinux-AppImages` + `Anylinux-sharun` — **adopt**

Commit `da7649b9443971ef70da92f532e8a2e65a9f97f6`. `HOW-TO-MAKE-THESE.md` is
the argument; `useful-tools/quick-sharun.sh` (3,956 lines) is the
implementation. What transfers:

| mechanism | why |
|---|---|
| **run the bundled loader, hand it `--library-path`** | not `LD_LIBRARY_PATH`, which children inherit; the how-to cites a zen-browser bug that took months to find because of it |
| **sharun hardlinked per program** | `/proc/self/exe` names the application instead of `ld-linux`, which the plain `ld.so` AppRun cannot fix |
| **uruntime + dwarfs**, not type-2 + squashfs | and no `bwrap`, so no unprivileged user namespace — which the how-to itself calls a thing "you cannot even rely on" |
| the layout `lib/`, `bin/`, `shared/bin/`, `shared/lib -> ../lib` | `quick-sharun.sh:44-46` and `:464`. ⛔ It differs from upstream sharun, and building it upstream's way gets `Interpreter not found!` |

⛔ **What this project does that neither upstream does:** sharun finds
libraries with `ldd` plus `LD_DEBUG=libs`/strace. **A nixpkgs closure is not a
heuristic** — it is the exact set the derivation declared, `dlopen`'d libraries
included. So the closure replaces the discovery step.

### `nix-community/nix-ld` — **adopt, for T-033 rather than for the bundler**

Commit `2cced31ac171b55dd6ab8cc04502cb1ad012d7cf`, MIT, 2,023 lines of
`#![no_std] #![no_main]` Rust. ⚠ It is not what it first looks like: it does
**not** merely `execve` the real loader. `src/elf.rs:139-280` **maps
`PT_LOAD` segments itself**, computes the load bias, anonymous-maps the BSS,
and `jump_with_sp` enters it; `src/fixup.rs` self-relocates before any libc
exists; `src/main.rs:223-249` rewrites `AT_ENTRY` to a trampoline so its own
`LD_LIBRARY_PATH` edit can be reverted before the program starts.

⭐ **That is 407 lines of mapping logic against solo's 2,707**, aimed at the
same place `TODO` T-033 route D needs a mapper. It maps one object — the
loader — and does no symbol binding, so it is a floor, not a solution.

### `nix-community/nix-user-chroot` — **confirms, with a caveat that matters**

Commit `987302aef4e3aa267355cfad00027b730bcb389b`. Runs nix as an ordinary user
in a user namespace. ⛔ **Its own README says Ubuntu 23.10+ gates unprivileged
user namespaces behind AppArmor and RHEL/CentOS 7 ship them disabled**, so it
is not a universal answer to "no root" either — which is why `TODO` T-051
puts it third behind pushing the no-nix route further.

### `pkgforge/soarpkgs` — **adopt: the two recipe shapes**

Pinned at the operator's `55c774a5e24d9f17af69911a4d70884dfb566626`, fetched
with a new `mine-repo.sh --ref` because a pin was the whole point and the
script had no way to express one.

- `binaries/bash/static.nixpkgs.stable.yaml` — the static shape, and its build
  is one `nix-build -A pkgsStatic.bash`, i.e. finding 1.
- `packages/chromium/nixappimage.nixpkgs.stable.yaml` — the bundle shape, and
  ⭐ **it answers the operator's open question about desktop files**: the
  `.desktop` and the icon are **found in the closure** with `find`
  (`:99-120`), never patched in. `internal/bundle/appimage.go` does the same.
  ⚠ Its `#Purge Bloatware` section deletes locale directories and then
  **symlinks them back to the host** (`:127-132`) — a trade to take or reject
  deliberately.

### `nix-community/nixGL` — **filed elsewhere** (T-052)

Commit `b6105297e6f0cd041670c3e8628394d4ee247ed5`. Mined, **not read**. It is
the inverse shape — make the HOST's GL reachable from a nix program — and the
operator names it as the thing that "killed the dream of a universal builder".
⛔ Nothing here measures it and this entry claims nothing about it.

### `NixOS/patchelf`, `nix-community/patsh` — **filed elsewhere** (T-053)

Commits `7688b17c18d16f67fa8d5a82a2404c2e3a18648d` and
`4e105d962bbce8a85e7ed2feffaddb52d127c87d`. Mined, **not read**. `patsh`
patches store paths in shell scripts, which is precisely the hole
`internal/bundle/appimage.go` names and does not fill.

### `pkgforge-dev/kdenlive-AppImage-Enhanced` — **filed elsewhere** (T-055)

Commit `f7c394a0dd2f589de258eb3a0e4b14f3e6ac8a3c`. Mined, **not read**. It is
the named comparison target for the bundle claim, and no column of that
comparison has been measured.

### `yasunori0418/nput`, `grigio/docker-nixuser` — **refused**

Read at README depth. `nput` is a nix plugin/module manager for user
configuration and has nothing to say about fetching or bundling; the operator's
reading list carried it as an unknown and this is the answer. `docker-nixuser`
is nix-user-chroot in a container, and needing a container is the condition
`TODO` T-051 is trying to avoid.

---

## ⚠ Known-weak claims, read before the recommendations

1. **The 47% figure is a sample of nineteen.** It is enough to establish "not
   0 and not 100", which is what the design turns on, and it is **not** enough
   to quote as a rate.
2. ⛔ **This revision already corrected itself once on that number.** An
   ad-hoc sample of twelve hand-picked popular packages gave 8-of-12 and was
   nearly written down as the rate. It is a sample of the author's taste in
   packages, not of anything.
3. **"The closure is the exact set" is nix's claim, not a measurement here.**
   It is true by construction of how nix builds, and this project has not
   independently verified that a bundle built from a closure never misses a
   library — only that galculator starts on four environments.
4. **"Reaches GTK" is not "works".** The bed has no X server, so what is
   measured is that the loader, the libraries and GTK's initialisation all
   ran. Nothing here clicks a button.
5. **The build environment is a moving part.** `pgb nix build` results in this
   session changed when `bison` was added to the pinned image. A ladder result
   is against that environment and not against nixpkgs alone.

---

## The instruments, all committed and runnable

| | |
|---|---|
| `internal/nixx/fetch.go` | resolve, closure, fetch — with `--selftest` |
| `internal/nixx/nar.go` | NAR, nix-base32, ed25519, narinfo verification — 28 checks including RFC 8032 vectors and two committed real narinfos |
| `internal/nixx/drv.go` | the ATerm derivation reader — 12 checks including both nix escapes and two refusals |
| `internal/nixx/plan.go` | derivation → build plan, shared by both routes |
| `internal/elfx/needed.go` | the one ELF edit the bundler needs, with a toolchain-built fixture |
| `internal/bundle/appimage.go` | the bundler, with `--selftest` for the wrapper-following logic |
| `experiments/80-` | nixpkgs with no nix, oracle-checked against a real nix |
| `experiments/83-` | the `.drv` route, its availability rate, and the reader against the evaluator |

⭐ **The test, in one sentence: could somebody who distrusts this re-run every
load-bearing claim without asking?** For findings 1–3, yes — the commands are
above. For OpenGL and for the bundle comparison, **no, because they have not
been measured**, and that is why they are entries and not claims.
