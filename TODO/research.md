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
