# glibc versions: why the pin is old, and what a future glibc can break

⭐ **The short answer to "isn't glibc backwards compatible, so why build on an
old distro?": glibc's backward compatibility is real, it is the reason
everyone else builds old — and it is IRRELEVANT to this project's output,
because that output is static.** The pin is old for an entirely different
reason, it is a FLOOR rather than a ceiling, and it is measured.

## 1. The usual reason to build old, and why it does not apply here

The familiar rule — build against the oldest glibc you must support — is about
**symbol versioning in a DYNAMIC binary**. A dynamically linked program built
on glibc 2.38 records `strlcpy@GLIBC_2.38` in its `.dynsym`; run it on a host
with glibc 2.31 and the loader refuses. Build on 2.17 and you get
`memcpy@GLIBC_2.2.5`, which every later glibc still provides, forever. That
asymmetry — old binaries run on new glibc, new binaries do not run on old — is
what `manylinux` and every "build in a CentOS 7 container" pipeline exist for.

⛔ **A `pgb` binary has none of that machinery.** `experiments/76-` records its
subject's headers:

```
subject : 4,407,960 bytes, PT_INTERP=0 DT_NEEDED=0
```

No interpreter, no `DT_NEEDED`, no versioned imports, no `.dynsym` to satisfy.
The host's glibc is never consulted, never mapped, and never compared against.
⭐ **So the entire backward-compatibility calculus is void for the output**, and
the floor it runs against is the kernel, not a libc: `file` reports
`for GNU/Linux 3.2.0`.

⚠ **This is the project's strongest forward-compatibility property and it is
worth stating plainly: a future glibc release cannot break a binary `pgb`
already produced, because that binary does not contain a reference to one.**

## 2. So why pin at all? A FLOOR, and it is measured

`internal/cfg/cfg.go` pins `debian:13` (glibc 2.41) by manifest digest. Two
reasons, and neither is backward compatibility.

⭐ **The pin was `debian:12` (glibc 2.36) until 2026-09-02**, and §3 below is
the argument that moved it. What follows in §2a is the FLOOR, which is why
there is a pin at all; §3 is the CEILING, which is what decides where above the
floor it sits.

### 2a. ⛔ glibc 2.34 is a hard floor for the NSS mechanism

glibc 2.34 built the `files` and `dns` NSS services **into libc**. Below that
they are still `dlopen`'d modules, so `__nss_configure_lookup()` does not
remove the `dlopen` — it only **moves** it, swapping the host's `resolve`
module for the host's `files` module and loading a foreign libc either way.

⭐ **That was reasoning until `experiments/21-` measured it.** Same source,
built against glibc 2.31 and against 2.36, run on the *same* target root:

| build glibc / arm | exit | host NSS modules opened |
|---|---|---|
| 2.31 plain | 0 | `libnss_dns.so.2`, `libnss_files.so.2` |
| **2.31 + nssfix** | 0 | ⛔ **`libnss_dns.so.2`, `libnss_files.so.2`** |
| 2.36 plain | 0 | none |
| 2.36 + nssfix | 0 | none |

⛔ **The override is cosmetic below 2.34.** That is the floor, and the pin
clears it. ⭐ **Re-measured at the new pin**, same target, same method, with the
2.31 rows kept because they are the control that makes a `none` mean anything:

| build glibc / arm | host NSS modules opened |
|---|---|
| **2.41 plain** | **none** |
| **2.41 + nssfix** | **none** |

⚠ **A "none" from an instrument that cannot see modules looks exactly the
same**, which is not hypothetical: a first attempt at this measurement had an
unquoted shell variable, read a trace file that did not exist, and printed
`none` for every arm. The 2.31 rows are what distinguish the two readings.

### 2b. Reproducibility

Pinned by manifest digest, not by tag, so a result describes a known compiler
and a known libc. `archlinux:latest` in the *target* bed is a rolling tag and
`docs/AGENTS.md` §8 says what re-pulling it silently costs.

