# MIGRATION.md

**The refactor and migration of this repository into `Azathothas/pg-toolkit`.**

You are a fresh agent. This file is the whole brief: it assumes you have read
nothing else in this tree and it tells you what to read, in what order, and
what to do. Everything it claims about the current state was measured on
2026-09-05 by a command printed beside it, so you can re-run any of them and
disagree.

Read this file end to end before you touch a file. It is long because the job
is large; it is not a summary of a plan held somewhere else.

---

## 1. The one-paragraph version

This repository (`polaris0xff/glibc-research`) contains a working toolchain,
`pgb`, that builds ordinary Linux ELF binaries against glibc which run on both
glibc and musl systems. The engineering is sound and measured. The
**presentation is not**: comments carry project history instead of engineering
constraints, documents narrate solved problems, and 478 of 490 commits carry AI
attribution trailers. The operator's judgement is that this makes the project
unreadable to agents and repellent to humans, and both are true. Your job is to
comb the tree, keep every hard-won lesson, delete every piece of narrative, and
land the result in a new repository as a **single commit titled `Init Project`**
with no trace of where it came from.

The measurement work this project exists for is **paused, not cancelled**. Do
not start experiments. The only runs you may make are those the refactor itself
needs to verify.

---

## 2. Read these, then start

In this order. Print the receipt described in section 3 for each.

| # | file | why |
| --- | --- | --- |
| 1 | this file | the mandate |
| 2 | `docs/AGENTS.md` | what the project is, what works, what does not. 844 lines. It is also the single worst offender for the style you are removing, so read it as both content and specimen |
| 3 | `TODO/RESUME.md` | what was in flight when work stopped. Sections "THE PIVOT" and "THE OPERATOR'S FOUR DECISIONS" are the origin of this file |
| 4 | `TODO/INDEX.md` | the 70 task entries, 26 open |
| 5 | `TODO/RULES.md` | how this repository is worked on |
| 6 | `references/Azathothas__TEMPLATE/tree/ROUTE.md` | the template's router |
| 7 | `references/Azathothas__TEMPLATE/tree/docs/conventions/prose.md` | the writing rules you are adopting |
| 8 | `references/Azathothas__TEMPLATE/tree/docs/methodology/history.md` | where deleted narrative goes |
| 9 | `references/Azathothas__TEMPLATE/tree/docs/README.md` | the document set you are rebuilding towards |

`references/Azathothas__TEMPLATE` was vendored on 2026-09-05 at commit
`620616638320147aa2465b304c1240b20eb2d097`. Its `PROVENANCE.md` records the one
gap (discussions, GraphQL-only). Its own `AGENTS.md` was deliberately not
carried in - a file with that name anywhere under this tree is read as
instructions by the tools working in it.

**Do not read `ADOPT.md` as your procedure.** It is written for an agent
working inside somebody else's repository and its safety contract forbids
exactly what you are instructed to do here: it says never delete, never rewrite
history, never commit before a human sees the diff. The operator owns this
repository and has ruled otherwise. Read `ADOPT.md` only for its Phase 0
diagnostic and its Windows notes.

---

## 3. Before you act: the receipt

For each file section 2 names, report its line count and the heading of its last
section:

```sh
wc -l FILE && grep '^#' FILE | tail -1
```

A line count is available from a listing; the last heading is not. Reaching it
means reaching the end of the file, which is the part a skim drops. A receipt
for a file you did not read is a fabricated measurement, and it is worse than
saying you skipped it.

---

## 4. Your environment

Work may happen on a remote Linux container or on a **Windows dev machine with
WSL and podman**. This document assumes the second and does not assume the
first. Probe once, then use what you found:

```powershell
(Get-Command curl -EA SilentlyContinue).CommandType; $env:TEMP
```

```sh
command -v curl wget; printf 'scratch: %s\n' "${TMPDIR:-/tmp}"
```

Three traps, measured by the template on a Windows 11 machine:

- `/tmp` does not exist on Windows. The scratch directory is `$env:TEMP`. A
  POSIX layer's `/tmp` is inside that layer, not a path a native program opens.
- `curl` in Windows PowerShell 5.1 is an **alias for `Invoke-WebRequest`** and
  takes different arguments. Use `curl.exe` by name. A `CommandType` of `Alias`
  means do not use it.
- A native PowerShell session had no `sed`, and `sort` resolved to
  `Sort-Object`. The missing tool fails loudly; the aliased one succeeds and
  returns a different answer, which is worse.

Every check under `references/Azathothas__TEMPLATE/tree/scripts/common/` ships
as a pair, `NAME.sh` and `NAME.ps1`, with the same rules and exit codes. On
Windows run the `.ps1` half with `pwsh -NoProfile -File`.

**Nothing you write may hardcode this project's current container
assumptions**: uid 0, `chroot`, `/var/lib/pgb-rootfs`, `unshare --mount`,
`dockerd`. `docs/AGENTS.md` section 9 currently lists podman as "untested"; it becomes
a first-class target. You are not required to make podman work during the
refactor - that is M-24 - but you are required to stop writing documents and
scripts that assume it away.

