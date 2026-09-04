---
tags:
  - appendix
  - dependencies
  - resolver
---

# Install-If Triggers

Install-if is a conditional installation mechanism specific to Alpine Linux. A package can declare trigger conditions that cause it to be automatically installed when all conditions are met.

## Format

The `i:` field in the APKINDEX lists one or more trigger package names:

```
P:gdk-pixbuf-loader-svg
V:2.58.0-r0
D:gdk-pixbuf librsvg
i:gdk-pixbuf librsvg
```

This means: install `gdk-pixbuf-loader-svg` automatically if **both** `gdk-pixbuf` and `librsvg` are present in the resolved set. The conditions use AND semantics — every trigger must be satisfied.

## Use cases

Alpine uses install-if for packages that bridge two libraries:

- **Image format loaders** — `gdk-pixbuf-loader-svg` is installed when both the pixbuf framework and the SVG library are present
- **GTK input methods** — installed when both GTK and the input method library are present
- **Font rendering backends** — installed when both fontconfig and the specific renderer are present

Without install-if, these bridge packages would either need to be hard dependencies of one side (pulling in the other unnecessarily) or manually requested by the user.

## Evaluation

Install-if triggers are evaluated in a fixpoint loop after both the main BFS resolution and the RPM rich dependency evaluation complete:

1. Scan every package in the index that has install-if conditions
2. For each, check if ALL conditions are satisfied by the current resolved set
3. If all conditions are met and the package isn't already installed, add it and resolve its hard dependencies
4. Repeat until no new packages are triggered

The loop runs last because:
- The main BFS must complete first so the resolved set is populated
- Rich dependency evaluation may add packages that satisfy install-if conditions
- Installing an install-if triggered package may satisfy conditions for another triggered package

## Version constraints

Install-if conditions can include version constraints:

```
i:libglycin~2.1.0 gtk4~4.14
```

FlatRoot strips the version constraints during parsing and checks only whether the named package is present. This is consistent with Alpine's behavior — the version constraint is a hint, not a hard requirement for triggering.

--8<-- "_glossary.md"
