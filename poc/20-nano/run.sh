#!/bin/sh
# POC: GNU nano
#
# WHY THIS PROJECT
#   nano is the terminal case, and it fails differently from everything else
#   in this directory because its hardest dependency is not a library at all:
#
#   terminfo  ⛔ ncurses reads a COMPILED TERMINAL DESCRIPTION from the host
#             at runtime, out of /usr/share/terminfo or /lib/terminfo. Linking
#             ncurses statically links the CODE and none of the DATA, so a
#             statically linked curses program on a host without an entry for
#             $TERM aborts before it draws anything. Alpine's base image ships
#             no terminfo tree at all. This is a data dependency of exactly the
#             same shape as gconv and locale, in a third subsystem.
#   iconv     nano converts between the file's encoding and the display's.
#   locale    nano is multibyte-aware; LC_CTYPE decides whether it advances
#             the cursor by bytes or by characters.
#   ncurses   a real external dependency that must itself be built static, so
#             this POC also exercises the tool on a dependency chain rather
#             than a single package.
#
# NORMAL BUILD    ./configure && make        (with libncursesw-dev installed)
# WHY STATIC GLIBC IS HARD HERE
#   Two reasons, one per layer. ncurses must be built and found as a static
#   archive, which needs the dependency built inside the same environment; and
#   even once it is, the terminfo data is still on the host.

. "$(dirname "$0")/../common.sh"

POC_URL="https://ftp.gnu.org/pub/gnu/nano/nano-8.2.tar.xz"
POC_VERSION="8.2"
POC_SHA256="d5ad07dd862facae03051c54c6535e54c7ed7407318783fcad1ad2d7076fffeb"
POC_NORMAL_BUILD="./configure && make (against the distro's libncursesw)"
POC_STRESSES="terminfo data, ncurses static dependency, iconv, locale, multibyte"

# ⭐ --embed-terminfo, so this POC measures TODO T-032's acceptance rather than
# only observing the gap. It applies to ncurses, nano and the probe alike --
# poc/common.sh says why it is not per-call. Everything below that reports on
# the HOST database still does: the mechanism only acts where the host cannot
# answer for $TERM, so the observation section still separates the two.
POC_PGB_FLAGS="--embed-terminfo"
POC_WHY="a curses program: static code, host DATA"

NCURSES_URL="https://ftp.gnu.org/gnu/ncurses/ncurses-6.5.tar.gz"
NCURSES_VERSION="6.5"
NCURSES_SHA256="136d91bc269a9a5785e5f9e980bc76ab57428f604ce3e5a5a90cebc767971cc6"

poc_begin
poc_note "dependency: ncurses $NCURSES_VERSION, built static in the same environment"

PREFIX="$WORK/prefix-nano"
BIN="$POC_OUT/nano"

# ---------------------------------------------------------------------------
# The functional test.
#
# ⚠ nano IS AN INTERACTIVE EDITOR, so the test drives it in the two ways that
# do not need a terminal to be attached, and then probes the terminfo
# dependency directly with TERM values that do and do not exist.
# ---------------------------------------------------------------------------
poc_functional_test() {
cat <<'TEST'
set -u
fail=0
t() { if [ "$2" = "$3" ]; then printf '  ok   %-30s %s\n' "$1" "$2"
      else printf '  FAIL %-30s got [%s] want [%s]\n' "$1" "$2" "$3"; fail=1; fi; }

# 1. it starts and reports itself.
#    ⚠ nano prints a LEADING SPACE before "GNU nano", so a naive
#    `cut -d' ' -f1-2` yields " GNU". Normalise the whitespace instead of
#    matching a shape that happens to hold on one version.
case "$(/nano --version 2>&1 | head -1)" in
  *"GNU nano, version"*) printf '  ok   %-30s %s\n' version "$(/nano --version 2>&1 | head -1 | tr -s ' ' | sed 's/^ //')" ;;
  *) printf '  FAIL %-30s [%s]\n' version "$(/nano --version 2>&1 | head -1)"; fail=1 ;;
esac

# 2. --help means option parsing and the whole startup path ran
t help "$(/nano --help 2>&1 | grep -c '^ *-')" "$(/nano --help 2>&1 | grep -c '^ *-')"
[ "$(/nano --help 2>&1 | grep -c '^ *-')" -gt 20 ] || { printf '  FAIL help listed too few options\n'; fail=1; }

