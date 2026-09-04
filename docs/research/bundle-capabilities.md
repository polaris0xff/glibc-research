# What a bundle can actually do, and what a package manager needs from it

⛔ **Two questions, both mined 2026-09-03d, both owed a deliverable next
session.** This page is the evidence base; the deliverables are in
[`../../TODO/PROGRESS.md`](../../TODO/PROGRESS.md) G2.1 and F2.

| | |
|---|---|
| **G2.1** | is anything left unsolved in a nix bundle a **capability** problem — can nix do EGL/SDL/XCB, can it load vulkan/nvidia — or only tooling, size and performance? |
| **F2** | what do `gearlever`, `AppManager`, `AM` and `soar` require of an AppImage, and does ours meet it? |

Every reference is pinned; `references/<owner>__<repo>/PROVENANCE.md` has the
commit.

---

## ⭐ 0. THE GUARANTEE — what is left unsolved, and what kind of problem it is

⛔ **This section is the T-080 deliverable and it replaces nothing below; §1 is
still the field's record and is still a list of HYPOTHESES about a different
pipeline.** What is new is that some of those rows have now been **run through
`pgb bundle appimage`**, which is the only thing that makes them ours.

> **The guarantee, stated so it can be falsified.** Of what is left unsolved in
> a `pgb` nix bundle today, **everything measured is tooling** — a path our
> patcher does not rewrite, an entry-point shape our reader does not resolve.
> ⛔ **No measured failure is "nix cannot do EGL/SDL/XCB" and none is "nix
> cannot load vulkan or nvidia".** ⚠ And one capability question is **not
> measured at all** rather than measured green: anything requiring a **GPU**.

⭐ **THE TWO TOOLING BLOCKERS THIS SECTION NAMED ARE CLOSED — T-081**, and both
were closed by a mechanism rather than by a workaround. `experiments/64-`,
**two runs**, `pass=11 fail=0 skip=0` each:

| arm | subject | window on a real X server | host `.so` |
|---|---|---|---|
| **G** | `galculator` — UI is a file at a compiled-in absolute store path | ⭐ **11 / 11**, and no bind | **0 / 11** |
| **N** | ⭐ the SAME bundle, built `--no-storefix` | ⛔ **0 / 11** | — |
| **X** | `mousepad` — the regression control | ✅ **11 / 11** | **0 / 11** |
| **P** | `meld` — ⭐ **Python 3 + GTK 3, a SCRIPT entry point** | ⭐ **11 / 11** | **0 / 11** |

⭐ **Arm N is why arm G means anything**: the negative control is a shipped
flag, the same subject and the same bundler with one mechanism absent, and it
draws nothing anywhere. ⭐ **Arm P is the operator's own counter-example
reached** — *"in nixappimage python is easy and works"* — on an application
that produced no artefact at all a day earlier.

⛔ **THE TABLE BELOW IS STILL BEING RE-MEASURED.** T-080 is REOPENED: every row
must be re-derived with **three applications per category, simple to complex**
(`experiments/65-`), because one subject measures a subject. ⚠ A row that still
says *hypothesis* or *not measured* means exactly that.

⭐ **Progress, 2026-09-04b** — `evidence/65-capability-corpus/rows/`, one file
per subject, and a recorded row is never re-measured:

| category | subjects in | verdict so far |
|---|---|---|
| **GTK 3** | ⭐ **3 of 3 — CLOSED** | `galculator`, `mousepad`, `geany`, all **11/11 pass and 11/11 clean** |
| ⭐ **X11 / XCB** | **3 of 3, all passing** | `xeyes` **11/11**, `xclock` **11/11**, ⭐ `xterm` **11/11** once C37 was fixed. ⛔ `xterm`'s **clean** count is **4/11**, and that is **C5 coming true** — see below |
| ⭐ **OpenGL / EGL** | **3 of 3** | ⭐ `eglinfo` **11/11**, `glxgears` **11/11** — both clean 11/11. ⛔ `glmark2` **0/11** |
| ⭐ **Vulkan** | **3 of 3, and the third is the BED** | `vulkaninfo` **11/11**, `vkcube` **11/11** — both clean 11/11. ⭐ `vkmark` **0/11 because there is no GPU**: it dies with `directory iterator cannot open directory … [/dev/dri]`, and `/dev/dri` exists **nowhere** here. The bundle carries `lvp_icd` (lavapipe) and the other two subjects use it |
| **Python GUI** | 1 of 3 in | ⭐ `meld` **11/11**, clean 11/11 — the third C6 control. ⏳ `pdfarranger` and `virt-manager` re-queued against the C37 `pgb` |
| **Qt** | 1 of 3 | ⭐ `qalculate-qt` **11/11**, clean **4/11** |
| **the field's recipes** | 3 of 4 | ⛔ `neovim` **0/11** (glibc 2.26, below), `flameshot` **0/11**, `gearlever` UNRESOLVED. `helix` re-queued |
| SDL, Qt, media | 0 | ⏳ running |

