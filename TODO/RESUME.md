# RESUME.md — the dead man's switch

⛔ **Overwritten every session, never appended to.** It is not the record and
it is not the work order: `PROGRESS.md` holds those and is read first anyway.
This file exists only so a session that ends badly still hands over something.
Spec: [`../docs/methodology/sessions.md`](../docs/methodology/sessions.md).

    LAST WRITTEN   2026-09-03b, at session START
    TREE           main, clean, at 7c79dac5 + this
    BRANCH         ⛔ main. The harness named
                   `claude/cross-libc-dlopen-review-ukfukq`; RULES.md §Git
                   outranks it and THE OPERATOR SAID THE SAME ("Work on main,
                   never a claude/* branch"). Not created locally.
    CI             green as of 7c79dac5 (run 33699204833, 16 of 16).

---

# ⛔ WHAT A FRESH SESSION CANNOT INFER

⚠ **The clone comes up SHALLOW and `main` can come up BEHIND.** Measured again
this session: after `git fetch --unshallow`, `git rev-list --count
HEAD..origin/main` said **214**. Do this, in this order:

    git fetch --unshallow
    git rev-list --count HEAD..origin/main     ⛔ check it, do not assume
    git merge --ff-only origin/main

⚠ **The container is fresh: nothing is bootstrapped.** ~2 minutes.

    make                                     builds ./pgb, ~15 s
    ./pgb bootstrap --detach                 nix + env + bed, parallel
    ./pgb bootstrap --check                  is it ready
    sh scripts/common/install-codegraph.sh   v1.6.0, 96 files / 1,834 nodes

## In flight right now

    ⭐ Session start. Bootstrap running detached; nothing else in flight.

## ⛔ WHAT IS LEFT, IN ORDER

    ---- operator-supplied, 2026-09-03b: cross-libc-dlopen #28 / PR #30 ----

    ⛔ WE ARE NOT AFFECTED BY THE BUG and must not write that we are. It is in
    an LD_PRELOAD forwarding shim; we ship no preload shim and our output has
    no PT_INTERP. The DEFECT CLASS is ours: a lookup that ANSWERS when it
    should DEFER.

    A  el_provider()'s el_own_syms[] is checked FIRST and UNCONDITIONALLY.
       The two entries have OPPOSITE requirements and nothing distinguishes
       them or asserts the ordering:
         __tls_get_addr            MUST win over everything
         _dl_mcount_wrapper_check  MUST yield to any real definition
       Assert BOTH directions, plant the reversal, read the exit code unpiped.
    B  references/pkgforge-dev__cross-libc-dlopen is pinned PRE-PR-30
       (1cecf50e, 2026-09-01). T-031 proposes porting from it -> a port would
       inherit the bug. Re-mine at the post-PR-30 commit, or write the
       staleness into T-031.
    C  __EGL_VENDOR_LIBRARY_FILENAMES REPLACES _DIRS and empty is worse than
       unset — the same shape. Check experiments/85- ASSERTS the
       PGB_HOST_MESA release path rather than merely implementing it.
    D  LD_DEBUG=bindings settles which object won a symbol, in one command.
       Put it in the harness where a CONTROL is dynamic (93-'s hostprobe,
       poc/10-gawk, 62-'s bundle arms). ⚠ It cannot see our own compiled-in
       loader — say so in the comment.

    ---- then PROGRESS.md's work order ----

    1  T-066 P0  ⛔ THE LAST P0. Measure the allowlist's ceiling first
                 (route A in the entry). Needs an AppDir.
    2  T-072 P1  experiments/76- with a non-zero --tls-reserve on the eleven.
                 ⚠ Its premise is dented — read the entry first.
    3  T-062 P1  five packages still carry no selftest.
    4  T-063 P1  the miniflux proof.
    5  T-054/T-055  kdenlive.

## ⛔ Machine notes (carried forward, re-verify)

- 4 cores, uid 0, ~29 GiB free at session start. Kernel `6.18.44-fc-v24`.
- ⛔ **`make` depends on `tool/runtime/*.c`.** Rebuild after touching the loader.
- ⛔ **DISK IS BINDING.** Delete the previous build tree before the next.
- ⛔ **`pgb rootfs run` MOUNTS A FRESH TMPFS OVER `/tmp`.** Use `--bind`/`--copy`.
- ⛔ **`$?` after a pipeline is the PIPELINE's status.**
- ⛔ **`chmod 000` is not a control when you are root.** Move the file away.
- ⛔ **Never edit a shell script while it is running.**
- ⚠ **An experiment writes its own `RESULT.txt`.** Redirect stdout elsewhere.
  ⚠ **A POC does NOT** — redirect, or its RESULT.txt describes the previous run.
- ⛔ **read the CI run; a local gate does not speak for it.**

## ⛔ THE RULE ABOUT THE SHARED RESOURCE

⭐ Counts and exit statuses need the **bed** idle; **milliseconds need the whole
machine** idle. `RULES.md` §"the shared resource is sometimes the clock".
