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
`CURL_CA_BUNDLE`; `HISTORY/6fcdb3630a1e342d6d4066aba2290e5cf10a84a7/scripts/common/rootfs-run.sh` replicates that anchor into
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
happened from a runner yet"), by §13 item 2, and by `HISTORY/entries/ci.md` T-040, which
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
a green run. `HISTORY/entries/ci.md` T-040.


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
| ⛔ **P1** | the docker and podman engines carried no TLS trust anchor, where `HISTORY/6fcdb3630a1e342d6d4066aba2290e5cf10a84a7/scripts/common/rootfs-run.sh` replicates one into the chroot and explains why | the first `pgb --engine docker env create` died at `RUN sh /opt/pgb/build-libiconv.sh` with **exit 60** — curl's "unable to get local issuer certificate". `apt-get` had just succeeded in the same image because Debian's sources are `http`, so it reads as "libiconv is broken" |
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
`internal/ociimg` so they can be made to agree deliberately rather than by luck.

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

**Claimed** by `HISTORY/entries/poc.md` T-001's premise, and marked as read rather than
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

**Claimed** implicitly by `HISTORY/6fcdb3630a1e342d6d4066aba2290e5cf10a84a7/tool/lib/wrappers.sh` since the wrappers were
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

## C16 — "onelf cannot pack our payload"

**Claimed** by `experiments/90-kdenlive-vs-enhanced.sh` for two sessions
running, in the second-worst form a claim can take: an arm that SKIPPED, with
the reason recorded against the competitor.

**Measured.** The arm skipped for two independent reasons, and neither of them
was onelf.

⚠ **The first was the machine.** onelf's runtime stub needs `musl-gcc` and the
`x86_64-unknown-linux-musl` rust target. Neither was installed, and the build
died at `error[E0463]: can't find crate for 'std'`. That is an honest skip and
the arm said so — but it hid the second reason behind it.

⛔ **The second was ours, and it is a one-character class of bug.** With the
toolchain installed, onelf built and then refused:

    error: entrypoint path 'bin/.kdenlive-wrapped' not found in directory

