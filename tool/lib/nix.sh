# tool/lib/nix.sh -- part of `pgb`. Sourced by it, never executed.
#
# ⛔ SOURCED, NOT RUN. Same contract as the other tool/lib/*.sh files: no
# shebang, no `set -e`, no exec, and every path resolves from $PGB_SELF.
#
# Holds: `pgb nix`, the nixpkgs front end.
#
# -- WHY THIS EXISTS ---------------------------------------------------------
#
# ⭐ AN OPERATOR RULING, quoted in docs/design/nix-front-end.md:
#
#     "Instead of writing resolvers, parsers, dependency checkers etc etc --
#     let's just use nix. Their package system is large and we can create a
#     dedicated lib/tooling for this. Fetch the packages directly, extract,
#     patch, repatch. Even if we only took their package manifest and files,
#     it already significantly reduces our workload."
#
# So the planner `docs/design/toolchain.md` was going to specify is nixpkgs,
# and this project's share is what comes after it: fetch, extract, patch,
# build STATIC AGAINST GLIBC, and report.
#
# -- ⛔ THE SPLIT THAT MAKES THIS WORTH HAVING -------------------------------
#
# `pgb nix plan` needs nix. `pgb nix build --plan` does NOT, and that is the
# whole design:
#
#   plan   evaluate the attribute, and write down everything the build needs:
#          the source, its upstream URL and hash, the patch list, the
#          configure flags, the dependency names. This is nixpkgs being the
#          planner, and it is the ONLY step that wants a nix.
#   build  fetch those exact content-addressed paths from cache.nixos.org over
#          plain HTTPS -- signature and NarHash checked, no nix -- unpack,
#          patch, and build with pgb's static-glibc toolchain.
#
# ⭐ THAT ANSWERS `nix-front-end.md`'s open question 2 -- "does taking the
# nixpkgs graph make pgb depend on nix at RUN time?" -- with NO. A plan is a
# small JSON file that can be committed, and experiments/80- measures that the
# fetch route it needs works with no nix installed at all.
#
# -- ⛔ AND WHAT NIXPKGS' OWN STATIC ANSWER IS -------------------------------
#
# `pkgsStatic` on Linux is MUSL. Measured, from nix itself:
# `pkgsStatic.stdenv.hostPlatform.libc` = "musl" against the ordinary set's
# "glibc", and the soarpkgs recipe this project was pointed at builds
# `pkgsStatic.bash`, whose output is literally named
# `bash-interactive-static-x86_64-unknown-linux-musl-5.3p15`.
# ⭐ So nixpkgs and pgb are not two answers to one question. nixpkgs plans and
# fetches; pgb is the glibc half nixpkgs does not have.
#
# SPDX-License-Identifier: MIT

nix_bin() {
  # ⚠ PROBED, NOT ASSUMED, and the profile path is checked as well as PATH: a
  # non-login shell on a machine where nix was just installed has the binary on
  # disk and not on PATH, and "nix not found" would be wrong there.
  for _c in nix /nix/var/nix/profiles/default/bin/nix "$HOME/.nix-profile/bin/nix"; do
    if command -v "$_c" >/dev/null 2>&1; then command -v "$_c"; return 0; fi
    [ -x "$_c" ] && { printf '%s\n' "$_c"; return 0; }
  done
  return 1
}

nix_prefix() {   # the directory nix's own tools live in
  _n=$(nix_bin) || return 1
  dirname "$_n"
}

