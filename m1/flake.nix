{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";

    # Dev tools
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs = inputs:
  inputs.flake-parts.lib.mkFlake { inherit inputs; } {
    systems = import inputs.systems;
    imports = [
      inputs.treefmt-nix.flakeModule
    ];
    perSystem = { config, self', pkgs, lib, system, ... }:
      let
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        nonRustDeps = [
          pkgs.libiconv
          pkgs.zlib
          pkgs.darwin.apple_sdk.frameworks.IOKit
          pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
          pkgs.darwin.apple_sdk.frameworks.AppKit
        ]; 
        rust-toolchain = pkgs.symlinkJoin {
          name = "rust-toolchain";
          paths = [ pkgs.rustc pkgs.cargo pkgs.cargo-watch pkgs.rust-analyzer pkgs.rustPlatform.rustcSrc ];
        };
      in
      {
        # Rust package
        packages.default = pkgs.rustPlatform.buildRustPackage {
          inherit (cargoToml.package) name version;
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          buildInputs = lib.optional (lib.strings.hasPrefix "darwin" system) pkgs.darwin.apple_sdk.frameworks.IOKit;
        };

        # Rust dev environment
        devShells.default = pkgs.mkShell {
          inputsFrom = [
            config.treefmt.build.devShell
          ];
          shellHook = ''
            # For rust-analyzer 'hover' tooltips to work.
            export RUST_SRC_PATH=${pkgs.rustPlatform.rustLibSrc}

            # Set MACOSX_DEPLOYMENT_TARGET for compatibility
            export MACOSX_DEPLOYMENT_TARGET="10.13"
            echo "MACOSX_DEPLOYMENT_TARGET set to: $MACOSX_DEPLOYMENT_TARGET"

            export OLD_PS1="$PS1"
            PS1="(M1-nix) $PS1"

            echo
            echo "🍎🍎 Run 'just <recipe>' to get started"
          '';
          buildInputs = nonRustDeps;
          nativeBuildInputs = with pkgs; [
            just
            rust-toolchain
          ] ++ lib.optional (lib.strings.hasPrefix "darwin" system) pkgs.darwin.apple_sdk.frameworks.IOKit;
          RUST_BACKTRACE = 1;
        };

        # Add your auto-formatters here.
        # cf. https://numtide.github.io/treefmt/
        treefmt.config = {
          projectRootFile = "flake.nix";
          programs = {
            nixpkgs-fmt.enable = true;
            rustfmt.enable = true;
          };
        };
      };
  };
  
}