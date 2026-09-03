#!/bin/sh
# THE QUESTION
#
#   Does a PYTHON GUI APPLICATION, bundled out of a nixpkgs closure by
#   `pgb bundle appimage`, actually run on all eleven environments?
#
# ⛔ WHY THIS EXPERIMENT EXISTS, AND IT IS THE OPERATOR'S OWN COUNTER-EXAMPLE.
# `Anylinux-AppImages`' `HALL-OF-FAME.md` grades **Python "Utter garbage"** and
# **GTK "Garbage"**. The operator's answer, 2026-09-03d:
#
#   *"in nixappimage for instance, python is easy and works, choose any python
#    gui app and it works"*
#
# ⭐ AND THE GRADES ARE NOT OURS TO QUOTE. They were earned deploying **Arch
# packages** through `quick-sharun`, where a library's data files and plugin
# directories must be discovered by hand. A nix closure is the opposite: it is
# the exact set the derivation declared. ⛔ T-080's rule is that a row not run
# through `pgb bundle appimage` is a **HYPOTHESIS**. This experiment converts
# two of them — Python and GTK — into measurements, or fails trying.
#
# -- WHY meld ----------------------------------------------------------------
#
# ⭐ IT IS BOTH BOTTOM ROWS AT ONCE. `meld` is a Python 3 application whose UI
# is GTK 3 reached through PyGObject, so one artefact exercises:
#
#   Python      an interpreter that must find its own stdlib, its site-packages
#               and its compiled extension modules inside the bundle
#   GTK         a typelib + introspection stack that must find its .typelib
#               files, its loaders and its schemas inside the bundle
#
# ⚠ It is ONE application. It is not evidence about every Python or GTK
# program, and §"what this does not establish" says so at the end.
#
# -- ⭐ WHAT IS CLAIMED, AND SAID IN THE SENTENCE ITSELF ---------------------
#
# ⛔ T-080 names OVERCLAIMING as the first of two easy failures. So, precisely:
#
#   CLAIMED     the bundle starts on a distribution that has no meld, no
#               Python of the right version and no GTK, runs the application's
#               OWN code to completion, and loads its Python and GTK stacks
#               from inside itself.
#   NOT CLAIMED that a window appeared. There is no display on this machine and
#               no GPU; `experiments/85-` owns the offscreen GL claim and
#               T-059 owns real hardware. ⛔ Do not read a green row here as
#               "the GUI works on a desktop".
#
# -- THE CONTROL -------------------------------------------------------------
#
# ⭐ WITHOUT IT A GREEN ROW IS A PROGRAM PRINTING A STRING. The control is that
# the target has no meld of its own: `command -v meld` must fail on every one
# of the eleven, so the capability demonstrably came out of the bundle.
#
# -- ⭐ PRE-REGISTERED EXPECTATION -------------------------------------------
#
# ⛔ COMMITTED BEFORE THE RUN. PROGRESS.md delivery rule 1.
#
#   R1  the bundle runs meld's own code on 11 of 11.
#   R2  zero HOST shared objects on 11 of 11, by the `62-` classifier.
#   R3  the trace shows libpython AND libgtk-3 opened FROM THE BUNDLE on
#       every row — i.e. the stacks the field grades worst are the ones
#       demonstrably loading.
#   R4  no target has a meld of its own (the control), 11 of 11.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "64 - a Python + GTK application, bundled from a nixpkgs closure, on the eleven"

WORK="${PGB_EXP64_WORK:-/var/tmp/t080}"
mkdir -p "$WORK" || exit 2
IMG="${PGB_EXP64_IMG:-$WORK/meld.AppImage}"
SUBJ=meld
RUN_TIMEOUT="${PGB_EXP64_TIMEOUT:-120}"

command -v strace >/dev/null 2>&1 || { exp_note "no strace on PATH"; exit 2; }

