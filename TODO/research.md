# research — sweeps and prior art

Binding: [`../docs/methodology/references.md`](../docs/methodology/references.md).

---

## T-020 — Sweep the nix-appimage family

**Source** operator, session of 2026-09-01.
**Category** research · **Priority** P1 · **Effort** M · **Status** ✅ done

**Problem.** Nix has the largest package database and `nix bundle` is the
closest existing thing to `pgb build <spec>`. What that family hits is the most
direct evidence available about what T-012 will hit.

**Premise.** Operator's, and ⭐ **confirmed**: these projects get abandoned
because the bundle ends up containing a portable container, or is too large and
too slow without one.

**Closed with.** `docs/research/nix-appimage.md`. Five repositories mined and
kept under `references/`, pinned; plus `pkgforge/soarpkgs`' chromium recipe at
`6f1cbb9b`. The container is **forced** by nix's absolute-path store — the
maintainer says so in `ralismark/nix-appimage` issue #10 — and it costs every
application that sandboxes itself, because `unshare(2)` returns `EPERM` for
`CLONE_NEWUSER` inside a chroot. The chromium recipe's `#Purge Bloatware`
section deletes locales and then **symlinks them back to the host**.

⛔ **Gap, recorded:** discussions were not fetched for any of them — GraphQL
only, no credential-free route. Nothing was executed; these are code and
tracker reads.

## T-021 — Build one nix-appimage and run it on the matrix

**Source** follow-on from T-020 · **Category** research · **Priority** P2 · **Effort** M · **Status** open

**Problem.** T-020's claims about what nix-appimage *costs* come from its
tracker, which is evidence of intent and never of behaviour.

**Approach.** `nix bundle` the same subject `experiments/60-` uses, run it on
all 11 with the `62-` instrument. ⚠ Needs nix on the build host, which this
environment does not have.

**Prove.** `evidence/64-nix-appimage/RESULT.txt` with the coverage and
host-object columns filled, comparable to `60-` and `62-`.

## T-022 — Spike a nixpkgs front end for the planner

⭐ **RULED ON BY THE OPERATOR, 2026-09-01b: IN SCOPE.** The open question this
entry existed to settle — *"does depending on nix defeat the point?"* — is
answered. `../docs/design/nix-front-end.md` records the ruling verbatim, the
two reference recipe shapes at their pinned commit, the "use nix without
installing nix" reading list, and what the next session owes before writing
anything. ⛔ **The mining is the FIRST task of the next session** and this
entry should be re-scoped from it, not from the text below.

**Source** follow-on from T-020 · **Category** research · **Priority** P2 · **Effort** M · **Status** open

**Problem.** T-012's planner needs a dependency graph. Building one from distro
metadata is strictly harder than reading one that already exists.

**Premise.** ⭐ Every nix derivation names its inputs exactly. Taking the graph
while refusing the store layout is the interesting move.

**Prove.** For one package, `pgb` prints the transitive input list obtained
from nix, and each entry marked link-statically / build-static / bundle.

## T-052 — The libGL problem, and whether a bundle can honestly claim "universal"

**Source** ⭐ **operator, 2026-09-01c**: *"I also remember an infamous 'libGl'
problem with the nixappimages created, which further killed the dream of a
'universal' builder as many graphical apps didn't work. See:
https://github.com/nix-community/nixgl"*
**Category** research · **Priority** P1 · **Effort** M · **Status** open

**The problem, stated before it is measured.** OpenGL and Vulkan are the one
class of library that **must** come from the host, because it is the host's
kernel driver that the userspace half has to match. Bundling a `libGL.so` from
nixpkgs and running it against another machine's GPU driver is not the same
kind of portability as bundling `libpng`. ⚠ Everything in this paragraph is a
statement of the problem, not a measurement, and this entry exists to replace
it with one.

**What the mining already gives us.**
- `nix-community/nixGL` exists for exactly this and wraps a program so that
  the host's GL implementation is found. It is the inverse shape to ours.
- The `Anylinux-AppImages` flow bundles a **debloated mesa** and has
  `DEPLOY_OPENGL`/`DEPLOY_VULKAN` on by default, `SHARUN_MESA_PATH` to point
  at an external mesa at run time, and `SHARUN_NO_NVIDIA_EGL_PRIME` /
  `SHARUN_ALLOW_SYS_VKICD` for the nvidia and ICD cases
  (`references/pkgforge-dev__Anylinux-sharun`, README). ⭐ **So the project the
  operator points at as the quality bar has already decided this is bundled
  mesa plus escape hatches**, not host GL.

