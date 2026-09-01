#!/bin/sh
# POC: curl
#
# WHY THIS PROJECT
#   curl is the direct test of the failure this whole project started from.
#
#   NSS   ⛔ curl resolves names with getaddrinfo(), which is THE function the
#         linker warns about at every static link: "Using 'getaddrinfo' in
#         statically linked applications requires at runtime the shared
#         libraries from the glibc version used for linking". Everything
#         experiment 20 measured -- the host nsswitch.conf, the dlopen of host
#         NSS modules, the SIGFPE on Arch and openSUSE -- is on curl's main
#         path, not a corner of it.
#   TLS   a real cryptographic dependency (OpenSSL) that must be built static,
#         plus a CA bundle, which is a HOST DATA dependency in the same family
#         as gconv, locale and terminfo. A curl that resolves but cannot
#         verify a certificate has not been made portable.
#   deps  zlib and OpenSSL both built in the same environment: three packages
#         in a chain.
#
# NORMAL BUILD    ./configure --with-openssl && make
# WHY STATIC GLIBC IS HARD HERE
#   The name resolution path is the one glibc explicitly does not support
#   statically. Without the NSS override this binary is the exact program that
#   dies on Arch Linux.

. "$(dirname "$0")/../common.sh"

POC_URL="https://curl.se/download/curl-8.11.0.tar.gz"
POC_VERSION="8.11.0"
POC_SHA256="264537d90e58d2b09dddc50944baf3c38e7089151c8986715e2aaeaaf2b8118f"
POC_NORMAL_BUILD="./configure --with-openssl && make"
POC_STRESSES="getaddrinfo/NSS, DNS, TLS, CA bundle data, 3-package dependency chain"

# ⭐ --embed-cacert, so this POC measures TODO T-032's acceptance rather than
# only observing the gap. The mechanism finds the host's own trust store
# wherever it is and materialises an embedded copy only where there is none;
# the observation section below still reports which of the two happened.
POC_PGB_FLAGS="--embed-cacert"
POC_WHY="the getaddrinfo case the linker warns about, end to end over real TLS"

SSL_URL="https://github.com/openssl/openssl/releases/download/openssl-3.0.15/openssl-3.0.15.tar.gz"
SSL_VERSION="3.0.15"
SSL_SHA256="23c666d0edf20f14249b3d8f0368acaee9ab585b09e1de82107c66e1f3ec9533"
ZLIB_URL="https://zlib.net/fossils/zlib-1.3.1.tar.gz"
ZLIB_VERSION="1.3.1"
ZLIB_SHA256="9a93b2b7dfdac77ceba5a558a580e74667dd6fede4585b91eefb60f03b72df23"

poc_begin
poc_note "dependencies: OpenSSL $SSL_VERSION and zlib $ZLIB_VERSION, both built static in the same environment"

PREFIX="$WORK/prefix-curl"
BIN="$POC_OUT/curl"

# ---------------------------------------------------------------------------
# ⚠ THIS TEST NEEDS THE NETWORK, and it says so rather than passing quietly
# when there is none. A DNS test that silently degrades to "no network, call
# it a pass" is how a name-resolution regression ships.
# ---------------------------------------------------------------------------
poc_functional_test() {
cat <<'TEST'
set -u
fail=0
t() { if [ "$2" = "$3" ]; then printf '  ok   %-30s %s\n' "$1" "$2"
      else printf '  FAIL %-30s got [%s] want [%s]\n' "$1" "$2" "$3"; fail=1; fi; }

# 1. it starts, and reports the features it was built with
t version "$(/curl --version 2>&1 | head -1 | cut -d' ' -f1)" "curl"

# 2. TLS is actually compiled in. A curl without it would pass every plaintext
#    check below and be useless.
if /curl --version 2>&1 | grep -qi 'openssl'; then
  printf '  ok   %-30s OpenSSL present\n' tls-built
else
  printf '  FAIL %-30s no TLS backend\n' tls-built; fail=1
fi

# 3. ⭐ THE NSS PATH, WITHOUT THE NETWORK. Resolving a name out of /etc/hosts
#    goes through exactly the same getaddrinfo() dispatcher as a DNS lookup,
#    so this exercises the NSS machinery even where there is no route out.
#    --connect-timeout keeps a firewalled environment from hanging.
printf '127.0.0.1 pgb-poc-local\n' >> /etc/hosts 2>/dev/null || true
out=$(/curl -sS --connect-timeout 5 http://pgb-poc-local:1/ 2>&1)
case "$out" in
  *"Could not resolve"*|*"Couldn't resolve"*)
    printf '  FAIL %-30s NSS could not resolve a /etc/hosts name\n' nss-hosts-file; fail=1 ;;
  *)
    printf '  ok   %-30s resolved via files, connection refused as expected\n' nss-hosts-file ;;
