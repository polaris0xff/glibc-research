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

## ⭐ THE CORPUS IS COMPLETE — 26 OF 26 SUBJECTS, EVERY ROW A COUNT OUT OF ELEVEN

⭐ **2026-09-04c**, `evidence/65-capability-corpus/RESULT.txt` and one row file
per subject under `rows/`. ⛔ **No row is a blank and none is "not run"**: every
one of the twenty-six carries a count and, where that count is not eleven, the
reason it is not.

    subjects in the corpus                   26
    subjects that produced an artefact       26
    ⛔ UNRESOLVED (not a pass, not a fail)    0
    ⛔ INSTRUMENT errors                      0
    ⭐ subjects passing on all 11            21
    ⭐ subjects clean on all 11              24

    C6  control subjects MEASURED             3 of 3      ✅
    C6  controls at 11 of 11, as 64- had them 3 of 3      ✅
    C7  no subject failed its criterion's sanity check    ✅
    C8  every subject measured on all 11                  ✅

| category | subjects | verdict |
|---|---|---|
| **GTK 3** | ⭐ **3 of 3 — CLOSED** | `galculator`, `mousepad`, `geany`, all **11/11 pass and 11/11 clean** |
| ⭐ **X11 / XCB** | ⭐ **3 of 3 — CLOSED** | `xeyes` **11/11**, `xclock` **11/11**, `xterm` **11/11** once C37 was fixed. ⛔ `xterm`'s **clean** count is **4/11**, and that is **C5 coming true** — see below |
| ⭐ **OpenGL / EGL** | ⭐ **3 of 3 — CLOSED, all clean** | `eglinfo` **11/11**, `glxgears` **11/11**, `glmark2` **11/11** — **11/11 clean each** |
| ⭐ **Vulkan** | **3 of 3, and the third is the BED** | `vulkaninfo` **11/11**, `vkcube` **11/11** — both clean 11/11. ⭐ `vkmark` **0/11 because there is no GPU**: it dies with `directory iterator cannot open directory … [/dev/dri]`, and `/dev/dri` exists **nowhere** here. The bundle carries `lvp_icd` (lavapipe) and the other two subjects use it |
| ⭐ **Qt** | ⭐ **3 of 3 — CLOSED** | `qalculate-qt` **11/11** (clean **4/11**, unexplained and recorded as such), `keepassxc` **11/11** clean **11/11**, `qbittorrent` **11/11** clean **11/11** — 127, 101 and 120 store paths compiled in |
| ⭐ **SDL** | ⭐ **3 of 3 — CLOSED, all clean** | `dosbox` **11/11** (**181** store paths compiled in, 169 resolving — the largest closure in the corpus), `stella` **11/11** (179/167), `scummvm` **11/11** (180/168) |
| ⭐ **media / codecs** | **1 of 3 in, and it passes** | ⭐ `mpv` **11/11, clean 11/11**, 151 store paths compiled in — the row that read **0/11** until **C39**, a subject that had answered completely against an assertion that could not match it. ⛔ The row runs `--version`, so it does **not** close T-091: nothing decodes |
| ⭐ **Python GUI** | ⭐ **2 of 3 passing**, and the third is not ours | `meld` **11/11** clean 11/11 — the third C6 control. ⭐ `virt-manager` **11/11, clean 11/11**, 239 store paths compiled in — **0/11 → 11/11 by C41**, exactly as pre-registered. ⛔ `pdfarranger` **0/11**, and re-measured against the same fixed tool it does **not move**, also as pre-registered |
| **the field's recipes** | ⭐ **1 of 4 passing, and ALL FOUR are explained** | ⭐ `helix` **11/11 pass, 11/11 clean** — it was `0/11` and **silent**, and the cause was **C43**, a real bundler defect and a fourth entry-point shape. `neovim` **0/11** — the closure's own glibc 2.26 (C35), not ours. `flameshot` **0/11** — ⭐ **not a bundler failure**: a tray application with no toplevel, and no session bus in this bed. ⭐ `gearlever` — was **UNRESOLVED**, now **measured**: **C42** makes it build and **C41** gets it past its own imports, and it fails on all eleven with one reproducible line — `RuntimeError: could not create new GType: gearlever+preferences+Preferences (subclass of void)`, a libadwaita question **not established as ours**. **Clean 11/11** |

