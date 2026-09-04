# research — sweeps and prior art

Binding: [`../docs/methodology/references.md`](../docs/methodology/references.md).

⚠ **Open entries only.** The 9 closed ones are
[`../HISTORY/entries/research.md`](../HISTORY/entries/research.md); the
long-form findings behind the entries below are
[`../HISTORY/entries/research-open.md`](../HISTORY/entries/research-open.md).

---

## T-021 — Build one nix-appimage and run it on the matrix

**Source** follow-on from T-020 · **Category** research · **Priority** P2 · **Effort** M · **Status** open

**Problem.** T-020's claims about what nix-appimage *costs* come from its
tracker, which is evidence of intent and never of behaviour.

**What is left.** `nix bundle` the same subject `experiments/60-` uses, run it
on all 11 with the `62-` instrument. ⚠ Needs nix on the build host — `pgb
bootstrap` installs it, so this is no longer the blocker it was.

**Prove.** `evidence/64-nix-appimage/RESULT.txt` with the coverage and
host-object columns filled, comparable to `60-` and `62-`.

📚 [detail](../HISTORY/entries/research-open.md)

## T-057 — The bundler: a maintained nix-appimage descendant, on the Anylinux mechanisms

**Source** ⭐ **operator, 2026-09-01c**, the second of three goals: *"make the
'universal' bundler true via a modern, updated, maintained 'nixappimage'
descendant that uses or rather reimplements many of the anylinux tooling,
iterating/improving them, and debloating nixappimages, correctly packing them,
and also solving the opengl problem"*.
**Category** research · **Priority** P1 · **Effort** L · **Status** open

⭐ **Landed already.** `internal/bundle/appimage.go` builds one: uruntime +
dwarfs + sharun instead of appimage-type2-runtime + mksquashfs + a bwrap
AppRun, with the nixpkgs **closure** replacing sharun's ldd-and-strace library
discovery. `experiments/86-` measured it against a hand-built Anylinux
AppImage on `jq 1.8.2`: **11 of 11 both sides, 0 host objects both sides**.

⭐ **THE SPEED HALF OF THE BAR IS MET ON THIS SUBJECT, 2026-09-03d** — eleven
environments × two arms (`evidence/86-bundler-vs-anylinux/per-environment.jq.txt`):

| | ours | the field | ratio |
|---|---|---|---|
| size | 6,806,407 B | 4,006,949 B | 1.70× ⭐ struck from the bar |
| cold start | 54–64 ms, mean **58.3** | 50–63 ms, mean **58.4** | ⭐ **1.00× — parity**, and ours is faster on **6 of 11 rows** |
| warm start | 7–10 ms, mean 8.5 | 8–12 ms, mean 9.3 | ⭐ **0.92× — ours is faster** |

⚠ **Read the warm row as "no difference measurable, possibly ours".** Its
medians are equal (9 ms both) and only the means separate; `docs/AGENTS.md`
§10's noise floor applies to a difference this size.

⛔ **AND THE WARM COLUMN'S ARITHMETIC IS UNVERIFIED — deep review, 2026-09-03d.**
`86-` obtains warm as `(six invocations − cold) / 5`, which assumes the cold
run is the first of the same series. ⚠ It is not: the cold sample is one
chroot enter and the six-invocation total is a **second** enter, begun within
the 4–6 s window `experiments/99-` measured for uruntime's mount reuse — so
the first of the six is warm too, and the formula subtracts a cold run that is
not in the series it divides. That is the same assumption class C24 is about,
and it is why single-digit and near-zero warm figures appear in the table
(2 ms on one row). ⭐ **The cold column is sound** — its reap is a whole-rootfs
reap, which really does kill the mount. ⛔ Do not quote the warm figure as a
magnitude; `experiments/clock.sh` is the shape to carry into `86-` next.

⛔ **How it got here, because the closure did not change and neither did the
sweep.** Two constants in `internal/bundle/appimage.go`:

| | jq cold, ours vs the field | what changed |
|---|---|---|
| before 2026-09-03d | **2.07×** | — |
| after `experiments/77-` | **1.28×** | uruntime v0.5.6 **full** → v0.5.9 **lite** |
| after `experiments/81-` | ⭐ **1.00×** | dwarfs block `-S26` (64 MiB) → `-S18` (256 KiB) |

⚠ **The size row moved the other way** — 1.44× after the runtime change, 1.70×
after the block size, because smaller blocks compress worse. That is the trade
the 2026-09-03c ruling licenses, and it is stated rather than hidden.

⛔ **The earlier numbers, and why they were wrong twice.** This entry once said
*"162–198 ms vs 79–107 ms, about 1.9×"* and no version of that evidence file
ever carried them — 162 ms was this entry's own **build-host** figure compared
against an eleven-environment competitor one. Deep review 1 replaced that with
the re-derived **2.07×**, which was correct for the runtime shipping at the
time. `../docs/history/corrections.md` C23 and C24.

⛔ **The claim, stated so it cannot drift**, and its two halves now score
differently under the operator's ruling of 2026-09-03c:

