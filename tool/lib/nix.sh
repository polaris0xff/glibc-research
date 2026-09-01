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
  NIX_TRY_NIX=0
  while [ $# -gt 0 ]; do
    case "$1" in
      --plan)      shift; NIX_PLAN_FILE="${1:-}" ;;
      --out)       shift; NIX_OUT="${1:-}" ;;
      --configure) shift; NIX_CONFIGURE_EXTRA="$NIX_CONFIGURE_EXTRA ${1:-}" ;;
      --keep)      NIX_KEEP=1 ;;
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
  _pf="$1"; _dest="$2"
  _sp=$(plan_get "$_pf" src.store)
  _urls=$(plan_get "$_pf" src.urls | tr '\n' ' ')
  _oh=$(plan_get "$_pf" src.outputHash)
  [ -n "$_sp" ] || die "the plan has no source" 1
  _src=$(nix_fetch_path "$_sp" "$_urls" "$_oh" "$_dest") \
    || die "could not fetch the source: $_sp" 1
  # ⚠ The patches are fetched here too, in plan order, because `patch` applied
  # out of order fails in ways that read like a corrupt source tree.
  : > "$_dest/.patches"
  _i=0
  while :; do
    _psp=$(plan_get "$_pf" "patches.$_i.store") || break
    [ -n "$_psp" ] || break
    _purls=$(plan_get "$_pf" "patches.$_i.urls" | tr '\n' ' ')
    _poh=$(plan_get "$_pf" "patches.$_i.outputHash")
    _p=$(nix_fetch_path "$_psp" "$_purls" "$_poh" "$_dest") \
      || die "could not fetch patch $_i: $_psp" 1
    printf '%s\n' "$_p" >> "$_dest/.patches"
    _i=$((_i + 1))
  done
  printf '%s\n' "$_src"
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
nix_cmd_build() {
  nix_parse_common "$@"
  _pf=$(nix_resolve_plan) || return 1

  _pname=$(plan_get "$_pf" pname)
  _ver=$(plan_get "$_pf" version)
  _work="${NIX_OUT:-$PGB_STATE/nix-build/$_pname-$_ver}"
  mkdir -p "$_work/dl" "$_work/build" "$_work/out"

  say "attr        $(plan_get "$_pf" attr)"
  say "package     $_pname $_ver"
  say "nixpkgs     $(plan_get "$_pf" nixpkgs)"
  say "work        $_work"

  _src=$(nix_fetch_plan "$_pf" "$_work/dl") || return 1
  say "source      $_src"

  # -- unpack ---------------------------------------------------------------
  # ⚠ THE TOP DIRECTORY IS READ FROM THE ARCHIVE, never guessed from the file
  # name. `foo-1.2.tar.gz` unpacking to `foo` rather than `foo-1.2` is common
  # and the guess fails silently into an empty build.
  rm -rf "$_work/build"
  mkdir -p "$_work/build"
  case "$_src" in
    *.tar.gz|*.tgz|*.tar.bz2|*.tar.xz|*.tar.zst|*.tar)
      tar -xf "$_src" -C "$_work/build" 2>/dev/null \
        || die "could not unpack $_src" 1 ;;
    *.zip)
      (cd "$_work/build" && unzip -q "$_src") || die "could not unpack $_src" 1 ;;
    *)
      # A store path that is already a directory (a fetchgit, say): copy it in
      # writable, because a store path is read-only and patches have to apply.
      if [ -d "$_src" ]; then
        cp -a "$_src" "$_work/build/$(basename "$_src")"
        chmod -R u+w "$_work/build"
      else
        die "unknown source shape: $_src" 1
      fi ;;
  esac
  _top=$(find "$_work/build" -mindepth 1 -maxdepth 1 -type d | head -1)
  [ -n "$_top" ] || die "the archive unpacked to no directory" 1
  say "unpacked    $_top"

  # -- patch ----------------------------------------------------------------
  # ⚠ nixpkgs' own patchFlags, when it set any. bash is the case that proves
  # this matters: its upstream patches are -p0 and the default -p1 fails on
  # every one of them.
  _pflags=$(plan_get "$_pf" nix_only.patchFlags)
  [ -n "$_pflags" ] || _pflags="-p1"
  _np=0
  if [ -s "$_work/dl/.patches" ]; then
    while IFS= read -r _p; do
      [ -n "$_p" ] || continue
      if (cd "$_top" && patch $_pflags -s -f -i "$_p" >/dev/null 2>&1); then
        _np=$((_np + 1))
      else
        warn "patch did not apply: $(basename "$_p")"
      fi
    done < "$_work/dl/.patches"
  fi
  say "patches     $_np applied ($_pflags)"

  # -- build ----------------------------------------------------------------
  nix_build_tree "$_top" "$_pf" "$_work" || return 1

  # -- collect --------------------------------------------------------------
  # ⛔ COLLECT BY WHAT THE FILE IS, not by where it sits. `make` leaves
  # libtool wrapper scripts named exactly like the program beside the real
  # binary, and copying by name picks the script.
  find "$_top" -type f -perm -u+x 2>/dev/null | while IFS= read -r _f; do
    case "$(head -c 4 "$_f" | tr -d '\0')" in
      "$(printf '\177')ELF")
        if head -c 20 "$_f" | od -An -tx1 | grep -q '02 00 3e'; then
          cp -f "$_f" "$_work/out/" 2>/dev/null || true
        fi ;;
    esac
  done
  say ""
  say "built into  $_work/out"
  ls -la "$_work/out" 2>/dev/null | sed 's/^/  /'
  return 0
}

