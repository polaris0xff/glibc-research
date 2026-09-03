# The portable-nix and AppBundle sweep — the mechanisms, at file and line

⛔ **This is the usable half.** The verdicts, the ranking and the reasoning are
in [`portable-nix.md`](portable-nix.md); this page exists for the session that
does the work, and it is written to be **used later** rather than admired now.

⚠ **Every line reference is against the commit in the provenance table of the
findings page.** The corpus is tracked in `references/`, so a citation is
checkable without re-fetching anything.

⛔ **Almost nothing here was executed.** §1 was measured
(`experiments/98-published-static-nix.sh`); every other mechanism is a reading,
and is unverified until the session that copies it runs it.

---

## 1. Fetch a published static `nix` — the one command

⭐ **For T-051.** The nix flake publishes a static CLI. The attribute name
changed at nix 2.26.0 and `containerbase/nix-prebuild` carries the boundary in
production code (`tree/bin/builder.sh:21-26`):

```sh
target=.#nix-cli-static
if dpkg --compare-versions "${TOOL_VERSION}" lt "2.26.0"; then
  target=.#nix-static
fi
nix --extra-experimental-features "nix-command flakes" build ${target}
cp result/bin/nix "${tp}/bin/nix"
```

⚠ **That still needs nix to run it**, which is what T-060 rung 1 exists to
remove. For a host that merely needs *a* nix, the release artefacts of
`containerbase/nix-prebuild` are `nix-<version>-<arch>.tar.xz` and its whole
packaging step is one `tar -cJf`.

✅ **CHECKED, `experiments/98-published-static-nix.sh`, pass=7 fail=0.**
PT_INTERP 0, DT_NEEDED 0, 37,908,480 B, and `nix --version` answers on **11 of
11**. ⛔ **Do not use `strings | grep 'GNU C Library'` as the libc test** — it
returns 1 here, from a licence sentence, and says the opposite of the truth.
The discriminator is the compiled-in store path:

```sh
strings -a ./nix | grep -oE 'nix-static-[a-z0-9_]+-unknown-linux-[a-z0-9]+'
# -> nix-static-x86_64-unknown-linux-musl
```

## 2. The runtime fallback chain, and the probe that lies

⭐ **For T-051 step 3.** `nix-portable` picks a virtualisation runtime in this
order and **caches the answer** in `$dir/conf/last_auto_runtime`
(`tree/default.nix:382-417`):

```
nix --store   ->   bwrap   ->   proot
```

The probes, verbatim in shape:

```sh
# rung 1: does nix's own local-store sandbox work?
"$NP_NIX" --store "$dir/tmp/__store" shell -f "$dir/mini-drv.nix" \
    -c "$dir/bin/nix" store add-file --store "$dir/tmp/__store" "$dir/tmp/testfile"

# rung 2: does bubblewrap work?
"$NP_BWRAP" --bind "$dir/emptyroot" / --bind "$dir/" /nix \
    --bind "$dir/busybox/bin/busybox" "$dir/true" "$dir/true"
```

