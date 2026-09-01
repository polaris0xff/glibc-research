# SUMMARY.md — the session of 2026-09-01d

⛔ **Saved as well as printed**, per
[`../docs/methodology/sessions.md`](../docs/methodology/sessions.md), so it
survives the chat scrolling away. It is the fastest orientation into what the
last session actually did. Overwritten each session.

⭐ **Every cell is grounded in something that can be pointed at**, and where a
thing was not measured this says so rather than giving a number.

| row | before | after |
|---|---|---|
| **Elapsed** | 2026-09-01T18:10Z | 2026-09-01T19:35Z — **≈1h25m** |
| **Commits** | `544bfa61` | `60f997e8` — **11 commits**, every one on `main`, pushed as they landed |
| **Work** | 4 required POCs assigned, 30 entries, 17 open, 13 done | ⭐ **4 of 4 completed**, **0 deferred, 0 failed**. 32 entries, 17 open, 15 done — T-032 and T-052 closed, T-058 and T-059 opened |
| **Changes** | — | **64 files**, 2,840 insertions(+), 440 deletions(-) |
| **Size** | 25,702 lines | **27,299 lines** (+1,597), excluding `references/` and `evidence/` |
| **Checks** | `sh TODO/check.sh` green | green. Plus **9 selftests re-run, all pass** (bootstrap, oci-pull, rootfs-run, mine-repo, nix-fetch, nix-appimage, nix-drv, nix-nar, elf-needed) |
| **Cost** | — | ⚠ **not metered.** What can be pointed at: qtbase 6.11.1 source (50.6 MB) and two mesa-demos closures (392 MB each, one from cache) fetched from cache.nixos.org; a jq closure (39 MB); Arch `pacman -Sy base-devel` in the test bed; four pinned anylinux tools. Disk went from 29 GiB free to ~15 GiB. No paid service was used. |
| **Health** | ⛔ acceptance bar: 6 of 9 issues closed | ⭐ **8 of 9 closed**, one left (host plugins). **5 tool defects + 2 instrument defects found and fixed**; **2 new debts, both carried as open entries** (T-058, T-059). Tree **clean**, `main` pushed, no `ephemeral-*` branches, **no branch debt**. |

## The four required POCs, discharged

| # | required | result | evidence |
|---|---|---|---|
| 1 | `poc/90-qt` — Qt 6, static | ✅ **11 of 11, zero host objects** | 19 assertions, 0 fail |
| 2 | `experiments/85-opengl` | ✅ **Mesa/swrast on 11 of 11**, control clean | 7 assertions, 0 fail |
| 3 | `experiments/86-bundler-vs-anylinux` | ✅ **both 11 of 11; ours 3.05× the size** | 7 assertions, 0 fail |
| 4 | `poc/20` + `poc/30` reruns | ✅ **11 of 11 each** | 12 assertions each, 0 fail |

## The seven defects, because "7" is not a finding

⭐ **Five were in the tools and two were in this session's own instruments.**
Every one produced a plausible result while being wrong — that is the pattern,
and it is why building above the current class is worth more than the features
it produces.

| # | defect | what it looked like |
|---|---|---|
| 1 | pgb appended `-march=x86-64` **after** the caller's argv | *"x86 intrinsics support missing. **Check your compiler settings.**"* — Qt blaming the user for pgb's flag |
| 2 | `make_wrappers` opened with `rm -rf` on a directory bind-mounted into a **running** build | `cc: not found` from inside somebody else's ninja, minutes in |
| 3 | `nix-appimage.sh` fell back to the first binary in `bin/` when `--name` missed | packed `quadstrip-flat` instead of `eglinfo`, and said so in one line among eleven |
| 4 | `nix-appimage.sh` read nixpkgs' `out` output | `no entry point in ...-jq-1.8.2/bin` — reads like a broken package, is a wrong output |
| 5 | `pgb-cacert.c` missing `<stdio.h>` | implicit `snprintf`; a warning under gcc 12, an **error** under C23 |
| 6 | 86-'s startup instrument reaped between runs, killing the dwarfs mount | both arms **~14,500 ms** where the real warm figure is **17 ms** |
| 7 | two runs on the shared bed at once | `poc/30-curl`'s voidlinux row came back **SIG9** with nothing wrong with the binary |

⚠ **And two assertion defects inside `poc/90-qt` itself**, caught before its
evidence was committed: the plugin archive was looked for at nixpkgs'
`INSTALL_PLUGINSDIR` rather than Qt's own default (reporting a FAILURE for a
plugin built correctly), and the "no shared libraries" check was scoped to
`libQt6*.so`, which would have passed a Qt whose *plugins* were still shared —
the half that POC is about.

## What was NOT measured

- **Anything on a GPU.** Every GL row is `swrast`. **T-059.**
- **Qt against a real display.** `-no-xcb`; the offscreen QPA is what ran.
  T-054 rung 2.
- **KF6 and kdenlive.** Untouched. T-054 rungs 3 and 4.
- **A second machine or kernel.** One machine, one day, as every number in
  this tree still is.
