# The battle-test corpus, classified by the mechanism it needs

⛔ **Set by the operator, 2026-09-04**: a list of applications *"to battletest
and prove our bundler is best in class"*, with the instruction to **sort the
tasks by what will auto-fix or complete what — not easy first**.

⚠ **This page is research, not results.** Nothing here has been bundled. Each
row says which mechanism a subject exercises and what the field already knows
about it; the measurement is [`TODO/research.md`](../../TODO/research.md) T-087
and the sessions after it. `experiments/65-` is the harness.

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

**What is still missing: the bundle half.** Arm S is a synthetic AppDir with
host stand-in programs, so it says nothing about sharun, uruntime, the closure,
or host shared objects. ⛔ **No experiment has yet run a second program out of
a real bundle** — that is `68-` arm B, which needs the bed.

**Study.** `internal/bundle/assemble.go` `installProgram`;
`tool/runtime/pgb-apprun.c` (130 lines, and it is the answer);
`experiments/68-` arm S for what each rule actually does.

**Study.** `internal/bundle/assemble.go` `installProgram`;
`tool/runtime/pgb-apprun.c` (the whole file is 130 lines and it is the answer).

### Rung 2 · A static or raw-syscall binary. ⭐ THE ONE ROW `store-paths.md` MARKS "NOT MEASURED"

`syncthing` (Go), `powershell` (.NET), `lilipod` (Go).

⛔ **This is not a low-priority "easy" row, it is a named hole in a shipped
claim.** [`../design/store-paths.md`](../design/store-paths.md) §3's comparison
table has one row marked **NOT MEASURED**: the interposer *"works for a static
binary, or one issuing raw syscalls — **no**"*, because there is no PLT to win.
A Go program from the closure with a store path compiled in is exactly that
shape, and `pgb` itself is one. Until a Go subject is bundled and run, that row
is reasoning.

**Expect it to FAIL, and pre-register that.** The honest outcome is that the
build **reports** the compiled-in path and the program cannot resolve it. If it
passes, the reasoning was wrong and the record says so.

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

⛔ **What is still open is the application half**, and it is arm G: a real Go
subject is usually *both* shapes at once, and a store path in its `.rodata`
does not mean it ever opens one. Its result is **reported, not predicted**.

**Study.** `docs/design/store-paths.md` §3 and §4; `experiments/100-` arm P.

### Rung 3 · GTK's hardcoded prefix. ⭐ THE FIELD'S OWN "GARBAGE" ROW, AND OUR INTERPOSER IS THE ANSWER

`gnome-maps`, `gnome-chess`, `gnome-music`, `gnome-calendar`,
`gnome-disk-utility`, `pinta`, `rnote`, `bleachbit`, `flameshot`.

The field's `HALL-OF-FAME.md` opens its GTK section with: *"Every single GTK app
has the path to its locales hardcoded at the prefix (`/usr/share/locale`) and
there is no env variable to change this."* ⭐ **That is a compiled-in absolute
path with no search variable — the exact shape T-081's interposer resolves**,
and the exact shape `--embed-terminfo`-style variable redirection cannot.

⛔ **So this rung is the differentiator, and it should be measured as one**: a
GNOME app under a non-English `LANG`, asserting a translated string, against the
same bundle built `--no-storefix`. A window is not enough here — the window
appears either way.

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
We emit one of the four and none of the scanner. And *"`gst-plugin-scanner`
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

**Study.** `https://github.com/VHSgunzo/lux-wine` and
`https://github.com/VHSgunzo/lw-runtime` (the operator's links; **not
vendored** — fetch before relying on either);
`https://github.com/ivan-hc/Steam-appimage`.

### Rung 7 · Container tooling. ⭐ THE ROW THE FIELD IS ON RECORD FAILING

`distrobox`, `podman`, `docker`, `lilipod`.

⛔ **The operator's framing is the right one**: *"AnyLinux's AppImage fail to
pack this, can we?"* — so this is the only rung where a green row is a claim the
field cannot match. The Prove bar the operator set is deliberately low: **at
least one of distrobox/podman works, rootless is fine.**

**What it actually needs**, and each is a separate finding:
- `newuidmap`/`newgidmap` are **setuid host binaries**; a rootless podman
  without them is limited to a single uid. A bundle cannot ship setuid.
- `distrobox` is a **shell script suite**, so rung 1's script entry point and
  the `PATH` it assembles are the whole problem.
- `lilipod` is a **static Go binary** — so it is also a rung-2 subject, and it
  is the last resort precisely because it asks the least of us.

⭐ **THE VENDORED ISSUE SET CONTAINS NO ATTEMPT TO PACKAGE IT**, which is
weaker evidence than "they failed" and is what can honestly be said: every one
of the ~40 `distrobox` mentions in `api/issues.json` is distrobox used as a
**test environment** (*"tested on distrobox alpine"*), never as a subject.

**Study.** `https://github.com/pkgforge-dev/distrobox-AppImage` (**not
vendored** — fetch it, and read what it says it cannot do before claiming we
can).

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

⚠ **Named by the operator and NOT vendored** — fetch before relying on them:
`github.com/ivan-hc` (AM, Steam-appimage), `pkgforge-dev/distrobox-AppImage`,
`flatimage/flatimage`, `gameimage/gameimage`, `flatroot/flatroot`,
`VHSgunzo/lux-wine`, `VHSgunzo/runimage-nvidia-drivers`, `VHSgunzo/lw-runtime`,
`VHSgunzo/Run-wrapper`.

⭐ **What to look for in the four unvendored runtime projects**, so the reading
has a question: all four solve *"a program that expects a root filesystem"* —
`flatimage` and `flatroot` with a portable root, `gameimage` with a per-game
prefix, `Run-wrapper` with argv dispatch. `pgb`'s answer is the symlink farm
plus the interposer. **The question is whether any of them handles a case the
farm cannot**, not whether they are nicer.
