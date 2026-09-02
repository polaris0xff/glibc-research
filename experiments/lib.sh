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
exp_run_status() { # rootfs copyspec cmd...
  _r="$1"; _c="$2"; shift 2
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
