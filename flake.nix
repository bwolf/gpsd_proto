{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rust = pkgs.rust-bin.stable."1.56.1";
        rust-bin = rust.default;
        rust-src = rust.rust-src;
      in {
        devShell = pkgs.mkShell {
          buildInputs = [
            rust.default
            rust.rust-src
            pkgs.rust-analyzer
          ];

          RUST_BACKTRACE=1;
          RUST_SRC="${rust.rust-src}/lib/rustlib/src/rust/library";
        };
      });
}
