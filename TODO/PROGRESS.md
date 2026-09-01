# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log
and the entries.

    STATE     2026-09-01d
    COUNTS    33 entries, 15 open, 18 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, NINE POCs
              CI: GREEN, 15 jobs, and it asserts criterion 2
              throughput: glibc 4.53 ns/op vs musl 584.71 (malloc, 4 threads)
    NEW       ⭐ Qt 6.11.1 STATIC on 11 of 11 with zero host objects
              ⭐ the bundled GL stack on 11 of 11, with a control
              ⭐ our bundler measured against a hand-built Anylinux AppImage
              ⭐ 8 of 9 issues on the acceptance bar are now closed

## ⛔ THE PREVIOUS SESSION'S STOP CONDITION IS DISCHARGED

All four required POCs are closed, each with its `Prove` command run and the
output in its entry.

| # | required | closed | evidence |
|---|---|---|---|
| 1 | `poc/90-qt` — a Qt 6 program, static, 11/11 or the rung recorded | ✅ **11 of 11, zero host objects** | `evidence/poc/90-qt/RESULT.txt` — 19 assertions, 0 fail |
| 2 | `experiments/85-opengl` — the bundled GL stack on all eleven | ✅ **Mesa/swrast on 11 of 11**, `--no-gl` control produces none | `evidence/85-opengl/RESULT.txt` — 7 assertions, 0 fail |
| 3 | `experiments/86-bundler-vs-anylinux` — ours against a hand-built one | ✅ **both 11 of 11, zero host objects; ours 3.05× the size** | `evidence/86-bundler-vs-anylinux/RESULT.txt` — 7 assertions, 0 fail |
| 4 | `poc/20` + `poc/30` reruns — `--embed-terminfo`, `--embed-cacert` | ✅ **11 of 11 each** | 12 assertions each, 0 fail |

Entries closed: **T-032**, **T-052**, T-054's **rung 1**, T-057's item 5.

## ⭐ The operator's three goals

Quoted, because the framing is load-bearing:

> *"1. Make the 'universal' builder true via pgb + nix. 2. make the 'universal'
> bundler true via a modern, updated, maintained 'nixappimage' descendant that
> uses or rather reimplements many of the anylinux tooling, iterating/improving
> them, and debloating nixappimages, correctly packing them, and also solving
> the opengl problem ... 3. poc a kdenlive static (exhaust all resources), if
> impossible, pivot to kdenlive.nixappimage, but it must be smaller, load
> faster, run faster than pkgforge-dev/kdenlive-AppImage-Enhanced."*

| goal | entries | where it stands |
|---|---|---|
| 1. the builder | T-050, T-051, T-012 | ⚠ **working**: six packages built static from nixpkgs plans, two verified 11/11 — and now **Qt 6.11.1**, whose source came from a nixpkgs plan and whose build is 1283 targets |
| 2. the bundler | T-057, T-052, T-053 | ⚠ **measured against the field for the first time.** ⭐ **T-052 CLOSED**: the bundled GL stack reaches `Mesa Project`/`swrast` on all eleven with zero host objects, against a `--no-gl` control that reaches nothing anywhere. ⭐ **T-057's comparison exists**: 3.05× the size, ~1.9× cold start, ~1.4× warm start, and identical on runs-everywhere and loads-nothing. Debloat, wrapper scripts and lib32 are still open |
| 3. kdenlive | T-054, T-055 | ⭐ **rung 1 climbed, and it was the rung everyone assumed was the wall.** A static Qt 6 widget program runs on 11 of 11. Rungs 2 (a real display / xcb), 3 (KF6) and 4 (kdenlive) are untouched |

## ⭐ The headline: 8 of 9 issues on the operator's bar are closed