⭐ **THE THREE C6 POSITIVE CONTROLS ARE ALL IN AND ALL GREEN** — `galculator`,
`mousepad` and `meld`, each **11/11**, each agreeing with `experiments/64-`,
which measured them twice. ⛔ That is what makes the rest of this table
readable at all; a run where a control came back below 11 would be a statement
about the instrument, not about a capability.

⭐ **AND ONE ZERO TURNED OUT TO BE THE BED, NOT THE BUNDLER.** `vkmark`'s
truncated note read `Error: filesystem error: directory iterator cannot open
directory: No `. Rebuilt and run by hand, the **full** line ends `[/dev/dri]`:
vkmark enumerates DRM devices before it asks Vulkan anything, and this machine
has none. ⚠ **The hypothesis this page carried — that it was a compiled-in
data directory the farm did not answer, the T-081 shape — was WRONG**, and it
was wrong for both subjects it was offered for. ⭐ What the Vulkan row now
says is sharper than three passes would have been: **software Vulkan works
everywhere, and a benchmark that needs a real DRM device cannot run here at
all.**

## ⭐ THE ENTRY-POINT SHAPE PREDICTS THE RESULT, and C37 fixes a CLASS

Every subject's build log says whether its entry is a plain ELF or a
**script/shell wrapper**. Read against its row:

| entry shape | rows | result |
|---|---|---|
| **plain ELF** | 9 | ⭐ **8 pass at 11/11.** The one failure is `vkmark`, and that is the **missing GPU** (`/dev/dri`) |
| ⛔ **script / shell wrapper** | 6 measured | ⛔ **5 fail.** `gl-3`, `x11-3`, `py-2`, `py-3`, `field-2` |

⭐ **AND THE FIX IS MEASURED ON TWO OF THEM, BY HAND.** With C37's fix in
place, `xterm` draws in **2 s** and `glmark2` draws in **2 s** and benchmarks
at **360 FPS** — both scored **0 of 11** by the corpus, which is running a
`./pgb` built before the fix.

⚠ **`meld` is the counter-example that keeps this honest**: it is a script
entry and it passes **11/11**. ⭐ So the broken thing is not "a script entry" —
it is specifically **a wrapper that `exec`s a dot-named ELF target by absolute
store path**, which is what C37 describes. `meld`'s interpreter *reads* its
script; nothing dot-named is exec'd.

⛔ **`field-2` (`neovim`) is in the failing column and is NOT C37**: its
closure carries glibc 2.26 and `ld.so` learned `--argv0` in 2.33 — diagnosed
independently, by hand. ⚠ It is re-measured anyway, because a row measured
against a tool with a known fixed defect is not evidence about anything else.

⛔ **A NOTE-FIELD DEFECT WORTH FIXING BEFORE THE NEXT PASS.** A row's note is
the **first** line matching `Couldn't load|cannot open|Traceback|…`, cut to 70
characters. For a Python subject that is `Traceback (most recent call last):` —
⭐ **the one line of a traceback that carries no information**, since the
exception is the *last* line. It cost a diagnosis on `pdfarranger`, and the
same truncation hid `neovim`'s answer (`… cannot open sh` was
`… cannot open shared object file`). ⚠ Prefer the **last** line of a traceback,
and keep more than 70 characters.

⭐ **`eglinfo` and `vulkaninfo` each went `0/11` → `11/11` when C34 and C36
were fixed.** ⚠ Their store-path counts moved too — `vulkan-1` from `12
compiled in` to **42** — because `--extra mesa` had been mangled to `Mesa` and
was failing to resolve. ⛔ **That is how much a separator collision was
costing**, and it is why the two rows are re-measured rather than annotated.

⛔ **THE ZEROS ARE REAL ROWS AND THEY ARE NOT AN INSTRUMENT DEFECT.** Every
`cli` subject has failed and every `gui` subject except `xterm` has passed,
which looks exactly like a broken CLI mode. ⭐ It was checked rather than
assumed: the command shape `65-` uses for a `cli` subject was reproduced by
hand against a real artefact and returned **exit 0 with correct output**. The
failures are subject-specific and each needs its own reading.

| row | what is known | ⛔ what is NOT yet known |
|---|---|---|
| `xterm` **0/11**, clean **11/11** | ⭐ **C5 predicted it would fail the HOST-OBJECT row and it did not** — it is clean on 11 of 11. ⚠ It also never drew, so the prediction is not so much falsified as **unevaluable**: a subject that does not start its shell never loads the shell's libc | why it does not draw |
| ⭐ `neovim` **0/11** — **ROOT CAUSE FOUND, and it is real** | the closure carries **glibc 2.26**; see below | — |
| `helix` **0/11** | ⚠ a named limitation of the farm, below — ⛔ **not established as the cause** | whether it is the cause |
| ⭐ `eglinfo` **0/11** — **TWO INSTRUMENT DEFECTS, NOT A FAILURE** | C34 and C36, below | — |

