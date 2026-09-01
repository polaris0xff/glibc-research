{ pkgs }:

{ drv
, name ? drv.pname or drv.name or "AppImage"
, bundle
, desktopFile
, icon
}:

let
  appimageRuntime = import ./fetchAppImageRuntime.nix { inherit pkgs; };

  # Extract the Exec= binary name from the .desktop file.
  desktopContent = builtins.readFile desktopFile;
  execLine = builtins.head (
    builtins.filter
      (line: builtins.match "Exec=.*" line != null)
      (pkgs.lib.splitString "\n" desktopContent)
  );
  execCmd = builtins.head (
    pkgs.lib.splitString " " (builtins.replaceStrings ["Exec="] [""] execLine)
  );
  execBasename = builtins.baseNameOf execCmd;

  # Determine icon file extension from the path
  iconPath = builtins.toString icon;
  iconExt = pkgs.lib.last (pkgs.lib.splitString "." iconPath);
in

pkgs.stdenv.mkDerivation {
  pname = "${name}-appimage";
  version = drv.version or "0";

  src = null;
  dontUnpack = true;
  dontFixup = true;

  nativeBuildInputs = [ pkgs.file pkgs.squashfsTools ];

  buildPhase = ''
    # Construct AppDir
    mkdir -p AppDir/usr/bin AppDir/usr/lib

    # Copy bundle contents into usr/
    cp -a ${bundle}/bin/. AppDir/usr/bin/
    if [ -d "${bundle}/lib" ]; then
      cp -a ${bundle}/lib/. AppDir/usr/lib/
    fi

    # Copy any extra dirs from the bundle (e.g., plugins, modules, share)
    for dir in ${bundle}/*/; do
      dirname=$(basename "$dir")
      if [ "$dirname" != "bin" ] && [ "$dirname" != "lib" ]; then
        cp -a "$dir" "AppDir/usr/$dirname"
        # Nix store files are read-only; restore write perms so later phases
        # (e.g., installing icons under share/) can create subdirectories.
        chmod -R u+w "AppDir/usr/$dirname"
      fi
    done

    # Install .desktop file at AppDir root
    cp ${desktopFile} AppDir/${name}.desktop

    # Patch Exec= to just the binary name (AppImage convention)
    sed -i "s|^Exec=.*|Exec=${execBasename}|" AppDir/${name}.desktop

    # Install icon at AppDir root (file manager thumbnail)
    cp ${icon} AppDir/${name}.${iconExt}
    ln -s ${name}.${iconExt} AppDir/.DirIcon

    # Install icon in FreeDesktop standard path (taskbar/window icon)
    mkdir -p AppDir/usr/share/icons/hicolor/256x256/apps
    cp ${icon} AppDir/usr/share/icons/hicolor/256x256/apps/${name}.${iconExt}

    # Patch Icon= field in .desktop to match installed icon name
    sed -i "s|^Icon=.*|Icon=${name}|" AppDir/${name}.desktop

    # Create AppRun
    cat > AppDir/AppRun <<'EOF'
#!/bin/sh
set -e
APPDIR="$(dirname "$(readlink -f "$0")")"
export PATH="$APPDIR/usr/bin:$PATH"
export LD_LIBRARY_PATH="$APPDIR/usr/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
# libxkbcommon is compiled with a hardcoded /nix/store xkeyboard-config path
# that won't exist at runtime; without this, Qt Wayland's keymap dispatch
# calls xkb_context_ref on a NULL context and segfaults.
if [ -d "$APPDIR/usr/share/X11/xkb" ]; then
  export XKB_CONFIG_ROOT="$APPDIR/usr/share/X11/xkb"
fi
exec "$APPDIR/usr/bin/${execBasename}" "$@"
EOF
    chmod +x AppDir/AppRun

    # Build AppImage: create squashfs with gzip (broad compatibility), then concatenate with runtime
    mksquashfs AppDir appimage.squashfs -root-owned -noappend -comp gzip
    cat ${appimageRuntime} appimage.squashfs > ${name}.AppImage
    chmod +x ${name}.AppImage
  '';

  installPhase = ''
    mkdir -p $out
    cp ${name}.AppImage $out/
  '';
}
