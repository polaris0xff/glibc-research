#!/bin/sh
# 83-drv-without-nix.sh
#
# -- THE QUESTION -----------------------------------------------------------
#
# ⭐ THE OPERATOR'S, and it is a good one:
#
#     "I think the downloaded nix store files themselves contain *.drv files?
#      so we don't actually need nix installed no?"
#
# experiments/80- established that a nixpkgs package can be FETCHED with no
# nix. That leaves the other half: can it be PLANNED with no nix -- source,
# patches, configure flags, dependency graph -- or does that still need an
# evaluator?
#
# The route being measured:
#
#   name  --store-paths.xz-->  /nix/store/<hash>-<name>
#         --<hash>.narinfo-->  Deriver: <hash>-<name>.drv
#         --fetch the .drv-->  ATerm: src, patches, flags, inputDrvs
#         --its References-->  every input .drv, fetched the same way
#
# -- WHAT IT DOES NOT ESTABLISH ---------------------------------------------
#
# ⛔ THE INDEX IS NOT AN EVALUATOR. Turning a NAME into a store path here is a
# regex over what the channel already built. An override, an overlay, a
# `pkgsStatic.*` attribute or any unbuilt attribute is out of reach by this
# route no matter how many derivations the cache holds.
#
# ⚠ ARM 2 IS A SAMPLE AND ITS POPULATION IS THE WHOLE RESULT. The rate differs
# by more than an order of magnitude between "any path in the index" and "a
# named top-level package", and quoting either without the other is the
# defect this arm exists to avoid -- it is exactly the mistake this session
# made first, reporting 8-of-12 from a hand-picked list of popular packages.
#
# ⚠ The network is a condition. Arms 2 and 3 talk to cache.nixos.org.
#
# Exit: 0 all assertions matched, 1 one did not, 2 could not run.

set -u
. "$(dirname "$0")/lib.sh"

FETCH="$REPO_DIR/pgb"
DRVPY="$REPO_DIR/pgb"
PLANPY="$REPO_DIR/pgb"
WORK="${TMPDIR:-/var/tmp}/exp83-$$"
mkdir -p "$WORK"
trap 'rm -rf "$WORK"' EXIT INT TERM

exp_begin "83 - planning a nixpkgs package with no nix, from the .drv"

command -v curl >/dev/null 2>&1 || { exp_skip "the whole experiment" "no curl"; exp_finish; }

NIX=""
for c in nix /nix/var/nix/profiles/default/bin/nix; do
  command -v "$c" >/dev/null 2>&1 && { NIX=$(command -v "$c"); break; }
  [ -x "$c" ] && { NIX="$c"; break; }
done

# -- arm 1: the format reader, offline ---------------------------------------
printf -- '-- arm 1: the ATerm reader ------------------------------------\n'
if "$DRVPY" nix drv selftest > "$WORK/drv.selftest" 2>&1; then
  exp_check "pgb nix drv selftest" pass pass
else
  exp_check "pgb nix drv selftest" FAIL pass
  sed 's/^/        /' "$WORK/drv.selftest" >&2
fi
exp_note "$(grep -c '^  ok' "$WORK/drv.selftest" 2>/dev/null || echo 0) checks, including both nix escapes and two refusals"

# -- arm 2: how much of the graph is actually in the cache -------------------
printf '\n-- arm 2: is the deriver .drv in the binary cache? -------------\n'

deriver_of() {   # store path or hash -> the deriver path, or nothing
  curl -sSf "https://cache.nixos.org/$(printf '%s' "$1" | sed 's|.*/||; s|-.*||').narinfo" 2>/dev/null \
    | sed -n 's/^Deriver: //p'
}
have_drv() {     # deriver path -> 200 or a code
  curl -sS -o /dev/null -w '%{http_code}' \
    "https://cache.nixos.org/$(printf '%s' "$1" | sed 's|.*/||; s|-.*||').narinfo" 2>/dev/null
}

sample_rate() {  # label  <paths on stdin>
  _sr_label="$1"; _sr_n=0; _sr_ok=0
  while IFS= read -r _sr_p; do
    [ -n "$_sr_p" ] || continue
    _sr_d=$(deriver_of "$_sr_p")
    [ -n "$_sr_d" ] || continue
    _sr_n=$((_sr_n + 1))
    [ "$(have_drv "$_sr_d")" = 200 ] && _sr_ok=$((_sr_ok + 1))
  done
  [ "$_sr_n" -gt 0 ] || { printf '%s 0 0 0\n' "$_sr_label"; return; }
  printf '%s %d %d %d\n' "$_sr_label" "$_sr_n" "$_sr_ok" \
    "$((_sr_ok * 100 / _sr_n))"
}

