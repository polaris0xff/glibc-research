# lib.sh - shared by every numbered experiment. Sourced, never run.
#
# Two jobs, and both come straight from docs/methodology/experiments.md:
#
#   - print the CONDITIONS on the way out, so a number can never be quoted
#     without the machine, the toolchain and the date that produced it;
#   - give every experiment the same exit-code meaning, so a caller (and CI)
#     can read a run without knowing which experiment it was:
#
#       0  the measurement ran and matched what the script expected
#       1  the measurement ran and the result was NOT what was expected
#       2  the measurement could not be taken at all
#
# ⛔ 1 AND 2 ARE NOT THE SAME AND MUST NOT BE MERGED. "the portable binary
# crashed on Alpine" and "Alpine was never fetched" both fail a build, and only
# the first is a finding. Every helper here keeps them apart.

# ⛔ NEVER `cd`. docs/methodology/experiments.md: "no dependence on the
# directory it runs from - resolve paths from the script's own location."
EXP_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_DIR=$(cd "$EXP_DIR/.." && pwd)
ROOTFS_DIR="${PGB_ROOTFS_DIR:-/var/lib/pgb-rootfs}"
OUT_DIR="${PGB_EVIDENCE_DIR:-$REPO_DIR/evidence}"

# ---------------------------------------------------------------------------
# ⛔ ONE SOURCE OF TRUTH FOR THE PINNED BUILD ENVIRONMENT, AND IT IS cfg.go.
#
# Eight experiments each carried `pgb-env-debian12` as their own fallback.
# T-070 measured the glibc pin move and found that changing `cfg.go` alone
# would have left every one of them looking at the OLD environment, in two
# ways and neither of them loud:
#
#   - on a machine where that directory is gone they skip, which is an exit 2
#     nobody reads as a regression;
#   - on a machine where it is still on disk -- every machine that ever built
#     it -- they MEASURE THE OLD GLIBC AND SAY NOTHING.
#
# The second is not hypothetical: it is what `PGB_ENV_NAME` did to arm 5 of
# `experiments/91-` on 2026-09-02e, caught only by reading `.comment` out of a
# binary the POC had just produced. TODO/toolchain.md T-070.
#
# ⛔ AN UNREADABLE OR UNPARSABLE cfg.go IS exit 2, NOT AN EMPTY STRING. An
# empty name makes "$ROOTFS_DIR/$ENV_NAME" the rootfs directory ITSELF, which
# exists, so every `[ -d ... ]` guard downstream passes and the experiment
# chroots into a tree holding eleven distributions.
PGB_CFG_GO="$REPO_DIR/internal/cfg/cfg.go"

# exp_cfg_const NAME -> the string value of that Go constant on stdout.
# ⛔ Returns non-zero rather than calling `exit`: this runs inside `$(...)`,
# which is a SUBSHELL, and an `exit` there ends the subshell and leaves the
# caller running with an empty variable -- the exact silent-empty-name failure
# the block above exists to prevent. The caller must check, and does.
exp_cfg_const() {
  [ -r "$PGB_CFG_GO" ] || {
    printf 'lib.sh: cannot read %s\n' "$PGB_CFG_GO" >&2; return 2; }
  _cc_v=$(sed -n 's/^[[:space:]]*'"$1"'[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' \
          "$PGB_CFG_GO" | head -1)
  [ -n "$_cc_v" ] || {
    printf 'lib.sh: %s defines no %s\n' "$PGB_CFG_GO" "$1" >&2; return 2; }
  printf '%s' "$_cc_v"
}

# The pinned build environment's directory name, overridable for a candidate
# pin the way `experiments/91-` does. Set once at source time: it is read by
# nine scripts and it cannot change under a running experiment.
PGB_ENV_NAME_DEFAULT=$(exp_cfg_const DefaultEnvName) || exit 2
[ -n "$PGB_ENV_NAME_DEFAULT" ] || {
  printf 'lib.sh: DefaultEnvName came back empty\n' >&2; exit 2; }
ENV_NAME="${PGB_ENV_NAME:-$PGB_ENV_NAME_DEFAULT}"
ENV_ROOT="$ROOTFS_DIR/$ENV_NAME"

# ⛔ THE BUNDLER'S PACK SETTINGS, READ FROM THE GO SOURCE, FOR THE SAME REASON
# `exp_cfg_const` exists. `experiments/77-` copied `-S26` out of
# `internal/bundle/appimage.go` into its own `pack()` and said in a comment
# that it was "the production one" — true when it was written, false the
# moment `81-` moved the shipped block size to `-S18`. An experiment packing
# unlike production measures something nobody ships, and says nothing while it
# does it.
#
# ⚠ Matched on the argv STRING, not on "-S" anywhere in the file: the comment
# block above that line is a table of seven block sizes, and a loose match
# would take whichever appeared first.
PGB_APPIMAGE_GO="$REPO_DIR/internal/bundle/appimage.go"
exp_pack_blocksize() {  # -> the -S exponent, or non-zero if it cannot be read
  [ -r "$PGB_APPIMAGE_GO" ] || {
    printf 'lib.sh: cannot read %s\n' "$PGB_APPIMAGE_GO" >&2; return 2; }
  _pb=$(sed -n 's/.*"-C", "zstd:level=[0-9]*", "-S\([0-9][0-9]*\)".*/\1/p' \
        "$PGB_APPIMAGE_GO" | head -1)
  [ -n "$_pb" ] || {
    printf 'lib.sh: %s has no recognisable -S argument\n' "$PGB_APPIMAGE_GO" >&2
    return 2; }
  printf '%s' "$_pb"
}

