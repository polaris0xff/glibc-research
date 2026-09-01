# runtime — the three mechanisms, and reaching the plugin class

`tool/runtime/*.c`. Routes: [`../docs/AGENTS.md`](../docs/AGENTS.md) §13 item 4.

---

## T-030 — `--wrap-dlopen` against a compiled-in table

**Source** `docs/research/prior-art.md`, `allyourcodebase/pipewire`.
**Category** runtime · **Priority** P1 · **Effort** M · **Status** open

**Problem.** A program loading its own plugins is servable only by hand today
(POC 50). The generic mechanism is not built.

**Premise.** ⭐ Proven prior art, read at file level:
`references/allyourcodebase__pipewire/tree/src/wrap/dlfcn.zig` exports
`__wrap_dlopen`/`__wrap_dlsym`/`__wrap_dlclose` against a compiled-in table.
That is the same delivery mechanism `pgb` already uses for `iconv_open`.

**Approach.** Cheapest of the three routes. Generate the table from the plugins
the build produced; wrap at the final link as `pgb-iconv.c` does.

**Prove.** POC 50's CPython rebuilt with `--wrap-dlopen` instead of hand-written
`Modules/Setup.local`, passing the same matrix.

## T-031 — Port cross-libc-dlopen's full rewrite, not one function

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

## T-032 — `--embed-terminfo` and a CA-bundle answer

**Source** `docs/limitations.md` §3 · **Category** runtime · **Priority** P2 · **Effort** S · **Status** open

**Problem.** Two of five host data dependencies are open. Both are reached
through an environment variable, which is the shape `--embed-locale` already
proved.

**Prove.** POC 20's `setupterm()` probe passing on all 11, and POC 30's curl
verifying TLS on all 11 with the harness's own CA variables unset.