**You do not need to bootstrap the build environment to do the refactor.** The
Go tree typechecks with no bed, no rootfs and no docker:

```sh
CGO_ENABLED=0 go build -o /tmp/pgb-check ./cmd/pgb    # verified green 2026-09-05
```

That is your behaviour baseline. It must stay green at every commit.

---

## 5. The destination, and the four hard constraints

| | |
| --- | --- |
| **from** | `polaris0xff/glibc-research` - frozen, archival, do not push the refactor to it |
| **to** | `Azathothas/pg-toolkit` |
| **history** | one commit, titled exactly `Init Project` |
| **licence** | 0BSD, `Copyright (c) 2026 Azathothas` |

### 5.1 The old repository is frozen

Do the work in a full local clone of `polaris0xff/glibc-research`. Clone it
deep - a shallow clone cannot produce the commit corpus of section 7.5, and the
default clone in some harnesses is shallow:

```sh
git clone https://github.com/polaris0xff/glibc-research
cd glibc-research && git rev-list --count HEAD    # expect 491 or more
```

Do not push the refactor to that repository; its history is the archive and the
operator wants it intact. The new repository receives the finished tree.

**This file does not travel.** `MIGRATION.md` names the old repository on every
other page, so it cannot be in the `Init Project` commit without breaking section 5.3.
Delete it at M-20, after its task list has been rewritten into
`TODO/INDEX.md`. It is a migration artefact, not a project document. The same
applies to the commit corpus of section 7.5: keep it outside the tree.

### 5.2 One commit

The final state of `Azathothas/pg-toolkit` is a single commit titled
`Init Project`. If work is needed after the push, **amend that commit** and
force-push. There is never a second commit before the operator says so.

This is stronger than the earlier ruling that said "strip the trailers from all
488 commits". It supersedes it. The consequence is that **every commit message
is discarded** - 17,445 lines of them, containing 1,416 markers and a
substantial amount of reasoning that exists nowhere else. Mining them is not
optional; see section 7.

```sh
git log --format='%B' | wc -l          # 17445
git log --format='%B' | grep -c '⛔\|⭐\|⚠'   # 1416
```

### 5.3 No mention of the origin

The new repository must not name `polaris0xff`, `glibc-research`, or the old
repository's URL anywhere. Measured surface: **155 files**, excluding
`references/`.

```sh
git grep -l -i -e 'polaris0xff' -e 'glibc-research' -- . ':!references' | wc -l
```

| where | files | what the string is |
| --- | --- | --- |
| `evidence/` | 90 | absolute paths recorded in committed measurement output, e.g. `/home/user/glibc-research/evidence/97-timezone` |
| `internal/` | 51 | the Go module path in import lines |
| `cmd/` | 7 | the same |
| `TODO/`, `HISTORY/`, `scripts/`, `go.mod`, `LICENSE` | 6 | module name, copyright line, a hardcoded path in `watchdog.sh`, dead GitHub Actions URLs, and a `claude/*` branch name quoted in `RULES.md` |

Handle them in three different ways, because they are three different things.
Section 9 has the procedure. **Do not run one global find-and-replace across all 154
files** - one of the three classes must be deleted rather than rewritten, and a
blanket substitution silently converts dead links into plausible live ones.

There is a fourth trace worth naming: at least one committed evidence file
contains a harness scratchpad path of the form
`/tmp/claude-0/-home-user-glibc-research/<uuid>/`. Sweep for `claude-0` and for
`/tmp/claude` as well as for the repository name.

### 5.4 0BSD, and it is currently a three-way contradiction

The repository is already 0BSD in intent, so this is a consistency pass. It is
a larger one than the record suggests:

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
are `go:embed`ed into `pgb` and compiled into **every binary the tool
produces**. A user's binary today carries an MIT notice for code the repository
licenses as 0BSD.

**One real obligation survives the pass and must not be lost.** `pgb` links GNU
libiconv, which is **LGPL**, statically into binaries that call `iconv`. The
LGPL relinking obligation attaches to those binaries. This repository does not
redistribute libiconv, and `--no-iconv` produces a binary without it. That
paragraph must appear in the new `README.md` and in whichever docs page covers
the iconv mechanism. Losing it is the one licensing mistake this refactor can
actually make.

---

## 6. The seven problems, measured today

Re-run any row before you trust it. Every command below was run on 2026-09-05
against `main` at commit `b0b915fa`.

| # | the operator's diagnosis | measured now | what "fixed" means |
| --- | --- | --- | --- |
| 1 | comments are history, changelog and lore | Go **4,776 of 25,081 lines (19.0%)**; shell **9,446 of 24,915 (37.9%)**; C **1,794 of 5,215 (34.4%)**. **16,016 comment lines total** | every one read; gotchas kept, narrative mined then deleted |
| 2 | the docs are the same and they misguide | **13,111 lines** across our markdown, plus `HISTORY/` at **16,703 lines in 34 files** | same rule, applied to prose |
| 3 | scripts scattered that should be functions | **77 shell files**: 56 in `experiments/`, 13 in `poc/`, 7 in `scripts/`, 1 in `TODO/` | shared logic lives in one library; the experiments stay shell (section 10) |
| 4 | the codebase only grows | `internal/bundle` **6,781 lines across 15 Go files** (5,473 excluding selftests); `internal/nixx` 4,652 | components separated along the pga/pgb/pgc/pgd boundary |
| 5 | reliance on third parties that are themselves forks | 3 runtime tools; **1 is unpinned and none is checksummed** (section 11) | pinned, checksummed, vendored, then reimplemented |
| 6 | AI attribution and emoji | **478 of 490** commits carry `Co-Authored-By` and `Claude-Session`; **474 authored by `Claude`** | the squash removes all of it; the author field too |
| 7 | references mined then forgotten | **27,914 files, 18 MB, 56 trees - 98% of the repository by file count** | reference study becomes mandatory input to task rewriting (section 12) |