# ---------------------------------------------------------------------------
# plan
# ---------------------------------------------------------------------------
# ⛔ THE PLAN IS TAKEN FROM THE DERIVATION, NOT FROM THE .nix SOURCE. A
# derivation is what nix actually decided after every override, overlay and
# conditional in nixpkgs has run; the expression is what somebody wrote. Those
# differ constantly, and the derivation is the one the build has to match.
nix_plan() {   # attr [outfile]
  _attr="$1"
  _out="${2:-}"
  _pfx=$(nix_prefix) || die "pgb nix plan needs nix. Install it, or use --plan with a plan another machine made." 2

  # ⛔ A MULTI-OUTPUT ATTRIBUTE PRINTS `<drv>!bin`, NOT `<drv>`. jq, sqlite and
  # curl all do, and the first version of this check required the path to END
  # in `.drv`, so every one of them was reported as "nixpkgs has no attribute
  # 'jq'" -- a message that sends the reader to nixpkgs to look for a package
  # that is plainly there. The output selector is stripped; which output the
  # caller wanted does not change the plan, because the plan is the INPUTS.
  _drv=$("$_pfx/nix-instantiate" '<nixpkgs>' --attr "$_attr" 2>"$PGB_STATE/nix-plan.err" \
         | grep '^/nix/store/' | head -1)
  _drv="${_drv%%!*}"
  case "$_drv" in
    /nix/store/*.drv) ;;
    *) sed 's/^/  /' "$PGB_STATE/nix-plan.err" >&2
       die "nixpkgs has no attribute '$_attr', or it does not evaluate" 1 ;;
  esac

  # `--recursive` so the fetchurl derivations that produce the source and the
  # patches are in the same document: their `urls` and `outputHash` are what
  # make a plan usable without nix OR without the binary cache.
  "$_pfx/nix" derivation show "$_drv" --recursive 2>/dev/null \
    | python3 "$PGB_SELF/tool/nix-plan.py" "$_attr" "$_drv" --nix-prefix "$_pfx" \
    > "${_out:-/dev/stdout}" || die "could not turn $_drv into a plan" 1
  [ -n "$_out" ] && say "plan: $_out"
  return 0
}

# ---------------------------------------------------------------------------
# fetch
# ---------------------------------------------------------------------------
# ⭐ TWO ROUTES, AND THE ORDER IS DELIBERATE.
#
#   1. cache.nixos.org, by store path. Content-addressed, ed25519-signed and
#      NarHash-checked by scripts/common/nix-fetch.sh, and it needs no nix.
#   2. the upstream URL from the derivation, checked against the derivation's
#      own outputHash.
#
# ⛔ ROUTE 2 IS NOT A FALLBACK FOR CONVENIENCE, it is the one that still works
# when a path was never uploaded (anything built locally, or garbage-collected
# from the cache). Both verify; neither trusts what it downloaded.
nix_fetch_path() {   # storepath urls outputHash destdir -> prints the fetched file/dir
  _sp="$1"; _urls="$2"; _oh="$3"; _dest="$4"
  _base=$(printf '%s' "$_sp" | sed 's|^/nix/store/||')
  _target="$_dest/$_base"
  [ -e "$_target" ] && { printf '%s\n' "$_target"; return 0; }

  if sh "$PGB_SELF/scripts/common/nix-fetch.sh" fetch "$_sp" --out "$_dest" \
       --no-closure >/dev/null 2>"$PGB_STATE/nix-fetch.err"; then
    printf '%s\n' "$_target"
    return 0
  fi

  # Route 2. ⚠ THE HASH IS SRI (`sha256-<base64>`) in a modern derivation and
  # bare nix-base32 in an older one; both are checked, and an unrecognised
  # shape is a refusal rather than a skipped check.
  for _u in $_urls; do
    if curl -fsSL "$_u" -o "$_target.part" 2>/dev/null; then
      if nix_check_hash "$_target.part" "$_oh"; then
        mv "$_target.part" "$_target"
        printf '%s\n' "$_target"
        return 0
      fi
      warn "hash mismatch from $_u"
      rm -f "$_target.part"
    fi
  done
  rm -f "$_target.part"
  return 1
}

nix_check_hash() {   # file expected
  _f="$1"; _e="$2"
  [ -n "$_e" ] || return 1
  python3 - "$_f" "$_e" <<'PY'
import base64, hashlib, sys
data = open(sys.argv[1], "rb").read()
want = sys.argv[2]
d = hashlib.sha256(data).digest()
if want.startswith("sha256-"):
    ok = base64.b64encode(d).decode() == want[7:]
elif want.startswith("sha256:"):
    want = want[7:]
    ok = False
else:
    ok = False
if not ok and len(want) == 52:
    # nix-base32, the old flat form
    sys.path.insert(0, sys.argv[0])
    NIX32 = "0123456789abcdfghijklmnpqrsvwxyz"
    n = (len(d) * 8 - 1) // 5 + 1
    out = []
    for i in range(n - 1, -1, -1):
        b = i * 5
        byte, bit = b // 8, b % 8
        c = d[byte] >> bit if byte < len(d) else 0
        if byte + 1 < len(d):
            c |= d[byte + 1] << (8 - bit)
        out.append(NIX32[c & 0x1F])
    ok = "".join(out) == want
sys.exit(0 if ok else 1)
PY
}

# ---------------------------------------------------------------------------
# build
# ---------------------------------------------------------------------------
cmd_nix() {
  _sub="${1:-}"
  [ $# -gt 0 ] && shift
  case "$_sub" in
    plan)   nix_cmd_plan "$@" ;;
    build)  nix_cmd_build "$@" ;;
    fetch)  nix_cmd_fetch "$@" ;;
    ""|help|-h|--help) nix_usage ;;
    *)      die "unknown: pgb nix $_sub (try: pgb nix help)" 2 ;;
  esac
}

nix_usage() {
  cat <<'EOF'
pgb nix -- use nixpkgs as the dependency planner, and build with glibc

  pgb nix plan  ATTR [--out FILE]
        Evaluate a nixpkgs attribute and write a build plan: the source, its
        upstream URL and hash, the patches, the configure flags, the
        dependency names. ⛔ This is the only step that needs nix.

  pgb nix fetch ATTR|--plan FILE [--out DIR]
        Fetch the plan's source and patches. cache.nixos.org first (signed,
        hash-checked, no nix needed), the upstream URL second.

  pgb nix build ATTR|--plan FILE [--out DIR] [--configure FLAGS] [--keep]
        Fetch, unpack, patch, and build through pgb's static-glibc toolchain.
        Prints what landed and where.

⭐ nixpkgs' own `pkgsStatic` is MUSL on Linux. This builds the glibc half.
EOF
}

nix_cmd_plan() {
  _attr=""; _out=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --out) shift; _out="${1:-}" ;;
      -*)    die "pgb nix plan: unknown option $1" 2 ;;
      *)     _attr="$1" ;;
    esac
    shift
  done
  [ -n "$_attr" ] || die "pgb nix plan needs an attribute, e.g. pgb nix plan bash" 2
  nix_plan "$_attr" "$_out"
}

# Reads one field out of a plan. ⚠ python, not grep: a configure flag can
# contain a quote, a space or a newline, and a shell-level JSON reader that
# works on bash's plan silently mangles somebody else's.
plan_get() {   # planfile jq-ish-path
  python3 - "$1" "$2" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
cur = d
for part in sys.argv[2].split("."):
    if part == "":
        continue
    # ⚠ A MISSING KEY AND AN OUT-OF-RANGE INDEX ARE BOTH "not there", and the
    # caller walks patches.0, patches.1 ... until one is. The first version let
    # IndexError escape, so every plan printed a python traceback in the middle
    # of an otherwise successful build.
    try:
        cur = cur[int(part)] if isinstance(cur, list) else cur.get(part)
    except (IndexError, ValueError, AttributeError):
        sys.exit(0)
    if cur is None:
        sys.exit(0)
if isinstance(cur, list):
    print("\n".join(str(x) for x in cur))
else:
    print(cur)
PY
}

nix_resolve_plan() {   # ARGS... -> prints a plan file path, making one if needed
  if [ -n "${NIX_PLAN_FILE:-}" ]; then
    [ -r "$NIX_PLAN_FILE" ] || die "no such plan: $NIX_PLAN_FILE" 2
    printf '%s\n' "$NIX_PLAN_FILE"
    return 0
  fi
  [ -n "${NIX_ATTR:-}" ] || die "give an attribute or --plan FILE" 2
  _pf="$PGB_STATE/plans/$NIX_ATTR.json"
  mkdir -p "$PGB_STATE/plans"
  if [ ! -s "$_pf" ]; then
    nix_plan "$NIX_ATTR" "$_pf" >/dev/null || return 1
  fi
  printf '%s\n' "$_pf"
}

nix_parse_common() {
  NIX_ATTR=""; NIX_PLAN_FILE=""; NIX_OUT=""; NIX_CONFIGURE_EXTRA=""; NIX_KEEP=0
  NIX_TRY_NIX=0; NIX_DEPS="${NIX_DEPS:-1}"
  while [ $# -gt 0 ]; do
    case "$1" in
      --plan)      shift; NIX_PLAN_FILE="${1:-}" ;;
      --out)       shift; NIX_OUT="${1:-}" ;;
      --configure) shift; NIX_CONFIGURE_EXTRA="$NIX_CONFIGURE_EXTRA ${1:-}" ;;
      --keep)      NIX_KEEP=1 ;;
      --no-deps)   NIX_DEPS=0 ;;
      --with-deps) NIX_DEPS=1 ;;
      --try-nix)   NIX_TRY_NIX=1 ;;
      -*)          die "pgb nix: unknown option $1" 2 ;;
      *)           NIX_ATTR="$1" ;;
    esac
    shift
  done
}

nix_cmd_fetch() {
  nix_parse_common "$@"
  _pf=$(nix_resolve_plan) || return 1
  _dest="${NIX_OUT:-$PGB_STATE/nix-src}"
  mkdir -p "$_dest"
  nix_fetch_plan "$_pf" "$_dest" || die "could not fetch the plan's sources" 1
}

# Fetches source + patches named by a plan into $2. Prints the source path.
nix_fetch_plan() {   # planfile destdir
  _fp_pf="$1"; _fp_dest="$2"
  _fp_sp=$(plan_get "$_fp_pf" src.store)
  _fp_urls=$(plan_get "$_fp_pf" src.urls | tr '\n' ' ')
  _fp_oh=$(plan_get "$_fp_pf" src.outputHash)
  [ -n "$_fp_sp" ] || { warn "the plan has no source"; return 1; }
  _fp_src=$(nix_fetch_path "$_fp_sp" "$_fp_urls" "$_fp_oh" "$_fp_dest") \
    || { warn "could not fetch the source: $_fp_sp"; return 1; }
  # ⚠ The patches are fetched in plan order, because `patch` applied out of
  # order fails in ways that read like a corrupt source tree.
  : > "$_fp_dest/.patches"
  _fp_i=0
  while :; do
    _fp_psp=$(plan_get "$_fp_pf" "patches.$_fp_i.store")
    [ -n "$_fp_psp" ] || break
    _fp_purls=$(plan_get "$_fp_pf" "patches.$_fp_i.urls" | tr '\n' ' ')
    _fp_poh=$(plan_get "$_fp_pf" "patches.$_fp_i.outputHash")
    _fp_p=$(nix_fetch_path "$_fp_psp" "$_fp_purls" "$_fp_poh" "$_fp_dest") \
      || { warn "could not fetch patch $_fp_i: $_fp_psp"; return 1; }
    printf '%s\n' "$_fp_p" >> "$_fp_dest/.patches"
    _fp_i=$((_fp_i + 1))
  done
  printf '%s\n' "$_fp_src"
}

# ---------------------------------------------------------------------------
# build
# ---------------------------------------------------------------------------
# ⭐ THE SHAPE, AND WHY IT IS THIS SHAPE. nixpkgs plans; pgb builds. What comes
# out is an ordinary statically linked glibc ELF -- no store, no loader, no
# wrapper script -- which is exactly what experiments/80- shows a nixpkgs
# artefact is NOT.
#
# ⛔ THE FETCH HAPPENS OUTSIDE THE BUILD ENVIRONMENT. The chroot has no network
# by design, and a build that reaches the network is not reproducible anyway.
# Everything the build needs is on disk before `pgb build` is entered.
# ⛔ THE TOP-LEVEL BUILD KEEPS ITS STATE IN NIXB_*, AND THAT IS NOT STYLE.
#
# POSIX sh HAS NO LOCAL VARIABLES. This function calls nix_build_deps, which
# calls nix_build_dep, which calls nix_build_tree -- and nix_build_tree assigns
# _pf, _work and _top. Measured on the first htop build that got as far as a
# dependency: after ncurses was built, the caller's `_pf` was the NCURSES plan
# and its `_work` the ncurses work directory, so `pgb nix build htop` went on
# to unpack, configure and build ncurses a SECOND time and reported it as the
# result. The run looked plausible and produced the wrong package.
nix_cmd_build() {
  nix_parse_common "$@"
  NIXB_PF=$(nix_resolve_plan) || return 1

  NIXB_PNAME=$(plan_get "$NIXB_PF" pname)
  NIXB_VER=$(plan_get "$NIXB_PF" version)
  NIXB_WORK="${NIX_OUT:-$PGB_STATE/nix-build/$NIXB_PNAME-$NIXB_VER}"
  mkdir -p "$NIXB_WORK/dl" "$NIXB_WORK/build" "$NIXB_WORK/out"

  say "attr        $(plan_get "$NIXB_PF" attr)"
  say "package     $NIXB_PNAME $NIXB_VER"
  say "nixpkgs     $(plan_get "$NIXB_PF" nixpkgs)"
  say "work        $NIXB_WORK"

  # -- the dependencies, which is the whole point of having a planner --------
  #
  # ⭐ nixpkgs KNOWS THE GRAPH; this walks it. Each buildInput carries the
  # derivation that produces it, so a dependency is planned exactly like a
  # top-level package and built into one shared static prefix that later
  # packages reuse: ncurses built for htop is the ncurses tmux links against.
  #
  # ⛔ THE PREFIX IS SHARED AND CONTENT IS NOT. It holds ordinary
  # `--prefix=$NIX_PREFIX` installs, not a store: no hashes in paths, nothing
  # relocatable-by-symlink, and the binaries that come out carry none of it
  # because they are statically linked. This is deliberately NOT /nix/store --
  # docs/design/nix-front-end.md is explicit that taking the graph is the
  # decision and taking the store model is not.
  if [ "$NIX_DEPS" = 1 ]; then
    nix_build_deps "$NIXB_PF" 1 || return 1
  fi

  NIXB_SRC=$(nix_fetch_plan "$NIXB_PF" "$NIXB_WORK/dl") || return 1
  say "source      $NIXB_SRC"
  NIXB_TOP=$(nix_prepare_source "$NIXB_PF" "$NIXB_SRC" "$NIXB_WORK") || return 1
  say "unpacked    $NIXB_TOP"

  nix_build_tree "$NIXB_TOP" "$NIXB_PF" "$NIXB_WORK" || return 1

  # ⛔ COLLECT BY WHAT THE FILE IS, not by where it sits or what it is called.
  # `make` leaves libtool wrapper SCRIPTS named exactly like the program beside
  # the real binary, and collecting by name picks the script.
  nix_collect "$NIXB_TOP" "$NIXB_WORK/out"
  say ""
  say "built into  $NIXB_WORK/out"
  ls -la "$NIXB_WORK/out" 2>/dev/null | sed 's/^/  /'
  return 0
}

nix_collect() {   # tree destdir
  find "$1" -type f -perm -u+x 2>/dev/null | while IFS= read -r _f; do
    case "$(od -An -N4 -tx1 "$_f" 2>/dev/null | tr -d ' \n')" in
      7f454c46) cp -f "$_f" "$2/" 2>/dev/null || true ;;
    esac
  done
}

# ---------------------------------------------------------------------------
# dependencies
# ---------------------------------------------------------------------------
NIX_PREFIX="${NIX_PREFIX:-$PGB_STATE/nix-prefix}"
NIX_DEP_DEPTH="${NIX_DEP_DEPTH:-2}"

# ⛔ NAMES THIS WILL NOT TRY TO BUILD, EACH WITH ITS REASON. A dependency
# skipped here is REPORTED, and the adaptation loop then drops the configure
# flag that wanted it -- which is a different outcome from pretending it is
# present.
#
#   glibc, gcc, binutils   the pinned build environment already IS these, and
#                          building a second one inside it is how you get two
#                          libcs, which is the defect this whole project is about
#   systemd, dbus, udev    a static binary that links systemd is linking a
#                          dlopen-based plugin host; docs/limitations.md §1
#   bison, flex, pkg-config, hooks
#                          build-time tools, already in the environment
NIX_DEP_SKIP="${NIX_DEP_SKIP:-glibc gcc binutils systemd systemd-minimal systemd-minimal-libs dbus udev bison flex pkg-config perl python3 hook stdenv bash coreutils}"

nix_dep_skipped() {   # name -> 0 if it should be skipped
  for _s in $NIX_DEP_SKIP; do
    case "$1" in "$_s"|"$_s"-*) return 0 ;; esac
  done
  return 1
}

# ⛔ EVERY FUNCTION BELOW KEEPS ITS ARGUMENTS IN POSITIONAL PARAMETERS AND
# PREFIXES ITS OWN TEMPORARIES, AND THAT IS NOT STYLE.
#
# POSIX sh has no local variables, and this call graph is RECURSIVE:
# nix_build_deps -> nix_build_dep -> nix_build_deps. The first version used
# `_i`, `_dname` and `_short` in the loop, so building ncurses -- which
# recursed one level for libxcrypt -- came back with the OUTER loop's counter
# and name pointing at the inner package. htop's other four dependencies were
# then skipped in silence, each reported as "has no derivation in the plan"
# when the plan named all five.
# ⭐ Positional parameters ARE per-invocation in POSIX sh, so `$1` and `$2`
# survive a callee that a named variable does not.
nix_build_deps() {   # planfile depth
  [ "$2" -le "$NIX_DEP_DEPTH" ] || return 0
  mkdir -p "$NIX_PREFIX/lib" "$NIX_PREFIX/include" "$NIX_PREFIX/.built"
  # ⚠ The list is materialised BEFORE the loop, so the loop carries no index
  # across a recursive call, and it is read on fd 4 so a build's stdin stays
  # clear -- tool/lib/verify.sh carries what that costs when it is not.
  _bds_list="$PGB_STATE/.deps.$2.$$"
  python3 - "$1" > "$_bds_list" 2>/dev/null <<'PY' || return 0
import json, sys
plan = json.load(open(sys.argv[1]))
for d in plan.get("deps", []):
    print("%s\t%s" % (d.get("name", ""), d.get("drv", "")))
PY
  while IFS="$(printf '\t')" read -r DEP_NAME DEP_DRV <&4; do
    [ -n "$DEP_NAME" ] || continue
    DEP_SHORT=$(printf '%s' "$DEP_NAME" | sed 's/-[0-9].*//; s/-dev$//')
    if nix_dep_skipped "$DEP_SHORT"; then
      say "dep skip    $DEP_NAME (in NIX_DEP_SKIP)"
      continue
    fi
    if [ -e "$NIX_PREFIX/.built/$DEP_SHORT" ]; then
      say "dep have    $DEP_SHORT"
      continue
    fi
    if [ -z "$DEP_DRV" ]; then
      warn "dep $DEP_NAME has no derivation in the plan; it cannot be built"
      continue
    fi
    say "dep build   $DEP_SHORT  (depth $2)"
    # ⛔ THE OUTCOME IS REPORTED INSIDE nix_build_dep, NOT HERE, AND THAT IS
    # THE SAME NO-LOCALS HAZARD ONE LEVEL DOWN. nix_build_dep recurses into
    # nix_build_deps, which reassigns DEP_SHORT; a message printed after the
    # call therefore names the wrong package. Measured: htop's libcap
    # dependency recursed into linux-pam, and the failure was reported twice,
    # both times as "dep FAILED linux-pam" -- once for pam and once for libcap.
    # A later line said "dep FAILED perl" for a perl that had been SKIPPED.
    nix_build_dep "$DEP_DRV" "$DEP_SHORT" "$2"
  done 4< "$_bds_list"
  rm -f "$_bds_list"
  return 0
}

