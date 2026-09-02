# SUMMARY.md — the session of 2026-09-02

⛔ **Saved as well as printed**, per
[`../docs/methodology/sessions.md`](../docs/methodology/sessions.md), so it
survives the chat scrolling away. Overwritten each session.

⭐ **Every cell is grounded in something that can be pointed at**, and where a
thing was not measured this says so rather than giving a number.

| row | before | after |
|---|---|---|
| **Elapsed** | 2026-09-02T04:0xZ | 2026-09-02T05:5xZ — **≈2h**, ended by an operator checkpoint rather than by the work finishing |
| **Commits** | `184b1c56` | `fe62869d` — **24 commits**, every one on `main` |
| **Work** | T-061 open, nothing written | ⭐ **the whole toolchain ported to Go**, 8 defects found in code that had been trusted, gates 1/2/3/4/6 met, gate 5 at 9 of 10 POCs and 21 of 23 experiments |
| **Changes** | — | **169 files**, 30,681 insertions(+), 1,429 deletions(-) |
| **Size** | — | **17,690 lines** of Go replacing **8,343** of shell and Python, which are retired under `HISTORY/` rather than deleted |
| **Checks** | both gates green, ⛔ **CI red and unnoticed** | both gates green, ⭐ **124 carried selftests** (up from 72), and ⭐ **CI repaired** — it had been failing since the first port commit |
| **Cost** | — | ⚠ **not metered.** What can be pointed at: the real 399,356,002-byte `packages.json` fetched once, a 648,570-byte `cache.nixos.org` NAR, mesa and Qt closures for the bundler experiments. ⛔ **The session's disk allowance ran out**, which is what stopped `poc/91-qt-xcb`. No paid service used |
| **Health** | 34 entries, 16 open | **34 entries, 15 open.** ⭐ **T-056 closed** — superseded by T-061, which took the Python helpers to Go rather than Rust. T-061 stays open for its remainder. Tree clean, `main` pushed |

## What actually happened

⭐ **`pgb` is one statically linked Go binary.** The driver, the compiler
wrappers it puts on `PATH`, the nixpkgs planner, the verifier and the bundler
are the same executable, built `CGO_ENABLED=0`, carrying the C runtime sources
it compiles. There is nothing to clone beside it: a copy alone in an empty
directory builds a static binary and passes its own selftests.

The shell and Python were moved with `git mv` into `HISTORY/<commit>/<original
path>`, per the operator's ruling, and every gate was measured against them
rather than against a claim.

## The eight defects, all found by something disagreeing

⛔ **None of these was found by reading the code.**

1. **`nix-plan.py` was not deterministic.** An output store path can be
   claimed by more than one derivation — 14 of them in git's graph — and the
   Python was last-writer-wins over document order. Ten shuffles of the same
   graph: 4 gave one plan, 6 gave another. The Go planner sorts the claimants
   and gives one plan 10 times out of 10.
2. **The shell bootstrap built the wrong environment.** It called `pgb env
   create` with no engine after starting dockerd, so the "chroot environment"
   it produced was a second docker one.
3. **`pgb nix deps` did not converge from a cold prefix.** `poc/91-qt-xcb` was
   five X libraries short because libxcb was attempted before xcb-proto
   existed; a second run would have fixed it, which is not convergence.
4. **Requirement 3 was written and never wired.** `internal/logx/stamp.go` had
   the columns, the parser and the heartbeat, and nothing called
   `NewStamper`: `pgb --ts build` printed no timestamps at all.
5. **`pgb selftest <typo>` printed "0 cases, all pass".**
6. ⛔ **CI had been red for every commit of the port.** `pgb` is a built
   binary and is not committed, so three jobs did `actions/checkout` and then
   ran `./pgb` against nothing, and a fourth ran `sh -n` over it. A
   `toolchain` job now builds one static pgb and hands it to the others as an
   artefact — the distribution claim under test, since that binary is then
   used inside the pinned Debian 12 container with nothing installed for it.
7. **A skip counted as a failure**, found by the repaired CI within minutes:
   the runner is not root, `rootfs-run` needs root, and `Report.Write`
   returned 1 for it. It returns 0 / 1 / 2 now, like everything else here.
8. **The libiconv build needed `msgfmt`**, which the pinned image does not
   carry. It had been worked around on this machine by installing gettext,
   which is the wrong direction for a project whose argument is not needing
   things on the host. `--disable-nls` removes the dependency instead.

## zstd, because the environment has none

cache.nixos.org serves NARs as `.nar.zst` and the pinned build environment
carries no `zstd` binary, so `pgb nix build` inside it stopped dead. The
retired Python reached `libzstd.so.1` through ctypes and `CGO_ENABLED=0` has
no equivalent. `internal/zstd` decodes RFC 8878 with nothing outside the
standard library, measured byte-identical against the reference encoder over
120 frames at levels 1 to 22, whole and one byte at a time; 572 truncations
refused, and of 400 flipped bytes none decoded to different content without an
error. ⚠ `xz` still shells out.

## What was NOT done

⛔ **Gate 5 is not complete and nothing is extrapolated.** `poc/91-qt-xcb`'s
cold re-run confirmed the dependency fix — all 22 X packages built, the static
xcb link and the qtbase configure both passed — and then died at Qt object
1,538 of 1,644 with `cannot write PCH file: No space left on device`.
`experiments/86-` and `experiments/90-` were never started, for the same
reason. `experiments/90-`'s recorded onelf row is still the wrong one.

⛔ **The operator's post-port instruction has not been started**: codegraph,
the deprecation sweep, two deep reviews, retiring `tmp/`, and a
`docs/AGENTS.md` a session with no memory can start from alone.
