# docs/methodology/

The three documents this project is bound by, mirrored here so a reader has them
without a network and so a later session can tell **which revision** the work was
done against.

| file | binding on |
| --- | --- |
| [`experiments.md`](experiments.md) | producing our own numbers |
| [`references.md`](references.md) | studying somebody else's project |
| [`vendoring.md`](vendoring.md) | third-party code living in this tree |

## Provenance

| | |
| --- | --- |
| upstream | `Azathothas/TEMPLATE`, `docs/methodology/` |
| taken at | `main`, fetched 2026-09-01 |
| local changes | **none** — byte-identical to upstream at the fetch date |

Re-fetch:

```sh
for f in experiments references vendoring; do
  curl -sSL -o "docs/methodology/$f.md" \
    "https://raw.githubusercontent.com/Azathothas/TEMPLATE/refs/heads/main/docs/methodology/$f.md"
done
```

⚠ These are copies for reference, not this project's own rules. Where this
project's practice is stricter or narrower, that is stated in
[`../methodology.md`](../methodology.md) with the reason.

⚠ They link to sibling documents (`history.md`, `../conventions/prose.md`,
`../security/remote-ops.md`) that are **not** mirrored here, because this project
does not use them. Those links will not resolve locally; follow them upstream.

## How they shaped this project

| rule | where it shows up here |
| --- | --- |
| an experiment is a numbered file in the tree, with the question in its header | [`../../experiments/`](../../experiments/) |
| every input pinned | `allocators/allocators.lock.json`, `toolchains/pins.env` |
| conditions printed on the way out | `run.json`, and the report's Conditions section |
| exit 0 / 1 / 2, and **2 is never a pass** | every script and both binaries |
| a negative result is committed | the unsupported rows, and `results/published/.../evidence/` |
| never a fabricated number; a dash where unknown | `report.rs`, and the validator that makes a missing value an error |
| measure from **outside** the thing you are measuring | `crates/alloc-runner/src/measure.rs` |
| the instrument is the deliverable | `alloc-runner` ships with every claim it produced |
| give the instrument an expectation flag and a non-zero exit | `identify --expect-*`, `aslr-probe --expect`, `archive-check --expect-providers` |
| a sweep keeps the corpus, tracked | [`../../references/`](../../references/) |
| read the tracker, not just the code | issues 245/247/256/258 shaped the design; see [`../AGENTS.md`](../AGENTS.md) §2 |
| vendored code is fixed here, and nothing is opened on anybody else's repository | [`../../scripts/vendor/README.md`](../../scripts/vendor/README.md) |