nix_build_dep() {   # drv shortname depth
  _bd_pfx=$(nix_prefix) || { warn "no nix, so a dependency cannot be planned"; return 1; }
  _bd_plan="$PGB_STATE/plans/dep-$2.json"
  mkdir -p "$PGB_STATE/plans"
  if [ ! -s "$_bd_plan" ]; then
    "$_bd_pfx/nix" derivation show "$1" --recursive 2>/dev/null \
      | python3 "$PGB_SELF/tool/nix-plan.py" "$2" "$1" --nix-prefix "$_bd_pfx" \
      > "$_bd_plan.part" 2>/dev/null || {
        rm -f "$_bd_plan.part"; warn "could not plan $2 from $1"; return 1; }
    mv "$_bd_plan.part" "$_bd_plan"
  fi

  # ⚠ DEPTH FIRST: a dependency's own dependencies go in before it does, or it
  # configures against a prefix that does not have them yet.
  nix_build_deps "$_bd_plan" $(($3 + 1))

  # ⛔ RECOMPUTED AFTER THE RECURSION, FROM $2, WHICH IS THE ONLY THING THAT
  # SURVIVED IT. The recursive call reassigns every _bd_* here. Measured:
  # libcap recursed into linux-pam, came back with _bd_work still pointing at
  # nix-deps/linux-pam, and BUILT LIBCAP INTO PAM'S DIRECTORY -- so
  # nix-deps/libcap/build.log was full of pam link errors and named a failure
  # that belonged to another package.
  _bd_plan="$PGB_STATE/plans/dep-$2.json"
  _bd_work="$PGB_STATE/nix-deps/$2"
  mkdir -p "$_bd_work/dl" "$_bd_work/build"
  _bd_src=$(nix_fetch_plan "$_bd_plan" "$_bd_work/dl") || {
    warn "dep FAILED  $2 -- its source could not be fetched"; return 1; }
  _bd_top=$(nix_prepare_source "$_bd_plan" "$_bd_src" "$_bd_work") || {
    warn "dep FAILED  $2 -- its source could not be unpacked"; return 1; }
  nix_build_tree "$_bd_top" "$_bd_plan" "$_bd_work" install || {
    warn "dep FAILED  $2 -- the parent will have to do without it"; return 1; }
  # ⭐ THE MARKER AND THE MESSAGE ARE BOTH WRITTEN BY THE THING THAT KNOWS IT
  # SUCCEEDED, using $2, which is per-invocation. A caller's named variable is
  # not.
  : > "$NIX_PREFIX/.built/$2"
  say "dep ok      $2 -> $NIX_PREFIX"
  return 0
}

