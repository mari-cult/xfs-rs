{
  description = "The purely functional package manager";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs =
    {
      self,
      nixpkgs,
    }:
    let
      eachSystem = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
        "riscv64-linux"
      ];
      mkPkgs = system: nixpkgs.legacyPackages.${system};
    in
    {
      devShells = eachSystem (
        system:
        let
          pkgs = mkPkgs system;
        in
        with pkgs;
        {
          default = mkShell rec {
            nativeBuildInputs = [
              nixfmt
              nixd
              rustfmt
              clippy
              rustc
              cargo
              rust-analyzer
              cargo-nextest
              xfsprogs
            ];

            buildInputs = [
            ];

            LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;
          };
        }
      );
    };
}
