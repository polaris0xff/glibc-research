#!/bin/sh
# 90 - our kdenlive bundle against pkgforge-dev/kdenlive-AppImage-Enhanced.
#
# ⛔ THE BAR IS A COMPARISON, NOT A BUILD. T-055 quotes the operator:
# *"if impossible, pivot to kdenlive.nixappimage, but it must be smaller, load
# faster, run faster than pkgforge-dev/kdenlive-AppImage-Enhanced."* An
# AppImage that works is not this entry; three measured columns against a
# named competitor is.
#
#   size   bytes, both artefacts, same day
#   load   first-run and warm-run startup. ⚠ experiments/40-'s noise floor
#          applies: a difference at or under it is "no difference measurable"
#   run    ⭐ A REAL RENDER. `melt` -- kdenlive's engine, and the workload
#          poc/80-mlt already uses -- encodes a generated clip to an MP4 and
#          the wall clock is taken. `--version` would pass on a bundle that
#          cannot decode anything.
#
# ⭐ AND THE TWO SIDES SHIP THE SAME UPSTREAM RELEASE, which is luck rather
# than design and is worth stating either way: nixpkgs has kdenlive 26.08.0
# and the competitor's tag is 26.08.0-1.
#
# ⚠ WHAT THIS CANNOT SETTLE. Neither artefact is exercised as a GUI: none of
# the eleven environments has a display, and the build host's Xvfb is a
# software X server with no compositor. What the GUI column would need is
# T-059's hardware. So "load" is process start, and "run" is the render
# engine, and the window is only proved to OPEN (`poc/91-qt-xcb` does that on
# a static Qt).
#
# Exit: 0 matched, 1 did not, 2 could not run.
# SPDX-License-Identifier: MIT

set -u
. "$(dirname "$0")/lib.sh"
# ⛔ THE CLOCK IS A LIBRARY NOW, AND THIS EXPERIMENT IS WHY.
# `docs/history/corrections.md` C23 recorded that four runs of this comparison
# gave cold-start ratios of 2.52x, 3.48x, 4.92x and 5.02x with warm ABOVE cold
# in two of them; C24 found the mechanism -- the cold column below obtained
# "cold" by copying the file, and uruntime keys its mount on CONTENT, so the
# copy reused the live mount and the column reported a warm start.
. "$(dirname "$0")/clock.sh"

exp_begin "90 - our kdenlive bundle against kdenlive-AppImage-Enhanced AND onelf"

RR="$REPO_DIR/pgb"
BUNDLER="$REPO_DIR/pgb"
WORK="${PGB_KDEN_WORK:-/var/tmp/t055}"
CACHE="${PGB_KDEN_CACHE:-/var/tmp/pgb-appimage-kden}"
RUN_TIMEOUT="${PGB_KDEN_TIMEOUT:-300}"
WARM_RUNS="${PGB_KDEN_WARM:-5}"
ROUNDS="${PGB_KDEN_ROUNDS:-7}"
RENDER_ROUNDS="${PGB_KDEN_RENDER_ROUNDS:-3}"
mkdir -p "$WORK" || exit 2

# ⛔ THE INSTRUMENT MUST OWN $TMPDIR, because that is where uruntime puts its
# mountpoint and reaping it is the whole of the corrected cold protocol. A
# reap under a shared /tmp could take a mount belonging to something else.
MTMP="$WORK/tmp"; mkdir -p "$MTMP" || exit 2
TMPDIR="$MTMP"; export TMPDIR

# ⛔ NOT `pkill -f` and NOT a process name: docs/AGENTS.md §14 -- the
# artefact's path is in this runner's own command line. The handle that is
# neither is the MOUNTPOINT PATH.
reap_mounts() {
  for _d in "$MTMP"/.mount_*; do
    [ -e "$_d" ] || continue
    case "$_d" in *.pid) continue ;; esac
    fusermount3 -u "$_d" 2>/dev/null || fusermount -u "$_d" 2>/dev/null \
      || umount -l "$_d" 2>/dev/null || :
  done
  rm -rf "$MTMP"/.mount_* 2>/dev/null || :
}

# ⛔ PINNED. The competitor publishes under a moving `latest` and a dated tag;
# two runs a week apart would otherwise compare different artefacts with
# nothing saying so.
ENH_TAG="26.08.0-1@2026-09-01_1788277742"
ENH_URL="https://github.com/pkgforge-dev/kdenlive-AppImage-Enhanced/releases/download/$ENH_TAG/Kdenlive-26.08.0-1-anylinux-x86_64.AppImage"
ENH="$WORK/kdenlive-enhanced.AppImage"
OURS="$CACHE/kdenlive/kdenlive-anylinux-x86_64.AppImage"

B="$EXP_OUT/build"; mkdir -p "$B" || exit 2
: > "$EXP_OUT/per-environment.txt"

