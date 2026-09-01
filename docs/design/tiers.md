# Design: tiered output, and what "universal" can honestly mean

⚠ **STATUS. Tier 1 is built and measured. Tiers 2 and 3 are DESIGN ONLY —
nothing in them has been built or run.** Every claim about them is reasoning
from measurements taken on tier 1 and from prior art read at the depth
`../research/prior-art.md` records. Treat this as a plan, not a result.

---

## The idea

`pgb` picks the least invasive mechanism that actually works for a given
program, escalating only on evidence, and reports what it chose and why. That
maps directly onto the preference order the project's brief sets out: no
application changes, then toolchain changes, then a generic runtime technique,
then a packaging format — earlier is better, and you only move when the
evidence says you must.

| tier | output | serves | cost | status |
|---|---|---|---|---|
| **1** | one static ELF, no interpreter | programs not loading host plugins | none beyond size | ✅ **measured**, 11/11 on five real projects |
| **2** | bundled-glibc **dynamic** binary + directory, with `cross-libc-dlopen` preloaded | programs that must load **host** plugins | not one file; carries a loader | ⛔ design only |
| **3** | tier 2 collapsed to one file via a memfd bootstrap | same as tier 2, single-file | not a plain ELF any more | ⛔ design only |

## Why tier 2 has to exist, and why it is not a patch to tier 1

`experiments/50-host-plugin-feasibility.sh` measured this rather than assuming
it. Porting `cross-libc-dlopen`'s symbol-version rewrite into a **static**
binary changes the outcome on **zero of 11** environments, because the failure
is not symbol versioning: the process dies inside glibc's loader
(`_dl_call_libc_early_init` assertion, `elf_machine_rela_relative` assertion,
SIGFPE). A static binary has no loader, so `dlopen` borrows the **host's**
`ld.so` and `libc.so.6`, and that pairing is what breaks.

⭐ **The diagnosis names the requirement: the process must carry its own loader
and its own libc.** That is precisely the setting `cross-libc-dlopen` was
built for — an `LD_PRELOAD` for a process that already has both — so in tier 2
it applies **unmodified**. `../limitations.md` §1 has the full table.

## Tier 3: single file without extraction

Tier 2 costs the single-file property. `onelf` already solved that: it packs a
directory into one executable with three execution modes, the first being
**in-memory via `memfd`** (`references/QaidVoid__onelf/tree/README.md`), and
`userland-execve-rust` is the mechanism for mapping a bundled loader and
jumping into it without `execve(2)`.

⚠ **Tier 3 gives up "a normal ELF", which is the thing tier 1 exists to
protect.** The brief refuses inventing a *new* format; reusing an existing one
for the class that genuinely needs it is a different decision, and it is the
operator's, not the tool's.

## The hard part is selection, not mechanism

⛔ **"Does this program need host plugins?" is not statically decidable.** A
`dlopen` on a code path no test exercises is invisible to any amount of ELF
analysis. The brief lists this under dependencies that are
"difficult/impossible to discover statically", and it is right.

So selection must be **evidence-driven and conservative**:

1. **Static pass — necessary, not sufficient.** Does the program reference
   `dlopen`/`dlsym`? Absence is close to proof it needs no tier above 1;
   presence proves nothing about whether the target is a *host* object.
2. **Runtime pass — the one that decides.** Build tier 1, then run the
   project's own test suite or a caller-supplied workload under
   `pgb verify`, which already traces and reports host `.so` loads per
   environment. A tier-1 binary that loads zero host objects across the matrix
   while doing real work is the evidence for staying at tier 1.
3. **Escalate only on a failure that tier 1 cannot fix**, and ⛔ **never
   silently.** The tool must say which tier it chose, what evidence moved it,
   and what property that cost. A tool that quietly hands back a
   self-extracting bundle when the user asked for a static binary is the black
   box the brief forbids.
4. **Fail conservative.** With no workload to trace, report "cannot determine —
   tier 1 built, unverified for plugin use" rather than pre-emptively
   escalating. An unnecessary tier 3 is a worse answer than an honest
   uncertainty.

## What "universal" can and cannot mean

⛔ **"Works everywhere" is not a claim this project can make at any tier**, and
the brief forbids treating it as one. The kernel is never abstracted, the CPU
baseline is a build-time choice, and no matrix can enumerate Linux.

⭐ **The claim that IS available is better, because it is falsifiable:** *"built
at tier N; ran correctly and loaded no host object on these 11 named
environments; here is the command that re-checks it on yours."* `pgb verify`
already produces exactly that, and it is the honest replacement for
"universal".

⚠ **Two things cut across all three tiers and are unsolved in every one of
them:** terminfo and TLS CA bundles (`../limitations.md` §3). Bundling a libc
does not supply a terminal database or a trust store. A genuinely
"ship-anywhere" story needs `--embed-terminfo` and a CA answer as well as the
tier work — they are not made redundant by it.

## First step for whoever builds this

Not the tier machinery. **Read
`references/pkgforge-dev__cross-libc-dlopen/tree/docs/limits.md`** — that
project's own measured list of what its approach cannot do. It was **not** read
during this project's sweep, and it is the cheapest available check on whether
tier 2 delivers what this page assumes. Then `../AGENTS.md` §13 item 3 has the
build sequence and the acceptance test.