## 3. ⛔ The ceiling, and it moves against us every year

`--host-dlopen` changes the calculus, because now a **host** object's imports
have to be satisfiable by **our** static glibc. `experiments/73-` measured the
gap as **class B — the host glibc is NEWER than the pin**:

| symbol | at | objects |
|---|---|---|
| `__isoc23_strtol` | `GLIBC_2.38` | 547 |
| `__isoc23_strtoul` | `GLIBC_2.38` | 466 |
| `__isoc23_sscanf` | `GLIBC_2.38` | 266 |
| `strlcpy` | `GLIBC_2.38` | 86 |
| … 20 distinct symbols, 14 of them the `__isoc23_*` family | | |

⛔ **This is a ceiling pointing the opposite way from the floor, and no single
pin satisfies both ends** — `docs/research/solo.md` says so, and it is worth
restating with its consequence:

⚠ **Class B is not a fixed cost. It WIDENS with every glibc release the pin
does not follow.** At the 2.36 pin it was 20 symbols against hosts running
2.38–2.43. ⭐ **Nothing else in this project degrades merely by the passage of
time; this does.**

### ⭐ The pin was lower than it needed to be, and T-070 moved it

The floor is **2.34**. The pin was **2.36**. The output is static, so there is
**no upward pressure at all** — nothing forces the pin to stay near the floor.
Meanwhile the ceiling argues for the newest glibc available.

⛔ **So the pin sat at 2.36 for no measured reason.** T-070 costed the move
before making it — the cost had to be checked, not assumed — and **all four
costs came back zero**, `experiments/91-glibc-pin-candidates.sh`:

| what the move could have cost | measured | verdict |
|---|---|---|
| the **kernel floor** a static binary declares. If a newer glibc raises `for GNU/Linux 3.2.0`, that trades a real property for a symbol-coverage one | `.note.ABI-tag` **3.2.0** at both pins, `readelf -n` and `file(1)` agreeing | no cost |
| **class C** — a symbol the newer glibc REMOVED | **empty on all 11 rows at BOTH pins** | no cost |
| that the **NSS override still works** — `experiments/21-` re-run at the new pin | **`none`**, with the 2.31 arm firing as the control | holds |
| that **all ten POCs still build**, under gcc 12.2.0 → 14.2.0 | ⭐ **10 of 10 build and pass their full matrices** | no cost |

⭐ **And what it buys, `experiments/73-` at both pins, same day, same eleven:**

| environment | class B @ 2.36 | class B @ 2.41 | symbols served |
|---|---|---|---|
| opensuse-leap-15.6 | 13 | **0** | 993 → **1005** |
| fedora-42 | 15 | **0** | 961 → **976** |
| archlinux-latest | 20 | **5** | 1198 → **1213** |
| debian-11 / ubuntu-20.04 / rockylinux-8 | 0 | 0 | unchanged |
| debian-12 | 0 | 0 | 851 → **849** ⚠ the one cost |

    class B, distinct symbols   20 at 2.36  ->  5 at 2.41

⭐ **The whole `__isoc23_*` family at `GLIBC_2.38` is gone.** The five that
remain are at `GLIBC_2.42`/`2.43` on `archlinux-latest` alone —
`__memset_explicit_chk`, `free_sized`, `free_aligned_sized`, `__inet_pton_chk`,
`__inet_ntop_chk`. ⚠ **Which restates the section's own point: a rolling
distribution is always ahead of any pin.** Moving to 2.41 does not end class B;
it empties it for every fixed-release environment measured and leaves the
rolling one, which is the residue that regrows.

## 4. What a future glibc CAN break, by dependency class

⭐ **Inventory of every glibc symbol `tool/runtime/` depends on**, with its
stability class read out of `libc.so.6` rather than assumed:

| symbol | class | what a future glibc does to us |
|---|---|---|
| `__nss_configure_lookup` | ⭐ **public**, `@@GLIBC_2.2.5` | nothing. Versioned, and glibc does not remove versioned symbols |
| `__errno_location` | ⭐ **public**, `@@GLIBC_2.2.5` | nothing |
| `__tls_get_addr` | ⭐ **public**, `@GLIBC_2.3` — and ⭐ **we DEFINE it**, we do not import it | nothing. It is `ld.so`'s side of a frozen ABI contract |
| `_dl_tls_static_size` | ⛔ **INTERNAL** — not in `libc.so.6`'s dynamic symbol table at all, present only in `libc.a` | see below |
| `_dl_tls_static_used` | ⛔ **INTERNAL** | see below |
| `_dl_tls_static_align` | ⛔ **INTERNAL** | see below |

### ⛔ The three internals, and both ways they can move

These are how `pgb-elfload.c` places **initial-exec TLS** in the surplus glibc
already reserves. They are not versioned, not exported, and not promised.

| change | outcome | why |
|---|---|---|
| **renamed or removed** | ⭐ **safe** | the references are `__attribute__((weak))`, so the addresses come back NULL and the loader refuses initial-exec TLS **by name**. A build simply loses the objects that need it |
| **same name, different meaning** — different units, different base, a different definition of "used" | ⛔ **would have been a silent wrong answer**: a plausible offset, and a module handed thread storage that overlaps somebody else's | ⭐ **now guarded** |

⭐ **The guard validates an INTERNAL variable with a PUBLIC fact.** `errno` is
thread-local and its address is ordinary API, so it must lie inside the block
`_dl_tls_static_size` describes. Measured on the build host:

```
thread pointer       = 0xc9a8380
&errno               = 0xc9a8340   (tp -64)
_dl_tls_static_size  = 3264
_dl_tls_static_used  = 88
  -> errno is inside [tp-3264, tp): CONSISTENT
  headroom for a dlopen'd module = 3176 bytes
```

`el_tls_bookkeeping_sane()` checks exactly that, once, and refuses initial-exec
TLS with a message naming all three values if it ever stops holding. ⛔ **So the
worst case degrades to the same outcome as a rename — lost capability, never
corrupted memory.**

## 5. What a future glibc CANNOT break

| | |
|---|---|
| **a binary already produced** | ⭐ it contains no reference to a host glibc. §1 |
| **the provider table** | it is **generated per build** from whatever `libc.a` is in the environment, so it tracks the pin automatically rather than encoding a snapshot |
| **the `--wrap` mechanisms** | link-time rewriting of undefined references; a linker feature, not a libc one |
| **`__tls_get_addr`** | we define it |

## 6. The standing rules this produces

1. ⛔ **Do not build below glibc 2.34.** `experiments/21-` measures the override
   merely moving the `dlopen` there. Already in `docs/AGENTS.md` §14.
2. ⛔ **Do not treat "build old for compatibility" as applying here.** It is the
   right instinct for a dynamic binary and it buys this project nothing, while
   the class B ceiling makes it actively cost something.
3. ⚠ **Re-run `experiments/73-` whenever the pin moves**, in both directions:
   class B is what a newer pin buys, class C — empty today — is what it could
   cost.
4. ⛔ **Any new dependency on a glibc internal needs a public-fact guard**
   before it is merged, on the model of §4. An internal that fails loudly is
   acceptable; one that can fail quietly is not.
5. ⛔ **The pin is three constants in `internal/cfg/cfg.go` and nowhere else.**
   `TODO/check.sh` fails if a copy appears in `experiments/`, `poc/`,
   `scripts/`, `internal/`, `cmd/` or `.github/`. ⚠ The move of 2026-09-02
   found **nine** copies of the name and **two** of the digest — one of the
   latter an `env.BUILD_IMAGE` in CI that nothing had ever read, because the
   `env` context is unavailable in a job's `container.image`.
6. ⭐ **The move is cheap and the ceiling regrows, so re-cost it periodically**
   rather than waiting for a failure. `experiments/91-` runs the whole veto —
   kernel floor, class B, class C, NSS floor — before anything expensive.