```sh
find . -name '*.go' -not -path './references/*' -not -path './HISTORY/*' -print0 | xargs -0 cat | wc -l
find . -name '*.go' -not -path './references/*' -not -path './HISTORY/*' -print0 | xargs -0 grep -hE '^[[:space:]]*(//|/\*|\*)' | wc -l
git log --format='%an' | sort | uniq -c | sort -rn
git ls-files | grep -c '^references/'
```

### 6.1 What the operator's own numbers missed

The diagnosis named Go at 19%. **Shell is worse: 37.9%, over nearly the same
number of lines.** Shell is half the comb, not a footnote to it. Budget for it.

### 6.2 What the template's checks will and will not catch

Run this before you assume a green gate means the job is done.

Tree-wide marker density is **6.6 per 100 non-blank lines** against the
template's ceiling of **30**. Only **6 of 246** marker-carrying files exceed it:

| file | per 100 |
| --- | --- |
| `TODO/SUMMARY.md` | 55 |
| `docs/research/bundle-capabilities.md` | 47 |
| `TODO/RESUME.md` | 42 |
| `TODO/poc.md` | 37 |
| `docs/research/app-corpus.md` | 34 |
| `TODO/PROGRESS.md` | 32 |

So `check-markers.sh` would **substantially pass this tree today**, and the
operator's complaint would be entirely unaddressed. The check counts density;
it cannot see that a marker is used wrongly, that a human-facing page carries
any marker at all, or that a paragraph is shouting.

Two things the check does catch, and both are real here:

```sh
git ls-files | grep -v '^references/' | xargs grep -o '⛔⛔\|⭐⭐' | wc -l   # 24
```

**24 stacked markers.** `prose.md` forbids them outright: there is no `⛔⛔`.
Escalation is how a marker vocabulary stops meaning anything.

```sh
git ls-files '*.md' | grep -vE '^(references|HISTORY|docs/methodology|MIGRATION)' \
  | xargs grep -oE "\b([A-Z][A-Z0-9'-]{2,} +){2,}[A-Z][A-Z0-9'-]{2,}\b" | wc -l
```

**325 runs across 26 files**, worst in `docs/history/corrections.md` (57),
`docs/research/bundle-capabilities.md` (41), `TODO/RESUME.md` (31) and
`docs/research/app-corpus.md` (28). No check looks for this. It is the single
largest contributor to the "repulsive reaction" the operator described, and you
must remove it by reading. `HISTORY/` is excluded from the command above and
carries its own; comb it too.

### 6.3 The character allowlist, and the trap inside it

`check-markers.sh` enforces an allowlist of exactly five non-ASCII characters:
the three prose markers ⛔ ⭐ ⚠ and the two status glyphs ✅ ❌. Everything else
must be ASCII. Measured across our **174** Go, C and shell files:

| character | files |
| --- | --- |
| U+2014 em dash `-` | **108** |
| U+00A7 section sign `section ` | **62** |
| U+2026 ellipsis `…` | **22** |
| `é ï ü ø × ≥ → · − └ 日 本 中 🙂 😀` | 1-6 each |

`prose.md` bans the em dash by name, so 108 files is the real number here.
`section ` and `…` are ASCII-replaceable throughout.

**The trap: the last row is test data, and deleting it silently weakens the
tests that prove this project's central mechanism.** Every accented character,
every CJK character and both emoji live in Unicode round-trip fixtures:

```
ci/probe.c                  é          the CI probe's iconv arm
experiments/30-gconv-and-locale.sh     é
experiments/78-bundle-cli-bench.sh     é ï 日 本
experiments/88-nonix-end-to-end.sh     é 中
poc/40-jq/run.sh            é ï 😀     jq's Unicode and surrogate-pair round trip
poc/50-python/run.sh        é
internal/logx/logx_selftest.go  ï ø ü 🙂   multibyte log output
internal/bundle/assemble.go     × ≥      and its selftest
```

A blanket "make everything ASCII" pass turns the iconv proof into a test that
proves nothing and still passes. This is failure mode A from section 7.3, applied to
characters instead of comments. Fix the em dashes, the section signs and the
ellipses; leave the fixtures, and add an exemption to the check rather than
editing the data.

This file holds itself to the same rule and measures at **2.4 markers per 100
non-blank lines**, no em dashes and no section signs. The non-ASCII that remains
in it is the fixture table above and the ellipses inside quoted tree comments,
which is the exemption `prose.md` grants a page that states the rule: a document
banning a character cannot otherwise show a reader which character it means.
The `⛔⛔` in the previous paragraph is the same exemption.

