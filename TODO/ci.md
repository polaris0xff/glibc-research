# ci

⚠ **Open entries only.** T-040, which ran CI for the first time, is
[`../HISTORY/entries/ci.md`](../HISTORY/entries/ci.md); the long-form detail
behind the entry below is
[`../HISTORY/entries/ci-open.md`](../HISTORY/entries/ci-open.md).

---

## T-041 — aarch64

**Source** [`../docs/AGENTS.md`](../docs/AGENTS.md) §13 · **Category** ci · **Priority** P2 · **Effort** M · **Status** open

**Problem.** Every number in this repository is x86_64, one machine, one day.
`--arch arm64` exists in `pgb rootfs pull` and `pgb rootfs fetch` and
re-resolves by tag, trading the digest pin away.

**Premise.** ⚠ Expect IFUNC and CPU-baseline questions x86_64 did not raise.
`experiments/61-` shows glibc's advantage is largely IFUNC-dispatched
routines, so the throughput result may not carry.

**Prove.** `experiments/61-` and `62-` run on an aarch64 runner with their
tables filled.

📚 [detail](../HISTORY/entries/ci-open.md)

## T-077 — ⛔ the head-to-head was measured on the RETIRED pin, and nobody re-ran it

**Source** ⭐ **deep review 6, 2026-09-03c**, by asking of `docs/AGENTS.md` §9's
*"all 32 experiments · every one measured"* the same question that found T-076:
measured **by which version of itself?**
**Category** ci · **Priority** P1 · **Effort** M · **Status** open

**Problem.** Seven experiments had committed evidence older than the last
non-comment change to their own script. Four of them changed in the same way,
and it is not cosmetic:

    -ENV_ROOT="$ROOTFS_DIR/${PGB_ENV_NAME:-pgb-env-debian12}"
    +# ENV_ROOT comes from lib.sh, out of internal/cfg/cfg.go. T-070.

⛔ **So the committed numbers were measured inside `pgb-env-debian12` —
glibc 2.36 — and the script now measures `pgb-env-debian13`, glibc 2.41.**
The build environment the whole experiment runs in changed underneath its
evidence.

⛔ **And 60-, 61- and 62- ARE THE HEAD-TO-HEAD.** `docs/comparison.md` and
`docs/REQUIREMENTS.md`'s table — artefact size, malloc throughput, the
eleven-row coverage against AppImage, Flatpak, snap, onelf and static musl —
all come from them.

⚠ **This is not a claim that the numbers are wrong.** T-070 measured the pin
move and found its four named costs at zero, so they may be unchanged. It is a
claim that **nobody has checked**, and that the record read as though somebody
had.

⭐ **Three of the seven were re-run on 2026-09-03c and are now current** —
`30-` (pass=11 fail=0 skip=1), `70-` (pass=1 fail=0 skip=2, and its table now
says `pgb-env-debian13` where it said `pgb-env-debian12`) and `80-` (pass=16
fail=0). ⛔ **Four are pinned in [`../evidence/STALE-EVIDENCE.txt`](../evidence/STALE-EVIDENCE.txt)**
because between them they build five delivery formats and run two benchmark
matrices: hours, and the clock rows need the machine to themselves.

**What is left.** Re-run `60-`, `61-`, `62-` and `88-` on the current pin, on
an idle machine, and delete their lines from the ledger. ⚠ **Compare, do not
overwrite blindly** — `corrections.md` C23 is what happens when a re-run
silently replaces the numbers an entry quotes.

⭐ **The class is gated now**, which is the durable half:
`scripts/common/check-docs.sh` **gate 10** fails when an experiment's committed
evidence predates a non-comment change to its script. Comment-only edits do not
count, and an exemption is pinned to **both** commits so it cannot outlive its
reason.

**Prove.** `evidence/STALE-EVIDENCE.txt` is empty of entries, and gate 10
reports `0 pinned stale`.


---

## T-096 — ⛔ gate 10 keyed on the evidence DIRECTORY, so eight stale pairs were invisible

**Source** deep review 1, 2026-09-05, after silencing the gate by accident.
**Category** ci · **Priority** P1 · **Effort** S · **Status** open

⛔ **`check-docs.sh` gate 10 exists to catch a committed `RESULT.txt` that
describes an older instrument.** It compared the script's last commit against

```sh
ec=$(git log -1 --format=%H -- "$d")      # $d = evidence/<name>
```

⭐ **`git log -1 -- <directory>` takes the newest commit touching ANY file
under it.** So committing a README, a note, or a new sub-store *beside* the
results refreshes the date the gate compares against, and the gate goes quiet
over a result that has not been re-run.

⚠ **It is not hypothetical, and it was not found by reading.** It was done
here by accident: `evidence/65-capability-corpus/spawns/README.md` was added
in the same commit as a change to `65-` itself, and gate 10 — read carefully
an hour earlier — reported green.

## ⭐ MEASURED ACROSS THE TREE

**Eight** evidence directories had their newest commit on a **non-RESULT**
file. Re-keyed on `RESULT*`, the gate reports **8 disagreements where it
reported 0**:

| | why it now fires |
|---|---|
| `10-`, `20-`, `40-`, `50-` | ⭐ **NEW, and they are one change**: the shell-to-Go port. Each script moved from `sh scripts/common/rootfs-run.sh` to `pgb rootfs run` (and `50-`/`40-` from `sh "$REPO_DIR/pgb"` to `"$REPO_DIR/pgb"`), all at commit `4ef2acc7`. ⛔ Their committed numbers were produced by the **shell predecessor** while the scripts now drive the **Go tool** |
| `60-`, `62-`, `85-` | already pinned — their pins named the **directory** commit and stop matching once the key is the result. Mechanical; the reasons are unchanged |
| `65-` | its re-run was in flight when the key changed |

⚠ **The four new ones may well be unchanged**: `HISTORY/` is kept precisely
because it is the oracle the port was measured against, and `docs/AGENTS.md`
§9 records the docker engine's output as byte-identical. ⛔ **But nobody has
re-run them**, and that is the sentence this ledger exists to force.

## ⭐ THE FIX, SHIPPED

The key is `"$d/RESULT"*`, falling back to the directory when an evidence
directory carries no result file at all. The "being re-run in this very
commit" escape is narrowed the same way — an uncommitted README beside the
results is not a re-run and must not read like one.

**What is left.** Re-run `10-`, `20-` and `50-` (they need the bed) and `40-`
(it needs the **machine** — 400 execs × 7 rounds), then delete their lines
from [`../evidence/STALE-EVIDENCE.txt`](../evidence/STALE-EVIDENCE.txt).
⚠ **Compare, do not overwrite blindly** — `corrections.md` C23.

**Prove.** Gate 10 green with those four lines gone, and each `RESULT.txt`
naming a run newer than `4ef2acc7`.
