# poc — harder applications, until something breaks

⭐ **This is priority one.** The way static glibc gets pushed further is by
building progressively harder software with `pgb`, watching it fail, and fixing
`pgb` or `tool/runtime/*.c` until it does not.

⚠ **Open entries only.** The 3 closed ones are
[`../HISTORY/entries/poc.md`](../HISTORY/entries/poc.md); the long-form
findings behind the entries below are
[`../HISTORY/entries/poc-open.md`](../HISTORY/entries/poc-open.md).

---

## T-054 — kdenlive, static: exhaust it

**Source** ⭐ **operator, 2026-09-01c**: *"is there something about static
kdenlive in our research, did we actually build/prove it was possible to build
it statically?"* — and then: *"poc a kdenlive static (exhaust all resources)"*.
**Category** poc · **Priority** P1 · **Effort** L · **Status** open

⭐ **Where it stands, rung by rung.**

| rung | what | state |
|---|---|---|
| — | kdenlive's **ENGINE**: ffmpeg 7.1 (142 MB `libavcodec.a`), MLT 7.30.0, a 105 MB static `melt` with eight `dlopen`'d modules compiled in | ⭐ **DONE** — renders a real MP4 on **11 of 11, zero host shared objects** (`poc/80-mlt`) |
| 1 | a static Qt 6 widget program on the eleven | ⭐ **CLOSED** (`poc/90-qt`) |
| 2 | a static Qt 6 application opening a real window | ⭐ **CLOSED** (`poc/91-qt-xcb`) |
| 3 | KF6 | ⛔ **open — and it is TWO direct inputs, not a framework set**: `kio-extras` and `qqc2-desktop-style`. The sprawl is transitive: `kio-extras` pulls `kio`, which pulls much of KF6 |
| 4 | kdenlive itself | ⛔ **open** |

⭐ **Rung 4's direct demand is measured: 13 buildInputs, 8 native.** kdenlive
26.08.0 wants `qtbase-6.11.1` — ⭐ **the exact version rungs 1 and 2 already
proved** — plus `qtsvg`, `qtmultimedia`, `qtnetworkauth`, `qtimageformats`
(none attempted); `ffmpeg-full-9.0`, `mlt-7.40.0`, `ffmpegthumbs` (⚠ ours are
**older** — 7.1 and 7.30.0 — and the version drift is real work); and
`KDDockWidgets-2.4.1`, `v4l-utils`, `opentimelineio-0.18.1`, untouched.

⛔ **"Qt/KF6 are impossible then" — no.** `poc/80-mlt` says **NOT ATTEMPTED**,
which is not a failure: there is no error, no log and no rung that stopped.
Qt 6 supports `-static` upstream as a documented configuration.

⛔ **AND THE RUNG-4 PLAN NEEDED NIX.** `pgb nix plan kdePackages.kdenlive`
reported `no nix-free route resolved … falling back to evaluation` — the
nix-free route reaches `kdenlive` but not the dotted `kdePackages.` attribute.
⚠ So that measurement is **not reproducible on a host with no nix**. T-060
owns the gap.

⭐ **Cost bound, taken free 2026-09-03c**: Qt with xcb, TLS, network and SQL is
**under half an hour on four cores**, measured twice by the POC suite. The
"hours, not minutes" line this entry inherited is wrong.

**Prove.** A kdenlive binary, or a named rung with the log of what stopped it.

📚 [detail](../HISTORY/entries/poc-open.md)

## T-055 — If static will not reach it, a kdenlive bundle that BEATS the field

**Source** operator, 2026-09-01c: *"if impossible, pivot to
kdenlive.nixappimage, but it must be smaller, load faster, run faster than
pkgforge-dev/kdenlive-AppImage-Enhanced"*.
**Category** poc · **Priority** P1 · **Effort** L · **Status** open

⛔ **THE BAR IS A COMPARISON, NOT A BUILD**, and the comparison exists —
`experiments/90-kdenlive-vs-enhanced.sh`, kdenlive 26.08.0 on both sides.
⛔ **The bar is NOT met.**

| column | P — ours, one command | E — `kdenlive-AppImage-Enhanced` | | under the 2026-09-03c ruling |
|---|---|---|---|---|
| size | 397,903,295 B | 191,900,604 B | 2.07× | ⭐ **acceptable** |
| render (melt → a real MP4) | **3,625 ms** | 2,001 ms | ⛔ 1.81× | ⛔ **binding** |
| start, cold | **3,344 ms** | 1,325 ms | ⛔ 2.52× | ⛔ **binding** |
| start, warm | **139 ms** | 34 ms | ⛔ 4.1× | ⛔ **binding** |
| runs on the eleven | 11 of 11 | 11 of 11 | equal | |
| the MP4 | 4,149 B, 48 frames, libx264 | 4,162 B | equal | |

