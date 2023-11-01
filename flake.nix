{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, naersk, fenix }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };

      toolchain = fenix.packages.${system}.fromToolchainName {
        name = "stable";
        sha256 = "sha256-rLP8+fTxnPHoR96ZJiCa/5Ans1OojI7MLsmSqR2ip8o=";
      };

      package-name = (naersk.lib.${system}.override {
        inherit (toolchain) cargo rustc;
      }).buildPackage { src = ./.; };

    in {

      packages.${system} = {
        default = package-name;
        inherit package-name;
      };

      devShells.${system}.default = nixpkgs.legacyPackages.${system}.mkShell {
        packages = [
          (toolchain.withComponents [
            "cargo"
            "rustc"
            "rust-src"
            "rustfmt"
            "clippy"
          ])

          fenix.packages.${system}.rust-analyzer

          pkgs.cargo-audit
          pkgs.cargo-bloat
          pkgs.cargo-outdated
          pkgs.cargo-release
          pkgs.cargo-watch
        ];
        RUST_BACKTRACE = 1;
      };
    };
}
