#!/bin/sh
# experiments/88-nonix-end-to-end.sh
#
# ⛔ THE NO-NIX ROUTE, FINISHED: PLANNED, FETCHED **AND BUILT** WITH NO NIX
# INSTALLED AND NO ROOT, ON A HOST THAT HAS NEITHER. T-050 and T-051.
#
# `experiments/83-` measured the only name->derivation link the channel-index
# route has -- the narinfo `Deriver:` field -- at **3%** of paths sampled by
# stride, **1%** of paths with no output suffix and **47%** of twenty packages
# a person would name, and concluded that an evaluation fallback was
# mandatory. This experiment builds the fallback and measures it.
#
# ⭐ THE FALLBACK IS NOT AN EVALUATOR. hydra BUILT the channel, so it holds the
# derivation for every job it ran:
#
#   hydra.nixos.org/job/<project>/<jobset>/<attr>.<system>/latest-finished
#     -> drvpath, system, and every output's store path
#
# That is an index of BUILDS rather than a field somebody happened to upload
# beside a NAR, so `Deriver:` availability does not bound it. Arm 1 measures
# the rate over the SAME twenty packages 83- used, so the two numbers are
# comparable; arm 2 checks the drvpath against what a local `nix-instantiate`
# computes, which is the only control that can say the index is telling the
# truth; arm 3 compares the two routes' plans field by field.
#
# ⭐ ARM 5 IS THE ENTRY. A rootfs with a C toolchain, no `nix` on PATH, no
# `/nix` directory, and a NON-ROOT uid -- and jq planned, fetched and built
# inside it.
#
# Exit: 0 matched, 1 did not, 2 could not run.
# SPDX-License-Identifier: MIT
. "$(dirname "$0")/lib.sh"

exp_begin "88 - plan, fetch and BUILD a nixpkgs package with no nix and no root"

WORK="${PGB_WORK:-/var/tmp/pgb-exp88}"
rm -rf "$WORK"; mkdir -p "$WORK" || { echo "cannot create $WORK" >&2; exit 2; }
NF="$REPO_DIR/pgb"
PGB="$REPO_DIR/pgb"

# The PATH a host with no nix has. ⛔ Not `PATH` with a grep -v: the point is a
# fixed, stated PATH, so a nix that arrived through some other directory
# cannot quietly serve a route that claims not to need one.
NONIX_PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
nonix() { env -u NIX_PATH -u NIX_REMOTE PATH="$NONIX_PATH" "$@"; }

exp_note "no-nix PATH: $NONIX_PATH"
exp_check "and nix really is not on it" \
          "$(nonix sh -c 'command -v nix >/dev/null 2>&1 && echo present || echo absent')" "absent"

# ---------------------------------------------------------------------------
# ARM 1 -- availability, on experiments/83-'s OWN population and predicate.
# ---------------------------------------------------------------------------
printf -- '-- arm 1: name -> derivation, 83-'"'"'s population C -------------\n'
# ⛔ 83-'s LIST AND 83-'s TEST, COPIED RATHER THAN IMPROVED, or the two numbers
# do not compare. Its predicate is not "does the narinfo mention a Deriver" --
# it is "does the narinfo name a Deriver AND does that .drv narinfo return
# 200", which is the question a planner actually asks.
PKGS="bash coreutils gawk jq curl git tmux nano htop sqlite zlib openssl
      ncurses python3 ffmpeg vim less grep sed findutils"
hyd_ok=0; hyd_no=0; drv_ok=0; drv_no=0; n=0
for p in $PKGS; do
  n=$((n + 1))
  if nonix "$NF" nix cache drv "$p" >"$WORK/drv.$p" 2>"$WORK/drv.$p.err"; then
    hyd_ok=$((hyd_ok + 1))
  else
    hyd_no=$((hyd_no + 1)); printf '        hydra   MISS %s\n' "$p"
  fi
  # 83-'s route: a store path from the index, its narinfo's Deriver, and
  # whether THAT .drv is in the cache.
  sp=$(nonix "$NF" nix cache resolve "/nix/store/[a-z0-9]{32}-$p-[0-9][^/]*$" --limit 1 2>/dev/null | head -1)
  dv=""
  [ -n "$sp" ] && dv=$(nonix "$NF" nix cache info "$sp" 2>/dev/null | sed -n 's/^Deriver: //p')
  if [ -n "$dv" ] && nonix "$NF" nix cache info "$dv" >/dev/null 2>&1; then
    drv_ok=$((drv_ok + 1))
  else
    drv_no=$((drv_no + 1)); printf '        Deriver MISS %s\n' "$p"
  fi
