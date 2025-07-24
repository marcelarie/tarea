{
  description = "A Nix-flake-based Rust development environment";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs: let
    supportedSystems = ["x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin"];
    forEachSupportedSystem = f:
      inputs.nixpkgs.lib.genAttrs supportedSystems (system:
        f {
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              inputs.fenix.overlays.default
              inputs.self.overlays.default
            ];
          };
          system = system;
        });
  in {
    packages = forEachSupportedSystem ({
      pkgs,
      system,
    }: {
      default = let
        rustPlatform = pkgs.makeRustPlatform {
          rustc = pkgs.rustToolchain;
          cargo = pkgs.rustToolchain;
        };
      in
        rustPlatform.buildRustPackage {
          pname = "tarea";
          version = "0.1.0";

          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          buildInputs = [pkgs.sqlite];
          nativeBuildInputs = [pkgs.pkg-config];
          doCheck = false;
        };
    });

    overlays.default = final: prev: {
      rustToolchain =
        inputs.fenix.packages.${prev.stdenv.hostPlatform.system}
      .latest.withComponents [
          "cargo"
          "clippy"
          "rustc"
          "rustfmt"
          "rust-src"
        ];
    };

    devShells = forEachSupportedSystem ({pkgs, ...}: {
      default = pkgs.mkShell {
        packages = with pkgs; [
          rustToolchain
          rust-analyzer
          openssl
          pkg-config
          cargo-deny
          cargo-edit
          cargo-watch
          sqlite
        ];

        RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
        shellHook = ''
          export RUSTFLAGS="-C link-arg=-L${pkgs.sqlite.out}/lib $RUSTFLAGS"
          export LD_LIBRARY_PATH=${pkgs.sqlite.out}/lib:$LD_LIBRARY_PATH
        '';
      };
    });
  };
}
