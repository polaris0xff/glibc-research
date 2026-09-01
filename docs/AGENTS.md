# AGENTS.md — read this first, and you can work

Standalone. Assumes no prior context. Every claim here is produced by a script
in this tree that you can re-run.

---

## 1. The project

Answer, with evidence, whether a **normal Linux ELF** built against glibc can
run reliably on both glibc and musl systems with no packaging format and no
significant overhead — and ship the tool that does it.

⛔ **Before you decide what to work on, read
[`REQUIREMENTS.md`](REQUIREMENTS.md).** It carries the operator's binding
acceptance bar — *works everywhere, or strictly better than every existing
format and technique* — which this project **does not meet**, and it tracks
which half of that bar each piece of work discharges. ⚠ Not "does not yet":
half of that bar has now been **measured false**, not merely left undone, and
the page says which half and by how much.

The tool is [`../pgb`](../pgb) (portable glibc build): a POSIX-sh driver plus
three small C runtime pieces. Output is an ordinary statically linked
executable. No launcher, no AppDir, no loader, nothing beside it.

**The answer reached: yes, for programs that do not need to load host
plugins.** Five real projects prove it; §7 states the limits.

⭐ **What that is worth, measured on the axis that matters.** `tmp/START.md`
asks for static binaries "using GLIBC **rather than MUSL** … while avoiding the
usual drawbacks", so musl is the thing being avoided, not a rival to beat on
its own ground. `experiments/61-` measures the ground that is actually
contested — steady-state throughput, same machine, same compiler, libc the only
variable:

| | glibc static | musl static |
|---|---|---|
| malloc, 4 threads | **4.53 ns/op** | 584.71 ns/op |
| qsort | **93.20 ns/op** | 921.49 ns/op |
| strlen/strchr/strstr | **149.14 ns/op** | 1051.09 ns/op |

⭐ **And the row that is this project's actual product:** on **Alpine**, where
the ordinary choice is a musl build, a `pgb` binary does that 4-thread
allocator workload in **4.68 ns/op** against musl's **592**. Same glibc
numbers, on a machine that ships no glibc — and `pgb` costs nothing to get
them: 0.99×–1.05× against plain `gcc -static` on the same workloads.

⚠ **A previous revision of this page said the opposite** — that static musl
"ties pgb and beats it" — from an experiment that measured startup and size,
the two axes musl wins by construction. `history/corrections.md` C7 has what
went wrong and why. ⛔ Do not reintroduce that framing.

The alternative that genuinely competes is **`Anylinux-AppImages`**, not musl
and not vanilla AppImage: it bundles glibc, its loader and its gconv tree, and
`experiments/62-` measures it running on 11 of 11 at glibc's speed too. What
separates `pgb` from it is shape and reach, not portability or performance —
[`comparison.md`](comparison.md) has both sides of that.

## 2. The problem

`gcc -static` against glibc is **not** self-contained, though `file` and `ldd`
say otherwise. glibc's NSS and gconv are plugin systems: the plugin is named in
**host** configuration and loaded with `dlopen` at run time. Static linking
links the dispatcher, never the plugins. Each host plugin carries
`DT_NEEDED libc.so.6`, so a second libc and the dynamic loader enter the
process.

glibc 2.34 made the `files` and `dns` NSS services builtin. That removed the
*default* dlopen only; `resolve`, `myhostname`, `mymachines`, `mdns4_minimal`,
`compat`, `sss` and `systemd` remain external and are named by default on
modern distributions.

Measured, plain `gcc -static`, across the 11 pinned environments:

| | |
|---|---|
| host NSS modules loaded | 5 of 11; **SIGFPE on Arch and openSUSE Leap** (openSUSE's via `passwd: compat`, not DNS) |
| `iconv_open` | **SIGFPE/SIGABRT on Debian 11/12 and Ubuntu 20.04** where the host gconv path matches the build's; 11 of 12 encodings silently unavailable where it does not. **There is no working case.** |
| `setlocale` UTF-8 | `ANSI_X3.4-1968` on all 4 musl environments |

A probe exercising NSS **and** iconv (`ci/probe.c`) fails on **all 11**.

## 3. Success criterion

On every environment in §8:

1. runs — the program's own exit status, no signal;
2. loads **no host shared object**, checked by syscall trace attributed to its
   own pid, never by `ldd`;
3. real functional assertions pass, not `--version`.

⭐ **Criterion 2 is about shared objects, not host data.** glibc still *opens*
`/etc/nsswitch.conf` under the override, and honouring the host's locale where
one exists is correct. The property is **independence** — working whether or
not host data is present — and the musl rows, which have none of it,
demonstrate it. Data reads are reported in their own column, never asserted.

## 4. How it works

Three mechanisms, no application source changed by any of them:

| | mechanism | file |
|---|---|---|
| **NSS** | constructor calls `__nss_configure_lookup()` (public `GLIBC_2.2.5`, present in `libc.a`), pinning all 14 databases to services glibc ≥ 2.34 implements inside libc. Passed as a plain object with `-Wl,-u,pgb_runtime_anchor`, because a constructor with no referenced symbol is dropped from an archive. | [`../tool/runtime/pgb-nssfix.c`](../tool/runtime/pgb-nssfix.c) |
| **iconv** | `-Wl,--wrap=iconv_open,--wrap=iconv,--wrap=iconv_close` onto static GNU libiconv. Acts at the final link, so it catches calls from any object including archives built before this tool existed. Lives in an archive, so a program that never calls `iconv_open` links none of it — 940 KiB vs 2.1 MiB, same source. | [`../tool/runtime/pgb-iconv.c`](../tool/runtime/pgb-iconv.c) |
| **locale** (opt-in `--embed-locale`) | `-Wl,--wrap=setlocale`; C.UTF-8 embedded, written out only when the host cannot answer a UTF-8 request. The only mechanism that touches the filesystem, hence opt-in. | [`../tool/runtime/pgb-locale.c`](../tool/runtime/pgb-locale.c) |

**Delivery:** compiler wrappers on `PATH` plus `CC`/`CXX`. autotools, CMake,
meson and make pick them up unmodified. Each wrapper reads its own argv:
`-c`/`-E`/`-S`/`-M` = compile; `-shared` = **passed through untouched**, which
is what lets `./configure`'s shared-library probes still work; anything else =
executable link. `sh pgb explain` prints every injected flag.

**Build environment:** pinned `debian:12` (glibc 2.36) by manifest digest,
unpacked by `oci-pull.sh` and entered by `chroot`. Verified not to be host
contamination: output `.comment` reads `GCC: (Debian 12.2.0-14+deb12u1)` where
a host build reads `Ubuntu 13.3.0`.

## 5. Repository layout

```
pgb                       the tool; its header comment is the manual
tool/runtime/*.c          the three mechanisms
ci/probe.c                the binary CI runs on 11 distributions
scripts/common/
  oci-pull.sh             OCI image -> rootfs, no daemon      (--selftest)
  rootfs-run.sh           chroot into one, private mount ns   (--selftest)
  fetch-rootfs.sh         materialise the test bed
  rootfs-images.txt       the 11 environments, pinned by digest
  mine-repo.sh            reference-sweep fetcher, vendored    (--selftest)
scripts/build-libiconv.sh GNU libiconv 1.18, pinned
experiments/lib.sh        conditions block, assertions, pid-attributed tracing
experiments/NN-*.sh       numbered; exit 0 matched, 1 did not, 2 could not run
docs/REQUIREMENTS.md      the operator's acceptance bar, and how far short it is
poc/common.sh             the POC contract
poc/NN-*/run.sh           the five proof-of-concept projects
evidence/                 committed RESULT.txt per experiment and POC
references/               12 upstream trees + trackers, tracked, PROVENANCE.md each
.github/workflows/portability.yml
docs/                     see §11
tmp/START.md              the original brief
```

## 6. Running it

```sh
sh pgb doctor                        # what this machine can do
sh scripts/common/fetch-rootfs.sh    # the test bed, ~1.5 GiB, digest-pinned
sh pgb env create                    # pinned build env + static libiconv

for e in experiments/*.sh; do case $e in */lib.sh) ;; *) sh "$e";; esac; done
for p in poc/*/run.sh; do sh "$p"; done

sh pgb build -- make                 # your project, unmodified
sh pgb verify ./yourprogram          # run it on all 11
```

Requires root + `CAP_SYS_ADMIN` (the bed is `unshare --mount` + `chroot`),
`curl`, `python3`, `strace`, a C toolchain. First POC run builds OpenSSL and
CPython; budget ~30 minutes.

⚠ **`experiments/60-` needs more than the others**, and skips the arms it
cannot build rather than failing: `cargo` plus `musl-gcc` and the
`x86_64-unknown-linux-musl` rust target (onelf builds its runtime stub as
static musl), `mksquashfs` (snap), and `flatpak` with
`org.freedesktop.Platform//24.08` already installed. It also fetches
`appimagetool`, pinned by sha256 in the script. Budget ~30 minutes for the run
itself — the AppImage arm times out under `strace -f` on every row by design;
the script explains why.

## 7. Limits — measured, not guessed

Full detail with reproductions: [`limitations.md`](limitations.md).

1. **`dlopen` of a *host* object is host-dependent, and success is the worse
   outcome.** gawk's own extension **loads** on Debian 12 and Arch (dragging in
   the host loader and libc), is refused on the other nine.
   ⛔ **`experiments/50-` settles whether prior art can fix this: it cannot.**
   Porting cross-libc-dlopen's symbol-version rewrite into a static binary
   changes the outcome on **zero of 11** environments, because no environment
   fails on symbol versions — they die inside glibc's loader
   (`_dl_call_libc_early_init` assertion, `elf_machine_rela_relative`
   assertion, SIGFPE). A static binary has no loader, so `dlopen` borrows the
   host's `ld.so` and `libc.so.6`, and *that pairing* is the failure. Fixing it
   requires the process to carry its own loader and libc — a bundled-glibc
   **dynamic** binary, which is a second output mode, not a patch to this one.
   §13 item 3.
   **The class this tool serves is: programs that do not need host plugins.** A
   program loading its *own* plugins is fine — build them in, as POC 50 does.