done
exp_note "packages tried: $n"
exp_note "hydra route          : $hyd_ok resolved, $hyd_no not"
exp_note "narinfo Deriver route: $drv_ok resolved, $drv_no not"
exp_check "the hydra route resolves at least as many as Deriver" \
          "$([ "$hyd_ok" -ge "$drv_ok" ] && echo yes || echo no)" "yes"
exp_check "packages the hydra route cannot resolve" "$hyd_no" "1"
# ⭐ AND THE ONE MISS IS NOT A GAP IN THE ROUTE, it is a name that is not a
# nixpkgs attribute: there is no `grep`, there is `gnugrep`. Asserted both
# ways so "19 of 20" cannot be read as a 5% failure rate.
exp_check "the miss has no attribute under that name either" \
          "$(nonix "$NF" nix cache attr grep >/dev/null 2>&1 && echo found || echo absent)" "absent"
exp_check "and its real attribute resolves" \
          "$(nonix "$NF" nix cache drv gnugrep 2>/dev/null | sed -n 's/^Nixname: //p')" "gnugrep-3.12"
exp_note "⚠ THE TWO ROUTES ANSWER DIFFERENT QUESTIONS and the comparison only"
exp_note "  holds for a package NAME. Deriver answers \"which .drv made this"
exp_note "  store path\"; hydra answers \"which .drv does this attribute name\"."
exp_note "  83-'s populations A and B are store paths sampled by stride, which"
exp_note "  cannot be asked of hydra at all -- there is no attribute to ask for."

# ---------------------------------------------------------------------------
# ARM 2 -- ⭐ THE CONTROL. The index must agree with evaluation.
# ---------------------------------------------------------------------------
printf -- '\n-- arm 2: the drvpath against a local nix-instantiate ---------\n'
NIXBIN=""
for c in /nix/var/nix/profiles/default/bin/nix-instantiate "$(command -v nix-instantiate 2>/dev/null)"; do
  [ -n "$c" ] && [ -x "$c" ] && { NIXBIN="$c"; break; }
done
if [ -z "$NIXBIN" ]; then
  exp_skip "arm2: agreement with evaluation" "no nix on this machine to check against"
else
  agree=0; disagree=0
  for p in jq gawk zlib openssl; do
    # ⚠ ASK THE ROUTE HERE rather than reading arm 1's file: a file left by a
    # FAILED lookup is empty, and an empty string compared against an empty
    # string is a pass nobody meant to write.
    nonix "$NF" nix cache drv "$p" >"$WORK/a2.$p" 2>/dev/null
    hy=$(sed -n 's/^Drv: //p' "$WORK/a2.$p" 2>/dev/null)
    ev=$("$NIXBIN" --store "$WORK/evalstore" '<nixpkgs>' -A "$p" 2>/dev/null \
         | grep '^/nix/store/' | head -1)
    ev="${ev%%!*}"
    if [ -n "$hy" ] && [ "$hy" = "$ev" ]; then
      agree=$((agree + 1)); exp_note "  $p: $hy  (identical)"
    else
      disagree=$((disagree + 1))
      exp_note "  $p: hydra=$hy"
      exp_note "     eval =$ev"
    fi
  done
  exp_check "drvpaths identical to evaluation's" "$agree" "4"
  exp_check "drvpaths that differ" "$disagree" "0"
  exp_note "⭐ identical drv HASHES mean the index is naming the same"
  exp_note "   derivation an evaluator would compute, not a similar one."
fi

