#!/bin/sh
# THE QUESTION — the operator's, 2026-09-04c, and it is a sharp one:
#
#   "Can our bundler pack something like `file` without patching it? `file` can
#    be statically compiled but without its magic file it is useless. The
#    AppImage approach has to provide a custom AppRun for each app, or at the
#    very least add ENV VARS to tell the `file` cli where the magic file is.
#    Can our bundler not require any of this and simply just work?"
#
# ⭐ `file` IS THE CLEANEST SUBJECT IN THE WHOLE CORPUS FOR THIS, because the
# data it needs is not optional and not cosmetic: with no magic database it
# does not degrade, it says `could not find any valid magic files!` and exits
# non-zero. There is no "it printed something" answer to mistake for a pass.
#
# ⛔ AND THE PATH IS COMPILED IN WITH NO SEARCH VARIABLE BY DEFAULT. nixpkgs
# builds `file` with its magic at
# `/nix/store/<hash>-file-<ver>/share/misc/magic.mgc`, and that absolute path
# is what the binary carries. `MAGIC` exists as an override — which is exactly
# the env var the question says the field has to add — so the discriminating
# question is whether OUR artefact needs it.
#
# -- ⛔ PRE-REGISTERED EXPECTATIONS -----------------------------------------
#
#   F1  ⭐ THE ARTEFACT IDENTIFIES A FILE, ON ALL ELEVEN. A four-byte PNG
#       signature written inside each rootfs, and `file` must answer
#       `PNG image data`. ⚠ The fixture is written by the harness in the
#       rootfs's own /tmp, so no row can read another row's answer.
#   F2  ⭐ AND IT NEEDS NO `MAGIC` VARIABLE TO DO IT. The bundle's `.env` is
#       read and asserted to contain no `MAGIC` and no `MAGIC_PATH`. ⛔ This
#       is the operator's actual question: a green F1 with `MAGIC` set in
#       `.env` would be the field's answer, not a better one.
#   F3  ⭐ AND NO CUSTOM AppRun. The AppRun in the artefact must be the stock
#       sharun one — the same file the bundler ships for every subject — not
#       a per-application shell script. Measured by comparing its bytes
#       against `AppDir/sharun`, which is what `installSharun` links it to.
#   F4  ⛔ THE NEGATIVE CONTROL, AND IT IS A SHIPPED FLAG. The same closure
#       built `--no-storefix` cannot resolve the compiled-in path, so `file`
#       must FAIL. A control that still worked would mean F1 says nothing
#       about the mechanism.
#
# ⚠ WHAT THIS DOES NOT ESTABLISH. One program and one data file. It says the
# mechanism reaches a compiled-in absolute store path with no search variable;
# it says nothing about a path in a config file (which the interposer would
# also reach, and which nothing here tests) or about a `/usr`-prefixed path
# (which it would NOT reach, by construction — `pgb-storefix.c`'s `fix()`).
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "105 - file(1) out of a bundle: a compiled-in data path, no AppRun, no env var"

WORK="${PGB_EXP105_WORK:-/var/tmp/t105}"
mkdir -p "$WORK" || exit 2
ATTR="${PGB_EXP105_ATTR:-file}"
PROG=file
RUN_TIMEOUT="${PGB_EXP105_TIMEOUT:-90}"

