# runtime — the four mechanisms, and reaching the plugin class

`tool/runtime/*.c`. Routes: [`../docs/AGENTS.md`](../docs/AGENTS.md) §7.

⚠ **Open entries only.** The 6 closed ones are
[`../HISTORY/entries/runtime.md`](../HISTORY/entries/runtime.md); the
long-form findings behind the entry below are
[`../HISTORY/entries/runtime-open.md`](../HISTORY/entries/runtime-open.md).

---

## T-031 — Port cross-libc-dlopen's full rewrite, not one function

**Source** [`../docs/limitations.md`](../docs/limitations.md) §1 · **Category** runtime · **Priority** P2 · **Effort** L · **Status** open

**Problem.** `experiments/50-` ported `cld_strip_versions()` — one function of
roughly forty from a 2015-line file — and found no effect. The two steps it did
not port are the ones aimed at the failure it observed: dropping the
`DT_NEEDED` edges that pull a foreign libc in, and rebinding the remaining
imports.

⛔ **AND THE ROUTE IS THE ONE `AGENTS.md` §7 CALLS BACKWARDS.** Route B lets
host objects *in*; route D is shipped and measured (`--host-dlopen`, 11 of 11).
`experiments/50-` measured no effect from the partial port. ⛔ **Do not port
the shim stack** without a reason this entry does not currently have.

⚠ **The reference moved.** Re-mined at **`793f3f3f`** (PR 30's merge commit),
not the `1cecf50e` a port would have inherited a fixed bug from. ⭐ **We are
not affected by that bug and no document here may say we are** — it is an
`LD_PRELOAD` interposition defect and this tool ships no preload shim. The
defect *class* — a lookup that ANSWERS when it should DEFER — is ours, and the
live instance was found and fixed as T-073.

**What is left.** If it is taken at all: `CROSS_LIBC_DLOPEN_DRYRUN` makes the
rewrite path testable with no GPU and no Alpine.

**Prove.** `experiments/51-*.sh` re-runs `50-`'s two arms plus a third carrying
the full rewrite, and the table shows what changed on each of 11.

📚 [detail](../HISTORY/entries/runtime-open.md)

---

## T-097 — ⛔ the interposer stops at `execve`, and `posix_spawn` is what GLib and CPython use

**Source** deep review 5, 2026-09-05, measured (`docs/history/corrections.md` **C60**).
**Category** runtime · **Priority** P1 · **Effort** S · **Status** open

⛔ **`tool/runtime/pgb-storefix.c` interposes `execve` and `execv` and stops.**
`execvp`, `execl`, `execlp`, `execle`, `execvpe`, `posix_spawn` and
`posix_spawnp` each resolve the path themselves and then call glibc's internal
exec, so an absolute `/nix/store/…` path handed to any of them is **never
rewritten**.

⭐ **The tree had already written down why it must interpose these**, two
bullets above its own list in
[`../docs/design/store-paths.md`](../docs/design/store-paths.md): *glibc
reaches its own entry points without going through the PLT, so `fopen`,
`opendir`, `realpath` and friends are interposed BY NAME.* Same shape, left
out.

## ⭐ MEASURED, WITH A CONTROL — C60 has the table

| rewritten | not rewritten |
|---|---|
| `open`, `execve`, `execv`, and `system` (transitively, via the child shell) | ⛔ `execvp`, `execl`, `posix_spawn` |

⛔ **And the imports were counted, so it is not theoretical**: on one ordinary
host **21** shared libraries import `execvp`, **15** `posix_spawn`, **14**
`posix_spawnp`. Among the twenty-five are **`libglib-2.0.so.0`** — in every GTK
subject of the corpus — **all four `libpython3.*`**, `libdbus-1`, `libarchive`,
`libmagic` and `libsystemd`.

## ⚠ WHAT IS NOT CLAIMED

⛔ **No corpus row is attributed to this.** It was found by reading the source
against its own stated principle and then measured; it is not the diagnosis of
an observed failure. ⚠ And it bites only on an **absolute** store path — a bare
program name goes to a `PATH` search the bundle already controls.

## ⭐ THE ROUTE

Interpose the seven by name and forward, exactly as the existing wrappers do.
⚠ Two need care rather than a copy:

1. `execl`/`execlp`/`execle` are **variadic**. Repack the argument list into an
   array and forward to the `v` form, which is what glibc does internally.
2. `posix_spawn`/`posix_spawnp` take the path plus a file-actions struct;
   ⛔ **the file actions can carry paths too** (`posix_spawn_file_actions_addopen`),
   and those are opaque. Rewrite the executable path, and record whether the
   file-actions case is reachable rather than assuming it is not.

**Prove.** The probe from C60, extended: all seven rewritten with the
interposer, none rewritten without it, and `pgb selftest` still green. ⛔ Then
one bundle experiment re-run, because this changes the object every bundle
carries — the same debt C53 incurred.
