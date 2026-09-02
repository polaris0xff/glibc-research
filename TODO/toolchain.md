# toolchain — pgb itself

Design: [`../docs/design/toolchain.md`](../docs/design/toolchain.md).

---

## T-010 — Split `pgb` into `tool/lib/*.sh`

**Source** operator, session of 2026-09-01.
**Category** toolchain · **Priority** P1 · **Effort** S · **Status** ✅ done

**Problem.** `pgb` is one file of ~730 lines and the dependency planner is not
written yet. It only grows from here. ⚠ It was 813 by the time this was
started, which is the entry's own point about it getting harder.

**Premise.** ⭐ Decided, recorded in `docs/design/toolchain.md`: the driver
stays POSIX `sh` because `pgb build` re-enters itself inside the build
environment and `pgb verify` enters every target rootfs, where `sh` is the only
thing guaranteed present. ⚠ This is a decision to confirm, not one taken — see
T-011.

**Approach.** Sourced, not executed, so the re-entry stays one process:
`tool/lib/{common,env,wrappers,build,verify}.sh`, with `pgb` reduced to option
parsing and dispatch. No behaviour change in this entry.

**Prove.** `sh pgb doctor && sh pgb explain && sh pgb verify <a known binary>`
produce byte-identical output to the pre-split version, and
`sh experiments/60-versus-alternatives.sh` still exits 0.

**Closed with.** `pgb` 813 → 141 lines; the rest is five sourced libraries.
The segmentation was checked to be lossless before anything was written —
concatenating the ten segments with newlines stripped reproduces the original
file exactly — so nothing could be dropped or duplicated by hand.

⭐ **Eight commands, byte-identical, exit codes equal**, against a copy of the
pre-split file kept for the purpose:

```
IDENTICAL  doctor    exit 0    IDENTICAL  ccdir     exit 0
IDENTICAL  explain   exit 0    IDENTICAL  version   exit 0
IDENTICAL  envinfo   exit 0    IDENTICAL  badcmd    exit 2
IDENTICAL  help      exit 0    IDENTICAL  verify    exit 0   (11 rows, all ok, none)
```

⭐ **And the part a byte comparison cannot reach: the re-entry.** A split
breaks `pgb build` if the libraries are not visible on the far side of the
engine boundary, and `doctor`/`explain` would never notice. All three engines
were run after the split:

| engine | result | size | `.comment` of the binary it produced |
|---|---|---|---|
| host | ok | 2,237,528 | `GCC: (Ubuntu 13.3.0-6ubuntu2~24.04.1)` — this machine |
| chroot | ok | 2,160,816 | `GCC: (Debian 12.2.0-14+deb12u1)` — the pinned environment |
| docker | ok | 2,160,816 | `GCC: (Debian 12.2.0-14+deb12u1)` — the pinned environment |

⭐ **The chroot and docker binaries are byte-identical** (`cmp` clean), and both
differ from the host build. That is a stronger statement than either arm alone:
the two engines are not merely both working, they are **interchangeable**, which
is what "pinned build environment" has to mean to be worth pinning. It also
retires the last reason to treat the docker engine as an approximation of the
chroot one.

⚠ On the first attempt the chroot arm exited 2, because no chroot environment
had been created on this machine — the docker image had. The pre-split file,
kept and run as the control, printed the same message and the same code, so it
was never a split regression. `pgb env create` then made the row above real
rather than argued.

⚠ **`experiments/60-` was not re-run.** It needs `cargo`, `musl-gcc`,
`mksquashfs` and `flatpak` and skips the arms it cannot build, so on this
machine it would have exercised two arms and reported 0 either way — weaker
evidence than the three-engine table above, not stronger. The next machine
that can build its arms should run it.

⚠ **Found while proving this, and deliberately NOT fixed here** because this
entry is *no behaviour change*: `die()` prints its exit code as part of its
message (`pgb: no build environment. run: pgb env create 2`). It uses `$*`
where it means `$1`. Fixed in the commit after this one.

## T-011 — Confirm or overturn the language decision

**Source** operator · **Category** toolchain · **Priority** P1 · **Effort** S · **Status** ✅ done

**Problem.** `docs/design/toolchain.md` records "keep the driver in POSIX sh"
as a recommendation. It has not been ratified, and it gets harder to revisit
the longer it stands.

**Premise.** The constraint that decides it: anything running *inside* the
build environment or a target rootfs must exist there. ⚠ Untested assumption —
whether a static Rust or Zig helper could simply be *carried in* has not been
tried, and if it can, the constraint weakens considerably.

**Decision.** Recommend `sh` for the driver, a real language outside. The
alternative loses on bootstrap, not on ergonomics. ⭐ **If the planner outgrows
the split in T-010, that is the signal to revisit — not a reason to have chosen
differently now.**

**Prove.** A one-paragraph ruling appended to `docs/design/toolchain.md` and
this entry closed with it quoted.

**Closed with.** ⛔ **The premise was disproved, and it keeps its title.** The
"⚠ Untested assumption" above is now tested and the answer is that a
carried-in helper is exactly as available as `sh`.

`experiments/70-carried-helper.sh`, `evidence/70-carried-helper/RESULT.txt`.
The same helper — open a file on whatever filesystem it landed on, parse it,
report — built five ways and carried into the eleven pinned rootfs **plus the
pinned build environment**, twelve targets:

```
                        sh   c-plain-static  c-pgb  rust-gnu-static  rust-musl-static
  12 targets, ok on     12         12          12          12               12
```

⚠ **Read narrowly.** It measures whether a helper **executes**, not whether it
is correct on every libc path — which is why plain `gcc -static` also scores
12 of 12 here where `ci/probe.c` fails on 11 of 11. Availability, nothing more.

**The ruling**, quoted from `docs/design/toolchain.md`:

> ⭐ **The driver stays POSIX `sh` — but not for the reason above, and the door
> this page had closed is measured open.** […] ⛔ So "the alternative loses on
> bootstrap" is withdrawn as written, and with it any reading of this page
> that says the planner cannot be written in a real language. It can.

⚠ **A bootstrap circularity is named there and is not solved here:** a
carried-in planner must be built before it can be carried, so it is built with
a plain static toolchain first and made portable by `pgb` second.

## T-012 — `pgb build <url-or-package>`

**Source** operator · **Category** toolchain · **Priority** P1 · **Effort** XL · **Status** open

**Problem.** A developer still has to know how to build the project.
`pgb build -- make` is a toolchain injector, not a toolchain.

**Premise.** ⭐ The interface is achievable over a large package database:
`nix bundle nixpkgs#chromium` does it today
(`docs/research/nix-appimage.md`). ⛔ Its store model must not be copied.

**Approach.** ⚠ **XL — this is two or more entries pretending to be one.**
Split before starting: spec resolution (URL or package name → source tree),
build-system detection, and the dependency planner are separate.

**Prove.** `pgb build <a git URL>` produces a binary that `pgb verify`
passes on 11 of 11, with no other input from the operator.

## T-013 — Measure developer friction

**Source** operator · **Category** toolchain · **Priority** P2 · **Effort** S · **Status** open

**Problem.** `docs/comparison.md` states the friction axis from one session's
record. Nothing re-runs it, so it goes stale silently.

**Approach.** `experiments/63-developer-friction.sh`: count external artefacts
fetched, files authored, environment variables required, and whether each route
completes unattended.

**Prove.** `sh experiments/63-developer-friction.sh` exits 0 and
`evidence/63-developer-friction/RESULT.txt` carries the counts.

## T-014 — `pgb verify` ignores `--engine`

**Source** found while getting CI green (T-040), session of 2026-09-01.
**Category** toolchain · **Priority** P1 · **Effort** M · **Status** ✅ done

**Problem.** ⛔ **A documented flag does nothing on this command.** `--engine`
is parsed globally and honoured by `cmd_build` and `env_create`; `cmd_verify`
never reads it. It calls `internal/rootfs/run.go` directly, which is
`unshare --mount` + `chroot`, so `pgb verify` needs root and `CAP_SYS_ADMIN`
and **cannot run on a CI runner at all** — the one place a user most wants the
tool's own verdict.

**Premise.** ⭐ **Measured, this session.** The same eleven digest-pinned
environments were run both ways: through the chroot bed with `pgb verify`, and
by hand with `docker run --entrypoint <image@digest>`. Both returned 11 of 11
correct with zero host shared objects, on both a host-built and a
docker-built binary. The two beds agree, so a docker engine for `verify` is a
port of the harness, not a new instrument.

⚠ **What does not port for free is the evidence, and it is the whole point of
the command.** `trace_host_objects` and `trace_host_data` run `strace -f`
*outside* the target and attribute by the pid that `execve`d the binary.
Under docker the process runs inside the daemon's namespaces, so the
equivalent is `docker run --cap-add=SYS_PTRACE` with `strace` **inside** the
container — which needs an strace in the target image, and eleven distribution
images do not have one. ⭐ Static `strace` is exactly the class of artefact
this project builds; carrying one in is the candidate answer and it is
untried.

**Approach.** `cmd_verify` gains the same `case $(pick_engine)` the other
commands have. The chroot arm is today's code unchanged. The docker arm runs
the binary by digest and reports the run column; the trace columns say
`unmeasured` until the carried-in tracer works, ⛔ **never `none`** — an
absence is not a zero, `../docs/methodology/experiments.md`.

**Prove.** `sh pgb verify --engine docker <binary>` prints the eleven-row table
on a machine with no `CAP_SYS_ADMIN`, its RESULT column matches the chroot
arm's on the same binary, and the workflow gains a job that runs it.

### Closed with

