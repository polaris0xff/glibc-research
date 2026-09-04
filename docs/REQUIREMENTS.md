# Requirements — the operator's acceptance bar

⛔ **This page is binding and it is not a summary of the project's state.** It
records what the operator requires, verbatim, and tracks — separately, below —
how much of it has been discharged. The requirement text is **not** to be
softened, deleted, or marked satisfied by any agent; only the *status* section
moves, and only when a measurement moves it.

---

# ⛔ HARD REQUIREMENT (operator, binding, NOT yet met)

> **pgb must produce a binary that works _everywhere_ — or, failing that, one
> that is strictly better and/or faster than every existing format and
> technique.**

## ⭐ AMENDMENT — part 2 replaced (operator ruling)

> *"replace with per part claim, also anylinux is a bundle, our primary goal
> is still a static glibc binary that has none of the issues"*

⛔ **Part 2 is no longer a comparison against bundles.** anylinux is a bundle —
it mounts or extracts a small distribution — and `pgb` is a toolchain whose
output is one ordinary ELF; scoring them on one axis asks the wrong question
([`design/toolchain.md`](design/toolchain.md)). Part 2 is now *a static glibc
binary that has none of the issues*, and the issues are enumerated below.
⚠ **Part 1 is UNCHANGED and now carries the whole bar.** The head-to-head
numbers stay in [`comparison.md`](comparison.md) as measurement, not as a test.

## ⭐ AMENDMENT — the BUNDLER's bar (operator ruling)

⚠ **Scoped: it changes the bundler's bar and neither part of the requirement
above.** The axes table it rewrites is in
[`design/toolchain.md`](design/toolchain.md) "Static first, bundle last".

> *"us having a bigger size than anylinux-appimages and onelf is acceptable as
> long as ours performs better and packaging is just one command not a
> multiline shell script"*

⭐ **Two conditions, conjunctive: perform better, and package in one command.**
Size is struck. ⛔ **`pgb`'s bundler meets the second and fails the first** —
one command from a package name against five separately versioned binaries
plus a 121 KB driver script, but **2.07× the cold start on `jq`** measured over
eleven environments at a mean of five samples each, and slower on kdenlive on
every one of four runs. ⚠ The size ratios this page quotes in the head-to-head
below (**2.1 MB vs 3.7 MB**) are about the **static binary**, not the bundle,
and are untouched by the ruling.

This is the project's acceptance bar. It is **not** met today: the current
class is "programs that do not need to load host plugins"
([`AGENTS.md`](AGENTS.md) §7), which is broad but not everything.

**How to hold this bar without lying about it.** "Everywhere" cannot be
verified — no matrix enumerates Linux, the kernel is never abstracted, and the
CPU baseline is a build-time choice. So the directive is discharged in two
parts, and **both** are required:

1. **No known environment where it fails.** Every failure found is either
   fixed or written into [`limitations.md`](limitations.md) with the
   measurement. The matrix grows over time ([`AGENTS.md`](AGENTS.md) §13); a
   failure that is known and unfixed means the bar is not met, and the status
   must say so.
