# Sweep: nix-appimage, and why bundlers end up shipping a container

⭐ **Why this family was swept.** Nix has the largest package database in
existence, and `nix bundle` turns any of it into a single file with one
command. That is the closest anything gets to what
[`../design/toolchain.md`](../design/toolchain.md) describes — *name a package,
get a binary* — so what these projects hit is the most direct evidence
available about what `pgb build <spec>` will hit.

⛔ **The finding, in one sentence: the bundle contains a container, and the
container is not optional.** It is forced by nix's absolute-path store, it
costs the class of applications that use namespaces themselves, and everything
downstream of it is size-shaving that re-introduces host dependencies.

**Corpus.** Five repositories mined with `scripts/common/mine-repo.sh` and kept
under `references/`, plus one recipe file fetched at a pinned commit:

| reference | commit | depth |
|---|---|---|
| `ralismark/nix-appimage` | `7946addb` | README, `mkAppImage.nix`, `extra-files.sh`, 22 issues + 7 PRs |
| `pkgforge/nix-appimage` | `b6e7dfce` | README, `mkAppImage*.nix`, 37 PRs |
| `of-the-stars/nix-appimage` | `71c292eb` | ⚠ tree is a single dangling symlink into `/nix/store`; nothing to read |
| `logos-co/nix-bundle-appimage` | `04a3cf89` | `mkAppImage.nix`, `fetchAppImageRuntime.nix`, `tests/smoke.sh` |
| `VHSgunzo/runimage` | `f1512f2c` | README, 13 issues |
| `pkgforge/soarpkgs` | `6f1cbb9b` | one file: `packages/chromium/nixappimage.nixpkgs.stable.yaml` |

⛔ **Gaps, per `../methodology/references.md`.** Discussions were **not
fetched** for any of these — the credential-free route is REST and discussions
are GraphQL only, recorded in every `PROVENANCE.md`. `of-the-stars` was fetched
and has no readable tree. Nothing here was executed: these are code and tracker
reads, not measurements.

---

## 1. What the shape is

`nix bundle --bundler github:ralismark/nix-appimage nixpkgs#hello` produces
`hello.AppImage`. The bundler computes the derivation's closure, copies every
path into the image, and at run time **recreates `/nix/store` at that absolute
path** so the binaries' baked-in store paths resolve.

⭐ **That last step is the whole design, and it is where everything follows
from.** Nix binaries hard-code `/nix/store/<hash>-…` into their `RPATH` and
their interpreter. A bundle therefore cannot simply place files anywhere — it
has to make `/nix/store` exist at `/nix/store` on a machine that has no `/nix`.

## 2. ⛔ The container is forced, and the maintainer says so

The mechanism is an unprivileged user namespace plus a chroot. From
`ralismark/nix-appimage` issue #10, the maintainer:

> "nix-appimage uses unprivileged user namespaces itself — it works by copying
> all required nix store files into the AppImage, then 'mounting' them at
> `/nix/store`. I don't see any other way of doing that without user
> namespaces."

and, on why the chroot is also unavoidable:

> "i need chroot in order to mount the bundled `/nix/store`. on systems without
> nix, I'm pretty sure i need there to already be a `/nix` directory in order
> to mount, and since i can't make that directory, i need to make a copy of `/`
> with the extra directory and chroot"

⛔ **The cost is exact and it is in the same issue.** A reporter found that
bundled `unshare`, Steam, Chromium and Electron all fail, and identified the
kernel rule that explains it — `unshare(2)`, `EPERM` since Linux 3.9:

> "CLONE_NEWUSER was specified in flags and the caller is in a chroot
> environment"

So: the bundle needs a namespace to exist, which needs a chroot, and **a
process inside a chroot cannot create a user namespace**. Every application
that sandboxes itself — every browser, every Electron app — is locked out by
the mechanism that makes the bundle work at all. Issue #10 is open.

⭐ **This is the strongest external evidence for `pgb`'s shape that this
project has.** A static ELF needs no store path to exist, so it needs no mount,
no namespace and no chroot — and therefore does not spend the target's one
namespace budget. `experiments/62-` measures the same property against
`Anylinux-AppImages`, which reaches the same conclusion from the other
direction by bundling libraries at relative paths rather than absolute ones.

⚠ **`runimage` is the same idea taken to its end**, and is honest in its own
first line: *"Portable single-file Linux container in unprivileged user
namespaces"* — a whole Arch Linux rootfs, `bubblewrap`, `tini` as init,
DwarFS. Its issue #15, open: *"runimage doesn't work when apparmor is
enabled"*. That is the same fragility one layer up.