⭐ **THE THREE ZEROS THAT ARE NOT OURS, each measured rather than argued:**

| row | what the zero is |
|---|---|
| `vkmark` | ⛔ **the BED.** No `/dev/dri` on this machine or in any rootfs |
| `neovim` | ⛔ **the CLOSURE.** glibc 2.26; `ld.so` learned `--argv0` in 2.33 (C35) |
| ⭐ `flameshot` | ⛔ **the SUBJECT'S SHAPE.** Measured by hand 2026-09-04c: given a session bus it runs for 180 s and never exits, and the only window it puts on the server is a **3×3 `Qt Selection Owner`**. It is a tray/daemon screenshot tool — `flameshot` with no subcommand draws nothing by design. ⭐ So Qt initialised, connected to X and owned a selection out of the bundle; a `gui` row demanding a toplevel ≥50×50 cannot pass on it. ⚠ And it needs a **session DBus**, which no rootfs here provides |

⚠ **`flameshot` is also where the ≥50×50 rule earned its keep**: a crude
"is there any window" check counted that 3×3 selection owner and would have
scored the row green.

### ⭐ THE TWO PYTHON ZEROS ARE DIFFERENT FAILURES, and the row note is what says so

⛔ **Until 2026-09-04c both notes read `Traceback (most recent call last):`** —
the harness took the FIRST matching line of the stderr and cut it to 70
characters, which for a Python traceback is always that sentence and never the
cause (**C40**). The note is now the LAST matching line at 180 characters, and
the two rows stop looking alike:

| row | the note | what it is | predicted | ⭐ measured |
|---|---|---|---|---|
| `py-3` `virt-manager` | `ModuleNotFoundError: No module named 'virtManager'` | ⭐ **C41**, byte for byte: the interposer defined `stat`/`lstat`/`fstatat` and not the `64` names a nixpkgs Python imports, so a compiled-in store path was rewritten when OPENED and not when STATTED — and Python's import finder stats before it imports | **moves** | ⭐ **11/11, clean 11/11** |
| `py-2` `pdfarranger` | `FileNotFoundError: … '/usr/local/share/pdfarranger/pdfarranger.ui'` | ⛔ **a different class, and it is the field's own "Garbage — GTK" row arriving on OUR pipeline.** `/usr/local`, not `/nix/store` — so `pgb-storefix.c`'s `fix()` returns it unchanged by construction, and no interposer change can reach it | **does not move** | ⛔ **0/11, same line** |

⭐ **BOTH HALVES OF THE PREDICTION HELD**, and it was committed before the run
that settled it. ⚠ The point is not that the guesses were right — it is that
until the note was fixed the two rows were *indistinguishable*, so no
prediction about either was possible at all.

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

## ⭐ THE ENTRY-POINT SHAPE PREDICTS THE RESULT — and there are FOUR

Every subject's build log names its entry shape. ⚠ The count went two → three
→ **four**, each time because a zero was run down rather than assumed, and the
fourth was found on 2026-09-04c:

| shape | what the build log says | subjects | result |
|---|---|---|---|
| **plain ELF** | `entry …/bin/<name>` | `dosbox`, `vkmark`, `xeyes`, … | ⭐ pass. The one failure is `vkmark`, and that is the **missing GPU** |
| ⭐ **nixpkgs wrapper, RESOLVED at build time** | `bin/<n> is a nixpkgs wrapper -> .<n>-wrapped`, and the entry becomes the **dot-named ELF** | `mousepad`, `meld`, `flameshot` | ⭐ **works** — the wrapper is consumed, its environment lifted into `.env`, and no shell is left in the path |
| ⛔ **generic SCRIPT** | `bin/<n> is a SCRIPT; its entry point is bash + the script itself`, and the entry becomes **bash** | `xterm`, `glmark2` | ⛔ **both were `0/11`** → ⭐ **both `11/11` after C37** |
| ⛔ **nixpkgs wrapper, NOT resolved** — its target is a **symlink into another store path** | ⛔ **the arrow line is ABSENT**; only `wrapper env N variable(s) lifted` appears | `helix` | ⛔ **`0/11`, silently** — exit 255 with not one byte of output → ⭐ `helix 25.07.1`, exit 0 after **C43** |

