#!/bin/sh
# THE QUESTION
#
#   Every capability row in docs/research/bundle-capabilities.md §0, measured
#   through `pgb bundle appimage` on all eleven environments, with THREE
#   applications per category ordered simple -> complex.
#
# ⛔ WHY THREE AND NOT ONE, AND IT IS THE OPERATOR'S INSTRUCTION.
# 2026-09-03f: *"every capability listed in docs/research/bundle-capabilities.md
# including ones already measured, must be remeasured with 3 applications per
# category in order of simple to complex applications"*.
#
# ⭐ AND THE REASON IS ALREADY IN THE RECORD. `experiments/64-` scored GTK on
# ONE subject, galculator, and got 0 of 11 — from which "GTK does not work out
# of a nix closure" would have been the obvious and WRONG conclusion. A second
# subject, mousepad, drew 11 of 11 through the same bundler on the same day.
# ⛔ ONE SUBJECT MEASURES A SUBJECT. Three, ordered by how much of the stack
# they drag in, measure a capability.
#
# -- ⛔ THE SUCCESS CRITERION, AND WHY IT IS NOT THE PROGRAM'S OWN OUTPUT -----
#
# ⛔ `Gtk-WARNING: cannot open display` IS NOT A RESULT — it is printed on real
# hardware WITH a display too. `experiments/64-` scored eleven green rows on it
# and the operator rejected them. So a GUI subject is scored by asking the
# X SERVER, from outside the process, whether a window exists:
#
#   gui  a toplevel window on a real Xvfb display, seen with `xwininfo`
#   cli  the program's own exit status AND a required string in its output
#
# ⭐ EVERY subject, in both modes, additionally has to load ZERO HOST SHARED
# OBJECTS, by `experiments/lib.sh`'s `exp_classify_trace` — attributed across
# the process tree, matching `.so`/`.so.N` at the END of a path rather than
# anywhere in it, and pairing a split `openat` with its own result line.
#
# -- ⛔ EVERY SUBJECT RUNS IN EXTRACT MODE, AND IT IS NOT A PREFERENCE -------
#
# ⛔ `strace` DEADLOCKS ON A dwarfs FUSE MOUNT. Measured on `experiments/64-`
# arm P, twice: strace reads a path argument out of the tracee's address
# space, that page is backed by the mount, and the only process that can serve
# it is the FUSE daemon — which strace has itself ptrace-STOPPED. strace ends
# up in state D, `kill` cannot end a process in D, and the row freezes.
#
# ⭐ `APPIMAGE_EXTRACT_AND_RUN=1` removes the daemon: uruntime unpacks to a
# tmpfs and runs from there. ⚠ IT IS APPLIED TO EVERY SUBJECT rather than to
# the ones that were seen to hang, because "the ones somebody hit" is the
# shape of list T-081 exists to replace. Mount and extract are two DELIVERY
# modes of the same artefact and neither criterion here depends on which is
# used — the window is the X server's fact, the host objects are the process
# tree's. ⚠ It costs about a gigabyte of tmpfs per row, and this machine has
# 16 GiB. ⛔ AND IT COSTS TIME, WHICH IS THE PART THAT WAS GUESSED AT AND WAS
# WRONG: this comment said "ten to twenty seconds" and the window budget was
# set from that guess. MEASURED 2026-09-04, machine otherwise idle: mousepad's
# bundle put its first toplevel on the X server at **t+21s** on alpine-3.22 —
# unpack included, under strace. See the budget block below and C26.
#
# -- ⭐ PRE-REGISTERED EXPECTATION -------------------------------------------
#
# ⛔ COMMITTED BEFORE THE RUN. PROGRESS.md delivery rule 1.
#
#   C1  every subject that BUILDS reaches 11 of 11 on its criterion.
#   C2  every subject that builds loads zero host shared objects on 11 of 11.
#   C3  ⛔ the GPU categories do NOT reach C1 on hardware terms: every GL and
#       Vulkan row here is a SOFTWARE rasteriser (`llvmpipe` / `lavapipe`),
#       and this experiment CANNOT say anything about a real GPU. T-059.
#   C4  a subject nixpkgs cannot resolve, or whose closure will not fetch, is
#       reported as UNRESOLVED with its reason — never counted as a pass and
#       never counted as a failure of the capability.
#   C5  ⛔ `xterm` is expected to FAIL C2, and the reason is the application
#       rather than the bundler: xterm's whole job is to run the user's
#       SHELL, which is a HOST program, so a host libc enters the process by
#       construction. It is in the corpus BECAUSE of that — it is what
#       "complex" means on the X11 row. ⚠ If it passes, the prediction was
#       wrong and the record says so rather than quietly dropping it.
#   C6  ⭐ THE POSITIVE CONTROL, AND THE FIRST VERSION OF THIS FILE HAD NONE.
#       THREE of the twenty-six subjects have already been measured at 11 of
#       11 by a DIFFERENT experiment, twice each: `gtk3-1` (galculator) and
#       `gtk3-2` (mousepad) are `experiments/64-` arms G and X, and `py-1`
#       (meld) is arm P. ⛔ If any of the three comes back BELOW 11 of 11
#       here, the disagreement is with a known-good measurement, so the
#       INSTRUMENT is the first suspect and this run's other rows cannot be
#       read as capability results. That is what C6 asserts, and it is the
#       assertion that would have caught the 25-second budget on the first
#       run instead of after eleven rows.
#
# ⚠ C6 IS NOT CIRCULAR. It does not assert that the corpus passes; it asserts
# that three subjects an independent experiment measured green come back green
# through this instrument. A run where C6 fails and C1 passes is a run whose
# greens mean nothing.
#
# ⚠ C3 IS PRE-REGISTERED AS A LIMIT RATHER THAN A RESULT, deliberately: it is
# the sentence T-080's guarantee has to keep saying, and writing it here before
# the run is what stops a green table from being read as a GPU claim.
#
# -- ⭐ THE `field` ROWS ARE T-081's OWN "Prove" LINE, PARTLY PAID ----------
#
# T-081 asks for a corpus run over *"the 13 active `nixappimage` recipes in
# `references/pkgforge__soarpkgs`"*, measuring **how many store paths each
# route leaves behind**. Four of the thirteen are here — `helix`, `neovim`,
# `flameshot` and `gearlever` — and the `PATHS` column is that measurement for
# our route: how many compiled-in store paths a bundle carries and how many of
# them resolve inside it.
#
# ⛔ THE OTHER NINE ARE NOT RUN AND THE REASON IS DISK, NOT CHOICE. They are
# chromium, brave, ungoogled-chromium, telegram-desktop, simplex-desktop and
# four discord channels; each closure is multiple GB and this machine has
# ~17 GiB free with a 2.3 GiB cache per subject. ⚠ An absence is not a zero:
# what is measured here is four of thirteen, and the four are the small ones.
#
# -- ⚠ THE ATTRIBUTES WERE PROBED BEFORE THE RUN ----------------------------
#
# ⭐ `pgb nix plan <attr>` resolves an attribute without building anything, and
# all twenty-one non-obvious ones in the corpus resolved on 2026-09-03f. That
# is worth an hour: a corpus that spends twenty minutes discovering a typo per
# subject measures the typo.
#
# ⛔ IT IS NOT A GUARANTEE THAT THE SUBJECT WILL BUILD. Resolution says the
# attribute exists; the closure still has to fetch, the entry point still has
# to resolve, and `--name` still has to match a program in `bin/`. Each of
# those failures is reported as UNRESOLVED with its own reason, and the reason
# for the last one is a list of what IS in `bin/`.
#
# -- ⛔ WHAT THIS COSTS, SO THE NEXT SESSION DOES NOT RE-RUN IT BY ACCIDENT ---
#
# Each subject is a closure fetch, an AppDir, a dwarfs pack and eleven chroot
# runs. ⚠ DISK IS THE BINDING CONSTRAINT: a cache is ~2.3 GiB and an artefact
# ~170 MiB, so the runner DELETES each subject's cache and artefact as soon as
# its row is recorded, and writes the row to `evidence/65-capability-corpus/`
# immediately. It is RESUMABLE: a subject with a recorded row is not re-run.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "65 - the capability corpus: three applications per category, simple to complex"