# ---------------------------------------------------------------------------
# ARM 3 -- the two plans, field by field.
# ---------------------------------------------------------------------------
printf -- '\n-- arm 3: the no-nix plan against the evaluated plan ----------\n'
nonix "$PGB" nix plan jq --out "$WORK/jq.nonix.plan" >"$WORK/plan.nonix.log" 2>&1
rc_a=$?
if [ -z "$NIXBIN" ]; then
  exp_skip "arm3: field-by-field comparison" "no nix to produce the other side"
elif [ "$rc_a" != 0 ]; then
  exp_check "arm3: the no-nix plan was produced" "rc=$rc_a" "rc=0"
  sed -n '1,15p' "$WORK/plan.nonix.log" | sed 's/^/        /'
else
  PGB_NIX_FORCE_EVAL=1 PATH="$(dirname "$NIXBIN"):$PATH" \
    "$PGB" nix plan jq --out "$WORK/jq.eval.plan" >"$WORK/plan.eval.log" 2>&1
  if [ ! -s "$WORK/jq.eval.plan" ]; then
    exp_skip "arm3: field-by-field comparison" "the evaluation route produced no plan"
  else
    same=$(python3 - "$WORK/jq.nonix.plan" "$WORK/jq.eval.plan" <<'PY'
import json, sys
a = json.load(open(sys.argv[1])); b = json.load(open(sys.argv[2]))
# `nixpkgs` is the local channel path and exists only on the evaluated side;
# it is the one field that CANNOT agree and saying so is the point.
skip = {"nixpkgs"}
keys = sorted((set(a) | set(b)) - skip)
same = [k for k in keys if a.get(k) == b.get(k)]
diff = [k for k in keys if a.get(k) != b.get(k)]
for k in diff:
    print("      differs: %s\n        no-nix: %r\n        eval  : %r"
          % (k, a.get(k), b.get(k)), file=sys.stderr)
print("%d/%d" % (len(same), len(keys)))
PY
)
    exp_note "fields identical: $same"
    exp_check "every comparable plan field agrees" "${same%%/*}" "${same##*/}"
  fi
fi

# ---------------------------------------------------------------------------
# ARM 4 -- ⛔ THE PLATFORM DEFECT, and the fix, asserted both ways.
# ---------------------------------------------------------------------------
printf -- '\n-- arm 4: store-paths.xz is every system the channel built ----\n'
dar=$(nonix "$NF" nix cache resolve '/nix/store/[a-z0-9]{32}-nix-2\.35\.2$' --limit 4 2>/dev/null | wc -l)
exp_note "store paths named nix-2.35.2 in the index: $dar"
exp_check "a name match really is ambiguous across systems" \
          "$([ "$dar" -gt 1 ] && echo yes || echo no)" "yes"
sysline=$(nonix "$NF" nix cache attr jq 2>/dev/null | sed -n 's/^System: //p')
exp_check "the attribute index states the system" "$sysline" "x86_64-linux"
outn=$(nonix "$NF" nix cache attr jq 2>/dev/null | sed -n 's/^OutputName: //p')
exp_check "and the default output, which is not 'out'" "$outn" "bin"
bashname=$(nonix "$NF" nix cache attr bash 2>/dev/null | sed -n 's/^Name: //p')
exp_check "and the name an attribute really produces" "$bashname" "bash-interactive-5.3p15"
# ⭐ THE ROUTE IS SYSTEM-EXPLICIT IN BOTH DIRECTIONS, and both halves are
# asserted. Asking for darwin gets a darwin answer -- hydra really does build
# jq there -- which is the point: the SYSTEM IS A PARAMETER now, not something
# a name match decides by accident. And a system nobody builds is refused
# rather than answered with whatever sorted first.
dsys=$(nonix "$NF" nix cache drv jq --system aarch64-darwin 2>/dev/null | sed -n 's/^System: //p')
exp_check "asking for darwin returns a darwin build" "$dsys" "aarch64-darwin"
lsys=$(nonix "$NF" nix cache drv jq 2>/dev/null | sed -n 's/^System: //p')
exp_check "and the default is the one this project targets" "$lsys" "x86_64-linux"
if nonix "$NF" nix cache drv jq --system s390x-none >/dev/null 2>&1; then
  exp_check "a system nobody builds is refused" "answered" "refused"