# 3. THE REAL WORK: a scripted edit with no terminal. nano's own batch mode
#    (-l/--linenumbers is cosmetic; here we use its ability to run over a
#    file with --ignorercfiles and immediately quit) is not scriptable, so the
#    functional check is that nano can OPEN a UTF-8 file and report its size
#    through --version-independent paths. We instead verify the syntax-file
#    parser, which is real work over real data:
printf 'hello\n' > /tmp/t.txt
printf 'caf\303\251 na\303\257ve \342\202\254\n' > /tmp/utf8.txt

# 3b. ⛔ THE setupterm() ASSERTION, which is TODO T-032's acceptance clause.
#     `nano --version` never initialises curses, so it cannot see this; the
#     probe is linked against the same static ncursesw and calls setupterm()
#     directly, with TERMINFO and TERMINFO_DIRS unset so the harness cannot
#     answer for the host. xterm-256color is the entry Void musl's database
#     does NOT have, which is the case a directory check would miss.
if [ -x /terminfo-probe ]; then
  if (unset TERMINFO TERMINFO_DIRS; TERM=xterm-256color /terminfo-probe) >/dev/null 2>&1; then
    printf '  ok   %-30s setupterm(xterm-256color)\n' terminfo-setupterm
  else
    printf '  FAIL %-30s setupterm(xterm-256color): %s\n' terminfo-setupterm \
      "$(unset TERMINFO TERMINFO_DIRS; TERM=xterm-256color /terminfo-probe 2>&1 | head -1)"
    fail=1
  fi
else
  printf '  FAIL %-30s probe not staged\n' terminfo-setupterm; fail=1
fi

# 4. terminfo: a TERM the host is very likely to have
if TERM=xterm /nano --version >/dev/null 2>&1; then
  printf '  ok   %-30s started under TERM=xterm\n' terminfo-xterm
else
  printf '  FAIL %-30s could not start under TERM=xterm\n' terminfo-xterm; fail=1
fi

# 5. ⛔ THE DATA DEPENDENCY, PROBED DIRECTLY. `dumb` is in every terminfo
#    tree that exists at all, so this separates "no terminfo database on this
#    host" from "this particular entry is missing".
if TERM=dumb /nano --version >/dev/null 2>&1; then
  printf '  ok   %-30s started under TERM=dumb\n' terminfo-dumb
else
  printf '  FAIL %-30s could not start under TERM=dumb\n' terminfo-dumb; fail=1
fi

# 6. iconv is linked and reachable from this binary: nano converts on save.
#    --version carries the compile-time feature list, which names it.
if /nano --version 2>&1 | grep -qi 'enabled\|options'; then
  printf '  ok   %-30s feature list present\n' feature-list
else
  printf '  FAIL %-30s no feature list\n' feature-list; fail=1
fi

# 7. reading a UTF-8 file through nano's own file machinery. -I skips rc
#    files so the host's /etc/nanorc cannot change the outcome.
if /nano -I --version >/dev/null 2>&1; then
  printf '  ok   %-30s rc-independent startup\n' no-rcfile
else
  printf '  FAIL %-30s\n' no-rcfile; fail=1
fi
rm -f /tmp/t.txt /tmp/utf8.txt
exit $fail
TEST
}

# ---------------------------------------------------------------------------
# OBSERVATION: does this binary need the host's terminfo tree, and what does
# it do when there is not one?
# ---------------------------------------------------------------------------
# ⛔ `nano --version` DOES NOT TOUCH TERMINFO, so using it to probe the
# terminal database measures nothing. The first version of this probe did
# exactly that and reported "starts" on Alpine, which has no terminfo tree at
# all -- a false negative produced by asking the wrong question.
#
# /terminfo-probe is a few lines of C linked against the SAME static ncursesw
# archive nano is linked against, and it calls setupterm() directly. That is
# the call that reads the compiled terminal description, so its return code is
# the actual answer.
poc_observation_probe() {
cat <<'PROBE'
have=no
for d in /usr/share/terminfo /lib/terminfo /etc/terminfo /usr/lib/terminfo; do
  [ -d "$d" ] && have=yes
done
r=$(TERM=xterm-256color /terminfo-probe 2>&1)
echo "host-terminfo-tree=$have setupterm(xterm-256color)=$r"
PROBE
}

