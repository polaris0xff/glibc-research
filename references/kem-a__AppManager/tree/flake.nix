{
  description = "AppManager - AppImage installer and manager";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }: let
    forAllSystems = nixpkgs.lib.genAttrs [
      "x86_64-linux"
      "aarch64-linux"
    ];
  in {
    packages = forAllSystems (system: {
      app-manager = nixpkgs.legacyPackages.${system}.callPackage ./package.nix {};
      default = self.packages.${system}.app-manager;
    });

    devShells = forAllSystems (system: let
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      default = pkgs.mkShell {
        inputsFrom = [ self.packages.${system}.app-manager ];
      };
    });
  };
}
