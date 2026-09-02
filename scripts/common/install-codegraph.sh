#!/bin/sh
# install-codegraph.sh - put `codegraph` on PATH and index this repository.
#
# codegraph is a pre-indexed code knowledge graph. `TODO/RULES.md` §Reading
# existing code makes it the FIRST instrument an agent reaches for and grep the
# second; this script is how a fresh machine gets it, because the container this
# project runs in is ephemeral and comes up without it.
#
# The release is a self-contained bundle: its own Node runtime, a Rust kernel as
# a native module, and the JavaScript around them. Nothing is compiled here and
# no package manager is involved.
#
# The tarball is pinned by version AND by sha256, and the digest is checked
# before anything is unpacked. An unpinned installer that pipes a URL into a
# shell is what this project's fetch rules exist to avoid.
#
# Usage:
#   sh scripts/common/install-codegraph.sh            install, then index
#   sh scripts/common/install-codegraph.sh --check    report, change nothing
#   sh scripts/common/install-codegraph.sh --no-index install only
#
# Exit: 0 ready, 1 the install ran and failed, 2 it could not run here.
# SPDX-License-Identifier: MIT
set -u

VERSION="1.6.0"
PREFIX="${CODEGRAPH_PREFIX:-/opt}"
BINDIR="${CODEGRAPH_BINDIR:-/usr/local/bin}"

# sha256 of each linux asset of v1.6.0, transcribed from the release's own
# SHA256SUMS. Re-fetch that file to bump the pin; do not weaken the check.
SHA_X64="de3391f79ed42622d937e6cd5b7642a7ea8bb7d1473607e80b879ba73ef216b0"
SHA_ARM64="6dc935a7b8f1a61e688a578b98ea34680eb2e36d7b91db079d64f4011f1a668f"

R=$(cd "$(dirname "$0")/../.." && pwd) || exit 2
say() { printf '  %-6s %s\n' "$1" "$2"; }

check_only=0
do_index=1
for a in "$@"; do
  case "$a" in
    --check) check_only=1 ;;
    --no-index) do_index=0 ;;
    *) printf 'install-codegraph.sh: unknown option %s\n' "$a" >&2; exit 2 ;;
  esac
done

case "$(uname -s)" in
  Linux) ;;
  *) say SKIP "this script installs the Linux build only"; exit 2 ;;
esac

case "$(uname -m)" in
  x86_64|amd64) asset="codegraph-linux-x64.tar.gz"; want="$SHA_X64"; dir="codegraph-linux-x64" ;;
  aarch64|arm64) asset="codegraph-linux-arm64.tar.gz"; want="$SHA_ARM64"; dir="codegraph-linux-arm64" ;;
  *) say SKIP "no pinned codegraph build for $(uname -m)"; exit 2 ;;
esac

have=""
command -v codegraph >/dev/null 2>&1 && have=$(codegraph version 2>/dev/null | tr -d '\r\n')

if [ "$check_only" -eq 1 ]; then
  printf 'codegraph: pinned v%s for %s\n' "$VERSION" "$(uname -m)"
  if [ -n "$have" ]; then say ok "installed: v$have"; else say -- "not installed"; fi
  if [ -d "$R/.codegraph" ]; then say ok "indexed: $R/.codegraph"; else say -- "not indexed"; fi
  [ -n "$have" ] && [ -d "$R/.codegraph" ] && exit 0
  exit 1
fi

if [ "$have" = "$VERSION" ]; then
  say ok "codegraph v$VERSION already installed"
else
  command -v curl >/dev/null 2>&1 || { say SKIP "curl is absent"; exit 2; }
  command -v sha256sum >/dev/null 2>&1 || { say SKIP "sha256sum is absent"; exit 2; }
  command -v tar >/dev/null 2>&1 || { say SKIP "tar is absent"; exit 2; }

  base="https://github.com/colbymchenry/codegraph/releases/download/v$VERSION"
  tmp=$(mktemp -d) || exit 2
  trap 'rm -rf "$tmp"' EXIT INT TERM

  # RULES.md §Fetching: try the origin, and fall back to the reverse proxy the
  # moment it refuses. The whole original URL, scheme included, is the path.
  if ! curl -fsSL -o "$tmp/$asset" "$base/$asset" 2>/dev/null; then
    say '--' "the origin refused; retrying through api.rv.pkgforge.dev"
    curl -fsSL -o "$tmp/$asset" "https://api.rv.pkgforge.dev/$base/$asset" 2>/dev/null ||
      { say SKIP "no route to $asset"; exit 2; }
  fi

  got=$(sha256sum "$tmp/$asset" | cut -d' ' -f1)
  if [ "$got" != "$want" ]; then
    say FAIL "sha256 mismatch: want $want, got $got"
    exit 1
  fi
  say ok "sha256 $got"

  tar xzf "$tmp/$asset" -C "$tmp" || { say FAIL "the tarball did not unpack"; exit 1; }
  rm -rf "$PREFIX/codegraph-$VERSION"
  mkdir -p "$PREFIX" || { say FAIL "cannot write $PREFIX"; exit 1; }
  mv "$tmp/$dir" "$PREFIX/codegraph-$VERSION" || { say FAIL "cannot install into $PREFIX"; exit 1; }
  ln -sfn "$PREFIX/codegraph-$VERSION" "$PREFIX/codegraph"
  mkdir -p "$BINDIR" || { say FAIL "cannot write $BINDIR"; exit 1; }
  ln -sf "$PREFIX/codegraph/bin/codegraph" "$BINDIR/codegraph"

  command -v codegraph >/dev/null 2>&1 || { say FAIL "$BINDIR is not on PATH"; exit 1; }
  say ok "codegraph v$(codegraph version 2>/dev/null | tr -d '\r\n') at $BINDIR/codegraph"
fi

# This repository opts out of the anonymous usage stats the tool sends by
# default. It is a research tree measuring other people's software; it does not
# report on itself to a third party.
codegraph telemetry off >/dev/null 2>&1 || true

[ "$do_index" -eq 0 ] && exit 0

cd "$R" || exit 2
if [ -d "$R/.codegraph" ]; then
  codegraph sync >/dev/null 2>&1 || { say FAIL "codegraph sync"; exit 1; }
  say ok "graph synced"
else
  codegraph init >/dev/null 2>&1 || { say FAIL "codegraph init"; exit 1; }
  say ok "graph built"
fi

codegraph status 2>/dev/null | sed -n 's/^  \(Files\|Nodes\|Edges\):.*/  &/p' | sed 's/^ *//'
exit 0
