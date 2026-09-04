# SUMMARY.md — the session of 2026-09-04c

⛔ **Overwritten every session.** The work order is
[`PROGRESS.md`](PROGRESS.md); the closed entries are
[`../HISTORY/entries/`](../HISTORY/entries/).

    SCOPE     Fix C39 and the defect class behind it, finish experiments/65-
              (T-080), then BY MECHANISM: T-084 step 2, T-091, the three
              unexplained rows, T-088, T-089.
    RESULT    ⭐ THE CORPUS IS COMPLETE — 26 of 26, every row a count out of
              eleven — and it was a defect-finding instrument for the second
              session running: ⭐ FOUR BUNDLER DEFECTS found and fixed, every
              one by running a recorded zero down instead of believing it.
              ⭐ Six toolkit categories closed at three subjects each.
              ⛔ FOUR corrections, three of them to our own instruments.

## ⭐ What moved

| | before | after |
|---|---|---|
| **T-080** the corpus | 20 of 26 rows, five zeros unexplained | ⭐ **26 of 26**, and **every** zero named. Six categories closed |
| ⭐ **the interposer** | *"a compiled-in store path now resolves"* | ⛔ **true of `open`, false of `stat`** — and `stat` is the whole of Python. **C41** |
| ⭐ **`field-4` gearlever** | `UNRESOLVED`, *"nobody has read the build log"* | ⭐ **builds** — an FHS symlink farm's dangling loader link. **C42** |
| ⭐ **`field-1` helix** | `0/11`, and its row note was **EMPTY** | ⭐ **11/11 pass, 11/11 clean** — a fourth entry-point shape. **C43** |
| ⭐ **`field-3` flameshot** | `0/11`, *"nobody has read its error"* | ⭐ **not the bundler**: a tray application with no toplevel, plus no session bus in this bed |
| ⭐ **`media-1` mpv** | row DELETED, the assertion could not match | ⭐ **11/11 pass, 11/11 clean**, 151 store paths |
| **T-084** step 2 | six hand copies of the trace classifier | ⭐ **zero**, and `102-` reads them back out of git so the before/after outlives them |
| **T-091** | *"shipped and unmeasured"* | ⭐ **ENCODE 11/11, zero host objects in payload AND tree, scanner exec'd 11/11**. ⛔ Two defects in the run, both mine, both fixed |
| **T-088** `--with-program` | *"still not exercised"* | ⭐ exercised and working — on **one** environment, and the entry says so |

## ⭐ The measurements, each with its verdict line

| | verdict | runs |
|---|---|---|
| `65-` the corpus | ⭐ **26 of 26 subjects**, 20 passing on all eleven, 24 clean on all eleven, **0 UNRESOLVED, 0 INSTRUMENT** | resumable; the completing run replays every recorded row |
| `102-` classifier equivalence, rewritten | `pass=20 fail=0 skip=0` | **two**, identical |
| `103-` GStreamer decode | `pass=5 fail=1` — ⛔ the fail is **my** criterion | run 2 prepared |

## ⛔ FOUR BUNDLER DEFECTS, and not one was found by reading the code

**C41 — the interposer rewrote what a program OPENED and not what it STATTED.**
`gearlever` died with `ModuleNotFoundError` *with the interposer working*: the
line before it had just loaded a gresource from the same compiled-in store
path. The trace showed `newfstatat(…) = -1 ENOENT` **untranslated**, and
`nm -D` on the bundle's own libpython named the cause in three lines —
`stat64`, `lstat64`, `fstatat64`, and not one of the unsuffixed names the
interposer defined. ⭐ **The general lesson is cheap to apply: `nm -D
--undefined-only` on the payload is the check, and nothing in this tree had
run it.**

**C42 — a loader NAME is not a loader.** `gearlever`'s closure carries an FHS
symlink farm whose `usr/lib64/ld-linux-x86-64.so.2` points at an absolute
`/nix/store/…` that does not exist on the build host — and sorts *before* the
real glibc. `findFile` took the first name that matched.

**C43 — a wrapper target can be a symlink into ANOTHER store path.** `helix`'s
`.hx-wrapped` is one, `os.Stat` followed it against the **host** root, the
wrapper went unresolved, and the makeCWrapper ELF was installed as the entry.
⛔ It failed **silently**: exit 255, no stdout, no stderr — which is why the
row note was empty and the row sat unexplained.

