# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02, at the START of the session (RULES.md §RESUME)
    TREE           main, clean, at 184b1c56
    BRANCH         ⛔ main. The harness named `claude/go-port-critical-review-ha7n9g`;
                   the operator ruled AGAIN this session that RULES.md §Git wins.
                   The harness branch exists on the remote at main's commit and
                   is left alone — the git proxy refuses remote deletes.

---

# ⛔ THIS SESSION: T-061, THE GO PORT, AND NOTHING ELSE

**Read [`toolchain.md`](toolchain.md) §T-061, then
[`../docs/design/toolchain.md`](../docs/design/toolchain.md) "Language and
structure".** Port the tooling to Go, reach parity, ship one static `pgb`.

## ⭐ Operator rulings taken at the start of THIS session

Asked interactively before any code was written. All four are binding.

| # | question | ruling |
|---|---|---|
| 1 | branch: `main` or the harness's `claude/*`? | ⛔ **`main`.** RULES.md §Git outranks the harness, again |
| 2 | what happens to the shell/Python once ported? | ⭐ **`git mv` it to `HISTORY/<commit-hash>/<original path>`** — retired, not deleted. Not "delete", not "keep both live" |
| 3 | how far to push gate 5 (11 envs × 9 POCs) before recording the port as landed? | **cheap first, heavy in background.** Gates 1,2,3,4,6 in full; experiments and the fast POCs; 50/80/90/91 in background, reported per-row as they land, ⛔ never extrapolated |
| 4 | is the vendored `mine-repo.sh` / the `check.sh` + `check-docs.sh` gates in scope? | **no, all three stay shell.** One is vendored (`vendoring.md`), two are the oracle |

## Where the work stands

    MACHINE  READY. 11 of 11 rootfs, chroot env, docker env, nix, zstd.
             ⛔ bootstrap.sh's `env` step built the DOCKER environment, not
             the chroot one — it calls `pgb env create` with no engine after
             starting dockerd, so pick_engine returns docker. The chroot env
             was built by hand with `sh pgb --engine chroot env create`.
             `pgb bootstrap` in Go must not repeat it.

    DONE     the Go pgb: driver, wrappers (argv[0] multi-call), env, build,
             verify, rootfs (native unshare+chroot, no shell), OCI pull,
             ELF/ar reader, and the nix NAR/drv/index readers.
             72 carried selftests, all pass.
             ⭐ GATE 4 MET: same source through the Go pgb and the shell pgb
             is BYTE-IDENTICAL (sha256 251cec64…).
             ⭐ GATE 2 MET: byte-identical NARs, identical hashes, identical
             signature decisions, on fixtures AND on a real cache.nixos.org
             object. evidence/92-go-port/RESULT.txt has both.

    NEXT     nix-fetch + nix plan/build (tool/lib/nix.sh, 1259 lines), then
             the bundler (tool/nix-appimage.sh, 1139), then bootstrap, then
             repoint experiments/ and poc/ at the Go entry points and run
             gates 1, 3 and 5.

    ⚠ THE SHELL IS STILL LIVE and still the oracle. Nothing has moved to
      HISTORY/ yet; that happens per file as its gate passes.

## ⛔ Machine notes a fresh session cannot infer

- **Go 1.24.7 is at `/usr/local/go/bin/go`.** `CGO_ENABLED=0 go build` produces
  a static ELF here — verified on a stdlib `debug/elf` probe before any port
  work started.
- **nix is NOT installed** on this machine (the previous session's was). The
  bootstrap installs it; ⚠ its flake route is broken behind the harness proxy
  (`api.github.com` → 403), so `nixpkgs#attr` fails and
  `nix-instantiate '<nixpkgs>' --attr X` is the route that works.
- **`pgb env create` ignores a trailing `--engine`**; the global one works:
  `sh pgb --engine chroot env create`.
- **4 cores, ~15 GiB RAM, 29 GiB free disk** at session start.
- ⛔ **DO NOT EDIT A SHELL SCRIPT WHILE IT IS RUNNING.** `sh` re-reads from a
  byte offset, so an edit mid-run corrupts the running process. Copy the tree
  to `/var/tmp/frozen-<what>` keeping the layout, or wait.
  ⭐ **This whole class is why T-061 exists**, and a Go binary is immune to it.

## Not lost, but not finished either — carried from the last session

    ⚠ experiments/90- IS FIXED AND NOT RE-RUN. The onelf arm's argv[0]
      dispatch defect was ours. The recorded onelf row in
      evidence/90-kdenlive-vs-enhanced/ is the WRONG one and says so nowhere.

    ⚠ 488,934,276 bytes of the kdenlive AppDir's lib/ (2,300 files, 39%) is
      unreachable from the four programs or any plugin directory. T-061
      requirement 6 says to rewrite that sweep in Go, in internal/bundle.

    ⚠ T-060 rung 1's /var/tmp build tree is GONE with the old container.
      The run is idempotent; treat it as never started.