A nixpkgs wrapper leaves the real ELF beside itself as `.NAME-wrapped`.
`pgb bundle onelf-recipe` writes the recipe from a **readdir**, which returns
dotfiles, so it correctly named `bin/.kdenlive-wrapped`. The experiment staged
the directory with

    cp -al "$OURDIR"/shared/bin/* "$D/bin/"

and ⛔ **a shell glob never matches a leading dot**, so the two `-wrapped` ELFs
were silently dropped. The recipe named an entrypoint the packed directory did
not contain. The line immediately below it already used the correct `lib/.`
form, which is what makes this an oversight rather than a judgement.

**Now:** `shared/bin/.` , and the arm produces an artefact. The three-arm row
exists for the first time, and it is the only comparison in this tree that
isolates the PACKER — same payload, same 5,276 libraries, same zstd level:

| | P — ours | O — onelf, our payload |
|---|---|---|
| size | **477,191,058 B** | 595,859,196 B |
| startup, cold | **181 ms** | 597 ms |
| render | 3,559 ms | **2,068 ms** |

⭐ **The lesson is the one C-series entry that keeps recurring: a skipped arm
is not a result about the thing that was skipped.** This is the SECOND defect
in this same arm blamed on onelf — the first was our symlink dispatching on
argv[0] (`TODO/poc.md` T-055). Both times the instrument was wrong and
the competitor carried the verdict.

---

## C17 — "a loader that reports success has relocated the object"

**Then:** `pgb-elfload.c` read `DT_RELA` and `DT_JMPREL` and treated that as
the relocation set. On nine of eleven environments it was.

**The disagreement:** `experiments/76-`'s native arm was **SIG11 on exactly
Fedora 42 and Arch and nowhere else**. The loader's own trace said why in one
line:

```
pgb-elfload:   init_array[0] 0x670
```

where every working row printed a mapped address. `0x670` is the **unrelocated
vaddr**.

**Now:** those distributions build with `ld -z pack-relative-relocs`, which
compresses the `R_X86_64_RELATIVE` entries — the overwhelming majority of any
shared object's relocations — into a bitmap under **`DT_RELR`**. A loader
reading only `DT_RELA` finds almost nothing to apply, **reports success**, and
hands back an object whose pointers still hold link-time offsets.

⛔ **The lesson is the failure MODE, not the missing tag.** This was not a
crash the loader could report; it was a silent wrong answer that only became
visible because a constructor happened to be called through one of the
unrelocated pointers. Had `libz` been the only test object, the tag would still
be unimplemented and the loader would still look correct.

---

## C18 — "a linker script in `/usr/lib` is an archive"

**Then, TWICE.** `docs/research/solo.md` already warned that this trap fired
once. It fired again in a new instrument: the provider-table generator read
`/usr/lib/x86_64-linux-gnu/libm.a` with `readelf`, got **zero symbols in
silence**, and produced a table of 4,891 names instead of 7,216 — every maths
symbol missing.

**The disagreement:** the 904-object sweep failed on `pow@GLIBC_2.29` for a
third of the objects it tried, and `readelf -sW libm.a | grep pow` returned
nothing at all.

**Now:** `libm.a` is `GROUP ( libm-2.39.a libmvec.a )`, ASCII text.
`elfx.ExpandLinkerScripts` resolves it, and `internal/elfx/provider.go` carries
the note so a third occurrence is a documentation failure rather than a
discovery.

⚠ **`elfx.DefinedExternalSymbols` still does not expand ld scripts**, and that
is deliberate: it reads objects a *build* produced, which are never scripts.
The two readers are separate because their inputs are.

---

## C19 — "forking per sample makes each measurement cold, which is what we want"

**Then:** the first time-to-first-symbol measurement forked for every sample so
each `dlopen` would be a genuine cold load, and reported:

| | best of 200 |
|---|---|
| the compiled-in loader | 673,989 ns |
| the host `ld.so` | 64,484 ns |

⛔ **Ten times slower**, and it would have gone into T-064's entry as a failure
against the "faster to first symbol" half of its bar.

**The disagreement:** loading two *different* objects in one process gave
84,130 ns and 41,118 ns — the same order as `ld.so`, not ten times it.

**Now:** the fork was the measurement. The subject is a 4.4 MB static binary
with a 7,216-entry provider table in `.data.rel.ro`; its child pays
copy-on-write faults that a 16 KB dynamic binary never does, and the clock
starts after the fork but the faults land inside the timed region. The arms
were not comparable at all.

⭐ **The lesson: a control that differs from the subject in a way the
instrument touches is not a control.** `experiments/78-` and `76-` now measure
two loads in one process, and `76-` carries the caveat in its own output.

---

## C20 — "a debloat rule that passes the integrity check is safe"

**Then:** the reachability sweep was wired into `debloat()`, every DT_NEEDED in
the bundle still resolved, `jq` got 57.4% smaller and answered its workload
byte-identically on every arm.

**The disagreement:** `experiments/90-` — ours rendered **0 bytes of MP4**
where the previous run rendered 4,149.

**Now:** `Build()` writes `.env` **after** `debloat()`, so the sweep read a
file that did not exist, saw no plugin directories, and classified kdenlive's
MLT modules as unreachable. They are loaded by name at run time through
`MLT_REPOSITORY` and nothing links against them — which is precisely the case
the sweep reads `.env` to learn about. `DropUnreachable` now runs after
`writeEnv` and before `integrity`.

⛔ **The integrity check could not have caught this**, and that is the point: it
asserts that every `DT_NEEDED` resolves, and a deleted plugin has no
`DT_NEEDED` pointing at it. A structural check is only as wide as the structure
it knows about.

⭐ **And the methodology lesson is worth more than the fix.** `TODO` T-066 says
to iterate on a CLI instead of kdenlive, and that was right — four measured
iterations in the time one kdenlive build takes. But **`jq` did not catch this
and could not**: a CLI with no plugin directories has nothing at risk. The fast
subject is for iterating; the plugin-heavy one is the control, and it has to
run before a size number is believed.

---

## C21 — "the plain -static control fails on all 11"

**Then:** `docs/AGENTS.md` §2 stated it flatly, and CI's control steps carried
`continue-on-error: true` so that the control could die without failing the
build.

**The disagreement, and it took two defects to surface:**

1. ⛔ **The exit code was never recorded.** The step's comment said *"Its exit
   status is recorded, never asserted"*, and the recording did not happen:
   GitHub's default shell for `run:` is `bash -e`, so the script aborted at the
   failing `docker run` and the `echo "plain -static exit=$?"` after it never
   executed. Reproduced: `bash -e -c 'set -x; false; echo "exit=$?"'` prints
   nothing. **So nobody had ever seen the control's exit code on any row of any
   run.** The red annotation was its only trace, and an annotation says a step
   failed, not how.
2. Replacing `continue-on-error` with an assertion made the codes visible for
   the first time, and CI reported: **`the plain -static control PASSED on
   debian-12`**.

**Now:** the claim is instrument-dependent and §2 says the mechanism itself,
one paragraph above the sentence that was wrong: the iconv failure lands *"where
the host gconv path matches the build's"*, and is *"11 of 12 encodings silently
unavailable"* where it does not. The chroot bed builds and runs in the same
place; **CI builds on the Ubuntu runner and runs in a debian-12 container**, so
the paths do not match, the iconv arm degrades quietly, and the probe exits 0.

⭐ **Both halves are corrections and the second is the smaller one.** §2 now
names its instrument. But the finding that matters is the first: a step that
claimed to record something recorded nothing, for the entire life of the
workflow, and `continue-on-error` made the silence look deliberate. The control
now records its code on every row and warns when it is zero.

⚠ **And the assertion that replaced it was ALSO too strong to start with** —
it failed the build on that first passing row. A control whose behaviour is
instrument-dependent is recorded and surfaced, never gated on.

---

## C22 — "the build environment is pinned at glibc 2.36", and the numbers taken there

**Superseded 2026-09-02f, T-070.** The pin moved to `debian:13` / **glibc
2.41**, gcc 12.2.0 → 14.2.0, with all four measured costs at zero. The pages in
`docs/` carry the 2.41 reading; this is the 2.36 one, kept so a number quoted
from an older commit can be placed.

| | at the 2.36 pin | at 2.41 |
|---|---|---|
| `experiments/73-` provider symbols | 7,074 | **7,566** |
| class B, distinct symbols | **20**, 14 of them `__isoc23_*` at `GLIBC_2.38` | **5**, at `GLIBC_2.42`/`2.43`, `archlinux-latest` alone |
| opensuse-leap-15.6 served | 993 (97.9%) | **1005 (99.1%)** |
| fedora-42 served | 961 (97.8%) | **976 (99.3%)** |
| archlinux-latest served | 1198 (94.3%) | **1213 (95.5%)** |
| debian-11 served | 905 (93.4%) | 905 (93.4%) |
| ⚠ debian-12 served | 851 (94.6%) | **849 (94.3%)** — the one measured cost |
| ubuntu-20.04 / rockylinux-8 | 893 / 1049 | unchanged |
| class C, class E | empty on all 11 | empty on all 11 |
| POC 10's host-extension `dlopen` | LOADED on **Debian 12 and Arch** | LOADED on **Fedora 42** alone |

⛔ **AND A NUMBER THAT DISAGREED WITH ITS OWN TABLE.** `docs/research/solo.md`
said *"5,807 objects across the seven glibc ones"* directly above a table whose
seven glibc rows sum to **6,007** — and the eleven rows to **6,392**. The
figure had been copied into `limitations.md`, `REQUIREMENTS.md`, `AGENTS.md`
and two `TODO/` entries. ⚠ Its companion range, *"90.8%–97.8%"*, was wrong in a
second way: 97.8% was fedora's, while opensuse's 97.9% was the actual maximum.
Both corrected against the file the numbers come from.

⭐ **What this cost, and it is the reason T-070's landing has three steps.** The
name `pgb-env-debian12` had **nine** copies in code — eight experiments plus
`cfg.go` — and the digest **two** more in CI, one of them an `env.BUILD_IMAGE`
that nothing had ever read. Changing `cfg.go` alone would have left every one
of them measuring the old glibc **and saying nothing**. `TODO/check.sh` now
fails on a copy.

---

## C23 — the bundler's MILLISECONDS, quoted from evidence files that were later overwritten

⛔ **Found by deep review 1 on 2026-09-03c, hours after the operator made those
milliseconds the entire acceptance bar** — *"acceptable as long as ours
performs better"*. Every size figure in the record re-derives; the timing
figures do not.

**Claimed**, by `TODO/toolchain.md` T-066 and `TODO/poc.md` T-055, each citing
`experiments/90-`:

| entry | render | cold start | artefact |
|---|---|---|---|
| T-066 | 4,947 vs 2,033 ms | 300 vs 61 ms | 471,033,944 B |
| T-055 | 3,625 vs 2,001 ms | 3,344 vs 1,325 ms | 397,903,295 B |

**Disproved by** reading the cited file. `evidence/90-kdenlive-vs-enhanced/RESULT.txt`
in the tree today says **24,074 vs 13,680 ms render and 5,941 vs 1,183 ms
cold** — neither entry's numbers, and it is the run T-066's own prose
disclaims as contaminated. The quoted numbers are real; they are in
**superseded versions of that same file**:

    git show 68be1bcd:evidence/90-kdenlive-vs-enhanced/RESULT.txt   -> T-066's
    git show 0d4a2a94:evidence/90-kdenlive-vs-enhanced/RESULT.txt   -> T-055's
    git show 572e9b77:evidence/90-kdenlive-vs-enhanced/RESULT.txt   -> the tree's

⛔ **The gate could not catch it.** `check-docs.sh` asserts that cited evidence
*is in the repository*; all three runs write the same path, so an overwrite is
invisible to it. This is C5's shape — *"a committed evidence file described a
build configuration that no longer exists"* — one level down: the file is
current, the **run** is not.

⭐ **AND THE SPREAD IS THE REAL FINDING.** Four runs of the same comparison, the
same two artefacts, the same machine:

| run | ours cold | theirs cold | ratio | ours warm | theirs warm |
|---|---|---|---|---|---|
| `0d4a2a94` | 3,344 | 1,325 | 2.52× | 139 | 34 |
| `68be1bcd` | **300** | **61** | 4.92× | 239 | ⚠ **82 — above its own cold** |
| `572e9b77` | 5,941 | 1,183 | 5.02× | 337 | 129 |
| `run.log` | 181 | 52 | 3.48× | ⚠ **221 — above its own cold** | 48 |

⛔ **A 20× spread in the absolute cold figure and warm above cold in two of the
four runs.** The direction is consistent and survives — **ours is slower on
every run** — but the magnitude is not pinned, and "warm is slower than cold"
is not a load artefact, it is the instrument's cold/warm distinction
collapsing on this subject.

⚠ **The same overwrite hit `experiments/86-`, and there the committed evidence
is BETTER than the entry.** T-057 quotes *"cold 162–198 ms vs 79–107 ms, about
1.9×"*; `evidence/86-bundler-vs-anylinux/per-environment.jq.txt` carries eleven
environments × two arms, each a mean of five, and re-derives as **P 128–149
(mean 139), A 62–74 (mean 67) — 2.07×**, with warm **14.9 vs 10.8 — 1.38×**,
which is the one figure the entry got right. The entry's cold range appears in
no version of that file; 162 ms is the **build-host** figure from the same
entry's own prose, compared against an eleven-environment competitor number.

**Now:** T-066, T-055 and T-057 cite the commit their numbers came from, not
just the path. ⭐ And `PROGRESS.md` N1 — *re-measure every lever on the clock* —
is ordered **after** fixing the instrument. ⛔ **The mechanism was found the
next day and it is C24; the last sentence of this entry was wrong.** It read
*"`experiments/86-`'s per-environment method (eleven rows, mean of five, cold
by a fresh copy) is the shape to carry into `90-`"* — and *cold by a fresh
copy* is precisely the defect.

---

## C24 — "a fresh copy is cold by construction", and it is a warm start

⛔ **This is C23's mechanism, found by `experiments/99-` on 2026-09-03d.**
C23 recorded that four runs of the kdenlive comparison gave cold-start ratios
spanning 2.52×–5.02× with **warm above cold in two of them**, and named the
cause as *"the instrument's cold/warm distinction collapsing on this
subject"*. It did not say why it collapses.

**Claimed**, in `experiments/90-`'s own comment, and repeated in C23:

> *"A cold mount is obtained WITHOUT killing anything, by giving the cold run
> its own copy: uruntime keys its mount on the image, so a file nothing has
> run before is cold by construction."*

**Disproved by** `experiments/99-`, which measures the three states side by
side in one interleave, median of nine, with an A/A control:

| protocol | median | ratio to warm | resolves |
|---|---|---|---|
| `90-`'s fresh copy | 12.7 ms | **1.02×** | ⛔ **no** |
| the live mount reaped first | 85.0 ms | 6.80× | ✅ yes |
| warm, a mount alive by construction | 12.5 ms | — | — |

⭐ **uruntime keys its mountpoint on the artefact's CONTENT, not its path**, and
leaves the mount alive for a few seconds after the process exits. A
byte-identical copy therefore reuses it, so `cold_of()` measured a warm start
whenever anything had run the same bytes recently — which, in `90-`, the
render step immediately before it always had.

⭐ **AND THE HIDDEN VARIABLE IS THE CLOCK ON THE WALL.** `99-` section 1: the
same file, the same command, differing only in how many seconds have passed:

| gap | 0 s | 2 s | 4 s | 6 s | 10 s |
|---|---|---|---|---|---|
| cold start | 13.2 ms | 14.3 | 13.5 | **82.2** | 89.0 |

The mount is torn down between 4 s and 6 s after the last run — **6.24×,
decided by nothing but elapsed time.** Two measurements of the same state
differ only by noise, and noise has a sign; that is why warm came out above
cold in half of C23's runs.

**Now:** `experiments/clock.sh` is the instrument — median of N, arms
interleaved with a rotating start, and an **A/A control**, one artefact under
two names through the identical protocol, whose ratio is the floor below which
no row may be believed. `99-` **asserts** that the A/A pair does not resolve.
The cold protocol reaps the live mount by its mountpoint path first.
⚠ `90-` still carries the old `cold_of()` and its committed numbers are still
the ones C23 disclaims — `TODO/toolchain.md` T-066 owns it.

---

## C25 — "a trace line containing ENOENT is the whole of a failed open"

⛔ **The classifier that decides "zero host shared objects" counted FAILED
opens as loads**, and the shape is `strace`'s, not the bundle's. A long call is
split across two lines:

```
openat(AT_FDCWD, "/usr/lib/x86_64-linux-gnu/libGLX.so.1", O_RDONLY <unfinished ...>
<... openat resumed>)                   = -1 ENOENT (No such file or directory)
```

The **path** is on the first line and the **result** on the second. Every copy
of the classifier filtered with `!/ENOENT|= -1/`, which the first line
satisfies — so the probe was recorded as an object the program had loaded.

⭐ **Found by a disagreement, not by reading**, which is this tree's usual
route: `experiments/64-` reported **2 host shared objects** for a galculator
bundle on `alpine-3.22` — a musl image with no `/usr/lib/x86_64-linux-gnu` at
all. Both entries were `libGLX.so.1` probes that returned ENOENT.

⚠ **It surfaced only after T-081 lengthened the library path.** `copyLibraries`
had carried nine named subdirectories of `lib/` and now carries every one, so
`sharun --gen-lib-path` writes more entries, glibc tries each under
`glibc-hwcaps/x86-64-v{4,3,2}`, and a run that made a few hundred failed opens
now makes a few thousand — which is what made a split line likely.

⭐ **THE ERROR ONLY EVER RUNS ONE WAY.** It can turn a clean row dirty; it can
never turn a dirty row clean. So every committed **zero** stands, and this
correction retracts no published verdict. ⛔ A committed **non-zero** may be
inflated, and the claim at risk is named: `comparison.md` and `docs/AGENTS.md`
§9 quote *"host objects 0 of 11 against 4 of 11"* for kdenlive, and that 4 is
the competitor's count from `experiments/90-`.

**The fix** is one implementation instead of seven: `experiments/lib.sh`'s
`exp_classify_trace` pairs a split `openat` with its own `<... openat resumed>`
line by pid and records the path only when the result is not an error.
`experiments/64-` and `65-` use it. ⚠ The other **six** copies carry a comment
naming this correction and are converted and re-run by **T-084**.

⛔ **This paragraph said "nine" and "seven" on 2026-09-03f and both were
wrong** — corrected 2026-09-04 by counting `classify_trace()` definitions
rather than trusting the sentence. There are **six** hand copies (`60-`, `62-`,
`85-`, `86-`, `89-`, `90-`) and one shared implementation. ⚠ `77-` was named
among them and has no `strace` and no classifier at all. ⛔ **And the
conversion is not the deletion T-084 first described**: all six take a `mode`
argument — `payload` or `tree` — that `exp_classify_trace` does not implement,
so the shared function has to grow it before anything can be deleted.

---

## C26 — a window budget carried across a change of delivery mode, and a corpus with no positive control

⛔ **`experiments/65-` scored `galculator` 0 of 11 on 2026-09-04. Two days
earlier `experiments/64-` had measured the same subject, the same bundler and
the same eleven environments at 11 of 11, twice.** The corpus was stopped after
its first row rather than allowed to produce twenty-five more.

**The mechanism, and it is a copied constant.** `experiments/64-` waits
`WIN_WAIT=25` seconds for a window in **mount** mode, where a bundle starts in
about two seconds, and **150** seconds for the one arm it runs in **extract**
mode. `experiments/65-` runs **every** subject in extract mode — `strace`
deadlocks on the dwarfs FUSE mount, `experiments/64-` arm P measured that
twice — and kept the mount-mode 25.

⭐ **MEASURED RATHER THAN REASONED ABOUT.** With the machine otherwise idle,
`mousepad`'s bundle put its first toplevel on the X server at **t+21 s** on
`alpine-3.22`, unpack included, under `strace`. That is **four seconds** inside
a budget the row also had to share with a 195 MB copy into the rootfs and the
teardown of the row before it.

⛔ **The header comment had the number and nobody believed it enough to act on
it**: it estimated extract mode at *"ten to twenty seconds per row"* — beside a
25-second budget. An estimate written next to a constant it invalidates is a
correction waiting to be made by a measurement.

⭐ **The fix is to stop timing with a constant.** The window poll now ends when
a window appears, when the process exits, or when `timeout` kills the tree — so
the budget is the run budget, and a number chosen separately from the thing it
times cannot drift away from it again.

⭐ **AND THE REAL DEFECT IS THE MISSING CONTROL, WHICH IS WHY THIS IS A
CORRECTION AND NOT A TUNING NOTE.** The corpus pre-registered five expectations
and none of them was a positive control, so a broken instrument produced a
publishable-looking table: `0/11` in the pass column beside `11/11` in the
clean column, no error text, and a store-path report that proved the bundle had
run. ⛔ **Nothing in the experiment could tell a broken subject from a broken
instrument** — which is `AGENTS.md` §0b's rule, applied to the harness instead
of to the code.

**C6 was added and both of its failure modes were planted before it was
believed**: a control row at 0 of 11 turns the verdict red and names the
instrument as the first suspect; a control id that no corpus row carries turns
it red for the other reason, so renaming a subject cannot silently drop the
control. The controls are `gtk3-1`, `gtk3-2` and `py-1` — `experiments/64-`
arms G, X and P, each measured at 11 of 11 twice by an experiment that shares
no code with this one but the classifier.

⚠ **The one row this cost is retracted, not corrected**: `gtk3-1`'s recorded
row was deleted, because a row measured with an instrument that cannot fail for
the right reason is not a datum to adjust.

---

## C27 — "the field's regex does not stop at `<`", written by a scanner that did not stop at `<` either

⛔ **T-081's "Prove" section said this, and it was published:**

> ⭐ The one that is not is `…-dejavu-fonts-minimal-2.37<`. Its trailing `<` is
> an **XML markup boundary**: the match is a path followed by the start of the
> next tag, because the field's `[^ \"']*` does not stop at `<`.

⛔ **`internal/bundle/appimage.go`'s own `storeRefRe` was
`/nix/store/[a-z0-9]{32}-[^" ']*`.** The same three excluded characters. The
sentence identified the defect precisely and attributed it to the other route.

**Found by reading a build log rather than the code.** The galculator bundle
reported six compiled-in store paths with no target, and three of the six were
the scanner:

    ...-python3-3.14.7\0\0\0\0\0\0Exception     NUL is not in the class, so the
    ...-glibc-2.42-84\0\0\0\0\0\0\0             match ran into the next string
    ...-dejavu-fonts-minimal-2.37<              nor is `<`

⭐ **Why a wider match is not merely untidy.** Every caller cuts the match at
the first `/` and looks the base up in the closure. A boundary error therefore
does not widen a string — it manufactures a **base no closure can contain**,
which is reported as "does not resolve" and left unrewritten. And for
`…2.37</dir>` the cut lands *inside* the markup, so a rewrite that did fire
would have eaten the `<`.

**Re-measured on the same AppDir, both scanners:**

| scanner | occurrences | distinct | in the closure | NOT in it |
|---|---|---|---|---|
| `[^" ']*` | 415 | 13 | 12 | 1 |
| corrected | 425 | 13 | ⭐ **13** | ⭐ **0** |

⭐ **`dejavu-fonts-minimal-2.37` was in the closure all along**, and the binary
scan's residue fell from **6 to 3** — none of the three a missing dependency.
`HISTORY/entries/toolchain.md` T-081 has the residue table.

⛔ **The claim that survives, and it is the one that mattered.** The field's
regex 5 **substitutes** on a mis-bounded match; this route never substitutes on
a match it cannot resolve against the closure. With a boundary this bad the
worst it could do was **report a path it should have rewritten** — the safe
direction, arrived at for the wrong reason. ⭐ That is the difference between
reporting and guessing, and it held while the instrument was wrong, which is
the strongest thing that can be said for a design.

⚠ **The class stays a blacklist and the direction is deliberate.** Nix does not
constrain the names of files *inside* a store path, so a whitelist would
TRUNCATE a legitimate tail — and truncation is the dangerous failure, because
every rewrite substitutes the prefix and copies the tail verbatim: a short tail
is written back and corrupts the file. Over-capture fails safe.

⛔ **And one of the four new selftest cases could not fail.** The first draft
asserted on `StoreRefToBundle`, which does not use this regex; it passed under
the planted defect. It was replaced with an assertion on the **base** every
caller looks up. ⚠ Three of the four now fail under the old regex; the fourth
is a regression guard against the whitelist mistake and passes either way,
which its comment says.

---

## C28 — a review's own hypothesis, falsified by the plant it wrote

⛔ **The review said**: `StoreRefToBundle` substitutes into `.env` **without**
asking the closure, unlike `storeRefRe`'s callers, so its four-character class
`[^/:; ]*` must be corrupting values — a quote or a comma captured into the
name and written back.

⭐ **The plant said otherwise.** With the old class restored, the two cases
written to prove corruption **passed**. The substitution is `"store/" + name`,
so an over-captured name is reproduced verbatim and the text comes out
byte-identical. ⚠ A third case using `strings.Contains` also could not fail,
for a second reason: the correct name is a **prefix** of the over-captured one.

⭐ **What the review found instead is a real coupling nothing checks.** The
name written after `store/` must be the directory `buildStoreFarm` created —
`shortStoreName(base)`, the base minus its 32-character hash. Two patterns
derive that name independently, and **no check reads a `.env` value back
against the tree**: `integrity()` walks `DT_NEEDED`, `manifestIntegrity()`
reads the ICD manifests. That absence is what let `${SHARUN_DIR}` expand to
nothing for a whole session.

**Landed**: one `storeRefStop` shared by every store-path pattern in the
package — there were **four classes in four places**, which is the regex
cascade T-081 said it was replacing — and three selftest cases asserting the
two patterns agree **on the name**, which fail under the old class.
⚠ One divergence survives as **T-092**: on a short-name collision the farm uses
the full `<hash>-<name>` and the `.env` still emits the short one.

---

## C29 — three instrument defects in one session, each caught by running rather than reading

⭐ **All three were in experiments written THIS session, and none survived to
a published number.** They are recorded together because they are the same
shape: a check that reported something other than what it was asked.

**1. `env` took `--` as the program.** `experiments/68-` arm S drove the
selector with `env -u ARGV0 VAR=VAL -- prog`. Once an assignment has been seen,
`--` is no longer in option position, so `env` ran a program called `--`. Six
of eighteen checks read `(none)`; ⚠ **three passed anyway**, because their call
had no assignment and the `--` did still follow the options — which is the part
worth keeping: a partly-working harness reported a mixture, not a clean
failure.

**2. `exec -a` and `"${@:3}"` are bash, and the experiments are `sh`.** Setting
`argv[0]` independently of the path exec'd is the only way to test the
selector's third rule, and POSIX `sh` cannot do it. ⭐ Replaced with an
eleven-line C `execas` helper rather than by changing the interpreter — the
experiments are the independent acceptance harness and stay shell
(`docs/AGENTS.md` §0b).

**3. ⛔ A NEGATIVE CONTROL THAT PASSED BECAUSE IT COULD SEE NOTHING.** The same
arm built its no-default control into `$WORK` instead of the AppDir. The
selector derives its AppDir from `/proc/self/exe`, so the control was pointed
at a directory with **no programs in it at all**: it exited 127 because nothing
matched, not because it had no default. The check went green.
⭐ **A control that passes for the reason under test and for an unrelated
reason is not a control**, and the fix was one path. Delivery rule 6 — check
that the criterion can fail *for the right reason* — applies to controls too,
and this is the second session running in which it has fired.

**4. "no `lsns`" reported as "zero namespaces".** `experiments/69-` N7 counted
`lsns -t user | tail -n +2 | wc -l`, which yields `0` both when there is
nothing to count and when the binary is absent. On `alpine-3.22`, whose busybox
ships no `lsns`, N7 reported `0, expected 1` and read as a real disagreement
about namespaces. ⛔ **A skip is neither a pass nor a failure** (`docs/AGENTS.md`
§0b); it now reports as a skip with its reason. ⚠ Caught by the **second run on
a different rootfs** — delivery rule 3 doing exactly what it exists for.

**Landed**: `experiments/68-` arm S `pass=18 fail=0`; `experiments/69-`
`pass=9 fail=0 skip=0` on `debian-12` three times and `pass=8 fail=0 skip=1`
on `alpine-3.22`.

---

## C30 — `pgb-apprun.c`'s header stated the dispatch order wrongly, and two documents copied it

⛔ **The code was right; the comment and both documents that quoted it were
not.** `tool/runtime/pgb-apprun.c` opened with:

    1. argv[0]'s basename names a program in shared/bin  -> run it
    2. argv[1] names a program in shared/bin             -> run it, drop argv[1]
    3. otherwise                                          -> run the default

`docs/research/app-corpus.md` rung 1 carried a third variant —
*"`ARGV0` → `argv[0]` basename → `$1`"*. ⭐ **The measured order**
(`experiments/68-` arm S, E2) is:

    $ARGV0's basename  ->  argv[1] (DROPPED)  ->  argv[0]'s basename  ->  default

⛔ **`ARGV0` is the rule that matters and the source comment named it nowhere.**
uruntime sets `ARGV0` to the AppImage's own path, so it is the rule a **renamed
or symlinked** artefact lands on — the whole feature the comment was describing.
And `argv[1]` beats `argv[0]`, not the reverse.

⚠ **This was PRE-REGISTERED as a prediction about our own source** and
committed before the run (`2b1daeff`), which is why it counts: E2 said the
comment was wrong and the code right, and the run agreed. ⭐ The comparison
against the field also survives and is now stated from measurement rather than
from the comment: Anylinux's `AppRun.sh` is
`ARG0="${ARGV0:-$0}"` → `bin/${ARG0##*/}` → `bin/$1` → `MAIN_BIN`, so ours is a
**superset** — theirs collapses `ARGV0` and `$0` into one test and never
re-checks `$0` once `ARGV0` is set but unmatched.

---

## C31 — a COMMENT broke `pgb-apprun.c`, and the fallback would have shipped a shell AppRun

⛔ **The worst shape a defect can have in this tree: it makes the build
succeed and the artefact worse.**

A revision of `tool/runtime/pgb-apprun.c`'s header quoted Anylinux's dispatch
rule verbatim — `ARG0="${ARGV0:-$0}"` → `bin/${ARG0##*/}` → `bin/$1` →
`MAIN_BIN`. ⭐ **`${ARG0##*/}` contains `*/`, which ends a C block comment.**
Everything after it parsed as code and the file stopped compiling.

⛔ **Three things had to line up for that to be invisible, and they did:**

1. `tool/runtime/*.c` are **embedded as strings** and compiled by `cc` at build
   or bundle time, so `go build` and `make` both succeed on a C file that
   cannot compile;
2. `buildStaticAppRun` **catches** a compile failure and falls back to
   `writeShellAppRun` — a shell AppRun, run by the **host's** interpreter,
   loading the **host's** libc. That is exactly what the file exists to avoid,
   measured at 1–4 host objects per glibc row in `experiments/90-`;
3. the fallback announces itself as a **warning in a build log**, not an error.

⭐ **What caught it was `experiments/68-` arm S** — an experiment written the
same session, which compiles the real source rather than reading it. Nothing
else in the tree would have.

**Landed**: the comment no longer quotes the expansion and says in one line why
it must not; ⭐ `TODO/check.sh` **check 10** requires every `tool/runtime/*.c`
to pass `cc -fsyntax-only` (13 checked, all pass, verified to fail on the
planted defect). The other twelve files were already fine.

---

## C32 — "copyLibraries flattens everything", read from half a function

⚠ **A wrong claim about our own code, caught by a deep review before it was
measured, and it had already been written into an entry and two comments.**

T-091's first version argued that Anylinux-sharun's GStreamer branch — which
sets all four `GST_PLUGIN_*` variables for a `gstreamer-*` directory under
`shared/lib` — **cannot fire here**, because *"`copyLibraries` flattens every
shared object into a single `lib/`, so there is no such directory"*.

⛔ **It flattens loose objects and it also carries every directory under a
store path's `lib/` WHOLE** — `copyTreeNoClobber`, logged as
`lib trees N directories under lib/ carried whole` — and `assemble.go`
symlinks `shared/lib` → `../lib`. ⭐ **So `shared/lib/gstreamer-1.0` exists and
that branch can fire.** Whether it does is still unmeasured, and the entry now
says so instead of asserting the opposite.

⭐ **What survives is the part that mattered**: `GST_PLUGIN_SCANNER` is
definitely not set. sharun sets it only when the scanner sits *beside* the
plugins, and nixpkgs puts it in `libexec/gstreamer-*/`, so that test cannot
succeed on a nixpkgs closure however the plugin directory is laid out.

⚠ **The reading error is the lesson**: the head comment of `copyLibraries` says
what it does; the second half of the function says what *else* it does. Nothing
measured this wrong — nothing had measured it at all.

---

## C33 — three controls that would have passed on a dead subject

⭐ **C29's third defect, found again in three new places in one review**, all
in experiments written this session, none yet run against a subject.

| where | what it counted | why that is not a measurement |
|---|---|---|
| `101-` **L2** | arm N opened no catalogue under the bundle | ⛔ also what a control that **never started** reports. It now takes three observations: opened none, **was seen attempting `/nix/store`**, and still drew. ⚠ The `/nix/store` count was already being computed and **never read** — the pre-registration promised an observation the script did not make |
| `101-` **L4** | arm T loaded zero host shared objects | ⛔ a corpse loads zero too. Gated on the row having drawn |
| `68-` **E11** | the second program loaded zero host objects | ⛔ same. Gated on the row having run |

⛔ **"An absence is not a zero" (delivery rule 4) applies to controls, not only
to searches.** A control passes only when it is seen doing the thing that
distinguishes it from a failure — a positive observation, never the lack of a
negative one.

---

## C34 — `experiments/65-`'s `cli` criterion scores a correct answer as a failure

⛔ **The OpenGL / EGL row read `0 of 11` and the capability works.**

`65-` scores a `cli` subject as `exit 0` **AND** the assertion. `eglinfo` in a
headless environment:

| | |
|---|---|
| runs, printing a full EGL config table | ⭐ yes |
| the corpus assertion `(llvmpipe\|Mesa\|softpipe)` | ⭐ **matches 20 times** |
| exit status | ⛔ **3** |

⚠ **And it is not the bed.** With `XDG_RUNTIME_DIR` set and every `error:` line
gone, it **still exits 3** — that is how the program reports that some EGL
platform (wayland, gbm) is unavailable, while answering completely about the
one that is.

⭐ **How it was found**: `mesa-demos` bundles `glxgears + 309 more`, so
`eglinfo` was reached out of the corpus's own `gl-2` artefact by **renaming
it** — `experiments/68-`'s dispatch rule used as a diagnostic, and an
independent confirmation of that rule on a 310-program bundle.

**The rule that is wrong**: an assertion is *"checked IN ADDITION to the mode's
criterion, never instead of it"*. That is right for a subject with no
assertion and wrong for one with a good assertion and a noisy exit status. ⭐
**When a subject carries an assertion, the assertion is the criterion** and the
status is reported beside it.

⛔ **Not fixed in this session**: `65-` was running, and editing an executing
script re-enters it at a shifted byte offset (C31's neighbour). The `gl-1` row
must be **deleted** and re-measured after the fix — a row from a broken
instrument is deleted, never adjusted. ⚠ `vulkan-1` and `media-1` are `cli`
with assertions and must be re-read under the new rule too.

---

## C35 — `neovim`'s bundle could not start, and one old glibc explains two messages

⛔ **The corpus note is truncated at 70 characters and that hid the answer.**
The full error is the dynamic loader's: `--argv0: error while loading shared
libraries: --argv0: cannot open shared object file`.

The `neovim` closure carries **`glibc-2.26-115`**. sharun starts a dynamic
payload as `<loader> --library-path <p> --argv0 <a> [--preload …] <bin> …`, and
`ld.so` learned `--preload` in **2.30** and `--argv0` in **2.33**. ⭐ A loader
with no option parsing takes the first argument as the **program**, which is
exactly what the message says; measured, that loader rejects even `--version`
the same way.

⭐ **And the same old glibc explains the other warning that build printed** —
*"the interposer was NOT installed: the bundle's libc does not define dladdr,
dlsym"*. That `libc.so.6` exports **0** of the two: they lived in `libdl.so`
until glibc **2.34**. ⚠ Two messages that look unrelated, one cause, and `pgb`
already had the evidence.

⚠ **A plausible wrong hypothesis, recorded because it was plausible**: the
build log also names `/nix/store/eeee…-glibc-2.23`. That is nixpkgs'
**placeholder** hash, not a store path; the real loader is on the `loader` line.

**Landed**: `checkLoaderOptions` reads the loader's glibc version at build time
and warns with the exact runtime string. ⚠ It **warns** rather than refusing,
because sharun skips the loader command line entirely for a static or
already-patched payload, so such a closure still works.

---

## C36 — the corpus separator was `|` and the assertions are regexes that alternate

⛔ **Two capability rows read `0 of 11` on capabilities that demonstrably work,
and neither was a bundler failure or the C34 criterion.** The cause is one
character.

`experiments/65-`'s corpus is `|`-separated, and an `assertion` is a `grep -E`
pattern — the useful ones **alternate**. `cut -d'|' -f6` cuts at the first
alternation. For `gl-1`:

| field | what it should be | what it got |
|---|---|---|
| `assert` | `(llvmpipe\|Mesa\|softpipe)` | ⛔ **`(llvmpipe`** — `grep: Unmatched ( or \(`, exit 2, **can never match** |
| `extras` | `mesa` | ⛔ `Mesa` — the build log says `pgb: could not resolve --extra Mesa: no hydra job` |
| `args` | *(none)* | ⛔ `softpipe)` — handed to the program as an argument |

`vulkan-1` was mangled the same way: `assert` `(lavapipe`, `extras` `llvmpipe`,
`args` `Vulkan Instance)`.

⭐ **The evidence that these are not real failures**, taken by hand out of the
corpus's own artefacts:

| | |
|---|---|
| `eglinfo`, run exactly as the corpus runs it, inside a rootfs, with a real display | prints a full EGL table; the **real** assertion matches **30 times** |
| `vulkaninfo --summary` | exit **0**, `GPU0 deviceName llvmpipe (LLVM 21.1.8)`, `apiVersion 1.4.354` |

⚠ **The build log had been saying so all along** — `could not resolve --extra
Mesa` is not a sentence anything in the corpus should produce, and nobody read
it.

**Landed**: the separator is `;`, which appears in no field and which a
`grep -E` pattern has no use for. Both rows **deleted** and re-measured.

⛔ **AND IT HID BEHIND A REAL DEFECT.** C34 — the `cli` criterion being
`exit 0 AND the assertion` — is genuine and was fixed first; `gl-1` was
re-measured under the fix and **still read 0 of 11**. ⭐ That second zero is
what forced the search that found this. A correct fix that does not move the
number is information, and stopping at "fixed it" would have buried this one.

---

## C37 — a shell-wrapped nixpkgs application could not start, and the message named the wrong file

⛔ **A whole class of subjects, and the error pointed at a file that was
present.** `xterm` scored `0 of 11`, dying with:

    shared/script/xterm: line 3: /nix/store/<hash>-xterm-410/bin/.xterm-wrapped:
    No such file or directory

⭐ **THE MECHANISM, IN FOUR STEPS, EACH MEASURED.**

1. A nixpkgs `bin/<name>` can be a **shell** wrapper rather than a
   `makeBinaryWrapper` ELF. Its last line is
   `exec -a "$0" "/nix/store/<hash>-xterm-410/bin/.xterm-wrapped"`.
2. `assemble.go` skipped **dot-named** files in `bin/` — a rule added so
   `.meld-wrapped` would not become a second entry point. ⛔ *"Not a program"*
   and *"not in the bundle"* are different things, and conflating them left the
   wrapper's target out entirely.
3. Installing it was **necessary and not sufficient**. The interposer rewrote
   the path correctly — confirmed with `PGB_STOREFIX_DEBUG=1`, three times per
   run — and the target then resolved to a real **1,034,328-byte** executable.
   It still failed.
4. ⭐ **Because the farm's `bin` resolved to `shared/bin`, the RAW payloads.**
   That target is a dynamic ELF whose `PT_INTERP` names a `/nix/store` loader
   the bundle does not carry, so `execve` returns **ENOENT for the
   interpreter** — and the shell prints it against the **program** path.
   Running it directly says `cannot execute: required file not found`, which
   is the message that gives it away.

⚠ **That is why it read as "the file is missing" when the file was right
there**, and it is a confusion worth keeping: ENOENT from `execve` names the
program, never the interpreter.

**Landed**, two halves, and neither works alone:

| | |
|---|---|
| `assemble.go` | a dot-named file in `bin/` is copied into `shared/bin` as a **payload** |
| `sharun.go` | it gets its **`bin/` sharun hardlink** but **no top-level hardlink** and is **not counted** in `programs` — so it never enters the selector |
| `storefix.go` | ⭐ `mergedFor["bin"]` and `["sbin"]` resolve to **`bin`**, the sharun hardlinks, not to `shared/bin`. A compiled-in `<store>/bin/<x>` now reaches something that sets the library path and runs the bundled loader |

⭐ **Measured**: `xterm` draws a real toplevel window on a real X server in
**2 seconds**, from `0`. ⭐ **Regression control**: `galculator` — the T-081
acceptance subject — still draws, in 4 seconds, with **identical** store-path
resolution (`88 compiled in, 85 resolve`), so the `mergedFor` change did not
disturb the path that was already working.

⭐ **CONFIRMED IN THE CORPUS, ON TWO SUBJECTS.** `./pgb` was rebuilt once
`experiments/65-` could be stopped, and the rows measured against the old tool
were deleted and re-taken:

| row | before | after |
|---|---|---|
| `x11-3` `xterm` | `0/11` | ⭐ **`11/11`** (clean `4/11`, which is C5) |
| `gl-3` `glmark2` | `0/11` | ⭐ **`11/11`**, clean `11/11` |

---

## C38 — "the classifier error only runs one way" is true of C25 and FALSE of the copies

**Found** 2026-09-04b by `experiments/102-`, which diffs the six hand copies
against `experiments/lib.sh`'s `exp_classify_trace` on fixtures — **no bundle
build at all**. `TODO/ci.md` T-084 asserted, and this tree repeated in four
places:

> *"THE ERROR ONLY RUNS ONE WAY. It can turn a clean row dirty and can never
> turn a dirty row clean, so every committed ZERO stands."*

⭐ **That is a claim about C25, and it was being read as a claim about the
copies.** The copies carry a **second** difference, and it runs the other way.

**The mechanism.** Both implementations clear their result set when the
artefact's own path is `execve`d. The shared classifier guards that on the
mode; the copies do not, and they disagree about which way:

| | on `execve("<artefact>")` | consequence |
|---|---|---|
| `exp_classify_trace` | clears **only in `payload` mode** | correct in both |
| ⛔ `62-` `85-` `86-` `89-` `90-` | clears **unconditionally** | in **`tree`** mode it discards everything counted before the exec — a **dirty row read clean** |
| ⛔ `60-` | **never** clears there | in **`payload`** mode it keeps objects the exec unmapped — a **clean row read dirty** |

⛔ **The two experiments that call `tree` mode are `62-` and `90-`** — and
`90-` is the source of the competitor's *"4 of 11"* that T-084 names as the
number at risk. So that number could be too **low**, which is the opposite of
what the entry warned about.

⭐ **AND THE HAZARD IS LATENT IN THE DELIVERY SHAPE MEASURED.** `102-` arm R1
counts `execve("<artefact>")` lines in a real corpus trace: **one**. With one
exec the unconditional clear has nothing to clear. ⛔ It is one delivery shape;
a wrapper that re-execs the artefact under its own name would make it fire.

⛔ **AND ONE OF THE SIX IS EXACTLY THAT SHAPE.** `102-` arm S asks, per
experiment, which mode it calls and how many times its **traced** run invokes
the artefact — the two facts that decide whether C38 can reach it:

| experiment | mode | invocations | C38 reaches it |
|---|---|---|---|
| `60-` | payload + tree | 1 | no |
| `62-` | payload + tree | 1 | no |
| `85-` | payload | 1 | no |
| `86-` | payload | 4 | no — its copy differs only in **tree** mode |
| `89-` | payload | 1 | no |
| ⛔ `90-` | **tree** | **2** | ⛔ **YES** |

⛔ **`90-`'s test script runs the artefact twice** — `melt -version`, then a
real encode — and every one of its eleven rows recorded `P=ok E=ok`, so both
invocations ran on every row. Its host counts therefore describe only the
**second** invocation. ⛔ **That includes our own `0 of 11`, not only the
competitor's `4 of 11`** — being wrong in our own favour is the case the
delivery rules exist to catch. ⚠ How much it moves is **not measured**: the
second invocation plausibly loads a superset of the first, and *plausibly* is
not a measurement. `90-` must be re-run — T-084 step 2.

⛔ **A SECOND CORRECTION, IN THIS EXPERIMENT'S OWN PRE-REGISTRATION.** `102-`
predicted *"six files, **two** implementations"*, measured by hashing the
comment-stripped bodies. It read **three**. ⭐ A hash measures **text**:
`90-` is `62-`'s code with one rule reflowed across four lines, so it hashes
differently and behaves identically. The prediction is kept in the file as
written, and the number it *meant* is now measured behaviourally — every
copy's output over five fixtures in both modes — which reads **two**. ⚠ Two
texts that behave alike are one implementation, and only running them says so.

**Landed.** `experiments/102-classifier-equivalence.sh`, `pass=15 fail=0`,
**two runs identical**. It moves no committed number and says so; T-084 step 2
still owes the conversion and the re-run.

---

## C39 — `media-1`'s assertion cannot match the answer `mpv` actually gives

**Found** 2026-09-04b, the **fourth** instrument defect in `experiments/65-`
and the third of its kind. The row read **`0 / 11`**. The subject answered
completely.

The corpus line is:

    media-1;media / codecs;mpv;mpv;cli;mpv [0-9];mesa;--version

and `mpv --version` prints:

    mpv v0.41.0 Copyright © 2000-2025 mpv/MPlayer/mplayer2 projects

⛔ **There is a literal `v` between `mpv` and the digit.** `mpv [0-9]` cannot
match it; `mpv v[0-9]` does. Measured against the captured banner: **0** and
**1**.

⭐ **The row is DELETED, not adjusted** — the rule that makes a resumable
corpus safe. It is re-measured once the assertion is fixed:

    PGB_EXP65_ONLY='media-1'  sh experiments/65-capability-corpus.sh

⚠ **Why this keeps happening, stated plainly.** Three of the four zeros run
down in this corpus were the instrument, and all three were the *criterion*
rather than the subject: C34 (`exit 0 AND the assertion`), C36 (the separator
ate an alternating regex), and now C39 (a regex that never matched the real
output). ⛔ **A `cli` assertion is written from what the program is expected
to print and is never checked against what it does print.**

### ⭐ THE LOOP IS CLOSED — 2026-09-04c

`experiments/65-` now **interrogates its own assertion on the first
environment** and abandons the subject with an **INSTRUMENT** row rather than
scoring it zero eleven times. Two tests, and they catch different defects:

| test | catches | how |
|---|---|---|
| does the pattern **compile**? | ⭐ **C36** | `grep -E` exits **2** on a malformed pattern and **1** on a valid one that matched nothing, so empty input separates them |
| did the program print the assertion's **literal prefix** while the full pattern missed? | ⭐ **C39** | `assert_anchor` is the leading run of the pattern before the first regex metacharacter: `mpv v[0-9]` → `mpv `, `helix [0-9]` → `helix `, `(llvmpipe\|Mesa)` → empty |

⛔ **The criterion is deliberately NOT "the assertion matched nothing."**
`neovim` really does score 0 of 11 — its closure's `ld.so` rejects `--argv0`,
so the program never runs (C35) — and reporting that as an instrument error
would throw a real result away. ⭐ **The anchor is what separates *the program
answered and we misread it* from *the program never spoke*.**

⭐ **Checked against every historical case before it was trusted**, which is
the point of the rule it exists to serve:

| assertion | output it was given | verdict |
|---|---|---|
| `mpv [0-9]` (C39, the defect) | `mpv v0.41.0 Copyright …` | ⭐ **INSTRUMENT**, anchor `mpv ` |
| `mpv v[0-9]` (the fix) | the same | ✅ pass |
| `NVIM v[0-9]` (C35, a real zero) | `ld.so: unrecognized option '--argv0'` | ⭐ **fail, and kept as a fail** |
| `(llvmpipe\|Mesa\|softpipe)` | an `eglinfo` config table | ✅ pass |
| `(llvmpipe` (C36, the cut pattern) | the same | ⭐ **INSTRUMENT**, does not compile |
| `helix [0-9]` | no output at all | fail, and kept as a fail |

⛔ **An INSTRUMENT subject writes NO row**, so it is re-measured on the next run
once the pattern is fixed — the same rule that makes deleting a bad row safe.
**C7** is checked before C1 and C2, for the reason C6 is: a subject whose
criterion cannot recognise its own answer is not a capability result.

⭐ **And the same run measured something T-091 needed.** `mpv`'s build log
reads *`gstreamer  scanner installed as a program (GST_PLUGIN_SCANNER
follows)`* — T-091's shipped code firing on a real corpus subject, not on a
synthetic one. ⛔ It still does not close T-091, and now for a precise reason:
the row runs `--version`, which never launches the scanner, so its
host-object count cannot say which process it counted.

---

## C40 — the corpus row note threw the answer away twice over

**Found** 2026-09-04b, fixed 2026-09-04c. Not a wrong measurement — a
**diagnostic** that could not carry the diagnosis, which is how three rows sat
unexplained for a session.

`experiments/65-` builds each failing row's note like this:

    note=$(cat "$WORK/err.$id."* | grep -m1 -E "…|Traceback|…|Error" | cut -c1-70)

⛔ **Both halves discard the answer, for different reasons.**

| | |
|---|---|
| `grep -m1` — the **FIRST** matching line | ⛔ A Python traceback **opens** with `Traceback (most recent call last):`. The line naming the cause is the **LAST** one. `py-2`'s note was therefore the word "Traceback" and nothing else |
| `cut -c1-70` | ⛔ Truncated two real answers: `neovim`'s loader message, and `vkmark`'s `[/dev/dri]` — **the entire finding**, which had to be recovered by hand from the raw stderr |
| `cat "$WORK/err.$id."*` | ⚠ A **passing** environment's stderr noise sorts in among a failing one's, so the chosen line need not belong to the failure at all |

⭐ **It is now the LAST matching line of the FIRST environment that has one, at
180 characters.** Per-file rather than over the concatenation, so the note
describes one environment's failure completely instead of the pile's.

⚠ **The general shape is worth naming**, because it is the cheapest defect in
this tree to reintroduce: **a diagnostic that is nearly right is worse than one
that is absent**, because a reader treats it as the answer. `py-2`'s note said
`Traceback` for a whole session and nobody re-read the stderr.

---

## C41 — the interposer rewrote what a program OPENED and not what it STATTED

**Found** 2026-09-04c, by running `field-4` `gearlever`'s UNRESOLVED down. ⛔ A
shipped claim — *"a compiled-in `/nix/store` path now resolves"* — was true of
one syscall family and false of another, and the gap is the whole of Python.

`gearlever` started, and died at:

    ModuleNotFoundError: No module named 'gearlever'

⚠ **With the interposer loaded and working.** The line before it,
`Gio.Resource.load(os.path.join(pkgdatadir, 'gearlever.gresource'))`, had just
succeeded at the *same* compiled-in store path. The trace says exactly what
happened:

| call | path | result |
|---|---|---|
| `openat` for the gresource | ⭐ rewritten to `…/store/gearlever-3.4.7/share/gearlever/gearlever.gresource` | opened |
| `newfstatat(AT_FDCWD, …)` for the directory | ⛔ **`/nix/store/plv6…/share/gearlever`, untranslated** | `-1 ENOENT` |

⭐ **`nm -D` on the bundle's own libpython names the cause in three lines:**

    U stat64@GLIBC_2.33   U lstat64@GLIBC_2.33   U fstatat64@GLIBC_2.33

⛔ **`pgb-storefix.c` defined `stat`, `lstat` and `fstatat` and not one of the
`64` names.** A caller built with `_FILE_OFFSET_BITS=64` — which is every
nixpkgs Python — references the `64` symbol, so an interposer that defines
only the unsuffixed name is never asked. Python's path-based finder asks
`stat` whether a directory exists before it will import from it, so
`sys.path.insert(1, pkgdatadir)` was a no-op for every bundled Python program.

**Added**, each forwarding the buffer and reading nothing: `stat64`,
`lstat64`, `fstatat64`, the pre-2.33 `__xstat64`/`__lxstat64`/`__fxstatat64`
for the same reason `__xstat` was already there, and `readlinkat`, which
libpython also imports and whose absence made the bundle answer two different
things about one path.

⭐ **MEASURED, and the traceback moved:** gearlever's failure is now *inside*
its own modules at the compiled-in store path —
`/nix/store/plv6…/share/gearlever/gearlever/preferences.py` — so the import
resolved. ⛔ It now fails later and elsewhere, on
`RuntimeError: could not create new GType: gearlever+preferences+Preferences
(subclass of void)`, which is a libadwaita question and is **not established
as ours**.

⚠ **THE GENERAL LESSON, and it is cheap to check.** An `LD_PRELOAD` interposer
is only asked about the symbols its callers actually import. ⛔ **`nm -D
--undefined-only` on the payload is the check**, and nothing in the tree ran
it before this. The same question is open for every other family the
interposer defines by its unsuffixed name only.

---

## C42 — a loader NAME is not a loader, and an FHS symlink farm is where they differ

**Found** 2026-09-04c, the first half of the same gearlever run. The build
died before it produced anything:

    pgb: open …/6zj308…-appimage-run-fhsenv-rootfs/usr/lib64/
         ld-linux-x86-64.so.2: no such file or directory

and the corpus recorded the subject as **UNRESOLVED** — a gap in the corpus
rather than a result about the capability, which is why nobody had read it.

⛔ **`gearlever`'s closure contains an FHS environment**, a store path that
emulates a `/usr` tree out of symlinks. Its `usr/lib64/ld-linux-x86-64.so.2`
points at an **absolute** `/nix/store/…-glibc-2.42-84/lib/…`, and there is no
`/nix/store` on the machine doing the bundling. ⚠ It also sorts *before* the
glibc store path that holds the real loader, so `findFile` — which takes the
first name that matches — returned the one that reaches nothing.

⭐ **The resolver already existed and was already the idiom elsewhere.**
`b.storeResolve` re-roots a `/nix/store/…` link at the fetched closure and is
what `copyDesktop` and the icon copy already used. `findResolvable` is
`findFile` with that one condition — a candidate is a candidate once it
reaches a real file — and it returns both paths, because the found path
carries the NAME the bundle must use and the resolved path carries the BYTES.

⚠ `checkLoaderOptions` reads the RESOLVED path now for the same reason: an FHS
farm's link is named `ld-linux-x86-64.so.2` under a store path with no glibc
version in it, so the old-loader warning would have had nothing to read and
would have said nothing.

**MEASURED:** gearlever went UNRESOLVED → **builds**, 907.6 MiB, 542 store
paths, loader `ld-linux-x86-64.so.2` from the closure's own glibc.

---

## C43 — a wrapper target can be a symlink into ANOTHER store path, and `os.Stat` asked the host

**Found** 2026-09-04c. ⛔ `field-1` `helix` had read **0 / 11** with an
**empty** row note for a whole session, and the emptiness was the clue nobody
followed: the harness looks for six error signatures in the subject's stderr
and matched none, because **the program printed nothing at all**.

Measured on the host, so the bed is not in it:

    $ ./helix.AppImage --version ; echo $?
    255

⛔ **No stdout. No stderr. Exit 255.** `LD_DEBUG=libs` says the bundle is fine:
libc, `libpgb-storefix.so` and the program are all loaded from the extraction
directory, initialisers run, and control is transferred. `strace` says what
happens next:

    execve(".../store/helix-25.07.1/bin/.hx-wrapped", …) = -1 ENOENT
    exit_group(-1)

⭐ **The interposer did its job** — the compiled-in `/nix/store` path was
rewritten to the bundle's farm. The target simply was not there.

⭐ **THE CAUSE IS A FOURTH ENTRY-POINT SHAPE.** `bin/hx` is a makeCWrapper
**ELF** that execs `bin/.hx-wrapped`, and `.hx-wrapped` is itself a **symlink
naming an absolute path in a DIFFERENT store path**:

    .hx-wrapped -> /nix/store/8c63…-helix-unwrapped-25.07.1/bin/hx

The resolver tested it with `os.Stat`, which **follows** the link — against the
**host** root, where there is no `/nix/store`. It reported ENOENT, the wrapper
was not resolved, and the makeCWrapper ELF was installed as the entry point.

⚠ **The build log said so and nobody read it that way.** For `flameshot` it
prints `bin/flameshot is a nixpkgs wrapper -> .flameshot-wrapped`; for `helix`
that line is **absent** and only `wrapper env 1 variable(s) lifted` appears.
⭐ The presence or absence of the arrow is the diagnostic.

⭐ **The fix is `b.storeResolve`, the same resolver C42 needed** — it re-roots
`/nix/store/…` at the fetched closure. Following through to the real file is
also better than stopping at the link, because what then gets installed is the
**unwrapped program itself**:

    entry  …/store/8c63…-helix-unwrapped-25.07.1/bin/hx

**MEASURED**, same subject, same environment, one changed line:

| | before | after |
|---|---|---|
| `hx --version` | ⛔ exit **255**, no output | ⭐ `helix 25.07.1`, exit **0** |

⛔ **AND THE PATTERN ACROSS C42 AND C43 IS ONE THING**: an absolute
`/nix/store` symlink inside a fetched closure must be resolved against the
**closure**, never against the machine doing the bundling. Two different code
paths made the same mistake, and the resolver that gets it right already
existed in the same file.

---

## C44 — a failed `cp` was scored as a failed subject, on four environments out of eleven

**Found** 2026-09-04c. `field-4` `gearlever` came back as

    field-4  gearlever  gui  0/4  4/4  221 compiled in: 208 res
             /bin/sh: /subj65: not found

⛔ **Two things in that line are wrong and neither is the subject.** The
denominator is **4**, not 11; and the note is the shell failing to find a file
the harness was supposed to have put there.

`experiments/65-` staged each subject with

    rm -f "$root/subj65"; cp "$img" "$root/subj65" 2>/dev/null; chmod +x …

⛔ **`2>/dev/null` on the copy.** gearlever's artefact is **907 MiB**, the
machine was under disk pressure with two corpus instances holding multi-GiB
closure caches, and `cp` failed with ENOSPC into a discarded stderr. The row
then ran the subject that was not there, `/bin/sh` said `not found`, and the
harness recorded a capability result.

⚠ **AND THE DENOMINATOR IS THE WORSE HALF.** `rows` counts environments the
loop reached, so a subject measured on four scores `pass/4`. ⛔ **A subject that
passed all four would have been recorded as passing — `4/4` — and read as a
green row.** Nothing in the verdict compared `rows` against the eleven.

⭐ **Both fixed:**

- staging happens **before** the row is counted, and a failed copy is a
  **SKIP with the copy's own error text**, not a zero;
- ⭐ **C8**: every subject that was measured must have been measured on all
  eleven environments. A short row now fails the run instead of being read as
  a small green one.

⛔ **The row was DELETED and re-measured**, which is the rule that makes a
resumable corpus safe. ⚠ And the general shape is the one this file keeps
recording: *a discarded error is a wrong answer waiting to be believed.*

---

## C45 — "`--with-program` is NOT exercised" was false, and a shipped experiment had been exercising it all along

**Found** 2026-09-04c, by reading `experiments/90-` rather than the entry that
described it. ⛔ T-088 carried, as its first open item:

> *"`--with-program` is NOT exercised. It is the branch that searches the
> whole closure rather than one `bin/`, and it is the one a helper living in a
> dependency needs. **Nothing has run it.**"*

⭐ **`experiments/90-` runs it, and has since it was written:**

    ./pgb bundle appimage kdenlive --with-program melt --with-program ffmpeg
    …
    /kd-arm melt -version
    /kd-arm melt color:blue out=12 -consumer avformat:/tmp/kd.mp4 …

`melt` lives in the **`mlt`** store path, not in `kdenlive`'s `bin/`, so it is
exactly the whole-closure branch the item is about. ⭐ And `90-` does not merely
start it: it makes it **render an MP4**, on **all eleven** environments, with
`P=ok E=ok` recorded per row. ⚠ The script even knows the assertion
discriminates — its own comment records that the *competitor's* artefact runs
kdenlive with `melt` as an argument instead.

⚠ **Why it was missed is worth stating**: `90-` is filed as the kdenlive
head-to-head, so nobody looking for a multi-entry measurement looked in it.
⛔ **This is delivery rule 5** — *verify your own write-up against the source* —
catching a claim on a TODO page rather than in a document.

⭐ **What T-088 still owes is therefore much narrower** than the entry said:
a second closure on the eleven, not the branch itself.

---

## C46 — the compiled-in loader discarded `dlopen`'s flags, and the field's objection landed on us

**Found** 2026-09-04c, by `experiments/104-`, written **because** the operator
said to take the field's *"can't fix / unfixable / hard"* claims seriously one
by one. `pkgforge-dev/Anylinux-AppImages`' FAQ, on why it does not use
`solo`/`detour` — the route this project took:

> *"it seems none of the solutions implement `dlmopen`, so you are likely to
> run into a lot of symbol collisions with host libraries."*

⛔ **`docs/research/bundle-capabilities.md` answered that from the DESIGN** —
"each loaded object's undefined symbols are resolved by us, against the static
glibc already linked in, and nothing is added to a global search scope" — and
said in the same sentence that it was **read off the design, not measured**.
⭐ **Measured, the design reading was wrong.**

**The probe.** Two objects, each defining `int pgb_which(void)` — A returns 1,
B returns 2 — and B also exports `int b_calls_which(void) { return
pgb_which(); }`. ⭐ A call to a default-visibility function *inside one shared
object* still goes through the PLT and is interposable, so the number
`b_calls_which()` returns says which scope won.

| | glibc's `ld.so` | ⛔ ours, before | ⭐ ours, after |
|---|---|---|---|
| `liba` opened `RTLD_GLOBAL` | **1** | 1 | **1** |
| `liba` opened `RTLD_LOCAL` | **2** | ⛔ **1** | ⭐ **2** |

⛔ **`pgb_elf_dlopen` began with `(void)flags;`.** The flags were discarded
entirely, and `el_resolve` searched **every loaded object in load order before
the requesting object's own definition**. So the first object loaded won every
name for every object loaded after it — which is exactly the collision the FAQ
describes, in a loader nobody had asked.

⭐ **The fix is ld.so's order, and it keeps a deliberate fallback:**

1. our static glibc's "first" table (the executable's scope);
2. the requester itself, if it is `DT_SYMBOLIC`;
3. ⭐ **objects in the GLOBAL scope only** — `RTLD_GLOBAL`, propagated to their
   `DT_NEEDED` tree, which is what ld.so does;
4. ⭐ **the requester itself, then its `DT_NEEDED` tree** — its local scope;
5. ⚠ **every remaining loaded object**, which is *not* ld.so's behaviour and is
   kept on purpose: `experiments/93-` loads **882 of 1,527** host objects here,
   and some resolve only because a sibling outside their `DT_NEEDED` tree
   happens to be mapped. Narrowing to be strictly correct would turn loads that
   work today into failures. ⭐ **The reordering fixes which definition WINS;
   this step preserves whether one is found at all.**
6. the provider table's "last" entries.

⚠ **AND THE EXPERIMENT'S FIRST VERSION COULD NOT HAVE SEEN THE FIX.** It
asserted `== 2` and ran the subject **only** with `RTLD_GLOBAL`, where **1 is
the correct answer** — so it failed against a correct loader for the wrong
reason. ⭐ The criterion is now *agreement with `ld.so` in BOTH scopes*, and
the two answers differ from each other, which is what stops the agreement from
being a constant. **Delivery rule 8, again**: a control that cannot be told
from its subject is not a control — and neither is an experiment that only
asks one of the two arms.

---

## C47 — an absolute symlink resolved against the host, for the THIRD time in one day

**Found** 2026-09-04c, in a shell script written that afternoon. `is_musl()`
in `scripts/common/bed-fixtures.sh` tested

    [ -e "$root/lib/ld-musl-x86_64.so.1" ]

and reported **Void Linux musl as glibc**, so the installer tried to put a
glibc locale into it. ⛔ Void's loader is a chain of two links and the second
one is **absolute**:

    <root>/lib                        -> usr/lib          (relative, fine)
    <root>/lib/ld-musl-x86_64.so.1    -> /usr/lib64/libc.so

`-e` follows that against the **host** root, where `/usr/lib64/libc.so` does
not exist, so it answers *no*. `-L` asks whether the **link** is there, which
is the actual question.

⭐ **THE POINT IS NOT THE ONE-CHARACTER FIX. IT IS THE THIRD SIGHTING:**

| | where | what it broke |
|---|---|---|
| **C42** | `copyLoader`, Go | an FHS farm's dangling loader link aborted a build |
| **C43** | the wrapper resolver, Go | `.hx-wrapped -> /nix/store/…` read as ENOENT; `helix` failed **silently** |
| **C47** | `is_musl`, shell | Void reported as glibc |

⛔ **This is a structural hazard of what this repository does**, not three
coincidences: almost everything here operates on an **unpacked root from
outside it**, and every absolute symlink inside such a tree points somewhere
that means something different to the process looking at it. ⭐ **The rule, so
it stops being rediscovered: inside an unpacked root, resolve a symlink
against THAT ROOT or ask only whether the link exists. Never let the host
resolve it.** The Go side has `b.storeResolve` for exactly this; the shell
side has `-L`.

---

## C48 — the locale criterion could never fire, and the bed was never the reason

**Found** 2026-09-04c, by acting on the ruling in `docs/AGENTS.md` §0b instead
of accepting a recorded limit.

`experiments/101-` — T-087 rung 3's differentiator, *"a `.mo` catalogue opened
under the bundle, against the same bundle built `--no-storefix`"* — was
**stopped**, and both this file and `app-corpus.md` recorded the reason:

> *"no environment has a non-C locale compiled, so `setlocale` fails,
> `LC_MESSAGES` stays `C`, and gettext never consults `LANGUAGE`."*

⭐ **That was true, and it was not the cause.** `scripts/common/bed-fixtures.sh`
gave the bed a compiled `de_DE.UTF-8` — `experiments/106-` measures
`setlocale` succeeding on **7 of 7** glibc rows with the locale in effect — and
`101-` was re-run. ⛔ **It still read `0` in BOTH arms**, including the
control's *"tries `/nix/store`, opens none"* row, which says the subject never
attempted a catalogue open at all.

⭐ **The artefact had no catalogues in it.** Counted:

| | |
|---|---|
| `.mo` files in the built AppDir | ⛔ **0** |
| locale directories in `mousepad`'s own store path | ⭐ **54** |
| what the build log had been saying all along | `locale catalogues (kept: none)` |

⛔ **`internal/bundle/debloat.go` drops every directory under `share/locale`
unless `--keep-locales` names one, and the default is empty.** The code says so
in a comment — *"Locales are a real trade and it is stated as one: the
application's own translations go with them"* — so this is a **documented
default nobody connected to the experiment**, not a hidden bug.

⚠ **And the trade is real**, which is why the default is not simply wrong:
measured on `qalculate-qt`'s closure, `share/locale` is **113 MiB** across 30
store paths — `gtk+3` 28 MiB, `iso-codes` 22 MiB, `glib` 11 MiB — against a
238 MiB artefact. ⭐ Note also that `qalculate-qt`'s **own** store path ships
no catalogues at all, so "keep the application's own" would have been free
there and would have changed nothing.

⭐ **The fix for the EXPERIMENT is one flag**: `--keep-locales de` in both
arms, which is now in `101-` with this correction beside it.

⛔ **THE LESSON, AND IT IS THE THIRD TIME TODAY.** A zero was explained by the
first plausible cause somebody found — and the explanation was *checkable* and
was never checked. C39 blamed the subject when it was the assertion; C44
blamed the subject when it was a failed `cp`; C48 blamed the bed when the
build log had been printing `kept: none` on every run.

---

## C49 — "host" was a prefix list, and `/usr/bin/ld.so` fell outside it

⛔ **This one runs in the DANGEROUS direction.** C25 can only turn a clean row
dirty; a committed **zero** survives it. C49 turns a *dirty* row **clean**, and
a committed zero does not survive that.

`exp_classify_trace` splits every loaded shared object into `host` and
`bundled`. The test was a single prefix pattern, and **`bundled` was its
complement**:

```awk
if (p ~ /^\/(usr\/)?(local\/)?lib(32|64)?\//) out["host " p] = 1
else                                          out["bundled " p] = 1
```

⭐ So an object anywhere the pattern did not name was scored as *the
artefact's own* — the safe-looking answer — with nothing said.

⚠ **Measured, not argued.** Every `.so`/`.so.N` on all eleven pinned rootfs
that the old pattern missed, in full — this is the complete set, not a sample:

| path | on how many of the eleven | loaded by |
|---|---|---|
| `/usr/libexec/coreutils/libstdbuf.so` | 10 | `stdbuf(1)`, via `LD_PRELOAD` |
| `/usr/libexec/sudo/*.so` (8 files) | 1 (fedora-42) | `sudo` |
| ⛔ `/usr/bin/ld.so` | 4 (arch, fedora-42, debian-12, debian-13) | anything that runs it |

⛔ **THE THIRD ROW IS THE FINDING.** `/usr/bin/ld.so` is **the host's dynamic
loader**, shipped in bindir by Arch and Fedora. An artefact that ran the host
loader is the precise failure this whole tree exists to detect — and under the
old predicate that row scored `bundled` and the experiment printed **CLEAN**.

⭐ **THE FIX** extends the host test to the directories the bed actually uses,
and it is one line beside the first:

```awk
if (p ~ /^\/(usr\/)?(s?bin|libexec|opt)\//) { out["host " p] = 1; return }
```

⚠ **Why extending the list is safe rather than a whack-a-mole.** Nothing
bundled ever lands in those directories: uruntime extracts under `/tmp`
(`appimage_extracted_*`), `--extract` writes `./squashfs-root`, and the
artefact is staged at `/subj*`. So the change can only move a row from clean
to dirty — the same one-way direction as C25 — and never the reverse.

⛔ **WHAT IS AND IS NOT RE-MEASURED.** The committed corpus numbers were taken
under the old predicate and have **not** been re-measured under the new one.
The argument that none of them moves is that all thirteen files are reachable
only by running `stdbuf`, `sudo`, or the host loader by that path, and no
corpus subject does any of those.

⭐ **AND THE ARGUMENT NOW HAS THREE REAL TRACES UNDER IT.** Every distinct
successfully-opened `.so` path was extracted from three captured `strace`
transcripts — one taken inside the bed on `ubuntu-20.04`, two on the runner
host — and classified under both predicates:

| trace | distinct `.so` | host under the OLD predicate | ⭐ host under the NEW prefixes only |
|---|---|---|---|
| bed, `ubuntu-20.04`, mousepad `--no-storefix` | 437 | 33 | **0** |
| runner host, mousepad arm T | 454 | 33 | **0** |
| runner host, mousepad arm N | 437 | 33 | **0** |

⚠ **Three traces and one subject is not the corpus.** It shows the change is
inert where it has been looked at; it does not retire the debt. Pinned in
`evidence/STALE-EVIDENCE.txt` until a corpus re-run says so directly.

⭐ Covered by two new rows in `sh experiments/lib.sh --selftest` (11 pass),
using the two real paths above as fixtures rather than invented ones.

---

## C50 — a check in `102-` that could never fire, for the reason `65-` is resumable

`experiments/102-`'s **R1** asks the question the whole C38 re-run rested on:
*does a real trace exec the artefact once, or twice?* It looked for a trace in
one place:

```sh
TR=$(ls -1t /var/tmp/t065/tr.* /var/tmp/t065b/tr.* 2>/dev/null | head -1)
```

⛔ **And `experiments/65-` deletes each trace the moment it has counted it** —
disk is that experiment's binding constraint and its own header says so. The
one directory R1 looked in is therefore empty *by construction*, so R1 could
only ever report `no trace on disk`.

⚠ It did not read as a defect because a **skip is not a failure**: `102-` ran
`pass=20 fail=0 skip=0` on a machine that still had leftovers, and
`pass=19 fail=0 skip=1` on a clean one. Both look fine.

⭐ **THE FIX** is to search any experiment work directory, which costs nothing
because R1 already takes the artefact name *out of the trace* instead of
assuming it — a `/subjA` from `107-` reads exactly like a `/subj65` from
`65-`. `experiments/107-` now deliberately keeps the first environment's pair
as `keep.tr.*`, so there is a trace on disk for R1 to find.

⛔ **THE LESSON.** A skip that is structural is a check that does not exist.
Neither gate can tell the two apart, so the only thing that catches it is
asking, of each skip, *what would have to be true for this to fire?*

---

## C51 — `T .mo = 43` is 43 catalogue FILES read, not 43 translations, and the control is what makes it mean anything

⚠ **This entry does not overturn a result. It corrects how one is READ**, and
it is here because the number came within one step of being reported as
something it is not.

`experiments/101-` (rung 3's differentiator) reports `T .mo` — catalogues
opened under the bundle — and every one of the eleven rows reads **43**. ⭐
Three things about that number invited a wrong reading, and all three were
chased down against a kept trace rather than argued:

**1. ⛔ 43 is exactly the number of catalogues the bundle carries.** A count
that equals the file count looks like an enumeration, not a lookup. It is:
`AppDir/share/locale/de/LC_MESSAGES/` holds exactly 43 `.mo` files and all 43
are read.

**2. ⛔ The domains do not match the application.** Among the 43 are `acl.mo`,
`elfutils.mo`, `gdbm.mo`, `attr.mo` and `aspell.mo`. **No GTK text editor binds
those domains.** So "43 catalogues opened" is not "43 translations resolved for
`mousepad`", and the docs must not say it is.

**3. ⛔ The artefact also WRITES 43 `.mo` files, on both arms.** uruntime
extracts before it runs, and the extractor creates every catalogue it carries:

    open("share/locale/de/LC_MESSAGES/acl.mo", O_WRONLY|O_CREAT|O_EXCL|…) = 5

⚠ **That write is a RELATIVE path through `open(2)`, not `openat(2)`**, so
`mo_opened`'s `openat\(.*\.mo"` pattern never saw it — the instrument was
correct by accident. ⭐ An explicit `O_WRONLY|O_RDWR|O_CREAT` exclusion is now
in it anyway, because "correct by accident" is not a property to rely on.

## ⭐ AND THE CONTROL IS WHAT SETTLES IT

The worry worth having was the operator's bar: *a criterion must fail for the
right reason, and must not be something a broken bundle also satisfies.* If
the 43 were a directory walk, a bundle whose `gettext` was completely broken
would still show 43. ⭐ **The control answers that directly**, measured on
`alpine-3.22` with both traces kept:

| | arm T (interposer) | arm N (`--no-storefix`) |
|---|---|---|
| trace lines | 244,968 | 72,954 |
| **`openat` catalogue lookups** | ⭐ **258** | ⛔ **0** |
| of those, succeeding | 43 | 0 |
| extraction writes (relative `open`) | 43 | 43 |
| unrewritten `/nix/store` opens that FAILED | — | ⭐ **33** |

⛔ **Both arms extract identically. Only the subject ever LOOKS UP a
catalogue.** The difference is not 43 versus 0 files on disk — it is 258
lookups versus none, from the same artefact with one mechanism switched off.
That is a one-directional difference the interposer causes, which is what
rung 3 claims.

⭐ **The correct sentence is therefore**: *with the interposer, the compiled-in
locale prefix resolves and the catalogues under the bundle are read; without
it, the process never reaches a catalogue path at all because it fails further
up, on the store paths themselves.*

---

## C52 — L2 demanded a syscall its own mechanism prevents

`experiments/101-`'s **L2** is the control's positive observation, and it asked
for the control to be seen **attempting an unrewritten `/nix/store/….mo`**.
It read **0 of 11** — a red row on every environment.

⛔ **It cannot be anything else.** Measured on the kept traces: without the
interposer the run fails *above* gettext. Nearly every `/nix/store` path it
touches returns `ENOENT` — **33 of them on `alpine-3.22`**, glibc's own locale
data among them — so no catalogue path is ever constructed and the syscall L2
waits for is never issued.

⚠ **This is the third criterion in this file that could not fire** (C48 the
locale itself, C50 `102-`'s R1, and now this), and the family is one thing: *a
criterion written from what the mechanism ought to look like rather than from
what it does.*

⭐ **THE FIX keeps delivery rule 4 and moves the layer.** A positive
observation is still required; it is now *"tried an unrewritten store path and
was refused"* — the mechanism itself — instead of *"tried a catalogue path"*,
which the mechanism prevents.

⭐ **AND TWO COLUMNS WERE ADDED WHILE THE TRACES WERE OPEN**, both for things
the instrument had been blind to:

* `Thost` / `Nhost` — catalogues read from the **host** under `/usr`.
  `mo_opened` excludes `/usr/`, so an arm that fell back to the host's German
  catalogues would report the same **0** as an arm that read nothing at all.
  `debian-11` ships 15 of them and `fedora-42` ships 33, so the fallback was
  available. ⭐ Measured: **0 on both arms** — it does not happen. Same family
  as **C49**, "not under a known-bad prefix" standing in for "under the
  bundle", but for DATA, which is why the host-object count cannot see it.
* ⛔ **The first environment's traces are now KEPT.** This experiment deleted
  every trace it took, so the one question its own zeros raised — *what did
  the control do instead?* — could not be answered without re-running all
  eleven. One pair is ~46 MiB; eleven pairs is not affordable here.

---

## C53 — the interposer had a LOADED AND INERT state, and it looked exactly like working

⛔ **This is the silent-failure family again, in the one place it costs the
most**: `tool/runtime/pgb-storefix.c` is the mechanism nearly every result in
this tree turns on.

`fix()` is a pure substitution driven by a map the bundler writes to
`.storemap`. Its guard read:

```c
if (!ready)
    load_map();
if (!nrows || !appdir_len)
    return p;          /* ⛔ and not a word */
```

⭐ **So an artefact whose interposer is preloaded but whose AppDir could not be
found — or whose `.storemap` is missing or empty — rewrites NOTHING and says
NOTHING.** It behaves precisely like a bundle built `--no-storefix`, and every
experiment here that asks *does the interposer work* would score that as a
legitimate negative result rather than as a broken instrument.

⚠ **`load_map`'s own "no `.storemap`" line is gated behind
`PGB_STOREFIX_DEBUG`**, which is not set in any run that matters, so it could
not close the gap either.

⭐ **THE FIX is one line, once, unconditionally** — and only after a real
`/nix/store` path has actually been seen, so a bundle that never touches one
stays quiet:

    pgb-storefix: a /nix/store/ path was seen but NOTHING CAN BE REWRITTEN
    (the AppDir was not found). The interposer is loaded and INERT.

⚠ **It cannot fire spuriously.** `fix()` calls `load_map()` immediately above,
so the map load has been attempted by the time the guard runs — an early call
during libc initialisation cannot trip it. And `--no-storefix` does not ship
the object at all, so a legitimate control cannot trip it either.

⭐ **VERIFIED FOUR WAYS** against a standalone build of the object, before
commit:

| case | expected | measured |
|---|---|---|
| inert, a `/nix/store` path touched | one warning | ⭐ one |
| inert, **no** store path touched | silence | ⭐ 0 lines |
| inert, three store paths in ONE process | once | ⭐ 1 |
| ⭐ **valid `.storemap`** | silent **and rewriting** | ⭐ 0 warnings, and `cat /nix/store/deadbeef-thing-1.0/hello` printed the file from the fake AppDir |

⚠ **How it was found**: not by a gate, and not by looking for it. Three probe
runs on the RUNNER HOST read 0 catalogues where the bed reads 43, and the trace
said the interposer was loaded. Chasing *why the artefact behaved as though it
were not there* is what turned up a state in which that is exactly what
happens, with no diagnostic. ⛔ The host puzzle itself is NOT explained by this
entry and is not claimed to be — the runner host is not the measurement bed and
nothing here is reasoned from it.

---

## C54 — "clean on all eleven" counted subjects whose program NEVER STARTED

⛔ **This one reaches a headline number.** `docs/AGENTS.md` §9 and
`TODO/PROGRESS.md` both quote the corpus as *24 of 26 subjects clean on all
eleven*, and `experiments/65-` computed that with one line:

```sh
nhost=$(exp_classify_trace "$tr" /subj65 | grep -c '^host ')
[ "$nhost" = 0 ] && clean=$((clean+1))
```

⭐ **Zero host shared objects is also what a subject that never started
reports.** The tree knows this — `experiments/68-`'s E11 and `101-`'s L4 both
require the subject to have *drawn* before its cleanliness counts — and the
corpus, which produces the number everyone quotes, had no such guard.

⭐ **MEASURED, off the committed rows.** Five of the twenty-six read
`pass 0/11` and `clean 11/11`:

| id | subject | did the program run? |
|---|---|---|
| `field-3` | `flameshot` | ⭐ **yes** — it put a 3×3 `Qt Selection Owner` on the server; it just has no toplevel, so a `gui` row cannot pass on it |
| `field-4` | `gearlever` | ⭐ yes — it reaches a `RuntimeError` from libadwaita |
| `py-2` | — | yes |
| `vulkan-3` | `vkmark` | yes; the bed has no `/dev/dri` |
| ⛔ `field-2` | `neovim` | ⛔ **NO.** Its closure's own glibc 2.26 rejects the loader invocation, so **nothing of the artefact was ever mapped** |

⛔ **For `neovim`, "it loaded no host object" is not a cleanliness result — it
is the ABSENCE of a measurement**, and it was counted toward the headline.

⭐ **THE DISCRIMINATOR IS THE OTHER HALF OF THE CLASSIFIER'S OWN OUTPUT, which
was already being computed and thrown away.** A subject that started loaded at
least one object *out of the bundle*; one that never started loaded nothing at
all. So a row now counts as clean only when

    bundled > 0   AND   host == 0

and a row with **both** at zero is reported by name —
`⛔ <id>/<env>: loaded NOTHING, host or bundled — the artefact never started`
— rather than silently not counted. A new `NOSTART` line appears in the
summary.

⚠ **The guard is one-way**: it can only stop a row being counted clean, never
add one. ⚠ **And it would be wrong for a static payload**, which loads no
shared object by construction — the bundler refuses those (no loader in the
closure, see `store-paths.md` §3), so no row here is one.

⛔ **THE COMMITTED NUMBER IS NOT RE-MEASURED.** `24 of 26` was taken under the
old rule and stands in the record until the corpus is re-run; it is pinned in
`evidence/STALE-EVIDENCE.txt` alongside **C49**, which owes the same re-run.
⚠ At most one subject is affected on the evidence above, but "at most one" is
an argument and the ledger takes measurements.

---

## C55 — the corpus's last unexplained row is explained, and my diagnosis of it was wrong

`experiments/65-` had one row that disagreed with itself: **`qt-1`
`qalculate-qt`, pass 11/11, clean 4/11.** `experiments/107-` measured it —
**9 pass, 1 fail**, and the failure is the finding.

| check | result |
|---|---|
| **Q1** arm A reproduces the corpus | ⭐ **clean on 4 of 11**, window on **11 of 11** |
| **Q2** `/bin/sh` is dynamic on all eleven | ⭐ **11** |
| **Q3a** no row leaks a Qt/X11/GL host object | ⭐ **0** |
| **Q3b** every dirty row spawned a shell | ⭐ **7 of 7** |
| ⛔ **Q4** arm B (`--with-program dbus-daemon dbus-launch`) is clean on all eleven | ⛔ **4** |

## ⛔ TWO PRE-REGISTERED CLAIMS OF MINE FELL, WHICH IS WHY THEY WERE WRITTEN DOWN

**1. "The four clean rows are the four musl ones, because Alpine ships a STATIC
busybox."** ⛔ Refuted by `readelf -l` *before the run*: `/bin/sh` is a dynamic
PIE on **all eleven**, alpine's busybox included (`interpreter
/lib/ld-musl-x86_64.so.1`). ⭐ The musl/glibc correlation is nonetheless
**real** — it just has a different cause, below.

**2. "Qt autolaunches a session bus, fails to find `dbus-launch`, and falls
back through a shell; `--with-program dbus-launch` closes the row."** ⛔ **Arm
B carries `dbus-daemon` AND `dbus-launch` and is clean on exactly the same four
rows.** The mechanism was wrong.

## ⭐ THE REAL MECHANISM, OFF THE TRACE

    execve("/bin/sh", ["sh", "-c", "--",
            "/nix/store/b5c8ki47…-gnuplot-6.0.5/…"])

**`qalculate-qt` probes for GNUPLOT through a shell.** That is the spawn, on
every environment, in both arms.

⭐ **AND THE musl/glibc SPLIT IS THE SHELL'S FATE, NOT ITS LINKAGE.** On the
four musl rows the exec never completes — the child goes straight to
`+++ exited with 127 +++` having loaded **nothing**. On the seven glibc rows
the shell **runs**, loads the host libc, and only *then* fails to find the
(untranslated) path.

⭐ **AND THE COUNT TRACKS WHICH SHELL THE ENVIRONMENT SHIPS**, which is the
clearest evidence that none of it is the bundle's:

| environments | host objects | shell |
|---|---|---|
| debian-11, debian-12, ubuntu-20.04 | **1** | `dash` → `libc.so.6` |
| fedora-42 | **2** | |
| rockylinux-8, archlinux | **3** | `bash` → `libc.so.6`, `libdl.so.2`, `libtinfo.so.6` |
| opensuse-leap-15.6 | **4** | |

## ⭐ WHAT THE ROW MEANS

**The bundler is not leaking.** Q3a is zero on every row: no Qt, X11, GL or GTK
object is ever loaded from the host. The residue is a **host shell the
application asked for** and the bundle never carried.

⚠ **AND WHAT WOULD ACTUALLY CLOSE IT IS NOT A PATH FIX.** An application that
shells out to the host loads the host's libc *through that shell*, and no
amount of path rewriting prevents it — the bundle would have to **carry a
shell** and be found first. Not implemented, not measured; see
`TODO/research.md` **T-094**.

---

## C56 — the `--wrap=iconv` claim had never been measured, because its arm was skipped every run

⛔ **`docs/research/bundle-capabilities.md` calls this "the one place our answer
is structurally better than theirs".** It was reasoning, not a result: the arm
that measures it had been **SKIPPED on every run of `experiments/30-`**, and a
skip is not a failure so both gates stayed green over it.

`pgb` builds GNU libiconv **inside the build environment**, so the archive is
at `<env root>/opt/pgb-libiconv/lib/libiconv.a` — `cmd/pgb/doctor.go` looks for
it exactly there. `experiments/30-` looked at `/opt/pgb-libiconv` on the
**HOST**, where it has never been:

    SKIP  arm B (static libiconv)  (no libiconv.a at /opt/pgb-libiconv …)

⭐ **Same family as C48, C50 and C52** — an instrument looking where the thing
never is, made invisible by a skip. The fix tries both paths, host first,
**reports which one it used**, and separates *"the archive is not there"* from
*"it is there and the link failed"*, which the old message could not.

## ⭐ AND THE RESULT IS LARGE

`pass=24 fail=0 skip=0`, up from 23 pass and 1 skip — **zero skips remain in
the committed evidence of the whole tree.**

| | arm A: `-static`, plain glibc `iconv` | arm B: `--wrap=iconv` → static GNU libiconv |
|---|---|---|
| the four musl rows | `opened=1 failed=11 roundtrip=0` | ⭐ `opened=12 failed=0 roundtrip=1` |
| the seven glibc rows | ⛔ `<no output>` — **it crashes** | ⭐ `opened=12 failed=0 roundtrip=1` |

⭐ **So the shipped mechanism takes a static binary from one encoding of twelve
— and a crash on every glibc host — to twelve of twelve with a byte-exact round
trip, on all eleven environments.** It cost ~1.9 MiB on the probe.

⚠ The claim was true. It was simply never checked, and "true and unchecked" is
what this file exists to catch.

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