Two of the template's checks are already clean on our tree and need no work:
`check-control-bytes` found 6 files, **all in `references/`**;
`check-placeholders` found hits only inside the vendored template's own check
scripts, which describe the pattern they look for. Do not spend time there.

---

## 7. The keep/delete rule

**This is the part the operator is most worried about**, and the worry is
specific: an agent will either wipe comments that took hours to learn, or it
will keep everything and only change the capitalisation. Both outcomes have
happened to this tree's ancestors. Read this section twice.

### 7.1 The test

For each **claim** in a comment or a document, ask:

> If this sentence were gone, would a competent agent reintroduce a defect?

- **Yes** → it is a constraint. **Keep it**, compressed to the constraint.
- **No** → it is history. **Mine it, then delete it.**

The unit of decision is the **claim, not the line and not the block**. Most
comments in this tree fuse both classes into one paragraph, and triage at line
granularity gets them wrong in both directions.

A comment that says what the block does may stay, if it earns its line. A
comment that tells the story of how the block came to be must not.

### 7.2 Four worked examples, verbatim from this tree

**Example A - `internal/bundle/appimage.go:659`. Split; both halves matter.**

> `⚠ It excluded three characters until docs/history/corrections.md C27, and`
> `stopped at neither NUL nor '<'. The selftest carries both cases.`
> `storeRefStop is the ONE definition of where a store reference ends, shared by …`

- **Keep**: `storeRefStop` is the one definition of where a store reference
  ends. Four hand-written character classes had drifted apart; a fifth is the
  obvious thing to write and it is a defect.
- **Delete**: the C27 citation and what it used to exclude.
- **Rewrite to**: *"storeRefStop is the one definition of where a store
  reference ends. Do not write a second character class - four had drifted
  apart. The selftest covers the NUL and `<` terminators."*

Deleting the whole comment because it cites a correction number loses the rule.
Keeping it whole preserves a story about a fixed bug. Both are wrong.

**Example B - `tool/runtime/pgb-storefix.c:18-22`. Keep, and keep the reason.**

> `AND THE OTHER OBVIOUS ROUTE IS REFUSED ON SECURITY GROUNDS BEFORE IT WAS`
> `BUILT: '/nix/store/' and '/tmp/.pgbs/' are both 11 bytes, so a same-length`
> `prefix rewrite inside the ELF needs no relocation - and a fixed, predictable`
> `path under a world-writable directory is squattable by any local user, on a`
> `tree that is loadable code.`

This is the canonical keep. A future agent will independently rediscover the
11-byte coincidence as a clever optimisation and reintroduce a local privilege
escalation. The comment exists to stop that, and the *reason* is the whole
value - a bare "do not use a fixed /tmp path" would be re-litigated within a
session. Compress the caps, drop the "BEFORE IT WAS BUILT", keep the mechanism
and the consequence.

**Example C - `tool/runtime/pgb-storefix.c:9-15`. Delete from code; it is a
docs page.**

> `THE FIELD'S ROUTE IS FIVE OVERLAPPING sed REGEXES ending in "replace any`
> `remaining store path with /" … the difference is the whole entry: THEIRS IS`
> `WRITTEN BY HAND, PER RECIPE. 'pgb' computes the closure …`

Design rationale and a comparison against another project. Genuinely valuable,
and it does not belong in a C file. It moves to the bundler's store-path
document. Nobody changing this C file needs it to change it correctly.

**Example D - `internal/bundle/appimage.go:262`. Keep, unchanged in substance.**

> `AFTER the sweep, for the same reason integrity() is: the manifests have to be`
> `checked against what actually ships, not against what the AppDir held before`
> `anything was deleted.`

An ordering constraint. Moving the call breaks the check silently, and nothing
fails. Six words shorter, same content.

### 7.3 The two failure modes, and how to detect them in your own output

**Failure mode A - the wipe.** You delete a comment because it mentions a
correction number, an experiment number, a task ID or a date, and the
constraint that was fused to it goes with it.

Detection: read your rewritten comment and ask whether it still says **why**.
If it now only says *what*, you have wiped a lesson. Example A is the shape.

**Failure mode B - the bloat.** You keep the comment because it "seems
load-bearing", and change only the capitalisation and the markers.

Detection: grep your own output. A surviving comment must not contain any of
these unless it is naming a live, checked artefact:

```
C[0-9][0-9]   T-[0-9][0-9][0-9]   "used to"   "previously"   "at first"
"the session of"   "was wrong"    "we thought"  "it turned out"
"originally"       "now"          a bare date  an experiment number
```

An experiment number is allowed where it names a **re-runnable** artefact that
still exists and that a reader would run - `experiments/76-` proving the loader
works is a pointer; "`experiments/64-` arm G is why this file exists" is a
story. The difference is whether the reader would go there to *do* something.

### 7.4 Mine before you delete. This is not optional.

Nothing is deleted until its surviving value has a home. Exactly one of:

