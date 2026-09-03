# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log
and the entries.

    STATE     2026-09-03b
    COUNTS    48 entries, 15 open, 33 done
    BASELINE  pgb: 11/11 run, 11/11 no host object, TEN POCs
              CI: GREEN before this session; selftests 239 pass, 1 could not
              run (no zstd)
              throughput: glibc 4.53 ns/op vs musl 584.71 (malloc, 4 threads)
    NEW       ⭐ THE OPERATOR'S cross-libc-dlopen REVIEW, all four items:
              T-073 a live silent-wrong-answer defect in the loader's
              own-symbol table; T-074 a selftest that could not fail on the
              state it was written to catch; T-075 LD_DEBUG=bindings on the
              dynamic control; and the vendored reference re-mined past PR 30.
              ⭐ T-066 ROUTE A'S CEILING IS MEASURED — 218.5 MiB of 938.8 on a
              mesa bundle, against 6.3% the sweep can prove dead — and getting
              there found the sweep rooting versioned libraries at themselves.

## ⛔ READ THIS FIRST: the toolchain is Go now, and the shell is the oracle

⭐ **`pgb` is one statically linked Go binary.** The driver, the compiler
wrappers, the planner, the verifier and the bundler are the same executable,
built `CGO_ENABLED=0`, carrying the C runtime sources it compiles. The shell
and Python it replaced are under `HISTORY/<commit>/`, unedited, and are the
oracle every gate is measured against.

**Read [`toolchain.md`](toolchain.md) §T-061 for what was required, then
[`../docs/design/toolchain.md`](../docs/design/toolchain.md) "Language and
structure" for the decision, the architecture and the six gates.**

⛔ **The experiments and the POCs stay shell** and are the acceptance harness.
An entry below that proposes editing shell under `tool/` or `scripts/` is
describing files that no longer exist; check `HISTORY/` before acting on it.

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
| 1. the builder | T-050 ✅, T-051, T-060, T-012 | unchanged this session |
| 2. the bundler | T-057 ⚠, T-052 ✅, T-053 ✅, T-066, T-071 | unchanged this session; ⛔ **the size gap is structural** |
| 3. kdenlive | T-054, T-055 | unchanged this session. ⛔ **The bar is NOT met**: 2.22× the size |

⚠ **This session worked the top of the work order, which is glibc's remaining
quirks and future-proofing — not the three goals.** That is the ordering
`INDEX.md` argues for and it is deliberate.

## What this session did

⭐ **The operator supplied a review of `pkgforge-dev/cross-libc-dlopen#28` /
PR 30 with four items, in order. All four are done, and three of them turned up
a live defect rather than a paragraph.**

⛔ **WE ARE NOT AFFECTED BY UPSTREAM'S BUG and nothing here says we are.** It is
an `LD_PRELOAD` interposition defect in a forwarding shim: with no forwarding
target it kept 358 entry points as zero-returning stubs which, winning
interposition, shadowed the real `glGetString` the process could still serve.
This tool ships no preload shim and its output has no `PT_INTERP`, so
interposition cannot reach it — `docs/AGENTS.md` §14 already refuses that route.

⭐ **The DEFECT CLASS is ours, and it is the one this tree keeps paying for: a
lookup that ANSWERS when it should DEFER.**

### 1. ⭐ T-073 — the live instance, and the value hides it

`el_provider()` checked `el_own_syms[]` **first and unconditionally**. Its two
entries have **opposite** requirements and nothing distinguished them:

    __tls_get_addr             MUST WIN over every loaded object -- the module
                               ids are minted by this loader's own DTPMOD64 case
    _dl_mcount_wrapper_check   MUST YIELD to any real definition -- it is a
                               no-op stand-in and answering shadows the real one

⛔ **One table cannot express both, and it was safe by accident**: `el_resolve()`
calls `el_provider()` last, which is right for one name and wrong for the other.
Nothing asserted that ordering.

Two tables now, consulted at two points. `experiments/94-`: **pass=16 fail=0**,
and **11 of 11 on the bed** including all four musl rows.

| arm | fixed | reversal |
|---|---|---|
| a loaded object defines `__tls_get_addr` | ⭐ `decoy_calls=0` | ⛔ `decoy_calls=2` |

⛔ **THE VALUE IS `tls=0x5eeded` IN BOTH COLUMNS.** The decoy returns its own
slab to every caller, so a thread-local round trip through it is
self-consistent. **Only the call count separates them.** An experiment written
around the value — the obvious way to write it — would have measured nothing
and reported a pass.

