# Comparing the approaches

⛔ **Every cell is either a measurement taken in this repository, or a dash.**
A dash means *not measured here* and never "probably fine".

⚠ **`pgb` is a toolchain, and half of this page compares it to formats.** That
comparison is worth having — it establishes that the output holds up against
what people ship today — but it is not the axis `pgb` is developed along. See
[`design/toolchain.md`](design/toolchain.md), and the last section here.

Evidence: `evidence/60-versus-alternatives/`, `evidence/61-libc-throughput/`,
`evidence/62-anylinux-appimage/`.

---

## Who is actually in this race

⛔ **musl is not a rival, it is the thing being avoided.** `tmp/START.md` asks
for static binaries "using GLIBC **rather than MUSL** … while avoiding the
usual drawbacks and portability problems". A static musl binary is portable and
small; what it is not is glibc, and that costs throughput — which is the whole
reason the brief specifies glibc.

⛔ **Vanilla AppImage is not the AppImage that competes.** It deliberately does
not bundle glibc; its documented practice is to build against the oldest glibc
you support, so it does not start on a musl host. The one that competes is
[`Anylinux-AppImages`](https://github.com/pkgforge-dev/Anylinux-AppImages),
which bundles the libc, the loader, the gconv tree and the NSS modules.

**So there are three serious ways to ship one glibc program everywhere**, and
this page is mostly about the second and third:

| | |
|---|---|
| static musl | not glibc — a different answer to a different question |
| **`pgb`** | one static glibc ELF |
| **anylinux AppImage** | bundled glibc + loader + gconv, behind an AppImage runtime |

## Coverage: three-way tie, and it is not close for anyone else

Same program, same 11 environments. "Runs" means its own functional assertions
passed with no signal; "payload clean" means the process the program runs in
opened no host shared object.

| approach | runs | payload clean | evidence |
|---|---|---|---|
| **`pgb`** | **11 / 11** | **11 / 11** | `60-`, `62-` |
| **anylinux AppImage** | **11 / 11** | **11 / 11** | `62-` |
| static musl | **11 / 11** | **11 / 11** | `60-` |
| onelf bundle | 3 / 11 | 8 / 11 | `60-` |
| native dynamic glibc | 2 / 11 | 4 / 11 | `60-` |
| vanilla AppImage | 2 / 11 | 4 / 11 | `60-` |
| plain `gcc -static` | **1 / 11** | 7 / 11 | `60-` |
| Flatpak | 0 / 11 | — | `60-` |
| snap | 0 / 11 | — | `60-` |

⚠ **"Clean" is not "passed"** — a binary that cannot start loads nothing and
scores clean. Read the two columns together.

⚠ **Flatpak and snap were built here and cannot run on any target**: 0 of 11
images ship `flatpak`, 0 of 11 ship `snap`. Their deliverables are ~4 KB
because the weight is on the host — 623 MB of runtime, or a daemon needing
systemd.

⭐ **onelf's 3/11 is the most instructive failure on this page.** It bundles
glibc *and* its loader, and that half works everywhere: zero host objects on
all 11, musl included. It then fails the encoding assertions on 8, because
bundling a libc does not bundle **gconv** — and the 3 it passes, it passes by
reaching the *host's* gconv modules. ⛔ That is not an argument against
bundling: anylinux bundles the gconv tree on purpose and passes 11/11.
See [`design/tiers.md`](design/tiers.md).

## Throughput: the axis the brief is actually about

Same machine, same compiler, libc the only variable. ns per operation, lower is
better (`experiments/61-`, 3 rounds, best round).

| workload | glibc static | musl static | musl slower by |
|---|---|---|---|
| malloc, 4 threads | **4.53** | 584.71 | **129×** |
| qsort | **93.20** | 921.49 | 9.9× |
| strlen/strchr/strstr | **149.14** | 1051.09 | 7.1× |
| malloc, 1 thread | **12.95** | 42.20 | 3.3× |
| snprintf | **344.42** | 989.67 | 2.9× |
| math (pow/exp/log/sin) | **28.58** | 32.07 | 1.12× |
| memcpy, 8 B–256 KiB | **970.29** | 1036.82 | 1.07× |

⭐ **glibc's advantages are where its engineering is**: per-thread malloc arenas
against a contended allocator, and IFUNC-dispatched SIMD string routines
against generic C. Where neither applies — `memcpy` at sizes that are
bandwidth-bound, libm — the two are level. ⚠ This is not a claim that musl is a
bad libc; it is smaller and starts faster, and for short-lived processes that
can matter more than any row above.

**Does the advantage travel, and does `pgb` cost anything to carry it?**

| | on Alpine 3.22, malloc 4 threads |
|---|---|
| `pgb` | **4.34–4.68 ns** |
| anylinux AppImage | **3.66–7.20 ns** |
| static musl | 592–636 ns |

⭐ **That row is the product.** On a machine that ships no glibc, both glibc
deliveries give you glibc's numbers. And `pgb` adds nothing over a plain static
glibc build on the same workloads: **0.99×–1.05×** (`experiments/61-` arm B),
which is the steady-state counterpart to `experiments/40-`'s startup result.

## `pgb` against the anylinux AppImage, which is the real comparison

Neither wins on portability or on speed. They differ in **shape** and in
**reach**.

| | `pgb` | anylinux AppImage |
|---|---|---|
| runs / payload clean | 11 / 11 | 11 / 11 |
| throughput | glibc | glibc |
| what you ship | **2,097,824 B** | 3,706,288 B |
| what the target does to run it | **executes an ELF** | mounts it, or extracts it into `/tmp`, then runs a shell `AppRun` |
| writes to the filesystem | **nothing** | a mount point or an extraction directory |
| `PT_INTERP` / `DT_NEEDED` | **absent / zero** | a bundled loader, invoked explicitly |
| host objects in the *delivery* | **none** | the host `/bin/sh` and its libraries, where `/bin/sh` is dynamic |
| serves programs with a large dynamic dependency graph | ⛔ **not yet** | ✅ **today** |

⛔ **The last row is the open problem, and it is not in `pgb`'s favour.** From
that project's own HOW-TO: *"Compile statically! Sure, that works, go and
compile all of kdenlive statically and get back to me once you get it done."*
Desktop toolkits, GPU stacks and host plugins are served by bundling today.

⭐ **That is a target, not a boundary.** `pgb`'s answer is to push each
dependency as far up the brief's preference order as it will go — link it,
build a static library for it, wrap its `dlopen` against a compiled-in table —
and bundle only what survives all of that.
[`design/toolchain.md`](design/toolchain.md) has the plan and the bar a `pgb`
bundle would have to clear; `AGENTS.md` §13 item 4 has the three untried routes
to the plugin case.

⭐ **What `pgb` has meanwhile** is that its output is not a package. No runtime,
no mount, no extraction, nothing written, no shell in the delivery path, and
nothing on the target that has to cooperate. For a program that *can* be linked
statically, that is a smaller and simpler artefact for the same coverage and
the same speed.

⚠ **The host-object row deserves care in both directions.** The AppImage's
*payload* is clean on all 11 — its libc, gconv and NSS modules all come out of
its own bundle. What the tree picks up is the `AppRun` shell, and only on
distributions whose `/bin/sh` is dynamically linked. That is the delivery
mechanism, not the program, and no second libc enters the program's address
space: `execve` replaces it before the payload runs.

## The axis this page mostly leaves out: what the developer does

⛔ **Everything above compares artefacts. `pgb` is a toolchain, so the axis it
is actually developed along is what a developer has to know and assemble** —
and no experiment measures that yet. **T-013** carries it, and the experiment
does not exist: `TODO/toolchain.md` names the number it reserves.

What can be stated now, from building both routes in this repository:

| | `pgb` | anylinux AppImage |
|---|---|---|
| external binaries to fetch and pin | **0** — the environment is one pinned OCI digest | 5, across 4 upstreams: `sharun`, a forked `appimagetool`, `uruntime`, `mkdwarfs`, `cross-libc-dlopen` |
| driver script | the tool | plus a 121 KB `quick-sharun.sh` |
| files the developer authors | **none** | a `.desktop` entry and an icon |
| environment variables to set | **none** | ~9 for a non-default layout |
| build host | any, with root — the environment is pinned and unpacked | upstream guidance says Arch Linux specifically |
| commands | `pgb env create` once, then `pgb build` | install to `/usr`, deploy, then make the image |

⭐ **This is not a criticism of `quick-sharun`, which automates the hard part
well** — it finds a program's entire library closure including `dlopen`ed
libraries, and deploys the libc, loader, gconv tree and NSS modules without
being told to. `pgb` has nothing equivalent and should learn from it. The
difference is that a `pgb` user never learns what an AppDir is.

⚠ **And `pgb` does not yet close its own half of this.** `pgb build -- make`
still requires the developer to know how to build the project. `pgb build
<url-or-package>` is the target, and until it exists this table describes an
intention on one side and a shipped tool on the other. Read it that way.

## Startup and size

| approach | ship (B) | per exec |
|---|---|---|
| native dynamic glibc | 16,368 | 1040 µs |
| static musl | **447,264** | **160 µs** |
| vanilla AppImage | 948,728 | 4610 µs |
| plain `gcc -static` | 949,568 | 950 µs |
| onelf bundle | 1,761,241 | 3650 µs |
| **`pgb`** | 2,097,824 | 980 µs |
| anylinux AppImage | 3,706,288 | — |
| Flatpak | 4,228 | — (+623 MB runtime on the target) |
| snap | 4,096 | — (+snapd, which needs systemd) |

⛔ **`pgb` loses this table and it does not matter much.** musl is smaller and
starts faster because it is a smaller libc; the bundle formats are larger and
slower to start because they decompress a payload every run. ⭐ The one row
worth reading twice is `pgb` against **plain `gcc -static`**: 980 µs against
950 µs, which `experiments/40-` established is at this instrument's noise floor
in both directions. Portability costs nothing at startup; it costs 1.1 MB of
static libiconv, and only for programs that call `iconv`.

## What is and is not measured

| | state |
|---|---|
| coverage, host objects, size | **measured**, 11 environments, every runnable arm |
| steady-state throughput | **measured** (`61-`, `62-`) — glibc vs musl, `pgb` vs plain static, and all three on every environment |
| startup, peak RSS | **measured on this host** for the arms that run here |
| Flatpak and snap at run time | ⛔ **not measured.** Built here; `flatpak run` needs a D-Bus session bus that `dbus-daemon` cannot start with `cap_sys_resource` dropped, and `snapd` needs systemd. Their 0/11 coverage is decided by the targets, not by this |
| onelf in its preferred modes | ⛔ **not measured.** memfd, FUSE and tmpfs all need `unshare(CLONE_NEWUSER\|CLONE_NEWNS)`, which is EPERM inside this chroot bed, so every onelf row is its last fallback — cache mode. Its coverage result is unaffected: the failures are gconv, not delivery |
| a real application through both stacks | ⛔ **not measured**, and it is the gap that matters most now. Everything above is one small program. The claim that separates the two — that anylinux reaches software `pgb` cannot — is argued from the dependency graph, not demonstrated |
| aarch64, a second machine, a second kernel | ⛔ **not measured** |