⭐ **Every clause of the acceptance, on a runner.**
[Run 14](https://github.com/polaris0xff/glibc-research/actions/runs/33512788793),
15 jobs, all green, `verify-docker` among them:

```
success   pgb verify --engine docker
success   The control must NOT pass its own verify
```

That is the whole `Prove`: the eleven-row table printed on a machine with **no
`CAP_SYS_ADMIN`** — a GitHub runner grants none, which is why this command
could not run there at all before — its RESULT column matching the chroot
arm's, and a workflow job that runs it. ⚠ The second step is the one that
makes the first mean anything: a `verify` that passed everything would be
worthless, and it is the only step that would ever notice.

**Landed.** `cmd_verify` dispatches on `pick_engine` like every other command.
The docker/podman arm derives `repo@digest` from the same
`scripts/common/rootfs-images.txt` the chroot bed unpacks, so the two arms
describe the same environments rather than two sets with one name, and enters
with `--entrypoint` so the subject is the only process in the container.

⭐ **Both arms agree on the RESULT column**, which is what the entry asked for:

| binary | chroot | docker | verdict |
|---|---|---|---|
| `probe-portable` | 11 × ok | 11 × ok | exit 0 |
| `probe-plain` (control) | — | 4 × exit1, 3 × SIG11, 3 × SIG8, 1 × exit1 | exit 1, "NOT portable as built" |

⚠ The chroot arm's output is otherwise **byte-identical** to before this
change; `diff` against the T-010 baseline shows one added line, `engine:
chroot`.

⭐ **And criterion 2 is measured now too, by a tracer carried in.**
`tool/runtime/pgb-trace.c` is a `ptrace` open-tracer, statically linked and
mounted into the container beside the subject — the approach this entry
proposed, built. `experiments/70-` is what makes it viable: a carried-in
helper runs on 12 of 12.

The two arms now agree on **all eleven rows for both asserted columns**:

```
                     chroot                     docker (carried-in tracer)
  RESULT             11 x ok                    11 x ok
  HOST .so LOADED    11 x none                  11 x none
```

⭐ **And on the control the tracer shows what §2 has always claimed, directly:**

```
  debian-11    SIG6  ld-linux-x86-64.so.2  gconv/ISO8859-1.so  libc.so.6
  debian-12    SIG8  ld-linux-x86-64.so.2  gconv/ISO8859-1.so  libc.so.6
  opensuse     SIG8  ld-linux-x86-64.so.2  libnss_compat.so.2  libc.so.6
  archlinux    SIG8  ld-linux-x86-64.so.2  libnss_mymachines.so.2  libgcc_s  libc.so.6
  fedora-42   exit1  ld-linux, libnss_resolve.so.2, libnss_myhostname.so.2, libc.so.6, libm, libcap, libgcc_s
```

A plain `gcc -static` binary pulling in the host loader, the host libc, host
NSS modules and a host gconv module — and the pgb binary loading none of them
on the same eleven rows. That is the project's central claim, measured by the
tool's own command rather than by an experiment written to show it.

⛔ **`unmeasured` is still what it prints when the tracer does not attach**,
and never `none`. Emptiness is byte-for-byte what a clean binary produces, so
the tracer emits `status=traced` and the caller keys on **that** rather than
on having seen no paths. A row that reads `unmeasured` sets neither pass nor
fail, and the closing verdict names it so it cannot be read as criterion 2
holding.

⚠ **Two instrument defects were found and fixed while building it**, both in
`../docs/history/corrections.md`: reporting an open at syscall **entry**
counted every path merely probed for — a false positive on the criterion-2
column, since glibc probes several paths for a shared object and takes the
first that answers; and pairing entry/exit stops with a bare toggle drifts
forever after `execve`'s extra `SIGTRAP`, which made `/bin/true` — a binary
that unmistakably loads `libc.so.6` — report opening nothing at all.

⚠ **One environment difference between the beds is real and is now its own
entry (T-015):** the OCI pull unpacks an image's filesystem and drops its
config, so `LANG=C.UTF-8` from the `archlinux` image applies under docker and
not under chroot. It shows up only in the host **data** column, which is
never asserted.

⚠ **And the tracer is an observer that changes the answer on two rows**, for
the CONTROL binary only: `../docs/history/corrections.md` C11. Reproducible,
no verdict moves, and both instruments are kept because the disagreement is
what found the syscall-entry defect.

⛔ **A defect found on a runner, and the guard that followed.** The first
version of the tracer resumed the tracee with signal 0 at every non-syscall
stop, which **suppresses** it. A binary that takes SIGFPE therefore re-executes
the faulting instruction forever. `pgb verify --engine docker` on the control
got through 4 of 11 rows in **19 minutes** on a CI runner and was still going;
the portable binary, which faults nowhere, had done all 11 in 43 seconds.
Fixed by re-injecting the pending signal — never `SIGTRAP`, which is ptrace's
own and would kill the subject. The same matrix now completes in **3.9
seconds**.

⭐ **The bound is the durable part.** Every docker run in `cmd_verify` is
wrapped in `timeout ${PGB_VERIFY_TIMEOUT:-120}`, so the *next* instrument
defect costs one row and a visible `exit124` rather than a runner.

## T-015 — the OCI pull unpacks the filesystem and drops the image config

**Source** found while cross-checking the two `pgb verify` arms (T-014),
session of 2026-09-01.
**Category** toolchain · **Priority** P2 · **Effort** S · **Status** open

**Problem.** An OCI image is a filesystem **and** a configuration.
`internal/ociimg/pull.go` unpacks the layers and ignores the config, so the
chroot bed and a `docker run` of the same digest are not the same environment.

**Premise.** ⭐ **Measured**, `../docs/history/corrections.md` C10. The
`archlinux` image config carries `Env: LANG=C.UTF-8`; `docker run` applies it,
the chroot bed does not, and the same binary therefore takes a different
`setlocale` path under the two beds:

```
docker run --rm --entrypoint /usr/bin/env archlinux@sha256:818793c8… | grep LANG
  -> LANG=C.UTF-8
the chroot bed sets no LANG at all
  -> 0
```

⚠ **This invalidates no committed number.** The difference showed up only in
the **host data** column, which `../docs/AGENTS.md` §3 reports and never
asserts, and the two arms agree on all eleven rows for both asserted columns.
What was wrong was the *claim* that the beds are the same environment.

**Approach.** `internal/ociimg` already parses the manifest; the config blob it
points at carries `Env`, `Cmd`, `Entrypoint` and `WorkingDir`. Record them
into `.oci-provenance`, and have `internal/rootfs` (`pgb rootfs run`) apply
`Env` unless the caller overrides it.

⛔ **Applying it must be a decision, not a default that appears silently.** A
bed that starts exporting `LANG` changes what every locale-sensitive result
describes, so the change lands with the affected experiments re-run, or it
lands behind a flag.

**Prove.** `pgb verify --engine chroot` and `--engine docker` on the same
binary produce the same HOST DATA column on all eleven rows, and
`experiments/30-gconv-and-locale.sh` is re-run and its result compared before
and after.

## T-016 — the pinned build environment cannot run CMake or meson

**Source** found while starting T-001 (a C++ project with a real dependency
tree), session of 2026-09-01.
**Category** toolchain · **Priority** P1 · **Effort** S · **Status** ✅ done

**Problem.** ⛔ **A documented capability did not work.** `pgb explain` and the
README both say autotools, CMake, meson and plain make pick the wrappers up
without knowing `pgb` exists. That is true of the *wrappers* and useless if
the environment cannot run the build system. Measured in the pinned image:

```
make         present        cmake        ABSENT
pkg-config   present        meson        ABSENT
                            ninja        ABSENT
                            autoconf     ABSENT
                            automake     ABSENT
                            libtoolize   ABSENT
```

⚠ **One correction to this table, made before it was published.** The first
probe looked for a binary named `libtool` and reported it missing after the
fix too. Debian's `libtool` package installs **`libtoolize`**; there is no
standalone `/usr/bin/libtool`, because that script is generated per project by
`configure`. The package is present and the probe was wrong, not the
environment.

⚠ **And it explains something nobody had connected.** All five passing POCs
are autotools **tarballs** — projects that ship a pre-generated `configure`,
which needs only `sh` and `make`. That was not a coincidence or a preference:
it was the only build system the environment supported, and nothing in the
tree said so. A `docs/AGENTS.md` §9 table full of green rows was describing
one build system's worth of evidence and reading as four.

**Premise.** ⭐ Measured, not inferred: `command -v` for each tool inside
`pgb-env-debian12`. The first attempt to build a CMake project produced
`/bin/sh: 3: cmake: not found`.

**Approach.** `PGB_ENV_PACKAGES` gains `cmake ninja-build meson autoconf
automake libtool`. ⚠ **This changes what `pgb env create` produces**, so an
environment created before it must be deleted and rebuilt; the image is still
pinned by the same digest and only the installed package set moves.

**Prove.** `command -v` finds cmake, meson, ninja, autoconf and libtoolize inside a freshly created environment,
and a CMake project builds through `pgb build`.

**Closed with.** `cmake`, `meson`, `ninja`, `autoconf`, `automake` and `libtoolize` all present after a rebuild, and LevelDB 1.23 — CMake,
C++, static — built through `pgb build` and linked into a C++ subject that
exercises static initialisation order, exception unwinding across a static
link, and RTTI across translation units. `poc/60-leveldb/`.

## T-018 — a `pgb` binary has no `PT_GNU_EH_FRAME`

**Source** `pg83/solo` PR #3, read during the sweep in
`docs/research/solo.md`, session of 2026-09-01b.
**Category** toolchain · **Priority** P1 · **Effort** S · **Status** ✅ done

**Problem.** ⛔ **GCC suppresses `--eh-frame-hdr` for every `-static` link** —
`%{!static|static-pie:--eh-frame-hdr}` in `gcc/config/gnu-user.h` — so GNU ld
leaves the executable with no `.eh_frame_hdr` and no `PT_GNU_EH_FRAME`. An
unwinder that discovers tables only through that segment finds none, and the
failure mode is `std::terminate` at the first throw with `catch (...)` in
`main` never running: silent at build time, fatal at run time.

**Premise.** ⭐ **Measured here before it was believed**, and the measurement
changed the entry. Three arms on this machine:

```
  dynamic c++                    PT_GNU_EH_FRAME: 1
  plain g++ -static              PT_GNU_EH_FRAME: 0
  pgb build -- c++               PT_GNU_EH_FRAME: 0
```

⚠ **So it is not a pgb behaviour**, it is GCC's for every static link. And
⛔ **nothing was broken**: `nm` finds `_Unwind_Find_FDE`, `__register_frame_info`
and `__EH_FRAME_BEGIN__` in the binary, so `crtbeginT.o`'s registry answers and
exceptions work. `poc/60-leveldb` throws, catches and asserts on the payload on
all eleven, and it was passing for that reason and not by luck.

⭐ **The hazard is that the fallback belongs to the GNU runtime, not to the
format.** solo hit exactly this with static LLVM libunwind and its own CI
missed it — the one gcc leg did not run the smoke test.

**Approach.** Pass `-Wl,--eh-frame-hdr` on every `pgb` link. A no-op where the
toolchain already emits the header.

**Prove.** `readelf -lW` on a `pgb`-built binary finds `PT_GNU_EH_FRAME`, a
C++ throw is still caught, and `pgb verify` is unchanged on all eleven.

**Closed with.** `internal/wrapper/wrappers.go`, `link_flags()`. Measured after:

| | before | after | delta |
|---|---|---|---|
| `PT_GNU_EH_FRAME` | 0 | **1** | |
| C++ throw/catch | `caught:thrown` | `caught:thrown` | unchanged |
| `ci/probe.c` | 2,161,056 | 2,177,568 | **+16,512** |
| a C++ throw | 2,265,744 | 2,286,344 | **+20,600** |
| `pgb verify`, 11 rows | 11 ok / 11 none | **11 ok / 11 none** | unchanged |

⚠ **The cost is stated rather than hidden and it is not small.**
`.eh_frame_hdr` is a binary-search table over every FDE in the link — 16,004
bytes of section on a static glibc binary. ⭐ The size property
`docs/AGENTS.md` §10 rests on survives: a C program that never calls `iconv`
is **952,536** bytes with the flag, still under 1 MiB against 2.1 MiB for one
that does.

⭐ **And it is a prerequisite for `TODO` T-033.** A loader compiled into the
binary needs the executable's own unwind tables discoverable through program
headers, or an exception cannot cross the boundary in either direction.

## T-017 — `env create` builds one engine's environment; `pick_engine` may choose another

**Source** found running `poc/60-leveldb` (T-001), session of 2026-09-01.
**Category** toolchain · **Priority** P1 · **Effort** S · **Status** ✅ done

**Problem.** ⛔ **The two engines have independent environments and nothing
compares them.** `pgb env create` builds an environment for whichever engine
`pick_engine` returns *at that moment*; a later `pgb build` calls
`pick_engine` again and may get a different one, whose environment is stale or
absent. The failure is not a clear message, it is whatever the missing tool
says:

```
/bin/sh: 1: cmake: not found
```

**Premise.** ⭐ **Measured, twice, this session.** Both times the chroot
environment had just been rebuilt with a new `PGB_ENV_PACKAGES` (T-016) and
the docker image still carried the old package set, so the same
`sh pgb build` that worked with `--engine chroot` failed with the default
engine. ⚠ It is worse than a stale environment: `pick_engine` prefers
podman, then docker, then chroot, so **merely starting `dockerd` changes which
environment every subsequent command uses** — with no warning, and the docker
branch of `cmd_build` only checks that the image *exists*, never that it is
current.

**Approach.** `env create` records the package set and the image digest into
the environment it builds — the chroot arm already writes `.pgb-env` with
exactly that, and the docker arm writes nothing. Give the image the same
record as a label, and have `cmd_build` compare it against the current
settings and **refuse with a named difference** rather than running a build
that will fail confusingly.

⚠ **Do not "fix" this by making `env create` build every engine.** Building
three environments to use one is worse than the problem, and the chroot
environment alone is ~1 GiB. The fix is that a stale one is *detected*.

**Prove.** With a chroot environment created and the docker image absent or
built from a different `PGB_ENV_PACKAGES`, `sh pgb build -- true` names the
difference and exits 2, rather than failing inside the build.

### Closed with

⭐ **The premise was reproduced first, on this machine, before anything was
written.** `pgb doctor` reported `chosen engine: chroot`; one `dockerd` later
the same command reported `chosen engine: docker`, with no other change.

`env_stamp()` writes one canonical line — image, digest, iconv, sorted package
set — and `env_stamp_of()` reads it back from wherever the engine keeps it:
`.pgb-env-stamp` for chroot, an `org.pgb.stamp` **image label** for
docker/podman. ⭐ One function produces it and one consumes it, which is what
stops the two arms drifting, and that is how the defect arrived.
`env_require_current()` runs in `cmd_build` and `cmd_shell` **before** any
mount or container starts.

⚠ **An environment built before the stamp existed is reconstructed, not
refused**: `.pgb-env` already records the image, the digest and the package
set, and the archive's presence answers `iconv`. Exercised here — the chroot
environment on this machine has no `.pgb-env-stamp` and still passes.

**Six cases, measured:**

| case | result |
|---|---|
| docker chosen, no docker image, chroot env present | **refuses**, and names chroot as an engine that does have one |
| settings name a package the environment lacks | **refuses**: `packages MISSING from the environment: libpq-dev` |
| a different image digest | **refuses**, and says the build would *succeed* against a glibc the pin does not describe |
| **`--no-iconv` against an environment that HAS libiconv** | **passes** |
| settings name **fewer** packages than the environment has | **passes** |
| everything current | **passes**, `pgb build -- true` exits 0 |

⛔ **Row four is a defect this entry introduced and then removed.** The first
version compared every field for equality, so `--no-iconv` — a *build* option,
not an environment property — was refused against a perfectly good
environment. ⭐ A difference is not automatically a problem, and each field now
carries the rule it actually has: image differs at all → fatal; a wanted
package missing → fatal, an extra one → a note; `iconv` wanted 1 and have 0 →
fatal, the other direction → nothing.

⭐ **And the docker arm was proved end to end**, not argued: an image built
with a deliberately reduced `PGB_ENV_PACKAGES` is refused with

```
packages MISSING from the environment: autoconf automake bzip2 cmake file
libffi-dev libtool meson ninja-build patch perl xz-utils zlib1g-dev
```

and the control confirms the refusal is not a false positive —
`docker run --entrypoint /bin/sh <that image> -c 'cmake --version'` prints
`/bin/sh: 1: cmake: not found`, **the exact message this entry was opened
with**.

⚠ **A second, smaller defect of the same shape was fixed with it.**
`pgb doctor` probed `$PGB_LIBICONV_PREFIX/lib/libiconv.a` **on the host** and
printed `MISS GNU libiconv (static)` while the chosen engine was `chroot` and
the archive was sitting in the chroot. A MISS for something that is not
missing where it is used sends the reader to fix a working tool. It now
reports the state of the **chosen** engine.

⚠ **What this does NOT do**, stated: it does not compare the environment's
*contents*, only what it was built from. An environment somebody edited by
hand still passes.

## T-019 — the docker engine drops every build option at the container boundary

**Source** found while building `poc/70-sqlite-extensions` (T-002), session of
2026-09-01b.
**Category** toolchain · **Priority** P1 · **Effort** S · **Status** ✅ done

**Problem.** ⛔ **Every documented build option silently did nothing under the
docker and podman engines.** `chroot` inherits the caller's environment; a
container does not, and the docker branch of `cmd_build` passed exactly
`-e PGB_INNER=1`. So the whole `PGB_OPT_*` handoff — `--wrap-dlopen`,
`--embed-locale`, `--no-iconv`, `--arch-baseline`, `-v` — was dropped at the
boundary. No warning, no error, exit 0, and a binary that simply did not have
the mechanism the caller asked for.

**Premise.** ⭐ **Measured, same source, same command, engine the only
variable:**

```
  chroot   __wrap_dlopen=1  pgb_dlopen_libs=1  size=2,453,656
  docker   __wrap_dlopen=0  pgb_dlopen_libs=0  size=2,444,440
```

⚠ **AND IT HID BEHIND A REAL RESULT.** T-010 measured the two engines as
producing **byte-identical** binaries and that measurement stands — it was
taken on a build with **no options**, which is the one case where dropping
them all changes nothing. ⛔ A green cross-check that only exercises the
default path certifies the default path.

⚠ **How it surfaced**: `poc/70-sqlite-extensions` linked fifteen plugins with
`--wrap-dlopen` and the resulting binary contained no `__wrap_dlopen` at all.
It read like a defect in the plugin table.

**Approach.** One list, `PGB_OPT_VARS`, that `export_options` and the
container branches both derive from, so extending one and not the other is not
possible. `-e NAME` without `=VALUE`, so a value containing spaces — which
`PGB_OPT_WRAP_DLOPEN` always has with more than one plugin — cannot be torn
apart by word splitting.

**Prove.** The same `--wrap-dlopen` build under both engines produces binaries
with the wrapper present, and the two are byte-identical.

**Closed with.** `internal/buildx/build.go`:

```
  chroot   __wrap_dlopen=1  pgb_dlopen_libs=1  namespaced=2  size=2,453,656
  docker   __wrap_dlopen=1  pgb_dlopen_libs=1  namespaced=2  size=2,453,656
  cmp: BYTE-IDENTICAL
```

⭐ **So the byte-identical property now holds WITH options too**, which is a
stronger statement than the one T-010 could make.

⚠ **A second defect of the same shape, found and fixed in the same place.**
`cmd_shell` had **no docker branch at all** and fell through to `inner_build`
on the **host**, so `pgb shell` — documented as "an interactive shell inside
it" — handed the caller a shell on this machine with the wrappers on `PATH`.
Same class as T-014: a documented capability quietly doing something else.

## T-050 — Plan a nixpkgs package with NO nix, from the `.drv` in the cache

**Source** ⭐ **operator, session of 2026-09-01c**, quoted because the question
is the finding: *"I think the downloaded nix store files themselves contain
\*.drv files? so we don't actually need nix installed no?"*
**Category** toolchain · **Priority** P1 · **Effort** M · **Status** done

**They are right, and it is measured.** A narinfo names its own producer
(`Deriver: <hash>-<name>.drv`), that `.drv` is **itself a store path in the
binary cache** with its own signature and NarHash, and its `References` are the
`.drv` paths of every input. So the derivation graph is reachable over plain
HTTPS with no nix and no evaluation.

**Landed.** `internal/nixx/drv.go` reads nix's ATerm derivation format (12-check
selftest, including the two escapes that matter and two refusal cases) and
emits the same document `nix derivation show` prints, so `internal/nixx/plan.go`
is shared by both routes. `nix_plan_nonix` in `internal/nixx/build.go` is tried first
and falls back to evaluation.