⚠ **Reachable today only through objects refused for unrelated reasons.** Of
2,514 host shared objects, **four** define either name: `ld-linux`, `libasan`
and `libtsan` for `__tls_get_addr`, `libc.so.6` for the other. All four are
declined as interposers or answered as a served soname. That is why it had not
bitten; it is not a reason it was safe.

### 2. ⚠ The vendored reference predated the fix, and re-mining UNDID a recorded deletion

`references/pkgforge-dev__cross-libc-dlopen` was pinned at `1cecf50e`
(2026-09-01); PR 30 merged 2026-09-02. **T-031 proposes porting from that tree,
so a port would have inherited the bug.** Re-mined at `793f3f3f`, PR 30's merge
commit, and T-031 now carries the whole argument.

⛔ **And the re-mine put back the file `docs/AGENTS.md` §12 records as "one
deliberate deletion"**, overwriting the `PROVENANCE.md` section that recorded
its removal. The deletion had been a session's action; nothing in
`mine-repo.sh` knew it had ever happened.

⭐ **`trim_tree()` does it at fetch time now and writes the trim into
`PROVENANCE.md`**, so it survives the next re-mine. ⛔ **Sweeping all 34
references found TWO MORE that had never been noticed** —
`grigio__docker-nixuser/tree/AGENTS.md` and `yasunori0418__nput/tree/CLAUDE.md`.
`check-docs.sh` gate 7 asserts none is vendored, asking the repository rather
than the disk, and it fired on both before the deletions were staged.

### 3. ⭐ T-074 — the host-policy selftest could not fail on the state it was written to catch

Five "is unset" assertions read the VALUE, and the helper returned `""` for a
key that is **absent** and for one emitted as `KEY=`. For
`__EGL_VENDOR_LIBRARY_FILENAMES` those are the safe state and the dangerous one:
`getenv` returns a non-NULL empty string, libglvnd takes the branch, returns,
and the bundle has **no EGL at all**. The assertion labelled *"unset, not
set-and-empty"* was green on both.

⭐ **The product was always correct** and `hostpolicy.go` is byte-identical.
One planted defect through both instruments:

    old, by value      ok   ... the bundle stops pinning the EGL vendor list =
    new, by presence   FAIL ... RELEASED, not emptied = yes, wanted no

⚠ **And `experiments/85-` never sets `PGB_HOST_MESA` at all**, so the release
path's only assertion was the blind one.

### 4. ⭐ T-075 — LD_DEBUG=bindings, stolen, and placed where it works

Upstream settled *which object won a symbol* in one command; this tree spent
four probe builds and a SIGSEGV handler on the same question. ⛔ **It cannot be
asked of the subject** — `LD_DEBUG` is read by glibc's dynamic loader and our
output has no `PT_INTERP` — **but every control run against glibc's own `ld.so`
is dynamic.** `experiments/93-`'s control captures it for the rows where the two
loaders disagree.

⛔ **It found a defect in itself before it was committed**: `LD_DEBUG_OUTPUT`
appends `.<pid>` and `timeout` forks, so two logs appear. On `libz.so.1`,
`.22171` had 186 lines and **not one mention of libz**; glob order yields it
first and the first version took it. Chosen by content now.

### 5. ⭐ T-066, the last open P0 — route A's ceiling is MEASURED, and getting there found the sweep rooting libraries at themselves

⭐ **`pgb bundle sweep --cut FROM=>TO`** treats one `DT_NEEDED` edge as absent
and reports what becomes unreachable without it. ⛔ **The entry said this was
cheap — "the AppDir and `pgb bundle sweep`, no rebuild". That was wrong**, and
three things had to be fixed before a cut moved a single byte:

1. ⛔ **A versioned library was a ROOT OF ITSELF.** The soname scans excluded
   the object's own **filename**, but `libunistring.so.5.2.1` carries
   `DT_SONAME libunistring.so.5` and the index holds that name. The two strings
   are never equal, so **nearly every ordinary shared library** was its own
   root and `DropUnreachable` could not drop it. jq AppDir: roots **40 → 28**.
   ⚠ The selftest had a self-naming case and it passed throughout, because its
   fixture used a file whose name equals its SONAME.
2. The cut has to reach the soname **string** scan — a `DT_NEEDED` entry IS a
   string in `.dynstr`, and a rebuild removes both.
