#!/bin/bash -e
mkdir -p ./plugins

# We will inherit the avalanchego binary from the environment.
# But, we will set the plugin path to the plugin dir in this directory.
export AVALANCHEGO_EXEC_PATH=$(which avalanchego)
export AVALANCHEGO_PLUGIN_PATH=$(pwd)/plugins

# Build the subnet
cargo build -p subnet

# export SUBNET_ID=$(subnet-cli create VMID ./target/debug/subnet)
export SUBNET_ID=srEXiWaHZNDcVtfHLb38cFiwKVLJ8xnhDF5qpWbYdowxEiyid

echo $SUBNET_ID
# Copy the subnet binary to the plugin dir
cp ./target/debug/subnet ${AVALANCHEGO_PLUGIN_PATH}/${SUBNET_ID}
# cp ./target/debug/subnet ${AVALANCHEGO_PLUGIN_PATH}/subnet

# Start the avalanche-network-runner
avalanche-network-runner server

