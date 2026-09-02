# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log
and the entries.

    STATE     2026-09-02
    COUNTS    34 entries, 16 open, 18 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, NINE POCs
              CI: GREEN, 15 jobs, and it asserts criterion 2
              throughput: glibc 4.53 ns/op vs musl 584.71 (malloc, 4 threads)
    NEW       ⛔ T-061 IS P0 AND IT IS THE NEXT SESSION'S ONLY WORK

## ⛔ READ THIS BEFORE ANYTHING ELSE: the next session ports the tooling to Go

⭐ **Operator ruling, 2026-09-02**, and it is not a suggestion about ordering:

> *"when some backticks in some comments inside a shell script break everything
> and lead to hours of wasted time, i think it's time we rewrite the tooling
> properly … create a P0 XL task to port everything to go in next session and
> pass all tests/experiments, reach current feature parity … After the next
> session ports the whole thing to go, the next session after that will return
> back to usual tasks."*

**The entry is [`T-061`](toolchain.md), and it carries the whole brief**: the
reading order, the six operator requirements beyond "port it", and the six
workload gates that stand in for "done". ⛔ **Do not start from this file's
summary of it.** Read the entry, then
[`../docs/design/porting-report.md`](../docs/design/porting-report.md) in full.

⚠ **Everything else in this file is context for the session AFTER that one.**
An entry below that touches shell or Python under `tool/` or `scripts/` is work
that the port will throw away.

## ⭐ The operator's three goals

Quoted, because the framing is load-bearing:

> *"1. Make the 'universal' builder true via pgb + nix. 2. make the 'universal'
> bundler true via a modern, updated, maintained 'nixappimage' descendant that
> uses or rather reimplements many of the anylinux tooling, iterating/improving
> them, and debloating nixappimages, correctly packing them, and also solving
> the opengl problem ... 3. poc a kdenlive static (exhaust all resources), if
> impossible, pivot to kdenlive.nixappimage, but it must be smaller, load
> faster, run faster than pkgforge-dev/kdenlive-AppImage-Enhanced."*

| goal | entries | where it stands |
|---|---|---|
| 1. the builder | T-050 ✅, T-051, T-060, T-012 | ⭐ **T-050 CLOSED**: `experiments/88-` plans, fetches **and builds** a nixpkgs package with no nix and no root, 25 assertions. ⚠ It still needs a C toolchain on the host, which is T-051; **T-060** is the static-glibc nix that removes the last crutch and is **rung 1 of 3**, 31 of nix's dependencies built |
| 2. the bundler | T-057 ⚠started, T-052 ✅, T-053 ✅ | ⭐ **items 1, 3 and 4 landed**: debloating with a three-arm control (`experiments/89-`), wrapper environments lifted into `.env`, and the lib32 path. ⛔ **item 2, a 32-bit application, is still untried** |
| 3. kdenlive | T-054, T-055 | ⭐ **rung 2 climbed**: `poc/91-qt-xcb` — a static Qt 6 opening a **real xcb window**, 26 assertions, 11/11, zero host objects. ⛔ **T-055's bar is NOT met**: ours 395,294,317 B against 191,900,604 B |

## What the last session did

### 1. T-054 rung 2 — a static Qt 6 application, with a real display

⛔ **The operator refused the previous rung as too narrow**: `poc/90-qt` built
Qt with `-no-xcb -no-opengl -no-network -no-sql` and an **offscreen** QPA —
*"not a Qt application, a Qt library that links."* `poc/91-qt-xcb` answers it:
the X stack built static from the qtbase plan, `TEST_xcb_syslibs` overridden
**with the static link proved first as the evidence**, `Q_IMPORT_PLUGIN` on the
real platform plugin, and a probe that asserts an xcb connection, a screen, an
**exposed** window, `QWidget::grab()`, OpenSSL linked and `QSQLITE`.

    47,188,344 B static, no PT_INTERP, 0 DT_NEEDED
    11 of 11 pass; host shared objects loaded: none, on every row

### 2. T-055 — the kdenlive comparison exists, and the bar is not met

| | ours | kdenlive-AppImage-Enhanced | onelf, our payload |
|---|---|---|---|
| size | 395,294,317 B | **191,900,604 B** | 500,644,533 B |
| render (melt → MP4) | 2,698 ms | **551 ms** | ⚠ not re-measured |
| cold / warm start | 1,725 / 114 ms | **972 / 37 ms** | ⚠ not re-measured |
| runs on the eleven | **11 of 11** | 11 of 11 | ⚠ |
| **host objects loaded** | ⭐ **0 on all 11** | 1–10 on every glibc row | ⚠ |

