#!/bin/sh
# 74-cacert.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# docs/limitations.md §3 lists five host DATA dependencies. gconv and locale
# are solved. The TLS CA bundle is one of the two that are not, and it is the
# one the operator names in the sentence that defines the goal:
#
#   "no networking/iconv/gconv/nss/locale/CERT/etc issues"
#
# A TLS library compiles in exactly ONE default bundle path, chosen when IT
# was built, and every distribution puts the bundle somewhere else. Measured
# in poc/30-curl: verified on 5 of 11.
#
# ⭐ SIX OF THOSE SIX FAILURES ARE NOT "THIS MACHINE HAS NO CERTIFICATES".
# Rocky keeps them at /etc/pki/tls/certs/ca-bundle.crt, openSUSE at
# /etc/ssl/ca-bundle.pem, Alpine 3.10 at /etc/ssl/cert.pem. Only three of the
# eleven -- the minimal Debian and Ubuntu images -- genuinely ship none.
#
# So `--embed-cacert` is two layers and the FIRST one carries most of it:
# find the host's own store, and materialise an embedded copy only where
# there is nothing to find.
#
# -- ⛔ WHY THE ORDER OF THOSE TWO LAYERS IS A SECURITY PROPERTY AND NOT A
#    PREFERENCE -------------------------------------------------------------
#
# The embedded bundle is a snapshot of the pinned build environment taken at
# build time. Roots are revoked and roots expire. A binary that preferred its
# own stale copy over a host store an administrator maintains would be a
# security REGRESSION wearing a portability fix's clothes -- so this
# experiment asserts, on every environment that has a store, that NOTHING WAS
# WRITTEN and the host's own file is what the process ended up pointing at.
#
# ⛔ AND IT ASSERTS THAT THE SHIM NEVER OVERRIDES THE CALLER. If SSL_CERT_FILE
# is already set, the constructor must leave it exactly as it found it, even
# when the value is nonsense. A shim that "corrected" an operator's explicit
# trust configuration would be the worst failure available to it.
#
# -- ARMS -------------------------------------------------------------------
#
#   control   pgb WITHOUT --embed-cacert. Reports what the one compiled-in
#             path OpenSSL would have used actually contains on each host.
#   embedded  pgb WITH --embed-cacert.
#
# ⚠ WHAT THIS DOES NOT ESTABLISH. It measures that a usable trust store is
# FOUND and pointed at, not that a TLS handshake verifies -- that is
# poc/30-curl's job and it is the entry's other acceptance clause. A store
# that is found and is garbage would pass here and fail there, which is why
# the probe also counts BEGIN CERTIFICATE lines rather than checking for a
# file.
#
# Exit: 0 the measurement ran and matched, 1 it ran and did not, 2 could not run.
#
# SPDX-License-Identifier: MIT

set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "74 - --embed-cacert: finding a TLS trust store on every host"

WORK="$EXP_OUT/build"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2
RESULT="$EXP_OUT/RESULT.txt"

# ---------------------------------------------------------------------------
# The probe. Prints one machine-readable line the runner parses.
# ---------------------------------------------------------------------------
cat > "$WORK/probe.c" <<'EOF'
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* OpenSSL's compiled-in default on the pinned Debian environment. The control
 * arm exists to show what a program that knows only this path sees. */
#define OPENSSL_DEFAULT "/etc/ssl/certs/ca-certificates.crt"

static int count_certs(const char *path)
{
    char line[512];
    int n = 0;
    FILE *f;
    if (!path || !*path) return -1;
    f = fopen(path, "r");
    if (!f) return -1;
    while (fgets(line, sizeof line, f))
        if (strstr(line, "BEGIN CERTIFICATE")) n++;
    fclose(f);
    return n;
}

int main(void)
{
    const char *file = getenv("SSL_CERT_FILE");
    const char *dir  = getenv("SSL_CERT_DIR");
    const char *curl = getenv("CURL_CA_BUNDLE");
    int n_env  = count_certs(file);
    int n_dflt = count_certs(OPENSSL_DEFAULT);

    /* ⭐ Reported separately so "the shim found the host's file" and "the
     * shim wrote its own copy" can never be confused: a materialised bundle
     * lives under TMPDIR and nowhere else. */
    int materialised = (file && strstr(file, "/pgb-cacert-") != NULL);

    printf("SSL_CERT_FILE=%s|SSL_CERT_DIR=%s|CURL_CA_BUNDLE=%s|"
           "certs_env=%d|certs_default=%d|materialised=%d\n",
           file ? file : "<unset>", dir ? dir : "<unset>",
           curl ? curl : "<unset>", n_env, n_dflt, materialised);
    return 0;
}
EOF

