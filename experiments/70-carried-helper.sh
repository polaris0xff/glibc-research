#!/bin/sh
# 70-carried-helper.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# `pgb`'s language is decided by one constraint, written in
# docs/design/toolchain.md: whatever the driver is written in has to EXIST
# inside the pinned build environment and inside every target rootfs `pgb
# verify` reaches, and `sh` is the only thing guaranteed to be there.
#
# TODO/toolchain.md T-011 records that constraint with an explicit warning
# attached: "⚠ Untested assumption -- whether a static Rust or Zig helper
# could simply be CARRIED IN has not been tried, and if it can, the constraint
# weakens considerably."
#
# So: DOES A CARRIED-IN HELPER RUN?
#
# ⭐ The question is not academic and it is not about Rust. It is the same
# question this whole project exists to answer, asked about its own tooling: a
# helper that runs on eleven foreign userlands without anything installed
# there is exactly what `pgb` produces. If the answer is yes, then "sh is the
# only thing that exists there" stops being a constraint on the LANGUAGE and
# becomes a constraint on DELIVERY -- which is a different, and much cheaper,
# problem.
#
# -- ARMS -------------------------------------------------------------------
#
#   sh                a POSIX shell script. The incumbent, and the baseline
#                     every other arm has to match to be worth considering.
#   c-plain-static    gcc -static. The control that says what a naive static
#                     binary does here.
#   c-pgb             the same C, built by pgb. The positive control: this one
#                     is already known to run on 11 of 11.
#   rust-gnu-static   rustc, x86_64-unknown-linux-gnu, +crt-static
#   rust-musl-static  rustc, x86_64-unknown-linux-musl
#
# ⛔ EVERY ARM DOES THE SAME NON-TRIVIAL WORK, and it is chosen to be what a
# PLANNER helper would actually do rather than what is easy to make pass:
# read a file from the filesystem it landed on, parse it, and report. A
# helper that only prints a constant would pass everywhere and prove nothing
# about the work `docs/design/toolchain.md` wants moved out of shell --
# "dependency graph resolution, ELF analysis, and parsing package metadata".
#
# ⛔ AND EVERY ARM PROVES IT RAN. Each prints a marker line that the harness
# greps for, on top of exiting 0. docs/methodology/experiments.md: "an absence
# is not a zero. A probe that found nothing may have been looking in the wrong
# place." A silent exit 0 from a binary that never started would otherwise
# read as a pass.
#
# -- NUMBERING --------------------------------------------------------------
#
# 63- and 64- are named by open entries (T-013 developer friction, T-021
# nix-appimage) and a number is never reused, so this takes 70- and leaves
# them free.
#
# Exit: 0 the measurement ran and matched, 1 it ran and did not, 2 could not run.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "70 - can a helper written in a real language be CARRIED IN?"

WORK="$EXP_OUT/build"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
RESULT="$EXP_OUT/RESULT.txt"

MARKER="pgb-carried-helper-ok"

# ---------------------------------------------------------------------------
# The subject, in three languages. Same job in each: open /etc/os-release on
# whatever filesystem the helper landed on, count how many KEY=VALUE lines it
# has, and print the marker with that count. On a rootfs that has no
# /etc/os-release the count is 0 and that is still a run -- the marker is what
# is asserted, never the count.
# ---------------------------------------------------------------------------
cat > "$WORK/helper.c" <<'EOF'
#include <stdio.h>
#include <string.h>
int main(void) {
    FILE *f = fopen("/etc/os-release", "r");
    int n = 0;
    if (f) {
        char line[512];
        while (fgets(line, sizeof line, f))
            if (strchr(line, '=')) n++;
        fclose(f);
    }
    printf("pgb-carried-helper-ok c %d\n", n);
    return 0;
}
EOF

cat > "$WORK/helper.rs" <<'EOF'
fn main() {
    let n = std::fs::read_to_string("/etc/os-release")
        .map(|s| s.lines().filter(|l| l.contains('=')).count())
        .unwrap_or(0);
    println!("pgb-carried-helper-ok rust {}", n);
}
EOF

