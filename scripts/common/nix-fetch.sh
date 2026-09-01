#!/bin/sh
# nix-fetch.sh - resolve and fetch nixpkgs store paths with NO nix installed.
#
# ⭐ WHAT QUESTION THIS ANSWERS. `docs/design/nix-front-end.md` records an
# operator ruling that nixpkgs is the dependency planner this project should
# use instead of writing one, and then asks the question the ruling turns on:
#
#     "Does taking the nixpkgs graph make pgb depend on nix at RUN time?"
#
# ⛔ THIS SCRIPT IS THE MEASUREMENT, NOT AN OPINION ABOUT IT. It reaches
# nixpkgs through three plain HTTPS endpoints and nothing else:
#
#   1. https://channels.nixos.org/<channel>/store-paths.xz
#      ⭐ A 302 to a release URL that NAMES THE REVISION, so following the
#      redirect once gives a pin, and the body is the complete list of store
#      paths that channel built -- 308,271 lines for nixpkgs-unstable on
#      2026-09-01. That is the index that turns a package NAME into a store
#      path without evaluating a single nix expression.
#   2. https://cache.nixos.org/<hash>.narinfo
#      the metadata: NAR url, compression, hashes, references, signature.
#   3. https://cache.nixos.org/<the narinfo's URL>
#      the NAR itself.
#
# ⛔ AND WHAT IT DOES NOT ANSWER, said here rather than in a footnote: this
# fetches what a channel ALREADY BUILT. It does not evaluate nix expressions,
# so it cannot reach an attribute nobody has built, cannot apply an overlay,
# and cannot build from source. `docs/research/nix.md` has the consequence for
# the front-end design; the short version is that the cache serves the
# common case and `nix` itself is still needed for anything else.
#
# ⚠ EVERY FETCH IS VERIFIED, and refusing is the point:
#   - the narinfo's ed25519 signature against the pinned cache.nixos.org key
#   - the NAR's sha256 against the NarHash that signature covers
# A run that cannot verify exits non-zero with the reason. See nix-nar.py.
#
# Usage:
#   sh nix-fetch.sh channel [--channel nixpkgs-unstable]
#   sh nix-fetch.sh resolve REGEX [--channel C] [--limit N]
#   sh nix-fetch.sh info    STOREPATH-OR-HASH
#   sh nix-fetch.sh closure STOREPATH            # the transitive references
#   sh nix-fetch.sh fetch   STOREPATH --out DIR [--no-closure]
#   sh nix-fetch.sh --selftest
#
# Environment:
#   NIX_FETCH_CACHE   where the index and narinfos are kept
#                     (default /var/tmp/pgb-nix-cache)
#   NIX_FETCH_SUBSTITUTER   default https://cache.nixos.org
#
# Exit codes: 0 did what was asked, 1 did not, 2 could not run.

set -u

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
NAR_PY="$HERE/nix-nar.py"
CACHE="${NIX_FETCH_CACHE:-/var/tmp/pgb-nix-cache}"
SUBST="${NIX_FETCH_SUBSTITUTER:-https://cache.nixos.org}"
CHANNEL="nixpkgs-unstable"
LIMIT=40
OUT=""
CLOSURE=1
CMD=""
ARG=""

die()  { printf 'nix-fetch: %s\n' "$1" >&2; exit "${2:-1}"; }
note() { printf '%s\n' "$1" >&2; }

command -v curl >/dev/null 2>&1 || die "curl not found" 2
command -v python3 >/dev/null 2>&1 || die "python3 not found" 2
[ -f "$NAR_PY" ] || die "nix-nar.py is missing beside this script" 2

while [ $# -gt 0 ]; do
  case "$1" in
    --channel)     shift; CHANNEL="${1:-$CHANNEL}" ;;
    --limit)       shift; LIMIT="${1:-$LIMIT}" ;;
    --out)         shift; OUT="${1:-}" ;;
    --no-closure)  CLOSURE=0 ;;
    --selftest)    CMD=selftest ;;
    -h|--help)     awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"; exit 0 ;;
    -*)            die "unknown argument: $1" 2 ;;
    *)             if [ -z "$CMD" ]; then CMD="$1"; else ARG="$1"; fi ;;
  esac
  shift
done

mkdir -p "$CACHE" || die "cannot create $CACHE" 2

# -- the channel, and the pin hiding in its redirect -------------------------
#
# ⭐ THE REDIRECT IS THE PIN. `channels.nixos.org/<c>/store-paths.xz` 302s to
# `releases.nixos.org/nixpkgs/nixpkgs-<version>.<rev>/store-paths.xz`, and that
# version string is exactly what a later session needs to fetch the SAME index
# again. Resolving it and writing it down costs one HEAD request and is the
# difference between a reproducible fetch and "whatever the channel said today".
channel_release_url() {
  curl -sSI -o /dev/null -w '%{redirect_url}' \
    "https://channels.nixos.org/$CHANNEL/store-paths.xz"
}

index_file() {
  _idx="$CACHE/store-paths.$CHANNEL.txt"
  _pin="$CACHE/store-paths.$CHANNEL.pin"
  if [ ! -s "$_idx" ]; then
    _url=$(channel_release_url)
    [ -n "$_url" ] || die "the channel did not redirect: is $CHANNEL a channel name?"
    printf '%s\n' "$_url" > "$_pin"
    note "nix-fetch: index <- $_url"
    curl -sSL "$_url" | xz -dc > "$_idx.part" || die "could not fetch the index"
    mv "$_idx.part" "$_idx"
  fi
  printf '%s\n' "$_idx"
}

