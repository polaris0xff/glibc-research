#!/bin/sh
# THE QUESTION
#
#   Does a `gcc -static` glibc binary load the HOST's NSS modules at runtime,
#   and does one constructor calling __nss_configure_lookup() stop it, without
#   giving up name resolution?
#
# This is the load-bearing question of the whole project. If the answer to the
# second half is no, a portable glibc binary needs a launcher, a bundled
# loader, or a packaging format, and the "normal ELF executable" goal is dead.
#
# -- WHY A PLANTED MODULE AND NOT JUST strace ---------------------------------
#
# ⛔ AN ABSENCE OF openat() IS NOT EVIDENCE THAT NOTHING LOADED.
# docs/methodology/experiments.md: "a probe that found nothing may have been
# looking in the wrong place, and the two are distinguishable only by a
# positive control that the probe DOES find."
#
# So arm A of this experiment plants a REAL NSS module, `libnss_evil.so.2`,
# into the target root filesystem and names it in that filesystem's
# /etc/nsswitch.conf. The module writes marker files from inside its own
# constructor and from inside its own lookup entry point. A marker on disk
# afterwards is proof the host's code was mapped into the "static" process AND
# executed there -- which an openat() alone does not establish, and which no
# amount of `file`/`ldd` output can see at all.
#
# Arm B is the same source, the same static link, plus one constructor.
#
# -- THE THIRD ARM, WHICH IS THE ONE THAT MAKES A AND B MEAN ANYTHING ---------
#
# ⚠ Arm C is a static binary that performs NO name lookup. On a distribution
# where arm A crashes, arm C answers the question "do static binaries just not
# work here?" Without it, a crash in A is unattributable.
#
# Exit: 0 every arm behaved as expected on every environment present,
#       1 an arm did not, 2 the experiment could not run.

. "$(dirname "$0")/lib.sh"

exp_begin "20 - does a static glibc binary dlopen host NSS modules, and can that be stopped"

command -v ${CC:-cc} >/dev/null 2>&1 || { printf 'no compiler\n'; exit 2; }

B="$EXP_OUT/build"
rm -rf "$B"; mkdir -p "$B" || exit 2

# ---------------------------------------------------------------------------
# The probe. One source, two links: the -DNSSFIX arm differs by a constructor
# and nothing else, which is what makes the pair a control rather than two
# programs.
# ---------------------------------------------------------------------------
cat > "$B/probe.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <string.h>
#include <netdb.h>
#include <pwd.h>
#include <sys/types.h>
#include <sys/socket.h>

#ifdef NSSFIX
/* Public glibc symbol, versioned GLIBC_2.2.5, present in libc.a. It replaces
 * the nsswitch.conf line for one database, in-process, for this process only.
 *
 * Priority 101 rather than a bare constructor: it must win against any other
 * constructor in the program that might perform a lookup. Priorities below
 * 101 are reserved by the implementation. */
extern int __nss_configure_lookup(const char *db, const char *line);
__attribute__((constructor(101)))
static void nssfix(void) {
    static const char *const files_only[] = {
        "passwd","group","shadow","gshadow","aliases","ethers","initgroups",
        "netgroup","networks","protocols","publickey","rpc","services", NULL };
    for (const char *const *d = files_only; *d; d++)
        __nss_configure_lookup(*d, "files");
    /* "dns" is builtin from glibc 2.34; on an older build glibc this line
     * still dlopens libnss_dns.so.2, which experiment 21 measures. */
    __nss_configure_lookup("hosts", "files dns");
}
#endif

int main(void) {
    struct addrinfo hints, *res;
    memset(&hints, 0, sizeof hints);
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    int st = getaddrinfo("example.com", "80", &hints, &res);
    printf("getaddrinfo=%d\n", st);
    if (st == 0) freeaddrinfo(res);
    struct passwd *pw = getpwuid(0);
    printf("getpwuid=%s\n", pw ? pw->pw_name : "(null)");
    return 0;
}
EOF

# The planted module. Its markers are the oracle.
cat > "$B/nss_evil.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <netdb.h>
#include <nss.h>
#include <pwd.h>
__attribute__((constructor))
static void evil_ctor(void) {
    FILE *f = fopen("/EVIL-LOADED", "w");
    if (f) { fputs("host NSS module constructor ran in-process\n", f); fclose(f); }
}
enum nss_status _nss_evil_gethostbyname4_r(const char *name, struct gaih_addrtuple **pat,
        char *buffer, size_t buflen, int *errnop, int *h_errnop, int32_t *ttlp) {
    FILE *f = fopen("/EVIL-CALLED", "w");
    if (f) { fprintf(f, "host NSS module was asked to resolve %s\n", name); fclose(f); }
    (void)pat;(void)buffer;(void)buflen;(void)errnop;(void)h_errnop;(void)ttlp;
    return NSS_STATUS_UNAVAIL;
}
enum nss_status _nss_evil_getpwuid_r(uid_t uid, struct passwd *pw, char *buf,
        size_t buflen, int *errnop) {
    FILE *f = fopen("/EVIL-PASSWD", "w");
    if (f) { fprintf(f, "host NSS module got a passwd lookup for uid %u\n", (unsigned)uid); fclose(f); }
    (void)pw;(void)buf;(void)buflen;(void)errnop;
    return NSS_STATUS_UNAVAIL;
}
EOF

