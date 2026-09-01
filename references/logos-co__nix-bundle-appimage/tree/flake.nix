{
  description = "Bundle Nix derivations into AppImages";

  inputs = {
    logos-nix.url = "github:logos-co/logos-nix";
    nixpkgs.follows = "logos-nix/nixpkgs";
    nix-bundle-dir = {
      url = "github:logos-co/nix-bundle-dir";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, logos-nix, nixpkgs, nix-bundle-dir }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f {
        inherit system;
        pkgs = nixpkgs.legacyPackages.${system};
      });
    in
    {
      lib = forAllSystems ({ pkgs, system, ... }: {
        mkAppImage = import ./mkAppImage.nix { inherit pkgs; };
      });

      bundlers = forAllSystems ({ pkgs, system, ... }:
        let
          mkAppImage = import ./mkAppImage.nix { inherit pkgs; };
          bundleDirBundlers = nix-bundle-dir.bundlers.${system};

          # Build a bundler that uses a specific nix-bundle-dir bundler variant
          mkBundler = bundleDirBundler: drv:
            let
              name = drv.pname or drv.name or "AppImage";
              bundle = bundleDirBundler drv;

              # Find .desktop file in the derivation
              desktopDir = "${drv}/share/applications";
              desktopDirExists = builtins.pathExists desktopDir;
              desktopFiles =
                if desktopDirExists
                then builtins.filter
                  (f: pkgs.lib.hasSuffix ".desktop" f)
                  (builtins.attrNames (builtins.readDir desktopDir))
                else [];
              desktopFile =
                if desktopFiles == []
                then throw ''
                  nix-bundle-appimage: No .desktop file found in ${desktopDir}.
                  Use the mkAppImage library function directly to specify a custom .desktop file:
                    nix-bundle-appimage.lib.''${system}.mkAppImage {
                      drv = <your-derivation>;
                      bundle = <your-bundle>;
                      desktopFile = ./your-app.desktop;
                      icon = ./your-icon.png;
                    }
                ''
                else "${desktopDir}/${builtins.head desktopFiles}";

              # Find icon in the derivation
              iconSearchDirs = [
                "${drv}/share/icons/hicolor/256x256/apps"
                "${drv}/share/icons/hicolor/128x128/apps"
                "${drv}/share/icons/hicolor/scalable/apps"
                "${drv}/share/icons/hicolor/512x512/apps"
                "${drv}/share/icons/hicolor/64x64/apps"
                "${drv}/share/icons/hicolor/48x48/apps"
                "${drv}/share/pixmaps"
              ];

              findIcon = dirs:
                if dirs == []
                then throw ''
                  nix-bundle-appimage: No icon found in ${drv}/share/icons/ or ${drv}/share/pixmaps/.
                  Use the mkAppImage library function directly to specify a custom icon:
                    nix-bundle-appimage.lib.''${system}.mkAppImage {
                      drv = <your-derivation>;
                      bundle = <your-bundle>;
                      desktopFile = ./your-app.desktop;
                      icon = ./your-icon.png;
                    }
                ''
                else
                  let
                    dir = builtins.head dirs;
                    rest = builtins.tail dirs;
                    exists = builtins.pathExists dir;
                    files = if exists then builtins.attrNames (builtins.readDir dir) else [];
                    imageFiles = builtins.filter
                      (f: pkgs.lib.hasSuffix ".png" f
                        || pkgs.lib.hasSuffix ".svg" f
                        || pkgs.lib.hasSuffix ".xpm" f)
                      files;
                  in
                    if imageFiles != []
                    then "${dir}/${builtins.head imageFiles}"
                    else findIcon rest;

              iconFile = findIcon iconSearchDirs;
            in
              mkAppImage {
                inherit drv name bundle;
                desktopFile = desktopFile;
                icon = iconFile;
              };
        in
          # Mirror each nix-bundle-dir bundler variant as an AppImage bundler
          builtins.mapAttrs (_name: mkBundler) bundleDirBundlers
      );
    };
}
