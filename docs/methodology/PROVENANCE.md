# Vendored methodology

Upstream: `Azathothas/TEMPLATE`, pinned at commit
`620616638320147aa2465b304c1240b20eb2d097`, path `docs/methodology/`.

Fetched via `raw.githubusercontent.com` (the `github.com/.../raw/` route
returns 403 through this environment's proxy; `api.rv.pkgforge.dev` also
works). Unmodified — do not edit these in place; re-fetch at a newer pin and
record the change here.

| file | sha256 (first 16) | why it is here |
|---|---|---|
| `experiments.md` | `a02b56b9736a66cb` | binding on every `experiments/NN-*.sh`. ⛔ **Eleven tracked files cited this path before it existed** |
| `references.md` | `657a42fb43083c0a` | binding on any clone/mine/survey task |
| `vendoring.md` | `0367f6ec0be6ed34` | `tmp/START.md` names it required reading |
| `work-todo.md` | `2118118acf3ec1b3` | defines the `TODO/` model this repo uses |
| `authoring.md` | — | how a `TODO/` entry is written |
| `sessions.md` | `1c7bc4a9a7838276` | ⭐ the spec for the session boundary — `RESUME.md`, the summary table's rows, and the next prompt. `work-todo.md` cited it and it had never been fetched |
| `history.md` | `76aad32279d81445` | where a superseded explanation goes; cited by `experiments.md` and `vendoring.md` |

⚠ **Not vendored, but they exist upstream** and are one fetch away:
`choosing-a-work-model.md`, `gate.md`, `ingest.md`, `initialize.md`,
`lean-adoption.md`, `reviews.md`, `template-sync.md`, `work-stages.md`.

Re-fetch:

```sh
PIN=620616638320147aa2465b304c1240b20eb2d097
curl -fsSL -o docs/methodology/NAME.md \
  "https://raw.githubusercontent.com/Azathothas/TEMPLATE/$PIN/docs/methodology/NAME.md"
```

⚠ **These files link to TEMPLATE paths that are not vendored here.** ⭐ The
list is derived, not remembered — it is every link under `docs/methodology/`
that does not resolve, and re-deriving it is one command:

```sh
# ⚠ PROVENANCE.md is excluded: it is THIS project's file, not a vendored one,
# and its own example below otherwise matches itself.
for f in $(ls docs/methodology/*.md | grep -v PROVENANCE); do
  grep -oE '\]\(([^)#][^)]*)\)' "$f" | sed 's/](\(.*\))/\1/' | while read -r l; do
    case "$l" in http*|\#*) continue ;; esac
    [ -e "docs/methodology/${l%%#*}" ] || echo "$l"
  done
done | sort -u
```

    ../../scripts/doctor/     ../conventions/shell.md    gate.md
    ../conventions/code.md    ../security/remote-ops.md  reviews.md
    ../conventions/prose.md   choosing-a-work-model.md   work-stages.md

Those links do not resolve in this tree and that is expected: ⛔ **vendored
files are not edited**, per `vendoring.md`. Fetch the named file at the pin
above if you need one.
