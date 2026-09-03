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

# ⚠ RESULT.txt IS A CONVENTION, NOT AN AUTOMATION, and it is written down here
# because it was folklore. Nothing in this file creates it: every tracked
# `evidence/poc/*/RESULT.txt` is the POC's own stdout, saved by hand:
#
#     sh poc/NN-name/run.sh > evidence/poc/NN-name/RESULT.txt 2>&1
#
# ⛔ So a POC that is re-run without that redirect leaves a RESULT.txt
# describing the PREVIOUS run, beside per-environment logs describing this
# one. Do the redirect, or the two halves of the evidence disagree and nothing
# checks them.
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
  poc_record_env
  printf -- '---------------------------------------------------------------\n'
}

# ⛔ WHICH TOOLCHAIN BUILT THIS. A POC's RESULT.txt used to say what it built
# and how it behaved on eleven environments, and nothing at all about the
# compiler and libc that produced the binary -- so "do the POCs still pass at
# the new pin?" could not be answered from committed evidence, only by running
# them again. T-070 hit exactly that: `PGB_ENV_NAME=... sh poc/*/run.sh` built
# against the INCUMBENT on a machine running dockerd and no POC output said so.
#
# ⚠ Read out of the environment on disk, never from cfg.go: the question is
# what BUILT the binary, and cfg.go only says what pgb would build with. They
# differ exactly when it matters -- when PGB_ENV_* names a candidate.
POC_ENV_GCC=""
poc_record_env() {
  _pe_root=$("$PGB" env info 2>/dev/null | awk '$1=="root"{print $2; exit}')
  _pe_stamp="$_pe_root/.pgb-env"
  if [ -n "$_pe_root" ] && [ -f "$_pe_stamp" ]; then
    POC_ENV_GCC=$(sed -n 's/^gcc: *//p' "$_pe_stamp" | awk '{print $NF}')
    printf 'build env     : %s\n' "$_pe_root"
    printf 'build image   : %s @ %s\n' \
      "$(sed -n 's/^image: *//p'  "$_pe_stamp")" \
      "$(sed -n 's/^digest: *//p' "$_pe_stamp")"
    printf 'build gcc     : %s\n' "$(sed -n 's/^gcc: *//p'   "$_pe_stamp")"
    printf 'build glibc   : %s\n' "$(sed -n 's/^glibc: *//p' "$_pe_stamp")"
  else
    # ⚠ An absence is not a zero. The docker engine builds from an image
    # rather than from a rootfs on disk, so there is no stamp to read and the
    # answer is "not recorded here", not "no environment".
    printf 'build env     : NOT RECORDED -- no .pgb-env under %s\n' "${_pe_root:-<unknown>}"
    printf 'build image   : %s @ %s (from pgb env info, not from a stamp)\n' \
      "$("$PGB" env info 2>/dev/null | awk '$1=="image"{print $2; exit}')" \
      "$("$PGB" env info 2>/dev/null | awk '$1=="digest"{print $2; exit}')"
  fi
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
# ⚠ POC_PGB_FLAGS lets a POC ask for an OPT-IN mechanism -- --embed-cacert,
# --embed-terminfo, --embed-locale -- for its own builds. It is deliberately
# not a per-call argument: a POC that used a flag for the final link and not
# for the dependencies it links in would be measuring two different toolchains
# and calling the result one.
poc_in_env() { "$PGB" --bind "$WORK" ${POC_PGB_FLAGS:-} build -- /bin/sh -c "$1"; }

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
  # ⛔ A MISSING FUNCTIONAL TEST IS A FALSE PASS, AND IT WAS SILENT.
  # Below, `poc_functional_test > .../pgb-poc-test.sh` writes an EMPTY file
  # when the function is undefined. `sh` on an empty script exits 0, so
  # _res=ok, `poc_check ... ok` passes, the trace of that empty script finds
  # no objects, and the POC reports ELEVEN GREEN ROWS having executed nothing
  # at all. Found in poc/60-leveldb, which omitted the function -- and it
  # would have certified any POC that did.
  if ! command -v poc_functional_test >/dev/null 2>&1; then
    printf '\n  ⛔ this POC defines no poc_functional_test(), so the matrix\n'
    printf '     below would assert on an empty script and pass. Refusing.\n'
    POC_FAIL=$((POC_FAIL+1))
    return 1
  fi
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
    "$REPO_ROOT/pgb" rootfs run "$_r" -- /bin/sh /pgb-poc-test.sh \
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
  # ⛔ TRUNCATE, DO NOT APPEND. The rows below are written with >>, so before
  # this line a second run of the POC left observation.txt holding BOTH runs
  # back to back -- 22 rows for an 11-environment matrix, the older half
  # describing a binary that no longer exists. Same defect class as the POCs
  # that appended to the target's /etc/hosts. docs/history/corrections.md.
  : > "$POC_OUT/observation.txt"
  # ⛔ A MISSING PROBE MUST NOT READ AS A MEASUREMENT. Without this check the
  # loop below still runs, `poc_observation_probe` fails with "not found" once
  # per environment, and every row prints OUTCOME `<none>` and HOST OBJECTS
  # `none` -- which is indistinguishable, in the committed table, from eleven
  # environments that were measured and found clean. Found in poc/60-leveldb,
  # which omitted the function. docs/methodology/experiments.md: an absence is
  # not a zero.
  if ! command -v poc_observation_probe >/dev/null 2>&1; then
    printf '\n  OBSERVATION -- %s: NOT MEASURED\n' "$_label"
    printf '    ⛔ this POC defines no poc_observation_probe(), so nothing was\n'
    printf '       observed. That is not the same as observing nothing.\n'
    printf 'NOT MEASURED: this POC defines no poc_observation_probe()\n' \
      >> "$POC_OUT/observation.txt"
    POC_SKIP=$((POC_SKIP+1))
    return 0
  fi
  printf '\n  OBSERVATION -- %s (measured, not asserted):\n' "$_label"
  printf '    %-20s %-6s %-10s %s\n' ENVIRONMENT LIBC OUTCOME 'HOST OBJECTS PULLED IN'
  while read -r ref name libc digest; do
    case "$ref" in ''|\#*) continue ;; esac
    _r="$ROOTFS_DIR/$name"
    [ -d "$_r" ] || continue
    cp "$_bin" "$_r/$_base" 2>/dev/null || continue
    poc_stage_extras "$_r" "$@"
    poc_observation_probe > "$_r/pgb-poc-probe.sh"
    _out=$("$REPO_ROOT/pgb" rootfs run "$_r" -- /bin/sh /pgb-poc-probe.sh 2>&1 | tail -1)
    _libs=$(poc_trace "$_r" "/$_base" pgb-poc-probe.sh)
    printf '    %-20s %-6s %-10s %s\n' "$name" "$libc" "${_out:-<none>}" "${_libs:-none}"
    printf '%s\n' "$name: $_out | $_libs" >> "$POC_OUT/observation.txt"
    poc_unstage_extras "$_r" "$@"
    rm -f "$_r/$_base" "$_r/pgb-poc-probe.sh"
  done < "$REPO_ROOT/scripts/common/rootfs-images.txt"
}

