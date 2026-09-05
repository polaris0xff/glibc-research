# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log,
the entries, and [`../HISTORY/`](../HISTORY/).

    STATE     2026-09-04c, at the END. ⭐ The capability corpus is COMPLETE
              (26 of 26) and all three of its unexplained rows are explained
              — the last, `qalculate-qt`, by `experiments/107-`: it spawns
              GNUPLOT through the host's /bin/sh, so the residue is a shell
              the application asked for and the bundle never carried (C55).
              ⛔ THE SESSION'S REAL YIELD WAS INSTRUMENT DEFECTS. Five
              separate criteria in this tree COULD NOT FIRE and no gate could
              see any of them, because a SKIP is not a failure and a ZERO
              looks like a result: C48, C50, C52, C54, C56.
              ⭐ Two of the corrections can move a committed number (C49, C54)
              and ONE re-run of `65-` clears both. ⭐ One (C56) turned a claim
              that had never been measured — `--wrap=iconv` — into a
              measurement: 1 encoding of 12 and a crash, against 12 of 12
              with a byte-exact round trip on all eleven.
    COUNTS    69 entries, 25 open, 44 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, TEN POCs
              CI: green through the session (read after every push)
              throughput: glibc 8.40 ns/op vs musl 704.79 (malloc, 4 threads)
    NEW       ⭐ FOUR BUNDLER DEFECTS FOUND AND FIXED, each by running a
              recorded zero down instead of believing it:
                C41 the interposer answered `open` and not `stat64` — which
                    is the whole of Python. virt-manager 0/11 → 11/11.
                C42 an FHS symlink farm's dangling loader link aborted the
                    build. gearlever UNRESOLVED → builds.
                C43 a wrapper target that is an ABSOLUTE SYMLINK into
                    another store path. helix 0/11 → 11/11, and it had been
                    failing SILENTLY: exit 255, not one byte of output.
                +   a nixpkgs SHELL FRAGMENT lifted into `.env` as if it
                    were a value, shadowing the real plugin path.
              ⭐ T-084 step 2: the six hand copies of the trace classifier
              are GONE; `102-` rewritten to read them back out of git so the
              before/after outlives them.
              ⭐ T-091 MEASURED: a bundled GStreamer pipeline runs on 11/11
              with ZERO host objects in payload AND tree, and
              gst-plugin-scanner is exec'd on 11/11.
              ⛔ THREE CORRECTIONS TO OUR OWN INSTRUMENTS: C39 (the
              assertion), C40 (the row note), C44 (a silent `cp`).
              ⭐⭐ AND FIVE MORE FROM THE DEEP-REVIEW PASS, of which C49 and
              C53 are the two that could have moved a committed number:
                C49 ⛔ "host" was a PREFIX LIST and `bundled` was its
                    complement, so a host object outside /lib and /usr/lib
                    read CLEAN. Measured across all eleven: 13 such files,
                    and one of them is `/usr/bin/ld.so` — THE HOST LOADER,
                    on arch, fedora-42 and both Debians. Runs in the
                    DANGEROUS direction (dirty → clean), unlike C25.
                C50 `102-`'s R1 looked for a trace only in the directory
                    `65-` deletes traces from, so it could never fire — and
                    a SKIP is not a failure, so both gates stayed green over
                    a check that did not exist. It fires now: 20/0/0.
                C51 `T .mo = 43` is 43 catalogue FILES read, not 43
                    translations. The CONTROL is what makes it a result:
                    258 lookups against 0, from the same artefact.
                C52 `101-`'s L2 demanded a syscall its own mechanism
                    prevents — the third criterion that could not fire.
                C53 ⛔ `pgb-storefix.c` had a LOADED AND INERT state that
                    looked exactly like working: no AppDir or no `.storemap`
                    meant it rewrote nothing and said NOTHING. One line,
                    once, unconditionally — verified four ways.
              ⭐ THE `-static` ROW IS ANSWERED, from evidence already in the
              tree: a fully static closure carries no loader, so the bundler
              REFUSES it and there is no artefact to ask the question of —
              and the refusal is correct (arm L: the raw binary, no bundle,
              11/11 with 0 host objects).
              ⭐ RUNG 3'S LOCALE CRITERION FIRES AT LAST — 11/11 — after two
              recorded causes that were both wrong (C48).

