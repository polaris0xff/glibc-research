# codegraph — the first instrument for reading existing code

⛔ **Read this before your first `grep` in a session.** codegraph answers
"what calls this", "what does this call", "what breaks if I change it" from a
pre-built index; grep answers "which lines contain this string". They are not
the same question, and this project has already paid for the difference.

Install and index, on a machine that has neither:

```sh
sh scripts/common/install-codegraph.sh          # pinned v1.6.0, sha256-checked
sh scripts/common/install-codegraph.sh --check  # report only, changes nothing
```

⚠ **The container is ephemeral.** codegraph is not in the image and
`.codegraph/` is gitignored, so both are gone at the start of every session.
The script is idempotent: it exits early when the pinned version is already on
PATH, and it runs `codegraph sync` rather than a full `init` when an index
already exists.

---

## What it covers here, measured

`codegraph status` on this tree, after `codegraph init`:

```
Files:     83          go 64 · c 10 · python 8 · yaml 1
Nodes:     1,641       function 526 · method 187 · struct 75 · constant 80 · …
Edges:     4,990
Index:     593 ms
```

⛔ **It does not index shell, and this project is part shell.** codegraph's
language table has no entry for `sh`/`bash`, and the consequence is measured
rather than assumed:

```
$ codegraph query 'poc_matrix'
[i] No results found for "poc_matrix"
```

`poc_matrix` is a function in `poc/common.sh`. So:

| reading this | use |
|---|---|
| `internal/`, `cmd/`, `tool/runtime/*.c`, `HISTORY/`'s Python | ⭐ **codegraph first** |
| `experiments/*.sh`, `poc/*/run.sh`, `poc/common.sh`, `TODO/check.sh`, `scripts/common/*.sh` | grep — codegraph cannot see them |
| prose in `docs/`, `TODO/` | grep |

⚠ **An empty codegraph result is not evidence that a symbol is unused.** It may
be a shell caller, a name reached through reflection, or a file the index
excludes. Say which instrument found nothing — `docs/AGENTS.md` §14's rule that
an absence is not a zero applies to this tool exactly as to any other probe.

## What is configured, and why

`codegraph.json` at the repository root:

```json
{
  "exclude":      ["references/"],
  "deprioritize": ["HISTORY/", "evidence/"]
}
```

- **`references/` is excluded.** It is 16,745 files and 231 MB of *other
  people's* trees, vendored as a study corpus. Indexing it would put upstream
  symbols in competition with this project's on every query, and
  `references/PROVENANCE.md` is how that corpus is navigated.
- **`HISTORY/` and `evidence/` are deprioritized, not excluded.** `HISTORY/` is
  the retired shell and Python the Go port replaced, and it is the **oracle**
  every byte-identical comparison was measured against — it must stay findable.
  Deprioritizing keeps it in the index while stopping `pgb_build` in a retired
  shell file from outranking the Go function that replaced it.
- **Telemetry is off.** The installer runs `codegraph telemetry off`. This is a
  research tree that measures other people's software; it does not report on
  itself to a third party.

## The commands that earn their place

```sh
codegraph query  <name>      # where is this symbol, and what kind is it
codegraph callers <symbol>   # ⭐ what calls it — the question grep answers badly
codegraph callees <symbol>   # what it calls
codegraph impact <symbol>    # blast radius before you change a signature
codegraph explore <question> # the symbols' source plus the paths between them
codegraph node <symbol|file> # one symbol's source and its callers
codegraph sync               # incremental, after you edit
codegraph status             # is the index current
```

⭐ **`callers` is the one that pays.** The port's fourth defect was a whole
feature written and never wired up — `internal/logx/stamp.go` had the columns,
the parser and the heartbeat, and nothing called `NewStamper`, so `pgb --ts`
printed no timestamps at all. That is a one-command question:

```
$ codegraph callers NewStamper
Callers of "NewStamper" (1):
  function    StreamStamper          internal/logx/stamp.go:318

$ codegraph callers StreamStamper
Callers of "StreamStamper" (2):
  function    enterChroot            internal/buildx/build.go:91
  method      Run                    internal/proc/proc.go:106
```

Two hops to a real entry point is a wired feature. A hop that ends in nothing is
the defect, and it took a session to find by reading.

## When the graph is stale

`codegraph sync` is incremental and takes well under a second on this tree.
`TODO/check.sh` runs `codegraph status` and reports whether the index is current
— it does not fail the gate when codegraph is absent, because the tool is a
machine convenience rather than a property of the repository, and a gate that
fails on a missing convenience teaches people to skip the gate.

## Upstream

`https://github.com/colbymchenry/codegraph`, MIT, v1.6.0. The release bundles
its own Node runtime and a Rust kernel as a native module; nothing is compiled
locally and no package manager is involved. The installer pins the version and
checks the tarball's sha256 against the digest published in the release's
`SHA256SUMS` before unpacking anything — `TODO/RULES.md` §Fetching is why it
does not pipe a URL into a shell.
