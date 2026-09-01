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

⭐ **Tracked, in this tree, under `references/`.** Twelve repositories, each
with metadata, issues and pull requests in both states, comments, review
comments, releases, tags, and the tree at a captured commit. Re-fetch any of
them with:

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
| `pkgforge-dev/cross-libc-dlopen` | `1cecf50e` | 3 passes, `src/` in full | **adopt an idea, refuse the architecture** |
| `pkgforge-dev/Anylinux-AppImages` | `da7649b9` | `useful-tools/lib/anylinux.c` in full | **adopt**, at file and line |
| `QaidVoid/onelf` | `74b4c9a4` | README + `docs/guide/cross-libc.md` | **refused**, with the reason |
| `VHSgunzo/sharun` | `b1ef7449` | README + `src/main.rs` structure | **refused** for this goal |
| `pkgforge-dev/userland-execve-rust` | `ce431314` | `src/` file list, `loader.rs` skim | **refused**, not applicable |
| `pkgforge-dev/Anylinux-sharun` | — | README | **filed elsewhere** |
| `leleliu008/elftool` | `037310b7` | `src/` file list | **confirms** |
| `leleliu008/ppkg`, `leleliu008/patches` | — | tree structure | **filed elsewhere** |
| `a2flo/standalone_musl` | `368cf49f` | tree structure | **refused**: musl, not glibc |
| `altipla-consulting/distroless-glibc` | `88e4453c` | README + Dockerfile | **anti-pattern exhibit** |
| `allyourcodebase/pipewire` | `5b4930b8` | fetched, not read | **not reached** |



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

`docs/guide/cross-libc.md` (commit `74b4c9a4`) is a clear statement of the same
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
