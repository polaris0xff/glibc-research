# Prior art: what was read, what transfers, and what does not

⚠ **What this sweep did NOT establish, first, because a reader skimming for
the recommendation will not come back for it.**

- **Only x86_64 was run.** Every verdict below that rests on a measurement
  rests on one machine, one kernel (Linux 6.18.44), one day.
- **Depth reached varies by reference and is stated per row.** Two references
  were read at README + architecture level only and are marked as such. A
  verdict of "refused" taken at that depth is a verdict about *fit*, never
  about quality.
- **No tracker discussion was fetched for any repository.** `mine-repo.sh`
  reports it in every `PROVENANCE.md`: discussions are GraphQL-only and the
  credential-free route is REST. That is a real gap, and for a design-heavy
  project it is where the argument often lives.
- **Nothing here was executed.** These are code reads. Every number in this
  repository comes from `experiments/` and `poc/`, which measure *this*
  project's own mechanisms, not these references'.

## Route the reader

| you have | read |
|---|---|
| two minutes | the verdict table |
| ten minutes | the verdict table and §"What the sweep changed about the plan" |
| implementation to do | §"Mechanisms worth taking", at file and line |
| a reason to distrust this | `references/*/PROVENANCE.md`, then the code at the commit named |

## The corpus

⭐ **Tracked, in this tree, under `references/`.** **55** repositories
(`ls references/ | wc -l`), each with metadata, issues and pull requests in
both states, comments, review comments, releases, tags, and the tree at a
captured commit. Re-fetch any of them with:

```sh
sh scripts/common/mine-repo.sh OWNER/REPO --out references
```

⛔ One deliberate deletion:
`references/pkgforge-dev__cross-libc-dlopen/tree/docs/AGENTS.md`. The
vendoring methodology forbids carrying a third party's agent instruction file
into this tree, because a file with that name is read as instructions by the
tools working here. Recorded in that repository's `PROVENANCE.md`; the file
exists upstream at the commit named there.

## Verdicts

| reference | commit | depth | verdict |
|---|---|---|---|
| `pkgforge-dev/cross-libc-dlopen` | `1cecf50e` | `src/` in full, `docs/limits.md`, `docs/overview.md` | **adopt an idea; the full rewrite is untried here** |
| `pkgforge-dev/Anylinux-AppImages` | `da7649b9` | `lib/anylinux.c`, `HOW-TO-MAKE-THESE.md`, `quick-sharun.sh` | **adopt the mechanism; measure against the architecture** |
| `QaidVoid/onelf` | `74b4c9a4` | README + `docs/guide/cross-libc.md` | **refused**, with the reason |
| `VHSgunzo/sharun` | `b1ef7449` | README + `src/main.rs` structure | **refused** for this goal |
| `pkgforge-dev/userland-execve-rust` | `ce431314` | `src/` file list, `loader.rs` skim | **refused**, not applicable |
| `pkgforge-dev/Anylinux-sharun` | — | README | **filed elsewhere** |
| `leleliu008/elftool` | `037310b7` | `src/` file list | **confirms** |
| `leleliu008/ppkg`, `leleliu008/patches` | — | tree structure | **filed elsewhere** |
| `a2flo/standalone_musl` | `368cf49f` | tree structure + README | **not a libc to adopt; unweighed prior art for the plugin problem** |
| `altipla-consulting/distroless-glibc` | `88e4453c` | README + Dockerfile | **anti-pattern exhibit** |
| `allyourcodebase/pipewire` | `5b4930b8` | `src/wrap/dlfcn.zig` | ⭐ **adopt**: `--wrap` on `dlopen` against a compiled-in table |
| `leleliu008/python-distribution` | `987e937a` | `build.sh` CPython recipe, `linux-portable.sh` in full | ⭐ **corroborates `limitations.md` §0, and is the smallest tier-2 implementation seen** |
| ⭐ **`pg83/solo`** | `79451211` | four passes; `dlfcn.cpp` and `elf_loader.h` in full, `elf_loader.cpp` at the resolver/provider/TLS sites, `musl_tls.c`, `dev/`, CI; three build attempts, all failed | ⭐ **adopt the mechanism, refuse the implementation.** A `.so` loader compiled INTO a static binary — §7 **route D**. Its 5,948-line glibc→musl bridge is exactly what a static-glibc host does not need, and `experiments/73-` measures why. Own page: [`solo.md`](solo.md) |



