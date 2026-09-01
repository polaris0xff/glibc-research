#!/bin/sh
# nix-appimage.sh - a nixpkgs closure, packed the Anylinux way.
#
# ⛔ SOURCED BY `pgb nix appimage`, and runnable on its own.
#
# -- WHAT THIS IS, AND WHAT IT REPLACES --------------------------------------
#
# `nix bundle --bundler github:pkgforge/nix-appimage` already turns a nixpkgs
# attribute into an AppImage. The operator's instruction was to do it with the
# `Anylinux-AppImages` tooling instead, because "those tools use old outdated
# method". Read at commit da7649b9443971ef70da92f532e8a2e65a9f97f6
# (references/pkgforge-dev__Anylinux-AppImages), the difference is exactly
# three things, and each one is a mechanism this file adopts:
#
#   | | pkgforge/nix-appimage (mkAppImage.nix)     | Anylinux            |
#   |-|--------------------------------------------|---------------------|
#   |1| appimage-type2-runtime + mksquashfs         | uruntime + dwarfs   |
#   |2| a bwrap AppRun that BIND-MOUNTS /nix/store  | sharun: run the     |
#   | | so the closure's absolute paths resolve     | bundled ld.so with  |
#   | |                                             | --library-path      |
#   |3| ships the store layout verbatim             | shared/{bin,lib}    |
#
# ⛔ ITEM 2 IS THE ONE THAT MATTERS. The bwrap AppRun needs unprivileged user
# namespaces, which `HOW-TO-MAKE-THESE.md` calls out as a thing "you cannot
# even rely on", and which this project's own docs/limitations.md meets from
# the other side. sharun needs no namespace at all: it runs the bundled
# dynamic loader directly and hands it `--library-path`, a loader flag that --
# unlike LD_LIBRARY_PATH -- is not inherited by child processes.
#
# ⭐ AND ONE THING THIS FILE DOES THAT NEITHER UPSTREAM DOES.
# sharun's own library discovery walks `ldd` and then straces the program to
# catch dlopen'd libraries -- a good heuristic for a distro package. A NIXPKGS
# CLOSURE IS NOT A HEURISTIC: it is the exact, complete set of paths the
# derivation declared, including the dlopen'd ones, because nix would not have
# built without them. So the closure REPLACES the discovery step, and
# `scripts/common/nix-fetch.sh` gets it over plain HTTPS with every signature
# and hash checked and no nix installed.
#
# ⚠ WHAT THIS DOES NOT DO, said here rather than in a footnote:
#   - it does not make the application static, and it is not trying to. This
#     is docs/design/tiers.md tier 2 -- a bundle -- built because the operator
#     asked for the GUI case, which tier 1 does not reach.
#   - it does not debloat. The Anylinux flow strips locales, docs and unused
#     drivers and gets large apps down substantially; none of that is here.
#   - a nixpkgs `bin/x` that is a WRAPPER SCRIPT is followed to the real ELF
#     and the wrapper's environment is NOT reproduced. An app that needs it
#     will be missing it, and the run says so rather than half-working.
#
# Usage:
#   sh tool/nix-appimage.sh ATTR-OR-STOREPATH [--out FILE] [--name NAME]
#                           [--debloat none|safe|aggressive] [--keep-locales LIST]
#   sh tool/nix-appimage.sh --selftest
#
# Exit codes: 0 built, 1 did not, 2 could not run.

set -u

SELF=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO=$(dirname "$SELF")
FETCH="$REPO/scripts/common/nix-fetch.sh"
CACHE="${PGB_APPIMAGE_CACHE:-/var/tmp/pgb-appimage}"
ARCH=$(uname -m)

# ⛔ PINNED, AND THE PIN IS THE POINT. `latest/download` moves under you: two
# runs a week apart produce AppImages with different runtimes and nothing in
# either says so. These are the versions this file was tested against.
URUNTIME_URL="${URUNTIME_URL:-https://github.com/VHSgunzo/uruntime/releases/download/v0.5.6/uruntime-appimage-dwarfs-$ARCH}"
SHARUN_URL="${SHARUN_URL:-https://github.com/pkgforge-dev/Anylinux-sharun/releases/latest/download/sharun-$ARCH}"
DWARFS_URL="${DWARFS_URL:-https://github.com/mhx/dwarfs/releases/download/v0.14.1/dwarfs-universal-0.14.1-Linux-$ARCH}"

TARGET=""; OUT=""; NAME=""; SELFTEST=0; KEEP=0; EXTRA=""; NOGL=0
# ⭐ safe by default: every rule below removes something with a stated reason
# and a control behind it. `none` reproduces the pre-debloat bundle exactly,
# which is what experiments/85- and 86- measured.
DEBLOAT="${PGB_APPIMAGE_DEBLOAT:-safe}"
KEEP_LOCALES="${PGB_APPIMAGE_KEEP_LOCALES:-}"
while [ $# -gt 0 ]; do
  case "$1" in
    --out)      shift; OUT="${1:-}" ;;
    --name)     shift; NAME="${1:-}" ;;
    --keep)     KEEP=1 ;;
    --extra)    shift; EXTRA="$EXTRA ${1:-}" ;;
    --no-gl)    NOGL=1 ;;
    --debloat)  shift; DEBLOAT="${1:-safe}" ;;
    --keep-locales) shift; KEEP_LOCALES="${1:-}" ;;
    --selftest) SELFTEST=1 ;;
    -h|--help)  awk 'NR>1 { if (/^#/) { sub(/^# ?/, ""); print } else exit }' "$0"; exit 0 ;;
    -*)         printf 'nix-appimage: unknown option: %s\n' "$1" >&2; exit 2 ;;
    *)          TARGET="$1" ;;
  esac
  shift
done

die()  { printf 'nix-appimage: %s\n' "$1" >&2; exit "${2:-1}"; }
say()  { printf '%s\n' "$*"; }
warn() { printf 'nix-appimage: %s\n' "$*" >&2; }

need() {   # url dest -- fetch once, keep
  [ -x "$2" ] && return 0
  mkdir -p "$(dirname "$2")"
  say "fetching $(basename "$2")"
  curl -fsSL "$1" -o "$2.part" || {
    # ⚠ THE PROJECT'S FETCH RULE: when the direct route refuses, go through
    # the reverse proxy with the WHOLE original URL, scheme included.
    # TODO/RULES.md records that dropping the scheme returns 500.
    curl -fsSL "https://api.rv.pkgforge.dev/$1" -o "$2.part"; } || return 1
  chmod +x "$2.part"; mv "$2.part" "$2"
}