printf '#include <stdio.h>\nint main(void){printf("ran\\n");return 0;}\n' > "$B/nonss.c"

${CC:-cc} -static -O2 -o "$B/probe-plain"  "$B/probe.c"          2>"$B/build-plain.log"  || exit 2
${CC:-cc} -static -O2 -DNSSFIX -o "$B/probe-nssfix" "$B/probe.c" 2>"$B/build-nssfix.log" || exit 2
${CC:-cc} -static -O2 -o "$B/probe-nonss"  "$B/nonss.c"          2>/dev/null             || exit 2
${CC:-cc} -shared -fPIC -O2 -o "$B/libnss_evil.so.2" "$B/nss_evil.c" 2>/dev/null         || exit 2

# ⭐ The toolchain says this out loud at link time and it is routinely ignored.
# Recording it here makes it part of the evidence rather than build noise.
if grep -q "requires at runtime the shared libraries" "$B/build-plain.log"; then
  exp_check "linker warns about getaddrinfo in a static link" yes yes
else
  exp_check "linker warns about getaddrinfo in a static link" no yes
fi

# ---------------------------------------------------------------------------
# PART 1 -- a synthetic root filesystem carrying a hostile nsswitch.conf.
#
# Built rather than borrowed so the hostile line is certain to be there: no
# stock image is guaranteed to name a module this experiment controls.
# ---------------------------------------------------------------------------
SYN="$B/synthetic-root"
rm -rf "$SYN"; mkdir -p "$SYN/etc" "$SYN/bin" || exit 2
TRIPLET=$(${CC:-cc} -dumpmachine 2>/dev/null || echo x86_64-linux-gnu)
LIBDIR="$SYN/lib/$TRIPLET"
mkdir -p "$LIBDIR" "$SYN/lib64"
cp "$B/libnss_evil.so.2" "$LIBDIR/" || exit 2

# The module is a normal shared object, so making it loadable means giving the
# root filesystem the host's libc AND its loader. That requirement is itself
# the finding: a "static" binary that reaches this path needs both.
HOST_LIBC=$(${CC:-cc} -print-file-name=libc.so.6 2>/dev/null)
[ -f "$HOST_LIBC" ] || HOST_LIBC=$(ls /lib/*/libc.so.6 /lib64/libc.so.6 2>/dev/null | head -1)
HOST_LD=$(ls /lib64/ld-linux-x86-64.so.2 /lib/ld-linux-aarch64.so.1 /lib/*/ld-linux-*.so.* 2>/dev/null | head -1)
if [ -f "$HOST_LIBC" ] && [ -f "$HOST_LD" ]; then
  cp "$HOST_LIBC" "$LIBDIR/"; cp "$HOST_LD" "$LIBDIR/"; cp "$HOST_LD" "$SYN/lib64/" 2>/dev/null
  printf 'hosts: files evil dns\npasswd: files evil\ngroup: files evil\n' > "$SYN/etc/nsswitch.conf"
  printf '127.0.0.1 localhost\n' > "$SYN/etc/hosts"
  printf 'root:x:0:0:root:/root:/bin/sh\n' > "$SYN/etc/passwd"
  cp "$B/probe-plain" "$B/probe-nssfix" "$SYN/bin/"

  run_syn() { # binary -> echoes the markers left behind, or "none"
    rm -f "$SYN"/EVIL-*
    "$REPO_DIR/pgb" rootfs run "$SYN" --no-net -- "/bin/$1" >/dev/null 2>&1
    m=$(cd "$SYN" && ls 2>/dev/null | grep '^EVIL-' | sort | tr '\n' ',' | sed 's/,$//')
    printf '%s' "${m:-none}"
  }

  printf '\n  synthetic root, /etc/nsswitch.conf = "hosts: files evil dns"\n'
  A=$(run_syn probe-plain)
  exp_check "A plain -static: host module markers" "$A" "EVIL-CALLED,EVIL-LOADED"
  exp_note "the module's own constructor and its own lookup entry point both ran"
  B_=$(run_syn probe-nssfix)
  exp_check "B +__nss_configure_lookup: host module markers" "$B_" "none"

  # ⚠ THE PRECISE CLAIM. The override does NOT stop glibc reading the file; it
  # stops the file's modules being used. Stating it the other way round would
  # be wrong, and a reader would design against a property that is not there.
  strace -f -e trace=openat -o "$B/syn-nssfix.trace" \
    "$REPO_DIR/pgb" rootfs run "$SYN" --no-net -- /bin/probe-nssfix >/dev/null 2>&1
  if grep -q 'nsswitch.conf' "$B/syn-nssfix.trace" 2>/dev/null; then
    exp_note "arm B still OPENS /etc/nsswitch.conf; what it does not do is load what the file names"
  fi
  if grep -q 'libnss_evil' "$B/syn-nssfix.trace" 2>/dev/null; then
    exp_check "B opened the host module anyway" yes no
  else
    exp_check "B never opened the host NSS module" yes yes
  fi