⭐ **THE PREDICTOR IS THE THIRD SHAPE AND NOTHING ELSE.** It is not "a script
entry" — `meld` hits **both** wrapper handlers (`nixpkgs wrapper -> .meld-wrapped`,
then `.meld-wrapped is a SCRIPT; its entry point is python3`) and passes
**11/11**. It is not "a nixpkgs wrapper" — `mousepad` is one and passes
**11/11**. It is **bash running a generic script that `exec`s a dot-named ELF
by absolute store path**, which is exactly what C37 describes.

⭐ **AND THAT SETTLED A SUSPICION ABOUT `flameshot`.** Its build log says
*`bin/flameshot is a nixpkgs wrapper -> .flameshot-wrapped`* — the **resolved**
shape, the same handler `mousepad` passes through. ⛔ So its `0/11` is **not**
C37, and it is not the bundler at all: `flameshot` is a tray application that
draws no toplevel, measured by hand 2026-09-04c.

⭐ **THE ARROW IS THE DIAGNOSTIC, AND ITS ABSENCE IS THE FOURTH SHAPE.**
`helix`'s log carries `wrapper env 1 variable(s) lifted` and **no arrow**: the
resolver tested the wrapper's target with `os.Stat`, which follows an absolute
`/nix/store` symlink against the **host** root, so `.hx-wrapped ->
/nix/store/…-helix-unwrapped/bin/hx` read as ENOENT and the makeCWrapper ELF
was installed instead of the program. `docs/history/corrections.md` C43 — and
it is the **same** root cause as C42's loader, in a different code path.

⭐ **AND THE FIX IS NOW CONFIRMED IN THE CORPUS ITSELF, ON BOTH SUBJECTS.**
It was first measured by hand — `xterm` drawing in 2 s, `glmark2` drawing in
2 s and benchmarking at **360 FPS**. Then `./pgb` was rebuilt, the affected
rows deleted, and both re-measured:

| | before C37 | after |
|---|---|---|
| `x11-3` `xterm` | **0 / 11** | ⭐ **11 / 11** (clean **4/11** — that is C5, below) |
| `gl-3` `glmark2` | **0 / 11** | ⭐ **11 / 11**, clean **11 / 11** |

