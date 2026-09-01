# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log
and the entries.

    STATE     2026-09-01d (in flight -- rewritten at session end)
    COUNTS    31 entries, 18 open, 13 done
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
| 2. the bundler | T-057, T-052, T-053 | ⚠ **started**: a GTK app starting on musl, and ⭐ **the mesa half of the OpenGL problem solved and measured** — `EGL vendor string: Mesa Project`, `driver name: swrast`, on a machine with no GPU and no host GL. NVIDIA, debloat, and any comparison against a hand-built AppImage are all still open |
| 3. kdenlive | T-054, T-055 | ⛔ **not started, and NOT ruled out.** The ENGINE is already static and proved (POC 80, 11 of 11, renders MP4); Qt/KF6 was **never attempted by anybody** — see T-054, which lists why the mechanism this project already has is the one Qt needs |

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

### ⭐ The OpenGL problem: the mesa half is solved

⛔ **It is not what its name suggests.** A nixpkgs GL program depends on
**libglvnd**, not mesa, and libglvnd finds the implementation by reading
`share/glvnd/egl_vendor.d/*.json` and dlopen'ing what they name — HOST
configuration, so mesa is not in the closure at all. mesa-demos' 111-path
closure carries libglvnd and **not one driver**.

⭐ **So the libGL problem is `docs/limitations.md` §1 arriving from the GL
side**, and `nix-community/nixGL` answers it the same way the bundler now
does: pull nixpkgs' own mesa in and point it at itself. Three parts, each
added because a real run failed — mesa into the closure, the `lib/dri` and
`lib/gbm` trees copied as directories rather than flattened, and ⛔ **the ICD
JSONs rewritten off their absolute `/nix/store` `library_path`**, which had
libglvnd opening a path that was not there while the library sat in `lib/`
beside it.

Measured with no GPU: `EGL vendor string: Mesa Project`, `EGL driver name:
swrast`, OpenGL and OpenGL_ES. ⚠ NVIDIA proprietary is untouched and cannot be
bundled; nothing has run on the eleven; the bundle is 163 MB because nothing
debloats mesa. T-052.

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

## ⛔ THE STOP CONDITION FOR THE NEXT SESSION

⭐ **Operator instruction, 2026-09-01c: the next session has LESS BUDGET, and
it ends when the required POCs are done.** They are these four, in this order,
and each closes with its `Prove` command run and the output in its entry:

| # | POC | closes | done when |
|---|---|---|---|
| 1 | **`poc/90-qt`** — a Qt 6 program, static, through `pgb nix build` | T-054 rung 1 | a Qt widget program runs on 11 of 11 with zero host shared objects, **or** the rung that stopped it is recorded at file and line, the way `evidence/72-.../CPYTHON-FAILURE.txt` already does |
| 2 | **`experiments/85-opengl`** — the bundled GL stack on all eleven | T-052 | a row per environment for `eglinfo`, with the GPU-less caveat stated in the file |
| 3 | **`experiments/86-bundler-vs-anylinux`** — our bundle against a hand-built one | T-057 | size, startup and host-object columns for the same application, both artefacts, same day |
| 4 | **`poc/20` and `poc/30` reruns** — `--embed-terminfo`, `--embed-cacert` | T-032 | both already wired; only the runs are owed |

⛔ **Stop after those four.** Not before — a session that does three has not
finished. Not after — T-055 (the kdenlive bundle comparison) and T-050/T-051
are the session after that, and starting them is how the four above end up
half-done.

⚠ **1 and 2 can run at the same time**: the Qt build is CPU-bound for a long
time and the GL matrix is not. `scripts/common/bootstrap.sh` exists so setup
overlaps with reading, too.

## Work order, after the stop condition above

    T-050   finish the no-nix route; T-051 the no-root host
    T-055   the kdenlive bundle comparison -- blocked on 1 and 2 above
    then P2 by category

## Open questions for the operator

⭐ **None blocking.** Everything asked this session is either answered above or
carried as an entry with the shape of the answer written down.

1. ⭐ **The branch debt from the previous two sessions is CLEARED.**
   `git ls-remote --heads origin` at session end lists `refs/heads/main` and
   nothing else. ⚠ The explicit remote delete still errored with `remote ref
   does not exist`, so what removed them is not established — check
   `git ls-remote` rather than assuming the proxy can now delete branches.
2. **T-015 changes what the bed is.** Unchanged from the previous session.
3. ⚠ **A GPU.** T-052 cannot be honestly closed on a machine with no graphics
   hardware. This bed can show that a bundle's GL stack loads and initialises;
   it cannot show that it renders against somebody's driver.
