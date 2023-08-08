#!/bin/bash
################################################################################
# This script is intended to be run on a portable environment.
# It assumes that Rust, Cargo, and developer dependencies are already installed.
#
# This script performs the following tasks:
# - ...
#
# Prerequisites:
# - Rust and Cargo installed
# - Developer dependencies installed
#
# Usage:
# - Run this script on the portable environment:
#     $ ./install.sh
#
# Note: Make sure to review and customize the script variables and paths 
#       according to your specific requirements before running.
#
# Author: Liam Monninger
# Version: 1.0
################################################################################

set -e

# Set global variables
MOVEMENTCTL_URL="https://raw.githubusercontent.com/movemntdev/movement-hack/main/bin/movementctl.sh"
RELEASES_URL="https://api.github.com/repos/movementdev/M1/releases"
AVALANCHEGO_VERSION="v1.10.3"
AVALANCHEGO_DIR="$HOME/.avalanchego"
MOVEMENT_DIR="$HOME/.movement"
MOVEMENT_WORKSPACE="$MOVEMENT_DIR/workspace"
PLUGINS_DIR="$MOVEMENT_DIR/plugins"
BIN_DIR="$MOVEMENT_DIR/bin"

# CLI arguments
LATEST=true
BUILD=false
VERSION=""
DEV=false
ARCH=""
FARCH=""
OS=""
SOS=""
# Parse command line arguments
parse() {

  while [ "$#" -gt 0 ]; do
        case "$1" in
            --latest)
                LATEST=true
                if [ ! -z "$VERSION" ]; then
                    echo "Error: --latest cannot be used with --version."
                    exit 1
                fi
                shift
                ;;
            --build)
                BUILD=true
                shift
                ;;
            --version)
                if [ "$#" -lt 2 ]; then
                    echo "Error: --version requires an argument."
                    exit 1
                fi
                LATEST=false
                VERSION="$2"
                if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
                    echo "Error: Invalid version format. Expected format: major.minor.patch"
                    exit 1
                fi
                shift 2
                ;;
            --dev)
                DEV=true
                shift
                ;;
            --arch)
                if [ "$#" -lt 2 ]; then
                    echo "Error: --arch requires an argument."
                    exit 1
                fi
                ARCH="$2"
                shift 2
                ;;
            *)
                echo "Error: Unknown argument: $1"
                exit 1
                ;;
        esac
    done

    if [ $LATEST = true ]; then
      VERSION="latest"
    fi

}

# map the architecture onto the appropriate full architecture name
map() {

  case $ARCH in
    amd64)
      FARCH="x86_64"
      ;;
    arm64)
      FARCH="aarch64"
      ;;
    "")
      FARCH="aarch64"
      ;;
    *)
      echo "Unsupported architecture: $ARCH"
      exit 1
      ;;
  esac



}

detect() {
    # Detect the OS
    OS=$(uname -s)
    case $OS in
        Linux*)     OS=linux;;
        Darwin*)    OS=macos;;
        CYGWIN*)    OS=windows;;
        *)          echo "Unsupported OS: $OS"; exit 1;;
    esac

    # Detect the architecture
    ARCH=$(uname -m)
    case $ARCH in
        x86_64*)   ARCH=amd64;;
        aarch64*)  ARCH=arm64;;
        arm64*)    ARCH="";; # Apple M1
        *)         echo "Unsupported architecture: $ARCH"; exit 1;;
    esac

    map

}



pull() {

  rm -rf "$MOVEMENT_DIR/M1"
  git clone --recursive https://github.com/movemntdev/M1 "$MOVEMENT_DIR/M1"
  git submodule init
  git pull origin main
  git submodule update --recursive --remote

}

setup() {

  mkdir -p "$AVALANCHEGO_DIR" "$MOVEMENT_DIR" "$PLUGINS_DIR" "$BIN_DIR" "$MOVEMENT_WORKSPACE"

}

deps() {

    cd $MOVEMENT_WORKSPACE

    cd "$MOVEMENT_DIR/movement-subnet/vm/aptos-vm"
    ./script/dev_setup.sh

}

