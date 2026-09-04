# SUMMARY.md — the session of 2026-09-04b

⛔ **Overwritten every session.** The work order is
[`PROGRESS.md`](PROGRESS.md); the closed entries are
[`../HISTORY/entries/`](../HISTORY/entries/).

    SCOPE     Finish experiments/65- (T-080), then BY MECHANISM: T-088
              rung 1, T-089 rung 2, T-087 rungs 3+, T-090 rung 5, then
              T-084 / T-091 / T-092.
    RESULT    ⭐ SIX of the seven moved, five with numbers, and the corpus
              turned into a defect-finding instrument: FIVE of its zeros
              were run down to root cause, TWO were BUNDLER BUGS with
              shipped fixes and THREE were the instrument itself.
              ⭐ FOUR capability categories CLOSED at three subjects each.
              ⛔ T-080 IS STILL RUNNING and that is the honest state.
              ⛔ ELEVEN corrections. Six about instruments; four were
              defects in work written this same session.

## ⭐ What moved

| | before | after |
|---|---|---|
| **T-088** multi-entry dispatch | shipped, **never run** | ⭐ `experiments/68-`, `pass=24`, **twice**: a SECOND program out of a real bundle, by its own name, **11/11**, zero host objects |
| **T-089** the interposer's static row | ⛔ **NOT MEASURED** in a shipped design document | ⭐ measured, and it was **two mechanisms**; then a static subject moved the question **off the linker entirely** |
| **T-090** the sandbox | *"EPERM in the chroot bed"*, no cause | ⭐ the cause is **`chroot`**, isolated in three arms, and **`pivot_root` permits the call** |
| **T-084** the classifier's six hand copies | one known defect, and *"the error only runs one way"* | ⭐ `experiments/102-`, `pass=17`, **twice**: **2 behaviours** not six, a **SECOND** defect running the other way, and the **one** committed number it reaches |
| **T-091** GStreamer | *"we emit one of four"* | ⛔ the entry described the **wrong layer**; all four emitted, the scanner installed as a program — ⭐ now seen **firing on a real closure**, still **unmeasured** and it says why |
| **T-092** the `.env` coupling | a known divergence, no check | ⭐ one naming rule, the missing check, and a **gate that found two more** hand-slices on its first run |
| **T-087** rung 3 | pre-registered | ⛔ the **locale** half is **not measurable in this bed** — no environment has a non-C locale. ⭐ The mechanism claim holds on a different discriminator |
| the corpus | 1 of 26 rows | **20 of 26**, ⭐ **four categories CLOSED**, and five zeros run down to root cause |
| `references/` | 46 trees, text said 34 | **55**, and rungs 6 and 7 rewritten against what arrived |

## ⭐ The measurements, each with its verdict line

| | verdict | runs |
|---|---|---|
| `68-` multi-entry, arms S and B | `pass=24 fail=0 skip=0` | **two**, every cell identical |
| `69-` user namespaces | `pass=9 fail=0 skip=0` | **three** on `debian-12`; `pass=8 skip=1` on `alpine-3.22` |
| `100-` static payload, arms P, G, L | `pass=13 fail=3` | arm P **two**, arm G **two** |
| `102-` classifier equivalence | `pass=17 fail=0 skip=0` | **two**, identical |
| `101-` rung 3 | ⛔ **stopped, no result recorded** | the criterion cannot fire in this bed |
| `65-` the corpus | ⏳ **20 of 26** | resumable; a recorded row is never re-measured, and a row from a broken instrument is DELETED |

## ⭐ FOUR CAPABILITY CATEGORIES ARE CLOSED

⛔ **"Closed" means three subjects, simple → complex, each drawing a real
toplevel on a real X server seen from outside with `xwininfo`, on all eleven
environments.**

| category | subjects | pass | clean |
|---|---|---|---|
| **GTK 3** | `galculator`, `mousepad`, `geany` | **11/11** each | **11/11** each |
| **X11 / XCB** | `xeyes`, `xclock`, `xterm` | **11/11** each | 11, 11, ⭐ **4** |
| **OpenGL / EGL** | `eglinfo`, `glxgears`, `glmark2` | **11/11** each | **11/11** each |
| **Qt** | `qalculate-qt`, `keepassxc`, `qbittorrent` | **11/11** each | ⚠ **4**, 11, 11 |
| **Vulkan** | `vulkaninfo`, `vkcube`, `vkmark` | 11, 11, ⛔ **0** | 11/11 each |
| **SDL** | `dosbox`, `stella` (2 of 3) | **11/11** each | **11/11** each |