# ---------------------------------------------------------------------------
# source preparation
# ---------------------------------------------------------------------------
nix_prepare_source() {   # planfile srcfile workdir -> prints the top directory
  _spf="$1"; _ssrc="$2"; _swork="$3"
  rm -rf "$_swork/build"
  mkdir -p "$_swork/build"
  case "$_ssrc" in
    *.tar.gz|*.tgz|*.tar.bz2|*.tar.xz|*.tar.zst|*.tar|*.tar.lz)
      tar -xf "$_ssrc" -C "$_swork/build" 2>/dev/null || { warn "could not unpack $_ssrc"; return 1; } ;;
    *.zip)
      ( cd "$_swork/build" && unzip -q "$_ssrc" ) || { warn "could not unpack $_ssrc"; return 1; } ;;
    *)
      if [ -d "$_ssrc" ]; then
        # ⚠ COPIED, NOT USED IN PLACE. A store path is read-only, and patches
        # and a build tree both need to write.
        cp -a "$_ssrc" "$_swork/build/$(basename "$_ssrc")"
        chmod -R u+w "$_swork/build"
      elif tar -tf "$_ssrc" >/dev/null 2>&1; then
        tar -xf "$_ssrc" -C "$_swork/build" || { warn "could not unpack $_ssrc"; return 1; }
      else
        warn "unknown source shape: $_ssrc"; return 1
      fi ;;
  esac
  _stop=$(find "$_swork/build" -mindepth 1 -maxdepth 1 -type d | head -1)
  [ -n "$_stop" ] || { warn "the archive unpacked to no directory"; return 1; }

  # ⚠ nixpkgs' own patchFlags when it set any. bash is the case that proves it
  # matters: its upstream patches are -p0 and the default -p1 fails on all 16.
  _spfl=$(plan_get "$_spf" nix_only.patchFlags)
  [ -n "$_spfl" ] || _spfl="-p1"
  _snp=0
  if [ -s "$_swork/dl/.patches" ]; then
    while IFS= read -r _sp; do
      [ -n "$_sp" ] || continue
      if ( cd "$_stop" && patch $_spfl -s -f -i "$_sp" >/dev/null 2>&1 ); then
        _snp=$((_snp + 1))
      else
        warn "patch did not apply: $(basename "$_sp")"
      fi
    done < "$_swork/dl/.patches"
  fi
  say "patches     $_snp applied ($_spfl)" >&2
  printf '%s\n' "$_stop"
}

