{
  description = "Molerat client implementation";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = { self, nixpkgs }:
  let 
    system = "aarch64-darwin";
    pkgs = nixpkgs.legacyPackages.${system};
  in {
    devShells.${system}.default = 
      pkgs.mkShell {
        buildInputs = with pkgs; [
          rustc
          rustfmt
          cargo
          rust-analyzer
          libiconv
          clippy
          pkg-config
          darwin.apple_sdk.frameworks.SystemConfiguration
        ];
        shellHook = ''
          exec zsh
        '';
      };
  };
}