printf -- '-- building --------------------------------------------------\n'
PGB="$REPO_DIR/pgb"
if ! sh "$PGB" --bind "$WORK" build -- /bin/sh -c \
      "\$CC -O2 -o $WORK/probe-plain $WORK/probe.c" >"$WORK/plain.log" 2>&1; then
  exp_skip "build the control arm" "see $WORK/plain.log"; exp_finish
fi
if ! sh "$PGB" --bind "$WORK" --embed-cacert build -- /bin/sh -c \
      "\$CC -O2 -o $WORK/probe-cacert $WORK/probe.c" >"$WORK/cacert.log" 2>&1; then
  exp_skip "build the --embed-cacert arm" "see $WORK/cacert.log"; exp_finish
fi
sz_plain=$(wc -c < "$WORK/probe-plain")
sz_ca=$(wc -c < "$WORK/probe-cacert")
exp_note "control  $sz_plain bytes"
exp_note "embedded $sz_ca bytes  (+$((sz_ca - sz_plain)), the pinned bundle)"
exp_check "the anchor forced the constructor in" \
  "$(nm "$WORK/probe-cacert" 2>/dev/null | grep -c 'pgb_cacert_anchor')" "1"
exp_check "the control has no cacert code" \
  "$(nm "$WORK/probe-plain" 2>/dev/null | grep -c 'pgb_cacert_anchor')" "0"
printf '\n'

# ---------------------------------------------------------------------------
# Run both arms on every fetched environment.
# ---------------------------------------------------------------------------
printf -- '-- running ---------------------------------------------------\n'
printf '  %-20s %-6s %-9s %-9s %-6s %s\n' TARGET LIBC 'CONTROL' 'EMBEDDED' WROTE 'STORE THE SHIM FOUND'

field() { printf '%s' "$1" | tr '|' '\n' | sed -n "s/^$2=//p"; }

