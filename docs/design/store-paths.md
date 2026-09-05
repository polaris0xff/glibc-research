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

⚠ **A regex still says where a candidate BEGINS and ENDS, and ours got the end
wrong** — `[^" ']*`, three excluded characters, so in a binary the match ran
through the terminating NUL and in XML through the closing `<`.
[`../history/corrections.md`](../history/corrections.md) C27 has the
measurement. ⭐ **It is the difference between the two routes that contained
the damage**: a mis-bounded match produced a base no closure contains, so the
worst outcome was a path **reported** that should have been rewritten. A route
that substitutes on the regex instead would have written the mis-bounded match
into the file. ⛔ The lesson is not "the regex was fine"; it is that where the
decision lives decides what a bad regex costs.

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
| works for a **STATIC** binary | ⛔ **no — measured** | yes |
| works for one issuing **RAW SYSCALLS** | ⛔ **no — measured, and for a different reason** | yes |

## ⭐ The last row was NOT MEASURED, and measuring it split it in two

⛔ **It was one row and it is two mechanisms.** `experiments/100-` arm P puts
a planted store path — one that does **not** exist on this machine — behind the
real interposer and a real `.storemap`, and runs the *same source* three ways.
**Two runs, identical:**

| probe | our object mapped in it | resolves the path |
|---|---|---|
| ⭐ dynamic, `fopen` through the PLT — **the positive control** | **yes** | ✅ **yes** |
| ⛔ the same source built `-static` | **no** | ⛔ no |
| ⛔ the same source calling `syscall(SYS_openat, …)` | **yes** | ⛔ no |
| ⭐ dynamic with **no preload** — the negative control | no | ⛔ no |

⭐ **So the two halves of the old row fail for different reasons, and only one
of them is about linking.** A static payload defeats the interposer because
there is no loader, so `LD_PRELOAD` never happens and **nothing of ours is in
the process**. A raw-syscall payload defeats it **with our object loaded** —
the call simply leaves through a path the PLT entry we replaced never sees.
⛔ A subject can therefore be perfectly dynamic and still defeat this
mechanism, which the single row could not say.

⚠ **The `MAPPED` column is what makes that a finding rather than a story.**
Both failing probes otherwise report the same thing — "it did not work" — and
the two causes would be indistinguishable without asking each process whether
`libpgb-storefix.so` is in its own `/proc/self/maps`.

⛔ **What is still not measured, and one attempt has already failed to supply
it.** `experiments/100-` arm G bundled `syncthing` expecting a static payload;
its pre-registered shape check reported **`dynamic`** — nixpkgs' build carries
a `PT_INTERP`. ⭐ The subject ran on **11 of 11**, host-object-clean on **11 of
11**, with **8** store paths compiled in and **7** resolving; ⚠ but that is a
*dynamic* subject and says nothing about the `-static` row.

## ⭐ AND THE `-static` ROW IS NOW ANSWERED — by a refusal, which is the answer

⛔ **This section used to say "a genuinely static application is still owed —
`lilipod`, or `pgb` itself".** The evidence for closing it was already in
`experiments/100-` arm L and had not been read back against this row.

⭐ **A FULLY STATIC APPLICATION CANNOT BE BUNDLED AT ALL, AND THE BUNDLER SAYS
SO IN ONE LINE:**

    closure     4 store paths
    libraries   0 from the closure
    pgb: the closure carries no dynamic loader

⛔ **So there is no artefact to ask the interposer question of.** The `-static`
row is not an untested configuration — it is one a `pgb` artefact **cannot
reach**, because the thing that would carry the interposer (a loader, and the
`LD_PRELOAD` that rides on it) is exactly what such a closure lacks.

⭐ **AND THE REFUSAL IS CORRECT, MEASURED RATHER THAN ASSUMED.** Arm L ran
`lilipod`'s raw static binary with **no bundle at all**:

| | |
|---|---|
| the static ELF executes | ⭐ **11 / 11** |
| host shared objects loaded | ⭐ **0 on 11 / 11** |

A statically linked ELF is already portable — that is what static linking buys
— so bundling it would add nothing. ⚠ Arm L also split that result: the
**application** completed on only 2 of 11, for its own reasons (`failed to
find …`), which is a fact about `lilipod` and not about the ELF.

⚠ **WHAT REMAINS, STATED PRECISELY RATHER THAN LEFT AS "STILL OWED".** One
configuration could still reach the `-static` row for real: a **mixed**
closure — a statically linked main program in a closure that *does* carry a
loader because its sibling programs are dynamic. That closure would be
bundled, and its entry point would defeat the interposer exactly as arm P
predicts. ⛔ No such subject is known here; `syncthing` was checked and
nixpkgs builds it dynamic. **That, and not "a static application", is what
would move this row.**