else
  exp_check "a system nobody builds is refused" "refused" "refused"
fi
# ⚠ AND THE MATCH THAT IS NOT AN EXACT ATTRIBUTE SAYS SO. `sed` reaches
# `freebsd.sed` through pname -- a real package for the wrong userland.
exp_check "a pname-only match reports how it matched" \
          "$(nonix "$NF" nix cache attr sed 2>/dev/null | sed -n 's/^Matched: //p')" "pname"
exp_check "and an exact attribute reports that instead" \
          "$(nonix "$NF" nix cache attr jq 2>/dev/null | sed -n 's/^Matched: //p')" "attr"

# ---------------------------------------------------------------------------
# ARM 5 -- ⭐ THE ENTRY: a host with a toolchain, no nix, no /nix, no root.
# ---------------------------------------------------------------------------
printf -- '\n-- arm 5: plan + fetch + BUILD inside a host with neither -----\n'
ENVROOT="$ENV_ROOT"      # lib.sh, out of internal/cfg/cfg.go. T-070.
if [ ! -d "$ENVROOT" ]; then
  exp_skip "arm5: the end-to-end build" "no $ENVROOT (./pgb --engine chroot env create)"
else
  HOMEDIR="$WORK/home"
  mkdir -p "$HOMEDIR"
  # uid 12000 exists in no image here, so `id -u` inside is unambiguous.
  chown -R 12000:12000 "$HOMEDIR" 2>/dev/null || true
  # ⚠ HARNESS PLUMBING, NAMED SO IT IS NOT MISTAKEN FOR THE RESULT. This
  # environment's outbound HTTPS goes through a proxy whose CA bundle lives
  # under /root, which uid 12000 cannot read -- and the first run of this arm
  # failed on exactly that, reported by `nix-fetch` as "hydra has no finished
  # build for jq". A readable copy travels in with the rest of the harness.
  _ca=""
  for v in "${CURL_CA_BUNDLE:-}" "${SSL_CERT_FILE:-}" "${REQUESTS_CA_BUNDLE:-}" \
           /etc/ssl/certs/ca-certificates.crt; do
    [ -n "$v" ] && [ -f "$v" ] && { _ca="$v"; break; }
  done
  if [ -n "${_ca:-}" ] && [ -f "$_ca" ]; then
    cp "$_ca" "$HOMEDIR/ca-bundle.crt" && chmod 0644 "$HOMEDIR/ca-bundle.crt"
    exp_note "harness: CA bundle copied in from $_ca (uid 12000 cannot read /root)"
  fi

  cat > "$WORK/inner.sh" <<'INNER'
#!/bin/sh
# Runs INSIDE the rootfs, as uid 12000, with a PATH that has no nix on it.
set -u
export HOME=/nonixhome
export PGB_STATE=/nonixhome/state
export NIX_PREFIX=/nonixhome/prefix
export NIX_FETCH_CACHE=/nonixhome/nixcache
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
if [ -f /nonixhome/ca-bundle.crt ]; then
  CURL_CA_BUNDLE=/nonixhome/ca-bundle.crt; SSL_CERT_FILE=/nonixhome/ca-bundle.crt
  export CURL_CA_BUNDLE SSL_CERT_FILE