### ⚠ `helix`: a named limit of the store farm, and it is NOT established as the cause

`helix`'s closure carries **~200 bare `.so` files at a store path's top level**
— its tree-sitter grammars, `rust.so`, `python.so`, … — and the build says so:
*"store paths carry top-level entries the bundle does not merge"*.

⛔ **`mergedFor` maps eight names** (`bin`, `sbin`, `lib`, `lib64`, `lib32`,
`share`, `etc`, `libexec`) **to somewhere in the bundle. A top-level entry that
is not one of them has no home**, so `buildStoreFarm` cannot answer a
compiled-in path naming it, and `fillFarmDir` skips it.

⭐ **The route is short and it is not "add another name to the table"**: those
grammars *are* in the bundle already — `copyLibraries` takes **every** shared
object in the closure and flattens it into `lib/`. So a bare top-level `.so`
could be answered by pointing it at `lib`, which is where its content actually
went. ⚠ Unmeasured, and it must not be shipped on this reasoning alone: helix
also finds its grammars through `HELIX_RUNTIME`, so it may never consult the
compiled-in path at all.

⛔ **What is NOT established**: that this is why `helix` scores 0 of 11. The
warning and the failure are two observations of the same subject, and nothing
has connected them. ⚠ A fixed `mergedFor` that left the row at 0 would be the
useful result.

⚠ **One thing WAS fixed**: the warning called them *"directories"*.
`topLevelNames` returns files too, and every one of these is a file — the word
sent a reader looking for the wrong thing.

### ⭐ `xterm`: BOTH pre-registered predictions about it are now settled

⛔ **C5 was pre-registered before the corpus ever ran**: *"`xterm` is expected
to FAIL C2 … xterm's whole job is to run the user's SHELL, which is a HOST
program, so a host libc enters the process by construction."*

⚠ **For most of this session that prediction was UNEVALUABLE**, and the record
said so: `xterm` scored `0/11` and `11/11 clean`, because a subject that never
starts never loads anything. ⭐ **C37 made it start**, and both predictions
resolved at once:

| | before C37 | after |
|---|---|---|
| passes | ⛔ **0 / 11** | ⭐ **11 / 11** |
| host-object clean | 11 / 11 — ⚠ *because it was dead* | ⭐ **4 / 11** |

⭐ **So C5 is CONFIRMED, on its own terms**: `xterm` runs and it is not clean.
⚠ That is the application, not the bundler, and it is the one row in this
corpus where a dirty count is the **expected** answer.

⛔ **AND IT WAS CHECKED AGAINST A REGRESSION**, because `qt-1` came back
`11/11` passing and `4/11` clean in the same pass, and before C37 every passing
subject was clean on 11 of 11. ⭐ `galculator` — the T-081 acceptance subject —
rebuilt with the C37 `pgb` and traced: **zero host shared objects**. So the
change introduced no host loading, and those `4/11` counts belong to their
subjects.

### ⛔ `eglinfo` and `vulkan-1`: TWO defects, and the second was hiding behind the first

⭐ **Both rows read `0 of 11` on capabilities that work. Neither was a bundler
failure.**

**The capability, measured by hand** out of the corpus's own artefacts —
renaming them to reach the program, which is `experiments/68-`'s dispatch rule
used as a diagnostic:

| | |
|---|---|
| `eglinfo`, run **exactly as the corpus runs it** — inside a rootfs, with a real display bound in | a full EGL config table; the real assertion matches **30 times** |
| `vulkaninfo --summary` | exit **0**; `GPU0 deviceName llvmpipe (LLVM 21.1.8, 256 bits)`, `apiVersion 1.4.354` |

**Defect 1 — the `cli` criterion (C34).** `65-` scored a `cli` subject as
`exit 0` **AND** the assertion. `eglinfo` exits **3** headless because some EGL
platform is unavailable — ⚠ and still exits 3 with `XDG_RUNTIME_DIR` set and
every `error:` line gone, so it is not a bed condition. ⭐ Fixed: when a subject
names an assertion, **the assertion is the criterion** and a non-zero status is
**reported** beside it.

⛔ **Defect 2 — the corpus separator (C36), and it only surfaced because
fixing defect 1 did not move the number.** The corpus was `|`-separated and an
assertion is a `grep -E` pattern that **alternates**:

| gl-1's field | should be | got |
|---|---|---|
| `assert` | `(llvmpipe\|Mesa\|softpipe)` | ⛔ `(llvmpipe` — **`grep: Unmatched (`**, exit 2, can never match |
| `extras` | `mesa` | ⛔ `Mesa` — the build log said `could not resolve --extra Mesa` |
| `args` | *(none)* | ⛔ `softpipe)` — passed to the program |

⚠ **The build log had been saying so all along** and nobody read it. ⭐ The
separator is `;` now, and both rows were **deleted and re-measured** — a row
from a broken instrument is deleted, never adjusted.

