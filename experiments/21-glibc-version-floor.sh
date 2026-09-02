#!/bin/sh
# THE QUESTION
#
#   Does the __nss_configure_lookup() override actually REMOVE the dlopen, or
#   does it only MOVE it -- and does the answer depend on the version of glibc
#   the binary was BUILT against?
#
# -- WHY THIS IS THE MOST LOAD-BEARING UNMEASURED CLAIM IN THE PROJECT --------
#
# The whole tool pins its build environment above glibc 2.34 and the
# reason given everywhere is: "glibc 2.34 built the `files` and `dns` NSS
# services into libc, so pinning the databases to those services leaves
# nothing to dlopen."
#
# ⛔ THAT WAS REASONING, NOT A MEASUREMENT. If it is wrong, the override is
# cosmetic on older build glibcs -- it would swap the host's `resolve` module
# for the host's `files` module and load a foreign libc either way, while
# every experiment in this repository reported success because it only ever
# looked for module names it recognised as external.
#
# The experiment builds the SAME source three ways -- against glibc 2.31
# (Debian 11, below the floor), against glibc 2.36 (Debian 12, above it), and
# against WHATEVER cfg.go PINS TODAY -- and compares what each one opens at
# run time. ⭐ The third arm is why a pin move re-validates the floor here
# automatically instead of leaving this file measuring a glibc the tool no
# longer builds against.
#
# ⭐ THE CONTROL THAT MAKES THE ANSWER READABLE. Both binaries are run on the
# SAME target root filesystem, so any difference between them is the build
# glibc and nothing else. The target chosen is Debian 11, because it actually
# ships libnss_files.so.2 -- on a target with no such file the pre-2.34 binary
# would fail to find it and the trace would look deceptively clean.
#
# Exit: 0 the floor behaves as the project claims, 1 it does not (in which case
#       the pin is wrong and docs must change), 2 could not run.

. "$(dirname "$0")/lib.sh"

exp_begin "21 - is glibc 2.34 really the floor for the NSS override"

[ "$(id -u)" = 0 ] || { printf 'needs root\n'; exit 2; }

B="$EXP_OUT/build"
rm -rf "$B"; mkdir -p "$B" || exit 2

# The probe: identical source, built twice. -DNSSFIX is the only difference
# between the arms, exactly as in experiment 20.
cat > "$B/probe.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <string.h>
#include <netdb.h>
#include <pwd.h>
#include <sys/socket.h>
#ifdef NSSFIX
extern int __nss_configure_lookup(const char *db, const char *line);
__attribute__((constructor(101)))
static void nssfix(void) {
    static const char *const f[] = {"passwd","group","shadow","gshadow","aliases",
        "ethers","initgroups","netgroup","networks","protocols","publickey",
        "rpc","services", NULL};
    for (const char *const *d = f; *d; d++) __nss_configure_lookup(*d, "files");
    __nss_configure_lookup("hosts", "files dns");
}
#endif
int main(void) {
    struct addrinfo h, *r;
    memset(&h, 0, sizeof h); h.ai_socktype = SOCK_STREAM;
    int s = getaddrinfo("example.com", "80", &h, &r);
    if (s == 0) freeaddrinfo(r);
    struct passwd *pw = getpwuid(0);
    printf("getaddrinfo=%d getpwuid=%s\n", s, pw ? pw->pw_name : "(null)");
    return 0;
}
EOF

