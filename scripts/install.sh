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
MOVEMENTCTL_URL="https://raw.githubusercontent.com/movemntdev/M1/main/scripts/movementctl.sh"
RELEASES_URL="https://github.com/movemntdev/M1/releases"
AVALANCHEGO_VERSION="v1.10.3"
AVALANCHEGO_DIR="$HOME/.avalanchego"
MOVEMENT_DIR="$HOME/.movement"
MOVEMENT_WORKSPACE="$MOVEMENT_DIR/workspace"
PLUGINS_DIR="$MOVEMENT_DIR/plugins"
BIN_DIR="$MOVEMENT_DIR/bin"
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
  rm -rf "$AVALANCHEGO_DIR" "$AVALANCHEGO_DIR/plugins" "$MOVEMENT_DIR" "$PLUGINS_DIR" "$BIN_DIR" "$MOVEMENT_WORKSPACE"
  log_info "Making new .movement directories."
  mkdir -p "$AVALANCHEGO_DIR" "$AVALANCHEGO_DIR/plugins" "$MOVEMENT_DIR" "$PLUGINS_DIR" "$BIN_DIR" "$MOVEMENT_WORKSPACE"

}


pull() {

  rm -rf "$MOVEMENT_DIR/M1"
  if [[ $LATEST = true ]]; then
    local URL="$RELEASES_URL/latest/download/m1-with-submodules.tar.gz"
    log_info "Downloading full source from $URL..."
    curl -sSfL $URL -o "$MOVEMENT_DIR/M1.tar.gz"
  else
    local URL="$RELEASES_URL/download/$VERSION/m1-with-submodules.tar.gz"
    log_info "Downloading full source from $URL..."
    curl -sSfL $URL -o "$PLUGINS_DIR/M1.tar.gz"
  fi

  tar -xzf "$MOVEMENT_DIR/M1.tar.gz" -C "$MOVEMENT_DIR"

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
  log_info "Setting up Avalanche."
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
  echo "export PATH=\"$AVALANCHEGO_DIR:\$PATH\"" >> "$HOME/.bashrc"

}


build() {
    

    # Build the subnet binary
    cargo build --release -p subnet

    # Move the subnet binary to the plugin directory
    mv "$MOVEMENT_DIR/movement-subnet/vm/aptos-vm/target/release/subnet" "$PLUGINS_DIR/subnet"

    # Symlink the subnet binary with its subnet ID
    ln -sf "$PLUGINS_DIR/subnet" "$PLUGINS_DIR/$SUBNET_ID" 
    ln -sf "$PLUGINS_DIR/subnet" "$AVALANCHEGO_DIR/plugins/$SUBNET_ID" 

    # Build the movement binary
    cargo build --release -p movement

    # Move the movement binary to the appropriate directory
    mv "$MOVEMENT_DIR/movement-subnet/vm/aptos-vm/target/release/movement" "$BIN_DIR"
}

dev() {
  dev_deps
}

download(){

  log_info "Downloading released binaries for subnet and movement@$VERSION $FARCH-$OS."

  if [[ $LATEST = true ]]; then
    log_info "Downloading subnet from $RELEASES_URL/latest/download/subnet-$FARCH-$OS."
    curl -sSfL "$RELEASES_URL/latest/download/subnet-$FARCH-$OS" -o "$PLUGINS_DIR/subnet"
    log_info "Downloading movement from $RELEASES_URL/latest/download/movement-$FARCH-$OS."
    curl -sSfL "$RELEASES_URL/latest/download/movement-$FARCH-$OS" -o "$BIN_DIR/movement"
  else
    log_info "Downloading subnet from $RELEASES_URL/download/$VERSION/subnet-$FARCH-$OS."
    curl -sSfL "$RELEASES_URL/download/$VERSION/subnet-$FARCH-$OS" -o "$PLUGINS_DIR/subnet"
    log_info "Downloading movement from $RELEASES_URL/download/$VERSION/movement-$FARCH-$OS."
    curl -sSfL "$RELEASES_URL/download/$VERSION/movement-$FARCH-$OS" -o "$BIN_DIR/movement"
  fi


  # Symlink the subnet binary with its subnet ID
  ln -sf "$PLUGINS_DIR/subnet" "$PLUGINS_DIR/$SUBNET_ID" 
  ln -sf "$PLUGINS_DIR/subnet" "$AVALANCHEGO_DIR/plugins/$SUBNET_ID"

  chmod -R 755 $BIN_DIR

}