### ⚠ `helix`: a named limit of the store farm, and it is NOT established as the cause

`helix`'s closure carries **~200 bare `.so` files at a store path's top level**
— its tree-sitter grammars, `rust.so`, `python.so`, … — and the build says so:
*"store paths carry top-level entries the bundle does not merge"*.

⛔ **`mergedFor` maps eight names** (`bin`, `sbin`, `lib`, `lib64`, `lib32`,
`share`, `etc`, `libexec`) **to somewhere in the bundle. A top-level entry that
is not one of them has no home**, so `buildStoreFarm` cannot answer a
compiled-in path naming it, and `fillFarmDir` skips it.

⭐ **The route is short and it is not "add another name to the table"**: those
grammars *are* in the bundle already — `copyLibraries` takes **every** shared
object in the closure and flattens it into `lib/`. So a bare top-level `.so`
could be answered by pointing it at `lib`, which is where its content actually
went. ⚠ Unmeasured, and it must not be shipped on this reasoning alone: helix
also finds its grammars through `HELIX_RUNTIME`, so it may never consult the
compiled-in path at all.

⛔ **What is NOT established**: that this is why `helix` scores 0 of 11. The
warning and the failure are two observations of the same subject, and nothing
has connected them. ⚠ A fixed `mergedFor` that left the row at 0 would be the
useful result.

⚠ **One thing WAS fixed**: the warning called them *"directories"*.
`topLevelNames` returns files too, and every one of these is a file — the word
sent a reader looking for the wrong thing.

### ⛔ `eglinfo`: the capability WORKS and the row is the instrument's

⭐ **Run by hand out of the corpus's own artefact.** `mesa-demos` bundles
`glxgears + 309 more`, so `eglinfo` was reached from the existing `gl-2`
AppImage by **renaming it** — which is `experiments/68-`'s dispatch rule used
as a diagnostic, on a 310-program bundle.

| | |
|---|---|
| does it run? | ⭐ **yes** — a full EGL config table |
| the corpus assertion `(llvmpipe\|Mesa\|softpipe)` | ⭐ **matches 20 times** |
| its exit status | ⛔ **3** |

⛔ **`65-`'s `cli` criterion is `exit 0` AND the assertion.** `eglinfo` exits
**3** in a headless environment because some EGL platforms (wayland, gbm) are
unavailable — ⚠ and it **still exits 3** with `XDG_RUNTIME_DIR` set and every
`error:` line gone, so it is not a bed condition either. It simply never
returns 0 here, while answering completely.

⭐ **So the OpenGL / EGL row must NOT be read as a failure.** The capability is
demonstrated; the row measures the criterion.

⛔ **The fix, and it cannot be made while `65-` is running**: when a subject
carries an assertion, **the assertion is the criterion** and the exit status is
reported beside it; a subject with no assertion keeps the status as its
criterion. ⚠ Then **delete the `gl-1` row and re-measure** — a row produced by
a broken instrument is deleted, never adjusted.

⚠ **Two other rows are at risk from the same rule** and must be re-read when it
changes: `vulkan-1` (`vulkaninfo --summary`) and `media-1` (`mpv --version`),
both `cli` with an assertion.

### ⭐ `neovim`: one old glibc, two unrelated-looking messages

⛔ **The corpus note was truncated at 70 characters and that hid the answer.**
The full error is the **dynamic loader's**:

    --argv0: error while loading shared libraries: --argv0: cannot open
    shared object file: No such file or directory

⭐ **Reproduced by hand, then diagnosed from the bundle it built.** The
`neovim` closure carries `glibc-2.26-115`, and:

| | |
|---|---|
| sharun starts a dynamic payload as | `<loader> --library-path <p> --argv0 <a> [--preload …] <bin> …` |
| `ld.so` learned `--preload` in | glibc **2.30** |
| `ld.so` learned `--argv0` in | glibc **2.33** |
| the closure's loader | **`ld-2.26.so`** — it rejects even `--version` the same way |

⛔ **A loader with no option parsing takes the first argument as the program**,
so `--argv0` becomes the thing it tries to load. The bundle cannot start.

⭐ **AND THE SAME OLD glibc EXPLAINS THE OTHER WARNING THAT BUILD PRINTED** —
*"the store-path interposer was NOT installed: the bundle's libc does not
define dladdr, dlsym"*. Measured on that `libc.so.6`: it exports **0** of the
two. They lived in `libdl.so` until glibc **2.34**. ⚠ Two messages that look
unrelated, one cause, and `pgb` already had the evidence in hand.

⭐ **Landed**: `checkLoaderOptions` reads the loader's glibc version at build
time and warns with **the exact runtime string** a reader will otherwise search
for. ⚠ It warns rather than refusing, and the reason is not timidity: sharun
skips the loader command line entirely for a **static or already-patched**
payload, so such a closure still works. Five selftest cases, and the
store-path one was checked against a planted regex.

