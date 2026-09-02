#!/bin/sh
# 80-nix-without-nix.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# ⭐ The operator ruled that nixpkgs is this project's dependency planner:
# "instead of writing resolvers, parsers, dependency checkers etc etc -- let's
# just use nix ... even if we only took their package manifest and files, it
# already significantly reduces our workload."
# docs/design/nix-front-end.md records the ruling and three open questions.
# This experiment answers the two that are measurable:
#
#   Q1  Can a nixpkgs package be RESOLVED and FETCHED with no nix installed?
#       If yes, "use nix" costs pgb a manifest format rather than a daemon.
#   Q2  What does a fetched nixpkgs binary actually do when you run it?
#
# -- WHAT IT DOES NOT ESTABLISH ---------------------------------------------
#
# ⛔ THE CACHE IS NOT AN EVALUATOR. Everything below fetches paths a channel
# ALREADY BUILT. It cannot evaluate a nix expression, apply an overlay, or
# build an attribute nobody built, and no amount of HTTP makes it able to.
# Where that boundary lands for the front end is docs/research/nix.md.
#
# ⚠ ONE MACHINE, ONE DAY, one channel revision. The revision is printed in the
# result, because a store path index is only meaningful beside one.
#
# ⚠ THE NETWORK IS A CONDITION HERE. Unlike most experiments in this tree this
# one talks to cache.nixos.org, so a failure can be theirs. Arms that need the
# network say so; the verification arms have committed fixtures and do not.
#
# Exit: 0 all assertions matched, 1 one did not, 2 could not run.

set -u
. "$(dirname "$0")/lib.sh"

FETCH="$(dirname "$0")/../pgb"
NARPY="$(dirname "$0")/../pgb"
WORK="${TMPDIR:-/var/tmp}/exp80-$$"
mkdir -p "$WORK"
trap 'rm -rf "$WORK"' EXIT INT TERM

exp_begin "80 - reaching nixpkgs with no nix, and what comes back"

command -v curl >/dev/null 2>&1 || { exp_skip "the whole experiment" "no curl"; exp_finish; }

# ⭐ THE ORACLE. Where a real nix is present, every claim this experiment makes
# about the protocol is checked against nix itself rather than against the
# tool that produced it. Where it is absent those arms SKIP -- they do not
# quietly pass.
NIX=""
for c in nix /nix/var/nix/profiles/default/bin/nix; do
  command -v "$c" >/dev/null 2>&1 && { NIX="$c"; break; }
  [ -x "$c" ] && { NIX="$c"; break; }
done
NIXSTORE=""
if [ -n "$NIX" ]; then
  NIXSTORE=$(dirname "$NIX")/nix-store
  [ -x "$NIXSTORE" ] || NIXSTORE=""
fi
exp_note "oracle: $([ -n "$NIX" ] && "$NIX" --version 2>/dev/null | head -1 || echo 'no nix on this machine -- oracle arms will SKIP')"
printf '\n'

# -- arm 1: the index, and the pin hiding in a redirect ----------------------
printf -- '-- arm 1: resolve a package name with no nix -------------------\n'

REV=$("$FETCH" nix cache channel 2>/dev/null | sed -n 's/^revision  //p')
exp_check "the channel redirect names a revision" "$([ -n "$REV" ] && echo yes || echo no)" yes
exp_note "channel revision: ${REV:-<none>}"

IDX_N=$("$FETCH" nix cache resolve 'bash' --limit 100000 2>/dev/null | wc -l | tr -d ' ')
exp_check "the index answers a name query" "$([ "${IDX_N:-0}" -gt 100 ] && echo yes || echo no)" yes
exp_note "store paths matching 'bash' in this channel: $IDX_N"

# ⛔ THE SUBJECT IS PINNED BY HASH, not taken from the index, so that a channel
# bump does not silently change what every row below describes.
BASH_PATH=/nix/store/9ipfvwnqp1q8ijnmi5sxvlx9r8w34lw3-bash-5.3p15
BASH_HASH=9ipfvwnqp1q8ijnmi5sxvlx9r8w34lw3

# -- arm 2: the signature, and the negative control --------------------------
printf '\n-- arm 2: verification, and what happens when it should fail ---\n'

FIXDIR="$(dirname "$0")/../scripts/common/fixtures/nix"
KEY=$("$NARPY" nix nar pubkey)
ok=no
"$NARPY" nix nar verify-narinfo "$KEY" < "$FIXDIR/bash-5.3p15.narinfo" >/dev/null 2>&1 && ok=yes
exp_check "a real narinfo verifies (offline fixture)" "$ok" yes

