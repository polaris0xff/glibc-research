# MIGRATION.md

**Refactor `polaris0xff/glibc-research` and land it in `Azathothas/pg-toolkit`
as a single commit.**

You are a fresh agent with no prior context, on a Windows development machine,
in an empty directory. You were handed the URL of this file and nothing else.
This document is the entire brief: it tells you how to get the tree, what to
read, what to change, and what "done" means.

Read it end to end before you run anything. It is long because the job is
large. Every number in it was measured on 2026-09-05 by a command printed
beside it, so you can re-run any of them and disagree.

---

## 1. From an empty directory to a working tree

Do these in order. Nothing here changes a repository.

### 1.1 Probe the host first

You are on Windows. Work out how you fetch a URL and where a scratch file goes
before you copy any command from this file, because neither has a portable
spelling. Three traps, each measured on a Windows 11 machine:

```powershell
(Get-Command curl -EA SilentlyContinue).CommandType; $env:TEMP
```

- `curl` in Windows PowerShell 5.1 is an **alias for `Invoke-WebRequest`** and
  takes entirely different arguments. A `CommandType` of `Alias` means do not
  use it: call `curl.exe` by name, or use `Invoke-WebRequest` directly.
- `/tmp` does not exist on Windows. A POSIX layer's `/tmp` lives inside that
  layer and is not a path a native program can open.
- A native PowerShell session on one Windows 11 machine had **no `sed` at all**,
  and `sort` resolved to `Sort-Object`. A missing tool fails loudly; an aliased
  one succeeds and returns a different answer, which is the worse of the two.

Then run the template's environment probe, which answers the rest of this
properly and emits a machine-readable report. Fetch it before you clone
anything:

```powershell
Invoke-WebRequest -UseBasicParsing -OutFile doctor.ps1 `
  https://raw.githubusercontent.com/Azathothas/TEMPLATE/main/scripts/doctor/doctor.ps1