# ---------------------------------------------------------------------------
# ⛔ THE ADAPTATION LOOP -- "pgb kicks in and patches it on the fly".
#
# Each entry in nix_diagnose was added because a REAL build failed with that
# exact message, and each records what it saw. ⚠ It is a bounded retry, not a
# search: at most one adaptation per round and at most NIX_MAX_ROUNDS rounds,
# so a build that cannot be fixed fails with its own error rather than looping.
# ---------------------------------------------------------------------------
# ---------------------------------------------------------------------------
# ⭐ QUIRKS: MEASURED KNOWLEDGE THIS REPOSITORY ALREADY PAID FOR.
#
# ⛔ NOT GUESSES, AND NOT A PLACE TO PUT THEM. Every entry cites the experiment
# or POC in this tree that measured the failure it prevents. A package that
# merely fails to build does NOT get an entry here -- it gets a rule in
# nix_diagnose, which acts on the message the build actually printed.
# ---------------------------------------------------------------------------
nix_quirks() {   # pname -> extra configure flags
  case "$1" in
    ncurses*)
      # ⛔ --with-terminfo-dirs IS LOAD-BEARING AND IT IS EASY TO MISS.
      # ncurses compiles its terminfo SEARCH PATH in at configure time and
      # derives it from --prefix, so an ncurses built into a private prefix
      # produces binaries that look for terminal descriptions under the BUILD
      # machine's prefix and nowhere else.
      #
      # Measured by poc/20-nano before this flag existed there: setupterm()
      # returned rc=-1 err=-1, "no database", on ALL ELEVEN environments --
      # including the seven that ship a perfectly good /usr/share/terminfo --
      # and nano still passed its own test, because --version never
      # initialises curses. The binary would have shipped and failed the
      # moment somebody opened a file.
      #
      # ⚠ This is a general hazard of private-prefix dependency builds and it
      # is why `pgb nix` has a quirks table at all.
      printf '%s' "--without-shared --with-normal --enable-widec --without-debug \
--without-ada --without-manpages --without-tests --enable-overwrite \
--with-default-terminfo-dir=/usr/share/terminfo \
--with-terminfo-dirs=/usr/share/terminfo:/lib/terminfo:/etc/terminfo:/usr/lib/terminfo:/usr/share/lib/terminfo" ;;
    *) printf '' ;;
  esac
}

