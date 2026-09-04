#!/bin/sh
# THE QUESTION — and it exists because two criteria were WRONGLY parked.
#
#   ⛔ OPERATOR RULING, 2026-09-04c:
#
#     "You keep deferring stuff as 'unmeasurable' on this host when you very
#      well could create a script that can create a less 'minimal' image. You
#      will never have access to real hw, so keep stalling and deferring — use
#      fixtures, seams, emulators, dummies whenever a 'real' hardware is
#      required."
#
#   Two things in this tree had been recorded as *the bed cannot answer it*:
#
#     a NON-C LOCALE   `experiments/101-` was stopped and T-087 rung 3's
#                      locale criterion declared unmeasurable, because
#                      `setlocale(LC_ALL,"de_DE.UTF-8")` fails on all eleven.
#     a HOST THEME     0 of 11 rootfs carry anything under `usr/share/themes`,
#                      so "does the bundle follow the system theme" had
#                      nothing to follow.
#
#   ⭐ BOTH ARE FIXTURES, AND `scripts/common/bed-fixtures.sh` BUILDS THEM.
#   This experiment measures what they make measurable.
#
# -- ⛔ PRE-REGISTERED EXPECTATIONS -----------------------------------------
#
#   L0  the fixture is installed where it can be: 7 glibc rows, and the 4 musl
#       rows report `n/a` rather than a failure. ⚠ musl implements only C and
#       C.UTF-8; that is a property of musl, not a gap in the fixture.
#   L1  ⭐ `setlocale(LC_ALL,"de_DE.UTF-8")` SUCCEEDS on the glibc rows.
#       ⛔ This is the claim "not measurable in this bed" was made about, and
#       it is a `pgb` STATIC binary asking — so it is our own glibc 2.41
#       reading a locale this machine compiled, which is the shape that
#       matters for our artefacts.
#       ⚠ NOT PREDICTED AS 7/7: a compiled locale carries a version stamp and
#       the oldest row here is Rocky 8's glibc 2.28. The number is REPORTED.
#   L2  ⛔ THE WITHIN-ROW NEGATIVE CONTROL. The same probe asking for
#       `xx_XX.UTF-8`, which no fixture provides, must FAIL on every row. A
#       `setlocale` that succeeded for anything would mean L1 says nothing.
#   L3  ⭐ AND THE LOCALE IS ACTUALLY IN EFFECT, not merely accepted:
#       `nl_langinfo(CODESET)` must read `UTF-8`, and the decimal point must
#       become `,` — German, and different from C's `.`. ⛔ A `setlocale` that
#       returns non-NULL without changing behaviour is the same class of
#       non-discriminating criterion as `Gtk-WARNING: cannot open display`.
#
#   T1  ⭐ a GTK application out of a bundle, told `GTK_THEME=PgbFixture`,
#       OPENS the fixture's `gtk.css` under the HOST's `/usr/share/themes`.
#       ⛔ That is host integration measured rather than asserted: the bundle
#       carries its own GTK and its own themes, and it still reaches the host
#       tree because `XDG_DATA_DIRS` puts the host after itself.
#   T2  ⛔ THE CONTROL: the same bundle with `GTK_THEME` unset must NOT open
#       it. Without this row T1 could be GTK enumerating every theme it can
#       find, which is not "following" anything.
#   T3  and the subject still draws — a theme that stops the application
#       working is not integration. ⚠ Reported beside T1 rather than AND-ed
#       into it, so a drawing failure is visible as itself.
#
# ⛔ WHAT THIS DOES NOT ESTABLISH. It says the bundle READS the host's theme
# when told to; it does not say the window LOOKS different, which needs a
# screenshot comparison and is a different experiment. And the locale arm is
# `setlocale` and `nl_langinfo`, not a translated string: a `.mo` catalogue
# needs an application that ships one, which is T-087 rung 3's own row.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "106 - the two criteria that were parked as unmeasurable, measured against a fixture"

WORK="${PGB_EXP106_WORK:-/var/tmp/t106}"
mkdir -p "$WORK" || exit 2
PGB="$REPO_DIR/pgb"
[ -x "$PGB" ] || { exp_note "no ./pgb — run make"; exit 2; }
FIX="$REPO_DIR/scripts/common/bed-fixtures.sh"
THEME=PgbFixture
LOCALE=de_DE.UTF-8

