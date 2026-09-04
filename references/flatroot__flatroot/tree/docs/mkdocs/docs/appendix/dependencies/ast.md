---
tags:
  - appendix
  - dependencies
  - resolver
---

# Rich-Dep AST

An **abstract syntax tree** (AST) is a tree-shaped data structure representing the parsed form of an expression. Leaves hold operands; internal nodes hold operators. flatroot uses an AST to store RPM rich dependencies whose truth value cannot be known at parse time.

## Why an AST is needed

RPM rich deps are boolean expressions embedded in the package index:

```
(appstream-data if PackageKit)
((feh and xrandr) if Xserver)
(kernel-modules unless kernel-modules-extra)
```

Expressions containing only `and`, `or`, `with`, `without` can be evaluated at parse time — they reduce to plain dependencies or plain alternative groups and are flattened directly into the `dependencies` table. No tree is kept.

Expressions containing `if` or `unless` cannot be evaluated at parse time: the condition references packages whose presence in the resolved set is only known after the main BFS completes. These expressions are preserved in tree form until the post-BFS fixpoint pass evaluates them against the fully-resolved set.

See [Rich Dependencies](rich.md) for the evaluation semantics.

## Node types

| Node | Meaning |
|------|---------|
| `Pkg { name, version_constraint }` | Leaf — a package reference, optionally with a version constraint |
| `And { left, right }` | Conjunction — both sides must hold |
| `Or { left, right }` | Disjunction — either side suffices |
| `If { payload, condition }` | Install `payload` only when `condition` is in the resolved set |
| `Unless { payload, condition }` | Install `payload` only when `condition` is NOT in the resolved set |
| `With { left, right }` | Same package, multiple constraints (version range) |
| `Without { left, right }` | Feature exclusion — treated as a plain dependency on `left` |

Nesting is arbitrary: `((A or B) if (C and D))` is a valid tree.

## Storage

The AST is JSON-serialized and stored in the `rich_deps.ast` column of the package index database. See [Database Reference — rich_deps](../../reference/database.md#rich_deps).

Example serialization for `(appstream-data if PackageKit)`:

```json
{"If":{"payload":{"Pkg":{"name":"appstream-data","version_constraint":null}},"condition":{"Pkg":{"name":"PackageKit","version_constraint":null}}}}
```

JSON was chosen over a binary format because the `rich_deps` table is small (thousands of rows on RPM distros, zero on others), human-readable serialization makes `flatroot query` output legible without a custom decoder, and SQLite handles TEXT columns natively — a BLOB codec would be strictly more code for no benefit at this scale.

## Further reading

- [Rich Dependencies](rich.md) — evaluation rules for each operator node.
- [Database Reference — rich_deps table](../../reference/database.md#rich_deps)
- [BFS Traversal](bfs.md) — the main resolution pass whose output the rich-dep fixpoint reads.

--8<-- "_glossary.md"