⛔ **What is NOT done, and it is the reason this entry stays open:**

1. **The cache does not have every `.drv`.** Sampled with `experiments/83-`:
   the rate is real, well under 100%, and a route that works for most packages
   and silently falls back for the rest needs the rate stated wherever the
   route is claimed.
2. **Name → store path is still an index lookup, not evaluation.** An
   override, an overlay, a `pkgsStatic.*` attribute or anything the channel did
   not build is out of reach by this route, by construction.
3. The fallback chain (no-nix → evaluation → a committed `--plan`) is not
   covered by a test that exercises all three.

**Prove.** `experiments/83-drv-without-nix.sh`: the availability rate over a
sample, a plan built by both routes for the same package **compared field by
field**, and the no-nix route driven with nix removed from `PATH`.

## ⭐ CLOSED — `experiments/88-nonix-end-to-end.sh`, 25 assertions, 0 fail

⛔ **The fallback is built, and it is not an evaluator.** The three items above
are answered by an index nobody in this tree had looked for: **hydra built the
channel**, so it holds the derivation for every job it ran.

    hydra.nixos.org/job/<project>/<jobset>/<attr>.<system>/latest-finished
      -> drvpath, system, and every output's store path

That is an index of **builds** rather than a field somebody happened to upload
beside a NAR, so `Deriver:` availability does not bound it. Measured on
**83-'s own twenty packages, with 83-'s own predicate** (the narinfo names a
Deriver **and** that `.drv` is fetchable), so the two numbers compare:

| route | resolved | not |
|---|---|---|
| ⭐ **hydra** | **19** | 1 |
| narinfo `Deriver:` | 9 | 11 |

⭐ **And the one miss is not a gap in the route**: there is no `grep` attribute
in nixpkgs, there is `gnugrep`. Asserted both ways, so "19 of 20" cannot be
read as a 5% failure rate.

⭐ **The control that makes it evidence.** An index can be confidently wrong.
For `jq`, `gawk`, `zlib` and `openssl` the drvpath hydra returns is
**byte-identical** to the one a local `nix-instantiate` computes — same 32-character
hash — so the index is naming the derivation an evaluator *would* compute, not
a similar one. And the two routes' **plans agree on 19 of 19 comparable
fields**.

### ⭐ Item 2 is answered by a second index: `packages.json.br`

It sits beside `store-paths.xz` in the same pinned release directory, is served
with `Content-Encoding: br` so `curl --compressed` decodes it with **no brotli
library on the host**, and carries per attribute:

| | |
|---|---|
| `Name` | `bash` → **`bash-interactive-5.3p15`**, which `docs/research/nix.md` finding 3b says no name match can know |
| `OutputName` | `jq` → **`bin`**, which is the case `internal/bundle/appimage.go` got wrong once |
| `System` | **x86_64-linux**, which is the defect below |
| `Pname` | so `sed` can reach an attribute at all |

### ⛔ THE DEFECT THIS FOUND, AND IT HAD BEEN THERE FROM THE START

**`store-paths.xz` is every system the channel built.** Resolving `nix-2.35.2`
by name in this tree returned an **aarch64-darwin** build — fetched, signature
verified, NarHash checked, and a **Mach-O arm64 executable**. Nothing in the
route could tell. What gave it away was its closure: 57 paths **with no glibc
in it**, and two Apple-only libraries (`libiconv-115.100.1`, `libresolv-96`).
The index has **three** store paths named `nix-2.35.2`.

⭐ Every route that can know the system now states it, and a system nobody
builds is **refused** rather than answered with whatever sorted first. A match
that was not an exact attribute says so too: `sed` reaches **`freebsd.sed`**
through `pname` — a real package for the wrong userland — and the answer
carries `Matched: pname` rather than passing it off as an attribute.

### ⛔ Two more defects, each of which produced a plausible result

- **The streaming reader for `packages.json` dropped a whole chunk on every
  refill**: 103,571 attributes of 149,812, a **31% loss that looks exactly like
  an index**. Caught by comparing against `json.load`'s count on the same file.
  The selftest now runs the walk at four chunk sizes against that control.
- **`nix-plan.py` read dependency derivations out of `env` and ignored
  `structuredAttrs`**, so `src.urls` was **empty for every package on BOTH
  routes** and the upstream-URL fetch fallback — the one that still works when
  a path was never uploaded or has been garbage-collected — had never once been
  exercised.
- **`src.outputHash` came out in two encodings**, hex from the raw `.drv` and
  SRI from `nix derivation show`, so the two routes' plans differed in exactly
  one field and it read like a disagreement about the source when the bytes
  were identical. Normalised to hex.

**Item 3** — the fallback chain covered by one test — is `experiments/88-`
itself: arm 1 exercises hydra, arm 1's Deriver column exercises the channel
index, arm 3 exercises evaluation, and `PGB_NIX_NO_HYDRA=1` and
`PGB_NIX_FORCE_EVAL=1` select the lower rungs.

## T-051 — Enough nix for a host with no root, no docker and no nix

**Source** operator, 2026-09-01c: *"find the least invasive way to 'install'
enough nix so pgb is usable even on the minimal of hosts like containers that
can't run docker images or install nix because no root"*.
**Category** toolchain · **Priority** P1 · **Effort** M · **Status** open

**Problem.** T-050 removes nix from the *planning* step for packages whose
`.drv` is cached. It does not remove it from the cases T-050 lists as out of
reach, and `pgb nix build` still wants a build environment.

**What the mining already says.** `references/nix-community__nix-user-chroot`
(commit `987302aef4e3aa267355cfad00027b730bcb389b`) runs nix as an ordinary
user in a user namespace — and its own README says Ubuntu 23.10+ gates
unprivileged user namespaces behind AppArmor and RHEL/CentOS 7 ship them off,
so it is not a universal answer either. `grigio/docker-nixuser` is the
container form of the same and needs a container.

**Approach, cheapest first.**
1. Push T-050 as far as it goes and measure exactly what is left.
2. For what is left, a static `nix` binary from the cache, run against a store
   under `$HOME` with `--store`. ⚠ Unverified; it is the obvious first probe.
3. `nix-user-chroot` where namespaces are available, with the AppArmor case
   detected and named rather than hit.

⛔ **Not `curl | sh` as root.** That is what this environment did once, on the
operator's explicit authorisation, and it is not the shape the entry is for.

## ⭐ STEP 1 IS DONE AND IT WENT FURTHER THAN THE ENTRY EXPECTED

`experiments/88-nonix-end-to-end.sh` arm 5, **8 assertions, 0 fail**. A rootfs
with a C toolchain and nothing else — and jq **planned, fetched, its
dependency planned and built, and itself built**, inside it:

    uid=12000            not root
    nix on PATH          no
    /nix                 absent
    a C compiler         yes
    build exit status    0
    the binary           4,129,368 B, no PT_INTERP, and `.a[1]` on
                         {"a":["x","é中"]} answers é中

⛔ **One defect had to be fixed for the dependency to build**, and it is the
one that would have stopped this entry at the first real package.
`nix_build_dep` opened with `nix_prefix() || warn "no nix, so a dependency
cannot be planned"` — so the nix-free route reached exactly as far as a package
with no dependencies. **The dependency's own `.drv` is already in the parent's
plan**, so planning it needs no evaluation: `nix_plan_from_drv` now serves it
and the evaluated route is the fallback. jq's oniguruma is planned from
5 derivations fetched over HTTPS and built into the shared prefix, with no nix
anywhere.

⚠ **What arm 5 does NOT show, stated rather than implied:**

- **The chroot is the harness, not the claim.** It is how a host with no `nix`
  and no `/nix` is produced on a machine that has both. Everything asserted is
  about what the process inside could see and do, and it drops to uid 12000
  before `pgb` is reached.
- ⛔ **The host still needs a C toolchain.** `pgb` is a build tool; the rootfs
  used is `pgb-env-debian12`, which has gcc and the static libiconv. A host
  with *no compiler* is not served by this and is what step 2 (a carried,
  relocatable toolchain) is for.
- **This environment's CA bundle lives under `/root`**, which uid 12000 cannot
  read; a readable copy travels in with the harness. The first run failed on
  exactly that and `nix-fetch` reported it as *"hydra has no finished build for
  jq"* — an error naming the failure it expected instead of the one it had.
  Fixed: curl's own message is printed.

**What is left on this entry** is steps 2 and 3 — a host with **no compiler**
— which is `T-060`, the static-glibc nix.

## T-056 — Port the python helpers to Rust

**Source** operator, 2026-09-01c, explicitly filed as *"far future"*.
**Category** toolchain · **Priority** P2 · **Effort** L · **Status** done

⭐ **Superseded by T-061**, which took them to Go instead of Rust.

The entry asked for the four Python helpers — the planner, the derivation
reader, the DT_NEEDED reader and the NAR reader — to stop being Python,
because Python is not present on every host this project claims. T-061 did
that in Go: they are `internal/nixx/plan.go`, `internal/nixx/drv.go`,
`internal/elfx/needed.go` and `internal/nixx/nar.go`, inside the one static
binary, and the Python is retired under `HISTORY/`.

⚠ **Rust was the language this entry named and it is not the language that
was used.** `docs/design/toolchain.md` "Language and structure" carries the
comparison and the reason: Rust wins on rigour, Go wins on the combination
this port needed. ⭐ `nixie-dev/nixie`, which the operator named as the shape
a minimal relocatable nix might take, is still unexamined — that belongs to
T-060, not here.

## T-058 — two `pgb build`s at once share one wrapper directory

**Source** found while running `poc/90-qt` and `poc/20-nano` on the same
machine, 2026-09-01d.
**Category** toolchain · **Priority** P1 · **Effort** S · **Status** done

**Problem.** `make_wrappers` wrote one directory, `$PGB_STATE/bin`, and
`internal/buildx/build.go` bind-mounts `$PGB_STATE` **into** the build environment
(`--bind "$PGB_STATE:$PGB_STATE"`, lines 87 and 141). So the compiler a
running build is executing out of is a directory the next `pgb build` rewrites.

**Half of it is fixed and the fix is in.** The function used to open with
`rm -rf "$wd"`, which takes the wrappers away from a running build between two
compiler invocations — it surfaces as `cc: not found` from inside somebody
else's ninja, minutes into a long build, with nothing pointing at the cause.
Each wrapper is now written to a temporary name and renamed into place, which
is atomic and leaves an already-exec'd wrapper untouched.

⛔ **The half that is left is the OPTIONS, and it is the worse half.** The
wrappers embed `CF` and `LF`, which depend on `--embed-terminfo`,
`--embed-cacert`, `--embed-locale`, `--no-iconv` and `--wrap-dlopen`. Two
concurrent builds with different options therefore share one set of flags,
last writer wins, and **neither build reports anything**: the loser silently
links a runtime it did not ask for, or loses one it did. That is the same
defect class as T-019 (options lost at the engine boundary) arriving from the
other side, and T-019's own note — *"a binary byte-identical to one built
without it, with no error anywhere"* — is what it would look like.

⚠ **This is why this session ran `poc/90-qt` to completion before starting
`poc/20-nano`**, rather than using the idle cores. On a 4-core machine that is
a real cost.

**Approach.** A per-invocation wrapper directory is the obvious answer —
`make_wrappers` already RETURNS the directory it made and the caller exports
it, so the name is not load-bearing anywhere except `pgb cc-dir`. The open
questions are what `pgb cc-dir` should then print, and who removes the
directory when the build is killed rather than finished.

**Prove.** Two `pgb build`s started together, one `--embed-terminfo` and one
plain, both completing, and the binaries checked for the terminfo constructor:
present in exactly one.

## ⭐ CLOSED — `experiments/87-concurrent-build-options.sh`, 8 assertions, 0 fail

**The fix is content-addressing, not a directory per invocation**, and the
choice is what answers this entry's two open questions. `wrapper_dir()` keys
the directory on the wrappers' own inputs — `compile_flags`, both
`link_flags` variants, the baseline, whether `--wrap-dlopen` is in play, and
the resolved real compilers — with `cksum`, the same mechanism `runtime_dir()`
already used:

    cc-dir plain            /root/.local/state/pgb/bin-2110419477
    cc-dir --embed-terminfo /root/.local/state/pgb/bin-4185406337

- `pgb cc-dir` prints a directory that is **stable** for a given option set, so
  hand-driving still works and the name can be re-derived tomorrow;
- **nobody has to remove a directory when a build is killed**, because the next
  build with the same options reuses it byte-for-byte. A per-pid name would
  accumulate one directory per crash and need a reaper that must not run while
  somebody else's build is live — which is this defect rebuilt one layer out.

⛔ **A second, smaller defect had to be fixed for the first fix to be safe.**
`make_wrappers` resolved the real compiler with `command -v` and skipped only
the directory it was *about to write*. With more than one wrapper directory in
existence and `inner_build` putting one on `PATH`, that resolves to a
**wrapper**, and wrapping it again appends pgb's flags twice. `real_compiler()`
now walks `PATH` skipping anything under `$PGB_STATE`.