cat > "$WORK/helper.sh" <<'EOF'
#!/bin/sh
n=0
if [ -r /etc/os-release ]; then
  n=$(grep -c '=' /etc/os-release 2>/dev/null || echo 0)
fi
printf 'pgb-carried-helper-ok sh %s\n' "$n"
EOF
chmod +x "$WORK/helper.sh"

# ---------------------------------------------------------------------------
# Build the arms. An arm that cannot be built is SKIPPED, never silently
# dropped: a missing row and a failing row are different findings.
# ---------------------------------------------------------------------------
ARMS=""
arm_add() { ARMS="$ARMS $1"; }

printf -- '-- building the arms -----------------------------------------\n'

# sh
cp "$WORK/helper.sh" "$WORK/a-sh"
arm_add "sh:$WORK/a-sh"
printf '  built    %-18s %s\n' "sh" "POSIX shell script"

# plain gcc -static
if cc -static -O2 -o "$WORK/a-c-plain" "$WORK/helper.c" 2>"$WORK/c-plain.log"; then
  arm_add "c-plain-static:$WORK/a-c-plain"
  printf '  built    %-18s %s bytes\n' "c-plain-static" "$(wc -c < "$WORK/a-c-plain")"
else
  exp_skip "build c-plain-static" "see $WORK/c-plain.log"
fi

# the same C through pgb
if "$REPO_DIR/pgb" --engine host build -- \
     sh -c "\$CC -O2 -o $WORK/a-c-pgb $WORK/helper.c" >"$WORK/c-pgb.log" 2>&1 \
   && [ -f "$WORK/a-c-pgb" ]; then
  arm_add "c-pgb:$WORK/a-c-pgb"
  printf '  built    %-18s %s bytes\n' "c-pgb" "$(wc -c < "$WORK/a-c-pgb")"
else
  exp_skip "build c-pgb" "see $WORK/c-pgb.log"
fi

# rust, glibc target, statically linked
if command -v rustc >/dev/null 2>&1; then
  if rustc -O -C target-feature=+crt-static --edition 2021 \
        -o "$WORK/a-rust-gnu" "$WORK/helper.rs" >"$WORK/rust-gnu.log" 2>&1; then
    arm_add "rust-gnu-static:$WORK/a-rust-gnu"
    printf '  built    %-18s %s bytes\n' "rust-gnu-static" "$(wc -c < "$WORK/a-rust-gnu")"
  else
    exp_skip "build rust-gnu-static" "see $WORK/rust-gnu.log"
  fi

  if rustc -O --target x86_64-unknown-linux-musl --edition 2021 \
        -o "$WORK/a-rust-musl" "$WORK/helper.rs" >"$WORK/rust-musl.log" 2>&1; then
    arm_add "rust-musl-static:$WORK/a-rust-musl"
    printf '  built    %-18s %s bytes\n' "rust-musl-static" "$(wc -c < "$WORK/a-rust-musl")"
  else
    exp_skip "build rust-musl-static" "target not installed; see $WORK/rust-musl.log"
  fi
else
  exp_skip "build rust arms" "no rustc on this machine"
fi

[ -n "$ARMS" ] || { printf 'nothing could be built\n'; exit 2; }
printf '\n'

# ---------------------------------------------------------------------------
# The targets: the eleven pinned rootfs PLUS the pinned build environment.
#
# ⭐ The build environment is in the list on purpose. It is the one place
# `pgb build` re-enters itself, so a helper that runs on eleven distributions
# but not there would still not answer T-011's question.
# ---------------------------------------------------------------------------
TARGETS=""
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  [ -d "$ROOTFS_DIR/$name" ] && TARGETS="$TARGETS $name:$libc"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"
[ -d "$ROOTFS_DIR/pgb-env-debian12" ] && TARGETS="$TARGETS pgb-env-debian12:glibc"

[ -n "$TARGETS" ] || { printf 'no rootfs fetched: "./pgb" rootfs fetch\n'; exit 2; }

