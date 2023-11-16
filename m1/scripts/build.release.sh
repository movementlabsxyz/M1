#!/usr/bin/env bash
set -xue

if ! [[ "$0" =~ scripts/build.release.sh ]]; then
  echo "must be run from repository root"
  exit 255
fi

PROTOC_VERSION=$(protoc --version | cut -f2 -d' ')
if [[ "${PROTOC_VERSION}" == "" ]] || [[ "${PROTOC_VERSION}" < 3.15.0 ]]; then
  echo "protoc must be installed and the version must be greater than 3.15.0"
  exit 255
fi

# "--bin" can be specified multiple times for each directory in "bin/*" or workspaces
cargo build -p subnet \
--release \
--bin subnet

./target/release/subnet --help

./target/release/subnet genesis "hello world"
./target/release/subnet vm-id timestampvm
