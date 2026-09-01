# poc/common.sh -- shared by every proof of concept. Sourced, never run.
#
# WHAT A POC IN THIS DIRECTORY OWES
#
# The brief this project answers asks, for every proof of concept: the
# upstream version, its normal build, why static glibc is hard for it, the
# build through the tool, an inspection, a run on glibc, a run on musl,
# EXERCISED FUNCTIONALITY, recorded failures, recorded runtime dependencies,
# and a reproducible script. Every one of those is a field or a step below, so
# a POC that skips one cannot quietly look complete.
#
# ⛔ THE FUNCTIONAL TEST IS NOT OPTIONAL AND IT IS NOT `--version`.
# A binary that prints its version has demonstrated that it can start. Every
# failure this project is about -- NSS, gconv, locale, dlopen -- happens LATER,
# on the first real piece of work. So each POC declares poc_functional_test(),
# it runs INSIDE each target environment, and its exit status is the result.
#
# ⚠ A POC THAT FAILS IS KEPT. docs/methodology/experiments.md: "a negative
# result is a result, and it gets committed". A POC whose verdict is
# "this class does not work" is doing its job.

POC_DIR=$(cd "$(dirname "$0")" && pwd)
POC_ROOT=$(cd "$POC_DIR/.." && pwd)
REPO_ROOT=$(cd "$POC_ROOT/.." && pwd)
PGB="$REPO_ROOT/pgb"
ROOTFS_DIR="${PGB_ROOTFS_DIR:-/var/lib/pgb-rootfs}"
WORK="${PGB_POC_WORK:-/var/tmp/pgb-poc}"
EVIDENCE="${PGB_EVIDENCE_DIR:-$REPO_ROOT/evidence}/poc"

POC_PASS=0; POC_FAIL=0; POC_SKIP=0

poc_begin() {
  POC_NAME=$(basename "$POC_DIR")
  POC_OUT="$EVIDENCE/$POC_NAME"
  mkdir -p "$POC_OUT" "$WORK" || exit 2
  printf '===============================================================\n'
  printf 'POC %s -- %s\n' "$POC_NAME" "${POC_WHY:-}"
  printf '===============================================================\n'
  printf 'upstream      : %s\n' "${POC_URL:-?}"
  printf 'version       : %s\n' "${POC_VERSION:-?}"
  printf 'sha256        : %s\n' "${POC_SHA256:-<not pinned>}"
  printf 'normal build  : %s\n' "${POC_NORMAL_BUILD:-?}"
  printf 'stresses      : %s\n' "${POC_STRESSES:-?}"
  printf 'date (UTC)    : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'evidence      : %s\n' "$POC_OUT"
  printf -- '---------------------------------------------------------------\n'
}

poc_check() { # label actual expected
  if [ "$2" = "$3" ]; then printf '  ok    %-44s = %s\n' "$1" "$2"; POC_PASS=$((POC_PASS+1))
  else printf '  FAIL  %-44s = %s, expected %s\n' "$1" "$2" "$3"; POC_FAIL=$((POC_FAIL+1)); fi
}
poc_skip() { printf '  SKIP  %-44s (%s)\n' "$1" "$2"; POC_SKIP=$((POC_SKIP+1)); }
poc_note() { printf '        %s\n' "$*"; }

# Fetch and verify. A POC built from an unverified download is not reproducible.
poc_fetch() { # url outfile [sha256]
  _u="$1"; _o="$2"; _s="${3:-}"
  if [ -f "$_o" ] && [ -n "$_s" ]; then
    _h=$(sha256sum "$_o" 2>/dev/null | cut -d' ' -f1)
    [ "$_h" = "$_s" ] && return 0
  fi
  curl -sSfL --retry 3 --retry-delay 2 -o "$_o.part" "$_u" || return 1
  if [ -n "$_s" ]; then
    _h=$(sha256sum "$_o.part" | cut -d' ' -f1)
    if [ "$_h" != "$_s" ]; then
      # ⛔ NEVER CONTINUE PAST THIS. A changed tarball means the POC would
      # measure a different program than the one its write-up names.
      printf 'poc: %s sha256 MISMATCH\n  got  %s\n  want %s\n' "$_u" "$_h" "$_s" >&2
      rm -f "$_o.part"; return 1
    fi
  fi
  mv "$_o.part" "$_o"
}