WORK="${PGB_EXP65_WORK:-/var/tmp/t065}"
ROWS="${PGB_EXP65_ROWS:-$REPO_DIR/evidence/65-capability-corpus/rows}"
mkdir -p "$WORK" "$ROWS" || exit 2
# ⚠ THE TIMEOUT IS A BOUND, NOT A GUESS AT HOW LONG A PROGRAM TAKES. A `cli`
# subject is waited for rather than killed, so a hanging one costs this many
# seconds ELEVEN times; a `gui` subject is killed as soon as a window appears.
# A subject that genuinely needs longer than this to print its version is a
# finding, not a timeout to raise.
#
# ⛔ THE FIRST VERSION OF THIS FILE CARRIED experiments/64-'s 25-SECOND WINDOW
# BUDGET ACROSS A CHANGE THAT INVALIDATED IT, AND THE RUN SCORED galculator
# 0 OF 11 — a subject `experiments/64-` had measured at 11 of 11, TWICE.
# 64- uses 25s for the arms it runs in MOUNT mode, where a program starts in
# about two seconds, and 150s for the ONE arm it runs in EXTRACT mode. This
# experiment runs EVERY subject in extract mode (strace deadlocks on the FUSE
# mount) and kept the mount-mode number. ⭐ MEASURED, 2026-09-04, machine
# otherwise idle: mousepad's bundle put its first toplevel on the X server at
# t+21s on alpine-3.22 — four seconds inside a budget that had to absorb a
# 195 MB unpack as well. docs/history/corrections.md C26.
#
# ⭐ SO THE WINDOW BUDGET IS NOT A CONSTANT ANY MORE, IT IS THE RUN BUDGET.
# The poll ends when a window appears, when the process exits, or when
# `timeout` kills it — never on a number chosen separately from the thing it
# is timing. A failing gui row therefore costs RUN_TIMEOUT and a passing one
# costs the subject's start time.
RUN_TIMEOUT="${PGB_EXP65_TIMEOUT:-150}"
WIN_WAIT="${PGB_EXP65_WIN_WAIT:-$RUN_TIMEOUT}"
ONLY="${PGB_EXP65_ONLY:-}"          # run only subjects whose id matches this