| half | state |
|---|---|
| *"produced by one command from a package name"* | ⭐ **MET, and now a REQUIREMENT** — the ruling asks for *"one command not a multiline shell script"*, and the competitor's route is five separately versioned binaries plus a 121 KB driver script, a `.desktop`, an icon and ~nine environment variables. **Publish this.** |
| *"performs better"* | ⭐ **MET ON THIS SUBJECT** — cold **1.00×**, warm **0.92×**, eleven environments. ⚠ Parity is not "better"; the honest sentence is *"no difference measurable on cold, possibly ahead on warm"* |
| size | ⭐ **struck from the bar**, and it went the other way: 1.70× |

⭐ **AND THEY DO CARRY — kdenlive WAS re-measured, 2026-09-03d.** `90-` now
uses the corrected protocol and its cold row went **4.92× against us → 0.74×
FOR us** (380.2 vs 513.9 ms, A/A control 1.02 against a 1.06 floor), with host
objects **0 of 11 against the competitor's 4 of 11**. ⛔ Its warm row is 3.45×
against us and unexplained, and its render direction is unresolved. T-055.

**What is left.**

1. ⚠ **Debloating** — demoted twice. It was the explanation for the size ratio
   and size no longer counts; then `experiments/84-` measured that removing
   bytes buys **0.024–0.031 ms per MiB**, so it is not a clock lever either.
   ⛔ Its remaining value is not startup.
2. **OpenGL** — T-052 closed the mechanism; T-059 owns real hardware.
3. **Wrapper scripts.** A nixpkgs `bin/x` that is a shell wrapper is followed
   to its ELF and the wrapper's ENVIRONMENT is dropped. T-053; `patsh` is
   aimed at exactly this.
4. ⚠ **32-bit is IMPLEMENTED, the MEASUREMENT is missing.** `lib32` routing,
   the 32-bit loader copy and the by-name warning all exist and now carry
   seven hermetic `elfClass` cases; **no 32-bit application has been put
   through it.**
5. ✅ **A GUI subject IS measured against the competitor** — `experiments/90-`,
   kdenlive, 2026-09-03d. T-055 carries the table.
6. ⭐ **`xplshn/pelf` IS READ, 2026-09-03c** — findings in
   [`../docs/research/portable-nix.md`](../docs/research/portable-nix.md),
   mechanisms at file and line in
   [`../docs/research/portable-nix-mechanisms.md`](../docs/research/portable-nix-mechanisms.md).
   ⭐ **It is the closest thing in the corpus to our own bundler**: a Go
   program producing dwarfs/squashfs single-file bundles, and it claims faster
   startup than AppImage — which is the axis the operator just made binding.
   ⛔ **Nobody has run it here.** Three things it has and we do not:
   - a **size-thresholded startup policy** — mount below 350 MB, EXTRACT
     above it. Our kdenlive bundle is 565,332,219 B (398 MB before `-S18`);
     the competitor's is 192 MB;
   - the parsed runtime config cached in an **extended attribute on the
     artefact itself**, so later starts skip re-parsing the ELF;
   - **live-mount reuse** across invocations, exposed as `REUSE_INSTANCES`.
7. ⭐ **AND IT ANSWERS AN OPERATOR OPEN QUESTION ABOUT `.desktop` FILES** —
   `.DirIcon` and `*.desktop` sit at the AppDir top level in 99.99% of
   AppImages, so they can be lifted rather than authored. `design/nix-front-end.md`.

**Prove.** `evidence/86-bundler-vs-anylinux/RESULT.jq.txt` and
`RESULT.mpv.txt` — the same application both ways, on all eleven, with size,
startup and host-object columns.

📚 [detail](../HISTORY/entries/research-open.md) — including why the startup
instrument had to be rewritten after its first version measured itself.

## T-059 — GL on real hardware, and the NVIDIA case

**Source** split out of T-052 when it closed, 2026-09-01d. ⭐ It is the
operator's own open question 3 — *"a GPU. T-052 cannot be honestly closed on a
machine with no graphics hardware"*.
**Category** research · **Priority** P1 · **Effort** M · **Status** open

**Blocked on hardware, and that is stated rather than worked around.**
`experiments/85-` shows a bundle carries a complete GL stack that initialises
and names its driver on eleven distributions with none of their own. It shows
nothing about **iris, radeonsi, amdvlk or NVIDIA** — this machine has no GPU
and `swrast` is what it can reach.

**Two questions, and they are different.**

1. **Bundled mesa against a real DRM device.** ⚠ mesa's userspace/kernel
   contract is loose, so the expectation is that it works — an expectation is
   not `evidence/`.
2. ⛔ **NVIDIA proprietary, where the userspace half MUST match the running
   kernel module.** `nixGL` reads `/proc/driver/nvidia/version` and **fetches**
   a matching driver. A bundle cannot fetch at run time, so the honest options
   are: detect the host's NVIDIA userspace and use it (reintroducing
   `limitations.md` §1 deliberately), carry several and pick, or say the case
   is unserved. ⭐ The Anylinux flow already chose the first, with a switch.

