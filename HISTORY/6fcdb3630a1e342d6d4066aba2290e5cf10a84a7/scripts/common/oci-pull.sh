#!/bin/sh
# oci-pull.sh - fetch an OCI/Docker image and unpack it to a root filesystem
# directory, WITHOUT a container daemon.
#
# -- THE QUESTION THIS EXISTS TO ANSWER --------------------------------------
#
# "Does this binary run on Alpine / Debian 11 / CentOS 7?" is a runtime
# question and the only honest way to answer it is to run the binary there.
# This project's development machine has root and CAP_SYS_ADMIN but NO docker
# daemon (`dial unix /var/run/docker.sock: no such file or directory`), so the
# usual `docker run` route is closed.
#
# A registry is a plain HTTPS blob store. A daemon is not required to read one.
# This script does the anonymous-token dance, resolves a tag to a per-platform
# manifest DIGEST, downloads the layer blobs, and untars them in order into a
# directory that scripts/common/rootfs-run.sh can chroot into.
#
# -- REPRODUCIBILITY ---------------------------------------------------------
#
# docs/methodology/experiments.md: "every input pinned ... An experiment
# against `latest` measures a different thing each week and says so nowhere."
#
# So every pull writes <out>/.oci-provenance describing exactly what landed:
# the reference asked for, the index digest, the resolved per-platform manifest
# digest, the layer digests, and the date. Re-running with
# --digest <manifest-digest> reproduces that exact filesystem, and the pinned
# digests for every image this project uses live in
# scripts/common/rootfs-images.txt.
#
# -- WHITEOUTS ---------------------------------------------------------------
#
# Multi-layer images delete files from lower layers with `.wh.NAME` marker
# entries and clear a whole directory with `.wh..wh..opq`. Extracting layers
# with a plain sequential `tar -x` and no whiteout pass silently RESURRECTS
# deleted files, which for a libc test bed can mean a stale loader or a
# libnss_*.so that the image does not actually ship. Layers are therefore
# unpacked one at a time into a staging directory and merged with the markers
# honoured.
#
# Usage:
#   "./pgb" rootfs pull alpine:3.20 --out /var/rootfs/alpine-3.20
#   "./pgb" rootfs pull debian:11 --out DIR --arch arm64
#   "./pgb" rootfs pull alpine:3.20 --out DIR --digest sha256:c64c...
#   "./pgb" rootfs pull --selftest        prove the whiteout pass, offline
#
# Exit codes: 0 unpacked, 1 the fetch or unpack failed, 2 could not run.
#
# Reads only. No credentials of any kind are sent: the token endpoint is asked
# for an anonymous pull scope and nothing else.

set -u

REF=""
OUT=""
ARCH=""
PIN=""
KEEP=0
SELFTEST=0
QUIET=0

usage() { awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"; }

while [ $# -gt 0 ]; do
  case "$1" in
    --out)      shift; OUT="${1:-}" ;;
    --arch)     shift; ARCH="${1:-}" ;;
    --digest)   shift; PIN="${1:-}" ;;
    --keep-blobs) KEEP=1 ;;
    --quiet)    QUIET=1 ;;
    --selftest) SELFTEST=1 ;;
    -h|--help)  usage; exit 0 ;;
    -*)         printf 'oci-pull: unknown argument: %s\n' "$1" >&2; exit 2 ;;
    *)          REF="$1" ;;
  esac
  shift
done

say() { [ "$QUIET" = 1 ] || printf '%s\n' "$*"; }
die() { printf 'oci-pull: %s\n' "$*" >&2; exit "${2:-1}"; }