⚠ **`pkgforge/nix-appimage` forks the original specifically to lean on the
container**: its README says the changes are to *"Use BubbleWrap & Static
Binaries"* and a *"Universal AppRun"*. It is a better-engineered container, not
a way of avoiding one.

## 3. ⛔ It does not remove the glibc problem, it relocates it

Issue #19, closed. A `hello.AppImage` built on one machine, run on another:

```
/usr/lib64/libc.so.6: version `GLIBC_2.34' not found
```

The reporter's own conclusion: *"the issue is the bundled apps relying on
glibc, not the appimage itself"*. ⚠ The README's claim of being "without the
glibc dependency" is about the **bundler**, not about what it bundles.

⭐ **Two things this project already knows are confirmed here from outside.**
Nix carries its own glibc in the closure, which is the tier-2 shape
`../design/tiers.md` describes — and the same shape `experiments/60-` measured
onelf failing on, for gconv. And issue #5's comment thread records that nix
patches glibc with `dont-use-system-ld-so-cache.patch` to stop it consulting
the host's cache: an upstream acknowledgement that a bundled glibc reaching for
host state is a real failure mode, which is the same class of problem
`tool/runtime/pgb-nssfix.c` exists to solve for NSS.

## 4. ⛔ The size problem, and what people do about it

`nix bundle` gives you the *entire* closure — correct, complete, and enormous.
`pkgforge/soarpkgs`' chromium recipe (`6f1cbb9b`) is the clearest available
record of what that costs, because it is roughly sixty lines of post-processing
under a heading that reads `#Purge Bloatware`:

- delete every `share/locale`, `share/fonts`, `share/man` directory —
  ⛔ **and symlink them back to the host's `/usr/share/locale`**;
- delete `.a`, `.cmake`, `.jmod`, docs, `LICENSE`, `Makefile`, `include/`,
  `nix-support/`, tests, `__pycache__`, `ensurepip`, `mkspecs`, translations;
- inside llvm, perl and systemd directories, delete every file that is not a
  `.so`;
- then repack with `zstd -Xcompression-level 22`.

⭐ **The symlink to `/usr/share/locale` is the finding.** To make the bundle
small enough to ship, the recipe re-introduces exactly the host dependency the
bundle existed to remove. That is the trade the operator described — *too large
and too slow, else it contains a container* — visible as code.

⚠ **It is not incompetence, it is the closure.** Nothing in the nix model lets
the bundler know that a locale directory is unreachable from this entry point,
so the only available lever is deletion after the fact, by pattern, per
package, by hand.

## 5. What transfers, and what must not

| | |
|---|---|
| ⭐ **`nix bundle`'s interface is the target** | `nix bundle nixpkgs#chromium` is precisely the ergonomics `pgb build <spec>` is aiming at, and it is proof the interface is achievable over a large package database |
| ⭐ **nixpkgs is a dependency graph already solved** | every derivation names its inputs exactly. `../design/toolchain.md`'s planner needs that graph from somewhere, and building it from distro metadata is strictly harder than reading one that exists |
| ⛔ **the absolute-path store must not transfer** | it is what forces the namespace, the chroot, and the loss of the sandboxing class. `pgb` links statically and has no store to recreate; keep it that way |
| ⛔ **"delete by pattern after the fact" must not transfer** | it is size control without a model of what is reachable. Static linking's `--gc-sections` and archive semantics already answer this at link time, per-symbol rather than per-directory |
| ⚠ **the OpenGL hole is instructive** | issue #5: bundled Mesa does not work on a foreign host, `nixGL` is the workaround, and a commenter's objection stands — needing nix on the target *"defeats the purpose of distributing an appimage"*. `cross-libc-dlopen` exists to solve exactly this and `Anylinux-AppImages` wires it in |

## 6. What this changes about the plan

1. ⭐ **A nixpkgs front end for `pgb build` is worth a spike**, and it is
   cheaper than a distro-metadata resolver: nix can produce a derivation's
   source and its complete input graph, which is the hard half of the planner.
   Taking the graph while refusing the store layout is the interesting move.
2. ⛔ **Do not adopt any bundler that needs a user namespace on the target.**
   Issue #10 is the measured cost, in the maintainer's own words, and it is
   permanent — it is a kernel rule, not a bug.
3. ⚠ **A bundle's size must be decided at link time, not by post-hoc `rm`.**
   Anything else ends where the chromium recipe ends: symlinking host paths
   back in.
4. ⚠ **None of this is measured here.** These are code and tracker reads. The
   claims above about what nix-appimage *does* come from its source at the
   commits named; the claims about what it *costs* come from its tracker, which
   is evidence of intent and not of behaviour. Building one and running it on
   the matrix is an experiment nobody has run.