⚠ **And an open question the run raised rather than settled.** Go issues raw
syscalls for much of its file I/O even when dynamic, so the raw-syscall row
predicts the interposer loses — yet 7 of 8 paths resolved. Three explanations
fit and none is measured: build-time **text** rewriting, the program never
opening them, or those opens routing through libc in this build. `LD_DEBUG`
plus one traced row would separate them.

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
- ⛔⛔ **Not covered, and MEASURED 2026-09-05: the rest of the `exec` family and
  `posix_spawn`.** The list above stops at `execve`/`execv`, and the reason
  given two bullets up — *glibc reaches its own entry points without going
  through the PLT, so they are interposed by name* — applies to these
  identically. They are not interposed, so they are not rewritten:

  | call | a `/nix/store/…` path passed to it |
  |---|---|
  | `open`, `execve`, `execv` | ⭐ rewritten |
  | `system` | ⭐ rewritten, transitively — the child shell inherits the preload and its own `execve` is hooked |
  | ⛔ `execvp`, `execl` (and `execlp`, `execle`, `execvpe`) | **not rewritten** |
  | ⛔ `posix_spawn`, `posix_spawnp` | **not rewritten** |

  ⭐ **Measured with a control**, against a real build of the interposer and a
  real `.storemap`: without the preload all seven read *not rewritten*, so the
  probe discriminates. ⛔ **And it is not theoretical.** Counting `UND` imports
  across the shared libraries on one ordinary host: **21** import `execvp`,
  **15** `posix_spawn`, **14** `posix_spawnp`, **5** `execl`. Among the 25
  libraries are **`libglib-2.0.so.0`** — which is in every GTK subject of the
  corpus — **all four `libpython3.*`**, `libdbus-1`, `libarchive`, `libmagic`
  and `libsystemd`.

  ⚠ **The bound**: it bites only when the path passed is an **absolute store
  path**. A bare program name goes to a `PATH` search, where the bundle's own
  `PATH` already points inside it and there is nothing to rewrite.
  `docs/history/corrections.md` **C60**; the fix is **T-097**.

- ⛔ **Not covered: a path the program derives by string arithmetic** that does
  not begin with a store path (none observed; not searched for exhaustively).

## 5. The build-time half, which is the part that must not guess

**Four** rewrites happen at build time, all against the same closure set.
⚠ This sentence said *three* over a list of four until 2026-09-04; the count
was checked against the list rather than against the tree, which is the same
mistake in miniature that `AGENTS.md` §4 and `limitations.md` §3 each carried:

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
store paths it found compiled in, which of them resolved, and which did not.

⚠ **A REPORT IS A FINDING, NOT A VERDICT, AND THIS PAGE USED TO SAY OTHERWISE**
— *"so a bundle that will fail says so at build time"*. It does not follow, and
the measurement says it does not: the `galculator` bundle reports **three**
compiled-in paths with no target in it and draws on **11 of 11**. A reported
path is one the program may never open.

## 6. ⭐ What the report actually found, and why none of it is resolved by guessing

On the `galculator` bundle, 2026-09-04, after
[`../history/corrections.md`](../history/corrections.md) C27 fixed where a
store reference **ends**:

    88 compiled in: 85 resolve inside the bundle, 3 do not

| reported | what it is |
|---|---|
| `a3hr…-glib-glib-2.88.3` | the **same hash** as the closure's `a3hr…-glib-2.88.3`. It sits in `libglib`'s `.rodata` in a region with **no NUL**: the bytes read `…-glib-glib-2.88.3/lib` and run straight into `g_base64_decode_inplace`. Nothing delimits the end of that string |
| `eeee…eeee-cups-2.4.19` | a hash of thirty-two `e`s is **nix's self-reference placeholder, not a hash**. The closure carries `vjaz8yglcqmbihslm7qj5gkrdz7cd3hi-cups-2.4.19-lib` |
| `eeee…eeee-libunistring-1.4.2` | the same shape. The closure carries `yh8rykx8wakl1ccn8rc351f6r2wbg4cn-libunistring-1.4.2` |

⛔ **All three have a real target in the closure, and all three stay
unresolved on purpose.** Matching `eeee…-cups-2.4.19` to
`vjaz…-cups-2.4.19-lib` means matching **by name with the hash ignored**, and
two derivations can share a name — a `cups` from another nixpkgs revision, or
another output of the same one. ⭐ The whole route is *exact match against a
known finite set*; a near-match that resolves to the wrong derivation is the
failure mode this design exists to make impossible, and it would be silent.

⭐ **In the bundle's TEXT files the corrected scanner leaves nothing at all**:
425 occurrences, 13 distinct store paths, **13 of 13 in the closure**. The
residue is entirely in binaries, and entirely in these two byte-level shapes.
