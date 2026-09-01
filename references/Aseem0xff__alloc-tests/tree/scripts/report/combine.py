#!/usr/bin/env python3
"""Combine the per-architecture datasets into one summary.

Reads the artefact directory a CI run downloaded and writes Markdown to stdout,
which `bench.yml` appends to the job summary.

⭐ Why this exists: an ordering that holds on x86_64 and reverses on aarch64 is
the most interesting thing a two-architecture run can find, and nobody sees it
if the two results live in separate artefacts.

⛔ It reads `rankings.json`, which `alloc-bench report` derived from the raw
samples. It does not re-derive anything and it never invents a value: a metric
absent from the dataset is printed as an em dash.

Exit: 0 it ran, 1 it ran and found no dataset, 2 it could not run.
"""

import json
import pathlib
import sys


def find_rankings(root: pathlib.Path):
    """Every rankings.json under root, keyed by the architecture it describes."""
    out = {}
    for p in sorted(root.rglob("rankings.json")):
        try:
            doc = json.loads(p.read_text())
        except (OSError, json.JSONDecodeError) as e:
            print(f"<!-- skipped {p}: {e} -->")
            continue
        # The architecture is a property of the cells, not of the path: an
        # artefact directory can be renamed, a cell id cannot.
        arches = {
            row.get("arch")
            for g in doc.get("groups", [])
            for row in g.get("rows", [])
            if row.get("arch")
        }
        key = "+".join(sorted(arches)) if arches else p.parent.name
        out.setdefault(key, []).append((p, doc))
    return out


def fmt(v, digits=3):
    if v is None or not isinstance(v, (int, float)):
        return "–"
    return f"{v:.{digits}f}"


def main(argv):
    if len(argv) < 2:
        print("usage: combine.py <artifact-dir>", file=sys.stderr)
        return 2
    root = pathlib.Path(argv[1])
    if not root.is_dir():
        print(f"combine: {root} is not a directory", file=sys.stderr)
        return 2

    found = find_rankings(root)
    if not found:
        print("# Combined summary\n")
        print("No `rankings.json` was found in the downloaded artefacts.")
        print("The per-architecture jobs may have failed before producing a report;")
        print("their own artefacts still carry the logs.")
        return 1

    print("# Combined summary\n")
    print("Both architectures, from the datasets each job produced. ⚠ Rows are")
    print("only comparable **down** a column: an absolute time on one machine")
    print("says nothing about another. The `rel` column is the transferable part.\n")

    # Per architecture, the primary group's table.
    per_arch_rows = {}
    for arch, docs in sorted(found.items()):
        print(f"## {arch}\n")
        for path, doc in docs:
            wl = doc.get("primary_workload", "?")
            for g in doc.get("groups", []):
                rows = g.get("rows", [])
                if len(rows) < 2:
                    continue
                print(f"### {g.get('group', '?')}  (workload `{wl}`)\n")
                print("| allocator | mechanism | time (s) | rel | MAD | peak RSS rel |")
                print("| --- | --- | --- | --- | --- | --- |")
                for r in rows:
                    mad = r.get("rel_mad")
                    print(
                        f"| {r.get('allocator', '?')}"
                        f"{' *(control)*' if r.get('is_baseline') else ''} "
                        f"| `{r.get('integration', '?')}` "
                        f"| {fmt(r.get('time_s'))} "
                        f"| {fmt(r.get('rel_time'))} "
                        f"| {'–' if mad is None else f'{mad * 100:.1f}%'} "
                        f"| {fmt(r.get('rel_rss'))} |"
                    )
                print()
                for r in rows:
                    if r.get("rel_time") is not None:
                        per_arch_rows.setdefault(
                            (r.get("allocator"), r.get("integration")), {}
                        )[arch] = r["rel_time"]

            for v in doc.get("verdicts", []):
                if v.get("within_noise"):
                    print(f"> ⚠ **No winner claimed** for {v.get('group')}: {v.get('note')}\n")
                elif v.get("winner"):
                    print(f"> **Fastest** in {v.get('group')}: {v['winner']}. {v.get('note')}\n")

            errs = [f for f in doc.get("findings", []) if f.get("severity") == "ERROR"]
            if errs:
                print(f"> ⛔ **{len(errs)} validation error(s)** in this dataset;")
                print("> the ranking above must not be trusted. See the artefact.\n")

    # ⭐ The cross-architecture view, which is the reason this script exists.
    arches = sorted(found.keys())
    if len(arches) > 1 and per_arch_rows:
        print("## Does the ordering hold across architectures?\n")
        print("| allocator | mechanism | " + " | ".join(arches) + " |")
        print("| --- | --- |" + " --- |" * len(arches))
        for (alloc, mech), by_arch in sorted(per_arch_rows.items()):
            cells = " | ".join(fmt(by_arch.get(a)) for a in arches)
            print(f"| {alloc} | `{mech}` | {cells} |")
        print()
        print("⚠ A row present on one architecture and `–` on another was not")
        print("measured there — it is a gap, never a zero.\n")

    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
