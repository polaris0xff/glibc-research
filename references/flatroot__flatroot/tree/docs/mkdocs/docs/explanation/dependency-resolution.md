---
tags:
  - explanation
  - resolver
  - dependencies
---

# Dependency resolution

The seed list is assembled. It names the user's `bash` and `firefox`, the distro's mandatory `glibc` and `coreutils`, and Debian's Essential packages. The resolver's job is to turn that list into the complete set of every package the rootfs will contain: all transitive dependencies, every alternative picked, every virtual name mapped to a real provider, every conditional payload whose trigger is satisfied.

The gap between the seed and the closure is large. A small seed on a Debian bookworm install grows into a much larger closure after resolution — each new package was pulled in because some other package declared it was needed. The resolver's output is not just the set — it is the set *in order*: the breadth-first visit order, in which the seed packages come first and each package precedes its own dependencies. Extraction follows that order, and it matters because the layout-creating packages — which live in the base/Essential seed set — must be unpacked before the packages that write through the directory symlinks they create.

This page develops why the resolver uses breadth-first traversal, how it handles the complications real dependency graphs throw at it, and what it deliberately leaves for later phases.

## Why breadth-first

The resolver walks the dependency graph from the seed outward. At each step it pops a package name from a queue, looks up that package's dependencies, and queues every newly discovered name. When the queue is empty, every package reachable from the seed has been found.

The traversal order — breadth-first versus depth-first — matters for two reasons that are not about algorithmic elegance.

**Install order.** Some packages create directory symlinks that later packages depend on. On a modern Debian system the `usrmerge` package establishes the merged-`/usr` symlinks (`/bin → usr/bin` and friends); on Arch the `filesystem` package does the same. Packages extracted before those symlinks exist write their `/bin` entries into a real directory; packages extracted after write through the symlink into `usr/bin`. If a later package's files land in the real `/bin` while the rest of the system expects them under `usr/bin`, binaries are not found at the paths their dependents expect.

Breadth-first traversal commits packages in the order it reaches them. Each iteration queues the current package's dependencies onto the back of the queue and then commits the current package to the install order — so a package is committed *before* the dependencies it just queued, and within any one chain the order runs parent-first (`app`, then `lib`, then `libc`). What keeps the layout correct is therefore not the chain order but the seed order: the base and Essential packages — among them the layout-creating `usrmerge`/`filesystem` — are queued ahead of the user's requests, so they are committed first and extracted before anything that writes through their symlinks.

Depth-first would undermine this. A depth-first walk would dive down the first seed's dependency chain to its leaves before reaching the next seed — so a deep transitive dependency of the user's package could be committed before a base seed like `filesystem` was even visited. Breadth-first keeps every seed (and the shallow packages near them) at the front of the order, so the seeded layout-creators stay ahead of the deeper closure.

**Cycle handling.** Real dependency graphs have cycles: `bash` depends on `libc6`, `libc6` depends on `gcc-12-base`, `gcc-12-base` depends on `libc6` again. Breadth-first makes the cycle visible: the visited set short-circuits the second encounter, the duplicate is skipped, and the walk continues. The cycle's existence is evident from the order in which names appear — a package and its dependency appear close together in the output, making it clear that the dependency edge looped back.

## The visited set

The resolver maintains a set of package names it has already committed to the install set. Every time a name comes off the queue, the resolver checks the set. If the name is already there, the iteration ends — no resolution, no dependency walk, no edge recording.

The check happens twice per iteration. The first check is against the popped name. The second check happens after the name has been resolved to a real package, because two different dependency declarations can reach the same real package through different names. A declaration of `Depends: awk` and a declaration of `Depends: gawk` both ultimately mean "install the package called `gawk`." The visited set is keyed on the real package name, not the declared name, so the second encounter is caught.

Without the second check, virtual names could cause duplicate expansion. The walker would resolve `awk` to `gawk`, walk `gawk`'s dependencies, and commit `gawk`. Later, a declaration of `Depends: gawk` would look up `gawk` in the index, find it, and walk its dependencies again — producing duplicate edges and potentially re-queueing already-visited packages.