⭐ **What can be done here WITHOUT hardware, and should be first:** implement
the host-NVIDIA detection path and assert it finds **nothing** on all eleven —
the negative half is available now and it is the half that says the detection
code runs at all.

**Prove.** A row per environment for a machine that has a GPU, with vendor and
renderer strings, plus the detection path exercised on the eleven that do not.

📚 [detail](../HISTORY/entries/research-open.md)

## T-080 — ⛔ REOPENED: the capability guarantee, on THREE applications per category

**Source** ⭐ **operator, 2026-09-03d** for the guarantee itself; ⛔ **reopened
by the operator, 2026-09-03f**: *"every capability listed in
docs/research/bundle-capabilities.md including ones already measured, must be
remeasured with 3 applications per category in order of simple to complex
applications ... all capabilities are closed as 'MEASURED, AND IT WORKS'"*.
**Category** research · **Priority** P1 · **Effort** L · **Status** open

⚠ **The closed version and everything it established are
[`../HISTORY/entries/research-open.md`](../HISTORY/entries/research-open.md)
`T-080 · retired detail`.** It is not repeated here.

⛔ **WHY ONE SUBJECT WAS NOT ENOUGH, AND THE RECORD ALREADY SHOWS IT.**
`experiments/64-` scored GTK on ONE application, `galculator`, and got **0 of
11** — from which *"GTK does not work out of a nix closure"* would have been
the obvious and WRONG conclusion. A second subject, `mousepad`, drew **11 of
11** through the same bundler on the same day, and the boundary turned out to
be a compiled-in data path rather than GTK. ⭐ One subject measures a subject.

**What is left.** `experiments/65-` is the corpus: three applications per
category, ordered by how much of the stack they drag in, each scored by a
**window on a real X server** (or, for a CLI subject, its exit status AND a
required string) plus **zero host shared objects** on all eleven.

