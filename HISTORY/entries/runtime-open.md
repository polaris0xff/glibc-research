# HISTORY/entries/runtime-open.md — retired DETAIL of runtime entries that are STILL OPEN

⚠ **These entries are open. This file is not the entry** — the entry is in
[`../../TODO/runtime.md`](../../TODO/runtime.md) and is deliberately short. What is
here is the long-form record each one accumulated: the measurements, the
corrections, the routes costed and the routes killed.

⛔ **Read the TODO entry first.** Come here when you need to know WHY it says
what it says, or before re-running something to check whether it was already
run. ⭐ A number quoted in the TODO entry was derived here.

⚠ The headings below deliberately do NOT use the `## T-NNN — ` form, because
that form is what `sh TODO/check.sh` treats as *the* entry, and there must be
exactly one of those per id.

---

## T-031 · retired detail — Port cross-libc-dlopen's full rewrite, not one function

**Source** `docs/limitations.md` §1 · **Category** runtime · **Priority** P2 · **Effort** L · **Status** open

**Problem.** `experiments/50-` ported `cld_strip_versions()` — one function of
roughly forty from a 2015-line file — and found no effect. The two steps it did
not port are the ones aimed at the failure it observed.

**Premise.** ⚠ The untested steps drop the `DT_NEEDED` edges that pull a
foreign libc in (`cross-libc-dlopen.c:1857`) and rebind the remaining imports.
Upstream's `docs/limits.md` says the static-glibc case is one where `dlopen`
*works* and labels all three static cases unverified.

**Approach.** `CROSS_LIBC_DLOPEN_DRYRUN` makes the rewrite path testable with
no GPU and no Alpine — cheaper than the instrument `50-` built.

**Prove.** `experiments/51-*.sh` re-runs `50-`'s two arms plus a third carrying
the full rewrite, and the table shows what changed on each of 11.

### ⛔ READ THIS BEFORE PORTING ANYTHING — the reference moved, 2026-09-03

⚠ **The vendored tree was pinned at `1cecf50e` (fetched 2026-09-01), which is
BEFORE `pkgforge-dev/cross-libc-dlopen` PR 30 merged on 2026-09-02.** A port
taken from that revision would have inherited a fixed bug. It is now re-mined
at **`793f3f3f`**, PR 30's merge commit; `PROVENANCE.md` names it.

⛔ **What PR 30 fixed, because it is the shape this tree keeps paying for.** On
a host with `mesa-gl` but neither `mesa-egl` nor `mesa-gles`, `gles-fwd.so`
found no forwarding target and kept **358 entry points as zero-returning
stubs**. Preloaded, it won interposition and those stubs **shadowed** the real
`glGetString` the process could still serve: a real GL context, a NULL version
string, a black window (issue #28). The repair is one pass in
`glfwd_fill_addr` — `dlsym(RTLD_NEXT, name)` once per name when the shim has no
target, so a name something behind the shim can still serve is served from
there; a miss keeps the stub.

⛔ **WE ARE NOT AFFECTED BY THAT BUG and no document here may say we are.** It
is an `LD_PRELOAD` interposition defect. This tool ships no preload shim, its
output has no `PT_INTERP`, and `docs/AGENTS.md` §14 already refuses that route,
so interposition cannot reach it. ⭐ **The DEFECT CLASS is ours** — a lookup
that ANSWERS when it should DEFER — and the live instance was found and fixed
as **T-073**.

⚠ **And the route itself is still the one AGENTS.md §7 calls backwards.** Route
B lets host objects *in*, which is the opposite direction to route D, and route
D is shipped and measured (`--host-dlopen`, 11 of 11). `experiments/50-`
measured no effect from the partial port. ⛔ Do not port the shim stack.

