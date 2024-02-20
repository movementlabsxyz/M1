{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
    }:
    flake-utils.lib.eachSystem ["aarch64-darwin" "x86_64-linux"] (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        avalanche-network-runner = import ./avalanche-network-runner.nix { inherit pkgs; };

        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rust;
          rustc = rust;
        };

        runtimeDependencies = with pkgs; [
          openssl
        ];

        frameworks = pkgs.darwin.apple_sdk.frameworks;

        buildDependencies = with pkgs; [
            libclang.lib
            libz
            clang
            pkg-config
            rustPlatform.bindgenHook]
          ++ runtimeDependencies
          ++ lib.optionals stdenv.isDarwin [
            frameworks.Security
            frameworks.CoreServices
            frameworks.SystemConfiguration
            frameworks.AppKit
          ];

        developmentDependencies = with pkgs; [
            rust
            avalanchego
            avalanche-network-runner
          ]
          ++ buildDependencies;
        
        subnet-cargo-toml = builtins.fromTOML (builtins.readFile ./subnet/Cargo.toml);
      in
        with pkgs; {
          packages = flake-utils.lib.flattenTree rec {
            subnet = rustPlatform.buildRustPackage {
              pname = subnet-cargo-toml.package.name;
              version = subnet-cargo-toml.package.version;
              
              env = { LIBCLANG_PATH = "${libclang.lib}/lib"; }
              // (lib.optionalAttrs (stdenv.cc.isClang && stdenv.isDarwin) { NIX_LDFLAGS = "-l${stdenv.cc.libcxx.cxxabi.libName}"; });

              src = ./.;
              cargoLock = {
                lockFile = ./Cargo.lock;
              };

              nativeBuildInputs = buildDependencies;
              buildInputs = runtimeDependencies;

              doCheck = false;
            };
            
            default = subnet;
          };

          devShells.default = mkShell {
            NIX_LDFLAGS="-l${stdenv.cc.libcxx.cxxabi.libName}";
            buildInputs = developmentDependencies;
            shellHook = ''
              export PATH=$PATH:${avalanche-network-runner}/bin
              export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
            '';
          };
        }
    );
}