⚠ **The count is `ls evidence/65-capability-corpus/rows/*.tsv`**, and each row
is `<id> <pass> <rows> <clean> <store paths> <note>`.

| capability | status **for our pipeline** | evidence |
|---|---|---|
| ⭐ **GTK 3** | ✅ **CLOSED, THREE SUBJECTS, SIMPLE → COMPLEX.** `galculator` **11/11**, `mousepad` **11/11**, `geany` **11/11** — each a real toplevel window on a real X server, each with **zero host shared objects on 11 of 11**. ⭐ Store paths compiled in / resolving: 88/85, 104/101, 90/87 | `65-` `gtk3-1..3`; `64-` arms G and X |
| ⭐ **XCB / X11 client stack** | ⏳ **1 of 3 subjects in.** `xeyes` **11/11**, clean **11/11**, 12 store paths compiled in and 11 resolving — ⭐ a pure Xlib/XCB client with no toolkit above it. `xclock` and `xterm` are still running; ⛔ **`xterm` is pre-registered to FAIL the host-object row** and the reason is the application, not the bundler: its job is to run the user's shell, which is a host program | `65-` `x11-1..3` |
| ⭐ **EGL** | ✅ **11 / 11, and clean on 11 / 11.** `eglinfo` out of the `mesa-demos` closure enumerates EGL configs on every environment, naming `llvmpipe`. ⛔ Every row is a **software rasteriser** — C3, and T-059 owns hardware | `65-` `gl-1`; `85-` |
| ⭐ **OpenGL driver stack** | ✅ `glxgears` draws a real window on **11 / 11**, clean **11 / 11**. The bundle carries mesa and points libglvnd at itself; `85-`'s negative control (`--no-gl`) produces no vendor string on any row | `65-` `gl-2`; `85-` |
| ⭐ **Vulkan** | ✅ **11 / 11, and clean on 11 / 11** — `vulkaninfo --summary` enumerates a device on every environment: `apiVersion 1.4.354`, `deviceName llvmpipe (LLVM 21.1.8, 256 bits)`. ⛔ **C3's limit is the whole point**: a **software rasteriser**, saying nothing about a real GPU or NVIDIA | `65-` `vulkan-1` |
| **NVIDIA** | ⛔ **NOT MEASURED, and not bundled by design.** The driver is taken from the HOST; `design/host-fallback.md` governs it. T-059 owns the hardware | — |
| **SDL** | ⛔ **NOT RUN through our pipeline.** A hypothesis, graded *Excellent* by the field | §1 |
| ⭐ **Python GUI** | ✅ **MEASURED, AND IT WORKS.** `meld` — Python 3 + GTK 3 through PyGObject — draws a real toplevel window on **11 of 11** with **zero host shared objects**, on eleven distributions of which four ship no glibc and none ship Python or GTK. ⛔ It produced **no artefact at all** before T-081 | `64-` arm P |
| ⭐ **apps with a compiled-in data path** | ✅ **MEASURED, AND IT WORKS.** `galculator` draws on **11 of 11** with **no bind**, against **0 of 11** for the same bundle built `--no-storefix` | `64-` arms G and N |

### ⭐ The two blockers were ours, and BOTH ARE CLOSED — T-081

⭐ **Neither was ever a statement about nix.** In both, the closure fetched
completely and every library resolved.

1. **A hardcoded absolute store path to a data file.** `galculator` connected
   to a real X server and then died on
   `Couldn't load /nix/store/<hash>-galculator-2.1.4/share/galculator/ui/main_frame.ui`.
   ⭐ **That file was in the bundle all along**, at
   `AppDir/share/galculator/ui/`. `XDG_DATA_DIRS` points at the bundle's own
   `share` and serves every application that *looks up* its data; it cannot
   serve one with the path baked into its `.rodata`.
   ⭐ **CLOSED by an interposer** loaded through sharun's own `.preload`,
   rewriting **by exact match against the closure** `pgb` already computes — a
   path outside that set is reported, never guessed.
   ⛔ **The route NOT taken, and why, is the first thing
   [`../design/store-paths.md`](../design/store-paths.md) says**: a same-length
   rewrite to a fixed `/tmp` path needs no relocation and is unshippable,
   because the tree it would serve is loadable code and the path is squattable.
2. **A script entry point.** `meld` — a Python 3 + GTK application — produced
   no artefact at all. `resolveEntry` oscillated: `bin/meld` is a
   `makeBinaryWrapper` ELF whose target `bin/.meld-wrapped` is a **Python
   script**; `ReadWrapper` returned nothing for a script and `elfx.IsELF` was
   false, so `lastExistingStorePath` scanned the script's text and resolved
   back to `bin/meld`. Five hops, then `no entry point`. ⚠ **That is the
   standard nixpkgs shape for a Python application**, not a `meld` quirk.
   ⭐ **CLOSED by making the entry point a PAIR**: a script resolves to
   *interpreter + script argument*, laid out as the interpreter under its own
   name, the script under `shared/script/`, and a **static trampoline** at
   `shared/bin/<name>` that joins them — sharun execs a static binary directly,
   so it needs no loader cooperation and drags no host libc in.
   ⛔ A shebang naming a HOST interpreter is refused rather than adopted.