3. The cut is by target **file**: cutting `libunistring.so.5` left
   `libunistring.so`, a substring of the same string, to keep it a root.

⭐ **THE CEILING, on `mesa-demos` at `--debloat none`, 806 files / 938.8 MiB:**

    baseline                     61,882,072 B (59.0 MiB, 6.3%)
    --cut '*=>libLLVM.so.21.1'  250,081,656 B      delta 188,199,584 B
    + the three icu names       291,032,720 B      delta  40,951,064 B
    ⭐ CEILING                    229,150,648 B = 218.5 MiB = 23.3%

⛔ **The sweep can prove 6.3% of this bundle dead; two rebuild recipes are worth
23.3%** — nearly four times what deletion reaches, from two of the corpus's 24.
That is the entry's "subtractive cannot win" argument measured on **our own
bundle** rather than read out of somebody's build script.

⚠ **And the entry's `jq` headline was stale.** `experiments/78-`'s committed RESULT predated
the commit gating `DropUnreachable` to `aggressive`, so its `safe` and
`aggressive` are the same number. Re-run against the field's 4,006,916 B:
`none` 3.06×, `safe` 1.83×, ⭐ `aggressive` **1.58×** — not the 1.22× quoted,
which described a build where `safe` also swept.

### 6. ⛔ Findings, and again not one came from reading code

1. **`main` came up 214 commits behind** on a shallow clone.
2. ⛔ **A re-mine silently undid a recorded deliberate deletion**, and two more
   third-party agent-instruction files had been sitting in `references/`
   unnoticed the whole time.
3. ⛔ **An assertion whose own label named the distinction could not make it** —
   T-074, found by asking what its helper returns for an absent key.
4. ⛔ **A versioned library was a root of itself**, found because a `--cut` that
   reported hitting an edge moved zero bytes.
5. ⛔ **A committed evidence file described a build configuration that no longer
   exists**, found by `git merge-base --is-ancestor` against the commit that
   changed the gating.
6. ⚠ **`LD_DEBUG_OUTPUT` writes more than one file** and glob order picks the
   useless one.
7. ⚠ **`pgb bundle appimage --out` threw away a whole build** for a missing
   output directory, and reported it as `mkdwarfs failed` over a log saying it
   wrote 0 files.
8. ⛔ **`$?` after a pipeline is the pipeline's status** — paid again this
   session: `make 2>&1 | tail -2 && echo BUILD_OK` printed BUILD_OK over a
   failed build.


## ⛔ FOUR DEEP REVIEWS, RUN AT THE OPERATOR'S INSTRUCTION

⚠ **These were run AFTER the session record above was written, and two of them
found things that change it.** The order is the four lenses this tree uses:
does the claim hold when the command is run; what did the change stop
measuring; what was deferred; and is the code right.

### Review 1 — does every claim hold when the command is run?

⭐ **Re-run, not re-read.** Every load-bearing number published this session was
re-derived from the command:

| claim | re-run | verdict |
|---|---|---|
| T-066 ceiling **229,150,648 B = 218.5 MiB** | three sweeps of the mesa AppDir | ⭐ **reproduces exactly** |
| `pgb selftest` **371 cases** | `./pgb selftest` | ⭐ 371 pass, 1 could not run |
| `experiments/76-` `pass=7 fail=0` | re-run after the wrapper change | ⭐ still 7/0 |
| `experiments/93-` `ok=882 crash=45` | re-run after the loader change | ⭐ identical in every column |
| ⛔ *"`_dl_mcount_wrapper_check` is not in `libc.a`"* — quoted from a COMMENT | `nm --defined-only libc.a` | ⭐ **0**, and **0** in the generated provider table. The claim was true; it had not been measured by me |

### Review 2 — what did these changes stop measuring?

⛔ **THE REAL GAP, AND IT WAS OPEN WHEN THE SESSION RECORD WAS WRITTEN.** The
`selfKeys` fix makes `DropUnreachable` delete MORE — versioned libraries it
previously could never drop. ⚠ **`experiments/89-`, whose entire stated purpose
is *"debloating, and the control that says nothing was lost"*, had not been
run.** It has now:

    pass=10 fail=0 skip=0
    ok  debloated arm AGREES with the undebloated one          = 11
    ok  the AGGRESSIVE arm agrees too, on this OpenGL subject   = 11
    ok  debloated arm loaded no host shared object             = 11

⭐ **The control holds.** ⚠ And 89-'s own caveat now covers more ground than it
did: the aggressive arm agrees **on an OpenGL subject**, and says nothing about
a Vulkan application. My change widened what aggressive deletes, so that
caveat is worth more than it was.

