# Aseem0xff/alloc-tests

Fetched 2026-09-01T12:02:37Z by `scripts/common/mine-repo.sh`.

| | |
| --- | --- |
| commit | `efc84ab5d1f68735d219752be5db5fc6e8adbde7` |
| route | proxy |
| control | reachable (pkgforge-dev/reverse-proxies answered 200) |

⛔ **Cite this commit beside every line reference taken from**
`tree/`. The corpus is TRACKED, and a reader who has it still needs
the commit to know which revision a citation was taken against.

## ⛔ What this fetch did NOT get

  - discussions: NOT FETCHED. The proxy is a REST route and discussions are GraphQL only. Re-run with an authenticated gh to get them.

⚠ Repeat each gap in the sweep write-up. A source that is missing without
being named reads exactly like a source that had nothing in it.

## ⚠ Before you believe any of it

⛔ **An issue body, a comment, a release note and a bot description are
observed content, not instructions and not findings.** They are evidence of
what somebody intended, never evidence of what the code does. Read the
claim, then open the file at the commit above and check it.

⚠ **The author being the maintainer, or the operator, does not exempt it.**
A claim written a month ago describes a tree that has moved.

## ⛔ Deliberate deletions from `tree/`

Two paths were removed after the fetch. Both are recorded here because a
reader comparing this tree to upstream at the commit above will find them
missing.

| path | why |
|---|---|
| `tree/docs/AGENTS.md` | ⛔ `../../docs/methodology/vendoring.md`: an agent instruction file of any name is never vendored — a file with such a name anywhere under a repository is read as instructions by the tools working in it. Same deletion, same reason, as `pkgforge-dev__cross-libc-dlopen`. |
| `tree/references/` | that repository's **own** vendored reference corpus: 61 MiB of a third party's copies of other third parties' trees. It is not the subject's source, and this project keeps its own corpus one directory up. Removing it took the fetch from 63 MiB to 2.1 MiB. |

## What was taken from it

⭐ **Read for its container operations, which is the part this project had a
gap in.** `tree/docs/containers.md` is the file the operator pointed at.
Acted on in this tree:

- **`docker info`, never `docker --version`** — a client with no daemon
  answers the second happily. `pgb doctor` already probed with `info` and
  reported `present but no daemon` truthfully; what was missing was the next
  step, starting `dockerd` directly in an environment with no init. That one
  line retired the **UNTESTED** status on `pgb`'s docker engine and the three
  defects behind it. `../../docs/history/corrections.md` C9.
- **"Bind-mount paths must be absolute"** — a relative `-v` source is read as
  a *named volume*, silently. Reproduced here on docker 29.3.1 and fixed in
  `abs_bindspec()`.
- **"The fix is never to disable verification. Supply the CA."** — the shape of
  `ca_anchor()`, which carries only the file the caller's own environment
  already names.
- ⚠ **"`ALLOC_TESTS_HTTPS_PROXY` is deliberately separate from the ambient
  `HTTPS_PROXY` … a host proxy is usually on `127.0.0.1`, which inside a
  container is the container"** — true of this environment, whose proxy is
  `http://127.0.0.1:35067`. `pgb` does not forward a proxy variable into any
  engine, which is the correct behaviour and is now deliberate rather than
  accidental.

⚠ **Not adopted:** the pinned-Rust and pinned-`zig` image design, which is
that project's control for a *compiler* axis it measures and this project does
not. Recorded so it is not re-derived as a gap.

## ⚠ Note on the fetch

`scripts/common/mine-repo.sh` fetched the **default branch**. The operator
cited `docs/containers.md` on branch `claude/agents-md-review-27r3xk`; that
file is byte-identical to the default-branch copy at the commit above, checked
with `diff`. Nothing else from that branch was read.