command -v strace >/dev/null 2>&1 || { exp_note "no strace on PATH"; exit 2; }

# ---------------------------------------------------------------------------
# ⭐ THE CORPUS. Fields, separated by '|':
#
#   id | category | nixpkgs attribute | program | mode | assertion | extras | args
#
# ⛔ ORDERED SIMPLE -> COMPLEX WITHIN EACH CATEGORY, and "complex" means how
# much of the stack the subject drags in rather than how big it is:
# galculator is gtk3 and nothing else; geany is gtk3 plus a plugin ABI, a
# terminal widget and its own data tree.
#
# ⚠ `assertion` is a grep -E pattern the program's own output must match, and
# it is checked IN ADDITION to the mode's criterion, never instead of it. An
# empty assertion means the mode's criterion is the whole test.
# ---------------------------------------------------------------------------
CORPUS=$(cat <<'EOF'
gtk3-1|GTK 3|galculator|galculator|gui|||
gtk3-2|GTK 3|mousepad|mousepad|gui|||
gtk3-3|GTK 3|geany|geany|gui|||
x11-1|X11 / XCB|xorg.xeyes|xeyes|gui|||
x11-2|X11 / XCB|xorg.xclock|xclock|gui|||
x11-3|X11 / XCB|xterm|xterm|gui|||
gl-1|OpenGL / EGL|mesa-demos|eglinfo|cli|(llvmpipe|Mesa|softpipe)|mesa|
gl-2|OpenGL / EGL|mesa-demos|glxgears|gui||mesa|
gl-3|OpenGL / EGL|glmark2|glmark2|gui||mesa|
vulkan-1|Vulkan|vulkan-tools|vulkaninfo|cli|(lavapipe|llvmpipe|Vulkan Instance)|mesa|--summary
vulkan-2|Vulkan|vulkan-tools|vkcube|gui||mesa|
vulkan-3|Vulkan|vkmark|vkmark|gui||mesa|
sdl-1|SDL|dosbox|dosbox|gui|||
sdl-2|SDL|stella|stella|gui|||
sdl-3|SDL|scummvm|scummvm|gui|||
qt-1|Qt|qalculate-qt|qalculate-qt|gui|||
qt-2|Qt|keepassxc|keepassxc|gui|||
qt-3|Qt|qbittorrent|qbittorrent|gui|||
py-1|Python GUI|meld|meld|gui|||
py-2|Python GUI|pdfarranger|pdfarranger|gui|||
py-3|Python GUI|virt-manager|virt-manager|gui|||
media-1|media / codecs|mpv|mpv|cli|mpv [0-9]|mesa|--version
field-1|the field's own recipes|helix|hx|cli|helix [0-9]||--version
field-2|the field's own recipes|neovim|nvim|cli|NVIM v[0-9]||--version
field-3|the field's own recipes|flameshot|flameshot|gui|||
field-4|the field's own recipes|gearlever|gearlever|gui|||
EOF
)