# Run a command inside the pgb build environment.
# ⚠ $WORK IS OUTSIDE THE REPOSITORY, so the build environment cannot see it
# without an explicit bind. Without this the chroot reports "cd: can't cd to
# /var/tmp/..." and it reads like a missing tarball rather than a missing mount.
poc_in_env() { sh "$PGB" --bind "$WORK" build -- /bin/sh -c "$1"; }

# ---------------------------------------------------------------------------
# The matrix run. This is what the POC is FOR.
#
# For each pinned environment: copy the binary in, run the POC's own
# functional test inside, record the exit status, and separately record every
# shared object or gconv path the binary opened.
# ---------------------------------------------------------------------------
poc_matrix() { # binary-path  [extra files to copy: src:dst ...]
  _bin="$1"; shift
  _base=$(basename "$_bin")
  printf '\n  functional test across the pinned matrix:\n'
  printf '    %-20s %-6s %-10s %-28s %s\n' ENVIRONMENT LIBC RESULT 'HOST .so LOADED' 'HOST DATA READ'
  while read -r ref name libc digest; do
    case "$ref" in ''|\#*) continue ;; esac
    _r="$ROOTFS_DIR/$name"
    [ -d "$_r" ] || { poc_skip "$name" "not fetched"; continue; }

    cp "$_bin" "$_r/$_base" || { poc_check "$name: copy" failed ok; continue; }
    poc_stage_extras "$_r" "$@"
    # ⛔ THE TEST BED IS SHARED AND MUST COME BACK UNCHANGED. A functional
    # test that needs a resolvable name writes it into the target's
    # /etc/hosts; without a restore those writes ACCUMULATE across runs. Found
    # by inspection after the curl POC left six identical lines in
    # debian-12's /etc/hosts, which is exactly the kind of drift that makes a
    # later result depend on how many times an earlier POC was run.
    _hosts_bak=""
    if [ -f "$_r/etc/hosts" ]; then
      _hosts_bak=$(mktemp); cp "$_r/etc/hosts" "$_hosts_bak"
    fi

    # The POC's own test script, written into the target and run there.
    poc_functional_test > "$_r/pgb-poc-test.sh"
    sh "$REPO_ROOT/scripts/common/rootfs-run.sh" "$_r" -- /bin/sh /pgb-poc-test.sh \
       >"$POC_OUT/$name.log" 2>&1
    _st=$?
    case $_st in
      0) _res=ok ;;
      13[0-9]|1[4-6][0-9]) _res="SIG$((_st-128))" ;;
      *) _res="exit$_st" ;;
    esac
    _libs=$(poc_trace "$_r" "/$_base" pgb-poc-test.sh)
    _data=$(poc_trace_data "$_r" "/$_base" pgb-poc-test.sh)
    printf '    %-20s %-6s %-10s %-28s %s\n' "$name" "$libc" "$_res" "${_libs:-none}" "${_data:-none}"
    poc_check "$name: functional test" "$_res" ok
    # ⛔ ONLY THE SHARED-OBJECT COLUMN IS ASSERTED, and the reason is a
    # correction. Loading a host .so is the two-libc failure this whole
    # project is about. READING host data is not the same thing and must not
    # be treated as one: glibc still opens /etc/nsswitch.conf under the NSS
    # override, and a program that finds and honours the host's locale is
    # behaving correctly, not leaking.
    #
    # What matters is INDEPENDENCE, not abstinence -- the program must work
    # whether or not the data is there. The matrix proves that directly: the
    # four musl environments have no glibc locale data, no gconv tree and (on
    # Alpine) no terminfo, and the same binary passes there. Asserting "reads
    # no host data" would have failed CPython for correctly reading the
    # C.utf8 locale Debian provides.
    [ -n "$_libs" ] && poc_check "$name: host shared objects loaded" "$_libs" none

    [ -n "$_hosts_bak" ] && { cp "$_hosts_bak" "$_r/etc/hosts"; rm -f "$_hosts_bak"; }
    poc_unstage_extras "$_r" "$@"
    rm -f "$_r/$_base" "$_r/pgb-poc-test.sh"
  done < "$REPO_ROOT/scripts/common/rootfs-images.txt"
}

