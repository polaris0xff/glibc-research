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
  sh "$REPO_DIR/scripts/common/rootfs-run.sh" "$_r" --copy "$_c" -- "$@" >/dev/null 2>&1
  printf '%s' "$?"
}

# ---------------------------------------------------------------------------
# Trace ONE binary inside a rootfs and report only ITS file opens.
#
# ⛔ THE DEFECT THIS EXISTS TO CATCH, and it produced a wrong reading here
# before it was written: `strace -f` over the whole runner also captures the
# runner's own helpers. rootfs-run.sh copies the artefact in with `cp -a`, and
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
    sh "$REPO_DIR/scripts/common/rootfs-run.sh" "$_tr_root" "$@" -- "$_tr_bin" \
    >/dev/null 2>&1
  awk -v want="$_tr_bin" '
    { pid = $1 }
    $0 ~ ("execve\\(\"" want "\"") { target = pid; seen = 1; next }
    seen && pid == target && /open(at)?\(/ { print }
  ' "$_tr_out"
}

# The same, reduced to the shared objects and gconv/NSS data the target opened.
exp_trace_libs() {   # rootfs in-root-path tracefile [extra args...]
  exp_trace_opens "$@" \
    | grep -vE 'ENOENT|= -1' \
    | grep -oE '"[^"]*\.so[^"]*"|"[^"]*gconv[^"]*"|"[^"]*/locale[^"]*"' \
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
