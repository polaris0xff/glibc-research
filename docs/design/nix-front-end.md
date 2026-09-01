# The package front end: nix, ruled by the operator

⛔ **This page records a DECISION and a READING LIST. Nothing on it has been
built, and — apart from what is explicitly marked measured — nothing on it has
been verified.** It was written at the end of the session of 2026-09-01b, at
the operator's instruction, immediately before that session ended. The mining
it calls for is the **first task of the next session**.

---

## ⭐ The ruling

The operator's words, quoted rather than paraphrased because the reasoning is
the load-bearing part:

> *"Instead of writing resolvers, parsers, dependency checkers etc etc — let's
> just use nix. Their package system is large and we can create a dedicated
> lib/tooling for this. Fetch the packages directly, extract, patch, repatch.
> Even if we only took their package manifest and files, it already
> significantly reduces our workload."*

⛔ **This settles `TODO` T-022** — *"Is a nixpkgs front end in scope, or does
depending on nix defeat the point?"* — which had been an open question for the
operator since the previous session. **It is in scope.** T-022 is no longer a
spike to decide something; it is the design.

⛔ **And it changes T-012.** `pgb build <url-or-package>` was scoped as spec
resolution + build-system detection + a dependency planner, all written here.
⭐ The planner is now **nixpkgs**, and this project's share is the part after
it: fetch, extract, patch, repatch, and link.

⚠ **What it does NOT settle**, and `research/nix-appimage.md` already says why
this matters: **nix's STORE MODEL must not be copied.** T-020's finding was
that a bundler built on nix ends up shipping a container. Taking the *graph*
and the *manifests* is the decision; taking `/nix/store` as a runtime layout is
not, and this page is not a licence for it.

## The two shapes, both from `pkgforge/soarpkgs`

⛔ **Pin `55c774a5e24d9f17af69911a4d70884dfb566626`.** The operator states that
newer versions of that repository **abandoned this approach entirely**, so the
tag is the whole point and a later commit is a different subject.

| shape | reference recipe |
|---|---|
| **static first** | `binaries/bash/static.nixpkgs.stable.yaml` |
| **dynamic / AppImage** | `packages/chromium/nixappimage.nixpkgs.stable.yaml` |

⭐ **This is the same static-first / bundle-last split
[`toolchain.md`](toolchain.md) already argues for**, arrived at independently
by somebody who shipped it. That agreement is worth more than either statement
alone, and checking it is the first thing the mining should do.

⚠ **The operator names one thing to avoid**: *"we would have to avoid all the
complex patching for desktop files etc, there must be a way to automatically
get these."* ⭐ Finding that way is an open question, below.

## ⛔ Not installing nix is now the interesting part

The operator's own note: soarpkgs built inside a container that had nix
pre-installed via `pkgforge/devscripts`' `install_nix.sh`, and

> *"tooling has evolved since then and we don't need to install nix to use
> nix."*

**The reading list, in the operator's order.** ⛔ None of these has been
fetched, read, or run:

| reference | what it is claimed to be |
|---|---|
| `nix-community/nix-user-chroot` | run nix without root, in a user namespace |
| `grigio/docker-nixuser` | the containerised form of the same |
| `yasunori0418/nput` | (unread) |
| `nix-community/nix-ld` | run unpatched dynamic binaries on NixOS |
| `simonfxr/nix-download` | fetch from the binary cache without nix |

⭐ **`simonfxr/nix-download` is the one to read first if only one is read**, on
its name alone: if a store path can be fetched from `cache.nixos.org` with an
ordinary HTTP client, then "use nix" costs this project a *manifest format*
rather than a *daemon*, and the whole of `nix-user-chroot`/`docker-nixuser`
becomes unnecessary. ⚠ That is a guess from a repository name and is exactly
the kind of claim this project does not accept without measurement.

⭐ **`nix-community/nix-ld` is the interesting one for a different reason**: it
solves the *inverse* of this project's problem — running a foreign dynamic
binary on a system whose libraries are not where the binary expects. That is
`docs/limitations.md` §1 seen from the other side, and it may say something
about `TODO` T-033's route D.

## What the next session owes

⛔ **Mine before writing.** `docs/methodology/references.md` is binding:

```sh
sh scripts/common/mine-repo.sh nix-community/nix-user-chroot --out references
sh scripts/common/mine-repo.sh nix-community/nix-ld         --out references
sh scripts/common/mine-repo.sh yasunori0418/nput            --out references
sh scripts/common/mine-repo.sh simonfxr/nix-download        --out references
sh scripts/common/mine-repo.sh grigio/docker-nixuser        --out references
sh scripts/common/mine-repo.sh pkgforge/soarpkgs            --out references
```

⚠ `pkgforge/soarpkgs` is **already in `references/`** from an earlier sweep.
`references.md` trap 7 says re-mine anyway — but ⛔ **check which commit the
existing copy is at first**, because the operator's pin is what makes it
useful and a newer copy is a different repository.

Then, and only then:

1. read the two recipe files at the pinned commit and write down what a
   `*.nixpkgs.stable.yaml` actually contains — the fields, not a summary;
2. answer whether a store path can be fetched **without a nix installation**;
3. answer the operator's open question about desktop files, below;
4. turn the answer into `TODO` entries under T-012 and T-022, sized.

## ⛔ Open questions this page cannot answer

1. **Desktop files, icons and MIME data.** The operator asks for a way to get
   them "automatically". nixpkgs derivations do carry them, in known places
   under a store path. ⚠ Whether that generalises, and whether this project
   should be shipping desktop integration at all when its output is *one
   ordinary ELF*, is unanswered.
2. **Does taking the nixpkgs graph make `pgb` depend on nix at RUN time?** It
   must not. If the answer to "fetch without installing nix" is no, then the
   dependency is a build-time one and needs saying out loud.
3. **What happens to `--wrap-dlopen` and the four runtime mechanisms** when the
   sources come from nixpkgs rather than an upstream tarball? Nothing suggests
   they break; nothing has checked.
