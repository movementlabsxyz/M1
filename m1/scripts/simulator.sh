# run nix develop then call this script 

init_env() {
    echo "Initializing Environment"
    ./scripts/build.debug.sh \
    && VM_PLUGIN_PATH=$(pwd)/target/debug/subnet
}

start_network() {
    echo "Starting network with 5 validators..."
    ./target/debug/simulator start
}

simulate_partition() {
    echo "Simulating network partition..."
    # Placeholder for command to partition the network
}

reconnect_nodes() {
    echo "Reconnecting partitioned nodes..."
    # Placeholder for command to reconnect nodes
}

observe_health() {
    echo "Observing consensus health..."
    # Placeholder for command to observe the network's consensus health
}

# Script execution flow
init_env

# Step 1: Start the network
start_network

# Wait for the network to stabilize
sleep 5

# Step 2: Simulate network partition
# simulate_partition

# Simulate some duration of network partition
# sleep 10

# Step 3: Reconnect the nodes
# reconnect_nodes

# Observe the network for a period to assess impact on consensus health
sleep 5
# observe_health