# ---------------------------------------------------------------------------
# Build one arm inside a given distro rootfs, using ITS gcc and ITS glibc.
# ---------------------------------------------------------------------------
#
# ⛔ THE VERSION LABEL IS READ OUT OF THE ENVIRONMENT, NEVER TYPED. The first
# version of this file labelled its arms "2.31" and "2.36" as literals. That
# was true when it was written and would have gone on printing "2.36" after
# T-070 moved the pin to 2.41 -- a table saying one thing about a binary built
# from another, which is the exact failure T-070 spent a session finding in
# `PGB_ENV_NAME`. `ldd --version` inside each environment is the label now.
build_in() {   # rootfs-name  out-prefix  [prepared]
  _rn="$1"; _op="$2"; _prepared="${3:-}"
  _src="$ROOTFS_DIR/$_rn"
  [ -d "$_src" ] || { exp_skip "build in $_rn" "rootfs not fetched"; return 1; }
  if [ -n "$_prepared" ]; then
    # ⭐ The pinned build environment already HAS a compiler, so it is used in
    # place rather than copied and apt-ed. ⚠ Nothing is written into it — see
    # the bind mount below.
    _env="$_src"
  else
    _env="$ROOTFS_DIR/exp21-$_rn"
    if [ ! -f "$_env/.exp21-ready" ]; then
      rm -rf "$_env"; cp -a "$_src" "$_env" || return 1
      "$REPO_DIR/pgb" rootfs run "$_env" -- /bin/sh -c \
        'export DEBIAN_FRONTEND=noninteractive; apt-get update -qq && \
         apt-get install -y -qq --no-install-recommends gcc libc6-dev' \
        >"$B/$_rn-apt.log" 2>&1 || { exp_skip "build in $_rn" "package install failed"; return 1; }
      : > "$_env/.exp21-ready"
    fi
  fi
  # ⛔ THE OUTPUT GOES INTO A BIND MOUNT, NOT INTO THE ROOTFS. The first
  # version of this wrote the probe to `/` inside each environment and copied
  # the results back out; the second tried `/tmp` to keep the PINNED
  # environment clean and produced `cc1: fatal error: ... No such file or
  # directory` on all three arms -- ⚠ `pgb rootfs run` MOUNTS A FRESH TMPFS
  # OVER /tmp, so a file copied there from outside is not there inside.
  # Verified by hand: `ls -la /tmp` inside shows an empty `drwxrwxrwt`.
  # A bind mount of the experiment's own build directory has neither problem.
  "$REPO_DIR/pgb" rootfs run "$_env" --bind "$B:$B" --workdir "$B" -- /bin/sh -c \
    "gcc -static -O2 -o '$B/$_op-plain' probe.c && \
     gcc -static -O2 -DNSSFIX -o '$B/$_op-nssfix' probe.c" \
    >>"$B/$_rn-build.log" 2>&1 || { exp_skip "build in $_rn" "compile failed"; return 1; }
  [ -f "$B/$_op-plain" ] && [ -f "$B/$_op-nssfix" ] || {
    exp_skip "build in $_rn" "the compiler reported success and produced nothing"; return 1; }
  # The glibc this arm was actually built against, for its label.
  _v=$("$REPO_DIR/pgb" rootfs run "$_env" -- /bin/sh -c 'ldd --version' 2>/dev/null \
       | head -1 | awk '{print $NF}')
  eval "VER_$_op=\${_v:-unknown}"
  exp_note "built in $_rn: glibc ${_v:-unknown}"
  return 0
}

HAVE_OLD=0; HAVE_NEW=0; HAVE_PIN=0
VER_old=unknown; VER_new=unknown; VER_pin=unknown
build_in debian-11 old && HAVE_OLD=1
build_in debian-12 new && HAVE_NEW=1

# ⭐ THE THIRD ARM, AND IT IS THE ONE THAT FOLLOWS THE PIN. ENV_NAME comes from
# lib.sh, out of internal/cfg/cfg.go, so moving the pin re-validates the NSS
# floor here automatically instead of leaving this experiment measuring a glibc
# the tool no longer builds against. T-070.
# ⚠ Its rootfs is the pinned build environment, which already has gcc.
if [ -d "$ENV_ROOT" ]; then
  build_in "$ENV_NAME" pin prepared && HAVE_PIN=1
else
  exp_skip "the pinned build environment ($ENV_NAME)" \
           "absent; run: ./pgb --engine chroot env create"
fi

if [ "$HAVE_OLD" = 0 ] || [ "$HAVE_NEW" = 0 ]; then
  exp_note "both a below-floor and an above-floor build environment are required"
  exp_finish
fi

# ---------------------------------------------------------------------------
# Run every arm on ONE target that really ships libnss_files.so.2.
# ---------------------------------------------------------------------------
TARGET="$ROOTFS_DIR/debian-11"
exp_check "target ships libnss_files.so.2" \
  "$(ls "$TARGET"/lib/*/libnss_files.so.2 >/dev/null 2>&1 && echo yes || echo no)" yes

printf '\n  what each build opens on debian-11 (glibc 2.31 target):\n'
printf '    %-32s %-8s %s\n' 'BUILD glibc / arm' 'EXIT' 'HOST NSS MODULES OPENED'