⛔ **RUNG 1's PROBE IS A FALSE POSITIVE AND THIS IS THE MOST USEFUL LINE ON THIS
PAGE.** It passes on Debian 11, Debian 12 and Arch, and `nix run nixpkgs#hello`
then fails with `error: setting up a private mount namespace: Operation not
permitted` (issue #98, three independent reporters). ⭐ **A probe that builds a
trivial derivation does not predict a real workload.** If `pgb` ever probes a
runtime, probe it with the workload, not with a token.

⚠ Escape hatches worth copying, because a wrong auto-selection is otherwise
unrecoverable: `NP_RUNTIME`, `NP_NIX`, `NP_BWRAP`, `NP_PROOT`, and `NP_RUN`,
which replaces the whole command.

## 3. Start-up: extract instead of mount, above a size threshold

⭐ **For T-066 N1/N2.** `pelf`'s runtime
(`tree/appbundle-runtime/appbundle-runtime.go:745-778`) has four modes:

| mode | behaviour |
|---|---|
| 0 | FUSE mount only |
| 1 | never mount: extract and run |
| 2 | try to mount, fall back to extraction |
| 3 | ⭐ mount **below 350 MB**, extract above it |

```go
case 3:
    // As above, but if the image size is less than 350 MB (default)
    const defaultSizeLimit = 350 * 1024 * 1024
    if cfg.elfFileSize < defaultSizeLimit {
        if err := mountImage(cfg, fh, fs); err != nil { /* fall back */ }
    } else {
        if err := extractImage(cfg, fh, fs, ""); err != nil { ... }
    }
```

⛔ **Our kdenlive bundle is 398 MB; the competitor's is 192 MB.** One is over
the line and one is under it. ⚠ It is a default in one project, not a published
benchmark.

⛔ **THIS PAGE SAID "a lever `pgb` does not have" AND THAT WAS WRONG, corrected
2026-09-03d.** `uruntime` — the runtime `pgb` already ships — carries the same
mode selector. The strings in the binary `pgb bundle appimage` produces include
`URUNTIME_EXTRACT`, `URUNTIME_MOUNT`, `URUNTIME_CLEANUP` and
`REUSE_CHECK_DELAY`, with `=0`, `=2` and `=3` among them: pelf's taxonomy, in
our own artefact. ⭐ **It is a lever `pgb` does not SET**, which is a different
sentence and a much cheaper problem.

⛔ **AND THE SECOND HALF OF THAT CORRECTION IS ALSO WRONG — see
[`nix-bundle-patching.md`](nix-bundle-patching.md) §1.** These are not
run-time-only environment variables: they are compile-time constants laid out
as patchable ASCII strings, present in the artefact `pgb` ships, so a one-byte
overwrite changes the mode with nothing beside the artefact.

⚠ **And N2 is answered, which changes what this lever is for.**
`experiments/84-` measured that image size is not the time column: 0.0243–0.0312
ms per MiB, so the whole 196 MiB separating the two kdenlive bundles is about
5 ms of a gap never observed below 129 ms. So extract-over-mount cannot be
motivated by *size* any more. ⚠ Whether it pays for another reason is
**unmeasured**: a hand probe on a 7 MB `jq` bundle put `URUNTIME_EXTRACT=1`
cold at 70–87 ms against mounting's 85–94 ms — inside the noise, and taken
without `experiments/clock.sh`, so it is a reading and not a result.
⭐ **The instrument to settle it exists**: `84-`'s padding machinery makes an
artefact of any size, and `clock.sh` gives it an A/A control.

## 4. Two warm-start techniques

⭐ **For T-066, once N0 has made the clock trustworthy.**

```go
// appbundle-runtime.go:204-208 — cache the parsed config on the file itself
xattrData := fmt.Sprintf("%s\n%d\n%s\n%s\n%s\n%s\n%s\n%d\n",
    cfg.appBundleFS, cfg.archiveOffset, cfg.exeName, cfg.pelfVersion,
    cfg.pelfHost, cfg.hash, T(cfg.disableRandomWorkDir, "1", ""), cfg.mountOrExtract)
xattr.FSet(f.file, "user.RuntimeConfig", []byte(xattrData))
```

Every later start reads the xattr instead of re-parsing the ELF
(`appbundle-runtime.go:150-168` takes that path when the xattr is present).

```go
// appbundle-runtime.go:94-111 — reuse a live mount rather than remounting
if isMounted(cfg.mountDir) { /* reuse it, rewrite the .pid file */ }
```

⚠ Exposed to users as `REUSE_INSTANCES=[0,1]` (pelf issue #3).
⛔ Both are **warm**-path levers; the kdenlive gap is worst on **cold**.

## 5. The `.desktop` and the icon, without patching anything

⭐ **This answers the operator's open question in
[`../design/nix-front-end.md`](../design/nix-front-end.md)** — *"there must be a
way to automatically get these"*. From `pelf` issue #3, an AppImage packager
answering the pelf maintainer:

> *"Any appimage made with `linuxdeploy` or `appimagetool` or
> `go-appimagetool` (aka 99.99% of appimages) will have a `.DirIcon` file in
> the top level of the appimage. Sometimes that file is a symlink and one has
> to be careful when extracting it. … Same applies for the `.desktop`."*

So: **`.DirIcon` and `*.desktop` at the AppDir top level**, and `ivan-hc/AM` is
cited in the same thread as a working handler.
⚠ Guard for the symlink case.

## 6. The alternative to environment variables, named so it can be refused

⛔ **For T-053**, and it is a tier-4 mechanism this project's preference order
puts below what it does today. Same thread:

> *"access to files in `usr/` is done by linuxdeploy as it **patches the
> binary** to have its path relative to the `usr` inside the appimage instead
> of `/usr` from the filesystem. that way it can always find what it needs
> without needing to use stuff like `LD_PRELOAD` or `LD_LIBRARY_PATH`."*

⭐ It removes the wrapper-environment problem entirely, at the cost of editing
the application. ⚠ Record the decision either way; do not walk into it.

## 7. What a bundle's AppRun contract actually is

⚠ Worth knowing before assuming a nixpkgs `bin/x` behaves like an AppDir entry
point. From the same thread, the AppImage side:

> *"at the very minimum the AppRun has to launch the main binary … And
> sometimes AppRun is just a symlink to the binary inside `usr/bin`."*

and, asked which variables an AppRun expects to be set: **"None."**

---

## ⛔ The instruments this sweep owes and does not have

`methodology/references.md` §5: *"every measured claim ships with the thing that
measured it."* ⭐ **One finding was measured and it shipped its instrument** —
`experiments/98-published-static-nix.sh`, pinned by SHA-512. Every other
finding is a reading at a cited line, and the page says so.

⚠ **What still owes an instrument**, each with the probe that would settle it:

1. ~~is the published `nix-static` musl or glibc~~ ✅ **done**, and it shipped
   its instrument: `experiments/98-published-static-nix.sh`;
2. does `nix --store` under `$HOME` work on this project's eleven — the probe
   in §2, run with a **real** workload rather than the trivial derivation;
3. does extract-over-mount beat mount on a large bundle — ⭐ **N0 is done**
   (`experiments/clock.sh` and `99-`), and `84-`'s padding machinery builds
   the large artefact, so this probe is now one experiment away rather than
   blocked. ⚠ Its *motivation* has changed: `84-` rules out size as the
   reason, so the question is whether extraction beats mounting at all, not
   whether it beats it above 350 MB.
