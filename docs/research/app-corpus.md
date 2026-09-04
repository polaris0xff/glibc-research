# The battle-test corpus, classified by the mechanism it needs

⛔ **Set by the operator, 2026-09-04**: a list of applications *"to battletest
and prove our bundler is best in class"*, with the instruction to **sort the
tasks by what will auto-fix or complete what — not easy first**.

⚠ **This page was research when it was written, and three rungs now carry
measurements.** Each row says which mechanism a subject exercises, what the
field already knows about it, and — where one has been taken — the number.
⛔ **Read the rung, not this header**: a rung that still says *hypothesis*,
*not measured* or *read off the source* means exactly that.

| rung | state |
|---|---|
| 1 · multi-entry dispatch | ⭐ the **dispatch table** is measured (`experiments/68-` arm S, 18 of 18). ⛔ No second program has yet come out of a real bundle — arm B |
| 2 · static / raw-syscall payload | ⭐ the **mechanism** is measured (`experiments/100-` arm P, two runs) and it split into two. ⛔ No real application of either shape — arm G |
| 3 · GTK's locale prefix | ⛔ **the locale half is NOT MEASURABLE in this bed** — no environment has a non-C locale, so no catalogue is opened by either arm. ⭐ The mechanism claim holds on a different discriminator, now with two subjects |
| 5 · namespaces | ⭐ the **cause** is isolated and a route exists (`experiments/69-`, `pass=9`). ⛔ No browser has been bundled |
| 4, 6, 7, 8 | research. Rungs 6 and 7 were **corrected** against the vendored trees |

`experiments/65-` is the harness for the capability corpus;
[`TODO/research.md`](../../TODO/research.md) T-087 owns the rest.

⭐ **The ordering rule.** A subject is not hard or easy — a **mechanism** is
present or missing. Ordering by app difficulty re-measures the same mechanism
five times; ordering by mechanism makes each rung close a whole column at once.
The ladder below is that order.

---

## The ladder — do these in this order

### Rung 1 · Multi-entry dispatch. ⭐ SHIPPED, NEVER MEASURED

`rnote`+`rnote-cli`, `nicotine`+`nicotine-plus`, `mkvtoolnix`+`mkvtoolnix-cli`,
`stirling-pdf`+`stirling-pdf-desktop`, `imagemagick` (`magick`, `convert`,
`identify`, …), `lmms`, the three `dosbox` forks.

**What exists.** `assemble.go` installs the entry point and then **every other
program in the same store path's `bin/`** automatically — dot-named files
excluded, because `.meld-wrapped` is a wrapper target and not a program.
`--with-program NAME` adds one found anywhere in the closure, for the case where
a helper lives in a dependency. `tool/runtime/pgb-apprun.c` is a **static**
selector, built whenever a bundle carries more than one program.

⭐ **THE DISPATCH ORDER IS MEASURED — `experiments/68-` arm S, 18 of 18** —
and it is **not** the order this page and the source's own header comment both
carried until it was run:

    $ARGV0's basename  →  argv[1] (and it is DROPPED)  →  argv[0]'s basename  →  default

⛔ **`ARGV0` is the one that matters and neither text named it.** uruntime sets
`ARGV0` to the AppImage's own path, so a **rename or a symlink lands on rule 1**
— which is what makes `./rnote-cli` work — and `./app.AppImage rnote-cli …`
lands on rule 2. Measured with it: a selected `argv[1]` is consumed and later
arguments survive; an `argv[1]` that did *not* select is passed through
untouched; the child's `argv[0]` is the **absolute** path (load-bearing for
sharun); `ARGV0` is unset and `APPDIR` set in the child; ⛔ **a name containing
`/` is never a program**, so a path cannot be injected through `argv[1]`; and a
name in `shared/bin` alone, without `bin/`, is not a program.

⭐ **And the negative control is what makes those mean anything**: the same
source built with an *empty* default and given no name exits **127** with
`no default program`, while the same binary still dispatches when told one.

⭐ **AND THE BUNDLE HALF IS MEASURED TOO — `68-` arm B, `pass=24 fail=0
skip=0`.** `mkvtoolnix`, a real nixpkgs closure, on all eleven environments:

| | |
|---|---|
| the build's own entry-point count | ⭐ **`programs mkvmerge + 4 more`** — the operator's question 3 answered by a number the tool emits |
| the selector actually built | ⭐ the **static** `pgb-apprun`, not a shell |
| ⭐ **the SECOND program (`mkvextract`) runs, by its own name** | ✅ **11 / 11** |
| the entry program (`mkvmerge`), same artefact — the within-row control | ✅ **11 / 11** |
| the second program's host shared objects | ⭐ **0 on 11 / 11** |

⛔ **The assertion is each program's OWN identity, not a shared version
string**: `mkvmerge --version` says `mkvmerge` and `mkvextract --version` says
`mkvextract`, so a dispatch that quietly ran the default **fails**. That is why
the subject was chosen over `imagemagick`, whose `convert` and `identify` are
symlinks to one binary printing one string.

⚠ **It measures ONE closure.** A second multi-program subject is a different
measurement, not a repeat of this one.

