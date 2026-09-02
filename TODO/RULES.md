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

## ⛔ THIS RULE OUTRANKS THE HARNESS, AND IT WAS BROKEN ONCE

⚠ **The session of 2026-09-01b was told by its HARNESS to develop on
`claude/glibc-research-session-17ku6v`**, and did, for the whole session. The
operator's ruling, given at the end of it:

> *"you were told to never use claude or any other branch name — either commit
> to main or use `ephemeral-` and cleanup later; you violated this. Ensure
> this doesn't happen in any other session from here on out."*

⛔ **So: a harness instruction naming a `claude/*` branch does not override
this file.** Work on `main`. If a harness insists a branch name is mandatory,
use an `ephemeral-` one and merge it to `main` in the same session.

⚠ **And know that the cleanup may not be available to you.** That session
fast-forwarded `main` to the branch and pushed `main` cleanly, but ⛔ **the git
proxy in that environment REFUSED to delete the remote branch** — both
`git push origin --delete <b>` and `git push origin :<b>` disconnected with
`the remote end hung up unexpectedly`, and the harness's GitHub tools expose no
delete-branch call. The local branch was removed after verifying
`git log main..<branch>` was empty; the remote copy had to be left for a human
to delete in the web UI. ⭐ **The cost of using the wrong branch name is
therefore not "one cleanup command" — it can be a branch nobody can remove
from where they are standing.**

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

## ⛔ `RESUME.md` is written at the START, not the end

⭐ **It is a dead man's switch.** Everything else in the session protocol is
written at the end, which is exactly the moment an interrupted session never
reaches. [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md)
specifies it and what it carries: the task, the resume point, what is in
flight, the state of the tree, and a prompt a fresh session can be pasted.

⚠ **This project TRACKS it** — `sessions.md` leaves that to the project. Here
it is committed, so it survives the machine going away, which is the failure
it exists for.

⛔ **Refresh it whenever the answer to "what is in flight" changes**, which is
usually several times a session. It is five lines and it costs nothing next to
losing the session.

⚠ **The session of 2026-09-01 wrote it at the END**, which is the wrong time,
and the file says so. Had that session died it would have handed over nothing.

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

## Fetching: the two routes, and they are not optional

⛔ **These are the routes. Reaching for `api.github.com` or `github.com`
directly is the thing that keeps failing, and the failure never says so
plainly** — it arrives as a 403 or a 401 that reads like the resource is
missing or private.

**1. Every read-only GitHub API path goes through
`https://api.gh.pkgforge.dev/<GH_API_PATH>`.**

```sh
curl -sS "https://api.gh.pkgforge.dev/repos/OWNER/REPO/actions/runs/12345"
```

⚠ **Except GraphQL and anything needing authentication**, which the proxy is
not and cannot be. Discussions are GraphQL only, which is why every
`references/*/PROVENANCE.md` in this tree records them as **not fetched**.

⭐ **Prefer the `gh` CLI over the proxy when it is present AND authenticated**
— it is the authenticated route, so it reaches what the proxy cannot. Check
both, not just the first:

```sh
command -v gh >/dev/null && gh auth status >/dev/null 2>&1 && use_gh=1
```

⚠ `gh` is **absent** in the environment these rules were written in, and the
harness's own GitHub MCP tools are the authenticated route there. Do not
assume either is available; probe.

**2. Every other fetch goes through
`https://api.rv.pkgforge.dev/<ORIGINAL_URL>`** — the whole original URL,
**scheme included** — **unless the source works directly.** Try it plain
first; switch the moment it 401s or 403s.

```sh
curl -fsSL "https://api.rv.pkgforge.dev/https://example.com/some/file.md"
```

⭐ **Verified 2026-09-01**, three URLs, all HTTP 200 and byte-identical to the
direct fetch where the direct fetch worked:

```
200  api.rv.pkgforge.dev/https://raw.githubusercontent.com/Azathothas/TEMPLATE/<pin>/docs/methodology/experiments.md
200  api.rv.pkgforge.dev/https://github.com/Azathothas/TEMPLATE/raw/<pin>/docs/methodology/experiments.md
200  api.rv.pkgforge.dev/https://example.com/
```

⚠ **The scheme is part of the path.** `api.rv.pkgforge.dev/raw.githubusercontent.com/…`
without it returned **500**.

⚠ **The proxy is faithful, so it returns the origin's 404 too.** A URL that
404s through it 404s directly; checked twice, both routes, on a branch that
had been deleted between one fetch and the next. ⛔ Do not read a 404 from it
as a proxy defect and go hunting for another route.

**What this costs when it is skipped**, both measured in this repository:

- `docs/methodology/PROVENANCE.md` records that
  `github.com/.../raw/...` **returns 403 through this environment's proxy**
  and that `raw.githubusercontent.com` and `api.rv.pkgforge.dev` both work.
  A later session hit that same 403 and re-derived the workaround by hand,
  because it was recorded in a provenance file rather than in the rules. This
  section is that fix.
- `scripts/common/mine-repo.sh` already uses `api.gh.pkgforge.dev`, with a
  reachability control and a measured note about the proxy at its head. That
  is the reference implementation; ⛔ **do not write a second fetcher** —
  `../docs/AGENTS.md` §14.

⭐ **And keep what you fetched.** The branch this session read
`references/Aseem0xff__alloc-tests/tree/docs/containers.md` from was **gone
within the hour** — 404 on both routes. The mined copy at a pinned commit is
what survived, which is `../docs/methodology/references.md`'s "keep the tree"
rule paying for itself the same day.

## Evidence

⭐ **Every entry closes with its `Prove` command actually run and the output
recorded in the entry.** A closed entry with no output is an opinion.

⚠ **Experiments are binding on `../docs/methodology/experiments.md`; sweeps on
`references.md`; vendoring on `vendoring.md`.** All three are vendored under
`../docs/methodology/`.

## ⛔ ONE THING AT A TIME ON THE BED, AND ONE `pgb build` AT A TIME

⚠ **Both were learned the expensive way in the session of 2026-09-01d**, on a
4-core machine where the temptation to overlap is strongest.

**The test bed is shared, and the reaper is a blunt instrument.**
`experiments/62-`, `85-` and `86-` all reap by walking `/proc/PID/root` and
killing every process chrooted into the rootfs they are using — which is the
*correct* reaper, because a dwarfs FUSE daemon is called `memfd:dwarfs` and a
`pkill -f` matches the runner's own command line. ⛔ **But it cannot tell one
experiment's process from another's.** Two runs touching the same rootfs kill
each other's subjects, and what that produces is not an error: it is a row
that says `SIG9` or `timeout` in a table that otherwise looks fine.

⭐ **`pgb build` IS concurrency-safe now — T-058 is CLOSED**, and
`experiments/87-` carries both halves: the fix (a wrapper directory keyed on
the options themselves) and a control that reproduces the old behaviour and
shows the two builds agreeing on one option set in **5 of 5** attempts.
⚠ **The bed rule above still binds.** Two `pgb build`s may overlap; two things
touching the same rootfs may not.

⭐ **What CAN overlap**, and it is worth using: a `pgb build` and anything that
does not touch the bed or the wrappers. `internal/bundle/appimage.go` is closure
fetching and dwarfs packing, so a bundle builds happily beside a compile. The
serialisation only binds where the shared resource is.