PASS=0
FAIL=0
SKIP=0

exp_begin() {   # title
  EXP_TITLE="$1"
  EXP_NAME=$(basename "$0" .sh)
  EXP_OUT="$OUT_DIR/$EXP_NAME"
  mkdir -p "$EXP_OUT"
  printf '===============================================================\n'
  printf '%s\n' "$EXP_TITLE"
  printf '===============================================================\n'
  exp_conditions
  printf '\n'
}

exp_conditions() {
  printf -- '-- conditions ------------------------------------------------\n'
  printf 'date (UTC)   : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'host kernel  : %s\n' "$(uname -sr)"
  printf 'host arch    : %s\n' "$(uname -m)"
  if [ -r /etc/os-release ]; then
    printf 'host distro  : %s\n' "$(. /etc/os-release 2>/dev/null; printf '%s' "${PRETTY_NAME:-unknown}")"
  fi
  printf 'host libc    : %s\n' "$(ldd --version 2>&1 | head -1)"
  printf 'cc           : %s\n' "$({ ${CC:-cc} --version 2>/dev/null || echo none; } | head -1)"
  printf 'ld           : %s\n' "$(ld --version 2>/dev/null | head -1)"
  printf 'evidence     : %s\n' "$EXP_OUT"
  printf -- '--------------------------------------------------------------\n'
}

# A single assertion. Prints the value it actually saw, always, because a bare
# "ok" hides a value that was right for the wrong reason.
exp_check() {   # label actual expected
  if [ "$2" = "$3" ]; then
    printf '  ok    %-46s = %s\n' "$1" "$2"
    PASS=$((PASS+1))
  else
    printf '  FAIL  %-46s = %s, expected %s\n' "$1" "$2" "$3"
    FAIL=$((FAIL+1))
  fi
}

# ⚠ A SKIP IS NOT A PASS AND IT IS NOT A FAILURE. It is "this could not be
# measured here", and it must stay visible or an unrun matrix row reads as a
# green one.
exp_skip() {    # label why
  printf '  SKIP  %-46s (%s)\n' "$1" "$2"
  SKIP=$((SKIP+1))
}

exp_note() { printf '        %s\n' "$*"; }

# ⛔ exp_count EXISTS BECAUSE `$(grep -c … || echo 0)` IS WRONG AND KEEPS BEING
# WRITTEN ANYWAY. `grep -c` PRINTS the count and then EXITS 1 when the count is
# zero, so the fallback fires as well and the value becomes two lines, "0\n0",
# which never equals "0" — and it fails at exactly the moment everything passed.
#
# ⚠ THE TREE HAD ALREADY DIAGNOSED THIS THREE TIMES — in `79-`, `91-` and `95-`,
# each in a comment beside one call site — and the pattern was reintroduced in
# five more files, because a comment is not a mechanism. ⭐ One helper and one
# gate (`TODO/check.sh`) is the mechanism.
#
# ⚠ `|| echo 0` after `wc -c < file` is NOT the same bug and is correct: when
# the file is missing the redirection fails, `wc` never runs, and nothing is
# printed at all.
exp_count() {   # pattern file -> exactly one integer, always
  _xc=$(grep -ac "$1" "$2" 2>/dev/null) || _xc=${_xc:-0}
  printf '%s' "${_xc:-0}"
}

exp_rootfs() {  # name -> echoes path, or empty when absent
  if [ -d "$ROOTFS_DIR/$1" ]; then printf '%s' "$ROOTFS_DIR/$1"; fi
}

# ⚠ MULTIARCH IS NOT AN EDGE CASE, IT IS DEBIAN. The first version of this
# looked in {,/usr}/lib*/libc.so.6 and reported "unknown" for Debian 11,
# Debian 12, Ubuntu 20.04 and openSUSE Leap, all of which are plainly glibc
# systems that keep it in /lib/<triplet>/. The 10- gate caught it because the
# rootfs-images.txt row states the expected libc and the check compares
# against it, which is the whole reason that column is in the file.
exp_rootfs_libc() { # name -> musl | glibc | unknown
  r="$ROOTFS_DIR/$1"
  if ls "$r"/lib/ld-musl-*.so.* >/dev/null 2>&1; then printf 'musl'; return; fi
  for p in "$r"/lib/libc.so.6 "$r"/lib64/libc.so.6 "$r"/usr/lib/libc.so.6 \
           "$r"/usr/lib64/libc.so.6 "$r"/lib/*/libc.so.6 "$r"/usr/lib/*/libc.so.6; do
    [ -e "$p" ] && { printf 'glibc'; return; }
  done
  printf 'unknown'
}