movementctl() {
  log_info "Installing movementctl."
  curl -sSfL "$MOVEMENTCTL_URL" -o "$BIN_DIR/movementctl"
  chmod +x "$BIN_DIR/movementctl"
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
  
  # parse the args
  parse "$@"

  # detect the OS and architecture
  detect

  # show the configuration
  show_config

  if [[ "$FARCH-$OS" != x86_64-linux && "$FARCH-$OS" != aarch64-linux ]]; then
    log_error "$FARCH-$OS is not yet supported. Please use the --build option."
    exit 1
  fi

  # setup the .movement directory
  setup

  # include avalanche for movementctl
  avalanche_setup

  # if we're building or using dev, we'll need to pull the repo
  if [[ ("$SOURCE" = true) ]]; then
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

# from Aptos setup
function install_build_essentials {
  PACKAGE_MANAGER=$1
  #Differently named packages for pkg-config
  if [[ "$PACKAGE_MANAGER" == "apt-get" ]]; then
    install_pkg build-essential "$PACKAGE_MANAGER"
  fi
  if [[ "$PACKAGE_MANAGER" == "pacman" ]]; then
    install_pkg base-devel "$PACKAGE_MANAGER"
  fi
  if [[ "$PACKAGE_MANAGER" == "apk" ]]; then
    install_pkg alpine-sdk "$PACKAGE_MANAGER"
    install_pkg coreutils "$PACKAGE_MANAGER"
  fi
  if [[ "$PACKAGE_MANAGER" == "yum" ]] || [[ "$PACKAGE_MANAGER" == "dnf" ]]; then
    install_pkg gcc "$PACKAGE_MANAGER"
    install_pkg gcc-c++ "$PACKAGE_MANAGER"
    install_pkg make "$PACKAGE_MANAGER"
  fi
  #if [[ "$PACKAGE_MANAGER" == "brew" ]]; then
  #  install_pkg pkgconfig "$PACKAGE_MANAGER"
  #fi
}

function install_protoc {
  INSTALL_PROTOC="true"
  echo "Installing protoc and plugins"

  if command -v "${INSTALL_DIR}protoc" &>/dev/null && [[ "$("${INSTALL_DIR}protoc" --version || true)" =~ .*${PROTOC_VERSION}.* ]]; then
     echo "protoc 3.${PROTOC_VERSION} already installed"
     return
  fi

  if [[ "$(uname)" == "Linux" ]]; then
    PROTOC_PKG="protoc-$PROTOC_VERSION-linux-x86_64"
  elif [[ "$(uname)" == "Darwin" ]]; then
    PROTOC_PKG="protoc-$PROTOC_VERSION-osx-universal_binary"
  else
    echo "protoc support not configured for this platform (uname=$(uname))"
    return
  fi

  TMPFILE=$(mktemp)
  rm "$TMPFILE"
  mkdir -p "$TMPFILE"/
  (
    cd "$TMPFILE" || exit
    curl -LOs "https://github.com/protocolbuffers/protobuf/releases/download/v$PROTOC_VERSION/$PROTOC_PKG.zip" --retry 3
    sudo unzip -o "$PROTOC_PKG.zip" -d /usr/local bin/protoc
    sudo unzip -o "$PROTOC_PKG.zip" -d /usr/local 'include/*'
    sudo chmod +x "/usr/local/bin/protoc"
  )
  rm -rf "$TMPFILE"

  # Install the cargo plugins
  if ! command -v protoc-gen-prost &> /dev/null; then
    cargo install protoc-gen-prost --locked
  fi
  if ! command -v protoc-gen-prost-serde &> /dev/null; then
    cargo install protoc-gen-prost-serde --locked
  fi
  if ! command -v protoc-gen-prost-crate &> /dev/null; then
    cargo install protoc-gen-prost-crate --locked
  fi
}

function install_pkg {
  package=$1
  PACKAGE_MANAGER=$2
  PRE_COMMAND=()
  if [ "$(whoami)" != 'root' ]; then
    PRE_COMMAND=(sudo)
  fi
  if command -v "$package" &>/dev/null; then
    echo "$package is already installed"
  else
    echo "Installing ${package}."
    if [[ "$PACKAGE_MANAGER" == "yum" ]]; then
      "${PRE_COMMAND[@]}" yum install "${package}" -y
    elif [[ "$PACKAGE_MANAGER" == "apt-get" ]]; then
      "${PRE_COMMAND[@]}" apt-get install "${package}" --no-install-recommends -y
      echo apt-get install result code: $?
    elif [[ "$PACKAGE_MANAGER" == "pacman" ]]; then
      "${PRE_COMMAND[@]}" pacman -Syu "$package" --noconfirm
    elif [[ "$PACKAGE_MANAGER" == "apk" ]]; then
      apk --update add --no-cache "${package}"
    elif [[ "$PACKAGE_MANAGER" == "dnf" ]]; then
      dnf install "$package"
    elif [[ "$PACKAGE_MANAGER" == "brew" ]]; then
      brew install "$package"
    fi
  fi
}

function install_pkg_config {
  PACKAGE_MANAGER=$1
  #Differently named packages for pkg-config
  if [[ "$PACKAGE_MANAGER" == "apt-get" ]] || [[ "$PACKAGE_MANAGER" == "dnf" ]]; then
    install_pkg pkg-config "$PACKAGE_MANAGER"
  fi
  if [[ "$PACKAGE_MANAGER" == "pacman" ]]; then
    install_pkg pkgconf "$PACKAGE_MANAGER"
  fi
  if [[ "$PACKAGE_MANAGER" == "brew" ]] || [[ "$PACKAGE_MANAGER" == "apk" ]] || [[ "$PACKAGE_MANAGER" == "yum" ]]; then
    install_pkg pkgconfig "$PACKAGE_MANAGER"
  fi
}

function install_shellcheck {
  if ! command -v shellcheck &> /dev/null; then
    if [[ $(uname -s) == "Darwin" ]]; then
      install_pkg shellcheck brew
    else
      install_pkg xz "$PACKAGE_MANAGER"
      MACHINE=$(uname -m);
      TMPFILE=$(mktemp)
      rm "$TMPFILE"
      mkdir -p "$TMPFILE"/
      curl -sL -o "$TMPFILE"/out.xz "https://github.com/koalaman/shellcheck/releases/download/v${SHELLCHECK_VERSION}/shellcheck-v${SHELLCHECK_VERSION}.$(uname -s | tr '[:upper:]' '[:lower:]').${MACHINE}.tar.xz"
      tar -xf "$TMPFILE"/out.xz -C "$TMPFILE"/
      cp "${TMPFILE}/shellcheck-v${SHELLCHECK_VERSION}/shellcheck" "${INSTALL_DIR}/shellcheck"
      rm -rf "$TMPFILE"
      chmod +x "${INSTALL_DIR}"/shellcheck
    fi
  fi
}

function install_openssl_dev {
  PACKAGE_MANAGER=$1
  #Differently named packages for openssl dev
  if [[ "$PACKAGE_MANAGER" == "apk" ]]; then
    install_pkg openssl-dev "$PACKAGE_MANAGER"
  fi
  if [[ "$PACKAGE_MANAGER" == "apt-get" ]]; then
    install_pkg libssl-dev "$PACKAGE_MANAGER"
  fi
  if [[ "$PACKAGE_MANAGER" == "yum" ]] || [[ "$PACKAGE_MANAGER" == "dnf" ]]; then
    install_pkg openssl-devel "$PACKAGE_MANAGER"
  fi
  if [[ "$PACKAGE_MANAGER" == "pacman" ]] || [[ "$PACKAGE_MANAGER" == "brew" ]]; then
    install_pkg openssl "$PACKAGE_MANAGER"
  fi
}

function install_lcov {
  PACKAGE_MANAGER=$1
  #Differently named packages for lcov with different sources.
  if [[ "$PACKAGE_MANAGER" == "apk" ]]; then
    apk --update add --no-cache  -X http://dl-cdn.alpinelinux.org/alpine/edge/testing lcov
  fi
  if [[ "$PACKAGE_MANAGER" == "apt-get" ]] || [[ "$PACKAGE_MANAGER" == "yum" ]] || [[ "$PACKAGE_MANAGER" == "dnf" ]] || [[ "$PACKAGE_MANAGER" == "brew" ]]; then
    install_pkg lcov "$PACKAGE_MANAGER"
  fi
  if [[ "$PACKAGE_MANAGER" == "pacman" ]]; then
    echo nope no lcov for you.
    echo You can try installing yourself with:
    echo install_pkg git "$PACKAGE_MANAGER"
    echo cd lcov;
    echo git clone https://aur.archlinux.org/lcov.git
    echo makepkg -si --noconfirm
  fi
}

function install_tidy {
  PACKAGE_MANAGER=$1
  #Differently named packages for tidy
  if [[ "$PACKAGE_MANAGER" == "apk" ]]; then
    apk --update add --no-cache  -X http://dl-cdn.alpinelinux.org/alpine/edge/testing tidyhtml
  else
    install_pkg tidy "$PACKAGE_MANAGER"
  fi
}


function install_xsltproc {
    if [[ "$PACKAGE_MANAGER" == "apt-get" ]]; then
      install_pkg xsltproc "$PACKAGE_MANAGER"
    else
      install_pkg libxslt "$PACKAGE_MANAGER"
    fi
}

function install_lld {
  # Right now, only install lld for linux
  if [[ "$(uname)" == "Linux" ]]; then
    install_pkg lld "$PACKAGE_MANAGER"
  fi
}

# this is needed for hdpi crate from aptos-ledger
function install_libudev-dev {
  # Need to install libudev-dev for linux
  if [[ "$(uname)" == "Linux" ]]; then
    install_pkg libudev-dev "$PACKAGE_MANAGER"
  fi
}

main "$@"