**Study.** `internal/bundle/assemble.go` `installProgram`;
`tool/runtime/pgb-apprun.c` (130 lines, and it is the answer);
`experiments/68-` arm S for what each rule actually does.

### Rung 2 · A static or raw-syscall binary. ⭐ THE MECHANISM IS MEASURED; THE APPLICATION IS NOT

`syncthing` (Go), `powershell` (.NET), `lilipod` (Go).

⛔ **This was a named hole in a shipped claim**, not a low-priority "easy" row:
[`../design/store-paths.md`](../design/store-paths.md) §3 carried one row marked
**NOT MEASURED** — the interposer *"works for a static binary, or one issuing
raw syscalls — **no**"* — because there is no PLT to win.

**The failure was pre-registered as a failure** before the run, and it held.

⭐ **THE MECHANISM HALF IS NOW MEASURED — `experiments/100-` arm P, two runs
identical — AND IT SPLIT THE ROW IN TWO.** A planted store path that does not
exist on this machine, behind the real interposer and a real `.storemap`, with
the *same source* built three ways:

| probe | our object mapped in it | resolves |
|---|---|---|
| ⭐ dynamic through the PLT — **the positive control** | yes | ✅ **yes** |
| ⛔ `-static` | **no** | ⛔ no |
| ⛔ `syscall(SYS_openat, …)` | **yes** | ⛔ no |
| ⭐ dynamic, no preload — the negative control | no | ⛔ no |

⭐ **The two shapes fail for different reasons and only one is about linking**:
static because there is no loader and **nothing of ours is in the process**;
raw-syscall **with our object loaded**. So a subject can be fully dynamic and
still defeat the interposer. ⚠ The positive control is what makes this a
result — the first version of the harness wrote the `.storemap` in the wrong
format, every lookup missed, and both predicted failures "passed" for entirely
the wrong reason.

## ⛔ AND THE APPLICATION HALF FALSIFIED THIS RUNG'S PREMISE

⭐ **`experiments/100-` arm G ran, and the pre-registered check that caught it
is the one that failed.** This page said *"A Go program from the closure with a
store path compiled in is exactly that shape, and `pgb` itself is one."*

| G3 — is the payload actually static? | ⛔ **NO. `dynamic`** — nixpkgs' `syncthing` carries a `PT_INTERP` |
|---|---|
| G1 — store paths compiled in | **8**, of which **7 resolve inside the bundle** and 1 does not |
| ⭐ G2 (**reported, never predicted**) | `syncthing` **ran on 11 of 11**, host-object-clean on **11 of 11** |

⛔ **So `syncthing` was the wrong subject for the static claim**, and the run
says so rather than the claim being quietly adjusted. Arm P's mechanism result
is untouched — it does not depend on any subject.

⚠ **WHAT THIS DOES NOT ESTABLISH, and the gap is interesting rather than
tidy.** Go issues **raw syscalls** for much of its file I/O even when the
binary is dynamic, and arm P's S3 shows a raw-syscall caller defeats the
interposer *with our object loaded*. Yet the subject ran everywhere with 7 of
8 paths resolving. ⛔ **Three explanations are consistent with that and none is
measured**: the paths may have been rewritten in **text at build time** rather
than at run time; the program may never **open** them; or this build may route
those opens through libc after all. Establishing which needs `LD_DEBUG` and a
trace on one row, not another subject.

## ⭐ THE STATIC SUBJECT RAN, AND IT MOVED THE QUESTION OFF THE LINKER

`lilipod` — `experiments/100-` arm L. It is genuinely static (`L2`: no
`PT_INTERP`, no dynamic section), and the first thing that happened is that
**`pgb bundle appimage` refused it**:

    closure     4 store paths
    libraries   0 from the closure
    pgb: the closure carries no dynamic loader

⛔ **So for a fully static payload the interposer question never arises**:
there is no artefact to ask it of. Run the raw binary instead, no bundle at
all, on all eleven:

| | |
|---|---|
| ⭐ **L5 — the static ELF EXECUTES** | ✅ **11 / 11.** Every row ran lilipod's own code and printed lilipod's own message |
| ⛔ **L3 — the APPLICATION completes** | **2 / 11** |
| L4 — zero host shared objects | **1 / 11** |

⭐ **AND THE NINE FAILURES ARE ONE CAUSE, NAMED BY THE PROGRAM ITSELF**:
*"failed to find dependency `getsubids`, can't recover"*. `getsubids` is a
**host program on `$PATH`** (shadow-utils). Probed across the bed: exactly
**two** rootfs carry it — `archlinux-latest` and `opensuse-leap-15.6` — and
those are **exactly** the two that completed. ⚠ One of the two needed the
network as well: openSUSE has no `tar`, so lilipod **downloaded busybox at run
time**, which is not portability either.

⛔ **THE FINDING IS THAT THIS IS A DIFFERENT AXIS ENTIRELY.** A statically
linked ELF is portable — L5 is 11 of 11 and that is what static linking buys.
A statically linked *application* need not be, and no linker, loader trick or
bundler addresses a dependency that is **another program**. ⭐ Static linking
answers *"will this binary start"*; it says nothing about *"will this program
find the tools it shells out to"*.