2. **NSS beyond `files`/`dns` is gone**: no LDAP, SSSD, NIS, mDNS,
   systemd-resolved. Measured cost: on Fedora 42 a plain static binary resolves
   the machine's own hostname via `libnss_myhostname` and the pgb binary does
   not.
3. **Five host *data* dependencies exist and static linking touches none of
   them.** gconv ✅ solved (static libiconv), locale ✅ solved (opt-in),
   **terminfo ⛔ unsolved**, **CA bundle ⛔ unsolved**, a runtime's own library
   tree ⚠ shipped (CPython's 98 MiB stdlib).
4. **`-static` resolves what dynamic linking defers**, so an
   incompletely-static optional dependency fails the link (CPython's `nis` via
   `libtirpc`/GSSAPI).
5. **A private-prefix dependency build can bake the build prefix into runtime
   search paths** (ncurses/terminfo).
6. **The kernel is not abstracted**: the bed shares the host kernel
   (Linux 6.18.44). It can falsify "runs on musl"; it cannot test kernel-version
   behaviour. It is also not a security boundary.
7. **x86_64 only. One machine, one day.**

## 8. Test environments

Pinned by manifest digest in `scripts/common/rootfs-images.txt`. ⛔ Re-pulling
a tag without updating that file silently changes what every result describes;
`archlinux:latest` is a rolling tag.

- **musl (4):** Alpine 3.22, 3.20, 3.10; Void Linux musl
- **glibc (7):** Debian 11, 12; Ubuntu 20.04; Rocky 8; openSUSE Leap 15.6;
  Fedora 42; Arch Linux

⚠ Compatibility is claimed for these and nothing else. Eleven filesystems on
one kernel is not "works on Linux".

## 9. Status

| item | status |
|---|---|
| test bed, 11 environments, 3 selftests | **COMPLETE** |
| `experiments/10-probe-the-host.sh` | **COMPLETE** |
| `experiments/20-static-glibc-nss-dlopen.sh` | **COMPLETE** — 37 assertions |
| `experiments/21-glibc-version-floor.sh` | **COMPLETE** — confirms the ≥2.34 pin |
| `experiments/30-gconv-and-locale.sh` | **COMPLETE** — 24 assertions |
| `experiments/40-overhead.sh` | **COMPLETE** — §10 |
| `experiments/50-host-plugin-feasibility.sh` | **COMPLETE** — settles §13 item 3 |
| `experiments/60-versus-alternatives.sh` | **COMPLETE** — 8 arms head to head, `REQUIREMENTS.md` part 2. ⚠ Its AppImage arm used **vanilla** `appimagetool` and its performance columns were startup and size only; both are corrected by 62- and 61-. `history/corrections.md` C7 |
| `experiments/61-libc-throughput.sh` | **COMPLETE** — the axis 60- got wrong. glibc vs musl at steady state, and whether the gap travels to a musl host |
| `experiments/62-anylinux-appimage.sh` | **COMPLETE** — `pgb` against `Anylinux-AppImages`, the AppImage that actually competes |
| `pgb` chroot engine, host engine | **COMPLETE** |
| `pgb` docker/podman engines | **UNTESTED** — no daemon here; code exists, never run. CI is where it first runs. |
| NSS / iconv / locale mechanisms | **COMPLETE** — 11 of 11 each |
| POCs 10 gawk, 20 nano, 30 curl, 40 jq, 50 CPython | **COMPLETE** — all 11 environments each |
| CI workflow | **WRITTEN, NEVER RUN** — no push has happened from a runner yet |
| aarch64 | **UNTESTED** |
| host `dlopen`, terminfo, CA bundle | **KNOWN LIMITATION** — §7 |

**POCs**, all stock tarballs, stock `./configure`, **no source patches**:

| | project | stresses |
|---|---|---|
| 10 | GNU awk 5.3.1 | locale, iconv, `dlopen` extension API |
| 20 | GNU nano 8.2 + ncurses 6.5 | terminfo data, static dependency chain, multibyte |
| 30 | curl 8.11.0 + OpenSSL 3.0.15 + zlib 1.3.1 | `getaddrinfo`/NSS, real DNS, real TLS, CA bundle |
| 40 | jq 1.7.1 + oniguruma 6.9.9 | Unicode round trip, surrogate pairs, optional-dep detection |
| 50 | CPython 3.12.7 | 49 extension modules linked **in**, `lib-dynload` empty, NSS via `socket`/`pwd` |

## 10. Overhead

`evidence/40-overhead/RESULT.txt`. Same source, three ways; 400 execs × 7
rounds, best round; peak RSS via `os.wait4`. **One machine, one day.**

| arm | size | per exec | peak RSS |
|---|---|---|---|
| native dynamic | 16,304 B | 1275 µs | 5356 KiB |
| plain `gcc -static` | 1,057,760 B | 1177 µs | 5380 KiB |
| **`pgb`** | **2,138,296 B** | **1205 µs** | **5352 KiB** |

⛔ **Only the size column is a real difference.** Two runs of this experiment
on the same machine put `pgb`'s per-exec cost 42 µs then 28 µs above plain
static, and its peak RSS 56 KiB above then **28 KiB below** — a sign change.
⭐ **Startup and memory differences here are at or under this instrument's
noise floor, and must be reported as "no difference measurable", never as a
figure.** Anyone wanting a real number needs a lower-noise instrument and more
rounds.

The size cost is unambiguous and attributable: static GNU libiconv roughly
doubles a small binary, **and only for programs that call `iconv`** — a
program that does not links none of it (940 KiB vs 2.1 MiB, same source).

## 11. Documents

| file | what it is |
|---|---|
| this file | current state; read to orient |
| [`REQUIREMENTS.md`](REQUIREMENTS.md) | ⛔ **the operator's binding acceptance bar, and how far short of it the project is.** Read before choosing work |
| [`limitations.md`](limitations.md) | what it cannot do, each with a reproduction |
| [`comparison.md`](comparison.md) | ⭐ **the head-to-head, now measured**: eight ways to ship the same program across the same 11 environments, and where `pgb` loses |
| [`research/prior-art.md`](research/prior-art.md) | the reference sweep, verdicts, provenance |
| [`design/tiers.md`](design/tiers.md) | ⛔ **design only, nothing built.** The tiered-output plan for covering the host-plugin class, and what "universal" can honestly mean |
| [`history/corrections.md`](history/corrections.md) | ⚠ claims measured wrong, instrument defects, refused approaches. **Read on demand, not to orient.** |

## 12. Provenance

- `references/` — 12 upstream trees at captured commits, each with
  `PROVENANCE.md` naming commit, route, and what could not be fetched
  (discussions are GraphQL-only and were **not** fetched for any repository).
  Re-fetch: `sh scripts/common/mine-repo.sh OWNER/REPO --out references`.
- **One deliberate deletion:**
  `references/pkgforge-dev__cross-libc-dlopen/tree/docs/AGENTS.md`, removed
  because the vendoring methodology forbids carrying a third party's agent
  instruction file into this tree. Recorded in that repo's `PROVENANCE.md`.
- **Vendored:** `scripts/common/mine-repo.sh` from `Azathothas/TEMPLATE`.
  GNU libiconv 1.18 is fetched and built, not committed.
- ⚠ **Licensing consequence of the iconv mechanism.** GNU libiconv is **LGPL**,
  and `pgb` links it **statically** into binaries that call `iconv`. The LGPL's
  relinking obligations therefore attach to those binaries. This repository
  does not redistribute libiconv, and `--no-iconv` produces a binary without
  it (at the cost of §2's gconv failures). Anyone shipping `pgb` output should
  check this against their own requirements; see `LICENSE`.
- **No patches exist.** Two POCs pass *configuration*: CPython's
  `Modules/Setup.local` generated from configure's own `Modules/Setup.stdlib`
  with `*shared*` → `*static*` (CPython's own documented mechanism), plus
  `py_cv_module_nis=n/a` and `--disable-test-modules`; and ncurses'
  `--with-terminfo-dirs`.

## 13. Next steps, in order

0. **Decide what this tool is for, now that the comparison exists.**
   `experiments/60-` measured what `REQUIREMENTS.md` part 2 asked for, and the
   answer removes a claim rather than adding one: **static musl ties `pgb` on
   coverage and beats it on startup and size.** ⛔ This is not a defect to fix
   and it is not work an agent can close. Either a column is found where `pgb`
   wins outright, or the operator replaces "strictly better than every existing
   technique" with the class-restricted claim `comparison.md` now leads with.
   Everything below is worth doing either way; this decides what the project
   says about itself while it happens.

1. **Run CI.** `.github/workflows/portability.yml` and `ci/probe.c` are written
   and have **never executed on a runner**. Locally the probe passes on all 11
   and the plain control fails on all 11, so the workflow should be green on
   the portable arm — but that is a prediction, not a result. Running it is
   also the only way the **UNTESTED** docker/podman engines get exercised, and
   the workflow needs its first push to prove its own YAML.
2. **aarch64.** `oci-pull.sh --arch arm64` and `fetch-rootfs.sh --arch arm64`
   exist and re-resolve by tag, which trades the digest pin away and says so.
   Nothing has been run. Expect IFUNC and CPU-baseline questions that x86_64
   did not raise.
3. **A second output mode, for the host-plugin class — the one thing that
   would broaden this beyond its current class.** Full plan and the honest
   limits of the idea: [`design/tiers.md`](design/tiers.md). `experiments/50-`
   has already done the feasibility work and the answer is precise:

   - ⚠ **"Do not port cross-libc-dlopen into the static output" was stated
     more broadly than the measurement supports, and is downgraded.**
     `experiments/50-` ported **one function out of roughly forty**
     (`cld_strip_versions`, 7 lines) from a 2015-line file, and found no effect
     on 11 of 11. The rewrite in `cross-libc-dlopen.c:1857` is three
     coordinated steps, and 50- tested only the first:
     ```
     cld_strip_versions(&e);                      // ported by 50-
     e.dyn[drop_idx[i]].d_tag = CLD_NEUTRAL_TAG;  // NOT ported: drops the
                                                  // DT_NEEDED edges that pull
                                                  // a foreign libc in
     cld_apply_renames(&e, dry_run);              // NOT ported
     ```
     ⛔ The failures 50- recorded — `_dl_call_libc_early_init: Assertion
     'sym != NULL' failed` and friends — are what happens when the host object
     drags the **host libc** in, which is exactly what step two removes. So the
     untested part is the part most likely to matter. ⭐ And upstream's own
     `docs/limits.md`, which this project's sweep never read, says the static
     glibc case is one where `dlopen` **works** and that "the real blocker is
     more likely the preload path" — a static binary has no `LD_PRELOAD`. It
     also labels all three static cases **UNVERIFIED** upstream. Re-test with
     the full rewrite and `CROSS_LIBC_DLOPEN_DRYRUN` before concluding again.
   - ⭐ **Do build a bundled-glibc dynamic mode**, where cross-libc-dlopen
     applies **as designed and unmodified** — it is an `LD_PRELOAD` for a
     process that already carries a libc and a loader, which is exactly what
     that mode provides. The corpus is already in `references/`; its
     `scripts/build.sh` builds it, and `docs/integrating.md` covers wiring it
     into a bundle.
   - ⛔ **And a second cost, measured since this item was written:
     `experiments/60-` ran onelf, which is already exactly this shape — bundled
     glibc plus its own loader in one file.** The bundling half works
     everywhere: no host object on any of the 11, musl included. But it fails
     the encoding assertions on **8 of 11**, because bundling glibc does not
     bundle gconv — and on the 3 it passes it is reaching the *host's* gconv
     modules to do it. **Tier 2 is not a superset of tier 1.** Whatever builds
     it must carry the `--wrap` onto static libiconv across, or it will trade
     the host-plugin class for the gconv result tier 1 already has.
   - The cost is the property the current mode exists for: one normal ELF, no
     interpreter. So it is a **mode**, chosen per project, not a replacement —
     and `pgb verify` should report which mode a binary is.
   - Sequence: (a) teach `pgb` to emit a bundled-glibc dynamic binary with its
     own loader; (b) verify the two-libc invariant holds via
     `cross-libc-dlopen`'s own `tests/invariants.c`; (c) re-run POC 10's
     observation arm in that mode — the gawk extension loading on 11 of 11
     instead of 2 of 11 is the acceptance test.
   - ⚠ Read `references/pkgforge-dev__cross-libc-dlopen/tree/docs/limits.md`
     first: it is that project's own measured list of what its approach cannot
     do, and it was **not** read during this project's sweep.
4. **`--embed-terminfo`.** The mechanism is already proven by `--embed-locale`
   and ncurses reads `TERMINFO` from the environment, so it is the same shape.
   Decide first whether a *glibc* portability tool should own a terminal
   database — §7.3 argues both ways.
5. **A CA-bundle answer.** Same shape again (`SSL_CERT_FILE`). One compiled-in
   path works on 5 of 11.
6. **Second machine, second kernel.** Every number here is one machine.
7. **Sweep depth.** `allyourcodebase/pipewire` was fetched and never read;
   sharun, userland-execve-rust, ppkg and elftool were read at README/file-list
   depth. See `research/prior-art.md` for what that does and does not support.

## 14. Do not redo these

- **Do not try to make host NSS modules load correctly.** Keeping them out is
  the fix and it is measured.
- **Do not bundle glibc's gconv modules.** They carry `DT_NEEDED libc.so.6`,
  so bundling reintroduces the second libc on every musl host.
- **Do not use `ldd`/`file` output as a test.** §3.
- **Do not build below glibc 2.34** — `experiments/21` measures the override
  merely *moving* the dlopen there.
- **Do not write a new OCI puller or reference fetcher.** Both exist with
  selftests.
- **Do not assert a limitation without measuring it.** `history/corrections.md`
  C1 is what that cost.
- **Do not rebuild the head-to-head from scratch.**
  `experiments/60-versus-alternatives.sh` already builds all eight arms,
  including a real Flatpak bundle and a real `.snap`. Re-run it; do not start
  a new comparison.
- **Do not write "strictly better than the alternatives" anywhere.** It was
  measured and it is false — static musl ties `pgb` on coverage and beats it on
  startup and size. `comparison.md` has the claim that *is* supported.
- **Do not match `.so` as a substring** when deciding what a binary loaded:
  `/etc/ld.so.cache` is an index, not an object, and both `poc_matrix` and
  `pgb verify` assert on that value. Require `.so` or `.so.N` at the end.
- **Do not attribute a bundle format's trace to one pid**, and do not reduce
  traced paths to basenames. Both make a bundling format look clean when it is
  not, or the reverse. `experiments/60-`'s `classify_trace` is the working
  instrument; `history/corrections.md` says why.
- **Do not reap test processes with `pkill -f`.** The pattern appears in the
  runner's own command line, so it kills the experiment. `pkill -x` by name.