`docs/REQUIREMENTS.md` part 2 enumerates nine issues a static glibc binary has.
It read **six closed, three open**. It now reads **eight closed, one open**:

    NSS ✅   gconv/iconv ✅   locale ✅   networking/DNS ✅   own plugins ✅
    C++ unwinding ✅   CA bundle ✅ NEW   terminfo ✅ NEW
    host plugins ⛔  <- the only one left, and the hardest of the nine

⛔ **Being last does not make it small.** Four routes, none exhausted;
`docs/AGENTS.md` §13 item 4, and T-033 is the best-evidenced.

## What this session did

### 1. Qt 6, static, on eleven distributions

⛔ **Nobody had attempted it.** `poc/80-mlt` wrote *"Qt 6 / KDE Frameworks ⛔
NOT ATTEMPTED — the next rung"* and `grep -rn Qt poc/ experiments/` found four
lines, all of them that POC's own prose. There was no error, no log and no rung
that stopped.

Source from `pgb nix plan/fetch qt6.qtbase` — signature and NarHash checked
from cache.nixos.org — as an ordinary tarball, **stock, with none of nixpkgs'
eleven patches applied**: every one of them is about finding things on disk at
run time, which a static build with compiled-in plugins does not do.

```
libQt6Core.a 23.0 MB  Gui 19.6 MB  Widgets 22.3 MB   and NOT ONE shared object
five static plugin archives: qoffscreen qminimal qjpeg qico qgif
the probe: 28,123,352 bytes, no PT_INTERP, 0 DT_NEEDED
11 of 11 pass; host shared objects loaded: none, on every row
```

The functional test is not `--version`: the QPA plugin resolving to
`offscreen` from its own compiled-in default, a UTF-8 round trip over CJK plus
an astral-plane codepoint, `QLocale(de_DE)` formatting 1234.5 as `1.234,5` out
of Qt's compiled-in CLDR, `QPainter` filling a `QImage` with the pixel read
back, `QWidget::grab()` rendering a 120×90 widget with the pixel read back,
`QFile` I/O, a Latin-1 `QStringConverter` round trip, and `QTimer` driving
`QApplication::exec()` to a clean return.

⭐ **And a negative control**: asking for a QPA plugin that is **not** compiled
in aborts on all eleven and pulls in **no host shared object** on any of them.
A static Qt does not fall back to the host's plugin directory, which is what
makes the clean rows mean something.

### 2. The GL stack, with the control that makes it a measurement

Arm A (bundled mesa) reports `EGL vendor string: Mesa Project`,
`EGL driver name: swrast` on **11 of 11** with zero host objects. Arm B, the
identical closure built `--no-gl`, reports **no vendor anywhere** — its EGL
client extensions string is empty, because libglvnd with no vendor does not
know the surfaceless platform extension exists.

⚠ No GPU is present. Every row is software rasterisation, nothing is drawn to
a screen, and NVIDIA is untouched. **T-059** carries that half so it is not
closed by silence.

### 3. Our bundler against a hand-built one

`sh tool/nix-appimage.sh jq` against the Anylinux flow's own documented route
on Arch, same release (jq 1.8.2), all eleven:

| | ours | hand-built |
|---|---|---|
| size | 12,230,824 B | 4,006,946 B |
| cold start | 162–198 ms | 79–107 ms |
| warm start | 11–22 ms | 9–16 ms |
| runs / host objects | 11 of 11 / **0** | 11 of 11 / **0** |

### 4. Five defects, every one found by building above the current class

⛔ **`pgb`'s CPU baseline silently overrode every project's own `-march`.**
The wrappers ran `exec "$REAL" "$@" $CF`, so pgb's `-march=x86-64` landed
*after* the caller's argv and gcc takes the last one. Qt's intrinsics probe is
compiled `-march=cannonlake`; every AVX-512 intrinsic in it failed and
configure stopped with *"x86 intrinsics support missing. Check your compiler
settings."* ⭐ **The compiler settings were pgb's.** Every codebase that
compiles one translation unit per ISA level behind a runtime CPU check —
ffmpeg, mesa, x264, zlib-ng, glib — has this shape. Compile flags now lead;
`-march=native` is rewritten in the caller's own argv so the portability
guarantee is kept. `docs/history/corrections.md` C15.

