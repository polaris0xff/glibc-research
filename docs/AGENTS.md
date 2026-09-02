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
| malloc, 4 threads | **4.53 ns/op** | 584.71 ns/op |
| qsort | **93.20 ns/op** | 921.49 ns/op |
| strlen/strchr/strstr | **149.14 ns/op** | 1051.09 ns/op |

⭐ **The product, in one row:** on **Alpine**, where the ordinary choice is a
musl build, a `pgb` binary does that 4-thread allocator workload in
**4.68 ns/op** against musl's **592**. glibc's numbers on a machine that ships
no glibc — and `pgb` costs nothing to carry them: 0.99×–1.05× against plain
`gcc -static` on the same workloads.

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

A probe exercising NSS **and** iconv (`ci/probe.c`) fails on **all 11**.

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
| **own plugins** (opt-in `--wrap-dlopen`) | `-Wl,--wrap=dlopen,--wrap=dlsym,--wrap=dlclose,--wrap=dlerror` onto a table `pgb` **generates** with `nm` from the objects the build produced. ⭐ A program loading its *own* plugins never needs a loader — the code is in the link and `dlopen` is only doing a name lookup. Nothing is mapped, so no second libc can enter. 11 of 11, zero host objects, +544 B. ⚠ Not for **host** plugins — §7. | [`../tool/runtime/pgb-dlopen.c`](../tool/runtime/pgb-dlopen.c) |

**Delivery:** compiler wrappers on `PATH` plus `CC`/`CXX`. autotools, CMake,
meson and make pick them up unmodified. Each wrapper reads its own argv:
`-c`/`-E`/`-S`/`-M` = compile; `-shared` = **passed through untouched**, which
is what lets `./configure`'s shared-library probes still work; anything else =
executable link. `pgb explain` prints every injected flag.

**Build environment:** pinned `debian:12` (glibc 2.36) by manifest digest,
unpacked by `internal/ociimg` (`pgb rootfs pull`) and entered by `chroot`.
Verified not to be host
contamination: output `.comment` reads `GCC: (Debian 12.2.0-14+deb12u1)` where
a host build reads `Ubuntu 13.3.0`.

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
tool/runtime/*.c          the four mechanisms, and pgb-trace.c, the carried-in
                          tracer `pgb verify` uses where strace cannot follow
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
experiments/NN-*.sh       numbered; exit 0 matched, 1 did not, 2 could not run
docs/REQUIREMENTS.md      the operator's acceptance bar, and how far short
docs/methodology/         vendored, pinned; binding on experiments and sweeps
TODO/                     the work: PROGRESS, INDEX, RULES, RESUME, entries
TODO/check.sh             the gate; run before every commit
poc/common.sh             the POC contract
poc/NN-*/run.sh           the proof-of-concept projects
evidence/                 committed RESULT.txt per experiment and POC
HISTORY/<commit>/         ⛔ the shell and Python the Go port replaced. Kept
                          because it is the ORACLE every byte-identical
                          comparison was made against. Nothing here runs
