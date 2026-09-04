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
selector, built whenever a bundle carries more than one program, dispatching on
`ARGV0` → `argv[0]` basename → `$1` → the default. ⭐ **So renaming or
symlinking the AppImage selects the program, exactly as an AppImage does**, and
`./app.AppImage rnote-cli …` works as a subcommand form.

**What is missing: a measurement.** No experiment has ever run a *second*
program out of a bundle. The claim above is read off the source.

**First task, and it is cheap.** One `experiments/65-` row per extra entry
point: bundle `rnote`, symlink the artefact to `rnote-cli`, run it, assert the
CLI's own output. ⛔ It also answers the question the operator asked — *"can our
nix parser/walker tell how many entry points an app has"* — with a number
printed by the build (`programs <prog> + N more`), not with a reading of the
code.

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

**Study.** `docs/design/store-paths.md` §3 and §4.

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

**Study.** `HALL-OF-FAME.md` "Garbage - GTK";
`references/pkgforge-dev__Anylinux-AppImages/tree/useful-tools/hooks/fix-gnome-csd.hook`
(GNOME draws no server-side decorations, so a GNOME app must draw its own —
a bundle that gets this wrong shows a window with no titlebar, which the
geometry criterion still counts).

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

### Rung 5 · Namespaces and the sandbox. ⛔ NOT MEASURABLE IN THIS BED

`brave`, `firefox`, `google-chrome`, `signal-desktop`, every Electron app,
`bottles`.

⛔ **The bed forbids it.** `unshare(CLONE_NEWUSER|CLONE_NEWNS)` is `EPERM`
inside the chroot bed — already recorded in
[`../comparison.md`](../comparison.md) as the reason every `onelf` row runs in
its last-resort mode. A Chromium sandbox needs exactly that call. ⭐ **So this
rung needs a bed change before it needs a bundler change**, and a row run
without one measures `--no-sandbox`, which is a different program.

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

**Study.**
`references/pkgforge-dev__Anylinux-AppImages/tree/useful-tools/hooks/fix-namespaces.md`
(vendored; the operator's link) and `fix-namespaces.hook` beside it;
`HALL-OF-FAME.md` "Excellent - Chromium/Electron" — the toolkit is easy, the
sandbox is not.

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

**Study.** `https://github.com/pkgforge-dev/distrobox-AppImage` (**not
vendored**).

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
| an unprivileged user namespace | `EPERM` in the chroot bed — rung 5 |
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