# Shared objects and gconv data the binary itself opened, attributed by pid.
#
# ⛔ A SHARED OBJECT ENDS IN .so OR .so.N. Matching ".so" anywhere in the path
# also matches `/etc/ld.so.cache`, which is an index and not an object, and
# poc_matrix ASSERTS on this value -- so a binary that opened the cache and
# loaded nothing would have failed a POC over a file it only read. The wrong
# expression reached committed evidence: `evidence/poc/10-gawk/RESULT.txt`
# lists /etc/ld.so.cache on all seven glibc rows of the observation table. No
# verdict there was wrong, because a real object sits beside it on every one of
# those rows. docs/history/corrections.md, instrument defects.
poc_trace() { # rootfs in-root-binary script-name
  command -v strace >/dev/null 2>&1 || { printf ''; return; }
  _t=$(mktemp) || { printf ''; return; }
  strace -f -e trace=openat,open,execve -o "$_t" \
    "$REPO_ROOT/pgb" rootfs run "$1" -- /bin/sh "/${3:-pgb-poc-test.sh}" \
    >/dev/null 2>&1
  awk -v want="$2" '
    { pid = $1 }
    $0 ~ ("execve\\(\"" want "\"") { target = pid; seen = 1; next }
    seen && pid == target && /open(at)?\(/ && !/ENOENT|= -1/ { print }
  ' "$_t" | grep -oE '"[^"]*\.so(\.[0-9]+)*"' | tr -d '"' | sort -u | tr '\n' ' '
  rm -f "$_t"
}