# ⛔ THE ADAPTATION LOOP -- "pgb kicks in and patches it on the fly".
#
# Each entry below was added because a REAL build failed with that exact
# message, and each records what it saw. ⚠ It is a bounded retry, not a
# search: at most one adaptation per round and at most NIX_MAX_ROUNDS rounds,
# so a build that cannot be fixed fails with its own error rather than looping.
NIX_MAX_ROUNDS="${NIX_MAX_ROUNDS:-6}"

nix_build_tree() {   # srcdir planfile workdir
  _top="$1"; _pf="$2"; _work="$3"
  _log="$_work/build.log"
  : > "$_work/adaptations.txt"

  # nixpkgs' configure flags, minus the ones that name a /nix/store path: those
  # point at a dependency that does not exist outside nix, and keeping them
  # makes configure fail on a path rather than on the real question.
  _flags=""
  for _f in $(plan_get "$_pf" configureFlags); do
    case "$_f" in
      */nix/store/*|--*=/nix/store/*) warn "dropped store-path flag: $_f"; continue ;;
    esac
    _flags="$_flags $_f"
  done
  _flags="$_flags $NIX_CONFIGURE_EXTRA"

  _hooks=$(plan_get "$_pf" buildSystemHooks | tr '\n' ' ')
  [ -n "$_hooks" ] && say "build system: $_hooks"

  _round=0
  while [ "$_round" -lt "$NIX_MAX_ROUNDS" ]; do
    _round=$((_round + 1))
    say "round $_round: configure$_flags"
    if nix_try_build "$_top" "$_flags" "$_log" "$_hooks"; then
      say "round $_round: built"
      return 0
    fi
    _fix=$(nix_diagnose "$_log" "$_top")
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
# decided, and says so through the setup hooks in nativeBuildInputs:
# `autoreconf-hook` means the source has no ./configure and one must be
# generated; a cmake or meson hook names the generator outright. Sniffing the
# tree gets this wrong on any project that ships more than one build system,
# and htop is the case that proved it: a bare git export with configure.ac and
# no configure, where `make` reported "No targets specified".
nix_try_build() {   # srcdir flags log hooks
  _t="$1"; _fl="$2"; _lg="$3"; _hooks="${4:-}"
  ( cd "$_t" && make distclean >/dev/null 2>&1; true )
  _j=$(nproc 2>/dev/null || echo 2)

  _pre=""
  case " $_hooks " in
    *" autoreconf "*)
      # ⚠ NOT `autoreconf -i` UNCONDITIONALLY: a tree that already has a
      # configure gets nothing, because regenerating one with the build
      # environment's autotools is a change nobody asked for.
      _pre="[ -x ./configure ] || { autoreconf -fi || ./autogen.sh || ./bootstrap; } &&" ;;
  esac

  case " $_hooks " in
    *" cmake "*)
      _cmd="cmake -S . -B _pgbbuild -DBUILD_SHARED_LIBS=OFF -DCMAKE_BUILD_TYPE=Release $(plan_flags cmake) && cmake --build _pgbbuild -j $_j" ;;
    *" meson "*)
      _cmd="meson setup _pgbbuild --default-library=static --prefer-static && ninja -C _pgbbuild" ;;
    *)
      # ⛔ --disable-shared --enable-static ARE NOT OPTIONAL and they come
      # AFTER the plan's flags, so a package that already states one keeps its
      # own answer and everything else gets pgb's.
      _cmd="./configure --disable-shared --enable-static $_fl && make -j $_j" ;;
  esac

  sh "$PGB_SELF/pgb" build --bind "$_t:$_t" -- sh -c "
    cd '$_t' && $_pre $_cmd
  " > "$_lg" 2>&1
}

# Extra flags for a generator, from the plan. Kept small on purpose: nixpkgs'
# cmakeFlags routinely name /nix/store paths, and those are dropped upstream in
# nix_build_tree for the same reason the configure ones are.
plan_flags() { printf '%s' ""; }

# ⛔ ONE PATTERN, ONE FIX, AND THE OBSERVED MESSAGE IS QUOTED. A diagnoser that
# guesses is worse than none: it turns a clear failure into a loop.
nix_diagnose() {   # log srcdir -> a fix directive, or nothing
  _lg="$1"

  # bash 5.3, round 1. nixpkgs passes --with-installed-readline because it
  # builds against nixpkgs' readline; there is no static readline in the pgb
  # environment, and bash ships its own copy, so dropping the flag builds the
  # bundled one.
  if grep -q 'checking version of installed readline library' "$_lg" 2>/dev/null &&
     grep -qE 'configure: error.*readline|Bad or missing version|WARNING: could not find a version of the installed readline' "$_lg" 2>/dev/null; then
    printf 'drop:--with-installed-readline\n'; return 0
  fi
  if grep -q 'cannot find -lreadline\|readline/readline.h: No such file' "$_lg" 2>/dev/null; then
    printf 'drop:--with-installed-readline\n'; return 0
  fi

  # A configure that refuses because a static libc cannot dlopen: the honest
  # answer is to tell it so rather than to fight it.
  if grep -q 'cannot find -lncurses\|ncurses.h: No such file' "$_lg" 2>/dev/null; then
    printf 'add:--without-curses\n'; return 0
  fi

  # ⚠ A LINK THAT NEEDS -ldl UNDER -static. glibc's libdl is folded into libc
  # from 2.34, but a configure script that tests for it separately can still
  # emit a link line without it on an older tarball.
  if grep -q "undefined reference to \`dlopen'" "$_lg" 2>/dev/null; then
    printf 'env:LIBS=-ldl\n'; return 0
  fi

  return 1
}
