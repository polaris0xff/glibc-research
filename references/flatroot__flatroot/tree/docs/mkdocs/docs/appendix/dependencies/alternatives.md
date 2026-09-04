---
tags:
  - appendix
  - dependencies
  - resolver
---

# Alternative Dependencies

An alternative dependency can be satisfied by any one of several packages. The resolver tries each option in order and picks the first that exists and satisfies any version constraint.

## Explicit alternatives (Debian / Ubuntu)

Debian and Ubuntu use the pipe (`|`) syntax in dependency declarations:

```
Depends: debconf (>= 0.5) | cdebconf
```

This means "debconf at version 0.5 or higher, OR cdebconf." The resolver tries `debconf` first. If it exists and satisfies the version constraint, it's selected. If not, `cdebconf` is tried.

Multiple alternatives can be chained:

```
Depends: awk | mawk | gawk | original-awk
```

## Implicit alternatives (Arch / Alpine / RPM)

Arch, Alpine, and RPM don't have alternative syntax in their dependency declarations — each dependency names exactly one package. Alternatives arise implicitly through virtual packages: when multiple packages provide the same virtual name, they form an alternative group.

For example, if packages A, B, and C all provide `libGL.so.1`, a dependency on `libGL.so.1` is effectively an alternative between A, B, and C. The resolver picks the first provider (alphabetically sorted) that satisfies any version constraint.

## Resolution order

For each alternative (whether explicit or implicit):

1. Get all candidates — the real package name, or all providers of the virtual
2. For each candidate, check the version constraint against the provides version (for virtual deps) or the package version (for direct deps)
3. Accept the first candidate that satisfies

If no candidate of any alternative satisfies the constraint, the dependency is unresolvable:

- For required dependencies (`Depends`, `Pre-Depends`) — resolution fails with an error
- For optional dependencies (`Recommends`, `Suggests`) — silently skipped

--8<-- "_glossary.md"
