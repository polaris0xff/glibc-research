# The package front end: nix, ruled by the operator

⛔ **THE MINING IS DONE AND MOST OF THIS PAGE IS NOW HISTORY.** It was written
at the end of the session of 2026-09-01b as a decision plus a reading list,
with nothing verified. The session of 2026-09-01c mined all six references,
built the front end, and measured the questions this page could not answer.

⭐ **Read [`../research/nix.md`](../research/nix.md) instead of this page** for
what is true; it carries the findings, the instruments and the known-weak
claims. This page is kept for the ruling it records verbatim and for the
answers now written under each open question below.

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

**The reading list, in the operator's order.** ⛔ Only one has been fetched,
and **none has been read or run**:

| reference | what it is claimed to be |
|---|---|
| `nix-community/nix-user-chroot` | run nix without root, in a user namespace. ⚠ **MINED AND NOT READ** — the corpus is in `references/nix-community__nix-user-chroot/` at commit `987302aef4e3aa267355cfad00027b730bcb389b`, because the fetch had already completed when the operator stopped the sweep. A tree in `references/` that nobody has opened is exactly the thing `methodology/references.md` warns is not a finding |
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
# nix-user-chroot is ALREADY mined (unread) -- re-mine only per trap 7
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

## The open questions, ANSWERED

1. **Desktop files, icons and MIME data.** ✅ **Automatic, and no patching.**
   A nixpkgs derivation installs them where the freedesktop spec says, so
   finding them is a `find` and not a rule per application.
   `tool/nix-appimage.sh` does it, and so does the soarpkgs chromium recipe
   this page pointed at.
   ⛔ **With one trap, paid for here:** a closure carries every dependency's
   `share/` too, so the first `.desktop` in the merged tree was **GTK's own**
   `gtk3-widget-factory.desktop`. The application's own store path is searched
   first now.
2. **Does taking the nixpkgs graph make `pgb` depend on nix at RUN time?**
   ✅ **No, and less than expected at BUILD time either.** `pgb nix build`
   from a saved plan needs no nix; and `pgb nix plan` itself now has a
   nix-free route, because a `.drv` is a store path in the binary cache like
   any other. ⚠ That route resolves **47% of named packages** and 3% of the
   store at large (`experiments/83-`), so evaluation stays as the fallback —
   `TODO` T-050 and T-051 carry what is left.
3. **What happens to `--wrap-dlopen` and the four runtime mechanisms** when
   the sources come from nixpkgs? ⚠ **Still unchecked, and now checkable.**
   Every package built through the front end so far — bash, gawk, jq, sqlite,
   htop, tmux — went through the ordinary `pgb build` path with all four
   mechanisms on, and all eleven rows are clean for the two that were
   verified. Nothing has yet built a nixpkgs package that *needs*
   `--wrap-dlopen`.

## ⛔ What the mining found that this page did not expect

- **`pkgsStatic` is musl.** The reference recipe this page names builds a
  **musl** static bash. `pgb` is the glibc half, not a competitor.
- **The `.drv` files are in the cache.** The operator asked; they are; the
  rate is measured.
- **nix-ld maps ELF segments itself** rather than exec'ing the real loader,
  which makes it a reference for `TODO` T-033 route D rather than for this
  page.
