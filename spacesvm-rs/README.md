[<img alt="crates.io" src="https://img.shields.io/crates/v/spacesvm.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/spacesvm)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-spacesvm-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/spacesvm)
![Github Actions](https://github.com/ava-labs/spacesvm-rs/actions/workflows/test-and-release.yml/badge.svg)

# `spacesvm-rs`

### status: experimental

`spacesvm-rs` is a [EIP-712](https://eips.ethereum.org/EIPS/eip-712) compatible Rust port of the [SpacesVM](https://github.com/ava-labs/spacesvm) virtual machine using the Avalanche [Rust SDK](https://github.com/ava-labs/avalanche-types-rs).

_Although the goal is to achieve feature parity with its Golang counterpart, it is not yet compatible with the existing spaces network._

## Core features
### Authentication
Any mutations to the key space require the signature of the owner.
### EIP-712 Compatibility
Typed structured data hashing and signing.

## Current Functionality
### Claim
Interacting with the SpacesVM starts with a `ClaimTx`. This reserves your own
"space" and associates your address with it (so that only you can make changes
to it and/or the keys in it).

### Set/Delete
Once you have a space, you can then use `SetTx` and `DeleteTx` actions to
add/modify/delete keys in it.

### Resolve
When you want to view data stored in SpacesVM, you call `Resolve` on the value
path: `<space>/<key>`. If you stored a file at a particular path, use this
command to retrieve it: `spaces-cli resolve-file <path> <destination
filepath>`.

#### Getting Started
The easiest way to test and interact with spacesvm is with the included e2e scripts.

> Note: spacesvm requires protocol buffers to be installed locally to build successfully. [Download](https://github.com/protocolbuffers/protobuf#protocol-compiler-installation) the `protoc` compiler for your system and ensure it's working correctly before continuing.

```bash
# build the spacesvm plugin, run e2e tests, and keep the network running
./scripts/build.release.sh \
&& VM_PLUGIN_PATH=$(pwd)/target/release/spacesvm \
./scripts/tests.e2e.sh

# or, specify the custom avalanchego binary
./scripts/build.release.sh \
&& VM_PLUGIN_PATH=$(pwd)/target/release/spacesvm \
./scripts/tests.e2e.sh ~/go/src/github.com/ava-labs/avalanchego/build/avalanchego

# (optional) set NETWORK_RUNNER_ENABLE_SHUTDOWN=1 in "tests.e2e.sh"
# to shut down the network afterwards
```
#### Compile from source.

```bash
cargo build \
  --release \
  --bin spacesvm

./target/release/spaces-vm
```

```bash
cargo build \
  --release \
  --bin spaces-cli

./target/release/spaces-cli

```

To test `spacesvm` APIs, try the following commands with the CLI or shell commands:
```bash
SpacesVM CLI for issuing RPC commands

Usage: spacescli [OPTIONS] --endpoint <ENDPOINT> <COMMAND>

Commands:
  claim  
  set     
  delete  
  get     
  help    Print this message or the help of the given subcommand(s)

Options:
      --endpoint <ENDPOINT>                  Endpoint for RPC calls
      --private-key-file <PRIVATE_KEY_FILE>  Private key file [default: .spacesvm-cli-pk]
  -h, --help                                 Print help information
  -V, --version                              Print version information
```

## Client
#### Public Endpoints (`/public`)

#### spacesvm.ping
```bash
# "2FdEyx8mgicqvQaGN3HGkDwo7NbhKAY6pgTXUB1UkHW4meySUv" is the blockchain Id
curl -X POST --data '{
    "jsonrpc": "2.0",
    "id"     : 1,
    "method" : "spacesvm.ping",
    "params" : []
}' -H 'content-type:application/json;' 127.0.0.1:9650/ext/bc/2FdEyx8mgicqvQaGN3HGkDwo7NbhKAY6pgTXUB1UkHW4meySUv/public

# {"jsonrpc":"2.0","result":{"success":true},"id":1}
```
#### spacesvm.issueTx
```bash
# "2FdEyx8mgicqvQaGN3HGkDwo7NbhKAY6pgTXUB1UkHW4meySUv" is the blockchain Id
curl -X POST --data '{
  "jsonrpc": "2.0",
  "method": "spacesvm.issueTx",
  "params":{
    "typedData":<EIP-712 compliant typed data>,
    "signature":<hex-encoded sig>
  },
  "id": 1
}'
# IssueTxResponse {"tx_id":<ID>}
```
#### spacesvm.resolve
```bash
curl -X POST --data '{
  "jsonrpc": "2.0",
  "method": "spacesvm.resolve",
  "params":{
    "path":<string | ex:jim/twitter>
  },
  "id": 1
}'
# ResolveResponse {"exists":<bool>, "value":<base64 encoded>, "valueMeta":<chain.ValueMeta>}
```

## License
`spacesvm-rs` is under the BSD 3.0 license. See the [LICENSE](LICENSE) file for details.
