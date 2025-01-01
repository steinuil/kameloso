{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      naersk,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        naerskBuildPackage = (pkgs.callPackage naersk { }).buildPackage;
      in
      rec {
        defaultPackage = packages.kameloso;

        packages.kameloso = naerskBuildPackage {
          src = ./.;
        };

        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rustc
            cargo
          ];

          buildInputs = with pkgs; [
            rust-analyzer
            clippy
            lldb
            rustfmt
            typescript-language-server
          ];
        };
      }
    );
}
