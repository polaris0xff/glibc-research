---
tags:
  - explanation
  - resolver
  - dependencies
---

# Conditional dependencies

The main BFS walk described in [Dependency resolution](dependency-resolution.md) handles every dependency that can be evaluated from the index alone. A package declares it needs another package; the resolver looks it up, checks any version constraint, and queues it. The condition is always true: the dependency is unconditional.

Two distro families publish dependency declarations whose truth depends on what else is being installed. RPM's rich dependency syntax can express `(appstream-data if PackageKit)` — install `appstream-data`, but only if `PackageKit` is also in the install set. Alpine's install-if syntax can express `i: gdk-pixbuf librsvg` — install this package when both `gdk-pixbuf` and `librsvg` are in the install set. Both forms reference the install set itself, which is what the main walk is busy producing. They cannot be evaluated until the walk finishes.

This page develops why these declarations need fixpoint loops, how transactional sub-walks keep the install set consistent when an addition fails, and why the two forms use different failure semantics.

## The evaluation problem

During the main walk, the install set grows with every iteration. A conditional declaration encountered early in the walk might reference a package that has not been visited yet — the condition is false at that moment, but it will be true by the time the walk finishes. Evaluating the conditional at the moment of encounter would produce the wrong answer.

The alternative — evaluating every conditional after the main walk finishes, once — also produces wrong answers when conditionals chain. Suppose:

- Package A declares `(B if C)` — install B when C is in the set.
- Package B declares `(D if E)` — install D when E is in the set.
- The main walk pulls in A, C, and E.
- A single post-walk pass evaluates `(B if C)`. C is in the set, so B is folded in.
- The same pass would need to evaluate `(D if E)`, but that conditional belongs to B, which was not in the set when the pass started. The conditional is never considered, and D is missing from the final install set despite every condition for it being satisfied.

The fix is to loop. Evaluate every conditional against the current install set. Fold in payloads whose conditions hold. Then evaluate every conditional again — packages added in the previous iteration might now satisfy conditions that earlier failed. Stop when a full pass adds nothing.

This is **fixpoint convergence**: a function applied repeatedly until applying it again produces no change. The install set only grows; once it stops growing, no new conditional can fire.

## Two forms, two fixpoints

RPM rich dependencies and Alpine install-if triggers are both conditional, both run after the main walk, and both use fixpoint loops. They are separate phases rather than a combined pass because they differ in what they express and how they handle failure.

**RPM rich dependencies** use a small set of boolean operators — `and`, `or`, `with`, `without`, `if`, and `unless`. The `if` and `unless` operators are the conditional ones; the rest compile into regular hard dependencies during index parsing and are handled by the main walk. A rich-dep conditional says "this payload must be installed when this condition is met." If the condition is met and the payload's own hard dependencies cannot be resolved, the index is internally inconsistent — the distro published a conditional that cannot be satisfied. The resolver treats this as a fatal error and aborts the install. This is the *strict* failure mode.

**Alpine install-if** triggers are conjunctive: all trigger names must be present, not just one. A package with `i: gdk-pixbuf librsvg` fires only when both are in the set. The semantics are advisory — if a triggered package's own dependencies cannot be resolved, the trigger is dropped from the install set with a warning, and the install continues. This is the soft* failure mode. The reasoning is that install-if is common in Alpine's rolling-release model, where a shared library soname bump can temporarily make a satellite package unresolvable. Aborting the install would block every user until the distro rebuilds the satellite.

The two fixpoints run sequentially: rich-dep first, install-if second. The order matters because a rich-dep payload folded in by the first fixpoint might satisfy an install-if trigger in the second.

## Transactional sub-walks

Both fixpoints share a problem. When a conditional fires, the resolver must walk the payload's hard dependencies and add them to the install set. That walk can fail partway through — a hard dependency that cannot be resolved, a version constraint that cannot be satisfied. If the walk has already partially modified the install set when the failure is discovered, the set is left in an inconsistent state: half the payload's dependencies are present, half are not, and the next conditional evaluation runs against corrupted state.

The solution is a **transactional sub-walk**: a mini resolver run that either commits its entire result to the install set or leaves the set exactly as it found it. Before the sub-walk starts, it snapshots the install set's state — every package committed so far, the order they were committed in, the edges recorded so far. All mutations happen on the live state. If the sub-walk completes without finding an unresolvable dependency, the snapshot is discarded — the mutated state is the committed result.

If the sub-walk hits an unresolvable dependency, the snapshot is moved back, replacing whatever the sub-walk committed. The state is now exactly as it was before the sub-walk started. The difference between strict and soft is what happens after the rollback: strict propagates the failure as a fatal error; soft returns *false* and lets the caller continue with the next trigger.

## Why not a single unified conditional system

A natural question is whether the two conditional forms could be unified — parse both into the same internal representation and run one fixpoint loop over the combined set.

They could be, but the failure semantics would have to be weakened to the lowest common denominator. Unifying them into a single loop with soft failure would mean RPM rich-dep failures — which indicate a broken index — would be silently swallowed. Unifying with strict failure would mean Alpine install-if failures — which are routine in a rolling distro — would abort installs. Keeping them separate lets each form keep the failure semantics that match its distro's contract.

The two phases share the transactional sub-walk primitive. The snapshot-and-restore mechanism is identical in both. What differs is how the caller interprets the return value, and that difference is one `if` statement in each fixpoint's loop body.

--8<-- "_glossary.md"
