{ pkgs ? import <nixpkgs> {} }:

pkgs.stdenv.mkDerivation {
  name = "simulator";
  buildInputs = [
    pkgs.rustc
    pkgs.cargo
  ];

  shellHook = ''
    echo "Welcome to the development environment for your-project-name"
    # Any setup commands you need to run when entering the shell
  '';
}