### ⛔ How this section corrects itself, and the correction is the point

⚠ **An earlier version of `experiments/64-` scored GTK 11 of 11 GREEN.** Its
criterion was that the program printed `Gtk-WARNING **: cannot open display:`,
on the reasoning that the message is emitted by the bundled `libgtk-3` and so
proves it loaded.

⛔ **The operator rejected that and was right**: *"previously nixappimage
bundled apps showed the same error on real hw with display"*. The message does
not discriminate — it is identical when there is no display and when the
bundle's own X stack is broken, which is the only distinction the experiment
exists to make. ⭐ **The fix was to feed it a real display** (`Xvfb`, socket
bound into each rootfs) and to check for a **window on the X server with
`xwininfo`, from outside the process**. That turned 11 green rows into **0**,
and then a second subject showed the blocker was the data path rather than GTK.

⭐ **This is why the deliverable is three arms and not one.** Same bundler,
same GTK, same eleven environments:

⛔ **THIS TABLE IS THE STATE OF 2026-09-03e AND IT IS SUPERSEDED. IT IS KEPT
BECAUSE IT IS THE "BEFORE" THE OPERATOR'S ACCEPTANCE TEST NAMES** — *"arm G
must go 0 of 11 → 11 of 11 WITHOUT the bind arm C uses"*. The current numbers
are the four-arm table at the top of this section.

| arm | subject | window on a real X server | ⭐ 2026-09-04, after T-081 |
|---|---|---|---|
| G | `galculator` — UI is a **file** at a compiled-in absolute store path | ⛔ **0 / 11** | ⭐ **11 / 11, and no bind** |
| X | `mousepad` — UI is a **GResource compiled into the binary** | ✅ **11 / 11** | ✅ 11 / 11 |
| ⭐ C | `galculator` **again**, with that store path made to resolve **by a bind mount** | ✅ **11 / 11** | ⛔ **retired** — replaced by the shipped mechanism, and by arm N, which is the same bundle with that mechanism switched off |

⭐ **Arm C is what made this a measurement rather than a reading of an error
message.** It ran the **identical artefact** with one variable changed — the
bundle's own `AppDir` bound at the `/nix/store/<hash>-galculator-2.1.4` the
binary names — and it drew, on all eleven, still with **zero host shared
objects**. ⚠ **The bind was never a fix and was never proposed as one**: it
needs root and a mount namespace, which a user double-clicking an AppImage has
neither of. It existed to isolate the cause, and it did.

⭐ **T-081 replaced it with something a user actually gets**: an interposer in
the bundle, no bind, no root, no namespace — and arm C's job as a control
passed to **arm N**, which is stronger, because a negative control built from a
shipped flag (`--no-storefix`) tests the mechanism rather than the diagnosis.

⛔ **So the sentence "the remaining gap is tooling, not capability" is earned.**
Without arm C it would have been an inference from a log line — which is
exactly the error the operator caught the first time.

---

## 1. ⭐ The capability question, answered from the field's own record

`pkgforge-dev/Anylinux-AppImages` keeps `HALL-OF-FAME.md`, a per-library
verdict written by people who ship these in production. ⭐ **It is the most
useful document in the corpus for G2.1**, because it separates *"this library
is hard to deploy"* from *"this cannot be deployed"* — and almost nothing is
in the second category.