⭐ **`xterm`'s `4/11` is the one dirty count this corpus EXPECTS**, and it was
pre-registered as C5 before the corpus ever ran: xterm's job is to run the
user's **shell**, a host program, so a host libc enters by construction. ⛔
`qalculate-qt`'s `4/11` is **not** explained and is recorded as unexplained.
⛔ `vkmark`'s zero is the **bed**: it enumerates DRM devices before asking
Vulkan anything, and `/dev/dri` exists nowhere here. ⛔ **C3's limit stands**:
every GL and Vulkan row is a software rasteriser.

## ⛔ Four findings that changed what a question means

**1. The sandbox refusal is `chroot`'s, and there is a route.** `pgb rootfs run`
does *two* things, so the bed row could blame neither. Run apart, same rootfs,
same probe: the host permits all five namespaces; **`chroot` alone reproduces
the EPERM**; `unshare --mount` alone does not; and the same rootfs entered by
**`pivot_root` permits it**. ⛔ Not a licence to change the bed — isolation,
teardown and whether a browser then sandboxes are three further measurements.

**2. A static ELF is portable; a static application need not be.**
`lilipod` is genuinely static, and `pgb bundle appimage` **refuses** a
loader-less closure — so the interposer question never arises. Run raw: the
**ELF executes on 11 of 11**, the **application completes on 2**. Every failure
names *"failed to find dependency `getsubids`"*, a **host program**. ⭐ Nothing
a linker, loader or bundler does supplies another program.

**3. ⛔ "The classifier error only runs one way" was true of C25 and FALSE of
the copies.** Four documents repeated it. `experiments/102-` diffed the six
hand copies against the shared classifier on fixtures — **no bundle build** —
and found a **second** difference running the other way: five of them clear
their result set on the artefact's own `execve` **unconditionally**, so in
`tree` mode a **dirty row reads clean**. ⭐ Then arm S named the single
committed number it reaches — `experiments/90-`'s, because its test script runs
the artefact **twice**. ⛔ **That includes our own `0 of 11`, not only the
competitor's.**

**4. ⭐ THE ENTRY-POINT SHAPE PREDICTS THE RESULT — and it is THREE shapes.**
Read out of every subject's build log:

| shape | subjects | result |
|---|---|---|
| plain ELF | `dosbox`, `vkmark`, `xeyes`, … | ⭐ pass |
| ⭐ nixpkgs wrapper, **resolved** at build time | `mousepad`, `meld`, `flameshot` | ⭐ works — no shell left in the path |
| ⛔ generic **SCRIPT**, entry becomes **bash** | `xterm`, `glmark2` | ⛔ both `0/11` → ⭐ both `11/11` after C37 |

⭐ It is **not** "a script entry" — `meld` hits both wrapper handlers and
passes 11/11. It is **not** "a nixpkgs wrapper" — `mousepad` is one and passes.

## ⛔ ELEVEN corrections — and FOUR were in this session's own work

⭐ **None was found by reading the code that contained it.**

- **C29** three instrument defects in `68-`/`69-`: `env` running `--`, bash-only
  syntax in an `sh` harness, and ⛔ **a negative control that passed because it
  could see nothing**.
- **C30** `pgb-apprun.c`'s header stated the dispatch order wrongly and two
  documents copied it. ⭐ Pre-registered as a prediction about our own source.
- **C31** ⛔ **a comment broke `pgb-apprun.c`** — `${ARG0##*/}` contains `*/`.
  `buildStaticAppRun` **falls back to a shell AppRun** and says so only as a
  warning, so every multi-program bundle would have regained a host libc.
  `check.sh` check 10 now parses all 13 runtime `.c` files.
