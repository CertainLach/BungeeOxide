{
  description = "Prodocol Description Something 2";
  inputs.nixpkgs.url = "github:nixos/nixpkgs/release-26.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs = {
      nixpkgs.follows = "nixpkgs";
      flake-utils.follows = "flake-utils";
    };
  };
  inputs.crane = {
    url = "github:ipetkov/crane";
    inputs.nixpkgs.follows = "nixpkgs";
  };
  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    crane,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs =
        import nixpkgs
        {
          inherit system;
          overlays = [(import rust-overlay)];
        };
      rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      craneLib = (crane.mkLib pkgs).overrideToolchain rust;
    in {
      devShells.default = craneLib.devShell {
        packages = with pkgs; [
          tree-sitter
          nodejs
          binutils
          cargo-edit
          pkg-config
          openssl
        ];
      };
    });
}