## ⛔ READ THIS FIRST

⭐ **`pgb` is one statically linked Go binary.** The driver, the compiler
wrappers, the planner, the verifier and the bundler are the same executable,
built `CGO_ENABLED=0`, carrying the C runtime sources it compiles. The shell
and Python it replaced are under [`../HISTORY/`](../HISTORY/), unedited, and
are the oracle every gate is measured against.

⭐ **Every millisecond in this tree goes through
[`../experiments/clock.sh`](../experiments/clock.sh)** — median of N, arms
interleaved with a rotating start, and an **A/A control** whose ratio is the
floor below which no row may be believed.

⛔ **AND A GUI CLAIM NEEDS A DISPLAY, NOT A LOG LINE.** A bundle once scored
11 of 11 green on `Gtk-WARNING: cannot open display`, which real hardware WITH
a display prints too; a real display turned those 11 rows into 0.
`experiments/64-` and `65-` start `Xvfb`, bind its socket into each rootfs, and
ask the **X server** whether a window exists. ⛔ A program's own output is not
evidence that it drew something.

⭐ **AND THE ≥50×50 RULE IS NOT DECORATION.** `flameshot` puts a **3×3**
`Qt Selection Owner` window on the server and no toplevel at all. A crude "is
there any window" check counts it and scores the row green.

## ⭐ The operator's rulings in force

| when | ruling | where it is recorded |
|---|---|---|
| 2026-09-01b | *"replace with per part claim, also anylinux is a bundle, our primary goal is still a static glibc binary that has none of the issues"* | [`../docs/REQUIREMENTS.md`](../docs/REQUIREMENTS.md) |
| 2026-09-01b | nixpkgs IS the planner | [`../docs/design/nix-front-end.md`](../docs/design/nix-front-end.md) |
| 2026-09-02b | *"pgb bundle isn't good enough, it is bloated, slow and a complete failure"* | T-066 |
| 2026-09-03c | *"us having a bigger size than anylinux-appimages and onelf is acceptable as long as ours performs better and packaging is just one command"* | [`../docs/design/toolchain.md`](../docs/design/toolchain.md) |
| 2026-09-03d | *"Defer comparing speed/startup/performance with anylinux-appimages & onelf for now"* | this page |
| 2026-09-03e | *"you may take on T-081 as you go since it is required now to complete your own tasks"* | T-081, **closed** |
| 2026-09-03e | *"you may be measuring the wrong success criteria ... confirm it properly by feeding it a fake/emulated display"* | `experiments/64-`, and §"read this first" |
| ⭐ **2026-09-04** | *"All three apps open, run and work, use emulated/dummy stubs for hw gaps."* | `experiments/65-`, and the Prove bar in [`../docs/research/app-corpus.md`](../docs/research/app-corpus.md) |

## ⭐ The operator's three goals

> *"1. Make the 'universal' builder true via pgb + nix. 2. make the 'universal'
> bundler true via a modern, updated, maintained 'nixappimage' descendant ...
> and also solving the opengl problem ... 3. poc a kdenlive static (exhaust all
> resources), if impossible, pivot to kdenlive.nixappimage, but it must be
> smaller, load faster, run faster than pkgforge-dev/kdenlive-AppImage-Enhanced."*

