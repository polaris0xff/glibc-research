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

⛔ **The bar is NOT met, and part 2 is now not-met as a MEASURED RESULT rather
than as a gap.** That is a different and worse answer than "unmeasured", and it
is the honest one.

| part | state | why |
|---|---|---|
| **1. No known environment where it fails** | ⛔ **not met** | A known, unfixed failure is measured: `dlopen` of a **host** shared object ([`limitations.md`](limitations.md) §1). Tier 2 in [`design/tiers.md`](design/tiers.md) is the route and nothing of it is built. Two host **data** dependencies — terminfo and the TLS CA bundle — are also unsolved ([`limitations.md`](limitations.md) §3). |
| **2. Strictly better than the alternatives** | ⛔ **measured, and FALSE** | `experiments/60-versus-alternatives.sh` built the same program eight ways and ran every runnable one on the same 11 environments. **A static musl binary ties `pgb` on coverage and beats it on startup and size.** Details below. |

### Part 2: what the measurement actually says

The comparison the directive asked for exists now. All five named alternatives
were built — AppImage, Flatpak, snap, onelf and static musl — plus the two
controls. [`comparison.md`](comparison.md) carries the table;
`evidence/60-versus-alternatives/RESULT.txt` is the run.

| | `pgb` | static musl |
|---|---|---|
| ran correctly, 11 environments | 11 / 11 | 11 / 11 |
| loaded zero host shared objects | 11 / 11 | 11 / 11 |
| per exec | 980 µs | **160 µs** |
| artefact size | 2,097,824 B | **447,264 B** |

⛔ **So "strictly better and/or faster than every existing format and
technique" is false, and no wording fixes it.** `pgb` beat every *packaging
format* on this matrix — AppImage 2/11, onelf 3/11, Flatpak and snap 0/11
because no target ships anything to run them with — and it did not beat the
technique of simply building against musl.

⭐ **What the evidence does support** is the claim `pgb verify` already emits,
narrowed by one clause: *built at tier 1, ran correctly and loaded no host
object on these 11 named environments, **for a glibc build***. The last clause
is the whole value. Where a program can be rebuilt against musl, the
measurement says to do that instead; `pgb` is for where it cannot — a
dependency that will not cross to musl, a prebuilt glibc-linked archive,
glibc-specific behaviour, or `--wrap` onto objects compiled before this tool
existed.

**What would move part 2 to met.** Either a column where `pgb` beats static
musl and the formats at once, or the operator agreeing that "strictly better"
was the wrong bar and replacing it with the class-restricted claim above.
⛔ That second one is the operator's call and not an agent's: do not rewrite
this requirement to match the result.

### What is still unmeasured, and is not counted either way

- **Flatpak and snap at run time.** Both artefacts were built here; neither
  could be executed on this machine (`flatpak run` needs a D-Bus session bus
  and `dbus-daemon` cannot raise its fd limit with `cap_sys_resource` dropped;
  `snapd` needs systemd). ⚠ This does not affect their 0/11 coverage, which is
  decided by the targets, but their startup and memory cells stay dashes.
- **onelf in its preferred modes.** The chroot bed denies the user-namespace
  calls its memfd, FUSE and tmpfs modes need, so every onelf row is its last
  fallback. Its coverage result is unaffected — the failures are gconv, not
  delivery — but its startup figure is a worst case.