---

## The one mechanism this project actually took

⭐ **`__nss_configure_lookup()`, from
`references/pkgforge-dev__Anylinux-AppImages/tree/useful-tools/lib/anylinux.c`
lines 559–608.**

That file's `init_nssfix()` is a `__attribute__((constructor(101)))` that
resolves `__nss_configure_lookup` with `dlsym(RTLD_DEFAULT, ...)` and pins
fourteen NSS databases to `files` (and `hosts` to `files dns`), with the stated
purpose of stopping glibc dlopening NSS modules an AppImage does not bundle.

**What transfers and what had to change.** The idea transfers completely; the
delivery does not. anylinux.c is an `LD_PRELOAD` object for a **dynamically
linked** bundled application, so it finds the symbol at run time with `dlsym`.
This project needs the same effect in a **statically linked** binary, where
there is no preload and no `dlsym` to do it with. The check that made the
transfer possible is that `__nss_configure_lookup` is a **public, versioned
(`GLIBC_2.2.5`) symbol present in `libc.a`**, not a `GLIBC_PRIVATE` one — so it
can simply be declared and called. `experiments/10-probe-the-host.sh` asserts
that on every run, because the whole design fails quietly if it ever stops
being true. The result is `tool/runtime/pgb-nssfix.c`, which is a direct call
rather than a `dlsym`, and covers the same fourteen databases for the same
reason.

⚠ **The database list is load-bearing and a hosts-only version would have been
wrong.** Measured here: openSUSE Leap 15.6's crash arrives through
`passwd: compat`, not through `hosts`.

---

## cross-libc-dlopen: adopt an idea, refuse the architecture

**What it is.** An `LD_PRELOAD`ed `dlopen` interposer that lets a bundled
application load the **host's** GPU drivers across a libc boundary. Its
mechanism (`src/cross-libc-dlopen.c`): parse the host object, neutralise its
symbol-version tags in a private copy, drop `DT_NEEDED` edges naming a musl
libc, and load the rewritten copy.

**The construction, at file and line** (commit `1cecf50e`):

- `cld_strip_versions()`, `src/cross-libc-dlopen.c:811-817` — rewrites
  `DT_VERSYM`, `DT_VERNEED`, `DT_VERDEF` and `DT_VERDEFNUM` to an unknown tag
  (`'ANYL'`), which `ld.so` ignores. ⭐ **All four together**: the comment at
  :809 records that a verdef left without its versym table segfaults `ld.so`.
- `cld_is_musl_libc()`, `:1652` — matches `.musl` / `ld-musl` and the caller at
  `:1857-1860` drops those `DT_NEEDED` entries, "so a second libc never enters
  the process".
- `cld_is_core_library()`, `:1656`, with the `cld_never_touch` list ending at
  `:1647` — libc, the loader, libpthread, librt are never rewritten.

**⚠ A disagreement between its documentation and its code, which is the finding
that is worth more than either.** The file header at `:8-9` says version
requirements are stripped "from a private **memfd** copy". There is no
`memfd_create` anywhere in `src/` — `grep -rn memfd src/` matches only that
comment. The actual emitter, `cld_emit_copy()` at `:1100-1176`, writes a **real
file** under `XDG_RUNTIME_DIR`/`TMPDIR`/`/tmp` using `O_TMPFILE` + `linkat`,
with an `mkstemp` + `rename` fallback, and the comment at `:1116` explains why:
the result must survive as a normal file so `realpath()` works on it. That is a
different security and lifetime story from an anonymous memfd, and the file
knows it — there is a `sweep_stale_cld_tmp()` constructor at `:103` deleting
day-old leftovers, which an anonymous memfd would never need.