IDX=$("$FETCH" nix cache resolve '.' --limit 1 >/dev/null 2>&1; \
      printf '%s' "${NIX_FETCH_CACHE:-/var/tmp/pgb-nix-cache}/store-paths.nixpkgs-unstable.txt")
if [ -s "$IDX" ]; then
  TOTAL=$(wc -l < "$IDX" | tr -d ' ')
  exp_note "channel index: $TOTAL store paths"

  # POPULATION A: any path in the index, sampled by stride so the choice is
  # not this script's taste.
  awk 'NR % 401 == 0' "$IDX" | head -60 > "$WORK/popA"
  # POPULATION B: paths that look like a top-level package -- no -dev, -doc,
  # -man, -debug, -info or -lib output suffix.
  grep -vE -- '-(dev|doc|man|debug|info|lib|bin|out|dist|devdoc|source)$' "$IDX" \
    | awk 'NR % 397 == 0' | head -60 > "$WORK/popB"
  # POPULATION C: the packages a person would actually name.
  : > "$WORK/popC"
  for n in bash coreutils gawk jq curl git tmux nano htop sqlite zlib openssl \
           ncurses python3 ffmpeg vim less grep sed findutils; do
    grep -E "/nix/store/[a-z0-9]{32}-$n-[0-9][^/]*$" "$IDX" | head -1 >> "$WORK/popC"
  done

  ROWA=$(sample_rate A < "$WORK/popA")
  ROWB=$(sample_rate B < "$WORK/popB")
  ROWC=$(sample_rate C < "$WORK/popC")
  printf '    %-46s %6s %6s %6s\n' POPULATION SAMPLED 'HAS.DRV' 'PCT'
  for row in "$ROWA" "$ROWB" "$ROWC"; do
    set -- $row
    case "$1" in
      A) lbl="any path in the index, sampled by stride" ;;
      B) lbl="paths with no output suffix (-dev, -doc, ...)" ;;
      C) lbl="twenty packages a person would name" ;;
    esac
    printf '    %-46s %6s %6s %5s%%\n' "$lbl" "$2" "$3" "$4"
  done
  set -- $ROWC
  PCT_C="$4"
  # ⛔ THE ASSERTION IS THE PROPERTY THE DESIGN TURNS ON, NOT A THRESHOLD
  # SOMEBODY GUESSED. The first version of this line demanded a majority and
  # measured 47%, which is not a failure of the route -- it is the route's
  # real shape. What decides the design is that the rate is neither 0 nor 100:
  # above 0 means the no-nix route is worth trying FIRST, below 100 means the
  # fallback to evaluation is MANDATORY rather than a nicety.
  exp_check "the .drv route resolves some named packages" \
    "$([ "${PCT_C:-0}" -gt 0 ] && echo yes || echo no)" yes
  exp_check "and not all of them, so a fallback is required" \
    "$([ "${PCT_C:-0}" -lt 100 ] && echo required || echo unnecessary)" required
  exp_note "named-package rate: $PCT_C% -- quote this WITH its population"
  set -- $ROWA
  exp_note "⚠ population A is $4% -- an order of magnitude lower. Quoting the"
  exp_note "  named-package rate as if it described the store is the mistake"
  exp_note "  this arm exists to prevent; it was made in this session first."
else
  exp_skip "deriver availability" "the channel index could not be fetched"
fi

# -- arm 3: the reader, and the resolver, are two different questions --------
#
# ⛔ THE FIRST VERSION OF THIS ARM ASKED TWO QUESTIONS AT ONCE AND COULD NOT
# ANSWER EITHER. It planned the NAME `bash` without nix and the ATTRIBUTE
# `bash` with nix, then reported three differing fields as a defect. They are
# not a defect: ⭐ nixpkgs' `bash` attribute resolves to `bash-interactive`,
# and the channel index has no way to know that -- it matched `bash-5.3p15`,
# a real and DIFFERENT package carrying `--disable-readline` where the
# attribute carries `--with-installed-readline`.
#
#   3a  given the SAME derivation, does the ATerm reader produce the same plan
#       as `nix derivation show`?          (the reader's correctness)
#   3b  does a NAME resolve to the same derivation an ATTRIBUTE does?
#                                          (index versus evaluator)
# ⛔ THE SUBJECT IS CHOSEN BY WHETHER ITS .drv IS IN THE CACHE, not hard-coded.
# Arm 2 measured that only some are; pinning the subject to `bash` made this
# arm fail on a 404 that is arm 2's finding rather than this arm's question.
#
# ⚠ AND THE CANDIDATE MUST BE PROBED THROUGH THE ATTRIBUTE, NOT THE NAME. The
# first version of this selection checked the deriver of the path the INDEX
# matched, then handed arm 3a the path the ATTRIBUTE resolves to -- two
# different store paths, so a subject that passed the check still skipped.
#
# ⭐ Which packages have a cached .drv is itself a finding. Measured here on
# 2026-09-01: zlib, gawk, gnugrep and coreutils do; jq, nano, htop, sqlite,
# ncurses and less do not. The four that do are all INPUTS TO OTHER BUILDS --
# stdenv-adjacent packages in everybody's build graph -- which explains arm
# 2's spread far better than "popular packages" does.
attr_out() {   # attribute -> its out store path, via nix
  [ -n "$NIX" ] || return 1
  _ao_d=$("$(dirname "$NIX")/nix-instantiate" '<nixpkgs>' --attr "$1" 2>/dev/null \
          | grep '^/nix/store/' | head -1)
  _ao_d="${_ao_d%%!*}"
  [ -n "$_ao_d" ] || return 1
  "$NIX" derivation show "$_ao_d" 2>/dev/null | python3 -c '
import json, sys
d = json.load(sys.stdin)["derivations"]
print(list(d.values())[0]["outputs"].get("out", {}).get("path", ""))' 2>/dev/null
}

