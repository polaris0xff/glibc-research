#!/bin/sh
# THE QUESTION
#
#   Rung 5 of docs/research/app-corpus.md -- browsers, Electron, anything with
#   a sandbox -- is marked ⛔ NOT MEASURABLE IN THIS BED, because
#   `unshare(CLONE_NEWUSER)` is EPERM inside it. ⭐ WHY is it EPERM, and is
#   there a route?
#
# ⛔ WHAT WAS IN THE RECORD BEFORE THIS RAN, AND WHY IT WAS NOT ENOUGH.
# docs/comparison.md and app-corpus.md both say "EPERM inside the chroot bed"
# and stop there. That sentence is true and it names no cause, so it reads as
# a property of the bed that nothing can change -- and the entry built on it
# says the rung "needs a bed change before it needs a bundler change" without
# saying WHICH change. ⭐ A cause you have not isolated is not a blocker, it
# is an unfinished measurement.
#
# -- ⚠ HOW THESE EXPECTATIONS WERE ARRIVED AT, STATED PLAINLY ---------------
#
# ⚠ THIS FILE IS NOT A BLIND PRE-REGISTRATION AND SAYS SO. The six arms below
# were run by hand, twice, on two different rootfs (debian-12 and alpine-3.22)
# before this script existed; the numbers agreed on both. What the script adds
# is that the result is RE-RUNNABLE and that drift is caught -- which is the
# thing a hand run cannot give. ⛔ Delivery rule 1 exists so a prediction is
# not written after the fact and passed off as one; pretending these arms were
# blind would be exactly that. They are recorded as what they are.
#
# ⭐ THE ONE GENUINELY UNTESTED CLAIM IS N5's CONSEQUENCE and it is registered
# here before anybody acts on it: if N5 holds, then `pgb rootfs run` entering
# the bed by `pivot_root` instead of `chroot` would make rung 5 measurable.
# ⛔ THIS EXPERIMENT DOES NOT SHOW THAT. It shows the kernel permits the call
# after a pivot_root; whether the bed still isolates correctly, and whether a
# browser then sandboxes, are two further measurements nobody has taken.
#
# -- ⛔ EXPECTATIONS ---------------------------------------------------------
#
#   N1  ⭐ THE CONTROL: on the HOST, all five namespaces unshare OK. Without
#       this row every EPERM below could be the machine, the kernel or the
#       container, and the experiment would be measuring the wrong thing.
#   N2  inside the bed as `pgb rootfs run` enters it, CLONE_NEWUSER is EPERM.
#   N3  ⭐ ISOLATION: a plain `chroot` with NO unshare reproduces N2.
#   N4  ⭐ ISOLATION: `unshare --mount` with NO chroot does NOT reproduce it.
#       ⛔ N3 AND N4 ARE EACH OTHER'S CONTROL. Same rootfs, same probe, same
#       kernel; the only thing that differs is how the root was entered. That
#       pair is what makes "chroot is the cause" a measurement rather than a
#       plausible story -- `pgb rootfs run` does BOTH, so neither alone could
#       have been blamed from the bed row.
#   N5  ⭐ THE ROUTE: entering the SAME rootfs by `pivot_root` instead of
#       `chroot` permits CLONE_NEWUSER *and* CLONE_NEWUSER|CLONE_NEWNS.
#   N6  the sysctl is NOT the limit: /proc/sys/user/max_user_namespaces reads
#       non-zero inside the bed. ⚠ On a real Ubuntu >= 23.10 target the
#       limit is a DIFFERENT knob (apparmor_restrict_unprivileged_userns), so
#       a target that refuses for that reason is not this row.
#   N7  ⭐ ASKED, NOT GUESSED: `lsns -t user` inside the bed reports exactly
#       ONE user namespace -- the initial one. The operator's instruction for
#       this rung was "check with lsns -t user, do not guess", and a count of
#       one is what says nothing nested exists to have been missed.
#   N8  the OTHER namespaces (mount, pid, net) are permitted inside the bed,
#       so N2 is specific to CLONE_NEWUSER and not a blanket refusal.
#
# ⛔ WHAT THIS EXPERIMENT DOES NOT MEASURE, and it is most of rung 5:
#   - whether a bundled browser runs at all (that is a corpus row);
#   - whether its sandbox WORKS once the namespace is available;
#   - anything about a real Ubuntu target's apparmor restriction.
# ⚠ Two questions that must not be merged: "does the bundle carry a working
# Chromium" is answerable today with --no-sandbox; "does the sandbox work" is
# not, and a row run without the namespace measures --no-sandbox.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "69 - the sandbox: why CLONE_NEWUSER is EPERM in the bed, and the route out"