⚠ **AND A CONTROL WAS WEAKENED, WHICH NOTHING WILL REPORT.**
`sonamesMentionedNaive` is the oracle the fast soname scan is compared against.
I applied the `selfKeys` fix to **both**, so they still agree exactly — but they
now **share** the helper, and the equivalence assertion can no longer catch a
defect inside `selfKeys` itself. ⛔ That is a real reduction in what
`bundle-soname-scan` proves, and it is recorded here rather than left to be
discovered.

⛔ **AND THE POCs WERE NOT RUN.** The wrapper's link path changed — every
`pgb build` link now scans its `.a`/`.o` inputs — and the ten POCs are this
project's acceptance harness. `experiments/76-` covers the loader; **nothing
covers the wrapper change against a real project build.** Five link shapes were
checked by hand (C+C++ archive, that plus an explicit `-lstdc++`, a bare C++
`.o`, a `c++` driver link, a plain C link) and `libc.a` — the biggest archive in
the toolchain — correctly reports no C++ demand. ⚠ **That is not the same as
ten real projects on eleven environments.**

### Review 3 — what was deferred, named plainly

1. ⛔ **The POC suite, after a change to the link hot path.** Above.
2. ⚠ **T-075's other two placements.** `LD_DEBUG=bindings` went into 93-'s
   control; `poc/10-gawk` and `experiments/62-` were left, each needing one
   measurement first. The operator's item 4 named three places and one was done.
3. ⚠ **T-066's fixpoint lever.** The soname scan counts mentions from EVERY
   object including unreachable ones, so an unreachable `libicui18n` keeps
   `libicuuc` alive. Named, not taken.
4. ⚠ **T-063 arm S was never re-run** with `--without-icu` removed, so the C++
   fix is proved on a synthetic subject and not on postgres.
5. ⚠ **kdenlive untouched**, and the same-day `safe` vs `aggressive` timing
   comparison is still owed.

### Review 4 — is the code right, and what does the record say about it?

⭐ **The link change was checked in five shapes** and the loader change in three
experiments. ⛔ **But the review found a record defect on the most load-bearing
page in the tree:**

⛔ **`T-033` IS STILL `open` AND IT DESCRIBES WORK THAT IS DONE.** T-033 is
*"route D: compile an ELF loader in, resolve against our own static glibc"* —
which is exactly what **T-064** did and closed, 11 of 11, and what
`docs/AGENTS.md` §13 records as closed. Two entries, one route, one of them
stale.

⛔ **AND `docs/REQUIREMENTS.md` — the operator's binding acceptance bar —
points its ONE remaining unmet issue at T-033**, in a sentence that does not
mention that `pgb build --host-dlopen` exists, ships, and is measured on eleven
environments. A reader of the bar cannot tell that the mechanism was built.

⚠ **The row is NOT closed and must not be marked so** — REQUIREMENTS' own text
forbids softening it, and the measurement does not support closing it: a host
`.so` loads on **7 of 7 glibc rows** and is **refused on all four musl rows**,
and **882 of 1,527** host objects load on the build host. The issue *"dlopen of
a host `.so` is host-dependent"* is **substantially served and still
host-dependent**. ⭐ What is wrong is the POINTER and the omission, and fixing
those is a status move backed by a measurement, which is the one edit that page
permits.

## ⭐ Work order — ⛔ REPRIORITISED 2026-09-03 BY THE FOUR REVIEWS