⛔ **The rows that must come out MEASURED rather than assumed** are the ones
§0 currently labels otherwise: **SDL** (never run through this pipeline),
**Vulkan** (the ICD mechanism is relocatable by design and nothing has made a
Vulkan call here), **Qt**, **Python GUI** (T-081 unblocked it) and **apps with
a compiled-in data path** (T-081's own subject).

⚠ **TWO ROWS CANNOT BE CLOSED ON THIS MACHINE AND MUST SAY SO IN THE
SENTENCE**: every GL and Vulkan row here is a SOFTWARE rasteriser — `llvmpipe`
and `lavapipe` — and **NVIDIA is not bundled by design**. `T-059` owns
hardware, and a green corpus is not a GPU claim.

**Prove.** Every row of
[`../docs/research/bundle-capabilities.md`](../docs/research/bundle-capabilities.md)
§0 carrying a count out of eleven and the subject that produced it, with the
UNRESOLVED subjects listed by name and reason rather than dropped.

---

## T-087 — ⭐ the battle-test corpus: 40+ applications, ordered by MECHANISM

**Source** ⭐ **operator, 2026-09-04**, with a list of applications *"to
battletest and prove our bundler is best in class"* and an explicit ordering
rule: *"sort the tasks for next session based on completing/fixing what will
auto fix/complete what, not easy first"*.
**Category** research · **Priority** P1 · **Effort** XL · **Status** open

⛔ **The classification is done and it is
[`../docs/research/app-corpus.md`](../docs/research/app-corpus.md).** This entry
is the work; that page is the argument for its order. Read the page first — the
eight rungs are the task list, and each names what it unblocks.

**Why the order is not "easy first".** A subject is neither hard nor easy; a
**mechanism** is present or missing, and ordering by app re-measures the same
mechanism five times. Rungs 1–3 are three mechanisms that between them decide
about twenty of the subjects.

⭐ **Rung 1 and rung 2 are each a day and each closes a claim now made from
source rather than from a run** — T-088 and T-089. Do them first for that
reason, not because they are small.

**⚠ THE MEASURE-TWICE RULE IS SUSPENDED FOR THIS ENTRY**, by the operator:
*"Use cached/prebuilt fetches to make the install/build fast, and get rid of
the measure twice rule; we need to cover more cases for now, we can refine
later."* ⛔ Delivery rules 1, 2, 4, 5, 6 and 7 all still hold — only rule 3
is lifted, and only here.

**Prove.** *"All three apps open, run and work, use emulated/dummy stubs for
hw gaps."* Open = a toplevel ≥50×50 on a real X server seen with `xwininfo`.
Work = an assertion the application answers, never a log line a broken bundle
also prints. Stubs are for **hardware only** and each is a stated limit.

**Study, in this order.** `docs/research/app-corpus.md`; then, in the tree,
`references/pkgforge-dev__Anylinux-AppImages/tree/HALL-OF-FAME.md` (per-toolkit
grades), `tree/useful-tools/demo/*` (eleven minimal per-toolkit recipes — the
fastest way to see what a toolkit needs), `tree/useful-tools/hooks/*` (fifteen
named workarounds), and ⭐ `api/issues.json` + `api/comments.json`, **825
issues and 1,000 comments**, which is where the per-application knowledge is.
⚠ `ivan-hc/*`, `pkgforge-dev/distrobox-AppImage`, `flatimage`, `flatroot`,
`gameimage`, `VHSgunzo/{lux-wine,lw-runtime,Run-wrapper,runimage-nvidia-drivers}`
are named by the operator and are **NOT vendored** — fetch before relying on
them.

---

## T-088 — ⛔ multi-entry dispatch is SHIPPED and has never been run

**Source** the operator's question, 2026-09-04: *"Can we handle and are we
handling apps that provide multiple binaries like busybox … does simply
renaming or symlinking the bundle with the app we want to enter work?"*
**Category** research · **Priority** P1 · **Effort** S · **Status** open

⭐ **The answer read off the source was YES, and that was the problem.**
`assemble.go` installs every non-dot program in the entry store path's `bin/`
automatically; `--with-program` adds one from anywhere in the closure; and
`tool/runtime/pgb-apprun.c` is a **static** selector.

## ⭐ MEASURED — `experiments/68-`, `pass=24 fail=0 skip=0`

**Arm S, the dispatch table** (a synthetic AppDir; no bed, no display) —
18 of 18, and it **corrected the source's own header comment**, which stated
the order wrongly and named `ARGV0` nowhere:

    $ARGV0's basename  →  argv[1] (and it is DROPPED)  →  argv[0]  →  default

with it: a selected `argv[1]` is consumed and later arguments survive; an
`argv[1]` that did not select is passed through; the child's `argv[0]` is the
**absolute** path; `ARGV0` is unset and `APPDIR` set in the child; ⛔ a name
containing `/` is **never** a program, so a path cannot be injected; and
`shared/bin` without `bin/` is not a program. ⭐ The negative control — the
same source with an empty default — exits **127** with `no default program`
while still dispatching when told a name.

**Arm B, a real closure on all eleven** — `mkvtoolnix`:

| | |
|---|---|
| the build's entry-point count | ⭐ **`programs mkvmerge + 4 more`** |
| the selector actually built | the **static** `pgb-apprun`, not a shell |
| ⭐ the SECOND program (`mkvextract`), by its own name | ✅ **11 / 11** |
| the entry (`mkvmerge`), same artefact — within-row control | ✅ **11 / 11** |
| the second program's host shared objects | ⭐ **0 on 11 / 11** |

⛔ **The assertion is each program's OWN identity**: `mkvextract --version`
says `mkvextract`, so a dispatch that quietly ran the default fails. That is
why `imagemagick` was rejected as the subject — its `convert` and `identify`
are symlinks to one binary printing one string.

⭐ **The operator's second question is answered by a number the tool emits**,
not by an inference: `programs <prog> + N more`.

## ⛔ WHAT THE PROVE LINE ASKS FOR AND THIS DID NOT DO

⚠ **The entry stays open, and the goalposts are not being moved.**

1. ⛔ **THIS ITEM SAID `--with-program` WAS NEVER EXERCISED AND THAT WAS
   FALSE — `experiments/90-` HAS BEEN EXERCISING IT SINCE IT WAS WRITTEN.**
   Found 2026-09-04c by reading the script rather than the entry, which is
   delivery rule 5 doing its job on this page:

       ./pgb bundle appimage kdenlive --with-program melt --with-program ffmpeg
       …
       /kd-arm melt -version
       /kd-arm melt color:blue out=12 -consumer avformat:/tmp/kd.mp4 …

   ⭐ **`melt` is in the `mlt` store path, not in `kdenlive`'s `bin/`**, so it
   is precisely the whole-closure branch this item is about — and `90-` does
   not merely start it, it makes it **render an MP4**, on **all eleven**
   environments, and reports `P=ok E=ok` per row. ⚠ It also knows the
   assertion discriminates: its own comment records that the competitor's
   artefact runs *kdenlive* with `melt` as an argument.

   ⭐ **A second, independent exercise, 2026-09-04c**, on a subject chosen for
   it rather than for something else: `flameshot`, whose own closure carries
   `dbus-daemon` and `dbus-run-session` in a **dependency** store path —
   exactly the case this branch exists for, because they are not in the
   entry's `bin/`:

       pgb bundle appimage flameshot --name flameshot \
           --with-program dbus-daemon --with-program dbus-run-session
       →  programs    flameshot + 2 more
          apprun      pgb-apprun, static, 785704 bytes (no host interpreter)

       ./flameshot.AppImage dbus-daemon --version    (debian-12, in the bed)
       →  D-Bus Message Bus Daemon 1.16.2

   ⭐ **The assertion discriminates**: `dbus-daemon` prints its OWN identity,
   so a dispatch that quietly ran the default would print flameshot's banner
   instead. ⚠ **One environment, by hand** — a second closure rather than a
   second eleven, and this entry does not count it as the Prove line.

   ⛔ **So what this item actually still owes is narrow**: the `flameshot`
   demonstration on the eleven. The whole-closure branch itself is measured
   across the bed by `90-`, twice over — a start and a render.
2. **Two names, not N.** The bundle carried five programs and the run asserted
   two of them. A third name costs one more row and would catch a selector
   that only ever gets the first two right.
3. **One closure.** A second multi-program subject — `rnote`/`rnote-cli` is
   the rung-1 example — is a different measurement, not a repeat.

**Prove.** One artefact, N names, N assertions, on all eleven — and the
`--with-program` path exercised at least once, because it is the branch that
searches the whole closure rather than one `bin/`.

**Study.** `tool/runtime/pgb-apprun.c` (130 lines, and it is the answer);
`internal/bundle/assemble.go` `installProgram` and the `bin/` walk above it.

---

## T-089 — ⛔ the interposer row marked NOT MEASURED: a static or raw-syscall payload

**Source** [`../docs/design/store-paths.md`](../docs/design/store-paths.md) §3,
which marks exactly one row of its own comparison table as **NOT MEASURED**.
**Category** research · **Priority** P1 · **Effort** S · **Status** open

The interposer wins calls that go **through the PLT**. A statically linked
program, or one issuing raw syscalls — a Go binary is the common case — has no
PLT, so the mechanism cannot see its `open`. ⛔ That row is reasoning about a
mechanism, not a result, and it sits in a shipped design document.

**The subject is named and it is in the corpus**: `syncthing`, a Go program
from the closure. `lilipod` and `powershell` are the same shape.

⛔ **PRE-REGISTER THE FAILURE.** The expected outcome is that the build
**reports** the compiled-in store path and the program cannot resolve it at run
time. If it passes, the reasoning was wrong and the record says so rather than
being quietly corrected.

**Prove.** One Go subject bundled and run on all eleven, with the build's own
`store paths … compiled in` line quoted beside the result, and
`docs/design/store-paths.md` §3's row changed from NOT MEASURED to whatever
came out.

## ⭐ MEASURED — `experiments/100-`, three arms, `pass=13 fail=3`

⛔ **The three failures are the findings and none of them is the one that was
pre-registered.**

**Arm P — the mechanism** (planted store path, real interposer, real
`.storemap`, same source built three ways; **two runs identical**). ⭐ **The one
row split into two, failing for different reasons:**

| probe | our object mapped | resolves |
|---|---|---|
| ⭐ dynamic through the PLT — the positive control | yes | ✅ yes |
| ⛔ `-static` | **no** | no |
| ⛔ `syscall(SYS_openat, …)` | **yes** | no |
| ⭐ dynamic, no preload — the negative control | no | no |

⛔ Only the first is about linking. **A subject can be fully dynamic and still
defeat the interposer.**

**Arm G — `syncthing`.** ⛔ **G3 falsified the entry's own premise**: nixpkgs'
build is **dynamic**, not static, so the subject named here for the static row
was the wrong shape. It ran **11 of 11**, clean **11 of 11**, 8 store paths
compiled in and 7 resolving.

**Arm L — `lilipod`, genuinely static.** ⛔ **`pgb bundle appimage` REFUSES a
loader-less closure**, so the interposer question never arises — there is no
artefact. The raw binary, no bundle at all:

| | |
|---|---|
| ⭐ **the static ELF EXECUTES** | ✅ **11 / 11** |
| ⛔ **the APPLICATION completes** | **2 / 11** |
| zero host shared objects | **1 / 11** |

⭐ **Nine failures, one cause, named by the program itself**: *"failed to find
dependency `getsubids`, can't recover"* — a **host program on `$PATH`**.
Exactly two rootfs carry it (`archlinux-latest`, `opensuse-leap-15.6`) and
those are exactly the two that completed. ⚠ One of them also needed the
**network**: openSUSE has no `tar`, so it downloaded busybox at run time.

⭐ **THE AXIS MOVED.** A statically linked **ELF** is portable — that is what
static linking buys, and it is 11 of 11. A statically linked **application**
need not be, and nothing a linker, loader or bundler does addresses a
dependency that is **another program**. ⚠ It also explains the host-object row:
when a static program execs host programs, the host's libraries enter the
**process tree** — the same mechanism `experiments/65-` pre-registers for
`xterm` (C5), reached independently on a different subject.

## ⛔ What is still owed

1. **The `-static` row still has no application behind it.** Arm P is a
   mechanism result; arm L never got to ask the question because the bundler
   refused. A static subject that *does* need a compiled-in path is still
   wanted — `powershell` is the remaining named candidate.
2. ⚠ **An open question arm G raised rather than settled.** Go issues raw
   syscalls for much of its file I/O even when dynamic, so S3 predicts the
   interposer loses — yet 7 of 8 paths resolved. Build-time **text** rewriting,
   the program never opening them, or those opens routing through libc are all
   consistent. `LD_DEBUG` plus one traced row separates them.
3. ⭐ **The refusal message sends the reader to the wrong problem.** It names a
   missing **loader**; the useful sentence is that a payload which already
   starts everywhere does not need bundling.

---

## T-090 — ⛔ the sandbox rung needs a BED change, not a bundler change

**Source** the operator's browser list, 2026-09-04, and its link to the
field's `fix-namespaces` hook.
**Category** research · **Priority** P1 · **Effort** M · **Status** open

`unshare(CLONE_NEWUSER|CLONE_NEWNS)` is `EPERM` in the chroot bed — already the
recorded reason every `onelf` row in [`../docs/comparison.md`](../docs/comparison.md)
runs in its last-resort mode. A Chromium sandbox needs exactly that call, so a
browser row run in this bed measures `--no-sandbox`, which is a different
program.

## ⭐ THE CAUSE IS ISOLATED AND THERE IS A ROUTE — `experiments/69-`

⛔ **"EPERM in the chroot bed" named no cause, and a cause you have not
isolated is an unfinished measurement rather than a blocker.** `pgb rootfs run`
does **two** things — `unshare --mount` *and* `chroot` — so neither could be
blamed from the bed row alone. Run without the other, on the same rootfs,
same kernel, same probe:

| arm | `unshare(CLONE_NEWUSER)` |
|---|---|
| ⭐ the HOST, one process per call (**the control**) | **OK** — all five namespaces |
| the bed, entered as `pgb rootfs run` enters it | ⛔ **EPERM** |
| ⭐ **`chroot` ALONE**, no unshare | ⛔ **EPERM** |
| ⭐ **`unshare --mount` ALONE**, no chroot | ✅ **OK** |
| ⭐ **the same rootfs entered by `pivot_root`** | ✅ **OK**, and `CLONE_NEWUSER\|CLONE_NEWNS` too |

⭐ **So the refusal is `chroot`'s.** It is not the machine (the host permits
every namespace), not the kernel's policy here, and not a sysctl —
`/proc/sys/user/max_user_namespaces` reads **64230** *inside* the bed. And it
is specific to `CLONE_NEWUSER`: mount, pid and net all unshare inside the bed
(`N8`). ⭐ `lsns -t user` inside the bed reports exactly **1** — the operator's
"check, do not guess", answered.