esac

# 4. localhost, the name every resolver is expected to know
out=$(/curl -sS --connect-timeout 5 http://localhost:1/ 2>&1)
case "$out" in
  *"resolve"*) printf '  FAIL %-30s could not resolve localhost\n' nss-localhost; fail=1 ;;
  *)           printf '  ok   %-30s resolved\n' nss-localhost ;;
esac

# 5. numeric address: no NSS at all. If THIS fails the problem is not NSS,
#    which is what makes it the control for checks 3 and 4.
out=$(/curl -sS --connect-timeout 5 http://127.0.0.1:1/ 2>&1)
case "$out" in
  *"resolve"*) printf '  FAIL %-30s numeric address went through a resolver\n' numeric-control; fail=1 ;;
  *)           printf '  ok   %-30s control: no resolver involved\n' numeric-control ;;
esac

# 6. url parsing and local file transfer: real work, no network
printf 'hello pgb\n' > /tmp/src.txt
t file-scheme "$(/curl -sS file:///tmp/src.txt 2>&1)" "hello pgb"

# 7. ⭐ REAL DNS AND REAL TLS, when there is a route out. Skipped loudly, never
#    silently, so a green run on an offline machine cannot be mistaken for
#    evidence that DNS works.
if /curl -sS --connect-timeout 8 --max-time 25 -o /tmp/out.html \
        https://example.com/ 2>/tmp/err.txt; then
  if [ -s /tmp/out.html ]; then
    printf '  ok   %-30s real DNS + TLS handshake + body\n' live-https
  else
    printf '  FAIL %-30s empty body\n' live-https; fail=1
  fi
else
  e=$(head -c 120 /tmp/err.txt)
  case "$e" in
    *"resolve"*)
      printf '  FAIL %-30s DNS FAILED: %s\n' live-https "$e"; fail=1 ;;
    *"certificate"*|*"CA"*|*"SSL"*)
      # ⛔ THIS USED TO BE `ok`, AND IT WAS RIGHT TO BE. Before
      # --embed-cacert there was no mechanism, so a missing trust store was a
      # DATA dependency being reported rather than a defect. With the
      # mechanism it is a failure: the store is supposed to be found now.
      printf '  FAIL %-30s TLS trust store not found: %s\n' live-https "$e"; fail=1 ;;
    *)
      printf '  SKIP %-30s no route out: %s\n' live-https "$e" ;;
  esac
fi
# 8. ⭐ TODO T-032's ACCEPTANCE CLAUSE: verify TLS with the HARNESS's own CA
#    variables unset, so what answers is the host's store or the embedded
#    copy and never the development proxy's anchor. The observation section
#    below explains why that distinction was worth a correction.
(
  unset CURL_CA_BUNDLE SSL_CERT_FILE SSL_CERT_DIR CURL_CA_PATH
  if /curl -sS --connect-timeout 8 --max-time 25 -o /dev/null https://example.com/ 2>/tmp/e2.txt; then
    printf '  ok   %-30s harness CA variables unset\n' https-verify-host
  else
    printf '  FAIL %-30s harness CA variables unset: %s\n' https-verify-host \
      "$(head -c 100 /tmp/e2.txt)"
    exit 1
  fi
) || fail=1

rm -f /tmp/src.txt /tmp/out.html /tmp/err.txt /tmp/e2.txt
exit $fail
TEST
}

# ---------------------------------------------------------------------------
# OBSERVATION: the CA bundle. curl compiles in a default path; whether the
# host has anything there is a property of the host.
#
# ⛔ THE INSTRUMENT WAS PERTURBING THE MEASUREMENT, and the first run of this
# probe reported a false result because of it.
#
# docs/methodology/experiments.md: "check whether observing changed the answer
# ... a probe that had to relax one setting to see anything CHANGED what it
# was watching". Here nothing was relaxed, but something was ADDED: this
# development environment routes HTTPS through a proxy and exports
# CURL_CA_BUNDLE and SSL_CERT_FILE pointing at the proxy's own trust anchor,
# which scripts/common/rootfs-run.sh replicates into the target so builds can
# fetch. curl reads CURL_CA_BUNDLE in preference to its compiled-in path.
#
# The result was https-verify=verified on Debian 11, Debian 12 and Ubuntu
# 20.04 -- images whose own scan says host-ca-bundle=none. Those three had no
# trust store at all and "verified" was measuring the HARNESS.
#
# Unsetting both variables inside the probe measures the host. The functional
# test above deliberately does NOT unset them: there its job is to prove DNS
# and the TLS handshake work, and the trust anchor is a means to that.
poc_observation_probe() {
cat <<'PROBE'
have=none
for f in /etc/ssl/certs/ca-certificates.crt /etc/pki/tls/certs/ca-bundle.crt \
         /etc/ssl/ca-bundle.pem /etc/ssl/cert.pem; do
  [ -f "$f" ] && { have="$f"; break; }
done
unset CURL_CA_BUNDLE SSL_CERT_FILE SSL_CERT_DIR CURL_CA_PATH
if /curl -sS --connect-timeout 8 --max-time 20 -o /dev/null https://example.com/ 2>/dev/null; then
  v=verified
else
  v=unverified
fi
echo "host-ca-bundle=$have host-only-verify=$v"
PROBE
}

