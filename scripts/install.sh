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

# Colors
RED="\033[0;31m"
GREEN="\033[0;32m"
YELLOW="\033[0;33m"
RESET="\033[0m"

log_info() {
  echo -e "${GREEN}[INFO]${RESET} $1"
}

log_warning() {
  echo -e "${YELLOW}[WARNING]${RESET} $1"
}

log_error() {
  echo -e "${RED}[ERROR]${RESET} $1"
}

# Set global variables
MOVEMENTCTL_URL="https://raw.githubusercontent.com/movementlabsxyz/M1/main/scripts/movementctl.sh"
RELEASES_URL="https://github.com/movementlabsxyz/M1/releases"
AVALANCHEGO_VERSION="v1.10.3"
AVALANCHEGO_DIR="$HOME/.avalanchego"
MOVEMENT_DIR="$HOME/.movement"
MOVEMENT_WORKSPACE="$MOVEMENT_DIR/workspace"
PLUGINS_DIR="$MOVEMENT_DIR/plugins"
BIN_DIR="$MOVEMENT_DIR/bin"
SOURCE_DIR="$MOVEMENT_DIR/source"
SUBNET_ID="2gLyawqthdiyrJktJmdnDAb1XVc6xwJXU6iJKu3Uwj21F2mXAK"

# CLI arguments
LATEST=true
SOURCE=false
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
            --source)
                SOURCE=true
                shift
                ;;
            --build)
                BUILD=true
                SOURCE=true
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
                SOURCE=true
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

    if [[ ! -z "$ARCH" ]]; then
      echo $ARCH
      map
      return
    fi

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

show_config(){

  DEV_DISPLAY=""
  $DEV && DEV_DISPLAY="with dev dependencies"|| DEV_DISPLAY="without dev dependencies"
  BUILD_DISPLAY=""
  $BUILD && BUILD_DISPLAY="built locally"|| BUILD_DISPLAY="downloaded from releases"
  log_info "Installing movement@$VERSION $OS $ARCH ($FARCH), $DEV_DISPLAY, $BUILD_DISPLAY."

}

setup() {
  
  log_warning "Removing previous installation, if one exists."
  rm -rf "$MOVEMENT_DIR" "$PLUGINS_DIR" "$BIN_DIR" "$MOVEMENT_WORKSPACE" "$SOURCE_DIR"
  log_info "Making new .movement directories."
  mkdir -p "$AVALANCHEGO_DIR" "$AVALANCHEGO_DIR/plugins" "$MOVEMENT_DIR" "$PLUGINS_DIR" "$BIN_DIR" "$MOVEMENT_WORKSPACE" "$SOURCE_DIR"

}


pull() {

  rm -rf "$MOVEMENT_DIR/M1"
  if [[ $LATEST = true ]]; then
    local URL="$RELEASES_URL/latest/download/m1-with-submodules.tar.gz"
    log_info "Downloading full source from $URL ..."
    curl -SfL $URL -o "$SOURCE_DIR/M1.tar.gz" --progress-bar
    log_info "Downloaded full source from $URL ."
  else
    local URL="$RELEASES_URL/download/$VERSION/m1-with-submodules.tar.gz"
    log_info "Downloading full source from $URL..."
    curl -SfL $URL -o "$PLUGINS_DIR/source/M1.tar.gz" --progress-bar
    log_info "Downloaded full source from $URL."
  fi

  log_info "Extracting full source..."
  tar -xzf "$SOURCE_DIR/M1.tar.gz" -C "$SOURCE_DIR"
  log_info "Extracted full source."

}

# Function to install Rust
install_rust() {
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
}

builder_deps(){

  # Detect OS
  if [ "$(uname)" = "Darwin" ]; then
      # macOS
  
      # Install Homebrew if not installed
      which brew > /dev/null || /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
  
      # Install build essentials
      brew install automake autoconf libtool
  
      # Install Rust
      install_rust
  
  elif [ "$(expr substr $(uname -s) 1 5)" = "Linux" ]; then
    # Linux

    if grep -qE "(Microsoft|WSL)" /proc/version &> /dev/null ; then
        # Windows Subsystem for Linux
        powershell.exe "iex ((New-Object System.Net.WebClient).DownloadString('https://chocolatey.org/install.ps1'))" # Install Chocolatey
        powershell.exe "choco install rust-msvc -y" # Install Rust using Chocolatey
    else
        # Pure Linux
        . /etc/os-release

        case $ID in
            debian|ubuntu)
                sudo apt update
                sudo apt install -y build-essential
                install_rust
                ;;
            fedora)
                sudo dnf install -y make automake gcc-c++ kernel-devel
                install_rust
                ;;
            centos|rhel)
                sudo yum groupinstall 'Development Tools'
                install_rust
                ;;
            suse|opensuse|sles)
                sudo zypper install -y -t pattern devel_basis
                install_rust
                ;;
            arch|manjaro)
                sudo pacman -S base-devel
                install_rust
                ;;
            *)
                echo "Unsupported Linux distribution."
                exit 1
                ;;
        esac
    fi

  else
      echo "Unsupported OS."
      exit 1
  fi

  
}

