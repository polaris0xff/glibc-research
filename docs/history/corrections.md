# Corrections: claims this project made and then measured to be wrong

⚠ **Read on demand, not to orient.** `docs/AGENTS.md` is the current state.
This file exists so a later session can tell *how much to trust* that state,
and so a discarded idea is not re-derived.

Each entry: the claim, what disproved it, and where the guard now lives.

---

## C1 — "A static glibc binary cannot `dlopen` an extension"

**Claimed** while writing POC 10 (gawk), and asserted in the POC as a
requirement.

**Disproved by** pointing gawk at its own `filefuncs.so`, built by the same
build. It **loads** on Debian 12 and Arch Linux and is refused on the other
nine environments. The trace on the two that load shows the host's `ld-linux`
and `libc.so.6` entering the process.

⛔ **The assertion had also been passing for the wrong reason**: the POC was
built with `--disable-extensions`, so gawk had no `-l` option at all.

**Now:** the POC builds with extensions enabled, and the behaviour is recorded
by `poc_observe()` — measured per environment, never asserted.
`docs/limitations.md` §1.

## C2 — "The nano binary handles terminals"

**Claimed** because POC 20's functional test passed on all 11.

**Disproved by** a `setupterm()` probe linked against the same static
ncursesw: it failed on **all 11**, including the seven with a good
`/usr/share/terminfo`. ncurses compiles its terminfo search path in from
`--prefix`, so a private-prefix build looked under the build prefix.
`nano --version` never initialises curses, so the functional test could not
see it.

**Now:** `--with-terminfo-dirs` in POC 20's ncurses configure, and the probe is
part of the POC. `docs/limitations.md` §5.

## C3 — "curl verifies TLS on Debian and Ubuntu"

**Disproved by** noticing the same row reported `host-ca-bundle=none`. This
development environment routes HTTPS through a proxy and exports
`CURL_CA_BUNDLE`; `scripts/common/rootfs-run.sh` replicates that anchor into
the target so builds can fetch, and curl prefers it over its compiled-in path.
The probe was measuring the harness.

**Now:** POC 30's observation probe unsets `CURL_CA_BUNDLE`, `SSL_CERT_FILE`,
`SSL_CERT_DIR` and `CURL_CA_PATH`. The functional test deliberately does not,
because there the anchor is a means to testing DNS and the handshake.
`docs/limitations.md` §3.

## C4 — "`--embed-locale` works"

It produced a binary 361 KiB larger and behaved exactly as if unset, twice
over, for two different reasons:

1. options were parsed in the outer process and **dropped** when `pgb build`
   re-entered itself inside the chroot. Now carried as `PGB_OPT_*`.
2. the locale data symbols were **weak `const` definitions in the file that
   also read them**, so GCC constant-folded the count to 0 and the strong
   definitions bound to a symbol no code read.
   Reproduction: `nm pgb-locale.o | grep pgb_locale_nfiles` — `V` with no `U`
   reference means it was folded; `U` is correct.

A third defect in the same feature: the generator embedded top-level regular
files only, dropping the `LC_MESSAGES` **directory**. A glibc locale is a tree,
and a missing category fails the whole `LC_ALL` composite silently — it looked
like it worked on Debian only because glibc fell through to the host's own
`/usr/lib/locale` for the missing piece.

**Now:** `tool/runtime/pgb-locale.c` carries the reproduction beside the code.

## C5 — "The portable binary must read no host data"

`poc_matrix` asserted over a set that included gconv and locale **data** reads.

**Wrong, and undesirable.** glibc still opens `/etc/nsswitch.conf` under the
NSS override, and CPython reading Debian's `C.utf8` tree is correct behaviour.

**Now:** the property is **independence** — working whether or not the data is
there, which the four musl rows demonstrate. Host `.so` loads are asserted;
data reads are reported in their own column. `docs/AGENTS.md` §3.

## C6 — "glibc ≥ 2.34 is required for the NSS override"

⭐ **This one was reasoning, and the measurement CONFIRMED it** — recorded
here because it was unproven for most of the project's life and is the
justification for the pinned build image.

`experiments/21-glibc-version-floor.sh`, same source, same target
(Debian 11, which ships `libnss_files.so.2`):

| build glibc | arm | host NSS modules opened |
|---|---|---|
| 2.31 | plain | `libnss_dns.so.2`, `libnss_files.so.2` |
| 2.31 | **+ override** | `libnss_dns.so.2`, `libnss_files.so.2` |
| 2.36 | plain | none |
| 2.36 | **+ override** | none |

Below the floor the override **moves** the dlopen rather than removing it.

## C7 — "static musl ties pgb and beats it; pgb is not better than the alternatives"