| goal | entries | where it stands |
|---|---|---|
| 1. the builder | T-051, T-060, T-012 | ⛔ Blocked on **building nix's own closure static**, not on a tool |
| 2. the bundler | T-057, T-066, T-081 | ⭐ **PROVEN ACROSS SIX TOOLKIT CATEGORIES**, three subjects each, real windows on a real X server, zero host objects. The failures that remain are named, and four of them were bundler defects now fixed |
| 3. kdenlive | T-054, T-055 | ⭐ cold start 0.74×, host objects 0 of 11 against 4 of 11. ⛔ *smaller* not met; *run faster* unresolved. ⚠ `90-`'s host counts are owed a re-run — T-084 |

## ⭐ Work order

    ---- ⭐ WHAT THE NEXT SESSION SHOULD DO ----

    ⭐ T-080's CORPUS IS COMPLETE. 26 of 26 rows, and
       docs/research/bundle-capabilities.md §0 closes every one with a
       count out of eleven. ⛔ What is left is NOT "run the corpus". It is
       the four rows that are not eleven, three of which are not ours:

       pdfarranger 0/11  ⛔ THE ONE THAT IS OURS, AND IT IS A CLASS:
                         `/usr/local/share/pdfarranger/…`, asked of Python
                         at RUN TIME. pgb-storefix.c answers `/nix/store/…`
                         and nothing else, by construction. The same class
                         blocks a bundled dbus-daemon
                         (`/etc/dbus-1/session.conf`). flatimage's portable
                         root is the shape that serves it — docs/research/
                         app-corpus.md rung 3.
       flameshot   0/11  the SUBJECT: a tray application with no toplevel,
                         and no session bus in this bed.
       neovim      0/11  the CLOSURE: its own glibc 2.26.
       vkmark      0/11  the BED: no /dev/dri anywhere.

    ---- then, in order. ⛔ THE ORDER IS BY MECHANISM, NOT BY EASE ----

    T-084  ⭐ THE CONVERSION IS DONE — six hand copies deleted, every call
        site on lib.sh's exp_classify_trace, and 102- rewritten to read the
        copies back out of git at a pinned commit so the before/after
        outlives them (pass=20, two runs). ⛔ STEP 2 STILL OWES THE RE-RUN,
        and 102- arm S says it is exactly ONE experiment: 90-, whose
        committed host counts describe only the SECOND of two invocations.
        It needs the machine to itself.
    T-091  ⭐ MEASURED, experiments/103-, run 1: ENCODE on 11/11, ZERO host
        objects in the payload AND the tree, and gst-plugin-scanner exec'd
        on 11/11 — which answers "which process did you count". ⛔ TWO
        DEFECTS IN RUN 1, both fixed and both owed a second run: the decode
        leg used `wavenc` from gst-plugins-GOOD, which is not in the
        closure; and THE CONTROL WAS CONFOUNDED — the bundle's only GST_*
        variable came from a nixpkgs SHELL FRAGMENT lifted verbatim out of
        the wrapper, present in subject and control alike.
    T-088  ⭐ --with-program IS EXERCISED and works: `flameshot + 2 more`,
        and `./flameshot.AppImage dbus-daemon --version` answers
        `D-Bus Message Bus Daemon 1.16.2` — the helper's OWN identity, so a
        dispatch that ran the default would fail. ⛔ ONE environment, by
        hand. The eleven are owed.
    T-089  open: a static application that needs a compiled-in PATH
        (powershell). syncthing and lilipod are done.
    T-090  ⭐ the cause is isolated: the sandbox EPERM is CHROOT's, and
        pivot_root permits the call. ⛔ Whether the bed still isolates
        under pivot_root is unmeasured, so a browser row measures
        --no-sandbox.
    T-059  a real GPU. ⛔ Every GL and Vulkan row is still swrast.
    T-066  ⛔ still the only open P0, and its remaining column is SIZE,
        which the operator struck on 2026-09-03c and deferred on
        2026-09-03d.
    T-082  vendor + patch + drift detection. XL — start early, finish late.
    T-083  desktop integration; what is left is the managers themselves.

    ---- then the builder, by how foundational ----

    T-060  rungs 1→3, the static nix. THE TARGET: `nix-cli-static`.
    T-051  a published (musl) static nix serves "enough nix on a minimal host".
    T-012  items 2, 3 and 4. ⛔ ITEM 1, the git/URL route, is DEFERRED.

    ---- then kdenlive ----

    T-054  rung 3 then rung 4.    T-063  arm S: src/interfaces.

    ---- P2 by category ----

    T-013  T-015  T-021  T-031  T-041