WORK="${PGB_EXP69_WORK:-/var/tmp/t069}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
command -v cc >/dev/null 2>&1 || { exp_note "no cc on PATH"; exit 2; }
command -v unshare >/dev/null 2>&1 || { exp_note "no unshare on PATH"; exit 2; }

# ⛔ ONE unshare() PER PROCESS. The first version of this probe made every call
# in one process, so each call after the first was measured from INSIDE the
# namespace the previous one created -- it reported EPERM for
# CLONE_NEWUSER|CLONE_NEWNS on a host where a fresh process gets it. That is a
# different question answered by accident, and it is the reason this file
# takes the flag set as argv[1].
#
# ⚠ IT IS BUILT -static ON PURPOSE: it has to run unchanged inside four musl
# rootfs as well as seven glibc ones, and a dynamic probe would measure the
# rootfs's loader rather than the kernel's policy.
cat > "$WORK/nsprobe.c" <<'EOF'
#define _GNU_SOURCE
#include <sched.h>
#include <stdio.h>
#include <string.h>
#include <errno.h>
int main(int argc, char **argv)
{
    int flags = 0;
    const char *what = argc > 1 ? argv[1] : "user";
    if (!strcmp(what, "user"))        flags = CLONE_NEWUSER;
    else if (!strcmp(what, "userns")) flags = CLONE_NEWUSER | CLONE_NEWNS;
    else if (!strcmp(what, "mount"))  flags = CLONE_NEWNS;
    else if (!strcmp(what, "pid"))    flags = CLONE_NEWPID;
    else if (!strcmp(what, "net"))    flags = CLONE_NEWNET;
    else { fprintf(stderr, "unknown flag set: %s\n", what); return 2; }
    if (unshare(flags) == 0) { printf("OK\n"); return 0; }
    printf("E%d %s\n", errno, strerror(errno));
    return 1;
}
EOF
cc -O2 -static -o "$WORK/nsprobe" "$WORK/nsprobe.c" 2>"$WORK/cc.log" \
  || { exp_note "could not build the probe; see $WORK/cc.log"; exit 2; }

# verdict <output> -> OK | EPERM | other
verdict() {
  case "$1" in
    OK*)   printf 'OK' ;;
    E1\ *) printf 'EPERM' ;;
    *)     printf 'other(%s)' "$1" ;;
  esac
}

ROOT=$(exp_rootfs "${PGB_EXP69_ENV:-debian-12}") || true
[ -n "$ROOT" ] || { exp_note "rootfs not fetched; run: pgb rootfs fetch"; exit 2; }

printf '\n-- N1: the HOST, one process per call (⭐ the control) --------------\n'
host_all=ok
for w in user userns mount pid net; do
  v=$(verdict "$("$WORK/nsprobe" "$w" 2>&1)")
  printf '        host %-7s %s\n' "$w" "$v"
  [ "$v" = OK ] || host_all=no
done
exp_check "N1  ⭐ CONTROL: every namespace unshares on the HOST" "$host_all" ok

cp "$WORK/nsprobe" "$ROOT/nsprobe" || exit 2
chmod +x "$ROOT/nsprobe"
# ⚠ every arm below is cleaned up even when one fails, or the next run of this
# experiment finds a probe binary in a rootfs the corpus also uses.
cleanup() { rm -f "$ROOT/nsprobe"; rmdir "$ROOT/.oldroot" 2>/dev/null; }
trap 'cleanup' EXIT INT TERM

printf '\n-- N2: the bed, entered exactly as `pgb rootfs run` enters it -------\n'
bed_user=$(verdict "$(timeout 60 "$REPO_DIR/pgb" rootfs run "$ROOT" -- \
  /nsprobe user 2>&1 | tail -1)")
exp_check "N2  CLONE_NEWUSER inside the bed" "$bed_user" EPERM

printf '\n-- N3/N4: ⭐ the isolation pair, and they are each other'"'"'s control --\n'
# `pgb rootfs run` does unshare --mount AND chroot. Neither alone could be
# blamed from N2, so each is run without the other, on the same rootfs.
n3=$(verdict "$(chroot "$ROOT" /nsprobe user 2>&1 | tail -1)")
exp_check "N3  ⭐ chroot ALONE reproduces it" "$n3" EPERM
n4=$(verdict "$(unshare --mount -- "$WORK/nsprobe" user 2>&1 | tail -1)")
exp_check "N4  ⭐ unshare --mount ALONE does NOT" "$n4" OK

