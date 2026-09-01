#!/bin/sh
# fetch-rootfs.sh - materialise the test bed named in rootfs-images.txt.
#
# Every environment an experiment runs against comes from here, at the digest
# pinned in that file, so a result taken today and a result taken next month
# describe the same filesystem.
#
# Usage:
#   sh scripts/common/fetch-rootfs.sh                 all rows
#   sh scripts/common/fetch-rootfs.sh alpine-3.22 debian-12
#   sh scripts/common/fetch-rootfs.sh --list          what is pinned, and what is on disk
#   sh scripts/common/fetch-rootfs.sh --arch arm64    re-resolve by TAG, not digest
#
# Exit codes: 0 every requested row is on disk, 1 one or more failed, 2 could
# not run.

set -u

HERE=$(cd "$(dirname "$0")" && pwd)
LIST="$HERE/rootfs-images.txt"
DEST="${PGB_ROOTFS_DIR:-/var/lib/pgb-rootfs}"
ARCH=""
ONLY_LIST=0
WANT=""

while [ $# -gt 0 ]; do
  case "$1" in
    --list) ONLY_LIST=1 ;;
    --arch) shift; ARCH="${1:-}" ;;
    --dest) shift; DEST="${1:-$DEST}" ;;
    -h|--help) awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"; exit 0 ;;
    -*) printf 'fetch-rootfs: unknown argument: %s\n' "$1" >&2; exit 2 ;;
    *)  WANT="$WANT $1" ;;
  esac
  shift
done

[ -f "$LIST" ] || { printf 'fetch-rootfs: %s is missing\n' "$LIST" >&2; exit 2; }

if [ "$ONLY_LIST" = 1 ]; then
  printf '%-34s %-20s %-6s %s\n' REFERENCE NAME LIBC 'ON DISK'
  while read -r ref name libc digest; do
    case "$ref" in ''|\#*) continue ;; esac
    if [ -d "$DEST/$name" ]; then
      have=$(sed -n 's/^manifest digest: *//p' "$DEST/$name/.oci-provenance" 2>/dev/null)
      if [ "$have" = "$digest" ]; then st="yes (pinned digest)"
      elif [ -n "$have" ]; then st="yes (DIGEST DIFFERS: $have)"
      else st="yes (no provenance)"; fi
    else
      st=no
    fi
    printf '%-34s %-20s %-6s %s\n' "$ref" "$name" "$libc" "$st"
  done < "$LIST"
  exit 0
fi

want_this() {
  [ -z "$WANT" ] && return 0
  for w in $WANT; do [ "$w" = "$1" ] && return 0; done
  return 1
}

rc=0
matched=0
while read -r ref name libc digest; do
  case "$ref" in ''|\#*) continue ;; esac
  want_this "$name" || continue
  matched=$((matched+1))
  if [ -n "$ARCH" ]; then
    # ⚠ A NON-NATIVE ARCH TRADES THE PIN AWAY. The digests in rootfs-images.txt
    # are amd64 manifests; resolving another architecture means going back
    # through the tag, which is not the same input. Said out loud rather than
    # done quietly, because a result taken here is not comparable to one taken
    # against the pinned row.
    printf '== %s (%s) arch=%s: resolving by TAG, the pinned digest does not apply\n' "$name" "$ref" "$ARCH"
    sh "$HERE/oci-pull.sh" "$ref" --arch "$ARCH" --out "$DEST/$name" || rc=1
  else
    printf '== %s (%s) %s\n' "$name" "$ref" "$libc"
    sh "$HERE/oci-pull.sh" "$ref" --digest "$digest" --out "$DEST/$name" || rc=1
  fi
done < "$LIST"

if [ "$matched" = 0 ]; then
  printf 'fetch-rootfs: nothing in %s matched:%s\n' "$LIST" "$WANT" >&2
  exit 2
fi
exit $rc
