# Compiled-in store paths, and the route taken to resolve them

⛔ **This page answers the security question the operator required to be
answered BEFORE anything was built**, and the answer is **no**. It then names
the route taken instead, and the boundary of that route.

[`../../TODO/toolchain.md`](../../TODO/toolchain.md) T-081 is the entry;
[`../research/bundle-capabilities.md`](../research/bundle-capabilities.md) §0 is
where the cost was measured.

---

## 1. The problem, measured rather than asserted

A nixpkgs program compiles its own store path into `.rodata`:

    $ strings AppDir/shared/bin/galculator | grep /nix/store
    /nix/store/<32-char hash>-galculator-2.1.4/share/galculator

`galculator` then opens `<that>/ui/main_frame.ui`, which the bundle **carries**
at `AppDir/share/galculator/ui/main_frame.ui` — and dies, because the path it
names does not exist on the target.

`experiments/64-` measured exactly this on 2026-09-03e, with a positive
control:

| arm | subject | window on a real X server |
|---|---|---|
| G | `galculator` — UI is a file at a compiled-in store path | ⛔ 0 of 11 |
| X | `mousepad` — UI is a GResource compiled into the binary | ✅ 11 of 11 |
| C | `galculator` again, with that store path made to resolve | ✅ 11 of 11 |

⭐ Arm C is why *"a hardcoded store path is what stops it"* was a measurement
rather than a reading of an error message. ⚠ Arm C's mechanism — binding the
bundle's own `AppDir` at the store path with a mount namespace — **was never a
fix**: it needs root, which a user double-clicking an AppImage does not have.

⚠ **Arm C is RETIRED**, and the experiment now runs arm **N** in its place:
the same bundle built with `--no-storefix`, the one mechanism absent. Arm C
answered *"is the store path the cause"*; arm N answers *"is the mechanism
what fixed it"*, which is the question once there is a mechanism.

---

## 2. ⛔ THE SECURITY QUESTION, AND THE ANSWER IS NO

**The candidate.** A rewrite inside an ELF cannot *lengthen* a string —
`.rodata` is fixed and everything around it is at a fixed offset. But
`/nix/store/` is 11 bytes and so is `/tmp/.pgbs/`, so a same-length prefix
substitution needs no relocation, no section resize and no `patchelf`. The
launcher then makes that one directory resolve.

⛔ **Rejected.** A fixed, predictable path under a world-writable directory is
squattable by any local user, and both branches of the resulting design are
unacceptable:

| the launcher… | what a local attacker gets |
|---|---|
| **follows** `/tmp/.pgbs` when it already exists | an arbitrary-write primitive as the victim: the launcher populates a tree at a location the attacker chose, by pre-creating `/tmp/.pgbs` as a symlink |
| **refuses** when `/tmp/.pgbs` is not ours | a permanent, one-command denial of **every** `pgb` bundle for **every other user** on the machine — `mkdir /tmp/.pgbs` |

⭐ **And the read side is worse than a data leak, because the data is code.**
The tree this path serves is a program's `share/` directory. A GTK `.ui` file
names GModules to load; a `gdk-pixbuf` `loaders.cache` names shared objects to
`dlopen`; a `gtk-3.0/settings.ini` names a theme engine. An attacker who
controls that directory controls what the victim's process loads. This is not
"a stale copy" — it is **arbitrary code execution**, and
[`host-fallback.md`](host-fallback.md)'s rule (*never prefer a stale or
attacker-controllable copy*) forbids it twice over.

⚠ **The safe-`mkdir` dance does not rescue it.** `mkdir(0700)`, `EEXIST` →
`O_NOFOLLOW` open → verify `st_uid == geteuid()` and no group/other write, is
the correct sequence and it does close the write primitive. What it cannot
close is the second row: the name is fixed, so refusing is the only safe answer
and refusing is a denial of service that any user can trigger and no user can
clear.

⭐ **A per-uid name does not rescue it either**, and the reason is
structural: the uid is not known at bundle time, and the string being rewritten
is fixed at bundle time.

**So: no `/tmp` store, at any name.**

---

## 3. The route taken: an interposer, exact-match against the closure

⛔ **The route is NOT "nicer regexes".** `pgb` computes the closure, so it knows
the exact, finite set of store paths that are in the bundle. Every rewrite is
an **exact match against that set**; a store path that is not in it is
**reported as a finding**, never silently substituted.

    tool/runtime/pgb-storefix.c   the interposer
    internal/bundle/storefix.go   the map, the farm, the ABI check, the report
    AppDir/lib/libpgb-storefix.so where it ships
    AppDir/.storemap              the table, generated from the closure
    AppDir/.preload               sharun's own preload mechanism
    AppDir/store/<name>-<ver>     one directory per store path in the closure

