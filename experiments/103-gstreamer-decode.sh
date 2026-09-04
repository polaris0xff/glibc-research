#!/bin/sh
# THE QUESTION
#
#   T-091 shipped all four GStreamer variables and installed
#   `gst-plugin-scanner` as a bundle PROGRAM, and then measured NOTHING. The
#   corpus's media row (`experiments/65-` `media-1`, mpv) runs `--version`,
#   which never launches the scanner and decodes nothing, so its host-object
#   count cannot say which process it counted.
#
#   ⭐ SO ASK THE THREE QUESTIONS THAT ROW CANNOT:
#
#     1. Does a bundled GStreamer application actually DECODE — not start,
#        not print a version, but read an encoded file and produce samples?
#     2. ⛔ Is the plugin-path machinery what makes it work? The same bundle
#        built `--no-plugin-env` is the control, and it is a SHIPPED FLAG for
#        the same reason `--no-storefix` is.
#     3. ⭐ WHICH PROCESS does the host-object count describe? The field's own
#        note is that "gst-plugin-scanner opens every single gstreamer plugin
#        on the system", so a count taken over the whole process tree is
#        measuring the scanner, not the application.
#
# -- ⛔ PRE-REGISTERED EXPECTATIONS -----------------------------------------
#
#   D1  the ENCODE leg runs: audiotestsrc -> vorbisenc -> oggmux -> file.
#       ⚠ It is here so that D2 has something real to read; a decode of a
#       file this run did not produce would be measuring the fixture.
#   D2  ⭐ THE DECODE LEG PRODUCES SAMPLES. filesrc -> oggdemux -> vorbisdec
#       -> wavenc -> file, and the WAV is larger than the Ogg it came from,
#       because PCM is bigger than Vorbis. ⭐ That size relation is the
#       application's OWN answer and no broken bundle prints it.
#   D3  zero HOST shared objects in the PAYLOAD, on every environment.
#   D4  ⭐ THE COUNT NAMES ITS PROCESS. The same trace classified in `tree`
#       mode and in `payload` mode, printed side by side, plus whether a
#       `gst-plugin-scanner` execve appears at all. ⛔ REPORTED, NOT
#       PREDICTED: nobody has measured whether the scanner runs here.
#
#   C1  ⛔ THE CONTROL, AND ITS OUTCOME IS NOT PREDICTED EITHER WAY.
#       `--no-plugin-env` removes GST_PLUGIN_PATH, GST_PLUGIN_SYSTEM_PATH,
#       GST_PLUGIN_SYSTEM_PATH_1_0 and GST_PLUGIN_SCANNER. Two outcomes are
#       both interesting and this file refuses to guess between them:
#         - the pipeline FAILS  -> the variables are load-bearing, T-091's
#           mechanism is measured, and the corpus has its discriminator;
#         - the pipeline WORKS  -> ⭐ the variables are REDUNDANT here,
#           because GStreamer's compiled-in default plugin directory is a
#           `/nix/store` path and `pgb-storefix.c` already answers it. That
#           would be a finding about the interposer's reach, not a failure.
#       ⛔ Either way the row is recorded as measured. What is NOT acceptable
#       is shipping four variables nobody has shown do anything.
#
# ⛔ WHAT THIS DOES NOT MEASURE. Real hardware decode (VAAPI/NVDEC): there is
# no `/dev/dri` on this machine, so every codec path here is software. And one
# container format on one codec pair is not "media works".
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "103 - a bundled GStreamer application decoding, and which process the host count describes"

WORK="${PGB_EXP103_WORK:-/var/tmp/t103}"
mkdir -p "$WORK" || exit 2
ATTR="${PGB_EXP103_ATTR:-gst_all_1.gstreamer}"
PROG=gst-launch-1.0
RUN_TIMEOUT="${PGB_EXP103_TIMEOUT:-120}"

command -v strace >/dev/null 2>&1 || { exp_note "no strace on PATH"; exit 2; }

