# AGENTS.md — read this first, and you can work

Standalone. Assumes no prior context. Every claim here is produced by a script
in this tree that you can re-run.

---

## 0. Read these, in this order, then you have everything

⛔ **Six files. Nothing crucial lives outside them.** Each links onward; §11 is
the full map.

| # | read | why you cannot skip it |
|---|---|---|
| 1 | **this file** | the state, the mechanisms, the open problems, the rules |
| 2 | [`../TODO/PROGRESS.md`](../TODO/PROGRESS.md) | ⭐ **what to do next, and ⛔ THE STOP CONDITION.** The work order, the required POCs and the open questions live here and nowhere else |
| 3 | [`../TODO/INDEX.md`](../TODO/INDEX.md) | every entry, and the argument behind the ordering |
| 4 | [`../TODO/RULES.md`](../TODO/RULES.md) | ⛔ how this repository is worked on — git, the fetch routes, the record, no deferral |
| 5 | [`REQUIREMENTS.md`](REQUIREMENTS.md) | the operator's binding bar, which is **not met** |
| 6 | [`design/toolchain.md`](design/toolchain.md) | ⭐ what `pgb` is (a toolchain, not a format) and where it is going |

⚠ **Then, before you write an experiment or read somebody else's code**, the
methodology binding on that work: [`methodology/sessions.md`](methodology/sessions.md),
[`methodology/experiments.md`](methodology/experiments.md),
[`methodology/references.md`](methodology/references.md),
[`methodology/vendoring.md`](methodology/vendoring.md). They are vendored, and
`methodology/PROVENANCE.md` says at which commit.

⭐ **`sh TODO/check.sh` is the gate.** Run it before every commit; it fails if
the record disagrees with itself.

---

## 0b. The shape of a session

⭐ **Everything in this block is answered here so you do not have to infer it.**
[`methodology/sessions.md`](methodology/sessions.md) is the full specification;
this is what it means in this repository.

| question | answer |
|---|---|
| **What do I do?** | [`../TODO/PROGRESS.md`](../TODO/PROGRESS.md) "Work order", then [`../TODO/RESUME.md`](../TODO/RESUME.md) for what the last session had in flight |
| **In what order?** | the work order, top down. P0 before P1 before P2; [`../TODO/INDEX.md`](../TODO/INDEX.md) carries the argument for the ordering and it is meant to be re-derived, not re-argued. ⛔ Do not skip down the list for something easier |
| **How many?** | as many as fit. ⛔ One task is not a session |
| **How long?** | until the operator interrupts, or every task in the work order is complete **with evidence**. ⛔ Not "until a task is done" |
| **What if I am blocked?** | finish everything that does not depend on the blocker, write the blocker into `PROGRESS.md` "Open questions" with what you tried, and keep going. ⛔ Do not stop and wait |
| **What if it cannot be done?** | that is a result. Record which rung stopped it, verbatim, the way the POCs do. A recorded failure is worth more than a skipped row |
| **How do I end?** | [`methodology/sessions.md`](methodology/sessions.md) "Ending": rewrite `PROGRESS.md`, refresh `RESUME.md`, write `SUMMARY.md`, both gates green, CI green, push, then print the summary table |

⛔ **Write [`../TODO/RESUME.md`](../TODO/RESUME.md) at the START**, not the end.
It is a dead man's switch: everything else in the protocol is written at the
moment an interrupted session never reaches. Refresh it whenever what is in
flight changes.

⛔ **Do not idle.** Do not end a turn to wait for a build, and do not reach for
a scheduler or wake-up tool to do the waiting for you — that is the same thing
wearing a disguise. Long builds run in the background while you work on
something else; block on the job's own output, not on a timer.

### The discipline

- **Every claim carries its measurement.** A number with no command that
  produced it does not go in a document.
- **An absence is not a zero.** A probe that found nothing may have looked in
  the wrong place — say which.
- **Exit codes: 0 ok, 1 it ran and failed, 2 it could not run.** ⛔ A skip is
  neither a pass nor a failure. A check that quietly runs nothing and reports
  success is the worst answer this codebase can give.
- ⛔ **Verify before you trust.** Every defect in this tree was found by
  something disagreeing — a gate, a control, CI — and essentially never by
  reading. Run the thing.
- **Read code with codegraph first, grep second** ([`codegraph.md`](codegraph.md);
  it indexes no shell).
- ⛔ **The experiments and POCs stay shell** — they are the independent
  acceptance harness. **`HISTORY/` and `methodology/` are never edited.**
- **Comments say what the block does.** Historical lore goes to
  [`history/`](history/), never into code or these pages.
- **Commit as work lands and push.** ⛔ Never one commit at the end.
- ⛔ **Never edit a shell script while it is running** — `sh` re-reads from a
  byte offset.

⚠ **§14 carries the rest**: the rules on language and on superseding a finding,
and the list of things measured once that must not be redone.

---

## 1. The project

Answer, with evidence, whether a **normal Linux ELF** built against glibc can
run reliably on both glibc and musl systems with no packaging format and no
significant overhead — and ship the tool that does it.

⛔ **Before you decide what to work on, read
[`REQUIREMENTS.md`](REQUIREMENTS.md).** It carries the operator's binding
acceptance bar — *works everywhere, or strictly better than every existing
format and technique* — which this project **does not meet**, and it tracks
what each piece of work does about that.
⭐ **Part 2 of that bar was replaced by an operator ruling on 2026-09-01b**: it
is no longer a comparison against bundles but *a static glibc binary with none
of the issues*, and the issues are an enumerated list of nine, six closed and
three open. `history/corrections.md` C13.

The tool is [`../pgb`](../pgb) (portable glibc build): ⭐ **one statically
linked Go binary**, built `CGO_ENABLED=0`, carrying the four small C runtime
pieces it compiles. ⚠ It is a BUILD PRODUCT and is not committed — run `make`.
Output is an ordinary statically linked executable. No launcher, no AppDir, no
loader, nothing beside it.

**The answer reached: yes, for programs that do not need to load host
plugins.** ⭐ **Ten real projects prove it, and the largest is Qt 6** — a
static Qt 6.11.1 widget program runs on 11 of 11 with zero host shared
objects, which is the rung `poc/80-mlt` named as untried and nobody had
attempted. §7 has the open problems and the route to each.