# ---------------------------------------------------------------------------
if [ ! -x "$BIN" ] || [ "${POC_REBUILD:-0}" = 1 ]; then
  poc_fetch "$ZLIB_URL" "$WORK/zlib-$ZLIB_VERSION.tar.gz" "$ZLIB_SHA256" || exit 2
  poc_fetch "$SSL_URL"  "$WORK/openssl-$SSL_VERSION.tar.gz" "$SSL_SHA256" || exit 2
  poc_fetch "$POC_URL"  "$WORK/curl-$POC_VERSION.tar.gz" "$POC_SHA256" || exit 2

  rm -rf "$WORK/zlib-$ZLIB_VERSION" "$WORK/openssl-$SSL_VERSION" "$WORK/curl-$POC_VERSION" "$PREFIX"
  for t in zlib-$ZLIB_VERSION openssl-$SSL_VERSION curl-$POC_VERSION; do
    tar xzf "$WORK/$t.tar.gz" -C "$WORK" || exit 2
  done

  poc_in_env "cd '$WORK/zlib-$ZLIB_VERSION' && ./configure --prefix='$PREFIX' --static \
      >'$POC_OUT/zlib-configure.log' 2>&1 && make -j\$(nproc) >'$POC_OUT/zlib-make.log' 2>&1 && \
      make install >>'$POC_OUT/zlib-make.log' 2>&1" \
    || { poc_note "zlib build failed"; tail -15 "$POC_OUT/zlib-make.log" 2>/dev/null; exit 1; }

  # ⚠ no-shared AND no-dso. OpenSSL's engine machinery is a dlopen, and a
  # static build that still believes it can load engines carries the same
  # two-libc hazard the gawk POC measured.
  poc_in_env "cd '$WORK/openssl-$SSL_VERSION' && \
      ./Configure linux-x86_64 --prefix='$PREFIX' --openssldir='$PREFIX/ssl' \
        no-shared no-dso no-tests \
      >'$POC_OUT/openssl-configure.log' 2>&1 && \
      make -j\$(nproc) >'$POC_OUT/openssl-make.log' 2>&1 && \
      make install_sw >>'$POC_OUT/openssl-make.log' 2>&1" \
    || { poc_note "openssl build failed"; tail -20 "$POC_OUT/openssl-make.log" 2>/dev/null; exit 1; }

  # ⭐ --with-ca-bundle names the path curl looks for at runtime. Left to
  # configure it would record the BUILD host's path, which is a Debian path
  # and wrong on Fedora, Rocky, Arch and Alpine alike. Naming the common
  # Debian/Ubuntu location and letting --ca-native pick up others is a
  # deliberate, stated choice, and the observation arm measures what each host
  # actually has.
  poc_in_env "cd '$WORK/curl-$POC_VERSION' && \
      PKG_CONFIG_PATH='$PREFIX/lib/pkgconfig:$PREFIX/lib64/pkgconfig' \
      ./configure --with-openssl='$PREFIX' --with-zlib='$PREFIX' \
        --disable-shared --enable-static --disable-ldap --disable-ldaps \
        --without-libpsl --without-libidn2 --without-brotli --without-zstd \
        --without-nghttp2 --without-librtmp \
        --with-ca-bundle=/etc/ssl/certs/ca-certificates.crt \
      >'$POC_OUT/configure.log' 2>&1 && \
      make -j\$(nproc) >'$POC_OUT/make.log' 2>&1" \
    || { poc_note "curl build failed"; tail -25 "$POC_OUT/make.log" 2>/dev/null; exit 1; }

  cp "$WORK/curl-$POC_VERSION/src/curl" "$BIN" || exit 2
fi

poc_check "built" "$([ -x "$BIN" ] && echo yes || echo no)" yes
poc_inspect "$BIN"
poc_matrix "$BIN"
poc_observe "$BIN" "TLS trust: does the host have a CA bundle where curl looks"
poc_finish
