{
  description = "A devShell example";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }@inputs:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        darwinFrameworks = if system == "x86_64-darwin" then with pkgs.darwin.apple_sdk.frameworks; [
          IOKit
          SystemConfiguration
          AppKit
        ] else [];
      in
      with pkgs;
      {
        devShells.default = mkShell {
          buildInputs = [
            clang
            libcxx
            rocksdb
            openssl
            pkg-config
            eza
            fd
            rust-bin.stable.latest.default
          ] ++ darwinFrameworks;

          shellHook = ''
            alias ls=eza
            alias find=fd

            echo "> Entered Nix-powered M1 simulator environment"
            export OLD_PS1="$PS1"
            PS1="(M1-nix) $PS1"

            # Set MACOSX_DEPLOYMENT_TARGET for compatibility
            export MACOSX_DEPLOYMENT_TARGET="10.13"
            echo "MACOSX_DEPLOYMENT_TARGET set to: $MACOSX_DEPLOYMENT_TARGET"

            # Explicitly set the C++ compiler to use clang++, ensuring it's the Nix-provided one
            export CXX="${clang}/bin/clang++"

            # Adding -v for verbose output during compilation to help diagnose linker issues
            export NIX_CFLAGS_COMPILE="-v $NIX_CFLAGS_COMPILE"
            
            # Ensure linker flags include the path to libc++ and enable verbose output
            export NIX_LDFLAGS="$NIX_LDFLAGS -L${libcxx}/lib -lc++ -v"

            # Set LDFLAGS for manual make/cmake builds
            export LDFLAGS="-L${libcxx}/lib -lc++ -v"
          '';
        };
      }
    );
}