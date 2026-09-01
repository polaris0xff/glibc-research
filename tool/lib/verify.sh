# tool/lib/verify.sh -- part of `pgb`. Sourced by it, never executed.
#
# ⛔ SOURCED, NOT RUN. `pgb build` re-enters itself inside the build
# environment as `pgb __inner-build`, and `pgb verify` enters every target
# rootfs. Both depend on that being ONE process: a library executed as a child
# would put a shell between `pgb` and the thing it is measuring, and the
# PGB_OPT_* handoff in `../../pgb` exists precisely because that boundary is
# already where options got lost once. So: `. "$PGB_SELF/tool/lib/verify.sh"`,
# no shebang, no `set -e`, no exec.
#
# ⚠ Every path here resolves from $PGB_SELF, which `../../pgb` sets from its
# own location. Nothing resolves from the caller's working directory.
#
# Holds: `pgb verify`; the two pid-attributed strace readers the chroot arm
#        rests on; and the carried-in ptrace tracer the docker arm rests on,
#        because strace cannot follow a process into the daemon's namespaces.
#
# SPDX-License-Identifier: MIT


# ---------------------------------------------------------------------------
# The carried-in tracer, for the engines whose target this process cannot see.
#
# ⭐ WHY THIS EXISTS. Criterion 2 is decided from a syscall trace. Under the
# chroot engine `strace` runs OUTSIDE the target and works, because the target
# is an ordinary child. Under docker the subject lives in the daemon's
# namespaces where that reader cannot follow it, and `strace` cannot be
# installed into the eleven target images -- they are pinned by digest and
# adding a package to one changes what every result about it describes.
#
# So the tracer is carried IN, which experiments/70- measured is possible for
# exactly this class of artefact. tool/runtime/pgb-trace.c is the tracer.
#
# ⚠ PLAIN `-static`, not the full pgb link. The tracer calls ptrace, fork,
# waitpid and fprintf and touches neither NSS, nor iconv, nor setlocale, so
# none of pgb's mechanisms apply to it -- and experiments/70-'s
# `c-plain-static` arm measured that a static binary of this shape runs on 12
# of 12. Linking it the full way would make the instrument depend on the thing
# it is measuring.
build_tracer() {
  rd=$(runtime_dir); mkdir -p "$rd" 2>/dev/null
  src="$PGB_SELF/tool/runtime/pgb-trace.c"
  out="$rd/pgb-trace"
  if [ ! -x "$out" ] || [ "$src" -nt "$out" ]; then
    ${CC:-cc} -O2 -static -o "$out" "$src" >/dev/null 2>&1 || { printf ''; return 1; }
  fi
  printf '%s' "$out"
}

# Classify one traced path. ⛔ A SHARED OBJECT ENDS IN .so OR .so.N -- matching
# the substring ".so" also matches /etc/ld.so.cache, which is an index and not
# an object, and this value decides pass/fail. Same rule as
# trace_host_objects(); docs/history/corrections.md, instrument defects.
trace_classify() {  # reads the tracer's stderr on stdin; prints "libs|data"
  awk '
    /^open / {
      p = substr($0, 6)
      if (p ~ /\.so$/ || p ~ /\.so\.[0-9]+$/) { libs[p] = 1; next }
      if (p ~ /gconv/)            { data["gconv-cfg"] = 1; next }
      if (p ~ /^\/usr\/lib\/locale/) { data["/usr/lib/locale/*"] = 1; next }
      if (p == "/etc/nsswitch.conf")  { data[p] = 1; next }
      if (p ~ /terminfo/)         { data["terminfo"] = 1; next }
    }
    END {
      l = ""; for (k in libs) l = l k " "
      d = ""; for (k in data) d = d k " "
      printf "%s|%s", l, d
    }
  '
}

