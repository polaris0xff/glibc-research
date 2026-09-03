# PROGRESS.md

⛔ **Carries no history.** Rewritten every session. The history is the git log
and the entries.

    STATE     2026-09-03b
    COUNTS    48 entries, 16 open, 32 done
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


## ⭐ Work order

    ---- glibc's remaining quirks, and future-proofing ----

    T-070   ✅ CLOSED. TEN of ten POCs pass at 2.41 through the normal POC
            path, 73- and 21- re-run, CI green at debian:13 16 of 16.
    T-068   ✅ CLOSED. 93- green at pass=6 fail=0, and the control it passes
            read 10, then 1, then 0 as each defect was fixed.
    T-073   ✅ CLOSED. experiments/94- pass=16 fail=0, 11 of 11 on the bed,
            with the reversal planted and the exit code read unpiped.
    T-074   ✅ CLOSED. pgb selftest 312 -> 314 cases; the instrument itself is
            asserted and the product is byte-identical.
    T-075   ✅ CLOSED. LD_DEBUG=bindings on 93-'s control. ⚠ Two further
            placements stay open, each needing ONE measurement first:
            does LD_DEBUG print anything when ld.so arrives as a library
            (poc/10-gawk), and is the question worth asking in 62- where
            classify_trace already answers the load question.

    T-072   ✅ CLOSED. experiments/76- carries the pair now: a 56,248-byte
            INITIAL-EXEC module refused cleanly at reserve 0 and loaded at
            65536, 11 of 11, with the reserve arm traced separately for host
            objects (zero). Size cost 88 bytes -- the reserve is .tbss.
            ⛔ The premise stays DENTED and recorded: liblsan.so is refused as
            an interposer before TLS is considered, and zero of 71 PT_TLS
            objects on this host exhaust the surplus.

    ---- the bundle, and the one class that is all DATA ----

    T-071   ✅ CLOSED. experiments/85- RUN: pass=10 fail=0, and the
            data-coherence arm's negative control fired. Item 3 is T-066's,
            item 4's other half is T-059's.
    T-066   ⚠ P0, STILL OPEN, premise significantly advanced.
            ⭐ ROUTE A'S CEILING IS MEASURED: `pgb bundle sweep --cut` exists
            and 218.5 MiB of a 938.8 MiB mesa bundle -- 23.3% -- is reachable
            ONLY through edges two `-mini` recipes delete, against 6.3% the
            sweep can prove dead. Subtractive cannot win, now measured on our
            own bundle.
            ⛔ WHAT IS LEFT: build the allowlist (bounded by that ceiling),
            and cost route B. ⚠ The jq headline moved 1.22x -> 1.58x when a
            stale pre-gating evidence file was re-run; the kdenlive AppDir
            still does not exist.

    ---- then, unchanged in relative order ----

    T-063   the miniflux proof: arm S has a static postgres on Alpine;
            src/interfaces does not build
    T-062   ⭐ `verifyx` (29 cases) and `fail` (16) now have one too;
            selftests went 307 -> 359. THREE packages left: buildx, logx,
            proc. verifyx was the one that mattered -- pgb verify decides
            criterion 2 from four pure functions and nothing asserted them.
            The control is the historical `.so` substring defect, planted:
            3 of 343 cases fail.
    T-060   rungs 2 and 3, the static nix
    T-054   rungs 3 (KF6) and 4 (kdenlive static)
    T-057   item 2: a 32-bit application through the lib32 path
    T-051   the no-compiler host
    T-012   pgb build <url-or-package>
    then    P2 by category

⭐ **Two pieces of real work are NAMED and are not entries**, because each is
one clear fix inside T-063 arm S:

    the static link-order problem   AC_SEARCH_LIBS probes -lreadline alone and
                                    libreadline.a's ncurses references go
                                    unresolved. poc/91-qt-xcb answered the same
                                    class with -Wl,--start-group
    a C link that pulled a C++      ✅ FIXED 2026-09-03. elfx.NeedsCXXRuntime
    archive                         reads the link line's archives for an
                                    UNDEFINED operator new/delete or an
                                    __cxxabiv1 vtable and the wrapper appends
                                    -lstdc++ -lm after the link flags. Fails
                                    before, passes after; carried as
                                    `cxx-runtime` with a negative control.
                                    ⚠ It does not by itself build postgres
                                    with ICU -- it removes the named blocker.

## Open questions for the operator

⭐ **None blocking.**

1. ⚠ **Two branches exist on the remote that no session created deliberately**
   — `claude/glibc-kdenlive-validation-2x7c3c` and
   `claude/glibc-research-foundations-7pjoqe`, both named by the harness.
   `RULES.md` §Git outranks the harness and every commit is on `main`. The git
   proxy refuses remote deletes, so they are left for a human to remove in the
   web UI.
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