⭐ **What IS established, and it is the half T-054 could not reach**: a
kdenlive that renders, produced by **one command from a package name**, on
eleven distributions including four musl ones — engine, Qt stack, KF6, MLT and
ffmpeg all from the nixpkgs closure with no nix installed. ⭐ Under the ruling,
one-command packaging is now half the bar and this meets it.

**What is left, re-ordered by the ruling** (size struck, clock binding):

1. ⛔ **The clock is the bar now.** Start and render are dominated by mounting
   a 398 MB dwarfs image against a 192 MB one — **the size column IS the time
   column here**, which is the one place the struck size work still scores.
   ⚠ Nobody has measured that claim; it is an inference from the numbers.
2. **`--debloat aggressive`** is untried on this artefact and removes ~90 MiB
   of Vulkan ICDs for GPUs this architecture has (`experiments/89-`: 0.78× on
   a GL bundle).
3. **`share/` is 368 MB**, most of it `breeze-icons` (108 MB). A debloat rule
   for unused icon sizes is the biggest single lever left.
4. **The `store/` shard is still 405 MB**; de-duplicating what is already in
   `lib/` would help again.

⚠ **The honest risk, named:** the competitor is hand-crafted per application
by people who do this full time. Beating it **by automation** is the claim
worth making and it is not the same claim as beating it at all.

**Blocked on** T-054 answering first, and on T-052/T-059 for OpenGL.

📚 [detail](../HISTORY/entries/poc-open.md) — including the two defects this
comparison found in our own bundle (MLT's compiled-in module directory; the
store shard copying whole packages) and the one in the measurement (a
multi-program bundle needs a selector, a selector is a shell script, and a
script is run by the HOST's `/bin/sh`).

## T-063 — miniflux with an embedded PostgreSQL, against onelf's ~70 MB

**Source** operator, 2026-09-02: *"prove pgb can build something as complex as
this"*, naming onelf's miniflux guide.
**Category** poc · **Priority** P1 · **Effort** L · **Status** open

⛔ **THE SUBJECT IS CHOSEN FOR WHAT IT IS NOT: ONE PROGRAM.** Two programs,
five postgres helpers, a share tree, `dlopen`'d extensions, a `$libdir`
computed at RUN TIME, and ⛔ a **shell orchestrator** as the entry point —
which is the thing pgb's whole argument is against.

⭐ **The whole difficulty is PostgreSQL.** `pgb nix plan miniflux` reports
`buildInputs: []` — miniflux 2.3.3 is pure Go and already a static ELF.
`postgresql` reports **16 buildInputs**.

⭐ **ARM S IS NOT "NO", and 2026-09-03c took it further.**

    src/backend/postgres   statically linked, PT_INTERP absent, DT_NEEDED 0
    ./postgres --version                             -> PostgreSQL 18.6
    pgb rootfs run alpine-3.22 -- /postgres --version -> PostgreSQL 18.6

⭐ **And now WITH ICU** — the C++-archive fix was proved to reach the real
subject on 2026-09-03c (3,911 `icu_78` symbols, `PT_INTERP` 0), where before
it only held on a synthetic one.

**What is left.**

1. ⚠ **`src/interfaces` (libpq, ecpg — the CLIENT libraries) still fails**, so
   the `make install` that would give `initdb`, `pg_ctl`, `psql`, `createdb`
   and `createuser` did not complete. ⛔ Nothing yet claims the miniflux stack
   runs.
2. ⛔ **readline is not missing — the probe is.** `libreadline.a` and
   `libncursesw.a` are both in the prefix; `AC_SEARCH_LIBS` probes
   `-lreadline` alone and the archive's ncurses references go unresolved.
   ⚠ **`--start-group` is NOT the fix and this row once said it was** —
   measured: grouping fixes ORDER, it cannot fix ABSENCE.
   ⭐ **The real fix is the shape ICU already uses**: read the archives and
   append what they need. `elfx.NeedsCXXRuntime` is that mechanism
   specialised to one target; generalising it needs a symbol index over the
   candidate archives, which `elfx.DefinedExternalSymbols` already builds for
   `--wrap-dlopen`. ⛔ Not built, and nothing has measured what it costs on a
   real `configure` run.
3. ⚠ **The round budget.** The default is 8 and this plan needed
   `NIX_MAX_ROUNDS=24`. A budget below the number of optional features a
   distro plan enables reads as *"this package cannot be built"*.
4. ⛔ **The entry point.** Whether pgb can do this with **no shell in the
   delivery path**, and the honest answer may be "not yet".

**Prove.** `evidence/poc/92-miniflux/` — the stack serving HTTP on
`127.0.0.1:8080`, or the named rung with the log of what stopped it.

📚 [detail](../HISTORY/entries/poc-open.md)
