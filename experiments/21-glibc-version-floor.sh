#!/bin/sh
# THE QUESTION
#
#   Does the __nss_configure_lookup() override actually REMOVE the dlopen, or
#   does it only MOVE it -- and does the answer depend on the version of glibc
#   the binary was BUILT against?
#
# -- WHY THIS IS THE MOST LOAD-BEARING UNMEASURED CLAIM IN THE PROJECT --------
#
# The whole tool pins its build environment to Debian 12 (glibc 2.36) and the
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
# The experiment builds the SAME source against glibc 2.31 (Debian 11, below
# the floor) and against glibc 2.36 (Debian 12, above it) and compares what
# each one opens at run time.
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
build_in() {   # rootfs-name  out-prefix
  _rn="$1"; _op="$2"
  _src="$ROOTFS_DIR/$_rn"
  [ -d "$_src" ] || { exp_skip "build in $_rn" "rootfs not fetched"; return 1; }
  _env="$ROOTFS_DIR/exp21-$_rn"
  if [ ! -f "$_env/.exp21-ready" ]; then
    rm -rf "$_env"; cp -a "$_src" "$_env" || return 1
    "$REPO_DIR/pgb" rootfs run "$_env" -- /bin/sh -c \
      'export DEBIAN_FRONTEND=noninteractive; apt-get update -qq && \
       apt-get install -y -qq --no-install-recommends gcc libc6-dev' \
      >"$B/$_rn-apt.log" 2>&1 || { exp_skip "build in $_rn" "package install failed"; return 1; }
    : > "$_env/.exp21-ready"
  fi
  cp "$B/probe.c" "$_env/probe.c"
  "$REPO_DIR/pgb" rootfs run "$_env" -- /bin/sh -c \
    'cd / && gcc -static -O2 -o /probe-plain probe.c 2>/dev/null && \
     gcc -static -O2 -DNSSFIX -o /probe-nssfix probe.c 2>/dev/null' \
    >>"$B/$_rn-build.log" 2>&1 || { exp_skip "build in $_rn" "compile failed"; return 1; }
  cp "$_env/probe-plain"  "$B/$_op-plain"  || return 1
  cp "$_env/probe-nssfix" "$B/$_op-nssfix" || return 1
  _v=$("$REPO_DIR/pgb" rootfs run "$_env" -- /bin/sh -c 'ldd --version' 2>/dev/null | head -1)
  exp_note "built in $_rn: $_v"
  return 0
}

HAVE_OLD=0; HAVE_NEW=0
build_in debian-11 old && HAVE_OLD=1
build_in debian-12 new && HAVE_NEW=1

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
printf '    %-28s %-8s %s\n' 'BUILD glibc / arm' 'EXIT' 'HOST NSS MODULES OPENED'

# ⛔ THIS FUNCTION RETURNS A VALUE ON STDOUT AND MUST PRINT NOTHING ELSE
# THERE. The first version also wrote its own table row to stdout, so every
# caller captured the row TEXT as the module list -- and the comparisons below
# then compared two formatted table rows instead of two module lists. The row
# goes to stderr, which is where the operator sees it and $( ) does not.
modules_for() {  # binary label -> echoes the comma list, table row on stderr
  cp "$1" "$TARGET/exp21-probe"
  "$REPO_DIR/pgb" rootfs run "$TARGET" -- /exp21-probe >/dev/null 2>&1
  _st=$?
  _m=$(exp_trace_opens "$TARGET" /exp21-probe "$B/tr-$2.txt" \
       | grep -oE 'libnss_[a-z0-9_]*\.so[^"]*' | sort -u | tr '\n' ',' | sed 's/,$//')
  rm -f "$TARGET/exp21-probe"
  printf '    %-28s %-8s %s\n' "$2" "$_st" "${_m:-none}" >&2
  printf '%s' "${_m:-none}"
}

OLD_PLAIN=$(modules_for "$B/old-plain"  "2.31 plain")
OLD_FIX=$(modules_for   "$B/old-nssfix" "2.31 +nssfix")
NEW_PLAIN=$(modules_for "$B/new-plain"  "2.36 plain")
NEW_FIX=$(modules_for   "$B/new-nssfix" "2.36 +nssfix")

printf '\n'
# ⭐ THE ASSERTION THE PROJECT'S PIN RESTS ON.
exp_check "2.36 build + override loads no NSS module" "$NEW_FIX" none

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
  exp_note "  2.31 + override -> $OLD_FIX"
  exp_note "  2.36 + override -> $NEW_FIX"
fi
exp_note "for reference, without the override: 2.31 -> $OLD_PLAIN ; 2.36 -> $NEW_PLAIN"

{
  printf 'build glibc 2.31 plain   : %s\n' "$OLD_PLAIN"
  printf 'build glibc 2.31 +nssfix : %s\n' "$OLD_FIX"
  printf 'build glibc 2.36 plain   : %s\n' "$NEW_PLAIN"
  printf 'build glibc 2.36 +nssfix : %s\n' "$NEW_FIX"
} > "$EXP_OUT/floor.txt"

exp_finish