⛔ **Two `pgb build`s share one wrapper directory.** `$PGB_STATE` is
bind-mounted *into* the build environment, and `make_wrappers` opened with
`rm -rf` on it. Half fixed (atomic rename); the option-dependent half is
**T-058**.

⛔ **The bundler packed the wrong program.** `--name eglinfo-nogl` on a package
with no such binary fell back to the first thing in `bin/` — `quadstrip-flat` —
and said so in one line among eleven. Now refused, with two selftest cases
keeping the derived-name fallback and the explicit-name refusal apart.

⛔ **The bundler read nixpkgs' `out` output**, where a multi-output package
keeps its executables in `bin`. `sh tool/nix-appimage.sh jq` failed with
`no entry point in ...-jq-1.8.2/bin`, which reads like a broken package and is
a wrong output. Output selection is now `bin` → `out` → the single output.

⛔ **`tool/runtime/pgb-cacert.c` was missing `<stdio.h>`**, so every
`--embed-cacert` build warned on an implicit `snprintf`. `pgb-locale.c` carries
the same include with a note saying the same mistake was fixed *there* first.

### 5. Two instrument defects, both in this session's own work

⛔ **86-'s startup column measured the instrument.** Timing one chroot enter
per invocation and reaping after each kills uruntime's dwarfs mount, so every
run paid a cold mount and both arms read ~14,500 ms. The real figures are
162 ms cold and **17 ms** warm.

⛔ **Running two things on the bed at once corrupted a row.** `poc/30-curl`'s
voidlinux row came back `SIG9` because `experiments/85-` was reaping the same
rootfs. Nothing was wrong with the binary. ⭐ **`RULES.md` now carries the
rule**, and it was learned by breaking it twice.

⚠ **And two assertion defects in `poc/90-qt` itself**, both caught before the
evidence was committed: the plugin archive was looked for at nixpkgs'
`INSTALL_PLUGINSDIR` rather than Qt's own default, reporting a FAILURE for a
plugin that was built correctly; and the "no shared libraries" check was scoped
to `libQt6*.so`, which would have passed a Qt whose *plugins* were still
shared — the half that POC is about.

## In progress

Nothing half-written. See `RESUME.md`.

## ⭐ Work order for the next session

⛔ **The stop condition above is discharged, so this is a real work order
again**, in the order `INDEX.md`'s argument implies:

    T-055   the kdenlive bundle comparison -- UNBLOCKED now: T-054 rung 1 and
            T-052 both answered, which is exactly what it was waiting on
    T-054   rungs 2-4: Qt against a real display (xcb, and its plugin under
            --wrap-dlopen), then KF6, then kdenlive
    T-050   finish the no-nix route; T-051 the no-root host
    T-058   S, and it is the one blocking parallel work on a 4-core machine
    T-057   the debloat, with 85-'s 95 MiB of mesa as the number to beat
    then P2 by category

⚠ **T-054 and T-055 are the same question from two sides**, and the operator's
goal 3 says to try static first and pivot only if it will not reach. Rung 1
says it might.

## Open questions for the operator

⭐ **None blocking.** Everything asked this session is answered above or
carried as an entry.

1. ⭐ **No branch debt.** The harness named `claude/glibc-research-poc-0mwrxy`;
   `RULES.md` §Git outranks it and every commit is on `main`. The branch was
   deleted locally and `git ls-remote --heads origin` lists `refs/heads/main`
   and nothing else — it never reached the remote.
2. ⚠ **A GPU** — now **T-059** rather than a question. The bed can show a
   bundle's GL stack loading and initialising; it cannot show it rendering
   against anybody's driver.
3. **T-015 changes what the bed is.** Unchanged from the previous session.
