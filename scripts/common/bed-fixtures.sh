#!/bin/sh
# bed-fixtures.sh - give the pinned rootfs bed the host state a criterion needs.
#
# ⛔ WHY THIS FILE EXISTS, AND IT IS AN OPERATOR RULING RATHER THAN A
# CONVENIENCE. 2026-09-04c:
#
#   "You keep deferring stuff as 'unmeasurable' on this host when you very
#    well could create a script that can create a less 'minimal' image. You
#    will never have access to real hw, so keep stalling and deferring — use
#    fixtures, seams, emulators, dummies whenever a 'real' hardware is
#    required."
#
# Three criteria had been parked as "the bed cannot answer it", and every one
# of them was a fixture nobody had written:
#
#   a non-C LOCALE   `experiments/101-` was stopped because `setlocale` fails
#                    on all eleven, so gettext opens no catalogue and rung 3's
#                    criterion cannot fire.
#   a host THEME     0 of 11 rootfs carry anything under `usr/share/themes`,
#                    so "does the bundle follow the system theme" has nothing
#                    to follow.
#   a session DBUS   0 of 11 run one, so a tray application prints
#                    `Unable to connect via DBus` and draws nothing.
#
# ⭐ ALL THREE ARE HERE NOW, and the third took one more step to see. A session
# bus is a running PROCESS rather than a file, so at first it looked like it
# belonged to an experiment and not to the bed — but measured by hand, the
# thing that stops a bundle starting its OWN `dbus-daemon` is not the daemon:
# it is a FILE.
#
#     dbus[…]: Failed to start message bus:
#              Failed to open "/etc/dbus-1/session.conf": No such file
#
# ⛔ `dbus-daemon` reads an absolute `/etc` path with no search variable — the
# same class as `pdfarranger`'s `/usr/local/share/…`, and one the interposer
# does not reach because it is not `/nix/store`. ⭐ So the bed gets the config
# file (DATA), and the bundle brings the daemon (`--with-program dbus-daemon`,
# measured 2026-09-04c). Between them a subject that needs a session bus can
# be measured.
#
# ⚠ WHAT A FIXTURE MAY AND MAY NOT BE.
#   * It may add DATA — a locale, a theme, a catalogue. ⭐ Data cannot change
#     a host-shared-object count, which is the number every other experiment
#     in this tree depends on, so installing these cannot silently move
#     somebody else's result.
#   * ⛔ It may NOT add a shared object, a program, or a library search path.
#     That would change what "zero host objects" means, and no fixture is
#     worth that.
#   * ⛔ It is idempotent and reversible, and `--check` says which rootfs have
#     it, so a run can state the bed it measured rather than assuming one.
#
# ⚠ AND THE LOCALE FIXTURE IS GLIBC-ONLY, WHICH IS A PROPERTY OF musl AND NOT
# A GAP IN THIS SCRIPT. musl implements only the C and C.UTF-8 locales and has
# no `localedef` and no `/usr/lib/locale`, so a `de_DE.UTF-8` cannot exist on
# the four musl rows however hard this tries. ⭐ An experiment using it must
# report a 7/7 and a 4-row musl limit, not a 7/11.
#
# Usage:
#   sh scripts/common/bed-fixtures.sh --install [theme|locale|all]
#   sh scripts/common/bed-fixtures.sh --check
#   sh scripts/common/bed-fixtures.sh --remove
#   sh scripts/common/bed-fixtures.sh --selftest
#
# Exit: 0 ok, 1 something did not apply, 2 could not run.
set -u

HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../.." && pwd)
ROOTFS_DIR="${PGB_ROOTFS_DIR:-/var/lib/pgb-rootfs}"

# ⭐ THE THEME'S NAME IS DELIBERATELY NOT A REAL ONE. `Adwaita` exists on some
# machines and a test that passes because the host already had the theme is not
# a test. Nothing outside this tree ships `PgbFixture`.
THEME=PgbFixture
LOCALE=de_DE.UTF-8

envs() {
  awk '!/^#/ && NF {print $2}' "$REPO/scripts/common/rootfs-images.txt"
}