⛔ **THE ORDERING CHANGED, AND A MEASUREMENT CHANGED IT.** The previous order
put *"build the allowlist"* at the top of T-066. ⭐ **Route A's ceiling is now
measured at 23.3% of a bundle's library tree, and the gap to close is 2.2×.**
An allowlist cannot get there — so the next T-066 step is not the allowlist, it
is **costing route B**, which is where the size actually is.

    ---- 0. ⛔ THE DEBT THIS SESSION TOOK ON. Do these FIRST. ----

    R1  ✅ DONE 2026-09-03c. ⭐ TEN OF TEN, rc=0, fail=0 skip=0 on every one,
        167 assertions, 55 minutes wall (03:52Z -> 04:48Z), PGB_ENGINE=chroot
        to match the engine every committed RESULT.txt names.
          10 13  20 13  30 13  40 13  50 13  60 13
          70 20  80 21  90 21  91 27
        ⭐ The debt is cleared: the wrapper's link hot path change
        (elfx.NeedsCXXRuntime scanning every link's .a/.o inputs) is now
        validated by the acceptance harness on ten real projects across
        eleven environments, not by five link shapes checked by hand.
        ⚠ Run against the pgb built at session start -- i.e. BEFORE this
        session's own buildx/sweep changes, which is what R1 was for.
        ⛔ AND CHECKING THE RESULT FOUND A GAP IN THE HARNESS ITSELF.
        `poc_inspect` carries the ONLY assertion that can tell a binary built
        by the named environment from one built by the incumbent -- the
        .comment gcc check that caught T-070 arm 5. THREE of the ten POCs
        never call poc_inspect: 70-sqlite-extensions, 80-mlt and 91-qt-xcb
        each inspect by hand instead. So three of the ten reported green
        having never compared their binary's compiler to the environment's.
        ⭐ Split out as `poc_check_built_by_env` and called from all three.
        Verified on all four branches: right gcc -> ok, wrong env gcc -> FAIL,
        no .comment -> SKIP, no recorded env gcc -> SKIP.
        ⚠ The three binaries WERE correct (all read 14.2.0, checked by hand);
        the gap was that nothing asserted it.
        ⛔ RE-RUN OWED: those three RESULT.txt files describe runs without the
        new assertion. Re-run 70, 80 and 91 once the bed is free.
    R2  ✅ DONE 2026-09-03c, AND IT FOUND A SECOND ROOT-OF-ITSELF.
        `sonamesMentionedNaive` computes the self-set itself now, with
        `os.SameFile` (device+inode) against `selfKeys`'s path-string
        resolution — two instruments, no shared code. ⛔ They disagreed on the
        first new case: a HARDLINKED SONAME. `EvalSymlinks` cannot see through
        a hardlink, so one inode under two names landed in two groups and the
        library became a root of itself. `selfKeys` keys on `dev:ino` now.
        Three cases pin `selfKeys()` directly as well. T-066.
    R3  ⭐ DONE 2026-09-03c, AND IT PAID FOR ITSELF: the C++-archive fix DID
        NOT REACH THE REAL SUBJECT, and only postgres could say so.
        `--with-icu` survived all 14 adaptation rounds -- configure accepts it
        -- and then the LINK died on `operator delete` and
        `__cxxabiv1::__si_class_type_info` out of libicuuc.a.
        ⛔ CAUSE: `cxxRuntimeDemand` skipped every argument beginning with
        `-`, so it only ever saw archives named as LITERAL PATHS. postgres's
        own Makefile.global says
          ICU_LIBS = -L/…/lib -licui18n -licuuc -licudata -lpthread -lm
        and every one of those starts with `-`. ⭐ That is exactly why the fix
        passed a synthetic subject: its fixture is `cc -o prog main.c
        libcxxthing.a`, a literal path.
        ⛔ AND THE SELFTEST HAD ENCODED THE DEFECT AS THE INTENT -- one case
        read "cxx-demand: a flag is never opened as an input", and it passed
        because every path in that block is deliberately non-existent, so
        "considered" and "skipped" give the same answer.
        ⭐ FIXED: -lNAME/-l:NAME resolve against -L, .a only, system dirs not
        searched. BEFORE rc=1 with 2 C++ undefined refs; AFTER rc=0, the
        binary runs, PT_INTERP 0, DT_NEEDED 0. `cxxCandidates` is split out so
        what the scan WOULD open is assertable without a filesystem; ten cases
        pin the rule.
        ⭐ AND THE RE-RUN WITH THE FIX: ICU link errors 0, the backend BUILT,
        and the build advanced to the failure T-063 already records.
          src/backend/postgres   101,647,216 B  (was 63,889,168 WITHOUT icu)
          PT_INTERP 0, DT_NEEDED 0, ⭐ 3,911 icu_78 symbols in the image
          ./postgres --version                   -> PostgreSQL 18.6
          on alpine-3.22 (no glibc at all)       -> PostgreSQL 18.6
        ⚠ It now stops in src/interfaces/libpq, and on a DIFFERENT kind of
        problem: `libpq must not be calling any function which invokes exit`
        is postgres's OWN policy check on the SHARED libpq, which a static
        build has no use for. The next rung is "stop building the shared
        client library", not "make it link".
        ⛔ THE LINK HOT PATH CHANGED AGAIN, so the ten POCs must be re-run --
        and ⛔ A PLAIN RE-RUN WOULD HAVE PROVED NOTHING: every POC skips its
        build when the artefact exists and only five of ten honour
        POC_REBUILD. ⭐ `poc/run-all.sh --rebuild` now exists and is the
        command; it deletes the shared work tree so all ten build against the
        toolchain as it is NOW.

    ---- 1. T-066 P0, and the route order is now ARGUED rather than assumed ----

    ⭐ The ceiling says an allowlist tops out around a quarter of the tree:

        the sweep can PROVE dead                      6.3%
        two `-mini` rebuild edges are worth          23.3%   (218.5 MiB)
        the gap to the field on kdenlive             2.22x

    B1  ✅ DONE 2026-09-03c — `experiments/95-`, pass=3 fail=0. ⭐ AND IT
        OVERTURNS THE ARGUMENT AGAINST ROUTE B. kdenlive's closure is 676
        paths; the whole `--add-common` -mini set forces 161 of them (23.8%)
        from source and leaves 515 in the binary cache. qtbase alone is 78
        (11.5%), not "the entire KDE/Qt subtree". ⭐ And qtbase is FREE once
        mesa is paid for: `downstream of qtbase but NOT of mesa = 0`.
        ⚠ A FLOOR (runtime references, not build inputs) and NOT costed in
        wall clock, which is the number that decides whether the bundle stays
        one command. gtk3 and glycin are NOT IN this closure.
    B1b ⛔ NEXT: cost route B in WALL CLOCK. One of the 161 is qtbase and Qt
        does not build in a minute. That needs a rebuild.
    B2  ⛔ THE ALLOWLIST -- AND 2026-09-03c SAYS THE **PATH**-LEVEL FORM OF IT
        IS THE WRONG GRANULARITY. Two cheap measurements:
          jq       0 of 7 store paths are entirely unreachable. SIX of seven
                   are 100% kept and ONE (glibc) carries every deletable
                   byte -- 269 objects, 8,990,808 B, all of it INSIDE one
                   path. A path-level allowlist saves nothing here.
          kdenlive the -dev outputs are 80 of 676 paths (11.8%) but only
                   70,581,624 of 2,941,485,288 B (2.4%). No -debug/-doc/-man
                   in the closure at all. ⚠ And they are in the RUNTIME
                   closure, so something references them -- that is T-053's
                   wrapper-script problem, not dead weight.
        ⭐ So the lever that works at this granularity is the FILE-level sweep,
        which exists. A path-level allowlist should not be built on the
        strength of the ceiling number alone.
    B3  ⭐ TAKEN 2026-09-03c AS A MEASURING DEVICE: `pgb bundle sweep
        --fixpoint`. Worth +7 files / +978,576 B on the jq AppDir (262 ->
        269 unreachable, 45.8% -> 51.4%), 3 rounds, scan set 283 -> 14.
        ⭐ The baseline row is BYTE-IDENTICAL to the one already recorded, so
        the loop is a no-op when off. The seven are six glibc CJK gconv
        HELPER libraries plus libresolv.so.2, held up by gconv modules the
        baseline already drops.
        ⛔ NOT WIRED INTO DEBLOAT and 89- has NOT been run against it. The
        seven names sharpen the safety question: a bundle converting a CJK
        encoding reaches those helpers through iconv_open, which leaves no
        DT_NEEDED and no mention -- the libSDL3 shape.
    B3b ⛔ NEXT: the control. ⚠ AND IT IS NOT ONE COMMAND -- read this before
        planning it, because "run 89- against --fixpoint" understates it the
        way the entry once understated route A's ceiling:
          1. `--fixpoint` is deliberately NOT wired into `bundle appimage`,
             so 89- CANNOT take it as an arm today. A fourth arm needs the
             lever plumbed into the debloat path behind its own flag.
          2. 89- builds THREE mesa-demos bundles. The AppDir at `--debloat
             none` alone was 1.2 GB last session and each arm carries a
             ~400 MB closure (N and A are hard-linked, S is not). Budget
             several GB and ~10 min a bundle. ⛔ DISK IS BINDING here.
          3. 89-'s assertion is an EGL one -- same vendor and driver on all
             eleven. ⚠ The seven files the fixpoint drops on jq are gconv
             helpers and libresolv, which an eglinfo subject never touches,
             so 89- passing would NOT clear the lever for a program that
             converts a CJK encoding. That needs a subject that does.
        ⭐ BUT THE BAR IS LOWER THAN IT LOOKS, and it is measured: the
        fixpoint can only drop a library whose every supporter is itself
        already dropped. Checked on the jq bundle by real file -- 0 reachable
        objects mention any of the seven. libresolv.so.2's only supporters are
        libnss_dns.so.2 and libnss_hesiod.so.2, WHICH THE BASELINE ALREADY
        DROPS. So the lever removes an incoherence (deleting the consumer and
        keeping the library) rather than taking a new risk, and what 89- has
        to clear is the baseline's judgement, which it already covers.
    B4  ⚠ A kdenlive AppDir does not exist. B1 needs only the closure.

    ---- 2. then, by how foundational they are ----

    T-062   ✅ DONE 2026-09-03c. buildx, logx and proc are covered and
            NOTHING IS LEFT: 375 -> 506 cases. Each was proved able to fail by
            planting a defect -- and buildx's planted defect is ⭐ T-019
            ITSELF (drop c.ContainerEnvArgs()), which goes red naming all 18
            option variables. logx's Quote is checked THROUGH A REAL SHELL,
            29 hard arguments round-tripped, because "paste this into a
            terminal" is its whole contract. proc's classify is measured on
            real children: kill -9 -> 137, signal 9.
            ⚠ Needed a small product refactor -- ChrootBinds() and
            ContainerRunArgv() split out of the runners so the argv can be
            asserted without a bed. ✅ REAL-BUILD VALIDATION DONE: ./pgb
            rebuilt with the refactor, `PGB_ENGINE=chroot sh poc/40-jq/run.sh`
            -> pass=13 fail=0, and the .comment gcc assertion confirms it
            built in the right environment. ⚠ CI does NOT cover this path: it
            uses `--engine host`, which goes straight to Inner() and never
            reaches ChrootBinds or ContainerRunArgv.
    T-075   ✅ DONE 2026-09-03c. `experiments/96-`, pass=14 fail=0. BOTH rows
            measured by one experiment, and the answer to both is NO.
            ⭐ Same source, same compiler, one -static and one not, both
            dlopening the same host object:
              LD_DEBUG=bindings   dynamic 143 lines   static 0
              LD_DEBUG=all        dynamic 485 lines   static 0
              LD_DEBUG=help       dynamic exits       static RUNS TO COMPLETION
            ⛔ And ld.so IS in the static process -- strace shows it opening
            ld-linux-x86-64.so.2. The loader is present and reads the variable
            never, because LD_DEBUG is parsed during ld.so's OWN startup.
            ⛔ So it must not go into poc/10-gawk: it would not fail, it would
            produce an EMPTY CAPTURE that reads as "no bindings". And it
            settles 62- too -- the instrument can describe every competitor
            arm (they run a bundled ld-linux) and not ours.
    T-057   a 32-bit application through the lib32 path.
            ⚠ CORRECTED 2026-09-03c: the entry said "No 32-bit path. lib32
            exists in the Anylinux layout and not here". ⭐ THAT IS STALE --
            assemble.go reads EI_CLASS, routes 32-bit objects to lib32, copies
            a 32-bit loader, warns by name when there is none, and makes the
            shared/lib32 symlink. ⛔ What is missing is the MEASUREMENT: no
            32-bit application has been put through it. ⭐ And elfClass -- the
            decision that keeps an i386 libfoo.so.1 from shadowing the x86_64
            one -- had NO carried coverage; seven hermetic cases now pin it.
    T-060   rungs 2 and 3, the static nix
    T-054   rungs 3 (KF6) and 4 (kdenlive static)
    T-051   the no-compiler host
    T-012   pgb build <url-or-package>
    then    P2 by category

⭐ **Two pieces of real work are NAMED and are not entries**, because each is
one clear fix inside T-063 arm S:

    the static link-order problem   ⚠ CORRECTED 2026-09-03 BY MEASURING IT.
                                    -Wl,--start-group fixes ORDER (arm B,
                                    rc=0) and CANNOT fix ABSENCE (arm C,
                                    rc=1) -- and AC_SEARCH_LIBS -lreadline is
                                    absence: libncursesw.a is never on the
                                    probe's line. ⭐ The real fix is the SAME
                                    SHAPE as the C++ one landed this session:
                                    read the archives' undefined symbols and
                                    append what defines them. Not built.
    a C link that pulled a C++      ✅ FIXED 2026-09-03. elfx.NeedsCXXRuntime
    archive                         reads the link line's archives for an
                                    UNDEFINED operator new/delete or an
                                    __cxxabiv1 vtable and the wrapper appends
                                    -lstdc++ -lm after the link flags. Fails
                                    before, passes after; carried as
                                    `cxx-runtime` with a negative control.
                                    ⚠ Proved on a synthetic subject -- R3.

## Open questions for the operator

⭐ **None blocking.**

0. ⛔ **NEW 2026-09-03c, AND IT IS A PRODUCT DEFECT RATHER THAN A QUESTION:
   `pgb build -- cc ...` BYPASSES THE WRAPPERS.** Found by walking into it
   while building a fixture.

   `buildx.Inner` puts the wrapper directory on `PATH` in the child's `Env`,
   then runs `proc.Cmd{Argv: argv, Env: env}` → `exec.Command(argv[0], …)`.
   ⛔ **`exec.Command` resolves the program against the PARENT process's PATH,
   not against the `Env` set on the Cmd**, and the inner pgb's own PATH does
   not contain the wrapper directory. Measured directly rather than reasoned:

       parent PATH = .../real          child Env PATH = .../wrap:.../real
       exec.Command("mytool")  ->  resolved Path=".../real/mytool"
                                   output="I AM THE REAL ONE"

   ⚠ **It does not bite the POCs or `pgb build -- make`**, which is why it has
   survived. `poc_in_env` runs `build -- /bin/sh -c "…"`: an ABSOLUTE path, so
   no lookup happens, and the shell then resolves `cc` using the environment it
   was handed — which does have the wrapper directory. `make` resolves to the
   real make, which is correct. ⛔ **It bites exactly when the command IS a
   wrapped tool** — `cc`, `gcc`, `c++`, `g++`, `cpp` — which is a documented
   use of `pgb build [--] CMD...`.

   ⚠ **What is NOT established** is what the bypassed build produces. The
   symptom seen was `cc: fatal error: cannot execute 'cc1'` with a malformed
   `-iprefix /../lib/gcc/…`, so on this machine it FAILS rather than silently
   producing a non-portable binary — which is the better of the two outcomes
   and must not be assumed to hold elsewhere.

   ✅ **FIXED 2026-09-03c.** `proc.Cmd.build()` resolves `argv[0]` against the
   PATH in the environment the CHILD will run with (`lookPathIn`), and only
   when the caller set one and the name has no separator — the two conditions
   under which the answer can differ. A failure to resolve is left to exec, so
   the error stays the one it would have given.

   ⭐ **The end-to-end case IS the defect**: two programs of the same name, the
   parent's PATH naming one and the child's `Env` naming the other, asserting
   which ran. Plus `lookPathIn` pinned directly — first directory wins, order
   decides, no PATH resolves nothing, a name with a separator is not looked up,
   a non-executable file is skipped in favour of the next directory. 8 cases.

       reversal (the resolution removed)
         FAIL  a Cmd runs the program its own Env's PATH names = REAL, wanted WRAPPER

   ⛔ **POC VALIDATION OWED**: this changes `proc`, which every child process
   goes through. The suite running when it landed was validating the `-l`/`-L`
   fix against a pgb without it, so the suite must run once more with both.

1. ✅ **RESOLVED 2026-09-03.** The two harness-named branches this asked about
   are gone: `git ls-remote --heads origin` returns **`main` and nothing else**.
   ⚠ The harness named `claude/cross-libc-dlopen-review-ukfukq` this session
   and it was never pushed; the local copy is deleted with `git branch -d`,
   which confirmed it was fully merged. `RULES.md` §Git outranks the harness
   and the operator said the same.
2. ⚠ **A GPU** — **T-059**, not a question. Every GL row is `swrast`, and
   T-071 item 4's second half cannot be settled without one.
3. ⚠ **Docker Hub rate-limits anonymous pulls in this environment.**
   ⭐ `pgb rootfs pull` does the anonymous-token dance and succeeds where
   `docker pull` 429s; ⭐ `docker buildx imagetools inspect` reads metadata
   without pulling and answered for every tag this session.
4. ⚠ **`musl-gcc` is absent**, which is the one remaining blocker on
   `experiments/90-`'s arm O.
5. ⚠ **`--tls-reserve` now costs ~1.15 MB on EVERY `--host-dlopen` build**,
   because the iconv wrapper has to be forced out of its archive whether or not
   the program calls iconv. Measured: 4,575,160 → 5,725,864 bytes. A narrower
   fix would need the provider table to reference the wrapper strongly for
   those three names only; it is not obviously worth the machinery.