| arm | what | result |
|---|---|---|
| 1 | the fix, chroot engine, two concurrent builds | `--embed-terminfo` binary has **2** `pgb_terminfo` symbols, the plain one **0** |
| 2 | ⭐ **the control**: `PGB_T058_SHARED_WRAPPERS=1` puts the single shared `bin` back, five attempts | **the two builds agree on one option set in 5 of 5** — `0/0`, `2/2`, `0/0`, `0/0`, `2/2` |
| 3 | ⭐ **the scope**: the same control under docker | `2` and `0` — the container boundary isolates them |

⭐ **Arm 2 is the entry.** A run where both binaries carry the terminfo runtime
is a build that got an option it never asked for; a run where neither does is a
build that lost one it did. **Neither build reported anything either way.**

⛔ **And arm 3 is why the first run of the experiment could not reproduce the
defect at all.** `cmd_build`'s docker branch passes `-e PGB_STATE=...` but does
**not** bind-mount it, so each container gets a private, empty state directory
inside its own ephemeral filesystem. The defect was therefore **chroot-only**,
and an experiment run under the engine `pick_engine` chooses by default passes
for the wrong reason. ⚠ The other consequence of that isolation is a cost, not
a safety property: **every docker build recompiles the pgb runtime objects from
scratch**, because the cache `runtime_dir()` maintains never survives the
container. Not fixed here; it is a performance entry, not a correctness one.

## T-060 — ⭐ STATIC-GLIBC nix: the entry that makes "no root, no docker, no nix" true

**Source** ⭐ **operator, session of 2026-09-01e**, quoted because the framing
is the entry: *"nixpkgs' pkgsStatic is musl and this project is the glibc half.
Produce a static-glibc nix toolchain path end to end: pgb builds nix's own
dependency closure static, or carries enough of one to plan and fetch on a host
with nothing. `nixie-dev/nixie` is the shape the operator named. This is the
entry that makes 'no root, no docker, no nix' true rather than aspirational."*
**Category** toolchain · **Priority** P1 · **Effort** L · **Status** open

**Why it exists, given T-050 closed.** `experiments/88-` arm 5 plans, fetches
and builds a nixpkgs package on a host with **no nix and no root** — and it
still needs **a C toolchain on that host**. Two cases are left:

1. a host with no compiler at all;
2. the cases the index route cannot reach by construction — an override, an
   overlay, a `pkgsStatic.*` attribute, anything hydra never built — which need
   **evaluation**, and evaluation needs a nix binary.

⛔ **And nixpkgs has no static-glibc nix to fetch.** `pkgsStatic` is musl,
measured (`docs/research/nix.md` finding 1), and this project is the glibc half.

**The three rungs, and each is recorded whether it is reached or not** — the
shape `poc/90-qt` used:

| rung | what |
|---|---|
| 1 | nix's dependency closure built **static-glibc by pgb** — how far the dependency walk gets, dependency by dependency, with what stopped each one it could not |
| 2 | nix itself linked against them: a `nix-instantiate` with no `PT_INTERP` |
| 3 | that binary evaluating a nixpkgs attribute **inside a rootfs with no nix, no `/nix` and no root**, on the eleven |

⚠ **The honest risks, named now.** nixpkgs builds nix as **eight component
derivations** (`nix-util`, `nix-store`, `nix-expr`, `nix-fetchers`,
`nix-flake`, `nix-main`, `nix-cmd`, `nix-cli`) under meson, not one autotools
tree; its closure carries **boost, libgit2, libarchive, lowdown, editline,
sqlite, curl+openssl, libsodium, brotli, toml11 and the AWS CRT**, and the AWS
half is optional while boost is not. Any one of those can refuse `-static` the
way MLT's `add_library(mlt SHARED)` did.

⭐ **A cheaper second reading of the same goal, which the operator allowed:**
*"or carries enough of one"*. `internal/nixx/fetch.go` already fetches
nix's own closure from `cache.nixos.org` with no nix and no root — 57 store
paths, 142 MB, signature and NarHash checked — and `experiments/80-` arm 5
already showed that a nixpkgs binary handed **the loader fetched beside it**
runs in a rootfs with no `/nix`. That is a relocatable nix without a single
line of C, and it is the fallback rung if the static build stops.


## ⭐ RUNG 1 MEASURED: `pgb nix deps`, and eleven defects it found

⛔ **`pgb nix deps` is new and it is what rung 1 needed.** nixpkgs' `nix` is
**seven component derivations over ONE source tree**, so no single plan
describes the build — but the union of those plans is exactly the library
closure a static nix needs. `pgb nix deps --plan X` builds a plan's
dependencies into the shared static prefix and stops.

**First pass: 24 built, 32 failed.** The failures were specific enough to fix
rather than to report, and fixing them is the "universal builder" work goal 1
names:

| what stopped | why | fixed by |
|---|---|---|
| boost | `bootstrap.sh` + `b2` is a build system pgb did not know; it said *"No targets specified and no makefile found"* | a b2 branch |
| zstd, libblake3, icu4c | the build file is in `build/cmake/`, `c/`, `icu4c/source/` | `pgb_build_root()` finds the shallowest one and says which it chose |
| lowdown | `oconfigure` takes `PREFIX=` and prints its key list when handed `--prefix` | an oconfigure branch |
| libseccomp | `configure: error: please install gperf` | `gperf` in the environment |
| **all six nix components** | `Meson version is 1.0.1 but project requires >= 1.1` | `meson==1.9.1` pip-installed into the environment, **with the pip set in the environment stamp** |
| doctest | its CMake config looks for MPI | test-only dependencies skipped by default |

### ⛔ And five defects in pgb itself, each of which produced a plausible result

1. **`pgb nix` ran every inner `pgb build` through `pick_engine`**, so
   `--engine chroot` was used for nothing and a freshly rebuilt chroot
   environment was ignored for a stale docker one. T-017 one layer in.
2. **The environment stamp's `packages=[\(.*\)]$` is greedy**, so adding a
   second bracketed field made it capture `a b] pip=[c` — and pgb refused a
   just-created environment for missing a package that was in it.
3. **The Dockerfile put the pip step above the trust anchor**, so pip died
   with *"certificate verify failed"* five retries deep: a message about pypi
   that is really about the order of two lines.
4. ⛔ **The `cpp` wrapper appended LINK flags and the runtime OBJECT to the
   preprocessor.** `cpp foo.c` has no `-c`/`-E`/`-S`, so the wrapper chose
   `mode=link`. On libX11 that produced `pgb-nssfix.o:4:457: warning: null
   character(s) ignored` and `configure: error: .../cpp defines unix with or
   without -undef`. `cpp` is a query tool and is passed through untouched now.
5. ⛔ **`PKG_CONFIG_PATH` had `lib/pkgconfig` and not `share/pkgconfig`**,
   where every architecture-independent package puts its `.pc`. libxcb's
   configure said *"Package 'xcb-proto', required by 'virtual:world', not
   found"* about a package built one directory over.

### ⛔ Three more, and the last one is the worst kind

6. **`pgb nix deps` keyed its per-dependency directories on the NAME alone**,
   so two runs with different `$NIX_PREFIX` shared
   `$PGB_STATE/nix-deps/openssl`; one run's tree was rebuilt under the other's
   feet and the loser failed compiling a demo. T-058 arriving in `pgb nix`.
7. **nixpkgs writes its output paths as placeholders** — `--libdir=$(out)/lib`
   and `--libdir=/02qcpld…52chars/lib` — and passed through, **libxml2
   installed itself into `/02qcpld1y6xhs5gz9bchpxaw0xdhmsp5dv88lh25r2ss44kh8dxz/lib`**.
   `.built/libxml2` was written, the prefix had nothing in it, and libxkbcommon
   then failed for want of a library that had just "built".
8. ⛔ **meson and openssl install into `lib64` while everything else looks in
   `lib`.** `dep ok libxkbcommon` was printed, `.built/libxkbcommon` was
   written, `xkbcommon.pc` was sitting in `lib64/pkgconfig`, and qtbase's
   configure said `XKB_FOUND = "FALSE"`. openssl's `libcrypto.a` was there too,
   in **both** prefixes. ⭐ **A dependency that builds, installs and is
   invisible is the worst of the three outcomes**, and pgb produced it twice.

### ⭐ And one adaptation rule worth having

meson prints the option that turns a feature off, in the error:
*"You can disable the Wayland xkbcli programs with `-Denable-wayland=false`."*
`nix_diagnose` takes that literally — only the disable form, only when the flag
is not already set. Measured on libxkbcommon: two rounds, two of meson's own
suggestions, built on the third.

⚠ **And the plan carries three flag lists while only `configureFlags` was ever
read**, so every cmake and meson package got none of the options nixpkgs chose
for it — including nix's own `-Dgc=enabled`, `-Dcpuid=enabled` and
`-Dseccomp-sandboxing=enabled`.

**Where rung 1 stands:** the closure is being rebuilt with all of the above in
place. ⛔ **Rungs 2 and 3 are not reached and this entry stays open.**

**Prove.** `evidence/89-static-nix/RESULT.txt`: the rung reached, with the
error and the file it came from for the rung that stopped — plus, for any rung
reached, `nix-instantiate` naming a derivation inside a rootfs that has no nix.

---

## T-061 — ⛔ PORT THE WHOLE TOOLCHAIN TO GO, and ship one static `pgb`

**Source** ⭐ **operator, session of 2026-09-02**, and the trigger is quoted
because it is the whole argument: *"when some backticks in some comments inside
a shell script break everything and lead to hours of wasted time, i think it's
time we rewrite the tooling properly."*
**Category** toolchain · **Priority** P0 · **Effort** XL · **Status** ✅ done

⭐ **CLOSED 2026-09-02b, when gate 5's last three rows landed.** The operator's
framing when it was opened: *"create a P0 XL task to port everything to go in
next session and pass all tests/experiments, reach current feature parity …
After the next session ports the whole thing to go, the next session after that
will return back to usual tasks."* That has happened; no entry waits on this
one any more.

### ⭐ WHERE IT STANDS — ALL SIX GATES MET

**The tool is Go.** One static binary; the shell and Python are retired under
`HISTORY/` and are the oracle. 200 carried selftests pass and 1 cannot run here
(no `zstd` binary), `sh TODO/check.sh` and `sh scripts/common/check-docs.sh`
are green, and CI is green at 16 of 16 jobs.

| gate | state |
|---|---|
| 1 nix-index | ⭐ met — identical TSV from the real 399,356,002-byte input; python 5.10 s / 88,756 KiB, go 3.50 s / 12,284 KiB |
| 2 nix-nar | ⭐ met — byte-identical NARs, identical nix-base32 hashes, identical signature decisions, on fixtures and a real cache.nixos.org object |
| 3 parity | ⭐ met — doctor, env info, attr, info, closure, hydra drv and a whole nix-free plan, all identical |
| 4 wrappers | ⭐ met in its strongest form — the same source through both toolchains is BYTE-IDENTICAL |
| 5 the matrix | ⭐ **met** — **ten of ten POCs and twenty-three of twenty-three experiments**, every row measured. The last three landed 2026-09-02b: `poc/91-qt-xcb` (27 assertions, 11/11), `experiments/86-` (7 cases) and `experiments/90-` (10 cases, **0 skipped**) |
| 6 the artefact | ⭐ met — statically linked, no `PT_INTERP`, no `DT_NEEDED` |

Requirement 2's second half is met too: pgb built by pgb inside the pinned
environment is byte-identical to the host build.

⭐ **The remainder is discharged.** Gate 5's unfinished rows are measured, and
the operator's post-port instruction is done: codegraph is installed and wired
into the gates and the rules (`e44a6519`), the Go tree was swept with
staticcheck and gopls' `modernize` (`aa3b7474`, `4376c735`), both deep reviews
ran and their findings are fixed or filed as **T-062**, the unreferenced-document
question is answered in `../tmp/README.md`, and `../docs/AGENTS.md` §0b is the
cold start.

⚠ Requirement 6 is only partly discharged and that is deliberately **not** this
entry's: the reachability sweep exists in Go with a 12-case selftest, but
nothing consumes it — `--debloat` still has its own rules and the sweep is a
reporting command. ⛔ Re-confirmed 2026-09-02b with codegraph: `Sweep` has
exactly two callers, `bundleSweep` and its own selftest. Wiring it in is
T-055's cut.

### The defect that caused it, in one paragraph

`internal/nixx/build.go` composed a build command as a double-quoted assignment with a
COMMENT inside it, and the comment named a file in backticks: `` `.built` ``.
Backticks inside double quotes are command substitution, so the composing shell
ran `.built` and printed `pgb: 1: .built: not found` at the exact moment boost's
round 1 began. ⛔ **The message named neither the file it came from nor the
construct**, an hour went into diagnosing a boost build that was never failing,
and the fix was to delete two characters from a comment. ⚠ **No linter in this
tree would have caught it**, and `sh -n` accepts it, because it is valid shell
that means something nobody wanted. `docs/history/corrections.md` C16.

### The reading, in this order

1. ⭐ [`design/toolchain.md`](../docs/design/toolchain.md) "Language and
   structure" — **read it in full before writing any Go.** The ranked language
   comparison (Go 1, Rust 2, Nim 3, and why Zig is not in the top three), what
   "single binary" can and cannot mean, the package layout, and ⛔ **the six
   workload gates the port must pass**, which are not "does it compile".
   ⚠ It was written from a commissioned analysis of the tree at commit
   `2e4c6169`, which T-061 folded in and then deleted at the operator's
   request; the design page is now the only copy.
2. [`design/toolchain.md`](../docs/design/toolchain.md) — the CURRENT language
   decision (T-011), which this entry overturns. ⭐ Overturning it is the
   operator's call and it is recorded, not argued: the report is the argument.
3. `evidence/70-carried-helper/RESULT.txt` — the only compiled-helper evidence
   this tree has: static Rust, GNU **and** musl, 12 of 12. ⛔ **There is no Go
   row.** Step 2 of the migration sequence adds one, and it comes BEFORE the
   rewrite, not after.

### ⛔ What the operator required beyond "port it"

Each is a gate, not a preference. Quoted, then what it means here.

