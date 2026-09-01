# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

⚠ **This project commits it.** `sessions.md` leaves that to the project; here
it is tracked, so it survives the machine going away, which is the failure it
exists for.

⭐ **Written at the START of the session of 2026-09-01b and refreshed three
times**, which is the debt the previous session recorded against itself.

    LAST WRITTEN   2026-09-01b, session end
    TREE           clean, everything committed and pushed to main
    CHECKS         sh TODO/check.sh green
    BRANCH         main  (see the branch note below)

---

| | |
|---|---|
| **the task** | ⭐ **Mine the nix references named in [`../docs/design/nix-front-end.md`](../docs/design/nix-front-end.md), then re-scope T-022 and T-012 from what they say.** The operator ruled that **nixpkgs is the planner** — the six `mine-repo.sh` commands are written out in that page. |
| **the resume point** | `docs/design/nix-front-end.md` §"What the next session owes", step 1. Then `TODO/PROGRESS.md` "Work order". |
| **in flight** | ⚠ **T-032's two POC runs.** `poc/20-nano` and `poc/30-curl` are wired to `--embed-terminfo` / `--embed-cacert` and **neither has been run to completion** — the session ended during `poc/20`'s ncurses build. The entry names both steps and the confound to watch for. Nothing is half-*written*. |
| **the state of the tree** | Clean, pushed. No `ephemeral-*` branches. |
| **the paste** | `Read ./docs/AGENTS.md in full & follow.` |

## ⛔ Start here, in this order

1. **`docs/design/nix-front-end.md`** — the operator's ruling, quoted, plus the
   six repositories to mine and the questions the mining must answer. ⛔ **The
   mining is the first task; nothing on that page is verified.**
   ⚠ `pkgforge/soarpkgs` is already in `references/` — check which commit
   before re-mining, because the operator's pin
   `55c774a5e24d9f17af69911a4d70884dfb566626` is what makes it useful (newer
   commits abandoned the approach).
2. **Finish T-032** — two POC runs, both already wired. It is the only entry
   with landed, measured code and an unmet acceptance.
3. Then the work order in `PROGRESS.md`.

## ⛔ The branch rule, and what breaking it cost

This session was told **by its harness** to develop on
`claude/glibc-research-session-17ku6v` and did so for its whole length. The
operator's ruling: never again — `main`, or an `ephemeral-` branch merged in
the same session. `RULES.md` §Git now says the harness does not override it.

⚠ **Cleanup was only half possible.** `main` was fast-forwarded to the work and
pushed; the local branch was deleted after verifying `git log main..<branch>`
was empty. ⛔ **The remote branch could NOT be deleted**: this environment's git
proxy disconnects on both `git push origin --delete <b>` and
`git push origin :<b>`, and the harness's GitHub tools expose no delete-branch
call. **`origin/claude/glibc-research-session-17ku6v` is still on GitHub, at a
commit `main` already contains, and needs one click in the web UI to remove.**

## ⚠ What this machine does NOT have cached

**0 of 11 rootfs and no static libiconv at session start.** A fresh session
pays `sh pgb env create` + `sh scripts/common/fetch-rootfs.sh` (~1.5 GiB)
before anything can be built or verified. `dockerd` is not running by default;
starting it changes which engine `pick_engine` returns, which T-017 now
detects and refuses rather than failing inside a build.

⚠ **`/var/tmp/pgb-poc` holds ~1.5 GiB of built POC artefacts** — including
ffmpeg, MLT and a 105 MB `melt-static`. POC 80 reuses them and is fast if they
survive; from scratch it is 30–60 minutes.
