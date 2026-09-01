{ pkgs }:

let
  version = "20251108";

  sources = {
    x86_64-linux = {
      url = "https://github.com/AppImage/type2-runtime/releases/download/${version}/runtime-x86_64";
      sha256 = "2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d";
    };
    aarch64-linux = {
      url = "https://github.com/AppImage/type2-runtime/releases/download/${version}/runtime-aarch64";
      sha256 = "00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444";
    };
  };

  src = sources.${pkgs.stdenv.hostPlatform.system}
    or (throw "AppImage runtime: unsupported system ${pkgs.stdenv.hostPlatform.system}");
in

pkgs.fetchurl {
  inherit (src) url sha256;
}