**Verdict: refused as an architecture, for a stated reason.** It solves the
*opposite* problem. It exists to let host objects **in** across a libc
boundary; this project exists to keep them **out**. Its own README says so at
line 66: "This is a preload, not an AppImage feature. It needs a dynamically
linked process" — and this project's output has no interpreter at all.

⭐ **What was taken from it is a fact, not code:** it demonstrates that a host
NSS or gconv module *could* be made loadable into a foreign-libc process by
rewriting it. That is precisely the design this project rejected, and reading
the machinery required — ELF parsing, version stripping, dependency-edge
surgery, a private on-disk copy per object, a cache, a provider scan — is what
makes "do not load them at all" obviously the cheaper answer.

---

## onelf: refused, and it names the wall itself

`references/QaidVoid__onelf/tree/docs/guide/cross-libc.md` (commit `74b4c9a4`)
is a clear statement of the same
problem this project addresses, and of a different answer to it: bundle the
libc and inject an **`AT_EXECFN` bootstrap** into each bundled executable,
which reads `AT_EXECFN`, computes the bundled loader's path relative to the
binary, maps it, and jumps into it, so "the host's own loader is never
consulted".

⭐ **One detail is worth carrying even though the architecture is refused.**
That document records that earlier versions rewrote `PT_INTERP` to a relative
path and it "broke when the new path didn't fit in the original slot" — a
`PT_INTERP` string can only be replaced in place with something no longer.
Anyone tempted by interpreter rewriting as a cheap portability trick should
read that paragraph first.

**Verdict: refused**, and the brief refuses it too — it is a self-extracting
single-file format with three execution modes (memfd, FUSE, on-disk cache).
This project's requirement is a *normal* ELF with no launcher and no
extraction. onelf is the right answer to a different requirement: bundling an
arbitrary directory, including graphical stacks that a static link cannot
absorb.

---

## sharun, userland-execve-rust: refused, not applicable

**sharun** (`b1ef7449`) assembles a `--library-path` and drives a bundled
loader for a **dynamically linked** application in an AppDir. Everything it
does is about arranging shared-object search order. A static binary has no
search order to arrange.

**userland-execve-rust** (`ce431314`, `src/loader.rs`, `src/stack.rs`,
`src/exec.rs`) maps an ELF and its interpreter into the current process and
transfers control without `execve(2)`. It is real and it is useful for
executing a *different* loader from inside a process. This project's binaries
have no interpreter to redirect.

⚠ **Read at README plus file-list depth only.** The verdict is about
applicability, which is decidable at that depth; it is not a judgement of the
implementations.

---

## distroless-glibc: kept as an anti-pattern exhibit

A Dockerfile producing a minimal glibc base image. Kept **on purpose**, because
it is a clean illustration of the assumption this project's measurements
falsify: that "glibc is present" is what a glibc binary needs. It is not. The
measurements here show a static glibc binary needing, variously, the host's
NSS modules, its gconv tree at a *specific* path, its locale archive, a
terminfo database and a CA bundle — none of which follow from a libc being
installed, and several of which differ by path between distributions that all
ship glibc.

---

## `leleliu008/python-distribution` @ `987e937a`

⭐ **Read because a CPython claim was about to be published without checking
whether somebody had already built one**, and it is the second time this
session that reading first changed what got written.

**It corroborates [`../limitations.md`](../limitations.md) §0 rather than
contradicting it.** For a self-contained CPython it sets CPython's own
`MODULE_BUILDTYPE=static` (`build.sh:1035`) with
`--enable-static --disable-shared` (`:1029`), so **every stdlib module is
linked into the interpreter**. Nobody there attempts a static interpreter with
shared modules, which is what §0 says cannot work. ⚠ That is agreement from an
independent implementation, not proof; §0's proof is
`experiments/72-static-host-plugin-abi.sh`.

⭐ **`linux-portable.sh` is the interesting half, and it is tier 2 of
[`../design/tiers.md`](../design/tiers.md) in about sixty lines of C.** For
each dynamically linked executable in the tree it:

1. renames `foo` to `foo.bin` and generates `foo.c`, compiled with
   `gcc -static -Os -s -flto` (`:242`) — so **what the user runs is an
   ordinary static ELF**;
