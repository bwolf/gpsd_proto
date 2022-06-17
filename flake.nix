{
  inputs = {
    nixpkgs.url = github:nixos/nixpkgs/nixpkgs-unstable;
    fenix = {
      url = github:nix-community/fenix;
      inputs.nixpkgs.follows = "nixpkgs";
    };
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, fenix, utils, ... }:
    utils.lib.eachDefaultSystem (system:
      let
        toolchain = fenix.packages.${system}.toolchainOf {
          channel = "1.61.0";
          sha256 = "sha256-oro0HsosbLRAuZx68xd0zfgPl6efNj2AQruKRq3KA2g=";
        };
        pkgs = import nixpkgs { inherit system; };
      in rec {
        devShell = nixpkgs.legacyPackages.${system}.mkShell {
          packages = [
            (toolchain.withComponents [
              "cargo" "rustc" "rust-src" "rustfmt" "clippy"
            ])

            fenix.packages.${system}.rust-analyzer
            pkgs.cargo-release
          ];
          RUST_BACKTRACE=1;
        };
      });
}