# ---------------------------------------------------------------------------
# ⭐ A REAL DISPLAY. ⛔ Xvfb IS REQUIRED, NOT OPTIONAL.
# ---------------------------------------------------------------------------
XDISP="${PGB_EXP65_DISPLAY:-:99}"
for t in Xvfb xwininfo; do
  command -v "$t" >/dev/null 2>&1 || {
    exp_note "no $t on PATH — Debian/Ubuntu: apt-get install xvfb x11-utils"
    exit 2; }
done
XVFB_PID=""
if ! DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1; then
  Xvfb "$XDISP" -ac -screen 0 1024x768x24 >"$WORK/xvfb.log" 2>&1 &
  XVFB_PID=$!
  _w=0
  while [ "$_w" -lt 20 ]; do
    DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1 && break
    _w=$((_w+1)); sleep 1
  done
fi
DISPLAY="$XDISP" xdpyinfo >/dev/null 2>&1 || {
  exp_note "Xvfb did not come up on $XDISP; see $WORK/xvfb.log"; exit 2; }
trap '[ -n "$XVFB_PID" ] && kill "$XVFB_PID" 2>/dev/null' EXIT INT TERM

# ⛔ A WINDOW TITLE IS NOT A RELIABLE KEY ACROSS A CORPUS. `qalculate-qt`
# titles its window "Qalculate!" and `virt-manager` titles it "Virtual Machine
# Manager", so matching the program name would score a working bundle exactly
# like a broken one — the same class of non-discriminating criterion the
# operator rejected in `experiments/64-`.
#
# ⭐ SO THE KEY IS GEOMETRY, WHICH EVERY TOOLKIT AGREES ON: a DIRECT CHILD of
# the root window at least 50x50. ⚠ That floor is not decoration — GTK and Qt
# both create tiny 1x1 and 10x10 helper toplevels that exist whether or not
# the application ever draws, so counting "any child" would report a window
# for a program that died in its first second.
#
# ⭐ CHECKED IN BOTH DIRECTIONS BEFORE THE RUN, against real `xwininfo` output:
# meld's window tree — one 508x400 toplevel beside a 1x1 and two 10x10 helpers
# — counts 1, and an empty root ("0 children.") counts 0. An instrument that
# has only been checked on the passing case is half an instrument.
windows_real() {
  DISPLAY="$XDISP" xwininfo -root -children 2>/dev/null | awk '
    /^ +0x[0-9a-f]+/ {
      if (match($0, /[0-9]+x[0-9]+\+/)) {
        split(substr($0, RSTART, RLENGTH - 1), d, "x")
        if (d[1] + 0 >= 50 && d[2] + 0 >= 50) n++
      }
    }
    END { print n + 0 }'
}

