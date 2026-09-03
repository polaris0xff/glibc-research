# HISTORY/ — what is finished

Two kinds of thing live here, and they are kept for different reasons.

| | what | why |
|---|---|---|
| [`entries/`](entries/) | ⭐ **the CLOSED `T-` entries, and the long-form detail of the open ones** | so `TODO/` carries only what is left |
| `<commit>/` | the shell and Python implementation the Go port replaced | it is the ORACLE the port was measured against |

## `entries/` — the record's closed half

⛔ **Added 2026-09-03c on the operator's instruction:** *"strip away the fat,
things that are already resolved and fixed and just send them straight into
/HISTORY/\*, the TODO/\* must be lean and contain only what's left"*, and
*"first strip them all, all the md files in ./TODO/\*, keep them slim like
TODO/INDEX.md is"*.

`entries/<category>.md` holds every entry whose status is `done`, verbatim.
`entries/<category>-open.md` holds the long-form findings that were cut out of
entries that are **still open** — the measurements, the corrections, the routes
costed and the routes killed. ⭐ **The entry itself stays in `TODO/`** and is
short; this is where the numbers it quotes were derived.

⭐ **Every id keeps its row in [`../TODO/INDEX.md`](../TODO/INDEX.md)**, which
is what stops any of it being rediscovered, and `sh TODO/check.sh` checks this
directory against those rows exactly as it checks `TODO/` — including that an
entry is filed on the side its status says (check 4b), that no id carries two
entries (4c), and that the relative links survived the move (check 6).

⛔ **Do not reopen an entry here.** A defect that still matters is a NEW entry
in `TODO/`.

## `<commit>/` — the retired implementation

Each directory is named by the commit that was HEAD when its contents were
retired, so `git show <that hash>:<original path>` is the same file in place,
with its whole history attached.

Nothing here runs and nothing here is on any path. It is kept because the
shell and Python implementation is the ORACLE the port was measured against:
`evidence/92-go-port/RESULT.txt` records byte-identical output from both sides
for the compiler wrappers, the NAR format, the package index and the nixpkgs
planner, and those comparisons are only re-runnable while both halves exist.

⛔ Do not edit anything here, and do not fix a defect here. A defect that
still matters is a defect in the Go implementation; one that does not is
history. `docs/history/corrections.md` is where a superseded finding goes.