# Run a command inside a rootfs and echo ONLY its exit status.
# ⛔ Unpiped: the status has to be the command's own.
#
# ⛔ THE FIRST ARGUMENT IS A PATH, NOT A ROOTFS NAME, and until deep review 5
# on 2026-09-03c getting that wrong was INVISIBLE. Measured, three genuinely
# different situations returning one indistinguishable answer:
#
#     a rootfs path that does not exist  -> "2"
#     a rootfs NAME where a path is due  -> "2"
#     a real rootfs, program exits 2     -> "2"
#
# ⚠ That is this project's own exit convention -- 0 ok, 1 ran-and-failed,
# 2 COULD-NOT-RUN -- being collapsed by the helper that carries it. An
# experiment comparing the result against a number would go green on a run
# that never happened. `experiments/97-` walked into the name-versus-path half
# of it and printed nothing on all eleven rows.
#
# ⭐ SO A ROOTFS THAT IS NOT A DIRECTORY NOW RETURNS A NON-NUMERIC TOKEN.
# `exp_check "..." "$st" 0` then FAILS naming it, instead of matching some
# number by coincidence. ⚠ It does not catch every could-not-run -- `pgb
# rootfs run` can still fail for its own reasons and report 2 -- so a caller
# whose expected status is 2 still owes itself a positive control, as
# `experiments/80-` already has.
exp_run_status() { # rootfs-PATH copyspec cmd...
  _r="$1"; _c="$2"; shift 2
  if [ ! -d "$_r" ]; then
    printf 'no-rootfs(%s)' "$_r"
    return
  fi
  "$REPO_DIR/pgb" rootfs run "$_r" --copy "$_c" -- "$@" >/dev/null 2>&1
  printf '%s' "$?"
}