# ⛔ REAP BY WHAT A PROCESS IS CHROOTED INTO, NOT BY ITS NAME. docs/AGENTS.md §14.
reap_in_root() {
  _rr=$1
  for _p in /proc/[0-9]*; do
    _pid=${_p#/proc/}
    _rt=$(readlink "/proc/$_pid/root" 2>/dev/null) || continue
    case "$_rt" in "$_rr"|"$_rr"/*) kill -9 "$_pid" 2>/dev/null ;; esac
  done
}

# ⭐ THE CLASSIFIER IS experiments/lib.sh's `exp_classify_trace`, NOT A COPY.
# ⛔ Nine experiments carried the same awk by hand and they could not be
# corrected together: 2026-09-03f found that a split `openat( ... <unfinished
# ...>` carries the PATH and no result, so a filter dropping lines that contain
# ENOENT keeps the first half of a FAILED open and counts it as a load. A
# galculator bundle read 2 host shared objects on alpine-3.22 and both were
# ENOENT probes. One implementation is what stops that from having to be found
# nine times. docs/history/corrections.md.

ENVS=$(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt")
NENV=$(printf '%s\n' "$ENVS" | wc -l | tr -d ' ')

TOTAL=0; BUILT=0; UNRESOLVED=0; FULL=0; CLEANALL=0

# ⭐ C6, THE POSITIVE CONTROL. These three ids are `experiments/64-` arms G, X
# and P, each measured at 11 of 11 on these same eleven environments, twice.
# ⛔ They are named here rather than inferred, so that renaming a corpus id
# silently drops the control instead of silently passing it: CTRL_SEEN counts
# how many of them this run actually measured, and the check below compares it
# against the list.
CONTROLS="gtk3-1 gtk3-2 py-1"
NCONTROLS=$(printf '%s\n' $CONTROLS | wc -l | tr -d ' ')
CTRL_SEEN=0; CTRL_OK=0
note_control() {   # id pass rows
  case " $CONTROLS " in *" $1 "*) ;; *) return 0 ;; esac
  CTRL_SEEN=$((CTRL_SEEN+1))
  [ "$2" = "$3" ] && [ "$3" -gt 0 ] && CTRL_OK=$((CTRL_OK+1))
  return 0
}

show_row() { # id category subject mode pass clean paths note
  printf '  %-10s %-16s %-14s %-4s %-7s %-7s %-9s %s\n' \
    "$1" "$2" "$3" "$4" "$5" "$6" "$7" "$8"
}

printf '\n'
show_row ID CATEGORY SUBJECT MODE 'PASS/N' 'CLEAN/N' 'PATHS' NOTE

# ⛔ `for line in $CORPUS` WORD-SPLITS ON SPACES and a category is
# "OpenGL / EGL". Read line by line from a FILE on fd 3 instead: a pipeline
# would put the loop in a subshell and every counter below would be lost at
# the end of it, which is the quieter of the two bugs and the harder to see.
printf '%s\n' "$CORPUS" > "$WORK/corpus.txt"
while IFS= read -r line <&3; do
  [ -n "$line" ] || continue
  id=$(printf '%s' "$line" | cut -d'|' -f1)
  cat_=$(printf '%s' "$line" | cut -d'|' -f2)
  attr=$(printf '%s' "$line" | cut -d'|' -f3)
  prog=$(printf '%s' "$line" | cut -d'|' -f4)
  mode=$(printf '%s' "$line" | cut -d'|' -f5)
  assert=$(printf '%s' "$line" | cut -d'|' -f6)
  extras=$(printf '%s' "$line" | cut -d'|' -f7)
  args=$(printf '%s' "$line" | cut -d'|' -f8)
  [ -n "$ONLY" ] && { case "$id" in $ONLY) ;; *) continue ;; esac; }
  TOTAL=$((TOTAL+1))
  # every line AFTER this one, so the reclaim step below can ask whether the
  # closure is still wanted
  awk -v me="$id" 'seen {print} $0 ~ "^"me"\\|" {seen=1}' "$WORK/corpus.txt" > "$WORK/remaining.txt"

  # ⭐ A ROW IS STORED AS FIELDS, NOT AS THE TABLE LINE IT PRINTS.
  #
  # ⛔ THE FIRST VERSION STORED THE FORMATTED LINE AND COUNTED ONLY `BUILT`
  # when it read one back, so a RESUMED run — the whole reason the rows exist —
  # scored C1 as `FULL(0) != BUILT(n)` and reported a red corpus in which every
  # subject had passed. ⚠ And parsing the line back was never going to work
  # either: a category is "OpenGL / EGL", so the field positions move.
  #
  #     <id> TAB <pass> TAB <rows> TAB <clean> TAB <paths> TAB <note>
  #
  # `pass = -1` means UNRESOLVED: neither a pass nor a failure of the
  # capability.
  row="$ROWS/$id.tsv"
  if [ -s "$row" ]; then
    pass=$(cut -f2 < "$row"); rows=$(cut -f3 < "$row")
    clean=$(cut -f4 < "$row"); paths=$(cut -f5 < "$row"); note=$(cut -f6 < "$row")
    if [ "$pass" = "-1" ]; then
      UNRESOLVED=$((UNRESOLVED+1))
      show_row "$id" "$cat_" "$attr" "$mode" "-" "-" "-" "$note (recorded)"
    else
      BUILT=$((BUILT+1))
      [ "$pass" = "$rows" ] && [ "$rows" -gt 0 ] && FULL=$((FULL+1))
      [ "$clean" = "$rows" ] && [ "$rows" -gt 0 ] && CLEANALL=$((CLEANALL+1))
      note_control "$id" "$pass" "$rows"
      show_row "$id" "$cat_" "$attr" "$mode" "$pass/$rows" "$clean/$rows" "$paths" "$note (recorded)"
    fi
    continue
  fi

  img="$WORK/$id.AppImage"
  # ⭐ THE CACHE IS KEYED ON THE ATTRIBUTE, NOT THE SUBJECT. `vulkan-tools`
  # supplies both `vulkaninfo` and `vkcube`, and `mesa-demos` both `eglinfo`
  # and `glxgears`; keying on the id fetched each of those closures twice.
  cache="$WORK/cache-$(printf '%s' "$attr" | tr '/.' '__')"
  blog="$WORK/build-$id.log"
  if [ ! -s "$img" ]; then
    set -- bundle appimage "$attr" --out "$img" --name "$prog"
    [ -n "$extras" ] && set -- "$@" --extra "$extras"
    PGB_APPIMAGE_CACHE="$cache" "$REPO_DIR/pgb" "$@" >"$blog" 2>&1 || true
  fi
  if [ ! -s "$img" ]; then
    # ⛔ C4: UNRESOLVED IS NOT A FAILURE OF THE CAPABILITY and is not a pass.
    why=$(grep -aoE "nixpkgs has no attribute [^ ]*|no entry point in [^ ]*|could not fetch the closure[^\"]*|--name [^ ]* names no program" "$blog" 2>/dev/null | head -1)
    note="UNRESOLVED: ${why:-see $blog}"
    printf '%s\t-1\t0\t0\t-\t%s\n' "$id" "$note" > "$row"
    show_row "$id" "$cat_" "$attr" "$mode" "-" "-" "-" "$note"
    UNRESOLVED=$((UNRESOLVED+1))
    rm -rf "$cache"
    continue
  fi
  BUILT=$((BUILT+1))
  paths=$(grep -a -m1 '^store paths' "$blog" | sed 's/^store paths *//' | cut -c1-24)

  pass=0; clean=0; rows=0
  for name in $ENVS; do
    root=$(exp_rootfs "$name") || true
    [ -n "$root" ] || { exp_skip "$id/$name" "rootfs not fetched"; continue; }
    rows=$((rows+1))
    # ⛔ THE DISPLAY MUST BE IDLE BEFORE THE SUBJECT STARTS, or a window left
    # by the previous row is counted as this one's.
    _q=0
    while [ "$_q" -lt 10 ] && [ "$(windows_real)" != 0 ]; do sleep 1; _q=$((_q+1)); done
    # ⛔ AND WHEN IT DOES NOT GO IDLE, THE WAIT GAVE UP AND MEASURED ANYWAY.
    # That is the false positive the operator named — "a second program's
    # windows become a false positive nothing else catches" — and a loop that
    # proceeds after ten seconds is exactly how it gets in. ⭐ So the criterion
    # is a DELTA against what was already on the server, not a count: a
    # leftover window can no longer make this row pass, and a row is not
    # thrown away for something the previous subject left behind.
    base=$(windows_real)
    [ "$base" != 0 ] && exp_note "⚠ $id/$name: display not idle at launch ($base window(s)); counting the DELTA"

    rm -f "$root/subj65"; cp "$img" "$root/subj65" 2>/dev/null; chmod +x "$root/subj65"
    tr="$WORK/tr.$id.$name"
    strace -f -e trace=openat,open,execve,clone,clone3,vfork -o "$tr" \
      timeout "$RUN_TIMEOUT" "$REPO_DIR/pgb" rootfs run "$root" \
        --bind /tmp/.X11-unix:/tmp/.X11-unix -- \
        /bin/sh -c "DISPLAY=$XDISP APPIMAGE_EXTRACT_AND_RUN=1 /subj65 $args" \
      >"$WORK/out.$id.$name" 2>"$WORK/err.$id.$name" &
    _sp=$!
    win=0; _n=0
    if [ "$mode" = gui ]; then
      while [ "$_n" -lt "$WIN_WAIT" ]; do
        sleep 1; _n=$((_n+1))
        win=$(windows_real)
        [ "$win" -gt "$base" ] && break
        # ⭐ AND THE LOOP ENDS WITH THE PROCESS, NOT ON A NUMBER OF ITS OWN.
        # A gui program that WORKS does not exit, so this is the branch that
        # ends a FAILING row early: `timeout` kills the tree at RUN_TIMEOUT
        # and the poll notices.
        kill -0 "$_sp" 2>/dev/null || break
      done
      kill "$_sp" 2>/dev/null
    fi
    wait "$_sp" 2>/dev/null
    st=$?
    reap_in_root "$root"
    rm -f "$root/subj65"

    all=$(cat "$WORK/err.$id.$name" "$WORK/out.$id.$name" 2>/dev/null | tr -d '\r')
    ok=no
    if [ "$mode" = gui ]; then
      [ "$win" -gt "$base" ] && ok=yes
    else
      [ "$st" = 0 ] && ok=yes
    fi
    # ⚠ THE ASSERTION IS AND-ED WITH THE MODE'S CRITERION, never substituted
    # for it: a window with no renderer string is not an OpenGL row.
    if [ -n "$assert" ] && [ "$ok" = yes ]; then
      printf '%s' "$all" | grep -qE "$assert" || ok=no
    fi
    [ "$ok" = yes ] && pass=$((pass+1))
    nhost=$(exp_classify_trace "$tr" /subj65 | grep -c '^host ' || true)
    [ "$nhost" = 0 ] && clean=$((clean+1))
    rm -f "$tr"
  done

  note=""
  [ "$pass" -lt "$rows" ] && note=$(cat "$WORK/err.$id."* 2>/dev/null | tr -d '\r' \
    | grep -m1 -E "Couldn't load|cannot open|Traceback|error while loading|not found|Error" | cut -c1-70)
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$id" "$pass" "$rows" "$clean" "${paths:--}" "$note" > "$row"
  show_row "$id" "$cat_" "$attr" "$mode" "$pass/$rows" "$clean/$rows" "${paths:--}" "$note"
  [ "$pass" = "$rows" ] && [ "$rows" -gt 0 ] && FULL=$((FULL+1))
  [ "$clean" = "$rows" ] && [ "$rows" -gt 0 ] && CLEANALL=$((CLEANALL+1))
  note_control "$id" "$pass" "$rows"

  # ⛔ RECLAIM IMMEDIATELY. A cache is ~2.3 GiB; twenty-six of them is far
  # more disk than this machine has, and a run that dies on ENOSPC halfway
  # measures nothing. ⚠ But NOT while a later subject still needs the same
  # closure — the corpus keeps subjects sharing an attribute adjacent, and
  # this asks the remaining lines rather than assuming it.
  rm -f "$img" "$WORK/out.$id."* "$WORK/err.$id."*
  if ! grep -q "^[^|]*|[^|]*|$attr|" "$WORK/remaining.txt" 2>/dev/null; then
    rm -rf "$cache"
  fi
done 3< "$WORK/corpus.txt"

printf '\n'
printf -- '-- summary ---------------------------------------------------------\n'
printf '  %-40s %s\n' 'subjects in the corpus'            "$TOTAL"
printf '  %-40s %s\n' 'subjects that produced an artefact' "$BUILT"
printf '  %-40s %s\n' '⛔ UNRESOLVED (not a pass, not a fail)' "$UNRESOLVED"
printf '  %-40s %s\n' "⭐ subjects passing on all $NENV"     "$FULL"
printf '  %-40s %s\n' "⭐ subjects clean on all $NENV"       "$CLEANALL"

printf '\n'
# ⛔ WITHOUT THIS ROW, A RUN WHERE NOTHING BUILT SCORES GREEN: C1 and C2 both
# compare 0 against 0. A check that cannot fail on the state it exists to catch
# is the worst answer this codebase can give — docs/AGENTS.md §0b.
exp_check "at least one subject produced an artefact" \
  "$([ "$BUILT" -gt 0 ] && echo yes || echo no)" yes

# ⭐ C6 IS CHECKED BEFORE C1 AND C2 ON PURPOSE. If the instrument disagrees
# with a measurement another experiment took twice, nothing below it can be
# read as a capability result — so the control is the first line of the
# verdict, not a footnote under it.
exp_check "C6  ⭐ control subjects MEASURED ($CONTROLS)" "$CTRL_SEEN" "$NCONTROLS"
exp_check "C6  ⭐ control subjects at $NENV of $NENV, as 64- measured them" \
  "$CTRL_OK" "$CTRL_SEEN"
if [ "$CTRL_SEEN" -gt 0 ] && [ "$CTRL_OK" != "$CTRL_SEEN" ]; then
  exp_note "⛔ A CONTROL FAILED, SO THE INSTRUMENT IS THE FIRST SUSPECT."
  exp_note "   experiments/64- measured galculator, mousepad and meld at"
  exp_note "   $NENV of $NENV each, TWICE, on these same environments. A row"
  exp_note "   below $NENV here is a disagreement with that, and the rest of"
  exp_note "   this table cannot be read as capability results until it is"
  exp_note "   explained. docs/history/corrections.md C26 is the last time"
  exp_note "   this fired: a window budget carried across a change of"
  exp_note "   delivery mode."
fi

exp_check "C1  every subject that BUILT passes on all $NENV" "$FULL"     "$BUILT"
exp_check "C2  every subject that BUILT is clean on all $NENV" "$CLEANALL" "$BUILT"
exp_note "⛔ C3 IS A LIMIT AND IT IS NOT MEASURED AWAY BY A GREEN TABLE."
exp_note "   Every OpenGL and Vulkan row here is a SOFTWARE rasteriser —"
exp_note "   llvmpipe and lavapipe — on a machine with no GPU. This experiment"
exp_note "   says nothing about NVIDIA, about a real driver, or about"
exp_note "   performance. T-059 owns hardware."
exp_note "⛔ C4: an UNRESOLVED subject is a gap in THIS corpus, not evidence"
exp_note "   about the capability. Its reason is printed in its row."
exp_note "⚠ The rows are kept under evidence/65-capability-corpus/rows so a"
exp_note "   session that stops halfway does not lose hours of closure fetches."

exp_finish
