# "One Libc in the Process" — the sweep, and what it changes

⚠ **What this sweep did NOT establish, first, because a reader skimming for
the verdict will not come back for it.**

- ⛔ **Nothing in the paper was re-executed here.** Its T1 measurements come
  from one host — Gentoo, glibc 2.43-r2, GCC 15.3.0, binutils 2.46.1 — and this
  tree has neither that toolchain nor that libc. What follows is a read plus
  **two** targeted re-measurements of our own, both named below.
- ⛔ **It is not a repository and has no upstream.** No commit, no URL, no
  author, no stated licence. `references/operator__one-libc-in-the-process/PROVENANCE.md`
  says so at length; nothing is copied from it into this tree's code.
- ⚠ **Its central claim is not its own measurement.** H3 — that a userspace
  loader answering libc imports from the resident runtime satisfies all four
  constraints — is settled at its T2/T3, and its §10 says *"we did not
  construct a bridge of our own."*

## Provenance

| | |
|---|---|
| kind | a single supplied document, **not** a mined repository |
| stored | `references/operator__one-libc-in-the-process/tree/`, 1,148 lines |
| sha256 | `2506554740fd3414d183e2fcc8bb1530870ad4ad6cdc5addb73450a678a0bc6f` |
| depth | read in full, twice; two claims re-measured here |

## The verdict

⭐ **Useful, and it is not slop.** It states evidence tiers and keeps to them,
reports its own dead ends and one unexplained anomaly, lists its limitations
including the one that matters most, and records three review passes. ⛔ **And
its own central limitation is the thing this repository closed on the same
day**, which is the most interesting fact about it.

| | |
|---|---|
| would it have helped us **earlier**? | ⭐ **yes, on two counts**, §"What it adds" |
| does it still help **now**? | ⭐ yes — it prompted a check on `pgb-elfload.c` that could have been a silent second-libc defect. It came back clean, measured |
| is anything in it wrong for us? | ⚠ one generality, §"Where our data contradicts it" |

## What it adds that this tree did not have

### 1. ⭐ The source-level cause of `experiments/72-`

`experiments/72-` measured that a static executable's dynamic symbol table is
empty (`DYNSYM 0`) and that a host-loaded plugin therefore fails with
`undefined symbol: host_api_add`. ⭐ **The paper supplies the reason, at file
and comment** — glibc's `elf/dl-support.c`:

```c
/* A dummy link map for the executable, used by dlopen to access the global
   scope.  We don't export any symbols ourselves, so this can be minimal.  */
static struct link_map _dl_main_map = { ... };
```

We had the symptom; this is the mechanism, in glibc's own words. ⛔ **It also
makes the failure structural rather than incidental**: it is not that a static
binary *happens* to export nothing, it is that the loader's model of the main
program is a placeholder that consults nothing.

### 2. ⭐ The folklore route is dead TWICE, and we never tested it

A route this project never closed off: *"just link with `-rdynamic` so the
static binary exports its symbols, and the plugin resolves against them."*
The paper kills it at both ends, at T1:

| | |
|---|---|
| **F1** | on binutils 2.46.1, `gcc -static -Wl,--export-dynamic` produces a binary with **no dynamic section at all** — `readelf -d` says so. The flag is accepted and does nothing. `--export-dynamic-symbol-list` does create a `.dynsym`, but only under `-static-pie` |
| **F5** | and even then it is a **dead letter**: a binary with a well-formed six-entry dynamic symbol table including a defined `host_log` still fails to satisfy a plugin whose only reference is `host_log`, because resolution goes through the dummy map above |

⭐ **That is worth carrying because it is a route a future session would
otherwise try**, and it looks plausible right up to the second measurement.
`docs/AGENTS.md` §14's "do not redo these" is where it belongs.

### 3. ⚠ A third independent sighting of the linker-script trap

The paper's **F12**: a musl-built object with `DT_NEEDED: libc.so` fails on a
glibc host with `/lib64/libc.so: invalid ELF header`, because on a glibc host
the name `libc.so` is a **GNU ld linker script**, not an ELF.

⭐ **This tree hit the same trap twice on 2026-09-02c** — `libm.a` is
`GROUP ( libm-2.39.a libmvec.a )`, and reading it as an archive yielded zero
symbols in silence, twice (`docs/history/corrections.md` C18). Three sightings
in three different readers — an archive reader, a provider-table generator, and
a loader's search path — is enough to call it a property of the platform rather
than a mistake anyone keeps making.

### 4. musl's static archive, verified

**F9/F10**: musl 1.2.5's static `libc.a` contains `dlopen.lo` and **not** the
loader (`__dls2`/`__dls3` absent, `nm` count 0); the archive's `dlopen` is a
five-line stub that always fails with `Dynamic loading not supported`. Plus a
working static-PIE musl link recipe and the three toolchain traps around it —
⚠ potentially useful to `experiments/90-`'s onelf arm, which needs a musl
toolchain and skipped again on this run.