2. ⭐ **A static glibc binary with none of the issues** — the per-part claim,
   as amended above. `gcc -static` against glibc is not self-contained, and
   the ways it is not are **enumerable**. Discharging this means every one of
   them is closed, on the matrix, with the measurement:

   | | issue | state |
   |---|---|---|
   | NSS | host modules dlopen'd, second libc in the process | ✅ **closed** — `__nss_configure_lookup`, 11/11 |
   | gconv / iconv | 11 of 12 encodings unavailable, SIGFPE on 3 | ✅ **closed** — `--wrap` onto static libiconv, 11/11 |
   | locale | `ANSI_X3.4-1968` on every musl host | ✅ **closed**, opt-in `--embed-locale`, 11/11 — and ⭐ the *environment-default* codeset (an unset `LANG`, where musl beat both glibc columns 11-0) closed separately 2026-09-04 by opt-in `--utf8-default`, 11/11, with an explicit `LANG=C` still obeyed on 11/11 (`experiments/67-`) |
   | networking / DNS | `getaddrinfo` via host NSS | ✅ **closed** — POC 30 resolves and does real TLS on 11/11 |
   | own plugins | a program's own `dlopen` needs the host loader | ✅ **closed** — `--wrap-dlopen`, 11/11 |
   | C++ unwinding | no `PT_GNU_EH_FRAME` on any static link | ✅ **closed** — T-018 |
   | CA bundle | no compiled-in trust store; one path works on 5 of 11 | ✅ **closed** — opt-in `--embed-cacert`; POC 30 verifies real TLS on **11/11** with the harness's own CA variables unset. T-032 |
   | terminfo | host terminal database | ✅ **closed** — opt-in `--embed-terminfo`; POC 20's `setupterm(xterm-256color)` succeeds on **11/11** with `TERMINFO`/`TERMINFO_DIRS` unset. T-032 |
   | **host plugins** | `dlopen` of a host `.so` is host-dependent | ⛔ **open, and now SERVED BY A SHIPPED MECHANISM rather than untouched** — `pgb build --host-dlopen`, T-064 ✅, T-068 ✅. A `.so` built by the pinned glibc loads on **11 of 11** with zero host objects; a **real host** `.so` loads on **7 of 7 glibc rows** and is refused by name on **4 of 4 musl rows**; **882 of 1,527** host objects on the build host load. ⛔ It stays OPEN because the row says *host-dependent* and it still is |
   | ⭐ **timezone** | `tzset` reads the host's zone database; nothing is linked in | ✅ **closed** — opt-in `--embed-tzdata`; `experiments/97-` runs two arms, pass=13 fail=0. Arm A (plain `-static`) resolves `Europe/Berlin` on **7 of 11** and ⛔ **4 of 11 cannot and do not say so**, printing `Europe +0000` — the zone name **asked for**, at a UTC offset. Arm B resolves on **11 of 11** for **193,208 B** of carried zones. ⚠ A handful (20), not a database; a zone not carried is unchanged. T-076 |
   | ⭐ **network name databases** | `/etc/services`, `/etc/protocols`: `getservbyname`/`getprotobyname` read a host file that a static link does not absorb | ✅ **closed** — opt-in `--embed-netdb`, `experiments/66-` (`pass=12 fail=0 skip=0`, two runs identical). Found by the search T-079 asked for. Without it, `getservbyname("http","tcp")` returns **NULL on 3 of 11 — debian-11, debian-12, ubuntu-20.04, and ALL THREE ARE GLIBC**; all four musl environments ship the file. With it, **11 of 11**, and ⭐ **the host still wins where it has the file** (the wrappers call `__real_*` first) while a name nobody carries still answers NULL on 11 of 11. ⚠ **Not a restatement of NSS**: NSS is closed for *dispatch* — `__nss_configure_lookup` pins the `services` database to `files` — and that cannot conjure a `files` backing store the host does not have; dispatch and data were two failures and this is the second. ⛔ **ONE BOUNDARY STAYS OPEN, pre-registered as a failure before the run**: `getaddrinfo(NULL,"http")` is still **8 of 11**, because `--wrap` redirects the *public* symbol and glibc's `getaddrinfo` calls its own internal `__getservbyname_r`, which the linker cannot interpose |

   ## ⛔ THE LIST GREW NINE → TEN → ELEVEN, EACH TIME BY SEARCHING

   ⭐ **`experiments/82-` is that search and it is re-runnable.** It enumerates
   every absolute path the **pinned** `libc.a` names (78 at glibc 2.41),
   classifies each against the rows above, and prints the residue no row owns
   (19). `/etc/services` was in that residue.

   ⛔ **AND THE SEARCH SAYS WHERE IT DID NOT LOOK**, because an absence is not
   a zero. It sees absolute-path string literals and environment-variable names
   in `libc.a`. It does **not** see paths assembled at run time from `%s/%s`
   and a variable, host data belonging to **other** static libraries — ⚠ two of
   the closed rows, terminfo (ncurses) and the CA bundle (OpenSSL), are
   invisible to it **by construction** — or anything reached through a host
   daemon rather than a file.

   ⚠ **A second finding from the same run, and it is NOT a new row**:
   `gethostid()` dies with **SIGFPE on Arch**. With no `/etc/hostid` — **0 of
   11** have one — glibc falls back to resolving the machine's own hostname,
   which is an NSS `hosts` lookup. That is row 1 (NSS) reached through a
   function nothing in this tree had ever called; what was under-described was
   its **reach**, not the row. `pgb` fixes it: `experiments/63-` has
   `hostid` answering on 11 of 11 under `pgb` and crashing under vanilla.

   ## ⛔ TREAT THE ELEVEN AS THE CURRENT BEST ENUMERATION, NOT A CLOSED SET

   ⚠ **This page once said *"there is no unenumerated remainder"* about nine
   rows. That sentence was FALSE and load-bearing** — it is what made part 2
   countable. Two more rows have been found since, each by looking rather than
   by reasoning, and neither search was expensive.

   ⭐ **The right response is to keep attacking it.** The next candidates worth
   one measurement each: `/etc/nsswitch.conf` policy beyond what NSS covers,
   and `libgcc_s.so.1` for `pthread_cancel` and `backtrace` (⚠ **0 mentions in
   the build host's `libc.a`** at glibc 2.39 — likely already dead, but not
   measured on the **pinned** 2.41). ⚠ Also untested and the same shape as the
   eleventh: `/etc/networks`, `/etc/ethers`, `/etc/rpc`, and the `*ent`
   iteration family (`setservent`/`getservent`/`endservent`), which
   `--embed-netdb` does not wrap.