# ---------------------------------------------------------------------------
# Trace ONE binary inside a rootfs and report only ITS file opens.
#
# ⛔ THE DEFECT THIS EXISTS TO CATCH, and it produced a wrong reading here
# before it was written: `strace -f` over the whole runner also captures the
# runner's own helpers. `pgb rootfs run --copy` copies the artefact in, and
# on this host coreutils `cp` is dynamically linked, so the trace filled up
# with libacl, libattr, libblkid, libmount, libpcre2, libselinux and libc.so.6
# opens that had NOTHING to do with the binary under test. Read naively that
# says "the portable binary loaded seven host libraries", which is false.
#
# Two defences, both needed:
#   1. the copy happens FIRST, outside the trace;
#   2. only lines from the pid that actually execve()'d the target, and only
#      those AFTER that execve, are reported.
#
# Prints one `openat`/`open` line per file the target itself touched.
exp_trace_opens() {  # rootfs in-root-path tracefile [extra rootfs-run args...]
  _tr_root="$1"; _tr_bin="$2"; _tr_out="$3"; shift 3
  strace -f -e trace=openat,open,execve -o "$_tr_out" \
    "$REPO_DIR/pgb" rootfs run "$_tr_root" "$@" -- "$_tr_bin" \
    >/dev/null 2>&1
  awk -v want="$_tr_bin" '
    { pid = $1 }
    $0 ~ ("execve\\(\"" want "\"") { target = pid; seen = 1; next }
    seen && pid == target && /open(at)?\(/ { print }
  ' "$_tr_out"
}

# The same, reduced to the shared objects and gconv/NSS data the target opened.
#
# ⛔ A SHARED OBJECT ENDS IN .so OR .so.N -- IT IS NOT ANY PATH CONTAINING
# ".so". The first version matched the substring, so `/etc/ld.so.cache` -- an
# index, not an object, opened by every glibc process that reaches dlopen --
# was reported as a loaded shared object. It reached committed evidence:
# `evidence/poc/10-gawk/RESULT.txt` lists it on all seven glibc rows. No
# verdict was wrong there, because a real object sat beside it on every such
# row, but the same expression in `pgb verify` decides pass/fail, and a binary
# that opened the cache and loaded nothing would have been failed for it.
# docs/history/corrections.md, instrument defects.
exp_trace_libs() {   # rootfs in-root-path tracefile [extra args...]
  exp_trace_opens "$@" \
    | grep -vE 'ENOENT|= -1' \
    | grep -oE '"[^"]*\.so(\.[0-9]+)*"|"[^"]*gconv[^"]*"|"[^"]*/locale[^"]*"' \
    | tr -d '"' | sort -u
}

# exp_classify_trace reads a `strace -f` transcript and says which shared
# objects the ARTEFACT loaded, split into host and bundled.
#
# ⛔ THREE RULES, AND EVERY ONE OF THEM WAS A DEFECT HERE FIRST.
#
#  1. ⛔ NOT ONE PID. uruntime forks, mounts and re-execs, so the payload runs
#     in a descendant. The fork family has to be followed through BOTH halves
#     of a split call: strace writes `clone( <unfinished ...>` and
#     `<... clone resumed>) = 1234`, and the pid is only on the second.
#     experiments/62- carries what missing that cost.
#  2. ⛔ A SHARED OBJECT ENDS IN `.so` OR `.so.N`. `/etc/ld.so.cache` is an
#     index, not an object, and matching `.so` as a substring counts it.
#  3. ⭐ AN `openat` CAN BE SPLIT TOO, AND THIS ONE IS NEW — 2026-09-03f.
#     `openat(..., "path" <unfinished ...>` carries the PATH and no result;
#     `<... openat resumed>) = -1 ENOENT` carries the result and no path. A
#     filter that drops lines containing `ENOENT` therefore keeps the first
#     half of a FAILED open and counts it as a load. ⚠ Measured: a galculator
#     bundle on alpine-3.22 reported 2 host shared objects, and both were
#     `libGLX.so.1` probes that returned ENOENT on the resumed line.
#     ⛔ The error only ever runs one way — it can turn a clean row dirty and
#     can never turn a dirty row clean — so a committed ZERO is unaffected by
#     it. A committed NON-zero may be inflated. docs/history/corrections.md.
#
# Usage: exp_classify_trace <tracefile> <in-root artefact path>
# exp_classify_trace <tracefile> <in-root-path> [payload|tree]
#
# ⛔ `mode` IS THE LAST ARGUMENT AND IT DEFAULTS, and that ordering is the
# whole design. TODO/ci.md T-084 first said `mode` FIRST; with `mode` first a
# caller that had not been updated would pass the TRACEFILE as the mode and
# nothing as `want`, so no `execve` line would ever match, `inset` would stay
# empty, and EVERY ROW WOULD REPORT ZERO HOST SHARED OBJECTS -- green, on the
# one number the corpus exists to measure. ⚠ experiments/65- is RESUMABLE and
# re-sources this file, so that caller really exists.
#
#   tree     count opens across the whole process set (the default, and what
#            this function did before `mode` existed)
#   payload  count only opens in the pid that last execve'd, clearing the set
#            at each exec -- an object opened BEFORE the last exec is not
#            mapped in the running program
#
# ⛔ An unknown mode is a LOUD ERROR, never a fallback to `tree`.
exp_classify_trace() {
  _ct_mode=${3:-tree}
  case "$_ct_mode" in
    payload|tree) ;;
    *) printf 'exp_classify_trace: unknown mode "%s" (payload|tree)\n' "$_ct_mode" >&2
       return 2 ;;
  esac
  awk -v want="$2" -v mode="$_ct_mode" '
    function record(p) {
      if (p !~ /\.so(\.[0-9]+)*$/) return
      if (p ~ /^\/(usr\/)?(local\/)?lib(32|64)?\//) { out["host " p] = 1; return }
      # ⛔ "host" IS NOT ONLY /lib AND /usr/lib, AND THE PREFIX LIST WAS THE
      # WHOLE TEST. `bundled` is the COMPLEMENT here -- anything the first
      # pattern misses is scored as the artefact own -- so a host object in
      # an unlisted directory reads CLEAN. ⚠ That is the DANGEROUS direction:
      # C25 can only turn a clean row dirty, this one turns a dirty row clean,
      # which is the error a committed zero cannot survive.
      #
      # ⭐ MEASURED ACROSS ALL ELEVEN rather than guessed. Every `.so` on every
      # pinned rootfs that the first pattern misses, in full:
      #
      #     /usr/libexec/coreutils/libstdbuf.so   10 of 11 (the LD_PRELOAD stdbuf uses)
      #     /usr/libexec/sudo/*.so                fedora-42 only, 8 files
      #     /usr/bin/ld.so                        arch, fedora, debian 12 + 13
      #
      # ⛔ THE THIRD ONE IS WHY THIS MATTERS. `/usr/bin/ld.so` is the HOST
      # LOADER, shipped in bindir by Arch and Fedora. An artefact that ran the
      # host LOADER is the exact failure this whole tree exists to detect,
      # and under the old predicate that row scored `bundled` and reported
      # CLEAN.
      #
      # ⚠ WHY EXTENDING THE LIST IS SAFE. Nothing bundled ever lands in these
      # directories: uruntime extracts under /tmp (`appimage_extracted_*`),
      # `--extract` writes ./squashfs-root, and the artefact itself is staged
      # at /subj*. So this can only move a row from clean to dirty -- the same
      # one-way direction as C25 -- and never the reverse.
      if (p ~ /^\/(usr\/)?(s?bin|libexec|opt)\//) { out["host " p] = 1; return }
      out["bundled " p] = 1
    }
    { pid = $1 }
    $0 ~ ("execve\\(\"" want "\"") { inset[pid] = 1; payload = pid
                                    if (mode != "tree") delete out
                                    next }
    # ⚠ A LATER exec inside the set replaces what is mapped, in payload mode.
    inset[pid] && /execve\(/ && !/ENOENT|= -1/ {
      payload = pid
      if (mode != "tree") delete out
      next
    }
    ($0 ~ /(clone|clone3|vfork|fork)\(/ || $0 ~ /<\.\.\. (clone|clone3|vfork|fork) resumed>/) \
      && /= [0-9]+$/ { if (inset[pid]) inset[$NF] = 1; next }
    /open(at)?\(/ && /<unfinished \.\.\.>/ {
      if (!inset[pid]) next
      if (mode != "tree" && pid != payload) next
      if (match($0, /"[^"]*"/) == 0) next
      pending[pid] = substr($0, RSTART + 1, RLENGTH - 2)
      next
    }
    /<\.\.\. open(at)? resumed>/ {
      if (inset[pid] && pending[pid] != "" && $0 !~ /= -1/) record(pending[pid])
      delete pending[pid]
      next
    }
    inset[pid] && /open(at)?\(/ && !/ENOENT|= -1/ {
      if (mode != "tree" && pid != payload) next
      if (match($0, /"[^"]*"/) == 0) next
      record(substr($0, RSTART + 1, RLENGTH - 2))
    }
    END { for (k in out) print k }
  ' "$1" | sort -u
}

# ---------------------------------------------------------------------------
# exp_host_spawns reads the same `strace -f` transcript and says which HOST
# PROGRAMS the artefact's own process set execve'd, by name.
#
# ⭐ WHY THIS IS A SEPARATE QUESTION FROM `exp_classify_trace`, and it is the
# whole of T-094. `exp_classify_trace` counts shared objects the artefact
# LOADED. It cannot see the mechanism `docs/history/corrections.md` C55 found:
#
#     execve("/bin/sh", ["sh", "-c", "--", "/nix/store/…-gnuplot-6.0.5/…"])
#
# `qalculate-qt` probes for gnuplot through the HOST's shell. The shell then
# loads the host's libc, and the host objects show up in the object count with
# nothing to say WHERE they came from. ⛔ No amount of path rewriting prevents
# that — the bundle would have to carry a shell — so the first thing anybody
# needs is the COUNT: how many subjects do this at all?
#
# Usage: exp_host_spawns <tracefile> <in-root artefact path>
# Output: one line per distinct host program, `ok <path>` or `fail <path>`.
#
# ⛔ THE HOST TEST HERE IS THE COMPLEMENT OF THE ARTEFACT'S OWN LOCATIONS, AND
# THAT IS DELIBERATE — IT IS **NOT** THE PREFIX LIST C49 CORRECTED.
#
# C49's defect was a host PREFIX LIST whose complement was scored `bundled`, so
# a host object in an unlisted directory read CLEAN. That errs toward looking
# clean, which is the direction a committed zero cannot survive. ⭐ Here the
# test is inverted: an artefact's own program can only be in three places, all
# fixed by construction —
#
#     the staged artefact itself            $want   (/subj65)
#     uruntime's extraction root            /tmp/…  (appimage_extracted_*,
#                                                    .mount_*, and the AppDir
#                                                    the bundler stages there)
#     an unrewritten store path             /nix/store/…
#
# — and EVERYTHING ELSE is host. So an unanticipated location errs toward
# reporting a spawn that is really the artefact's own: it can only OVER-count,
# and every path is printed by name so an over-count is visible rather than
# inferred. ⛔ Do not "fix" this into a prefix list; the two functions err in
# opposite directions on purpose, because a missed host object hides a defect
# and a missed host spawn hides the whole finding.
#
# ⚠ A relative path is not host: `./AppRun` and `squashfs-root/…` are the
# artefact's, and an execve with no leading `/` is resolved against a cwd
# inside it.
#
# ⛔⛔ IT READS THE TRACE TWICE, AND A ONE-PASS VERSION SILENTLY MISSED THE ONE
# SPAWN IT EXISTS TO FIND. Measured against a REAL `strace -f` transcript on
# 2026-09-05, not against the fixture — the fixture could not show it:
#
#     8217  vfork( <unfinished ...>
#     8218  execve("/bin/sh", ["/bin/sh", "-c", "--", …] <unfinished ...>
#     8217  <... vfork resumed>)              = 8218
#     8218  <... execve resumed>)             = 0
#
# ⭐ `vfork` SUSPENDS THE PARENT UNTIL THE CHILD EXECS, so the child's `execve`
# is written BEFORE the line that first names the child's pid. A single pass
# has not learned 8218 belongs to the artefact yet and drops the spawn. ⛔ The
# probe then reported only the child's SECOND exec — the one that failed — so
# the row read `fail /usr/bin/gnuplot` and the `/bin/sh` that C55 is entirely
# about was invisible. A count that is wrong in the direction of its own
# thesis is the worst shape this instrument could have had.
#
# ⚠ `exp_classify_trace` is NOT affected and it is worth saying why rather than
# assuming: it counts `openat`, and a child's library opens happen AFTER its
# exec completes, by which time the parent has resumed and the pid is known.
# The `execve` line is the uniquely dangerous one because it is the syscall the
# child makes while the parent is still blocked.
#
# ⚠ The fork closure is by PID and carries no line-order constraint, so a pid
# the LAUNCHER cloned before the artefact exec'd would be attributed to the
# artefact. `pgb rootfs run` execs the artefact's shell in place and clones
# nothing before it, and the error runs in the over-reporting direction, which
# is the one this function is allowed to make.
exp_host_spawns() {
  awk -v want="$2" '
    function spawn(p, st) {
      # ⛔ THE ARTEFACT IS A PREFIX, NOT ONE PATH, AND THE SELFTEST CAUGHT IT.
      # The first version compared `p == want` and scored a second stage of the
      # artefact -- `/subj-stage2` beside `/subj` -- as a HOST SPAWN, which
      # would have reported every multi-stage subject as shelling out.
      # ⚠ This is the one exemption that runs in the under-counting direction,
      # so it is bounded rather than general: `want` is a path THIS HARNESS
      # creates at the rootfs root (`/subj65`, `/subjA`), and no pinned rootfs
      # ships anything under it. A host program cannot be there.
      if (index(p, want) == 1)    return       # the artefact, or a stage of it
      if (p !~ /^\//)             return       # relative: inside the artefact
      if (p ~ /^\/tmp\//)         return       # uruntime extraction root
      if (p ~ /^\/nix\/store\//)  return       # an unrewritten store path
      out[st " " p] = 1
    }
    # pass 1 -- the fork graph, and which pid became the artefact
    FNR == NR {
      if ($0 ~ ("execve\\(\"" want "\"")) { inset[$1] = 1; next }
      if (($0 ~ /(clone|clone3|vfork|fork)\(/ \
           || $0 ~ /<\.\.\. (clone|clone3|vfork|fork) resumed>/) && /= [0-9]+$/) {
        nk++; kid[nk] = $NF; par[nk] = $1
      }
      next
    }
    # ⭐ the descendant closure, computed once, before a single line is scored
    FNR == 1 {
      do {
        again = 0
        for (i = 1; i <= nk; i++)
          if (inset[par[i]] && !inset[kid[i]]) { inset[kid[i]] = 1; again = 1 }
      } while (again)
    }
    inset[$1] && /execve\(/ {
      if (match($0, /"[^"]*"/) == 0) next
      spawn(substr($0, RSTART + 1, RLENGTH - 2), (/= -1/ ? "fail" : "ok"))
    }
    END { for (k in out) print k }
  ' "$1" "$1" | sort -u
}

# ---------------------------------------------------------------------------
# exp_id_match says whether an id is selected by a subject filter.
#
# ⛔ THE FILTER USED TO BE ONE `case` PATTERN AND `'qt-*|py-*'` MATCHED NOTHING.
# `case` alternation is SYNTAX: a `|` that arrives through a variable expansion
# is an ordinary character in a single pattern, so a run filtered that way exited
# immediately with `at least one subject produced an artefact = no`. It cost a
# parallel instance on 2026-09-04c and the recipe in TODO/RESUME.md had to carry
# a warning instead.
#
# ⭐ So the list is SPLIT here — on `|`, on whitespace, or both — and each part
# is tried as its own pattern. An empty filter selects everything.
exp_id_match() {   # id filter
  [ -n "${2:-}" ] || return 0
  _im_old=$IFS
  IFS='|
	 '
  for _im_p in $2; do
    [ -n "$_im_p" ] || continue
    # shellcheck disable=SC2254  # the pattern is meant to glob
    case "$1" in $_im_p) IFS=$_im_old; return 0 ;; esac
  done
  IFS=$_im_old
  return 1
}

exp_finish() {
  printf '\n'
  printf -- '-- result ----------------------------------------------------\n'
  printf 'pass=%s fail=%s skip=%s\n' "$PASS" "$FAIL" "$SKIP"
  if [ "$FAIL" -gt 0 ]; then
    printf 'VERDICT: the measurement ran and did NOT match expectation.\n'
    printf -- '--------------------------------------------------------------\n'
    exit 1
  fi
  if [ "$PASS" = 0 ]; then
    printf 'VERDICT: nothing was actually measured.\n'
    printf -- '--------------------------------------------------------------\n'
    exit 2
  fi
  printf 'VERDICT: matched expectation.\n'
  printf -- '--------------------------------------------------------------\n'
  exit 0
}

# ---------------------------------------------------------------------------
# ⭐ THE CLASSIFIER'S SELFTEST. `sh experiments/lib.sh --selftest`
#
# ⛔ GUARDED ON $0, NOT ON $1 ALONE. This file is SOURCED by every experiment,
# and an experiment invoked with `--selftest` as its own first argument would
# otherwise run this instead of itself. When sourced, `$0` is the experiment.
#
# ⚠ The fixture is strace's real shape, including the one it exists for: a
# split `openat( ... <unfinished ...>` carries the PATH and the FOLLOWING line
# carries the RESULT, so a filter that only drops lines containing ENOENT keeps
# the first half of a FAILED open and counts it as a load.
# docs/history/corrections.md C25.
# ---------------------------------------------------------------------------
_ct_selftest() {
  _d=$(mktemp -d) || return 2
  _t="$_d/trace"
  cat > "$_t" <<'TRACE'
99 execve("/usr/bin/pgb", ["pgb", "rootfs", "run"], 0x7ffd) = 0
99 clone(child_stack=NULL) = 100
100 execve("/subj", ["/subj"], 0x7ffd) = 0
100 openat(AT_FDCWD, "/usr/lib/x86_64-linux-gnu/libhost.so.6", O_RDONLY) = 3
100 clone(child_stack=NULL) = 200
200 openat(AT_FDCWD, "/tmp/mnt/lib/libbundled.so.1", O_RDONLY) = 4
100 openat(AT_FDCWD, "/usr/lib/libfailed.so.1", O_RDONLY <unfinished ...>
100 <... openat resumed>) = -1 ENOENT (No such file or directory)
100 openat(AT_FDCWD, "/etc/ld.so.cache", O_RDONLY) = 6
100 openat(AT_FDCWD, "/usr/bin/ld.so", O_RDONLY) = 8
100 openat(AT_FDCWD, "/usr/libexec/coreutils/libstdbuf.so", O_RDONLY) = 9
200 execve("/tmp/appimage_extracted_ab/AppRun", ["AppRun"], 0x7ffd) = 0
200 vfork( <unfinished ...>
300 execve("/bin/sh", ["sh", "-c", "--", "/nix/store/zz-gnuplot-6.0.5/bin/gnuplot"], 0x7ffd <unfinished ...>
200 <... vfork resumed>)              = 300
300 <... execve resumed>)             = 0
300 execve("/usr/bin/gnuplot", ["gnuplot"], 0x7ffd) = -1 ENOENT (No such file or directory)
300 execve("/nix/store/zz-gnuplot-6.0.5/bin/gnuplot", ["gnuplot"], 0x7ffd) = -1 ENOENT (No such file or directory)
100 execve("/subj-stage2", ["/subj-stage2"], 0x7ffd) = 0
100 openat(AT_FDCWD, "/usr/lib/libafter.so.2", O_RDONLY) = 7
TRACE
  _p=0; _f=0
  _ck() {  # label got want
    if [ "$2" = "$3" ]; then printf '  ok    %-52s = %s\n' "$1" "$2"; _p=$((_p+1))
    else printf '  FAIL  %-52s = %s, expected %s\n' "$1" "$2" "$3"; _f=$((_f+1)); fi
  }
  _n() { exp_classify_trace "$_t" /subj "$2" | grep -c "^$1 " || true; }

  printf '\n-- exp_classify_trace --selftest ---------------------------------\n'
  # tree: the whole process set, and objects opened before the last exec count
  _ck "tree: host objects across the process set"   "$(_n host tree)"    4
  _ck "tree: the child pid's bundled object counts" "$(_n bundled tree)" 1
  # ⛔ HOST IS NOT ONLY /lib AND /usr/lib. Both of these are real files on the
  # pinned bed -- `/usr/bin/ld.so` is the host LOADER on Arch and Fedora -- and
  # under the original prefix-only predicate both scored `bundled`, so a row
  # that ran the host's loader reported CLEAN.
  _ck "⛔ /usr/bin/ld.so is the HOST loader, not bundled" \
      "$(exp_classify_trace "$_t" /subj tree | grep -c '^host /usr/bin/ld.so' || true)" 1
  _ck "⛔ /usr/libexec/.../libstdbuf.so is host, not bundled" \
      "$(exp_classify_trace "$_t" /subj tree | grep -c '^host /usr/libexec/' || true)" 1
  # payload: only the pid that last execve'd, cleared at each exec
  _ck "payload: only what the last exec mapped"     "$(_n host payload)" 1
  _ck "payload: ⛔ the child pid does NOT count"    "$(_n bundled payload)" 0
  # ⛔ C25: the split FAILED open must not be counted, in either mode
  _ck "⛔ a split FAILED open is not a load (tree)" \
      "$(exp_classify_trace "$_t" /subj tree | grep -c 'libfailed' || true)" 0
  _ck "⛔ ...nor in payload mode" \
      "$(exp_classify_trace "$_t" /subj payload | grep -c 'libfailed' || true)" 0
  # ⛔ /etc/ld.so.cache is an INDEX, not an object. docs/AGENTS.md §14.
  _ck "⛔ /etc/ld.so.cache is not an object" \
      "$(exp_classify_trace "$_t" /subj tree | grep -c 'ld.so.cache' || true)" 0
  # ⭐ the default is `tree`, which is what every existing caller relies on
  _ck "⭐ the default mode is tree (an old 2-arg call)" \
      "$(exp_classify_trace "$_t" /subj | grep -c '^host ' || true)" 4
  # ⛔ an unknown mode is an error, never a silent fallback
  exp_classify_trace "$_t" /subj nonsense >/dev/null 2>&1
  _ck "⛔ an unknown mode returns 2, not a fallback" "$?" 2

  # ⭐ exp_host_spawns — T-094's instrument, and the fixture carries C55's own
  # shape: a `/bin/sh -c --` probe for gnuplot from a pid the artefact cloned.
  printf '\n-- exp_host_spawns --selftest ------------------------------------\n'
  _s() { exp_host_spawns "$_t" /subj | grep -c "$1" || true; }
  # ⛔⛔ THE REGRESSION ROW. The fixture reproduces a REAL trace's ordering:
  # vfork suspends the parent, so pid 300's `execve("/bin/sh", …)` is written
  # BEFORE the `<... vfork resumed>) = 300` line that first names it. A
  # single-pass version had not learned 300 was the artefact's yet, dropped the
  # spawn, and reported only the child's SECOND (failed) exec — missing the one
  # mechanism C55 is about.
  _ck "⭐ the host shell C55 found is a spawn" "$(_s '^ok /bin/sh$')" 1
  _ck "⭐ a FAILED host exec is reported, not dropped" \
      "$(_s '^fail /usr/bin/gnuplot$')" 1
  _ck "⛔ uruntime's /tmp extraction root is NOT a host spawn" \
      "$(_s 'appimage_extracted')" 0
  _ck "⛔ an unrewritten /nix/store exec is NOT a host spawn" \
      "$(_s 'nix/store')" 0
  # ⛔ THE LAUNCHER IS NOT THE SUBJECT. `pgb rootfs run` execs before the
  # artefact does, from a pid that is not in the set, and counting it would
  # score EVERY subject as spawning a host program.
  _ck "⛔ an exec BEFORE the artefact's own is not counted" \
      "$(_s 'pgb')" 0
  _ck "⛔ the artefact re-execing itself is not a spawn" "$(_s '/subj')" 0
  _ck "⭐ two host spawns in total, and no more"  "$(exp_host_spawns "$_t" /subj | wc -l | tr -d ' ')" 2
  # ⛔ AND IT MUST BE ABLE TO REPORT ZERO. A counter that cannot say "none"
  # measures nothing -- docs/AGENTS.md §0b.
  printf '100 execve("/subj", ["/subj"], 0x7ffd) = 0\n' > "$_d/quiet"
  _ck "⛔ a subject that spawns nothing reports 0" \
      "$(exp_host_spawns "$_d/quiet" /subj | wc -l | tr -d ' ')" 0

  # ⭐ exp_id_match — the alternation the old one-`case` filter could not do
  printf '\n-- exp_id_match --selftest ---------------------------------------\n'
  _m() { exp_id_match "$1" "$2" && echo yes || echo no; }
  _ck "⛔ 'qt-*|py-*' selects qt-1 (a bare case could not)" "$(_m qt-1 'qt-*|py-*')" yes
  _ck "⛔ ...and py-3"                          "$(_m py-3 'qt-*|py-*')" yes
  _ck "⛔ ...and NOT gtk3-1"                    "$(_m gtk3-1 'qt-*|py-*')" no
  _ck "⭐ a space-separated list works too"     "$(_m sdl-2 'qt-* sdl-*')" yes
  _ck "⭐ one plain glob still works"           "$(_m field-4 'field-*')" yes
  _ck "⭐ an empty filter selects everything"   "$(_m anything '')" yes

  # ⛔ exp_count: the "0\n0" defect, which has bitten this tree four times
  printf 'alpha\nbeta\n' > "$_d/c"
  _ck "exp_count: a match counts"                 "$(exp_count alpha "$_d/c")" 1
  _ck "⛔ exp_count: NO match is one 0, not two"  "$(exp_count zeta "$_d/c")"  0
  _ck "⛔ exp_count: a MISSING file is 0"         "$(exp_count alpha "$_d/nope")" 0
  rm -rf "$_d"
  printf '\nlib.sh --selftest: %d pass, %d fail\n' "$_p" "$_f"
  [ "$_f" = 0 ]
}

case "$0" in
  */lib.sh|lib.sh)
    case "${1:-}" in
      --selftest) _ct_selftest; exit $? ;;
    esac
    ;;
esac
