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

## T-078 — The three-way parity matrix: vanilla `gcc -static`, ours, native musl static

**Source** ⭐ **operator, 2026-09-03d**: *"a markdown table covering 'vanilla'
GLIBC static binaries vs 'Ours' static binaries vs native MUSL static binaries
must be compared on all possible comparisons that they can be compared with"*.
**Category** runtime · **Priority** P1 · **Effort** L · **Status** open

**Problem.** The claim under test is the operator's: *"our static glibc binary
and a native musl static binary are at feature/standalone parity. No buts and
no ifs."* ⛔ Nothing in this tree states it as one comparison. The evidence is
spread across `experiments/20-`, `30-`, `40-`, `60-`, `61-`, `71-`, `75-`,
`76-`, `97-` and eleven POC result files, and **the musl column is inferred in
most of it rather than run**.

**What is left.** One matrix, three columns, every cell a measurement or a
dash — [`../docs/comparison.md`](../docs/comparison.md)'s rule. Candidate rows,
all of which this tree already has an instrument for:

    runs / payload clean on the eleven      NSS            iconv / gconv
    locale                                  terminfo       CA bundle
    timezone                                dlopen: own plugins
    dlopen: host objects                    throughput (malloc 1t/4t, qsort,
    startup                                   str*, snprintf, math, memcpy)
    peak RSS                                artefact size
    what it writes to the filesystem        PT_INTERP / DT_NEEDED
    what breaks it

⛔ **Every row needs the musl column actually RUN.** `experiments/60-` and
`61-` build musl arms already — start there rather than writing a new harness.
⚠ **`musl-gcc` is absent on this machine**; install it before starting or the
table has a hole in the column the operator named.

⭐ **One row is already written by somebody else and should be cited, not
re-derived**: the bundling camp's own account of glibc — `LOCPATH` not working
with locale archives, a missing gconv plugin failing silently and randomly, and
`ld-linux.so` needing a patch to stop it reading `/etc/ld.so.cache`.
[`../docs/research/bundle-capabilities.md`](../docs/research/bundle-capabilities.md) §1.

**Prove.** The matrix in `docs/comparison.md`. ⛔ **Four conditions, and the
first three are the ones that fail:**

1. **Every cell is a measurement or a dash**, and each measurement names the
   experiment beside it. ⚠ A dash means *not measured here* and never
   "probably fine" — that is this page's existing rule and it is the whole
   value of the table.
2. ⛔ **A SKIP IS NOT A DASH AND IT IS NOT A PASS.** `experiments/60-` and
   `61-` **skip arms they cannot build** rather than failing, so a missing
   musl toolchain produces a green run with an empty column. Check the skip
   count on every run and report it; `docs/AGENTS.md` §0b — *a check that
   quietly runs nothing and reports success is the worst answer this codebase
   can give*.
3. ⭐ **PRE-REGISTER which cells you expect to differ, before running.** Write
   the prediction down first. This tree's strongest results are the ones where
   *"the expectation was pre-registered BEFORE the run, not after"*, and its
   worst are the ones where a number was explained after it arrived.
4. ⛔ **The parity claim is the operator's and it is falsifiable: "no buts and
   no ifs".** If a row comes out against us, that row IS the deliverable —
   report it, do not soften the axis until it passes.

## T-079 — Enumerate the remaining glibc-static edge cases, by SEARCH

**Source** ⭐ **operator, 2026-09-03d**: *"GLIBC static is truly complete, no
edgecases exist ... No buts and no ifs."*
**Category** runtime · **Priority** P1 · **Effort** M · **Status** open

**Problem.** ⛔ **This exact question has been answered wrongly once.**
[`../docs/REQUIREMENTS.md`](../docs/REQUIREMENTS.md) said of its list of nine
host-data dependencies *"there is no unenumerated remainder"*. A **tenth** was
found the next day — the timezone database — by somebody taking the question as
one about completeness rather than about the list. `grep -rn zoneinfo` over the
whole tree returned **nothing**, and the row that came out of looking fails on
**four** environments, one of them glibc. `experiments/97-`, T-076.

⭐ **The list is now TEN and nine are closed.** ⛔ That is not evidence that
ten is the number.

**What is left.** Ask again, and answer with a **search**, not a sentence. The
tenth was found by asking *what else does glibc read from the host that a
static link does not absorb* and then grepping for it. Named starting points:
`nss`, `gconv`, `locale`, `terminfo`, CA bundles, `zoneinfo` — all closed —
then what is NOT in that list: `/etc/services`, `/etc/protocols`,
`/etc/resolv.conf` options, `iconv` aliases, `getpwnam` beyond NSS, `ld.so`
config, and whatever the search turns up that this sentence did not predict.

⚠ **The known-open one is host `dlopen` beyond `--host-dlopen`**, plus
`--tls-reserve`'s ~1.15 MB cost on every such build. T-072.

**Prove.** Either a new row in `REQUIREMENTS.md` with an experiment behind it,
or the enumeration written down with **the command that produced it and what it
looked at** — ⛔ never the bare sentence that failed last time.

⭐ **What "done" looks like, stated so it cannot be satisfied by prose:** a
reader can re-run your search and get your list. ⚠ **An absence is not a
zero** — if a probe found nothing, say **where it looked**, because the tenth
row was invisible to every probe that had been run and visible to the first one
that looked in `zoneinfo`.

⛔ **And the honest outcome may be "the list is still ten".** That is a result
if the search is shown; it is not a result if it is asserted.