narinfo_path() {   # hash -> cached narinfo file, fetched and VERIFIED
  _h="$1"
  _f="$CACHE/narinfo/$_h.narinfo"
  mkdir -p "$CACHE/narinfo"
  if [ ! -s "$_f" ]; then
    curl -sSf "$SUBST/$_h.narinfo" -o "$_f.part" || {
      rm -f "$_f.part"
      return 1
    }
    # ⛔ VERIFY BEFORE IT IS CACHED, never after. A narinfo that failed its
    # signature must not be left on disk where the next run reads it as
    # already-checked.
    if ! python3 "$NAR_PY" verify-narinfo "$(python3 "$NAR_PY" pubkey)" \
        < "$_f.part" >/dev/null; then
      rm -f "$_f.part"
      return 1
    fi
    mv "$_f.part" "$_f"
  fi
  printf '%s\n' "$_f"
}

hash_of() {   # /nix/store/HASH-name, or HASH-name, or HASH -> HASH
  printf '%s\n' "$1" | sed -e 's|^/nix/store/||' -e 's|^\([a-z0-9]\{32\}\).*|\1|'
}

field() { sed -n "s/^$2: //p" "$1"; }

case "$CMD" in
  channel)
    _url=$(channel_release_url)
    [ -n "$_url" ] || die "no redirect for channel $CHANNEL"
    printf 'channel   %s\n' "$CHANNEL"
    printf 'release   %s\n' "$_url"
    printf 'revision  %s\n' "$(printf '%s' "$_url" | sed -n 's|.*/nixpkgs-\([^/]*\)/.*|\1|p')"
    ;;

  resolve)
    [ -n "$ARG" ] || die "resolve needs a pattern" 2
    _idx=$(index_file) || exit 1
    grep -E -- "$ARG" "$_idx" | head -n "$LIMIT"
    ;;

  info)
    [ -n "$ARG" ] || die "info needs a store path or hash" 2
    _f=$(narinfo_path "$(hash_of "$ARG")") || die "no narinfo for $ARG, or it failed verification"
    cat "$_f"
    ;;

  closure)
    [ -n "$ARG" ] || die "closure needs a store path" 2
    # Breadth-first over References. ⚠ A store path references ITSELF in its
    # own narinfo, which is not a cycle to break but a fact to skip.
    _seen="$CACHE/.closure.$$"
    : > "$_seen"
    _todo=$(hash_of "$ARG")
    while [ -n "$_todo" ]; do
      _next=""
      for _h in $_todo; do
        grep -qx "$_h" "$_seen" && continue
        printf '%s\n' "$_h" >> "$_seen"
        _f=$(narinfo_path "$_h") || { rm -f "$_seen"; die "no narinfo for $_h"; }
        field "$_f" StorePath
        for _r in $(field "$_f" References); do
          _rh=$(hash_of "$_r")
          grep -qx "$_rh" "$_seen" || _next="$_next $_rh"
        done
      done
      _todo="$_next"
    done
    rm -f "$_seen"
    ;;

  fetch)
    [ -n "$ARG" ] || die "fetch needs a store path" 2
    [ -n "$OUT" ] || die "fetch needs --out DIR" 2
    mkdir -p "$OUT" || die "cannot create $OUT"
    if [ "$CLOSURE" = 1 ]; then
      _paths=$(sh "$0" closure "$ARG" --channel "$CHANNEL") || exit 1
    else
      _f=$(narinfo_path "$(hash_of "$ARG")") || die "no narinfo for $ARG"
      _paths=$(field "$_f" StorePath)
    fi
    _n=0
    for _p in $_paths; do
      _h=$(hash_of "$_p")
      _base=$(printf '%s' "$_p" | sed 's|^/nix/store/||')
      _dest="$OUT/$_base"
      [ -e "$_dest" ] && continue
      _f=$(narinfo_path "$_h") || die "no narinfo for $_h"
      _url=$(field "$_f" URL)
      _comp=$(field "$_f" Compression)
      _want=$(field "$_f" NarHash)
      # ⛔ ONE PASS: decompress, hash and extract from the same stream. See
      # nix-nar.py's `unpack`, which also records why this is a subcommand and
      # not a heredoc.
      if ! curl -sSfL "$SUBST/$_url" | python3 "$NAR_PY" unpack "$_comp" "$_want" "$_dest.part"; then
        rm -rf "$_dest.part"
        die "fetch failed for $_p"
      fi
      mv "$_dest.part" "$_dest"
      _n=$((_n + 1))
      printf '%s\n' "$_dest"
    done
    note "nix-fetch: $_n path(s) fetched into $OUT"
    ;;

  selftest)
    # ⛔ OFFLINE. The network parts are exercised by experiments/80-, which is
    # where a result that depends on a third party belongs.
    python3 "$NAR_PY" selftest || die "nix-nar selftest failed"
    _bad=0
    for _in in /nix/store/abcdefghijklmnopqrstuvwxyz012345-foo-1.0 \
               abcdefghijklmnopqrstuvwxyz012345-foo-1.0 \
               abcdefghijklmnopqrstuvwxyz012345; do
      _got=$(hash_of "$_in")
      if [ "$_got" = "abcdefghijklmnopqrstuvwxyz012345" ]; then
        printf '  ok    hash_of %s\n' "$_in"
      else
        printf '  FAIL  hash_of %s -> %s\n' "$_in" "$_got"; _bad=1
      fi
    done
    printf 'nix-fetch --selftest: %s\n' \
      "$([ "$_bad" = 0 ] && echo 'all checks pass.' || echo 'FAILURES above.')"
    exit "$_bad"
    ;;

  ""|*)
    die "unknown command: ${CMD:-<none>}. Try --help." 2
    ;;
esac
