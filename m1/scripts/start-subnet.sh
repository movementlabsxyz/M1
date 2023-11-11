#!/bin/bash -e
export AVALANCHEGO_EXEC_PATH=$(which avalanchego)
export AVALANCHEGO_PLUGIN_PATH=$(pwd)/plugins

echo hello > /tmp/subnet.genesis.json

avalanche-network-runner control start \
--log-level debug \
--endpoint="0.0.0.0:8080" \
--avalanchego-path="${AVALANCHEGO_EXEC_PATH}"
# --blockchain-specs '[{"vm_name":"subnet","genesis":"/tmp/subnet.genesis.json","blockchain_alias":"movement"}]' \

# avalanche-network-runner control create-blockchains '[{"vm_name":"movement", "subnet_id": "srEXiWaHZNDcVtfHLb38cFiwKVLJ8xnhDF5qpWbYdowxEiyid"}]' \
# --log-level debug \
# --endpoint="0.0.0.0:8080"

curl -X POST -k http://localhost:8081/v1/control/status