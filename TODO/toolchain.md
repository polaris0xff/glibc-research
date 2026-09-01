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

**Prove.** `sh pgb build <a git URL>` produces a binary that `pgb verify`
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
never reads it. It calls `scripts/common/rootfs-run.sh` directly, which is
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
entry (T-015):** `oci-pull.sh` unpacks an image's filesystem and drops its
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

## T-015 — `oci-pull.sh` unpacks the filesystem and drops the image config

**Source** found while cross-checking the two `pgb verify` arms (T-014),
session of 2026-09-01.
**Category** toolchain · **Priority** P2 · **Effort** S · **Status** open

**Problem.** An OCI image is a filesystem **and** a configuration.
`scripts/common/oci-pull.sh` unpacks the layers and ignores the config, so the
chroot bed and a `docker run` of the same digest are not the same environment.

**Premise.** ⭐ **Measured**, `../docs/history/corrections.md` C10. The
`archlinux` image config carries `Env: LANG=C.UTF-8`; `docker run` applies it,
the chroot bed does not, and the same binary therefore takes a different
`setlocale` path under the two beds:

```
docker run --rm --entrypoint /usr/bin/env archlinux@sha256:818793c8… | grep LANG
  -> LANG=C.UTF-8
grep -c LANG scripts/common/rootfs-run.sh
  -> 0
```

⚠ **This invalidates no committed number.** The difference showed up only in
the **host data** column, which `../docs/AGENTS.md` §3 reports and never
asserts, and the two arms agree on all eleven rows for both asserted columns.
What was wrong was the *claim* that the beds are the same environment.

**Approach.** `oci-pull.sh` already parses the manifest; the config blob it
points at carries `Env`, `Cmd`, `Entrypoint` and `WorkingDir`. Record them
into `.oci-provenance`, and have `rootfs-run.sh` apply `Env` unless the caller
overrides it.

⛔ **Applying it must be a decision, not a default that appears silently.** A
bed that starts exporting `LANG` changes what every locale-sensitive result
describes, so the change lands with the affected experiments re-run, or it
lands behind a flag.

**Prove.** `sh pgb verify --engine chroot` and `--engine docker` on the same
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

**Closed with.** `tool/lib/wrappers.sh`, `link_flags()`. Measured after:

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
**Category** toolchain · **Priority** P1 · **Effort** S · **Status** open

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