# ⛔ REAP BY WHAT A PROCESS IS CHROOTED INTO, NOT BY ITS NAME. uruntime leaves
# a dwarfs FUSE daemon behind on purpose — a mount that outlives the program is
# what mount mode IS — and its comm is `memfd:dwarfs`, not the artefact's.
# `pkill -f` is worse: the rootfs path is in the RUNNER's own command line, so
# a full-command-line match kills this script. docs/AGENTS.md §14.
reap_in_root() { # rootfs-path
  _rr=$1
  for _p in /proc/[0-9]*; do
    _pid=${_p#/proc/}
    _rt=$(readlink "/proc/$_pid/root" 2>/dev/null) || continue
    case "$_rt" in "$_rr"|"$_rr"/*) kill -9 "$_pid" 2>/dev/null ;; esac
  done
}

if [ ! -s "$IMG" ]; then
  exp_note "building the bundle — several minutes, ~100+ store paths"
  PGB_APPIMAGE_CACHE="$WORK/cache" "$REPO_DIR/pgb" bundle appimage "$SUBJ" \
    --out "$IMG" --name "$SUBJ" >"$WORK/build.log" 2>&1 || true
fi
[ -s "$IMG" ] || { exp_note "the bundle did not build; see $WORK/build.log"; exit 2; }

exp_check "the Python+GTK bundle built" "$([ -s "$IMG" ] && echo yes || echo no)" yes
exp_note "artefact: $IMG, $(wc -c < "$IMG") bytes"

# ⭐ THE CLASSIFIER IS `experiments/62-`'s, DELIBERATELY. A bundle's trace must
# NOT be attributed to one pid — uruntime forks, mounts and re-execs, so the
# payload runs in a descendant — and a path is only a shared object if it ENDS
# in .so or .so.N, because /etc/ld.so.cache is an index. Both rules are
# docs/AGENTS.md §14 and both were defects here once.
classify_trace() {  # tracefile /artefact
  awk -v want="$2" '
    { pid = $1 }
    $0 ~ ("execve\\(\"" want "\"") { inset[pid] = 1; next }
    ($0 ~ /(clone|clone3|vfork|fork)\(/ || $0 ~ /<\.\.\. (clone|clone3|vfork|fork) resumed>/) \
      && /= [0-9]+$/ { if (inset[pid]) inset[$NF] = 1; next }
    inset[pid] && /open(at)?\(/ && !/ENOENT|= -1/ {
      if (match($0, /"[^"]*"/) == 0) next
      p = substr($0, RSTART + 1, RLENGTH - 2)
      if (p !~ /\.so(\.[0-9]+)*$/) next
      if (p ~ /^\/(usr\/)?(local\/)?lib(32|64)?\//) print "host " p
      else print "bundled " p
    }
  ' "$1" | sort -u
}

ENVS=$(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt")
RAN=0; CLEAN=0; PY=0; GTK=0; NOHOST=0; ROWS=0

printf '\n'
printf '  %-20s %-6s %-5s %-6s %-8s %-7s %s\n' \
  ENVIRONMENT LIBC RUNS 'HOST.so' 'libpython' 'libgtk3' 'BUNDLED .so'
for name in $ENVS; do
  root=$(exp_rootfs "$name") || true
  [ -n "$root" ] || { exp_skip "$name" "not fetched"; continue; }
  ROWS=$((ROWS+1))
  libc=$(exp_rootfs_libc "$name")

  # ⭐ THE CONTROL: the target must not have a meld of its own, or a green row
  # says nothing about the bundle.
  own=$("$REPO_DIR/pgb" rootfs run "$root" -- /bin/sh -c 'command -v meld' 2>/dev/null | head -1)
  [ -z "$own" ] && NOHOST=$((NOHOST+1))

  rm -f "$root/subj64"; cp "$IMG" "$root/subj64" 2>/dev/null; chmod +x "$root/subj64"
  tr="$WORK/tr.$name"
  strace -f -e trace=openat,open,execve,clone,clone3,vfork -o "$tr" \
    timeout "$RUN_TIMEOUT" "$REPO_DIR/pgb" rootfs run "$root" -- /subj64 --version \
    >"$WORK/out.$name" 2>"$WORK/err.$name"
  st=$?
  reap_in_root "$root"
  rm -f "$root/subj64"

  cls=$(classify_trace "$tr" /subj64)
  nhost=$(printf '%s\n' "$cls" | grep -c '^host ' || true)
  nbund=$(printf '%s\n' "$cls" | grep -c '^bundled ' || true)
  haspy=$(printf '%s\n' "$cls"  | grep -c '^bundled .*libpython' || true)
  hasgtk=$(printf '%s\n' "$cls" | grep -c '^bundled .*libgtk-3' || true)

  # ⛔ "RUNS" IS THE APPLICATION'S OWN OUTPUT, NOT THE EXIT STATUS ALONE.
  # meld --version prints its version; a runtime that started and then failed
  # to import its stack can still exit 0 on some paths.
  out=$(tr -d '\r' < "$WORK/out.$name" | head -1)
  case "$out" in
    *meld*) runs=yes; RAN=$((RAN+1)) ;;
    *)      runs=no ;;
  esac
  [ "$nhost" = 0 ] && CLEAN=$((CLEAN+1))
  [ "$haspy"  -gt 0 ] && PY=$((PY+1))
  [ "$hasgtk" -gt 0 ] && GTK=$((GTK+1))

  printf '  %-20s %-6s %-5s %-6s %-8s %-7s %s\n' \
    "$name" "$libc" "$runs" "$nhost" \
    "$([ "$haspy" -gt 0 ] && echo yes || echo no)" \
    "$([ "$hasgtk" -gt 0 ] && echo yes || echo no)" "$nbund"
  printf '      out: %s  [exit %s]\n' "${out:-<none>}" "$st"
done

printf '\n'
exp_check "R4 control: no target has a meld of its own" "$NOHOST" "$ROWS"
exp_check "R1 the bundle ran the application's own code" "$RAN" "$ROWS"
exp_check "R2 host shared objects: rows with zero"       "$CLEAN" "$ROWS"
exp_check "R3 libpython loaded FROM THE BUNDLE"          "$PY" "$ROWS"
exp_check "R3 libgtk-3 loaded FROM THE BUNDLE"           "$GTK" "$ROWS"

exp_note "⭐ WHAT THIS ESTABLISHES: a Python 3 + GTK 3 application out of a"
exp_note "   nixpkgs closure starts and runs its own code on $RAN of $ROWS"
exp_note "   environments that have no meld, no matching Python and no GTK,"
exp_note "   loading both stacks from inside the artefact."
exp_note "⛔ WHAT IT DOES NOT ESTABLISH, and the sentence has to carry it:"
exp_note "   NOT that a window appeared — there is no display and no GPU on"
exp_note "   this machine. experiments/85- owns the offscreen EGL claim;"
exp_note "   T-059 owns real hardware. NOT that every Python or GTK program"
exp_note "   behaves this way: this is ONE application."
exp_note "⚠ The field's \"Utter garbage\" (Python) and \"Garbage\" (GTK) grades"
exp_note "   were earned on ARCH PACKAGES through quick-sharun, not on a nix"
exp_note "   closure. This row re-derives them for OUR pipeline and for one"
exp_note "   subject; it does not overturn their grades for theirs."

exp_finish