## What the main walk handles

The main BFS walk handles the complications that can be resolved without looking ahead at packages it has not yet visited. The two that need lookahead — RPM rich-deps and Alpine install-if triggers — defer to post-walk fixpoint passes (covered in [Conditional dependencies](conditional-dependencies.md)).

**Hard dependencies.** The declarations that say "this package needs that package." Every distro has them; they are the backbone of the graph. The walker looks up each dependency by name, resolves it to a real package, queues it.

**Cycles.** The visited set breaks them. A second encounter of a name that is already in the set produces no action.

**Alternatives.** A dependency can offer a choice: Debian's `Depends: pkg-a | pkg-b` says either will do. The walker delegates to the dependency mechanics layer, which scans the alternatives left to right and picks the first one that exists in the index and satisfies any version constraint. The same scan runs the same way every time, so the choice is deterministic.

**Soft dependencies.** Debian's `Recommends:` and `Suggests:`, Arch's `OptDepends`. The walker follows these only when the user opts in with `--with=recommends` or `--with=suggests`. Missing soft dependencies are silently skipped — they are best-effort by definition.

## What the main walk leaves for later

Two dependency forms cannot be evaluated during the main walk because they reference the install set itself, which is what the walk is building.

RPM **rich dependencies** can declare `(appstream-data if PackageKit)` — install `appstream-data`, but only if `PackageKit` is in the install set. During the main walk, the install set is incomplete, so the condition cannot be evaluated. These are stored during index parsing and deferred to a fixpoint phase that runs after the main walk finishes.

Alpine **install-if** triggers work the same way from the opposite direction. A package declares `i: gdk-pixbuf librsvg` — "install me when both `gdk-pixbuf` and `librsvg` are in the install set." The condition references the install set, so it is stored and deferred.

Both deferred forms are developed in [Conditional dependencies](conditional-dependencies.md).

## Provenance: recording why every package is in the set

The install command only needs the ordered list of package names. The analyze command needs more: for every package in the set, *why* is it there? Which package's dependency declaration pulled it in? Was it a hard requirement, a recommendation, or a conditional? If the declaration offered alternatives, which alternative was picked?

The resolver captures this metadata as **edges** during the walk. Every time a dependency declaration causes a package to be queued, the walker records one edge with:

- The parent package — the one whose declaration was being walked.
- The child package — the one that was queued.
- The dependency kind — hard, recommended, optional, or conditional.
- The picked alternative — for multi-alternative slots, which name was chosen (the declined alternatives are not recorded).
- The version constraint — the original constraint string from the index.

The install command discards the edges. The analyze command keeps them to classify each package, though its output is a flat, name-sorted list of entries — each carrying only its bare dependency names, not the full edge provenance (kind, picked alternative, constraint). Capturing provenance during the walk costs less than a second pass over the closure to reconstruct it, and it guarantees the provenance is recorded at the moment the decision was made — before later fixpoint phases modify the install set.

## The install order contract

The resolver's output is a flat list of package names in breadth-first visit order. The extraction stage iterates this list sequentially and unpacks each archive into the rootfs. The contract is *not* "dependencies first" — within a chain a package precedes its own dependencies. What the order guarantees is that the seeded packages, the base and Essential set among them, come first. The extraction stage does not reorder; it trusts the resolver.

The order matters because extraction is where directory symlinks are created. `usrmerge` (Debian) or `filesystem` (Arch) creates `/bin → usr/bin`. A package like `bash` writes `/bin/bash`, which must resolve through the symlink to `usr/bin/bash`. If `bash` were extracted before that symlink existed, its `/bin/bash` would land in a real `/bin` directory, and the symlink created later would shadow it — `/bin/bash` would no longer resolve to the file `bash` wrote. Seeding the layout-creating packages with the base/Essential set, ahead of the user's requests, keeps them early in the order and prevents this class of bug.

--8<-- "_glossary.md"
