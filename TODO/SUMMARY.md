# SUMMARY.md — the session of 2026-09-04b

⛔ **Overwritten every session.** The work order is
[`PROGRESS.md`](PROGRESS.md); the closed entries are
[`../HISTORY/entries/`](../HISTORY/entries/).

    SCOPE     Finish experiments/65- (T-080), then BY MECHANISM: T-088
              rung 1, T-089 rung 2, T-087 rungs 3+, T-090 rung 5, then
              T-084 / T-091 / T-092.
    RESULT    ⭐ FIVE of the seven moved, four with numbers, and the
              corpus turned into a defect-finding instrument: FOUR of its
              zeros were run down to root cause and TWO of them were
              BUNDLER BUGS with shipped fixes.
              ⛔ T-080 IS STILL RUNNING and that is the honest state.
              ⛔ NINE corrections. Five about instruments; three were
              defects in work written this same session.

## ⭐ What moved

| | before | after |
|---|---|---|
| **T-088** multi-entry dispatch | shipped, **never run** | ⭐ `experiments/68-`, `pass=24`, **twice**: a SECOND program out of a real bundle, by its own name, **11/11**, zero host objects |
| **T-089** the interposer's static row | ⛔ **NOT MEASURED** in a shipped design document | ⭐ measured, and it was **two mechanisms**; then a static subject moved the question **off the linker entirely** |
| **T-090** the sandbox | *"EPERM in the chroot bed"*, no cause | ⭐ the cause is **`chroot`**, isolated in three arms, and **`pivot_root` permits the call** |
| **T-091** GStreamer | *"we emit one of four"* | ⛔ the entry described the **wrong layer**; all four emitted, the scanner installed as a program. **Unmeasured, and it says so** |
| **T-092** the `.env` coupling | a known divergence, no check | ⭐ one naming rule, the missing check, and a **gate that found two more** hand-slices on its first run |
| **T-087** rung 3 | pre-registered | ⛔ **not measurable in this bed** — no environment has a non-C locale. Measured, not assumed |
| the corpus | 1 of 26 rows | **9 of 26**, and ⭐ **two categories closed**: GTK 3 and X11/XCB |
| `references/` | 46 trees, text said 34 | **55**, and rungs 6 and 7 rewritten against what arrived |

## ⭐ The measurements, each with its verdict line

| | verdict | runs |
|---|---|---|
| `68-` multi-entry, arms S and B | `pass=24 fail=0 skip=0` | **two**, every cell identical |
| `69-` user namespaces | `pass=9 fail=0 skip=0` | **three** on `debian-12`; `pass=8 skip=1` on `alpine-3.22` |
| `100-` static payload, arms P, G, L | `pass=13 fail=3` | arm P **two**, arm G **two** |
| `101-` rung 3 | ⛔ **stopped, no result recorded** | the criterion cannot fire in this bed |
| `65-` the corpus | ⏳ **9 of 26** | resumable; a recorded row is never re-measured |

## ⛔ Three findings that changed what a question means

**1. The sandbox refusal is `chroot`'s, and there is a route.** `pgb rootfs run`
does *two* things, so the bed row could blame neither. Run apart, same rootfs,
same probe: the host permits all five namespaces; **`chroot` alone reproduces
the EPERM**; `unshare --mount` alone does not; and the same rootfs entered by
**`pivot_root` permits it**, with `CLONE_NEWNS` too. ⛔ That is not a licence to
change the bed — isolation, teardown and whether a browser then sandboxes are
three further measurements.

**2. A static ELF is portable; a static application need not be.**
`lilipod` is genuinely static, and `pgb bundle appimage` **refuses** a
loader-less closure — so the interposer question never arises. Run raw: the
**ELF executes on 11 of 11**, the **application completes on 2**. Every failure
names *"failed to find dependency `getsubids`"*, a **host program**; exactly the
two rootfs that carry it are the two that completed. ⭐ Nothing a linker, loader
or bundler does supplies another program — and this is the class rung 7
predicted for container tooling, now measured on a rung-7 subject.

**3. The field did attempt distrobox, and calls it "VERY BROKEN".** This tree
said *"the row the field is on record failing"*. `pkgforge-dev/distrobox-AppImage`
ships two releases; its own repository description is `WIP (VERY BROKEN!)`. ⭐
Its build script names three costs in its own comments — it copies itself
**outside the FUSE mount** to work, `patchelf`s `crun`'s interpreter to a
`/tmp/<random>` path holding **the loader**, and needs `PATH_MAPPING`. Two of
those are things this project refuses in writing.