2. that launcher reads `/proc/self/exe`, derives `<self>.bin` and the bundled
   `lib/` directory beside it, and **`execv`s the bundled loader directly**
   (`:174`):

```c
argv2[0] = dynamicLoaderPath;
argv2[1] = "--library-path";  argv2[2] = libraryPath;
argv2[3] = "--argv0";         argv2[4] = selfExecPath;
argv2[5] = realExePath;
```

3. copies the needed system shared libraries into `lib/` and adds an
   `$ORIGIN`-relative RPATH with `patchelf`;
4. on musl, symlinks `ld-musl-<arch>.so.1` to `libc.musl-<arch>.so.1`, because
   there the loader and libc are one file under the libc's name (`:306`).

**Three details worth taking, none of which this project had written down:**

| | |
|---|---|
| ⭐ **`--argv0`** | without it the program sees the *loader's* path in `argv[0]`. CPython derives `sys.executable` from it, so a bundle that omits this has an interpreter that cannot find itself. A cheap mistake to make and an expensive one to debug. |
| ⭐ **`execv` the loader, do not set `LD_LIBRARY_PATH`** | the library path is an argument to the loader for this one process. It is not inherited by children and cannot leak into anything the program later runs. |
| ⚠ **no shell anywhere in the delivery path** | [`../design/toolchain.md`](../design/toolchain.md) names "no shell in the delivery path" as one axis a `pgb` bundle would have to win on. This implementation already meets it, so that axis is **not** available as a differentiator. |

⛔ **What it does not do**, and the reason it is not a rival to tier 1: the
artefact is a **directory** — launcher, `.bin`, and `lib/` — not one file. It
answers "how does this reach a machine", which
[`../design/toolchain.md`](../design/toolchain.md) says is a different
question from the one `pgb` answers.

⚠ **Depth: read, not run.** `build.sh` and `linux-portable.sh` were read in
full; nothing here was executed and no artefact was measured. A claim about
what it *achieves* on the eleven environments would need `experiments/`.

## What the sweep changed about the plan

1. **It supplied the central mechanism.** The NSS override came from
   anylinux.c. Without that read, this project would likely have gone down the
   route cross-libc-dlopen took — rewriting host modules so they *can* load —
   which is far more machinery for a worse outcome.
2. **It ruled out three architectures early**, with reasons that are written
   down so they are not re-derived: preload (needs a dynamic process), bundled
   loader + AppDir (needs a directory), self-extracting format (the brief
   refuses it).
3. **It supplied one trap.** onelf's `PT_INTERP`-length note, above.
4. **It did not supply a gconv answer.** No reference read here solves gconv
   for a static binary. Static GNU libiconv behind `-Wl,--wrap`
   (`experiments/30-`) is this project's own answer, and it is the part with no
   prior art behind it.

---

## Per-reference detail for the four rows that carry the most weight

### `pkgforge-dev/Anylinux-AppImages` — the architecture, not just `anylinux.c`

`useful-tools/lib/anylinux.c` is where this project's NSS override came from.
`HOW-TO-MAKE-THESE.md` and `useful-tools/quick-sharun.sh` carry the rest of the
architecture:

- ⭐ **bundle everything, including libc and the dynamic loader**, then run the
  *bundled* `ld-linux.so` explicitly with `--library-path`. Not
  `LD_LIBRARY_PATH` (inherited by children, and that page has a war story about
  it), not an `$ORIGIN` rpath patched in after the fact;
- ⛔ **it bundles the gconv tree on purpose** and reaches it with `GCONV_PATH`.
  `quick-sharun.sh:816` — "gconv is always deployed, removing it only saves
  ~30 KiB". Those modules' `DT_NEEDED libc.so.6` binds to the **bundled** libc,
  so no second libc enters. `AGENTS.md` §14's refusal to bundle gconv is
  correct **for a static binary** and does not generalise to a bundle that
  carries its own loader;
- it also bundles `libnss_*` and preloads `anylinux.so`, so NSS is pinned *and*
  the pinned modules are present.

