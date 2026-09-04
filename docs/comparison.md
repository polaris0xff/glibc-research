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

## ⭐ THE THREE-WAY PARITY MATRIX — vanilla `gcc -static` vs ours vs native musl static

⛔ **The operator named this table, 2026-09-03d**: *"a markdown table covering
'vanilla' GLIBC static binaries vs 'Ours' static binaries vs native MUSL static
binaries must be compared on all possible comparisons that they can be compared
with"*, against the claim *"our static glibc binary and a native musl static
binary are at feature/standalone parity. No buts and no ifs."*

⭐ **One probe, three builds, eleven environments, every cell run —
`experiments/63-`, `pass=16 fail=0 skip=0`, two runs with every cell
identical.** ⛔ **`skip=0` is the number to read first.** `60-` and `61-` skip
arms they cannot build, so a missing musl toolchain yields a green run with an
empty column; here every arm built and nothing was skipped.

⚠ **The vanilla column is built INSIDE the pinned environment**, so it differs
from `pgb` only by the injected mechanisms — same gcc 14.2.0, same glibc 2.41.
A host-built vanilla arm is carried as a **control** and agrees with it on every
capability axis, so the column does not depend on that choice.

| axis | vanilla `gcc -static` | **`pgb`** | native musl static | evidence |
|---|---|---|---|---|
| runs on the eleven | 11 / 11 | **11 / 11** | 11 / 11 | `63-` |
| ⛔ **rows with a crashed axis** | ⛔ **5 / 11** | ✅ **0 / 11** | ✅ 0 / 11 | `63-` |
| payload clean (host `.so` opened) | 0 envs | **0 envs** | 0 envs | `63-` |
| NSS — `getpwuid(0)` | 10 / 11 | **11 / 11** | 11 / 11 | `63-` |
| NSS — `gethostid()` | ⛔ **SIGFPE on Arch** | ✅ **11 / 11** | ✅ 11 / 11 | `63-`, `82-` |
| iconv — encodings accepted | 1 / 12 on 8 rows, and it **CRASHES on 3** — SIGABRT on debian-11 and ubuntu-20.04, SIGFPE on debian-12 | ✅ **12 / 12** | 10 / 12 | `63-` |
| locale — codeset **by environment** | ⛔ 0 / 11 | ⛔ 0 / 11 by default, ✅ **11 / 11** with `--utf8-default` | ⭐ 11 / 11 | `63-`, `67-` |
| locale — codeset **when requested** | 7 / 11 | ✅ **11 / 11** | 11 / 11 | `63-` |
| ⭐ **an EXPLICIT `LANG=C` still obeyed** | 11 / 11 | ✅ **11 / 11** with `--utf8-default` | 11 / 11 | `67-` |
| timezone — `TZ=Europe/Berlin` | 7 / 11 | ✅ **11 / 11** | 7 / 11 | `63-`, `97-` |
| **`/etc/services`, `/etc/protocols`** | ⛔ 8 / 11 | ⛔ 8 / 11 by default, ✅ **11 / 11** with `--embed-netdb` | ⛔ 8 / 11 | `63-`, `82-`, `66-` |
| ⛔ **`getaddrinfo(NULL, "http")`** | ⛔ 8 / 11 | ⛔ **8 / 11 — `--wrap` cannot reach glibc's internal `__getservbyname_r`** | ⛔ 8 / 11 | `66-` |
| terminfo | — | ✅ 11 / 11 (`--embed-terminfo`) | — | `75-` |
| CA bundle | 5 / 11 | ✅ 11 / 11 (`--embed-cacert`) | — | `74-` |
| `dlopen` of its OWN plugins | — | ✅ 11 / 11, zero host objects | — | `71-` |
| `dlopen` of a HOST object | — | ✅ 11 / 11 carried; 7 / 7 glibc rows real | ⛔ refused by name | `76-` |
| `PT_INTERP` / `DT_NEEDED` | none / none | **none / none** | none / none | `63-` |
| artefact size (same source) | 1,148,360 B | 2,722,968 B | **237,440 B** | `63-` |
| throughput — malloc, 4 threads | 8.40 ns | **6.4–11.4 ns on all 11** | 606–705 ns | `61-` |
| throughput — qsort | 83.96 ns | ≈ vanilla (1.01–1.02×) | 607–621 ns | `61-` |
| throughput — str\*, 3 ops | 241 ns | ≈ vanilla (1.01×) | 1355–1378 ns | `61-` |
| throughput — memcpy | ⚠ **no difference measurable** | ⚠ same | ⚠ same | `61-` |
| startup, peak RSS | ⚠ **no difference measurable** | ⚠ same | 160 µs / — | `40-`, `60-` |
| what it writes to the filesystem | nothing | nothing, unless an `--embed-*` fires | nothing | `63-`, `97-` |