ENVS=$(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt")

reap_in_root() {
  _rr=$1
  for _p in /proc/[0-9]*; do
    _pid=${_p#/proc/}
    _rt=$(readlink "/proc/$_pid/root" 2>/dev/null) || continue
    case "$_rt" in "$_rr"|"$_rr"/*) kill -9 "$_pid" 2>/dev/null ;; esac
  done
}
is_musl() {
  for _n in lib/ld-musl-x86_64.so.1 usr/lib/ld-musl-x86_64.so.1; do
    [ -L "$1/$_n" ] || [ -e "$1/$_n" ] && return 0
  done
  return 1
}

# ---------------------------------------------------------------------------
# The fixture. ⛔ INSTALLED BY THIS RUN, not assumed: an experiment that needs
# state on the bed and does not put it there measures whoever ran last.
# ---------------------------------------------------------------------------
printf -- '-- the fixture ----------------------------------------------------\n'
sh "$FIX" --install all > "$WORK/fixture.log" 2>&1 || true
sed 's/^/  /' "$WORK/fixture.log"
nglibc=0; nloc=0; nthm=0
for name in $ENVS; do
  root=$(exp_rootfs "$name") || true
  [ -n "$root" ] || continue
  [ -f "$root/usr/share/themes/$THEME/gtk-3.0/gtk.css" ] && nthm=$((nthm+1))
  if ! is_musl "$root"; then
    nglibc=$((nglibc+1))
    [ -f "$root/usr/lib/locale/$LOCALE/LC_CTYPE" ] && nloc=$((nloc+1))
  fi
done
exp_check "L0  the locale fixture is on every glibc rootfs" "$nloc" "$nglibc"
exp_check "L0  the theme fixture is on every rootfs" "$nthm" \
    "$(printf '%s\n' "$ENVS" | wc -l | tr -d ' ')"

# ---------------------------------------------------------------------------
# Arm L: a static probe. ⭐ `pgb build` so the glibc doing the reading is
# OURS — the same one an artefact of this project carries.
# ---------------------------------------------------------------------------
BUILDDIR=/var/tmp/pgb-exp106
rm -rf "$BUILDDIR"; mkdir -p "$BUILDDIR" || exit 2
cat > "$BUILDDIR/loc.c" <<'C'
#include <locale.h>
#include <langinfo.h>
#include <stdio.h>
#include <string.h>

/* argv[1] = the locale to ask for. Prints one line per fact so a shell can
 * read them without parsing prose. */
int main(int argc, char **argv)
{
    const char *want = argc > 1 ? argv[1] : "de_DE.UTF-8";
    const char *got  = setlocale(LC_ALL, want);
    struct lconv *lc;

    printf("SET=%s\n", got ? got : "(null)");
    if (!got) return 1;
    printf("CODESET=%s\n", nl_langinfo(CODESET));
    lc = localeconv();
    printf("DECIMAL=%s\n", lc && lc->decimal_point ? lc->decimal_point : "?");
    return 0;
}
C
printf -- '\n-- arm L: a static probe asking for the fixture --------------------\n'
if ! "$PGB" --engine chroot build --bind "$BUILDDIR" -- \
        sh -c "cd $BUILDDIR && cc -o locprobe loc.c" \
        >"$WORK/build-loc.log" 2>&1; then
  exp_note "could not build the locale probe; see $WORK/build-loc.log"
  tail -5 "$WORK/build-loc.log"; exit 2
fi
exp_check "L0  the probe is static (no PT_INTERP)" \
  "$(readelf -l "$BUILDDIR/locprobe" 2>/dev/null | grep -c INTERP || true)" 0

printf '  %-22s %-6s %-10s %-9s %s\n' ENVIRONMENT LIBC SET CODESET DECIMAL
l1=0; l2=0; l3=0; lrows=0
for name in $ENVS; do
  root=$(exp_rootfs "$name") || true
  [ -n "$root" ] || { exp_skip "$name" "rootfs not fetched"; continue; }
  if is_musl "$root"; then
    printf '  %-22s %-6s %s\n' "$name" musl 'n/a — musl has only C and C.UTF-8'
    continue
  fi
  lrows=$((lrows+1))
  rm -f "$root/locprobe"
  cp "$BUILDDIR/locprobe" "$root/locprobe" 2>"$WORK/cp.$name" || {
    exp_skip "$name" "could not stage the probe"; lrows=$((lrows-1)); continue; }
  chmod +x "$root/locprobe"
  timeout 60 "$PGB" rootfs run "$root" -- /bin/sh -c \
      "/locprobe $LOCALE; echo ---; /locprobe xx_XX.UTF-8" \
      >"$WORK/loc.$name" 2>&1 || true
  reap_in_root "$root"; rm -f "$root/locprobe"

  set=$(sed -n '1,/^---/p' "$WORK/loc.$name" | sed -n 's/^SET=//p' | head -1)
  cs=$(sed -n '1,/^---/p' "$WORK/loc.$name" | sed -n 's/^CODESET=//p' | head -1)
  dp=$(sed -n '1,/^---/p' "$WORK/loc.$name" | sed -n 's/^DECIMAL=//p' | head -1)
  bad=$(sed -n '/^---/,$p' "$WORK/loc.$name" | sed -n 's/^SET=//p' | head -1)
  [ -n "$set" ] && [ "$set" != "(null)" ] && l1=$((l1+1))
  [ "$bad" = "(null)" ] && l2=$((l2+1))
  [ "$cs" = "UTF-8" ] && [ "$dp" = "," ] && l3=$((l3+1))
  printf '  %-22s %-6s %-10s %-9s %s\n' "$name" glibc \
      "$([ -n "$set" ] && [ "$set" != '(null)' ] && echo ok || echo FAIL)" \
      "${cs:--}" "${dp:--}"
done

printf '\n'
exp_check "L1  ⭐ setlocale($LOCALE) succeeds on the glibc rows" "$l1" "$lrows"
exp_check "L2  ⛔ the control: setlocale(xx_XX.UTF-8) FAILS everywhere" "$l2" "$lrows"
exp_check "L3  ⭐ and the locale is in EFFECT — CODESET=UTF-8, decimal ','" "$l3" "$lrows"

# ---------------------------------------------------------------------------
# Arm T: a GTK bundle reading a HOST theme.
# ---------------------------------------------------------------------------
printf -- '\n-- arm T: a bundled GTK application reading the HOST theme ---------\n'
IMG="$WORK/gtk.AppImage"
if [ ! -s "$IMG" ]; then
  PGB_APPIMAGE_CACHE="$WORK/cache" "$PGB" bundle appimage galculator \
      --out "$IMG" --name galculator >"$WORK/build-gtk.log" 2>&1 || true
fi
if [ ! -s "$IMG" ]; then
  exp_skip "T1  a bundled GTK application reads the host theme" \
           "galculator did not build; see $WORK/build-gtk.log"
  exp_skip "T2  and does not read it when not told to" "no artefact"
else
  XDISP="${PGB_EXP106_DISPLAY:-:96}"
  DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1 || {
    Xvfb "$XDISP" -ac -screen 0 1024x768x24 >"$WORK/xvfb.log" 2>&1 &
    _w=0; while [ "$_w" -lt 20 ]; do
      DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1 && break; _w=$((_w+1)); sleep 1
    done; }
  printf '  %-22s %-10s %s\n' ENVIRONMENT 'GTK_THEME=' 'unset (control)'
  t1=0; t2=0; trows=0
  for name in $ENVS; do
    root=$(exp_rootfs "$name") || true
    [ -n "$root" ] || continue
    rm -f "$root/subj106"
    cp "$IMG" "$root/subj106" 2>"$WORK/cp.t.$name" || {
      exp_skip "$name" "could not stage the artefact"; continue; }
    chmod +x "$root/subj106"; trows=$((trows+1))
    for mode in themed plain; do
      case $mode in themed) gt="GTK_THEME=$THEME" ;; *) gt="GTK_THEME=" ;; esac
      timeout 120 strace -f -e trace=openat,open -o "$WORK/tr.$mode.$name" \
        "$PGB" rootfs run "$root" --bind /tmp/.X11-unix:/tmp/.X11-unix -- \
        /bin/sh -c "DISPLAY=$XDISP APPIMAGE_EXTRACT_AND_RUN=1 $gt /subj106" \
        >/dev/null 2>&1 &
      _sp=$!
      _n=0; while [ "$_n" -lt 60 ]; do
        sleep 1; _n=$((_n+1))
        grep -q "/usr/share/themes/$THEME" "$WORK/tr.$mode.$name" 2>/dev/null && break
        kill -0 "$_sp" 2>/dev/null || break
      done
      kill "$_sp" 2>/dev/null; wait "$_sp" 2>/dev/null
      reap_in_root "$root"
    done
    rm -f "$root/subj106"
    a=$(grep -c "/usr/share/themes/$THEME" "$WORK/tr.themed.$name" 2>/dev/null || true)
    b=$(grep -c "/usr/share/themes/$THEME" "$WORK/tr.plain.$name" 2>/dev/null || true)
    [ "${a:-0}" -gt 0 ] && t1=$((t1+1))
    [ "${b:-0}" = 0 ] && t2=$((t2+1))
    printf '  %-22s %-10s %s\n' "$name" "${a:-0} open(s)" "${b:-0} open(s)"
    rm -f "$WORK/tr.themed.$name" "$WORK/tr.plain.$name"
  done
  printf '\n'
  exp_check "T1  ⭐ the bundle READS the host theme when told to" "$t1" "$trows"
  exp_check "T2  ⛔ and does NOT when it is not told to" "$t2" "$trows"
fi

exp_note "⛔ WHAT THIS DOES NOT SAY. T1 is that the bundle READS the host's"
exp_note "   theme file, not that the window LOOKS different — that needs a"
exp_note "   screenshot comparison and is a different experiment."
exp_note "⚠ And arm L is setlocale and nl_langinfo, not a translated string:"
exp_note "   a .mo catalogue needs an application that ships one, which is"
exp_note "   T-087 rung 3's own row."
exp_note "⭐ THE FIXTURE IS THE POINT. Both criteria were recorded as 'the bed"
exp_note "   cannot answer it' and both were a script nobody had written."

exp_finish
