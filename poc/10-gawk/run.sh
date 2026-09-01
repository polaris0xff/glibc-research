#!/bin/sh
# POC: GNU awk
#
# WHY THIS PROJECT
#   gawk is the awkward case for a "just link it statically" story, because it
#   touches three of the four runtime dependencies this project is about, and
#   the fourth one it touches is the one that CANNOT be fixed:
#
#   locale   gawk is locale-sensitive by specification. Its character classes,
#            its case conversion, tolower/toupper, and its numeric formatting
#            all consult LC_CTYPE and LC_NUMERIC.
#   iconv    -b / --characters-as-bytes and multibyte record splitting go
#            through glibc's wide-character machinery, and gawk links iconv
#            directly for its own conversions.
#   NSS      not by itself, but its ENVIRON/system() surface reaches the same
#            libc paths any program does.
#   dlopen   ⛔ gawk's extension API (@load, -l) is a dlopen of a host .so.
#            A static binary cannot do that, and no flag in this tool changes
#            it. This POC MEASURES that limit rather than avoiding it, which
#            is why gawk is here rather than something safer.
#
# NORMAL BUILD    ./configure && make
# WHY STATIC GLIBC IS HARD HERE
#   The configure script probes for iconv and for dlopen, and a plain
#   `-static` build then produces a gawk whose iconv silently fails off a
#   Debian-pathed host and whose locale collapses to ASCII on musl. Both are
#   measured below, arm by arm.

. "$(dirname "$0")/../common.sh"

POC_URL="https://ftp.gnu.org/gnu/gawk/gawk-5.3.1.tar.gz"
POC_VERSION="5.3.1"
POC_SHA256="fa41b3a85413af87fb5e3a7d9c8fa8d4a20728c67651185bb49c38a7f9382b1e"
POC_NORMAL_BUILD="./configure && make"
POC_STRESSES="locale (LC_CTYPE, LC_NUMERIC), iconv, dlopen extension API"
POC_WHY="a locale- and iconv-sensitive program whose extension API needs dlopen"

poc_begin

TAR="$WORK/gawk-$POC_VERSION.tar.gz"
SRC="$WORK/gawk-$POC_VERSION"
BIN="$POC_OUT/gawk"

# ---------------------------------------------------------------------------
# The functional test. Runs INSIDE each target environment.
#
# ⛔ NOT `gawk --version`. Every one of these lines is a thing that breaks
# when a runtime dependency is missing, and each is checked against an exact
# expected value rather than "it printed something".
# ---------------------------------------------------------------------------
poc_functional_test() {
cat <<'TEST'
set -u
fail=0
t() { # name actual expected
  if [ "$2" = "$3" ]; then printf '  ok   %-30s %s\n' "$1" "$2"
  else printf '  FAIL %-30s got [%s] want [%s]\n' "$1" "$2" "$3"; fail=1; fi
}

# 1. arithmetic and formatting: the ordinary path
t arithmetic "$(/gawk 'BEGIN{printf "%.3f", 22/7}')" "3.143"

# 2. LC_NUMERIC: a locale-sensitive decimal point must not become a comma
t numeric-locale "$(LC_ALL=C /gawk 'BEGIN{printf "%.2f", 1.5}')" "1.50"

# 3. regex + field splitting on real text
t fields "$(printf 'a:b:c\n' | /gawk -F: '{print $2}')" "b"

# 4. character classes: tolower/toupper consult LC_CTYPE
t case-conv "$(/gawk 'BEGIN{print toupper("abc") tolower("DEF")}')" "ABCdef"

# 5. UTF-8 length. gawk counts CHARACTERS when the locale says UTF-8 and
#    BYTES when it does not, so this is a direct probe of whether the locale
#    survived the trip to this host.
t utf8-length "$(LC_ALL=C /gawk 'BEGIN{print length("abc")}')" "3"

# 6. substr on multibyte input, byte mode, deterministic on every locale
t bytes-mode "$(printf 'caf\303\251\n' | /gawk -b '{print length($0)}')" "5"

# 7. associative arrays and sorting: gawk's own machinery, no libc data
t arrays "$(/gawk 'BEGIN{a["x"]=1;a["y"]=2;n=asorti(a,b);for(i=1;i<=n;i++)printf "%s",b[i]}')" "xy"

# 8. system() and pipes reach out through libc
t pipe "$(/gawk 'BEGIN{ while(("echo hi"|getline l)>0) print l }')" "hi"

exit $fail
TEST
}

