# runtime — the four mechanisms, and reaching the plugin class

`tool/runtime/*.c`. Routes: [`../docs/AGENTS.md`](../docs/AGENTS.md) §13 item 4.

---

## T-030 — `--wrap-dlopen` against a compiled-in table

**Source** `docs/research/prior-art.md`, `allyourcodebase/pipewire`.
**Category** runtime · **Priority** P1 · **Effort** M · **Status** open

**Problem.** A program loading its own plugins is servable only by hand today
(POC 50). The generic mechanism is not built.

**Premise.** ⭐ Proven prior art, read at file level:
`references/allyourcodebase__pipewire/tree/src/wrap/dlfcn.zig` exports
`__wrap_dlopen`/`__wrap_dlsym`/`__wrap_dlclose` against a compiled-in table.
That is the same delivery mechanism `pgb` already uses for `iconv_open`.

**Approach.** Cheapest of the three routes. Generate the table from the plugins
the build produced; wrap at the final link as `pgb-iconv.c` does.

**Prove.** POC 50's CPython rebuilt with `--wrap-dlopen` instead of hand-written
`Modules/Setup.local`, passing the same matrix.

### Progress — the mechanism is built and measured; the acceptance is not met

⛔ **This entry stays OPEN.** The mechanism works and is proved on a purpose-
built subject, but the **Prove** above names CPython and CPython has not been
rebuilt. Per `RULES.md`, an entry closes on its own acceptance command and not
on a nearby one.

**Landed:**

| | |
|---|---|
| `tool/runtime/pgb-dlopen.h` | the table layout, shared by the reader and the generated writer |
| `tool/runtime/pgb-dlopen.c` | `__wrap_dlopen`/`dlsym`/`dlclose`/`dlerror` against that table |
| `pgb --wrap-dlopen NAME=OBJ[,OBJ...]` | repeatable; generates the table with `nm --defined-only --extern-only` |
| `pgb explain` | prints the four wraps and the generated table when the flag is given, and ⭐ is byte-identical to before when it is not |
| `experiments/71-wrap-dlopen.sh` | the measurement |

**Measured**, `evidence/71-wrap-dlopen/RESULT.txt`, 11 environments:

```
  wrapped: all six assertions pass, every environment      = 11 of 11
  wrapped: loaded no host shared object, every environment = 11 of 11
  plain arm ran (observed, not asserted)                   =  0 of 11
```

⭐ **Three of the six assertions are NEGATIVE**, because a wrapper that
returned a handle for everything would pass a naive "did dlopen work" test:
a file-local `static` must **not** resolve through `dlsym`, a plugin absent
from the table must **not** open, and `dlerror()` must be set in both cases.
The subject also calls **through** the returned pointer and checks the value,
since a non-NULL pointer proves a table lookup and not a working plugin.

⭐ **Cost: 544 bytes.** The wrapped binary is 1,012,960 against the plain
arm's 1,012,416 for this table. The mechanism is close to free; what a real
project pays is the size of the plugins it links in, which it would have paid
anyway.

### ⛔ The acceptance as written cannot be satisfied, and the reason is a result

**The premise keeps its title**, per `../docs/methodology/authoring.md`. What
follows is the correction.

The `Prove` above needs a subject: CPython with modules as
`lib-dynload/*.so`, so that the interpreter actually calls `dlopen` and
`--wrap-dlopen` has something to answer. **That subject cannot be built**, and
it fails nowhere near the plugin-loading code. CPython 3.12.7, `--disable-shared`,
modules left shared, built in the pinned environment:

```
ImportError: build/lib.linux-x86_64-3.12/math.cpython-312-x86_64-linux-gnu.so:
             undefined symbol: PyLong_AsLongLongAndOverflow
make: *** [Makefile:1125: checksharedmods] Error 1
```

Measured on that interpreter: `statically linked`, PT_INTERP 0, DT_NEEDED 0,
and ⛔ **`.dynsym` entries: 0**. `math.so` has 27 undefined `Py*` symbols. The
interpreter **does** define the missing one — `nm ./python` finds it — but
only in its static symbol table, which a dynamic loader never consults.

⭐ **`experiments/72-static-host-plugin-abi.sh` reduces that to a subject that
builds in a second**, so it is a check this project keeps rather than a story
about one afternoon:

| arm | `.dynsym` | outcome |
|---|---|---|
| dynamic host, `-rdynamic` (**positive control**) | 11 | `PASSED`, the plugin called back |
| static host | **0** | `FAIL dlopen: undefined symbol: host_api_add` |
| static host + `--wrap-dlopen` | 0 | `PASSED`, the plugin called back |

⛔ **The finding is structural, not a defect.** A statically linked executable
has an **empty dynamic symbol table**, so a shared object loaded into it can
never resolve a reference back to the host program. `-rdynamic` exports
through `.dynsym`, and there is no `.dynsym` to export through. ⚠ This is
*prior* to `experiments/50-`'s loader failures: even a loader that worked
perfectly would have nowhere to look.

⭐ **Which turns `--wrap-dlopen` from a convenience into the only thing that
can work** for a plugin that calls into its host — and explains why CPython's
own `Modules/Setup.local` mechanism has the shape it does. The two converge by
necessity rather than by coincidence.

**The corrected acceptance**, and ⛔ it is a proposal for the operator rather
than a rewrite an agent may adopt silently, because it changes what the entry
closes on:

> `--wrap-dlopen` builds a project whose plugin loading is **not**
> configurable at build time — i.e. one with no `Setup.local` equivalent —
> with its plugin directory emptied and the functionality intact, on 11 of 11.

⚠ CPython is a poor subject for this entry precisely *because* it already has
a static-modules mechanism: rebuilding it on `--wrap-dlopen` would demonstrate
the two routes agreeing, not the mechanism reaching something that was out of
reach. **T-002 names the right kind of subject** and this entry should take
its subject from there.

## T-031 — Port cross-libc-dlopen's full rewrite, not one function

**Source** `docs/limitations.md` §1 · **Category** runtime · **Priority** P2 · **Effort** L · **Status** open

**Problem.** `experiments/50-` ported `cld_strip_versions()` — one function of
roughly forty from a 2015-line file — and found no effect. The two steps it did
not port are the ones aimed at the failure it observed.

**Premise.** ⚠ The untested steps drop the `DT_NEEDED` edges that pull a
foreign libc in (`cross-libc-dlopen.c:1857`) and rebind the remaining imports.
Upstream's `docs/limits.md` says the static-glibc case is one where `dlopen`
*works* and labels all three static cases unverified.

**Approach.** `CROSS_LIBC_DLOPEN_DRYRUN` makes the rewrite path testable with
no GPU and no Alpine — cheaper than the instrument `50-` built.

**Prove.** `experiments/51-*.sh` re-runs `50-`'s two arms plus a third carrying
the full rewrite, and the table shows what changed on each of 11.

## T-032 — `--embed-terminfo` and a CA-bundle answer

**Source** `docs/limitations.md` §3 · **Category** runtime · **Priority** P2 · **Effort** S · **Status** open

**Problem.** Two of five host data dependencies are open. Both are reached
through an environment variable, which is the shape `--embed-locale` already
proved.

**Prove.** POC 20's `setupterm()` probe passing on all 11, and POC 30's curl
verifying TLS on all 11 with the harness's own CA variables unset.