⭐ **One column is ours and it is the one the project is about**: the
competitor loads host shared objects on all seven glibc rows and we load none
anywhere. ⛔ **Three columns are theirs** and the entry says so in its own
words rather than around them.

⭐ **The route to the bar is measured, not guessed**: a reachability sweep from
the four programs and every plugin directory says **488,934,276 bytes of
`AppDir/lib` — 2,300 files, 39% — is not reachable by anything.** ⚠ The sweep
was a scratch script and **was not committed**; T-061 carries rewriting it in
Go, where requirement 6 (no hardcoded layout) applies to it.

### 3. T-058, T-050 and T-053 closed, with controls

- **T-058**: `pgb build` is concurrency-safe — a wrapper directory keyed on the
  options themselves. ⭐ `experiments/87-` carries the **control that
  reproduces the old defect 5 times out of 5**.
- **T-050**: `experiments/88-`, 25 assertions. The route needed an **index of
  what was built**, not a field somebody uploaded — `packages.json.br` plus
  hydra's `latest-finished`.
- **T-053**: patchelf and patsh, used or refused with the reason.

### 4. The defects, and the one that caused T-061

⛔ **A comment inside a double-quoted string is a command substitution.**
`internal/nixx/build.go` named a file as `` `.built` `` in a comment inside a
`_cmd="..."` assignment; the composing shell ran it and printed
`pgb: 1: .built: not found` as boost's round 1 began. An hour went into a boost
build that was never failing. ⭐ **That defect is the whole argument for
T-061**, and the scan that would have caught it is one line:

```sh
awk '/^ *_cmd=/,/;; *$/' tool/lib/nix.sh | grep -c '`'      # must be 0
```

⛔ **onelf was reported as unable to run our payload. It runs it.** The arm
invoked the bundle through a symlink named `melt-onelf`; onelf dispatches on
argv[0]'s basename, matched no entrypoint, and **silently ran the package
default** — kdenlive, which needs a display and died in `QMessageLogger::fatal`
inside `QApplicationPrivate::init`. Through a symlink named `melt` the same
bundle answers in 0.4 s. ⭐ **The control that settles it**: a 141 MB,
188-library onelf package of the same nixpkgs ffmpeg runs on this machine. ⚠
`experiments/90-` is fixed and **has not been re-run**.

Eleven more, each a class rather than a quirk, are in the entries and in
`docs/history/corrections.md`.

### 5. The documentation, and a second gate for it

⛔ **`docs/` went thirteen commits without an edit while five entries changed
state**, and the way that was noticed was the operator saying so.
⭐ **`scripts/common/check-docs.sh` is the fix**: dead links, backticked repo
paths, cited evidence, referenced experiment numbers, quoted entry counts, and
the vendored set's own unresolved-link list — all derived, none typed in. It
found six real defects on its first green run, including a **wrong job count**
in `docs/research/solo.md` (nine, not six) that no reading had caught.

⭐ **`gate.md` and `reviews.md` are vendored now.** `sessions.md` §Ending said
*"Run the gate. All three parts"* and named a file this tree did not have, for
a whole session.

## In progress

⚠ **One thing, and it is a background build that survives being ignored.**
T-060 rung 1: `sh /var/tmp/pgb-t060/rung1.sh` → `/var/tmp/pgb-t060/rung1e.log`,
31 of nix's external dependencies built into `/var/tmp/pgb-t060/prefix`,
idempotent on re-run. ⛔ **It is on `/var/tmp` and the machine is ephemeral**;
treat it as gone.

## ⭐ Work order

    T-061   ⛔ THE PORT. The only work of the next session. Everything below
            waits, including anything that would edit shell under tool/.
    ---- and then, in the session after ----
    T-055   the bundle size: 489 MB of lib/ is unreachable and that is the cut
    T-060   rungs 2 and 3, the static nix
    T-054   rungs 3 (KF6) and 4 (kdenlive static)
    T-057   item 2: a 32-bit application through the lib32 path
    T-051   the no-compiler host
    then P2 by category

## Open questions for the operator

⭐ **None blocking.**

1. ⭐ **No branch debt.** The harness named `claude/glibc-nix-static-v2nttp`;
   `RULES.md` §Git outranks it and every commit is on `main`.
   `git ls-remote --heads origin` lists `refs/heads/main` and nothing else.
2. ⚠ **A GPU** — **T-059**, not a question. Every GL row is `swrast`.
3. ⚠ **`docs/design/porting-report.md` is a session artefact.** The operator
   asked for it to be removed once it is not needed. ⛔ **T-061 deletes it**
   when its content has moved into `design/toolchain.md` — not before.