# ⛔ THE NEGATIVE CONTROL IS THE HALF THAT MEANS ANYTHING. A verifier that
# always says yes passes the row above.
sed 's/^NarSize: \([0-9]*\)$/NarSize: 999999999/' "$FIXDIR/bash-5.3p15.narinfo" > "$WORK/tampered.narinfo"
bad=verified
"$NARPY" nix nar verify-narinfo "$KEY" < "$WORK/tampered.narinfo" >/dev/null 2>&1 || bad=refused
exp_check "a narinfo with one field changed is refused" "$bad" refused

wrongkey=verified
"$NARPY" nix nar verify-narinfo "cache.nixos.org-1:$(python3 -c '
import base64,sys
k=bytearray(base64.b64decode(sys.argv[1]));k[0]^=1;print(base64.b64encode(bytes(k)).decode())' "${KEY#*:}")" \
  < "$FIXDIR/bash-5.3p15.narinfo" >/dev/null 2>&1 || wrongkey=refused
exp_check "the same narinfo under a flipped key is refused" "$wrongkey" refused

# -- arm 3: the closure, against nix's own answer ----------------------------
printf '\n-- arm 3: the dependency closure -------------------------------\n'

"$FETCH" nix cache closure "$BASH_HASH" > "$WORK/ours.txt" 2>"$WORK/closure.err"
OURS_N=$(wc -l < "$WORK/ours.txt" | tr -d ' ')
exp_check "a closure comes back over plain HTTPS" "$([ "${OURS_N:-0}" -ge 2 ] && echo yes || echo no)" yes
exp_note "closure size: $OURS_N paths"
sed 's/^/        /' "$WORK/ours.txt"

if [ -n "$NIXSTORE" ]; then
  # ⭐ nix computes the same closure from the same narinfos. If the two differ,
  # this tool's graph walk is wrong and every fetch built on it is incomplete.
  if "$NIXSTORE" --realise "$BASH_PATH" >/dev/null 2>"$WORK/realise.err"; then
    "$NIXSTORE" -qR "$BASH_PATH" 2>/dev/null | sort > "$WORK/nix.txt"
    sort "$WORK/ours.txt" > "$WORK/ours.sorted"
    if diff -q "$WORK/nix.txt" "$WORK/ours.sorted" >/dev/null 2>&1; then
      exp_check "our closure == nix-store -qR" same same
    else
      exp_check "our closure == nix-store -qR" different same
      diff "$WORK/nix.txt" "$WORK/ours.sorted" | sed 's/^/        /' | head -20
    fi
  else
    exp_skip "our closure == nix-store -qR" "nix could not realise the subject"
  fi
else
  exp_skip "our closure == nix-store -qR" "no nix on this machine"
fi

# -- arm 4: the fetch, and whether the bytes are nix's bytes -----------------
printf '\n-- arm 4: the fetch ---------------------------------------------\n'

rm -rf "$WORK/store"
if "$FETCH" nix cache fetch "$BASH_HASH" --out "$WORK/store" >"$WORK/fetch.log" 2>&1; then
  exp_check "the closure fetches, hashes checked" ok ok
else
  exp_check "the closure fetches, hashes checked" failed ok
  sed 's/^/        /' "$WORK/fetch.log" | head -10
fi
GOT="$WORK/store/${BASH_PATH#/nix/store/}"
exp_check "bin/bash landed" "$([ -x "$GOT/bin/bash" ] && echo yes || echo no)" yes

if [ -n "$NIXSTORE" ] && [ -d "$BASH_PATH" ]; then
  # ⭐ THE STRONGEST ORACLE HERE: nix extracted the same NAR into /nix/store.
  # Comparing our tree to that one checks the NAR parser, the executable bit,
  # the symlinks and the file contents in a single assertion.
  if diff -r --no-dereference "$BASH_PATH" "$GOT" > "$WORK/treediff.txt" 2>&1; then
    exp_check "our extraction == nix's own /nix/store tree" identical identical
  else
    exp_check "our extraction == nix's own /nix/store tree" "differs" identical
    head -20 "$WORK/treediff.txt" | sed 's/^/        /'
  fi
else
  exp_skip "our extraction == nix's own tree" "no realised /nix/store copy to compare against"
fi

# -- arm 5: ⭐ what a nixpkgs binary DOES when you run it ---------------------
printf '\n-- arm 5: running what came back --------------------------------\n'

