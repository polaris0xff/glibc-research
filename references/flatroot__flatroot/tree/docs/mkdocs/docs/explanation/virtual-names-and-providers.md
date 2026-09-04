---
tags:
  - explanation
  - resolver
  - dependencies
  - path-index
---

# Virtual names and providers

The dependency resolver described in the previous page looks up names in the index and queues the packages it finds. The assumption is that every name in a dependency declaration corresponds to a row in the package table. That assumption is wrong often enough that the resolver needs a dedicated translation layer between the name that was declared and the package that will be installed.

A Debian package can declare `Depends: awk`. No package in the Debian index is called `awk`. Three packages — `gawk`, `mawk`, `original-awk` — each declare that they *provide* `awk`. The name `awk` is virtual: it exists only as a claim made by real packages. The resolver must map `awk` to one of the providers, deterministically, the same way every run.

An RPM package can declare `Requires: /bin/sh`. The string `/bin/sh` is not a package name; it is an absolute path. The package that owns `/bin/sh` — typically `bash` or `dash` — must be found through a path-to-package index. The resolver must map the path to its owning package.

This page develops the three-tier lookup that handles every name shape a dependency declaration can take, why provider selection must be deterministic, and why the version string used to check a constraint is not always the package's own version.

## The three tiers

When the resolver encounters a name in a dependency declaration, it does not know what kind of name it is. The name could be a real package, a virtual name, or a file path. The resolver tries three sources in fixed order. The first hit wins; later tiers are not consulted.

**Tier 1: real package.** The name appears as the primary name of a row in the index. This is the common case. `bash` resolves to `bash`. The lookup is one indexed query against the package table.

**Tier 2: virtual package.** The name does not appear as a primary name, but does appear in some package's provides list. Several packages might provide it. The resolver ranks the providers by the distribution's `Priority` ordinal first and by name only as a tiebreak, returning the top one. `awk` resolves to `mawk` when both `mawk` and `gawk` provide it, because `mawk` carries the more-preferred priority — even though `gawk` sorts first alphabetically.

**Tier 3: file path.** The name starts with `/`. The resolver consults a path-to-package index — a structure populated during index ingestion that maps absolute paths to owning packages — and returns whichever package owns the path. `/bin/sh` resolves to `dash` on Debian, `bash` on most RPM distros.

If none of the tiers hits, the resolver returns *no resolution*. What the caller does with that depends on context: a hard dependency that cannot be resolved is a fatal error; a soft dependency is silently skipped; a conditional payload is dropped.

## How a provider is chosen

Tier 2 leaves a real choice to make: which provider, when several packages provide the same virtual name? `awk` is provided by `gawk`, `mawk`, and `original-awk`. `sh` is provided by `bash`, `dash`, and `mksh`. The resolver ranks the candidates by the distribution's own `Priority` ordinal first, falling back to the name only to break a tie.

The ranking is both deterministic and faithful to the distribution. It is deterministic because two runs of the same install command against the same index always sort the same way, so the rootfs contents never diverge. And it is faithful because Debian ingests its `Priority:` field as that ordinal: a `required` provider outranks an `optional` one. Debian prefers `mawk` over `gawk` in fresh installs, and because `mawk` carries the more-preferred priority the resolver picks `mawk` too — not the alphabetically-first `gawk`. Where no priority distinguishes the candidates, the name is the final, stable tiebreak.

The rule is enforced at the database query level. Both provider queries order by `(p.priority IS NULL), p.priority, p.name` — priority-bearing rows first, lowest ordinal preferred, name as the tiebreak — so the sort lives in one place rather than being duplicated at every name-lookup site. Users who want a different provider can still name it explicitly on the command line; it is installed alongside the chosen one, and both coexist.

## Two operations: resolve and enumerate

Different callers need different shapes of answer from the name translation layer.

The main BFS loop needs a single answer: "what real package does this name refer to?" It uses the **resolve** operation, which returns the top-ranked hit from any tier (priority-then-name for virtual names), or *None* if nothing matches.

The dependency picker, when checking a versioned dependency, needs every candidate so it can scan them against the version constraint. A single answer is not enough — the top-ranked provider might not satisfy the constraint, but the next one might. The picker uses the **enumerate** operation, which returns every candidate: the real package if the name is a primary name, every provider in priority-then-name order if the name is virtual, the path-owner if the name is path-shaped.

Both operations share the same tier order. *Resolve* is the first item enumerate* would produce. They are separate because forcing every caller to discard a list it does not need would be awkward, and forcing the picker to call resolve repeatedly — checking one candidate, failing, calling resolve again for the next — would be inefficient and fragile.

## The provides version is not the package version

When the picker checks a version constraint against a virtual dependency, it must compare the constraint against the right version string. For a real package, the right string is the package's own version: `Depends: libc6 (>= 2.36)` compares `2.36` against the `libc6` package's version. For a virtual dependency, the package's version is often the wrong string.

A package called `gcc-libs` at package version `15.2.1` might declare it provides the virtual `libgomp.so` at version `1-64`. A dependency on `libgomp.so=1-64` must be checked against `1-64`, not against `15.2.1`. Asking the version comparator to evaluate `15.2.1` satisfies `=1-64` produces nonsense — the dependency's version space and the package's are unrelated. The version comparator would reject a satisfiable dependency, and the resolver would fail to install a package the index says is available.

The picker therefore consults the *provides version* first: what version did the provider declare for this virtual name? If the provides slot carried a version — Arch's `%PROVIDES%` entries can include `=1-64` suffixes, Debian's `Provides:` fields can include `(= 1.0)` — that version is what gets compared against the constraint. The package version is the fallback, used only when the provides slot was unversioned.

This logic has to live in the picker, not in the version comparator. The comparator just compares two strings; the picker decides *which* string to feed it. This is what makes versioned virtual dependencies work across an index where every soname is a versioned provides — Arch's primary use of the provides mechanism.

## Path dependencies and the path index

RPM packages can declare dependencies on file paths: `Requires: /usr/bin/python3`. The name starts with `/`, which triggers Tier 3.

The path-to-package index is not part of the SQLite database. It is a separate binary file (`.pathidx`) stored alongside the database, built during index ingestion from RPM `filelists.xml`, Debian/Ubuntu `Contents-<arch>.gz`, and Arch/CachyOS `.files.tar.gz`. The separation exists because the data is large — RPM `filelists.xml` ships millions of file entries across tens of thousands of packages — and a custom binary format with sorted string pools and binary search is an order of magnitude more compact than a SQLite table with per-row overhead.

The path index answers exactly one question for the resolver: "which package owns this absolute path?" The resolver asks it when a dependency name starts with `/`, and the answer is either a package name or *no owner*. The index itself exposes more — exact-path `query`, filename glob `query_glob`, and full-path glob `query_glob_path` (the latter two back the `--type library` and `--type path` search spaces) — but the resolver uses only the exact-path question.

--8<-- "_glossary.md"