## ⭐ Delivery rules — EIGHT, and rule 3 is suspended for T-087

⚠ **Five came from the session that wrote T-078/079/080. Rules 6, 7 and 8 were
each paid for by a discarded run.**

⛔ **RULE 3 IS SUSPENDED FOR T-087 ONLY**, by the operator, 2026-09-04:
*"Use cached/prebuilt fetches to make the install/build fast, and get rid of
the measure twice rule; we need to cover more cases for now, we can refine
later."* Every other rule holds everywhere, T-087 included.

    1. ⛔ PRE-REGISTER the expectation before the run, and COMMIT it
       before the run so the git log shows it was not written after.
       ⭐ 2026-09-04c: which of TWO Python zeros a fix would move was
       committed before the run that settled it, and BOTH halves held —
       py-3 moved 0/11 → 11/11, py-2 did not move. ⚠ The point is not
       that the guesses were right: until the row note was fixed the two
       rows were INDISTINGUISHABLE, so no prediction was possible at all.
    2. ⛔ A SKIP IS NOT A PASS. Read the skip count on every run.
    3. ⛔ TWO RUNS OR IT IS NOT A NUMBER.
    4. ⛔ AN ABSENCE IS NOT A ZERO. Say where you looked, AND where you
       could not.
    5. ⛔ VERIFY YOUR OWN WRITE-UP AGAINST THE SOURCE.
    6. ⛔ CHECK THAT YOUR SUCCESS CRITERION CAN FAIL FOR THE RIGHT
       REASON — ⭐ and CHECK IT AGAINST REAL OUTPUT. Three of the corpus's
       five zeros were the criterion rather than the subject (C34, C36,
       C39), and `experiments/65-` now interrogates its own assertion on
       the FIRST environment instead of scoring a subject zero eleven
       times. ⚠ It caught nothing on the completed corpus, which is the
       outcome that makes the remaining zeros readable.
    7. ⛔ AND CHECK THAT IT CAN FINISH.
    8. ⛔ CARRY A POSITIVE CONTROL, AND NEVER CARRY A CONSTANT ACROSS A
       CHANGE OF MODE. ⚠ 2026-09-04c adds the other half:
       ⛔ **A CONTROL THAT CANNOT BE TOLD FROM ITS SUBJECT IS NOT A
       CONTROL.** `experiments/103-` built one with `--no-plugin-env`, and
       the flag removed nothing that mattered because the variable in
       question came from somewhere the flag does not reach. The run
       printed "the variables are redundant" and that conclusion was not
       supported. ⭐ Assert the difference, do not assume it.

## ⛔ Open questions for the operator

⭐ **None blocking.**

1. ⚠ **A GPU** — **T-059**. Every GL and Vulkan row is `swrast`/lavapipe.
2. ⚠ **kdenlive's warm row is 3.45× against us and unexplained.** First
   candidate: at 565 MB it is over uruntime's 350 MB `MAX_EXTRACT_SELF_SIZE`,
   so it **extracts** where `jq` **mounts**.
3. ⚠ **Docker Hub rate-limits anonymous pulls here.** `pgb rootfs pull` does
   the anonymous-token dance and succeeds where `docker pull` 429s.
4. ⚠ **A session DBus does not exist in this bed**, and it is **not** a
   hardware gap, so the stub rule does not cover it. ⭐ The bundle can carry
   `dbus-daemon` itself (`--with-program`, measured) — but the daemon then
   reads `/etc/dbus-1/session.conf`, which is not a `/nix/store` path and
   which the interposer therefore does not rewrite. Same class as
   pdfarranger.