⭐ **The honest public claim, until then**, is the falsifiable one `pgb verify`
already emits: *built at tier N, ran correctly and loaded no host object on
these N named environments, and here is the command that re-checks it on
yours.* Use that in the README and anywhere else a claim is made. Do not write
"universal" or "works everywhere" into any document as a statement of fact
until part 1 and part 2 above are both discharged.

**Work this implies, in addition to [`AGENTS.md`](AGENTS.md) §13:** the tier
plan in [`design/tiers.md`](design/tiers.md) is the route to part 1 (it is what
brings the host-plugin class in scope), and a new head-to-head benchmark
experiment — `experiments/60-versus-alternatives.sh` — is part 2.

---

## Status against the bar

⛔ **The bar is NOT met**, and both parts are now not-met as measured results
rather than as unmeasured gaps.

| part | state | why |
|---|---|---|
| **1. No known environment where it fails** | ⛔ **not met** | One measured failure, and it is now the **only** one: `dlopen` of a **host** shared object ([`limitations.md`](limitations.md) §1). ⭐ **Route D was TAKEN and it SHIPPED** — `pgb build --host-dlopen`, an ELF loader compiled in (**T-064 ✅**, residue **T-068 ✅**), 11 of 11 carried with zero host objects and a real host `.so` on 7 of 7 glibc rows. ⛔ It remains not-met because the failure is *host-dependence*, and that persists: the four musl rows refuse a host object by design, and 645 of 1,527 host objects on the build host do not load (**374 of which glibc's own `ld.so` also fails** — plugins of a host program). |
| **2. A static glibc binary with none of the issues** | ⛔ **not met** | ⭐ **TEN of ELEVEN** enumerated issues are closed on all eleven environments; **one** is open, host plugins. ⛔ **The list grew nine → ten → eleven, each time because somebody searched** — so read "ten of eleven" as a count of the rows *found so far*, not of the rows that exist. ⚠ **And closing the eleventh revealed a boundary inside it**: `getaddrinfo` with a service name stays at 8 of 11, pre-registered as a failure before the run. The distance to the bar is ONE named problem plus that boundary. |

### The head-to-head, which is now evidence rather than the test

⚠ **Read this as background, not as the acceptance criterion.** The operator's
ruling of 2026-09-01b replaced part 2, on the ground that *anylinux is a
bundle* and the goal is a static glibc binary with none of the issues. What
follows is the measurement that was taken while part 2 was a comparison; it
stands as a result and no longer decides anything.

The comparison the original directive asked for exists: AppImage, Flatpak,
snap, onelf and static musl were all built, plus the two controls, and the
AppImage arm was then rebuilt against `Anylinux-AppImages` because the vanilla
one is not the competitor. [`comparison.md`](comparison.md) carries the table.

| | `pgb` | anylinux AppImage | static musl |
|---|---|---|---|
| ran correctly, 11 environments | 11 / 11 | 11 / 11 | 11 / 11 |
| loaded zero host objects in the payload | 11 / 11 | 11 / 11 | 11 / 11 |
| malloc, 4 threads, on Alpine | **4.3 ns** | **3.7 ns** | 592 ns |
| artefact size | **2.1 MB** | 3.7 MB | 447 KB |
| target does nothing but execute it | ✅ | ⛔ mounts or extracts | ✅ |
| serves a large dynamic dependency graph today | ⛔ not yet | ✅ | ⛔ |

What it says: `pgb` is not beaten on portability or on speed by anything
measured. It is **tied** with the anylinux AppImage on both, ahead of it on
artefact size and on shape — one ELF, nothing mounted, nothing written — and
**behind it on reach**: bundling serves software with a dependency graph
static linking has not yet been pushed hard enough to absorb. ⭐ That last one
is work, not a verdict — `AGENTS.md` §7 now has **four** routes, and
`experiments/73-` measures the newest one's demand at 90.8%–99.3% already met.

⭐ **What the evidence does support**, and it is a real claim:

> Built at tier 1, ran correctly and loaded no host shared object on these 11
> named environments, at glibc's throughput including on musl hosts, as one
> ordinary ELF that mounts nothing and writes nothing — for programs that can
> be statically linked today.

**What would move part 2 to met**, under the amended text: the open rows of the
issues table close, each on all eleven environments with the measurement
recorded. **One row is left: host plugins**, owned by T-064 (the mechanism,
closed) with its residue in T-068 (closed).

⭐ **THE OPERATOR ASKED WHETHER IT WAS ALREADY DONE — *"GLIBC static is truly
complete, no edgecases exist ... No buts and no ifs"* — AND THE ANSWER IS
STILL NO.** T-079 answered it by SEARCHING and found an eleventh row;
`--embed-netdb` then closed that row. ⛔ The answer is still no, for two
reasons that outlive it: the mechanism is a **flag a developer has to pass**,
and `getaddrinfo` with a service name stays at **8 of 11** behind it.

⛔ **TEN of ELEVEN are closed and ONE is not, so this is a countable deficit and
not a judgement.** Do not soften it: host plugins is the hardest of the eleven,
and being last does not make it small. ⭐ **The eleventh — the network name
databases — went from "no mechanism at all" to 11 of 11 in a day**
(`--embed-netdb`, `experiments/66-`), and closing it exposed a boundary that is
still open: `getaddrinfo` with a service name, 8 of 11, which `--wrap` cannot
reach. ⚠ And ⛔ **the sentence that used to sit here — *"nothing says an
eleventh is not"* — was right within a day.** The list has grown on three
consecutive days. Read "ten of eleven" as a snapshot of a search, never as a
distance to done.

### What is still unmeasured, and is not counted either way

- **Flatpak and snap at run time.** Both artefacts were built here; neither
  could be executed on this machine (`flatpak run` needs a D-Bus session bus
  and `dbus-daemon` cannot raise its fd limit with `cap_sys_resource` dropped;
  `snapd` needs systemd). ⚠ This does not affect their 0/11 coverage, which is
  decided by the targets, but their startup and memory cells stay dashes.
- **onelf in its preferred modes.** The chroot bed denies the user-namespace
  calls its memfd, FUSE and tmpfs modes need, so every onelf row is its last
  fallback. Its coverage result is unaffected — the failures are gconv, not
  delivery — but its startup figure is a worst case.
