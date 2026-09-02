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
| **1** | one static ELF, no interpreter | programs not loading host plugins | none beyond size | ✅ **measured**, 11/11 on ten real projects |
| **2** | bundled-glibc **dynamic** binary + directory, with `cross-libc-dlopen` preloaded | programs that must load **host** plugins | not one file; carries a loader; ⛔ **and gconv, unless the tier-1 iconv wrap comes with it** | ⛔ design only |
| **3** | tier 2 collapsed to one file via a memfd bootstrap | same as tier 2, single-file | not a plain ELF any more | ⛔ design only |

## ⭐ Tier 2 has a minimal reference implementation, and it is 60 lines of C

`references/leleliu008__python-distribution/tree/linux-portable.sh` @
`987e937a`. Read, not run. The shape:

- `foo` is renamed `foo.bin`; a generated `foo.c` is compiled `gcc -static`
  and takes its place, so **what the user runs is an ordinary static ELF**;
- it reads `/proc/self/exe`, derives `<self>.bin` and the `lib/` beside it,
  and **`execv`s the bundled loader** with `--library-path <lib> --argv0
  <self> <real>`;
- system shared libraries are copied into `lib/` with an `$ORIGIN` RPATH;
- on musl, `ld-musl-<arch>.so.1` is symlinked to `libc.musl-<arch>.so.1`,
  because there the loader and libc are one file under the libc's name.

⛔ **Two details this page did not have, and both are cheap to get wrong:**

- ⭐ **`--argv0` is not optional.** Without it the program sees the loader's
  path in `argv[0]`. CPython derives `sys.executable` from that, so an
  interpreter bundled without it cannot find itself.
- ⭐ **Pass the library path to the loader, never through
  `LD_LIBRARY_PATH`.** As a loader argument it applies to this one process; as
  an environment variable it is inherited by every child, including host
  programs the bundle later runs.

⚠ **And it closes off one axis.** [`toolchain.md`](toolchain.md) lists "no
shell in the delivery path" among the things a `pgb` bundle would have to be
better at. This implementation already has no shell in it, so that is not a
differentiator — what is left of that list is **shape** (one file, nothing
written) and **size**.

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

## ⛔ Tier 2 is not a superset of tier 1

`experiments/60-versus-alternatives.sh` did not set out to test this page, but
it did: **onelf is already exactly the tier-2 shape** — bundled glibc plus its
own loader, packed into one file — so running it on the same 11 environments is
a direct measurement of the assumption above.

**The half this page assumed, confirmed.** The bundled process carries its own
libc everywhere. On all 11, musl included, the trace shows
`ld-linux-x86-64.so.2`, `libc.so.6` and the runtime's own shim opened out of
the package and **not one host object**. A glibc program really does start on
Alpine 3.10 this way.

⛔ **The half this page missed.** It fails the encoding assertions on **8 of
11**, because *bundling glibc does not bundle gconv*. On the three it passes —
Debian 11, Debian 12, Ubuntu 20.04 — the trace shows it reaching the **host's**
`/usr/lib/x86_64-linux-gnu/gconv/` for `EUC-JP.so`, `ISO8859-1.so` and
`libJIS.so`. So it passes by finding host modules whose path happens to match,
and fails wherever no such path exists.

⭐ **Which means tier 2 regresses a result tier 1 already has — unless it is
told not to.** Tier 1 solved gconv with `-Wl,--wrap` onto static GNU libiconv,
and that mechanism is independent of which libc is in the process, so carrying
it across is one option.

⛔ **The other option is bundling the gconv modules, and the refusal in
`../AGENTS.md` §14 does not apply at tier 2.** That rule exists because each
module carries `DT_NEEDED libc.so.6` and would pull a second libc in — true
**for a static binary**, where there is no bundled libc for that edge to bind
to. A tier-2 process already carries its
own libc *and* its own loader, so the edge resolves inside the bundle and no
second libc enters.

⭐ **`Anylinux-AppImages` does exactly this and it works.** `quick-sharun.sh`
deploys the whole `/usr/lib/gconv` tree plus its config and reaches it with
`GCONV_PATH` — its own comment at line 816 reads "gconv is always deployed,
removing it only saves ~30 KiB". `experiments/62-` measures the result: the
same program in an anylinux AppImage passes the encoding assertions on **11 of
11**, musl included, opening its gconv modules out of its own bundle and no
host object at all. onelf fails 8 of 11 not because bundling cannot solve
gconv but because onelf does not bundle it.

⚠ **So tier 2 has two working answers for gconv and must pick one
deliberately**: carry tier 1's `--wrap`, or bundle the modules the way sharun
does. What it must not do is inherit tier 1's refusal, which was reasoned about
a different situation.

`../comparison.md` has the numbers and `evidence/60-versus-alternatives/per-environment.txt`
the per-cell object lists.

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

⛔ **"Does this program need host plugins?" is not decidable from the ELF
alone.** A `dlopen` on a code path no test exercises is invisible to static
analysis, which is why the brief files it under dependencies that are
"difficult/impossible to discover statically". ⭐ It is decidable from a
*run*, and that is what the selection procedure below uses — `quick-sharun`
reaches the same conclusion and traces the program to find its `dlopen`ed
libraries.

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
tier 2 delivers what this page assumes. Then `../AGENTS.md` §13 item 4 has the
build sequence and the acceptance test.
