#!/bin/sh
# 30-ripgrep-default-allocator.sh
#
# QUESTION: does an UNMODIFIED ripgrep build use the system allocator on musl?
#
# Why it is worth asking: if it does not, then "build ripgrep on Alpine and
# measure it" produces a control group that is not the control. Every ratio in
# every table would then be wrong by whatever the substituted allocator beats
# musl by -- which is the largest single effect this project measures.
#
# ORACLE: ripgrep's own source at the pinned commit, read directly. Not its
# documentation, not its release notes, and not this project's belief about it.
# The check is structural (an attribute and a dependency table), so it survives
# the block being reworded or moved.
#
# PINNED INPUTS
#   ripgrep   the commit in allocators/allocators.lock.json (entry `ripgrep`)
#
# EXIT CODES
#   0  ripgrep uses the system allocator on musl -- the trap is gone
#   1  it does NOT (the state today, and the reason patch-rg exists)
#   2  the measurement could not run
set -u

HERE=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
ROOT=$(CDPATH='' cd -- "$HERE/.." && pwd)
OUT="$HERE/out/30-ripgrep-default-allocator.txt"
WORK=${WORK:-${TMPDIR:-/tmp}/alloc-tests-30}
mkdir -p "$HERE/out" || exit 2

command -v git >/dev/null 2>&1 || { echo "30: git not found" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "30: python3 not found" >&2; exit 2; }

LOCK="$ROOT/allocators/allocators.lock.json"
[ -r "$LOCK" ] || { echo "30: no lock file at $LOCK" >&2; exit 2; }

COMMIT=$(python3 -c "
import json,sys
d=json.load(open('$LOCK'))
e=d['entries'].get('ripgrep')
print(e['commit'] if e else '', end='')
") || exit 2
[ -n "$COMMIT" ] || { echo "30: the lock file has no ripgrep entry" >&2; exit 2; }
TAG=$(python3 -c "
import json
print(json.load(open('$LOCK'))['entries']['ripgrep']['reference'], end='')
")

SRC="$WORK/ripgrep"
if [ ! -d "$SRC/.git" ] || [ "$(git -C "$SRC" rev-parse HEAD 2>/dev/null)" != "$COMMIT" ]; then
    rm -rf "$SRC"
    sh "$ROOT/scripts/build/fetch-source.sh" \
        https://github.com/BurntSushi/ripgrep "$COMMIT" "$SRC" >/dev/null 2>&1 \
        || { echo "30: could not fetch ripgrep at $COMMIT" >&2; exit 2; }
fi

MAIN="$SRC/crates/core/main.rs"
CARGO="$SRC/Cargo.toml"
[ -r "$MAIN" ] && [ -r "$CARGO" ] || { echo "30: ripgrep's layout is not what this expects" >&2; exit 2; }

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
    echo "ripgrep       $TAG  $COMMIT"
    echo "source        $SRC"
    echo

    echo "=== does the source declare a #[global_allocator]? ==="
    n=$(grep -rl '#\[global_allocator\]' "$SRC" --include='*.rs' 2>/dev/null | grep -cv '/target/' || true)
    echo "files declaring one: $n"
    if [ "$n" -gt 0 ]; then
        rc=1
        grep -rn -B4 -A2 '#\[global_allocator\]' "$SRC" --include='*.rs' 2>/dev/null \
            | grep -v '/target/' | sed 's|'"$SRC"'/||'
    fi
    echo

    echo "=== does it depend on a third-party allocator crate? ==="
    if grep -qE 'jemallocator|mimalloc|snmalloc|rpmalloc|tcmalloc' "$CARGO"; then
        rc=1
        grep -nE -B2 'jemallocator|mimalloc|snmalloc|rpmalloc|tcmalloc' "$CARGO"
    else
        echo "none found in Cargo.toml"
    fi
    echo

    echo "=== what this means ==="
    if [ "$rc" -eq 1 ]; then
        cat <<'TEXT'
⛔ ripgrep SELECTS A THIRD-PARTY ALLOCATOR on musl targets.

An unmodified `cargo build --target x86_64-unknown-linux-musl` therefore does
NOT produce a system-allocator binary. Publishing that as "the Alpine system
allocator baseline" would put a third-party allocator in the control group.

This project removes the declaration, and the dependency, from EVERY cell
including the baseline, and then asserts the resulting count:
  crates/alloc-runner/src/patchrg.rs
The result is checked again against the built binary by `alloc-runner identify`,
which requires the baseline to contain musl's allocator and no candidate.

exit 1 is the expected outcome today. A future ripgrep that dropped this would
make this script exit 0, and the patch step could then be reconsidered --
which is the result you want and cannot get from prose.
TEXT
    else
        cat <<'TEXT'
ripgrep declares no #[global_allocator] and depends on no allocator crate at
this commit. The baseline trap does not apply to this version.

⚠ Do NOT remove the patch step on this basis alone: it also guarantees that a
cell has exactly the allocator the plan says, which is a property worth keeping
whatever upstream does.
TEXT
    fi
    echo
    echo "=== what this probe cannot tell you ==="
    echo "- whether the declaration is REACHED on a given target. It is behind a"
    echo "  cfg; this script reports the declaration, and the ELF check in"
    echo "  \`alloc-runner identify\` is what confirms the built binary."
} > "$OUT" 2>&1

cat "$OUT"
exit "$rc"