⭐ **The route, therefore, is named rather than wished for**: entering the bed
by **`pivot_root`** instead of `chroot` permits the call. ⛔ **That is not the
same as saying `pgb rootfs run` should do it** — three things are still
unmeasured and none is implied by `69-`: whether the bed still isolates
correctly under `pivot_root`, whether teardown stays clean, and whether a
bundled browser then sandboxes. `69-` says the kernel permits the call; it says
nothing about the other three.

⚠ **Runs:** `pass=9 fail=0 skip=0` on `debian-12`, **three times**; and
`pass=8 fail=0 skip=1` on `alpine-3.22`, where `N7` **skips** because busybox
ships no `lsns`. ⛔ The alpine run is why that row is a skip: the first version
counted `lsns | wc -l` and reported **0**, which is a missing tool wearing a
disagreement's clothes. `history/corrections.md` C29.

⛔ **Two questions, and merging them is the error to avoid.**
1. *Does the bundle carry a working browser?* — answerable today with
   `--no-sandbox`, and worth having.
2. *Does the sandbox work?* — needs a bed that allows user namespaces, and a
   check that is not a guess: `lsns -t user` from inside the sandboxed process,
   or `ip netns list`.

⚠ **And a target-side condition that is not ours**: Ubuntu ≥ 23.10 sets
`kernel.apparmor_restrict_unprivileged_userns=1`, so the same bundle fails
there for a reason unrelated to bundling. The field ships a `pkexec` hook that
asks the user to disable it. ⛔ **Do not copy that hook** — it asks for a root
password — but **detect and report** the condition instead of showing a crash.

