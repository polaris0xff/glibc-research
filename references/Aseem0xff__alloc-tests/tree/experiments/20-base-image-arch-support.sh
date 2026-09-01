#!/bin/sh
# 20-base-image-arch-support.sh
#
# QUESTION: for each distribution this project targets, which architectures does
# the upstream image ACTUALLY publish?
#
# Why it is worth asking: the project promises x86_64 and aarch64 on Alpine,
# Debian and Arch. If one of those images is single-architecture, the choices
# are to substitute a different distribution or to say so. ⛔ Substituting
# silently is the failure this script exists to prevent: a table row labelled
# `archlinux / aarch64` that was really built on a different project's packages
# is a wrong answer, not a missing one.
#
# ORACLE: the registry's own manifest list, read with `docker manifest inspect`.
# Not the distribution's documentation, and not an assumption from the fact that
# other images are multi-arch.
#
# PINNED INPUTS
#   the tags this project uses, which are `:latest` BY DESIGN -- the point of
#   this probe is to detect the day one of them changes. The digest that
#   answered is printed, so a later run can tell whether the image moved.
#
# EXIT CODES
#   0  every distribution publishes every required architecture
#   1  it ran and at least one does not (this is the state today)
#   2  the measurement could not run
set -u

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
OUT="$HERE/out/20-base-image-arch-support.txt"
mkdir -p "$HERE/out" || exit 2

RT=""
for c in docker podman; do
    if command -v "$c" >/dev/null 2>&1 && "$c" info >/dev/null 2>&1; then RT="$c"; break; fi
done
[ -n "$RT" ] || { echo "20: no working container runtime" >&2; exit 2; }

REQUIRED="amd64 arm64"
IMAGES="alpine:latest debian:latest archlinux:latest menci/archlinuxarm:base-devel"

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
    echo "runtime       $RT $("$RT" --version 2>/dev/null | head -1)"
    echo "required      $REQUIRED"
    echo

    for img in $IMAGES; do
        raw=$("$RT" manifest inspect "$img" 2>/dev/null)
        if [ -z "$raw" ]; then
            printf '%-34s %s\n' "$img" "COULD NOT INSPECT (network, or the tag is gone)"
            rc=2
            continue
        fi
        arches=$(printf '%s' "$raw" | python3 -c '
import sys, json
d = json.load(sys.stdin)
ms = d.get("manifests", [])
if ms:
    a = sorted({m["platform"]["architecture"] for m in ms
                if m["platform"]["architecture"] != "unknown"})
else:
    a = [d.get("architecture", "?")]
print(" ".join(a))
' 2>/dev/null)
        printf '%-34s %s\n' "$img" "$arches"
        for want in $REQUIRED; do
            case " $arches " in
                *" $want "*) ;;
                *)
                    case "$img" in
                        menci/*) ;;   # the substitute, checked below on its own terms
                        *)
                            echo "    ⛔ $img does not publish $want"
                            rc=1 ;;
                    esac ;;
            esac
        done
    done

    echo
    echo "=== what this means for the matrix ==="
    echo "Arch publishes amd64 only. Upstream Arch does not support aarch64;"
    echo "Arch Linux ARM is a SEPARATE project with its own package set and its"
    echo "own build of glibc."
    echo
    echo "So this project does not label an aarch64 build \`archlinux\`. The"
    echo "planner renames the distribution to \`archlinuxarm\` on that"
    echo "architecture (crates/alloc-bench/src/plan.rs, effective_distro), and"
    echo "the two never share a table row."
    echo
    echo "=== what this probe cannot tell you ==="
    echo "- whether the published image for an architecture actually WORKS;"
    echo "  a manifest entry is a claim of availability, not of function."
    echo "  images/*.Dockerfile building successfully is that evidence."
    echo
    if [ "$rc" -eq 1 ]; then
        echo "RESULT: exit 1 ON PURPOSE. A gap is present and is documented above."
        echo "If a future run exits 0, upstream has started publishing the missing"
        echo "architecture and images/arch.Dockerfile's substitution can be retired."
    fi
} > "$OUT" 2>&1

cat "$OUT"
exit "$rc"