# ⛔ THIS IS THE ARM THAT MATTERS TO pgb. A nixpkgs binary is not relocatable:
# its PT_INTERP is an absolute /nix/store path, so it runs on a machine with
# that store and nowhere else. That is the same class of problem this project
# exists for, arriving from a different direction.
INTERP=$(python3 - "$GOT/bin/bash" <<'PY'
import struct, sys
with open(sys.argv[1], "rb") as f:
    d = f.read()
if d[:4] != b"\x7fELF":
    print("not-elf"); raise SystemExit
phoff, = struct.unpack_from("<Q", d, 0x20)
phentsize, phnum = struct.unpack_from("<HH", d, 0x36)
for i in range(phnum):
    off = phoff + i * phentsize
    p_type, = struct.unpack_from("<I", d, off)
    if p_type == 3:  # PT_INTERP
        p_offset, = struct.unpack_from("<Q", d, off + 8)
        p_filesz, = struct.unpack_from("<Q", d, off + 32)
        print(d[p_offset:p_offset + p_filesz].rstrip(b"\0").decode())
        break
else:
    print("none")
PY
)
exp_note "PT_INTERP: $INTERP"
case "$INTERP" in
  /nix/store/*) interp_class=nix-store-absolute ;;
  none)         interp_class=static ;;
  *)            interp_class=other ;;
esac
exp_check "the fetched binary's loader is an absolute store path" "$interp_class" nix-store-absolute

# ⛔ "IT DOES NOT RUN WITHOUT /nix" CANNOT BE MEASURED ON A MACHINE THAT HAS
# ONE, and this machine has one because the oracle needed it. So the claim is
# taken inside a rootfs that has no /nix at all -- the pinned build
# environment, or any of the eleven -- where the kernel's ENOENT is the real
# answer rather than an argument.
#
# ⛔ TWO OF THE THREE CANDIDATES THIS LOOP USED TO NAME COULD NEVER MATCH.
# They were `debian12` and `alpine322`; the local names in
# scripts/common/rootfs-images.txt are `debian-12` and `alpine-3.22`, so those
# two `[ -d ]` tests were dead and the arm rested entirely on the third,
# `pgb-env-debian12` — itself a hardcoded copy of a name that lives in
# internal/cfg/cfg.go. Both halves fixed here: ENV_ROOT comes from lib.sh, and
# the rest of the list is READ OUT OF THE IMAGES FILE rather than retyped, so
# it cannot drift from it again. T-070.
NOSTORE=""
for cand in "$ENV_ROOT" $(awk 'NF>=4 && $1 !~ /^#/ {print $2}' \
                          "$REPO_DIR/scripts/common/rootfs-images.txt"); do
  case "$cand" in /*) ;; *) cand="$ROOTFS_DIR/$cand" ;; esac
  [ -d "$cand" ] && [ ! -d "$cand/nix/store" ] && { NOSTORE="$cand"; break; }
done
if [ -n "$NOSTORE" ]; then
  st=$(exp_run_status "$NOSTORE" "$GOT/bin/bash:/subject-bash" /subject-bash -c 'exit 0')
  # ⚠ THE KERNEL REPORTS THE MISSING *INTERPRETER* AS A MISSING *BINARY*, so a
  # shell reports 127 and the message names the program, not the loader. That
  # misreading is why this row prints the status rather than a yes/no.
  exp_check "in a rootfs with no /nix, it does not run" \
    "$([ "$st" = 0 ] && echo ran || echo "refused($st)")" "refused(127)"
  exp_note "rootfs: $NOSTORE"
  # The positive control: the SAME runner, on the same rootfs, with a binary
  # that has no /nix dependency. Without it, "refused" could mean the runner
  # is broken rather than the binary.
  ctl=$(exp_run_status "$NOSTORE" "/bin/true:/subject-true" /bin/sh -c 'exit 0')
  exp_check "control: the runner itself works there" "$ctl" 0
else
  exp_skip "in a rootfs with no /nix, it does not run" "no rootfs available -- run pgb env create or pgb rootfs fetch"
fi

# ⭐ AND IT RUNS WHEN HANDED ITS OWN LOADER out of the fetched tree, which is
# what every nix bundler is doing under the surface: shipping the store.
LOADER=$(find "$WORK/store" -name 'ld-linux-x86-64.so.2' -o -name 'ld-linux-aarch64.so.1' 2>/dev/null | head -1)
LIBPATH=$(find "$WORK/store" -maxdepth 2 -type d -name lib 2>/dev/null | tr '\n' ':' | sed 's/:$//')
if [ -n "$LOADER" ]; then
  out=$("$LOADER" --library-path "$LIBPATH" "$GOT/bin/bash" -c 'echo RAN-$BASH_VERSION' 2>&1 | head -1)
else
  out="no loader in the closure"
fi
case "$out" in
  RAN-*) relocated=yes ;;
  *)     relocated="no: $out" ;;
esac
exp_check "it runs when given the loader we fetched beside it" "$relocated" yes
exp_note "output: $out"

# -- arm 6: ⛔ what nixpkgs means by 'static' ---------------------------------
printf '\n-- arm 6: what nixpkgs static actually is ------------------------\n'

# ⛔ THE FINDING THAT REDIRECTS THE WHOLE FRONT END. The reference recipe the
# operator pointed at (soarpkgs binaries/bash/static.nixpkgs.stable.yaml, at
# the pinned commit) builds with `nix-build <nixpkgs> --attr pkgsStatic.bash`.
# On Linux, pkgsStatic is NOT glibc-static -- it switches the whole package set
# to musl. So nixpkgs' answer to "static" is the thing tmp/START.md asks this
# project to avoid, and the two are complementary rather than competing.
if [ -n "$NIX" ]; then
  libc=$("$NIX"-instantiate --eval --expr \
    'with import <nixpkgs> {}; pkgsStatic.stdenv.hostPlatform.libc' 2>/dev/null | tr -d '"')
  [ -n "$libc" ] || libc=unknown
  exp_check "nixpkgs pkgsStatic.stdenv.hostPlatform.libc" "$libc" musl
  hostlibc=$("$NIX"-instantiate --eval --expr \
    'with import <nixpkgs> {}; stdenv.hostPlatform.libc' 2>/dev/null | tr -d '"')
  exp_check "and the ordinary package set is" "${hostlibc:-unknown}" glibc
else
  # Even with no nix the index says it: every pkgsStatic path carries the
  # target triple in its NAME.
  n=$("$FETCH" nix cache resolve 'static-x86_64-unknown-linux-musl' --limit 100000 2>/dev/null | wc -l | tr -d ' ')
  exp_check "the channel index shows static == musl triples" \
    "$([ "${n:-0}" -gt 0 ] && echo yes || echo no)" yes
  exp_note "paths named *-static-x86_64-unknown-linux-musl-*: $n"
  exp_skip "pkgsStatic libc, from nix itself" "no nix on this machine"
fi

# -- the result file ---------------------------------------------------------
RESULT="$EXP_OUT/RESULT.txt"
{
  printf 'experiment 80 - reaching nixpkgs with no nix\n'
  printf 'date            : %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'channel         : nixpkgs-unstable\n'
  printf 'revision        : %s\n' "${REV:-unknown}"
  printf 'subject         : %s\n' "$BASH_PATH"
  printf 'closure         : %s paths, fetched and hash-checked over HTTPS\n' "$OURS_N"
  printf 'oracle          : %s\n' "$([ -n "$NIX" ] && "$NIX" --version 2>/dev/null | head -1 || echo none)"
  printf '\n'
  printf 'Q1 can a nixpkgs package be resolved and fetched with no nix?\n'
  printf '   YES for anything the channel built. Three HTTPS endpoints:\n'
  printf '   store-paths.xz (an index, and its redirect is a pin), <hash>.narinfo\n'
  printf '   (signed metadata), and the NAR. Signature and NarHash both checked.\n'
  printf '   NO for anything it did not: there is no evaluator here.\n\n'
  printf 'Q2 what does a fetched nixpkgs binary do?\n'
  printf '   PT_INTERP = %s\n' "$INTERP"
  printf '   It is bound to an absolute /nix/store path and runs only where that\n'
  printf '   store exists, or when handed its own loader: %s\n' "$relocated"
  printf '   ⛔ That is why every nix bundler ends up shipping a store.\n\n'
  printf 'arm 6: nixpkgs pkgsStatic is MUSL, not static glibc.\n'
  printf '   The soarpkgs recipe this project was pointed at builds\n'
  printf '   pkgsStatic.bash, so its output is a musl binary. pgb answers the\n'
  printf '   glibc half, and experiments/61- has what the difference costs.\n'
} > "$RESULT"
exp_note "written: $RESULT"

exp_finish
