# The portable-nix and AppBundle sweep — findings

Four references, named by the operator on 2026-09-03c, mined the same day and
read the same day. The write-up opens with what it did **not** establish,
because a reader skimming for the answer will not reach an appendix.

The usable half — the lessons with the actual code lines — is
[`portable-nix-mechanisms.md`](portable-nix-mechanisms.md).

---

## ⛔ What this did NOT establish

| | |
|---|---|
| **never run** | nothing here was executed. Every claim is read at the captured commit, and the ones that matter are marked ⚠ **unverified by measurement** so the next session knows which to probe first. |
| **not fetched** | discussions, for all four. GraphQL only, no credential-free route; each `PROVENANCE.md` records it, and the gap is repeated here because a source missing without being named reads like a source that had nothing in it. |
| **read at README + key-source + tracker depth** | `containerbase/nix-prebuild` — it is 60 lines of shell and a Dockerfile, and pass 4 over it would be padding. Said rather than implied, per `methodology/references.md`. |
| **not compared** | none of these was benchmarked against `pgb`. `pelf` in particular is a live competitor on the axis the operator just made binding, and **nobody has run it here**. |

⚠ **Assume claims remain wrong.** This is revision 1 and it already carries one
correction of a claim this project had written down (below).

---

## Provenance

| reference | commit | depth reached |
|---|---|---|
| [`DavHau/nix-portable`](../../references/DavHau__nix-portable/PROVENANCE.md) | `91122e3d94ba51d7d83fe990fa81d3de0968fb32` | 4 passes + tracker (83 issues, 87 PRs, comments) |
| [`nixie-dev/nixie`](../../references/nixie-dev__nixie/PROVENANCE.md) | `d14c6c370489ec13b24d65df569e7769444ebebf` | 3 passes + tracker |
| [`containerbase/nix-prebuild`](../../references/containerbase__nix-prebuild/PROVENANCE.md) | `9302079d1cb625307f195273cee4632648ecbaec` | 2 passes + tracker; ⚠ it is 60 lines |
| [`xplshn/pelf`](../../references/xplshn__pelf/PROVENANCE.md) | `d3cb5c7be01ae6a672fe480a117bb84cc65fc438` | 4 passes + tracker (8 issues, 127 PRs, comments) |

⭐ **The corpus is TRACKED, in the tree**, per `methodology/references.md` §4's
second shape. Re-fetch with
`sh scripts/common/mine-repo.sh OWNER/REPO --out references`.

---

## ⛔ FINDING 1 — A STATIC `nix` IS PUBLISHED, AND THIS PROJECT WROTE DOWN THAT IT WAS NOT

⛔ **`TODO/toolchain.md` T-051 records, measured on 2026-09-03c:** *"nixpkgs
ships no static nix, so step 2 cannot begin with a fetch."* ⭐ **That statement
is true and it is answering the wrong question.** nixpkgs does not ship one.
**The nix flake does**, and three separate projects consume it:

| | how it names the target | at |
|---|---|---|
| `nix-portable` | `inp.nix.packages.${system}.nix-static`, with `nix.url = "nix/2.20.6"` | `tree/flake.nix:11,49` |
| `nixie` | `(import (nixPatched r.system)).packages.${r.system}.nix-cli-static` | `tree/static-bins/default.nix:59` |
| `nix-prebuild` | `.#nix-cli-static`, falling back to `.#nix-static` below 2.26.0 | `tree/bin/builder.sh:21-26` |

⭐ **`nix-prebuild` settles the naming, in production code:**

    target=.#nix-cli-static
    if dpkg --compare-versions "${TOOL_VERSION}" lt "2.26.0"; then
      target=.#nix-static
    fi

**So the attribute is `nix-static` before nix 2.26.0 and `nix-cli-static` from
2.26.0 on**, which reconciles the three and explains why they disagree: they pin
different nix versions.

⭐ **And it settles T-060 rung 1's naming problem too.** T-060 measured that
`pgb nix cache attr nix-cli` finds **no attribute** and concluded the components
are not index attributes. Correct — ⛔ **they are attributes of the NIX FLAKE,
not of nixpkgs**, and `nix-cli-static` is a real, published, buildable target.

⚠ **WHAT THIS DOES NOT CHANGE**, and the distinction is the whole of T-060:
these are built through `pkgsStatic`, which in nixpkgs is **musl**
(`research/nix.md` finding 1). ⭐ **So T-051 — "enough nix on a minimal host" —
is answerable today by fetching a published binary, and T-060 — "a static-GLIBC
nix produced by `pgb`" — is not.** Those two entries have been treated as the
same work seen from two sides; ⛔ **they are not, and this is the finding that
separates them.**

⚠ **Unverified by measurement here**: that the published `nix-static` is musl,
that it runs on this project's eleven, and that it operates a `--store` under
`$HOME`. All three are one probe each and none was run.

## ⛔ FINDING 2 — THE `nix --store` ROUTE IS THE DEFAULT, AND IT FAILS IN PRODUCTION

⭐ **This is the tracker paying for itself**, and it is a cost this project
would otherwise have paid in full.

