#!/bin/sh
# 10-probe-host.sh
#
# QUESTION: can this host run the benchmark, and what is it? Every number this
# repository publishes is conditional on the answer, so it is captured before
# anything is measured rather than reconstructed afterwards.
#
# ORACLE: the kernel and the container runtime are asked directly. Nothing is
# inferred from the presence of a binary on PATH -- a `docker` client with no
# daemon behind it answers `--version` happily and fails on the first real
# command, which is why the probe is `docker info`.
#
# PINNED INPUTS
#   none -- this describes the host, so it has no inputs to pin
#
# EXIT CODES
#   0  the host can run the benchmark
#   1  it ran and the host cannot (no runtime, or too little disk)
#   2  the measurement could not run
set -u

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/.." && pwd)
OUT="$HERE/out/10-probe-host.txt"
mkdir -p "$HERE/out" || exit 2

rc=0
# ⛔ THE BODY IS REDIRECTED, NOT PIPED, AND THAT IS LOAD-BEARING.
#
# `{ ...; } | tee "$OUT"` runs the body in a SUBSHELL, so an `rc=1` set inside
# it is lost and the script exits 0 -- reporting a detected defect as a pass.
# Observed here on 2026-09-01 in this script's first run. A redirection creates
# no subshell, so the exit code survives; the output is echoed afterwards.
{
    echo "=== conditions ==="
    echo "date          $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "host          $(uname -srm)"
    echo "cpus          $(nproc 2>/dev/null || echo '-')"
    echo "cpu model     $(grep -m1 'model name' /proc/cpuinfo 2>/dev/null | cut -d: -f2- | sed 's/^ *//' || echo '-')"
    echo "memory        $(awk '/MemTotal/{printf "%.1f GiB", $2/1048576}' /proc/meminfo 2>/dev/null || echo '-')"
    echo "disk free     $(df -Ph "$ROOT" 2>/dev/null | awk 'NR==2{print $4}' || echo '-')"
    echo

    echo "=== container runtime ==="
    runtime=""
    for c in docker podman; do
        if command -v "$c" >/dev/null 2>&1; then
            if v=$("$c" info --format '{{.ServerVersion}}' 2>/dev/null) && [ -n "$v" ]; then
                echo "$c            server $v   (responds to \`info\`, so a daemon is really there)"
                [ -z "$runtime" ] && runtime="$c"
            else
                echo "$c            present on PATH but \`info\` failed: no usable daemon"
            fi
        else
            echo "$c            not installed"
        fi
    done
    if [ -z "$runtime" ]; then
        echo
        echo "RESULT: FAIL -- no working container runtime. Install Docker or Podman."
        rc=1
    fi
    echo

    echo "=== supporting tools ==="
    for t in git curl python3 sha256sum; do
        if command -v "$t" >/dev/null 2>&1; then
            printf '%-14s%s\n' "$t" "$("$t" --version 2>&1 | head -1)"
        else
            printf '%-14s%s\n' "$t" "MISSING"
            [ "$t" = git ] || [ "$t" = curl ] && rc=1
        fi
    done
    echo

    echo "=== disk headroom ==="
    # Several images, eight allocator source trees and a 65 MB corpus. A run
    # that dies on ENOSPC half way through leaves a dataset that looks partial
    # rather than failed, which is the worst of the possible outcomes.
    avail_kb=$(df -Pk "$ROOT" 2>/dev/null | awk 'NR==2{print $4}')
    if [ -n "$avail_kb" ]; then
        gb=$((avail_kb / 1024 / 1024))
        echo "available     ${gb} GiB"
        if [ "$gb" -lt 20 ]; then
            echo "RESULT: FAIL -- 20 GiB is the recommended minimum for a full suite."
            rc=1
        fi
    else
        echo "available     could not determine"
    fi
    echo

    echo "=== what this probe cannot tell you ==="
    echo "- whether the CPU is shared with other tenants, which is the single"
    echo "  largest source of variance in the timings and is not observable"
    echo "  from inside the machine;"
    echo "- whether frequency scaling or thermal throttling is active. On a"
    echo "  cloud runner the governor is usually not exposed at all:"
    echo "    governor    $(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo 'not exposed')"
    echo "- anything about the architecture this host is NOT. Cells for another"
    echo "  architecture need a native runner; alloc-bench skips them by default."
    echo
    [ "$rc" -eq 0 ] && echo "RESULT: PASS -- this host can run the benchmark."
} > "$OUT" 2>&1

cat "$OUT"
exit "$rc"
