#!/usr/bin/env bash
# Smoke test for the AppImage packaging contract.
#
# `nix build` proves an AppImage was produced; it proves nothing about whether
# the thing inside it can start on someone else's machine, which is the only
# reason this repo exists. Everything asserted below is a way that has broken,
# or could break invisibly:
#
#   * the payload is copied with `cp -a ${bundle}/bin/. AppDir/usr/bin/`, and
#     `cp -a src/.` vs `cp -a src/*` differ on dotfiles. nix-bundle-dir puts
#     the real ELF beside a launcher as `.<name>.elf`, so getting that wrong
#     ships an AppImage whose launcher execs a file that is not there — and it
#     still builds, and `--appimage-extract` still succeeds.
#   * AppRun does `exec "$APPDIR/usr/bin/<exec>"`, which has to work whether
#     that entry is a plain binary or a launcher script. Both shapes exist,
#     so both are built here.
#   * the interpreter baked into the payload is what decides whether the
#     AppImage runs on a distro other than the build host. It is per-arch
#     (see PSABI below) and a wrong one is invisible until a user reports it.
#
# Two subjects, deliberately tiny, one per bundle shape:
#   hello        via qtCliApp -- plain binaries in bin/, no launcher
#   xkbcli       via qtApp    -- libxkbcommon in the closure pulls in the
#                               xkeyboard-config data, which is what makes
#                               nix-bundle-dir emit a launcher + companion ELF
#
# Usage:  tests/smoke.sh [flake-ref]     (default: the checkout, ".")
set -uo pipefail

FLAKE="${1:-.}"
# Absolutise a local flake ref BEFORE cd-ing into the scratch dir, or `.` would
# resolve to the scratch dir and nix would report "could not find a flake.nix".
# A ref containing ':' is a URL (github:, git+https:, path:) and is left alone.
case "$FLAKE" in
  *:*) ;;
  *)   FLAKE="$(cd "$FLAKE" 2>/dev/null && pwd)" || { echo "smoke: no such flake dir: ${1:-.}" >&2; exit 1; }
       FLAKE="path:$FLAKE" ;;
esac
export SMOKE_FLAKE="$FLAKE"

WORK="$(mktemp -d)"
trap 'chmod -R u+w "$WORK" 2>/dev/null; rm -rf "$WORK"' EXIT
cd "$WORK"

pass=0; fail=0
ok()   { printf '  \033[32mok\033[0m   %s\n' "$1"; pass=$((pass+1)); }
bad()  { printf '  \033[31mFAIL\033[0m %s\n' "$1"; [ -n "${2:-}" ] && printf '         %s\n' "$2"; fail=$((fail+1)); }
check(){ if eval "$2" >/dev/null 2>&1; then ok "$1"; else bad "$1" "${3:-}"; fi; }

# The loader path this architecture's psABI mandates. Deliberately not a single
# prefix: /lib64/ld-linux-aarch64.so.1 does not exist on current Fedora or
# openSUSE, so "just use /lib64" breaks arm64.
#
# Note this one assertion only discriminates on x86_64. The bundler used to set
# /lib/<loader>, which is wrong on x86_64 (Fedora keeps the 64-bit loader in
# /lib64) but happens to be exactly right on aarch64 -- so on arm it passed
# before the fix too. The DT_RPATH, shim and trampoline assertions below are
# what carry the arm job.
case "$(uname -m)" in
  x86_64)  PSABI=/lib64/ld-linux-x86-64.so.2 ;;
  aarch64) PSABI=/lib/ld-linux-aarch64.so.1  ;;
  *) echo "smoke: unsupported arch $(uname -m); nothing to assert"; exit 0 ;;
esac

interp()   { patchelf --print-interpreter "$1" 2>/dev/null; }
have_tag() { readelf -d "$1" 2>/dev/null | grep -q "($2)"; }

# ---------------------------------------------------------------------------
# Subjects. Kept here rather than in a committed test flake so this script
# stays runnable against any flake ref, local or remote, with no second lock
# file to drift. Each is a tiny derivation carrying the .desktop file and icon
# that the bundlers discover.
# ---------------------------------------------------------------------------
cat > subjects.nix <<'NIX'
{ pkgs }:

let
  # 1x1 PNG. Nothing reads it; the bundler only needs an icon to exist.
  icon = pkgs.runCommand "smoke-icon.png" { } ''
    printf '%s' iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg== \
      | base64 -d > $out
  '';

  desktop = name: exe: pkgs.writeText "${name}.desktop" ''
    [Desktop Entry]
    Type=Application
    Name=${name}
    Exec=${exe}
    Icon=${name}
    Categories=Utility;
  '';

  # Copy (not symlink) the one binary we care about: the bundler walks bin/,
  # and this keeps the subject to a single executable and its closure.
  mkSubject = { name, from, exe }: pkgs.runCommand name { } ''
    mkdir -p $out/bin $out/share/applications $out/share/icons/hicolor/256x256/apps
    cp ${from}/bin/${exe} $out/bin/${exe}
    chmod u+w $out/bin/${exe}
    cp ${desktop name exe} $out/share/applications/${name}.desktop
    cp ${icon} $out/share/icons/hicolor/256x256/apps/${name}.png
  '';
in
{
  cli = mkSubject { name = "smokecli"; from = pkgs.hello;         exe = "hello";  };
  gui = mkSubject { name = "smokegui"; from = pkgs.libxkbcommon;  exe = "xkbcli"; };
}
NIX

cat > build.nix <<'NIX'
let
  flake  = builtins.getFlake (builtins.getEnv "SMOKE_FLAKE");
  system = builtins.currentSystem;
  pkgs   = flake.inputs.nixpkgs.legacyPackages.${system};
  subjects = import ./subjects.nix { inherit pkgs; };
in
{
  cli = flake.bundlers.${system}.qtCliApp subjects.cli;
  gui = flake.bundlers.${system}.qtApp    subjects.gui;
}
NIX

echo "== building AppImages with $FLAKE =="
built_cli=0; built_gui=0
nix build --impure -f build.nix cli -o out-cli && built_cli=1
nix build --impure -f build.nix gui -o out-gui && built_gui=1

# Extract each AppImage with its own runtime (no FUSE needed) and assert the
# payload, then run it. --appimage-extract always lands in ./squashfs-root.
extract() {  # extract <appimage> <destdir>
  rm -rf "$2"; mkdir -p "$2"
  ( cd "$2" && "$1" --appimage-extract >/dev/null 2>&1 ) && [ -d "$2/squashfs-root" ]
}