`TODO/toolchain.md` T-051 names *"a static `nix` binary … run against a store
under `$HOME` with `--store`"* as its step 2 and records the `--store` half as
**unmeasured**. `nix-portable` has shipped exactly that since 2021, as its
**first-choice runtime**, ahead of bwrap and proot
(`tree/default.nix:396-416`). ⛔ **And its tracker says it does not work.**

Issue **#98**, *"cannot use `nix --store` runtime which is selected by
default"*, nine comments, reproduced by three people:

    $ nix-portable nix run nixpkgs#htop
    error: setting up a private mount namespace: Operation not permitted

⛔ **On systems where user namespaces are demonstrably available** — the
reporter shows `CONFIG_USER_NS_UNPRIVILEGED=y`, `unprivileged_userns_clone=1`
and `unshare -r -n echo YES` succeeding — and where `NP_RUNTIME=bwrap` works.
Reproduced on **Arch Linux (netboot 2024.04.01)**, **Debian 11** and
**Debian 12** by three different reporters.

⭐ **AND THE PROBE IS A FALSE POSITIVE, WHICH IS THE PART WORTH STEALING.**
nix-portable decides by running a real operation:

    "$NP_NIX" --store "$dir/tmp/__store" shell -f "$dir/mini-drv.nix" \
        -c "$dir/bin/nix" store add-file --store "$dir/tmp/__store" "$dir/tmp/testfile"

That **passes** on the machines above, and `nix run nixpkgs#hello` then fails.
⛔ So a probe that builds a trivial derivation does not predict a real workload
— which is this tree's own defect class (`RULES.md`: a check that cannot fail on
the state it was written to catch) found in somebody else's project.

⚠ The maintainer's own framing, issue **#94**: *"we could potentially get rid of
bubblewrap **if** the nix local-store sandbox turns out to be reliable"*. It is
aspirational. Upstream `NixOS/nix#6853` is cited in the thread as the cause and
was open at the captured commit.

⛔ **What this costs T-051:** step 2 as written — fetch a static nix, point it
at a store under `$HOME` — is **not a route to "works on a minimal host"**. It
is a route that works on some hosts and fails on Debian stable, which is the
opposite of the property the entry is for. ⭐ The fallback chain is the answer,
and `nix-portable` has already ordered it: **nix `--store` → bwrap → proot**,
with the decision cached and each rung probed.

## ⚠ FINDING 3 — `nixie` IS BUILT ON THE MECHANISM FINDING 2 SAYS IS UNRELIABLE

⭐ **The disagreement is the finding**, per `methodology/references.md`.

`nixie`'s README states its whole premise: it *"leverages a brand new feature
starting with Nix 2.10, which allows the default Nix binary to host a sandboxed
Nix store with no privileges"* — that is the `nix --store` local-store sandbox,
i.e. **exactly what nix-portable #98 documents failing on Debian 11, Debian 12
and Arch**.

⚠ **This is not a claim that nixie is broken.** It is a claim that its premise
is the one with a known production failure, that nixie's own README calls it
*"alpha software, provided as-is with no guarantee"*, and ⛔ **that the operator
named nixie as "the shape" in T-060 without either of us knowing that.**

⭐ **What nixie is still worth**, and it is not nothing: it is the only one of
the four that **builds nix from source with patches** rather than consuming a
release (`tree/static-bins/default.nix`), it carries per-system patch sets, and
it is the closest thing in the corpus to what T-060 rung 1 has to do.
⚠ Its patches are **Darwin-only** at this commit — `x86_64-linux` and
`aarch64-linux` both take `[]` — so it says nothing about what a Linux
static-glibc build needs.

## ⛔ FINDING 4 — SOMEBODY ELSE'S SIZE-VERSUS-SPEED THRESHOLD IS 350 MB, AND OURS IS 398

⭐ **This lands directly on the hypothesis `PROGRESS.md` N2 says nobody has
measured**: *"on kdenlive, start and render are dominated by mounting a 398 MB
dwarfs image against a 192 MB one — i.e. the size column IS the time column."*

`pelf`'s runtime carries a **four-mode startup policy**
(`tree/appbundle-runtime/appbundle-runtime.go:745-778`), and mode 3 is a size
threshold:

    case 3:
        // As above, but if the image size is less than 350 MB (default)
        const defaultSizeLimit = 350 * 1024 * 1024
        if cfg.elfFileSize < defaultSizeLimit { mount... } else { extract... }

⛔ **Above ~350 MB, somebody who ships this format for a living chooses
EXTRACTION over FUSE mounting.** Our kdenlive bundle is **398 MB** — over the
line. The competitor's is **192 MB** — under it.

⚠ **That is corroboration, not measurement**, and the threshold is a default in
one project rather than a published benchmark. ⭐ But it names a lever `pgb`
does not have at all: **extract instead of mount, decided by size**. N1 should
measure it before spending a session on byte-shaving.

## ⭐ FINDING 5 — TWO STARTUP TECHNIQUES WE DO NOT HAVE, AND THE BAR IS NOW STARTUP

