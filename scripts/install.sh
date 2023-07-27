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
#     $ ./install-movement.sh
#
# Note: Make sure to review and customize the script variables and paths 
#       according to your specific requirements before running.
#
# Author: Liam Monninger
# Version: 1.0
################################################################################

set -e

# Set the URL for fetching movementctl
MOVEMENTCTL_URL="https://raw.githubusercontent.com/movemntdev/movement-hack/main/bin/movementctl.sh"

# Set the AvalancheGo version
AVALANCHEGO_VERSION="v1.10.3"

# Set the directory paths
AVALANCHEGO_DIR="$HOME/.avalanchego"
MOVEMENT_DIR="$HOME/.movement"
MOVEMENT_WORKSPACE="$MOVEMENT_DIR/workspace"
PLUGINS_DIR="$MOVEMENT_DIR/plugins"
BIN_DIR="$MOVEMENT_DIR/bin"

# Create the necessary directories
mkdir -p "$AVALANCHEGO_DIR" "$MOVEMENT_DIR" "$PLUGINS_DIR" "$BIN_DIR" "$MOVEMENT_WORKSPACE"

cd $MOVEMENT_WORKSPACE

# Detect the OS
OS=$(uname -s)
case $OS in
  Linux*)     OS=linux-;;
  Darwin*)    OS=macos-;;
  CYGWIN*)    OS=windows-;;
  *)          echo "Unsupported OS: $OS"; exit 1;;
esac

# Detect the architecture
ARCH=$(uname -m)
case $ARCH in
  x86_64*)   ARCH=amd64-;;
  aarch64*)  ARCH=arm64-;;
  arm64*)    ARCH="";; # Apple M1
  *)         echo "Unsupported architecture: $ARCH"; exit 1;;
esac

# Download and install avalanche-network-runner
curl -sSfL https://raw.githubusercontent.com/ava-labs/avalanche-network-runner/main/scripts/install.sh | sh -s

# Add avalanche-network-runner binary to PATH
echo 'export PATH="$HOME/bin:$PATH"' >> "$HOME/.bashrc"

# Reload the bash profile
source "$HOME/.bashrc"

# Download and install AvalancheGo
if [ "$OS" == "linux-" ]; then
  AVALANCHEGO_RELEASE_URL="https://github.com/ava-labs/avalanchego/releases/download/$AVALANCHEGO_VERSION/avalanchego-linux-$ARCH-$AVALANCHEGO_VERSION.tar.gz"
  AVALANCHEGO_ARCHIVE="avalanchego-linux-$ARCH-$AVALANCHEGO_VERSION.tar.gz"
  wget "$AVALANCHEGO_RELEASE_URL" -O "$AVALANCHEGO_ARCHIVE"
  mkdir -p "$AVALANCHEGO_DIR"
  tar xvf "$AVALANCHEGO_ARCHIVE" -C "$AVALANCHEGO_DIR" --strip-components=1
elif [ "$OS" == "macos-" ]; then
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

# Reload the bash profile
source "$HOME/.bashrc"

# Clone the subnet repository if not already cloned
if [ ! -d "$MOVEMENT_DIR/movement-subnet" ]; then
  git clone https://github.com/movemntdev/movement-subnet "$MOVEMENT_DIR/movement-subnet"
fi

# Set up the developer environment if not already set up
cd "$MOVEMENT_DIR/movement-subnet/vm/aptos-vm"
./script/dev_setup.sh

# Build the subnet binary
cargo build --release -p subnet

# Move the subnet binary to the plugin directory
mv "$MOVEMENT_DIR/movement-subnet/vm/aptos-vm/target/release/subnet" "$PLUGINS_DIR/subnet"

# Symlink the subnet binary with its subnet ID
ln -s "$PLUGINS_DIR/qCP4kDnEWVorqyoUmcAtAmJybm8gXZzhHZ7pZibrJJEWECooU" "$PLUGINS_DIR/subnet"
ln -s "$AVALANCHEGO_DIR/plugins/qCP4kDnEWVorqyoUmcAtAmJybm8gXZzhHZ7pZibrJJEWECooU" "$PLUGINS_DIR/subnet"

# Clone the movement repository if not already cloned
if [ ! -d "$MOVEMENT_DIR/movement-subnet" ]; then
  git clone https://github.com/movemntdev/movement-subnet "$MOVEMENT_DIR/movement-subnet"
fi

# Set up the developer environment if not already set up
cd "$MOVEMENT_DIR/movement-subnet/vm/aptos-vm"
./script/dev_setup.sh

# Build the movement binary
cargo build --release -p movement

# Move the movement binary to the appropriate directory
mv "$MOVEMENT_DIR/movement-subnet/vm/aptos-vm/target/release/movement" "$BIN_DIR"

# Add movement binary directory to PATH
echo 'export PATH="$HOME/.movement/bin:$PATH"' >> "$HOME/.bashrc"

# Reload the bash profile
source "$HOME/.bashrc"

# Clone the subnet proxy repository if not already cloned
if [ ! -d "$MOVEMENT_DIR/subnet-request-proxy" ]; then
  git clone https://github.com/movemntdev/subnet-request-proxy "$MOVEMENT_DIR/subnet-request-proxy"
fi

# Download and install movementctl
curl -sSfL "$MOVEMENTCTL_URL" -o "$BIN_DIR/movementctl"
chmod +x "$BIN_DIR/movementctl"

echo "movementctl installed successfully."

# Add movement binary directory to PATH
echo 'export PATH="$HOME/.movement/bin:$PATH"' >> "$HOME/.bashrc"

# Reload the bash profile
source "$HOME/.bashrc"

# Clean up artifacts
cd $MOVEMENT_DIR
rm -rf $MOVEMENT_WORKSPACE