# ---------------------------------------------------------------------------
# verify -- the part that refuses to accept `file` as an answer
# ---------------------------------------------------------------------------
cmd_verify() {
  bin="${1:-}"
  [ -n "$bin" ] && [ -f "$bin" ] || die "pgb verify NEEDS a binary" 2
  shift
  rc=0
  saw_unmeasured=0
  say "pgb verify: $bin"
  say ""
  say "  -- static inspection (NOT the success criterion) -------------------"
  printf '    %-22s %s\n' "size" "$(wc -c < "$bin") bytes"
  printf '    %-22s %s\n' "file" "$(file -b "$bin" 2>/dev/null | cut -c1-90)"
  interp=$(readelf -l "$bin" 2>/dev/null | grep -c 'INTERP' || true)
  printf '    %-22s %s\n' "PT_INTERP" "$([ "${interp:-0}" -gt 0 ] && echo present || echo absent)"
  needed=$(readelf -d "$bin" 2>/dev/null | grep -c 'NEEDED' || true)
  printf '    %-22s %s\n' "DT_NEEDED entries" "${needed:-0}"
  printf '    %-22s %s\n' "nssfix linked in" \
    "$(strings -a "$bin" 2>/dev/null | grep -qx 'pgb-runtime' && echo yes || echo 'NO -- not a pgb binary')"
  say ""
  say "    ⚠ None of the above is the test. A binary can satisfy every line"
  say "      here and still die on Arch. The next section is the test."
  say ""

  eng=$(pick_engine)
  case "$eng" in
    chroot|host) vmode=chroot ;;
    docker|podman) vmode=$eng ;;
  esac
  # ⚠ `host` has no bed of its own: there is nothing to enter. Fall back to
  # the chroot bed, which is what every committed number was measured through.
  [ "$eng" = host ] && vmode=chroot

  say "  -- runtime, across the pinned matrix -------------------------------"
  say "     engine: $vmode"
  list="$PGB_SELF/scripts/common/rootfs-images.txt"
  [ -f "$list" ] || die "missing $list" 2
  any=0
  printf '    %-20s %-6s %-8s %-26s %s\n' ENVIRONMENT LIBC RESULT 'HOST .so LOADED' 'HOST DATA READ'
  while read -r ref name libc digest; do
    case "$ref" in ''|\#*) continue ;; esac
    base=$(basename "$bin")

    if [ "$vmode" = chroot ]; then
      root="$PGB_ROOTFS_DIR/$name"
      [ -d "$root" ] || { printf '    %-20s %-6s %-8s %s\n' "$name" "$libc" "-" "not fetched"; continue; }
      any=1
      cp "$bin" "$root/$base" 2>/dev/null || { printf '    %-20s copy failed\n' "$name"; rc=1; continue; }
      if [ $# -gt 0 ]; then
        sh "$PGB_SELF/scripts/common/rootfs-run.sh" "$root" -- "/$base" "$@" >/dev/null 2>&1
      else
        sh "$PGB_SELF/scripts/common/rootfs-run.sh" "$root" -- "/$base" >/dev/null 2>&1
      fi
      st=$?
      libs=$(trace_host_objects "$root" "/$base" "$@")
      data=$(trace_host_data "$root" "/$base" "$@")
      rm -f "$root/$base"
    else
      # ⭐ THE ENGINE THE FLAG ALWAYS CLAIMED TO SELECT. A CI runner has no
      # CAP_SYS_ADMIN, so `unshare --mount` is unavailable and the chroot bed
      # cannot be entered at all -- which is why `pgb verify` could not run
      # there before this arm existed. docs/history/corrections.md C9.
      #
      # The image is the SAME digest the chroot bed unpacked, derived from the
      # same file, so the two arms describe the same environment rather than
      # two environments with one name.
      repo=$(printf '%s' "$ref" | sed 's/:[^:/]*$//')
      img="$repo@$digest"
      any=1
      bindir=$(cd "$(dirname "$bin")" && pwd)
      tracer=$(build_tracer)
      if [ -n "$tracer" ]; then
        tdir=$(dirname "$tracer")
        # --entrypoint: the tracer, and the subject is its only child, so
        # nothing of the image's choosing runs and no other process can open a
        # file and have it charged to the binary.
        # ⛔ BOUNDED, ALWAYS. A tracer defect must cost one row and a visible
        # `exit124`, never an unbounded wait. Measured on a CI runner: a
        # tracer that suppressed the tracee's SIGFPE instead of re-injecting
        # it looped forever, and the job sat on four of eleven rows for 19
        # minutes before anyone looked. The bug is fixed in pgb-trace.c; this
        # bound is here so the NEXT one is cheap.
        tout=$(timeout "${PGB_VERIFY_TIMEOUT:-120}" \
                 $vmode run --rm --cap-add=SYS_PTRACE \
                 -v "$bindir:/pgb-verify:ro" -v "$tdir:/pgb-tracer:ro" \
                 --entrypoint /pgb-tracer/pgb-trace "$img" \
                 -- "/pgb-verify/$base" "$@" 2>&1 >/dev/null)
        st=$?
        # ⛔ THE TRACER SAYS WHETHER IT LOOKED, AND THE CALLER KEYS ON THAT.
        # A tracer that failed to attach prints no paths, which is byte for
        # byte what a clean binary produces. Emptiness is therefore never
        # read as cleanliness -- only `status=traced` is.
        case "$tout" in
          *"status=traced"*)
            cls=$(printf '%s\n' "$tout" | trace_classify)
            libs=${cls%%|*}; data=${cls#*|} ;;
          *) libs=unmeasured; data=unmeasured ;;
        esac
      else
        # ⛔ `unmeasured`, NEVER `none`. docs/methodology/experiments.md: "an
        # absence is not a zero. A probe that found nothing may have been
        # looking in the wrong place." Reporting `none` here would turn "we
        # did not look" into "we looked and it was clean", which is the single
        # most damaging thing this column could say.
        timeout "${PGB_VERIFY_TIMEOUT:-120}" \
          $vmode run --rm -v "$bindir:/pgb-verify:ro" \
          --entrypoint "/pgb-verify/$base" "$img" "$@" >/dev/null 2>&1
        st=$?
        libs=unmeasured
        data=unmeasured
      fi
    fi

    case $st in
      0) res=ok ;;
      13[0-9]|1[4-6][0-9]) res="SIG$((st-128))"; rc=1 ;;
      *) res="exit$st"; rc=1 ;;
    esac
    printf '    %-20s %-6s %-8s %-26s %s\n' "$name" "$libc" "$res" "${libs:-none}" "${data:-none}"
    # ⛔ ONLY A HOST SHARED OBJECT IS A FAILURE. Reading host data is not:
    # glibc still opens /etc/nsswitch.conf under the NSS override, and a
    # program that honours the host's locale when the host has one is correct.
    # The property that matters is INDEPENDENCE -- working whether or not the
    # data is there -- and the musl rows, which have none of it, are what
    # demonstrate that.
    # ⛔ `unmeasured` is not a pass and is not a failure. It must not set rc --
    # a row nobody looked at is not evidence of a defect -- but it must not be
    # forgotten either, or the closing VERDICT would claim criterion 2 held on
    # a row where it was never checked.
    case "$libs" in
      ''|none)     : ;;
      unmeasured)  saw_unmeasured=1 ;;
      *)           rc=1 ;;
    esac
  done < "$list"
  [ "$any" = 1 ] || { say "    nothing to run against. sh scripts/common/fetch-rootfs.sh"; return 2; }
  say ""
  if [ "$rc" = 0 ] && [ "${saw_unmeasured:-0}" = 1 ]; then
    say "  VERDICT: ran correctly on every environment."
    say "  ⚠ CRITERION 2 WAS NOT CHECKED on at least one row. Where the"
    say "    host-object column reads 'unmeasured' the tracer did not attach,"
    say "    so that row says nothing about whether a host shared object was"
    say "    loaded. --cap-add=SYS_PTRACE is what it needs."
  elif [ "$rc" = 0 ]; then
    say "  VERDICT: ran on every fetched environment and loaded no host shared object."
  else
    say "  VERDICT: NOT portable as built. A non-zero exit or a host object"
    say "           above is a real failure, not a labelling quibble."
  fi
  return $rc
}

