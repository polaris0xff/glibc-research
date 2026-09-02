# pkgforge/devscripts — `Linux/tss`

Fetched 2026-09-02, `github.com/pkgforge/devscripts/raw/refs/heads/main/Linux/tss/src/main.rs`.

⛔ **Fetched from `refs/heads/main`, NOT a commit.** Everything else in
`references/` is pinned; this one is not, because the operator gave the branch
URL and the session had no budget to resolve it to a sha. ⚠ **Pin it before
citing a line number** — `scripts/common/mine-repo.sh` is the tool for it.

| | |
|---|---|
| what it is | `tss`, a timestamp-stamping stream filter: reads a subprocess's output and prefixes every line with elapsed/wall time |
| why it is here | **T-061**. The operator: *"Ensure the pgb builder looks like docker build etc i.e it shows live logs with `ts` like timestamp, configurable."* This is the reference implementation of that behaviour |
| licence | see the upstream repository; not fetched with this file |
