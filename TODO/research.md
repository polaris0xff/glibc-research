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

**Source** follow-on from T-020 · **Category** research · **Priority** P2 · **Effort** M · **Status** open

**Problem.** T-012's planner needs a dependency graph. Building one from distro
metadata is strictly harder than reading one that already exists.

**Premise.** ⭐ Every nix derivation names its inputs exactly. Taking the graph
while refusing the store layout is the interesting move.

**Prove.** For one package, `pgb` prints the transitive input list obtained
from nix, and each entry marked link-statically / build-static / bundle.