| destination | for |
| --- | --- |
| **a check or a selftest** | a constraint that can be asserted. Strongly preferred: an assertion cannot go stale silently, a comment can |
| **a docs page** | design rationale, comparisons against other projects, "why not X" |
| **`docs/history/`** | a superseded belief, a reversed decision, a dead end with what it cost |
| **the code** | a named constant, a guard, or a panic message that says the thing the comment said |

The template's rule for `docs/history/` is **append, never edit**, and **moved,
not summarised** - a superseded passage arrives in its original words. A
summary of a retired explanation is a new document about an old one and it
loses the detail that made it worth keeping.

Its front page carries **the list of claims this project has published and
later withdrawn**. `docs/history/corrections.md` already holds ~60 of these
(C1-C60). They are the most valuable prose in the repository and they are
currently one 57-shout wall. Split them by topic, keep every one.

### 7.5 The commit messages are part of the corpus

17,445 lines that vanish at the squash. Extract them before you rewrite
anything:

```sh
git log --format='%H%n%B%n---' > ../commit-corpus.txt
```

Comb it by the same rule. Most is narrative. What is not - a measurement with
its command, a defect and its cause, a decision and its reason - goes to the
same four destinations as section 7.4. **Do this early.** After the squash it is
recoverable only from the archive repository, and after that repository is gone
it is not recoverable at all.

---

## 8. The target shape

### 8.1 The family

`pgb` becomes one member of a family, with `pg-toolkit` as the single entry
point that bundles everything needed.

| | name | what it is | state |
| --- | --- | --- | --- |
| **pga** | Portable GLIBC AppImage | the current nix bundler | in progress; core work outstanding |
| **pgb** | Portable GLIBC Binary | the current static glibc builder | ~95% complete |
| **pgc** | Portable GLIBC Container | packs a tiny container or distro itself, like `runimage` or `flatimage`. Probably what it takes to make genuinely complex applications - podman, docker - portable | draft only |
| **pgd** | Portable GLIBC Distro | a live, relocatable full Linux distro that still behaves like a native AppImage or binary | draft only |

In automode `pg-toolkit` tries **pgb first**, with as much versatility as
possible - prefer the host where it does not interfere, or be completely
standalone, smartly - **then pga, then pgc**. `pgd` should never be needed
except for something like a portable Alpine that beats containers and chroot.

`pgc` and `pgd` are **drafts**: a named task entry and a stub document each,
decided and authored later. Do not design them now.

### 8.2 Where the code is today

| family member | code | lines |
| --- | --- | --- |
| pga | `internal/bundle`, `internal/nixx` | 11,433 |
| pgb | `internal/wrapper`, `internal/buildx`, `internal/verifyx`, `tool/runtime/*.c` | - |
| shared | `internal/cfg`, `logx`, `proc`, `elfx`, `ociimg`, `rootfs`, `envx`, `selftest`, `fail`, `zstd` | - |

The Go module path changes with the repository. That is 60 files of import
lines and `go.mod`; it is mechanical and it is M-16.

### 8.3 The docs tree

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
| `AGENTS.md` | the router, with the task routing table. **Under 300 lines** - today's is 844 |
| a `README.md` under `docs/` | the map: which document answers which question |
| `TODO/PROGRESS.md` | the record, read first by every session |
| `TODO/INDEX.md` | the entry list |
| `TODO/RULES.md` | how the repository is worked on, with what each rule cost |
| `HUMAN.md` | the operator's side: setup, validation, runbooks, prompts. **New** |
| `SECURITY.md` | the threat model. Writing it is the audit. **New** |
| `CHANGELOG.md` | what shipped, when, and where the evidence is. **New** |
| a `README.md` under `docs/history/` | superseded wording, and the withdrawn-claims list |

### 8.4 Human-facing versus agent-facing

| | markers | voice |
| --- | --- | --- |
| **human-facing** - `README.md`, `HUMAN.md`, `CHANGELOG.md`, the public docs tree | **none** | a technical manual. Concise, present tense, exactly what is current and true |
| **agent-facing** - `AGENTS.md`, `TODO/*`, methodology | ⛔ ⭐ ⚠ only, **sparingly**, never stacked | the same voice, plus the stop signs |

Status glyphs ✅ ❌ are a separate, allowed pair, for machine output and result
tables only. A status glyph never carries a rule and a marker never reports a
result. Those five characters are the entire allowlist; nothing else.

`HISTORY/` moves to `docs/history/`. The template is explicit that this belongs
under `docs/`, not at the repository root: a capitalised prose directory beside
the source puts prose and code at the same level. Today's `docs/AGENTS.md` says
`HISTORY/` is never edited - **moving is not editing**, and its contents are
combed by the same rule as everything else. It is 16,703 lines and it holds the
shell and Python predecessor that every byte-identical comparison was made
against. Keep the code; comb the prose around it.

---

## 9. The origin scrub, in three classes

Do not do this with one substitution.

**Class A - mechanical, safe.** The Go module path (`go.mod` plus ~60 files of
imports), the `LICENSE` copyright line, the hardcoded path in
`scripts/common/watchdog.sh:83`. Rename `github.com/polaris0xff/glibc-research`
to `github.com/Azathothas/pg-toolkit` and rebuild.

