# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-02, at the operator's stop
    TREE           main, clean, pushed at 79c7e054
    BRANCH         ⛔ main. The harness named `claude/glibc-nix-static-v2nttp`
                   and RULES.md §Git outranks it. `git ls-remote --heads
                   origin` lists refs/heads/main ONLY.

---

# ⛔ NEXT SESSION: T-061, AND NOTHING ELSE

**Read [`toolchain.md`](toolchain.md) §T-061, then
[`../docs/design/porting-report.md`](../docs/design/porting-report.md) IN
FULL.** Port the tooling to Go, reach parity, ship one static `pgb`.

⛔ **Do not pick up any other entry.** The operator: *"After the next session
ports the whole thing to go, the next session after that will return back to
usual tasks."* Anything that edits shell or Python under `tool/` or `scripts/`
before the port is work the port throws away.

⚠ **`docs/design/porting-report.md` is a session artefact, not a permanent
document.** T-061 deletes it once its content has moved into
`design/toolchain.md` — the operator asked for that explicitly.

---

| | |
|---|---|
| **the task** | This session was measurement work; the operator stopped it twice. First for a docs review (done: `PROGRESS.md` rewritten, `scripts/common/check-docs.sh` written, `gate.md`/`reviews.md` vendored, six doc defects fixed), then to record **T-061**. |
| **the resume point** | **T-061.** Everything it needs is committed. |
| **the paste** | `Read ./docs/AGENTS.md in full & follow. Your only work is TODO/toolchain.md §T-061.` |

## Not lost, but not finished either

    ⚠ experiments/90- IS FIXED AND NOT RE-RUN. The onelf arm's argv[0]
      dispatch defect was ours, not onelf's. The recorded onelf row in
      evidence/90-kdenlive-vs-enhanced/ is therefore the WRONG one and says
      so nowhere. Re-run it after the port.

    ⚠ 488,934,276 bytes of the kdenlive AppDir's lib/ (2,300 files, 39%) is
      unreachable from the four programs or any plugin directory. That is
      T-055's route to the bar. The sweep was a scratch script and was NOT
      committed; T-061 requirement 6 says to rewrite it in Go.

    ⚠ T-060 rung 1 was building in the background when the session stopped:
      31 of nix's dependencies in /var/tmp/pgb-t060/prefix, boost in flight.
      ⛔ /var/tmp is ephemeral. Treat it as gone; the run is idempotent.

## ⛔ Machine notes a fresh session cannot infer

- **nix IS installed** here (Determinate Nix, pkgforge installer). ⚠ Its flake
  route is broken — the harness proxy answers `api.github.com` with 403, so
  `nixpkgs#attr` fails. `nix-instantiate '<nixpkgs>' --attr X` works.
- **`pgb env create` ignores a trailing `--engine`**; the global one works:
  `sh pgb --engine chroot env create`.
- **4 cores, ~15 GiB RAM.** ⛔ **DISK IS THE BINDING CONSTRAINT**: 7–9 GiB free
  with one AppDir on disk. Delete `AppDir` and `store` under
  `$PGB_APPIMAGE_CACHE` as soon as an artefact is measured.
- ⛔ **DO NOT EDIT A SHELL SCRIPT WHILE IT IS RUNNING.** `sh` re-reads from a
  byte offset, so an edit mid-run corrupts the running process: it cost a
  20-minute kdenlive pack with `Syntax error: end of file unexpected`. Copy the
  tree to `/var/tmp/frozen-<what>` keeping the repo's layout — every tool
  resolves its siblings from its own path — or wait.
  ⭐ **This whole class is why T-061 exists.**