avalanche_setup() {

  # Download and install avalanche-network-runner
  curl -sSfL https://raw.githubusercontent.com/ava-labs/avalanche-network-runner/main/scripts/install.sh | sh -s

  # Add avalanche-network-runner binary to PATH
  echo 'export PATH="$HOME/bin:$PATH"' >> "$HOME/.bashrc"

  # Reload the bash profile
  . "$HOME/.bashrc"

  # Download and install AvalancheGo
  if [ "$OS" == "linux" ]; then
    AVALANCHEGO_RELEASE_URL="https://github.com/ava-labs/avalanchego/releases/download/$AVALANCHEGO_VERSION/avalanchego-linux-$ARCH-$AVALANCHEGO_VERSION.tar.gz"
    AVALANCHEGO_ARCHIVE="avalanchego-linux-$ARCH-$AVALANCHEGO_VERSION.tar.gz"
    wget "$AVALANCHEGO_RELEASE_URL" -O "$AVALANCHEGO_ARCHIVE"
    mkdir -p "$AVALANCHEGO_DIR"
    tar xvf "$AVALANCHEGO_ARCHIVE" -C "$AVALANCHEGO_DIR" --strip-components=1
  elif [ "$OS" == "macos" ]; then
    AVALANCHEGO_RELEASE_URL="https://github.com/ava-labs/avalanchego/releases/download/$AVALANCHEGO_VERSION/avalanchego-macos-$AVALANCHEGO_VERSION.zip"
    AVALANCHEGO_ARCHIVE="avalanchego-macos-$AVALANCHEGO_VERSION.zip"
    wget "$AVALANCHEGO_RELEASE_URL" -O "$AVALANCHEGO_ARCHIVE"
    mkdir -p "$AVALANCHEGO_DIR"
    unzip "$AVALANCHEGO_ARCHIVE" -d "$AVALANCHEGO_DIR"
  else
    echo "Unsupported OS: $OS"
    exit 1
  fi

  # Add AvalancheGo binary directory to PATH
  echo 'export PATH="$HOME/.movement/avalanchego:$PATH"' >> "$HOME/.bashrc"

}

dev_setup() {
    cd "$MOVEMENT_DIR/movement-subnet/vm/aptos-vm"
    ./script/dev_setup.sh
}

build() {
    # Build the subnet binary
    cargo build --release -p subnet

    # Move the subnet binary to the plugin directory
    mv "$MOVEMENT_DIR/movement-subnet/vm/aptos-vm/target/release/subnet" "$PLUGINS_DIR/subnet"

    # Symlink the subnet binary with its subnet ID
    ln -s "$PLUGINS_DIR/qCP4kDnEWVorqyoUmcAtAmJybm8gXZzhHZ7pZibrJJEWECooU" "$PLUGINS_DIR/subnet"
    ln -s "$AVALANCHEGO_DIR/plugins/qCP4kDnEWVorqyoUmcAtAmJybm8gXZzhHZ7pZibrJJEWECooU" "$PLUGINS_DIR/subnet"

    # Build the movement binary
    cargo build --release -p movement

    # Move the movement binary to the appropriate directory
    mv "$MOVEMENT_DIR/movement-subnet/vm/aptos-vm/target/release/movement" "$BIN_DIR"
}

dev() {
  deps
  avalanche_setup
}

download(){

  curl -sSfL "$RELEASES_URL/download/$VERSION/subnet-$UNAME$FARCH" -o "$PLUGINS_DIR/subnet"
  curl -ssFL "$RELEASES_URL/download/$VERSION/movement-$UNAME$FARCH" -o "$BIN_DIR/movement"

  # Symlink the subnet binary with its subnet ID
  ln -s "$PLUGINS_DIR/qCP4kDnEWVorqyoUmcAtAmJybm8gXZzhHZ7pZibrJJEWECooU" "$PLUGINS_DIR/subnet"
  ln -s "$AVALANCHEGO_DIR/plugins/qCP4kDnEWVorqyoUmcAtAmJybm8gXZzhHZ7pZibrJJEWECooU" "$PLUGINS_DIR/subnet"

}

movementctl() {
  curl -sSfL "$MOVEMENTCTL_URL" -o "$BIN_DIR/movementctl"
  chmod +x "$BIN_DIR/movementctl"
}

path(){
  echo "export PATH=\"${BIN_DIR}:\$PATH\"" >> ~/.bashrc
}

cleanup(){
  # Clean up artifacts
  cd $MOVEMENT_DIR
  rm -rf $MOVEMENT_WORKSPACE
}

main() {
  
  # parse the args
  parse "$@"

  # detect the OS and architecture
  detect

  # setup the .movement directory
  setup

  # if we're building or using dev, we'll need to pull the repo
  if [ "$BUILD" = true | "$DEV" = true ]; then
      pull
  fi

  # if we're using dev, we'll need to setup the dev environment
  if [ "$DEV" = true ]; then
      dev
  fi

  # if we're building, we'll need to build the binaries
  if [ "$BUILD" = true ]; then
      build
  else 
      download
  fi

  movementctl

  path

  cleanup

}