dev_deps() {

    log_info "Entering M1."
    cd $MOVEMENT_WORKSPACE

    log_info "Entering aptos-pre-core."
    cd "$MOVEMENT_DIR/M1/aptos-pre-core"
    log_info "Running Aptos dev_setup."
    chmod 755 ./scripts/dev_setup.sh
    echo "yes" | ./scripts/dev_setup.sh

}

avalanche_setup() {

  # Download and install avalanche-network-runner
  # log_info "Setting up Avalanche."
  # curl -sSfL https://raw.githubusercontent.com/ava-labs/avalanche-network-runner/main/scripts/install.sh | sh -s

  # Add avalanche-network-runner binary to PATH
  echo 'export PATH="$HOME/bin:$PATH"' >> "$HOME/.bashrc"

  # Reload the bash profile
  . "$HOME/.bashrc"

  # Download and install AvalancheGo
  if [ "$OS" == "linux" ]; then
    AVALANCHEGO_RELEASE_URL="https://github.com/ava-labs/avalanchego/releases/download/$AVALANCHEGO_VERSION/avalanchego-linux-$ARCH-$AVALANCHEGO_VERSION.tar.gz"
    AVALANCHEGO_ARCHIVE="avalanchego-linux-$ARCH-$AVALANCHEGO_VERSION.tar.gz"
    # wget "$AVALANCHEGO_RELEASE_URL" -O "$AVALANCHEGO_ARCHIVE"
    # mkdir -p "$AVALANCHEGO_DIR"
    # tar xvf "$AVALANCHEGO_ARCHIVE" -C "$AVALANCHEGO_DIR" --strip-components=1
  elif [ "$OS" == "macos" ]; then
    AVALANCHEGO_RELEASE_URL="https://github.com/ava-labs/avalanchego/releases/download/$AVALANCHEGO_VERSION/avalanchego-macos-$AVALANCHEGO_VERSION.zip"
    AVALANCHEGO_ARCHIVE="avalanchego-macos-$AVALANCHEGO_VERSION.zip"
    # wget "$AVALANCHEGO_RELEASE_URL" -O "$AVALANCHEGO_ARCHIVE"
    # mkdir -p "$AVALANCHEGO_DIR"
    # unzip "$AVALANCHEGO_ARCHIVE" -d "$AVALANCHEGO_DIR"
  else
    echo "Unsupported OS: $OS"
    exit 1
  fi

  # Add AvalancheGo binary directory to PATH
  echo "export PATH=\"$AVALANCHEGO_DIR:\$PATH\"" >> "$HOME/.bashrc"

}


build() {

    cd $SOURCE_DIR/m1-with-submodules/movement-sdk
    
    # Notify use that we're building
    log_info "Building movement@$VERSION $FARCH-$OS."

    # Build the movement binary
    cargo build --release -p movement --features="aptos,sui"

    # Copy the movement binary to the appropriate directory
    cp "target/release/movement" "$BIN_DIR/movement"
}

dev() {
  dev_deps
}

download(){

  log_info "Downloading released binaries for subnet and movement@$VERSION $FARCH-$OS."

  if [[ $LATEST = true ]]; then
    log_info "Downloading movement from $RELEASES_URL/latest/download/movement-$FARCH-$OS."
    curl -SfL "$RELEASES_URL/latest/download/movement-$FARCH-$OS" -o "$BIN_DIR/movement" --progress-bar
  else
    log_info "Downloading movement from $RELEASES_URL/download/$VERSION/movement-$FARCH-$OS."
    curl -SfL "$RELEASES_URL/download/$VERSION/movement-$FARCH-$OS" -o "$BIN_DIR/movement" --progress-bar
  fi


  # Symlink the subnet binary with its subnet ID
  ln -sf "$PLUGINS_DIR/subnet" "$PLUGINS_DIR/$SUBNET_ID" 
  ln -sf "$PLUGINS_DIR/subnet" "$AVALANCHEGO_DIR/plugins/$SUBNET_ID"

  chmod -R 755 $BIN_DIR

}