⛔ **`pgb` is a TOOLCHAIN, not a delivery format**, and comparing it to one
gets the next step wrong. AppImage, Flatpak, snap and onelf answer *how does
this reach a machine*. `pgb` answers *how does a developer get from source to a
binary that runs*, and its output is deliberately not a format — it is an
ordinary ELF. The target shape is `pgb build <url-or-package>`, with the tool
resolving the source, working out dependencies, linking statically what it can,
and bundling only what is left. [`design/toolchain.md`](design/toolchain.md)
is the design and the language decision.

⭐ **Why glibc, measured.** `tmp/START.md` asks for static binaries "using
GLIBC **rather than MUSL** … while avoiding the usual drawbacks", so musl is
the drawback being avoided rather than a rival. `experiments/61-` measures the
contested ground — steady state, same machine, same compiler, libc the only
variable:

| | glibc static | musl static |
|---|---|---|
| malloc, 4 threads | **6.73 / 8.40 ns/op** | 606.39 / 704.79 ns/op |
| qsort | **84.14 / 83.96 ns/op** | 620.81 / 607.43 ns/op |
| strlen/strchr/strstr | **244.75 / 241.24 ns/op** | 1377.94 / 1355.08 ns/op |

⭐ **Two runs, 2026-09-03e**, on the first machine here to have `musl-gcc`
installed — this arm had been **skipped** for want of it, so earlier figures
came from another machine. ⚠ `memcpy` **changes sign** between the two runs and
is reported as *no difference measurable*; `comparison.md` has every row.

⭐ **The product, in one row:** on **Alpine**, where the ordinary choice is a
musl build, a `pgb` binary does that 4-thread allocator workload in
**6.86 ns/op** against musl's **1045.49**, and **6.38–11.43** against
**622.99–1069.39** across all eleven. glibc's numbers on a machine that ships
no glibc — and `pgb` costs 1.00×–1.13× against plain `gcc -static` on the same
workloads.

The stack to measure against is **`Anylinux-AppImages`**: it bundles glibc, its
loader and its gconv tree, and `experiments/62-` has it running on 11 of 11 at
glibc's speed too. It automates dependency bundling well and this project
should learn from it. What differs is what the developer has to assemble —
[`comparison.md`](comparison.md).

## 2. The problem

`gcc -static` against glibc is **not** self-contained, though `file` and `ldd`
say otherwise. glibc's NSS and gconv are plugin systems: the plugin is named in
**host** configuration and loaded with `dlopen` at run time. Static linking
links the dispatcher, never the plugins. Each host plugin carries
`DT_NEEDED libc.so.6`, so a second libc and the dynamic loader enter the
process.

glibc 2.34 made the `files` and `dns` NSS services builtin. That removed the
*default* dlopen only; `resolve`, `myhostname`, `mymachines`, `mdns4_minimal`,
`compat`, `sss` and `systemd` remain external and are named by default on
modern distributions.

Measured, plain `gcc -static`, across the 11 pinned environments:

| | |
|---|---|
| host NSS modules loaded | 5 of 11; **SIGFPE on Arch and openSUSE Leap** (openSUSE's via `passwd: compat`, not DNS) |
| ⚠ which instrument | the signals in this table are the **chroot bed's**. Under `pgb verify --engine docker`, whose tracer uses `ptrace`, Debian 11 and Ubuntu 20.04 report **SIGABRT** where the chroot bed reports **SIGSEGV** — the control reaches `abort()` when traced and segfaults when not. Reproducible, no verdict moves, and `history/corrections.md` C11 says why both are kept |
| `iconv_open` | **SIGFPE/SIGABRT on Debian 11/12 and Ubuntu 20.04** where the host gconv path matches the build's; 11 of 12 encodings silently unavailable where it does not. **There is no working case.** |
| `setlocale` UTF-8 | `ANSI_X3.4-1968` on all 4 musl environments |

A probe exercising NSS **and** iconv (`ci/probe.c`) fails on **all 11** *in the
chroot bed*. ⚠ **That is an instrument-dependent number and CI measures it
differently**: building on the runner and running in a container, the gconv
paths do not match, the iconv arm degrades silently instead of dying, and the
control **exits 0 on debian-12**. `history/corrections.md` C21.

## 3. Success criterion

On every environment in §8:

1. runs — the program's own exit status, no signal;
2. loads **no host shared object**, checked by syscall trace attributed to its
   own pid, never by `ldd`;
3. real functional assertions pass, not `--version`.

⭐ **Criterion 2 is about shared objects, not host data.** glibc still *opens*
`/etc/nsswitch.conf` under the override, and honouring the host's locale where
one exists is correct. The property is **independence** — working whether or
not host data is present — and the musl rows, which have none of it,
demonstrate it. Data reads are reported in their own column, never asserted.

## 4. How it works

Four mechanisms, no application source changed by any of them. The first three
are on by default; the last two are opt-in:

| | mechanism | file |
|---|---|---|
| **NSS** | constructor calls `__nss_configure_lookup()` (public `GLIBC_2.2.5`, present in `libc.a`), pinning all 14 databases to services glibc ≥ 2.34 implements inside libc. Passed as a plain object with `-Wl,-u,pgb_runtime_anchor`, because a constructor with no referenced symbol is dropped from an archive. | [`../tool/runtime/pgb-nssfix.c`](../tool/runtime/pgb-nssfix.c) |
| **iconv** | `-Wl,--wrap=iconv_open,--wrap=iconv,--wrap=iconv_close` onto static GNU libiconv. Acts at the final link, so it catches calls from any object including archives built before this tool existed. Lives in an archive, so a program that never calls `iconv_open` links none of it — 940 KiB vs 2.1 MiB, same source. | [`../tool/runtime/pgb-iconv.c`](../tool/runtime/pgb-iconv.c) |
| **locale** (opt-in `--embed-locale`) | `-Wl,--wrap=setlocale`; C.UTF-8 embedded, written out only when the host cannot answer a UTF-8 request. The only mechanism that touches the filesystem, hence opt-in. | [`../tool/runtime/pgb-locale.c`](../tool/runtime/pgb-locale.c) |
| **host plugins** (opt-in `--host-dlopen`) | ⭐ **an ELF loader compiled IN.** Maps the object, walks `DT_NEEDED`, relocates (`DT_RELR` included), honours versioning, runs the initialisers, and binds imports to the static glibc already linked. A `DT_NEEDED` the image already satisfies is ANSWERED, not opened, so no second libc enters; a libc it does not provide is REFUSED, which is the right answer on musl. 11 of 11, zero host objects, `experiments/76-` | [`../tool/runtime/pgb-elfload.c`](../tool/runtime/pgb-elfload.c) |
| **own plugins** (opt-in `--wrap-dlopen`) | `-Wl,--wrap=dlopen,--wrap=dlsym,--wrap=dlclose,--wrap=dlerror` onto a table `pgb` **generates** with `nm` from the objects the build produced. ⭐ A program loading its *own* plugins never needs a loader — the code is in the link and `dlopen` is only doing a name lookup. Nothing is mapped, so no second libc can enter. 11 of 11, zero host objects, +544 B. ⚠ Not for **host** plugins — §7. | [`../tool/runtime/pgb-dlopen.c`](../tool/runtime/pgb-dlopen.c) |

**Delivery:** compiler wrappers on `PATH` plus `CC`/`CXX`. autotools, CMake,
meson and make pick them up unmodified. Each wrapper reads its own argv:
`-c`/`-E`/`-S`/`-M` = compile; `-shared` = **passed through untouched**, which
is what lets `./configure`'s shared-library probes still work; anything else =
executable link. `pgb explain` prints every injected flag.

**Build environment:** pinned `debian:13` (glibc 2.41, gcc 14.2.0) by manifest
digest, unpacked by `internal/ociimg` (`pgb rootfs pull`) and entered by
`chroot`. Verified not to be host contamination: output `.comment` reads
`GCC: (Debian 14.2.0-19) 14.2.0` where a host build reads `Ubuntu 13.3.0`.

⭐ **The pin is a FLOOR and a CEILING pointing opposite ways, and it moved on
2026-09-02** — 2.36 → 2.41, T-070, all four measured costs zero, class B
(a host symbol newer than the pin) **20 → 5 distinct symbols**.
[`design/glibc-versions.md`](design/glibc-versions.md) is the argument.
⛔ **It is three constants in `internal/cfg/cfg.go` and nowhere else**;
`TODO/check.sh` fails if a copy appears in code. The move found **nine** copies
of the name and **two** of the digest.

## 5. Repository layout

```
pgb                       ⭐ THE TOOL, and it is a BUILD PRODUCT: one static
                          Go binary. `make` builds it; it is gitignored
cmd/pgb/                  option parsing and command dispatch
internal/logx             levels, per-subsystem debug, the composed command
                          printed before it runs, and the timestamped stream
internal/proc             every child process, argv arrays only
internal/cfg              settings, and the option handoff across an engine
internal/wrapper          the runtime objects, the injected flags, the wrapper
                          directory, the --wrap-dlopen table, `pgb explain`.
                          ⭐ The wrappers are pgb itself under another name
internal/envx             the environment stamp, env create/info, libiconv
internal/buildx           build, shell, the __inner-* re-entry points
internal/verifyx          the matrix, the strace reader and the carried tracer
internal/rootfs           unshare+chroot done natively; fetch the bed
internal/ociimg           the registry client and the whiteout merge
internal/elfx             ELF and `ar` read directly, not through nm/readelf
internal/nixx             NAR, nix-base32, ed25519, ATerm, the package index,
                          the planner, the dependency walk, the build rounds
internal/bootstrapx       `pgb bootstrap`: a fresh machine, in parallel
internal/selftest         the shape every carried-in selftest reports in
assets.go                 the C runtime sources and the pinned target list,
                          EMBEDDED, so a distributed pgb carries them
tool/runtime/*.c          the five mechanisms, and pgb-trace.c, the carried-in
                          tracer `pgb verify` uses where strace cannot follow.
                          pgb-elfload.c is the compiled-in ELF loader
internal/bundle           the bundler: uruntime+dwarfs+sharun, the
                          reachability sweep, the nixpkgs wrapper reader
ci/probe.c                the binary CI runs on 11 distributions
scripts/common/
  rootfs-images.txt       the 11 environments, pinned by digest
  mine-repo.sh            reference-sweep fetcher, vendored    (--selftest)
  check-docs.sh           the documentation gate
  install-codegraph.sh    ⭐ the code-reading index, pinned and sha256-checked
codegraph.json            what that index excludes and deprioritises
experiments/lib.sh        conditions block, assertions, pid-attributed tracing
experiments/clock.sh      ⭐ the WALL-CLOCK instrument, and the second library
                          here. Median of N, arms interleaved with a rotating
                          start, and an A/A CONTROL — one artefact under two
                          names — whose ratio is the floor below which no row
                          may be believed. ⛔ Every millisecond in this tree
                          goes through it; `history/corrections.md` C24 is what
                          the one-sample instrument before it was measuring
experiments/NN-*.sh       numbered; exit 0 matched, 1 did not, 2 could not run.
                          ⚠ A number can be taken by an `evidence/<NN>-*`
                          directory with no script — `92-go-port` is one
docs/REQUIREMENTS.md      the operator's acceptance bar, and how far short
docs/methodology/         vendored, pinned; binding on experiments and sweeps
TODO/                     the work: PROGRESS, INDEX, RULES, RESUME, entries
TODO/check.sh             the gate; run before every commit
poc/common.sh             the POC contract
poc/NN-*/run.sh           the proof-of-concept projects
poc/run-all.sh            ⭐ the acceptance suite as one command. `--rebuild`
                          after ANY wrapper change: only five of the ten
                          honour POC_REBUILD, so a plain re-run reuses
                          binaries the old toolchain produced
evidence/                 committed RESULT.txt per experiment and POC
HISTORY/<commit>/         ⛔ the shell and Python the Go port replaced. Kept
                          because it is the ORACLE every byte-identical
                          comparison was made against. Nothing here runs
references/               34 upstream trees + trackers, tracked, PROVENANCE.md
                          (`ls references/ | wc -l`, and each has one)
.github/workflows/portability.yml
docs/                     see §11
tmp/START.md              the original brief
```

## 6. Running it

⭐ **`pgb` is a build product. Two commands and the machine is working:**

```sh
make                                 # CGO_ENABLED=0 go build -o pgb ./cmd/pgb
./pgb bootstrap --detach             # build env + 11 rootfs + nix, PARALLEL
./pgb bootstrap --check              # is it ready yet
```

⛔ **Serially those steps are ~25 minutes of watching** — nix ~7, the build
environment ~8, the bed ~10 — and nothing in them depends on anything else in
them. **Two sessions paid that** before it was parallel. It is resumable (each
step skipped when its artefact is on disk, checked by looking at the disk
rather than at a marker it wrote), and `--check` changes nothing.

⛔ **It also starts dockerd AND builds the docker environment, together,
because starting the daemon alone breaks every build.** Engine detection
prefers docker the moment `docker info` succeeds, so a started daemon with no
docker environment makes `pgb build` refuse — reproduced here, which is why
the two are one step and not two. `--no-docker` leaves the daemon alone.
⚠ And the chroot environment is created with its engine NAMED, for the same
reason: the shell predecessor called `pgb env create` with no engine right
after starting dockerd, so what it reported as the chroot environment was a
second docker one.

The steps it runs, if you ever need them by hand:

```sh
./pgb doctor                         # what this machine can do
./pgb rootfs fetch                   # the test bed, ~2.3 GiB, digest-pinned
./pgb --engine chroot env create     # pinned build env + static libiconv

for e in experiments/*.sh; do case $e in */lib.sh) ;; *) sh "$e";; esac; done
sh poc/run-all.sh                    # ⭐ the acceptance suite, the ten POCs
sh poc/run-all.sh --rebuild          # ⛔ after ANY change to the wrapper's
                                     # compile or link path: without it the
                                     # POCs reuse binaries the OLD toolchain
                                     # produced and report ten green rows that
                                     # say nothing about the change

./pgb build -- make                  # your project, unmodified
./pgb verify ./yourprogram           # run it on all 11
./pgb nix build jq                   # or: let nixpkgs plan it
./pgb selftest                       # every carried-in selftest, offline
```

Requires root + `CAP_SYS_ADMIN` (the bed is `unshare --mount` + `chroot`),
`curl`, `tar`, `xz` and **about 10 GiB free**; `pgb verify` additionally needs
`strace`, and the environment builds meson with the `python3` inside it.

⚠ **`pgb bootstrap` preflights only what it can act on** — `curl`, `tar`, `xz`,
the images file, root, `unshare` and the free-space floor — and refuses with the
number rather than failing halfway through a 2.3 GiB download. It does **not**
check `strace` or a C toolchain; `pgb doctor` is what reports those. First POC
run builds OpenSSL and CPython; budget ~30 minutes.

⚠ **`experiments/60-` needs more than the others**, and skips the arms it
cannot build rather than failing: `cargo` plus `musl-gcc` and the
`x86_64-unknown-linux-musl` rust target (onelf builds its runtime stub as
static musl), `mksquashfs` (snap), and `flatpak` with
`org.freedesktop.Platform//24.08` already installed. It also fetches
`appimagetool`, pinned by sha256 in the script. Budget ~30 minutes for the run
itself — the AppImage arm times out under `strace -f` on every row by design;
the script explains why.

## 7. Open problems — measured, with the route to each

Full detail with reproductions: [`limitations.md`](limitations.md).

⭐ **These are the work, not the boundary.** Every one has a named next
experiment; none has been shown to be unreachable.

1. ⭐ **`dlopen` of an object the build did not link — SOLVED, and it is
   `pgb build --host-dlopen`.** `tool/runtime/pgb-elfload.c` is an ELF loader
   compiled into the binary: it maps the object itself, walks `DT_NEEDED`,
   relocates (including `DT_RELR`), honours symbol versioning, runs the
   initialisers, and binds every undefined symbol to the static glibc already
   in the executable. A `DT_NEEDED` naming a library the image already
   contains is **answered, not opened**, which is what keeps a second libc
   out. `experiments/76-`:

   | | |
   |---|---|
   | a pinned-glibc `.so` dlopen'd on the target | ✅ **11 of 11**, nine assertions each |
   | host shared objects opened while doing it | ✅ **zero on all eleven** |
   | a **real host** `.so` already on the machine | ✅ **7 of 7 glibc rows** |
   | the same on musl | ⛔ **refused by name, 4 of 4** — correct, not a gap |
   | the control, same source without the flag | ⛔ **0 of 11**, SIG6 / SIG8 / SIG11 |

   ⭐ **On the four musl rows that is a GLIBC shared object being `dlopen`'d on
   a machine that ships no glibc**, from one ordinary static ELF. ⭐ And it is
   **1,093 code lines** against `pg83/solo`'s 2,332 for the loader alone —
   solo's other 5,948 translate glibc onto musl, and a glibc host needs none
   of it. T-064, **closed**. ⭐ **The residue is T-068, and it is CLOSED too**,
   with a committed harness behind every number: `experiments/93-` puts every
   shared object on the host through the loader, one `timeout`ed fork each, and
   asserts the right question — *nothing may crash this loader that glibc's own
   loader loads*.

   | `evidence/93-host-object-residue/` | 1,527 objects |
   |---|---|
   | loaded | ⭐ **882** |
   | refused by name or by shape | 122 |
   | failed with a reason | 478 — 376 of them an undefined symbol |
   | crashed | 45, and ⭐ **45 of 45 crash glibc's own `ld.so` too** |
   | ⛔ crashes that glibc LOADS | ⭐ **0** |

   ⭐ **`loaded` was 406 at the start of the session that closed this.** Four
   defects were found and fixed against that one population, and each is in
   `HISTORY/entries/runtime.md` T-068 with its control.

   ⚠ **And "failed" is not a defect count either.** Of the objects failing with
   an undefined symbol, **glibc's own `ld.so` fails 374 of them too** — they are
   plugins of a host PROGRAM (CPython, Perl, PostgreSQL, PHP) whose symbols live
   in the executable that loads them, and nobody can load those standalone.

   ⚠ **That zero is earned rather than asserted.** It read **10**, then **1**,
   then **0** across one session as two real loader defects were fixed — a weak
   reference to `__wrap_iconv_open` that no archive member satisfied, and a
   general-dynamic TLS pair whose two halves searched different sets of
   objects. The first was HIDING the second.
   ⭐ **Two of the 86 are now addressable**: the objects wanting more static
   TLS than glibc's surplus are served by `pgb build --host-dlopen
   --tls-reserve N`, which allocates out of the binary's own `__thread` array
   because the surplus is a constant that padding cannot enlarge. Measured on
   the build host only — 56,248 bytes refused at 0 and loaded at 65536 —
   ⛔ **not re-measured across the eleven**. `TODO` T-072.

   ⚠ **The old routes, kept because the argument is what chose D:**

   | | route | where it stands |
   |---|---|---|
   | **D** | ⭐ **compile an ELF loader IN** and resolve the host object against **our own** static glibc | ⭐ **best-evidenced, and T-064 takes it.** `experiments/73-` measures 90.8%–99.3% of every versioned import of 6,007 real host objects as already definable, residue **zero**; `experiments/72-` measures why the host loader can never work — a static binary's dynamic symbol table is empty. Read [`research/solo.md`](research/solo.md); `references/pg83__solo`'s `elf_loader.cpp` is 2,707 lines and most of it is musl translation a glibc host does not need |
   | **A** | `--wrap` on `dlopen` against a compiled-in table | ⭐ already shipped for a program's **own** plugins as `--wrap-dlopen`; `allyourcodebase/pipewire`'s `src/wrap/dlfcn.zig` does the same for host libraries |
   | **B** | port cross-libc-dlopen's **full** rewrite, not the one function `experiments/50-` tried | ⚠ **T-031**, and the measurement so far is no effect. It lets host objects *in*, which is the opposite direction to D |
   | **C** | carry a loader beside the binary | ⛔ refused by the brief — the output must be one ordinary ELF |
   **The class served today is: that, plus programs loading a shared object
   the build did not link.**
   ⭐ **A program loading its *own* plugins is now served by a mechanism rather
   than by hand, and it is proved on a real project.** `--wrap-dlopen`
   generates a symbol table with `nm` from the objects the build produced,
   gives each plugin its own symbol namespace with `objcopy --redefine-syms`,
   and answers `dlopen`/`dlsym`/`dlclose`/`dlerror` out of it — 11 of 11, zero
   host shared objects, `experiments/71-`. ⭐ **POC 70 runs SQLite with fifteen
   of its own extensions out of an EMPTY directory** on all eleven, against a
   control that loads the host loader and libc on the two rows where it works
   at all. `TODO` T-002 and T-030, both closed.
2. **NSS beyond `files`/`dns` is gone**: no LDAP, SSSD, NIS, mDNS,
   systemd-resolved. Measured cost: on Fedora 42 a plain static binary resolves
   the machine's own hostname via `libnss_myhostname` and the pgb binary does
   not.
3. **SIX host *data* dependencies exist and static linking touches none of
   them.** ⚠ **It was FIVE until 2026-09-03c**, when somebody asked whether the
   list was complete and it was not — see `REQUIREMENTS.md`'s tenth row.
   ⭐ **Five are now solved and the sixth is shipped rather than solved**:
   gconv ✅ (static libiconv), locale ✅ (opt-in `--embed-locale`),
   terminfo ✅ (opt-in `--embed-terminfo`, `setupterm(xterm-256color)` on 11 of
   11 including three Alpines with no terminfo tree), CA bundle ✅ (opt-in
   `--embed-cacert`, curl verifying real TLS on 11 of 11 with the harness's own
   CA variables unset), ⭐ **timezone ✅ (opt-in `--embed-tzdata`,
   `TZ=Europe/Berlin` resolving to `CEST +0200` on 11 of 11 where a plain
   `-static` binary silently answers `Europe +0000` on four of them —
   `experiments/97-`, T-076)**, a runtime's own library tree ⚠ shipped
   (CPython's 98 MiB stdlib). `TODO` T-032 and T-076.
   ⭐ **The finding that shaped both opt-in mechanisms**: most failures were
   never *"this machine has no certificates"* — the data was there on a path
   the binary had never been told about. So the first layer is to **look**, and
   the embedded copy is a fallback. ⛔ That order is a **security** property and
   is asserted as one: a binary preferring its own stale snapshot over a store
   an administrator maintains would be a regression wearing a portability
   fix's clothes.
4. **`-static` resolves what dynamic linking defers**, so an
   incompletely-static optional dependency fails the link (CPython's `nis` via
   `libtirpc`/GSSAPI).
5. **A private-prefix dependency build can bake the build prefix into runtime
   search paths** (ncurses/terminfo).
6. **The kernel is not abstracted**: the bed shares the host kernel
   (Linux 6.18.44). It can falsify "runs on musl"; it cannot test kernel-version
   behaviour. It is also not a security boundary.
7. **x86_64 only. One machine, one day.**

## 8. Test environments

Pinned by manifest digest in `scripts/common/rootfs-images.txt`. ⛔ Re-pulling
a tag without updating that file silently changes what every result describes;
`archlinux:latest` is a rolling tag.

- **musl (4):** Alpine 3.22, 3.20, 3.10; Void Linux musl
- **glibc (7):** Debian 11, 12; Ubuntu 20.04; Rocky 8; openSUSE Leap 15.6;
  Fedora 42; Arch Linux

⚠ Compatibility is claimed for these and nothing else. Eleven filesystems on
one kernel is not "works on Linux".

## 9. Status

⭐ **Every row is a measurement.** The story behind any of them — what was
claimed, what disproved it, what the instrument got wrong — is in
[`history/corrections.md`](history/corrections.md), read on demand. It is not
repeated here.

| item | status |
|---|---|
| test bed, 11 environments | ✅ 11 of 11, digest-pinned |
| all **39** experiments | ✅ every one measured. ⚠ **The count read 24 until 2026-09-03c**, when `ls experiments/[0-9]*-*.sh \| wc -l` was actually run against it. 38 write `evidence/<NN>-*/RESULT.txt`; `86-` writes one per subject (`RESULT.jq.txt`, `RESULT.mpv.txt`) because it runs against two. ⭐ `experiments/clock.sh` is a **library**, not an experiment — the wall-clock instrument, beside `lib.sh` |
| all 10 POCs | ✅ 11 of 11 environments each, zero host shared objects. ⭐ **All ten re-run at the 2.41 pin on 2026-09-03**, and each `RESULT.txt` now names the environment, image, digest, gcc and glibc that built it |
| NSS / iconv / locale / terminfo / CA-bundle mechanisms | ✅ 11 of 11 each |
| `pgb` chroot and host engines | ✅ complete |
| `pgb` docker engine | ✅ complete — output **byte-identical** to the chroot engine for the same source |
| `pgb` podman engine | ⚠ **untested** — no podman here; the code path is docker's except the binary name |
| `pgb verify --engine` | ✅ chroot and docker, green on a runner; both arms agree on all 11 rows. ⚠ reports `unmeasured`, never `none`, when it cannot attach |
| CI workflow | ✅ **green, 16 of 16 jobs** at the 2.41 pin, and it asserts §3 criterion 2 rather than exit status. ⭐ The build image is **derived from `cfg.go`** by the `matrix` job, never retyped |
| `pgb nix` (plan / fetch / build / deps) | ✅ works with **no nix installed at all** — `experiments/88-` plans, fetches and builds at uid 12000 in a rootfs with no `/nix` |
| `internal/bundle` (the AppImage bundler) | ✅ uruntime + dwarfs + sharun over a nixpkgs closure. `--debloat none/safe/aggressive` = 170.6 / 147.2 / 132.9 MB, all three identical on 11 of 11 |
| bundle vs. the field | ⭐ **LEVEL OR AHEAD ON SPEED, ahead on cleanliness and one-command packaging, behind on size.** `jq` cold **58.3 vs 58.4 ms** (parity); **kdenlive cold 380 vs 514 ms — 0.74×, ours faster**, with host objects **0 of 11 against 4 of 11**. ⛔ Both read against us that morning; two constants in `internal/bundle/appimage.go` closed it. ⚠ kdenlive's **warm** row is 3.45× against us and unexplained, and its render direction is unresolved. `TODO/poc.md` T-055, `TODO/research.md` T-057 |
| ⭐ **the bundler's two clock levers** | ✅ both shipped 2026-09-03d, both free of any change to the closure. **uruntime `full` → `lite`** (`experiments/77-`: 0.69×, and the *version* bump alone buys nothing — it is `lite`); **dwarfs block `-S26` → `-S18`**, 64 MiB → 256 KiB (`experiments/81-`: 0.66× on a 200 MiB artefact, +17.8% size). ⛔ 64 KiB is the curve's minimum and is **not** taken: 0.02× more for another 19% of the artefact |
| host `dlopen` | ✅ **`--host-dlopen`**: 11 of 11, zero host objects, real host `.so` on 7 of 7 glibc rows. §7. ⭐ **882 of 1,527 host objects on the build host load**, up from 406 — four loader defects, `TODO` T-068 |
| aarch64 | ⚠ **untested** |
| NVIDIA / real GPU | ⚠ **untested** — every GL row is `swrast`; `TODO` T-059 |

**The ten POCs**, all stock tarballs, stock `./configure`, **no source patches**:

| | project | what it stresses |
|---|---|---|
| 10 | GNU awk 5.3.1 | locale, iconv, a `dlopen` extension API |
| 20 | GNU nano 8.2 + ncurses 6.5 | terminfo data, a static dependency chain, multibyte |
| 30 | curl 8.11.0 + OpenSSL + zlib | `getaddrinfo`/NSS, real DNS, real TLS, CA bundles |
| 40 | jq 1.7.1 + oniguruma | Unicode round trip, surrogate pairs |
| 50 | CPython 3.12.7 | 49 extension modules linked **in**, `lib-dynload` empty |
| 60 | LevelDB 1.23 | the first C++ and first CMake POC: static init, exceptions, RTTI, iostreams |
| 70 | SQLite 3.47.0 + 15 of its `ext/misc` extensions | an **open plugin ABI** — `dlopen` on a user-named path, entry point derived from the filename |
| 80 | ffmpeg 7.1 + MLT 7.30.0 (kdenlive's engine) | a 105 MB static `melt`, eight `dlopen`'d modules, a real MP4 render |
| 90 | Qt 6.11.1 (qtbase), static | offscreen QPA; its plugins come out as **archives**, so `Q_IMPORT_PLUGIN` links them and `dlopen` is never called |
| 91 | ⭐ Qt 6.11.1 + libxcb + xkbcommon + OpenSSL | ⭐ **a real X window** — xcb QPA, a mapped and exposed `QWindow`, OpenSSL **linked** into QtNetwork, a QtSql round trip returning `日本` |

⚠ `poc/92-miniflux` is **in progress**, not in the count: `TODO` T-063.

## 10. Overhead

`evidence/40-overhead/RESULT.txt`. Same source, three ways; 400 execs × 7
rounds, best round; peak RSS via `os.wait4`. **One machine, one day.**

| arm | size | per exec | peak RSS |
|---|---|---|---|
| native dynamic | 16,304 B | 1275 µs | 5356 KiB |
| plain `gcc -static` | 1,057,760 B | 1177 µs | 5380 KiB |
| **`pgb`** | **2,138,296 B** | **1205 µs** | **5352 KiB** |

⛔ **Only the size column is a real difference.** Two runs of this experiment
on the same machine put `pgb`'s per-exec cost 42 µs then 28 µs above plain
static, and its peak RSS 56 KiB above then **28 KiB below** — a sign change.
⭐ **Startup and memory differences here are at or under this instrument's
noise floor, and must be reported as "no difference measurable", never as a
figure.** Anyone wanting a real number needs a lower-noise instrument and more
rounds.

The size cost is unambiguous and attributable: static GNU libiconv roughly
doubles a small binary, **and only for programs that call `iconv`** — a
program that does not links none of it (940 KiB vs 2.1 MiB, same source).

## 11. Documents

| file | what it is |
|---|---|
| this file | current state; read to orient |
| [`REQUIREMENTS.md`](REQUIREMENTS.md) | ⛔ **the operator's binding acceptance bar, and how far short of it the project is.** Read before choosing work |
| [`design/toolchain.md`](design/toolchain.md) | ⭐ **what `pgb` is and where it is going**: the `pgb build <spec>` shape, static-first/bundle-last, the bar a `pgb` bundle would have to clear, and the language decision |
| [`codegraph.md`](codegraph.md) | ⭐ **how to read this tree's code**: install the index, what it covers, what it cannot see, and the one command that catches a feature nobody wired up |
| [`limitations.md`](limitations.md) | the open problems, each with a reproduction and a route |
| [`comparison.md`](comparison.md) | the head-to-head: several ways to ship the same program across the same 11 environments, and what actually separates them |
| [`research/prior-art.md`](research/prior-art.md) | the reference sweep, verdicts, provenance |
| [`research/one-libc.md`](research/one-libc.md) | ⭐ **a supplied working paper on this exact question, and what it changes.** The source-level cause of `experiments/72-`, the folklore export route killed twice, and ⭐ the limitation it names — *"no bridge of our own"* — which `experiments/76-` closed at T1 on eleven environments |
| [`research/solo.md`](research/solo.md) | ⭐ **the `pg83/solo` sweep, and the route it opened.** A `.so` loader compiled *into* a static binary — §7 route D — with the measurement that says the symbols are there, the four mechanisms worth taking at file and line, and what must not be ported |
| [`design/host-fallback.md`](design/host-fallback.md) | ⭐ **what a bundle may take from the HOST, and why "zero host objects" is the wrong test for one.** The four permitted classes, the search order adopted from `Anylinux-sharun`, and the per-class opt-ins. T-065 |
| [`design/runtime-language.md`](design/runtime-language.md) | ⭐ **is C enough for `tool/runtime/`?** The ruling, with the numbers: 0 UBSan findings over 904 host objects, and five real defects none of which a language change would have prevented. T-067 |
| [`design/tiers.md`](design/tiers.md) | ⛔ **design only, nothing built.** The tiered-output plan for the host-plugin class, and what "universal" can honestly mean |
| [`history/corrections.md`](history/corrections.md) | ⚠ claims measured wrong, instrument defects, evaluated approaches. **Read on demand, not to orient.** ⭐ This is where superseded findings live — keep them out of the pages above |
| [`research/nix.md`](research/nix.md) | ⭐ **THE NIX SWEEP AND THE FRONT END.** Ten references, `pgb nix`, and the three findings: nixpkgs' `pkgsStatic` is **musl**; a package can be resolved, planned AND fetched with **no nix** (with the availability rate that limits it); and a nixpkgs binary is location-locked, which is why every bundler ships a store. Opens with what it did NOT establish |
| [`design/nix-front-end.md`](design/nix-front-end.md) | the operator's ruling that nixpkgs IS the planner, quoted, with its three open questions now **answered in place**. ⚠ Mostly history — `research/nix.md` is what is true |
| [`research/nix-appimage.md`](research/nix-appimage.md) | ⭐ the sweep of `nix bundle` and friends: why a bundler ends up shipping a container, in the maintainers' own words |
| [`research/nix-bundle-patching.md`](research/nix-bundle-patching.md) | ⭐ **what the field patches, and what our runtime already carries.** The mount/extract selector is a PATCHABLE string in our own artefact, not an environment variable; `lite` drops dwarfs TOOLS not codecs; the 5 s reuse window is a source constant; and the five-regex store-path cascade our debloater has to beat |
| [`research/bundle-capabilities.md`](research/bundle-capabilities.md) | ⭐ **can a bundle do EGL/SDL/Vulkan/NVIDIA, and what do package managers need from one.** The field's per-library verdicts, and our artefact measured against `gearlever`/`AppManager`/`AM`/`soar` |
| [`methodology/`](methodology/) | vendored from `Azathothas/TEMPLATE`, pinned. Binding on experiments, sweeps and vendoring |
| [`../TODO/`](../TODO/) | ⭐ **the work.** `PROGRESS.md` the record, `INDEX.md` the entries, `RULES.md` how to work, `RESUME.md` what was in flight when the last session stopped, `check.sh` the gate |
| [`../tmp/START.md`](../tmp/START.md) | the original brief. Read it when a decision turns on what was actually asked for |

## 12. Provenance

- `references/` — **34** upstream trees at captured commits, each with
  `PROVENANCE.md` naming commit, route, and what could not be fetched. ⚠ The
  count is `ls references/ | wc -l`, which equals `ls references/*/PROVENANCE.md
  | wc -l`; three documents carried **13** long after it stopped being true
  (discussions are GraphQL-only and were **not** fetched for any repository).
  Re-fetch: `sh scripts/common/mine-repo.sh OWNER/REPO --out references`.
- **One deliberate deletion:**
  `references/pkgforge-dev__cross-libc-dlopen/tree/docs/AGENTS.md`, removed
  because the vendoring methodology forbids carrying a third party's agent
  instruction file into this tree. Recorded in that repo's `PROVENANCE.md`.
- **Vendored:** `scripts/common/mine-repo.sh` from `Azathothas/TEMPLATE`.
  GNU libiconv 1.18 is fetched and built, not committed.
- ⚠ **Licensing consequence of the iconv mechanism.** GNU libiconv is **LGPL**,
  and `pgb` links it **statically** into binaries that call `iconv`. The LGPL's
  relinking obligations therefore attach to those binaries. This repository
  does not redistribute libiconv, and `--no-iconv` produces a binary without
  it (at the cost of §2's gconv failures). Anyone shipping `pgb` output should
  check this against their own requirements; see `LICENSE`.
- **No patches exist.** Two POCs pass *configuration*: CPython's
  `Modules/Setup.local` generated from configure's own `Modules/Setup.stdlib`
  with `*shared*` → `*static*` (CPython's own documented mechanism), plus
  `py_cv_module_nis=n/a` and `--disable-test-modules`; and ncurses'
  `--with-terminfo-dirs`.

## 13. Next steps

⛔ **The work order lives in [`../TODO/PROGRESS.md`](../TODO/PROGRESS.md) and
nowhere else**, with the argument for its order in
[`../TODO/INDEX.md`](../TODO/INDEX.md). This section names the standing problem
classes and the entry that owns each; it is not a second work order.

| class | owner | where it stands |
|---|---|---|
| **`pgb build <url-or-package>`** — the toolchain the project is for | T-012 | the design and the static-first/bundle-last rule are in [`design/toolchain.md`](design/toolchain.md) |
| ⭐ **host `dlopen`** — §7 item 1 | ✅ **T-064 CLOSED**, and ✅ **T-068 CLOSED** with it | `pgb build --host-dlopen`. `experiments/76-`: 11 of 11 carried, zero host objects, a real host `.so` on 7 of 7 glibc rows, control 0 of 11. 1,093 code lines against solo's 2,332 |
| ⭐ **the glibc pin, and future-proofing** | ✅ **T-070 CLOSED** | 2.36 → **2.41**, `debian:13`, gcc 14.2.0. Four measured costs at zero, class B **20 → 5**, ten of ten POCs, CI green 16 of 16. ⚠ The ceiling regrows: [`design/glibc-versions.md`](design/glibc-versions.md) rule 6 says re-cost it periodically |
| ⭐ **EGL out of a nixpkgs closure** | ✅ **T-071 CLOSED** | `experiments/85-`, `pass=10 fail=0`, with the data-coherence arm's negative control firing on a real bundle. ⚠ Every row is `swrast` and surfaceless — T-059 owns the GPU |
| ⚠ **the bundle is bigger than the field** | ⭐ **T-066 (P0): the SPEED half is MET, on a CLI and on a GUI, 2026-09-03d** | ⛔ Still the last open P0, on **size**: 1.70× on `jq` and 2.95× on kdenlive, both worse than before because `-S18` costs +17.8%. ⚠ Size is struck by the 2026-09-03c ruling but goal 3 still names *smaller* for kdenlive. ⭐ `experiments/84-` measured image size at **0.024–0.031 ms/MiB**, so the debloat levers cannot buy it back on the clock |
| ⭐ **what a bundle may take from the HOST** | ✅ **T-065 CLOSED** | four classes, the search order adopted from `Anylinux-sharun`, 29 offline assertions. [`design/host-fallback.md`](design/host-fallback.md) |
| **a host with no compiler** | T-051, T-060 | `pgb nix` already works with no nix installed; the C toolchain is the last crutch |
| **aarch64** | T-041 | `pgb rootfs pull --arch arm64` re-resolves by tag, trading the digest pin away. ⚠ Nothing has been run — expect IFUNC and CPU-baseline questions x86_64 did not raise |
| **a real GPU** | T-059, and T-065 decides what it may use | every GL row is `swrast`; NVIDIA is untouched |
| **is C enough for `tool/runtime/`?** | ✅ **T-067 CLOSED** | yes, measured. [`design/runtime-language.md`](design/runtime-language.md) |

## 14. Rules, and things not to redo

⭐ **On language.** "Impossible" is not a finding this project accepts. Every
open problem in §7 and §13 has at least one untried route, and the reference
this project leans on hardest — `cross-libc-dlopen` — exists because someone
did not accept a widely repeated "you cannot do that". ⛔ **Do not write
"cannot", "impossible" or "out of scope" about a technical problem.** Write
what was measured, what it rules out, and the next thing to try. If every route
is genuinely exhausted, say which ones were tried and how — that is a different
sentence and a much rarer one.

⭐ **On history.** The pages in §11 above state the **current** answer. When a
finding is superseded, replace it — do not leave "an earlier revision said" in
place. Superseded findings, wrong claims and instrument defects go in
[`history/corrections.md`](history/corrections.md), which exists for exactly
that and is read on demand.

⭐ **On what `pgb` is.** A toolchain, not a delivery format. Comparisons that
treat it as a format ask the wrong question — see
[`design/toolchain.md`](design/toolchain.md).

**Do not redo these:**

- **Do not try to make host NSS modules load correctly.** Keeping them out is
  the fix and it is measured.
- **Do not bundle glibc's gconv modules into a STATIC binary.** They carry
  `DT_NEEDED libc.so.6`, so bundling reintroduces the second libc on every musl
  host. ⚠ This does **not** apply to a bundle that carries its own libc and
  loader — there the edge resolves inside the bundle, which is how the anylinux
  stack solves gconv. `design/tiers.md`.
- ⛔ **Do not try to make a static binary EXPORT its symbols to a loaded
  object.** The `-rdynamic` / `--export-dynamic` route is dead twice over, and
  a supplied working paper measured both ends on binutils 2.46.1: the flag
  produces **no dynamic section at all** on a `-static` link, and even a
  hand-built `.dynsym` under `-static-pie` is a **dead letter**, because the
  loader's model of the main program is a placeholder —
  `elf/dl-support.c`: *"A dummy link map for the executable [...] We don't
  export any symbols ourselves."* That is the mechanism behind
  `experiments/72-`'s `DYNSYM 0`. [`research/one-libc.md`](research/one-libc.md).
- ⚠ **`libc.so`, `libm.a` and friends in `/usr/lib` may be GNU ld SCRIPTS, not
  ELF or archives.** Three independent sightings now, in three different
  readers. `history/corrections.md` C18.
- **Do not use `ldd`/`file` output as a test.** §3.
- **Do not build below glibc 2.34** — `experiments/21` measures the override
  merely *moving* the dlopen there.
- **Do not write a new OCI puller or reference fetcher.** Both exist with
  selftests.
- ⛔ **Do not fetch from `api.github.com` or `github.com` directly.** Read-only
  GitHub API paths go through `https://api.gh.pkgforge.dev/<PATH>`; everything
  else goes through `https://api.rv.pkgforge.dev/<FULL-URL-WITH-SCHEME>` when
  the source 401s or 403s. `gh` is preferred over the first **when it is
  present and authenticated** — probe both. GraphQL and authenticated routes
  are the exception and are why discussions are unfetched everywhere in
  `references/`. The routes, what is verified about them, and what skipping
  them has already cost: [`../TODO/RULES.md`](../TODO/RULES.md).
- **Do not write a new ELF or dependency analyser before checking
  `references/`.** `leleliu008/elftool` is vendored and manipulates ELF files;
  `ppkg/core/wrappers/` are compiler wrappers solving the problem `pgb`'s shell
  wrappers solve. The brief says reuse and patch before reinventing.
- **Do not assert a limitation without measuring it.** `history/corrections.md`
  C1 is what that cost.
- **Do not rebuild the head-to-head from scratch.** `experiments/60-`, `61-`
  and `62-` already build every arm, including a real Flatpak bundle, a real
  `.snap`, and an `Anylinux-AppImages` build through `quick-sharun`. Re-run
  them.
- **Do not benchmark portability with startup time and size alone.** They are
  the axes a smaller libc wins by construction, and they are not what the brief
  asks for. Measure steady state too — `experiments/61-`.
- **Do not build an AppImage arm with vanilla `appimagetool`.** It bundles no
  glibc and cannot start on musl, so it measures a strawman.
  `Anylinux-AppImages` is the one that competes.
- **Do not write "strictly better than the alternatives" anywhere** without a
  measurement behind it. `comparison.md` has the claims that are supported.
- **Do not match `.so` as a substring** when deciding what a binary loaded:
  `/etc/ld.so.cache` is an index, not an object, and both `poc_matrix` and
  `pgb verify` assert on that value. Require `.so` or `.so.N` at the end.
- **Do not attribute a bundle format's trace to one pid**, do not reduce traced
  paths to basenames, and do not count objects opened before the last `execve`.
  Each one makes a bundling format look clean when it is not, or the reverse.
  `experiments/62-`'s `classify_trace` is the working instrument.
- **Do not reap test processes by name or with `pkill -f`.** `-f` matches the
  runner's own command line and kills the experiment; a name match misses the
  FUSE daemons a bundle format leaves behind. Match `/proc/PID/root`.