# Host DATA the binary read: glibc locale files, gconv configuration,
# nsswitch.conf, terminfo. Reported, never asserted -- see poc_matrix.
poc_trace_data() { # rootfs in-root-binary script-name
  command -v strace >/dev/null 2>&1 || { printf ''; return; }
  _t=$(mktemp) || { printf ''; return; }
  strace -f -e trace=openat,open,execve -o "$_t" \
    "$REPO_ROOT/pgb" rootfs run "$1" -- /bin/sh "/${3:-pgb-poc-test.sh}" \
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
  # ⭐ THE COMPILER THE BINARY ITSELF CARRIES, and it is ASSERTED against the
  # environment's own recorded gcc rather than printed for someone to read.
  # ⛔ This is the check that caught T-070 arm 5 building against the incumbent
  # while every other line of output looked right: `.comment` said
  # `GCC: (Debian 12.2.0-14+deb12u1)` where the named environment carried
  # 14.2.0. Nothing else in a POC's output can tell those two apart.
  # ⚠ A binary with no .comment reports `absent` and is NOT asserted — a
  # stripped or `-fno-ident` link is not a wrong compiler.
  _pi_cc=$(readelf -p .comment "$1" 2>/dev/null \
           | sed -n 's/.*GCC: (\{0,1\}[^)]*)\{0,1\} \([0-9][0-9.]*\).*/\1/p' | head -1)
  printf '    %-22s %s\n' '.comment gcc' "${_pi_cc:-absent}"
  poc_check_built_by_env "$1"
}

# ⭐ THE TOOLCHAIN ASSERTION ON ITS OWN, so a POC that does its own inspection
# can still make it.
#
# ⛔ WHY IT IS SPLIT OUT, AND IT IS A GAP THAT WAS FOUND BY COUNTING RATHER
# THAN BY READING. This assertion is the ONLY thing in a POC's output that can
# tell a binary built by the named environment from one built by the incumbent
# — it is what caught T-070 arm 5, where `.comment` said `GCC: (Debian
# 12.2.0-14+deb12u1)` and every other line looked right. ⚠ It lived inside
# `poc_inspect`, and THREE of the ten POCs never call `poc_inspect`:
# `70-sqlite-extensions`, `80-mlt` and `91-qt-xcb` each do their own PT_INTERP
# and DT_NEEDED checks instead. So three of the ten reported green having never
# compared their binary's compiler to the environment's.
#
# ⚠ A binary with no `.comment` SKIPS rather than passes — a stripped or
# `-fno-ident` link is not a wrong compiler, and a silent pass would be the
# absence-as-zero this harness exists to refuse.
poc_check_built_by_env() { # binary
  _cb_cc=$(readelf -p .comment "$1" 2>/dev/null \
           | sed -n 's/.*GCC: (\{0,1\}[^)]*)\{0,1\} \([0-9][0-9.]*\).*/\1/p' | head -1)
  if [ -z "$_cb_cc" ]; then
    poc_skip "built by the environment's own gcc" "no .comment in $(basename "$1")"
    return 0
  fi
  if [ -z "$POC_ENV_GCC" ]; then
    poc_skip "built by the environment's own gcc" "the environment recorded no gcc"
    return 0
  fi
  poc_check "built by the environment's own gcc" "$_cb_cc" "$POC_ENV_GCC"
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