⭐ **This is exactly the class rung 7 predicted for container tooling** —
`newuidmap`/`newgidmap` are setuid host binaries a bundle cannot ship, and
`getsubids` is the same family. Predicted there from reading; measured here on
a rung-7 subject.

⚠ **And it explains L4.** When a static program execs host programs, the host's
libraries enter the **process tree**, so the trace is no longer clean. That is
the same mechanism `experiments/65-` pre-registers for `xterm` (C5), whose job
is to run the user's shell — reached independently, on a different subject.

⭐ **So the refusal in L1 is right for a reason better than the one it gives**:
a payload that already starts everywhere does not need bundling. ⛔ But the
message names a missing **loader**, which sends the reader to the wrong
problem — and bundling `lilipod` would not have helped, because the missing
thing is a program, not a library.

**Study.** `docs/design/store-paths.md` §3 and §4; `experiments/100-`.

### Rung 3 · GTK's hardcoded prefix. ⭐ THE FIELD'S OWN "GARBAGE" ROW, AND OUR INTERPOSER IS THE ANSWER

`gnome-maps`, `gnome-chess`, `gnome-music`, `gnome-calendar`,
`gnome-disk-utility`, `pinta`, `rnote`, `bleachbit`, `flameshot`.

The field's `HALL-OF-FAME.md` opens its GTK section with: *"Every single GTK app
has the path to its locales hardcoded at the prefix (`/usr/share/locale`) and
there is no env variable to change this."* ⭐ **A compiled-in absolute path with
no search variable is the class T-081's interposer answers**, and the class
`--embed-terminfo`-style variable redirection cannot.

⚠ **But not that literal path, and the distinction matters.**
`tool/runtime/pgb-storefix.c`'s `fix()` returns a path **unchanged** unless it
begins with `/nix/store/`; `/usr/share/locale` is not rewritten and never was.
⭐ What makes the rung ours is that a **nixpkgs-built** GTK application does not
compile in `/usr/share/locale` — it compiles in *its own store path*, which is
precisely what the interposer answers. So we resolve **our form** of the field's
problem. ⛔ A distro-built binary with `/usr/share/…` in its `.rodata` is a case
the farm does not cover, and `flatimage`'s portable root is the shape that
would.

⛔ **So this rung is the differentiator, and it should be measured as one**: a
GNOME app under a non-English `LANG`, asserting a translated string, against the
same bundle built `--no-storefix`. A window is not enough here — the window
appears either way.

## ⛔ AND THE LOCALE HALF IS NOT MEASURABLE IN THIS BED — measured, not assumed

⭐ **`experiments/101-` was written, run twice with two subjects, and stopped.**
Every row reported *no catalogue opened*, in **both** arms. The cause is the
bed, and it was found by asking what locales the eleven environments actually
have rather than by reading the zeros:

| | |
|---|---|
| environments with a `de_DE` locale **compiled** | ⛔ **0 of 11** |
| what they do have | `C.UTF-8` / `C.utf8` on 7; **nothing at all** on the three Alpines and Void |
| environments carrying `share/locale/de` **message catalogues** | 6 of 11 — ⚠ the *translations* are there, the *locale* is not |

⛔ **So `setlocale(LC_ALL, "de_DE.UTF-8")` fails, `LC_MESSAGES` stays `C`, and
glibc's gettext then does not consult `LANGUAGE` — no `.mo` is ever opened, by
either arm.** The criterion cannot fire, so it cannot discriminate, so it is
not an instrument. ⚠ This sits beside the sandbox row: a **bed** limitation,
not a bundler result.

⭐ **THE MECHANISM CLAIM SURVIVES ON A DIFFERENT DISCRIMINATOR, AND IT NOW HAS
TWO SUBJECTS.** What rung 3 is really about — a compiled-in absolute path with
no search variable — is already measured by whether the application **draws**:

| subject | with the interposer | `--no-storefix` |
|---|---|---|
| `galculator` (`64-` arms G / N) | ⭐ **11 / 11** | ⛔ **0 / 11** |
| `gnome-chess` (`101-`, 4 rows before it was stopped) | drew | ⛔ **did not draw** |

⚠ `mousepad` is the counter-example that makes those mean something: it draws
in **both** arms (`WINDOWS 1/1`), because its UI is a GResource compiled into
the binary rather than a file at a store path.

⛔ **What is still owed for the locale instance specifically**: an environment
with a real non-C locale, or a bundle that carries one and points `LOCPATH` at
it. Neither exists here today.

⭐ **AND THE FIELD CONSIDERED OUR ROUTE FOR THIS AND CHOSE ANOTHER — this is the
citation that makes the rung worth doing.** Their `quick-sharun` **patches the
binary**: issue #45, *"the script is able to detect when you have a hardcode
path to `/usr/share/locale` or `/usr/share/icons` and will patch just that"*.
Issue #60 is the decision — *"replace ld-preload-open for pathmap, drop relative
path mapping"* — declined because *"if I were to replace this for pathmap,
**all libraries** would try to…"*, and it names what they used instead:
**relative symlinks in `/tmp`**, the route
[`../design/store-paths.md`](../design/store-paths.md) §2 rejects on security
grounds, in writing, before anything was built.