| # | required | what it means in this tree |
|---|---|---|
| 1 | *"pass all tests/experiments, reach current feature parity"* | every `experiments/NN-*.sh` and every `poc/NN-*/run.sh` passes against the Go `pgb`. ⭐ **They stay in shell** — the report §"Migration sequence" step 8: they are the independent acceptance harness, and rewriting the oracle with the thing it tests is how a port proves itself against itself |
| 2 | *"produce a single statically linked executable `pgb` we can distribute and devs don't need to clone the repo and run setup etc"* | `CGO_ENABLED=0`, everything embedded: `tool/runtime/*.c`, the wrapper templates, `rootfs-images.txt`, the nix fixtures. ⭐ **And `pgb` must then build itself with `pgb`** — a static-glibc toolchain whose own tool is not static is an argument against itself |
| 3 | *"the pgb builder looks like docker build … live logs with `ts` like timestamp, configurable"* | two references are vendored for it: `references/pkgforge__tss/main.rs` (the Rust `ts`) and `references/Azathothas__ToolKit/stamp.ps1`. ⚠ **The PowerShell one carries the part a naive port drops** — a HEARTBEAT when the stream is silent, so a four-minute link does not look like a hang |
| 4 | *"best in class debugger/verbose loggers"* | levels, per-subsystem selection, and ⛔ **the composed command printed before it runs**. The defect above was invisible for an hour because nothing printed what was about to be executed; `PGB_NIX_DEBUG_CMD` was added by hand mid-session to do exactly that. That must be a first-class facility, not an ad-hoc `printf` |
| 5 | *"all our crooked hacks must be written into safe, proper, ultra redundant functions"* | every construct whose failure mode is a plausible-looking wrong answer. The tree already lists them: `docs/history/corrections.md` is the inventory |
| 6 | *"our nix debloater should work today but also in the future if nix changes the tree/structure — do not rely on hardcoded values or rely on them as little as possible"* | ⛔ **the sharpest one, and the one with the most existing debt.** `internal/bundle/appimage.go`'s debloat rules, `store_resolve`, the baked-path table and the wrapper-env lifting all pattern-match nixpkgs' CURRENT layout. Each rule must state what it is looking for structurally (an ELF that nothing needs, a directory no environment variable names) and fall back to doing nothing rather than to guessing |

### ⛔ Do not lose these, they are this session's unrecorded measurements

Both were taken and neither is in an entry yet:

- ⭐ **488,934,276 bytes of the kdenlive AppDir's `lib/` (2,300 files, 39%) is
  unreachable** from the four programs or any plugin directory — the whole
  DT_NEEDED closure plus every `.so` under a plugin path. That is the route to
  T-055's bar (ours 395,294,317 B against the competitor's 191,900,604 B) and
  it is measured, not estimated. The sweep is a 90-line Python walk; ⚠ **it was
  not committed** and has to be rewritten. Do it in Go, in `internal/bundle`,
  as requirement 6 above says.
- ⭐ **onelf runs our payload.** `experiments/90-`'s arm O reported three
  failures that were all one defect in OUR harness: onelf dispatches on
  argv[0]'s basename and silently falls back to the package default, so a
  symlink named `melt-onelf` ran kdenlive, which needs a display. Fixed in
  `experiments/90-kdenlive-vs-enhanced.sh`.

  ⭐ **RE-RUN 2026-09-02b, and the row is real for the first time.** It took a
  second defect out of the same arm first: the staging step copied
  `shared/bin/*`, a glob never matches a leading dot, and a nixpkgs wrapper
  leaves the payload ELF beside itself as `.NAME-wrapped` — so the recipe,
  written from a readdir, named an entrypoint the packed directory did not
  contain. `../docs/history/corrections.md` C16. The machine also needed
  `musl-gcc` and the `x86_64-unknown-linux-musl` rust target, both installed.

  | | P — ours | E — Enhanced | O — onelf, OUR payload |
  |---|---|---|---|
  | size | 477,191,058 B | **191,900,604 B** | 595,859,196 B |
  | render | 3,559 ms | **1,323 ms** | 2,068 ms |
  | start cold | 181 ms | **52 ms** | 597 ms |
  | on the eleven | **11/11 clean** | 4/11 clean | — |

  ⛔ **The bar is still NOT met against E.** ⭐ **But arm O isolates the
  PACKER** — same payload, same 5,276 libraries, same zstd level — and there
  ours is smaller and starts 3.3x faster. onelf renders faster once running,
  which is a runtime difference on a payload both share.

### Prove

⛔ **Not "it builds".** The report's six workload gates, each with its output:

    1. nix-index    identical output from the real ~400 MB packages.json,
                    with wall time and peak RSS recorded against the Python
    2. nix-nar      identical hashes, extraction and SIGNATURE DECISIONS on
                    the fixtures and on real cache.nixos.org objects
    3. parity       pgb doctor / env info / nix plan / verify: same output,
                    same exit codes
    4. wrappers     compile and link classification parity, no per-invocation
                    regression that shows up in a real build
    5. the matrix   all 11 environments, all 9 POCs, every experiment
    6. the artefact `file`/`readelf` on the produced `pgb`: no PT_INTERP,
                    no DT_NEEDED, and its size recorded

`evidence/92-go-port/RESULT.txt`, plus a green `sh TODO/check.sh` and
`sh scripts/common/check-docs.sh`.

---

## T-062 — eight packages carry no selftest, and `internal/wrapper` is one

**Source** the code review of 2026-09-02, using codegraph against the suite
registry in `../cmd/pgb/commands.go`.
**Category** toolchain · **Priority** P1 · **Effort** M · **Status** open

**Problem.** `pgb selftest` registers ten suites and reports "123 cases pass",
which reads as coverage of the tool. Measured against the package list, the
suites reach seven of the seventeen packages:

    covered      ociimg  rootfs  elfx (via cmd/pgb)  zstd  nixx
                 bootstrapx  bundle
    NOT covered  ⛔ wrapper  ⛔ verifyx  ⛔ buildx  envx  cfg
                 logx  proc  fail

⛔ **`internal/wrapper` is the product.** `CompileFlags` and `LinkFlags` compose
the flag set that every `pgb build` injects, and nothing carried in the binary
asserts what they produce. The acceptance evidence for them is gate 4 — the
byte-identity comparison against the retired shell — which needs a build
environment, a network and half an hour, so it is not something a change can be
checked against while it is being made.

**Premise.** T-058 is the shape of the defect this leaves open: two concurrent
builds shared one wrapper directory, the flag sets silently overwrote each
other, and NEITHER BUILD REPORTED ANYTHING. It was found by an experiment that
had to be written for it. A carried assertion over `LinkFlags` for a given
option set would have failed the moment the flags stopped depending on the
options.

**Approach.** The offline-testable surface is already pure and needs no bed:

    CompileFlags / LinkFlags   a table of option sets against the flags each
                               must and must not contain -- --embed-terminfo,
                               --embed-cacert, --embed-locale, --no-iconv and
                               --wrap-dlopen are the axes T-058 named
    ParsePluginSpec            NAME=OBJECT[,OBJECT...], and what it refuses
    uniqueSorted, dlopenObjects, firstLine   pure helpers

⚠ **`verifyx` and `buildx` are a different problem** and this entry does not
pretend otherwise: both shell out to a bed. What can be carried is the parsing
and the decision logic, not the run — say which, rather than reporting a suite
that exercises neither.

**Prove.** `pgb selftest --list` names a `wrapper` suite; the case count rises;
and the suite fails when an option is disconnected from the flags it controls,
demonstrated by disconnecting one deliberately, the way `experiments/89-` uses
a control arm.

---

## T-066 — ⛔ P0: the bundler is bloated and slow. Rebuild it against a CLI benchmark

**Source** ⭐ **operator, 2026-09-02b**: *"pgb bundle isn't good enough, it is
bloated, slow and a complete failure. Restudy what all nixappimage related
references and fix/patch/reimplement/iterate+improve everything needed to fix
our bundles. best place is to bundle a cli first, bundle something like bash or
maybe 7z which can be benchmarked better, and smaller and less time to compare
after each iteration/improvements."*
**Category** toolchain · **Priority** P0 · **Effort** XL · **Status** open

⛔ **WORK UNTIL IT IS MET OR THE PREMISE IS SIGNIFICANTLY ADVANCED.**

**Problem, measured, and it is not close.**

| subject | ours | the field | |
|---|---|---|---|
| `jq` (`experiments/86-`) | **11,471,610 B**, 7 store paths | **4,006,916 B**, 68 libraries | ⛔ **2.86×** |
| kdenlive (`experiments/90-`) | **471,033,944 B** | 191,900,604 B | ⛔ **2.45×** |
| kdenlive render | 4,947 ms | 2,033 ms | ⛔ **2.43×** |
| kdenlive cold start | 300 ms | 61 ms | ⛔ **4.92×** |

⭐ **AND `aggressive` MOVES IT — run 6, same day.** With sweep deletion on,
`DropUnreachable` removed **1,712 objects, 227.4 MiB**, and the artefact came
out at **426,528,098 B = 2.22×**, against `safe`'s 2.45×. ⛔ **It still
rendered**, 4,149 bytes of MP4, byte-for-byte what `safe` produced — which is
the measurement runs 1 and 3 failed at, and the first evidence that the three
sweep fixes hold on a plugin-heavy subject.

⚠ **Run 6's render and startup MILLISECONDS are contaminated and are not
quoted here.** The competitor's fixed artefact moved 2,033 → 13,680 ms in the
same window, which is the control saying the machine was loaded rather than
the bundle slower. `RULES.md` §"the shared resource is sometimes the clock".

### ⛔ AppDir bytes are NOT artefact bytes, and the ratio is about 6 to 1

    safe        AppDir 2.53 GiB -> 2.16 GiB      artefact 471,033,944 B
    aggressive  AppDir 2.53 GiB -> 2.07 GiB      artefact 426,528,098 B
                then the sweep: -227.4 MiB       delta      44,505,846 B

    AppDir removed, extra   92.2 MiB (debloat) + 227.4 MiB (sweep) = 319.6 MiB
    artefact removed                                              =  42.4 MiB
    ⭐ ratio                                                       = 7.5 : 1

⭐ **Corroborated by a second route:** `aggressive`'s extra rules are the
Vulkan drivers `intel` 47.8, `radeon` 20.0, `nouveau` 21.4 and `virtio` 1.9
MiB — **91.1 MiB**, against the 92.2 MiB the two totals imply. The logs round
to two decimals, so the AppDir delta is 319.6 MiB ± ~20; the ratio is 7 to 8,
not a constant.

⭐ **So a debloat rule is worth about an EIGHTH of its raw size on the thing a
user downloads**, because dwarfs at `zstd:level=19` was already compressing
what got deleted. ⚠ **This reframes the lever this entry opened on**: "489 MB
of kdenlive's `lib/` is unreachable" is worth **~65 MB** of artefact, not 489.

⛔ **And it makes the arithmetic decisive.** Closing 426,528,098 → 191,900,604
means removing **223.8 MiB of artefact**, which at 7.5:1 is **~1.65 GiB of
AppDir** — out of the ~1.85 GiB that remains after the sweep. **You would have
to prove 89% of what is left is dead.** Deletion is not the route; where the
closure comes from is.

⚠ **A correction, and lens 3 of the review is what caught it.** This paragraph
first said "about a sixth" and "~1.4 GiB", computed from an AppDir delta of
250 MiB that omitted `aggressive`'s extra Vulkan rules. The conclusion did not
change; the denominator did.

⛔ **AND THE NUMBERS ABOVE LIVE IN A LOG `.gitignore` DISCARDS.** `debloat`,
the sweep total and `icd json N rewritten` are printed to
`evidence/*/build/build-ours.log`, which `.gitignore:15` excludes — so the next
run overwrites the only copy. `evidence/90-kdenlive-vs-enhanced/run6-build-summary.txt`
is that copy, kept deliberately; ⚠ **run 5's equivalent is already gone**, which
is why its `2.53 GiB -> 2.16 GiB` is cited from a transcript rather than from
the tree.

⛔ **THE kdenlive ROWS ARE RUN 5's (2026-09-02d, `safe`) AND THEY SUPERSEDE
WHAT WAS HERE.** The previous figures — 477,191,058 B, 3,559 ms, 181 ms — came
from before the five-run sequence in which runs 1 through 4 were each invalid
for a different reason, and no run before 5 both rendered and completed.
⭐ Run 5 is the first that did: `ours rendered on every environment = 11 of 11`.
⚠ Timings move with the machine, so the render and start rows are a same-day
comparison against the competitor and not a claim about last week's numbers.

⭐ The one column ours wins is host-object cleanliness — **11/11 against 4/11**,
and on `rockylinux-8` the competitor loads **10** host shared objects to our
zero — and T-065 is about whether that is even the right assertion for a
bundle.

**Why a CLI is the subject, and the operator named it.** kdenlive is a
20-minute build and a 477 MB artefact: an iteration loop nobody can run. `bash`
or `7z` is minutes and megabytes, benchmarks cleanly (startup dominated, no
display, a real workload in `7z b`), and every improvement shows up
immediately. ⛔ **Do not iterate on kdenlive.** Land the CLI numbers first,
then re-measure the big subjects once.

**Premise — the levers already measured and not pulled.**

1. ⛔ **The reachability sweep exists and NOTHING consumes it.** Confirmed with
   codegraph: `Sweep` has exactly two callers, `bundleSweep` and its own
   selftest. `--debloat` is pattern rules only. 489 MB of kdenlive's `lib/` is
   unreachable. **This is the single largest unused lever.**
   ⭐ **PULLED 2026-09-02c** — `DropUnreachable` consumes it — and then
   **gated to `aggressive`** on the same day, after three classes of
   runtime-loaded library turned out to be invisible to it.
   ⛔ **AND IT WAS QUADRATIC, which is a second finding inside the first.** The
   soname scan did one `bytes.Contains` per needle per object, so it re-read
   every byte of the bundle once for every library in it. On kdenlive that is
   ~1,000 objects against ~1,000 names over a 2 GiB tree.

   ⭐ **Measured while it ran, rather than argued from the shape of the loop:**
   `/proc/<pid>/io` showed `rchar` advancing **14 MiB per 5 s — 2.8 MiB/s** —
   at 101% CPU, sleeping on nothing. The bundler reads each object once and
   then scans that buffer a thousand times, so the *bundle* advances at
   2.8 MiB/s while the *scanner* runs at gigabytes per second. On this AppDir
   that is **~12 minutes for the sweep alone**, against ~8 minutes for the
   whole of the rest of the build — fetch, debloat, wrapper lifting and all.

   ⭐ Replaced with a single-pass scan that is **exactly** equivalent by
   construction — the splitting alphabet is derived from the needles
   themselves, so a needle occurrence cannot straddle a split, and only runs
   containing `.so` are kept because `IsSharedObject` requires it — with the
   original kept as `sonamesMentionedNaive`, the control its selftest compares
   against on fixtures built for the four ways the two could differ.

   ⭐ **MEASURED ON THE REAL kdenlive AppDir — 1,633 library files, 1.49 GiB,
   2,586 roots, 33 plugin directories:**

   ```
   pgb bundle sweep AppDir --env AppDir/.env --list
     naive   838 s      exit 0, 47 lines
     fast      7.07 s   exit 0, 47 lines
     diff    IDENTICAL
   ```

   ⭐ **118×, and the outputs are byte-for-byte identical on a real bundle** —
   which is a far stronger control than the fixture selftest, because it is
   1,633 real libraries and 2,586 real roots rather than five files written to
   have a known answer.

   ⚠ **The ratio is approximate in one direction and it is worth saying which.**
   The naive arm ran with other work on the box (a `go build`, two `pgb
   selftest`s, the gates), so 838 s is if anything generous to the fast path;
   the fast arm was measured on an idle machine. An earlier reading during run
   6's own build put the sweep at ~12 minutes, also under load. ⛔ So "about
   100×" is the honest claim and 118× is the arithmetic.
