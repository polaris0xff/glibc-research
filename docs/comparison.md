# Comparing the approaches

⛔ **Every cell is either a measurement taken in this repository, or a dash.**
A dash means *not measured here* and never "probably fine".

⭐ **This page used to be almost entirely dashes.** Every non-`pgb` row said
"read from the design, never run", and `REQUIREMENTS.md` part 2 named that as
the single largest gap against the operator's acceptance bar. It is now
measured: `experiments/60-versus-alternatives.sh` builds the **same program**
eight ways and runs every runnable one on the **same 11 environments**.
Evidence: `evidence/60-versus-alternatives/RESULT.txt`, with the per-cell
object lists in `per-environment.txt`.

---

## ⛔ The headline, which is not the one this project wanted

**`pgb` is not strictly better than every existing technique. A static musl
binary ties it on coverage and beats it on startup and size.**

| | `pgb` | static musl |
|---|---|---|
| ran correctly, 11 environments | **11 / 11** | **11 / 11** |
| loaded zero host shared objects | **11 / 11** | **11 / 11** |
| per exec | 980 µs | **160 µs** |
| artefact size | 2,097,824 B | **447,264 B** |
| is glibc | **yes** | no |

⭐ **So the honest claim is a narrower one, and it is the last row.** `pgb`'s
distinguishing property is not that it wins the table — it is that it delivers
those results *for a glibc build*. That matters when the program, or a
dependency you do not control, needs glibc rather than musl: glibc-specific
behaviour and extensions, a prebuilt glibc-linked archive, a build system that
will not cross to musl, `--wrap` onto code compiled long before this tool
existed. Where a project can simply be built against musl, ⛔ **this
measurement says to do that instead.**

⚠ The startup gap is real, not noise. `experiments/40-` established that
`pgb`'s per-exec cost is indistinguishable from plain `gcc -static` — both sit
around 950–1000 µs here, and the difference between *them* is still at the
noise floor. The 160 µs is musl's, and a 6× gap does not need a better
instrument to be believed.

---

## The table

Same source, same pinned `debian:12` build environment for every glibc arm,
same 11 targets. `runs` counts environments where the program's own functional
assertions passed and it did not take a signal.

| approach | runs | host `.so` in the payload | per exec | ship | also needed on the target |
|---|---|---|---|---|---|
| **native dynamic glibc** | 2 / 11 | 4 / 11 clean | 1040 µs | 16,368 B | the matching glibc |
| **plain `gcc -static` (glibc)** | **1 / 11** | 7 / 11 clean | 950 µs | 949,568 B | nothing |
| **`pgb` (this project)** | **11 / 11** | **11 / 11 clean** | 980 µs | 2,097,824 B | nothing |
| **`gcc -static` (musl)** | **11 / 11** | **11 / 11 clean** | **160 µs** | 447,264 B | nothing |
| **AppImage** (type 2) | 2 / 11 | 4 / 11 clean | 4610 µs | 948,728 B | nothing; FUSE only for mount mode |
| **onelf** bundle | 3 / 11 | 8 / 11 clean | 3650 µs | 1,761,241 B | nothing |
| **Flatpak** | **0 / 11** | — | — | 4,228 B | `flatpak` + a 623,352,581 B runtime |
| **snap** | **0 / 11** | — | — | 4,096 B | `snapd`, which needs systemd, + the `core24` base |

⚠ **"Clean" is not "passed".** A binary that cannot start loads nothing and
scores clean; the four musl rows are why `native dynamic` reads 4/11 clean
while running on 2. Read the two columns together, never one alone.

## What each row's failures actually were

**plain `gcc -static`, 1 of 11.** It passes on Debian 12 **only by loading the
host's `ld-linux`, `libc.so.6` and three gconv modules** — five host objects in
a binary `file` calls statically linked. It takes SIGABRT on Debian 11 and
Ubuntu 20.04, SIGFPE on openSUSE Leap, and fails the encoding assertions
everywhere else. ⭐ This is §2 of `AGENTS.md` reproduced by an independent
script.

**AppImage, 2 of 11.** Two distinct failures, and neither is a packaging bug:

- on all four musl targets the runtime starts and the **payload cannot**, because
  an AppImage does not bundle glibc — `execv error: No such file or directory`
  is the missing `ld-linux`;
- on Rocky 8, openSUSE Leap and Fedora 42 it is
  `/lib64/libc.so.6: version 'GLIBC_2.34' not found`.

⚠ **The second one is fixable and the first is not.** AppImage's documented
practice is to build against the oldest glibc you intend to support; this arm
was built in the same pinned `debian:12` as everything else, which sets its
floor at 2.36. An older build host would raise the glibc rows. ⛔ No build host
of any age changes a musl row. Where it does pass (Debian 12, Arch) it does so
by loading the host's libc and the host's gconv modules — the dependency this
project exists to remove.