ENVS=$(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt")
NENV=$(printf '%s\n' "$ENVS" | wc -l | tr -d ' ')

reap_in_root() {
  _rr=$1
  for _p in /proc/[0-9]*; do
    _pid=${_p#/proc/}
    _rt=$(readlink "/proc/$_pid/root" 2>/dev/null) || continue
    case "$_rt" in "$_rr"|"$_rr"/*) kill -9 "$_pid" 2>/dev/null ;; esac
  done
}

build() {  # out-image env-copy extra-flag...
  _img=$1; _envout=$2; shift 2
  if [ ! -s "$_img" ]; then
    PGB_APPIMAGE_CACHE="$WORK/cache" "$REPO_DIR/pgb" bundle appimage "$ATTR" \
      --out "$_img" --name "$PROG" "$@" >"$_img.log" 2>&1 || true
  fi
  cp "$WORK/cache/$PROG/AppDir/.env" "$_envout" 2>/dev/null || : > "$_envout"
  cp "$WORK/cache/$PROG/AppDir/AppRun" "$_envout.apprun" 2>/dev/null || :
  cp "$WORK/cache/$PROG/AppDir/sharun" "$_envout.sharun" 2>/dev/null || :
  [ -s "$_img" ]
}

printf -- '-- building the subject and its control ---------------------------\n'
IMG="$WORK/file.AppImage"
CTL="$WORK/file-nofix.AppImage"
build "$IMG" "$WORK/env.subject" \
  || { exp_note "subject did not build; see $IMG.log"; tail -5 "$IMG.log"; exit 2; }
exp_check "F0  the subject built" "$([ -s "$IMG" ] && echo yes || echo no)" yes

# ⭐ F2 AND F3 ARE READ OFF THE SUBJECT'S OWN AppDir, before anything runs.
magicvars=$(grep -cE '^MAGIC(_PATH)?=' "$WORK/env.subject" 2>/dev/null || true)
exp_check "F2  ⭐ the bundle sets NO MAGIC variable" "${magicvars:-0}" 0
exp_note "$(printf '   .env carries %s variable(s): %s' \
    "$(grep -c . "$WORK/env.subject" 2>/dev/null || echo 0)" \
    "$(cut -d= -f1 "$WORK/env.subject" 2>/dev/null | tr '\n' ' ')")"

# ⛔ THE AppRun MUST BE THE STOCK ONE. A per-application shell AppRun is
# precisely what the question is about, and `buildStaticAppRun` warns and falls
# back to one when it cannot build the selector (corrections.md C31), so this
# is a check with a real failure mode behind it.
same=no
if [ -s "$WORK/env.subject.apprun" ] && [ -s "$WORK/env.subject.sharun" ]; then
  [ "$(md5sum < "$WORK/env.subject.apprun")" = "$(md5sum < "$WORK/env.subject.sharun")" ] \
    && same=yes
fi
exp_check "F3  ⭐ AppRun is the stock sharun, not a per-app script" "$same" yes
exp_note "$(printf '   AppRun %s bytes, sharun %s bytes' \
    "$(wc -c < "$WORK/env.subject.apprun" 2>/dev/null || echo 0)" \
    "$(wc -c < "$WORK/env.subject.sharun" 2>/dev/null || echo 0)")"

build "$CTL" "$WORK/env.control" --no-storefix \
  || exp_note "control did not build; see $CTL.log"
exp_check "F0  ⭐ the control built (--no-storefix)" \
    "$([ -s "$CTL" ] && echo yes || echo no)" yes

printf -- '\n-- the eleven -----------------------------------------------------\n'
printf '  %-22s %-10s %s\n' ENVIRONMENT SUBJECT CONTROL
f1=0; f4=0; rows=0
for name in $ENVS; do
  root=$(exp_rootfs "$name") || true
  [ -n "$root" ] || { exp_skip "$name" "rootfs not fetched"; continue; }

  # ⛔ STAGE FIRST, AND A FAILED COPY IS A SKIP. corrections.md C44.
  rm -f "$root/subj105" "$root/ctl105"
  if ! cp "$IMG" "$root/subj105" 2>"$WORK/cp.$name"; then
    exp_skip "$name" "could not stage: $(tr -d '\n' < "$WORK/cp.$name" | cut -c1-80)"
    rm -f "$root/subj105" "$WORK/cp.$name"; continue
  fi
  chmod +x "$root/subj105"
  [ -s "$CTL" ] && { cp "$CTL" "$root/ctl105" 2>/dev/null && chmod +x "$root/ctl105"; }
  rows=$((rows+1))

  # ⭐ THE FIXTURE IS WRITTEN INSIDE THE ROOTFS, in its own tmpfs /tmp, so no
  # row can read another row's file. `printf` with the PNG signature is the
  # smallest thing `file` can identify unambiguously.
  timeout "$RUN_TIMEOUT" "$REPO_DIR/pgb" rootfs run "$root" -- /bin/sh -c '
    printf "\211PNG\r\n\032\n\000\000\000\015IHDR" > /tmp/f.png
    echo "SUBJ=$(APPIMAGE_EXTRACT_AND_RUN=1 /subj105 /tmp/f.png 2>&1 | tr -d "\n")"
    if [ -x /ctl105 ]; then
      echo "CTL=$(APPIMAGE_EXTRACT_AND_RUN=1 /ctl105 /tmp/f.png 2>&1 | tr -d "\n")"
    fi
  ' >"$WORK/out.$name" 2>"$WORK/err.$name" || true
  reap_in_root "$root"
  rm -f "$root/subj105" "$root/ctl105"

  s=$(sed -n 's/^SUBJ=//p' "$WORK/out.$name" | tail -1)
  c=$(sed -n 's/^CTL=//p'  "$WORK/out.$name" | tail -1)
  sok=no; cok=no
  printf '%s' "$s" | grep -q 'PNG image data' && { sok=yes; f1=$((f1+1)); }
  printf '%s' "$c" | grep -q 'PNG image data' && cok=yes
  [ "$cok" = no ] && f4=$((f4+1))
  printf '  %-22s %-10s %s\n' "$name" "$sok" "$cok"
  [ "$sok" = no ] && exp_note "$(printf '   %s subject said: %s' "$name" "$(printf '%s' "$s" | cut -c1-120)")"
  [ "$cok" = no ] && [ -n "$c" ] && exp_note "$(printf '   %s control said: %s' "$name" "$(printf '%s' "$c" | cut -c1-120)")"
done

printf '\n'
exp_check "F1  ⭐ the bundle identifies a PNG on all $rows" "$f1" "$rows"
exp_check "F4  ⛔ the --no-storefix control FAILS on all $rows" "$f4" "$rows"

if [ "$f1" = "$rows" ] && [ "$f4" = "$rows" ] && [ "$rows" -gt 0 ]; then
  exp_note "⭐ SO THE ANSWER TO THE QUESTION IS YES, AND F2/F3 ARE WHY IT MEANS"
  exp_note "   SOMETHING: the artefact carries no MAGIC variable and no"
  exp_note "   per-application AppRun, and the same bundle with the mechanism"
  exp_note "   switched off cannot read its own magic database."
fi
exp_note "⚠ ONE program and ONE data file. It says the mechanism reaches a"
exp_note "   compiled-in absolute STORE path with no search variable. It says"
exp_note "   nothing about a path in a config file, and a /usr-prefixed path"
exp_note "   it would NOT reach — pgb-storefix.c's fix() by construction."

exp_finish