# ---------------------------------------------------------------------------
# The whiteout merge, factored out so --selftest can drive it with no network.
#
# Merges $1 (a freshly unpacked layer) into $2 (the accumulating rootfs),
# honouring the two OCI marker conventions. Deletions are applied BEFORE the
# copy so a layer that both deletes and re-adds a path ends with the re-add.
# ---------------------------------------------------------------------------
merge_layer() {
  _ml_src="$1"; _ml_dst="$2"

  # .wh..wh..opq in a directory means: everything the lower layers put in this
  # directory is gone, even entries this layer does not mention.
  find "$_ml_src" -name '.wh..wh..opq' 2>/dev/null | while read -r opq; do
    _d=$(dirname "$opq")
    _rel=${_d#"$_ml_src"}
    _rel=${_rel#/}
    [ -n "$_rel" ] || continue
    [ -d "$_ml_dst/$_rel" ] && rm -rf -- "${_ml_dst:?}/$_rel"
    rm -f -- "$opq"
  done

  # .wh.NAME means: NAME is deleted.
  find "$_ml_src" -name '.wh.*' ! -name '.wh..wh..opq' 2>/dev/null | while read -r wh; do
    _d=$(dirname "$wh"); _b=$(basename "$wh")
    _target=${_b#.wh.}
    _rel=${_d#"$_ml_src"}
    _rel=${_rel#/}
    if [ -n "$_rel" ]; then
      rm -rf -- "${_ml_dst:?}/$_rel/$_target"
    else
      rm -rf -- "${_ml_dst:?}/$_target"
    fi
    rm -f -- "$wh"
  done

  # cp -a would fail to overwrite a directory with a file and vice versa; tar
  # through a pipe has the right replace semantics and preserves everything.
  ( cd "$_ml_src" && tar -cf - . ) | ( cd "$_ml_dst" && tar -xpf - --no-same-owner 2>/dev/null || tar -xpf - ) || return 1
  return 0
}

if [ "$SELFTEST" = 1 ]; then
  # ⚠ Probe the whiteout pass by RUNNING it against a fixture, because the
  # failure it guards is silent: without the pass the run still exits 0 and
  # still produces a rootfs, just one carrying files the image deleted.
  T=$(mktemp -d) || exit 2
  fails=0
  chk() { if [ "$2" = "$3" ]; then printf '  ok    %s = %s\n' "$1" "$2"; else printf '  FAIL  %s = %s, wanted %s\n' "$1" "$2" "$3"; fails=$((fails+1)); fi; }

  mkdir -p "$T/dst/usr/lib" "$T/dst/etc" "$T/dst/keep"
  : > "$T/dst/usr/lib/libnss_stale.so.2"
  : > "$T/dst/etc/nsswitch.conf"
  : > "$T/dst/keep/file"

  mkdir -p "$T/src/usr/lib" "$T/src/etc"
  : > "$T/src/usr/lib/.wh.libnss_stale.so.2"     # delete one file
  : > "$T/src/etc/.wh..wh..opq"                  # clear a whole directory
  : > "$T/src/etc/resolv.conf"                   # ... then re-add into it
  : > "$T/src/new"

  merge_layer "$T/src" "$T/dst"

  chk deleted-file      "$([ -e "$T/dst/usr/lib/libnss_stale.so.2" ] && echo present || echo gone)" gone
  chk opaque-dir-wiped  "$([ -e "$T/dst/etc/nsswitch.conf" ] && echo present || echo gone)" gone
  chk readd-after-opq   "$([ -e "$T/dst/etc/resolv.conf" ] && echo present || echo gone)" present
  chk untouched-kept    "$([ -e "$T/dst/keep/file" ] && echo present || echo gone)" present
  chk new-file-added    "$([ -e "$T/dst/new" ] && echo present || echo gone)" present
  chk markers-not-kept  "$(find "$T/dst" -name '.wh.*' | wc -l | tr -d ' ')" 0

  rm -rf "$T"
  if [ "$fails" = 0 ]; then
    printf 'oci-pull --selftest: 6 cases, all pass.\n'; exit 0
  fi
  printf 'oci-pull --selftest: %s of 6 cases FAILED.\n' "$fails"; exit 1
fi

[ -n "$REF" ] || { usage >&2; exit 2; }
[ -n "$OUT" ] || die "--out DIR is required" 2
command -v curl >/dev/null 2>&1 || die "curl is required" 2
python3 -c '' >/dev/null 2>&1 || die "python3 is required (JSON is parsed, never grepped)" 2

if [ -z "$ARCH" ]; then
  case "$(uname -m)" in
    x86_64|amd64)  ARCH=amd64 ;;
    aarch64|arm64) ARCH=arm64 ;;
    *)             ARCH=$(uname -m) ;;
  esac
fi

# Split REF into registry / repository / tag.
case "$REF" in
  */*/*) REGISTRY=${REF%%/*}; REST=${REF#*/} ;;
  */*)   case "${REF%%/*}" in
           *.*|*:*|localhost) REGISTRY=${REF%%/*}; REST=${REF#*/} ;;
           *)                 REGISTRY=registry-1.docker.io; REST=$REF ;;
         esac ;;
  *)     REGISTRY=registry-1.docker.io; REST="library/$REF" ;;
