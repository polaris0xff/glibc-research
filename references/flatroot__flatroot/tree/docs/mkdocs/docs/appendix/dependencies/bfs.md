---
tags:
  - appendix
  - dependencies
  - resolver
---

# BFS Traversal

**Breadth-first search** (BFS) is a graph traversal algorithm that explores nodes level by level: all direct neighbors first, then neighbors-of-neighbors, and so on. flatroot's dependency resolver uses BFS to walk the package dependency graph starting from the user's requested packages (plus base and Essential packages).

## Why BFS instead of DFS

Depth-first search (DFS) would also visit every reachable package, but BFS has two properties flatroot relies on:

- **Predictable, stable output order** — the resolved list is level-ordered: direct dependencies of requested packages come first, then their dependencies, and so on. Re-running the same command with the same index produces the same ordering, which supports reproducibility guarantees and makes test fixtures stable.
- **Trivial cycle handling** — a visited set prevents infinite loops. DFS would need the same check, but BFS's queue-based shape makes it obvious: before enqueuing, check visited; if already seen, skip.

flatroot does not run a separate topological sort — but the BFS visit order is *not* dependency-first either. The walker commits a package right after queueing its dependency edges (step 8 enqueues the dependencies, step 9 appends the package itself), and because those dependencies go to the *back* of the queue, the package is committed before them: within a chain the order is parent-first (`app`, then `lib`, then `libc`). The install pipeline iterates that visit order verbatim. What keeps the directory-symlink-creating packages (like the one that makes `/bin` → `usr/bin`) ahead of the packages that write through them is not the chain order but the seed order — those packages sit in the base/Essential seed set, which is queued before the user's requests.

## Algorithm

1. Seed the queue with the user-requested packages plus base / Essential packages.
2. Pop the next name from the front of the queue.
3. Skip if already visited.
4. Look up the name in the index. If not found, consult the providers map (virtual packages). If still not found, return an error.
5. Mark as visited.
6. Check conflicts/breaks declared by packages already in the resolved set against this package. Warn on conflict; do not block.
7. Register this package's own conflicts/breaks for future checks.
8. Resolve each dependency (handling alternatives and version constraints) and enqueue the result.
9. Append this name to the output list.
10. Repeat from step 2 until the queue is empty.

A full walk-through of each step (with virtual-package resolution, alternative selection, and version-constraint checking) is in [Resolver — main expansion](../../explanation/dependency-resolution.md).

## Post-BFS fixpoint loops

Two additional passes run after the main BFS completes, because their inputs depend on the fully-resolved set:

- **RPM rich dependencies** with `if` / `unless` operators — see [Rich Dependencies](rich.md).
- **Alpine install-if triggers** — see [Install-If Triggers](install-if.md).

Each pass repeats until no new packages are added (fixpoint convergence). Termination is guaranteed because the resolved set only grows and the index is finite.

## Further reading

- [Resolver](../../explanation/dependency-resolution.md)
- [Rich Dependencies](rich.md)
- [Install-If Triggers](install-if.md)

--8<-- "_glossary.md"
