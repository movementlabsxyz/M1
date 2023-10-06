#!/bin/bash

################################################################################
# Helper Script for Movement Control
#
# This script provides functions and commands for controlling the Movement
# environment. It includes functions to start and stop AvalancheGo, the
# avalanche-network-runner server, and the subnet-request-proxy Node.js server.
#
# Usage: movementctl [start/stop] [fuji/local/subnet-proxy] [--foreground]
#
# Author: Liam Monninger
# Version: 1.0
################################################################################

VM_NAME="movement"
SUBNET_ID="2gLyawqthdiyrJktJmdnDAb1XVc6xwJXU6iJKu3Uwj21F2mXAK"
PID_DIR="$HOME/.movement/pid"
mkdir -p "$PID_DIR"

RUN_IN_FOREGROUND="false"
if [[ "${@: -1}" == "--foreground" ]]; then
    RUN_IN_FOREGROUND="true"
fi

# Starts avalanchego with the specified network ID and subnet ID
function start_avalanchego() {
  network_id="$1"
  subnet_id="$2"
  
  if [[ $RUN_IN_FOREGROUND == "true" ]]; then
    avalanchego --http-host=0.0.0.0 --network-id="$network_id" --track-subnets "$subnet_id"
  else
    avalanchego --http-host=0.0.0.0 --network-id="$network_id" --track-subnets "$subnet_id" &
    echo $! >> "$PID_DIR/avalanchego.pid"
  fi
}

# Starts the avalanche-network-runner server
function start_avalanche_network_runner() {
  if [[ $RUN_IN_FOREGROUND == "true" ]]; then
    # Start a new tmux session in detached mode and run the first command
    tmux new-session -d -s lnet 'avalanche-network-runner server --log-level debug'

    # Introduce a delay (e.g., 5 seconds)
    sleep 5

    # Run the second command in the current terminal
    avalanche-network-runner control create-blockchains '[{"vm_name":"'$VM_NAME'", "subnet_id": "'$SUBNET_ID'"}]'

    tmux attach-session -t lnet

  else
    avalanche-network-runner server --log-level debug &
    echo $! >> "$PID_DIR/avalanche_network_runner.pid"
    # Introduce a delay (e.g., 5 seconds)
    sleep 5
    # Run the second command in the current terminal
    avalanche-network-runner control create-blockchains '[{"vm_name":"'$VM_NAME'", "subnet_id": "'$SUBNET_ID'"}]'
  fi
}

# Starts the subnet-request-proxy Node.js server
function start_subnet_proxy() {
  cd "$HOME/.movement/subnet-request-proxy"
  npm i
  
  if [[ $RUN_IN_FOREGROUND == "true" ]]; then
    node app.js
  else
    node app.js &
    echo $! >> "$PID_DIR/subnet_proxy.pid"
  fi
}

# Stops a process based on the provided PID file
function stop_process() {
  local process_name="$1"
  local pid_file="$PID_DIR/$process_name.pid"

  if [ -f "$pid_file" ]; then
    while read -r pid; do
      kill "$pid" || true
    done < "$pid_file"
    rm "$pid_file"
  else
    echo "No $process_name process found."
  fi
}

# Handle the start command
function start() {
  case $1 in
    fuji)
      start_avalanchego "fuji" "2gLyawqthdiyrJktJmdnDAb1XVc6xwJXU6iJKu3Uwj21F2mXAK"
      ;;
    local)
      start_avalanche_network_runner
      ;;
    subnet-proxy)
      start_subnet_proxy
      ;;
    *)
      echo "Invalid start command. Usage: movementctl start [fuji/local/subnet-proxy] [--foreground]"
      exit 1
      ;;
  esac
}

# Handle the stop command
function stop() {
  case $1 in
    fuji)
      stop_process "avalanchego"
      ;;
    local)
      stop_process "avalanche_network_runner"
      ;;
    subnet-proxy)
      stop_process "subnet_proxy"
      ;;
    *)
      echo "Invalid stop command. Usage: movementctl stop [fuji/local/subnet-proxy]"
      exit 1
      ;;
  esac
}

# Handle the provided command
case $1 in
  start)
    start "${@:2}"
    ;;
  stop)
    stop "${@:2}"
    ;;
  *)
    echo "Invalid command. Usage: movementctl [start/stop] [fuji/local/subnet-proxy] [--foreground]"
    exit 1
    ;;
esac