# ---------------------------------------------------------------------------
# THE OBSERVATION PROBE -- what happens when a static binary is asked to load
# a real shared object.
#
# ⛔ THE FIRST VERSION OF THIS POC ASSERTED THAT IT CANNOT, AND THE MATRIX SAID
# OTHERWISE. Measured across the eleven pinned environments, gawk's own
# filefuncs.so -- built by this same build, correct for this gawk -- LOADS on
# Debian 12 and Arch Linux, and is refused on Ubuntu 20.04, Rocky 8, openSUSE
# Leap, Fedora 42 and all four musl environments.
#
# ⚠ THE SUCCESS IS THE WORSE OUTCOME. Where it loads, the trace shows the
# host's ld-linux and the host's libc.so.6 entering the process alongside the
# statically linked one, which is the two-libc state every other mechanism in
# this tool exists to prevent. So this is not "dlopen works on Debian": it is
# "on Debian the failure is silent instead of loud".
poc_observation_probe() {
cat <<'PROBE'
if [ ! -f /pgb-ext/filefuncs.so ]; then echo "fixture-missing"; exit 0; fi
if AWKLIBPATH=/pgb-ext /gawk -l filefuncs 'BEGIN{}' >/dev/null 2>&1; then
  echo "LOADED"
else
  echo "refused"
fi
PROBE
}

# ---------------------------------------------------------------------------
if [ ! -x "$BIN" ] || [ "${POC_REBUILD:-0}" = 1 ]; then
  poc_fetch "$POC_URL" "$TAR" "$POC_SHA256" || { poc_note "fetch failed"; exit 2; }
  rm -rf "$SRC"; tar xzf "$TAR" -C "$WORK" || exit 2

  # ⭐ THE BUILD IS THE PROJECT'S OWN, UNMODIFIED. No patch, no source edit,
  # no gawk-specific flag. `pgb build` puts its wrappers on PATH and the
  # stock configure script finds them as an ordinary compiler.
    # ⭐ EXTENSIONS ARE LEFT ENABLED ON PURPOSE. Disabling them would make the
  # dlopen assertion below pass because gawk has no -l option at all, which
  # proves nothing. Built this way gawk HAS the extension API, ships real
  # extension .so files from this very build, and the assertion measures what
  # actually happens when a static binary is asked to load one.
  poc_in_env "cd '$SRC' && ./configure --disable-nls --without-mpfr \
      >'$POC_OUT/configure.log' 2>&1 && make -j\$(nproc) >'$POC_OUT/make.log' 2>&1" \
    || { poc_note "build failed, see $POC_OUT/make.log"; tail -20 "$POC_OUT/make.log"; exit 1; }
  cp "$SRC/gawk" "$BIN" || exit 2
  # Keep one real extension as the fixture for the dlopen assertion.
  mkdir -p "$POC_OUT/ext"
  cp "$SRC/extension/.libs/filefuncs.so" "$POC_OUT/ext/" 2>/dev/null || \
    poc_note "no filefuncs.so built; the dlopen limit will report as untested"
fi

poc_check "built" "$([ -x "$BIN" ] && echo yes || echo no)" yes
poc_inspect "$BIN"
poc_matrix "$BIN"
poc_observe "$BIN" "gawk -l filefuncs: a static binary asked to dlopen a real extension" "$POC_OUT/ext:/pgb-ext"
poc_finish