1. **The runtime caches its own parsed configuration in an extended attribute
   on its own file** — `xattr.FSet(f.file, "user.RuntimeConfig", …)`,
   `appbundle-runtime.go:206`. Every later start skips re-parsing the ELF.
2. **A live mount is reused across invocations** — `if isMounted(cfg.mountDir)`
   with a `.pid` file, `appbundle-runtime.go:94-111`, and a `REUSE_INSTANCES`
   environment variable exposed to the user (issue #3).

⚠ Both are warm-start levers. ⛔ Neither has been measured here, and
`corrections.md` C23 says our warm/cold instrument is the least trustworthy
thing in the tree — so measure the instrument first (N0), then these.

## ⭐ FINDING 6 — AN OPERATOR OPEN QUESTION IS ANSWERED, WITH A SOURCE

`docs/design/nix-front-end.md` records the operator's *"we would have to avoid
all the complex patching for desktop files etc, **there must be a way to
automatically get these**"* as an open question.

⭐ **There is, and it is conventional rather than clever.** From `pelf` issue
**#3**, in a maintainer-to-maintainer exchange with an AppImage packager:

> *"Any appimage made with `linuxdeploy` or `appimagetool` or `go-appimagetool`
> (aka 99.99% of appimages) will have a `.DirIcon` file in the top level of the
> appimage. Sometimes that file is a symlink and one has to be careful when
> extracting it. … Same applies for the `.desktop`."*

⚠ **And the same thread names a mechanism this project should decide about
deliberately**: `linuxdeploy` **patches the binary** so its paths are relative
to the bundled `usr/` *"instead of `/usr` from the filesystem — that way it can
always find what it needs without needing to use stuff like `LD_PRELOAD` or
`LD_LIBRARY_PATH`."* ⛔ That is tier 4 of this project's preference order
(automatic application patching) chosen over tier 3, and it is the alternative
to the environment-variable approach T-053 is about.

## ⚠ FINDING 7 — A STATIC BINARY BUILT BY NIX DID NOT WORK ON OTHER SYSTEMS

⭐ Recorded because it is this project's entire thesis, observed by somebody
else and worked around rather than solved. `nix-portable`'s `flake.nix:34-36`:

    # the static proot built with nix somehow didn't work on other systems,
    # therefore using the proot static build from proot gitlab

and `tree/proot/alpine.nix` then fetches **`proot-static-5.4.0-r0.apk` from
Alpine edge, through `web.archive.org`**, and copies `usr/bin/proot.static`
out of it.

⛔ **So the portable static binary in a portable-nix tool is an Alpine (musl)
package pinned to a Wayback Machine URL**, because the nix-built static one did
not travel. ⚠ *"somehow didn't work"* is the whole diagnosis on offer — the
tracker has no issue explaining it — so this is evidence that the problem is
real and nobody there chased it, not evidence about its cause.

⭐ **That is the gap `pgb` exists in.** It is also a warning: the fallback in
this corpus is always *"take a musl build from Alpine"*, and this project's
answer has to be better than that or it is not needed.

---

## ⭐ What the corpus corroborates about OUR enumerated quirks

`REQUIREMENTS.md` enumerates ten ways static glibc is not self-contained.
⭐ **Three of them appear in `nix-portable`'s tracker as real user reports**,
which is independent corroboration that the list is about real failures:

| our row | their issue |
|---|---|
| CA bundle | **#8** *"Problem with the SSL CA cert"* (15 comments), **#122** *"SSL errors when /etc/ssl/certs is missing ca-certificates.crt"*, **#114** *"CACert issue?"* |
| locale | **#119** *"set LOCALE_ARCHIVE for nix-shell"* |
| host plugins / dynamic loading | **#161** *"libidn not found"*, **#154** *"missing libssl.so.10"*, **#86** *"nix-store no libgssapi_krb5.so.2"* |

⛔ **No timezone issue appears in their tracker**, which is not evidence that
T-076 was wrong — nix-portable ships a whole nixpkgs closure, so its programs
get tzdata the way they get everything else. It is a reminder that the class of
tool decides which of the ten bite.

---

## ⚠ Known-weak claims, read before acting on any of this

1. ⛔ **Nothing here was run.** Findings 1, 2 and 4 are the ones a wrong reading
   would cost most, and each names its own probe.
2. ⚠ **The `nix-static` / `nix-cli-static` musl question is INFERRED**, from
   `pkgsStatic` being musl in nixpkgs. It was not checked against a fetched
   binary. If it is wrong, T-060's premise changes completely, so ⭐ **check it
   first**: fetch one and read its `PT_INTERP` and `DT_NEEDED`.
3. ⚠ **`pelf`'s 350 MB threshold is a default, not a measurement.** It is one
   project's judgement and may encode their dwarfs settings rather than a
   property of FUSE.
4. ⚠ **Issue #98 was open at the captured commit** and the last comment
   proposes switching the default to bwrap. A later version may have done so;
   ⛔ a tracker is evidence of intent, never of behaviour.
5. ⚠ **`nixie` is alpha by its own README** and its Linux patch sets are empty.
   Do not read finding 3 as a verdict on the project.
