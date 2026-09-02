# HISTORY/ — the implementation the Go port replaced

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