- **C32** *"copyLibraries flattens everything"* — read from **half a function**.
- **C33** three more controls that would have passed on a dead subject.
- ⛔ **`101-`'s filter matched the wrong delivery mode**, so four rows measured
  the filter. No result from it was recorded anywhere.
- ⛔ **T-084's step 1 said `mode` FIRST**, the one ordering that fails silently:
  a resumed `65-` calling the old way would report **every row zero host
  objects**. It goes last, with a default.
- **C34** the corpus's `cli` criterion was `exit 0 AND the assertion`, so a
  program that answered completely and exited 3 scored zero.
- **C35** `neovim`'s closure carries **glibc 2.26**; `ld.so` learned `--argv0`
  in 2.33. One old glibc, two unrelated-looking messages.
- ⭐ **C36** the corpus separator was `|` and the assertions **alternate**. It
  hid behind C34 and surfaced only because fixing C34 did **not** move the
  number.
- ⭐ **C37** a shell-wrapped nixpkgs application could not start at all: the
  farm's `bin` resolved to the raw payloads, whose `PT_INTERP` names a loader
  the bundle lacks.
- ⭐ **C38** the classifier copies differ from the shared one in **two** ways,
  and the second runs the **other** way. ⛔ Its own pre-registration was wrong
  too — *"two implementations"* measured by hashing **text**, which reads three;
  the behavioural count is two.
- ⭐ **C39** `media-1`'s assertion is `mpv [0-9]` and `mpv` prints
  `mpv v0.41.0`. **0/11** on a subject that answered completely.

## ⛔ THE CORPUS IS AN INSTRUMENT, NOT A SCOREBOARD

⛔ **Five rows read `0 of 11`. Not one was what it looked like, and THREE were
the instrument.**

| row | what the zero actually was |
|---|---|
| ⭐ `eglinfo` | **TWO instrument defects.** C34, fixed — and it **still read 0**, which forced the real search: C36, the separator. ⭐ **0/11 → 11/11** |
| ⭐ `vulkaninfo` | the same separator collision. ⭐ **0/11 → 11/11**, store paths **12 → 42** |
| ⭐ `xterm` | ⛔ **a real bundler bug, and a whole class** — C37. ⭐ **0/11 → 11/11** |
| ⭐ `glmark2` | ⛔ the **same** class, confirming C37 twice. ⭐ **0/11 → 11/11**, clean 11/11 |
| ⭐ `neovim` | ⛔ **real, and not ours** — the closure's own glibc 2.26. Now detected at build time with the exact runtime string |
| ⭐ `mpv` | **C39, the fourth instrument defect.** Row **DELETED** |

⚠ **Read a zero as a question, never as a result.** ⛔ And the pattern in the
three instrument defects is one thing: **a `cli` assertion is written from what
the program is expected to print and is never checked against what it does
print.** Nothing in the harness closes that loop.

## ⚠ What the next session inherits

    T-080  ⏳ 20 of 26. FINISH IT. RESUME.md has the PARALLEL recipe and
           the two-instance ceiling. Left: sdl-3, py-2, py-3,
           media-1 (after C39), field-1, field-2.
    ⛔ C39 IS OWED A ONE-CHARACTER FIX. `media-1`'s assertion must become
       `mpv v[0-9]`. It could not be made this session: 65- was executing,
       and editing a script that is being executed is catastrophic.
    ⛔ AND THE ROW-NOTE DEFECT IS STILL THERE. A note is the FIRST matching
       line cut to 70 chars — useless for a Python traceback, and it
       truncated both neovim's answer and vkmark's `[/dev/dri]`.
    T-084  ⭐ the cheap half is DONE. The expensive half is now ONE
           experiment, `90-`, not six — and `90-`'s number is OURS.
    T-091  ⭐ the scanner is seen installed on a REAL closure. ⛔ Still
           unmeasured, and `media-1` cannot be the row: `--version` never
           launches the scanner. Needs a subject that DECODES something.
    T-088  open: `--with-program` is still not exercised.
    T-089  open: a static application that needs a compiled-in PATH.
    ⛔ UNEXPLAINED, and recorded as unexplained: `flameshot` 0/11 (and it
       is NOT C37 — measured), `gearlever` UNRESOLVED, `qalculate-qt`'s
       4/11 clean count.