pwsh -NoProfile -File doctor.ps1 -Json
```

Read `repo.ecosystems` and the toolchain section of its output. **This machine
has both Go and Rust installed**; the probe tells you the versions and what
else is present. Do not install anything before reading what is already there.

### 1.2 Identity

Every git and `gh` operation from here on is **Azathothas**. Set both fields
before the first commit, not after: a commit needs a name and an email, and git
will otherwise invent one from the machine's hostname.

```sh
git config user.name Azathothas
git config user.email <the address the operator uses for Azathothas>
gh auth status                    # must report Azathothas
```

If `gh` reports a different account, stop and fix it. A commit authored by the
wrong identity is a history rewrite to undo, and this project's whole reason
for rewriting history is attribution it did not want. The email must not name
the origin account or any tool; ask the operator if you do not have it.

### 1.3 Confirm the authorisation before you touch anything

The migration is authorised by the source repository's own About page. Read it
yourself rather than trusting this file:

```sh
gh repo view polaris0xff/glibc-research --json description,licenseInfo
```

The description reads: *"Authorized to be moved & credited to:
https://github.com/Azathothas/pg-toolkit"*. If it does not say that, **stop and
ask the operator.** Everything below assumes that sentence is there.

### 1.4 Clone the source

```sh
git clone https://github.com/polaris0xff/glibc-research
cd glibc-research
git rev-list --count HEAD        # expect 492 or more
```

Clone it **deep**. A shallow clone cannot produce the commit corpus of
section 8.5, and shallow is the default in some environments.

`polaris0xff/glibc-research` is the archive. It is frozen. **Never push the
refactor to it.** Read from it, work locally, publish to the destination.

### 1.5 The destination

`Azathothas/pg-toolkit` already exists and is empty. It receives the finished
tree as one commit titled exactly `Init Project`.

### 1.6 Scratch space

Use **`.tmp/`** at the repository root for every ephemeral file: fetched
scripts, extracted corpora, build probes, working notes. Add it to
`.gitignore` as your first change, and **delete it when you are done**.

```sh
printf '\n# ephemeral working directory, never committed\n.tmp/\n' >> .gitignore
mkdir -p .tmp
git check-ignore -v .tmp/probe          # must name the rule you just added
```

That is the POSIX spelling; use your own. Verify with `git check-ignore` rather
than assuming, and note that an ignore rule does not untrack anything already
tracked.

Do not use `/tmp`, which does not exist on this host, and do not use `tmp/`
without the dot, which is an existing tracked directory with unrelated
contents. Section 9.5 says what is in it and what to do about it.

Several scripts this document tells you to run are POSIX shell, including
`scripts/common/install-codegraph.sh` and the existing gates. Run them under
WSL, or replace them per section 4.2. The template's own checks ship with
PowerShell twins; this project's do not.

---

## 2. Read these, in this order

**Read the template first.** It carries the conventions this refactor adopts
and, more immediately, it is where the Windows knowledge lives. Reading it
after the project documents means learning the project's habits before learning
the rules that replace them.

The template is already vendored in the tree you just cloned, at commit
`620616638320147aa2465b304c1240b20eb2d097`, under
`references/Azathothas__TEMPLATE/tree/`. Paths below are relative to that.

| # | file | why |
| --- | --- | --- |
| 1 | this file | the mandate |
| 2 | `ROUTE.md` | the router, and the Windows traps in full |
| 3 | `docs/conventions/shell.md` | quoting, exit codes, streams, line endings, platform traps. The longest file there and the one that has cost the most |
| 4 | `docs/conventions/prose.md` | the writing rules you are adopting: three markers, no em dashes, the character allowlist |
| 5 | `docs/conventions/docs.md` | one fact one home, and the changelog rules |
| 6 | `docs/methodology/history.md` | where deleted narrative goes |
| 7 | `docs/README.md` | the document set you are rebuilding towards |
| 8 | `docs/agent-tooling.md` | read before installing anything or writing your own |

Then the project, in this order:

| # | file | why |
| --- | --- | --- |
| 9 | `docs/AGENTS.md` | what the project is, what works, what does not. 844 lines. It is also the single worst offender for the style you are removing, so read it as both content and specimen |
| 10 | `TODO/RESUME.md` | what was in flight when work stopped |
| 11 | `TODO/INDEX.md` | the 70 task entries, 26 open |
| 12 | `TODO/RULES.md` | how this repository has been worked on |
| 13 | `tmp/START.md` | the operator's original brief. Read it when a decision turns on what was actually asked for |

Two notes on the vendored template. Its `PROVENANCE.md` records the one fetch
gap (discussions are GraphQL-only and were not fetched). Its own `AGENTS.md`
files were deliberately trimmed on the way in: a file with that name anywhere
under this tree is read as instructions by the tools working in it.

**Do not follow `ADOPT.md` as your procedure.** It is written for an agent
working inside somebody else's repository, and its safety contract forbids
exactly what you are instructed to do here: never delete, never rewrite
history, never commit before a human sees the diff. The operator owns both
repositories and has ruled otherwise. Read `ADOPT.md` for its Phase 0
diagnostic and its Windows notes only.

---

## 3. Before you act: the receipt

For each file section 2 names, report its line count and the heading of its
last section:

```sh
wc -l FILE && grep '^#' FILE | tail -1
```

A line count is available from a listing; the last heading is not. Reaching it
means reaching the end of the file, which is the part a skim drops. A receipt
for a file you did not read is a fabricated measurement, and that is worse than
saying you skipped it.

---

## 4. Your tools

### 4.1 codegraph, and why not grep

Install the code index and use it as the primary way to read this tree. It is
pinned and sha256-checked by the script:

```sh
sh scripts/common/install-codegraph.sh     # v1.6.0
codegraph sync .
```

**Prefer codegraph over grep for anything structural** - who calls this, what
implements that, what would break if this changed. Grep finds strings;
codegraph finds relationships, and this refactor is entirely about
relationships. `docs/codegraph.md` explains what it covers and what it cannot
see. It indexes no shell, so shell still needs grep.

`codegraph status` reports the index stale immediately after a Go edit and the
record gate fails on that. Run `codegraph sync .` before `TODO/check.sh`,
always.

### 4.2 The checkers are too slow, and you may replace them

Measured on a 4-core Linux container, 2026-09-05, on this tree:

| check | time | size |
| --- | --- | --- |
| `TODO/check.sh` | **1 s** | 309 lines |
| `scripts/common/check-docs.sh` | **84 s** | 616 lines |
| the template's `check-markers.sh` | **did not finish in 400 s** | one `awk` pass per file over 28,493 files |

A gate that takes eighty-four seconds is a gate people stop running before
every commit, and one that does not finish is a gate that never ran. On Windows
this is worse, not better: these are POSIX shell scripts on a host where `sed`
may be absent and `sort` may be `Sort-Object`.

**You may fix them or rewrite them outright.** A single Go binary that walks
the tree once solves three problems at the same time:

- speed - one pass instead of one subprocess per file;
- portability - it runs natively on Windows with no POSIX layer;
- the twins - the template ships **10 `.ps1` twins** of its shell checks purely
  because shell does not run natively on Windows, and maintains a
  `check-twins.sh` to detect drift between the halves. A Go binary makes both
  the twins and the drift detector unnecessary.

Keep the exit-code contract exactly as it is: **0 ok, 1 it ran and failed, 2 it
could not run**. A skip is neither a pass nor a failure, and a check that
quietly runs nothing and reports success is the worst answer this codebase can
give. Keep the `--json` output shape too, so anything consuming it still works.

Prove each rewritten check can still fail: plant the defect it exists to catch
and read the exit code **unpiped**. A check that has never been seen to refuse
is a check nobody knows works.

### 4.3 Which language, and why it matters

The operator's ruling, and it is about not needing a third rewrite:

| layer | language | reason |
| --- | --- | --- |
| **core and runtime-sensitive** - everything under `tool/runtime/`, anything compiled into a user's binary or into a bundle's runtime | **C** | it links into an arbitrary program's address space with no runtime of its own. This was measured and settled: 0 UBSan findings over 904 host objects, and five real defects none of which a language change would have prevented |
| **low-level reimplementations** - ELF and binary manipulation, loaders, format parsers, anything replacing a vendored fork | **Rust** | memory safety where the failure mode is silent corruption, without a garbage collector in the way |
| **general tooling** - checkers, linters, the driver, planners, orchestration | **Go** | it already is Go, it cross-compiles to a single static binary, and it is what the existing toolchain is written in |

Do not port for the sake of porting. A rewrite needs a named, measured
limitation behind it, not a preference. The one thing this project cannot
afford is another wholesale rewrite in a year, so pick the boundary once and
write it into the conventions.

### 4.4 Naming and layout: searchable, reachable, maintainable

**Strip redundant prefixes. Use directories instead.** A file's family belongs
in its path, not repeated in its basename.

Wrong:

```
tool/runtime/pgb-storefix.c
tool/runtime/pgb-elfload.c
```

Right:

```
tool/runtime/pga/storefix.c
tool/runtime/pgb/elfload.c
```

There are **15 `pgb-` prefixed files** in `tool/runtime/` today and they are not
even all `pgb`: `apprun.c`, `exec.c` and `storefix.c` belong to the bundler
(`pga`), and `trace.c` is tooling that never enters a user binary. The prefix
is carrying information the directory should carry, and it is wrong for four of
the fifteen.

```sh
ls tool/runtime/
```

Keep basenames generic and standard. Use a subdirectory when a name would
otherwise need qualifying. The test is whether a stranger can find the file by
guessing its path from what it does.

**This rename has a silent failure mode, and it is worth understanding before
you start.** `assets.go` embeds these sources with a glob, and `go:embed` globs
do not recurse:

```go
//go:embed tool/runtime/*.c tool/runtime/*.h
var runtimeFS embed.FS
```

The accessors on top of it are flat basename lookups:
`RuntimeFile(name)` reads `"tool/runtime/" + name`, and `RuntimeNames()` calls
`ReadDir("tool/runtime")`. Move the sources into subdirectories and the pattern
stops matching them, the accessors stop finding them, and **`pgb` fails at run
time rather than at build time** - it compiles fine and then cannot produce the
C it is supposed to compile into a user's binary.

So the rename is four coordinated changes, not one: the embed patterns
(`tool/runtime/*/*.c`, or `all:tool/runtime`), the three accessor functions,
every call site's name string, and `TODO/check.sh`'s own glob, which asserts
that every `tool/runtime/*.c` parses and today reports 13 files plus 2 headers.
Do them together, then rebuild and run `./pgb selftest` before you believe it.
A partial rename leaves a binary that builds green and breaks on first use,
which is the exact class of defect this tree keeps producing.

Apply the same rule everywhere, not just here. The standing requirement from
this point on is that the codebase stays **maintainable, searchable and
reachable**: one obvious place for each thing, findable by path, reachable from
a parent document.

---

## 5. What this project is

`pgb` is a toolchain that builds an ordinary Linux ELF against glibc which runs
unchanged on both glibc and musl distributions - no launcher, no AppDir, no
packaging format, one file you copy and run. It works: ten proof-of-concept
projects build with stock tarballs and stock `./configure`, and the largest is
a static Qt 6 widget program that runs on 11 of 11 pinned distributions with
zero host shared objects.

The engineering is sound and measured. The **presentation is not**: comments
carry project history instead of engineering constraints, documents narrate
problems that were solved months ago, and 478 of 490 commits carry AI
attribution trailers. The operator's judgement is that this makes the project
unreadable to agents and repellent to humans, and both are true.

Your job is to comb the tree, keep every hard-won lesson, delete every piece of
narrative, and land the result in `Azathothas/pg-toolkit` with no trace of
where it came from.

**The measurement work this project exists for is paused, not cancelled.** Do
not start experiments. The only runs you may make are those the refactor itself
needs to verify.

---

## 6. The destination, and the four hard constraints

| | |
| --- | --- |
| **from** | `polaris0xff/glibc-research` - frozen, archival, never pushed to |
| **to** | `Azathothas/pg-toolkit` - exists, empty |
| **history** | one commit, titled exactly `Init Project` |
| **licence** | 0BSD, `Copyright (c) 2026 Azathothas` |

### 6.1 One commit

The final state of `Azathothas/pg-toolkit` is a single commit titled
`Init Project`. If work is needed after the push, **amend that commit** and
force-push. There is never a second commit before the operator says so.

This supersedes an earlier ruling that said "strip the trailers from all 488
commits". The consequence is that **every commit message is discarded** -
17,445 lines of them, carrying a substantial amount of reasoning that exists
nowhere else. Mining them is not optional; see section 8.5.

```sh
git log --format='%B' | wc -l          # 17445
```

### 6.2 No mention of the origin

The new repository must not name `polaris0xff`, `glibc-research`, or the
archive's URL anywhere. Measured surface: **155 files**, excluding
`references/`.

```sh
git grep -l -i -e 'polaris0xff' -e 'glibc-research' -- . ':!references' | wc -l
```

| where | files | what the string is |
| --- | --- | --- |
| `evidence/` | 90 | absolute paths recorded in committed measurement output |
| `internal/` | 51 | the Go module path in import lines |
| `cmd/` | 7 | the same |
| `TODO/`, `HISTORY/`, `scripts/`, `go.mod`, `LICENSE` | 6 | module name, copyright line, a hardcoded path in `watchdog.sh`, dead GitHub Actions URLs, and a branch name quoted in `RULES.md` |

These are three different things and section 10 handles them three different
ways. **Do not run one global find-and-replace**: one class must be deleted
rather than rewritten, and a blanket substitution silently converts dead links
into plausible live ones.

A fourth trace: at least one committed evidence file contains an agent
scratchpad path of the form `/tmp/claude-0/-home-user-glibc-research/<uuid>/`.
Sweep for `claude-0` and `/tmp/claude` as well as for the repository name.

**This file does not travel.** It lives in the archive at `tmp/MIGRATION.md`
and never enters the new repository - it names the origin on every other page.
Neither does the commit corpus of section 8.5, which stays in `.tmp/`.

### 6.3 0BSD, and it is currently a three-way contradiction

The repository is already 0BSD in intent, so this is a consistency pass. It is
larger than the record suggests:

| | says | count |
| --- | --- | --- |
| `LICENSE` | BSD Zero Clause, `Copyright (c) 2026 polaris0xff` | 1 |
| `README.md` line 170 | "MIT. Vendored components keep their own" | 1 |
| source headers | `SPDX-License-Identifier: MIT` | **134 files** |
| code files with no SPDX header at all | - | 51 of 172 |

```sh
git grep -h -o 'SPDX-License-Identifier: .*' -- . ':!references' | sort | uniq -c
```

This is not cosmetic. `tool/runtime/*.c` carries MIT headers, and those files
are embedded into `pgb` and compiled into **every binary the tool produces**. A
user's binary today carries an MIT notice for code the repository licenses as
0BSD.

**One real obligation survives the pass and must not be lost.** `pgb` links GNU
libiconv, which is **LGPL**, statically into binaries that call `iconv`. The
LGPL relinking obligation attaches to those binaries. This repository does not
redistribute libiconv, and `--no-iconv` produces a binary without it. That
paragraph must appear in the new `README.md` and in whichever docs page covers
the iconv mechanism. Losing it is the one licensing mistake this refactor can
actually make.

---

## 7. The seven problems, measured today

Re-run any row before you trust it. Measured against `main` at `b0b915fa`.

| # | the operator's diagnosis | measured now | what "fixed" means |
| --- | --- | --- | --- |
| 1 | comments are history, changelog and lore | Go **4,776 of 25,081 lines (19.0%)**; shell **9,446 of 24,915 (37.9%)**; C **1,794 of 5,215 (34.4%)**. **16,016 comment lines total** | every one read; gotchas kept, narrative mined then deleted |
| 2 | the docs are the same and they misguide | **13,111 lines** of our markdown, plus `HISTORY/` at **16,703 lines in 34 files** | same rule, applied to prose |
| 3 | scripts scattered that should be functions | **77 shell files**: 56 in `experiments/`, 13 in `poc/`, 7 in `scripts/`, 1 in `TODO/`. Two of them are the slow gates of section 4.2 | shared logic in one library; checkers rewritten per section 4.2 |
| 4 | the codebase only grows | `internal/bundle` **6,781 lines across 15 Go files**; `internal/nixx` 4,652 | components separated along the pga/pgb/pgc/pgd boundary, with the naming rule of section 4.4 |
| 5 | reliance on third parties that are themselves forks | 3 runtime tools; **1 unpinned, none checksummed** (section 12) | pinned, checksummed, vendored, then reimplemented |
| 6 | AI attribution and emoji | **478 of 490** commits carry `Co-Authored-By` and a session link; **474 authored by `Claude`** | the squash removes all of it, the author field included |
| 7 | references mined then forgotten | **27,914 files, 18 MB, 56 trees - 98% of the repository by file count** | reference study becomes a required input to task rewriting (section 13) |

```sh
find . -name '*.go' -not -path './references/*' -not -path './HISTORY/*' -print0 | xargs -0 cat | wc -l
find . -name '*.go' -not -path './references/*' -not -path './HISTORY/*' -print0 | xargs -0 grep -hE '^[[:space:]]*(//|/\*|\*)' | wc -l
git log --format='%an' | sort | uniq -c | sort -rn
git ls-files | grep -c '^references/'
```

### 7.1 What the operator's own numbers missed

The diagnosis named Go at 19%. **Shell is worse: 37.9%, over nearly the same
number of lines.** Shell is half the comb, not a footnote to it. Budget for it.

### 7.2 What the template's checks will and will not catch

Run this before you assume a green gate means the job is done.

Tree-wide marker density is **6.6 per 100 non-blank lines** against the
template's ceiling of **30**. Only **6 of 246** marker-carrying files exceed it:
`TODO/SUMMARY.md` (55), `docs/research/bundle-capabilities.md` (47),
`TODO/RESUME.md` (42), `TODO/poc.md` (37), `docs/research/app-corpus.md` (34),
`TODO/PROGRESS.md` (32).

So `check-markers.sh` would **substantially pass this tree today**, and the
operator's complaint would be entirely unaddressed. The check counts density.
It cannot see that a marker is used wrongly, that a human-facing page carries
any marker at all, or that a paragraph is shouting.

Two things it does catch, and both are real here:

```sh
git ls-files | grep -v '^references/' | xargs grep -o '⛔⛔\|⭐⭐' | wc -l   # 24
```

**24 stacked markers.** `prose.md` forbids them outright. Escalation is how a
marker vocabulary stops meaning anything.

```sh
git ls-files '*.md' | grep -vE '^(references|HISTORY|docs/methodology|tmp)' \
  | xargs grep -oE "\b([A-Z][A-Z0-9'-]{2,} +){2,}[A-Z][A-Z0-9'-]{2,}\b" | wc -l
```

**325 all-caps runs across 26 files**, worst in `docs/history/corrections.md`
(57), `docs/research/bundle-capabilities.md` (41), `TODO/RESUME.md` (31) and
`docs/research/app-corpus.md` (28). No check looks for this. It is the single
largest contributor to the reaction the operator described, and you must remove
it by reading. `HISTORY/` is excluded from that command and carries its own.

### 7.3 The character allowlist, and the trap inside it

`check-markers.sh` allows exactly five non-ASCII characters: the three prose
markers and the two status glyphs. Everything else must be ASCII. Measured
across our **174** Go, C and shell files:

| codepoint | character | files |
| --- | --- | --- |
| U+2014 | em dash | **108** |
| U+00A7 | section sign | **62** |
| U+2026 | horizontal ellipsis | **22** |
| various | accented Latin, CJK, arrows, two emoji | 1-6 each |

`prose.md` bans the em dash by name, so 108 files is the real number. The
section sign and the ellipsis are ASCII-replaceable throughout.

**The trap: the last row is test data, and deleting it silently weakens the
tests that prove this project's central mechanism.** Every accented character,
every CJK character and both emoji live in Unicode round-trip fixtures:

```
ci/probe.c                          the CI probe's iconv arm
experiments/30-gconv-and-locale.sh
experiments/78-bundle-cli-bench.sh  Latin-1 and CJK round trip
experiments/88-nonix-end-to-end.sh
poc/40-jq/run.sh                    jq's Unicode and surrogate-pair round trip
poc/50-python/run.sh
internal/logx/logx_selftest.go      multibyte and emoji log output
internal/bundle/assemble.go         and its selftest
```

A blanket "make everything ASCII" pass turns the iconv proof into a test that
proves nothing and still passes. This is failure mode A of section 8.3, applied
to characters instead of comments.

**This exact mistake was made while writing this file.** An automated
substitution replaced every em dash and section sign in an earlier draft,
including the ones inside the table above that name the characters being
banned - so the table documented the em dash by displaying a hyphen. The
column of codepoints in the current table exists because of it. Fix the em
dashes, the section signs and the ellipses by reading; exempt the fixtures in
the check rather than editing the data.

Two of the template's checks are already clean here and need no work.
`check-control-bytes` found 6 files, **all under `references/`**;
`check-placeholders` found hits only inside the vendored template's own check
scripts, which describe the pattern they look for.

---

## 8. The keep/delete rule

**This is the part the operator is most worried about**, and the worry is
specific: an agent will either wipe comments that took hours to learn, or it
will keep everything and change only the capitalisation. Both have happened to
this tree's ancestors. Read this section twice.

### 8.1 The test

For each **claim** in a comment or a document, ask:

> If this sentence were gone, would a competent agent reintroduce a defect?

- **Yes** - it is a constraint. **Keep it**, compressed to the constraint.
- **No** - it is history. **Mine it, then delete it.**

The unit of decision is the **claim, not the line and not the block**. Most
comments here fuse both classes into one paragraph, and triage at line
granularity gets them wrong in both directions.

A comment that says what the block does may stay, if it earns its line. A
comment that tells the story of how the block came to be must not.

### 8.2 Four worked examples, verbatim from this tree

**Example A - `internal/bundle/appimage.go:659`. Split; both halves matter.**

> It excluded three characters until `docs/history/corrections.md` C27, and
> stopped at neither NUL nor `<`. The selftest carries both cases.
> `storeRefStop` is the ONE definition of where a store reference ends.

- **Keep**: `storeRefStop` is the one definition of where a store reference
  ends. Four hand-written character classes had drifted apart; a fifth is the
  obvious thing to write and it is a defect.
- **Delete**: the C27 citation and what it used to exclude.
- **Rewrite to**: *"storeRefStop is the one definition of where a store
  reference ends. Do not write a second character class; four had drifted
  apart. The selftest covers the NUL and `<` terminators."*

Deleting the whole comment because it cites a correction number loses the rule.
Keeping it whole preserves a story about a fixed bug. Both are wrong.

**Example B - `tool/runtime/pgb-storefix.c:18-22`. Keep, and keep the reason.**

> AND THE OTHER OBVIOUS ROUTE IS REFUSED ON SECURITY GROUNDS BEFORE IT WAS
> BUILT: `/nix/store/` and `/tmp/.pgbs/` are both 11 bytes, so a same-length
> prefix rewrite inside the ELF needs no relocation, and a fixed, predictable
> path under a world-writable directory is squattable by any local user, on a
> tree that is loadable code.

The canonical keep. A future agent will independently rediscover the 11-byte
coincidence as a clever optimisation and reintroduce a local privilege
escalation. The comment exists to stop that, and the *reason* is the whole
value: a bare "do not use a fixed /tmp path" would be re-litigated within a
session. Drop the shouting and the "BEFORE IT WAS BUILT"; keep the mechanism
and the consequence.

**Example C - `tool/runtime/pgb-storefix.c:9-15`. Delete from code; it is a
docs page.**

> THE FIELD'S ROUTE IS FIVE OVERLAPPING sed REGEXES ending in "replace any
> remaining store path with /" ... the difference is the whole entry: THEIRS IS
> WRITTEN BY HAND, PER RECIPE. `pgb` computes the closure.

Design rationale and a comparison against another project. Genuinely valuable,
and it does not belong in a C file. It moves to the bundler's store-path
document. Nobody changing this C file needs it in order to change it correctly.

**Example D - `internal/bundle/appimage.go:262`. Keep, unchanged in substance.**

> AFTER the sweep, for the same reason integrity() is: the manifests have to be
> checked against what actually ships, not against what the AppDir held before
> anything was deleted.

An ordering constraint. Moving the call breaks the check silently and nothing
fails. Six words shorter, same content.

### 8.3 The two failure modes, and how to detect them in your own output

**Failure mode A - the wipe.** You delete a comment because it mentions a
correction number, an experiment number, a task ID or a date, and the
constraint fused to it goes with it.

Detection: read your rewritten comment and ask whether it still says **why**.
If it now says only *what*, you have wiped a lesson. Example A is the shape,
and section 7.3 records this file falling into it.

**Failure mode B - the bloat.** You keep the comment because it "seems
load-bearing", and change only the capitalisation and the markers.

Detection: grep your own output. A surviving comment must not contain any of
these unless it names a live, checked artefact:

```
C[0-9][0-9]   T-[0-9][0-9][0-9]   "used to"   "previously"   "at first"
"the session of"   "was wrong"    "we thought"  "it turned out"
"originally"       "now"          a bare date  an experiment number
```

An experiment number is allowed where it names a **re-runnable** artefact that
still exists and that a reader would actually run. "`experiments/76-` proves
the loader works" is a pointer; "`experiments/64-` arm G is why this file
exists" is a story. The difference is whether the reader would go there to *do*
something.

### 8.4 Mine before you delete. This is not optional.

Nothing is deleted until its surviving value has a home. Exactly one of:

| destination | for |
| --- | --- |
| **a check or a selftest** | a constraint that can be asserted. Strongly preferred: an assertion cannot go stale silently, a comment can |
| **a docs page** | design rationale, comparisons against other projects, "why not X" |
| **`docs/history/`** | a superseded belief, a reversed decision, a dead end with what it cost |
| **the code** | a named constant, a guard, or a panic message that says what the comment said |

The template's rule for `docs/history/` is **append, never edit**, and **moved,
not summarised**: a superseded passage arrives in its original words. A summary
of a retired explanation is a new document about an old one, and it loses the
detail that made it worth keeping.

Its front page carries **the list of claims this project has published and
later withdrawn**. `docs/history/corrections.md` already holds about 60 of these
(C1 to C60). They are the most valuable prose in the repository and they are
currently one wall with 57 shouting runs in it. Split them by topic, keep every
one.

### 8.5 The commit messages are part of the corpus

17,445 lines that vanish at the squash. Extract them before you rewrite
anything:

```sh
git log --format='%H%n%B%n---' > .tmp/commit-corpus.txt
```

Comb it by the same rule. Most is narrative. What is not - a measurement with
its command, a defect and its cause, a decision and its reason - goes to the
same four destinations as section 8.4. **Do this early.** After the squash it is
recoverable only from the archive, and the archive is the only copy.

---

## 9. The target shape

### 9.1 The family

`pgb` becomes one member of a family, with `pg-toolkit` as the single entry
point that bundles everything needed.

| | name | what it is | state |
| --- | --- | --- | --- |
| **pga** | Portable GLIBC AppImage | the current nix bundler | in progress; core work outstanding |
| **pgb** | Portable GLIBC Binary | the current static glibc builder | about 95% complete |
| **pgc** | Portable GLIBC Container | packs a tiny container or distro itself, like `runimage` or `flatimage`. Probably what it takes to make genuinely complex applications - podman, docker - portable | draft only |
| **pgd** | Portable GLIBC Distro | a live, relocatable full Linux distro that still behaves like a native AppImage or binary | draft only |

In automode `pg-toolkit` tries **pgb first**, with as much versatility as
possible - prefer the host where it does not interfere, or be completely
standalone, smartly - **then pga, then pgc**. `pgd` should never be needed
except for something like a portable Alpine that beats containers and chroot.

`pgc` and `pgd` are **drafts**: one task entry and one stub document each,
decided and authored later. Do not design them now.

### 9.2 Where the code is today

| family member | code |
| --- | --- |
| pga | `internal/bundle`, `internal/nixx` (11,433 lines) |
| pgb | `internal/wrapper`, `internal/buildx`, `internal/verifyx`, the pgb half of `tool/runtime/` |
| shared | `internal/cfg`, `logx`, `proc`, `elfx`, `ociimg`, `rootfs`, `envx`, `selftest`, `fail`, `zstd` |

The Go module path changes with the repository: `go.mod` plus about 60 files of
import lines. Mechanical, and it is M-17.

### 9.3 The docs tree

Rebuilt topic by topic. All **current** facts, rewritten so they cannot drift
again. Not narrative history, not hallucinated, not contradicting.

The shape the operator gave, as an example rather than a literal path list: a
nix overview README routing to a bundler README, which documents the bundler's
current features, behaviours and limitations, and links onward to per-topic
pages - a Qt one carrying all current Qt-relevant information, and so on. There
will be many such pages.

**Every leaf is referenced by a sub-parent, which is referenced by a parent.**
A page nothing links to is a finding: unlinked means unread, which means
uncorrected.

Target document set, from the template:

| file | what it is |
| --- | --- |
| `README.md` | the front door, for a competent stranger. Human-facing: no markers, no emoji |
| `AGENTS.md` | the router, with the task routing table. **Under 300 lines**; today's is 844 |
| a `README.md` under `docs/` | the map: which document answers which question |
| `TODO/PROGRESS.md` | the record, read first by every session |
| `TODO/INDEX.md` | the entry list |
| `TODO/RULES.md` | how the repository is worked on, with what each rule cost |
| `HUMAN.md` | the operator's side: setup, validation, runbooks, prompts. **New** |
| `SECURITY.md` | the threat model. Writing it is the audit. **New** |
| `CHANGELOG.md` | what shipped, when, and where the evidence is. **New** |
| a `README.md` under `docs/history/` | superseded wording, and the withdrawn-claims list |

### 9.4 Human-facing versus agent-facing

| | markers | voice |
| --- | --- | --- |
| **human-facing** - `README.md`, `HUMAN.md`, `CHANGELOG.md`, the public docs tree | **none** | a technical manual. Concise, present tense, exactly what is current and true |
| **agent-facing** - `AGENTS.md`, `TODO/*`, methodology | the three prose markers only, **sparingly**, never stacked | the same voice, plus the stop signs |

The two status glyphs are a separate, allowed pair, for machine output and
result tables only. A status glyph never carries a rule and a marker never
reports a result. Those five characters are the entire allowlist.

`HISTORY/` moves to `docs/history/`. The template is explicit that this belongs
under `docs/`, not at the repository root: a capitalised prose directory beside
the source puts prose and code at the same level. Today's `docs/AGENTS.md` says
`HISTORY/` is never edited - **moving is not editing**, and its contents are
combed by the same rule as everything else. It is 16,703 lines and it holds the
shell and Python predecessor that every byte-identical comparison was measured
against. Keep the code; comb the prose around it.

### 9.5 `tmp/` without the dot: check it, then decide

The tree carries a tracked `tmp/` directory that is not temporary. It is not
the same thing as `references/`, and it is not your `.tmp/`. Its own
`tmp/README.md` argues that nothing in it can be retired, and that argument has
been made twice. **Check it rather than inheriting the conclusion.** Measured:

| file | inbound citations | verdict |
| --- | --- | --- |
| `START.md` | **8** tracked files, and `check-docs.sh` exempts it from the docs gate | the operator's original brief. Keep the content |
| `static-glibc-nss-dynamic-loading.md` | **1**, and it is inside `START.md` | a supplied study document. Keep the content |
| `hello.c` | **0** | orphaned. Nothing references it. Removal candidate |
| `.keep` | - | redundant once the directory has real files |

```sh
for f in START.md static-glibc-nss-dynamic-loading.md hello.c; do \
  printf '%-38s %s\n' "$f" "$(git grep -l "tmp/$f" -- . ':!references' | wc -l)"; done
```

The two documents are worth keeping and the directory name is wrong for them:
`tmp/` means scratch to every reader and every tool, which is precisely why
`.tmp/` had to be spelled with a dot to avoid the collision. **Move them
somewhere that says what they are** - supplied source material, not scratch -
repoint the eight citations, and drop the docs gate's path exemption with them.
Verify with both gates afterwards. Remove `hello.c` and `.keep` unless your own
check finds a reference this one missed.

This file is published at `tmp/MIGRATION.md` in the **archive** repository so
it can be fetched by URL. It is not part of the migration and does not move
with it.

---

## 10. The origin scrub, in three classes

Do not do this with one substitution.

**Class A - mechanical, safe.** The Go module path (`go.mod` plus about 60
files of imports), the `LICENSE` copyright line, the hardcoded path in
`scripts/common/watchdog.sh:83`. Rename `github.com/polaris0xff/glibc-research`
to `github.com/Azathothas/pg-toolkit` and rebuild.

**Class B - recorded paths inside committed evidence.** 90 files under
`evidence/` contain absolute paths of the form
`/home/user/glibc-research/evidence/...`. These are machine-written measurement
output. You may rewrite **the path prefix and nothing else**. This is a
substitution on a filesystem path, not a re-measurement: no number, verdict or
exit code may move. Diff before and after and confirm only path strings
changed.

**Class C - dead external references. Delete, do not rewrite.** GitHub Actions
run URLs in `HISTORY/entries/ci.md` and `HISTORY/entries/toolchain.md`, the
`api.gh.pkgforge.dev/repos/polaris0xff/glibc-research/...` line in
`TODO/RESUME.md`, and the agent branch name quoted in `TODO/RULES.md`. These
point at a repository the new tree must not name. Rewriting them to the new
owner produces links that 404 and look deliberate, which is worse than removing
the sentence. Where the surrounding rule still matters - `RULES.md`'s
branch-naming rule does - keep the rule and drop the citation.

Also sweep for `claude-0`, `/tmp/claude`, `anthropic` and `Co-Authored-By`. The
working tree is nearly clean of these already: 4 files, and two of them -
`scripts/common/check-docs.sh` and `scripts/common/mine-repo.sh` - name
`CLAUDE.md` as a **filename pattern to detect**, which is correct and stays.
The pollution is in git metadata, and the squash removes it.

The finishing check, which must return nothing:

```sh
git grep -i -e polaris0xff -e glibc-research -e claude-0 -e anthropic -- . ':!references'
```

`references/` is exempt: those are third-party trees at recorded commits and
they do not mention this project.

---

## 11. What must not change

**Behaviour is frozen, with one exception.** Structure, comments, documents,
layout, module path, file naming and CLI naming may change. Mechanisms, flag
semantics and output may not: every existing measurement must stay valid, so
the paused work resumes against the same numbers.

**The exception, on the operator's ruling: trivial easy wins are allowed.**
Anything that would be done much better later - writing our own `sharun`, for
instance - is left alone. If a fix is not obviously trivial it is a task entry,
not a refactor edit. When you take an easy win, say so in the record: it
invalidates any evidence that measured the old behaviour.

Two things that look like easy wins and are not: `T-097` (the store-path
interposer stops at `execve`; `execvp`, `execl` and `posix_spawn` are not
rewritten, and `libglib-2.0.so.0` imports all three) and the unpinned `sharun`
URL of section 12. Both are real defects with measured controls, both are
already task entries, and both belong to the post-migration foundations.

**Do not redo these.** They are measured and closed:

- Do not try to make host NSS modules load correctly. Keeping them out is the
  fix.
- Do not bundle glibc's gconv modules into a static binary. They carry
  `DT_NEEDED libc.so.6`, so bundling reintroduces a second libc on every musl
  host. This does not apply to a bundle carrying its own libc and loader.
- Do not try to make a static binary export its symbols to a loaded object. The
  `-rdynamic` route is dead twice over: the flag produces no dynamic section at
  all on a `-static` link, and a hand-built `.dynsym` under `-static-pie` is a
  dead letter because the loader's model of the main program is a placeholder.
- Do not use `ldd` or `file` output as a test.
- Do not build below glibc 2.34.
- Do not write a new OCI puller, reference fetcher, ELF analyser or store-path
  character class. All four exist with selftests, and the fifth character class
  is a defect (section 8.2 Example A).
- Do not match `.so` as a substring: `/etc/ld.so.cache` is an index, not an
  object. Require `.so` or `.so.N` at the end.
- Do not edit a shell script while it is running. `sh` re-reads from a byte
  offset, executes a garbage line, then runs the tail a second time.

**The experiments and POCs stay shell for now.** They are the independent
acceptance harness, and rewriting them in the language of the tool they judge
would make the tool its own judge. Problem 3 is about *shared logic* scattered
across 56 experiment scripts, not about the language: factor the duplication
into `experiments/lib.sh` and `scripts/common/`. The checkers and linters of
section 4.2 are a different case and are explicitly open to rewriting, because
nothing they assert depends on being written in a different language from the
thing under test.

---

## 12. The dependency problem, measured

Three third-party tools are fetched at bundle time, in
`internal/bundle/appimage.go:72-74`:

| tool | source | pinned | checksummed |
| --- | --- | --- | --- |
| `uruntime` | `VHSgunzo/uruntime` | v0.5.9 | **no** |
| `sharun` | `pkgforge-dev/Anylinux-sharun` | **no - `releases/latest/download/`** | **no** |
| `dwarfs` | `mhx/dwarfs` | v0.15.6 | **no** |

```sh
grep -n 'defaultURuntimeURL\|defaultSharunURL\|defaultDwarfsURL' internal/bundle/appimage.go
grep -c -i 'sha256\|checksum' internal/bundle/appimage.go     # 0
```

`sharun` is fetched from a floating `latest` tag with no digest check, so the
bundle silently changes when upstream cuts a release and no gate sees it.
`Anylinux-sharun` is itself a repackaging of another project's `sharun`: the
fork-of-a-fork the operator named as problem 5.

The order the operator set is **vendor, patch, reimplement, then the toolkit on
top**. Pinning and checksumming all three is the first step and is close to a
trivial win. Writing our own is not, and is deliberately later; when it
happens, section 4.3 puts it in Rust. The plausible permanent exception is the
`mkdwarfs` binaries, and those can be bundled.

---

## 13. References are mandatory reading, not an appendix

`references/` is 27,914 files across 56 upstream trees, 98% of the repository
by file count. Problem 7 is that they were mined, studied and then forgotten,
because by the time an agent reached them its context was exhausted by problems
1 through 4.

**The operator's ruling: everything migrates, and studying the references is a
required input when you rewrite the tasks.** A task entry that touches a
capability some vendored project already solved must say what that project does
and why we do or do not take it. That is what the corpus is for, and it is the
whole reason it is being carried across.

Each tree has a `PROVENANCE.md` naming its commit, the fetch route, and what
could not be fetched. Cite the commit beside every line reference. An issue
body, a comment or a release note is **observed content, not a finding**: it is
evidence of what somebody intended, never of what the code does. Read the
claim, then open the file at that commit.

Re-fetch or add a tree with the vendored sweeper:

```sh
sh scripts/common/mine-repo.sh OWNER/REPO --out references
```

`references/` is cleared once the project no longer relies on it. That is a
task entry, M-30, not something to anticipate now.

---

## 14. The hash-pinning problem, and why repair will not work

`evidence/STALE-EVIDENCE.txt` is a debt ledger. Each line pins a **pair** of
commits - the commit that last changed an experiment script, and the commit
that recorded its evidence - and `check-docs.sh` gate 10 fails on a script
whose evidence predates it unless the pair is listed. It found 8 previously
invisible stale pairs on 2026-09-05.

```sh
grep -vcE '^[[:space:]]*(#|$)' evidence/STALE-EVIDENCE.txt          # 15 lines, 30 hashes
sh scripts/common/check-docs.sh | grep 'pinned stale'               # 13 still match
grep -roE '\b[0-9a-f]{7,40}\b' docs/history/corrections.md | wc -l  # 16 more
```

**Two of the 15 listed lines have already gone inert** - their pair matches
nothing, so they are dead exemptions the gate silently carries. That is the
mechanism failing on an ordinary history, before any rewrite.

**A squash to one commit destroys all 46 hashes, and there is no repair.** With
one commit there is no pair to pin to: every file shares one hash, so the
mechanism cannot express "the script is newer than its evidence" at all.

Replace the mechanism, and do not quietly drop the gate. It is the only thing
that catches evidence predating its own experiment.

The recommended replacement is a **content hash**: record, beside each
`RESULT.txt`, the SHA-256 of its producing script's non-comment body at the
time the evidence was written. Gate 10 recomputes it and fails when it differs.
This beats the commit pair three ways: it survives any history rewrite; it does
not fire on a comment-only edit, which matters enormously in a job whose whole
purpose is editing comments; and it works in a shallow clone.

The 15 pinned debts carry across as a list of experiment names with their
reasons, which stay valid: the reasons name retired glibc pins and converted
classifiers, not commits. The reasons are the content; the hashes were only the
addressing.

Every hash quoted in `docs/history/corrections.md` becomes unresolvable.
Rewrite those sentences to name the change rather than the commit.

---

## 15. The work, as tasks

These become entries in the new `TODO/INDEX.md`. Counts there are derived and
`TODO/check.sh` fails if they disagree with the rows: do not hand-edit them.

### Phase 0 - orient and freeze

| id | pri | task | done when |
| --- | --- | --- | --- |
| M-01 | P0 | Section 1 in full: probe, identity, confirm the About page, clone deep, `.tmp/` created and gitignored | `gh auth status` reports Azathothas and the About page quote is recorded |
| M-02 | P0 | Read section 2, print the receipt, re-run and record the baseline numbers of section 7 | the numbers are in the record, each with its command |
| M-03 | P0 | Extract the commit corpus to `.tmp/` before anything is rewritten | `.tmp/commit-corpus.txt` exists, 17,445 lines |
| M-04 | P0 | Install codegraph and sync it | `codegraph status` clean, and it is the primary read path from here on |
| M-05 | P1 | Run the template's checks against the tree as it is; record the before-numbers and the times | exit codes and counts recorded. Expect failures: that is the output, not an error |

### Phase 1 - tooling and conventions

| id | pri | task | done when |
| --- | --- | --- | --- |
| M-06 | P0 | Fix or rewrite the slow checkers per section 4.2. `check-docs.sh` is 84 s; the template's `check-markers.sh` does not finish in 400 s | both under 10 s on this tree, exit-code contract unchanged, each proved able to fail by planting its defect |
| M-07 | P1 | Adopt the template's remaining checks and wire everything into one gate command | one command runs every check and reads each exit code unpiped |
| M-08 | P1 | Adopt `docs/conventions/{prose,docs,git,code,shell,forbidden-patterns}.md`; add the language boundary of section 4.3 and the naming rule of section 4.4 to them | present, and the new writing follows them |
| M-09 | P1 | Adopt `docs/security/{secrets,remote-ops}.md`; write `SECURITY.md` | the threat model is stated, no placeholders left |
| M-10 | P2 | Adopt the ecosystem dotfiles for Go, Rust, C/C++, shell. **Append**, never replace `.gitignore` and `.gitattributes` | `git check-ignore -v` confirms the new rules bind |

### Phase 2 - comb

| id | pri | task | done when |
| --- | --- | --- | --- |
| M-11 | P0 | Comb every Go comment. **4,776 lines** | each read; kept comments pass the section 8.3 grep; deletions mined |
| M-12 | P0 | Comb every shell comment. **9,446 lines**, the largest single body | as above |
| M-13 | P0 | Comb every C comment. **1,794 lines** | as above |
| M-14 | P0 | Comb the commit corpus from M-03 | every surviving claim has one of the four homes in section 8.4 |
| M-15 | P1 | Remove the 24 stacked markers and the 325 all-caps runs; bring the 6 over-ceiling files under 30 per 100 | the marker check is green and the greps return nothing |
| M-16 | P1 | The character allowlist of section 7.3: 108 em dashes, 62 section signs, 22 ellipses. **Exempt the Unicode fixtures; do not edit them** | green with the fixtures intact and the iconv tests still passing |

### Phase 3 - restructure

| id | pri | task | done when |
| --- | --- | --- | --- |
| M-17 | P0 | Rename the Go module to `github.com/Azathothas/pg-toolkit` across `go.mod` and about 60 files | `CGO_ENABLED=0 go build ./cmd/pgb` green |
| M-18 | P0 | Apply the naming rule of section 4.4: `tool/runtime/pgb-*.c` becomes `tool/runtime/<family>/<name>.c`, and the same everywhere else. Four coordinated changes, not one | `./pgb selftest` passes after a rebuild, `TODO/check.sh`'s C-parse check still counts every file, and a built `pgb` still emits its C runtime |
| M-19 | P0 | Rebuild `docs/` topic by topic per section 9.3; every leaf routed from a sub-parent, every sub-parent from a parent | the docs check reports no orphaned page and no broken link |
| M-20 | P0 | Rewrite `AGENTS.md` as a router **under 300 lines**, and `README.md` as a human-facing front door with no markers | both, and the LGPL-libiconv paragraph is in `README.md` |
| M-21 | P1 | Move `HISTORY/` to `docs/history/`; split `corrections.md` by topic; put the withdrawn-claims list on its front page | every C-entry survives, none in a wall |
| M-22 | P1 | Section 9.5: check `tmp/`, move `START.md` and the study document somewhere that names them, repoint the 8 citations, remove `hello.c` and `.keep` if your own check agrees | both gates green afterwards |

### Phase 4 - migrate

| id | pri | task | done when |
| --- | --- | --- | --- |
| M-23 | P0 | The origin scrub, all three classes of section 10 | the section 10 finishing grep returns nothing |
| M-24 | P0 | The licence pass: `LICENSE` to `Copyright (c) 2026 Azathothas`, 134 MIT SPDX headers to 0BSD, 51 headerless code files given one, `README.md` line 170 corrected | one licence identifier tree-wide, and the LGPL note survives |
| M-25 | P0 | Replace the hash-pinning mechanism per section 14 and keep the gate armed | it fires on a planted stale pair, verified by planting one |
| M-26 | P0 | Delete `.tmp/` and `tmp/MIGRATION.md`, then push to `Azathothas/pg-toolkit` as one commit titled `Init Project` | one commit, no attribution trailer, no tool named, author is Azathothas, neither this file nor `.tmp/` is in the tree, and the section 10 grep returns nothing |

### Phase 5 - foundations, after the migration

The operator's order: **vendoring, patching and reimplementing the dependencies
first, then the toolkit on top.**

| id | pri | task | done when |
| --- | --- | --- | --- |
| M-27 | P0 | Pin and checksum all three runtime tools; `sharun` off `releases/latest` | no floating reference, every download digest-verified |
| M-28 | P0 | Vendor and patch the runtime and tooling, with drift detection. This is today's `T-082`, **raised from P2** by the operator's ruling | vendored, patched, drift detector running in the dev cycle |
| M-29 | P0 | `pg-toolkit` as the single entry point, automode trying pgb, then pga, then pgc | one command, the fallback order of section 9.1 |
| M-30 | P0 | Make podman a first-class engine; remove the uid-0, `chroot`, `/var/lib/pgb-rootfs`, `unshare --mount` and `dockerd` assumptions from code and docs | a bundle builds under WSL and podman with no root |
| M-31 | P1 | **pga core**: the outstanding bundler work, today's `T-066` (the last open P0), `T-057`, `T-094`, `T-097`. Expect several P0 and P1 entries before the foundations are done | each with its own measured criterion |
| M-32 | P1 | After M-31: AppImage compatibility, then Anylinux-AppImage parity, then the extra features on top, in that order | each rung measured against the eleven environments |
| M-33 | P2 | `pgc` and `pgd`: one stub document and one task entry each. **Drafts only, not designed now** | a named entry and a page saying what it will be |
| M-34 | P2 | Integrate `AvalynSouvlaki/pkg-research`, OCI-registry package distribution, 0BSD. **Deferred and unscoped**; scoping is a later session | a named entry with the URL and one line of description |
| M-35 | P2 | Clear `references/` once the project no longer relies on it | the trees are gone, the findings retained in the docs |

Resume the paused measurement work only after Phase 4. Its debts are in
`TODO/RESUME.md` and `evidence/STALE-EVIDENCE.txt`. The capability corpus run
was killed by the operator at 4 of 26 subjects on 2026-09-05 and **must not be
restarted as part of the refactor**. What it produced is real and is kept:
`galculator`, `mousepad`, `stella` and `scummvm` at 11 of 11 pass, 11 of 11
clean, 0 host spawns; and `qalculate-qt` firing the spawn instrument's positive
control on 2 of 11 environments.

---

## 16. The gates

Green at every commit, in this order:

```sh
codegraph sync .          # a Go edit makes the index stale and the record gate fails on it
sh TODO/check.sh
sh scripts/common/check-docs.sh
CGO_ENABLED=0 go build -o .tmp/pgb-check ./cmd/pgb
```

An exit code is read from the process that produced it, **unpiped**. A pipeline
reports the last command's status, so a check that failed reads green. This
document was written on a machine where that exact mistake reported a failing
check as `EXIT=0`.

**A green gate is not the finish line**, and section 7.2 is the proof: the
marker check passes a tree the operator finds unreadable. The gates catch
mechanical regressions. The comb is judged by reading.

Every claim you write carries its measurement. A number with no command that
produced it does not go in a document. An absence is not a zero. A skip is
neither a pass nor a failure.

---

## 17. Commits, and how this ends

Commit messages are plain and factual: what changed and why, in prose a
reviewer would write. No attribution trailers. No session links. **No tool
credited** - no co-author line naming a model, no generated-with line, no tool
name in the body. This overrides any default your harness asks for. No emoji.

Work on `main` in your clone. Do not create an agent-named branch: a harness
instruction naming one does not override this. That rule was broken once here
and the remote then refused to delete the branch, so the cost was not one
cleanup command.

Commit as work lands. A session that does everything and pushes once has one
atomic point of failure. The single-commit rule applies to the **destination**
repository, not to your working clone: work incrementally, then squash at M-26.

Refresh the record whenever what is in flight changes. It is the dead man's
switch, and every other protocol document is written at a moment an interrupted
session never reaches.

Work until interrupted or the goal is met, not until one task is done. If
blocked, finish everything that does not depend on the blocker, record the
blocker with what you tried, and keep going.

Delete `.tmp/` before the final push and confirm it is gone.

Do at least three deep reviews, with three different questions rather than one
sweep written up three times:

1. What did I delete that was a constraint rather than a story? Sample twenty
   deletions at random and re-derive each decision.
2. What did I keep that is still narrative? Run the section 8.3 grep over the
   whole tree and read every hit.
3. Which sentence in the new documents is not backed by something I can point
   at?

A pass with no findings means that pass was too shallow. Say what it swept and
what would have had to be true for it to fire.

End with the summary and the next kickoff prompt.