⛔ **Measure against this stack, not against vanilla AppImage.** Vanilla
bundles no glibc and scores 2 of 11 (`experiments/60-`); built as this project
documents, the same program runs on **11 of 11**, musl included, with zero host
objects in the payload (`experiments/62-`).

⚠ **Its own criticism of this project deserves quoting rather than
paraphrasing**, from "The solution": *"Compile statically! Sure, that works, go
and compile all of kdenlive statically and get back to me once you get it
done."* That is the real boundary between the two approaches, and it is about
what can be built, not about what runs.

### `pkgforge-dev/cross-libc-dlopen` — what `docs/limits.md` says about the static case

That page has a section titled **"Static binaries: three cases, not one"**:

| its case | what it says |
|---|---|
| static musl | `dlopen` is a stub; genuinely out of scope |
| **static glibc** | `dlopen` **works**. ⭐ "The real blocker is more likely the preload path than `dlopen`: a fully static binary has no `LD_PRELOAD` mechanism, because there is no dynamic loader to honour it" |
| mostly static, libc dynamic | squarely in scope, the easiest |

⛔ **It labels all three UNVERIFIED**: "No measurement of any of them has been
taken in this repository." ⭐ So `experiments/50-` measured something its
upstream had not, which is worth reporting back.

⛔ **`experiments/50-` ported one function out of roughly forty, so what it
rules out is narrower than a verdict on the approach.** `cross-libc-dlopen.c`
is 2015 lines; 50- ported `cld_strip_versions()` alone. The rewrite in
`cross-libc-dlopen.c:1857` is three coordinated steps, and the other two are
the ones aimed at the failure 50- actually saw:

```
cld_strip_versions(&e);                       // ported by 50-
e.dyn[drop_idx[i]].d_tag = CLD_NEUTRAL_TAG;   // NOT ported: drop the DT_NEEDED
                                              // edges that pull a foreign libc in
cld_apply_renames(&e, dry_run);               // NOT ported
```

The failures 50- recorded — `_dl_call_libc_early_init: Assertion 'sym != NULL'
failed`, and friends — are what happens when the host object drags the **host
libc** in. Dropping that edge is exactly step two, so the untested steps are
the ones aimed at the observed failure. ⭐ Porting the full rewrite is
`AGENTS.md` §7, route B.

⭐ It also ships `CROSS_LIBC_DLOPEN_DRYRUN`, which "makes the whole rewrite
path testable with no GPU and no Alpine" — a cheaper instrument than the one
50- built.

### `allyourcodebase/pipewire` — the closest prior art to pgb's own mechanism

`src/wrap/dlfcn.zig`, in its own header comment: *"since it ships with the
necessary plugins, there's no reason we can't just bake these into the
executable and then stub out `dlopen` to avoid a dependency on the dynamic
linker."* It exports `__wrap_dlopen`, `__wrap_dlsym`, `__wrap_dlclose` and
resolves against a compiled-in table of libraries and symbols.

⭐ **That is `-Wl,--wrap` against `dlopen`, which is the same delivery
mechanism pgb already uses for `iconv_open`.** It is the missing automation
behind `AGENTS.md` §7's "a program loading its *own* plugins is fine — build
them in": POC 50 does this by hand for CPython, and this shows the generic
shape. It is route A of `AGENTS.md` §7, and the cheapest of the three.

### `a2flo/standalone_musl` — an integrated loader in a static binary

Its README: a musl fork whose **goals** are
*"'semi-static' binaries, i.e. statically linked binaries that integrate the
dynamic loader, allowing the use of `dlopen()`, `dlsym()`"*, plus *"ABI and API
compatibility with glibc, as far as necessary"*.

⚠ **That is a fourth route to the host-plugin class**, alongside the three in
`AGENTS.md` §7 — and it is an existence proof that a *statically
linked* binary can carry a working loader, which is the thing route C assumes
costs the single-ELF property. ⛔ It is not adoptable as a libc here: it is
musl, and its README concedes *"GNU indirect function support ('ifunc') is not
fully supported"* — which `experiments/61-` shows is exactly where musl's
throughput goes. **What is worth taking is the design, not the libc.**