printf '\n-- N5: ⭐ THE ROUTE -- the same rootfs entered by pivot_root ---------\n'
# ⚠ pivot_root needs the new root to be a mount point, hence the bind; and it
# needs a private propagation or the bind escapes into the host namespace.
mkdir -p "$ROOT/.oldroot"
n5=$(verdict "$(unshare --mount --propagation private -- sh -c \
  "mount --bind '$ROOT' '$ROOT' && cd '$ROOT' && pivot_root . .oldroot && /nsprobe user" \
  2>&1 | tail -1)")
exp_check "N5  ⭐ CLONE_NEWUSER after pivot_root" "$n5" OK
n5b=$(verdict "$(unshare --mount --propagation private -- sh -c \
  "mount --bind '$ROOT' '$ROOT' && cd '$ROOT' && pivot_root . .oldroot && /nsprobe userns" \
  2>&1 | tail -1)")
exp_check "N5  ⭐ ...and CLONE_NEWUSER|CLONE_NEWNS too" "$n5b" OK

printf '\n-- N6/N7/N8: what the bed is NOT refusing ---------------------------\n'
maxns=$(timeout 60 "$REPO_DIR/pgb" rootfs run "$ROOT" -- \
  /bin/sh -c 'cat /proc/sys/user/max_user_namespaces 2>/dev/null' 2>/dev/null | tr -dc '0-9')
exp_note "max_user_namespaces inside the bed: ${maxns:-(unreadable)}"
exp_check "N6  the sysctl is NOT the limit" \
  "$([ -n "$maxns" ] && [ "$maxns" -gt 0 ] && echo yes || echo no)" yes

# ⭐ ASKED, NOT GUESSED -- the operator's instruction for this rung.
#
# ⛔ "NO lsns" IS NOT "ZERO NAMESPACES", AND THE FIRST VERSION OF THIS BLOCK
# CONFLATED THEM. `lsns ... | wc -l` yields 0 both when there is nothing to
# count and when the binary is absent, so on alpine-3.22 -- whose busybox
# ships no lsns -- N7 reported `0, expected 1` and looked like a real
# disagreement about namespaces. It was a missing tool. ⚠ Caught by run 2 on
# a different rootfs, which is what delivery rule 3 is for. A skip is neither
# a pass nor a failure (docs/AGENTS.md §0b), so it is reported as one.
nuser=$(timeout 60 "$REPO_DIR/pgb" rootfs run "$ROOT" -- /bin/sh -c \
  'command -v lsns >/dev/null 2>&1 || { echo NOLSNS; exit 0; }
   lsns -t user | tail -n +2 | wc -l' 2>/dev/null | tr -d ' \r' | tail -1)
if [ "$nuser" = NOLSNS ]; then
  exp_skip "N7  ⭐ user-namespace count" "no lsns in $(basename "$ROOT") (util-linux absent; busybox ships none)"
  exp_note "   ⚠ Re-run with PGB_EXP69_ENV=debian-12 to take this row."
else
  exp_note "lsns -t user inside the bed reports: $nuser"
  exp_check "N7  ⭐ exactly ONE user namespace, nothing nested" "$nuser" 1
fi

others=ok
for w in mount pid net; do
  v=$(verdict "$(timeout 60 "$REPO_DIR/pgb" rootfs run "$ROOT" -- /nsprobe "$w" 2>&1 | tail -1)")
  printf '        bed  %-7s %s\n' "$w" "$v"
  [ "$v" = OK ] || others=no
done
exp_check "N8  mount/pid/net ARE permitted in the bed" "$others" ok

printf '\n'
exp_note "⭐ WHAT THIS ESTABLISHES: the refusal is CHROOT's, not the bed's, not"
exp_note "   the kernel's policy on this machine, and not a sysctl. N3 and N4"
exp_note "   isolate it; N5 shows the same rootfs entered by pivot_root permits"
exp_note "   the call. ⚠ No man page is installed here to cite for the kernel's"
exp_note "   own reason -- that was looked for and is absent, so the mechanism"
exp_note "   is recorded as MEASURED behaviour rather than as a quotation."
exp_note "⛔ WHAT IT DOES NOT ESTABLISH: that pgb rootfs run SHOULD pivot_root"
exp_note "   (isolation and teardown are unmeasured), that a bundled browser"
exp_note "   runs, or that a sandbox works once the namespace exists. Those are"
exp_note "   three further measurements. T-090."

exp_finish