## ⛔ Seven corrections — and THREE were in this session's own work

⭐ **None was found by reading the code that contained it.**

- **C29** three instrument defects in `68-`/`69-`: `env` running `--`, bash-only
  syntax in an `sh` harness, and ⛔ **a negative control that passed because it
  could see nothing**.
- **C30** `pgb-apprun.c`'s header stated the dispatch order wrongly and two
  documents copied it. ⭐ Pre-registered as a prediction about our own source,
  and confirmed.
- **C31** ⛔ **a comment broke `pgb-apprun.c`** — `${ARG0##*/}` contains `*/`.
  Three things hid it: the runtime `.c` files are embedded strings compiled
  later, `buildStaticAppRun` **falls back to a shell AppRun** on failure, and it
  says so only as a warning. Every multi-program bundle would have regained a
  host libc. `check.sh` check 10 now parses all 13.
- **C32** *"copyLibraries flattens everything"* — read from **half a function**;
  it also carries `lib/` subdirectories whole.
- **C33** three more controls that would have passed on a dead subject.
- ⛔ **`101-`'s filter matched the wrong delivery mode** (`/tmp/.mount_` where
  uruntime extracts to `appimage_extracted_*`), so four rows measured the
  filter. No result from it was recorded anywhere.
- ⛔ **T-084's step 1 said `mode` FIRST**, which is the one ordering that fails
  silently: a resumed `65-` calling the old way would report **every row zero
  host objects**. It goes last, with a default.

## ⭐ The corpus stopped being a scoreboard and became an instrument

⛔ **Four rows read `0 of 11`. Not one was what it looked like.**

| row | what the zero actually was |
|---|---|
| ⭐ `eglinfo` | **TWO instrument defects.** C34: the `cli` criterion was `exit 0 AND the assertion`, and `eglinfo` exits 3 headless while answering completely. Fixed — and it **still read 0**, which forced the real search: C36, the corpus was `\|`-separated and this assertion **alternates**, so `cut` handed `grep` an unmatched `(`, gave `--extra` the word `Mesa`, and passed `softpipe)` as an argument. ⭐ **0/11 → 11/11** |
| ⭐ `vulkaninfo` | the same separator collision. ⭐ **0/11 → 11/11**, and its store paths went **12 → 42** because `--extra mesa` had been failing to resolve |
| ⭐ `xterm` | ⛔ **a real bundler bug, and a whole class.** Its nixpkgs `bin/xterm` is a **shell** wrapper that execs a dot-named target by store path; the target was never installed, and the farm's `bin` resolved to the **raw payloads**, whose `PT_INTERP` names a loader the bundle lacks. `execve` returned ENOENT **for the interpreter** and the shell printed it against the program. ⭐ **Fixed: xterm now DRAWS in 2 s** |
| ⭐ `neovim` | ⛔ **real, and not ours.** The closure carries **glibc 2.26**; sharun passes `--argv0`, which `ld.so` learned in **2.33**. The same old glibc explains the interposer's `dladdr/dlsym` warning — they lived in `libdl.so` until 2.34. Now detected at build time with the exact runtime string |

⭐ **EGL, OpenGL and Vulkan all reach 11 of 11**, and §0 had marked Vulkan
*"NOT MEASURED. NOT CLAIMED."* that morning. ⛔ C3's limit is untouched: every
one of those rows is a **software rasteriser**.

## ⚠ What the next session inherits

⭐ **The corpus is the unfinished half and it is resumable.** Two categories
are closed; the zeros so far are **subject-specific, not an instrument defect**,
and that was checked rather than assumed.

    T-080  finish 65-. ⭐ It can be run in PARALLEL -- RESUME.md has the
           recipe, and the machine is 99% idle while it runs.
    T-087  ⭐ all four named unknowns are RUN DOWN. Two were the
           instrument (C34, C36), one was a bundler bug with a shipped fix
           (C37, xterm), one is the closure's own glibc (C35, neovim).
           ⛔ helix and flameshot and glmark2 are still open.
    ⛔ AND `make` IS OWED. C37's fix is committed and NOT in ./pgb, because
       the corpus was running. After `make`: delete the `x11-3` row and
       re-measure it, and any other shell-wrapped subject.
    T-091  landed and UNMEASURED. A media subject whose host-object count
           says WHICH PROCESS it counted.
    T-084  now unblocked reasoning, still blocked on 65- finishing.
    T-088  open: `--with-program` is still not exercised.