⛔ **The worst claim this project has made, and it survived a full write-up
into `comparison.md`, `REQUIREMENTS.md`, `AGENTS.md` §1, `limitations.md` and
the README before it was caught.**

**Claimed** by `experiments/60-versus-alternatives.sh`, from two measurements
that were themselves correct: a static musl binary ran on 11 of 11
environments loading no host object, started in 160 µs against pgb's 980, and
shipped 447 KB against 2.1 MB.

**Wrong because the metric was wrong, not because the numbers were.** Startup
and size are the two axes on which musl wins *by construction* — it is a
smaller libc with a shorter startup path — so choosing them decided the answer
before the experiment ran. ⛔ **Nobody reaches for glibc to start faster.**
`tmp/START.md` asks for static binaries "using GLIBC **rather than MUSL** …
while avoiding the usual drawbacks", which makes musl the thing being avoided,
not a rival to be beaten on its own ground.

**Disproved by** `experiments/61-libc-throughput.sh`, same machine, same
compiler, libc the only variable — the axis that was never measured:

| workload | glibc static | musl static | musl slower by |
|---|---|---|---|
| malloc, 4 threads | 4.53 ns | 584.71 ns | **129×** |
| qsort | 93.20 ns | 921.49 ns | 9.9× |
| strlen/strchr/strstr | 149.14 ns | 1051.09 ns | 7.1× |
| malloc, 1 thread | 12.95 ns | 42.20 ns | 3.3× |
| snprintf | 344.42 ns | 989.67 ns | 2.9× |

And the row that makes it this project's result rather than a libc trivia
table: on **Alpine 3.22**, where the ordinary choice is a musl build, a `pgb`
binary does the 4-thread allocator workload in **4.68 ns** against musl's
**592 ns**. ⭐ That is the actual product — glibc's throughput on a machine
that ships no glibc — and pgb costs nothing to get it: measured against plain
`gcc -static` on the same workloads, 0.99×–1.05×.

**Also wrong in the same experiment, and separately:** the AppImage arm was
built with **vanilla `appimagetool`**, which does not bundle glibc and scored
2 of 11. `tmp/START.md` names `pkgforge-dev/Anylinux-AppImages` as required
reading precisely because it is the competitive one; built that way in
`experiments/62-`, the same program in an AppImage runs on **11 of 11**,
musl included, with zero host objects in the payload. The 2/11 was a fact
about the wrong tool.

**Now:** `experiments/61-` and `62-` exist, `comparison.md` leads with
throughput, and the claim that survives is narrower and defensible — pgb and
the anylinux stack both deliver glibc everywhere at glibc's speed, and what
separates them is shape, not portability or performance. ⚠ **The general
lesson is the one worth carrying:** an experiment that picks its metric from
what is easy to measure will confirm whatever that metric favours. Choose the
axis from the brief, not from the instrument.

---

## Instrument defects that corrupted a reading