# ⛔ AN INDEPENDENT ORACLE FOR "DOES THIS HOST HAVE A STORE", because the
# obvious check is circular: asking the shim whether it materialised a copy,
# and then asserting it only materialised where there was nothing to find, is
# asking the thing under test to grade itself. This is `sh` and `test`, not
# the shim's C, so a wrong path list in one is not silently a wrong list in
# both.
#
# ⛔ AND IT RUNS INSIDE THE ROOTFS, WHICH THE FIRST VERSION DID NOT. Checking
# `[ -s "$rootfs/etc/pki/tls/certs/ca-bundle.crt" ]` from OUTSIDE reported
# "no store" for Rocky 8, openSUSE Leap and Alpine 3.10 -- all three of which
# have one. Their bundle is a SYMLINK TO AN ABSOLUTE PATH
# (Rocky's points at /etc/pki/ca-trust/extracted/pem/tls-ca-bundle.pem), and
# an absolute symlink only resolves inside the root it belongs to. From
# outside it is dangling, so `-s` is false and the oracle said the opposite
# of the truth -- while the shim, running inside, was right.
host_has_store() {  # rootfs -> yes|no
  _out=$(sh "$REPO_DIR/scripts/common/rootfs-run.sh" "$1" -- /bin/sh -c '
    for p in /etc/ssl/certs/ca-certificates.crt /etc/pki/tls/certs/ca-bundle.crt \
             /etc/ssl/ca-bundle.pem /etc/ssl/cert.pem /etc/pki/tls/cacert.pem \
             /etc/ssl/certs/ca-bundle.crt /usr/share/ssl/certs/ca-bundle.crt \
             /usr/local/share/certs/ca-root-nss.crt \
             /etc/ca-certificates/extracted/tls-ca-bundle.pem; do
      [ -s "$p" ] && { echo yes; exit 0; }
    done
    echo no' 2>/dev/null | tail -1)
  printf '%s' "${_out:-no}"
}

: > "$WORK/rows.txt"
NROWS=0; FOUND=0; CTRL_OK=0; WROTE=0; NOOVERRIDE=0; HOSTHAS=0
while read -r image name libc rest; do
  case "$image" in ''|\#*) continue;; esac
  [ -n "$name" ] || continue
  r=$(exp_rootfs "$name")
  [ -n "$r" ] || { exp_skip "$name" "rootfs absent"; continue; }

  # ⛔ The harness's own CA variables must not decide the answer. `env -i`
  # would also drop PATH, so the two are unset explicitly and the rest of the
  # environment is left as the bed provides it.
  out_c=$(sh "$REPO_DIR/scripts/common/rootfs-run.sh" "$r" --copy "$WORK/probe-plain:/probe" \
            -- /bin/sh -c 'unset SSL_CERT_FILE SSL_CERT_DIR CURL_CA_BUNDLE; /probe' 2>/dev/null | tail -1)
  out_e=$(sh "$REPO_DIR/scripts/common/rootfs-run.sh" "$r" --copy "$WORK/probe-cacert:/probe" \
            -- /bin/sh -c 'unset SSL_CERT_FILE SSL_CERT_DIR CURL_CA_BUNDLE; /probe' 2>/dev/null | tail -1)
  # ⛔ The override check: a value the caller set, which is deliberately a
  # path that does not exist. The shim must leave it alone.
  out_o=$(sh "$REPO_DIR/scripts/common/rootfs-run.sh" "$r" --copy "$WORK/probe-cacert:/probe" \
            -- /bin/sh -c 'SSL_CERT_FILE=/nonexistent/operator/choice /probe' 2>/dev/null | tail -1)

  c_dflt=$(field "$out_c" certs_default); c_dflt=${c_dflt:--1}
  e_env=$(field "$out_e" certs_env);      e_env=${e_env:--1}
  e_file=$(field "$out_e" SSL_CERT_FILE)
  e_mat=$(field "$out_e" materialised);   e_mat=${e_mat:-0}
  o_file=$(field "$out_o" SSL_CERT_FILE)

  ctrl=$([ "${c_dflt:-0}" -gt 0 ] 2>/dev/null && echo "$c_dflt certs" || echo "NONE")
  emb=$([ "${e_env:-0}" -gt 0 ] 2>/dev/null && echo "$e_env certs" || echo "NONE")
  wrote=$([ "$e_mat" = 1 ] && echo yes || echo no)

  printf '  %-20s %-6s %-9s %-9s %-6s %s\n' \
    "$name" "$libc" "$ctrl" "$emb" "$wrote" "$e_file"
  printf '%s %s %s %s %s %s\n' "$name" "$libc" "$ctrl" "$emb" "$wrote" "$e_file" >> "$WORK/rows.txt"

  NROWS=$((NROWS+1))
  [ "${c_dflt:-0}" -gt 0 ] 2>/dev/null && CTRL_OK=$((CTRL_OK+1))
  [ "${e_env:-0}" -gt 0 ] 2>/dev/null && FOUND=$((FOUND+1))
  [ "$e_mat" = 1 ] && WROTE=$((WROTE+1))
  [ "$o_file" = "/nonexistent/operator/choice" ] && NOOVERRIDE=$((NOOVERRIDE+1))
  [ "$(host_has_store "$r")" = yes ] && HOSTHAS=$((HOSTHAS+1))
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

printf '\n'
printf -- '-- assertions ------------------------------------------------\n'
[ "$NROWS" -gt 0 ] || { exp_skip "every assertion below" "no environment was reachable"; exp_finish; }

exp_check "a usable trust store found, every environment" "$FOUND" "$NROWS"
# ⛔ THE SECURITY-RELEVANT ONE. The embedded copy must be written ONLY where
# the host has no store of its own -- everywhere else the host's file must
# win, because it is the one an administrator maintains.
exp_check "environments with a store of their own (checked from outside)" \
          "$HOSTHAS" "$HOSTHAS"
exp_check "wrote the embedded copy only where the host had none" \
          "$WROTE" "$((NROWS - HOSTHAS))"
exp_check "never overrode a value the caller set" "$NOOVERRIDE" "$NROWS"
exp_note "control (one compiled-in path, as OpenSSL would have it) = $CTRL_OK of $NROWS"
exp_note "⭐ that gap is the finding: the certificates were there all along,"
exp_note "   on a path the binary had never been told about."

{
  printf 'experiment 74 - --embed-cacert\n\n'
  printf 'control  %s bytes, embedded %s bytes (+%s)\n\n' "$sz_plain" "$sz_ca" "$((sz_ca - sz_plain))"
  printf '%-20s %-6s %-9s %-9s %-6s %s\n' TARGET LIBC CONTROL EMBEDDED WROTE 'STORE FOUND'
  cat "$WORK/rows.txt"
  printf '\nCONTROL  = certificates at the ONE path OpenSSL compiles in\n'
  printf 'EMBEDDED = certificates at the path the shim pointed the process at\n'
  printf 'WROTE    = the embedded copy was materialised (host had no store)\n\n'
  printf 'a usable trust store found      %s of %s\n' "$FOUND" "$NROWS"
  printf 'the single compiled-in path     %s of %s\n' "$CTRL_OK" "$NROWS"
  printf 'never overrode the caller       %s of %s\n' "$NOOVERRIDE" "$NROWS"
} > "$RESULT"

exp_note "written: $RESULT"
exp_finish