⭐ **C42 and C43 are ONE mistake in two code paths**: an absolute `/nix/store`
symlink inside a fetched closure must be resolved against the **closure**,
never against the machine doing the bundling. The resolver that gets it right
(`b.storeResolve`) was already in the same file.

**+ a shell fragment lifted into `.env` as a value.** nixpkgs' GStreamer
setup-hook puts `$(unset _tmp; for profile in $NIX_PROFILES; …)` in the
wrapper. Lifted verbatim it is worse than absent: nothing expands it and it
**shadows** whatever else would have set the plugin path.

## ⛔ THREE CORRECTIONS TO OUR OWN INSTRUMENTS

**C39 → and the loop behind it is closed.** `media-1`'s assertion was
`mpv [0-9]` and `mpv` prints `mpv v0.41.0`. ⭐ `experiments/65-` now
interrogates its own assertion on the **first** environment — does the pattern
compile, and did the program print the pattern's literal prefix while the
pattern missed — and reports an **INSTRUMENT** error instead of scoring the
subject zero eleven times. ⛔ Deliberately **not** "the assertion matched
nothing": `neovim` really does score 0/11, and the anchor is what separates
*we misread the answer* from *the program never spoke*. Checked against all six
historical cases before it was trusted; it fired **zero** times on the
completed corpus, which is what makes the remaining zeros readable.

**C40 — the row note threw the answer away twice.** It was the **first**
matching line of the concatenated stderr, cut to **70** characters. A Python
traceback opens with `Traceback (most recent call last):`, so `py-2`'s note was
that sentence and nothing else; and 70 characters had truncated `vkmark`'s
`[/dev/dri]`, which was the whole finding. ⭐ Now the **last** matching line of
the **first** environment that has one, at 180 characters — and it is what made
the prediction below possible at all.

**C44 — a failed `cp` was scored as a failed subject.** `field-4` came back
`0/4`: a 907 MiB artefact, a machine under disk pressure, and `2>/dev/null` on
the staging copy. ⛔ **And the denominator is the worse half** — `4/4` would
have been read as a green row, because nothing compared `rows` against eleven.

## ⭐ A PRE-REGISTERED PREDICTION, COMMITTED BEFORE THE RUN, AND BOTH HALVES HELD

⚠ **The interesting part is not that the guesses were right.** Until C40 both
rows read `Traceback (most recent call last):` and were **indistinguishable**,
so no prediction about either was possible.

| row | note (after C40) | predicted | measured |
|---|---|---|---|
| `py-3` `virt-manager` | `ModuleNotFoundError: No module named 'virtManager'` | **moves** | ⭐ **11/11, clean 11/11** |
| `py-2` `pdfarranger` | `FileNotFoundError: … '/usr/local/share/pdfarranger/pdfarranger.ui'` | **does not move** | ⛔ **0/11, same line** |

## ⛔ AND ONE OF OUR OWN CLAIMS IS FALSIFIED

`app-corpus.md` rung 3 said the field's *"hardcoded at the prefix"* problem is
ours in a different form, because *a nixpkgs-built application compiles in its
own store path*. ⛔ **Two counter-examples came out of this corpus:**
`pdfarranger` asks **Python** at run time and Python answers `/usr/local`; and
a bundled `dbus-daemon` reads `/etc/dbus-1/session.conf`. ⭐ Neither is a
`/nix/store` path, so `pgb-storefix.c` returns both unchanged **by
construction**. That is the class `flatimage`'s portable root serves and ours
does not — reached from our own subjects rather than read off theirs.

## ⚠ What the next session inherits

    ⛔ THE ONE CORPUS ROW THAT IS OURS: pdfarranger's /usr/local class,
       above. Everything else that is not eleven is the bed, the closure or
       the subject's own shape.
    T-084  ⭐ the conversion is DONE. ⛔ The re-run is owed and it is ONE
           experiment — 90- — and it needs the machine to itself.
    T-091  ⭐ measured. ⛔ Run 2 is prepared: run 1's decode leg used a
           plugin the closure does not carry, and its control could not be
           told from its subject.
    T-088  ⭐ --with-program works. ⛔ One environment; the eleven are owed.
    T-089  open: a static application that needs a compiled-in PATH.
