{ profile ? "stable", date ? "2025-02-20", oxalica ? "d95582a9", rustup ? true }:
let
  oxalica_overlay = builtins.fetchTarball "https://github.com/oxalica/rust-overlay/archive/${oxalica}.tar.gz";
  pkgs = import <nixpkgs> {
    overlays = [ (import oxalica_overlay) ];
  };
  rust = (pkgs.rust-bin.${profile}.${date}.default.override { extensions = [ "rust-src" ]; });
in
pkgs.mkShell {
  buildInputs = with pkgs; [
    nixpkgs-fmt
    openssl
    pkg-config
    libiconv
    git
  ] ++ pkgs.lib.optional (!rustup) rust
  ++ pkgs.lib.optional (system == "aarch64-darwin") darwin.apple_sdk.frameworks.Security;
  shellHook = ''
      cat <<EOF >rust-toolchain.toml
    [toolchain]
    channel = "${profile}-${date}"
    components = [ "rust-src" ]
    EOF
  '';
}