**Study.**
`references/pkgforge-dev__Anylinux-AppImages/tree/useful-tools/hooks/fix-namespaces.md`
and `fix-namespaces.hook` beside it, both vendored;
`HALL-OF-FAME.md` "Excellent - Chromium/Electron", which grades the toolkit
easy and says nothing about the sandbox.

---

## T-091 — GStreamer needs four variables and a scanner; NOTHING was setting them

**Source** `HALL-OF-FAME.md` "Bad - GStreamer", read against
`internal/bundle/sharun.go` `bakedOverride`.
**Category** research · **Priority** P2 · **Effort** S · **Status** open

`bakedOverride` emitted `GST_PLUGIN_SYSTEM_PATH_1_0` and nothing else. The
field names four: `GST_PLUGIN_PATH`, `GST_PLUGIN_SYSTEM_PATH`,
`GST_PLUGIN_SYSTEM_PATH_1_0`, `GST_PLUGIN_SCANNER`.

## ⛔ THE ENTRY DESCRIBED THE WRONG LAYER, and reading the sources says so

⚠ **"We emit one of the four" implied sharun supplies the rest.** It sets all
four itself (`set_appdir_env.rs`, `dir.starts_with("gstreamer-")`) for a
directory under its own `shared/lib`.

⛔ **A FIRST VERSION OF THIS ENTRY SAID THAT BRANCH CANNOT FIRE HERE, AND THAT
WAS WRONG.** It reasoned that `copyLibraries` flattens every shared object into
one `lib/`. It flattens loose objects, but it also carries **every directory
under a store path's `lib/` whole** (`copyTreeNoClobber`, logged as
`lib trees N directories under lib/ carried whole`), and `shared/lib` is a
symlink to `../lib`. ⭐ **So `shared/lib/gstreamer-1.0` does exist and that
branch CAN fire.** Whether it does is **not established** and the corpus row is
what will say. Caught by a deep review of the change, before it was measured.

