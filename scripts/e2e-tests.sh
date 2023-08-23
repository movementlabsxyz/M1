#!/bin/bash -e
mkdir -p ~/.movement/plugins
cp ./m1/target/debug/subnet ~/.movement/plugins

# these are different from the environment variables typically used when running
# see in tests/e2e/lib.rs for more info
export AVALANCHEGO_PATH="${HOME}/.avalanchego/avalanchego"
export VM_PLUGIN_PATH="${HOME}/.movement/plugins"

cd ./m1/tests/e2e
cargo test