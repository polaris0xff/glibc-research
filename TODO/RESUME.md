# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02, refreshed mid-session as the port landed
    TREE           main, at 25a86348
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

## ⭐ A FIFTH INSTRUCTION ARRIVED MID-SESSION, FOR WHEN THE PORT IS DONE

⛔ **Read it only once the port is finished**, which is what it says itself.
Verbatim, in order:

1. Install codegraph (`https://github.com/colbymchenry/codegraph`), run
   `codegraph init` then `codegraph sync`, wire it into the gates/checks
   script and into the rules, so agents prefer codegraph first and grep
   second when reading existing code before changing it.
2. Using codegraph, sweep the whole Go tree for deprecated APIs and
   practices. The code must stay endlessly extendable and easily
   maintainable.
3. Two deep reviews of the code and all the docs, finishing the
   "carried from the last session" items below.
4. Leave the repo clean: docs concise, correct, free of historical lore, no
   contradicting or misleading docs or code comments.
5. Make sure the next session has a clean slate with CI green and can start
   from `docs/AGENTS.md` alone — what to do, in what order, how many tasks,
   how long, how to end, what discipline to keep.
6. Print the summary and end. No kickoff prompt: `docs/AGENTS.md` and what it
   references must be enough.

Also: **retire anything under `tmp/` and any other unused, unreferenced,
already-studied document into `HISTORY/`.**

## Where the work stands

    MACHINE  READY. 11 of 11 rootfs, chroot env, docker env, nix.
             ⛔ the retired bootstrap.sh's `env` step built the DOCKER
             environment, not the chroot one — it called `pgb env create`
             with no engine after starting dockerd. `pgb bootstrap` in Go
             names the engine and does not repeat it.

    DONE     the whole toolchain in Go, one static binary. The shell and
             Python are retired under HISTORY/<commit>/ and are the oracle.
             124 carried selftests, all pass. `sh TODO/check.sh` and
             `sh scripts/common/check-docs.sh` both green.
             ⭐ GATES 1, 2, 3, 4, 6 MET, per-row in
             evidence/92-go-port/RESULT.txt. Gate 5 is 9 of 10 POCs and
             every experiment; see NEXT.

    NEXT     1. poc/91-qt-xcb re-run from a COLD prefix — it is the proof of
                the fixed-point dependency walk (25a86348) and was running
                when this was written. /var/tmp/pgb-poc/91-qt-xcb/.
             2. experiments 85, 86, 89, 90 have never run against the Go
                bundler. 90's recorded onelf row is still the wrong one.
             3. T-061 requirement 2's second half: pgb building ITSELF with
                pgb has not been demonstrated.
             4. Then the fifth instruction above.

## ⛔ Machine notes a fresh session cannot infer

- **Go 1.24.7 is at `/usr/local/go/bin/go`.** `CGO_ENABLED=0 go build` produces
  a static ELF here — verified on a stdlib `debug/elf` probe before any port
  work started.
- **nix is NOT installed** on this machine (an earlier session's was). The
  bootstrap installs it; ⚠ its flake route is broken behind the harness proxy
  (`api.github.com` → 403), so `nixpkgs#attr` fails and
  `nix-instantiate '<nixpkgs>' --attr X` is the route that works.
- **There is no `zstd` binary in the pinned build environment**, and pgb no
  longer wants one: `internal/zstd` decodes RFC 8878 in Go. ⚠ `xz` is still
  shelled out and the build environment does have it; a target rootfs does
  not, so an xz-compressed NAR fetched from inside one would still stop.
- **4 cores, ~15 GiB RAM** at session start. Watch disk: the POC build trees
  under `/var/tmp/pgb-poc/` are large.
- ⛔ **DO NOT EDIT A SHELL SCRIPT WHILE IT IS RUNNING.** `sh` re-reads from a
  byte offset, so an edit mid-run corrupts the running process. The
  experiments and POCs are still shell; the tool is not.

## Not lost, but not finished either — carried from the last session

    ⚠ experiments/90- IS FIXED AND NOT RE-RUN. The onelf arm's argv[0]
      dispatch defect was ours. The recorded onelf row in
      evidence/90-kdenlive-vs-enhanced/ is the WRONG one and says so nowhere.

    ⚠ 488,934,276 bytes of the kdenlive AppDir's lib/ (2,300 files, 39%) is
      unreachable from the four programs or any plugin directory. ⭐ The sweep
      itself is now Go, in internal/bundle, with a 12-case selftest — but the
      kdenlive number above has not been re-measured with it.

    ⚠ T-060 rung 1's /var/tmp build tree is GONE with the old container.
      The run is idempotent; treat it as never started.