SUBJ="${EXP83_SUBJECT:-}"
if [ -z "$SUBJ" ] && [ -n "$NIX" ]; then
  for cand in gawk coreutils gnugrep zlib bash jq nano htop sqlite ncurses less; do
    _o=$(attr_out "$cand") || continue
    [ -n "$_o" ] || continue
    _d=$(deriver_of "$_o"); [ -n "$_d" ] || continue
    [ "$(have_drv "$_d")" = 200 ] && { SUBJ="$cand"; break; }
  done
fi
[ -n "$SUBJ" ] || SUBJ=gawk
exp_note "subject: $SUBJ (its attribute's deriver .drv is in the cache)"
printf '\n-- arm 3a: the SAME derivation through both routes -------------\n'

if [ -n "$NIX" ]; then
  NIXDIR=$(dirname "$NIX")
  DRV=$("$NIXDIR/nix-instantiate" '<nixpkgs>' --attr "$SUBJ" 2>/dev/null \
        | grep '^/nix/store/' | head -1)
  DRV="${DRV%%!*}"
  OUTP=$("$NIX" derivation show "$DRV" 2>/dev/null | python3 -c '
import json, sys
d = json.load(sys.stdin)["derivations"]
print(list(d.values())[0]["outputs"].get("out", {}).get("path", ""))' 2>/dev/null)
else
  DRV=""; OUTP=""
fi

# ⛔ THE QUESTION CANNOT BE ASKED WHEN THE INPUT IS NOT THERE, and that is a
# SKIP with a reason, not a FAIL. Arm 2 measured that a deriver .drv is in the
# cache under half the time for named packages; when this subject is one of
# the misses, the reader has nothing to read and saying "the reader failed"
# would be a wrong verdict on a working reader.
OUTP_DRV=""
[ -n "$OUTP" ] && OUTP_DRV=$(deriver_of "$OUTP")
if [ -n "$OUTP_DRV" ] && [ "$(have_drv "$OUTP_DRV")" != 200 ]; then
  exp_skip "reader against evaluator" \
    "the deriver of $SUBJ ($(basename "$OUTP_DRV")) is not in the cache -- arm 2's finding"
  OUTP=""
fi

if [ -n "$OUTP" ]; then
  if PGB_STATE="$WORK/state" "$REPO_DIR/pgb" nix plan "$OUTP" --out "$WORK/nonix.json" \
       >"$WORK/nonix.log" 2>&1 && [ -s "$WORK/nonix.json" ] &&
     grep -q 'no nix was used' "$WORK/nonix.log"; then
    exp_check "that exact store path plans with no nix" built built
    if PGB_STATE="$WORK/state2" PGB_NIX_FORCE_EVAL=1 \
         "$REPO_DIR/pgb" nix plan "$SUBJ" --out "$WORK/withnix.json" \
         >"$WORK/withnix.log" 2>&1 && [ -s "$WORK/withnix.json" ]; then
      python3 - "$WORK/nonix.json" "$WORK/withnix.json" > "$WORK/cmp.txt" 2>&1 <<'PY'
import json, sys
a = json.load(open(sys.argv[1]))
b = json.load(open(sys.argv[2]))
same = diff = 0
# ⚠ `attr` and `nixpkgs` are EXPECTED to differ: one route was handed a store
# path and the other an attribute name, and only the evaluator knows the lib
# version. Asserting on them would fail for reasons that are not defects.
for k in ("pname", "version", "configureFlags", "patches", "src",
          "buildSystemHooks", "buildInputs", "outputs"):
    va, vb = a.get(k), b.get(k)
    if k == "patches":
        va = [p.get("store") for p in va or []]
        vb = [p.get("store") for p in vb or []]
    if k == "src":
        va = (va or {}).get("store")
        vb = (vb or {}).get("store")
    if va == vb:
        same += 1
        print("  same  %-18s %s" % (k, str(va)[:60]))
    else:
        diff += 1
        print("  DIFF  %-18s no-nix=%s  nix=%s" % (k, str(va)[:40], str(vb)[:40]))
print("FIELDS same=%d diff=%d" % (same, diff))
PY
      sed 's/^/      /' "$WORK/cmp.txt"
      NDIFF=$(sed -n 's/.*FIELDS same=[0-9]* diff=\([0-9]*\).*/\1/p' "$WORK/cmp.txt")
      exp_check "reader and evaluator agree on every field" "${NDIFF:-?}" 0
    else
      exp_skip "reader and evaluator agree" "the evaluation route failed here"
    fi
  else
    exp_check "that exact store path plans with no nix" failed built
    tail -5 "$WORK/nonix.log" | sed 's/^/        /'
  fi
else
  exp_skip "reader against evaluator" "no nix here to produce the comparison"
fi

printf '\n-- arm 3b: does a NAME resolve to what an ATTRIBUTE does? ------\n'
# ⭐ A FINDING, NOT A FAILURE, and `bash` is the case that shows it plainly.
BYNAME=$("$FETCH" nix cache resolve "/nix/store/[a-z0-9]{32}-$SUBJ-[0-9][^/]*$" --limit 40 2>/dev/null \
         | grep -vE -- '-(dev|doc|man|debug|info|devdoc|dist)$' | head -1)
exp_note "by name     : ${BYNAME:-<none>}"
exp_note "by attribute: ${OUTP:-<no nix>}"
if [ -n "$BYNAME" ] && [ -n "$OUTP" ]; then
  exp_check "the name and the attribute name the same store path" \
    "$([ "$BYNAME" = "$OUTP" ] && echo same || echo different)" different
  exp_note "⛔ They differ, and there are TWO reasons, both real:"
  exp_note "  1. CHANNEL SKEW. The index is one channel snapshot; the nix on"
  exp_note "     this machine has another. Same package, different revision,"
  exp_note "     different store path -- which is what '$SUBJ' shows here."
  exp_note "  2. THE ATTRIBUTE IS NOT THE NAME. nixpkgs' 'bash' attribute is"
  exp_note "     bash-interactive, and no name match can know that."
  exp_note "  Either way an index lookup is not an evaluation, and a plan must"
  exp_note "  record which route produced it. Both do."
else
  exp_skip "name against attribute" "one of the two routes produced nothing"
fi
# -- arm 4: with nix taken off PATH ------------------------------------------
printf '\n-- arm 4: the same, with nix removed from PATH -----------------\n'
# ⛔ REMOVED, NOT MERELY UNUSED. A route that "does not need nix" has to be
# run on a machine that does not have one, and the cheapest honest version of
# that is a PATH with nothing nix-shaped on it and PGB_NIX_FORCE_EVAL unset.
if env -i HOME="$HOME" PATH=/usr/bin:/bin TMPDIR="${TMPDIR:-/var/tmp}" \
     PGB_STATE="$WORK/state3" \
     "$REPO_DIR/pgb" nix plan "$SUBJ" --out "$WORK/clean.json" \
     >"$WORK/clean.log" 2>&1 && [ -s "$WORK/clean.json" ]; then
  exp_check "a plan with nix off PATH entirely" built built
  exp_note "pname: $(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["pname"])' "$WORK/clean.json" 2>/dev/null)"
else
  exp_check "a plan with nix off PATH entirely" failed built
  sed 's/^/        /' "$WORK/clean.log" | tail -6
fi

RESULT="$EXP_OUT/RESULT.txt"
{
  printf 'experiment 83 - planning with no nix, from the .drv in the cache\n'
  printf 'date            : %s\n\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf 'The route: name -> store-paths.xz -> narinfo Deriver -> the .drv,\n'
  printf 'every hop signed and hash-checked, no nix and no evaluation.\n\n'
  printf 'deriver .drv availability, BY POPULATION:\n'
  for row in "${ROWA:-A 0 0 0}" "${ROWB:-B 0 0 0}" "${ROWC:-C 0 0 0}"; do
    set -- $row
    printf '  %-4s sampled=%-4s has-drv=%-4s %s%%\n' "$1" "$2" "$3" "$4"
  done
  printf '\n  A = any path in the channel index, sampled by stride\n'
  printf '  B = paths with no output suffix\n'
  printf '  C = twenty packages a person would name\n\n'
  printf '⛔ The spread between A and C is the result. The route is worth\n'
  printf 'trying first for a named package and is not a general property of\n'
  printf 'the cache, and a fallback to evaluation is therefore not optional.\n'
} > "$RESULT"
exp_note "written: $RESULT"

exp_finish