# ⛔ THIS FUNCTION RETURNS A VALUE ON STDOUT AND MUST PRINT NOTHING ELSE
# THERE. The first version also wrote its own table row to stdout, so every
# caller captured the row TEXT as the module list -- and the comparisons below
# then compared two formatted table rows instead of two module lists. The row
# goes to stderr, which is where the operator sees it and $( ) does not.
#
# ⚠ THE TRACE FILE IS NAMED BY A TAG, NOT BY THE LABEL. The label carries
# spaces and parentheses now that it is built from a version read at run time,
# and a path composed out of it is a filename waiting to be mishandled by the
# next thing that touches it.
modules_for() {  # binary label tag -> echoes the comma list, table row on stderr
  cp "$1" "$TARGET/exp21-probe"
  "$REPO_DIR/pgb" rootfs run "$TARGET" -- /exp21-probe >/dev/null 2>&1
  _st=$?
  _m=$(exp_trace_opens "$TARGET" /exp21-probe "$B/tr-$3.txt" \
       | grep -oE 'libnss_[a-z0-9_]*\.so[^"]*' | sort -u | tr '\n' ',' | sed 's/,$//')
  rm -f "$TARGET/exp21-probe"
  printf '    %-32s %-8s %s\n' "$2" "$_st" "${_m:-none}" >&2
  printf '%s' "${_m:-none}"
}

OLD_PLAIN=$(modules_for "$B/old-plain"  "$VER_old plain"   old-plain)
OLD_FIX=$(modules_for   "$B/old-nssfix" "$VER_old +nssfix" old-nssfix)
NEW_PLAIN=$(modules_for "$B/new-plain"  "$VER_new plain"   new-plain)
NEW_FIX=$(modules_for   "$B/new-nssfix" "$VER_new +nssfix" new-nssfix)
PIN_PLAIN=unmeasured; PIN_FIX=unmeasured
if [ "$HAVE_PIN" = 1 ]; then
  PIN_PLAIN=$(modules_for "$B/pin-plain"  "$VER_pin plain   (THE PIN)" pin-plain)
  PIN_FIX=$(modules_for   "$B/pin-nssfix" "$VER_pin +nssfix (THE PIN)" pin-nssfix)
fi

printf '\n'
# ⭐ THE ASSERTION THE PROJECT'S PIN RESTS ON.
exp_check "above-floor build + override loads no NSS module ($VER_new)" "$NEW_FIX" none
if [ "$HAVE_PIN" = 1 ]; then
  # ⛔ THE ONE THAT ACTUALLY GATES THE SHIPPED TOOL. The row above is the
  # historical above-floor witness; this one is whatever cfg.go pins today.
  exp_check "THE PIN's build + override loads no NSS module ($VER_pin)" "$PIN_FIX" none
else
  exp_skip "THE PIN's build + override" "the pinned environment is not on disk"
fi

# ⛔ THIS ONE IS DELIBERATELY NOT ASSERTED EITHER WAY. Whichever way it lands
# is a finding, and the point of the experiment is to find out rather than to
# confirm. It is compared and reported.
if [ "$OLD_FIX" = none ]; then
  exp_note "BELOW THE FLOOR: the 2.31 build ALSO loaded nothing."
  exp_note "  If that holds, the >=2.34 pin is more conservative than it needs"
  exp_note "  to be, and docs/AGENTS.md and pgb's env comment should say so."
else
  exp_note "BELOW THE FLOOR, CONFIRMED: the 2.31 build still loaded [$OLD_FIX]"
  exp_note "  even with the override, because those services are not builtin"
  exp_note "  before glibc 2.34. This is what the >=2.34 pin exists for."
fi

if [ "$OLD_FIX" != "$NEW_FIX" ]; then
  exp_note "the two builds DIFFER, which is the build glibc and nothing else:"
  exp_note "  $VER_old + override -> $OLD_FIX"
  exp_note "  $VER_new + override -> $NEW_FIX"
fi
exp_note "for reference, without the override: $VER_old -> $OLD_PLAIN ; $VER_new -> $NEW_PLAIN"

{
  printf 'build glibc %s plain   : %s\n' "$VER_old" "$OLD_PLAIN"
  printf 'build glibc %s +nssfix : %s\n' "$VER_old" "$OLD_FIX"
  printf 'build glibc %s plain   : %s\n' "$VER_new" "$NEW_PLAIN"
  printf 'build glibc %s +nssfix : %s\n' "$VER_new" "$NEW_FIX"
  if [ "$HAVE_PIN" = 1 ]; then
    printf 'THE PIN (%s) %s plain   : %s\n' "$ENV_NAME" "$VER_pin" "$PIN_PLAIN"
    printf 'THE PIN (%s) %s +nssfix : %s\n' "$ENV_NAME" "$VER_pin" "$PIN_FIX"
  else
    printf 'THE PIN (%s) : NOT MEASURED -- environment absent\n' "$ENV_NAME"
  fi
} > "$EXP_OUT/floor.txt"

exp_finish