⭐ **What survives, and it is the part that mattered**: `GST_PLUGIN_SCANNER` is
**definitely** not set. sharun sets it only when the scanner sits **beside** the
plugins, and nixpkgs puts it in `libexec/gstreamer-*/` — so that test cannot
succeed on a nixpkgs closure however the plugin directory is laid out.

⭐ **And the fourth is a different kind of thing from the first three.** Three
are directories, and the store farm already holds the original tree they point
into. `GST_PLUGIN_SCANNER` names a **program** that GStreamer runs as a child
process — and in nixpkgs it lives at `libexec/gstreamer-*/gst-plugin-scanner`,
a different top-level directory from the plugins, which is why sharun's
"beside the plugins" test could not have found it even with an unflattened
`lib/`.

⛔ **Naming the merged copy directly would have been the wrong fix.** That file
is an ordinary dynamic ELF, so starting it brings up the **host loader and the
host libc** — inside a bundle whose entire claim is that it does not. ⭐ The
field reaches the same conclusion from the other side: `quick-sharun`'s
`_handle_bins_scripts` **hardlinks sharun over every `gst-*` binary** and puts
it in the gstreamer libdir.

**Landed** (⛔ **UNMEASURED — see Prove**): `bakedOverride` emits the three
path variables, ⚠ **possibly redundantly with sharun's own and deliberately
so** — a duplicate entry in a path list is harmless where a missing one is
not, and ours name the store farm, which resolves to the same tree; `installGstScanner` installs the scanner as a bundle *program*,
so `bin/gst-plugin-scanner` is a sharun hardlink; `writeEnv` names it, keyed on
that program existing, exactly as every other line there is keyed. The name is
one constant, `gstScannerName`, because two literals are two things that have
to agree — the coupling class T-092 is about.

⚠ **A consequence to expect in a corpus row**: a GStreamer subject now carries
**two** programs, so it gets the static `pgb-apprun` selector rather than
sharun-as-AppRun. That path is measured (`experiments/68-` arm S) but this
*combination* is not.

⭐ **ONE HALF IS NOW MEASURED ON A REAL SUBJECT, 2026-09-04b.** The corpus
built `mpv` (`media-1`) and its build log reads:

    gstreamer   scanner installed as a program (GST_PLUGIN_SCANNER follows)

So `installGstScanner` fires on a real nixpkgs closure, not only on the
synthetic path. ⛔ **That is the build half and it is not the Prove line.**

⛔ **Prove — and NOTHING ABOVE IS A RESULT UNTIL THIS RUNS.** A media subject
that plays, with its host-object count explained: `gst-plugin-scanner` *"opens
every single gstreamer plugin on the system"*, so a count taken on such a
subject must say **which process** it counted. Any row for `lmms`, `handbrake`
or `gnome-music` has to name that. The negative control is the same subject
built without these variables.

⛔ **AND `media-1` AS THE CORPUS DEFINES IT CANNOT BE THAT ROW.** Its argument
is `--version`, which never launches the scanner, so its host-object count
cannot discriminate — the row would be green and mean nothing. ⚠ Fixing C39's
assertion made `media-1` a valid *corpus* row — **11/11 pass, 11/11 clean,
2026-09-04c** — and it did **not** make it T-091's.

## ⭐⭐ MEASURED — `experiments/103-`, `pass=7 fail=0 skip=0`, TWO runs identical

⭐ **A bundled GStreamer application ENCODES and DECODES on all eleven, with
ZERO host shared objects in the payload AND in the whole process tree**, and
`gst-plugin-scanner` is **exec'd on 11 of 11** — so the scanner is not a
hypothetical here, it runs, and the tree count is still zero.

| | |
|---|---|
| **D1** the encode leg — `audiotestsrc → vorbisenc → oggmux → file` | ⭐ **11 / 11** |
| **D2** ⭐ the decode leg — `filesrc → oggdemux → vorbisdec → audioconvert → file`, PCM **larger** than the Ogg it came from | ⭐ **11 / 11** |
| **D3** host shared objects in the payload | ⭐ **0 on 11 / 11** |
| **D4** ⭐ `gst-plugin-scanner` `execve` seen | **11 / 11** — and `HOST(tree)` is **0** anyway |
| **C0** the control ran | **11 / 11** |
| **C1** ⛔ the control failed | **0 of 11** — it did **not** fail |

