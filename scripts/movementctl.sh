#!/bin/bash

################################################################################
# Helper Script for Movement Control
#
# This script provides functions and commands for controlling the Movement
# environment. It includes functions to start and stop AvalancheGo, the
# avalanche-network-runner server, and the subnet-request-proxy Node.js server.
#
# Usage: movementctl [start/stop] [fuji/local/subnet-proxy]
#
# Author: Liam Monninger
# Version: 1.0
################################################################################

PID_DIR="$HOME/.movement/pid"

# Starts avalanchego with the specified network ID and subnet ID
function start_avalanchego() {
  network_id="$1"
  subnet_id="$2"
  avalanchego --network-id="$network_id" --track-subnets "$subnet_id" &
  echo $! >> "$PID_DIR/avalanchego.pid"
}

# Starts the avalanche-network-runner server
function start_avalanche_network_runner() {
  avalanche-network-runner server --log-level debug &
  echo $! >> "$PID_DIR/avalanche_network_runner.pid"
}

# Starts the subnet-request-proxy Node.js server
function start_subnet_proxy() {
  cd "$HOME/.movement/subnet-request-proxy"
  npm i
  node app.js &
  echo $! >> "$PID_DIR/subnet_proxy.pid"
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
      start_avalanchego "fuji" "qCP4kDnEWVorqyoUmcAtAmJybm8gXZzhHZ7pZibrJJEWECooU"
      ;;
    local)
      start_avalanche_network_runner
      ;;
    subnet-proxy)
      start_subnet_proxy
      ;;
    *)
      echo "Invalid start command. Usage: movementctl start [fuji/local/subnet-proxy]"
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
    echo "Invalid command. Usage: movementctl [start/stop]"
    exit 1
    ;;
esac