**Class B - recorded paths inside committed evidence.** 90 files under
`evidence/` contain absolute paths of the form
`/home/user/glibc-research/evidence/…`. These are machine-written measurement
output. You may rewrite **the path prefix and nothing else**. This is a
substitution on a filesystem path, not a re-measurement, and no number, verdict
or exit code may move. Diff the before and after and confirm that only path
strings changed.

**Class C - dead external references. Delete, do not rewrite.** GitHub Actions
run URLs in `HISTORY/entries/ci.md` and `HISTORY/entries/toolchain.md`, the
`api.gh.pkgforge.dev/repos/polaris0xff/glibc-research/...` line in
`TODO/RESUME.md`, the `claude/glibc-research-session-17ku6v` branch name quoted
in `TODO/RULES.md`. These point at a repository the new tree must not name.
Rewriting them to the new owner produces links that 404 and look deliberate,
which is worse than removing the sentence. Where the surrounding rule still
matters - `RULES.md`'s branch-naming rule does - keep the rule and drop the
citation.

Also sweep for `claude-0`, `/tmp/claude`, `anthropic`, and `Co-Authored-By`.
The working tree is nearly clean of these already (4 files, and two of them -
`scripts/common/check-docs.sh` and `scripts/common/mine-repo.sh` - name
`CLAUDE.md` as a **filename pattern to detect**, which is correct and stays).
The pollution is in git metadata, and the squash removes it.

The finishing check, which must return nothing:

```sh
git grep -i -e polaris0xff -e glibc-research -e claude-0 -e anthropic -- . ':!references'
```

`references/` is exempt: those are third-party trees at recorded commits and
they do not mention this project.

---

## 10. What must not change

**Behaviour is frozen, with one exception.** Structure, comments, documents,
layout, module path and CLI naming may change. Mechanisms, flag semantics and
output may not - every existing measurement must stay valid, so the paused
work resumes against the same numbers.

**The exception, on the operator's ruling: trivial easy wins are allowed.**
Anything that would be done much better later - writing our own `sharun`, for
instance - is left alone. If a fix is not obviously trivial, it is a task
entry, not a refactor edit. When you take an easy win, say so in the record;
it invalidates any evidence that measured the old behaviour.

Two things that look like easy wins and are not: `T-097` (the store-path
interposer stops at `execve`; `execvp`, `execl` and `posix_spawn` are not
rewritten, and `libglib-2.0.so.0` imports all three) and the unpinned `sharun`
URL (section 11). Both are real defects with measured controls, both are already task
entries, and both belong to the post-migration foundations.

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
  is a defect (section 7.2 Example A).
- Do not match `.so` as a substring: `/etc/ld.so.cache` is an index, not an
  object. Require `.so` or `.so.N` at the end.
- Do not edit a shell script while it is running. `sh` re-reads from a byte
  offset, executes a garbage line, and runs the tail a second time.

**The experiments and POCs stay shell.** They are the independent acceptance
harness; rewriting them in Go would make the tool its own judge. Problem 3 is
about *shared logic* scattered across 56 experiment scripts, not about the
language. Factor the duplication into `experiments/lib.sh` and
`scripts/common/`; leave the scripts themselves as scripts.

---

## 11. The dependency problem, measured

Three third-party tools are fetched at bundle time. Measured in
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
bundle silently changes when upstream cuts a release, and no gate sees it.
`Anylinux-sharun` is itself a repackaging of another project's `sharun` - the
fork-of-a-fork the operator named as problem 5.

The order the operator set is: **vendor, patch, reimplement - then the toolkit
on top.** Pinning and checksumming all three is the first step and is close to
a trivial win; writing our own is not, and is deliberately later. The plausible
permanent exception is the `mkdwarfs` binaries, and those can be bundled.

---

## 12. References are mandatory reading, not an appendix

`references/` is 27,914 files across 56 upstream trees - 98% of the repository
by file count. Problem 7 is that they were mined, studied and then forgotten,
because by the time an agent reached them its context was exhausted by problems
1 through 4.

**The operator's ruling: everything migrates, and studying the references is a
required input when you rewrite the tasks.** A task entry that touches a
capability some vendored project already solved must say what that project does
and why we do or do not take it. That is what the reference corpus is for and
it is the whole reason it is being carried.

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
task entry (M-27), not something to anticipate now.

---

## 13. The hash-pinning problem, and why repair will not work

`evidence/STALE-EVIDENCE.txt` is a debt ledger. Each active line pins a
**pair** of commits - the commit that last changed an experiment script, and
the commit that recorded its evidence - and `scripts/common/check-docs.sh`
gate 10 fails on a script whose evidence predates it unless the pair is listed.
It found 8 stale pairs on 2026-09-05 that had previously been invisible
(`T-096`).

```sh
grep -vcE '^[[:space:]]*(#|$)' evidence/STALE-EVIDENCE.txt          # 15 lines, 30 hashes
sh scripts/common/check-docs.sh | grep 'pinned stale'               # 13 still match
grep -roE '\b[0-9a-f]{7,40}\b' docs/history/corrections.md | wc -l  # 16 more
```