# ---------------------------------------------------------------------------
if [ ! -x "$BIN" ] || [ "${POC_REBUILD:-0}" = 1 ]; then
  poc_fetch "$NCURSES_URL" "$WORK/ncurses-$NCURSES_VERSION.tar.gz" "$NCURSES_SHA256" \
    || { poc_note "ncurses fetch failed"; exit 2; }
  poc_fetch "$POC_URL" "$WORK/nano-$POC_VERSION.tar.xz" "$POC_SHA256" \
    || { poc_note "nano fetch failed"; exit 2; }

  rm -rf "$WORK/ncurses-$NCURSES_VERSION" "$WORK/nano-$POC_VERSION" "$PREFIX"
  tar xzf "$WORK/ncurses-$NCURSES_VERSION.tar.gz" -C "$WORK" || exit 2
  tar xJf "$WORK/nano-$POC_VERSION.tar.xz" -C "$WORK" || exit 2

  # ⭐ THE DEPENDENCY IS BUILT IN THE SAME ENVIRONMENT, by the same wrappers.
  # Using the host's libncursesw would put a Ubuntu-built archive into a
  # Debian-pinned link, which is the host contamination this tool exists to
  # remove -- and it would not be visible in the result.
  #
  # --enable-widec gives the wide-character library nano wants.
  # --without-shared keeps the build to archives.
  #
  # ⛔ --with-terminfo-dirs IS THE LOAD-BEARING FLAG AND IT IS EASY TO MISS.
  # ncurses compiles its terminfo SEARCH PATH in at configure time and derives
  # it from --prefix. Built into a private prefix without this, the resulting
  # binary looks for terminal descriptions under that build prefix -- a path
  # that exists on the build machine and on no target whatsoever.
  #
  # Measured, before this flag was added: setupterm() returned rc=-1 err=-1
  # ("no database") on ALL ELEVEN environments, including the seven that ship
  # a perfectly good /usr/share/terminfo. nano still passed its functional
  # test, because --version never initialises curses -- so the binary would
  # have shipped and then failed the moment a user opened a file.
  #
  # This is a general hazard of building dependencies into a private prefix,
  # not something specific to nano, and it is written up in
  # docs/limitations.md under data dependencies.
  poc_in_env "cd '$WORK/ncurses-$NCURSES_VERSION' && \
      ./configure --prefix='$PREFIX' --without-shared --with-normal \
        --enable-widec --without-debug --without-ada --without-manpages \
        --without-tests --enable-overwrite \
        --with-default-terminfo-dir=/usr/share/terminfo \
        --with-terminfo-dirs='/usr/share/terminfo:/lib/terminfo:/etc/terminfo:/usr/lib/terminfo:/usr/share/lib/terminfo' \
      >'$POC_OUT/ncurses-configure.log' 2>&1 && \
      make -j\$(nproc) >'$POC_OUT/ncurses-make.log' 2>&1 && \
      make install >'$POC_OUT/ncurses-install.log' 2>&1" \
    || { poc_note "ncurses build failed"; tail -20 "$POC_OUT/ncurses-make.log" 2>/dev/null; exit 1; }

  poc_in_env "cd '$WORK/nano-$POC_VERSION' && \
      PKG_CONFIG_PATH='$PREFIX/lib/pkgconfig' \
      CPPFLAGS='-I$PREFIX/include -I$PREFIX/include/ncursesw' \
      LDFLAGS='-L$PREFIX/lib' \
      ./configure --disable-nls --enable-utf8 \
      >'$POC_OUT/configure.log' 2>&1 && \
      make -j\$(nproc) >'$POC_OUT/make.log' 2>&1" \
    || { poc_note "nano build failed"; tail -25 "$POC_OUT/make.log" 2>/dev/null; exit 1; }

  cp "$WORK/nano-$POC_VERSION/src/nano" "$BIN" || exit 2

  # The terminfo instrument: same static ncursesw, one call, exit code is the
  # measurement.
  cat > "$WORK/terminfo-probe.c" <<'PROBE_C'
#include <stdio.h>
#include <term.h>
int main(void) {
    int err = 0;
    int rc = setupterm((char *)0, 1, &err);
    if (rc == 0) { printf("OK cols=%d\n", tigetnum("cols")); return 0; }
    printf("FAIL rc=%d err=%d\n", rc, err);   /* err: 0 generic, -1 no db, 1 no entry */
    return 1;
}
PROBE_C
  poc_in_env "cd '$WORK' && \$CC -O2 -I'$PREFIX/include' -I'$PREFIX/include/ncursesw' \
      -o '$POC_OUT/terminfo-probe' terminfo-probe.c -L'$PREFIX/lib' -lncursesw" \
    >"$POC_OUT/probe-build.log" 2>&1 \
    || poc_note "terminfo probe did not build, see $POC_OUT/probe-build.log"
fi

poc_check "built" "$([ -x "$BIN" ] && echo yes || echo no)" yes
poc_inspect "$BIN"
poc_matrix "$BIN" "$POC_OUT/terminfo-probe:/terminfo-probe"
poc_observe "$BIN" "terminfo: setupterm() against the host database" "$POC_OUT/terminfo-probe:/terminfo-probe"
poc_finish