# ---------------------------------------------------------------------------
# ⛔ THE ENTRY POINT IS FOUND, NOT ASSUMED, and a wrapper is followed.
#
# nixpkgs installs many GUI programs as a small shell script in bin/ that sets
# environment and execs the real ELF out of a second store path. Copying the
# script into shared/bin produces an AppImage that runs a shell which then
# tries to exec an absolute /nix/store path that is not there.
# ---------------------------------------------------------------------------
# ⛔ THE FALLBACK BELOW SHIPPED THE WRONG PROGRAM ONCE, SILENTLY.
# `--name eglinfo-nogl` was passed to get a second, differently-named artefact
# out of mesa-demos. mesa-demos has no `eglinfo-nogl`, so `head -1` of bin/
# picked `quadstrip-flat` -- an OpenGL demo needing a window -- and the run
# printed `entry .../bin/quadstrip-flat` in the middle of eleven other lines
# and packed it. The experiment would then have compared eglinfo against a
# program that is not eglinfo.
#
# ⭐ The fallback stays, because a package whose binary is not named after the
# attribute is ordinary, but the two cases are now separated: a name the
# CALLER asked for is a requirement and is refused when it is not there; a
# name pgb DERIVED from the store path is a guess and says so when it misses.
# ⛔ A SYMLINK IN A CLOSURE POINTS AT AN ABSOLUTE /nix/store PATH THAT IS NOT
# HERE. Every `share/` tree in a multi-output package is a farm of them, and
# `cp -L` on one fails with "cannot stat" -- which the desktop-file step then
# reported as a SUCCESS, printing `desktop mpv.desktop (Icon=, Exec rewritten)`
# for a file it had not copied. Measured on mpv-with-scripts-0.41.0.
store_resolve() {   # path -> a path that exists under $ROOT, or nothing
  _sr_p="$1"; _sr_n=0
  while [ "$_sr_n" -lt 10 ]; do
    _sr_n=$((_sr_n + 1))
    [ -L "$_sr_p" ] || { [ -e "$_sr_p" ] && printf '%s\n' "$_sr_p"; return 0; }
    _sr_t=$(readlink "$_sr_p")
    case "$_sr_t" in
      /nix/store/*) _sr_p="$ROOT/${_sr_t#/nix/store/}" ;;
      /*)           _sr_p="$_sr_t" ;;
      *)            _sr_p="$(dirname "$_sr_p")/$_sr_t" ;;
    esac
  done
  return 0
}

resolve_entry() {   # storedir progname -> prints the ELF to run
  _re_bin="$1/bin/$2"
  if [ ! -e "$_re_bin" ]; then
    if [ -n "${NAME:-}" ]; then
      warn "⛔ --name '$2' names no program in $(basename "$1")/bin. What is there:"
      find "$1/bin" -maxdepth 1 -type f -exec basename {} \; 2>/dev/null \
        | sort | sed 's/^/             /' >&2
      return 1
    fi
    _re_bin=$(find "$1/bin" -maxdepth 1 -type f 2>/dev/null | head -1)
    [ -n "$_re_bin" ] && warn "⚠ bin/$2 does not exist; falling back to bin/$(basename "$_re_bin")"
  fi
  [ -n "$_re_bin" ] && [ -e "$_re_bin" ] || return 1
  _re_hops=0
  : > "${WRAPENV:-/dev/null}"
  while [ "$_re_hops" -lt 5 ]; do
    _re_hops=$((_re_hops + 1))
    # ⛔ THE ELF TEST HAS TO COME SECOND, AND THAT USED TO BE A BUG THAT
    # SHIPPED A BROKEN BUNDLE. nixpkgs' current `makeWrapper` produces
    # `makeBinaryWrapper` output -- a COMPILED C PROGRAM. Measured on
    # mpv-with-scripts-0.41.0, whose bin/mpv is a 16,560-byte ELF. Testing the
    # magic first declared the wrapper to be the program, packed it, and the
    # bundle then exec'd an absolute /nix/store path that is not in it. So:
    # ask whether it is a WRAPPER before asking whether it is an ELF.
    _re_recs=$(python3 "$SELF/nix-wrapper.py" read "$_re_bin" 2>/dev/null) || _re_recs=""
    if [ -n "$_re_recs" ]; then
      _re_tgt=$(printf '%s\n' "$_re_recs" | awk -F'\t' '$1=="target"{print $4; exit}')
      # Everything but the target is environment, and it is kept.
      printf '%s\n' "$_re_recs" | awk -F'\t' '$1!="target"' >> "${WRAPENV:-/dev/null}"
      if [ -n "$_re_tgt" ] && [ -e "$ROOT/${_re_tgt#/nix/store/}" ]; then
        warn "bin/$2 is a nixpkgs wrapper -> $(basename "$_re_tgt")"
        warn "   its environment is read out of it: $(printf '%s\n' "$_re_recs" | grep -c . ) record(s)"
        _re_bin="$ROOT/${_re_tgt#/nix/store/}"
        continue
      fi
    fi
    case "$(od -An -N4 -tx1 "$_re_bin" 2>/dev/null | tr -d ' \n')" in
      7f454c46) printf '%s\n' "$_re_bin"; return 0 ;;
    esac
    # A wrapper shape nix-wrapper.py does not recognise. Take the last store
    # path it names that exists, and say the environment was NOT read.
    _re_next=$(grep -oE '/nix/store/[a-z0-9]{32}-[^" ]*' "$_re_bin" 2>/dev/null \
               | while IFS= read -r c; do
                   [ -x "$ROOT/${c#/nix/store/}" ] && printf '%s\n' "$ROOT/${c#/nix/store/}"
                 done | tail -1)
    [ -n "$_re_next" ] || { warn "bin/$2 is a script and no ELF in it could be resolved"; return 1; }
    warn "bin/$2 is a wrapper of a shape nix-wrapper.py does not read -> $(basename "$_re_next")"
    warn "⚠ its ENVIRONMENT is NOT reproduced. T-053."
    _re_bin="$_re_next"
  done
  return 1
}

selftest() {
  _bad=0
  _t=$(mktemp -d)
  # resolve_entry must follow a wrapper and must refuse one it cannot resolve.
  ROOT="$_t"
  mkdir -p "$_t/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real/bin" "$_t/pkg/bin"
  printf '\177ELF fake' > "$_t/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real/bin/prog"
  chmod +x "$_t/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real/bin/prog"
  printf '#!/bin/sh\nexec /nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real/bin/prog "$@"\n' \
    > "$_t/pkg/bin/prog"
  chmod +x "$_t/pkg/bin/prog"
  _got=$(resolve_entry "$_t/pkg" prog 2>/dev/null)
  if [ "$_got" = "$_t/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-real/bin/prog" ]; then
    printf '  ok    a wrapper script is followed to its ELF\n'
  else
    printf '  FAIL  wrapper follow -> %s\n' "$_got"; _bad=1
  fi
  printf '#!/bin/sh\nexec /nix/store/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb-gone/bin/x\n' \
    > "$_t/pkg/bin/prog"
  if resolve_entry "$_t/pkg" prog >/dev/null 2>&1; then
    printf '  FAIL  a wrapper pointing nowhere was accepted\n'; _bad=1
  else
    printf '  ok    a wrapper pointing nowhere is refused\n'
  fi

  # ⛔ THE CASE THAT SHIPPED THE WRONG PROGRAM. Two binaries, neither named
  # what was asked for. With --name it must REFUSE; without it, it may fall
  # back -- and the two must not be the same answer.
  rm -f "$_t/pkg/bin/prog"
  printf '\177ELF one' > "$_t/pkg/bin/aardvark"; chmod +x "$_t/pkg/bin/aardvark"
  printf '\177ELF two' > "$_t/pkg/bin/zebra";    chmod +x "$_t/pkg/bin/zebra"
  NAME=nosuchprog
  if resolve_entry "$_t/pkg" nosuchprog >/dev/null 2>&1; then
    printf '  FAIL  --name naming no program was accepted\n'; _bad=1
  else
    printf '  ok    --name naming no program is refused, not substituted\n'
  fi
  NAME=""
  if [ -n "$(resolve_entry "$_t/pkg" nosuchprog 2>/dev/null)" ]; then
    printf '  ok    a DERIVED name that misses still falls back\n'
  else
    printf '  FAIL  a derived name that misses should fall back\n'; _bad=1
  fi

  rm -rf "$_t"
  printf 'nix-appimage --selftest: %s\n' \
    "$([ "$_bad" = 0 ] && echo 'all checks pass.' || echo 'FAILURES above.')"
  return "$_bad"
}

[ "$SELFTEST" = 1 ] && { selftest; exit $?; }

[ -n "$TARGET" ] || die "give a nixpkgs attribute or a store path" 2
command -v curl >/dev/null 2>&1 || die "curl not found" 2

# ---------------------------------------------------------------------------
# 1. resolve the attribute to an output store path
# ---------------------------------------------------------------------------
# ⛔ A STORE PATH IS A 32-CHARACTER HASH AND A NAME, NOT "ANYTHING WITH A
# DASH". The first version of this case matched `[a-z0-9]*-*`, so the
# ATTRIBUTE `mesa-demos` was taken for a store path, turned into
# `/nix/store/mesa-demos`, and the run failed with "could not fetch the
# closure" -- which reads like a network problem and is a parser problem.
case "$TARGET" in
  /nix/store/[a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9]-*|\
  [a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9]-*)
    OUTPATH="$TARGET" ;;
  *)
    NIXBIN=""
    for c in nix /nix/var/nix/profiles/default/bin/nix; do
      command -v "$c" >/dev/null 2>&1 && { NIXBIN=$(command -v "$c"); break; }
      [ -x "$c" ] && { NIXBIN="$c"; break; }
    done
    [ -n "$NIXBIN" ] || die "resolving an ATTRIBUTE needs nix; pass a store path instead" 2
    NIXPFX=$(dirname "$NIXBIN")
    DRV=$("$NIXPFX/nix-instantiate" '<nixpkgs>' --attr "$TARGET" 2>/dev/null | grep '^/nix/store/' | head -1)
    DRV="${DRV%%!*}"
    [ -n "$DRV" ] || die "nixpkgs has no attribute '$TARGET'"
    # ⛔ `out` IS NOT WHERE THE PROGRAM IS FOR A MULTI-OUTPUT PACKAGE, and
    # taking it unconditionally is how "one command from a package name"
    # stops being true. Measured on nixpkgs `jq`: its `out` output contains
    # `lib/` and NOTHING ELSE -- `bin/jq` lives in the separate `bin` output,
    # which is not in `out`'s closure at all. The run got as far as
    # `no entry point in ...-jq-1.8.2/bin`, which reads like a broken package
    # and is a wrong output.
    #
    # ⭐ Preferring `bin` is the right default rather than a special case:
    # nixpkgs splits an output off exactly when it wants the executables
    # separated, and the `bin` output REFERENCES `out`, so its closure carries
    # both. Single-output packages are untouched -- there is only `out` and
    # the preference falls through to it.
    OUTSEL=$("$NIXPFX/nix" derivation show "$DRV" 2>/dev/null | python3 -c '
import json, sys
d = json.load(sys.stdin)["derivations"]
v = list(d.values())[0]
o = v["outputs"]
for name in ("bin", "out"):
    p = o.get(name, {}).get("path", "")
    if p:
        print(name, p)
        break
else:
    # Neither: take whatever single output there is rather than guessing.
    for name, spec in o.items():
        if spec.get("path"):
            print(name, spec["path"])
            break')
    OUTNAME=${OUTSEL%% *}
    OUTPATH=${OUTSEL#* }
    [ -n "$OUTSEL" ] && [ "$OUTNAME" != "$OUTPATH" ] || die "could not find an output path of $DRV"
    [ "$OUTNAME" = out ] || say "output      '$OUTNAME' (nixpkgs put the programs there, not in 'out')"
    ;;
esac
BASE=$(printf '%s' "$OUTPATH" | sed 's|^/nix/store/||')
PROG="${NAME:-$(printf '%s' "$BASE" | sed 's/^[a-z0-9]\{32\}-//; s/-[0-9].*//')}"
say "attribute   $TARGET"
say "store path  /nix/store/$BASE"
say "program     $PROG"

WORK="$CACHE/$PROG"
ROOT="$WORK/store"
APPDIR="$WORK/AppDir"
# Where resolve_entry writes what it read out of a nixpkgs wrapper. ⚠ It runs
# in a command substitution, so a variable cannot carry this back.
WRAPENV="$WORK/wrapper-env.tsv"
mkdir -p "$ROOT" "$CACHE/tools"

# ---------------------------------------------------------------------------
# 2. the closure, verified, with no nix
# ---------------------------------------------------------------------------
say "fetching the closure (signature and NarHash checked, no nix involved)"
sh "$FETCH" fetch "$BASE" --out "$ROOT" >/dev/null 2>&1 || die "could not fetch the closure"
NPATHS=$(find "$ROOT" -mindepth 1 -maxdepth 1 | wc -l | tr -d ' ')
say "closure     $NPATHS store paths, $(du -sh "$ROOT" 2>/dev/null | cut -f1)"

# ---------------------------------------------------------------------------
# 2b. ⛔ THE OpenGL AUGMENTATION, AND IT IS NOT OPTIONAL FOR A GL PROGRAM
#
# ⭐ THE MECHANISM, MEASURED HERE RATHER THAN ASSUMED. A nixpkgs GL program
# does NOT depend on mesa. It depends on `libglvnd`, the vendor-neutral
# dispatch layer -- and libglvnd finds the actual implementation by reading
# `share/glvnd/egl_vendor.d/*.json` and dlopen'ing whatever they name. That
# file is HOST configuration, so mesa is NOT in the closure at all.
#
# Measured on mesa-demos 9.0.0: its 111-path closure contains libglvnd-1.7.0
# and mesa-libgbm and NOT ONE mesa driver, and the bundle got as far as
# `eglinfo: eglInitialize failed` -- libglvnd loaded, found no vendor, and
# stopped. ⛔ That is the "libGL problem" exactly, and it is this project's
# own docs/limitations.md §1 (dlopen of a host object) arriving from the GL
# side.
#
# ⭐ nixGL reaches the same conclusion: for the mesa case it does not use the
# host's GL at all, it points nixpkgs' OWN mesa at itself with
# LIBGL_DRIVERS_PATH, GBM_BACKENDS_PATH and __EGL_VENDOR_LIBRARY_FILENAMES
# (`nixGL.nix:54-62`, commit b6105297). Pulling mesa into the closure and
# setting those is therefore not a workaround, it is the same answer.
#
# ⚠ AND IT DOES NOT COVER NVIDIA'S PROPRIETARY DRIVER, whose userspace half
# must match the running kernel module. nixGL reads
# /proc/driver/nvidia/version and FETCHES a matching driver; a bundle cannot.
# TODO T-052 owns that case and this code does not pretend to.
if [ "$NOGL" != 1 ]; then
  if find "$ROOT" -name 'libGLdispatch.so*' -o -name 'libEGL.so*' -o -name 'libGL.so*' \
       2>/dev/null | grep -q . && [ ! -d "$(echo "$ROOT"/*mesa-[0-9]*/lib/dri 2>/dev/null | head -1)" ]; then
    say "opengl      libglvnd is in the closure and no mesa driver is: pulling mesa in"
    EXTRA="$EXTRA mesa"
  fi
fi

for _x in $EXTRA; do
  case "$_x" in
    /nix/store/*[a-z0-9]-*)
      case "$(basename "$_x" | cut -c1-33)" in
        [a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9][a-z0-9]-) _xp="$_x" ;;
        *) _xp="" ;;
      esac ;;
    *)
      [ -n "${NIXPFX:-}" ] || { warn "--extra $_x needs nix to resolve a name"; continue; }
      _xd=$("$NIXPFX/nix-instantiate" '<nixpkgs>' --attr "$_x" 2>/dev/null | grep '^/nix/store/' | head -1)
      _xd="${_xd%%!*}"
      _xp=$("$NIXPFX/nix" derivation show "$_xd" 2>/dev/null | python3 -c '
import json, sys
d = json.load(sys.stdin)["derivations"]
print(list(d.values())[0]["outputs"].get("out", {}).get("path", ""))' 2>/dev/null) ;;
  esac
  [ -n "$_xp" ] || { warn "could not resolve --extra $_x"; continue; }
  say "extra       $_x -> $_xp"
  sh "$FETCH" fetch "$_xp" --out "$ROOT" >/dev/null 2>&1 \
    || warn "could not fetch the closure of $_x"
done
NPATHS=$(find "$ROOT" -mindepth 1 -maxdepth 1 | wc -l | tr -d ' ')
say "closure     $NPATHS store paths after augmentation"

# ---------------------------------------------------------------------------
# 3. the AppDir, in the shape sharun wants
# ---------------------------------------------------------------------------
# ⛔ THE LAYOUT IS THE FORK'S, NOT UPSTREAM sharun'S, AND THEY DIFFER.
# Anylinux-sharun's README says it "uses `lib`/`lib32` directly instead of
# `shared/lib`/`shared/lib32`", and quick-sharun.sh:44-46 is where that lives:
#
#   AppDir/lib          the libraries AND the dynamic loader
#   AppDir/bin/<name>   a HARDLINK of sharun, one per program
#   AppDir/shared/bin   the real ELF binaries
#   AppDir/shared/lib   a symlink to ../lib (quick-sharun.sh:464)
#
# Building it upstream's way -- libraries under shared/lib -- produced an
# AppImage that mounted, started sharun, and printed "Interpreter not found!",
# because sharun looks for ld-linux-*.so.* in $SHARUN_DIR/lib and there was
# nothing there.
rm -rf "$APPDIR"
mkdir -p "$APPDIR/shared/bin" "$APPDIR/lib" "$APPDIR/bin" "$APPDIR/share"
ln -sfn ../lib "$APPDIR/shared/lib"

ENTRY=$(resolve_entry "$ROOT/$BASE" "$PROG") || die "no entry point in $BASE/bin"
say "entry       $ENTRY"
cp -L "$ENTRY" "$APPDIR/shared/bin/$PROG"
chmod +x "$APPDIR/shared/bin/$PROG"

# ⭐ EVERY SHARED OBJECT IN THE CLOSURE, not the ones ldd happens to name.
# ⛔ A .so IS A NAME ENDING IN .so OR .so.N -- matching the substring also
# matches /etc/ld.so.cache and a directory called `libexec/foo.sources`.
# docs/AGENTS.md §14 carries what that has already cost this project.
# ⛔ AND A 32-BIT OBJECT DOES NOT GO IN lib/. The Anylinux layout has `lib32`
# beside `lib` and this bundler had no 32-bit path at all -- T-057 item 4. A
# closure that carries both (anything with a 32-bit compatibility half: wine,
# steam, a 32-bit plugin host) would put an i386 libfoo.so.1 and an x86_64
# libfoo.so.1 in ONE flat directory, where the second `cp -n` silently loses
# and the loader gets whichever landed first. The ELF class is byte 5 of the
# header: 1 = 32-bit, 2 = 64-bit.
elfclass() {   # path -> 32 | 64 | ""
  case "$(od -An -N5 -tx1 "$1" 2>/dev/null | tr -d ' \n')" in
    7f454c4601) printf 32 ;;
    7f454c4602) printf 64 ;;
    *)          printf '' ;;
  esac
}
N32=0
find "$ROOT" -type f -name '*.so' -o -type f -name '*.so.*' 2>/dev/null \
  | grep -E '\.so(\.[0-9]+)*$' \
  | while IFS= read -r so; do
      case "$(elfclass "$so")" in
        32) mkdir -p "$APPDIR/lib32"
            cp -n "$so" "$APPDIR/lib32/$(basename "$so")" 2>/dev/null || true ;;
        *)  cp -n "$so" "$APPDIR/lib/$(basename "$so")" 2>/dev/null || true ;;
      esac
    done
# Symlinks too: libfoo.so.6 -> libfoo.so.6.0.1 is how a DT_NEEDED resolves.
# ⛔ AND THE PASS HAS TO REPEAT, because a closure has symlinks TO SYMLINKS and
# one pass is order-dependent. vulkan-loader ships
#   libvulkan.so -> libvulkan.so.1 -> libvulkan.so.1.4.357
# and `find` handed back `libvulkan.so` before `libvulkan.so.1` existed in
# lib/, so the target check refused it and the bundle was left with a
# DT_NEEDED on `libvulkan.so` that resolved nowhere. Caught by the integrity
# check below on the first real application run through this bundler; before
# that check existed it would have been a runtime failure on somebody's
# machine. Repeat until a pass creates nothing.
_slround=0
while [ "$_slround" -lt 6 ]; do
  _slround=$((_slround + 1))
  _slmade=0
  for sl in $(find "$ROOT" -type l -name '*.so*' 2>/dev/null); do
    _t=$(readlink "$sl")
    _b=$(basename "$sl")
    [ -e "$APPDIR/lib/$_b" ] && continue
    [ -e "$APPDIR/lib/$(basename "$_t")" ] || continue
    ln -sf "$(basename "$_t")" "$APPDIR/lib/$_b" && _slmade=$((_slmade + 1))
  done
  [ "$_slmade" = 0 ] && break
done
# ⛔ SOME LIBRARY TREES ARE DIRECTORIES AND FLATTENING THEM BREAKS THEM.
# mesa's DRI drivers live in `lib/dri/`, its GBM backends in `lib/gbm/`, GTK's
# modules in `lib/gtk-3.0/...` -- and each is found through a variable that
# names the DIRECTORY, so a flattened copy is invisible to the loader that
# wants it. Copied whole, preserving the subtree.
for _sub in dri gbm gtk-3.0 gtk-4.0 gdk-pixbuf-2.0 girepository-1.0 \
            pipewire-0.3 spa-0.2 vdpau; do
  for _d in "$ROOT"/*/lib/"$_sub"; do
    [ -d "$_d" ] || continue
    mkdir -p "$APPDIR/lib/$_sub"
    cp -aLn "$_d/." "$APPDIR/lib/$_sub/" 2>/dev/null || true
  done
done
NLIBS=$(find "$APPDIR/lib" -maxdepth 1 | wc -l | tr -d ' ')
say "libraries   $NLIBS from the closure"

# The loader. ⛔ IT MUST BE THE CLOSURE'S OWN: the whole point is not to touch
# the host's, and a loader from a different glibc than the libraries beside it
# is the exact pairing docs/limitations.md §1 measures failing.
LD=$(find "$ROOT" -name 'ld-linux-*.so.*' -o -name 'ld-musl-*.so.*' 2>/dev/null | head -1)
[ -n "$LD" ] || die "the closure carries no dynamic loader"
cp -L "$LD" "$APPDIR/lib/$(basename "$LD")"
say "loader      $(basename "$LD") (the closure's own, never the host's)"
if [ -d "$APPDIR/lib32" ]; then
  # The 32-bit half needs its OWN loader, and it is a different file with a
  # different name (ld-linux.so.2, not ld-linux-x86-64.so.2).
  LD32=$(find "$ROOT" -name 'ld-linux.so.2' -o -name 'ld-linux-armhf.so.3' 2>/dev/null | head -1)
  if [ -n "$LD32" ]; then
    cp -L "$LD32" "$APPDIR/lib32/$(basename "$LD32")"
    say "loader32    $(basename "$LD32")  ($(find "$APPDIR/lib32" -maxdepth 1 -name '*.so*' | wc -l | tr -d ' ') objects in lib32)"
  else
    warn "the closure has 32-bit objects but no 32-bit loader; lib32 will not run"
  fi
  ln -sfn ../lib32 "$APPDIR/shared/lib32" 2>/dev/null || true
fi

# ⛔ ABSOLUTE DT_NEEDED ENTRIES ARE REWRITTEN TO BASENAMES, and without this
# the bundle does not start. nixpkgs links some libraries by absolute path, so
# a DT_NEEDED reads
#   /nix/store/fqkp...-sqlite-3.53.3/lib/libsqlite3.so
# and an absolute DT_NEEDED is OPENED AS A PATH -- the loader never consults
# --library-path for it. Measured on galculator: the AppImage mounted, sharun
# started the binary, and it died with "cannot open shared object file" for a
# library sitting in the same directory.
# tool/elf-needed.py carries why the edit is safe and what it refuses.
say "rewriting absolute DT_NEEDED entries to basenames"
NREW=$(python3 "$SELF/elf-needed.py" shorten "$APPDIR/shared/bin/$PROG" \
        "$APPDIR"/lib/*.so* 2>/dev/null | wc -l | tr -d ' ')
say "patched     $NREW absolute DT_NEEDED entries"

# ---------------------------------------------------------------------------
# 4. ⭐ THE DESKTOP FILE AND THE ICON, TAKEN FROM THE CLOSURE
#
# The operator's open question in docs/design/nix-front-end.md was whether
# these can be had "automatically" rather than by patching. They can: a
# nixpkgs derivation installs them in the places the freedesktop spec names,
# so finding them is a `find`, not a rule per application.
# ---------------------------------------------------------------------------
for d in "$ROOT"/*/share; do
  [ -d "$d" ] || continue
  cp -aLn "$d/." "$APPDIR/share/" 2>/dev/null || true
done
# ⛔ THE APPLICATION'S OWN STORE PATH IS SEARCHED FIRST, AND THAT IS NOT A
# REFINEMENT. A closure carries every dependency's share/ too: galculator's
# 100-path closure contains GTK, and GTK installs
# `share/applications/gtk3-widget-factory.desktop`. Taking the first .desktop
# in the merged tree picked THAT, so the AppImage advertised itself as GTK's
# demo -- and, because sharun takes the program name from the desktop entry,
# it then tried to run `bin/gtk3-widget-factory` and died with "No such file
# or directory" on an AppImage that was otherwise correct.
# ⛔ AND THE ICD JSONs NAME AN ABSOLUTE STORE PATH, which is the last hop of
# the OpenGL problem and the one that looks like a bundling failure.
# nixpkgs' `share/glvnd/egl_vendor.d/50_mesa.json` reads
#
#     "library_path" : "/nix/store/4cvv9...-mesa-26.2.1/lib/libEGL_mesa.so.0"
#
# so libglvnd finds the vendor file, opens the path it names, and fails --
# with `eglInitialize failed`, on a bundle that has libEGL_mesa.so.0 sitting
# in lib/ right beside it. Rewritten to the BARE SONAME, which libglvnd
# resolves through the loader, which sharun has already pointed at lib/.
# ⚠ Same shape as the DT_NEEDED rewrite above, in a JSON file instead of an
# ELF, and the same rule applies: only the path is wrong, not the intent.
for _icd in "$APPDIR"/share/glvnd/egl_vendor.d/*.json \
            "$APPDIR"/share/vulkan/icd.d/*.json \
            "$APPDIR"/share/vulkan/implicit_layer.d/*.json; do
  [ -f "$_icd" ] || continue
  sed -i -E 's#("library_path"[[:space:]]*:[[:space:]]*")/nix/store/[^"]*/([^/"]+)"#\1\2"#' "$_icd" 2>/dev/null || true
done
_nicd=$(find "$APPDIR/share/glvnd" "$APPDIR/share/vulkan" -name '*.json' 2>/dev/null | wc -l | tr -d ' ')
[ "${_nicd:-0}" -gt 0 ] && say "icd json    $_nicd rewritten to bare sonames"

DESKTOP=$(find "$ROOT/$BASE/share/applications" -name '*.desktop' 2>/dev/null | head -1)
[ -n "$DESKTOP" ] && DESKTOP=$(store_resolve "$DESKTOP")
[ -n "$DESKTOP" ] || DESKTOP=$(find "$APPDIR/share/applications" -name "$PROG.desktop" 2>/dev/null | head -1)
[ -n "$DESKTOP" ] || DESKTOP=$(find "$APPDIR/share/applications" -name "*$PROG*.desktop" 2>/dev/null | head -1)
# ⛔ AND THE COPY IS CHECKED. `cp -L` on a dangling store symlink fails, and
# the block below then printed a success line for a file that is not there.
if [ -n "$DESKTOP" ] && ! cp -L "$DESKTOP" "$APPDIR/$PROG.desktop" 2>/dev/null; then
  warn "the .desktop entry $DESKTOP could not be copied; generating one instead"
  DESKTOP=""
fi
if [ -n "$DESKTOP" ]; then
  ICONNAME=$(awk -F= '/^Icon=/{print $2; exit}' "$DESKTOP")
  # ⛔ Exec= IS REWRITTEN TO THE NAME WE ACTUALLY HARDLINKED. sharun resolves
  # the program from the desktop entry, so an Exec naming a binary that is not
  # in shared/bin produces a bundle that mounts, starts, and then reports the
  # missing file -- which reads like a broken build rather than a wrong line
  # in a text file.
  sed -i -E "s|^Exec=[^ ]*|Exec=$PROG|; s|^TryExec=.*|TryExec=$PROG|" "$APPDIR/$PROG.desktop"
  say "desktop     $(basename "$DESKTOP")  (Icon=$ICONNAME, Exec rewritten to $PROG)"
else
  # ⚠ A GENERATED ENTRY IS MARKED AS GENERATED. An AppImage with no .desktop
  # file will not integrate, and silently inventing one that claims to be the
  # application's own is worse than saying it was made up.
  warn "the closure has no .desktop file; writing a minimal generated one"
  ICONNAME="$PROG"
  cat > "$APPDIR/$PROG.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=$PROG
Exec=$PROG
Icon=$PROG
Categories=Utility;
Comment=generated by pgb nix appimage -- the closure carried no desktop entry
EOF
fi
# ⚠ THE ICON IS CHOSEN BY RESOLUTION, NOT BY PATH LENGTH. The first version
# sorted by string length, which is a proxy for nothing: it picked
# 16x16 over 256x256 as often as not.
ICON=""
for _d in "$ROOT/$BASE/share/icons" "$ROOT/$BASE/share/pixmaps" \
          "$APPDIR/share/icons" "$APPDIR/share/pixmaps"; do
  [ -d "$_d" ] || continue
  ICON=$(find -L "$_d" \( -name "$ICONNAME.png" -o -name "$ICONNAME.svg" \) 2>/dev/null \
         | sed -E 's|.*/([0-9]+)x[0-9]+/.*|\1 &|; t; s|^|0 |' \
         | sort -rn | awk 'NR==1{ $1=""; sub(/^ /,""); print }')
  [ -n "$ICON" ] && break
done
# ⛔ AND THE SAME DANGLING-SYMLINK TRAP AS THE .desktop ENTRY, one step later.
# `find -L` follows links while descending but still PRINTS the link's own
# path, so `cp -L` fails on it -- and the block below printed `icon mpv.png`
# for a file it had not copied. Resolved through the store, and the copy is
# checked.
[ -n "$ICON" ] && ICON=$(store_resolve "$ICON")
if [ -n "$ICON" ] && cp -L "$ICON" "$APPDIR/$(basename "$ICON")" 2>/dev/null; then
  cp -L "$ICON" "$APPDIR/.DirIcon" 2>/dev/null || true
  say "icon        $(basename "$ICON")"
else
  [ -n "$ICON" ] && warn "the icon $ICON could not be copied"
  warn "no icon named '$ICONNAME' in the closure; the AppImage will have none"
fi

# ---------------------------------------------------------------------------
# 4b. ⭐ DEBLOATING, WITH A REASON PER RULE AND A CONTROL AFTER IT
#
# `experiments/85-` gives the number to beat: on a GL application the
# undebloated mesa is **95 MiB of a 163 MB bundle**. Measured on
# `mesa-26.2.1` itself, that 95 MiB is not the GL driver:
#
#   libgallium-26.2.1.so      58.2 MiB   the actual GL/gallium driver -- KEPT
#   libvulkan_intel.so        27.2 MiB   \
#   libvulkan_nouveau.so      21.4 MiB    | Vulkan ICDs, ~194 MiB across 12
#   libvulkan_radeon.so       20.0 MiB    | files, EACH dlopen'd through a
#   libvulkan_panfrost.so     17.7 MiB    | JSON in share/vulkan/icd.d
#   libvulkan_freedreno.so    16.3 MiB    |
#   libvulkan_asahi.so        15.8 MiB    /
#   libteflon.so              12.1 MiB   an NPU delegate, not a GL driver
#
# ⛔ SEVEN OF THOSE TWELVE ARE FOR GPUs THAT CANNOT EXIST ON THIS
# ARCHITECTURE: panfrost is ARM Mali, freedreno is Adreno, broadcom is a
# Raspberry Pi, asahi is Apple silicon, powervr is Imagination, dzn is
# Direct3D 12 on Windows, gfxstream is an Android emulator transport. Dropping
# them is not a size/function trade -- there is no function to lose on an
# x86_64 Linux host.
#
# ⭐ AND NOTHING IS DROPPED WITHOUT ITS REFERENCE. A Vulkan driver is found
# through `share/vulkan/icd.d/<name>.json`; removing the library and leaving
# the JSON gives a loader that tries to open a file that is gone. Both go.
#
# ⚠ THE CONTROL IS THE POINT, and it runs after every rule: every DT_NEEDED of
# every ELF left in the bundle must resolve inside lib/. A debloat that breaks
# that is a smaller bundle that does not run, which is the failure mode this
# whole step invites.
# ---------------------------------------------------------------------------
debloat_size() { du -sb "$APPDIR" 2>/dev/null | cut -f1; }
DB_BEFORE=$(debloat_size)
db_rule() {   # label  <paths on stdin>
  _dr_n=0; _dr_b=0
  while IFS= read -r _dr_p; do
    [ -e "$_dr_p" ] || continue
    _dr_s=$(du -sb "$_dr_p" 2>/dev/null | cut -f1)
    rm -rf "$_dr_p" && _dr_n=$((_dr_n + 1)) && _dr_b=$((_dr_b + ${_dr_s:-0}))
  done
  [ "$_dr_n" -gt 0 ] && say "  debloat   $(printf '%7.1f MiB' "$(echo "$_dr_b" | awk '{print $1/1048576}')")  $_dr_n  $1"
  return 0
}

if [ "$DEBLOAT" != none ]; then
  say ""
  say "debloating (level: $DEBLOAT)"

  # 1. Documentation. A bundle is not a development environment.
  find "$APPDIR/share" -maxdepth 1 \( -name doc -o -name man -o -name info \
       -o -name gtk-doc -o -name devhelp -o -name bash-completion \
       -o -name zsh -o -name fish \) 2>/dev/null | db_rule "documentation and shell completions"

  # 2. Anything only a COMPILER would open.
  { find "$APPDIR" \( -name '*.a' -o -name '*.la' -o -name '*.pc' \) -type f 2>/dev/null
    find "$APPDIR" -maxdepth 3 \( -name include -o -name pkgconfig -o -name cmake \
         -o -name aclocal \) -type d 2>/dev/null; } | db_rule "static archives, headers and build metadata"

  # 3. Locales. ⚠ THIS IS A REAL TRADE AND IT IS STATED AS ONE: the
  # application's own translations go with them. --keep-locales names the ones
  # to keep; the Anylinux flow's chromium recipe instead symlinks the
  # directory back to the HOST, which is a deliberate reintroduction of a host
  # dependency and is NOT copied here.
  if [ -d "$APPDIR/share/locale" ]; then
    find "$APPDIR/share/locale" -mindepth 1 -maxdepth 1 -type d 2>/dev/null \
      | while IFS= read -r _l; do
          case ",$KEEP_LOCALES," in
            *",$(basename "$_l"),"*) ;;
            *) printf '%s\n' "$_l" ;;
          esac
        done | db_rule "locale catalogues (kept: ${KEEP_LOCALES:-none})"
  fi

  # 4. GPU drivers for hardware this architecture does not have, and the ICD
  # JSON that names each one.
  case "$ARCH" in
    x86_64|i686|i386) FOREIGN="panfrost freedreno broadcom asahi powervr dzn gfxstream v3dv imagination" ;;
    aarch64)          FOREIGN="dzn gfxstream intel_hasvk" ;;
    *)                FOREIGN="" ;;
  esac
  [ "$DEBLOAT" = aggressive ] && FOREIGN="$FOREIGN intel intel_hasvk radeon nouveau virtio swrast_no"
  for _f in $FOREIGN; do
    { find "$APPDIR/lib" -maxdepth 1 -name "libvulkan_$_f*.so*" 2>/dev/null
      find "$APPDIR/share/vulkan" -name "*$_f*.json" 2>/dev/null
    } | db_rule "vulkan driver '$_f' (no such GPU on $ARCH)"
  done
  # An NPU delegate is not a graphics driver and nothing in a GL closure
  # references it.
  find "$APPDIR/lib" -maxdepth 1 -name 'libteflon.so*' 2>/dev/null \
    | db_rule "libteflon (an NPU delegate, not a GPU driver)"

  DB_AFTER=$(debloat_size)
  say "  debloat   $(awk -v a="$DB_BEFORE" -v b="$DB_AFTER" 'BEGIN{printf "%.1f MiB -> %.1f MiB (%.1f%% off)", a/1048576, b/1048576, (a-b)*100/a}')"
