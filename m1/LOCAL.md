## ANR
You may use the scripts in `./scripts`. 

Run `./scripts/anr.sh` to begin running anr in one process. Keep this process up.

Run `./scripts/start-subnet.sh` to start the subnet. This process should end gracefully and without errors from the RPC.

## `avalanchego`
You may use `avalanchego` to run the subnet against the Fuji network. You may follow the guides from avalanche to do this. However, we have been encountering a bootstrapping error. 

## In the container
- In the `mvlbs/m1` container, you may use `movementctl start local --foreground` to run a process similar to the ANR scripts mentioned above.
- In the `mvlbs/m1` container, you may use `movementctl start fuji --foreground` to run a process similar to the Fuji scripts mentioned above.