⚠ **NOT "they rejected LD_PRELOAD".** They ship one by default —
`ANYLINUX_LIB=1`, *"preloads library that fixes several common issues that
affect AppImage"* — and issue #393 moves their locale logic into its
constructor. What they declined was using a preload for **path mapping**
specifically. ⛔ Say it that way; the stronger version is not what the source
says.

⚠ **And a convergence worth knowing about before reinventing it**: issue #393
moved their locale handling into an `anylinux.so` **constructor** with the chain
`setlocale(LC_ALL,"")` → `LOCPATH=$APPDIR/...` → `en_US.UTF-8` → `C.UTF-8` →
`C`. That is `tool/runtime/pgb-locale.c` plus `--utf8-default`, arrived at
independently. Read theirs before extending ours.

## ⛔ Where they are ahead, from their own variable list — the parity answer

Read off `HOW-TO-MAKE-THESE.md` "Configurable environment variables". ⭐ **These
are features we do not have, and most are small.** They are not rungs; they are
the honest half of the parity answer the operator asked for.

| theirs | what it does | ours |
|---|---|---|
| ⭐ `OPTIMIZE_LAUNCH=1` | a **DWARFS profile image** — PGO for the mount, laying out blocks in access order | ⛔ **nothing.** We have `lite` and `-S18`, which are different levers. A named cold-start lever we have not tried — **T-066** |
| `x86-64-v3-check`, `x86-64-v4-check`, `vulkan-check`, `wayland-is-broken` hooks | the bundle **detects** the condition and prints a message instead of crashing | ⛔ **nothing.** Ours crashes silently. Cheap, and it is what a user sees |
| `GTK_CLASS_FIX=1` | a shim fixing `WM_CLASS` so the taskbar groups the window with its icon | ⛔ nothing — **T-083** territory |
| `self-updater.hook`, `udev-installer.hook` | update in place; install udev rules | ⛔ nothing |
| `QUICK_SHARUN_SKIP_DEPS_FOR` | skip a dependency subtree by name, e.g. `libqgtk3.so` so a Qt app does not drag GTK3 in | ⚠ `--debloat` is coarser and not by-name |
| `DEPLOY_PYTHON`, `DEPLOY_LOCALE`, `DEPLOY_OPENGL`, `DEPLOY_VULKAN` | opt in/out of whole subsystems | ⚠ ours are implied by the closure, which is usually right and is not steerable |
| `STRACE_MODE=1` | discover `dlopen`ed libraries by tracing | ⭐ **not needed** — the closure is the exact set the derivation declared |

⭐ **The one row that goes the other way is the last one, and it is the
project's whole thesis**: they trace to guess a dependency set; we are handed
one.

**Study.** `HALL-OF-FAME.md` "Garbage - GTK"; issues **#45, #60, #393, #616**
in the vendored `api/issues.json`;
`tree/useful-tools/hooks/fix-gnome-csd.hook` (GNOME draws no server-side
decorations, so a GNOME app must draw its own — a bundle that gets this wrong
shows a window with no titlebar, which the geometry criterion still counts).

### Rung 4 · Plugin trees found by scanning, not by linking

`lmms`, `mpv`, `handbrake`, `gnome-music`, `remmina` (FreeRDP plugins),
`openscad`.

**What exists.** `sharun.go`'s `bakedOverride` already emits
`GST_PLUGIN_SYSTEM_PATH_1_0`, `MLT_*`, `FREI0R_PATH`, `LADSPA_PATH`, `BABL_PATH`,
`GEGL_PATH` when it finds the matching subdirectory in the closure.

⛔ **What is missing, and the field names it.** GStreamer needs **four**
variables and a fifth pointing at the scanner binary: `GST_PLUGIN_PATH`,
`GST_PLUGIN_SYSTEM_PATH`, `GST_PLUGIN_SYSTEM_PATH_1_0`, `GST_PLUGIN_SCANNER`.
⭐ **CORRECTED 2026-09-04b — see [`TODO/research.md`](../../TODO/research.md)
T-091.** We emitted one; the other three may or may not be supplied by sharun's
own `dir.starts_with("gstreamer-")` branch, which **can** fire here because
`copyLibraries` carries `lib/` subdirectories whole. ⛔ **`GST_PLUGIN_SCANNER`
is the one that is definitely missing**: sharun sets it only when the scanner
sits beside the plugins and nixpkgs puts it in `libexec/`. All four are emitted
now, the scanner installed as a bundle *program* so it runs through sharun —
⛔ **unmeasured.** And *"`gst-plugin-scanner`
opens every single gstreamer plugin on the system, so we cannot easily determine
using `strace` what plugin an application needs"* — which also means a
host-object count taken on a GStreamer subject is measuring the scanner, not the
application.

**Study.** `HALL-OF-FAME.md` "Bad - GStreamer"; `internal/bundle/sharun.go`
`bakedOverride`.