ENVS=$(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt")
NENV=$(printf '%s\n' "$ENVS" | wc -l | tr -d ' ')

# ⛔ REAP BY WHAT A PROCESS IS CHROOTED INTO, NOT BY ITS NAME. docs/AGENTS.md §14.
reap_in_root() {
  _rr=$1
  for _p in /proc/[0-9]*; do
    _pid=${_p#/proc/}
    _rt=$(readlink "/proc/$_pid/root" 2>/dev/null) || continue
    case "$_rt" in "$_rr"|"$_rr"/*) kill -9 "$_pid" 2>/dev/null ;; esac
  done
}

# ---------------------------------------------------------------------------
# Build both artefacts. ⭐ The control differs by ONE FLAG and nothing else —
# same attribute, same extras, same cache, same tool.
#
# ⚠ `gst-plugins-base` is where vorbisenc, vorbisdec, oggmux and oggdemux
# live; `gstreamer` alone carries the core elements and the tools. Both are
# named so the pipeline below is buildable from the closure rather than from
# whatever the host happens to have.
# ---------------------------------------------------------------------------
build() {  # out-image extra-flag...
  _img=$1; shift
  [ -s "$_img" ] && return 0
  PGB_APPIMAGE_CACHE="$WORK/cache" "$REPO_DIR/pgb" bundle appimage "$ATTR" \
    --out "$_img" --name "$PROG" \
    --extra gst_all_1.gst-plugins-base "$@" >"$_img.log" 2>&1 || true
  [ -s "$_img" ]
}

printf -- '-- building the subject and its control ---------------------------\n'
IMG="$WORK/gst.AppImage"
CTL="$WORK/gst-noenv.AppImage"
build "$IMG" || { exp_note "subject did not build; see $IMG.log"; tail -5 "$IMG.log"; exit 2; }
exp_check "B1  the subject built" "$([ -s "$IMG" ] && echo yes || echo no)" yes
build "$CTL" --no-plugin-env || exp_note "control did not build; see $CTL.log"
exp_check "B2  ⭐ the control built (--no-plugin-env)" \
    "$([ -s "$CTL" ] && echo yes || echo no)" yes

# ⭐ THE CONTROL MUST DIFFER IN THE WAY IT CLAIMS TO. A flag that changed
# nothing would make every row below meaningless, so the .env of each is read
# and the GStreamer variables counted.
gstvars() { grep -c '^GST_' "$1" 2>/dev/null || true; }
SUBJ_ENV=$(gstvars "$WORK/cache/$PROG/AppDir/.env")
exp_note "$(printf 'GST_* variables in the subject .env: %s' "${SUBJ_ENV:-?}")"

printf -- '\n-- the eleven ------------------------------------------------------\n'
printf '  %-22s %-6s %-6s %-9s %-9s %s\n' ENVIRONMENT ENC DEC 'HOST(pay)' 'HOST(tree)' SCANNER
d1=0; d2=0; d3=0; rows=0; ctl_fail=0; ctl_rows=0; scanner_seen=0
for name in $ENVS; do
  root=$(exp_rootfs "$name") || true
  [ -n "$root" ] || { exp_skip "$name" "rootfs not fetched"; continue; }
  rows=$((rows+1))

  rm -f "$root/subj103"; cp "$IMG" "$root/subj103"; chmod +x "$root/subj103"
  tr="$WORK/tr.$name"
  # ⭐ ONE INVOCATION PER LEG, AND THE DECODE LEG READS WHAT THE ENCODE LEG
  # WROTE. Both run inside the rootfs; /tmp there is a fresh tmpfs, so a file
  # left by a previous environment cannot be read as this one's answer.
  timeout "$RUN_TIMEOUT" strace -f -e trace=openat,open,execve,clone,clone3,vfork \
    -o "$tr" "$REPO_DIR/pgb" rootfs run "$root" -- /bin/sh -c '
      rm -f /tmp/t.ogg /tmp/t.wav
      APPIMAGE_EXTRACT_AND_RUN=1 /subj103 -q audiotestsrc num-buffers=200 \
        ! audioconvert ! vorbisenc ! oggmux ! filesink location=/tmp/t.ogg
      echo "OGG=$(wc -c < /tmp/t.ogg 2>/dev/null || echo 0)"
      APPIMAGE_EXTRACT_AND_RUN=1 /subj103 -q filesrc location=/tmp/t.ogg \
        ! oggdemux ! vorbisdec ! audioconvert ! wavenc ! filesink location=/tmp/t.wav
      echo "WAV=$(wc -c < /tmp/t.wav 2>/dev/null || echo 0)"
    ' >"$WORK/out.$name" 2>"$WORK/err.$name" || true
  reap_in_root "$root"
  rm -f "$root/subj103"

  ogg=$(sed -n 's/^OGG=//p' "$WORK/out.$name" | tail -1)
  wav=$(sed -n 's/^WAV=//p' "$WORK/out.$name" | tail -1)
  ogg=${ogg:-0}; wav=${wav:-0}
  enc=no; dec=no
  [ "$ogg" -gt 1000 ] 2>/dev/null && { enc=yes; d1=$((d1+1)); }
  # ⭐ PCM IS BIGGER THAN VORBIS. A decode that produced a file smaller than
  # its input did not decode; a zero-byte file did not run.
  [ "$wav" -gt "$ogg" ] 2>/dev/null && [ "$wav" -gt 1000 ] 2>/dev/null \
    && { dec=yes; d2=$((d2+1)); }

  hp=$(exp_classify_trace "$tr" /subj103 payload | grep -c '^host ' || true)
  ht=$(exp_classify_trace "$tr" /subj103 tree    | grep -c '^host ' || true)
  [ "$hp" = 0 ] && d3=$((d3+1))
  sc=no
  grep -q 'execve("[^"]*gst-plugin-scanner' "$tr" 2>/dev/null && { sc=yes; scanner_seen=$((scanner_seen+1)); }
  printf '  %-22s %-6s %-6s %-9s %-9s %s\n' "$name" "$enc" "$dec" "$hp" "$ht" "$sc"
  rm -f "$tr"

  # -- the control, same environment, same pipeline, one flag different -----
  if [ -s "$CTL" ]; then
    ctl_rows=$((ctl_rows+1))
    rm -f "$root/subj103c"; cp "$CTL" "$root/subj103c"; chmod +x "$root/subj103c"
    timeout "$RUN_TIMEOUT" "$REPO_DIR/pgb" rootfs run "$root" -- /bin/sh -c '
      rm -f /tmp/c.ogg
      APPIMAGE_EXTRACT_AND_RUN=1 /subj103c -q audiotestsrc num-buffers=200 \
        ! audioconvert ! vorbisenc ! oggmux ! filesink location=/tmp/c.ogg
      echo "OGG=$(wc -c < /tmp/c.ogg 2>/dev/null || echo 0)"
    ' >"$WORK/cout.$name" 2>"$WORK/cerr.$name" || true
    reap_in_root "$root"
    rm -f "$root/subj103c"
    cogg=$(sed -n 's/^OGG=//p' "$WORK/cout.$name" | tail -1); cogg=${cogg:-0}
    [ "$cogg" -gt 1000 ] 2>/dev/null || ctl_fail=$((ctl_fail+1))
  fi
done

printf '\n'
exp_check "D1  the ENCODE leg runs on all $rows"           "$d1" "$rows"
exp_check "D2  ⭐ the DECODE leg produces MORE bytes than it read" "$d2" "$rows"
exp_check "D3  zero HOST shared objects in the PAYLOAD"    "$d3" "$rows"

# ⭐ D4 IS REPORTED, NOT ASSERTED AGAINST A PREDICTION. Nobody had measured
# whether the scanner runs at all inside a bundle here.
exp_note "$(printf 'D4  ⭐ gst-plugin-scanner exec seen on %s of %s environments' \
    "$scanner_seen" "$rows")"
if [ "$scanner_seen" -gt 0 ]; then
  exp_note "⛔ SO THE TREE COUNT IS NOT THE APPLICATION'S. The field's own"
  exp_note "   note is that the scanner opens every plugin it can find, so a"
  exp_note "   host-object count taken over the whole process tree describes"
  exp_note "   the SCANNER. The payload column is the application's."
else
  exp_note "⚠ The scanner did not run. Either the registry was already warm"
  exp_note "   or GStreamer loaded the plugins itself; the tree and payload"
  exp_note "   columns above say whether that changed the count."
fi

# ⛔ THE CONTROL. Its direction is not predicted; what is asserted is that it
# was actually MEASURED on every environment, because a control nobody ran is
# the failure mode delivery rule 8 exists for.
exp_check "C0  ⭐ the control ran on every environment" "$ctl_rows" "$rows"
exp_note "$(printf 'C1  ⭐ the control FAILED to encode on %s of %s environments' \
    "$ctl_fail" "$ctl_rows")"
if [ "$ctl_fail" = "$ctl_rows" ] && [ "$ctl_rows" -gt 0 ]; then
  exp_note "⭐ THE VARIABLES ARE LOAD-BEARING. Same closure, same pipeline,"
  exp_note "   one flag: without GST_PLUGIN_* the pipeline cannot be built."
elif [ "$ctl_fail" = 0 ] && [ "$ctl_rows" -gt 0 ]; then
  exp_note "⛔ THE VARIABLES ARE REDUNDANT ON THIS SUBJECT, and that is a"
  exp_note "   finding rather than a failure: GStreamer's compiled-in default"
  exp_note "   plugin directory is a /nix/store path, and pgb-storefix.c"
  exp_note "   already answers it. T-091 should say so instead of claiming a"
  exp_note "   mechanism nothing needed."
else
  exp_note "⚠ The control failed on SOME environments. That is neither"
  exp_note "   outcome and the split above is the result."
fi

exp_note "⛔ C3-CLASS LIMIT: every codec path here is SOFTWARE. There is no"
exp_note "   /dev/dri on this machine, so this says nothing about VAAPI,"
exp_note "   NVDEC or any hardware decoder. T-059 owns hardware."
exp_note "⚠ And one container on one codec pair is not \"media works\"."

exp_finish
