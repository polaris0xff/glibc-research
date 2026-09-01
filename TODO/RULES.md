# RULES.md — how this repository is worked on

Read with [`../docs/AGENTS.md`](../docs/AGENTS.md) §14, which carries the rules
about claims and language. This file carries the rules about *working*.

---

## Git

⛔ **Work on `main`. Commit and push as work progresses, not at the end.**
A session that does everything and pushes once has one atomic point of failure
and leaves nothing behind if it dies. Push each coherent step.

⛔ **Do not create `claude/*` or other agent-named branches.** They accumulate,
nobody prunes them, and they hide work from anyone reading `main`.

⚠ **If a temporary branch is genuinely needed** — a risky rewrite, a
bisect — prefix it `ephemeral-`, and delete it as soon as it has served:

```sh
git switch -c ephemeral-<what>          # local only unless it must be shared
git switch main && git merge --no-ff ephemeral-<what>
git branch -d ephemeral-<what>          # -d, never -D: it refuses if unmerged
git push origin --delete ephemeral-<what> 2>/dev/null || true
```

⛔ **`git branch -D` and `git push --force` are not cleanup.** `-d` refusing to
delete is the check working; find out why before overriding it.

⭐ **Leave no `ephemeral-*` branch behind.** `git branch --list 'ephemeral-*'`
should be empty at the end of a session.

## The record is part of the change

⛔ **`PROGRESS.md`, `INDEX.md` and the entry are edited in the same commit as
the work.** A session that fixes something and leaves the record saying it is
open has made the next session read a lie first.

Run the gate before committing:

```sh
sh TODO/check.sh
```

## No deferral

⛔ **Nothing closes as "won't fix", "upstream's problem" or "out of scope".**
A blocked entry stays open with the blocker named and what would unblock it.
See `../docs/methodology/work-todo.md`.

## Evidence

⭐ **Every entry closes with its `Prove` command actually run and the output
recorded in the entry.** A closed entry with no output is an opinion.

⚠ **Experiments are binding on `../docs/methodology/experiments.md`; sweeps on
`references.md`; vendoring on `vendoring.md`.** All three are vendored under
`../docs/methodology/`.