esac
case "$REST" in
  *:*) REPO=${REST%:*}; TAG=${REST##*:} ;;
  *)   REPO=$REST; TAG=latest ;;
esac
[ "$REGISTRY" = docker.io ] && REGISTRY=registry-1.docker.io

ACCEPT='application/vnd.oci.image.index.v1+json,application/vnd.docker.distribution.manifest.list.v2+json,application/vnd.oci.image.manifest.v1+json,application/vnd.docker.distribution.manifest.v2+json'

# Anonymous pull token. Docker Hub and ghcr.io both answer this shape; a
# registry that needs nothing simply gets an empty header.
AUTH=""
case "$REGISTRY" in
  registry-1.docker.io)
    T=$(curl -sSf "https://auth.docker.io/token?service=registry.docker.io&scope=repository:${REPO}:pull" 2>/dev/null \
        | python3 -c 'import json,sys; print(json.load(sys.stdin).get("token",""))' 2>/dev/null) || T=""
    [ -n "$T" ] && AUTH="Authorization: Bearer $T" ;;
  ghcr.io)
    T=$(curl -sSf "https://ghcr.io/token?service=ghcr.io&scope=repository:${REPO}:pull" 2>/dev/null \
        | python3 -c 'import json,sys; print(json.load(sys.stdin).get("token",""))' 2>/dev/null) || T=""
    [ -n "$T" ] && AUTH="Authorization: Bearer $T" ;;
  quay.io)
    T=$(curl -sSf "https://quay.io/v2/auth?service=quay.io&scope=repository:${REPO}:pull" 2>/dev/null \
        | python3 -c 'import json,sys; print(json.load(sys.stdin).get("token",""))' 2>/dev/null) || T=""
    [ -n "$T" ] && AUTH="Authorization: Bearer $T" ;;
esac

reg_get() {  # path outfile
  if [ -n "$AUTH" ]; then
    curl -sSfL --retry 3 --retry-delay 2 -H "$AUTH" -H "Accept: $ACCEPT" "https://$REGISTRY/v2/$1" -o "$2"
  else
    curl -sSfL --retry 3 --retry-delay 2 -H "Accept: $ACCEPT" "https://$REGISTRY/v2/$1" -o "$2"
  fi
}

WORK=$(mktemp -d) || exit 2
cleanup() { [ "$KEEP" = 1 ] || rm -rf "$WORK"; }
trap cleanup EXIT INT TERM

say "oci-pull: $REGISTRY/$REPO:$TAG  arch=$ARCH"

if [ -n "$PIN" ]; then
  MANIFEST_DIGEST="$PIN"
  INDEX_DIGEST="(pinned, index not consulted)"
