# toolchain — pgb itself

Design: [`../docs/design/toolchain.md`](../docs/design/toolchain.md).

---

## T-010 — Split `pgb` into `tool/lib/*.sh`

**Source** operator, session of 2026-09-01.
**Category** toolchain · **Priority** P1 · **Effort** S · **Status** open

**Problem.** `pgb` is one file of ~730 lines and the dependency planner is not
written yet. It only grows from here.

**Premise.** ⭐ Decided, recorded in `docs/design/toolchain.md`: the driver
stays POSIX `sh` because `pgb build` re-enters itself inside the build
environment and `pgb verify` enters every target rootfs, where `sh` is the only
thing guaranteed present. ⚠ This is a decision to confirm, not one taken — see
T-011.

**Approach.** Sourced, not executed, so the re-entry stays one process:
`tool/lib/{common,env,wrappers,build,verify}.sh`, with `pgb` reduced to option
parsing and dispatch. No behaviour change in this entry.

**Prove.** `sh pgb doctor && sh pgb explain && sh pgb verify <a known binary>`
produce byte-identical output to the pre-split version, and
`sh experiments/60-versus-alternatives.sh` still exits 0.

## T-011 — Confirm or overturn the language decision

**Source** operator · **Category** toolchain · **Priority** P1 · **Effort** S · **Status** open

**Problem.** `docs/design/toolchain.md` records "keep the driver in POSIX sh"
as a recommendation. It has not been ratified, and it gets harder to revisit
the longer it stands.

**Premise.** The constraint that decides it: anything running *inside* the
build environment or a target rootfs must exist there. ⚠ Untested assumption —
whether a static Rust or Zig helper could simply be *carried in* has not been
tried, and if it can, the constraint weakens considerably.

**Decision.** Recommend `sh` for the driver, a real language outside. The
alternative loses on bootstrap, not on ergonomics. ⭐ **If the planner outgrows
the split in T-010, that is the signal to revisit — not a reason to have chosen
differently now.**

**Prove.** A one-paragraph ruling appended to `docs/design/toolchain.md` and
this entry closed with it quoted.

## T-012 — `pgb build <url-or-package>`

**Source** operator · **Category** toolchain · **Priority** P1 · **Effort** XL · **Status** open

**Problem.** A developer still has to know how to build the project.
`pgb build -- make` is a toolchain injector, not a toolchain.

**Premise.** ⭐ The interface is achievable over a large package database:
`nix bundle nixpkgs#chromium` does it today
(`docs/research/nix-appimage.md`). ⛔ Its store model must not be copied.

**Approach.** ⚠ **XL — this is two or more entries pretending to be one.**
Split before starting: spec resolution (URL or package name → source tree),
build-system detection, and the dependency planner are separate.

**Prove.** `sh pgb build <a git URL>` produces a binary that `pgb verify`
passes on 11 of 11, with no other input from the operator.

## T-013 — Measure developer friction

**Source** operator · **Category** toolchain · **Priority** P2 · **Effort** S · **Status** open

**Problem.** `docs/comparison.md` states the friction axis from one session's
record. Nothing re-runs it, so it goes stale silently.

**Approach.** `experiments/63-developer-friction.sh`: count external artefacts
fetched, files authored, environment variables required, and whether each route
completes unattended.

**Prove.** `sh experiments/63-developer-friction.sh` exits 0 and
`evidence/63-developer-friction/RESULT.txt` carries the counts.
