# SUMMARY.md — the session of 2026-09-01

⛔ **Saved as well as printed**, per
[`../docs/methodology/sessions.md`](../docs/methodology/sessions.md), so it
survives the chat scrolling away. It is the fastest orientation into what the
last session actually did. Overwritten each session.

⭐ **Every cell is grounded in something that can be pointed at**, and where a
thing was not measured this says so rather than giving a number.

| row | before | after |
|---|---|---|
| **Elapsed** | 2026-09-01T11:49Z | 2026-09-01T13:52Z — **≈2h03m** |
| **Commits** | `b77e033` | `5412b27` — **12 commits**, squashed to 1 at the end |
| **Work** | 15 entries, 14 open, 1 done | **19 entries, 12 open, 7 done** — 6 completed, 4 opened, **0 deferred, 0 failed** |
| **Changes** | — | **236 files**, 63847 insertions(+), 704 deletions(-) |
| **Size** | 12,861 lines | **16,064 lines** (+3,203), excluding `references/` and `evidence/` |
| **Checks** | gate green; ⛔ **CI red, 10 runs, 10 failures** | gate green; ⭐ **CI green, 15 jobs**, and it now asserts §3 criterion 2 |
| **Cost** | — | ⚠ **not metered.** What can be pointed at: 11 target images pulled by digest + 1 build image, ~3.465GB of docker storage; 2 repositories mined into `references/` (~2.7 MiB kept after deleting 61 MiB of nested corpus); CPython 3.12.7 fetched and built once, then deleted. No paid service was used. |
| **Health** | 3 tracked files wrong about observable facts | ⭐ **9 defects found and fixed**, every one of which read as success. **4 new debts, all carried as open entries** (T-015, T-017, and the two operator decisions). Tree **clean**, `main` pushed, no `ephemeral-*` branches. |

## The nine defects, because "9" is not a finding

Every one of these produced a **passing** result while being wrong. That is
the pattern, and it is why the entries above are worth more than the features.

| # | defect | what it looked like |
|---|---|---|
| 1 | `pgb build --engine docker` flattened argv with `$*` | built nothing, **exited 0** |
| 2 | docker/podman engines carried no TLS anchor | "libiconv is broken" (curl exit 60) |
| 3 | `--bind` passed relative paths to `-v` | an empty **named volume**, exit 0 |
| 4 | `die()` used `$*` where it meant `$1` | printed its exit code into its message |
| 5 | a backtick in an unquoted heredoc | `nm` **executed** during `pgb explain` |
| 6 | tracer reported opens at syscall **entry** | counted paths merely probed for — a false positive on criterion 2 |
| 7 | tracer paired entry/exit with a bare toggle | `/bin/true` reported as opening **nothing** |
| 8 | tracer resumed with signal 0 | **hung forever** on exactly the binaries `verify` exists to catch |
| 9 | `poc_matrix` with no `poc_functional_test` | **11 green rows having executed nothing** |

⚠ **Nos. 6–8 were in code written this session**, and 8 was caught by CI —
the first time this workflow has found a defect rather than reported one.
⚠ **No. 9 affected no committed result**: all five pre-existing POCs define
the function. The harness could have certified a bad POC and had not yet.

## What was measured that had not been

| | |
|---|---|
| `experiments/70-carried-helper.sh` | a carried-in static Rust helper runs on **12 of 12** targets, exactly where `sh` does |
| `experiments/71-wrap-dlopen.sh` | `--wrap-dlopen`: **11 of 11**, zero host objects, **+544 bytes** |
| `experiments/72-static-host-plugin-abi.sh` | ⛔ a static executable's dynamic symbol table is **empty**, so a shared plugin can never call back into its host |
| `poc/60-leveldb` | the first **C++** and first **CMake** POC — **11 of 11** |
| chroot vs docker engines | **byte-identical** output for the same source |
