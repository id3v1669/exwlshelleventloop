{
  description = "exwlshelleventloop";

  inputs = { 
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default-linux";
  };

  outputs = inputs @ 
  { self
  , nixpkgs
  , systems
  , ... 
  }:
  let
    eachSystem = nixpkgs.lib.genAttrs (import systems);

    pkgsFor = (system: import nixpkgs {
      inherit system;
      overlays = [ ];
    });
  in 
  {
    packages = eachSystem (system: {
      default = nixpkgs.legacyPackages.${system}.callPackage ./nix/package.nix{ };
    });

    defaultPackage = eachSystem (system: self.packages.${system}.default);
    
    devShells = eachSystem (system: let pkgs = (pkgsFor system); in{
      default = pkgs.mkShell {
        name = "exwlshelleventloop devShell";
        nativeBuildInputs = with pkgs; [
          cargo
          rustc
          scdoc
          pkg-config

          # Libs to gen docs
          wayland
          glib

          # Tools
          cargo-audit
          cargo-deny
          clippy
          rust-analyzer
          rustfmt
        ];
      };
    });
  };
}