## ⭐ What WE have that it says it lacks

⛔ **This is the load-bearing paragraph.** The paper's §10 limitation 2:

> **No bridge of our own.** H3 is supported at T2/T3 [...] but this study did
> not construct, run, or independently re-measure a bridged loader end-to-end.

⭐ **`experiments/76-` is that measurement, at T1, on eleven environments**, and
it was made the same day: a static glibc binary loading a shared object it did
not link, with its own compiled-in loader, opening **zero host shared objects**
on 11 of 11 — against a control, the same source without `--host-dlopen`, that
fails on 11 of 11 with SIG6/SIG8/SIG11.

⭐ **And it is cheaper than the paper's taxonomy predicts, for a reason the
paper itself supplies.** Its §8.4 direction asymmetry says a resident **glibc**
can satisfy objects with little more than dependency bookkeeping, while a
resident **musl** needs a real bridge — which is why `solo` carries ~6,000
lines of `glibc_shim.cpp`. `pgb`'s carrier is static **glibc**, so the bridge
is not written at all:

| | |
|---|---|
| `pgb-elfload.c` | **1,398 code lines**, no ABI bridge |
| `solo` at `79451211` | 2,332 code lines of loader **plus** 5,948 of glibc→musl shim |

⚠ So on the paper's own scorecard (§9.1) this tree adds a **fifth row**: *native
carrier, userspace loader, no bridge required* — C1–C4 all satisfied, at T1,
because the carrier's libc and the object's libc are the same family.

## ⭐ The check it prompted, and the result

The paper's **F6**: under the stock loader, a loaded object **cannot call the
dl API itself** — a weak `dlvsym` reference resolves to NULL.

⛔ **Our loader is the opposite case and that is a hazard, not a feature.**
`pgb`'s generated provider table is built from `libc.a` and therefore contains
rows for `dlopen`, `dlsym`, `dlclose` and `dlerror`. If those rows held
*glibc's static* `dlopen`, then a loaded host object calling `dlopen` — GTK,
Qt and mesa all do — would reach the **host** loader and put a second libc in
the process, silently, through the one door `--host-dlopen` exists to shut.

⭐ **Measured rather than reasoned about, and it is safe.** `--wrap=dlopen`
rewrites *undefined references*, and the table's reference to `dlopen` is one,
so the linker rewrites the table entry too:

```
provider table 'dlopen' = 0x405780
__wrap_dlopen           = 0x405780
VERDICT: table points at OUR wrapper -- safe
```

⚠ **The safety is therefore a consequence of `--wrap` being on the link line,
not of anything in the table generator.** A build that produced the provider
table without `--wrap=dlopen` would reopen the hole. `internal/wrapper/flags.go`
adds both together and cannot add one without the other, which is where the
property lives.

## Where our data contradicts it

⚠ **F2, as stated, does not generalise.** The paper reports that
`dlopen("./plugin_libc.so")` from a plain static binary **succeeds** — "There
is no rejection" — and treats that as the behaviour of glibc 2.43.

⛔ **On eleven pinned environments it succeeds on two and dies on nine**, which
is `docs/limitations.md` §1 and `experiments/50-`: assertion failures inside
`_dl_call_libc_early_init` and `elf_machine_rela_relative`, and SIGFPE. The
paper's §10 limitation 1 already says "single toolchain", so this is not a
contradiction of its evidence — it is a caution about reading its F2 as a
general property. ⭐ **And it strengthens the paper's own argument**: its point
is that success is the *worse* outcome, and nine hosts where it does not even
succeed make that sharper.

⚠ **One further caution.** The paper's H1 — that self-contained objects are the
*only* class the stock loader serves, "and for no larger class" — is stated
about the stock loader and is correct there. ⛔ A reader who carries it forward
as a property of static binaries reaches the conclusion this tree disproved on
the same day. The paper anticipates that reader in its §7.4 and calls its own
earlier version of the claim "premature"; the anticipation is worth as much as
the finding.

## What to do with it

| | |
|---|---|
| ⭐ carried into `docs/AGENTS.md` §14 | the `-rdynamic` route is dead twice; do not try it |
| ⭐ carried into `docs/limitations.md` §1 | the dummy-map comment, as the mechanism behind `experiments/72-` |
| ⭐ carried into `docs/history/corrections.md` C18 | the third sighting of the linker-script trap |
| ⚠ **not** adopted | its taxonomy's vocabulary. `docs/design/tiers.md` and `limitations.md` already carry this tree's own routes A–D, and a second naming scheme for the same territory would be a documentation defect rather than a clarification |
| ⛔ **not** treated as authority | it is unsigned, has no upstream, and its central claim is T3. It is cited as "the working paper" and each claim taken carries its own tier |