### Rung 5 · Namespaces and the sandbox. ⭐ THE CAUSE IS `chroot`, AND THERE IS A ROUTE

`brave`, `firefox`, `google-chrome`, `signal-desktop`, every Electron app,
`bottles`.

⛔ **`unshare(CLONE_NEWUSER)` is `EPERM` inside the bed** — the recorded reason
every `onelf` row in [`../comparison.md`](../comparison.md) runs in its
last-resort mode, and a Chromium sandbox needs exactly that call. ⭐ **What was
missing was the CAUSE**, and `pgb rootfs run` does two things — `unshare
--mount` *and* `chroot` — so the bed row alone could not blame either.
`experiments/69-` runs each without the other, same rootfs, same probe:

| arm | `unshare(CLONE_NEWUSER)` |
|---|---|
| ⭐ the HOST, one process per call (**the control**) | **OK** — all five namespaces |
| the bed, as `pgb rootfs run` enters it | ⛔ **EPERM** |
| ⭐ **`chroot` ALONE** | ⛔ **EPERM** |
| ⭐ **`unshare --mount` ALONE** | ✅ **OK** |
| ⭐ **the same rootfs entered by `pivot_root`** | ✅ **OK**, and with `CLONE_NEWNS` too |

⭐ **The refusal is `chroot`'s.** Not the machine, not a sysctl —
`/proc/sys/user/max_user_namespaces` reads **64230** *inside* the bed — and not
a blanket refusal: mount, pid and net all unshare there. `lsns -t user` inside
the bed reports exactly **1**, which is the operator's *"check, do not guess"*
answered rather than assumed.

⭐ **So the bed change is now NAMED and small**: enter by `pivot_root` instead
of `chroot`. ⛔ **`69-` does not say `pgb rootfs run` should do that.** Three
things stay unmeasured and none follows from it — whether the bed still
isolates under `pivot_root`, whether teardown stays clean, and whether a
bundled browser then actually sandboxes. ⚠ Until one of those is taken, a
browser row here still measures `--no-sandbox`, which is a different program.

**Two separate questions, and they must not be merged.**
1. *Does the bundle carry a working Chromium?* — answerable today with
   `--no-sandbox`, and worth having.
2. *Does the sandbox work?* — needs a bed that allows user namespaces, and a
   check that is not a guess: `lsns -t user` from inside the sandboxed process,
   or `ip netns list`.

⚠ **Ubuntu ≥ 23.10 restricts unprivileged user namespaces by default**
(`kernel.apparmor_restrict_unprivileged_userns=1`), so on a real Ubuntu target
this fails for a reason that has nothing to do with the bundle. The field ships
a `pkexec` hook that asks the user to turn it off. ⛔ We should not copy that
hook — it asks for a root password — but we must **detect and report** the
condition rather than showing a crash.

⭐ **DETECT-AND-REPORT IS AN OPEN ISSUE ON THEIR SIDE**, which makes it
available: issue **#438**, still open — *"If unprivileged user-namespaces are
detected as missing, use `--no-sandbox`… A warning message about this would be
good."* Their shipped answer is the `pkexec` hook that asks the user to disable
the restriction. ⛔ Do not copy that; ship the detection they have not.

⚠ Issue **#795** is the second half of the same problem and it is about *their*
artefacts: an Electron sandbox refuses to launch an AppImage built in Electron,
and `--no-sandbox` from the CLI works.