# Report shared objects and gconv/NSS data the binary itself opened.
#
# ⛔ A SHARED OBJECT ENDS IN .so OR .so.N. Matching the substring ".so"
# anywhere in the path also matches `/etc/ld.so.cache`, which is an index and
# not an object -- and the caller turns a non-empty result here into a FAILED
# verify. A binary that opened the cache and loaded nothing would have been
# reported as not portable because of a file it only read.
# docs/history/corrections.md, instrument defects.
trace_host_objects() {
  root="$1"; inner="$2"; shift 2
  command -v strace >/dev/null 2>&1 || { printf ''; return; }
  t=$(mktemp) || { printf ''; return; }
  strace -f -e trace=openat,open,execve -o "$t" \
    sh "$PGB_SELF/scripts/common/rootfs-run.sh" "$root" -- "$inner" "$@" >/dev/null 2>&1
  awk -v want="$inner" '
    { pid = $1 }
    $0 ~ ("execve\\(\"" want "\"") { target = pid; seen = 1; next }
    seen && pid == target && /open(at)?\(/ && !/ENOENT|= -1/ { print }
  ' "$t" | grep -oE '"[^"]*\.so(\.[0-9]+)*"' | tr -d '"' | sort -u | tr '\n' ' '
  rm -f "$t"
}

# Host DATA the binary read. Reported for information, never a failure.
trace_host_data() { # rootfs in-root-binary [args...]
  root="$1"; inner="$2"; shift 2
  command -v strace >/dev/null 2>&1 || { printf ''; return; }
  t=$(mktemp) || { printf ''; return; }
  strace -f -e trace=openat,open,execve -o "$t" \
    sh "$PGB_SELF/scripts/common/rootfs-run.sh" "$root" -- "$inner" "$@" >/dev/null 2>&1
  awk -v want="$inner" '
    { pid = $1 }
    $0 ~ ("execve\\(\"" want "\"") { target = pid; seen = 1; next }
    seen && pid == target && /open(at)?\(/ && !/ENOENT|= -1/ { print }
  ' "$t" | grep -oE '"[^"]*gconv[^"]*"|"/usr/lib/locale[^"]*"|"/etc/nsswitch.conf"|"[^"]*terminfo[^"]*"' \
    | tr -d '"' | sed 's|/usr/lib/locale/.*|/usr/lib/locale/*|; s|.*gconv.*|gconv-cfg|; s|.*terminfo.*|terminfo|' \
    | sort -u | tr '\n' ' '
  rm -f "$t"
}
