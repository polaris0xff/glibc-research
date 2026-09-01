#!/bin/sh
# experiments/87-concurrent-build-options.sh
#
# ⛔ TWO `pgb build`s AT ONCE, WITH DIFFERENT OPTIONS. T-058.
#
# The defect: `make_wrappers` wrote ONE directory, `$PGB_STATE/bin`, and the
# chroot branch of `tool/lib/build.sh` bind-mounts `$PGB_STATE` INTO the build
# environment. The wrappers embed $CF and $LF, which depend on
# --embed-terminfo, --embed-cacert, --embed-locale, --no-iconv and
# --wrap-dlopen. So two concurrent builds with different options shared one set
# of flags, last writer wins, and NEITHER BUILD REPORTED ANYTHING: the loser
# silently linked a runtime it did not ask for, or lost one it did.
#
# ⭐ WHAT MAKES THIS A MEASUREMENT AND NOT A DEMO: the negative control. The
# same two builds run against a REPRODUCTION of the old behaviour --
# PGB_T058_SHARED_WRAPPERS=1 forces the single shared `bin` directory back --
# and the assertion is that the control DISAGREES with the fixed run. Without
# it, "both binaries came out right" is equally consistent with the race simply
# not having fired, which is what a timing-dependent defect does most of the
# time.
#
# ⭐ AND THE SCOPE OF THE DEFECT IS MEASURED TOO, because the first run of this
# experiment could not reproduce it at all. Under the DOCKER engine the two
# builds do not interfere: `cmd_build`'s docker branch passes
# `-e PGB_STATE=...` but does NOT bind-mount it, so each container gets a
# private, empty state directory inside its own ephemeral filesystem. Arm 3
# asserts that. ⚠ Its other consequence is a cost, not a safety property: every
# docker build recompiles the pgb runtime objects from scratch.
#
# Exit: 0 matched, 1 did not, 2 could not run.
# SPDX-License-Identifier: MIT
. "$(dirname "$0")/lib.sh"

exp_begin "87 - two concurrent pgb builds must not share one set of options"

WORK="${PGB_WORK:-/var/tmp/pgb-exp87}"
rm -rf "$WORK"; mkdir -p "$WORK" || { echo "cannot create $WORK" >&2; exit 2; }

PGB="$REPO_DIR/pgb"
[ -f "$PGB" ] || { exp_skip "pgb present" "no $PGB"; exp_finish; }

# ---------------------------------------------------------------------------
# The subject. Two of them, so the two builds cannot collide on an output path.
# ⭐ --embed-terminfo is the option under test because it is OBSERVABLE in the
# binary: it pulls tool/runtime/pgb-terminfo.c into the link, and that object
# defines symbols nothing else does. An option that left no trace would make
# the whole experiment pass by accident.
# ---------------------------------------------------------------------------
cat > "$WORK/subj.c" <<'EOF'
#include <stdio.h>
int main(void){ printf("subject ok\n"); return 0; }
EOF
cp "$WORK/subj.c" "$WORK/subj2.c"

marker_syms() {  # binary -> count of pgb terminfo symbols
  nm -a "$1" 2>/dev/null | grep -c 'pgb_terminfo' || true
}

build_one() {  # tag outbin src engine extra-opts...
  _tag=$1; _out=$2; _src=$3; _eng=$4; shift 4
  ( cd "$WORK" && sh "$PGB" --engine "$_eng" "$@" build -- sh -c \
      "\$CC -O2 -o $_out $_src" ) >"$WORK/$_tag.log" 2>&1
  echo $? > "$WORK/$_tag.rc"
}

# A pair of concurrent builds, one with --embed-terminfo and one without.
# Echoes "<terminfo-arm-symbols> <plain-arm-symbols>", or "x x" if either
# build produced nothing.
build_pair() {  # tag engine
  _t=$1; _e=$2
  rm -f "$WORK/$_t-terminfo" "$WORK/$_t-plain"
  build_one "$_t.t" "$WORK/$_t-terminfo" subj.c  "$_e" --embed-terminfo &
  _p1=$!
  build_one "$_t.p" "$WORK/$_t-plain"    subj2.c "$_e" &
  _p2=$!
  wait $_p1 2>/dev/null; wait $_p2 2>/dev/null
  if [ -f "$WORK/$_t-terminfo" ] && [ -f "$WORK/$_t-plain" ]; then
    printf '%s %s' "$(marker_syms "$WORK/$_t-terminfo")" \
                   "$(marker_syms "$WORK/$_t-plain")"
  else
    printf 'x x'
  fi
}

engine=$(cd "$WORK" && sh "$PGB" doctor 2>/dev/null | awk '/chosen engine:/{print $NF; exit}')
exp_note "engine pgb would choose on its own: ${engine:-unknown}"

# ⛔ THE DEFECT LIVES IN THE CHROOT ENGINE, so that is the engine arms 1 and 2
# must use. Running them under docker measures the container boundary instead
# and passes for the wrong reason -- which is exactly what the first run of
# this experiment did.
CHROOT_ENV="$ROOTFS_DIR/pgb-env-debian12"
if [ -d "$CHROOT_ENV" ]; then
  ENG=chroot
else
  ENG=""
fi

if [ -z "$ENG" ]; then
  exp_skip "arms 1-2 (the defect and its control)" \
           "no chroot build environment: run sh pgb --engine chroot env create"
else