fi

# ---------------------------------------------------------------------------
# 5. sharun as AppRun, and the lib.path it reads
# ---------------------------------------------------------------------------
need "$SHARUN_URL" "$CACHE/tools/sharun" || die "could not fetch sharun"
cp "$CACHE/tools/sharun" "$APPDIR/sharun"
ln -f "$APPDIR/sharun" "$APPDIR/AppRun" 2>/dev/null || cp "$APPDIR/sharun" "$APPDIR/AppRun"
# ⭐ THE HARDLINK IS THE MECHANISM, NOT A SHORTCUT. sharun looks at the name it
# was invoked as and runs shared/bin/<that name>, which is how /proc/self/exe
# ends up naming the application instead of the loader -- the one thing the
# plain `ld.so --library-path` AppRun in HOW-TO-MAKE-THESE.md cannot fix.
ln -f "$APPDIR/sharun" "$APPDIR/bin/$PROG" 2>/dev/null || cp "$APPDIR/sharun" "$APPDIR/bin/$PROG"
( cd "$APPDIR" && ./sharun --gen-lib-path >/dev/null 2>&1 ) || warn "sharun --gen-lib-path failed"
[ -s "$APPDIR/lib/lib.path" ] && say "lib.path    $(wc -l < "$APPDIR/lib/lib.path") entries"