path(){
  log_info "Adding $BIN_DIR to bash profile."
  echo "export PATH=\"${BIN_DIR}:\$PATH\"" >> ~/.bashrc
}

cleanup(){
  # Clean up artifacts
  log_info "Cleaning up workspace."
  cd $MOVEMENT_DIR
  rm -rf $MOVEMENT_WORKSPACE
}

main() {
  
  # parse the args into environment variables
  parse "$@"

  # detect the OS and architecture
  detect

  # show the configuration
  show_config

  if [[ "$FARCH-$OS" != x86_64-linux && "$FARCH-$OS" != aarch64-linux && "$BUILD" != true ]]; then
    log_error "$FARCH-$OS is not yet supported. Please use the --build option."
    exit 1
  fi

  # setup the .movement directory
  setup

  # if we're building or using dev, we'll need to pull the repo
  if [[ ("$SOURCE" = true) ]]; then
      pull
  fi

  # if we're using dev, we'll need to setup the dev environment
  if [ "$DEV" = true ]; then
      dev
  fi

  echo "Installing build essentials"
  # if we're building, we'll need to build the binaries
  if [ "$BUILD" = true ]; then
      # install the build dependencies
      build_dependencies
      build
  else 
      echo "Downloading"
      download
  fi

  path

  cleanup

}

# Ubuntu dependencies installation with checks
install_ubuntu_deps() {
    log_info "Updating package lists..."
    sudo apt-get update

    if ! dpkg -l | grep -qw build-essential; then
        log_info "Installing build-essential..."
        sudo apt-get install -y build-essential
    else
        log_info "build-essential is already installed."
    fi

    if ! command -v rustc &>/dev/null; then
        log_info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    else
        log_info "Rust is already installed."
    fi

    if ! dpkg -l | grep -qw lld; then
        log_info "Installing lld..."
        sudo apt-get install -y lld
    else
        log_info "lld is already installed."
    fi

    if ! dpkg -l | grep -qw libssl-dev; then
        log_info "Installing libssl-dev..."
        sudo apt-get install -y libssl-dev
    else
        log_info "libssl-dev is already installed."
    fi

    if ! dpkg -l | grep -qw libudev-dev; then
        log_info "Installing libudev-dev..."
        sudo apt-get install -y libudev-dev
    else
        log_info "libudev-dev is already installed."
    fi

    if ! dpkg -l | grep -qw libpq-dev; then
        log_info "Installing pq..."
        sudo apt-get install -y libpq-dev
    else
        log_info "libpq-dev (pq) is already installed."
    fi
}

# MacOS dependencies installation with checks
install_macos_deps() {
    if ! command -v brew &>/dev/null; then
        log_info "Installing Homebrew..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    else
        log_info "Homebrew is already installed."
    fi

    log_info "Updating Homebrew..."
    brew update

    if ! brew list gcc &>/dev/null; then
        log_info "Installing build-essential..."
        brew install gcc
    else
        log_info "gcc (build-essential) is already installed."
    fi

    if ! command -v rustc &>/dev/null; then
        log_info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    else
        log_info "Rust is already installed."
    fi

    if ! brew list llvm &>/dev/null; then
        log_info "Installing lld..."
        brew install llvm
    else
        log_info "lld (llvm) is already installed."
    fi

    if ! brew list openssl &>/dev/null; then
        log_info "Installing libssl..."
        brew install openssl
    else
        log_info "libssl (openssl) is already installed."
    fi

    # zstd
    if ! brew list zstd &>/dev/null; then
        log_info "Installing zstd..."
        brew install zstd
    else
        log_info "zstd is already installed."
    fi

}

# Windows unsupported configuration message
unsupported_windows() {
    log_warning "Windows dependency configuration is not currently supported."
}

# Install build dependencies
build_dependencies() {
    case $OS in
        linux)
            install_ubuntu_deps
            ;;
        macos)
            install_macos_deps
            ;;
        windows)
            unsupported_windows
            ;;
        *)
            log_error "Unsupported OS: $OS"
            exit 1
            ;;
    esac
}

main "$@"