fi
printf 'uid=%s\n' "$(id -u)"
printf 'nix-on-path=%s\n' "$(command -v nix >/dev/null 2>&1 && echo yes || echo no)"
printf 'nix-dir=%s\n' "$([ -d /nix ] && echo present || echo absent)"
printf 'cc=%s\n' "$(command -v cc >/dev/null 2>&1 && echo yes || echo no)"
mkdir -p "$PGB_STATE" || exit 3
/repo/pgb --engine host nix build jq --out /nonixhome/jqbuild
INNER
  cp "$WORK/inner.sh" "$HOMEDIR/inner.sh"; chmod +x "$HOMEDIR/inner.sh"
  chown -R 12000:12000 "$HOMEDIR" 2>/dev/null || true

  # ⚠ The chroot is the HARNESS, not the claim. It is how a host with no nix
  # and no /nix is produced on a machine that has both; everything asserted
  # below is about what the process INSIDE could see and do, and it drops to
  # an unprivileged uid before pgb is reached.
  timeout 5400 "$REPO_DIR/pgb" rootfs run "$ENVROOT" \
      --bind "$REPO_DIR:/repo" --bind "$HOMEDIR:/nonixhome" \
      -- /bin/sh -c 'exec setpriv --reuid 12000 --regid 12000 --clear-groups /bin/sh /nonixhome/inner.sh' \
      >"$WORK/inner.log" 2>&1
  inner_rc=$?
  tail -25 "$WORK/inner.log" | sed 's/^/        /'
  exp_check "arm5: uid inside is not root"      "$(sed -n 's/^uid=//p' "$WORK/inner.log" | head -1)" "12000"
  exp_check "arm5: nix on PATH inside"          "$(sed -n 's/^nix-on-path=//p' "$WORK/inner.log" | head -1)" "no"
  exp_check "arm5: /nix inside"                 "$(sed -n 's/^nix-dir=//p' "$WORK/inner.log" | head -1)" "absent"
  exp_check "arm5: a C compiler inside"         "$(sed -n 's/^cc=//p' "$WORK/inner.log" | head -1)" "yes"
  exp_check "arm5: the build exited cleanly"    "$inner_rc" "0"
  BIN=$(find "$HOMEDIR/jqbuild/out" -maxdepth 2 -type f -name jq 2>/dev/null | head -1)
  if [ -n "$BIN" ]; then
    exp_check "arm5: a jq binary came out"      "yes" "yes"
    exp_check "arm5: and it is static (no PT_INTERP)" \
      "$(readelf -l "$BIN" 2>/dev/null | grep -c INTERP)" "0"
    out=$("$BIN" -r '.a[1]' 2>&1 <<'JSON'
{"a":["x","é中"]}
JSON
)
    exp_check "arm5: and it answers a real query" "$out" "é中"
  else
    exp_check "arm5: a jq binary came out" "no" "yes"
  fi
fi

{
  printf '88 - plan, fetch and BUILD a nixpkgs package with no nix, no root\n\n'
  printf 'arm 1, name -> derivation over 83-\047s own twenty packages:\n'
  printf '  hydra route          %s resolved, %s not\n' "$hyd_ok" "$hyd_no"
  printf '  narinfo Deriver route %s resolved, %s not\n' "$drv_ok" "$drv_no"
  printf '\narm 2, the control: drvpath against a local nix-instantiate\n'
  printf '  identical: %s   differing: %s\n' "${agree:-skipped}" "${disagree:-skipped}"
  printf '\narm 3, no-nix plan vs evaluated plan, field by field: %s\n' "${same:-skipped}"
  printf '\narm 4, the platform defect:\n'
  printf '  store paths named nix-2.35.2 in one index: %s\n' "$dar"
  printf '  attr jq -> System %s, OutputName %s\n' "$sysline" "$outn"
  printf '  attr bash -> Name %s\n' "$bashname"
  printf '\narm 5, inside a host with a toolchain and neither nix nor root:\n'
  printf '  uid=%s nix-on-path=%s /nix=%s build-rc=%s\n' \
    "$(sed -n 's/^uid=//p' "$WORK/inner.log" 2>/dev/null | head -1)" \
    "$(sed -n 's/^nix-on-path=//p' "$WORK/inner.log" 2>/dev/null | head -1)" \
    "$(sed -n 's/^nix-dir=//p' "$WORK/inner.log" 2>/dev/null | head -1)" \
    "${inner_rc:-n/a}"
  printf '  binary: %s\n' "${BIN:-none}"
  printf '\nconditions: %s, %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(uname -sr)"
} > "$EXP_OUT/RESULT.txt"

exp_finish
