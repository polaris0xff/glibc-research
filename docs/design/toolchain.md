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
developer has to know and assemble**, and that is measured in
`experiments/63-`.

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
| **size** | only what static linking genuinely could not absorb, rather than the whole closure |
| **friction** | produced by the same one command, with no format-specific input from the developer |
| **honesty** | the tool names every component it could not link statically, and why |

⚠ **If a design cannot beat `sharun` + `uruntime` on at least one of those, the
right answer is to emit an anylinux AppImage and say so**, not to ship a
fifth mediocre bundler. That is a real option and it should stay on the table.

## Language and structure

⭐ **The driver stays POSIX `sh`, and the reason is a hard constraint rather
than a preference.** `pgb build` re-enters itself *inside* the build
environment as `pgb __inner-build`. Whatever the driver is written in has to
exist in that environment — and in every target rootfs `pgb verify` reaches.
`sh` always does. Python, Rust and Go do not: installing a runtime into the
pinned image to run the build tool changes the environment whose contents are
the whole point of pinning it.

Two more requirements point the same way. The brief requires the tool not be a
black box — a shell script is readable by the person debugging it, at the
moment they are debugging it. And most of the work is orchestrating other
programs: `git`, `curl`, `tar`, `chroot`, `gcc`, `ld`. That is what a shell is
for.

⚠ **What `sh` is bad at is exactly what is coming next**: dependency graph
resolution, ELF analysis, and parsing package metadata (`APKINDEX`, `dpkg`
control files, PKGBUILDs, JSON). Writing those in shell would be slow to run
and worse to trust.

⭐ **So the split is by where the code has to run, not by taste:**

| runs | language | why |
|---|---|---|
| **inside** the build environment or a target rootfs | POSIX `sh`, no bashisms | it is the only thing guaranteed present |
| **outside**, on the build host | a real language, or an existing tool | we control that toolchain, and the work needs data structures |

⛔ **And the driver has to be split now, before it grows.** `pgb` is one file
approaching a thousand lines and the planner has not been written yet. The
proposed layout, sourced rather than executed so the re-entry stays one
process:

```
pgb                    entry point: option parsing and dispatch only
tool/lib/common.sh     say/die/verbose, path and engine resolution
tool/lib/env.sh        env create/info -- OCI pull, chroot, libiconv
tool/lib/wrappers.sh   compiler wrapper generation, runtime objects
tool/lib/build.sh      build, __inner-build, shell
tool/lib/verify.sh     verify, and the trace attribution it depends on
tool/lib/source.sh     spec resolution: URL or package name to a source tree
tool/lib/deps.sh       dependency planning, static-first
```

⚠ **Before writing a new ELF or dependency analyser, check `references/`.**
`leleliu008/elftool` is already vendored and manipulates ELF files from the
command line; `ppkg`'s `core/wrappers/` are compiler wrappers in C solving the
same problem `pgb`'s shell wrappers solve. The brief's instruction is to reuse
and patch before reinventing, and it applies here.

⛔ **This is a decision to confirm, not a decision already taken.** It is
recorded here so the reasoning is visible and so the next agent does not
re-derive it — but "keep it in shell" is the kind of choice that gets harder to
revisit the longer it stands. If the planner turns out to need more than the
split above can carry, changing language is a legitimate outcome and the
constraint that decides it is the first row of the table above.