2. `store/` is 405 MB of the kdenlive bundle and duplicates what is already in
   `lib/`.
3. `share/` is 368 MB, most of it one icon theme shipping every size.
4. Start and render both track artefact size — the dwarfs image is mounted at
   launch — so 1–3 move all three columns at once.

### ⭐ THE COMPETITOR'S WHOLE PIPELINE IS 89 LINES, AND IT ANSWERS THE ENTRY

Read 2026-09-02d out of `references/pkgforge-dev__kdenlive-AppImage-Enhanced/tree/`
— `get-dependencies.sh` (37 lines) and `make-appimage.sh` (52). ⛔ **The gap is
not a debloat rule we have not written. It is the direction the two pipelines
run in.**

| | ours | theirs |
|---|---|---|
| starting set | nixpkgs' **complete closure** — every path every derivation declared, **2.53 GiB** | `pacman -Syu` of a **hand-picked list of 12 packages** |
| then | **subtract**: delete what can be *proved* unreachable | **add**: `quick-sharun` walks the DT_NEEDED closure of ~20 named paths |
| the heavy packages | whatever nixpkgs built | ⭐ `get-debloated-pkgs --add-common` — size-optimised **rebuilds**, which is `pkgforge-dev/archlinux-pkgs-debloated`, the corpus this entry already names |
| what must not come | nothing; the sweep has to *discover* it | ⭐ `pacman -Rsndd --noconfirm qt6-webengine` — an explicit **removal**, one line |
| subsystems | inferred — our soname-string rule scans every ELF for `libSDL3.so.0` at 2.8 MiB/s | ⭐ **declared**: `DEPLOY_OPENGL=1 DEPLOY_SDL=1 DEPLOY_PIPEWIRE=1` |

⛔ **SUBTRACTIVE CANNOT WIN AGAINST ADDITIVE HERE, AND IT IS STRUCTURAL RATHER
THAN A MATTER OF EFFORT.** The sweep is deliberately conservative — sweep.go's
own rule is *"anything a rule cannot classify counts as REACHABLE"* — so
everything it cannot **prove** unnecessary stays. An allowlist keeps only what
was **named**. Starting from a superset and deleting provable dead weight can
approach the allowlist's result only if the proof is complete, and it is not:
three classes of runtime-loaded library were invisible to it in a single
afternoon.

⭐ **And the arithmetic already said so.** `aggressive` deletes 250 MiB of
AppDir for 42.4 MiB of artefact — about **6 to 1**, because dwarfs was already
compressing what got deleted. Closing 426 MB → 192 MB by deletion alone would
need roughly **1.65 GiB** more of provably-dead AppDir, out of the ~1.85 GiB
that remains after the sweep — **89% of what is left**. There is not that much
left to prove.

⭐ **And the corpus names exactly the packages that dominate OUR bundle.**
`Anylinux-AppImages/HOW-TO-MAKE-THESE.md`: *"Installs a debloated MESA, Vulkan,
Qt, GTK, libicudata, and more"*, with `--prefer-nano`, `ffmpeg-mini` and
`intel-media-driver-mini`. ⚠ Compare what `experiments/85-` measured on our
side — the GL stack alone is **95 MiB of a 163 MB bundle** — and what run 6's
debloat log lists: mesa's Vulkan drivers, `libteflon`, the locale catalogues.
**The overlap is not partial; it is the same list.**

⚠ **What this does NOT say.** It is a reading of somebody else's build script,
not a measurement of ours-rebuilt-additively; the measurement backing it is the
6:1 ratio and the 2.22× gap, both from run 6. ⭐ **The route it indicates** —
`pgb bundle appimage` taking an allowlist of paths rather than a closure, and
sourcing heavy packages from a debloated corpus — is the next thing to build
and it is not yet built.

### ⭐ MINED 2026-09-02e — and an allowlist is NOT enough, which the corpus says in its own recipes

`references/pkgforge-dev__archlinux-pkgs-debloated`, commit
`f29738934d003731a37bb1ca191030927fd3fa1b`, route proxy, 24 recipes.
⛔ **Reading them changes this entry's named lever, because a "debloated
package" is not a package with files deleted. It is a package REBUILT with
different build options**, and what that removes is a `DT_NEEDED` **edge**:

| recipe | what it actually does | the edge it removes |
|---|---|---|
| `qt6-base-mini.sh` | inserts `-DFEATURE_icu=OFF`, `-DCMAKE_BUILD_TYPE=MinSizeRel`, `-O2`→`-Os` | ⭐ `libQt6Core.so` → `libicuuc/libicudata`, ~30 MiB |
| `mesa-mini.sh` | deletes `llvm-libs` from `depends`, `-D amd-use-llvm=false -D draw-use-llvm=false` | ⭐ `libgallium.so` → `libLLVM.so`, **150+ MiB** |
| `ffmpeg-mini.sh` | deletes `--enable-libsvtav1`, `--enable-vapoursynth`→`--enable-small` | `libavcodec` → `libx265.so`, ~20 MiB |
| `icu-mini.sh` | rebuilds `libicudata.so` | 30 MiB → **<3 MiB** |
| `opus-mini.sh` | Arch's own options | 5 MiB → **<500 KiB** |
| `libxml2-mini`, `gdk-pixbuf2-mini`, `librsvg-mini`, `glycin-mini` | drop icu / drop glycin | ~20 MiB of glycin |

And the set kdenlive gets is not a guess — `get-debloated-pkgs.sh:210`,
reached by `get-dependencies.sh`'s one call to `--add-common`:

    icu-mini  opus-mini  libxml2-mini  qt6-base-mini  gtk3-mini  gtk4-mini
    glycin-mini   + mesa-mini (ADD_MESA)  + vulkan-intel-mini  (+ intel-media-driver-mini)

⛔ **SO AN ALLOWLIST CANNOT REACH THIS, AND THAT IS THE CORRECTION.** The lever
this entry named is *"take an allowlist of paths rather than a closure"*. That
is necessary and it is still worth building — but it is **bounded**, and the
bound is not effort. An allowlist chooses which *paths* to carry; it cannot
remove a dependency a library **declares**. A perfect allowlist naming only
kdenlive's true dependencies still carries `libicudata.so`, because the
`libQt6Core.so` in the closure has a `DT_NEEDED` on it and deleting it breaks
the binary — which is exactly the assertion `b.integrity()` already makes
("every DT_NEEDED in the bundle resolves inside it"). Only a **rebuild**
removes the edge.

### ⛔ AND WHY THEIR SWAP IS CHEAP AND OURS IS NOT — the packaging models differ

⭐ **This is the structural reason, and it is about content addressing rather
than about bundlers.** Arch swaps `qt6-base` for `qt6-base-mini` and **nothing
downstream rebuilds**: the soname `libQt6Core.so.6` is unchanged, so kdenlive's
existing binary keeps resolving against it. One rebuild of one package, done
once in their CI and published.

Ours cannot do that. `internal/bundle/appimage.go:154` fetches the closure by
**exact store path** — `b.Nix.Fetch("/nix/store/"+b.Base, WithClosure: true)`
— and a nixpkgs store path is the hash of the derivation's inputs. Changing
qtbase's build options changes qtbase's hash, which changes the hash of
everything that depends on it. ⛔ **So a `qt6-base-mini` equivalent invalidates
the binary cache for kdenlive's entire KDE/Qt subtree, and every one of those
paths would have to be built from source** — the thing `pgb nix` exists to
avoid, and the reason `experiments/88-` is a *fetch* story.

⚠ **`pgb nix build` CAN already express the option change** — `nixArgs.Configure`
→ `Builder.ConfigureExtra` → `internal/nixx/tree.go:99` — so the mechanism is
present. What is absent is a costing of the rebuild it forces.

⭐ **Three routes, and the entry now carries the argument rather than one name:**

| | route | what it costs, and what would settle it |
|---|---|---|
| **A** | the **allowlist**, as already named | still worth building; bounded by the edges above. ⛔ Measure the bound: sum the sizes of the closure paths that are reachable ONLY through an edge a `-mini` rebuild would delete. That number is the ceiling an allowlist cannot pass, and it is cheap — it needs the AppDir and `pgb bundle sweep`, no rebuild |
| **B** | build our own `-mini` derivations through `pgb nix build --configure` | ⚠ forces a from-source build of every dependent path. Cost unknown; the first measurement is how many store paths kdenlive's closure has downstream of `qtbase` and `mesa` |
| **C** | splice a smaller library into the fetched closure post hoc | ⛔ breaks the closure's own guarantee — the NarHash no longer matches what was signed — and `b.integrity()` would have to be re-satisfied by hand. Cheapest to try, weakest to defend |

⛔ **Nothing here is measured yet on our own bundle**, and it must not be
written up as though it were. ⚠ **The AppDir this would be measured against —
`/var/tmp/pgb-appimage-kden`, 7 GB — did not survive the container**, so route
A's ceiling is a rebuild away and is the first thing to run when a bundle
exists again. ⭐ The claim carried here is only what the corpus's own recipes
say, at file and line, plus the store-path property read out of
`appimage.go:154`.

**Approach.** Restudy the family first, then iterate against the CLI:
`pkgforge__nix-appimage`, `ralismark__nix-appimage`, `of-the-stars__nix-appimage`,
`logos-co__nix-bundle-appimage`, `VHSgunzo__sharun`, `VHSgunzo__runimage`,
`pkgforge-dev__Anylinux-sharun`, `nix-community__patsh`, `leleliu008__elftool`.
⭐ **Iterate, patch and reimplement — the brief says reuse and improve before
reinventing.** Each change lands with the CLI numbers before and after.

**Prove.** ⛔ Not "it is smaller". A table with a row per iteration for the CLI
subject — bytes, cold start, warm start, and the workload's own time — showing
what each change bought; then the same three columns re-measured for `jq` and
for kdenlive. ⭐ **The bar is the field**: `experiments/86-`'s hand-built
Anylinux arm for the CLI, and `kdenlive-AppImage-Enhanced` for the big one.
`docs/AGENTS.md` §14 forbids "better" without the measurement.

## ⚠ Significantly advanced, and STILL OPEN — `experiments/78-`

⭐ **2.86× the field → 1.22×, on `jq`.** ⛔ Not parity, so this entry stays
open with the remaining lever named.

`evidence/78-bundle-cli-bench/RESULT.txt`. ⭐ **The subject is a CLI and that is
what made it possible**: a `jq` bundle builds in about a minute, so this was
four measured iterations in the time one kdenlive build takes.

    ARM          BYTES     COLD_MS  WARM_MS  WORKLOAD_MS  OUTPUT
    none      12261750         126        9           13  6
    safe       4890913          60       10           13  6
    aggressive 4890913          68       11           14  6

    field (Anylinux-AppImages, experiments/86-)   4,006,916 B
    ours, was                                    11,471,610 B   2.86×
    ours, now                                     4,890,913 B   1.22×

⛔ **A correctness column, because a smaller bundle that answers differently is
not a smaller bundle.** All three arms run the same job and their output is
compared byte for byte; that is the experiment's only assertion, because a
wall-clock figure from one machine on one day is not a threshold anything
should fail on.

**Two levers, both structural rather than a list of names.**

| | |
|---|---|
| ⭐ **the reachability sweep, which NOTHING consumed** | `sweep.go` computes the DT_NEEDED closure of every program plus every plugin directory, and had two callers: the `pgb bundle sweep` subcommand, which only prints, and its own selftest. The build path never called it, so every debloat rule was a rule about NAMES and the one structural answer in the tree was shown to a human and thrown away. ⭐ `codegraph callers Sweep` is the one command that shows it — `RULES.md`'s own example of what codegraph is for. **On `jq`: 277 objects, 12.0 MiB.** Safe to delete on because the sweep counts anything it cannot classify as REACHABLE, and `b.integrity()` re-checks every DT_NEEDED afterwards |
| ⭐ **`share/i18n` is 17 MiB of a 22 MiB `jq` bundle** | glibc's locale **SOURCE** data — the text `localedef` compiles FROM, not what a program reads. `cns11643_stroke` alone is 4.31 MiB in a bundle whose entire `lib/` is 4.8 MiB. The rule is **conditional**: a bundle shipping `localedef`, `locale` or `iconvconfig` keeps them. **On `jq`: 15.0 MiB** |

Debloat went from **12.7% off to 86.9% off** on the same closure.

⛔ **What is left, and it is why this stays open.**

1. ⛔ **kdenlive RAN AND FAILED, and the failure was mine.** `experiments/90-`,
   `pass=6 fail=2`:

       ARTEFACT                          BYTES
       P  ours (one command, nixpkgs)  267390365
       E  kdenlive-AppImage-Enhanced   191900604
          ratio P/E                        1.39x     (was 2.49x)

       render:  ours 0 bytes of MP4        enhanced 4162 bytes
       on 11:   ours rendered 0 of 11      enhanced 11 of 11
       clean:   ours 11 of 11              enhanced 4 of 11

   ⭐ The size moved the right way — 477,191,058 → 267,390,365 B, **2.49× →
   1.39×** — and ours is still the only arm that loads no host object on every
   row. ⛔ **But it could not render**, because the sweep ran before `.env`
   existed and deleted the MLT modules. `docs/history/corrections.md` C20; the
   ordering is fixed in `5fbf7ad0` and **the re-measurement has not been made**.
   ⚠ Until it is, the 1.39× is a size for a bundle that did not work, and it
   must not be quoted as a result.