# ---------------------------------------------------------------------------
# Run every arm on every target.
# ---------------------------------------------------------------------------
{
  printf 'experiment 70 - can a helper written in a real language be carried in?\n'
  printf 'date (UTC)   : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'host kernel  : %s\n' "$(uname -sr)"
  printf 'rustc        : %s\n' "$(rustc --version 2>/dev/null || echo absent)"
  printf 'cc           : %s\n' "$({ ${CC:-cc} --version 2>/dev/null || echo none; } | head -1)"
  printf '\n'
  printf '%-22s %-6s' 'TARGET' 'LIBC'
  for a in $ARMS; do printf ' %-17s' "${a%%:*}"; done
  printf '\n'
} > "$RESULT"

printf -- '-- running -----------------------------------------------------\n'
printf '%-22s %-6s' 'TARGET' 'LIBC'
for a in $ARMS; do printf ' %-17s' "${a%%:*}"; done
printf '\n'

for t in $TARGETS; do
  tname=${t%%:*}; tlibc=${t#*:}
  root="$ROOTFS_DIR/$tname"
  row=$(printf '%-22s %-6s' "$tname" "$tlibc")
  for a in $ARMS; do
    aname=${a%%:*}; apath=${a#*:}
    base=$(basename "$apath")
    cp "$apath" "$root/$base" 2>/dev/null || { row="$row $(printf '%-17s' 'copy-failed')"; continue; }
    out=$("$REPO_DIR/pgb" rootfs run "$root" -- "/$base" 2>/dev/null)
    st=$?
    rm -f "$root/$base"
    # ⛔ Exit 0 alone is not a pass. The marker has to be in the output, or a
    # binary that never started would read as a run that succeeded.
    if [ "$st" = 0 ] && printf '%s' "$out" | grep -q "$MARKER"; then
      cell=ok
    elif [ "$st" = 0 ]; then
      cell="exit0-NO-MARKER"
    else
      case $st in
        13[0-9]|1[4-6][0-9]) cell="SIG$((st-128))" ;;
        *)                   cell="exit$st" ;;
      esac
    fi
    row="$row $(printf '%-17s' "$cell")"
  done
  printf '%s\n' "$row"
  printf '%s\n' "$row" >> "$RESULT"
done

printf '\n'

# ---------------------------------------------------------------------------
# The assertions.
#
# ⛔ WHAT IS ASSERTED IS THE COMPARISON, NOT THE RUST ARM ON ITS OWN. T-011
# does not ask "does Rust work"; it asks whether a carried-in helper is as
# available as `sh` is. So the bar every non-sh arm is held to is: it runs on
# every target `sh` runs on. An arm that runs on ten of eleven has answered
# the question NO just as clearly as one that runs on none.
# ---------------------------------------------------------------------------
n_targets=$(printf '%s\n' $TARGETS | grep -c .)

# ⛔ Find the header by CONTENT, not by line number. An earlier version keyed
# off NR==6 and would have silently counted zero the moment a line was added
# to the preamble -- which reads as "no arm runs anywhere", the most alarming
# possible result, from a formatting change.
col_ok() { # armname -> count of `ok` cells in that column
  awk -v want="$1" '
    $1 == "TARGET" { for (i = 3; i <= NF; i++) if ($i == want) col = i; hdr = 1; next }
    hdr && col && $col == "ok" { n++ }
    END { print n + 0 }
  ' "$RESULT"
}

printf -- '-- assertions --------------------------------------------------\n'
for a in $ARMS; do
  aname=${a%%:*}
  n=$(col_ok "$aname")
  case "$aname" in
    sh) exp_check "sh runs on every target (the baseline)" "$n" "$n_targets" ;;
    c-pgb) exp_check "c-pgb runs on every target (positive control)" "$n" "$n_targets" ;;
    *) printf '  --    %-46s = %s of %s\n' "$aname runs on (observed, not asserted)" "$n" "$n_targets" ;;
  esac
done

printf '\n'
exp_note "The observed rows above are the answer to T-011. An arm matching sh's"
exp_note "count means a helper in that language can be carried in; anything less"
exp_note "means the constraint in docs/design/toolchain.md still binds."
printf '\n'
printf 'full table: %s\n' "$RESULT"

exp_finish
