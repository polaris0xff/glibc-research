# tmp/ — two files that are not temporary, and why they stay here

⛔ **Neither file in this directory is retirable, and the question has now been
asked twice.** This page exists so it is not asked a third time.

The standing instruction is to retire documents that are *not used, not
referenced, and already studied* into [`../HISTORY/`](../HISTORY/). Both files
here fail that test on the second clause.

## `START.md` — the operator's original brief

⛔ **Not this project's to edit, and not this project's to move.**

It is the brief the whole repository answers, and it is quoted as the authority
on what was actually asked for by
[`../docs/AGENTS.md`](../docs/AGENTS.md),
[`../docs/comparison.md`](../docs/comparison.md),
[`../docs/history/corrections.md`](../docs/history/corrections.md),
[`../docs/methodology/PROVENANCE.md`](../docs/methodology/PROVENANCE.md) and
three experiments. `../scripts/common/check-docs.sh` excludes it from the docs
gate for the same reason.

⭐ **And it is not retired.** `HISTORY/` holds superseded implementations that
serve as the oracle a gate is measured against. A brief that is still the
authority on scope is a different kind of document, and filing it as history
would say it had been superseded.

## `static-glibc-nss-dynamic-loading.md` — a study document

Referenced from exactly one place: inside `START.md`.

⛔ **Which is why it cannot move.** Its only inbound reference lives in a file
that must not be edited, so relocating it converts a working reference into a
dangling one in the one file nobody is allowed to repair.

## If this is revisited

Moving `START.md` means repointing eight tracked files and removing the docs
gate's exemption for it, and the operator's own brief would then live under a
directory that means "superseded". Both gates must be re-run afterwards:

    sh TODO/check.sh
    sh scripts/common/check-docs.sh