else
  reg_get "$REPO/manifests/$TAG" "$WORK/index.json" || die "cannot fetch manifest for $REPO:$TAG"
  INDEX_DIGEST=$(python3 - "$WORK/index.json" <<'PY'
import hashlib,sys
print("sha256:"+hashlib.sha256(open(sys.argv[1],'rb').read()).hexdigest())
PY
)
  # An index needs one more hop to the per-platform manifest; a single-platform
  # manifest is already the thing we want.
  MANIFEST_DIGEST=$(python3 - "$WORK/index.json" "$ARCH" <<'PY'
import json,sys
d=json.load(open(sys.argv[1])); arch=sys.argv[2]
if "manifests" in d:
    for m in d["manifests"]:
        p=m.get("platform") or {}
        if p.get("architecture")==arch and p.get("os")=="linux" and "variant" not in p:
            print(m["digest"]); sys.exit(0)
    for m in d["manifests"]:
        p=m.get("platform") or {}
        if p.get("architecture")==arch and p.get("os")=="linux":
            print(m["digest"]); sys.exit(0)
    sys.exit(3)
print("")            # already a per-platform manifest
PY
) || die "no linux/$ARCH manifest in $REPO:$TAG"
fi

if [ -n "$MANIFEST_DIGEST" ]; then
  reg_get "$REPO/manifests/$MANIFEST_DIGEST" "$WORK/manifest.json" || die "cannot fetch $MANIFEST_DIGEST"
else
  cp "$WORK/index.json" "$WORK/manifest.json"
  MANIFEST_DIGEST="$INDEX_DIGEST"
fi

LAYERS=$(python3 - "$WORK/manifest.json" <<'PY'
import json,sys
d=json.load(open(sys.argv[1]))
for l in d.get("layers", []):
    print(l["digest"], l.get("mediaType",""))
PY
) || die "cannot read layers"
[ -n "$LAYERS" ] || die "manifest lists no layers"

mkdir -p "$OUT" || die "cannot create $OUT" 2
# A rootfs is replaced wholesale, never merged onto a previous run: a leftover
# file from an earlier image is exactly the host contamination this bed exists
# to rule out.
if [ -n "$(ls -A "$OUT" 2>/dev/null)" ]; then
  say "oci-pull: $OUT is not empty, clearing it"
  rm -rf -- "${OUT:?}"/* "${OUT:?}"/.[!.]* 2>/dev/null || true
fi

n=0
printf '%s\n' "$LAYERS" | while read -r dg mt; do
  [ -n "$dg" ] || continue
  n=$((n+1))
  say "  layer $n: $dg"
  reg_get "$REPO/blobs/$dg" "$WORK/layer.tar.gz" || exit 1
  rm -rf "$WORK/stage"; mkdir -p "$WORK/stage"
  case "$mt" in
    *zstd*) tar --zstd -xpf "$WORK/layer.tar.gz" -C "$WORK/stage" 2>/dev/null || exit 1 ;;
    *)      tar -xpf "$WORK/layer.tar.gz" -C "$WORK/stage" 2>/dev/null || exit 1 ;;
  esac
  merge_layer "$WORK/stage" "$OUT" || exit 1
done || die "layer unpack failed"

{
  printf '# oci-pull provenance\n\n'
  printf 'reference:        %s\n' "$REF"
  printf 'registry:         %s\n' "$REGISTRY"
  printf 'repository:       %s\n' "$REPO"
  printf 'tag:              %s\n' "$TAG"
  printf 'architecture:     %s\n' "$ARCH"
  printf 'index digest:     %s\n' "$INDEX_DIGEST"
  printf 'manifest digest:  %s\n' "$MANIFEST_DIGEST"
  printf 'pulled (UTC):     %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf '\nlayers:\n'
  printf '%s\n' "$LAYERS" | sed 's/^/  /'
  printf '\nreproduce exactly:\n'
  printf '  "./pgb" rootfs pull %s --arch %s --digest %s --out DIR\n' "$REF" "$ARCH" "$MANIFEST_DIGEST"
} > "$OUT/.oci-provenance"

say "oci-pull: unpacked to $OUT  (manifest $MANIFEST_DIGEST)"
exit 0