**Two of the 15 listed lines have already gone inert** - their pair no longer
matches anything, so they are dead exemptions the gate silently carries. That is
the mechanism failing in miniature on an ordinary history, before any rewrite.

**A squash to one commit destroys all 46 hashes, and there is no repair.** With
a single commit there is no pair to pin to: every file in the tree shares one
hash, so the mechanism cannot express "the script is newer than its evidence"
at all. Mechanical rewriting is not available here the way it would be for a
filter-branch.

Replace the mechanism, and do not quietly drop the gate - it is the only thing
that catches evidence predating its own experiment.

The recommended replacement is a **content hash**: record, beside each
`RESULT.txt`, the SHA-256 of its producing script's non-comment body at the
time the evidence was written. Gate 10 recomputes that hash and fails when it
differs. This is strictly better than the commit pair for three reasons: it
survives any history rewrite; it does not fire on a comment-only edit, which
matters enormously in a session whose entire purpose is editing comments; and
it is checkable in a shallow clone.

The 15 currently-pinned debts are carried across as a list of experiment names
with their reasons, which stay valid - the reasons name retired glibc pins and
converted classifiers, not commits. The reasons are the content; the hashes
were only the addressing.

Every hash quoted in `docs/history/corrections.md` becomes an unresolvable
reference. Rewrite those sentences to name the change rather than the commit.

---

## 14. The work, as tasks

These become entries in the new `TODO/INDEX.md`. Counts there are derived and
`TODO/check.sh` fails if they disagree with the rows - do not hand-edit them.

### Phase 0 - orient and freeze

| id | pri | task | done when |
| --- | --- | --- | --- |
| M-01 | P0 | Read section 2, print the receipt, record the baseline numbers of section 6 by re-running them | the numbers are in the record, with the commands |
| M-02 | P0 | Extract the commit corpus to a file outside the tree before anything is rewritten | `commit-corpus.txt` exists, 17,445 lines |
| M-03 | P0 | Run the template's `check-docs`, `check-markers`, `check-control-bytes`, `check-no-secrets` against the tree as it is; record the before-numbers | four exit codes and four counts recorded. Expect failures - that is the output, not an error |

### Phase 1 - adopt the template

| id | pri | task | done when |
| --- | --- | --- | --- |
| M-04 | P0 | Adopt the template's checks and their `.ps1` twins into `scripts/common/`; wire them into the gate alongside the existing `TODO/check.sh` and `check-docs.sh` | `check-gate.sh` runs both sets and reads each exit code unpiped |
| M-05 | P1 | Adopt `docs/conventions/{prose,docs,git,code,shell,forbidden-patterns}.md` | present, and the new writing follows them |
| M-06 | P1 | Adopt `docs/security/{secrets,remote-ops}.md`; write `SECURITY.md` from the template skeleton | `SECURITY.md` states the threat model, with no placeholders left |
| M-07 | P2 | Adopt the ecosystem dotfiles for Go, C/C++, shell; **append**, never replace `.gitignore` and `.gitattributes` | `git check-ignore -v` confirms the new rules bind |

### Phase 2 - comb

| id | pri | task | done when |
| --- | --- | --- | --- |
| M-08 | P0 | Comb every Go comment. **4,776 lines** | each read; kept comments pass the section 7.3 grep; deletions mined |
| M-09 | P0 | Comb every shell comment. **9,446 lines** - the largest single body | as above |
| M-10 | P0 | Comb every C comment. **1,794 lines** | as above |
| M-11 | P0 | Comb the commit corpus from M-02 | every surviving claim has one of the four homes in section 7.4 |
| M-12 | P1 | Remove the 24 stacked markers and the 325 all-caps runs; bring the 6 over-ceiling files under 30 per 100 | `check-markers.sh` green, and the greps return nothing |
| M-12b | P1 | The character allowlist of section 6.3: 108 em dashes, 62 section signs, 22 ellipses. **Exempt the Unicode test fixtures, do not edit them** | `check-markers.sh` green with the fixtures intact and the iconv tests still passing |

### Phase 3 - rebuild the docs

| id | pri | task | done when |
| --- | --- | --- | --- |
| M-13 | P0 | Rebuild `docs/` topic by topic per section 8.3; every leaf routed from a sub-parent, every sub-parent from a parent | `check-docs.sh` reports no orphaned page and no broken link |
| M-14 | P0 | Rewrite `AGENTS.md` as a router **under 300 lines**, and `README.md` as a human-facing front door with no markers | both, and the LGPL-libiconv paragraph is in `README.md` |
| M-15 | P1 | Move `HISTORY/` to `docs/history/`; split `corrections.md` by topic; put the withdrawn-claims list on the front page | every C-entry survives, none in a wall |

### Phase 4 - migrate

