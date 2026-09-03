#!/bin/sh
# THE QUESTION
#
#   A static `nix` binary is published. Is it glibc or musl, and does it run on
#   the eleven?
#
# ⛔ WHY THIS EXISTS. `TODO/toolchain.md` T-051 recorded, measured on
# 2026-09-03c: *"nixpkgs ships no static nix, so step 2 cannot begin with a
# fetch."* That is true of **nixpkgs** and it answers the wrong question — the
# **nix flake** publishes one, and three separate projects consume it
# (`docs/research/portable-nix.md` finding 1). This experiment settles what
# that binary actually is, because T-060's whole framing is *"pkgsStatic is
# musl and this project is the glibc half"*, and if the published one were
# glibc that framing would be wrong.
#
# -- ⛔ THE FALSE POSITIVE THAT NEARLY DECIDED IT THE WRONG WAY ---------------
#
# The first probe was `strings nix | grep -c 'GNU C Library'`, which returns
# **1**, and one is not zero. Read as a libc test it says glibc. It is not a
# libc test: the hit is inside a LICENCE SENTENCE —
#
#     > component like the GNU C Library).
#
# ⭐ The discriminator that actually works is the nixpkgs STORE PATH compiled
# into the binary, which carries the target triple:
#
#     /nix/store/…-nix-static-x86_64-unknown-linux-MUSL-2.35.2/bin
#
# ⚠ So this experiment asserts the triple, and asserts the licence-sentence
# trap explicitly so nobody re-derives it.
#
# -- WHAT IS MEASURED --------------------------------------------------------
#
#   1. the artefact is genuinely static: PT_INTERP 0, DT_NEEDED 0
#   2. its target triple, from a compiled-in store path
#   3. ⛔ the "GNU C Library" string is present AND means nothing
#   4. `nix --version` on every fetched environment
#
# ⚠ NETWORK. It fetches one release asset and verifies it against a pinned
# SHA-512. No network, or a digest that disagrees, is exit 2 — could-not-run —
# and never a silent pass.
#
# Exit: 0 measured and matched, 1 measured and did not, 2 could not run.
set -u
. "$(cd "$(dirname "$0")" && pwd)/lib.sh"

exp_begin "98 - the published static nix: which libc, and does it run on the eleven?"

WORK="${PGB_EXP98_WORK:-/var/tmp/pgb-exp98}"
rm -rf "$WORK"; mkdir -p "$WORK" || exit 2

# ⛔ PINNED. `methodology/experiments.md`: an experiment against `latest`
# measures a different thing each week and says so nowhere.
NIX_VER=2.35.2
NIX_URL="https://github.com/containerbase/nix-prebuild/releases/download/${NIX_VER}/nix-${NIX_VER}-x86_64.tar.xz"
NIX_SHA=872019c96b60e52d061588a0dddbc34dced56d9c45064252bd4464b54b96d0197e02410ee52dfba0c62e4b4d1725dccff94e259dc3f255bb2b4a7463f868a300

exp_conditions

command -v curl >/dev/null 2>&1 || { exp_note "no curl"; exit 2; }
if ! curl -sSL --fail -o "$WORK/nix.tar.xz" "$NIX_URL" 2>"$WORK/curl.log"; then
  exp_note "could not fetch $NIX_URL: $(tail -1 "$WORK/curl.log")"
  exit 2
fi
got=$(sha512sum "$WORK/nix.tar.xz" | awk '{print $1}')
if [ "$got" != "$NIX_SHA" ]; then
  exp_note "digest disagrees; the pin is stale or the fetch was tampered with"
  exp_note "  want $NIX_SHA"
  exp_note "  got  $got"
  exit 2
fi
tar -xf "$WORK/nix.tar.xz" -C "$WORK" || exit 2
NIX=$(find "$WORK" -type f -name nix -perm -u+x | head -1)
[ -n "$NIX" ] || { exp_note "no nix binary in the tarball"; exit 2; }
exp_note "subject: $NIX ($(wc -c < "$NIX") bytes), from $NIX_URL"

# -- 1. is it actually static? -----------------------------------------------
exp_check "the published nix has no PT_INTERP" \
  "$(readelf -lW "$NIX" 2>/dev/null | grep -c INTERP)" 0
exp_check "...and no DT_NEEDED" \
  "$(readelf -dW "$NIX" 2>/dev/null | grep -c NEEDED)" 0

# -- 2. which libc, from the target triple -----------------------------------
#
# ⭐ THE STORE PATH IS THE EVIDENCE. nixpkgs writes the target triple into the
# output name, and the binary carries its own path.
triple=$(strings -a "$NIX" \
         | grep -oE 'nix-static-[a-z0-9_]+-unknown-linux-[a-z0-9]+' \
         | head -1)
exp_note "compiled-in output name: ${triple:-<none>}"
case "$triple" in
  *-linux-musl) libc=musl ;;
  *-linux-gnu)  libc=glibc ;;
  *)            libc=unknown ;;
esac
exp_check "the published static nix is built against" "$libc" musl
exp_note "⛔ so T-060's framing holds: the published one is the MUSL half, and"
exp_note "   a static-GLIBC nix is still something this project has to build."
exp_note "⭐ and T-051's is answered: this binary is enough nix for a minimal"
exp_note "   host, and it is one fetch."

# -- 3. ⛔ the trap, asserted so it is not re-derived -------------------------
exp_check "'GNU C Library' appears in the binary" \
  "$(strings -a "$NIX" | grep -c 'GNU C Library')" 1
exp_check "...and it is a LICENCE SENTENCE, not a libc" \
  "$(strings -a "$NIX" | grep -c 'component like the GNU C Library')" 1
exp_note '⛔ `strings | grep "GNU C Library"` IS NOT A LIBC TEST. It said'
exp_note "   glibc here and the triple says musl."

# -- 4. does it run on the eleven? -------------------------------------------
printf '\n'
printf -- '-- does the published static nix run where it is put? --------------\n'
printf '  %-22s %-6s %s\n' ENVIRONMENT LIBC "nix --version"
RAN=0; BAD=0; ROWS=0
for name in $(awk '!/^#/ && NF {print $2}' "$REPO_DIR/scripts/common/rootfs-images.txt"); do
  r=$(exp_rootfs "$name") || true
  [ -n "$r" ] || continue
  ROWS=$((ROWS+1))
  libc_row=$(exp_rootfs_libc "$name")
  # ⚠ nix prints `warning: unknown setting …` to stderr on a host whose nix.conf
  # it cannot read; that is not a failure and is filtered rather than hidden.
  out=$("$REPO_DIR/pgb" rootfs run "$r" --copy "$NIX:/nixbin" -- /nixbin --version 2>&1 \
        | grep -v '^warning:' | head -1 | tr -d '\r')
  case "$out" in
    *"(Nix) $NIX_VER") v=ok; RAN=$((RAN+1)) ;;
    *)                 v="⛔"; BAD=$((BAD+1)) ;;
  esac
  printf '  %-22s %-6s %s %s\n' "$name" "$libc_row" "${out:-<none>}" "$v"
done

printf '\n'
exp_check "every fetched environment was measured" "$((RAN+BAD))" "$ROWS"
exp_check "environments where it did NOT report its version" "$BAD" 0
exp_note "⭐ a musl-static binary running on glibc hosts is expected and is not"
exp_note "   the interesting direction; it is recorded because T-051 needs to"
exp_note "   know the binary works where it would be put, not why."

exp_finish