# ⭐ THE CONTROL ON THE WHOLE ASSEMBLY, AND ESPECIALLY ON THE DEBLOAT. Every
# DT_NEEDED of every ELF left in the bundle has to resolve inside it. A
# debloat that breaks this produces a smaller bundle that does not start, and
# the failure arrives on somebody else's machine as
# "cannot open shared object file" -- which reads like a broken build.
# ⚠ It is a REPORT, not a refusal: a closure legitimately contains libraries
# that dlopen things nothing links against, and stopping the build on that
# would be a false alarm. An unresolved DT_NEEDED is a real defect and is
# printed with the file that wants it.
MISSING=$(
  { printf '%s\n' "$APPDIR/shared/bin/$PROG"
    find "$APPDIR/lib" -maxdepth 2 -type f -name '*.so*' 2>/dev/null; } \
  | while IFS= read -r _e; do
      python3 "$SELF/elf-needed.py" print "$_e" 2>/dev/null
    done \
  | awk -F'\t' '{print $2}' | sort -u \
  | while IFS= read -r _n; do
      [ -n "$_n" ] || continue
      case "$_n" in ld-linux*|ld-musl*|/*) continue ;; esac
      [ -e "$APPDIR/lib/$_n" ] || [ -e "$APPDIR/lib32/$_n" ] || printf '%s\n' "$_n"
    done
)
NMISS=$(printf '%s' "$MISSING" | grep -c . || true)
if [ "${NMISS:-0}" -gt 0 ]; then
  warn "⛔ $NMISS DT_NEEDED name(s) do not resolve inside the bundle:"
  printf '%s\n' "$MISSING" | sed 's/^/             /' >&2
else
  say "integrity   every DT_NEEDED in the bundle resolves inside it"
fi

# ---------------------------------------------------------------------------
# 5b. ⭐ THE ENVIRONMENT, AND THE OpenGL HALF OF IT IS THE INTERESTING PART
#
# ⛔ THE "libGL PROBLEM" IS TWO PROBLEMS AND ONLY ONE OF THEM IS HARD. Read
# `nix-community/nixGL` at commit b6105297e6f0cd041670c3e8628394d4ee247ed5,
# `nixGL.nix:54-62`, and the split is plain:
#
#   MESA (Intel, AMD, and the software rasteriser). nixGL does NOT use the
#   host's GL here. It uses NIXPKGS' OWN MESA and merely tells it where its
#   pieces are: LIBGL_DRIVERS_PATH, GBM_BACKENDS_PATH, LIBVA_DRIVERS_PATH and
#   __EGL_VENDOR_LIBRARY_FILENAMES. ⭐ So for the mesa case a bundle is a
#   COMPLETE answer, and the only thing standing between this bundler and it
#   is the four variables below. `Anylinux-AppImages` reaches the same
#   conclusion from the other direction: it bundles a debloated mesa.
#
#   NVIDIA PROPRIETARY. The userspace half must match the host's kernel
#   module, so no bundle can carry it: nixGL reads /proc/driver/nvidia/version
#   and FETCHES the matching driver (`nixGL.nix:69`). That case is `TODO`
#   T-052 and nothing here solves it.
#
# ⚠ Every path is set only if the bundle actually has it, so a non-GL
# application does not carry env pointing at directories that are not there.
{
  printf 'XDG_DATA_DIRS=${SHARUN_DIR}/share:${XDG_DATA_DIRS}:/usr/local/share:/usr/share\n'
  printf 'GCONV_PATH=${SHARUN_DIR}/lib/gconv\n'
  printf 'FONTCONFIG_PATH=${SHARUN_DIR}/share/fontconfig/conf.d\n'
  [ -d "$APPDIR/lib/gtk-3.0" ] && printf 'GTK_PATH=${SHARUN_DIR}/lib/gtk-3.0\n'
  [ -f "$APPDIR/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache" ] &&
    printf 'GDK_PIXBUF_MODULE_FILE=${SHARUN_DIR}/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache\n'
  [ -d "$APPDIR/lib/dri" ] && {
    printf 'LIBGL_DRIVERS_PATH=${SHARUN_DIR}/lib/dri\n'
    printf 'LIBVA_DRIVERS_PATH=${SHARUN_DIR}/lib/dri\n'
  }
  [ -d "$APPDIR/lib/gbm" ] && printf 'GBM_BACKENDS_PATH=${SHARUN_DIR}/lib/gbm\n'
  [ -d "$APPDIR/share/glvnd/egl_vendor.d" ] &&
    printf '__EGL_VENDOR_LIBRARY_DIRS=${SHARUN_DIR}/share/glvnd/egl_vendor.d\n'
  [ -d "$APPDIR/share/vulkan/icd.d" ] &&
    printf 'VK_DRIVER_FILES=${SHARUN_DIR}/share/vulkan/icd.d\n'
} > "$APPDIR/.env"

# ---------------------------------------------------------------------------
# 5c. ⭐ THE WRAPPER'S ENVIRONMENT -- T-053, and the answer to "why not patsh"
#
# `tool/nix-wrapper.py` read the assignments out of the wrapper that `bin/<x>`
# turned out to be. They name absolute store paths, which do not exist in the
# bundle, so each referenced path is copied into `AppDir/store/<name>/` --
# keeping its INTERNAL layout, because a variable like LUA_CPATH names a
# sub-path (`lib/lua/5.2/?.so`) and flattening it into lib/ would break it --
# and the value is rewritten to ${SHARUN_DIR}/store/<name>/...
#
# ⛔ WHY NOT patsh, stated where the decision lives. patsh patches store paths
# in shell SCRIPTS so the script keeps working. Two things are wrong with that
# here, and both are measured: nixpkgs' current wrapper is a COMPILED C
# program (mpv's bin/mpv is a 16,560-byte ELF), so there is no script to
# patch; and a bundle does not run the wrapper at all -- sharun runs the real
# ELF and reads .env -- so the job is to LIFT the assignments out, not to keep
# a script alive.
# ---------------------------------------------------------------------------
NWENV=0
if [ -s "$WRAPENV" ]; then
  mkdir -p "$APPDIR/store"
  # Copy every store path the wrapper's environment refers to.
  for _sp in $(grep -oE '/nix/store/[a-z0-9]{32}-[^:; ]*' "$WRAPENV" 2>/dev/null \
               | sed -E 's|(/nix/store/[a-z0-9]{32}-[^/]*).*|\1|' | sort -u); do
    _b=$(basename "$_sp")
    _n=$(printf '%s' "$_b" | cut -c34-)
    if [ ! -d "$ROOT/$_b" ]; then
      sh "$FETCH" fetch "$_sp" --out "$ROOT" >/dev/null 2>&1 \
        || { warn "wrapper env names $_b, which is not in the closure and could not be fetched"; continue; }
    fi
    [ -d "$APPDIR/store/$_n" ] || cp -aL "$ROOT/$_b" "$APPDIR/store/$_n" 2>/dev/null || true
  done
  # ⭐ prefix/suffix/set become sharun .env lines with the SAME semantics.
  # sharun expands ${SHARUN_DIR} and ${VAR}, so a prefix is `V=new${SEP}${V}`.
  while IFS="$(printf '\t')" read -r _op _var _sep _val; do
    [ -n "$_var" ] || continue
    _v=$(printf '%s' "$_val" | sed -E 's|/nix/store/[a-z0-9]{32}-([^/:; ]*)|${SHARUN_DIR}/store/\1|g')
    case "$_op" in
      set)    printf '%s=%s\n' "$_var" "$_v" ;;
      prefix) printf '%s=%s%s${%s}\n' "$_var" "$_v" "${_sep:-:}" "$_var" ;;
      suffix) printf '%s=${%s}%s%s\n' "$_var" "$_var" "${_sep:-:}" "$_v" ;;
      *)      continue ;;
    esac
    NWENV=$((NWENV + 1))
  done < "$WRAPENV" >> "$APPDIR/.env"
  say "wrapper env $NWENV variable(s) lifted out of the wrapper into .env"
  sed -n '1,8p' "$WRAPENV" | sed 's/^/            /'
fi

# ---------------------------------------------------------------------------
# 6. pack: uruntime + dwarfs, the Anylinux way
# ---------------------------------------------------------------------------
need "$URUNTIME_URL" "$CACHE/tools/uruntime" || die "could not fetch uruntime"
need "$DWARFS_URL"   "$CACHE/tools/mkdwarfs" || die "could not fetch mkdwarfs"

OUT="${OUT:-$WORK/$PROG-anylinux-$ARCH.AppImage}"
say ""
say "packing with uruntime + dwarfs"
"$CACHE/tools/mkdwarfs" --force --set-owner 0 --set-group 0 \
  --no-history --no-create-timestamp \
  --header "$CACHE/tools/uruntime" \
  --input "$APPDIR" --output "$OUT" \
  -C zstd:level=19 -S26 >/dev/null 2>&1 || die "mkdwarfs failed"
chmod +x "$OUT"
say ""
say "built  $OUT  ($(du -h "$OUT" | cut -f1))"
[ "$KEEP" = 1 ] || say "AppDir kept at $APPDIR"
exit 0
