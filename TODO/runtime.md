# runtime — the four mechanisms, and reaching the plugin class

`tool/runtime/*.c`. Routes: [`../docs/AGENTS.md`](../docs/AGENTS.md) §7.

⚠ **Open entries only.** The 6 closed ones are
[`../HISTORY/entries/runtime.md`](../HISTORY/entries/runtime.md); the
long-form findings behind the entry below are
[`../HISTORY/entries/runtime-open.md`](../HISTORY/entries/runtime-open.md).

---

## T-031 — Port cross-libc-dlopen's full rewrite, not one function

**Source** [`../docs/limitations.md`](../docs/limitations.md) §1 · **Category** runtime · **Priority** P2 · **Effort** L · **Status** open

**Problem.** `experiments/50-` ported `cld_strip_versions()` — one function of
roughly forty from a 2015-line file — and found no effect. The two steps it did
not port are the ones aimed at the failure it observed: dropping the
`DT_NEEDED` edges that pull a foreign libc in, and rebinding the remaining
imports.

⛔ **AND THE ROUTE IS THE ONE `AGENTS.md` §7 CALLS BACKWARDS.** Route B lets
host objects *in*; route D is shipped and measured (`--host-dlopen`, 11 of 11).
`experiments/50-` measured no effect from the partial port. ⛔ **Do not port
the shim stack** without a reason this entry does not currently have.

⚠ **The reference moved.** Re-mined at **`793f3f3f`** (PR 30's merge commit),
not the `1cecf50e` a port would have inherited a fixed bug from. ⭐ **We are
not affected by that bug and no document here may say we are** — it is an
`LD_PRELOAD` interposition defect and this tool ships no preload shim. The
defect *class* — a lookup that ANSWERS when it should DEFER — is ours, and the
live instance was found and fixed as T-073.

**What is left.** If it is taken at all: `CROSS_LIBC_DLOPEN_DRYRUN` makes the
rewrite path testable with no GPU and no Alpine.

**Prove.** `experiments/51-*.sh` re-runs `50-`'s two arms plus a third carrying
the full rewrite, and the table shows what changed on each of 11.

📚 [detail](../HISTORY/entries/runtime-open.md)
