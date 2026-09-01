# Comparing the approaches

⛔ **Every cell is either a measurement taken in this repository, or a dash.**
A dash means *not measured here* and never "probably fine". Rows for other
projects describe what their design necessarily implies about the columns —
where that is a reading of their documentation and code rather than a run,
the row says so and the cell stays a dash.

⚠ **Only the last row was executed.** This project measured its own approach
across 11 environments; it did not build AppImages, run sharun, or pack an
onelf bundle. Reading `references/` tells you what those designs *are*; it does
not tell you how they perform, and this table does not pretend otherwise.

## The table

| approach | runs on glibc | runs on musl | NSS | gconv | locale | normal ELF | app changes | runtime overhead | complexity |
|---|---|---|---|---|---|---|---|---|---|
| **plain `gcc -static` (glibc)** | ⚠ **5 of 7** | ✅ 4 of 4 | ❌ loads host modules on 5 of 11; SIGFPE on Arch, openSUSE | ❌ crash on Debian/Ubuntu, 11 of 12 encodings lost elsewhere | ❌ ASCII codeset on all 4 musl | ✅ yes | none | none | trivial |
| **`gcc -static` (musl)** | — | — | n/a: musl has no NSS | — | — | ✅ yes | none | none | trivial, but **not glibc** |
| **bundled glibc + private loader** (onelf, sharun shape) | — | — | — | — | — | ❌ needs a loader and a directory or self-extraction | none | — | high |
| **cross-libc-dlopen shape** | — | — | n/a: it lets host objects *in*, the opposite goal | — | — | ❌ needs a dynamic process + `LD_PRELOAD` | none | — | high |
| **`pgb` (this project)** | ✅ **7 of 7** | ✅ **4 of 4** | ✅ zero host modules on 11 of 11 | ✅ 12 of 12 encodings on 11 of 11 | ✅ UTF-8 on 11 of 11 (opt-in) | ✅ yes, no interpreter | **none** | none measured yet — see below | moderate |

## Reading the first row correctly

⚠ **"5 of 7 on glibc" is not a stability claim, and it is the most misread
number here.** The two crashes are *not* the whole failure. The plain static
binary also:

- loads host NSS modules on Fedora 42, Rocky 8 and Void musl **without
  crashing** — a foreign libc enters the process and it survives;
- crashes on Debian 11, Debian 12 and Ubuntu 20.04 as soon as it calls
  `iconv_open`, which the NSS-only test never reached.

So which distributions "work" depends on *which subsystem the program
touches*, not on the distribution alone. There is no set of distributions on
which a plain static glibc binary is safe; there is a set on which a
*particular program* has not yet hit the path that breaks.

## What is and is not measured

| column | state |
|---|---|
| startup time, peak RSS | **measured** (`experiments/40-overhead.sh`) and the answer is **no measurable difference** between plain `-static` and `pgb`. Two runs put `pgb` 42 µs then 28 µs per exec above plain static, and its RSS 56 KiB above then 28 KiB **below** — a sign change, so both sit at or under this instrument's noise floor. ⛔ Do not quote either as a figure. |
| binary size | **measured**: 1,057,760 B plain static vs 2,138,296 B `pgb` for a program that calls `iconv`; 940 KiB vs 2.1 MiB for the same source with and without an `iconv` call. Static libiconv is the whole difference and only programs that use it pay. |
| steady-state runtime | not measured. `pgb` changes no application code, so there is no mechanism by which it would differ, but that is an argument from structure. |
| build time | measured but **not comparable**: `pgb`'s 247 ms includes entering the chroot build environment, which the other arms do not pay. |
| every non-`pgb` row's behaviour columns | **not run.** Read from design, marked with dashes. |

⭐ **Why "no measurable difference" is the expected result**, which is what
makes the measurement credible rather than surprising: `pgb` adds no process,
no loader, no extraction step and no supervising runtime. The output is an
ordinary `ET_EXEC` with no `PT_INTERP` and zero `DT_NEEDED`, so at run time
there is nothing to be slower than a plain static binary *except* the
constructor that calls `__nss_configure_lookup` fourteen times, and — only
with `--embed-locale`, and only when the host cannot answer a UTF-8
`setlocale` — one directory of files written once.

## Where each row's evidence lives

| row | evidence |
|---|---|
| plain `gcc -static` | `evidence/20-static-glibc-nss-dlopen/RESULT.txt`, `evidence/30-gconv-and-locale/RESULT.txt` |
| `pgb` | the same two files (arm B), plus `evidence/poc/*/RESULT.txt` |
| the others | `references/<name>/`, read at the depth stated in `docs/research/prior-art.md` |
