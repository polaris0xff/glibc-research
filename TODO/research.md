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

⭐ **RE-DERIVED FROM THE COMMITTED EVIDENCE, deep review 1, 2026-09-03c** —
eleven environments × two arms, each a mean of five
(`evidence/86-bundler-vs-anylinux/per-environment.jq.txt`):

| | ours | the field | ratio |
|---|---|---|---|
| size | 11,471,610 B | 4,006,916 B | 2.86× ⭐ struck from the bar |
| cold start | **128–149 ms**, mean 139 | **62–74 ms**, mean 67 | ⛔ **2.07×** |
| warm start | 12–19 ms, mean 14.9 | 8–14 ms, mean 10.8 | ⛔ 1.38× |

⛔ **This entry said "162–198 ms vs 79–107 ms, about 1.9×" and no version of
that evidence file ever carried those numbers** — 162 ms is this entry's own
**build-host** figure, compared against an eleven-environment competitor one.
The warm claim (~1.4×) was right. `../docs/history/corrections.md` C23.
⭐ **`86-`'s method is the one to carry into `90-`**, which takes one sample
per arm and whose ratios swing between 2.52× and 5.02× across four runs.

⛔ **The claim, stated so it cannot drift**, and its two halves now score
differently under the operator's ruling of 2026-09-03c:

| half | state |
|---|---|
| *"produced by one command from a package name"* | ⭐ **MET, and now a REQUIREMENT** — the ruling asks for *"one command not a multiline shell script"*, and the competitor's route is five separately versioned binaries plus a 121 KB driver script, a `.desktop`, an icon and ~nine environment variables. **Publish this.** |
| *"within measurable distance of one"* | ⛔ **not good enough on the clock** — "performs better" means **under 1.0×**, and cold start is **2.07×** |
| size, 3.05× | ⭐ **struck from the bar** |

**What is left.**

1. ⚠ **Debloating** — demoted. It was the explanation for the size ratio and
   size no longer counts. It stays open only insofar as fewer objects is less
   to mount, map and relocate, ⛔ **which nobody has measured.**
2. **OpenGL** — T-052 closed the mechanism; T-059 owns real hardware.
3. **Wrapper scripts.** A nixpkgs `bin/x` that is a shell wrapper is followed
   to its ELF and the wrapper's ENVIRONMENT is dropped. T-053; `patsh` is
   aimed at exactly this.
4. ⚠ **32-bit is IMPLEMENTED, the MEASUREMENT is missing.** `lib32` routing,
   the 32-bit loader copy and the by-name warning all exist and now carry
   seven hermetic `elfClass` cases; **no 32-bit application has been put
   through it.**
5. **Nothing is measured against a hand-built Anylinux AppImage for a GUI
   subject.** `experiments/90-` does it for kdenlive; T-055 owns that.
6. ⭐ **`xplshn/pelf` IS READ, 2026-09-03c** — findings in
   [`../docs/research/portable-nix.md`](../docs/research/portable-nix.md),
   mechanisms at file and line in
   [`../docs/research/portable-nix-mechanisms.md`](../docs/research/portable-nix-mechanisms.md).
   ⭐ **It is the closest thing in the corpus to our own bundler**: a Go
   program producing dwarfs/squashfs single-file bundles, and it claims faster
   startup than AppImage — which is the axis the operator just made binding.
   ⛔ **Nobody has run it here.** Three things it has and we do not:
   - a **size-thresholded startup policy** — mount below 350 MB, EXTRACT
     above it. Our kdenlive bundle is 398 MB; the competitor's is 192 MB;
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