### ⛔ The two rows that came out against us — both now have a mechanism, and a NEW row does not

⭐ **T-078's own rule**: *"If a row comes out against us, that row IS the
deliverable — report it, do not soften the axis until it passes."* Two rows
came out against us on 2026-09-03e. ⭐ **On 2026-09-04 both have a mechanism,
measured twice on eleven environments** — and the mechanisms produced a third
row that is still against us, which is what this section is for.

1. ✅ **The environment-default codeset** — the one axis where native musl beat
   BOTH glibc columns. With no `LANG` set, musl answers **UTF-8 on 11 of 11**
   and every glibc arm answered `ANSI_X3.4-1968` on **11 of 11**. ⭐ `pgb
   build --utf8-default` now answers **UTF-8 on 11 of 11** (`experiments/67-`,
   `pass=7 fail=0 skip=0`, two runs identical). ⛔ **The row that makes that
   result mean something is `LANG=C`**: a mechanism that forced UTF-8
   unconditionally would score U4 green and be a *bug*, because a program run
   under `LANG=C` asked for a single-byte locale. U5 measures exactly that and
   is green on 11 of 11. ⚠ `--embed-locale` alone does **not** move the axis
   (U3, 11 of 11) — the flags are separate on purpose, and `--utf8-default` is
   opt-in because it changes what a program without `LANG` sees. ⚠ **One axis:**
   `C.UTF-8` is the C locale with a UTF-8 charset, not a full locale, and this
   says nothing about collation or message catalogues. ⛔ The prediction
   registered before `63-` said the opposite on both halves; it is recorded as
   wrong in `experiments/63-`'s header rather than rewritten.
2. ✅ **`/etc/services` and `/etc/protocols`** — the **eleventh** host-data
   dependency, found by `experiments/82-`. All three columns failed it on the
   same three environments (debian-11, debian-12, ubuntu-20.04 — every one a
   glibc row that ships no `/etc/services`), so it was a row nobody won and
   `pgb` claimed to be the one that should. ⭐ `pgb build --embed-netdb` now
   answers `http/tcp = 80` and `tcp = 6` on **11 of 11** (`experiments/66-`,
   `pass=12 fail=0 skip=0`, two runs identical). ⭐ **The host still wins where
   it has the file** — the wrappers call `__real_*` first — and a name *nobody*
   carries still answers NULL on 11 of 11 (N3), so the mechanism is a fallback
   and not an override. ⚠ Its tables cost **−7,280 B**: the arm B artefact is
   *smaller* than arm A, so the size argument against carrying them does not
   survive contact with the number.
3. ⛔ **NEW, AND IT IS AGAINST US: `getaddrinfo` with a service name.**
   `getaddrinfo(NULL, "http", …)` still resolves on only **8 of 11** with
   `--embed-netdb` on. ⛔ **This is a boundary of the `--wrap` mechanism, not an
   oversight**: `--wrap` redirects the *public* symbol, and glibc's
   `getaddrinfo` reaches its own internal `__getservbyname_r`, which the linker
   cannot interpose. It was **pre-registered as a failure before the run**
   (`experiments/66-` N4) precisely so that a green result would have to be
   explained rather than pocketed. A program that needs it must pass the port
   number, or the mechanism has to change shape.

### What the matrix says about the parity claim

⭐ **On every axis measured here, `pgb` is now level with or ahead of native
musl static** — ahead on iconv (12/12 vs 10/12), timezone (11 vs 7),
`/etc/services` (11 vs 8) and throughput (by 84× on contended malloc), and
level on the environment-default codeset once `--utf8-default` is passed, at
11.5× the artefact size. ⛔ **The claim as stated — *"no buts and no ifs"* — is
still not true, and the reasons have changed**: the two axes that were flatly
lost are now **flags a developer has to pass**, not defaults; and
`getaddrinfo` with a service name is a measured "but" that no flag closes. ⚠ A
capability behind an opt-in flag is a capability the tool *has*; it is not
parity with a libc that does it unconditionally.

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

⭐ **Re-measured on 2026-09-03e, TWICE**, on a machine where `musl-gcc` was
installed at session start — arm A had been **skipped** for want of it, so the
figures this table used to carry came from an earlier machine. Both runs below;
the ratio is what carries, not the ns.