else
  exp_skip "synthetic hostile-nsswitch arm" "host libc.so.6 or loader not found"
fi

# ---------------------------------------------------------------------------
# PART 2 -- real distributions, unmodified, at the pinned digests.
#
# This is the part that cannot be argued with: the nsswitch.conf is whatever
# the distribution ships.
# ---------------------------------------------------------------------------
printf '\n  real distributions (unmodified, pinned digests):\n'
printf '    %-20s %-6s %-9s %-9s %-7s %s\n' ENVIRONMENT LIBC 'A plain' 'B nssfix' 'C ctrl' 'host NSS modules arm A pulled in'

# ⚠ THE EXIT CODE ALONE UNDERSTATES THIS. Fedora 42 loads two host NSS modules
# and still exits 0; Arch loads one and dies. Recording only the status would
# report Fedora as clean, when what is actually happening there is that a
# foreign libc got mapped into the process and the process survived it. The
# module list is the measurement; the exit code is one consequence of it.
nss_loaded() { # rootfs binary-path in-root-name -> comma list of host NSS modules opened
  _t="$B/trace-$2-$(basename "$1")"
  strace -f -e trace=openat -o "$_t" \
    "$REPO_DIR/pgb" rootfs run "$1" --copy "$3" -- "/$2" >/dev/null 2>&1
  _m=$(grep -oE '/[^"]*/libnss_[a-z0-9_]*\.so[^"]*' "$_t" 2>/dev/null \
       | sed 's|.*/||' | sort -u | tr '\n' ',' | sed 's/,$//')
  printf '%s' "${_m:-none}"
}

while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  r=$(exp_rootfs "$name")
  [ -n "$r" ] || { exp_skip "$name" "not fetched"; continue; }

  sa=$(exp_run_status "$r" "$B/probe-plain:/probe-plain"   /probe-plain)
  sb=$(exp_run_status "$r" "$B/probe-nssfix:/probe-nssfix" /probe-nssfix)
  sc=$(exp_run_status "$r" "$B/probe-nonss:/probe-nonss"   /probe-nonss)

  ma=$(nss_loaded "$r" probe-plain  "$B/probe-plain:/probe-plain")
  mb=$(nss_loaded "$r" probe-nssfix "$B/probe-nssfix:/probe-nssfix")

  d() { case "$1" in 0) printf 'ok' ;; 13[0-9]|1[4-6][0-9]) printf 'SIG%s' "$(($1-128))" ;; *) printf 'exit%s' "$1" ;; esac; }
  printf '    %-20s %-6s %-9s %-9s %-7s %s\n' "$name" "$libc" "$(d $sa)" "$(d $sb)" "$(d $sc)" "$ma"

  # ⭐ THE ONE ASSERTION THAT MATTERS. Whatever arm A did, arm B must pull in
  # NO host NSS module anywhere. This is the property the tool is built on, so
  # it is a gate rather than an observation.
  exp_check "$name: B loaded host NSS modules" "$mb" none

  # ⛔ ONLY ARM B IS ASSERTED. Arm A crashing is the FINDING this experiment
  # exists to record, not a failure of the experiment, so it is measured and
  # printed but never asserted -- an assertion on it would turn the discovery
  # into a red build. Arm C is asserted because if a plain static binary that
  # touches no NSS cannot run, nothing else measured here means anything.
  exp_check "$name: B (portable arm) runs" "$sb" 0
  exp_check "$name: C (no-NSS control) runs" "$sc" 0
  [ "$sa" = "$sb" ] || exp_note "$name: arm A and arm B DIFFER -- host NSS changes the outcome here"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

cp "$B/build-plain.log" "$EXP_OUT/linker-warning.txt" 2>/dev/null
exp_finish
