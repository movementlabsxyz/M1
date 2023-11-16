# `canonical-rpc`
The canonical RPC is the complete RPC implementation for the Canonical Move VM. It contains:
- `canonical/` routes which are routes supporting the combined execution of Aptos and Sui.
- `aptos/` routes which are compatible with the upstream Aptos RPC.
- `sui/` routes which are compatible with the upstream Sui RPC.
- `movement/` routes which are derived as a consequence of implementing within the `movement-sdk` framework.