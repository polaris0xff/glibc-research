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
## C8 — "The CI workflow has never run"

**Claimed** by `docs/AGENTS.md` §9 ("**WRITTEN, NEVER RUN** — no push has
happened from a runner yet"), by §13 item 2, and by `TODO/ci.md` T-040, which
called a green portable arm "a prediction, not a result".

**Disproved by** the GitHub Actions API. The workflow had run **ten times** by
the time it was read, on every push from run 1 (`79bbfa33`) to run 10
(`b77e0333`), and **all ten were red**. ⛔ Three tracked files asserted the
opposite of an observable fact about this repository, and one of them was the
first file a new session is told to read.

**What the ten runs actually contain**, which is the part the wrong claim cost:

| | |
|---|---|
| `build` job | ✅ green every time. `pgb --engine host build` produced a static probe, `readelf -d` counted 0 `NEEDED`. |
| matrix, 9 of 11 rows | ✅ green. `probe-portable` printed `PASSED: 0 failure(s)` on Alpine 3.22/3.20, Debian 11/12, Ubuntu 20.04, Rocky 8, openSUSE Leap, Fedora 42 and Arch. |
| the control, on Arch | ⛔ `Segmentation fault (core dumped)`, exit 139 — the positive control the local bed also reports, reproduced on somebody else's machine. |
| matrix, 2 of 11 rows | ⛔ red, and **the probe never ran on either**. |

⭐ **The two red rows are not a portability result. They are GitHub's Node.js
failing to start in a musl container**, before `actions/download-artifact` had
fetched anything:

```
voidlinux    exec /__e/node24/bin/node: no such file or directory
alpine-3.10  Error relocating /__e/node24_alpine/bin/node:
               pthread_getname_np: symbol not found
               secure_getenv: symbol not found
```

A job using `container:` has the runner inject its own Node to execute every
JavaScript action. That Node is dynamically linked, and the runner picks the
glibc build unless `/etc/os-release` says `ID=alpine` — so Void Linux gets a
glibc binary with no loader to run it, and Alpine 3.10's musl 1.1.22 predates
both symbols the alpine build needs.

⭐ **This is the project's own thesis, observed on the CI provider**, and it is
worth more than the green rows: a dynamically linked helper cannot be shipped
to an environment somebody else chose. `pgb`'s answer — run the helper where
its libc is, send a static binary into the target — is what the fix does.

**A second defect the same read found:** the matrix rows were **hand-written
tags** (`alpine:3.22`, `archlinux:latest`) while the local bed pins the same
eleven environments by manifest digest in
`scripts/common/rootfs-images.txt`. ⛔ CI and the local bed were two different
test beds reporting as one, and `archlinux:latest` is a rolling tag — the
exact hazard that file's own header warns about. Consistent with that, CI's
Arch row killed the plain control with **SIGSEGV** where the pinned local Arch
kills it with **SIGFPE**: the same experiment, a different Arch.

**A third:** the workflow's header claimed it was "how the docker/podman
engines in `pgb`, marked UNTESTED in docs/AGENTS.md, finally get exercised".
It never invoked either. The build job forces `--engine host`, and the matrix
used GitHub's `container:` feature rather than `pgb`. ⚠ And `pgb verify`
has no engine dispatch at all — `cmd_verify` calls `rootfs-run.sh` directly
and ignores `--engine` — so on a runner, which has no `CAP_SYS_ADMIN`, the
tool's own verification command could not run. Carried as `TODO` T-014.

**Now:** every job runs on the host and enters targets with
`docker run --entrypoint`, so the only process inside a target image is the
probe. The matrix is **generated** from `scripts/common/rootfs-images.txt`
and asserts it parsed eleven rows, so the two beds cannot drift again. The
build job asserts the two arms are different binaries — `pgb-runtime` present
in one and absent in the other — because every other assertion in the file is
made against the pgb arm, and a silently no-op `pgb` would otherwise produce
a green run. `TODO/ci.md` T-040.


## C9 — "the docker/podman engines are untested"

**Claimed** by `docs/AGENTS.md` §9: "**UNTESTED** — no daemon here; code
exists, never run. CI is where it first runs."

⚠ **The premise under it was wrong, not just the status.** The machine has
`docker` 29.3.1; it has no *init*, so nothing had started `dockerd`. Starting
it directly is one line, and the reference this session vendored says so and
says why probing with `docker --version` instead of `docker info` is what hides
it — `references/Aseem0xff__alloc-tests/tree/docs/containers.md` @ `efc84ab5`.
⭐ `pgb doctor` was already probing correctly with `info`, and reported
`docker  present but no daemon` truthfully all along. Nobody tried the next
step.

```sh
dockerd >/tmp/dockerd.log 2>&1 &
for i in $(seq 1 30); do docker info >/dev/null 2>&1 && break; sleep 1; done
```

**Running it found three defects in the first ten minutes**, each of which
`UNTESTED` had been carrying:

| | defect | how it presented |
|---|---|---|
| ⛔ **P0** | `cmd_build`'s docker branch ended `/bin/sh -c "$PGB_SELF/pgb __inner-build $*"`. `$*` joins argv with spaces and the inner `sh -c` re-splits it, so any single argument containing spaces is torn into words | `pgb build -- sh -c '$CC -O2 -o out/x x.c'` printed `sh: 0: Illegal option -O`, wrote **no output file**, and **exited 0**. A build that produced nothing reported success. The chroot branch never had this: it passes `"$@"` |
| ⛔ **P1** | the docker and podman engines carried no TLS trust anchor, where `scripts/common/rootfs-run.sh` replicates one into the chroot and explains why | the first `pgb --engine docker env create` died at `RUN sh /opt/pgb/build-libiconv.sh` with **exit 60** — curl's "unable to get local issuer certificate". `apt-get` had just succeeded in the same image because Debian's sources are `http`, so it reads as "libiconv is broken" |
| ⛔ **P1** | `--bind` passed its argument to `-v` unresolved | `docker run -v relbind:/mnt` does not mount `./relbind`. It creates an **empty named volume** called `relbind`, mounts that, and exits 0. Reproduced on docker 29.3.1 |

**Now:** argv is passed as argv, the anchor named by the caller's own
environment is carried in (and **only** that file — verification is never
disabled), and every bind source is made absolute for every engine.
`abs_bindspec()` and `ca_anchor()` in `../../pgb` carry the reasoning.

**Measured after the fixes**, and this is the part that makes the engine a
result rather than a claim: `pgb --engine docker env create` builds
`pgb-env:0.1.0`, and a binary built through it carries
`GCC: (Debian 12.2.0-14+deb12u1)` in `.comment` where a host build on this
machine carries `GCC: (Ubuntu 13.3.0-6ubuntu2~24.04.1)`. ⭐ The engine really
does build in the pinned environment rather than on the host, which is the
property `env create` exists for and which had never been checked for this
engine.

⚠ **What this does not change.** The chroot bed stays the test bed. Both
instruments were run against the same eleven digest-pinned environments in
this session and returned the same verdict — 11 of 11 correct, zero host
shared objects — so docker adds no portability signal chroot did not already
have, and every committed number was measured through chroot. What docker
adds is the two things chroot cannot reach: **these engines**, which chroot
cannot exercise by construction, and **CI parity**, because a runner has no
`CAP_SYS_ADMIN` and must use containers.

## C10 — "the chroot bed and a `docker run` of the same digest are the same environment"

**Claimed** implicitly, by deriving both `pgb verify` arms from the same
`scripts/common/rootfs-images.txt` and describing them as the same eleven
environments.

**Disproved by** running both arms on the same binary and reading the one cell
that differed. On Arch the docker arm reported the binary reading
`/usr/lib/locale/*` and the chroot arm did not.

⛔ **An OCI image is a filesystem PLUS a configuration, and `oci-pull.sh`
unpacks only the filesystem.** The `archlinux` image's config carries
`Env: LANG=C.UTF-8`, which `docker run` applies and a chroot unpack does not:

```
$ docker run --rm --entrypoint /usr/bin/env archlinux@sha256:818793c8… | grep LANG
LANG=C.UTF-8
$ grep -c 'LANG' scripts/common/rootfs-run.sh
0
```

So the binary takes a different `setlocale` path under the two beds. Same
digest, same files, different environment.

⚠ **What this does and does not invalidate.** It changes nothing about any
committed result: the difference appears only in the **host data** column,
which `docs/AGENTS.md` §3 states is reported and never asserted, and the two
arms agree on all eleven rows for both asserted columns — criterion 1 and
criterion 2. ⭐ It is recorded because the *claim* was too strong, not because
a number was wrong.

**Now:** the two beds are described as sharing a filesystem rather than an
environment, and `TODO` T-015 carries applying the image config's `Env` in
`oci-pull.sh` so they can be made to agree deliberately rather than by luck.

## C11 — "the docker arm and the untraced run report the same thing"

**Claimed** by T-014's acceptance, which asked only that the RESULT column
match between the chroot and docker arms — and it does, on the binary that
matters.

**Disproved, for the CONTROL binary, by** running it both ways twice each.
`docs/methodology/experiments.md` says to "check whether observing changed the
answer", and here it did:

| | untraced ×2 | traced ×2 |
|---|---|---|
| debian-11 | **SIG11** | **SIG6** |
| ubuntu-20.04 | **SIG11** | **SIG6** |
| debian-12 | SIG8 | SIG8 |

⭐ **Under `ptrace` the plain `gcc -static` control reaches `abort()`;
untraced it segfaults first.** Both are the same finding — the binary dies —
and no verdict moves, because criterion 1 fails either way and the portable
binary is `ok` on all eleven under both. ⚠ But a document that quotes a
specific **signal** is quoting an instrument as well as a binary, and
`docs/AGENTS.md` §2 does exactly that.

⛔ **Do not "fix" this by picking whichever signal reads better.** The
observer is real, it is reproducible, and the honest form is to say which
instrument produced a signal when one is quoted.

**Now:** §2's signals are the chroot bed's, and say so. The traced numbers are
`pgb verify --engine docker`'s and say so. ⭐ **The reason to keep both** is
that the disagreement is itself information: it is what found the
syscall-entry defect below, and what would find the next one.

## C12 — "a C++ build should work unchanged"

**Claimed** by `TODO/poc.md` T-001's premise, and marked as read rather than
measured: "⚠ Read, not measured: `pgb`'s wrappers pass `-shared` through
untouched and inject only at executable links, so a C++ build *should* work
unchanged. No C++ project has been built."

**Disproved by** building one. ⛔ **No C++ program linked at all**, and the
reason is nothing to do with `-shared`:

```
undefined reference to `__wrap_iconv_open'
... in .text._ZSt24__narrow_multibyte_charsPKcP15__locale_struct
```

libstdc++ calls `iconv` itself. `--wrap` rewrites those references to
`__wrap_iconv*` exactly as it does the application's — and the wrappers append
`pgb`'s flags to the **end** of the user's argv, after which the compiler
driver appends its own libraries. `gcc -###`: `-lpgbruntime` at 178,
`"-lstdc++"` at 180. An archive is scanned where it appears, so by the time
libstdc++ introduces those references the archive holding them is behind the
linker.

⚠ **It was invisible for five POCs** because all five are C, and C programs
that do not call `iconv` never introduce the reference.

**Now:** `-Wl,-u,__wrap_iconv_open` and friends for the **C++ drivers only**,
the same forcing technique already used for `pgb_runtime_anchor`. ⚠ The cost
is that a C++ program links the iconv shim whether or not it calls `iconv`;
the C property §10 measures is kept, re-measured after the change at
**1,008,152 bytes** for C and **2,160,440** for C++.

⭐ **The general lesson, which is not about C++:** every POC in the tree was an
autotools tarball, and that looked like a preference. It was a **constraint** —
the pinned environment contained no cmake, meson or autoconf (T-016), so
nothing else could be built. A status table full of green rows was describing
one build system's worth of evidence and reading as four. ⛔ When every
subject in a corpus shares a property, check whether that is a choice.
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
| `poc_functional_test` **undefined** | `poc_functional_test > script` writes an **empty file** when the POC does not define the function. `sh` on an empty script exits 0, so every row read `ok`, `poc_check` passed, and the trace of that empty script found no objects: ⛔ **eleven green rows having executed nothing at all.** Found by writing a POC that omitted it. ⚠ All five existing POCs define it, so no committed result was affected — the harness could have certified a bad POC and had not yet | `poc_matrix` refuses and counts a failure when the function is undefined — `poc/common.sh` |
| `poc_observation_probe` **undefined** | the same shape one function along: the probe failed per environment with "not found", and every row printed OUTCOME `<none>` and HOST OBJECTS `none` — indistinguishable, in the committed table, from eleven environments measured and found clean | `poc_observe` prints `NOT MEASURED`, says that is not the same as observing nothing, and counts a skip |
| counting objects opened **before** the last `execve` | `execve` replaces the address space, so those are not mapped in the running program. An AppImage `AppRun` is a shell script, so one pid runs `AppImage → AppRun → /bin/sh → payload`, and on a distribution whose `/bin/sh` is dynamic the shell's libc, readline and ncurses were charged to the payload. The anylinux arm was reported as "payload clean 4 of 11" when the program itself had none of them mapped | clear the accumulated set at each successful `execve` in **payload** mode; never clear it in **tree** mode, because "what did the machine load in total" is a different and also real question |
| matching a fork as **one** trace line | strace interleaves: `vfork( <unfinished ...>` then `<... vfork resumed>) = 1234`. The child pid appears only on the second line, which does not contain `vfork(`. Requiring both `vfork(` and a trailing `= N` matched neither line, so every interleaved fork child was dropped from the followed set. ⛔ An anylinux AppImage that ran and passed was recorded as opening **no objects at all** — not one, bundled or host — which is impossible for a program that executed | match `NAME(` **or** `<... NAME resumed>`, and take the pid from whichever line carries `= N` — `classify_trace` in `experiments/60-` and `62-` |
| reporting an open at syscall **entry** | the path is readable there but the RESULT is not, so every path the program merely PROBED FOR was counted as opened. Measured: the docker arm reported `/etc/nsswitch.conf` read on alpine-3.10, where that file does not exist. ⛔ On the criterion-2 column that is a **false positive**, not a cosmetic difference: glibc probes several paths for a shared object and takes the first that answers, so a binary that loaded nothing would have been failed for the ones that did not | stop at entry AND exit, hold the path from the entry stop, and report only when the exit stop shows a return >= 0 — `pgb-trace.c` |
| pairing ptrace entry/exit stops with a **bare toggle** | `execve` under `PTRACE_TRACEME` delivers an extra `SIGTRAP` stop that is indistinguishable from a syscall stop, so the toggle flipped one time too many at the very first syscall and every argument was then read at the exit stop and every result at the entry stop, for the whole run. ⛔ `/bin/true`, which unmistakably loads `libc.so.6`, was reported as opening **nothing at all** — the failure mode that reads as a clean binary | `PTRACE_O_TRACESYSGOOD`, so a syscall stop is `SIGTRAP\|0x80` and nothing else is, and the toggle cannot drift — `pgb-trace.c` |

## C13 — "part 2 of the acceptance bar is a comparison against other formats"

**Claimed** by `docs/REQUIREMENTS.md` as written, which discharged the
operator's directive in two parts and made the second one *"strictly better
than the alternatives, measured head to head"* against AppImage, Flatpak,
snap, onelf and static musl.

**Replaced by an operator ruling**, 2026-09-01b, quoted verbatim in
`REQUIREMENTS.md`:

> *"replace with per part claim, also anylinux is a bundle, our primary goal
> is still a static glibc binary that has none of the issues"*

⭐ **The reason is a category one, not a scoring one.** `Anylinux-AppImages`
is a **bundle** — it mounts or extracts a small distribution — and `pgb` is a
toolchain whose output is one ordinary ELF. `experiments/60-`, `61-` and `62-`
did the comparison and it stands as measurement: 11/11 for both, a tie on
throughput, `pgb` ahead on size and shape and behind on reach. ⛔ What changed
is that the tie no longer decides whether the bar is met.

⚠ **Nothing measured is withdrawn and no number moves.** `comparison.md` is
unchanged. Part 2 is now the enumerable list of things a static glibc binary
gets wrong — NSS, gconv, locale, networking, own plugins, C++ unwinding, CA
bundle, terminfo, host plugins — six closed and three open.

⛔ **This is the one kind of edit `REQUIREMENTS.md` forbids an agent to make
on its own, and it was not made on its own.** The page says so at the top; the
ruling is recorded in the page, in `TODO/RESUME.md` and here.

## C14 — "the acceptance for T-030 is CPython rebuilt on --wrap-dlopen"

**Claimed** by T-030's `Prove`, written when the entry was opened.

**Disproved by** `experiments/72-`: a static executable's dynamic symbol table
is empty, so the subject that acceptance needs — CPython with its modules left
as `lib-dynload/*.so` — cannot be built at all. The previous session proposed
a replacement and ⛔ **correctly refused to adopt it**, because it changes what
the entry closes on.

**Ruled on by the operator**, 2026-09-01b: the replacement is **accepted as
proposed**. T-030 now closes on a project whose plugin loading is not
configurable at build time, plugin directory emptied, functionality intact,
11 of 11. ⚠ The original `Prove` keeps its place in the entry: it is what the
entry was opened on, and 72- is why it moved.

## C15 — "pgb's compile flags are additions, so where they sit on the command line does not matter"

**Claimed** implicitly by `tool/lib/wrappers.sh` since the wrappers were
written, and stated in `pgb explain` as a flat list of flags with no ordering.

**Disproved by** `poc/90-qt`, on the first attempt to configure Qt 6.11.1. The
wrapper ran `exec "$REAL" "$@" $CF`, appending pgb's `-march=x86-64` **after**
the caller's argv, and **gcc takes the last `-march`**. So Qt's own intrinsics
probe, compiled by Qt as

```
c++ -march=cannonlake -mrdrnd -mrdseed -maes -msha -w -std=gnu++17 \
    -c config.tests/x86intrin/main.cpp
```

was silently downgraded to `x86-64` and every AVX-512 intrinsic in it failed:

```
/usr/lib/gcc/x86_64-linux-gnu/12/include/avx512fintrin.h:334:1: error:
  inlining failed in call to 'always_inline'
  '__m512i _mm512_setzero_si512()': target specific option mismatch
```

and configure stopped with:

```
ERROR: x86 intrinsics support missing. Check your compiler settings.
```

⭐ **The compiler settings were pgb's**, and the message points the user at
their own compiler. ⛔ **This is a class, not a Qt quirk**: every codebase that
compiles one translation unit per ISA level behind a runtime CPU check — Qt,
ffmpeg, mesa, x264, zlib-ng, glib — is built this way, and pgb was overriding
all of them. Where it did not stop the build outright it would have silently
disabled the dispatched code paths, which is the worse outcome because nothing
says so.

**Fixed** by making compile flags **lead** the command line, so they are
defaults the caller overrides, while link flags keep trailing because link
order is meaning rather than preference. ⭐ **The portability guarantee is
untouched**, because it was only ever about one flag: `-march=native` bakes in
the build machine's CPU. That one is now **rewritten** to the baseline in the
caller's own argv (and `-mtune=native`/`-mcpu=native` to `-mtune=generic`), so
an explicit `-march=<named cpu>` is honoured and `native` still cannot escape.

**Verified**, through `pgb build`, both directions in one run:

```
+ cc -march=cannonlake -c /tmp/marchtest.c -o /tmp/marchtest.o
CANNONLAKE-COMPILE-OK
+ PGB_VERBOSE=1 cc -march=native -c /tmp/marchtest.c -o /tmp/mt2.o
pgb[rewrote -march=native to x86-64]
pgb[compile] /usr/bin/cc -march=x86-64 -fno-plt -march=x86-64 -c ...
/tmp/marchtest.c: In function 'f':      <- AVX-512 correctly refused
```

⚠ **`-shared` mode is not rewritten**, because it is passed through untouched
by design so that `./configure`'s shared-library probes keep working.

---

## Approaches evaluated and refused

| approach | why refused |
|---|---|
| rewrite host NSS/gconv modules so they load into a foreign-libc process (the `cross-libc-dlopen` mechanism) | solves the opposite problem — it lets host objects *in*. Needs ELF surgery, version stripping, dependency-edge removal and a private on-disk copy per object, and requires a **dynamic** process. Not loading them is cheaper and complete. |
| bundle glibc's gconv modules beside the binary | every module carries `DT_NEEDED libc.so.6`, so this reintroduces a second libc on every musl host — the failure being avoided |
| bundled loader + AppDir (sharun shape) | needs a directory beside the binary; the brief requires a normal ELF |
| self-extracting single-file format (onelf shape) | the brief explicitly refuses a new packaging format |
| `LD_PRELOAD` interposer (anylinux.c shape) | needs a dynamically linked process; the output here has no interpreter |
| rewriting `PT_INTERP` to a relative path | onelf's own guide — vendored at `references/QaidVoid__onelf/tree/docs/guide/cross-libc.md` — records that it breaks when the new path does not fit the original slot |