**onelf, 3 of 11 — and the most instructive row on this page.** onelf bundles
glibc **and its loader**, so the process carries its own libc on every one of
the 11, musl included: the trace shows `ld-linux-x86-64.so.2`, `libc.so.6` and
`libonelf-env.so` opened out of the package's own directory and **not one host
object**, on Alpine 3.10 through Void musl. The delivery works everywhere.

⛔ **It then fails the encoding assertions on 8 of 11, because bundling glibc
does not bundle gconv.** It passes on Debian 11, Debian 12 and Ubuntu 20.04 —
and there the trace shows it reaching *the host's* `/usr/lib/x86_64-linux-gnu/gconv/`
for `EUC-JP.so`, `ISO8859-1.so` and `libJIS.so`. So the three it passes, it
passes by finding host modules whose path happens to match; the eight it fails,
it fails because no such path exists.

⭐ **This is a measured warning for [`design/tiers.md`](design/tiers.md).** Tier
2 is exactly this shape — bundled glibc plus its own loader — and it is the
project's route to the host-plugin class. The measurement says that shape gets
the two-libc property right and **loses the gconv result tier 1 already has**,
unless the `--wrap` onto static libiconv is carried into it. Tier 2 is not a
superset of tier 1 for free.

**Flatpak and snap, 0 of 11.** Both were **built** here — a real `.flatpak`
bundle exported from a real `org.freedesktop.Platform//24.08` runtime, and a
real `.snap` squashfs with its `meta/snap.yaml`. Neither can run on any target
in the matrix, and the reason is measured rather than assumed: ⛔ **0 of 11
images ship `flatpak` and 0 of 11 ship `snap`.** Their deliverables are tiny —
4 KB each — because the weight is on the target: 623 MB of runtime for
Flatpak, and for snap a daemon that requires systemd.

⚠ **Neither was executed even on this machine**, and that is a limit of this
bed, not a finding: `flatpak run` needs a D-Bus session bus, and `dbus-daemon`
cannot start in this container because `cap_sys_resource` is dropped and it
cannot raise its fd limit; `snapd` needs systemd, which is absent. So the
startup and RSS cells for those two rows are dashes and must stay dashes.

## What is and is not measured

| column | state |
|---|---|
| coverage, host objects loaded, size | **measured**, all runnable arms, 11 environments — `experiments/60-` |
| startup, peak RSS | **measured on this host** for N, S, P, M, A, O. ⛔ Not measured for Flatpak and snap: they could not be executed here at all |
| `pgb` vs plain `gcc -static` startup and RSS | **measured** (`experiments/40-`) and the answer is **no measurable difference**: two runs put `pgb` 42 µs then 28 µs per exec above plain static, and its RSS 56 KiB above then 28 KiB **below** — a sign change, so both sit at or under the instrument's noise floor. ⛔ Do not quote either as a figure |
| static musl's startup advantage | **measured**, and unlike the row above it is far outside the noise: 160 µs against 950–1040 µs for every glibc arm |
| onelf in its **preferred** modes | ⛔ **not measured.** Its memfd, FUSE and tmpfs modes all need `unshare(CLONE_NEWUSER\|CLONE_NEWNS)`, which returns EPERM inside this chroot bed, so every onelf row above is its **last** fallback — cache mode, which extracts to disk. Outside the bed on the same host, FUSE and tmpfs both work. Its startup figure is therefore its worst case, and its coverage result is unaffected: the failures are gconv, not delivery |
| Flatpak and snap **runtime** behaviour | ⛔ **not measured.** Needs a machine with systemd and a session bus |
| steady-state runtime | not measured for any arm. `pgb` changes no application code, so there is no mechanism by which it would differ, but that is an argument from structure |
| build time | measured but **not comparable**: `pgb`'s includes entering the chroot build environment, which the other arms do not pay |

⭐ **Why "no measurable difference" between `pgb` and plain static is the
expected result**, which is what makes it credible rather than surprising:
`pgb` adds no process, no loader, no extraction step and no supervising
runtime. The output is an ordinary `ET_EXEC` with no `PT_INTERP` and zero
`DT_NEEDED`, so at run time there is nothing to be slower than a plain static
binary *except* the constructor that calls `__nss_configure_lookup` fourteen
times, and — only with `--embed-locale`, and only when the host cannot answer a
UTF-8 `setlocale` — one directory of files written once.

The 3650 µs and 4610 µs of the two bundle formats are the opposite case, and
equally unsurprising: both decompress a payload and exec through it on every
run.

## Where each row's evidence lives

| row | evidence |
|---|---|
| every row in the head-to-head table | `evidence/60-versus-alternatives/RESULT.txt`; per-cell object lists in `per-environment.txt` |
| plain `gcc -static`, independently | `evidence/20-static-glibc-nss-dlopen/RESULT.txt`, `evidence/30-gconv-and-locale/RESULT.txt` |
| `pgb` startup and RSS against plain static | `evidence/40-overhead/RESULT.txt` |
| `pgb` on real projects | `evidence/poc/*/RESULT.txt` |
| the designs behind each alternative | `references/<name>/`, read at the depth `docs/research/prior-art.md` states |