| workload | glibc static | musl static | musl slower by (run 1 / run 2) |
|---|---|---|---|
| malloc, 4 threads | **6.73 / 8.40** | 606.39 / 704.79 | **90.1× / 83.9×** |
| qsort | **84.14 / 83.96** | 620.81 / 607.43 | 7.4× / 7.2× |
| strlen/strchr/strstr | **244.75 / 241.24** | 1377.94 / 1355.08 | 5.6× / 5.6× |
| malloc, 1 thread | **20.62 / 22.17** | 57.39 / 59.94 | 2.8× / 2.7× |
| snprintf | **585.88 / 574.47** | 722.80 / 719.61 | 1.23× / 1.25× |
| math (pow/exp/log/sin) | **43.41 / 42.86** | 47.10 / 46.44 | 1.09× / 1.08× |
| memcpy, 8 B–256 KiB | 1019.18 / 1132.95 | 1082.06 / 1090.71 | ⚠ **1.06× / 0.96×** |

⛔ **The `memcpy` row CHANGES SIGN between the two runs** — musl slower, then
musl faster — so it is at this instrument's noise floor and must be reported as
**"no difference measurable"**, never as a figure. That is the same rule
[`AGENTS.md`](AGENTS.md) §10 applies to startup and peak RSS, and the sign
change is why the rule exists. Every other row keeps its direction and its
rough magnitude across both runs.

⭐ **glibc's advantages are where its engineering is**: per-thread malloc arenas
against a contended allocator, and IFUNC-dispatched SIMD string routines
against generic C. Where neither applies — `memcpy` at sizes that are
bandwidth-bound, libm — the two are level. ⚠ This is not a claim that musl is a
bad libc; it is smaller and starts faster, and for short-lived processes that
can matter more than any row above.

**Does the advantage travel, and does `pgb` cost anything to carry it?**

| | on Alpine 3.22, malloc 4 threads |
|---|---|
| `pgb` | **6.39–6.86 ns** (both runs) |
| anylinux AppImage | **3.66–7.20 ns** |
| static musl | 626.42–1045.49 ns (both runs) |

⭐ **That row is the product.** On a machine that ships no glibc, both glibc
deliveries give you glibc's numbers. ⭐ **And it travels to all eleven**: `pgb`
does that workload in **6.25–12.32 ns** on every environment against static
musl's **622.99–1069.39** (`experiments/61-` arm C, one round per environment,
union of both runs). ⚠ **The committed `RESULT.txt` holds the SECOND run only**
— it is overwritten each time — so figures quoted from the first are a
replication check rather than something a reader can re-derive from the tree.
`pgb` adds **1.00×–1.13×** over a plain static glibc build on the same
workloads (arm B, both runs), which is the steady-state counterpart to
`experiments/40-`'s startup result.

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
bundle would have to clear; `AGENTS.md` §7 has the four untried routes
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

## ⭐ The axis that is now HALF THE BAR: what the developer does

⛔ **PROMOTED BY AN OPERATOR RULING, 2026-09-03c**, quoted in
[`design/toolchain.md`](design/toolchain.md): *"us having a bigger size than
anylinux-appimages and onelf is acceptable as long as ours performs better and
**packaging is just one command not a multiline shell script**."* ⭐ This table
is that half of the bar, and it is the half `pgb` wins — **publish it**.
⭐ **And as of 2026-09-03d it wins the other half too on this subject**: `jq`
cold start is **58.3 ms against the field's 58.4** across eleven environments,
warm ⚠ **no difference measurable** (8.5 against 9.3, equal medians, and
`86-`'s warm arithmetic is itself unverified — `TODO/research.md` T-057).
⛔ Cold read **2.07×** the same
morning; two constants in `internal/bundle/appimage.go` closed it — uruntime
`full` → `lite` (`experiments/77-`, 1.28×) and the dwarfs block `-S26` → `-S18`
(`experiments/81-`, 1.00×). ⚠ **Measured on a CLI and unmeasured on a GUI**:
kdenlive's figures predate both levers and were taken with the cold protocol
[`history/corrections.md`](history/corrections.md) C24 disproves.

⛔ **Everything above compares artefacts. `pgb` is a toolchain, so the axis it
is actually developed along is what a developer has to know and assemble** —
and ⚠ **no experiment measures it yet**, which matters more now than it did
when this page was written. **T-013** carries it, and the experiment
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
