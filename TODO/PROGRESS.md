# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log
and the entries.

    STATE     2026-09-01c
    COUNTS    30 entries, 17 open, 13 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, EIGHT POCs
              CI: GREEN, 15 jobs, and it asserts criterion 2
              throughput: glibc 4.53 ns/op vs musl 584.71 (malloc, 4 threads)
    NEW       pgb nix: nixpkgs plans, pgb builds static glibc
              bash, gawk, jq, sqlite3, htop, tmux built from nixpkgs plans
              a GTK app bundled the Anylinux way, starting on musl

## ⭐ The operator's three goals, given mid-session, and now the work order

Quoted, because the framing is load-bearing:

> *"1. Make the 'universal' builder true via pgb + nix. 2. make the 'universal'
> bundler true via a modern, updated, maintained 'nixappimage' descendant that
> uses or rather reimplements many of the anylinux tooling, iterating/improving
> them, and debloating nixappimages, correctly packing them, and also solving
> the opengl problem ... 3. poc a kdenlive static (exhaust all resources), if
> impossible, pivot to kdenlive.nixappimage, but it must be smaller, load
> faster, run faster than pkgforge-dev/kdenlive-AppImage-Enhanced."*

> *"If our project succeeds, suddenly we enable anyone with minimal of setup to
> build any cli/app no matter how complex and make it truly portable without
> spending 10hrs per app."*

| goal | entries | where it stands |
|---|---|---|
| 1. the builder | T-050, T-051, T-012 | ⚠ **started and working**: six packages built static from nixpkgs plans, two verified 11/11 |
| 2. the bundler | T-057, T-052, T-053 | ⚠ **started**: one GTK app bundled and starting on musl; no debloat, no OpenGL answer, nothing measured against a hand-built AppImage |
| 3. kdenlive | T-054, T-055 | ⛔ **not started.** The ENGINE is already static and proved (POC 80); Qt/KF6 and the application are untouched |

## What this session did

### The nix mining, and the front end it produced

- **Six references mined** (nix-ld, nput, nix-download, docker-nixuser,
  nix-user-chroot, and `pkgforge/soarpkgs` **at the operator's pin**, through a
  new `mine-repo.sh --ref`), four more when the operator's second message named
  them (nixGL, patchelf, patsh, kdenlive-AppImage-Enhanced).
  Write-up: **[`../docs/research/nix.md`](../docs/research/nix.md)**.
- ⭐ **`pgb nix plan|fetch|build`.** nixpkgs is the planner; `pgb` builds
  static glibc. The plan is read from the **derivation** — what nix decided
  after every override and overlay — not from the `.nix` source.
- ⭐ **A dependency walk.** Each `buildInput` carries its own derivation, so a
  dependency is planned exactly like a top-level package and built into one
  shared static prefix that later packages reuse.
- ⭐ **An adaptation loop**, which is the operator's "pgb kicks in and patches
  it on the fly": one bounded adaptation per round, each recorded with the
  exact message that triggered it, and **the same fix twice is treated as a
  wrong diagnosis rather than as another round**.

### ⛔ nixpkgs' `pkgsStatic` is MUSL

`pkgsStatic.stdenv.hostPlatform.libc = "musl"`, and the soarpkgs recipe this
project was pointed at builds `pkgsStatic.bash` — output named
`bash-interactive-static-x86_64-unknown-linux-musl-5.3p15`. **nixpkgs has no
glibc-static answer, so `pgb` is the other half rather than a competitor.**

### ⭐ nix is not needed to reach nixpkgs — and the limit is measured

The operator asked whether the `.drv` files make nix unnecessary. They largely
do: a narinfo names its `Deriver:`, that `.drv` is itself a signed store path,
and its `References` are every input `.drv`.

⚠ **Deriver availability by population** (`experiments/83-`): **3%** of any
path in the index, **1%** of paths with no output suffix, **47%** of twenty
named packages. The ones with a cached `.drv` are the ones that are **inputs to
other builds** — zlib, gawk, gnugrep, coreutils — not the popular ones. So the
fallback to evaluation is mandatory, and T-050 stays open saying so.

⛔ **An index lookup is not an evaluation**, two ways: nixpkgs' `bash`
attribute is `bash-interactive`, and the channel index is one revision while a
local nix is another.

### The GUI bundle

`tool/nix-appimage.sh`: uruntime + dwarfs + sharun instead of
appimage-type2-runtime + mksquashfs + a `bwrap` AppRun — which needs the
unprivileged user namespaces `HOW-TO-MAKE-THESE.md` itself calls unreliable.
galculator 2.1.4, 100 store paths, 837 libraries, 64 MB, reaching GTK's own
"cannot open display" on **alpine 3.22 and voidlinux, both musl**, and on
debian 11 and arch.

⭐ **And the closure replaces sharun's library discovery.** sharun walks `ldd`
and straces for `dlopen`'d libraries; a nixpkgs closure is the exact declared
set, dlopen'd libraries included.

### ⛔ Defects found, every one of which had produced a plausible run

- **`pgb verify` ate its own matrix.** It read `rootfs-images.txt` on stdin and
  ran the subject with that stdin inherited, so a static `bash` swallowed ten
  of the eleven rows and the tool declared the binary not portable. Invisible
  until now because every earlier subject ignores stdin.
- **POSIX sh has no locals and the dependency walk is recursive** — four
  defects from one cause, including libcap building **into pam's directory**,
  and a "dep FAILED perl" for a perl that had been skipped.
- **The build system was chosen while composing the command**, so an
  autoreconf hook that generates `./configure` was decided against before it
  ran.
- **`python3 - <<'PY'` cannot be the far end of a pipe** — python takes the
  program on stdin, so the fetch read an empty body and reported "short read".
- **`except ImportError` does not catch a broken accelerator**: Debian's
  python3-cryptography panics through Rust and raises a `BaseException`.
- **The first base32 vector was derived from the encoder under test**, which is
  how a wrong bit order passes its own test.
- **An ad-hoc sample of twelve popular packages** was nearly written down as a
  cache-availability rate. `experiments/83-` carries the correction.

## In progress

Nothing half-written. See `RESUME.md`.

## Work order

    T-052   the OpenGL question    ⛔ it gates goal 2 and goal 3 both
    T-057   the bundler: debloat, wrapper scripts, a measured comparison
    T-054   kdenlive static -- the engine is done, Qt/KF6 is the rung
    T-050   finish the no-nix route; T-051 the no-root host
    T-032   two POC runs, both already wired
    then P2 by category

## Open questions for the operator

⭐ **None blocking.** Everything asked this session is either answered above or
carried as an entry with the shape of the answer written down.

1. ⛔ **Three remote `claude/*` branches are on GitHub** and this environment's
   git proxy refuses to delete a remote branch. All point at commits `main`
   contains; they need one click each in the web UI.
2. **T-015 changes what the bed is.** Unchanged from the previous session.
3. ⚠ **A GPU.** T-052 cannot be honestly closed on a machine with no graphics
   hardware. This bed can show that a bundle's GL stack loads and initialises;
   it cannot show that it renders against somebody's driver.