2. ⚠ **The remaining 1.22× is a PACKAGE-SIZE gap, not a bundler one**, and
   `Anylinux-AppImages/FAQ.md` names it: their libraries come from packages
   optimised for size, ours from nixpkgs, and their own example is a
   `libicudata.so` that is *"less than 1 MiB"* in one and *"30 MiB"* in the
   other. ⭐ The next lever is therefore **where the closure comes from**
   (`pkgforge-dev/archlinux-pkgs-debloated` is the named corpus), not another
   debloat rule.
3. `--debloat aggressive` now buys **nothing** over `safe` on `jq` —
   4,890,913 B both. The sweep made the aggressive name-rules redundant for a
   CLI; whether that holds for a GL application is unmeasured.

---

## T-067 — ⛔ P0: does zig buy anything the C runtime pieces cannot?

**Source** ⭐ **operator, 2026-09-02b**: *"Look into using zig if existing c is
limited/slow, thought that shouldn't be the case"*.
**Category** toolchain · **Priority** P0 · **Effort** M · **Status** done

⛔ **WORK UNTIL IT IS MET OR THE PREMISE IS SIGNIFICANTLY ADVANCED**, and note
the operator's own expectation: *"that shouldn't be the case"*. ⭐ **A measured
"C is fine, here is the evidence" closes this entry.** It is a question, not a
migration.

**Problem.** `tool/runtime/` is C — `pgb-nssfix.c`, `pgb-cacert.c`,
`pgb-terminfo.c`, `pgb-trace.c` and the iconv wrappers — compiled into every
artefact. If C is limiting or slow anywhere, that cost is paid by every binary
pgb produces.

**Premise.** ⭐ There is prior art **in this tree's own corpus**:
`references/allyourcodebase__pipewire/src/wrap/dlfcn.zig` exports
`__wrap_dlopen`, `__wrap_dlsym` and `__wrap_dlclose` against a compiled-in
table — the same mechanism as `--wrap-dlopen`, written in zig. So the question
is not hypothetical and there is a working comparison to read.

⚠ **And there is a real constraint the answer must respect**: `pgb` is one
static Go binary built `CGO_ENABLED=0` that **carries its C sources and
compiles them with the target's toolchain**. Anything zig replaces must still
be compilable inside the pinned build environment with no new host dependency —
⛔ or it fails the same way the libiconv/`msgfmt` defect did, by needing a tool
the environment does not have.

