# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log
and the entries.

    STATE     2026-09-01, session end
    COUNTS    15 entries, 14 open, 1 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, five POCs
              throughput: glibc 4.53 ns/op vs musl 584.71 (malloc, 4 threads)
              pgb over plain gcc -static, same workloads: 0.99x-1.05x

## What this session did

- Relocated the operator directive to `docs/REQUIREMENTS.md`, linked from §1.
- Built `experiments/60-` (eight delivery routes), `61-` (libc throughput) and
  `62-` (`pgb` vs `Anylinux-AppImages`). Evidence committed.
- ⛔ Corrected a wrong headline: `60-` measured startup and size and concluded
  static musl beat `pgb`. `61-` measures the axis the brief is about.
  `history/corrections.md` C7.
- ⛔ Corrected the AppImage arm: vanilla `appimagetool` bundles no glibc and
  scored 2/11; built as `Anylinux-AppImages` documents it scores 11/11.
- Five instrument defects found and fixed, all in `history/corrections.md`.
- Read the references the earlier sweep had not: `cross-libc-dlopen/docs/limits.md`,
  `pipewire/src/wrap/dlfcn.zig`, `standalone_musl`, anylinux's architecture.
- Reframed the docs: `pgb` is a toolchain, not a format
  (`docs/design/toolchain.md`), and the language decision is recorded there.
- Swept the nix-appimage family (T-020, done) — `docs/research/nix-appimage.md`.
- Vendored the methodology docs. ⛔ Eleven tracked files cited
  `docs/methodology/experiments.md` and it had never existed.
- Created this `TODO/` tree.

## In progress

Nothing. No entry is half-done.

## Work order

    T-001  T-002  T-003        harder POCs, until something breaks
    T-010  T-011               split pgb, ratify the language decision
    T-040                      run CI once
    T-030                      --wrap-dlopen
    T-012                      pgb build <spec>  -- split it first, it is XL
    then P2 by category

The argument for that order is in `INDEX.md` and is meant to be re-derived, not
re-argued.

## Open questions for the operator

1. ⛔ **`REQUIREMENTS.md` part 2 is not met and the reason has changed.** `pgb`
   is not beaten on portability or throughput by anything measured — it ties
   the anylinux AppImage — but it does not *beat* it, and it is behind on the
   class of software each can serve. Either `pgb` grows to reach that class, or
   "strictly better than every existing format" is replaced. ⛔ **That is the
   operator's call and an agent must not make it.**
2. **Should T-010/T-011 bump above the POCs?** Flagged as likely.
3. **Is a nixpkgs front end (T-022) in scope**, or does depending on nix defeat
   the point? T-020 argues the graph is worth taking and the store layout is
   not.
