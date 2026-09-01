# Requirements — the operator's acceptance bar

⛔ **This page is binding and it is not a summary of the project's state.** It
records what the operator requires, verbatim, and tracks — separately, below —
how much of it has been discharged. The requirement text is **not** to be
softened, deleted, or marked satisfied by any agent; only the *status* section
moves, and only when a measurement moves it.

> Relocated here from the top of [`AGENTS.md`](AGENTS.md), where it was
> originally inserted for visibility with a note that a later agent should give
> it its own file and link it from §1. That is this file. Nothing in the
> requirement was altered in the move.

---

# ⛔ HARD REQUIREMENT (operator, binding, NOT yet met)

> **pgb must produce a binary that works _everywhere_ — or, failing that, one
> that is strictly better and/or faster than every existing format and
> technique.**

This is the project's acceptance bar. It is **not** met today: the current
class is "programs that do not need to load host plugins"
([`AGENTS.md`](AGENTS.md) §7), which is broad but not everything.

**How to hold this bar without lying about it.** "Everywhere" cannot be
verified — no matrix enumerates Linux, the kernel is never abstracted, and the
CPU baseline is a build-time choice. So the directive is discharged in two
parts, and **both** are required:

1. **No known environment where it fails.** Every failure found is either
   fixed or written into [`limitations.md`](limitations.md) with the
   measurement. The matrix grows over time ([`AGENTS.md`](AGENTS.md) §13); a
   failure that is known and unfixed means the bar is not met, and the status
   must say so.
2. **Strictly better than the alternatives, measured head to head.** This
   comparison was, when this directive was written, the single largest gap
   against it. `comparison.md` had dashes for every non-pgb row because
   nothing else was ever run. Discharging this means actually building the
   same program as an AppImage, a Flatpak, a snap, an onelf bundle and a
   static-musl binary, and measuring all of them on the same matrix for:
   coverage (environments it runs on), startup, memory, size, and host
   dependencies pulled in. Until that table has numbers instead of dashes,
   "better than existing techniques" is an assertion.

⭐ **The honest public claim, until then**, is the falsifiable one `pgb verify`
already emits: *built at tier N, ran correctly and loaded no host object on
these N named environments, and here is the command that re-checks it on
yours.* Use that in the README and anywhere else a claim is made. Do not write
"universal" or "works everywhere" into any document as a statement of fact
until part 1 and part 2 above are both discharged.

**Work this implies, in addition to [`AGENTS.md`](AGENTS.md) §13:** the tier
plan in [`design/tiers.md`](design/tiers.md) is the route to part 1 (it is what
brings the host-plugin class in scope), and a new head-to-head benchmark
experiment — `experiments/60-versus-alternatives.sh` — is part 2.

---

## Status against the bar

⛔ **The bar is NOT met.** The two parts are tracked separately because they
fail for different reasons and are discharged by different work.

| part | state | what would move it |
|---|---|---|
| **1. No known environment where it fails** | ⛔ **not met** | A known, unfixed failure exists and is measured: `dlopen` of a **host** shared object ([`limitations.md`](limitations.md) §1). Tier 2 in [`design/tiers.md`](design/tiers.md) is the route; nothing of it is built. Two host **data** dependencies — terminfo and the TLS CA bundle — are also unsolved ([`limitations.md`](limitations.md) §3). |
| **2. Strictly better than the alternatives** | ⚠ **partially discharged** | `experiments/60-versus-alternatives.sh` now exists and runs. See below for exactly which arms carry numbers and which do not. |

### Part 2, in detail

⚠ **"Partially" is the accurate word and it is not a softening.** The
experiment exists, it runs the same source through several delivery techniques
on the same 11-environment matrix, and [`comparison.md`](comparison.md) now
carries measurements where it carried dashes. What it does **not** yet carry is
every arm the requirement names.

Read [`comparison.md`](comparison.md) for the table and
`evidence/60-versus-alternatives/RESULT.txt` for the run that produced it. The
arms that could not be built or run here say so, with the reason and the probe
that establishes it — a stated blocker is a result; a dash is not.

⛔ **Until every named arm carries numbers, the sentence "strictly better than
every existing format and technique" is still an assertion.** What the evidence
supports is narrower and is what the documents say instead.
