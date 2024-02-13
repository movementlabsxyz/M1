{ pkgs ? import <nixpkgs> {}, darwin ? pkgs.darwin }:

pkgs.mkShell rec {
  name = "simulator";
  buildInputs = with pkgs; [
    clang
    libiconv
    rustup
    zlib
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.IOKit
    darwin.apple_sdk.frameworks.SystemConfiguration
    darwin.apple_sdk.frameworks.AppKit
    libcxx
  ];

  RUSTC_VERSION = builtins.readFile ./rust-toolchain;

  shellHook = ''
    export MACOSX_DEPLOYMENT_TARGET=10.13
    export CC="$(which clang)"
    export CXX="$(which clang++)"
    export RUSTFLAGS="-C link-arg=-stdlib=libc++ -C link-arg=-lc++"
    export LDFLAGS="-stdlib=libc++ -lc++"
    export LDFLAGS="$LDFLAGS -v"

    # Configure rustup to use the specified Rust version
    rustup override set $RUSTC_VERSION

    echo "Welcome to the movement simulator for the M1 network"

    # Run 'env' to validate the environment variables setup
    env
  '';
}