references/               13 upstream trees + trackers, tracked, PROVENANCE.md
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
for p in poc/*/run.sh; do sh "$p"; done

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

1. **`dlopen` of a *host* object is host-dependent, and success is the worse
   outcome.** gawk's own extension **loads** on Debian 12 and Arch (dragging in
   the host loader and libc) and is refused on the other nine. The failures are
   inside glibc's loader — `_dl_call_libc_early_init` assertion,
   `elf_machine_rela_relative` assertion, SIGFPE — because a static binary has
   no loader of its own, so `dlopen` borrows the host's `ld.so` and
   `libc.so.6`, and *that pairing* breaks.
   ⛔ **This is T-064, a P0**, and the four routes are these. None has been
   tried to exhaustion; none has been shown closed.

   | | route | where it stands |
   |---|---|---|
   | **D** | ⭐ **compile an ELF loader IN** and resolve the host object against **our own** static glibc | ⭐ **best-evidenced, and T-064 takes it.** `experiments/73-` measures 90.8%–97.8% of every versioned import of 5,807 real host objects as already definable, residue **zero**; `experiments/72-` measures why the host loader can never work — a static binary's dynamic symbol table is empty. Read [`research/solo.md`](research/solo.md); `references/pg83__solo`'s `elf_loader.cpp` is 2,707 lines and most of it is musl translation a glibc host does not need |
   | **A** | `--wrap` on `dlopen` against a compiled-in table | ⭐ already shipped for a program's **own** plugins as `--wrap-dlopen`; `allyourcodebase/pipewire`'s `src/wrap/dlfcn.zig` does the same for host libraries |
   | **B** | port cross-libc-dlopen's **full** rewrite, not the one function `experiments/50-` tried | ⚠ **T-031**, and the measurement so far is no effect. It lets host objects *in*, which is the opposite direction to D |
   | **C** | carry a loader beside the binary | ⛔ refused by the brief — the output must be one ordinary ELF |
   **The class served today is: programs that do not need host plugins.**
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
3. **Five host *data* dependencies exist and static linking touches none of
   them.** ⭐ **Four are now solved and the fifth is shipped rather than
   solved**: gconv ✅ (static libiconv), locale ✅ (opt-in `--embed-locale`),
   terminfo ✅ (opt-in `--embed-terminfo`, `setupterm(xterm-256color)` on 11 of
   11 including three Alpines with no terminfo tree), CA bundle ✅ (opt-in
   `--embed-cacert`, curl verifying real TLS on 11 of 11 with the harness's own
   CA variables unset), a runtime's own library tree ⚠ shipped (CPython's
   98 MiB stdlib). `TODO` T-032, closed on `poc/20-nano` and `poc/30-curl`.
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
| all 23 experiments | ✅ every one measured; `evidence/<NN>-*/RESULT.txt` |
| all 10 POCs | ✅ 11 of 11 environments each, zero host shared objects |
| NSS / iconv / locale / terminfo / CA-bundle mechanisms | ✅ 11 of 11 each |
| `pgb` chroot and host engines | ✅ complete |
| `pgb` docker engine | ✅ complete — output **byte-identical** to the chroot engine for the same source |
| `pgb` podman engine | ⚠ **untested** — no podman here; the code path is docker's except the binary name |
| `pgb verify --engine` | ✅ chroot and docker, green on a runner; both arms agree on all 11 rows. ⚠ reports `unmeasured`, never `none`, when it cannot attach |
| CI workflow | ✅ **green, 16 of 16 jobs**, and it asserts §3 criterion 2 rather than exit status |
| `pgb nix` (plan / fetch / build / deps) | ✅ works with **no nix installed at all** — `experiments/88-` plans, fetches and builds at uid 12000 in a rootfs with no `/nix` |
| `internal/bundle` (the AppImage bundler) | ✅ uruntime + dwarfs + sharun over a nixpkgs closure. `--debloat none/safe/aggressive` = 170.6 / 147.2 / 132.9 MB, all three identical on 11 of 11 |
| bundle vs. the field | ⚠ **behind on size and speed, ahead on cleanliness** — §7 and [`comparison.md`](comparison.md) |
| host `dlopen` | ⛔ **known limitation** — §7 |
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
| [`research/solo.md`](research/solo.md) | ⭐ **the `pg83/solo` sweep, and the route it opened.** A `.so` loader compiled *into* a static binary — §7 route D — with the measurement that says the symbols are there, the four mechanisms worth taking at file and line, and what must not be ported |
| [`design/tiers.md`](design/tiers.md) | ⛔ **design only, nothing built.** The tiered-output plan for the host-plugin class, and what "universal" can honestly mean |
| [`history/corrections.md`](history/corrections.md) | ⚠ claims measured wrong, instrument defects, evaluated approaches. **Read on demand, not to orient.** ⭐ This is where superseded findings live — keep them out of the pages above |
| [`research/nix.md`](research/nix.md) | ⭐ **THE NIX SWEEP AND THE FRONT END.** Ten references, `pgb nix`, and the three findings: nixpkgs' `pkgsStatic` is **musl**; a package can be resolved, planned AND fetched with **no nix** (with the availability rate that limits it); and a nixpkgs binary is location-locked, which is why every bundler ships a store. Opens with what it did NOT establish |
| [`design/nix-front-end.md`](design/nix-front-end.md) | the operator's ruling that nixpkgs IS the planner, quoted, with its three open questions now **answered in place**. ⚠ Mostly history — `research/nix.md` is what is true |
| [`research/nix-appimage.md`](research/nix-appimage.md) | ⭐ the sweep of `nix bundle` and friends: why a bundler ends up shipping a container, in the maintainers' own words |
| [`methodology/`](methodology/) | vendored from `Azathothas/TEMPLATE`, pinned. Binding on experiments, sweeps and vendoring |
| [`../TODO/`](../TODO/) | ⭐ **the work.** `PROGRESS.md` the record, `INDEX.md` the entries, `RULES.md` how to work, `RESUME.md` what was in flight when the last session stopped, `check.sh` the gate |
| [`../tmp/START.md`](../tmp/START.md) | the original brief. Read it when a decision turns on what was actually asked for |

## 12. Provenance

- `references/` — 13 upstream trees at captured commits, each with
  `PROVENANCE.md` naming commit, route, and what could not be fetched
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
| ⛔ **host `dlopen`** — §7's limitation, and the reason `REQUIREMENTS.md` part 1 is not met | ⛔ **T-064 (P0)** | ⭐ build **our own** ELF loader and resolve against our own static glibc. `experiments/73-` measured 90.8–97.8% of host imports already definable by it, residue **zero**; `experiments/72-` measured why the host loader can never be the answer. [`research/solo.md`](research/solo.md) is the read; T-033 is the research note behind it |
| ⛔ **the bundle is bigger and slower than the field** | ⛔ **T-066 (P0)** | 2.86× on `jq`, 2.49× and ~3× slower on kdenlive. ⭐ Iterate on a **CLI**, not kdenlive. The reachability sweep exists and ⛔ nothing consumes it — the largest unused lever. T-055 folds into this |
| ⛔ **what a bundle may take from the HOST** | ⛔ **T-065 (P0)** | anylinux defers to the host **deliberately** for drivers, bundled-first with a documented lowest-priority fallback. "Zero host objects" is right for a static ELF and wrong for a bundle |
| **a host with no compiler** | T-051, T-060 | `pgb nix` already works with no nix installed; the C toolchain is the last crutch |
| **aarch64** | T-041 | `pgb rootfs pull --arch arm64` re-resolves by tag, trading the digest pin away. ⚠ Nothing has been run — expect IFUNC and CPU-baseline questions x86_64 did not raise |
| **a real GPU** | T-059, and T-065 decides what it may use | every GL row is `swrast`; NVIDIA is untouched |
| **is C enough for `tool/runtime/`?** | ⛔ **T-067 (P0)** | a question, not a migration — a measured "C is adequate" closes it |

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