⚠ **`meld` is the counter-example that keeps this honest**: it is a script
entry and it passes **11/11**. `meld`'s interpreter *reads* its script;
nothing dot-named is exec'd.

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
| ⭐ **XCB / X11 client stack** | ✅ **CLOSED, THREE SUBJECTS, ALL PASSING.** `xeyes` **11/11** clean **11/11** (12 paths in, 11 resolving) — a pure Xlib/XCB client with no toolkit above it; `xclock` **11/11** clean **11/11** (23/22); `xterm` **11/11** (26/25) once C37 was fixed. ⛔ **`xterm`'s clean count is `4/11`, and that is C5 CONFIRMED** — pre-registered before the corpus ever ran: its job is to run the user's **shell**, a host program, so a host libc enters the process by construction. It is the one row here where a dirty count is the right answer | `65-` `x11-1..3` |
| ⭐ **EGL / OpenGL** | ✅ **CLOSED, THREE SUBJECTS, ALL PASSING AND ALL CLEAN.** `eglinfo` **11/11** enumerates EGL configs on every environment, naming `llvmpipe`; `glxgears` **11/11** draws a real window; `glmark2` **11/11** — a full benchmark suite, and a **shell-wrapper** entry that could not start before C37. The bundle carries mesa and points libglvnd at itself; `85-`'s negative control (`--no-gl`) produces no vendor string on any row. ⛔ Every row is a **software rasteriser** — C3, and T-059 owns hardware | `65-` `gl-1..3`; `85-` |
| ⭐ **Vulkan** | ✅ **THREE SUBJECTS IN; the two that can run here pass and are clean.** `vulkaninfo --summary` **11/11** enumerates a device on every environment (`apiVersion 1.4.354`, `deviceName llvmpipe (LLVM 21.1.8, 256 bits)`); `vkcube` **11/11**. ⛔ `vkmark` **0/11 and it is the BED**: it enumerates DRM devices before asking Vulkan anything, and `/dev/dri` exists **nowhere** here — measured, on the host and in all eleven rootfs. ⛔ **C3's limit is the whole point**: a **software rasteriser**, saying nothing about a real GPU or NVIDIA | `65-` `vulkan-1..3` |
| **NVIDIA** | ⛔ **NOT MEASURED, and not bundled by design.** The driver is taken from the HOST; `design/host-fallback.md` governs it. T-059 owns the hardware | — |
| ⭐ **SDL** | ✅ **CLOSED, THREE SUBJECTS, ALL PASSING AND ALL CLEAN.** `dosbox` **11/11**, `stella` **11/11**, `scummvm` **11/11**, each with **zero host shared objects on 11 of 11**, out of the three largest closures measured here — **181**, 179 and 180 store paths compiled in. ⛔ It was a *hypothesis* graded *Excellent* by the field and **NOT RUN through our pipeline** until these three rows | §1; `65-` `sdl-1..3` |
| ⭐ **Qt** | ✅ **CLOSED, THREE SUBJECTS, ALL PASSING.** `qalculate-qt` **11/11** (127 paths in, 124 resolving), `keepassxc` **11/11** clean **11/11** (101/98), `qbittorrent` **11/11** clean **11/11** (120/117). ⚠ `qalculate-qt`'s clean count is **4/11** and is **unexplained** — it is *not* a C37 regression: `galculator` rebuilt with the same `pgb` traces to zero host objects | `65-` `qt-1..3` |
| ⭐ **Python GUI** | ✅ **MEASURED, AND IT WORKS.** `meld` — Python 3 + GTK 3 through PyGObject — draws a real toplevel window on **11 of 11** with **zero host shared objects**, on eleven distributions of which four ship no glibc and none ship Python or GTK, and it is confirmed **twice**, by two experiments. ⛔ It produced **no artefact at all** before T-081. ⏳ `pdfarranger` and `virt-manager` are queued | `64-` arm P; `65-` `py-1` |
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

## ⭐ THEIR HARD CASES, TAKEN ONE BY ONE — 2026-09-04c

⛔ **Operator, 2026-09-04c**: *"Take on their 'can't fix', 'unfixable', 'hard'
etc seriously one by one; the more the bundler is exercised the better it
gets."* ⭐ This section is that pass, over the **vendored** `FAQ.md`,
`HALL-OF-FAME.md` and the **825 issues / 1,000 comments** in `api/`. Each row
quotes their claim, says what **we** measure against it, and says plainly when
we have measured nothing.

⚠ **Their claims are about a different pipeline** — a distro closure, sharun,
`quick-sharun`'s string patching — so an answer of ours is a different
artefact, not a refutation. What makes the exercise worth doing is that
several of their rows land on our own corpus rows independently.

### ⛔ *"Why not statically link everything? … a fully static binary is a very bad idea"*

⭐ **This is the FAQ answering our project's thesis directly**, so it is worth
taking apart claim by claim.