# ⛔ REPLACE, NEVER MERGE. `cp -a SRC DST` where DST already exists copies
# SRC *into* DST, so a second run nests the tree one level deeper and the
# paths the program expects stop resolving. Measured: Rocky 8 reported
# "No module named 'json'" while Fedora's copy of the same tree had grown from
# 103 MiB to 192 MiB -- two stacked copies, one of them unreachable.
poc_stage_extras() { # rootfs [src:dst ...]
  _sr="$1"; shift
  for extra in "$@"; do
    _s=${extra%%:*}; _d=${extra#*:}
    rm -rf "${_sr:?}$_d"
    mkdir -p "$_sr$(dirname "$_d")" 2>/dev/null
    cp -a "$_s" "$_sr$_d" 2>/dev/null
  done
}
poc_unstage_extras() { # rootfs [src:dst ...]
  _sr="$1"; shift
  for extra in "$@"; do
    _d=${extra#*:}
    case "$_d" in /|/etc|/usr|/lib|/bin|"") continue ;; esac
    rm -rf "${_sr:?}$_d"
  done
}

# ---------------------------------------------------------------------------
# An OBSERVATION run, for behaviour that is being characterised rather than
# required.
#
# ⛔ WHY THIS IS SEPARATE FROM poc_matrix, AND IT IS NOT A CONVENIENCE.
# A probe that deliberately calls dlopen() will of course load a host object,
# so folding it into the functional test would make the "loaded no host
# object" assertion fail for the one reason that is not a defect -- and, worse,
# would hide a real host-object load behind an expected one. The functional
# test says what the program must do; this says what the platform DOES.
#
# ⚠ NOTHING HERE IS ASSERTED. The result is printed per environment and goes
# into the write-up. An expectation would be a guess: the first version of the
# gawk POC asserted "a static binary cannot dlopen an extension" and the
# matrix disproved it on two distributions out of eleven.
poc_observe() {  # binary-path label [extra files: src:dst ...]
  _bin="$1"; _label="$2"; shift 2
  _base=$(basename "$_bin")
  printf '\n  OBSERVATION -- %s (measured, not asserted):\n' "$_label"
  printf '    %-20s %-6s %-10s %s\n' ENVIRONMENT LIBC OUTCOME 'HOST OBJECTS PULLED IN'
  while read -r ref name libc digest; do
    case "$ref" in ''|\#*) continue ;; esac
    _r="$ROOTFS_DIR/$name"
    [ -d "$_r" ] || continue
    cp "$_bin" "$_r/$_base" 2>/dev/null || continue
    poc_stage_extras "$_r" "$@"
    poc_observation_probe > "$_r/pgb-poc-probe.sh"
    _out=$(sh "$REPO_ROOT/scripts/common/rootfs-run.sh" "$_r" -- /bin/sh /pgb-poc-probe.sh 2>&1 | tail -1)
    _libs=$(poc_trace "$_r" "/$_base" pgb-poc-probe.sh)
    printf '    %-20s %-6s %-10s %s\n' "$name" "$libc" "${_out:-<none>}" "${_libs:-none}"
    printf '%s\n' "$name: $_out | $_libs" >> "$POC_OUT/observation.txt"
    poc_unstage_extras "$_r" "$@"
    rm -f "$_r/$_base" "$_r/pgb-poc-probe.sh"
  done < "$REPO_ROOT/scripts/common/rootfs-images.txt"
}

# Shared objects and gconv data the binary itself opened, attributed by pid.
poc_trace() { # rootfs in-root-binary script-name
  command -v strace >/dev/null 2>&1 || { printf ''; return; }
  _t=$(mktemp) || { printf ''; return; }
  strace -f -e trace=openat,open,execve -o "$_t" \
    sh "$REPO_ROOT/scripts/common/rootfs-run.sh" "$1" -- /bin/sh "/${3:-pgb-poc-test.sh}" \
    >/dev/null 2>&1
  awk -v want="$2" '
    { pid = $1 }
    $0 ~ ("execve\\(\"" want "\"") { target = pid; seen = 1; next }
    seen && pid == target && /open(at)?\(/ && !/ENOENT|= -1/ { print }
  ' "$_t" | grep -oE '"[^"]*\.so[^"]*"' | tr -d '"' | sort -u | tr '\n' ' '
  rm -f "$_t"
}

# Host DATA the binary read: glibc locale files, gconv configuration,
# nsswitch.conf, terminfo. Reported, never asserted -- see poc_matrix.
poc_trace_data() { # rootfs in-root-binary script-name
  command -v strace >/dev/null 2>&1 || { printf ''; return; }
  _t=$(mktemp) || { printf ''; return; }
  strace -f -e trace=openat,open,execve -o "$_t" \
    sh "$REPO_ROOT/scripts/common/rootfs-run.sh" "$1" -- /bin/sh "/${3:-pgb-poc-test.sh}" \
    >/dev/null 2>&1
  awk -v want="$2" '
    { pid = $1 }
    $0 ~ ("execve\\(\"" want "\"") { target = pid; seen = 1; next }
    seen && pid == target && /open(at)?\(/ && !/ENOENT|= -1/ { print }
  ' "$_t" | grep -oE '"[^"]*gconv[^"]*"|"/usr/lib/locale[^"]*"|"/etc/nsswitch.conf"|"[^"]*terminfo[^"]*"' \
    | tr -d '"' | sed 's|/usr/lib/locale/.*|/usr/lib/locale/*|; s|.*gconv.*|gconv-cfg|; s|.*terminfo.*|terminfo|' \
    | sort -u | tr '\n' ' '
  rm -f "$_t"
}

poc_inspect() { # binary
  printf '\n  the binary:\n'
  printf '    %-22s %s\n' size "$(wc -c < "$1") bytes"
  printf '    %-22s %s\n' file "$(file -b "$1" | cut -c1-84)"
  printf '    %-22s %s\n' PT_INTERP "$(readelf -l "$1" 2>/dev/null | grep -q INTERP && echo present || echo absent)"
  printf '    %-22s %s\n' DT_NEEDED "$(readelf -d "$1" 2>/dev/null | grep -c NEEDED)"
  printf '    %-22s %s\n' 'pgb runtime linked' \
    "$(strings -a "$1" 2>/dev/null | grep -qx 'pgb-runtime' && echo yes || echo NO)"
}

poc_finish() {
  printf '\n'
  printf -- '---------------------------------------------------------------\n'
  printf 'pass=%s fail=%s skip=%s\n' "$POC_PASS" "$POC_FAIL" "$POC_SKIP"
  if [ "$POC_FAIL" -gt 0 ]; then printf 'VERDICT: %s did NOT meet expectation.\n' "$POC_NAME"; exit 1; fi
  if [ "$POC_PASS" = 0 ]; then printf 'VERDICT: nothing was measured.\n'; exit 2; fi
  printf 'VERDICT: %s builds and works across the matrix.\n' "$POC_NAME"
  exit 0
}
