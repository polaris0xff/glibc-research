---
tags:
  - appendix
  - versioning
---

# Version Comparison

Every package format carries upstream version strings that must be compared to decide *"is A newer than B?"*, *"does X satisfy `>= 1.2.3`?"*, and *"which candidate wins when several versions of the same package appear in the index?"* Each distro family defines its own comparison algorithm, and flatroot implements one per family: **dpkg** (Debian/Ubuntu), **libalpm `vercmp`** (Arch/CachyOS), **apk-tools** (Alpine), and **rpm** (the RPM family).

## Distro → algorithm mapping

| Distro family | Algorithm |
|---------------|-----------|
| Debian, Ubuntu | dpkg |
| Arch, CachyOS | libalpm `vercmp` (rpmvercmp-derived) |
| Alpine | apk-tools |
| CentOS, Fedora, AlmaLinux, Rocky | rpm |
| openSUSE | rpm |

Each family gets its own comparator because the schemes only look alike. Arch's `vercmp` consults the packaging release only when both sides carry one; apk's scheme has ranked `_suffix` words and a trailing letter that the others do not — differences that change the verdict on real version strings, so a single shared algorithm would disagree with the upstream tooling.

## Version structure

The dpkg and pacman schemes both decompose a version into up to three ordered parts:

```
[epoch:]upstream[-revision]
```

- **epoch** — optional integer before `:`. Dominates everything: `2:1.0` is always newer than `1:999.0`. Used when an upstream renumbering would otherwise break ordering. Epoch `0` is the default; it's omitted from normalized strings.
- **upstream** — the main version, set by the upstream project. The interesting part of any comparison.
- **revision** — packaging changes that don't correspond to a new upstream release (e.g., Debian's `-2` in `5.2.15-2+b10`). dpkg compares it whenever the upstreams are equal; pacman's `vercmp` consults it only when *both* sides carry one, so `1.0-1` ranks equal to `1.0`.

The rpm scheme keeps the epoch but walks the rest (`version-release`) as a single segment stream rather than splitting out a separate revision tiebreak. apk uses a different shape again — dotted numeric components, an optional trailing letter, a ranked `_suffix` chain, and a `-rN` revision.

A missing section on either side is treated as empty and compared as-if it were `0` or `""` — whichever is correct for the segment type.

## dpkg-vercmp

Defined by `dpkg --compare-versions` in Debian. Each part (upstream, revision) is compared by alternating non-digit and digit passes:

1. Scan leading non-digit characters. Compare lexicographically with these rules:
   - `~` (tilde) sorts *before* everything, including end-of-string. `1.0~beta < 1.0`.
   - Letters sort before non-letter punctuation.
   - Otherwise, ASCII order.
2. Scan leading digit characters. Compare numerically. Leading zeros ignored. `1.10 > 1.9`.
3. Alternate until both parts are exhausted.

Separators (`.`, `-`, `_`, etc.) fall into the non-digit pass and each carries its own ASCII weight. This means `1.0` and `1-0` compare *differently* under dpkg.

## rpm-vercmp

Defined by `rpmvercmp()` in rpmlib. Same overall alternating digit/non-digit shape as dpkg, with three key differences:

1. **Separators delimit segments; they are not folded into the value.** The comparator skips separators (`.`, `-`, `_`) to find segment boundaries and compares the alphanumeric segments — so `1.10` is the two segments `1`,`10`, which is *not* the same as the single segment `110`.
2. **Tildes still sort before everything.** Same behavior as dpkg.
3. **Alphanumeric runs split on transitions.** `1.0a1` splits into `1`, `0`, `a`, `1` and compares segment-by-segment.

## Constraint operators

Syntax varies by source format:

| Index format | Operators in source | Operators flatroot evaluates |
|--------------|---------------------|------------------------------|
| Debian index | `<<`, `<=`, `=`, `>=`, `>>` (doubled angle brackets avoid shell ambiguity) | All five |
| Arch index | `<`, `<=`, `=`, `>=`, `>` (concatenated, e.g. `glibc>=2.27`) | All five — the pacman comparator accepts the single-character `<`/`>` spelling |
| Alpine index | Concatenated to the name: `pkg>=1.0`. Also `~=` and `~` (compatible-release) | `<=`, `=`, `>=`, `<`, `>` all evaluate; `~=`/`~` are component-prefix (fuzzy) matches — `~2.1` accepts every `2.1*` release |
| RPM index | Flag names (`LT`, `LE`, `EQ`, `GE`, `GT`) as XML attributes | All five (translated to `<<`, `<=`, `=`, `>=`, `>>`); the rpm comparator additionally accepts bare `<` / `>` |

Arch and CachyOS dispatch through `PacmanVersionCompare` and Alpine through `ApkVersionCompare`; both accept the single-character `<`/`>` spelling their grammars use, so those constraints enforce normally. Only the original `DpkgVersionCompare`, used for Debian and Ubuntu, is restricted to the doubled-angle forms — but Debian's index always writes `<<`/`>>`, so nothing is lost there. The resolver sees a constraint string regardless of source; the comparator decides what to do with it.

## Why flatroot carries one comparator per family

A single "good enough" algorithm that works for most versions was rejected: real-world Debian, Arch, Alpine, and RPM version strings disagree in subtle cases (separator weighting, tilde handling, apk's ranked suffixes, pacman's conditional release), and silently picking the wrong comparison would make flatroot's resolver disagree with `apt`, `pacman`, `apk`, or `dnf` on the same input. Matching each family's native algorithm exactly is the only way to guarantee flatroot produces the same resolved set as the upstream package manager, which is the guarantee the per-distro resolver tests verify.

## Further reading

- [deb-version(7) man page](https://man7.org/linux/man-pages/man7/deb-version.7.html)
- [RPM docs — dependencies](https://rpm-software-management.github.io/rpm/manual/dependencies.html)
- [glibc version reference](glibc.md) — concrete distro/glibc pairings for compatibility planning.
- [Snapshot Pinning](snapshot-pinning.md) — how to freeze the upstream versions fed to the comparator.

--8<-- "_glossary.md"
