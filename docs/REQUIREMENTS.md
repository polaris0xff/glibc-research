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

The comparison the directive asked for exists now: AppImage, Flatpak, snap,
onelf and static musl were all built, plus the two controls, and the AppImage
arm was then rebuilt against `Anylinux-AppImages` because the vanilla one is
not the competitor. [`comparison.md`](comparison.md) carries the table.

⚠ **A previous revision of this section answered this from the wrong
measurement** — startup and size, where musl wins by construction — and
concluded that static musl beat `pgb`. `history/corrections.md` C7 has the
error. The corrected picture:

| | `pgb` | anylinux AppImage | static musl |
|---|---|---|---|
| ran correctly, 11 environments | 11 / 11 | 11 / 11 | 11 / 11 |
| loaded zero host objects in the payload | 11 / 11 | 11 / 11 | 11 / 11 |
| malloc, 4 threads, on Alpine | **4.3 ns** | **3.7 ns** | 592 ns |
| artefact size | **2.1 MB** | 3.7 MB | 447 KB |
| target does nothing but execute it | ✅ | ⛔ mounts or extracts | ✅ |
| serves programs that cannot be linked statically | ⛔ | ✅ | ⛔ |

⛔ **"Strictly better and/or faster than every existing format and technique"
is still false, but for a completely different reason than the one recorded
before.** `pgb` is not beaten on portability or on speed by anything measured.
It is **tied** with the anylinux AppImage on both, ahead of it on artefact size
and on shape — one ELF, nothing mounted, nothing written — and **behind it on
reach**: bundling serves software that cannot be statically linked at all, and
static linking never will.

⭐ **What the evidence does support**, and it is a real claim:

> Built at tier 1, ran correctly and loaded no host shared object on these 11
> named environments, at glibc's throughput including on musl hosts, as one
> ordinary ELF that mounts nothing and writes nothing — for programs that can
> be statically linked.

**What would move part 2 to met.** Either `pgb` grows to serve the
bundling class as well (that is [`design/tiers.md`](design/tiers.md), and
⭐ the anylinux stack is now the reference implementation to measure against
rather than a rival to beat), or the operator agrees that "strictly better than
every existing format" was the wrong bar and replaces it with the claim above.
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