⭐ **The directories are SYMLINKS, not copies.** The bundle already holds every
file once, flattened — libraries in `lib/`, programs in `shared/bin/`, every
`share/`, `etc/` and `libexec/` tree merged — so `store/.root` carries one
symlink per top-level name and every store path points at it. A closure of 130
store paths costs 130 symlinks, not 130 copies of the tree.

⛔ **The ABI is checked, not hoped.** The interposer is compiled by the build
host's compiler against the build host's glibc; if that glibc is NEWER than the
closure's, the target's loader refuses the object and the bundle simply does
not run. `checkStorefixABI` reads every versioned symbol the object imports out
of its `.dynsym` and asserts the bundle's own `libc.so.6` defines it, at that
version. ⚠ A musl closure is refused outright and said so, rather than failing
at run time with a message about a symbol.

**How it works.** `Anylinux-sharun` reads `.preload` from the bundle root and
passes it to the loader as `--preload` (`src/main.rs`, `read_preload` in
`src/utils.rs`). The interposer is loaded before libc, so every call the
application and its shared libraries make **through the PLT** reaches it first.
It rewrites a path whose store-path component is in the table to the
corresponding location inside the bundle, and passes everything else through
untouched.

⭐ **Why this and not the binary rewrite.**

| | the interposer | rewriting `.rodata` |
|---|---|---|
| needs a writable fixed path | **no** | yes — §2 |
| modifies the application binary | **no** | yes |
| handles a path **assembled at run time** from a shorter compiled-in prefix | **yes** | only if the prefix itself is a literal |
| handles a store path inside a **script** or a data file | yes — and those are rewritten at build time as well | no |
| works for a **static** binary, or one issuing raw syscalls | ⛔ **no** | yes |

⛔ **The last row is the boundary and it is NOT MEASURED.** No subject in
`experiments/64-` or `65-` is statically linked or issues raw syscalls, so this
row is reasoning about a mechanism rather than a result. ⚠ What WOULD measure
it: a Go program out of the closure, bundled and run with a store path
compiled in — `pgb`'s own binary is exactly that shape. Until somebody runs
it, the honest statement is that the interposer has no PLT to win there and
that nothing has confirmed the consequence.

## 4. ⛔ What the interposer does NOT cover

**An absence is not a zero. Where this was looked for, and where it was not:**

- **Interposed:** the path-taking entry points the application and its shared
  libraries reach through the PLT — `open`/`openat` and their `64` forms,
  `fopen`/`fopen64`/`freopen`, `stat`/`lstat`/`fstatat` and the pre-2.33
  `__xstat` family, `statx`, `access`/`faccessat`, `opendir`, `readlink`,
  `realpath`, `execve`/`execv`, and `dlopen`. The list is the definitions in
  [`../../tool/runtime/pgb-storefix.c`](../../tool/runtime/pgb-storefix.c) and
  nowhere else.
- ⛔ **Not interposed: glibc's own internal calls.** `fopen` reaches
  `__open64_nocancel` inside libc without going through the PLT, so
  interposing `open` alone does not catch it — which is why `fopen`, `opendir`,
  `realpath` and friends are interposed *by name* rather than left to the
  syscall layer.
- ⛔ **Not covered: a statically linked program**, and **not covered: a program
  issuing raw syscalls** (a Go binary is the common case). Neither has a PLT to
  win. For those the bundle reports the compiled-in path as a finding and the
  application is expected to fail — which is the honest outcome, and it is
  visible rather than silent.
- ⛔ **Not covered: a path the program derives by string arithmetic** that does
  not begin with a store path (none observed; not searched for exhaustively).

## 5. The build-time half, which is the part that must not guess

Three rewrites happen at build time, all against the same closure set:

1. **A script entry point becomes interpreter + script.** The shebang is
   resolved against the closure at build time, so the script's own text needs
   no rewriting at all — a store path inside it reaches libc through the
   interpreter and the interposer answers it. ⛔ A shebang naming a HOST
   interpreter (`#!/bin/sh`) is refused rather than adopted: running the
   host's shell puts the host's libc in the process.
2. **`.env`** — `${SHARUN_DIR}` is what sharun expands, so a store path in a
   lifted wrapper variable becomes one.
3. **`.desktop` entries** — `Exec=` and `TryExec=` are rewritten to the
   bundled program, `Icon=` to the icon the bundle actually carries at its top
   level, and `DBusActivatable` is **removed**: a bundle cannot be D-Bus
   activated, so a launcher that believes it waits for a service that never
   appears. ⚠ Nothing here writes `${SHARUN_DIR}` into a desktop entry — a
   file manager expands nothing — so a store path that survives is reported.
4. **Manifests** — the ICD and vendor JSON already handled by
   `rewriteManifestPaths`, kept on the same list so the two rules cannot
   disagree about which files matter.

⛔ **And everything left over is a report.** `pgb bundle appimage` prints the
store paths it found compiled in, which of them resolved, and which did not —
so a bundle that will fail says so at build time instead of at the user's
double-click.