| id | pri | task | done when |
| --- | --- | --- | --- |
| M-16 | P0 | Rename the Go module to `github.com/Azathothas/pg-toolkit` across `go.mod` and ~60 files | `CGO_ENABLED=0 go build ./cmd/pgb` green |
| M-17 | P0 | The origin scrub, all three classes of section 9 | the section 9 finishing grep returns nothing |
| M-18 | P0 | The licence pass: `LICENSE` to `Copyright (c) 2026 Azathothas`, 134 MIT SPDX headers to 0BSD, 51 headerless code files given one, `README.md` line 170 corrected | one licence identifier tree-wide, and the LGPL note survives |
| M-19 | P0 | Replace the hash-pinning mechanism per section 13 and keep gate 10 armed | gate 10 fires on a planted stale pair, verified by planting one |
| M-20 | P0 | Delete `MIGRATION.md` (section 5.1), then push to `Azathothas/pg-toolkit` as one commit titled `Init Project`. Confirm with the operator who creates the destination repository and under whose credentials before pushing | one commit, no attribution trailer, no tool named, author is not a model, and the section 9 grep returns nothing |

### Phase 5 - foundations, after the migration

The operator's order: **vendoring, patching and reimplementing the
dependencies first, then the toolkit on top.**

| id | pri | task | done when |
| --- | --- | --- | --- |
| M-21 | P0 | Pin and checksum all three runtime tools; `sharun` off `releases/latest` | no floating reference, every download digest-verified |
| M-22 | P0 | Vendor and patch the runtime and tooling, with drift detection. This is today's `T-082`, **raised from P2** by the operator's ruling | vendored, patched, drift detector running in the dev cycle |
| M-23 | P0 | `pg-toolkit` as the single entry point, with automode trying pgb, then pga, then pgc | one command, the fallback order of section 8.1 |
| M-24 | P0 | Make podman a first-class engine; remove the uid-0/chroot/dockerd assumptions from code and docs | a bundle builds under WSL + podman with no root |
| M-25 | P1 | **pga core**: the outstanding bundler work, today's `T-066` (the last open P0), `T-057`, `T-094`, `T-097`. Expect several P0/P1 entries before the foundations are done | each with its own measured criterion |
| M-26 | P1 | After M-25: AppImage compatibility, then Anylinux-AppImage parity, then the extra features on top - in that order | each rung measured against the eleven environments |
| M-27 | P2 | `pgc` and `pgd`: a stub document and a task entry each. **Drafts only, not designed now** | a named entry and a page saying what it will be |
| M-28 | P2 | Integrate `AvalynSouvlaki/pkg-research` - OCI-registry package distribution, 0BSD. **Deferred and unscoped**; scoping is a later session | a named entry with the URL and one line of description |
| M-29 | P2 | Clear `references/` once the project no longer relies on it | 27,914 files gone, findings retained in the docs |

Resume the paused measurement work only after Phase 4. Its debts are in
`TODO/RESUME.md` and `evidence/STALE-EVIDENCE.txt`. The corpus run was killed
by the operator at 4 of 26 subjects on 2026-09-05 and **must not be restarted
as part of the refactor**. What it produced is real and is kept: `galculator`,
`mousepad`, `stella` and `scummvm` at 11/11 pass, 11/11 clean, 0 host spawns;
and `qalculate-qt` firing the spawn instrument's positive control on 2 of 11
environments.

---

## 15. The gates

Both must be green at every commit, in this order:

```sh
codegraph sync .          # before the record gate; a Go edit makes the index stale
sh TODO/check.sh
sh scripts/common/check-docs.sh
sh scripts/common/check-gate.sh          # once M-04 lands
CGO_ENABLED=0 go build -o /tmp/pgb-check ./cmd/pgb
```

An exit code is read from the process that produced it, **unpiped**. A pipeline
reports the last command's status, so a check that failed reads green.

**A green gate is not the finish line for this job**, and section 6.2 is the proof:
`check-markers.sh` passes a tree the operator finds unreadable. The gates catch
mechanical regressions. The comb is judged by reading.

Every claim you write carries its measurement. A number with no command that
produced it does not go in a document. An absence is not a zero. A skip is
neither a pass nor a failure.

---

## 16. Commits, and how this ends

Commit messages are plain and factual: what changed and why, in prose a
reviewer would write. No attribution trailers. No session links. No tool
credited - no co-author line naming a model, no generated-with line, no tool
name in the body. This overrides any default your harness asks for. No emoji.

Work on `main`. Do not create a `claude/*` branch: a harness instruction naming
one does not override this. That rule was broken once here and the remote
refused to delete the branch afterwards, so the cost was not one cleanup
command.

Commit as work lands. A session that does everything and pushes once has one
atomic point of failure. The single-commit rule applies to the **destination**
repository, not to your working clone: work incrementally, then squash at M-20.

Refresh the record whenever what is in flight changes. It is the dead man's
switch, and every other protocol document is written at a moment an interrupted
session never reaches.

Work until interrupted or the goal is met, not until one task is done. If
blocked, finish everything that does not depend on the blocker, record the
blocker with what you tried, and keep going.

Do at least three deep reviews, with three different questions rather than one
sweep written up three times:

1. What did I delete that was a constraint rather than a story? Sample twenty
   deletions at random and re-derive the decision.
2. What did I keep that is still narrative? Run the section 7.3 grep over the whole
   tree and read every hit.
3. Which sentence in the new documents is not backed by something I can point
   at?

A pass with no findings means that pass was too shallow. Say what it swept and
what would have had to be true for it to fire.

End with the summary and the next kickoff prompt.
