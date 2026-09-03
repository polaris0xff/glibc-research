# SUMMARY.md — the session of 2026-09-03d

⛔ **Overwritten every session.** The narrative is
[`../HISTORY/sessions/2026-09-03d.md`](../HISTORY/sessions/2026-09-03d.md);
the work order is [`PROGRESS.md`](PROGRESS.md).

    SCOPE     the operator: "focus the next session entirely on optimizing
              the nix bundler as much as possible" (PROGRESS.md N0–N6),
              then mid-session: defer the speed comparison, reprioritise
              the glibc-static and nix-bundle quirks, and mine every named
              reference with three passes each.
    RESULT    ⭐ THE BUNDLER IS LEVEL OR AHEAD ON SPEED, on a CLI and on a
              GUI, and the closure never changed.

## ⭐ What moved

| | before | after |
|---|---|---|
| `jq` cold start vs the field, 11 environments | 2.07× | ⭐ **1.00×**, ours faster on 6 of 11 rows |
| kdenlive cold start vs the competitor | 4.92× against us | ⭐ **0.74× — ours is faster** |
| kdenlive host shared objects | — | ⭐ **0 of 11**, against the competitor's 4 of 11 |
| `jq` artefact size | 2.86× | 1.70× ⚠ (1.44×, then `-S18` cost +17.8%) |
| kdenlive artefact size | 2.45× | ⛔ 2.95× |

⛔ **Two constants in `internal/bundle/appimage.go` did all of it**: uruntime
`full` → `lite` (`experiments/77-`, 0.69–0.76×) and the dwarfs block `-S26`
(64 MiB) → `-S18` (256 KiB) (`experiments/81-`, 0.66× on a large artefact).

## ⭐ The five findings behind it

1. **The instrument was measuring the wrong thing.** `90-`'s cold column
   obtained "cold" by copying the artefact; uruntime keys its mount on
   **content** and holds it **5 s**, so the copy reused the live mount and the
   column reported a warm start — 1.02× of warm, measured. `corrections.md`
   **C24**, and it is C23's missing mechanism.
2. **Size is not the time column.** `experiments/84-`: a 29.6× image and a
   138× file count each move cold start ~1.05×; image size costs
   **0.024–0.031 ms/MiB**, so the whole 196 MiB between the two kdenlive
   bundles is ~5 ms of a gap never seen below 129 ms. ⛔ N1's premise is gone.
3. **We were not running the runtime we were measured against.** `86-` stages
   the competitor's own toolchain — uruntime **lite**; we shipped **full**.
4. **The pin was decorative.** `download` returned early whenever the
   destination existed, whatever URL was asked for, so moving a pin changed
   nothing on any machine that had built a bundle. Now keyed on the URL.
5. **The dwarfs block size is a lever, and the sweep had to run three times**
   to find its minimum — 1 MiB, then 256 KiB, both monotonic to their own
   floor. 64 KiB is the minimum and is **not** shipped: 0.02× more for another
   19% of the artefact.

## ⭐ The reference sweep, and what it corrected

Every reference the operator named is vendored and pinned; three passes each.
⭐ **It corrected five claims this session had already published** — the
mount/extract selector is a **patchable constant**, not an environment
variable; `lite` drops **`dwarfsck`/`mkdwarfs`**, not codecs; the field's icon
rule is **at least 128×128**, the opposite of what was recorded; its desktop
rule takes the **first** match, not the smallest; and gearlever's gate is
**GIO's content type**, not the type-2 magic (`gio info` says
`application/vnd.appimage` for ours and the competitor's alike).

⭐ **And `experiments/99-`'s bisected 4–6 s reuse window turned out to be a
source constant**: `REUSE_CHECK_DELAY = "5s"`.

📚 [`../docs/research/nix-bundle-patching.md`](../docs/research/nix-bundle-patching.md),
[`../docs/research/bundle-capabilities.md`](../docs/research/bundle-capabilities.md).

## ⭐ What was built

| | |
|---|---|
| `experiments/clock.sh` | the wall-clock instrument: median of N, arms interleaved with a rotating start, and an **A/A control** whose ratio is the floor below which no row may be believed |
| `experiments/99-` | stands it up and **asserts** the control; found C24 |
| `experiments/84-` | is size the time column — no |
| `experiments/77-` | the runtime, five ways, one component at a time |
| `experiments/81-` | the dwarfs block size, seven ways, two subjects |
| `experiments/90-` | rebuilt on the corrected protocol; kdenlive re-measured |
| `lib.sh` `exp_pack_blocksize` | reads the block size out of the Go source so no experiment can copy a stale one |

## ⛔ Defects found in this session's own work

⭐ **Every review found one.** `clk_run_twin` was dead code that also passed an
empty argument. `90-`'s cache stamped on `[ -s ]`, so a truncated 99 MB
artefact would have been reused as a whole one. `77-` borrowed
`$CACHE/tools/mkdwarfs` and silently changed the meaning of three arms when the
pin moved. `86-`'s **warm** column subtracts a cold run that is not in the
series it divides. `92` was not a free number — `evidence/92-go-port` had it.
`check-docs.sh` enumerated tracked files only, so a **new** document was
unchecked by exactly the commit the gate exists to block; **CI caught that one,
not the local gate.**

⛔ **And a process failure**: `experiments/90-` was edited **while running**,
which cost a run. `sh` re-reads from a byte offset.

## ⛔ What is NOT done

1. `86-`'s warm arithmetic is unverified — `clock.sh` is the shape to carry in.
2. kdenlive's **warm** row is 3.45× against us and unexplained. First
   candidate: at 565 MB it is over uruntime's 350 MB threshold, so it
   **extracts** where `jq` **mounts**.
3. kdenlive's **render** direction is unresolved — two runs disagree.
4. `78-`, `85-`, `89-` carry evidence describing the old runtime.
5. `defaultSharunURL` is pinned to `latest`, in a constant block whose own
   comment says that is the thing not to do.