## ⛔ AND THE ANSWER IS THE BRANCH THE SCRIPT PRE-REGISTERED AS THE OTHER ONE

⭐ **The four variables are REDUNDANT on this subject**, and that is a finding
about the **interposer's reach** rather than a failure of the entry. The
control is a shipped flag (`--no-plugin-env`) and it was **verified to differ**
before the rows were read — `B3`: the subject carries **one** `GST_*` variable
and the control **zero** — and the pipeline works either way.

⛔ **So GStreamer is finding its plugins through its compiled-in default
directory, which is a `/nix/store` path, which `pgb-storefix.c` answers.**
⭐ That is the same mechanism `experiments/105-` measures on `file`(1), reached
from a completely different subject.

⚠ **What that does NOT mean.** It does not mean the variables are wrong to
emit: a closure whose plugin directory is *not* a compiled-in default — or a
subject that consults `GST_PLUGIN_SYSTEM_PATH` before its own default — would
need them, and none has been measured. ⛔ It means the entry may **not** claim
the variables are what makes GStreamer work here, because on the one subject
measured they are not.

## ⭐ AND THE RUN FOUND A REAL DEFECT ON THE WAY — a shell fragment lifted as a value

⛔ Run 1's control could not be told from its subject, and the reason was a
bundler bug rather than a harness one. `bin/gst-launch-1.0` is a nixpkgs
wrapper whose `GST_PLUGIN_SYSTEM_PATH_1_0` value is the GStreamer
**setup-hook's shell fragment**:

    $(unset _tmp; for profile in $NIX_PROFILES; do
        _tmp="$profile/lib/gstreamer-1.0…"; done; printf %s "$_tmp")

⛔ The wrapper reader lifted it verbatim into `.env`. There is no
`$NIX_PROFILES` in a bundle and nothing expands `$( )`, so the variable held
literal garbage **and shadowed** whatever else would have set the path — in
the subject and the control alike. ⭐ **Fixed**: a wrapper value containing
`$(` or naming `$NIX_PROFILES` is not a path and is not lifted, and the build
says so. ⚠ The test is deliberately narrow — a bare `$` is ordinary in a path
list and `${VAR}` is how `OpPrefix` composes.

## ⚠ Two defects in run 1 were MINE, and both are recorded rather than smoothed

- the decode leg ended `! wavenc !`, and `wavenc` is in **gst-plugins-good**,
  which this closure does not carry — so it read **0/11** on a bundle that had
  just encoded on **11/11**. ⛔ The fourth criterion-not-subject defect in this
  tree and the first written here;
- `B3` read the wrong `.env`: both bundles share one cache and one `--name`, so
  the AppDir on disk is whichever built **last**. ⚠ And the first fix was
  incomplete — on a **cached** run neither is built, so the copy has to happen
  only after a build that actually ran.

---

## ⭐ THE ROW AS PRE-REGISTERED — `experiments/103-`, 2026-09-04c

⛔ **Committed before it runs**, delivery rule 1, and it answers the three
questions `media-1` cannot:

| | |
|---|---|
| **does it DECODE?** | a real round trip inside each rootfs: `audiotestsrc → vorbisenc → oggmux → file`, then `filesrc → oggdemux → vorbisdec → wavenc → file`. ⭐ The criterion is that the WAV is **larger** than the Ogg it was decoded from, because PCM is bigger than Vorbis — the application's own answer, which no broken bundle prints |
| ⭐ **which PROCESS does the count describe?** | the same trace classified in **`payload`** and **`tree`** mode side by side, plus whether a `gst-plugin-scanner` `execve` appears at all. ⚠ **Reported, not predicted**: nobody has measured whether the scanner runs inside a bundle here |
| ⛔ **the negative control** | ⭐ **`--no-plugin-env` is a SHIPPED FLAG**, for the same reason `--no-storefix` is: it builds the same closure with none of `GST_PLUGIN_PATH`, `GST_PLUGIN_SYSTEM_PATH`, `GST_PLUGIN_SYSTEM_PATH_1_0` or `GST_PLUGIN_SCANNER`, and says so in the log |

⛔ **THE CONTROL'S OUTCOME IS NOT PREDICTED EITHER WAY, and both are written
into the script before the run:**

- the pipeline **fails** → the four variables are load-bearing and T-091's
  mechanism is measured;
- the pipeline **works** → ⭐ the variables are **redundant on this subject**,
  because GStreamer's compiled-in default plugin directory is a `/nix/store`
  path and `pgb-storefix.c` already answers it. That is a finding about the
  interposer's reach, not a failure — and it would mean this entry should say
  so rather than claim a mechanism nothing needed.

⚠ **What it will not measure**: hardware decode. There is no `/dev/dri` here,
so every codec path is software (the C3 limit again), and one container format
on one codec pair is not *"media works"*.