# ⛔ `-e` IS THE WRONG TEST INSIDE AN UNPACKED ROOT, AND THIS IS THE THIRD
# SIGHTING OF THAT ROOT CAUSE IN ONE DAY. Void's musl loader is
#
#     <root>/lib/ld-musl-x86_64.so.1 -> /usr/lib64/libc.so
#
# an ABSOLUTE link, and `-e` resolves it against the HOST root, where that
# path does not exist — so the first version of this function reported Void as
# glibc and tried to install a glibc locale into it. ⭐ `-L` asks whether the
# LINK is there, which is the question. docs/history/corrections.md C42, C43
# and C47: an absolute symlink inside an unpacked root must never be resolved
# against the machine holding it.
is_musl() {  # rootfs-path -> 0 when musl
  for _n in lib/ld-musl-x86_64.so.1 lib/libc.musl-x86_64.so.1 \
            usr/lib/ld-musl-x86_64.so.1 usr/lib/libc.musl-x86_64.so.1; do
    [ -L "$1/$_n" ] || [ -e "$1/$_n" ] && return 0
  done
  return 1
}

# ---------------------------------------------------------------------------
# The THEME fixture. A GTK 3 theme is a directory with an `index.theme` and a
# `gtk-3.0/gtk.css`; GTK finds it through `XDG_DATA_DIRS` and picks it when
# `GTK_THEME` names it. ⭐ The CSS sets one property that cannot come from
# anywhere else, so a test can assert the theme was READ rather than found.
# ---------------------------------------------------------------------------
install_theme() {  # rootfs-path
  _d="$1/usr/share/themes/$THEME"
  mkdir -p "$_d/gtk-3.0" "$_d/gtk-4.0" || return 1
  cat > "$_d/index.theme" <<EOF
[Desktop Entry]
Type=X-GNOME-Metatheme
Name=$THEME
Comment=a fixture installed by scripts/common/bed-fixtures.sh
EOF
  # ⚠ A colour no default theme uses, so a screenshot or a property read can
  # tell this theme from the built-in one.
  cat > "$_d/gtk-3.0/gtk.css" <<'EOF'
/* pgb bed fixture: one rule, one unmistakable value */
* { -gtk-icon-palette: success #010203; }
window { background-color: #010203; }
EOF
  cp "$_d/gtk-3.0/gtk.css" "$_d/gtk-4.0/gtk.css" || return 1
  # ⭐ An ICON theme too: the other half of "follows the system", and it is
  # what `XDG_DATA_DIRS` is actually searched for most often.
  _i="$1/usr/share/icons/$THEME"
  mkdir -p "$_i" || return 1
  cat > "$_i/index.theme" <<EOF
[Icon Theme]
Name=$THEME
Comment=a fixture installed by scripts/common/bed-fixtures.sh
Directories=16x16/apps
[16x16/apps]
Size=16
Type=Fixed
EOF
  mkdir -p "$_i/16x16/apps" || return 1
  return 0
}

have_theme() { [ -f "$1/usr/share/themes/$THEME/gtk-3.0/gtk.css" ]; }

remove_theme() {
  rm -rf "$1/usr/share/themes/$THEME" "$1/usr/share/icons/$THEME"
}

# ---------------------------------------------------------------------------
# The LOCALE fixture.
#
# ⛔ IT IS COMPILED ONCE, BY THE PINNED BUILD ENVIRONMENT, AND COPIED. Building
# it inside each rootfs is not possible — a minimal image has no `localedef`
# and no locale sources — and building it on the RUNNER would compile against
# the runner's glibc, which is not the one the bundle carries. ⭐ The pinned
# environment is the one place in this tree with a known glibc, so it is the
# one place a locale may be compiled.
#
# ⚠ A COMPILED LOCALE IS glibc-VERSION-SENSITIVE. glibc reads
# `/usr/lib/locale/<name>/LC_*` and checks a version stamp; a locale compiled
# by a much newer glibc can be rejected by an older one. The pinned
# environment is 2.41 and the oldest glibc row here is Rocky 8's 2.28, so the
# experiment that uses this MUST report which rows accepted it rather than
# assuming all seven did.
# ---------------------------------------------------------------------------
ENVROOT="$ROOTFS_DIR/$(awk -F'"' '/DefaultEnvName/ {print $2; exit}' \
                        "$REPO/internal/cfg/cfg.go" 2>/dev/null)"
LOCSRC=/var/tmp/pgb-bed-locale

build_locale() {
  [ -f "$LOCSRC/$LOCALE/LC_CTYPE" ] && return 0
  rm -rf "$LOCSRC"; mkdir -p "$LOCSRC" || return 2
  # ⛔ THE PINNED BUILD ENVIRONMENT CANNOT COMPILE IT, MEASURED: `debian:13`'s
  # minimal image ships `localedef` but not `/usr/share/i18n`, so it answers
  #   [error] cannot read character map directory `/usr/share/i18n/charmaps'
  # ⭐ The RUNNER can, and the runner's glibc is what compiles it. ⚠ THAT IS A
  # STATED PROPERTY OF THE FIXTURE, NOT A DETAIL: the locale is built by this
  # machine's glibc, and glibc checks a version stamp when it reads one. An
  # experiment using it MUST report which rootfs actually accepted it rather
  # than assuming the seven glibc rows did.
  command -v localedef >/dev/null 2>&1 || {
    echo "no localedef on this machine; apt-get install locales" >&2; return 2; }
  [ -d /usr/share/i18n/charmaps ] || {
    echo "no /usr/share/i18n; apt-get install locales" >&2; return 2; }
  # ⛔ --no-archive keeps it a DIRECTORY, which is the shape LOCPATH can point
  # at. glibc's locale ARCHIVE is the shape the field's HALL-OF-FAME says
  # LOCPATH cannot serve.
  localedef --no-archive -i de_DE -f UTF-8 "$LOCSRC/$LOCALE" \
      >"$LOCSRC/build.log" 2>&1 || {
    echo "localedef failed; see $LOCSRC/build.log" >&2; return 1; }
  [ -f "$LOCSRC/$LOCALE/LC_CTYPE" ]
}

install_locale() {  # rootfs-path
  is_musl "$1" && return 3          # 3 = not applicable, not a failure
  # ⛔ VERIFY THE SOURCE AND THE DESTINATION. The first version reported `ok`
  # for every row after `localedef` had failed, because it never checked
  # either — the same shape as C44's silent `cp`.
  [ -f "$LOCSRC/$LOCALE/LC_CTYPE" ] || return 1
  mkdir -p "$1/usr/lib/locale" || return 1
  rm -rf "$1/usr/lib/locale/$LOCALE"
  cp -a "$LOCSRC/$LOCALE" "$1/usr/lib/locale/$LOCALE" || return 1
  have_locale "$1"
}

have_locale() { [ -f "$1/usr/lib/locale/$LOCALE/LC_CTYPE" ]; }
remove_locale() { rm -rf "$1/usr/lib/locale/$LOCALE"; }

# ---------------------------------------------------------------------------
# The DBUS fixture: `/etc/dbus-1/session.conf`, and nothing else.
#
# ⛔ IT IS A CONFIG FILE, NOT A DAEMON. Nothing here installs a program: the
# bundle carries `dbus-daemon` and `dbus-run-session` out of its own closure
# (`--with-program`, measured). What the bed supplies is the file the daemon
# insists on reading from an absolute `/etc` path.
#
# ⚠ nixpkgs' OWN `session.conf` is not usable as-is here: it delegates the
# listening address to an `<include>` the bundle does not carry, and starting
# the daemon with it answers
#     Configuration file needs one or more <listen> elements giving addresses
# ⭐ So this is a MINIMAL session bus config, written out rather than copied,
# and its `<listen>` is a unix socket under /tmp — which every rootfs here
# mounts as a fresh tmpfs, so no run can see another's socket.
install_dbus() {  # rootfs-path
  mkdir -p "$1/etc/dbus-1" || return 1
  cat > "$1/etc/dbus-1/session.conf" <<'EOF'
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-BUS Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <listen>unix:tmpdir=/tmp</listen>
  <standard_session_servicedirs />
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
EOF
  have_dbus "$1"
}

have_dbus() { [ -f "$1/etc/dbus-1/session.conf" ]; }
remove_dbus() { rm -f "$1/etc/dbus-1/session.conf"; }

# ---------------------------------------------------------------------------
# ⭐ THE SELFTEST, and it runs against a temporary tree rather than the bed:
# a fixture installer that reports success without writing anything is exactly
# the class of check `docs/AGENTS.md` §0b calls the worst answer here.
# ---------------------------------------------------------------------------
selftest() {
  _t=$(mktemp -d) || return 2
  _fail=0
  install_theme "$_t" || _fail=$((_fail+1))
  have_theme "$_t" && echo "ok    theme installs and is detected" \
                   || { echo "FAIL  theme not detected after install"; _fail=$((_fail+1)); }
  grep -q '#010203' "$_t/usr/share/themes/$THEME/gtk-3.0/gtk.css" \
      && echo "ok    the css carries its unmistakable value" \
      || { echo "FAIL  css missing its value"; _fail=$((_fail+1)); }
  [ -f "$_t/usr/share/icons/$THEME/index.theme" ] \
      && echo "ok    the icon theme is installed too" \
      || { echo "FAIL  no icon theme"; _fail=$((_fail+1)); }
  remove_theme "$_t"
  have_theme "$_t" && { echo "FAIL  remove left the theme behind"; _fail=$((_fail+1)); } \
                   || echo "ok    remove is complete"
  # ⛔ THE ONE THAT MATTERS: a fixture must add no shared object, because every
  # other experiment's "zero host objects" depends on it.
  install_theme "$_t"
  _so=$(find "$_t" -name '*.so' -o -name '*.so.*' 2>/dev/null | grep -c . || true)
  [ "$_so" = 0 ] && echo "ok    the fixture adds no shared object ($_so)" \
                 || { echo "FAIL  the fixture added $_so shared object(s)"; _fail=$((_fail+1)); }
  # musl detection, both ways
  mkdir -p "$_t/lib"; : > "$_t/lib/ld-musl-x86_64.so.1"
  is_musl "$_t" && echo "ok    musl is detected by its loader" \
                || { echo "FAIL  musl not detected"; _fail=$((_fail+1)); }
  rm -f "$_t/lib/ld-musl-x86_64.so.1"
  is_musl "$_t" && { echo "FAIL  non-musl reported as musl"; _fail=$((_fail+1)); } \
                || echo "ok    a glibc tree is not reported as musl"
  install_dbus "$_t" >/dev/null && echo "ok    the dbus config installs" \
      || { echo "FAIL  dbus config not installed"; _fail=$((_fail+1)); }
  grep -q '<listen>' "$_t/etc/dbus-1/session.conf" 2>/dev/null \
      && echo "ok    the dbus config has a <listen> element" \
      || { echo "FAIL  no <listen>; the daemon refuses such a file"; _fail=$((_fail+1)); }
  remove_dbus "$_t"
  have_dbus "$_t" && { echo "FAIL  dbus remove left the file"; _fail=$((_fail+1)); } \
                  || echo "ok    dbus remove is complete"
  rm -rf "$_t"
  [ "$_fail" = 0 ] && { echo "VERDICT: the fixture installer works."; return 0; }
  echo "VERDICT: $_fail check(s) failed."; return 1
}

# ---------------------------------------------------------------------------
what=${1:---check}
which=${2:-all}
rc=0

case "$what" in
  --selftest) selftest; exit $? ;;
  --install)
    if [ "$which" = locale ] || [ "$which" = all ]; then
      build_locale || { echo "could not compile $LOCALE" >&2; rc=1; }
    fi
    for e in $(envs); do
      d="$ROOTFS_DIR/$e"
      [ -d "$d" ] || { printf '  %-22s SKIP (not fetched)\n' "$e"; continue; }
      _t=- ; _l=-
      if [ "$which" = theme ] || [ "$which" = all ]; then
        install_theme "$d" && _t=ok || { _t=FAIL; rc=1; }
      fi
      if [ "$which" = locale ] || [ "$which" = all ]; then
        install_locale "$d"; case $? in
          0) _l=ok ;; 3) _l='n/a (musl)' ;; *) _l=FAIL; rc=1 ;;
        esac
      fi
      _b=-
      if [ "$which" = dbus ] || [ "$which" = all ]; then
        install_dbus "$d" && _b=ok || { _b=FAIL; rc=1; }
      fi
      printf '  %-22s theme=%-6s locale=%-11s dbus=%s\n' "$e" "$_t" "$_l" "$_b"
    done
    ;;
  --check)
    for e in $(envs); do
      d="$ROOTFS_DIR/$e"
      [ -d "$d" ] || { printf '  %-22s SKIP (not fetched)\n' "$e"; continue; }
      have_theme  "$d" && _t=yes || _t=no
      if is_musl "$d"; then _l='n/a (musl)'
      else have_locale "$d" && _l=yes || _l=no; fi
      have_dbus "$d" && _b=yes || _b=no
      printf '  %-22s theme=%-6s locale=%-11s dbus=%s\n' "$e" "$_t" "$_l" "$_b"
    done
    ;;
  --remove)
    for e in $(envs); do
      d="$ROOTFS_DIR/$e"
      [ -d "$d" ] || continue
      remove_theme "$d"; remove_locale "$d"; remove_dbus "$d"
      printf '  %-22s removed\n' "$e"
    done
    ;;
  *) echo "usage: $0 --install|--check|--remove|--selftest [theme|locale|dbus|all]" >&2
     exit 2 ;;
esac
exit $rc