**Approach.** Name a specific place C is actually limiting before proposing a
language: measure the runtime pieces, read the zig prior art, and answer three
questions with evidence — is any runtime piece measurably slow; is any of it
hard to write correctly in C (T-064's loader is the honest candidate); and what
would adding a zig toolchain cost the build environment and the artefact.

**Prove.** A written comparison in `docs/design/`, with a number behind each
claim, ending in a ruling: adopt zig for a named component, or record that C is
adequate and why — so this is not re-asked. ⛔ A migration with no measured
limitation behind it is refused by this entry, not enabled by it.

## ✅ Done — [`../docs/design/runtime-language.md`](../docs/design/runtime-language.md)

⭐ **Ruling: C is adequate. Do not migrate.** The operator's expectation was
right; the work was measuring it, and finding what would change the answer.

⭐ **The subject was the best one available**: `pgb-elfload.c`, the compiled-in
ELF loader T-064 built this session — 1,578 lines of raw pointer arithmetic,
`mmap`, relocation and TLS, over half of all the C in `tool/runtime/`.

**1. Is any of it measurably slow? No, and mostly there is nothing to be slow.**
Six of nine files are one-shot constructors or `--wrap` shims. The whole
runtime's cost against plain `gcc -static` is at the noise floor —
`evidence/40-overhead`, where two runs put per-exec 42 µs then 28 µs above and
peak RSS 56 KiB above then **28 KiB below**, a sign change. The loader is the
only piece doing real work and it is **4.8× faster** than the host `ld.so`
reached from the same static binary (147,543 ns against 711,066).

**2. Is any of it hard to write correctly in C?** ⭐ **The instruments say no,
and the defect log says something stronger.**

| instrument | scope | result |
|---|---|---|
| `gcc -O2 -Wall -Wextra -fanalyzer` | all 9 files, 3,041 lines | **5 warnings, 0 errors** |
| ⭐ **UBSan**, running the loader over **904 real host shared objects** | `pgb-elfload.c` | ⭐ **0 runtime errors** |
| ASan | the same | ⛔ **could not run** — SEGV in its own `SetTLSFakeStack` before any of our code. Recorded as could-not-run, never as a pass |

⚠ **All five gcc warnings are one false positive**: `-Waddress` does not model
**weak** linkage, so it reports that `&pgb_provider_syms[0] == NULL` is always
false when a weak symbol's address is exactly what can be NULL — the mechanism
the whole provider table rests on. C's own analysis is wrong about this
runtime's most load-bearing construct, in the noisy direction.

⭐ **And the finding that actually decides it.** Five real defects were found
building the loader, every one by something disagreeing. **Not one is a
C-language defect** and a memory-safe language would have prevented none:
`libm.a` being a GNU ld script (the filesystem), `__tls_get_addr` being in no
archive (the ELF ABI), `DT_RELR` unhandled (a missing feature), `make` not
depending on the go:embed'd C (a Makefile bug), and a benchmark that forked per
sample (an instrument bug).

**3. What would zig cost?** ⛔ **The constraint this entry named is the one that
decides it.**

| | measured |
|---|---|
| zig in the pinned `debian:12` | ⛔ **not packaged** — `apt-cache policy zig` returns nothing |
| so it must be fetched | zig 0.15.2 x86_64-linux, **53,733,924 B**, from `ziglang.org/download/index.json` |
| against | `pgb` itself at 11,765,820 B, which would be smaller than its own dependency |

⚠ And it would be a **second** toolchain, not a replacement: the application is
still compiled by the target's `cc` through `pgb`'s wrappers. Two toolchains
feeding one link is a new class of problem for a benefit nothing above named.

⛔ **"Do not migrate" is not "never re-ask".** Four named conditions reopen it,
in `runtime-language.md`: a defect that IS a C-language defect; a runtime piece
measurably slow with its number; zig arriving in the pinned image; or a
component that cannot be written correctly in C — the loader was the candidate
for that last one and came out at 1,093 code lines against `pg83/solo`'s 2,332
for the same job in C++.

## T-070 — ⛔ the glibc pin is a FLOOR set to 2.36, and the ceiling moves every year

**Source** ⭐ **operator, 2026-09-02c**: *"focus on solving glibc's remaining
quirks, ensure future version won't break our tooling or binary built by your
tooling"*, and the question *"why do we compile on an older distro, isn't glibc
backwards compatible?"*
**Category** toolchain · **Priority** P0 · **Effort** M · **Status** open

⭐ **The question is answered in
[`../docs/design/glibc-versions.md`](../docs/design/glibc-versions.md) and the
answer produced this entry.** glibc's backward compatibility is real and is
irrelevant to our output, because the output is static:
`PT_INTERP=0 DT_NEEDED=0`, no versioned imports, no host glibc consulted. ⛔ So
there is **no** "build old to run on old hosts" pressure here at all.

**The pin exists for a FLOOR.** glibc 2.34 made `files` and `dns` NSS builtin;
below it `__nss_configure_lookup` only MOVES the dlopen. `experiments/21-`
measures it: a 2.31 build **with** nssfix still opens `libnss_dns.so.2` and
`libnss_files.so.2`; a 2.36 build opens none.

⛔ **And a CEILING points the other way, and unlike anything else in this
project it gets worse with time.** `--host-dlopen` needs a HOST object's
imports satisfiable by OUR glibc, and `experiments/73-`'s class B is where that
fails: 20 symbols, **14 of them `__isoc23_*` at exactly `GLIBC_2.38`**, plus
`strlcpy`/`strlcat` at 2.38. Every glibc release the pin does not follow widens
it.

⭐ **The pin is 2.36. The floor is 2.34. Nothing forces it to sit near the
floor** — that is the finding. A pin at ≥ 2.38 closes the majority of class B
by construction.

**⛔ Approach: measure the cost BEFORE moving it, because a static binary's
only host requirement is the kernel and that is the thing a newer glibc can
take away.**

| | must be measured |
|---|---|
| 1 | the **kernel floor** a newer glibc's static binaries declare. Today `file` says `for GNU/Linux 3.2.0`. ⛔ If a newer pin raises it, that trades a real portability property for a symbol-coverage one and the trade has to be stated, not discovered |
| 2 | `experiments/21-` re-run against the new pin — the NSS floor must still hold |
| 3 | all ten POCs still build. A newer glibc deprecates as well as adds |
| 4 | `experiments/73-` re-run: class B is what the move buys, and **class C — empty today — is what it could cost** |
| 5 | the `debian:13` (glibc 2.41) manifest digest, pinned as `debian:12` is |

**Prove.** A row per candidate pin — glibc version, kernel floor, class B
residue, class C residue, POCs building, `21-` verdict — and a ruling: move the
pin, or record why the floor-adjacent pin is right after all. ⛔ Not "it built".

### ⭐ MEASURED 2026-09-02d — `experiments/91-glibc-pin-candidates.sh`

⛔ **The veto clears, and it is the half that could have ended this.** A static
binary's only host requirement is the kernel, so the question was whether a
newer glibc's crt files declare a higher one. They do not:

| image | glibc | gcc | `.note.ABI-tag` | `file(1)` |
|---|---|---|---|---|
| `debian:12` (incumbent) | 2.36 | 12.2.0 | **3.2.0** | `for GNU/Linux 3.2.0` |
| `debian:trixie` | **2.41** | 14.2.0 | **3.2.0** | `for GNU/Linux 3.2.0` |
| `ubuntu:24.04` | — | — | ⚠ **could not run** | registry answered `429` |

⭐ **Two instruments that could have disagreed, and did not.** `readelf -n` reads
the note glibc encoded; `file` prints its own interpretation of the same bytes.

**And the ceiling collapses.** `experiments/73-` run once per pin, same day,
same eleven environments, `PGB_ENV_NAME` the only variable:

| environment | class B @ 2.36 | class B @ 2.41 | symbols served |
|---|---|---|---|
| debian-11 | 0 | 0 | 905 → 905 |
| debian-12 | 0 | 0 | 851 → **849** |
| ubuntu-20.04 | 0 | 0 | 893 → 893 |
| rockylinux-8 | 0 | 0 | 1049 → 1049 |
| opensuse-leap-15.6 | **13** | **0** | 993 → **1005** |
| fedora-42 | **15** | **0** | 961 → **976** |
| archlinux-latest | **20** | **5** | 1198 → **1213** |

    class B, distinct symbols   20 at 2.36  ->  5 at 2.41
    class C (the pin REMOVED it) empty on all 11 rows at BOTH pins

⭐ **The whole `__isoc23_*` family at `GLIBC_2.38` is gone**, and the five that
remain are at `GLIBC_2.42`/`2.43` on `archlinux-latest` alone —
`__memset_explicit_chk`, `free_sized`, `free_aligned_sized`, `__inet_pton_chk`,
`__inet_ntop_chk`. ⚠ **Which is the entry's own point restated: a rolling
distribution is always ahead of any pin.** Moving to 2.41 does not end class B;
it empties it for every fixed-release environment measured and leaves the
rolling one, which is the residue that regrows.

⚠ **THE ONE COST, REPORTED RATHER THAN ROUNDED OFF.** `debian-12` serves **two
fewer** symbols at 2.41 — 851 → 849, class A 5→6 and class S 41→42. Net across
the seven glibc rows is **+40 served, −2**. ⛔ Class C being empty is the
stronger statement: nothing a host object wants was *removed* by the newer
glibc, on any row.

**⭐ And the NSS floor holds at 2.41 — the arm that could still have vetoed.**

The `experiments/21-` probe, built against 2.41 in `pgb-env-debian-trixie`, run
on the `debian-11` target that really ships `libnss_files.so.2`:

| build glibc / arm | host NSS modules opened |
|---|---|
| 2.31 plain | `libnss_dns.so.2`, `libnss_files.so.2` |
| 2.31 + nssfix | `libnss_dns.so.2`, `libnss_files.so.2` |
| 2.36 plain | none |
| 2.36 + nssfix | none |
| **2.41 plain** | **none** |
| **2.41 + nssfix** | **none** |

⛔ **THE 2.31 ROWS ARE THE CONTROL AND THEY ARE WHY THE 2.41 ROWS MEAN
ANYTHING.** Both 2.41 arms print `none`, and so does 2.36 — because at or above
2.34 the services are inside libc and there is nothing to open with or without
the override. ⚠ **A "none" from an instrument that cannot see modules would look
exactly the same**, which is not hypothetical here: a first attempt at this
measurement had an unquoted shell variable, read a trace file that did not
exist, and printed `none` for every arm. `experiments/21-` supplies the arm that
can fail, on the same target and the same method, and it fires.

### ⛔ ARM 5, 2026-09-02e — and the first thing it found was OUR bug, not glibc's

⚠ **The recipe this entry carried did not measure the candidate at all.**
`PGB_ENV_NAME=pgb-env-debian-trixie sh poc/<name>/run.sh` builds against the
**incumbent** on a machine running dockerd, and nothing in a POC's output says
which glibc it used. Caught by reading `.comment` out of the binary the POC had
just produced: `GCC: (Debian 12.2.0-14+deb12u1)` where the candidate carries
14.2.0. Commit 333cb92f; arm 5 now asserts the `.comment` of every POC binary
against the environment's own recorded gcc, so the arm cannot pass for the
wrong environment again.

⭐ **Then, with the environment actually selected, gcc 14.2.0 / glibc 2.41
rejected no source at all — 8 of 10, run 1:**

    POC                    OUTCOME              GCC IN .comment
    10-gawk                ok   pass=12 fail=0  14.2.0
    20-nano                ok   pass=12 fail=0  14.2.0
    30-curl                ok   pass=12 fail=0  14.2.0   (OpenSSL + zlib)
    40-jq                  ok   pass=12 fail=0  14.2.0   (oniguruma)
    50-python              ok   pass=12 fail=0  14.2.0   (CPython, 49 modules)
    60-leveldb             ok   pass=12 fail=0  14.2.0   (C++, CMake)
    90-qt                  ok   pass=20 fail=0  unmeasured  (static Qt 6.11.1)
    91-qt-xcb              ok   pass=27 fail=0  unmeasured  (a real X window)
    70-sqlite-extensions   ⛔ exit 1             -- see below
    80-mlt                 ⛔ exit 1             -- the same cause

⚠ **`unmeasured` is not a weaker pass, and it is not a failure.** Both Qt POCs
leave no executable in their evidence directory — checked recursively, zero —
so there is no `.comment` to read. Their 20 and 27 assertions across eleven
environments are the measurement; the compiler column simply has no source for
them. ⛔ The first version of this check called that a mismatch and failed two
POCs that had passed everything.

⛔ **AND THE TWO FAILURES ARE `pgb`'s, NOT the pin's.** Both died at the LINK
with five undefined references — `pgb_elf_dlopen`, `pgb_elf_dlsym`,
`pgb_elf_dlclose`, `pgb_elf_dlerror`, `pgb_elf_available`:

    ld: .../pgb-dlopen.o: in function `pgb_elf_open':
        pgb-dlopen.c: undefined reference to `pgb_elf_dlopen'

`--wrap-dlopen` links `pgb-dlopen.o`, which falls through to the ELF loader;
`pgb-elfload.o` is compiled and linked **only** under `--host-dlopen`.
`pgb_elf_available()` exists so the first can be built without the second — its
own comment says *"so that a build without the loader keeps its existing honest
error"* — but a **strong** undefined reference fails the link before that check
can run.

⭐ **It looked like it worked for as long as the two were built in that order.**
The runtime objects are cached in a directory keyed on the COMPILER's identity,
so an earlier `--host-dlopen` build left `pgb-elfload.o` there and every later
`--wrap-dlopen` build linked it by name. **Change compiler — which is exactly
what moving the pin does — and the directory is new and empty.** Verified by
looking: the trixie runtime directory holds `pgb-dlopen.o` and no
`pgb-elfload.o` at all.

⚠ **This is the second time today a cached artefact made a build succeed for a
reason nobody intended**, and it is the same shape as PROGRESS.md finding 1.
⛔ **It also nearly produced the wrong ruling**: read at face value, arm 5 said
"the pin move breaks 2 of 10 POCs", which would have blocked a move whose four
measured costs are all zero.

**Fixed** by making the loader's five entry points weak (`pgb-elfload.h`) and
testing the address before the call (`pgb_elf_linked()` in `pgb-dlopen.c`).
Proved in a FRESH runtime directory, which is the condition that failed:

    --wrap-dlopen alone, empty runtime dir   app links, plug_answer=42, exit 0
    pgb-elfload.o in that directory          absent -- it linked WITHOUT it
    --host-dlopen, rebuilt after the fix     still loads, still refuses at 0

⭐ **AND THE CLASS WAS SWEPT RATHER THAN THE INSTANCE PATCHED.** The shape is
*"a runtime source references a symbol another runtime source defines, when a
DIFFERENT option decides whether that one is compiled"*. Every `tool/runtime/*.c`
was checked for cross-file `pgb_` references, and there is **exactly one such
pair in the tree** — `pgb-dlopen.c` → `pgb-elfload.c`, the five names above.
Confirmed with `nm` rather than grep, which is what says whether a reference is
weak or strong:

    nm pgb-dlopen.o -> pgb_elf_available  w   (was U)
                      pgb_elf_dlopen      w
                      pgb_elf_dlsym       w
                      pgb_elf_dlclose     w
                      pgb_elf_dlerror     w

⚠ `pgb_dlopen_libs` and the two provider-table symbols were already weak; they
are the pattern this fix follows rather than a new one.

### ⭐ RUN 2 — the last two, and the arm is COMPLETE at 10 of 10

Re-run against the link fix, same environment and same digest:

    70-sqlite-extensions   ok   pass=20 fail=0
    80-mlt                 ok   pass=21 fail=0

⚠ Both report `unmeasured` in the gcc column because neither keeps a binary in
its evidence directory. The compiler was then read from the binaries they
actually produced (`evidence/91-*/run2-comment-readings.txt`):

    70-sqlite/sqlite3-wrapped   gcc 14.2.0     80-kdenlive/melt-static      gcc 14.2.0
    70-sqlite/sqlite3-control   gcc 14.2.0     80-kdenlive/inst/bin/ffmpeg  gcc 14.2.0

⭐ **So 8 of the 10 are verified by `.comment`**; the two Qt POCs retain no
binary and are carried on their own 20 and 27 assertions instead. ⛔ That is
stated rather than rounded up to "all ten verified".

## ⭐ THE RULING — all four costs are measured, and every one is zero

| what the move could have cost | measured | verdict |
|---|---|---|
| the kernel floor a static binary declares | `.note.ABI-tag` **3.2.0** at both pins, two instruments agreeing | no cost |
| class C — a symbol the newer glibc REMOVED | **empty on all 11 rows at BOTH pins** | no cost |
| the NSS floor the whole project rests on | `none` at 2.41, with `experiments/21-`'s 2.31 arm firing as the control | holds |
| ten real projects under gcc 12.2.0 → **14.2.0** | ⭐ **10 of 10 build and pass their full matrices** | no cost |

⭐ **And what it buys:** class B — a host symbol newer than the pin — goes
**20 → 5 distinct symbols**, and the five that remain are at `GLIBC_2.42`/`2.43`
on `archlinux-latest` alone. Every fixed-release environment measured empties.

⚠ **The one measured cost, reported rather than rounded off:** `debian-12`
serves **two fewer** symbols at 2.41, 851 → 849. Net across the seven glibc
rows is **+40 served, −2**.

⛔ **THE RULING IS: MOVE THE PIN.** Nothing measured argues against it and one
thing argues for it.

### ⛔ AND THE PIN IS NOT ONE CONSTANT, IT IS NINE — which is why cfg.go is still untouched

⭐ **Found while costing the move, and it is the same defect this entry already
paid for once.** `cfg.go` holds `DefaultEnvImage`, `DefaultEnvDigest` and
`DefaultEnvName` — but **eight shell files hardcode the environment NAME as
their own fallback**, and they would not follow it:

    experiments/60-  61-  62-  73-     ENV_ROOT="$ROOTFS_DIR/${PGB_ENV_NAME:-pgb-env-debian12}"
    experiments/70-  80-  87-  88-     the path written out literally

⛔ **Change `cfg.go` alone and those eight keep looking for
`pgb-env-debian12`.** On a machine where that directory is gone they skip — an
exit 2 nobody reads as a regression. ⚠ On a machine where it is still on disk,
which is every machine that ever built it, **they measure 2.36 while the tool
builds 2.41 and say nothing** — which is precisely the failure this same entry
hit today with `PGB_ENV_NAME`, wearing different clothes.

⭐ **So the move lands in this order, and not otherwise:**

1. one source of truth for the default environment name — the eight read it out
   of `cfg.go`, as `experiments/91-` already does with `sed`, rather than each
   carrying its own copy;
2. then `cfg.go`;
3. then re-run the matrices, because ⛔ **every committed `RESULT.txt` in
   `evidence/` says `pinned build glibc : 2.36`** and would describe an
   environment the tool no longer builds.

⛔ **Step 1 requires editing `experiments/lib.sh`, which `experiments/85-` is
sourcing right now**, and this tree's rule is that a running shell script is
not edited. The ruling is recorded; the edit is the next thing to land.

---

## T-072 — the static TLS headroom is ~3,168 bytes and one real library wants 56,248

**Source** the residue of `experiments/76-` and T-068.
**Category** toolchain · **Priority** P1 · **Effort** M · **Status** open

⛔ **A glibc quirk with a named tunable, which is why it is its own entry.**
`pgb-elfload.c` places initial-exec TLS in the surplus glibc already reserves,
and the reserve is small: measured on the build host,
`_dl_tls_static_size = 3264`, `_dl_tls_static_used = 88`, so **3,176 bytes** of
headroom. Two of 904 host objects want more than that; one wants 56,248.

**The question.** glibc sizes the surplus in `_dl_tls_static_surplus_init()`
from the `glibc.rtld.optional_static_tls` tunable. ⚠ In a **static** binary
`__libc_setup_tls` runs before `main`, so the tunable would have to be in the
environment (`GLIBC_TUNABLES`) at exec time — which a library cannot arrange
for itself, and which `docs/design/host-fallback.md`'s AT_SECURE discipline
says must be ignored for a set-uid process anyway.

**Routes.**

| | |
|---|---|
| A | re-exec once with `GLIBC_TUNABLES=glibc.rtld.optional_static_tls=N` when a module needs more than the surplus. ⛔ Costs a re-exec and changes `/proc/self/exe` semantics; `pg83/solo` PR #5's ruling on intercepting routes-to-a-value is the read |
| B | link the binary so its OWN `PT_TLS` is padded, making `memsz` larger and the block bigger. ⛔ **MEASURED AND REFUTED — see below** |
| C | refuse, as now, and record the class. ⚠ 2 of 904 is the measured cost of doing nothing |
| D | ⭐ **NEW, and the same measurement that refuted B is what opens it:** hand out slices of the runtime's OWN `__thread` array instead of glibc's surplus |

### ⭐ MEASURED 2026-09-02d — B is refuted and D is opened by one probe

A static probe reads glibc's own `_dl_tls_static_size`/`_used`/`_align` and the
offsets of `errno` and of a padding array from the thread pointer. Built twice
in `pgb-env-debian12` (glibc 2.36, gcc 12.2.0), the only difference `-DPADSZ`:

    no pad     : size=3264  used=96     align=64  headroom=3168
    64 KiB pad : size=68864  used=65648  align=64  headroom=3216
                 pad at tp-65616

⛔ **Route B does not work.** Padding the executable's own `PT_TLS` raises
`_dl_tls_static_size` by 65,600 **and** `_dl_tls_static_used` by 65,552. The
headroom moves 3,168 → 3,216: **+48 bytes**, which is alignment rounding. The
surplus is a constant and does not scale with the program's own TLS, so "a
dummy `__thread` array sized by a flag" buys nothing *as a way of enlarging the
surplus*.

⭐ **But the same numbers show why D works.** The pad IS allocated, in every
thread, at a **stable offset from the thread pointer** — it is simply accounted
as `used` rather than as surplus. A loader that allocates initial-exec modules
out of **its own** `__thread` array therefore gets exactly what it reserved:
65,536 bytes against 3,168, and the one measured library wanting 56,248 fits.

⭐ **And D removes a dependency rather than adding one.** Placing a module in
our own array needs no `_dl_tls_static_used`/`_size`/`_align` at all; those
three glibc internals — the ones `docs/design/glibc-versions.md` §4 flags as
unversioned and unpromised — stay only for `el_tls_bookkeeping_sane()`'s
cross-check.

**Costs D must state, not discover.**

| | |
|---|---|
| every thread pays it | the reserve is in the executable's `PT_TLS`, so it is allocated per thread whether or not anything dlopens. ⛔ Sized by a flag, defaulting to not reserved |
| alignment | the array needs an explicit `aligned()` at least as large as any module `p_align` it serves — 64 observed here |
| threads | unchanged from today: the init image is seeded in the loading thread, and threads created later see zero |

### ⚠ And it explains a number this tree quotes two different ways

⚠ **Corrected 2026-09-02d, in four places.** `docs/limitations.md`, this
entry's title, `TODO/INDEX.md` and `TODO/runtime.md` all called the surplus
**3,456** bytes, while this entry's body and `docs/design/glibc-versions.md`
said `_dl_tls_static_size = 3264`. ⭐ **Both numbers were right, and the probe
above is why:** `_dl_tls_static_size` is the program's own
`PT_TLS` **plus** the surplus, so it moves with the binary — 3,264 for the
probe, 68,864 for the padded one. ⛔ **It is not the surplus**, and quoting it
as one invites the reader to conclude that a bigger binary has more room, which
is the opposite of true.

⭐ **The stable quantity is the headroom, `size − used`:** 3,168 bytes measured
today, 3,176 recorded previously. Where a number is quoted, that is the one to
quote.

### ⭐ IMPLEMENTED 2026-09-02e — `--tls-reserve N`, and the control fails the right way

`tool/runtime/pgb-elfload.c` now allocates initial-exec TLS out of its own
`__thread` array first and falls back to glibc's surplus. The size is
`pgb build --host-dlopen --tls-reserve N`, **default 0** — every thread pays
for the reserve whether or not anything is ever `dlopen`'d.

**The subject** is a shared object whose initial-exec `PT_TLS` is
`memsz 0xdbb8 = 56,248` bytes — the size the one real host object in the T-068
sweep wanted — and a probe that `dlopen`s it and calls into it:

    ARM  BUILT WITH                       RESULT
    A    --host-dlopen                    ⛔ REFUSED, exit 1:
                                          "static TLS surplus exhausted --
                                           needs 56248 bytes, 240 of 3456 used
                                           (reserve is 0)"
    B    --host-dlopen --tls-reserve 65536  ⭐ loaded, bigtls_touch(7)=1
    C    --host-dlopen --tls-reserve 1024   ⛔ REFUSED, and did NOT overflow:
                                          "needs 56248 bytes, 1280 of 4480 used
                                           (reserve is 1024)"
    C2   the same 1024 reserve, a 512-byte module  ⭐ loaded, =3

⭐ **A is the control and it is the whole argument**: same source, same loader,
same object, and the only difference is the flag. ⚠ **C is the second
control** — a reserve that exists but does not fit must refuse rather than
write past the array, and C2 in the same binary shows the reserve is genuinely
being used rather than bypassed.

⭐ **And the guard was verified against a known-bad change rather than trusted.**
With the fit check deleted from `el_tls_from_reserve()`, arm C stops refusing
and instead places the module past the end of the reserve: `dlsym` misses and
the process takes **SIGSEGV (exit 139)**. That is what the two lines of bounds
check are worth.

⚠ **`_dl_tls_static_size` reads 3,456 for this probe against 3,264 for the
one in the section above**, which is the same point restated: it is the
program's own `PT_TLS` PLUS the surplus, so it moves with the binary. The
headroom is 3,216 bytes here and 3,168 there.

⛔ **The threads limitation is UNCHANGED and is not fixed by this.** The
reserve is seeded with the module's init image in the loading thread only;
threads created afterwards see zero, exactly as with the surplus. Route D
changes where the storage comes from, not when it is initialised.

⛔ **NOT YET RE-RUN ACROSS THE ELEVEN.** `experiments/76-` is what would say
whether `--tls-reserve` costs anything on the matrix, and it has not been run
with it. The measurements above are the build host only.

**Prove.** ⭐ Arms A/B/C/C2 above, plus the known-bad change. ⚠ Outstanding:
`experiments/76-` on eleven environments with a non-zero reserve, and the two
objects from the T-068 sweep that fail today measured by name rather than by a
synthetic subject of the same size.
