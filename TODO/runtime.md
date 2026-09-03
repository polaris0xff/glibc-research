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

## T-076 — ⛔ the TENTH quirk: static glibc reads the host's timezone database

**Source** ⭐ **found 2026-09-03c** by taking the operator's *"fix all remaining
GLIBC quirks if there still are some"* as a question about **completeness**
rather than about the eight that are closed.
**Category** runtime · **Priority** P1 · **Effort** M · **Status** open

⛔ **`docs/REQUIREMENTS.md` said of its nine issues: *"there is no unenumerated
remainder."* That was false, and it was the sentence that made part 2 of the
operator's bar countable.** `grep -rn zoneinfo` over `docs/`, `TODO/`,
`experiments/`, `poc/`, `internal/` and `tool/` returned **nothing**. Nobody
had looked.

**Measured** — `experiments/97-timezone.sh`, **pass=10 fail=0 skip=0**,
`evidence/97-timezone/RESULT.txt`:

| | |
|---|---|
| static `libc.a` | names `/usr/share/zoneinfo`, `/etc/localtime` and honours `TZDIR`, and carries **no data** |
| resolve `Europe/Berlin` correctly | **7 of 11** — `CEST +0200`, hour 02 |
| ⛔ cannot, and do not say so | **4 of 11** — alpine 3.10, 3.20, 3.22 and ⚠ **ubuntu-20.04, which is glibc** |

⛔ **The failure mode is worse than "it returns UTC".** With no database glibc
re-reads `TZ=Europe/Berlin` as a POSIX zone specification — a bare abbreviation
with no offset — and prints:

    Europe +0000 00

⭐ **the zone name the caller ASKED FOR, with a UTC offset.** So `%Z`, the field
that looks like a confirmation, is an echo of the input, and the only field
carrying the defect is the offset. A log line reading `Europe` beside a
timestamp two hours out is the production shape of this bug.

⚠ **And it is not a musl story.** Three of the four are Alpine; the fourth is a
Debian-family image that simply does not install `tzdata`.

**What is left.**

1. ⭐ **The fix has a precedent and it is the same one twice over.** `terminfo`
   and the CA bundle are both host databases that some environments lack, and
   both were closed by an **opt-in `--embed-*`**. `--embed-tzdata` is the
   third of that family. ⚠ Unlike those two, glibc offers a documented hook —
   `TZDIR` — which may make it cheaper; ⛔ that is a guess and the mechanism
   has not been chosen.
2. ⚠ **Decide what "correct" means when the zone is unknown.** Silently
   answering with a UTC offset is the defect; refusing is a behaviour change
   for programs that do not care about time zones. The `--embed-*` family's
   answer — opt in, and be exact when you do — is the likely one.
3. ⛔ **Size.** A full `tzdata` is ~1,800 files and a few hundred KB
   compiled; embedding all of it is not obviously right for a program that
   uses one zone.

⭐ **AND THE METHOD MATTERS MORE THAN THE ROW.** One grep found a tenth issue
in a list called complete. The next candidates, each worth one measurement:
`/etc/services` and `/etc/protocols` for `getservbyname`; `libgcc_s.so.1` for
`pthread_cancel` and `backtrace` — ⚠ **probed the same day: 0 mentions in the
build host's `libc.a` at glibc 2.39**, so likely already dead upstream, but
**not measured on the pinned 2.41**.

**Prove.** All eleven resolve `Europe/Berlin` to `CEST +0200`, with the same
binary, and `experiments/97-` asserts `MISSING = 0`.