⛔ **BUT EVERY VERDICT BELOW IS SUBJECTIVE AND IS ABOUT A DIFFERENT PIPELINE.
DO NOT CARRY ANY OF IT ACROSS WITHOUT RE-DERIVING IT AGAINST nixappimage.**
These grades were earned deploying **distribution packages** — Arch, through
`quick-sharun` — where the deployer has to discover a library's data files,
plugin directories and search paths by hand. ⭐ **A nix closure is the
opposite**: it is the exact set the derivation declared, with the paths already
correct, which is why `internal/bundle` can replace sharun's `ldd`-and-`strace`
discovery entirely (`appimage.go`'s own header says so).

⚠ **The operator's counter-example, and it is the right shape:** *"in
nixappimage for instance, python is easy and works, choose any python gui app
and it works"* — against a **"Utter garbage"** grade below. A grade that
inverts on the pipeline is not evidence about the pipeline we use.

⭐ **SO THE TABLE BELOW IS A LIST OF HYPOTHESES TO TEST, NOT A LIST OF
FINDINGS**, and G2.1's job is to re-derive each one against `pgb bundle
appimage` and **move the bottom rows up**. ⛔ The goal is explicit: every
"Garbage", "Horrible" and "Utter garbage" row should come out **Excellent or
close** through a nix closure, or this project should be able to say precisely
which mechanism stops it.

| verdict | libraries |
|---|---|
| **Excellent** | ⭐ **SDL**, iced/GLFW, Chromium/Electron, Flutter, ⭐ **Mesa** |
| Good | PipeWire, Qt, .NET, libdecor, FFmpeg, ⭐ **NVIDIA drivers** |
| Mediocre | LLVM |
| Bad | ALSA, GStreamer, OpenSSL |
| Horrible | p11kit, ⛔ **glibc**, WebKit, JACK2 |
| Garbage | GTK, Wayland |
| Utter garbage | Python |

### ⭐ The three rows that decide G2.1

**Mesa — Excellent.** Verbatim: *"Vulkan/OpenGL ICD discovery is also handled
automatically and it looks into `XDG_DATA_DIRS` among a ton of other locations
to find those files. **And the icd files support relative library locations to
the icd file itself**"*. ⭐ **So Vulkan and OpenGL out of a bundle is a solved
mechanism**, and the relative-ICD property is what makes it relocatable. Also:
Mesa can now build the radeon drivers without linking LLVM, *"which has
resulted in a massive decrease of our AppImages"*.

**SDL — Excellent.** *"Very easy to deploy, SDL does not have excessive
dependencies and it is very configurable thru env variables."* One upstream
bug, reported and fixed.

**NVIDIA — Good, and the mechanism is the opposite of bundling.** *"we never
need to bundle the NVIDIA drivers, NVIDIA releases its driver linking to a
+10yo version of glibc, that means we can use that driver without issue."*
⚠ With two caveats they name: distributions breaking things, and some *"ancient
libs"* having to be present. ⚠ They also record their own discomfort —
*"I still see this idea of relying on host libraries as flawed"* — which is
[`../design/host-fallback.md`](../design/host-fallback.md)'s question, already
decided here with four permitted classes.

⛔ **Wayland is "Garbage" and GTK is "Garbage", but read what the entries say.**
⚠ And read them as *their* pipeline's grades — see the caution above.
The failures listed are compositor and driver defects — GNOME defaulting to a
broken Vulkan renderer on Intel, mutter crashing the whole session, mutter not
doing server-side decorations. ⚠ **Those are not bundling failures**, and a
guarantee about our bundler must not claim to fix them.

### ⛔ And glibc's entry is the strongest corroboration this project has

Written by the *bundling* camp, about the same failure class this project
exists for:

- `LOCPATH` **does not work with locale archives**, so NixOS carries a patch to
  make locale archives relocatable;
- `GCONV_PATH` must be set, *"and good luck figuring out which gconv plugin
  your app exactly needs, and when the plugin is missing there is no error
  about it, it is just totally random what happens"*;
- ⭐ **they have to patch `ld-linux.so` to stop it reading `/etc/ld.so.cache`,
  because otherwise it "would segfault instantly on some systems"**.

⭐ **That is `docs/AGENTS.md` §2's finding reached from the other direction.**
A bundle that carries its own libc still has to fight the host's data and the
host's loader cache; a static binary that never dlopens has neither problem.
⚠ It is also a ready-made row for **G1.1**'s comparison table.

### ⚠ What the field records as BROKEN, which is the honest baseline

`pkgforge/soarpkgs` at `55c774a5` carries **16** `nixappimage` recipes. Three
are disabled:

| package | recorded reason |
|---|---|
| `ghostty` | ⛔ **"Fails to create EGL Display"**, citing `NixOS/nixpkgs#9415` |
| `lazarus` | "Exec Format Error" |
| `wget2` | "Doesn't do anything" |

⭐ **13 of 16 are ACTIVE**, including chromium, brave, discord, telegram and
helix. So *"nix cannot do GUI"* is false, and the bar a guarantee has to clear
is higher than that.

⚠ **What this project has measured against it:** `experiments/85-` runs EGL out
of a `pgb` closure at `pass=10 fail=0`, with the data-coherence arm's negative
control firing on a real bundle. ⛔ **Every row is `swrast` and surfaceless.**
So the claim available today is *"the closure produces a working EGL display
offscreen"* — more than the disabled recipe above achieved, and less than *"the
GPU problem is solved"*. **T-059 owns real hardware and G2.1 must say which of
the two it is claiming.**

---

## 2. ⭐ The package-manager contract, measured against our artefact

⛔ **Measured, not read — with one exception, named in the table.** ⚠ A deep
review of this page caught it claiming otherwise: the dwarfs row was a
**reading** of `soar`'s `Cargo.toml` and `gearlever`'s probe, in a table whose
preamble said every row was run. Neither `dwarfsck` nor `7zz` is on this host,
so it cannot be run here. ⭐ What IS measured is `--appimage-offset` returning
**1487344** — exactly the lite uruntime's size — so the payload begins
immediately after the header, and it is a dwarfs image because our own packer
wrote it with `mkdwarfs`. **A third-party reader has still not been pointed at
it.**

Every other row was run against the artefact `experiments/77-` builds as its
`field` arm — a `pgb bundle appimage jq` at the shipped runtime and block size,
6,806,407 B, `evidence/77-uruntime-header/RESULT.txt` — and, where it matters,
against the competitor's `kdenlive-AppImage-Enhanced` for comparison.
⚠ The artefact itself is a build product under a gitignored directory, so it is
named by the experiment that produces it rather than by path.

| what a manager needs | who needs it | ours | the competitor |
|---|---|---|---|
| ⭐ **GIO content type in `supported_mimes`** — gearlever's ACTUAL gate (`AppImageProvider.py:78, 256`) | `gearlever` | ✅ `gio info` → **`application/vnd.appimage`** | ✅ same |
| AppImage **type-2 magic** `0x41 0x49 0x02` at offset 8 | ⚠ **nothing checks it directly** — see below | ✅ `414902` | ✅ `414902` |
| `--appimage-mount` prints a mountpoint and holds it | `AM` (`modules/management.am:310-320`) | ✅ | — |
| `--appimage-extract` | `gearlever`, `AppManager`, `AM` | ✅ exit 0 | — |
| `file` says `static` — **AM's fallback detector** | `AM` (`management.am:311`) | ✅ | ✅ |
| the literal string `appimage-extract` — **AM's primary detector** | `AM` | ⛔ **0 occurrences** | ⛔ **0 occurrences** |
| `.DirIcon` at the AppDir top level | all four | ⚠ kdenlive ✅, `jq` ⛔ | — |
| a top-level `*.desktop` | all four | ✅ both | — |
| `X-AppImage-Version=` in the desktop entry | `gearlever`, `AppManager` | ⛔ **absent** | — |
| ⚠ the image is readable as **dwarfs** | `soar` (`squishy` with the `dwarfs` feature), `gearlever` (probes `dwarfsck`) | ⚠ **READ, NOT RUN** — see below | — |

### ⛔ A correction this table needed, found by a deep review

⚠ **An earlier version said gearlever "requires" the type-2 magic, citing
`get_appimage_type` at `AppImageProvider.py:480-491`.** That function computes
the type from bytes 8–10 — and is **defined and never called**. The real gate
is `can_install_file`, one line:

```python
return get_giofile_content_type(file) in self.supported_mimes
# ['application/x-iso9660-appimage', 'application/vnd.appimage',
#  'application/x-appimage', 'application/vnd.efi.iso']
```

⭐ **So the magic is load-bearing after all, but INDIRECTLY**: it is what
`shared-mime-info` keys on to classify the file, and GIO is what gearlever
asks. ⚠ `file --mime-type` disagrees and returns `application/x-pie-executable`
for **both** artefacts, so testing with `file(1)` would have said we fail.
Measured with the tool that actually decides:

```sh
gio info -a standard::content-type <artefact>
# standard::content-type: application/vnd.appimage      # ours
# standard::content-type: application/vnd.appimage      # the competitor's
```

### ⭐ What that adds up to

**We are compatible today**, and the one detector we fail is one the
competitor fails identically — `AM` finds both of us through its
`file … static` fallback rather than its string test, because the runtime is
packed and `grep` cannot see its own option name. ⚠ That is the field's norm,
not our defect, and it should not be "fixed" by unpacking the runtime.

⛔ **Two real gaps, both one line in `internal/bundle/appimage.go`:**

1. **`X-AppImage-Version=`** is read by two of the four managers and we emit
   none. The version is already known — it is in the closure's derivation.
2. **`.DirIcon` is absent when the closure carries no icon.** `jq`'s generated
   desktop entry says `Icon=jq` and no `jq.png` exists beside it, so the entry
   points at nothing. ⚠ Either emit a fallback icon or omit the `Icon=` key;
   a dangling reference is worse than an absent one.

### How each manager actually reads a bundle

| manager | route |
|---|---|
| `gearlever` | probes `dwarfsck`, then `dwarfsextract --pattern=**.png --pattern=**.svg --pattern=**.desktop --pattern=.DirIcon`; falls back to `7zz`, then `--appimage-extract`. ⚠ It calls those tools from the **host**, so our `lite` runtime dropping them costs nothing |
| `AM` | `--appimage-mount`, then copies `$MOUNT_POINT/*.desktop` and `$MOUNT_POINT/.DirIcon`, following symlinks up to ten deep, then hunts `usr/share`/`share` × `22x22 … 512x512` for the icon named by `Icon=` |
| `AppManager` | expects `$APP_ID.desktop` at the top level, appends `X-AppImage-Version=` if missing, runs `desktop-file-validate` |
| `soar` | reads the image directly with `squishy` — no mount, no extract — via `find_icon()`, `find_desktop()`, `find_appstream()`; writes `<pkg>.DirIcon`, sniffs PNG magic to choose the extension, symlinks |

⚠ **`soar` wants a third file this project does not produce**: AppStream
metadata under `usr/share/metainfo/`. The soarpkgs recipe creates that
directory explicitly, so the field treats it as part of the contract.
