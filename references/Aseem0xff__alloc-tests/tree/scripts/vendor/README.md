# scripts/vendor/

Third-party code living in this tree. Governed by
[`docs/methodology/vendoring.md`](../../docs/methodology/vendoring.md).

## mine-repo.sh

| | |
| --- | --- |
| upstream | `Azathothas/TEMPLATE`, `scripts/common/mine-repo.sh` |
| taken at | `main`, fetched 2026-09-01 |
| licence | permissive (see upstream `LICENSE`: use, copy, modify, distribute granted) |
| why it is here | [`docs/methodology/references.md`](../../docs/methodology/references.md) states plainly: do not write your own fetcher. This project needs one, so it takes that one. |

### Local changes

**None.** The file is byte-identical to upstream at the fetch date.

### Acceptance command

The script carries its own offline selftest. It is the check that a future
copy still works:

```sh
sh scripts/vendor/mine-repo.sh --selftest
```

Exit 0 means the page joiner is intact. This runs in CI
(`.github/workflows/ci.yml`), so a broken re-vendor is caught rather than
discovered during a sweep.

### Re-fetching

```sh
curl -sSL -o scripts/vendor/mine-repo.sh \
  https://raw.githubusercontent.com/Azathothas/TEMPLATE/refs/heads/main/scripts/common/mine-repo.sh
```

If a future upstream revision changes it, reconcile by reading, per
`vendoring.md`: this tree has no patch to lose, so a clean re-fetch plus a
passing `--selftest` is the whole reconciliation.
