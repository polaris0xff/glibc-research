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

## ⭐ AMENDMENT — part 2 replaced, operator ruling, 2026-09-01b

⛔ **The requirement above keeps its text; part 2 of the two-part discharge
below is REPLACED.** The ruling was taken at the start of the session of
2026-09-01b, in answer to the open question the previous session recorded, and
it is quoted verbatim because the reason matters as much as the decision:

> *"replace with per part claim, also anylinux is a bundle, our primary goal
> is still a static glibc binary that has none of the issues"*

⭐ **What that changes.** "Strictly better than every existing format and
technique" measured `pgb` against `Anylinux-AppImages`, and the two are not
the same kind of thing: one is a **bundle** — it mounts or extracts a small
distribution — and `pgb` is a toolchain whose output is one ordinary ELF. A
comparison that scores them on one axis asks the wrong question, which is the
same point [`design/toolchain.md`](design/toolchain.md) makes about treating
`pgb` as a format.

⛔ **Part 2 is therefore no longer a comparison against bundles.** It is the
per-part claim, and the goal it names is the operator's own: *a static glibc
binary that has none of the issues.* The issues are enumerable and each is
either closed or open, below.

⚠ **Part 1 is UNCHANGED and still binding**, and it is now the part that
carries the whole bar. The head-to-head numbers stay in
[`comparison.md`](comparison.md) as measurement; they are no longer the test.

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
2. ⭐ **A static glibc binary with none of the issues** — the per-part claim,
   as amended above. `gcc -static` against glibc is not self-contained, and
   the ways it is not are **enumerable**. Discharging this means every one of
   them is closed, on the matrix, with the measurement:

   | | issue | state |
   |---|---|---|
   | NSS | host modules dlopen'd, second libc in the process | ✅ **closed** — `__nss_configure_lookup`, 11/11 |
   | gconv / iconv | 11 of 12 encodings unavailable, SIGFPE on 3 | ✅ **closed** — `--wrap` onto static libiconv, 11/11 |
   | locale | `ANSI_X3.4-1968` on every musl host | ✅ **closed**, opt-in `--embed-locale`, 11/11 |
   | networking / DNS | `getaddrinfo` via host NSS | ✅ **closed** — POC 30 resolves and does real TLS on 11/11 |
   | own plugins | a program's own `dlopen` needs the host loader | ✅ **closed** — `--wrap-dlopen`, 11/11 |
   | C++ unwinding | no `PT_GNU_EH_FRAME` on any static link | ✅ **closed** — T-018 |
   | CA bundle | no compiled-in trust store; one path works on 5 of 11 | ✅ **closed** — opt-in `--embed-cacert`; POC 30 verifies real TLS on **11/11** with the harness's own CA variables unset. T-032 |
   | terminfo | host terminal database | ✅ **closed** — opt-in `--embed-terminfo`; POC 20's `setupterm(xterm-256color)` succeeds on **11/11** with `TERMINFO`/`TERMINFO_DIRS` unset. T-032 |
   | **host plugins** | `dlopen` of a host `.so` is host-dependent | ⛔ **open** — T-033, and see part 1. ⭐ **The last one.** |

   ⚠ **The old text of this part is not deleted, it is superseded**, and the
   measurement it asked for was carried out: `experiments/60-`, `61-` and
   `62-` built the same program as an AppImage, a Flatpak, a snap, an onelf
   bundle and a static-musl binary and ran every one on the same eleven
   environments. That table is in [`comparison.md`](comparison.md) and stays
   there as evidence. What changed is that it is no longer the acceptance
   test — `history/corrections.md` C13.

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

⛔ **The bar is NOT met**, and both parts are now not-met as measured results
rather than as unmeasured gaps.

| part | state | why |
|---|---|---|
| **1. No known environment where it fails** | ⛔ **not met** | One measured, unfixed failure, and it is now the **only** one: `dlopen` of a **host** shared object ([`limitations.md`](limitations.md) §1). ⭐ **Four** routes to it, none exhausted — `AGENTS.md` §7. Route D is best-evidenced: `experiments/73-` measures 90.8%–99.3% of every glibc-versioned import of 6,007 real host shared objects as already definable by the pinned static glibc, with **zero** unexplained residue. T-033. ⭐ The two host **data** dependencies that used to sit on this row, terminfo and the TLS CA bundle, are **closed** as of 2026-09-01d. |
| **2. A static glibc binary with none of the issues** | ⛔ **not met, and now countable** | ⭐ **Eight of nine** enumerated issues are closed on all eleven environments — it was six of nine before 2026-09-01d. **One** is open: host plugins. The table above is the whole of it; there is no unenumerated remainder, so the distance to the bar is one named problem with four untried routes rather than an unknown quantity. |

### The head-to-head, which is now evidence rather than the test

⚠ **Read this as background, not as the acceptance criterion.** The operator's
ruling of 2026-09-01b replaced part 2, on the ground that *anylinux is a
bundle* and the goal is a static glibc binary with none of the issues. What
follows is the measurement that was taken while part 2 was a comparison; it
stands as a result and no longer decides anything.

The comparison the original directive asked for exists: AppImage, Flatpak,
snap, onelf and static musl were all built, plus the two controls, and the
AppImage arm was then rebuilt against `Anylinux-AppImages` because the vanilla
one is not the competitor. [`comparison.md`](comparison.md) carries the table.

| | `pgb` | anylinux AppImage | static musl |
|---|---|---|---|
| ran correctly, 11 environments | 11 / 11 | 11 / 11 | 11 / 11 |
| loaded zero host objects in the payload | 11 / 11 | 11 / 11 | 11 / 11 |
| malloc, 4 threads, on Alpine | **4.3 ns** | **3.7 ns** | 592 ns |
| artefact size | **2.1 MB** | 3.7 MB | 447 KB |
| target does nothing but execute it | ✅ | ⛔ mounts or extracts | ✅ |
| serves a large dynamic dependency graph today | ⛔ not yet | ✅ | ⛔ |

What it says: `pgb` is not beaten on portability or on speed by anything
measured. It is **tied** with the anylinux AppImage on both, ahead of it on
artefact size and on shape — one ELF, nothing mounted, nothing written — and
**behind it on reach**: bundling serves software with a dependency graph
static linking has not yet been pushed hard enough to absorb. ⭐ That last one
is work, not a verdict — `AGENTS.md` §7 now has **four** routes, and
`experiments/73-` measures the newest one's demand at 90.8%–99.3% already met.

⭐ **What the evidence does support**, and it is a real claim:

> Built at tier 1, ran correctly and loaded no host shared object on these 11
> named environments, at glibc's throughput including on musl hosts, as one
> ordinary ELF that mounts nothing and writes nothing — for programs that can
> be statically linked today.

**What would move part 2 to met**, under the amended text: the open rows of the
issues table close, each on all eleven environments with the measurement
recorded. ⭐ **T-032 closed two of them on 2026-09-01d** — the CA bundle and
terminfo — so **one** row is left: **T-033, host plugins**.
⛔ **Eight of nine are closed and one is not, so this is a countable deficit
and not a judgement.** Do not soften the one that remains: it is the hardest
of the nine, and being last does not make it small.

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
