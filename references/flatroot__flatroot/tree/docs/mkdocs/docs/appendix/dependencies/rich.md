---
tags:
  - appendix
  - dependencies
  - resolver
---

# Rich Dependencies

Rich dependencies are boolean expressions used in RPM-based distributions (CentOS, Fedora, AlmaLinux, Rocky, openSUSE) to encode conditional, alternative, and conjunctive relationships between packages. They appear in the `<rpm:entry>` `name` attribute as parenthesized expressions.

## Operators

| Operator | Meaning | Example |
|----------|---------|---------|
| `if` | Install left side only when right side is present | `(appstream-data if PackageKit)` |
| `unless` | Install left side only when right side is absent | `(kernel-modules unless kernel-modules-extra)` |
| `or` | Either side satisfies the dependency | `(ansible-core or ansible)` |
| `and` | Both sides are required | `(feh and xrandr)` |
| `with` | Version range on the same package | `(pkg >= 1.0 with pkg < 2.0)` |
| `without` | Feature exclusion | `(pkg without feature)` |

Nesting is supported: `((A or B) if (C and D))` is valid.

## Conditional vs non-conditional

Rich deps are parsed into a tree structure during index parsing. The handling depends on whether the tree contains conditional operators (`if` / `unless`):

**Non-conditional** (`and`, `or`, `with`, `without`) — can be evaluated immediately and are flattened into plain dependencies. An `(A or B)` becomes a dependency with two alternatives. An `(A and B)` becomes two separate dependencies.

**Conditional** (`if`, `unless`) — cannot be evaluated during parsing because the condition depends on which packages end up in the resolved set, which isn't known until after the main resolution completes. These are stored as trees and evaluated in a post-resolution fixpoint loop.

## Fixpoint evaluation

After the main BFS resolution completes, a fixpoint loop processes all conditional rich deps:

1. For each package already in the resolved set, evaluate its conditional dependencies against the current resolved set
2. `(A if B)` — if B is resolved, install A and its transitive dependencies
3. `(A unless B)` — if B is NOT resolved, install A
4. Conditions can include version constraints, evaluated using the distro's version comparison
5. Repeat until no new packages are triggered

The loop repeats because installing a new package may activate further conditionals from other packages. Convergence is guaranteed because the resolved set only grows.

## Examples

```
(appstream-data if PackageKit)
```
If `PackageKit` is being installed, also install `appstream-data`. If not, skip it. This is how Fedora avoids pulling in optional metadata for package managers that aren't present.

```
(ansible-core or ansible)
```
Either package satisfies this dependency. The resolver picks whichever is already resolved, or the first available.

```
(glibc-gconv-extra(x86-64) = 2.41-16.fc42 if glibc-gconv-extra(x86-64))
```
A version-constrained conditional — install the specific version of `glibc-gconv-extra` only if the package is already being installed.

--8<-- "_glossary.md"