reap_rootfs() {
  for _d in /proc/[0-9]*; do
    _p=${_d#/proc/}
    _r=$(readlink "$_d/root" 2>/dev/null) || continue
    case "$_r" in "$1"|"$1"/*) kill -9 "$_p" 2>/dev/null ;; esac
  done
  return 0
}
reap_all() {
  while read -r _ref _name _libc _digest; do
    case "$_ref" in ''|\#*) continue ;; esac
    _r=$(exp_rootfs "$_name"); [ -n "$_r" ] && reap_rootfs "$_r"
  done < "$REPO_DIR/scripts/common/rootfs-images.txt"
  return 0
}
trap 'reap_all' EXIT INT TERM

# ---------------------------------------------------------------------------
# the two artefacts
# ---------------------------------------------------------------------------
printf -- '-- the two artefacts ---------------------------------------------\n'
if [ ! -s "$ENH" ]; then
  exp_note "fetching the competitor, pinned at $ENH_TAG"
  curl -fsSL -o "$ENH" "$ENH_URL" \
    || curl -fsSL -o "$ENH" "https://api.rv.pkgforge.dev/$ENH_URL" \
    || { exp_note "could not fetch $ENH_URL"; exit 2; }
  chmod +x "$ENH"
fi
# ⛔ REBUILD WHEN THE BUNDLER IS NEWER THAN THE ARTEFACT, not only when the
# artefact is missing.
#
# ⚠ MEASURED, BY BEING BURNED BY IT. The previous rule was `[ ! -s "$OURS" ]`.
# A bundler fix was committed, this experiment was re-run to verify it, and the
# run reported the SAME byte count to the digit -- 267,390,365 -- because the
# cached artefact from the broken run was reused and the fix was never
# exercised. A re-run that cannot see a code change is not a re-run; it is the
# previous run with a new date on it.
#
# ⭐ Same staleness rule pgb uses for its own runtime objects: newer input
# means rebuild. `PGB_KEEP_ARTEFACT=1` forces reuse for someone deliberately
# re-measuring the same bytes.
#
# ⛔ AND ON THE BUILD OPTIONS, NOT ONLY ON THE MTIME -- WHICH IS THE SAME
# DEFECT AGAIN, IN A NEW COSTUME. The mtime rule above catches a changed
# BUNDLER. It does not catch a changed INVOCATION, and nothing in the cache
# path mentions one: `$CACHE/kdenlive/kdenlive-anylinux-x86_64.AppImage` is the
# same file whatever `--debloat` was. So running this experiment again with
# `PGB_APPIMAGE_DEBLOAT=aggressive` against an artefact built at `safe` would
# have reused the `safe` bytes and reported them as the aggressive row --
# silently, and to the digit, exactly as run 2 did.
#
# ⚠ It matters here specifically because sweep deletion is gated on
# `aggressive` (`internal/bundle/appimage.go`), so `safe` and `aggressive` are
# the two arms this experiment most needs to tell apart.
#
# ⭐ The fix is T-058's, one level up: key the cache on the options themselves.
# A stamp file beside the artefact records what produced it, and a different
# stamp is a rebuild.
_opts="debloat=${PGB_APPIMAGE_DEBLOAT:-safe}"
_stamp="$CACHE/kdenlive/.pgb-build-options"
_rebuild=no
_why=
if [ ! -s "$OURS" ]; then
  _rebuild=yes
elif [ -n "${PGB_KEEP_ARTEFACT:-}" ]; then
  :
elif [ "$(cat "$_stamp" 2>/dev/null)" != "$_opts" ]; then
  _rebuild=yes
  _why="the cached artefact was built with [$(cat "$_stamp" 2>/dev/null)], this run wants [$_opts]"
elif [ "$BUNDLER" -nt "$OURS" ]; then
  _rebuild=yes
  _why="the bundler is newer than the cached artefact"
fi
if [ "$_rebuild" = yes ]; then
  [ -n "$_why" ] && { exp_note "$_why: rebuilding"; rm -f "$OURS"; }
  exp_note "building ours ($_opts): ./pgb bundle appimage kdenlive --with-program melt"
  mkdir -p "$CACHE/kdenlive"
  PGB_APPIMAGE_CACHE="$CACHE" "$BUNDLER" bundle appimage kdenlive \
    --with-program melt --with-program ffmpeg >"$B/build-ours.log" 2>&1 || true
  # ⛔ Stamped only AFTER the artefact exists, so a build that died half way
  # does not leave a stamp claiming those options were measured.
  [ -s "$OURS" ] && printf '%s\n' "$_opts" > "$_stamp"
fi
exp_note "artefact built with: $(cat "$_stamp" 2>/dev/null || echo unknown)"
[ -s "$OURS" ] || { exp_note "ours did not build; see $B/build-ours.log"; exit 2; }
chmod +x "$OURS"

P_SZ=$(wc -c < "$OURS"); E_SZ=$(wc -c < "$ENH")
exp_check "ours exists"           "$([ -s "$OURS" ] && echo yes)" yes
exp_check "the competitor exists" "$([ -s "$ENH" ] && echo yes)" yes
printf '\n  %-34s %14s\n' ARTEFACT BYTES
printf '  %-34s %14s\n' "P  ours (one command, nixpkgs)" "$P_SZ"
printf '  %-34s %14s\n' "E  kdenlive-AppImage-Enhanced"  "$E_SZ"
printf '  %-34s %14s\n' "   ratio P/E" \
  "$(awk -v p="$P_SZ" -v e="$E_SZ" 'BEGIN{printf "%.2fx", p/e}')"
# ⛔ THE SIZE COLUMN IS THE ONE THE OPERATOR NAMED FIRST, and it is reported
# whichever way it comes out. `docs/AGENTS.md` §14 forbids "strictly better"
# without a measurement; it equally forbids hiding one.
exp_note "operator's bar: smaller, loads faster, runs faster than E"

# ---------------------------------------------------------------------------
# the render: melt, both artefacts, on the build host
# ---------------------------------------------------------------------------
printf -- '\n-- the render (melt, the engine kdenlive drives) ------------------\n'
# ⭐ A generated clip encoded to a real MP4. No input file, so the two arms
# cannot differ by what they read; the work is entirely inside their own
# ffmpeg and MLT.
MELT_ARGS="color:blue out=48 -consumer avformat:%OUT% vcodec=libx264 preset=ultrafast an=1"
# ⛔ THE PROGRAM NAME GOES IN argv[0] FOR ONE RUNTIME AND argv[1] FOR THE
# OTHER, AND GETTING IT WRONG DOES NOT LOOK LIKE A DISPATCH BUG.
# `onelf` resolves its entrypoint from argv[0]'s basename and falls back to the
# package's DEFAULT entrypoint when no name matches -- so invoking the bundle
# through a symlink called `melt-onelf` ran **kdenlive**, which needs a display,
# and died in `QMessageLogger::fatal` inside `QApplicationPrivate::init`.
# ⚠ What that printed was `Aborted` and nothing else, three runs in a row, and
# the arm was recorded as "onelf cannot run our payload". It ran it fine: the
# same bundle answers `melt -version` in 0.4 s through a symlink named `melt`.
# The backtrace is in the entry; the lesson is that a fallback that is silent
# turns a naming mistake into a capability claim about somebody else's tool.
# ⭐ So the selector is derived from the BASENAME the artefact is invoked under,
# which is the one thing both runtimes agree on, and arm O is always invoked
# through a directory whose file is named exactly after the entrypoint.
sel_of() {  # invocation path -> the argv[1] selector, empty for argv[0] dispatch
  case "$(basename "$1")" in melt) printf '' ;; *) printf 'melt' ;; esac
}
render() {  # artefact tag -> ms, or -1
  _a="$1"; _t="$2"
  rm -f "$WORK/$_t.mp4"
  _s=$(date +%s%N)
  _sel=$(sel_of "$_a")
  # shellcheck disable=SC2086
  timeout -k 10 "$RUN_TIMEOUT" "$_a" $_sel $(printf '%s' "$MELT_ARGS" | sed "s|%OUT%|$WORK/$_t.mp4|") \
    >"$B/render.$_t.log" 2>&1 || { printf '%s' -1; return; }
  _e=$(date +%s%N)
  [ -s "$WORK/$_t.mp4" ] || { printf '%s' -1; return; }
  printf '%s' $(( (_e - _s) / 1000000 ))
}
P_R=$(render "$OURS" ours)
E_R=$(render "$ENH" enh)
P_MP4=$( [ -f "$WORK/ours.mp4" ] && wc -c < "$WORK/ours.mp4" || echo 0)
E_MP4=$( [ -f "$WORK/enh.mp4" ] && wc -c < "$WORK/enh.mp4" || echo 0)
printf '  %-34s %10s ms   %10s bytes of MP4\n' "P  ours" "$P_R" "$P_MP4"
printf '  %-34s %10s ms   %10s bytes of MP4\n' "E  enhanced" "$E_R" "$E_MP4"
exp_check "ours rendered a real MP4"           "$([ "${P_MP4:-0}" -gt 1000 ] && echo yes || echo no)" yes
exp_check "the competitor rendered a real MP4" "$([ "${E_MP4:-0}" -gt 1000 ] && echo yes || echo no)" yes

# ---------------------------------------------------------------------------
# startup, on the build host, with the mount left alive
# ---------------------------------------------------------------------------
printf -- '\n-- startup (the instrument experiments/86- ended up with) ---------\n'
# ⚠ `melt -version` rather than `kdenlive --version`: kdenlive needs a display
# and the competitor's own AppRun refuses `--version` without one, so timing it
# would time an error path. melt starts the same runtime.
start_ms() {  # artefact n -> total ms for n invocations
  _a="$1"; _n="$2"; _i=0
  _sm=$(sel_of "$_a")
  _s=$(date +%s%N)
  while [ "$_i" -lt "$_n" ]; do
    # shellcheck disable=SC2086
    "$_a" $_sm -version >/dev/null 2>&1 || { printf '%s' -1; return; }
    _i=$((_i + 1))
  done
  _e=$(date +%s%N)
  printf '%s' $(( (_e - _s) / 1000000 ))
}
# ---------------------------------------------------------------------------
# ⛔ THE COLD PROTOCOL, REPLACED. `docs/history/corrections.md` C24.
#
# This obtained "cold" by giving the run its own COPY of the artefact, on the
# written reasoning that *"uruntime keys its mount on the image, so a file
# nothing has run before is cold by construction"*. ⛔ THE MOUNT IS KEYED ON
# CONTENT, NOT ON PATH, so a byte-identical copy reuses the live mount and the
# cold column reported a warm start -- measured by `experiments/99-` at 1.02x
# of warm, against 6.80x for the corrected protocol. That is the mechanism
# behind C23's 20x spread and its warm-above-cold rows: the two columns were
# measuring the SAME state, and noise has a sign.
#
# ⭐ COLD IS NOW OBTAINED BY REAPING THE LIVE MOUNT, and the mount lives under
# a $TMPDIR this experiment owns. ⚠ The empty `XDG_CACHE_HOME` is KEPT and
# still matters: onelf keys its extraction on a content hash rather than on a
# mount, so clearing its cache is what makes arm O cold. Two runtimes, two
# mechanisms, one prep that covers both.
#
# ⚠ And the copy trick is gone with the reasoning that motivated it, so its
# two traps go too -- the basename onelf dispatches on, and the `cp -L` for a
# symlinked arm. Nothing is copied any more.
# ---------------------------------------------------------------------------
CLK_CACHE="$WORK/cold-cache"
clk_prep() {  # arm -> make the next run genuinely cold
  reap_mounts
  rm -rf "$CLK_CACHE"; mkdir -p "$CLK_CACHE"
}
clk_run() {   # arm -> one timed cold start
  case "$1" in
    P) _ca="$OURS" ;;
    E) _ca="$ENH" ;;
    O) _ca="$O_DIR/melt" ;;
    twin) _ca="$TWIN" ;;
    *) printf -- '-1'; return ;;
  esac
  # shellcheck disable=SC2086
  XDG_CACHE_HOME="$CLK_CACHE" clk_time_once \
    timeout -k 10 "$RUN_TIMEOUT" "$_ca" $(sel_of "$_ca") -version
}

# ⛔ THE WARM FIGURE IS MEASURED, NOT SUBTRACTED. `86-`'s arithmetic --
# (n+1 runs − cold) / n -- assumes the cold run is part of the same series,
# and produced NEGATIVE warm times here (-489 ms and -298 ms, numbers that
# cannot be durations and were printed anyway). Warm is one run to establish
# the mount, then N timed runs inside the window it stays alive.
# ⚠ `experiments/99-` measured that window at 4-6 s on this machine, so a warm
# series must run back to back; a `sleep` between samples would measure cold.
warm_of() {  # artefact -> ms per run once the mount is warm
  # shellcheck disable=SC2086
  "$1" $(sel_of "$1") -version >/dev/null 2>&1 || { printf '%s' -1; return; }
  _t=$(start_ms "$1" "$WARM_RUNS")
  [ "$_t" = -1 ] && { printf '%s' -1; return; }
  printf '%s' $(( _t / WARM_RUNS ))
}

# ⭐ THE A/A TWIN, a byte copy of arm P, riding the same interleave as every
# arm it licenses. Its true ratio to P is 1.00, so what the instrument reports
# for the pair IS its floor here, today. `experiments/clock.sh`.
TWIN="$WORK/twin/$(basename "$OURS")"
mkdir -p "$WORK/twin"; cp -L "$OURS" "$TWIN"; chmod +x "$TWIN"
clk_run_twin() { XDG_CACHE_HOME="$CLK_CACHE" clk_time_once \
  timeout -k 10 "$RUN_TIMEOUT" "$TWIN" "$(sel_of "$TWIN")" -version; }

# measure_cold ARM... -> fills P_COLD / E_COLD / O_COLD from ONE interleave.
# ⛔ Called again when arm O exists, so O is never compared against a P
# measured in a different pass -- the separation `experiments/99-` had to fix.
measure_cold() {
  clk_init "$B/clk" || return 1
  # shellcheck disable=SC2086
  clk_interleave "$ROUNDS" "$@" twin
  reap_mounts
  clk_table
  CLK_AA=$(clk_aa P twin)
  CLK_FLOOR=$(clk_floor P twin "$CLK_AA")
  printf '\n  A/A control (P vs its own copy): ratio %s, floor %s, resolves %s\n' \
    "$CLK_AA" "$CLK_FLOOR" "$(clk_resolves "$CLK_AA" "$CLK_FLOOR")"
  P_COLD=$(clk_ms "$(clk_stat P median)")
  E_COLD=$(clk_ms "$(clk_stat E median)")
  for _a in "$@"; do
    [ "$_a" = O ] && O_COLD=$(clk_ms "$(clk_stat O median)")
  done
  return 0
}

# ---------------------------------------------------------------------------
# arm O -- ⭐ onelf, the third packer
# ---------------------------------------------------------------------------
# ⛔ THE OPERATOR NAMED IT, 2026-09-02: *"QaidVoid/onelf is an 'alternative' to
# AnyLinux AppImages, the 'competitor' that is currently winning all the tests
# and comparisons ... i want to see if anylinux still wins everyone."*
#
# ⭐ SAME PAYLOAD, DIFFERENT PACKER, WHICH IS THE ONLY WAY THIS ARM IS FAIR.
# Arm E ships Arch's kdenlive; arm P ships nixpkgs'. Giving onelf a THIRD build
# would measure three distributions rather than three packers. So arm O is
# built from **our own AppDir** -- the same nixpkgs binaries, the same 5,000
# libraries -- packed by onelf instead of by uruntime+dwarfs+sharun. The
# difference between P and O is therefore the packing and the runtime, and
# nothing else.
#
# ⚠ onelf is vendored at `references/QaidVoid__onelf` commit
# 74b4c9a40aa2bab1e78b7f2898583678780b6d85 and built from that source; its
# runtime stub needs a musl gcc and the x86_64-unknown-linux-musl rust target,
# and the arm is SKIPPED rather than faked when either is missing.
printf -- '\n-- arm O: the same payload, packed by onelf ----------------------\n'
ONELF_SRC="$REPO_DIR/references/QaidVoid__onelf/tree"
ONELF_BIN="$ONELF_SRC/target/release/onelf"
ONELF="$WORK/kdenlive-onelf.bin"
OURDIR="$CACHE/kdenlive/AppDir"
if [ ! -x "$ONELF_BIN" ]; then
  # ⛔ ITS PREREQUISITE IS A RUST TARGET, NOT A COMPILER, AND THE ERROR DOES
  # NOT SAY SO WHERE ANYONE READS IT. onelf builds a musl runtime, so it needs
  # the `x86_64-unknown-linux-musl` std, and a fresh container has only the gnu
  # one. What that produced was `error[E0463]: can't find crate for core`
  # eighteen lines into a cargo log, and what the table said was "onelf did not
  # build" -- which reads like a defect in onelf. ⭐ So the prerequisite is
  # added when it can be, and named exactly when it cannot.
  if command -v rustup >/dev/null 2>&1 &&
     ! rustup target list --installed 2>/dev/null | grep -q x86_64-unknown-linux-musl; then
    exp_note "adding the rust target onelf needs: x86_64-unknown-linux-musl"
    rustup target add x86_64-unknown-linux-musl >>"$B/onelf-build.log" 2>&1 || true
  fi
  command -v cargo >/dev/null 2>&1 && \
    ( cd "$ONELF_SRC" && ONELF_MUSL_CC=musl-gcc cargo build --release ) \
      >>"$B/onelf-build.log" 2>&1 || true
fi
O_SZ=0; O_R=-1; O_COLD=-1; O_WARM=-1; O_MP4=0
O_DIR="$WORK/onelf-argv0"
if [ ! -x "$ONELF_BIN" ]; then
  # ⚠ The reason, not just the absence: three different things stop this build
  # and only one of them is about onelf.
  _o_why="see $B/onelf-build.log"
  command -v cargo >/dev/null 2>&1 || _o_why="cargo is absent"
  command -v rustup >/dev/null 2>&1 &&
    ! rustup target list --installed 2>/dev/null | grep -q x86_64-unknown-linux-musl &&
    _o_why="the rust target x86_64-unknown-linux-musl is absent"
  command -v musl-gcc >/dev/null 2>&1 || _o_why="$_o_why (musl-gcc is also absent)"
  exp_skip "arm O (onelf)" "onelf did not build: $_o_why"
elif [ ! -d "$OURDIR" ]; then
  exp_skip "arm O (onelf)" "our AppDir is not on disk to repack"
else
  if [ ! -s "$ONELF" ]; then
    D="$WORK/onelfdir"; rm -rf "$D"; mkdir -p "$D/bin" "$D/lib"
    # ⭐ The payload ELFs and the flattened library tree, exactly as our own
    # bundle carries them. ⚠ `cp -al` where possible: two copies of 1.2 GB of
    # libraries is disk this machine does not have.
    # ⛔ `bin/.` AND NOT `bin/*`. A nixpkgs wrapper leaves the real ELF beside
    # it as `.NAME-wrapped`, a shell glob never matches a leading dot, and the
    # recipe -- written from a readdir, which does see them -- then names an
    # entrypoint the packed directory does not contain.
    cp -al "$OURDIR"/shared/bin/. "$D/bin/" 2>/dev/null || cp -a "$OURDIR"/shared/bin/. "$D/bin/"
    cp -al "$OURDIR"/lib/. "$D/lib/" 2>/dev/null || cp -a "$OURDIR"/lib/. "$D/lib/"
    [ -d "$OURDIR/share" ] && { cp -al "$OURDIR/share" "$D/share" 2>/dev/null || cp -a "$OURDIR/share" "$D/share"; }
    [ -d "$OURDIR/store" ] && { cp -al "$OURDIR/store" "$D/store" 2>/dev/null || cp -a "$OURDIR/store" "$D/store"; }
    # ⭐ ONELF GETS THE SAME INFORMATION sharun GETS, and at the same
    # compression. `pgb bundle onelf-recipe` turns our `.env` into `[env]` --
    # ${SHARUN_DIR} becomes ${ONELF_DIR}, a live ${VAR} becomes $${VAR}, and
    # repeated keys are folded because TOML cannot repeat one -- and sets
    # `[compression] level = 19` to match the dwarfs zstd level our own packer
    # uses. onelf's default is 12; comparing those would measure a default.
    # ⛔ Without the environment this arm fails the way OURS did before T-053:
    # melt starts, answers -version, and cannot find its modules.
    "$REPO_DIR/pgb" bundle onelf-recipe "$OURDIR" kdenlive --level 19 \
      > "$D/onelf.toml" 2>>"$B/onelf-pack.log" || true
    cp "$D/onelf.toml" "$B/onelf.toml"
    exp_note "packing $(ls "$D/lib" | wc -l) libraries and $(ls "$D/bin" | wc -l) programs with onelf"
    ( cd "$D" && "$ONELF_BIN" build -o "$ONELF" ) >>"$B/onelf-pack.log" 2>&1 \
      || exp_note "onelf build failed; see $B/onelf-pack.log"
  fi
  if [ -s "$ONELF" ]; then
    chmod +x "$ONELF"
    O_SZ=$(wc -c < "$ONELF")
    # ⛔ onelf DISPATCHES ON argv[0], NOT ON argv[1]. `pack --entrypoint
    # name=path` is "selected via argv[0]" in its own schema, so
    # `./pkg melt -version` runs KDENLIVE with `melt` as an argument -- which
    # is why this arm's first runs reported `Aborted` and no MP4. A symlink
    # named after the entrypoint is how onelf is meant to be invoked, and it
    # is also how sharun works, so the two runtimes agree on the mechanism.
    # ⚠ AND THE SYMLINK'S NAME IS THE ENTRYPOINT AND NOTHING ELSE. `melt-onelf`
    # is not `melt`; it matched no entrypoint and silently ran the default.
    # It therefore lives in its own directory: see sel_of above.
    O_DIR="$WORK/onelf-argv0"; mkdir -p "$O_DIR"; ln -sf "$ONELF" "$O_DIR/melt"
    O_R=$(render "$O_DIR/melt" onelf); O_MP4=$( [ -f "$WORK/onelf.mp4" ] && wc -c < "$WORK/onelf.mp4" || echo 0)
    # ⭐ RE-MEASURED WITH ALL THREE ARMS IN ONE INTERLEAVE. Arm O exists only
    # here, after P and E have already been timed, so timing it on its own
    # would compare it against a P measured in a different few seconds of this
    # machine -- the separation `experiments/99-` had to fix. `measure_cold`
    # re-runs the whole pass and overwrites P_COLD and E_COLD too, so all
    # three rows come from one interleave under one A/A control.
    measure_cold P E O || :
    O_WARM=$(warm_of "$O_DIR/melt"); reap_mounts
    exp_check "onelf packed the same payload" "$([ "$O_SZ" -gt 1000 ] && echo yes || echo no)" yes
    exp_check "and it rendered a real MP4"    "$([ "${O_MP4:-0}" -gt 1000 ] && echo yes || echo no)" yes
    printf '  %-34s %14s   %8s ms render   %6s ms cold  %5s ms warm\n' \
      "O  onelf (our payload)" "$O_SZ" "$O_R" "$O_COLD" "$O_WARM"
  else
    exp_skip "arm O (onelf)" "no artefact was produced"
  fi
fi

# ---------------------------------------------------------------------------
# the eleven
# ---------------------------------------------------------------------------
printf -- '\n-- melt on the eleven, both artefacts -----------------------------\n'
classify_trace() {  # tracefile /artefact payload|tree
  awk -v want="$2" -v mode="$3" '
    { pid = $1 }
    $0 ~ ("execve\\(\"" want "\"") { inset[pid] = 1; payload = pid; delete cur; next }
    ($0 ~ /(clone|clone3|vfork|fork)\(/ || $0 ~ /<\.\.\. (clone|clone3|vfork|fork) resumed>/) \
      && /= [0-9]+$/ { if (inset[pid]) inset[$NF] = 1; next }
    inset[pid] && /execve\(/ && !/ENOENT|= -1/ { payload = pid; if (mode != "tree") delete cur; next }
    inset[pid] && /open(at)?\(/ && !/ENOENT|= -1/ {
      if (mode != "tree" && pid != payload) next
      if (match($0, /"[^"]*"/) == 0) next
      p = substr($0, RSTART + 1, RLENGTH - 2)
      if (p !~ /\.so(\.[0-9]+)*$/) next
      if (p ~ /^\/(usr\/)?(local\/)?lib(32|64)?\//) cur["host " p] = 1
      else cur["bundled " p] = 1
    }
    END { for (k in cur) print k }
  ' "$1" | sort -u
}
count() { n=$(grep -c . 2>/dev/null) || n=0; printf '%s' "$n"; }

write_test() {  # rootfs
  cat > "$1/kd-test.sh" <<'SH'
#!/bin/sh
HOME=/tmp; export HOME
TMPDIR=/tmp; export TMPDIR
out=$(/kd-arm melt -version 2>&1) || exit 20
case "$out" in *melt*) ;; *) exit 21 ;; esac
/kd-arm melt color:blue out=12 -consumer avformat:/tmp/kd.mp4 vcodec=libx264 preset=ultrafast an=1 \
  >/dev/null 2>&1 || exit 22
[ -s /tmp/kd.mp4 ] || exit 23
exit 0
SH
}

run_arm() {  # rootfs artefact tag -> exit status
  _r="$1"; _a="$2"; _t="$3"
  rm -f "$_r/kd-arm"; cp "$_a" "$_r/kd-arm"; chmod +x "$_r/kd-arm"
  write_test "$_r"
  timeout -k 10 "$RUN_TIMEOUT" "$RR" rootfs run "$_r" -- /bin/sh /kd-test.sh \
    </dev/null >"$B/out.$_t" 2>&1
  _st=$?
  printf '%s' "$_st"
}

printf '  %-19s %-6s | %-8s %-6s | %-8s %-6s\n' ENVIRONMENT LIBC 'P:res' 'P:host' 'E:res' 'E:host'
ENVS=0; P_RUNS=0; E_RUNS=0; P_CLEAN=0; E_CLEAN=0
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  root=$(exp_rootfs "$name") || root=""
  [ -n "$root" ] || { exp_skip "$name" "not fetched"; continue; }
  ENVS=$((ENVS + 1))

  pst=$(run_arm "$root" "$OURS" "$name.P")
  timeout -k 10 "$RUN_TIMEOUT" strace -f \
    -e trace=openat,open,execve,clone,clone3,vfork,fork -o "$B/tr.$name.P" \
    "$RR" rootfs run "$root" -- /bin/sh /kd-test.sh </dev/null >/dev/null 2>&1
  reap_rootfs "$root"
  pnh=$(classify_trace "$B/tr.$name.P" /kd-arm tree | grep '^host ' | count)

  est=$(run_arm "$root" "$ENH" "$name.E")
  timeout -k 10 "$RUN_TIMEOUT" strace -f \
    -e trace=openat,open,execve,clone,clone3,vfork,fork -o "$B/tr.$name.E" \
    "$RR" rootfs run "$root" -- /bin/sh /kd-test.sh </dev/null >/dev/null 2>&1
  reap_rootfs "$root"
  enh_=$(classify_trace "$B/tr.$name.E" /kd-arm tree | grep '^host ' | count)

  rm -f "$root/kd-arm" "$root/kd-test.sh" "$root/tmp/kd.mp4"
  case "$pst" in 0) pres=ok ;; 124) pres=timeout ;; 13[0-9]|1[4-6][0-9]) pres="SIG$((pst-128))" ;; *) pres="exit$pst" ;; esac
  case "$est" in 0) eres=ok ;; 124) eres=timeout ;; 13[0-9]|1[4-6][0-9]) eres="SIG$((est-128))" ;; *) eres="exit$est" ;; esac
  [ "$pres" = ok ] && P_RUNS=$((P_RUNS + 1))
  [ "$eres" = ok ] && E_RUNS=$((E_RUNS + 1))
  [ "${pnh:-1}" = 0 ] && P_CLEAN=$((P_CLEAN + 1))
  [ "${enh_:-1}" = 0 ] && E_CLEAN=$((E_CLEAN + 1))
  printf '  %-19s %-6s | %-8s %-6s | %-8s %-6s\n' "$name" "$libc" "$pres" "${pnh:-?}" "$eres" "${enh_:-?}"
  printf '%-19s %-6s P=%s host=%s | E=%s host=%s\n' "$name" "$libc" "$pres" "${pnh:-?}" "$eres" "${enh_:-?}" \
    >> "$EXP_OUT/per-environment.txt"
done < "$REPO_DIR/scripts/common/rootfs-images.txt"

printf '\n'
# ⛔ THE INSTRUMENT'S OWN NEGATIVE CONTROL, ASSERTED. Two copies of arm P
# through the identical protocol have a true ratio of 1.00; if this instrument
# can tell them apart, every millisecond in this file is unreadable and the run
# must fail rather than publish. `experiments/clock.sh`, and
# `docs/history/corrections.md` C23-C24 for what the absence of this cost.
exp_check "the A/A pair is NOT resolvable (the control)" \
  "$(clk_resolves "$CLK_AA" "$CLK_FLOOR")" "no"

exp_check "environments measured"                  "$ENVS"    "11"
exp_check "ours rendered on every environment"     "$P_RUNS"  "$ENVS"
exp_check "the competitor did too"                 "$E_RUNS"  "$ENVS"
# ⛔ THE HOST-OBJECT COLUMN IS A COMPARISON HERE, NOT AN ABSOLUTE, AND THE
# REASON IS THE ARTEFACT'S OWN SHAPE. A multi-program bundle needs a SELECTOR,
# a selector is a shell script, and a script is run by the HOST's /bin/sh --
# which loads the host's libc. Both arms are built that way:
# `kdenlive-AppImage-Enhanced` ships `AppRun.sh` for exactly the same reason.
# ⭐ So what is asserted is that OURS IS NO WORSE THAN THE COMPETITOR on every
# row, and the four musl rows -- which have no glibc for a shell to load --
# come out at zero for both. `pgb bundle appimage` now takes the shell only
# when there is more than one program, so the single-program bundles
# `experiments/85-`, `86-` and `89-` measure are unaffected.
exp_check "ours is no worse than the competitor on host objects" \
          "$([ "$P_CLEAN" -ge "$E_CLEAN" ] && echo yes || echo no)" "yes"
exp_note "clean rows: ours $P_CLEAN/$ENVS, the competitor $E_CLEAN/$ENVS"
exp_note "⚠ both arms use a shell AppRun; the four musl rows are zero for both"

{
  printf '90 - our kdenlive bundle against pkgforge-dev/kdenlive-AppImage-Enhanced\n\n'
  printf 'competitor pinned at %s\n\n' "$ENH_TAG"
  printf '  %-34s %14s\n' ARTEFACT BYTES
  printf '  %-34s %14s\n' 'P  ours (one command, nixpkgs)' "$P_SZ"
  printf '  %-34s %14s\n' 'E  kdenlive-AppImage-Enhanced' "$E_SZ"
  printf '  %-34s %14s\n' '   ratio P/E' \
    "$(awk -v p="$P_SZ" -v e="$E_SZ" 'BEGIN{printf "%.2fx", p/e}')"
  printf '  %-34s %14s\n' 'O  onelf (our payload, their packer)' "${O_SZ:-skipped}"
  printf '\nrender (melt, a real MP4):\n'
  printf '  P %s ms, %s bytes of MP4\n' "$P_R" "$P_MP4"
  printf '  E %s ms, %s bytes of MP4\n' "$E_R" "$E_MP4"
  printf '  O %s ms, %s bytes of MP4\n' "${O_R:-skipped}" "${O_MP4:-0}"
  printf '\nstartup (melt -version):\n'
  printf '  ⭐ cold: median of %s, arms interleaved with a rotating start,\n' "$ROUNDS"
  printf '     each sample preceded by a REAP of the live dwarfs mount and an\n'
  printf '     emptied XDG_CACHE_HOME. docs/history/corrections.md C24: the\n'
  printf '     protocol this replaces obtained "cold" by copying the file, and\n'
  printf '     uruntime keys its mount on CONTENT, so the copy reused the live\n'
  printf '     mount and the column reported a warm start.\n'
  printf '     A/A control: ratio %s, floor %s, resolves %s (must be "no")\n' \
    "$CLK_AA" "$CLK_FLOOR" "$(clk_resolves "$CLK_AA" "$CLK_FLOOR")"
  printf '  P %s ms cold, %s ms warm\n' "$P_COLD" "$P_WARM"
  printf '  E %s ms cold, %s ms warm\n' "$E_COLD" "$E_WARM"
  printf '  O %s ms cold, %s ms warm\n' "${O_COLD:-skipped}" "${O_WARM:-skipped}"
  printf '\non the eleven: ours %s/%s rendered, %s/%s with zero host objects;\n' \
    "$P_RUNS" "$ENVS" "$P_CLEAN" "$ENVS"
  printf '               the competitor %s/%s rendered, %s/%s clean.\n' \
    "$E_RUNS" "$ENVS" "$E_CLEAN" "$ENVS"
  printf '\nconditions: %s, %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(uname -sr)"
} > "$EXP_OUT/RESULT.txt"

exp_finish