| their claim | what we measure |
|---|---|
| *"That is super hard, some libraries are not meant to be statically linked … a ton of patches are needed"* | ⭐ **Ten POCs, stock tarballs, stock `./configure`, NO source patches** — up to Qt 6.11.1 and ffmpeg+MLT. ⚠ Two pass *configuration*, not patches (`docs/AGENTS.md` §12). It is hard; it is not "a ton of patches" |
| *"we are not able to `dlopen` any library from the host, even optional ones"* | ⭐ **FALSIFIED for our artefact, and this is route D**: `pgb build --host-dlopen` compiles an ELF loader into the static binary. `experiments/76-`: **11 of 11**, zero host objects, a **real host `.so` on 7 of 7 glibc rows**. `experiments/93-`: **882 of 1,527** host objects on the build host load |
| *"It means goodbye to the proprietary nvidia driver"* | ⚠ **We agree, and say so**: the driver is HOST-always and never bundled (`design/host-fallback.md`), and every GL row here is a software rasteriser. T-059 owns it |
| ⛔ *"you are no longer able to use vulkan layers like mangohud"* | ⛔ **NOT MEASURED.** A Vulkan layer is a host `.so` loaded at run time, which is exactly what `--host-dlopen` is for — but nobody has pointed it at a layer. **A named next experiment, not an answer** |
| *"you are forever stuck with the version of MESA that was statically linked"* | ⚠ **True of a static binary and we do not dispute it.** ⭐ Our *bundle* has the host-fallback classes instead (`design/host-fallback.md`), which is the same lever as their `SHARUN_ALLOW_SYS_VKICD` |

### ⛔ *"Why not use solo or detour?"* — and their own UPDATE answers it

⭐ **The FAQ dismisses the route this project took, and then its own UPDATE
adopts it**: *"We now have a similar feature via **cross-libc-dlopen**, enabled
via `USE_HOST_DRIVERS_EXPERIMENTAL=1`"* — the repository we have vendored, and
whose approach `research/solo.md` compares against ours.

| their objection | ours |
|---|---|
| *"What happens if my application needs OpenGL 4.6 but the host Mesa only supports 4.5?"* | ⚠ **Real, and it is an argument for bundling Mesa rather than against a loader.** We bundle it; `--host-dlopen` is opt-in per call site |
| *"if I end up statically linking LLVM and the host's Mesa links to a different version"* | ⚠ Real. ⛔ **Not measured here** — no row has loaded a host Mesa into a binary carrying its own LLVM |
| ⭐ *"it seems none of the solutions implement `dlmopen`, so you are likely to run into a lot of symbol collisions"* | ⭐ **Ours does not have that shape.** `tool/runtime/pgb-elfload.c` resolves each loaded object's undefined symbols **itself**, against the static glibc already linked in; nothing is added to a global search scope, so a second object cannot bind to the first by accident. ⛔ **That is read off the design, not measured** — the experiment is two host objects defining the same symbol name, and it has not been run |

⚠ **And their three conditions for enabling it are worth copying rather than
arguing with**: no recent OpenGL requirement, no hard Vulkan dependency, and a
software-renderer fallback.

### ⛔ *"Horrible — Glibc"*, and one of the three is a latent bug we checked for

| their claim | ours |
|---|---|
| *"`LOCPATH` … doesn't work with locale archives"* | ⚠ Their problem is a **relocatable glibc**; ours is a **static** one. `--embed-locale` writes C.UTF-8 only when the host cannot answer, 11 of 11 |
| *"we also have to set `GCONV_PATH` and good luck figuring out which gconv plugin your app needs … when the plugin is missing there is no error"* | ⭐ **The static path does not have this problem at all**: `--wrap=iconv_open` onto static GNU libiconv, so no gconv module is ever looked for. It is the one place our answer is structurally better than theirs, and it costs ~1.2 MiB on a binary that calls `iconv` |
| ⛔ *"We also have to patch `ld-linux.so` to prevent it from reading `/etc/ld.so.cache` because otherwise it would segfault instantly on some systems"* | ⭐ **CHECKED, 2026-09-04c.** Traced on our `helix` artefact: `/etc/ld.so.cache` is opened **exactly once in the whole run**, and **not by the bundled loader** — it happens in sharun's own startup, before the loader is even read, and the loader is entered without a visible `execve` (userland-execve). ⚠ **One subject, one environment, and it does not prove the class is absent** — it says the read we do have is not the one their fix is about |

