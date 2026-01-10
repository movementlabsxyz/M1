#!/usr/bin/env bash
#!/usr/bin/env bash
set -xue

if ! [[ "$0" =~ scripts/build.debug.sh ]]; then
  echo "must be run from m1"
  exit 255
fi

PROTOC_VERSION=$(protoc --version | cut -f2 -d' ')
MIN_VERSION="3.15"

if ! printf "%s\n%s" "$PROTOC_VERSION" "$MIN_VERSION" | sort -V | tail -n 1 | grep -q "$PROTOC_VERSION"; then
  echo "protoc must be installed and the version must be greater than 3.15"
  exit 255
fi

# "--bin" can be specified multiple times for each directory in "bin/*" or workspaces
cargo build -p subnet --bin subnet

./target/debug/subnet --help

./target/debug/subnet genesis "hello world"
./target/debug/subnet vm-id subnet
