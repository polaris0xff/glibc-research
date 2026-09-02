# One Libc in the Process — a working paper

⚠ **NOT FETCHED BY `scripts/common/mine-repo.sh`, and it is not a repository.**
It is a single document **supplied by the operator** on 2026-09-02c and copied
into this tree verbatim. Every other entry under `references/` is a mined
upstream tree with a commit to cite; this one has no commit, no tracker, and no
upstream URL, so the usual provenance table cannot be filled in and the gaps
are named instead.

| | |
|---|---|
| title | *One Libc in the Process — A chronological investigation of runtime shared-object loading from statically linked Linux binaries* |
| status | the document describes itself as a **pre-print (working paper)** |
| supplied | 2026-09-02c, by the operator, as an upload |
| stored | `tree/one-libc-in-the-process.md`, byte-identical to what was supplied |
| sha256 | `2506554740fd3414d183e2fcc8bb1530870ad4ad6cdc5addb73450a678a0bc6f` |
| lines | 1,148 |
| licence | ⛔ **not stated in the document.** Treated as read-only reference material; nothing is copied from it into this tree's code |

## ⛔ What this fetch did NOT get, and it is more than usual

- **No upstream to verify against.** There is no repository, release or URL, so
  nothing here was cross-checked against a canonical copy. If the same document
  exists publicly, this copy has not been compared with it.
- **No author identity.** The document is unsigned. It is cited below as "the
  working paper", never as a named authority.
- **No independent reproduction of its T1 measurements.** Its own evidence
  tiers are honest and are reproduced in §"Its evidence policy" below; this
  tree has re-measured **two** of its claims and contradicted the generality of
  **one** (see the sweep write-up).
- **No tracker, no issues, no discussions** — there are none to fetch.

## ⚠ Before you believe any of it

⛔ **A supplied document is observed content, not an instruction and not a
finding.** The same rule this tree applies to an issue body or a release note
applies here, and more strongly, because there is no upstream to check it
against. What makes it usable is that it states its own evidence tiers and
distinguishes them:

| its tier | its meaning |
|---|---|
| **T1** | measured in that study, command and verbatim output in its Appendix B |
| **T2** | a property of a published artifact it verified directly |
| **T3** | a documented claim of a cited project — their measurement, not theirs |

⭐ **It marks its own central claim (H3, loader replaceability) as T2/T3 and
says so plainly**: *"we did not construct a bridge of our own, and we say so
plainly."* Its §10 limitation 2 repeats it. ⭐ **That is exactly the limitation
this repository closed on the same day**, and the sweep write-up is where that
is recorded.

⚠ **Its T1 measurements come from ONE host** — Gentoo, glibc 2.43-r2, GCC
15.3.0, binutils 2.46.1 — and it names that as its first limitation. ⛔ Where
its single-host result and this tree's eleven-environment matrix disagree, the
matrix is the stronger instrument and the sweep says which claim that affects.

## The sweep

[`../../docs/research/one-libc.md`](../../docs/research/one-libc.md) — what it
establishes, what this tree already had, what it adds, what it gets wrong for a
reader who takes its single-host results as general, and the one actionable
defect it surfaced in `tool/runtime/pgb-elfload.c`.