### ⛔ *"Utter Garbage — Python"* — and their first bullet is our own corpus row

⭐ **Their claim**: *"python apps are often written with a ton of hardcoded
paths, **even more than GTK apps**, so a lot of manual patches are needed."*

⭐ **Reached independently on our pipeline the same day**: `experiments/65-`
`py-2` `pdfarranger` fails on **all eleven** with

    FileNotFoundError: … '/usr/local/share/pdfarranger/pdfarranger.ui'

⛔ **And their own mechanism does not cover it either, in their own words**
(issue **#228**): *"we check for hardcoded paths in binaries and libraries and
patch them away for a random path in `/tmp`, and this only works when you
install the app to `/usr` … Very ideally apps should be installed to `/usr`
and not `/usr/local`."* ⭐ `pdfarranger`'s path is `/usr/local/share/…`.

⚠ **So the honest statement is that neither pipeline answers this row today**,
and the two routes are different: theirs patches the ELF's strings to a
**random `/tmp` path**, which [`../design/store-paths.md`](../design/store-paths.md)
§2 refuses on security grounds in writing; ours rewrites at the syscall and
only for `/nix/store`.

⛔ **AND ONE MORE OF THEIRS IS A LATENT ROW OF OURS.** Issue **#4**: *"the
ctypes library runs `/sbin/ldconfig -p` and this fails in alpine linux since it
doesn't support the `-p` flag … that stops any further lookup."* ⚠ Our two
passing Python subjects (`meld`, `virt-manager`) are **11/11 including the
three Alpines**, so it did not bite them — but nobody has checked whether they
call `ctypes.util.find_library` at all. **Not measured; a cheap next check.**

### ⭐ *"if the application interacts with executables outside the appimage, there is no way to make them use the libraries in the appimage"*

Issue **#171**, and they conclude *"for those cases it is better to just use a
container like RunImage."*

⭐ **We reached the same class twice, from two directions, before reading
this:**

| ours | |
|---|---|
| `experiments/65-` `x11-3` `xterm` | **11/11 pass, 4/11 clean** — and the dirty count was **pre-registered as C5**: xterm's job is to run the user's **shell**, a host program, so a host libc enters by construction |
| `experiments/100-` arm L `lilipod` | the static ELF **executes on 11 of 11**; the **application completes on 2**, every failure naming `getsubids`, a host **program** |

⭐ **Where we differ is what it means for a STATIC binary.** Their sentence is
about libraries the child process loads; a `pgb` static binary has none to
share either way. ⛔ What neither of us answers is a missing host **program** —
and that is the sharper statement: *static linking answers "will this binary
start"; it says nothing about "will this program find the tools it shells out
to."*

### ⭐⭐ THEIR OWN "CAN'T MAKE THEM WORK" LIST NAMES ONE DOMINANT CAUSE — AND IT IS THE CELL OUR MECHANISM OWNS

⭐ **Issue #460, open, titled *"appimages that can't make them work"*** — the
field's own list of applications they gave up on, with a reason beside each.
Read 2026-09-04c. ⛔ **The reasons repeat:**

| their subject | their reason, quoted |
|---|---|
| `KeeperFX` | *"if extracted works … need to set `INSTALL_PATH` inside `keeperfx.cfg`"* |
| `Myth-II` | *"similar to keeperfx, only works when extracted"* |
| `Rigs-of-Rods`, `OpenTESArena` | *"has a path line inside `plugins.cfg` that need to set"* |
| `FEX-Emu` | *"issues when loading the x86_64 rootfs"* |
| `FS-UAE` | *"complaining about `xcb` Qt"* |
| `NetHack-X11` | *"don't know how to dir inside `APPDIR/bin`"* |

⭐ **Four of the six are ONE cause: an absolute path that lives in a CONFIG
FILE, not in the ELF.** `quick-sharun`'s mechanism is a **build-time patch of
the strings inside binaries and libraries** (their #228, #330), so a path that
only ever exists in `keeperfx.cfg` or `plugins.cfg` is invisible to it.

⛔ **AND THAT IS THE DIFFERENCE BETWEEN THE TWO MECHANISMS, STATED AS A GRID
RATHER THAN AS A BOAST:**

| | the path is `/nix/store/…` | the path is `/usr/…`, `/usr/local/…`, `/etc/…` |
|---|---|---|
| **compiled into the ELF's strings** | ⭐ both reach it — theirs by patching, ours by rewriting the syscall | ⚠ **theirs**, not ours: `pgb-storefix.c`'s `fix()` returns anything not starting `/nix/store/` unchanged |
| ⭐ **in a config file, or assembled at run time** | ⭐ **OURS, and not theirs** — the interposer sees the path the program actually `open`s and does not care where the string came from | ⛔ **NEITHER**, and this is the cell that holds `pdfarranger` |

⚠ **The claim is about the mechanisms, not about those six subjects**: nobody
has put `KeeperFX` or `OpenTESArena` through `pgb bundle appimage`, and a
nixpkgs build of one is not the same artefact they were packing. ⭐ **What is
established is the shape**: a run-time rewrite at the syscall is reached by a
path from *any* source, and a build-time string patch is not. **The experiment
that would settle it** is a subject whose only compiled-in-free absolute store
path lives in a data file — and none of our 26 is one.

⭐ **AND THE BOTTOM-RIGHT CELL IS WHERE `flatimage` WINS**, which is the same
conclusion this page's runtime-projects table reached from the other side: a
portable **root** serves a program with any absolute path from any source.

### ⛔ Two more of their open issues land on classes we measured this session

| theirs | ours |
|---|---|
| **#110** *"What to do with applications with hard dependency on portals?"* — and their own answer in the thread is *"I believe there's nothing we can do about this beside convincing upstream"* | ⚠ **The same shape as our DBus row**: a portal is a **host D-Bus service**, and a bundle cannot ship one any more than it can ship `getsubids`. ⭐ Ours is the more general statement, reached on `lilipod` and `xterm`: *what a bundle cannot supply is another **process**, not another library* |
| **#664** `kitty`: *"`can't find '__main__' module` … a relative path resolution issue when kitty tries to execute its Python components"* | ⭐ **Byte-for-byte the family of C41** — a Python import failing inside a bundle because a path did not resolve. ⚠ Theirs is a squashfs AppImage and a different pipeline, so it is not the same bug; ⭐ but our version of it had a cause (`stat64` uninterposed) and a fix, and that is worth offering rather than filing |

### ⚠ The rows we have nothing to say about yet

| theirs | why it is open for us |
|---|---|
| *"Once `HOME` gets changed there is no way to roll it back"* (#12) | ⚠ We have an interposer and it does **not** touch `HOME`. Whether it should is undecided, and no row needs it yet |
| **WebKit** *"hardcoded to load some binaries in `/usr/lib` … no way to override"* | ⛔ the same `/usr` class as `pdfarranger`; no WebKit subject in the corpus |
| **p11-kit** *"You need to recompile the library to enable environment variables"* | ⛔ a compiled-in path with no variable. Ours answers it **only if** it is a `/nix/store` path |
| **JACK2** *"needs matching versions between server and client"* | ⛔ a host-daemon protocol version; nothing a bundler or a linker reaches |
| *"impossible to call it with the dynamic linker directly"* (#189, a `bun` binary) | ⚠ the same family as **C35** — our `neovim` row, where the closure's glibc 2.26 `ld.so` rejects `--argv0`. Theirs is a payload that cannot be *started* by the loader at all |
| *"making PGO builds in the CI is basically impossible"* (#14) | ⚠ this is their `OPTIMIZE_LAUNCH` lever, which we have never tried — **T-066** |

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