**Approach.**
1. Build a GL program (`glxinfo`, or `mpv`) through `tool/nix-appimage.sh` and
   run it on the eleven, with a software rasteriser present and absent.
   Report what each row does; ⛔ **do not report "works" from a machine with
   no GPU at all.** This bed has no GPU, which is a real limit of the
   measurement and has to be said.
2. Read nixGL's mechanism at file and line and write down which of its cases
   apply to a sharun bundle.
3. Decide, with the measurement in hand, between bundled mesa (Anylinux's
   answer), host GL by path detection (nixGL's), and a hybrid.

**Prove.** `evidence/85-opengl/RESULT.txt` with a row per environment and per
strategy, and the GPU-less caveat stated in the file rather than inferred.

## T-053 — patchelf and patsh: use them, or say why not

**Source** operator, 2026-09-01c: *"did we have no need to use tools like
patchelf & patsh here?"*
**Category** research · **Priority** P2 · **Effort** S · **Status** open

**Where it stands.** `tool/elf-needed.py` does ONE edit — rewrite an absolute
`DT_NEEDED` to its basename, in place, at the same `.dynstr` offset — because
that is the single edit `tool/nix-appimage.sh` needed and patchelf is not on
this machine. That is a reason for the sixty lines, not an argument against
patchelf.

⭐ **And the ground has moved since: `scripts/common/nix-fetch.sh` can now
fetch patchelf's own closure from cache.nixos.org**, so "not installed" is no
longer a blocker for either tool.

**The two questions, and they are different.**
- **patchelf** does what `elf-needed.py` does and much more (interpreter,
  RPATH, soname, shrinking). ⚠ It is also famous for producing binaries that
  break in subtle ways when it has to grow a section, which is exactly why
  `elf-needed.py` refuses to move anything. Decide per edit, not per tool.
- **patsh** patches store paths in **shell scripts**, which is the gap
  `tool/nix-appimage.sh` currently *names and does not fill*: a nixpkgs
  `bin/x` that is a wrapper script is followed to its ELF and the wrapper's
  environment is dropped. ⭐ That is a real hole in the bundler and patsh is
  aimed straight at it.

**Prove.** A written comparison at file and line, plus either a patsh-shaped
step in the bundler with a wrapper-using application working through it, or
the measurement that says why it is not needed.

## T-057 — The bundler: a maintained nix-appimage descendant, on the Anylinux mechanisms

**Source** ⭐ **operator, 2026-09-01c**, and it is the second of three goals:
*"make the 'universal' bundler true via a modern, updated, maintained
'nixappimage' descendant that uses or rather reimplements many of the anylinux
tooling, iterating/improving them, and debloating nixappimages, correctly
packing them, and also solving the opengl problem"*.
**Category** research · **Priority** P1 · **Effort** L · **Status** ⚠started

**Landed already.** `tool/nix-appimage.sh` builds one: uruntime + dwarfs +
sharun instead of appimage-type2-runtime + mksquashfs + a bwrap AppRun, with
the nixpkgs **closure** replacing sharun's ldd-and-strace library discovery.
galculator 2.1.4 reaches GTK's own "cannot open display" on alpine 3.22,
voidlinux (both musl), debian 11 and arch.

⛔ **What it does NOT do yet, each of which is between it and the bar:**

1. **No debloating at all.** galculator is 64 MB from a 315 MB closure and the
   Anylinux flow would strip locales, docs, static archives and unused
   drivers. ⚠ Their chromium recipe deletes locale directories and then
   **symlinks them back to the host**, which is a trade to copy deliberately
   or reject deliberately, not to miss.
2. **OpenGL** — T-052, and for a video editor it decides whether it runs.
3. **Wrapper scripts.** A nixpkgs `bin/x` that is a shell wrapper is followed
   to its ELF and the wrapper's ENVIRONMENT is dropped. T-053; `patsh` is
   aimed at exactly this.
4. **No 32-bit path.** `lib32` exists in the Anylinux layout and not here.
5. **Nothing is measured against a hand-built Anylinux AppImage.**
   `experiments/62-` compares `pgb` against one; nothing compares OUR bundle
   against one.

⭐ **The claim worth making, stated so it cannot drift:** not "as good as a
hand-crafted AppImage" but *"produced by one command from a package name, and
within measurable distance of one"*. `docs/AGENTS.md` §14 governs the
wording.

**Prove.** `evidence/86-bundler-vs-anylinux/RESULT.txt`: the same application
as an Anylinux AppImage and as ours, on all eleven, with size, startup, and
host-object columns — the instrument in `experiments/62-` already produces
three of those four.