| defect | symptom | fix |
|---|---|---|
| `strace -f` over the whole runner | the runner's `cp` appeared as seven host libraries the binary had not loaded | attribute by the pid that `execve`d the target — `exp_trace_opens` in `experiments/lib.sh` |
| `cp -a SRC DST` with DST present | nests instead of replacing; Rocky 8 reported "No module named 'json'" while Fedora's copy had grown to 192 MiB | `poc_stage_extras` removes the destination first |
| POCs appending to the target's `/etc/hosts` | six identical lines in `debian-12` after six runs | `poc_matrix` snapshots and restores `/etc/hosts` |
| `getrusage(RUSAGE_CHILDREN).ru_maxrss` for per-process RSS | a high-water mark across all reaped children; successive deltas read 10300, 128, 0 — an artefact of ordering | `os.wait4()`, which returns that child's rusage |
| a helper printing its table row to stdout **and** returning a value there | callers captured the formatted row as the value and compared two table rows | row to stderr, value to stdout |
| `libc.so.6` detection missing multiarch | Debian 11/12, Ubuntu 20.04 and openSUSE reported libc "unknown" | search `/lib/*/libc.so.6` too |
| the test bed's own `/etc/hosts` | Debian/Ubuntu base images have **no localhost line**; POCs assuming one failed | POCs create the name they resolve; bare `localhost` is an observation |
| matching `.so` as a **substring** of the path | `/etc/ld.so.cache` — an index, not an object, opened by every glibc process that reaches `dlopen` — was reported as a loaded shared object. It reached committed evidence: `evidence/poc/10-gawk/RESULT.txt` lists it on all seven glibc rows. ⛔ No recorded verdict was wrong, because a real object sits beside it on every one of those rows — but `poc_matrix` and `pgb verify` both **assert** on this value, so a binary that opened the cache and loaded nothing would have been failed over a file it only read | require the path to end in `.so` or `.so.N`: `experiments/lib.sh`, `poc/common.sh`, `pgb` |
| attributing a trace to the **one** pid that `execve`d the target | correct for a single static ELF and wrong for every bundling format, which forks, extracts and execs its payload in another process — whose library loads fall outside the filter, so the format scores a clean "loaded nothing" | follow `clone`/`fork`/`vfork` out of the target's pid, and report the payload process and the whole tree as separate columns — `trace_tree_libs`/`classify_trace` in `experiments/60-versus-alternatives.sh` |
| reducing traced object paths to **basenames** | a bundled `/root/.cache/onelf/<id>/lib/libc.so.6` is then indistinguishable from the host's `/usr/lib/libc.so.6`, so a bundle that correctly avoided the host libc reads as having loaded it | classify by path: under the target's own `/lib`, `/lib64`, `/usr/lib*` it is a **host** object, anywhere else it is the artefact's own **bundled** copy |
| `strace -f` on a program that leaves a helper running | strace does not return until every traced process exits, and an AppImage's FUSE helper does not exit with the payload. The run hung, and the tracee was left in ptrace-stop where SIGKILL to strace alone did not reap it | bound every traced run with `timeout`, retry without `-f`, and record that cell's tree column as **unmeasured** rather than clean |
| `pkill -f` to reap the artefact's leftovers | the pattern also appears in the **measuring shell's own command line** (`rootfs-run.sh … -- /pgb-vs-arm`), so the reaper killed the experiment | `pkill -x` against the process **name**, which only the artefact has |
| `poc_observe` **appending** to `observation.txt` | never truncated, so a second run of a POC left the file holding both runs back to back — 22 rows for an 11-environment matrix, the older half describing a binary that no longer existed. Found when re-running POC 10 to verify the `.so` fix above | `: > "$POC_OUT/observation.txt"` at the top of `poc_observe` |
| reaping test processes **by name** | an AppImage's uruntime leaves a DWARFS FUSE daemon running by design — a mount that outlives the program is what mount mode *is* — and its `comm` is `memfd:dwarfs`, not the artefact's. `pkill -x <artefact>` therefore reaped nothing: a full pass of `experiments/62-` left **22 daemons alive**, one per AppImage invocation, and the operator had to kill them by hand. ⛔ The experiment reported success while leaking | match on `/proc/PID/root`, which is the chroot a process is actually in, so every straggler of a cell is caught whatever it is called and nothing outside the bed can match. Plus an `EXIT`/`INT`/`TERM` trap, and a closing `exp_check` that **fails the experiment** if anything is still running in the bed |
| counting objects opened **before** the last `execve` | `execve` replaces the address space, so those are not mapped in the running program. An AppImage `AppRun` is a shell script, so one pid runs `AppImage → AppRun → /bin/sh → payload`, and on a distribution whose `/bin/sh` is dynamic the shell's libc, readline and ncurses were charged to the payload. The anylinux arm was reported as "payload clean 4 of 11" when the program itself had none of them mapped | clear the accumulated set at each successful `execve` in **payload** mode; never clear it in **tree** mode, because "what did the machine load in total" is a different and also real question |
| matching a fork as **one** trace line | strace interleaves: `vfork( <unfinished ...>` then `<... vfork resumed>) = 1234`. The child pid appears only on the second line, which does not contain `vfork(`. Requiring both `vfork(` and a trailing `= N` matched neither line, so every interleaved fork child was dropped from the followed set. ⛔ An anylinux AppImage that ran and passed was recorded as opening **no objects at all** — not one, bundled or host — which is impossible for a program that executed | match `NAME(` **or** `<... NAME resumed>`, and take the pid from whichever line carries `= N` — `classify_trace` in `experiments/60-` and `62-` |

---

## Approaches evaluated and refused

| approach | why refused |
|---|---|
| rewrite host NSS/gconv modules so they load into a foreign-libc process (the `cross-libc-dlopen` mechanism) | solves the opposite problem — it lets host objects *in*. Needs ELF surgery, version stripping, dependency-edge removal and a private on-disk copy per object, and requires a **dynamic** process. Not loading them is cheaper and complete. |
| bundle glibc's gconv modules beside the binary | every module carries `DT_NEEDED libc.so.6`, so this reintroduces a second libc on every musl host — the failure being avoided |
| bundled loader + AppDir (sharun shape) | needs a directory beside the binary; the brief requires a normal ELF |
| self-extracting single-file format (onelf shape) | the brief explicitly refuses a new packaging format |
| `LD_PRELOAD` interposer (anylinux.c shape) | needs a dynamically linked process; the output here has no interpreter |
| rewriting `PT_INTERP` to a relative path | onelf's `docs/guide/cross-libc.md` records that it breaks when the new path does not fit the original slot |