CLI_IMG=""; GUI_IMG=""
# Resolve the glob carefully: without nullglob, an unmatched `*.AppImage`
# expands to the literal pattern, which is non-empty and would green
# "AppImage built" before the existence check below fails for the wrong reason.
if [ "$built_cli" = 1 ]; then
  set -- "$(readlink -f out-cli)"/*.AppImage
  [ -f "$1" ] && CLI_IMG="$1"
fi
if [ "$built_gui" = 1 ]; then
  set -- "$(readlink -f out-gui)"/*.AppImage
  [ -f "$1" ] && GUI_IMG="$1"
fi

echo
echo "== the artifact is a single self-contained AppImage =="
for pair in "cli:$CLI_IMG" "gui:$GUI_IMG"; do
  tag="${pair%%:*}"; img="${pair#*:}"
  if [ -z "$img" ] || [ ! -f "$img" ]; then bad "$tag: AppImage built"; continue; fi
  ok "$tag: AppImage built"
  check "$tag: one executable file, $(basename "$img")" "[ -x '$img' ] && [ -f '$img' ]"
  # ELF magic, then 'AI' + type 2 at offset 8 -- the AppImage magic bytes.
  check "$tag: carries the type-2 AppImage magic" \
        "[ \"\$(od -An -tx1 -j8 -N3 '$img' | tr -d ' ')\" = '414902' ]" \
        "without it desktop integration and --appimage-* options do not apply"
done

[ -n "$CLI_IMG" ] && extract "$CLI_IMG" ex-cli && C=ex-cli/squashfs-root || C=""
[ -n "$GUI_IMG" ] && extract "$GUI_IMG" ex-gui && G=ex-gui/squashfs-root || G=""

echo
echo "== payload: plain path (qtCliApp -> no launcher) =="
if [ -n "$C" ]; then
  # Full ELF magic (\x7fELF), not a substring grep of the first four bytes —
  # "ELF" alone would also match a non-ELF file that happened to start with
  # those letters.
  check "usr/bin/hello is an ELF, not a script" \
        "[ \"\$(od -An -tx1 -N4 '$C/usr/bin/hello' | tr -d ' ')\" = '7f454c46' ]"
  check "no launcher and no hidden companion in usr/bin" \
        "! ls -a '$C/usr/bin' | grep -qE '^\.[^.]'" \
        "a headless bundle should ship the binary itself"
  check "PT_INTERP is the psABI path ($PSABI)" \
        "[ \"\$(interp '$C/usr/bin/hello')\" = '$PSABI' ]" \
        "got: $(interp "$C/usr/bin/hello")"
  check "DT_RPATH is set (bundled libs beat a stale LD_LIBRARY_PATH)" \
        "have_tag '$C/usr/bin/hello' RPATH"
  check "DT_RUNPATH is NOT set (it loses to LD_LIBRARY_PATH)" \
        "[ -f '$C/usr/bin/hello' ] && ! have_tag '$C/usr/bin/hello' RUNPATH"
  check "AppRun execs the bin/ entry we just checked" \
        "grep -q 'exec \"\$APPDIR/usr/bin/hello\"' '$C/AppRun'"
else
  bad "cli AppImage extracted"
fi

echo
echo "== payload: launcher path (qtApp + xkb -> launcher + companion ELF) =="
if [ -n "$G" ]; then
  check "usr/bin/xkbcli is a launcher script" "head -c2 '$G/usr/bin/xkbcli' | grep -q '#!'"
  # THE dotfile question: `cp -a src/.` copies hidden entries, `cp -a src/*`
  # does not. Get it wrong and AppRun execs a launcher whose companion is
  # missing -- a build that succeeds and an AppImage that cannot start.
  check "the companion .xkbcli.elf came across into usr/bin" \
        "[ \"\$(od -An -tx1 -N4 '$G/usr/bin/.xkbcli.elf' | tr -d ' ')\" = '7f454c46' ]" \
        "cp -a \${bundle}/bin/. must copy hidden entries; cp -a bin/* would not"
  check "the launcher exports XKB_CONFIG_ROOT" "grep -q XKB_CONFIG_ROOT '$G/usr/bin/xkbcli'"
  check "the xkb data it points at is in the payload" "[ -d '$G/usr/share/X11/xkb' ]"
  check "companion PT_INTERP is the psABI path ($PSABI)" \
        "[ \"\$(interp '$G/usr/bin/.xkbcli.elf')\" = '$PSABI' ]" \
        "got: $(interp "$G/usr/bin/.xkbcli.elf")"
  check "companion has DT_RPATH" "have_tag '$G/usr/bin/.xkbcli.elf' RPATH"
  # Guarded on existence: a `! have_tag` on a missing file passes vacuously,
  # which is how a dropped companion could look like a green run.
  check "companion has no DT_RUNPATH" \
        "[ -f '$G/usr/bin/.xkbcli.elf' ] && ! have_tag '$G/usr/bin/.xkbcli.elf' RUNPATH"
  # `cp -a ${bundle}/lib/.` is the other half of the payload copy. (The plain
  # subject has no lib/ at all -- hello only needs libc, which the bundler
  # leaves to the host -- so this is the subject that can prove it happened.)
  check "the library its payload needs is in usr/lib" \
        "ls '$G/usr/lib'/libxkbcommon.so* >/dev/null 2>&1"
  check "AppRun execs the launcher, which resolves the companion itself" \
        "grep -q 'exec \"\$APPDIR/usr/bin/xkbcli\"' '$G/AppRun'"
else
  bad "gui AppImage extracted"
fi

echo
echo "== regressions that have actually happened =="
for pair in "cli:$C" "gui:$G"; do
  tag="${pair%%:*}"; d="${pair#*:}"
  [ -n "$d" ] || continue
  # nixpkgs' Qt hooks leave `.<name>-wrapped` C wrappers next to their
  # binaries; shipping one exports /nix/store paths that do not exist.
  check "$tag: no nixpkgs '-wrapped' wrapper in usr/bin" "! ls -a '$d/usr/bin' | grep -q -- '-wrapped'"
  # The LD_PRELOAD shim existed only to fake readlink("/proc/self/exe") for
  # binaries run through an ld.so trampoline. No trampoline, no shim.
  check "$tag: no libprocself_fix.so in the payload" "! find '$d' -name libprocself_fix.so | grep -q ."
  check "$tag: no __BUNDLE_REAL_EXE handshake" "! grep -rqs __BUNDLE_REAL_EXE '$d/usr/bin' '$d/AppRun'"
done
if [ -n "$G" ]; then
  # A trampoline execs some OTHER program and passes the real binary to it as
  # an argument -- `exec "$p" "$REAL" "$@"`, where $p is a loader located at
  # runtime. Direct exec puts $REAL first. Matching that shape rather than the
  # string "ld-linux" matters: the old launcher held the loader path in a
  # variable, so grepping for the name passed against it and proved nothing.
  check "the launcher execs the companion, not a loader with it as an argument" \
        "! grep -qE 'exec +\"\\\$[A-Za-z_]+\" +\"\\\$REAL\"' '$G/usr/bin/xkbcli'" \
        "handing the program to ld.so makes /proc/self/exe lie inside the AppImage"
  check "the launcher has no runtime loader-path probe" \
        "! grep -qE '\\\$INTERP_NAME' '$G/usr/bin/xkbcli'" \
        "probing for the loader at runtime is what the psABI path replaced"
fi

echo
echo "== runs on other distros =="
# The whole promise of an AppImage is "runs on a machine that is not the build
# host", so this is the assertion that matters most. APPIMAGE_EXTRACT_AND_RUN
# because containers have no FUSE; -w /tmp because that extraction needs a
# writable cwd.
run_in() {  # run_in <image> <appimage> <args> <expected-regex> <label>
  local image="$1" f="$2" args="$3" want="$4" label="$5" out rc
  out="$(docker run --rm -v "$f:/app.AppImage:ro" -w /tmp -e APPIMAGE_EXTRACT_AND_RUN=1 \
           "$image" sh -c "/app.AppImage $args" 2>&1)"; rc=$?
  if [ "$rc" -eq 0 ] && printf '%s' "$out" | grep -qE -- "$want"; then
    ok "$image: $label"
  else
    bad "$image: $label" "exit $rc: $(printf '%s' "$out" | tail -2 | tr '\n' ' ')"
  fi
}
# SMOKE_DISTROS lets CI pin tags/digests (latest moves). Defaults stay
# convenient for local runs.
: "${SMOKE_DISTROS:=ubuntu:latest fedora:latest}"
if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
  for image in $SMOKE_DISTROS; do
    # A plain binary in bin/ ...
    [ -n "$CLI_IMG" ] && run_in "$image" "$CLI_IMG" "" 'Hello, world' "plain payload prints its greeting"
    # ... and a launcher, which must find its own companion ELF from whatever
    # argv[0] and cwd the AppImage runtime hands it.
    [ -n "$GUI_IMG" ] && run_in "$image" "$GUI_IMG" "--version" '[0-9]' "launcher payload resolves its companion"
  done
elif [ -n "${GITHUB_ACTIONS:-}" ]; then
  # Cross-distro exec is the primary signal this CI exists for. Skipping it
  # on a runner that lost Docker would green the layout checks and miss the
  # exact failure mode the job is meant to catch.
  echo "  FAIL docker is required under GitHub Actions (cross-distro run is the point)" >&2
  exit 1
else
  echo "  -- docker unavailable, skipping the cross-distro run"
  echo "     (the payload assertions above still ran; portability did not)"
fi

echo
echo "== $pass passed, $fail failed =="
[ "$fail" -eq 0 ]
