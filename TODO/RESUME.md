# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02, at the operator's checkpoint
    TREE           main, clean, at 894bfaec
    BRANCH         ⛔ main. The harness named `claude/go-port-critical-review-ha7n9g`;
                   the operator ruled AGAIN this session that RULES.md §Git wins.
                   The harness branch exists on the remote at main's commit and
                   is left alone — the git proxy refuses remote deletes.

---

# ⭐ THE TOOLCHAIN IS GO. T-061 IS SUBSTANTIALLY LANDED.

One statically linked binary: driver, compiler wrappers, planner, verifier,
bundler. `CGO_ENABLED=0`. The shell and Python are retired under
`HISTORY/<commit>/` and are the oracle every gate was measured against.

    124 carried selftests          all pass
    sh TODO/check.sh               green
    sh scripts/common/check-docs.sh  green
    CI                             ⭐ GREEN on a50542da: 16 of 16 jobs, all
                                   eleven matrix targets, and pgb verify
                                   --engine docker. It had been RED for every
                                   commit of the port -- pgb is a built binary
                                   and is not committed, so three jobs ran
                                   ./pgb against nothing. A `toolchain` job
                                   builds it and hands it over as an artefact,
                                   which is the distribution claim under test.
    gates 1, 2, 3, 4, 6            met, with output
    gate 5                         9 of 10 POCs, 21 of 23 experiments; 3 rows left

Evidence, per row, as each landed: `evidence/92-go-port/RESULT.txt`.
The decision, the architecture and the gates:
[`../docs/design/toolchain.md`](../docs/design/toolchain.md) "Language and
structure". What was required: [`toolchain.md`](toolchain.md) §T-061.

## ⛔ WHAT IS LEFT, IN ORDER

1. **Gate 5's three unfinished rows.** `poc/91-qt-xcb` (the cold re-run died
   at Qt object 1,538 of 1,644 with `No space left on device` — the machine,
   not the tool), `experiments/86-` and `experiments/90-`. All three want
   several GiB of free disk before they start.
2. **`experiments/90-`'s onelf row is still the wrong one.** Its argv[0]
   defect was fixed and the three-arm run has not been repeated since.
3. **The operator's post-port instruction**, given mid-session and to be
   followed once the port is finished: codegraph installed and wired into the
   gates and the rules, a deprecation sweep of the Go tree, two deep reviews
   of code and docs, `tmp/` and other unreferenced documents retired to
   `HISTORY/`, and a `docs/AGENTS.md` a session with no memory can start from
   alone.
4. **Prove pgb can build something as complex as onelf's miniflux example**
   (`https://github.com/QaidVoid/onelf/blob/main/docs/guide/examples/miniflux.md`)
   — miniflux plus an embedded PostgreSQL, its dlopen'd extensions and its
   share tree.

⚠ The operator holds a standalone prompt with all of this in full, including
the parts that are not in this repository.

## ⛔ Machine notes a fresh session cannot infer

- **Go 1.24.7 is at `/usr/local/go/bin/go`.** `make` builds `./pgb`;
  `make check` runs the selftests and both record gates.
- ⛔ **DISK IS THE BINDING CONSTRAINT.** This session's allowance ran out
  during a Qt build and `df` reads 100% with only ~25 GiB used, because the
  allowance is per-session rather than per-filesystem. Delete under
  `/var/tmp/pgb-poc`, `/var/tmp/pgb-appimage*` and `/var/tmp/pgb-nix-cache`
  before starting anything kdenlive- or Qt-sized.
- **nix is NOT installed** here. The bootstrap installs it; ⚠ its flake route
  is broken behind the harness proxy (`api.github.com` → 403), so
  `nix-instantiate '<nixpkgs>' --attr X` is the route that works.
- **No `zstd` binary in the pinned build environment**, and pgb no longer
  wants one: `internal/zstd` decodes RFC 8878 in Go. ⚠ `xz` is still shelled
  out; the build environment has it and a target rootfs does not.
- `musl-tools` was installed this session for `experiments/61-`. onelf's arm
  in `experiments/60-` still needs the rust `x86_64-unknown-linux-musl`
  target, which is absent.
- ⛔ **DO NOT EDIT A SHELL SCRIPT WHILE IT IS RUNNING.** `sh` re-reads from a
  byte offset. The experiments and POCs are still shell; the tool is not.

## Not lost, but not finished either — carried from earlier sessions

    ⚠ 488,934,276 bytes of the kdenlive AppDir's lib/ (2,300 files, 39%) is
      unreachable from the four programs or any plugin directory. ⭐ The sweep
      is Go now, in internal/bundle, with a 12-case selftest — but nothing
      consumes it: --debloat still has its own rules and `pgb bundle sweep`
      only reports. Wiring it in is T-055's cut.

    ⚠ T-060 rung 1's /var/tmp build tree is GONE with an old container.
      The run is idempotent; treat it as never started.

    ⚠ T-057 item 2, a 32-bit application through the lib32 path, is untried.