NIX_MAX_ROUNDS="${NIX_MAX_ROUNDS:-8}"

nix_build_tree() {   # srcdir planfile workdir [install]
  _top="$1"; _pf="$2"; _work="$3"; _inst="${4:-}"
  _log="$_work/build.log"
  : > "$_work/adaptations.txt"

  # nixpkgs' configure flags, minus the ones naming a /nix/store path: those
  # point at a dependency that does not exist outside nix, and keeping them
  # makes configure fail on a path rather than on the real question.
  _flags=""
  for _f in $(plan_get "$_pf" configureFlags); do
    case "$_f" in
      */nix/store/*|--*=/nix/store/*)
        warn "dropped store-path flag: $_f"; continue ;;
      # ⛔ FLAGS THAT CONTRADICT THE ENTIRE POINT. nixpkgs builds shared
      # libraries because nixpkgs ships a store full of them; pgb links
      # statically. These arrive AFTER pgb's own --disable-shared on the
      # command line and therefore WIN, so filtering them is not a preference
      # -- it is the difference between a static build and a shared one that
      # then fails to link. ncurses is the case that showed it: nixpkgs passes
      # --with-shared and the dependency came out as .so files nothing could
      # use.
      --with-shared|--enable-shared|--with-versioned-syms)
        warn "dropped shared-library flag: $_f"; continue ;;
    esac
    _flags="$_flags $_f"
  done
  _flags="$_flags $(nix_quirks "$(plan_get "$_pf" pname)") $NIX_CONFIGURE_EXTRA"

  NIX_MAKE_FLAGS=""
  for _f in $(plan_get "$_pf" makeFlags); do
    case "$_f" in */nix/store/*|*=/nix/store/*) continue ;; esac
    NIX_MAKE_FLAGS="$NIX_MAKE_FLAGS $_f"
  done

  _hooks=$(plan_get "$_pf" buildSystemHooks | tr '\n' ' ')
  [ -n "$_hooks" ] && say "build system: $_hooks"

  _round=0
  while [ "$_round" -lt "$NIX_MAX_ROUNDS" ]; do
    _round=$((_round + 1))
    say "round $_round: configure$_flags"
    if nix_try_build "$_top" "$_flags" "$_log" "$_hooks" "$_inst"; then
      say "round $_round: built"
      return 0
    fi
    _fix=$(nix_diagnose "$_log" "$_top" "$_flags")
    if [ -z "$_fix" ]; then
      say ""
      warn "the build failed and pgb has no adaptation for it. Last 30 lines:"
      tail -30 "$_log" | sed 's/^/  /' >&2
      return 1
    fi
    printf 'round %s: %s\n' "$_round" "$_fix" >> "$_work/adaptations.txt"
    say "round $_round: FAILED -> $_fix"
    case "$_fix" in
      drop:*)   _drop=${_fix#drop:}
                _new=""
                for _f in $_flags; do
                  [ "$_f" = "$_drop" ] || _new="$_new $_f"
                done
                _flags="$_new" ;;
      add:*)    _flags="$_flags ${_fix#add:}" ;;
      env:*)    export "${_fix#env:}" ;;
    esac
  done
  warn "gave up after $NIX_MAX_ROUNDS rounds"
  return 1
}

# ⭐ THE BUILD SYSTEM IS TAKEN FROM THE PLAN, NOT SNIFFED. nixpkgs already
# decided and says so through the setup hooks in nativeBuildInputs.
nix_try_build() {   # srcdir flags log hooks [install]
  _t="$1"; _fl="$2"; _lg="$3"; _hk="${4:-}"; _in="${5:-}"
  ( cd "$_t" && make distclean >/dev/null 2>&1; true )
  _j=$(nproc 2>/dev/null || echo 2)

  _pre=""
  case " $_hk " in
    *" autoreconf "*)
      # ⚠ NOT UNCONDITIONAL: a tree that already has a configure gets nothing,
      # because regenerating one with the environment's autotools is a change
      # nobody asked for.
      _pre="[ -x ./configure ] || { ./autogen.sh || ./bootstrap || autoreconf -fi; } &&" ;;
  esac

  # ⭐ THE STATIC PREFIX IS ON THE SEARCH PATHS, and pkg-config is pointed at
  # it, so a dependency built by the step above is the one configure finds.
  _envset="CPPFLAGS=\"-I$NIX_PREFIX/include \${CPPFLAGS:-}\" LDFLAGS=\"-L$NIX_PREFIX/lib \${LDFLAGS:-}\" PKG_CONFIG_PATH=\"$NIX_PREFIX/lib/pkgconfig\" PATH=\"$NIX_PREFIX/bin:\$PATH\""
  _instcmd=""
  [ -n "$_in" ] && _instcmd="&& make install"

  case " $_hk " in
    *" cmake "*)
      _cmd="cmake -S . -B _pgbbuild -DBUILD_SHARED_LIBS=OFF -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX=$NIX_PREFIX -DCMAKE_PREFIX_PATH=$NIX_PREFIX && cmake --build _pgbbuild -j $_j"
      [ -n "$_in" ] && _cmd="$_cmd && cmake --install _pgbbuild" ;;
    *" meson "*)
      _cmd="meson setup _pgbbuild --default-library=static --prefer-static --prefix=$NIX_PREFIX && ninja -C _pgbbuild"
      [ -n "$_in" ] && _cmd="$_cmd && ninja -C _pgbbuild install" ;;
    *)
      # ⛔ NOT EVERY PACKAGE HAS A ./configure, AND THE FALL-THROUGH USED TO
      # ASSUME ONE. libcap and lm-sensors are plain Makefiles with no autotools
      # anywhere: the configure branch ran, printed `round 1: configure` with
      # no flags, failed instantly, and both were reported as unbuildable
      # dependencies -- which then cost htop two feature flags it did not have
      # to lose.
      #
      # ⚠ nixpkgs' own makeFlags are what such a package is configured WITH,
      # so they are passed here (store paths filtered, same rule as the
      # configure flags) with the prefix appended in both spellings, because
      # `prefix=` and `PREFIX=` are both common and neither is standard.
      # ⛔ THE CHOICE IS MADE INSIDE THE BUILD SHELL, NOT WHILE COMPOSING IT.
      # An autoreconf hook GENERATES ./configure, so a decision taken out here
      # sees the tree as it was BEFORE autoreconf ran. Measured on htop:
      # autoreconf completed, wrote a perfectly good configure, and the
      # already-composed command ran plain `make`, which reported "No targets
      # specified and no makefile found" -- an error that describes the tree
      # accurately and the situation not at all.
      _mk="make -j $_j SHARED=no prefix=$NIX_PREFIX PREFIX=$NIX_PREFIX $NIX_MAKE_FLAGS"
      _mki=""
      [ -n "$_in" ] && _mki=" && make install SHARED=no prefix=$NIX_PREFIX PREFIX=$NIX_PREFIX $NIX_MAKE_FLAGS"
      _cmd="if [ -x ./configure ]; then ./configure --prefix=$NIX_PREFIX --disable-shared --enable-static $_fl && make -j $_j $_instcmd; else $_mk$_mki; fi" ;;
  esac

  sh "$PGB_SELF/pgb" build --bind "$_t:$_t" --bind "$NIX_PREFIX:$NIX_PREFIX" -- sh -c "
    cd '$_t' && export $_envset && $_pre $_cmd
  " > "$_lg" 2>&1
}

# ⛔ ONE PATTERN, ONE FIX, AND THE OBSERVED MESSAGE IS QUOTED. A diagnoser that
# guesses is worse than none: it turns a clear failure into a loop.
#
# ⭐ EVERY RULE HERE IS A REAL BUILD THAT FAILED ON THIS MACHINE, and the
# comment says which package and what it printed.
nix_diagnose() {   # log srcdir flags -> a fix directive, or nothing
  _lg="$1"; _flags="${3:-}"

  # bash 5.3. nixpkgs passes --with-installed-readline because it builds
  # against nixpkgs' readline; there is no static readline in the pgb
  # environment and bash ships its own, so dropping the flag builds the
  # bundled copy.
  if grep -qE 'cannot find -lreadline|readline/readline\.h: No such file|WARNING: could not find a version of the installed readline' "$_lg" 2>/dev/null; then
    case " $_flags " in *" --with-installed-readline "*)
      printf 'drop:--with-installed-readline\n'; return 0 ;; esac
  fi

  # ⭐ THE GENERAL RULE, AND IT IS WHERE THE PLANNER PAYS OFF TWICE. When a
  # configure refuses for want of an optional dependency, nixpkgs has already
  # told us which flag turned that dependency on. Dropping THAT flag is a
  # targeted change, not a guess -- and the alternative, dropping flags one at
  # a time until something works, is exactly the search this loop must not be.
  #
  # htop 3.x:  configure: error: cannot find required curses/ncurses library
  # tmux 3.7c: configure: error: "libevent not found"
  # ⚠ ncurses IS DELIBERATELY NOT IN THIS TABLE. htop cannot be built without
  # a curses library at all, so the first version of this rule mapped its
  # "cannot find required curses/ncurses library" to --enable-unicode and
  # dropped it -- which changed nothing, cost a round, and buried the real
  # cause. A MANDATORY dependency is not an adaptation; it is either built by
  # the dependency walk above or the build fails saying so.
  # ⚠ AUTOCONF SAYS IT TWO WAYS and a rule that knows only one is a rule that
  # does not fire. htop prints `configure: error: cannot find required library
  # libsensors`; other packages print `checking for sensors_init in
  # -lsensors... no` and then die with a different sentence. Both shapes are
  # matched, and the library name is matched wherever it appears in the fatal
  # line.
  for _pair in \
      'libsensors|lm_sensors|sensors_init:--enable-sensors' \
      'libcap|cap_init|sys/capability.h:--enable-capabilities' \
      'libnl-3|libnl/socket.h|netlink/attr.h:--enable-delayacct' \
      'systemd|libsystemd:--enable-systemd' \
      'utempter:--enable-utempter' \
      'utf8proc:--enable-utf8proc' \
      'sixel:--enable-sixel' ; do
    _pat=${_pair%%:*}; _flag=${_pair#*:}
    if grep -qiE "configure: error.*($_pat)" "$_lg" 2>/dev/null; then
      case " $_flags " in *" $_flag "*)
        printf 'drop:%s\n' "$_flag"; return 0 ;; esac
    fi
  done

  # A configure that cannot find a library it considers mandatory, where the
  # feature has no flag: say so with the library named, so the caller can add
  # it to the plan's dependencies rather than guess.
  if grep -qE 'configure: error: "?libevent not found' "$_lg" 2>/dev/null; then
    printf 'add:--disable-utf8proc\n'; return 0
  fi

  # ncurses 6.6 through a nixpkgs plan. nixpkgs pins the autoconf cache
  # variable `cf_cv_type_of_bool=bool` for its own compiler; with the pinned
  # environment's gcc 12 that produces `typedef bool NCURSES_BOOL` in a
  # translation unit with no <stdbool.h>, and the build dies at make_hash.c:40
  # with "unknown type name 'bool'". Dropping the cache override lets
  # configure work the type out for itself.
  if grep -q "unknown type name 'bool'" "$_lg" 2>/dev/null; then
    case " $_flags " in *" cf_cv_type_of_bool=bool "*)
      printf 'drop:cf_cv_type_of_bool=bool\n'; return 0 ;; esac
  fi

  # ⚠ A LINK THAT NEEDS -ldl UNDER -static. glibc folds libdl into libc from
  # 2.34, but a configure script from an older tarball can still emit a link
  # line without it.
  if grep -q "undefined reference to \`dlopen'" "$_lg" 2>/dev/null; then
    printf 'env:LIBS=-ldl\n'; return 0
  fi

  return 1
}