# ---------------------------------------------------------------------------
# ARM 1 -- the fix. Two builds started together, different options.
# ---------------------------------------------------------------------------
printf -- '-- arm 1: option-keyed wrapper directories (the fix), chroot --\n'
unset PGB_T058_SHARED_WRAPPERS
set -- $(build_pair a1 "$ENG")
nt=$1; np=$2
rc_t=$(cat "$WORK/a1.t.rc" 2>/dev/null); rc_p=$(cat "$WORK/a1.p.rc" 2>/dev/null)
exp_check "arm1: --embed-terminfo build exit status" "${rc_t:-missing}" "0"
exp_check "arm1: plain build exit status"            "${rc_p:-missing}" "0"
[ "${rc_t:-1}" = 0 ] || sed -n '1,15p' "$WORK/a1.t.log" | sed 's/^/        /'
[ "${rc_p:-1}" = 0 ] || sed -n '1,15p' "$WORK/a1.p.log" | sed 's/^/        /'
exp_note "pgb_terminfo symbols: --embed-terminfo=$nt  plain=$np"
exp_check "arm1: terminfo runtime present in the build that asked" "$nt" "2"
exp_check "arm1: terminfo runtime ABSENT from the build that did not" "$np" "0"

# ⭐ The mechanism itself, asserted directly: the two option sets must resolve
# to two different directories. This is the property arm 1 depends on.
d_plain=$(cd "$WORK" && sh "$PGB" cc-dir 2>/dev/null | head -1)
d_term=$(cd "$WORK" && sh "$PGB" --embed-terminfo cc-dir 2>/dev/null | head -1)
exp_note "cc-dir plain           : ${d_plain:-none}"
exp_note "cc-dir --embed-terminfo: ${d_term:-none}"
exp_check "arm1: the two option sets key to different directories" \
          "$([ -n "$d_plain" ] && [ "$d_plain" != "$d_term" ] && echo yes || echo no)" "yes"
d_plain2=$(cd "$WORK" && sh "$PGB" cc-dir 2>/dev/null | head -1)
exp_check "arm1: the same option set is stable across invocations" \
          "$([ "$d_plain" = "$d_plain2" ] && echo yes || echo no)" "yes"

# ---------------------------------------------------------------------------
# ARM 2 -- ⭐ THE NEGATIVE CONTROL. The old behaviour, reproduced on purpose.
# ---------------------------------------------------------------------------
printf -- '\n-- arm 2: one shared wrapper directory (the OLD behaviour) ----\n'
exp_note "PGB_T058_SHARED_WRAPPERS=1 forces \$PGB_STATE/bin back, as it was"
# ⚠ THE CONTROL IS A RACE, so it is asserted as one: what must be true is that
# the shared directory makes the two builds AGREE at least once across several
# attempts -- not that it fails on any particular attempt.
agree=0; i=0
export PGB_T058_SHARED_WRAPPERS=1
while [ $i -lt 5 ]; do
  set -- $(build_pair a2 "$ENG")
  exp_note "  attempt $((i+1)): terminfo-arm=$1 plain-arm=$2"
  { [ "$1" = 2 ] && [ "$2" = 2 ]; } && agree=1
  { [ "$1" = 0 ] && [ "$2" = 0 ]; } && agree=1
  i=$((i+1))
done
unset PGB_T058_SHARED_WRAPPERS
exp_check "arm2: the shared directory makes the two builds agree" \
          "$([ "$agree" = 1 ] && echo yes || echo no)" "yes"
exp_note "⭐ that agreement IS the defect: a build got an option it never asked"
exp_note "   for, or lost one it did, and NEITHER build said anything."

fi

# ---------------------------------------------------------------------------
# ARM 3 -- ⭐ THE SCOPE. Under docker the same control cannot reproduce it.
# ---------------------------------------------------------------------------
printf -- '\n-- arm 3: the docker engine does not share $PGB_STATE ---------\n'
if ! docker info >/dev/null 2>&1; then
  exp_skip "arm3: docker engine" "docker not usable here"
else
  export PGB_T058_SHARED_WRAPPERS=1
  set -- $(build_pair a3 docker)
  d_nt=$1; d_np=$2
  unset PGB_T058_SHARED_WRAPPERS
  exp_note "pgb_terminfo symbols: --embed-terminfo=$d_nt  plain=$d_np"
  exp_check "arm3: docker keeps the two builds apart even when told to share" \
            "$([ "$d_nt" = 2 ] && [ "$d_np" = 0 ] && echo yes || echo no)" "yes"
  exp_note "cmd_build's docker branch passes -e PGB_STATE but does not bind it,"
  exp_note "so each container gets a private empty state directory. ⚠ The cost"
  exp_note "of that is real: every docker build recompiles the runtime objects."
fi

{
  printf '87 - concurrent pgb builds with different options (T-058)\n\n'
  printf 'arm 1, option-keyed directories (the fix), engine=%s:\n' "${ENG:-skipped}"
  printf '  --embed-terminfo build : pgb_terminfo symbols = %s\n' "${nt:-n/a}"
  printf '  plain build            : pgb_terminfo symbols = %s\n' "${np:-n/a}"
  printf '  cc-dir plain           : %s\n' "${d_plain:-none}"
  printf '  cc-dir --embed-terminfo: %s\n' "${d_term:-none}"
  printf '\narm 2, one shared directory (the old behaviour, reproduced):\n'
  printf '  the two builds agreed on one option set at least once: %s\n' \
         "$([ "${agree:-0}" = 1 ] && echo yes || echo no)"
  printf '\narm 3, docker engine with the same control:\n'
  printf '  --embed-terminfo=%s plain=%s  -> the container boundary isolates them\n' \
         "${d_nt:-n/a}" "${d_np:-n/a}"
  printf '\nconditions: %s, %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(uname -sr)"
} > "$EXP_OUT/RESULT.txt"

exp_finish