**Study.**
`references/pkgforge-dev__Anylinux-AppImages/tree/useful-tools/hooks/fix-namespaces.md`
(vendored; the operator's link) and `fix-namespaces.hook` beside it; issues
**#438** and **#795**; `HALL-OF-FAME.md` "Excellent - Chromium/Electron" — the
toolkit is easy, the sandbox is not.

### Rung 6 · 32-bit and a second architecture in one bundle

`wine`, `wine64`, `bottles`, `winboat`, `steam`.

`assemble.go`'s `mergedFor` already maps `lib32` to its own directory, so the
tree shape exists. What is untested is a bundle whose payload runs **two**
loaders. `steam` additionally bootstraps itself into `$HOME` and updates in
place, which is a different problem again — it is not a bundling question but a
question about what the bundle is allowed to write.

⭐ **AND THE FIELD'S SHIPPED ANSWER TO THIS RUNG IS A CONTAINER, NOT A
BUNDLE** — read 2026-09-04b from the now-vendored trees, and it reframes the
rung. `lux-wine` (176 stars) runs Windows applications *"using a specially
configured Wine/Proton and **RunImage container**"*, with its own *"portable
`lwrun` container with isolation from the host system"* and ArchLinux/Chaotic-AUR
repositories connected inside it; `lw-runtime` is not a runtime in our sense at
all but a **tarball of Wine libraries** (`PKGBUILD`, one `runtime.tar.zst`
source). `ivan-hc/Steam-appimage` (132 stars) is likewise *"built using
Runimage"*.

⛔ **So nobody in the corpus solves 32-bit-plus-64-bit with a sharun-style
bundle**; they solve it by shipping a distribution. That is the same conclusion
[`nix-appimage.md`](nix-appimage.md) reaches from the other direction — *why a
bundler ends up shipping a container* — and it means a green row here would be
a genuinely different artefact, not a re-run of theirs. ⚠ It also means their
trees are weak evidence about `assemble.go`'s `lib32` path: they are not doing
the thing we would be doing.

**Study.** `references/VHSgunzo__lux-wine/` and `references/VHSgunzo__lw-runtime/`
and `references/ivan-hc__Steam-appimage/` — **all vendored**. Read them for the
container boundary, not for a two-loader bundle: none of them has one.

### Rung 7 · Container tooling. ⭐ THE FIELD DID ATTEMPT IT, AND LABELS IT "VERY BROKEN"

`distrobox`, `podman`, `docker`, `lilipod`.

⛔ **CORRECTED 2026-09-04b, and the correction is the useful part.** This page
said *"the vendored issue set contains no attempt to package it"* and called
this *"the row the field is on record failing"*. Both were artefacts of not
having fetched the repository. `pkgforge-dev/distrobox-AppImage` **is now
vendored**, and it is a real, sharun-built distrobox+podman AppImage:

| | |
|---|---|
| releases | **2**, `1.8.2.3-1`, x86_64 **and** aarch64, with download counts |
| what it packs | `distrobox*`, `podman*`, `/usr/lib/podman`, `conmon`, `crun`, `krun`, `criu` |
| ⛔ its own repository description | **`WIP (VERY BROKEN!)`** |

⭐ **So the honest sentence is not "they failed" and not "they succeeded": they
shipped it and call it very broken.** What makes the rung worth doing is that
their build script names **three costs in its own comments**, and each is a bar
we can be measured against:

1. ⛔ **The bundle writes itself outside the mount to work.**
   `AppDir/bin/run-outside.src.hook`: *"for some reason this fails to work when
   FUSE or unshare is in use: `Error: default OCI runtime "crun" not found`…
   so we will have to copy the binaries outside FUSE before running them"* —
   into `$XDG_CACHE_HOME/distrobox-appimage`. ⭐ **Nothing written to the
   filesystem is one of our measured parity rows**; this route gives it up.
2. ⛔ **`crun` is `patchelf --set-interpreter`'d to `/tmp/<random>`**, and a
   generated shell wrapper copies the loader there at run time. Their comment:
   *"crun gets broken when used with sharun … we will have to do some hacks"*.
   ⚠ The random name is generated at **build** time and baked into the wrapper,
   so it is a fixed path per artefact — ⛔ **the `/tmp` route
   [`../design/store-paths.md`](../design/store-paths.md) §2 rejects on
   security grounds, holding the dynamic loader, which is loadable code.**
3. ⚠ **It needs `PATH_MAPPING`** — `'/usr/bin/distrobox*:${SHARUN_DIR}/bin/distrobox*'`
   — which is a path-mapping mechanism they do have, and worth reading against
   ours before the rung is attempted.

⭐ **That turns the Prove bar into something sharper than "can we do it at
all"**: at least one of distrobox/podman working, rootless, **without writing
outside the artefact and without a `/tmp` loader copy**. The operator's bar
(*"rootless is fine"*) stands; these two are what would make it a result rather
than a re-run of theirs.

**What it actually needs**, and each is a separate finding:
- `newuidmap`/`newgidmap` are **setuid host binaries**; a rootless podman
  without them is limited to a single uid. A bundle cannot ship setuid.
  ⭐ **AND THIS CLASS IS NOW MEASURED, on `lilipod`** — `experiments/100-` arm
  L. The static binary **executed on 11 of 11** and the **application
  completed on 2**, every failure naming *"failed to find dependency
  `getsubids`"*, a shadow-utils **host program**. Exactly the two rootfs that
  carry `getsubids` are the two that completed. ⛔ So for this rung the blocker
  is confirmed to be host **programs**, not host libraries — which is the one
  thing neither static linking nor bundling addresses.
- `distrobox` is a **shell script suite**, so rung 1's script entry point and
  the `PATH` it assembles are the whole problem.
- `lilipod` is a **static Go binary** — so it is also a rung-2 subject, and it
  is the last resort precisely because it asks the least of us.

⚠ **The Anylinux issue set is still silent on it** — every one of the ~40
`distrobox` mentions in `api/issues.json` is distrobox used as a **test
environment** (*"tested on distrobox alpine"*), never as a subject. ⛔ That
absence is now known to mean the work happened **in its own repository**, which
is why an absence is not a zero: it was evidence about where to look, and this
page read it as evidence about whether the work existed.

**Study.** `references/pkgforge-dev__distrobox-AppImage/` — **vendored**.
`make-appimage.sh` (the `crun` hack, the `quick-sharun` list, `PATH_MAPPING`)
and `AppDir/bin/run-outside.src.hook` (the copy-outside-FUSE workaround, with
their reason in the first comment).

### Rung 8 · The heavy tail

`blender` (Python + GL + ~1 GB), `virtualbox` (needs host kernel modules —
bundling cannot supply them, and that limit should be *stated*, not discovered),
`filezilla` (**wxWidgets, which the field's Hall of Fame does not grade at
all** — a genuinely unknown toolkit and worth one row for that reason alone),
`imagemagick` (delegates via `delegates.xml`, absolute paths in a text file —
rung 1 plus the text rewrite).

---

## The list, as a table

| subject | category | rung | what it exercises first |
|---|---|---|---|
| `rnote` → `rnote rnote-cli` | GTK4 + Rust | 1 | multi-entry dispatch |
| `nicotine-plus` → `nicotine nicotine-plus` | Python GTK | 1 | multi-entry + Python |
| `mkvtoolnix`, `mkvtoolnix-cli` | Qt + CLI | 1 | multi-entry across two attributes |
| `stirling-pdf`, `stirling-pdf-desktop` | JVM + webview | 1 | two attributes, one application |
| `imagemagick` | CLI suite | 1 | multi-entry + `delegates.xml` text paths |
| `lmms` (and `lmms` full — ⚠ **does our walker tell them apart?**) | audio | 1, 4 | multi-entry + LADSPA/GStreamer |
| `syncthing` | Go, static | 2 | ⭐ the NOT-MEASURED interposer row |
| `powershell` | .NET | 2 | relative-path runtime, no PLT to win |
| `bleachbit` | Python GTK | 3 | GTK locale prefix |
| `pinta` | GTK + .NET | 3 | GTK locale prefix + .NET |
| `pdfarranger` | Python GTK | 3 | already in `65-` as `py-2` |
| `gnome-maps`, `-chess`, `-music`, `-calendar`, `-disk-utility` | GNOME | 3 | the locale prefix, five ways |
| `flameshot` | Qt + X11 grab | 3 | already in `65-` as `field-3`; screenshot needs a real X grab |
| `remmina` | GTK + FreeRDP | 4 | plugin `dlopen` out of a bundled tree |
| `openscad` | Qt + OpenGL | 4 | GL through the software rasteriser |
| `mpv` | media | 4 | already in `65-` as `media-1` |
| `handbrake` | media | 4 | ffmpeg + GStreamer |
| `dosbox`, `dosbox-staging`, `dosbox-x` | SDL | 4 | already in `65-` as `sdl-1`; three forks, one toolkit |
| `brave`, `firefox`, `google-chrome`, `signal-desktop` | browser/Electron | 5 | ⛔ namespaces — bed change first |
| `wine`, `wine64`, `bottles`, `winboat` | Windows compat | 6 | `lib32` and two loaders |
| `virtualbox` | virtualisation | 8 | ⛔ host kernel modules — state the limit |
| `blender` | 3D | 8 | Python + GL + size |
| `filezilla` | **wxWidgets** | 8 | ⭐ a toolkit the field has never graded |
| `steam` | game platform | 8 | `lib32` + self-update into `$HOME` |
| `distrobox`, `podman`, `docker`, `lilipod` | containers | 7 | ⭐ the row the field fails |

## The Prove bar, and the stubs it permits

⭐ **Operator, 2026-09-04**: *"All three apps open, run and work, use
emulated/dummy stubs for hw gaps."*

- **Open** — a toplevel window ≥50×50 on a real X server, seen from outside the
  process with `xwininfo` (`experiments/65-`'s `windows_real`).
- **Run and work** — an assertion the *application* answers: a CLI's own version
  string, a rendered vendor string, a translated label. ⛔ Never a log line that
  a broken bundle also prints.
- **Stubs are permitted for hardware and only for hardware**: `Xvfb` for the
  display, `llvmpipe`/`lavapipe` for GL and Vulkan, a dummy ALSA/PipeWire sink
  for audio. ⚠ A stub is a stated limit, not a pass: a software rasteriser says
  nothing about a real GPU (`experiments/65-` C3).

## ⛔ What this bed cannot answer, stated up front

| | why |
|---|---|
| a real GPU, NVIDIA | no device on this machine. T-059 |
| an unprivileged user namespace | ⭐ **the cause is `chroot`, not the bed** — `experiments/69-`: the same rootfs entered by `pivot_root` permits it. Until that change is taken and its isolation measured, a browser row measures `--no-sandbox`. Rung 5 |
| ⛔ **a non-C locale** | **0 of 11** environments have one compiled — only `C.UTF-8`, and the three Alpines and Void have nothing at all. So `setlocale` fails, `LC_MESSAGES` stays `C`, and gettext opens no catalogue: rung 3's locale criterion cannot fire. ⚠ Six of the eleven *do* carry `share/locale/de` message catalogues, which is what makes the absence easy to miss |
| ⛔ **a host program a subject shells out to** | `experiments/100-` arm L: `lilipod` needs `getsubids`, which **2 of 11** carry. Neither static linking nor bundling supplies another program — rungs 2 and 7 |
| a kernel module (`virtualbox`) | the bundle cannot supply one, ever |
| Wayland-only behaviour | no compositor here; `Xvfb` is X11 |
| a setuid helper (`newuidmap`) | a bundle cannot ship setuid — rung 7 |

## Sources, and which are on disk

⭐ **Vendored, so read them from the tree first** —
`references/pkgforge-dev__Anylinux-AppImages/`:
`tree/HALL-OF-FAME.md` (per-toolkit grades), `tree/FAQ.md`,
`tree/HOW-TO-MAKE-THESE.md`, `tree/useful-tools/hooks/*` (fifteen hooks, each a
named workaround), `tree/useful-tools/demo/*` (eleven minimal per-toolkit
AppImage recipes — ⭐ the fastest way to see what a toolkit needs), and
`api/issues.json` + `api/comments.json` — **825 issues and 1,000 comments**,
which is where the per-application knowledge actually is. ⚠ Search that JSON;
the operator is right that the specific issue comments are the best source.

⭐ **NOW VENDORED, 2026-09-04b** — six of the nine the operator named:
`pkgforge-dev/distrobox-AppImage` (rung 7), `VHSgunzo/lux-wine` and
`VHSgunzo/lw-runtime` and `ivan-hc/Steam-appimage` (rung 6),
`VHSgunzo/runimage-nvidia-drivers` (T-059), `VHSgunzo/Run-wrapper` (rung 1).
`ivan-hc/AM` was already here. `references/` is **52** trees
(`ls references/ | wc -l`).

⭐ **AND SO ARE THE LAST THREE.** `flatimage/flatimage`, `gameimage/gameimage`
and `flatroot/flatroot` are **vendored** too — the owner names this page
already carried were right (each is an organisation with a same-named
repository), and all three resolved on the first attempt. `references/` is
**55** trees.

⭐ **What the six that arrived actually said**, so the next session does not
re-read them for the same answer:

| tree | what it is | what it changes here |
|---|---|---|
| `distrobox-AppImage` | a real sharun distrobox+podman AppImage, 2 releases | ⛔ rung 7 rewritten: they attempted it, and call it `WIP (VERY BROKEN!)` |
| `lux-wine`, `lw-runtime`, `Steam-appimage` | **RunImage containers**, plus a Wine library tarball | ⛔ rung 6 rewritten: the field's 32-bit answer is a container, not a bundle |
| `runimage-nvidia-drivers` | builds a driver **image per NVIDIA version** from the vendor `.run`, placed beside the container | ⚠ T-059: it is per-version and host-matched, which is what [`../design/host-fallback.md`](../design/host-fallback.md) already says a driver must be |
| `Run-wrapper` | a 2-star Rust ELF wrapper that dispatches argv into RunImage's `Run.sh` | ⚠ rung 1: the same job as `pgb-apprun.c`, for a **shell script** target. Ours is static C with the dispatch table measured (`experiments/68-`); there is nothing here we need |

## ⭐ The question this page asked about the runtime projects, answered

It asked *"whether any of them handles a case the farm cannot"*, not whether
they are nicer. Read 2026-09-04b. **Two do, and one of them is a case we are
currently describing wrongly.**

| tree | what it actually is | a case the farm cannot |
|---|---|---|
| `flatimage` (C++, 124★) | a **portable root** in one ELF — sandboxed by default, DwarFS, *"all config embedded in the ELF binary's reserved space"*, statically linked with embedded tools | ⭐ **yes.** A full root serves a program with **any** absolute path compiled in. Our interposer serves `/nix/store/…` and nothing else — see below |
| `flatroot` (Rust, 6★) | ⭐ **not a bundler at all**: it builds distribution rootfs trees from official mirrors, *"without root privileges or a running package manager"*, ten distributions across deb/rpm/pacman/apk, one static binary, pinnable to historical snapshots | ⚠ **not our rung** — it is `pgb rootfs`'s problem, from mirrors where ours goes through OCI. Relevant to **T-051/T-060** (a host with no compiler) and to the bed, not to bundling |
| `gameimage` (Rust, 342★) | a **FlatImage** game packer — a front end over the row above | no; it inherits flatimage's answer |
| `Run-wrapper` (Rust, 2★) | an ELF wrapper dispatching argv into RunImage's shell `Run.sh` | no — the same job as `pgb-apprun.c`, for a **script** target |

⛔ **AND THE FIRST ROW EXPOSES AN IMPRECISION ON THIS PAGE, in rung 3.** That
rung says the field's *"hardcoded at the prefix (`/usr/share/locale`)"* is
*"the exact shape T-081's interposer resolves"*. ⚠ **It is not.**
`tool/runtime/pgb-storefix.c`'s `fix()` returns the path **unchanged** unless it
begins with `/nix/store/` — a literal `/usr/share/locale` is not rewritten and
never was.

⭐ **The mechanism still applies to our pipeline, for a reason worth stating
rather than glossing**: a nixpkgs-built GTK application does not compile in
`/usr/share/locale`; it compiles in *its own store path*, which is exactly what
the interposer answers. So the rung is real — but what we resolve is **our**
form of the field's problem, not theirs, and a distro-built binary with
`/usr/share/…` in its `.rodata` is a case the farm does not cover. ⛔ Say it
that way; the stronger version is not what the source does.

⚠ **`flatroot` also answers `experiments/69-` from the other side**: it replays
post-install scripts *"inside an unprivileged user-namespace sandbox"* — the
call our chroot bed refuses and `pivot_root` permits. A working example of the
route, in a static binary, is worth reading before changing the bed.
