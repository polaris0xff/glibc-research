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

⚠ **Not vendored, but they exist upstream** and are one fetch away:
`choosing-a-work-model.md`, `gate.md`, `history.md`, `ingest.md`,
`initialize.md`, `lean-adoption.md`, `reviews.md`, `sessions.md`,
`template-sync.md`, `work-stages.md`.

Re-fetch:

```sh
PIN=620616638320147aa2465b304c1240b20eb2d097
curl -fsSL -o docs/methodology/NAME.md \
  "https://raw.githubusercontent.com/Azathothas/TEMPLATE/$PIN/docs/methodology/NAME.md"
```

⚠ **These files link to TEMPLATE docs that are not vendored here** —
`../conventions/prose.md`, `../security/remote-ops.md`, `gate.md`,
`sessions.md`, `history.md`, `choosing-a-work-model.md`, `work-stages.md`.
Those links do not resolve in this tree and that is expected: ⛔ **vendored
files are not edited**, per `vendoring.md`. Fetch the named file at the pin
above if you need one.
