# Movement VM

The Move programming language poses numerous benefits to builders including direct
interaction with digital assets through custom resource types, flexibility with transaction script
declaration, on-chain verification, and bytecode safety privileges. However, the benefits of Move
are currently limited to Aptos and Sui, creating a divide for builders who also want to reap the
benefits of brilliant platforms like Avalanche. The Movement MoveVM is designed for the
modern-day Avalanche subnet, allowing users to seamlessly interact with and build on the Move
language underneath the established Avalanche umbrella. Through Avalanche warp messaging,
the Movement subnet will fundamentally be the first L1 built on Avalanche, creating a moat
around the Move language by launching multiple blockchains to adapt to Aptos MoveVM as
well as Sui MoveVM. Through multi-chain dynamic load balancing to address throughput
bottlenecks and parallel processing on top of the Avalanche consensus layer, Movement will be
able to hit 160,000+ theoretical TPS as the project scales to provide much needed performance to
protocols. In addition, Move bytecode verifiers and interpreters provide native solvency for the
reentrancy attacks and security woes that have plagued Solidity developers for years, resulting in
$3 billion lost last year. Movement will be the hub of smart contract development providing
performance to Aptos developers and security for Avalanche protocols.

## Status
`Movement VM` is considered **ALPHA** software and is not safe to use in
production. The framework is under active development and may change
significantly over the coming months as its modules are optimized and
audited.

We created the `Movement VM` with Avalanche's [`hypersdk`](https://github.com/ava-labs/hypersdk)
to test out our design decisions and abstractions. 

## Terminology
* `aptos-vm`: our first implementation of move execution layer, based on [`Aptos Move`](https://github.com/aptos-labs/aptos-core)
* `smart contract`: refers to smart contracts using [`Move`](https://github.com/aptos-labs/move) language 
* `transactions`: refers to aptos transaction built by [`aptos-client`](https://github.com/aptos-labs/aptos-core)

**Note**

The `Movement VM` is still under very early stage, there will be some testing purpose codes
in the repo for development. Also the compiling & testinng actions may fail because of frequently
commit and integration.


## Features
### Movement CLI
To allow builders to develop on the subnet locally, we offer a Movement command line interface
where users can launch smart contracts, debug, and operate nodes. Our website will feature an
in-depth documentation guide to show builders how to utilize the CLI to make requests to the
Avalanche subnet. Some critical functions of the CLI include `run` (executes a Move function),
`download` (downloads a package and stores it in a directory), `init` (creates a new Move package
at the given location), `prove` (validates Move package), `publish` (pushes Move modules in a
package to the Movement MoveVM subnet), and `test` (runs Move unit tests for a particular
package). The CLI will primarily be used for running local testnets, monitoring and executing
transactions between accounts, publishing Move packages, and debugging modules.

### Move Smart Contracts
For the enthusiastic Move developer looking to bring their talents to the Avalanche network, we
provide seamless integration with our MoveVM, bridging the gap between `Aptos` developers and
`Avalanche` builders. The Movement subnet will utilize the MoveVM for all operations: `account
management`, `Move module publishing`, and `fund transfer`. After a user writes the Move contract
designating the functionality of the module, the user then compiles the module through the CLI
preparing it for deployment. Finally, the user publishes the module to the designated account,s
storing it on the Movement `Move subnet` running on the `Avalanche` network. Developers can
then write smart contracts interacting with these published modules or use the CLI to see the
transaction and access the data. Move modules have built-in entry functions which are access
points that allow other transactions to make requests to the modules. The Movement team will
have detailed documentation on the website in the coming months outlining how to learn the
Move language, implement smart contracts, and connect to the Avalanche network.

### Movement SDK
For a builder looking to reap the benefits of the MoveVM without the technical expertise to write
Move smart contracts, the Movement TypeScript SDK is key. Bridging the gap between Web2
builders and blockchain technologies, Movement will allow developers to seamlessly interact
with the Move subnet without writing a single line of Move code. Furthermore, the SDK is not
just limited to `Typescript`. In the future, we will launch integrations like `Python` and `Rust` to bring
in developers from different realms into the ecosystem.

## Movement VM Transactions
### Transaction
```rust
pub struct RawTransaction {
    /// Sender's address.
    sender: AccountAddress,

    /// Sequence number of this transaction. This must match the sequence number
    /// stored in the sender's account at the time the transaction executes.
    sequence_number: u64,

    /// The transaction payload, e.g., a script to execute.
    payload: TransactionPayload,

    /// Maximal total gas to spend for this transaction.
    max_gas_amount: u64,

    /// Price to be paid per gas unit.
    gas_unit_price: u64,

    /// Expiration timestamp for this transaction, represented
    /// as seconds from the Unix Epoch. If the current blockchain timestamp
    /// is greater than or equal to this time, then the transaction has
    /// expired and will be discarded. This can be set to a large value far
    /// in the future to indicate that a transaction does not expire.
    expiration_timestamp_secs: u64,

    /// Chain ID of the Movement VM network this transaction is intended for.
    chain_id: ChainId,
}
```

### TransactionPayload
```rust
pub enum TransactionPayload {
    /// A transaction that executes code.
    Script(Script),
    /// A transaction that publishes multiple modules at the same time.
    ModuleBundle(ModuleBundle),
    /// A transaction that executes an existing entry function published on-chain.
    EntryFunction(EntryFunction),
}
```

`Movement VM` will have three different transactions supported, this might change due 
to the progress of the development.

## Movement VM Blocks
### BlockInfo
```rust
pub struct BlockInfo {
    /// The epoch to which the block belongs.
    epoch: u64,
    /// The consensus protocol is executed in rounds, which monotonically increase per epoch.
    round: Round,
    /// The identifier (hash) of the block.
    id: HashValue,
    /// The accumulator root hash after executing this block.
    executed_state_id: HashValue,
    /// The version of the latest transaction after executing this block.
    version: Version,
    /// The timestamp this block was proposed by a proposer.
    timestamp_usecs: u64,
    /// An optional field containing the next epoch info
    next_epoch_state: Option<EpochState>,
}
```

### BlockMetadata
```rust
/// Struct that will be persisted on chain to store the information of the current block.
///
/// The flow will look like following:
/// 1. The executor will pass this struct to VM at the end of a block proposal.
/// 2. The VM will use this struct to create a special system transaction that will emit an event
///    represents the information of the current block. This transaction can't
///    be emitted by regular users and is generated by each of the validators on the fly. Such
///    transaction will be executed before all of the user-submitted transactions in the blocks.
/// 3. Once that special resource is modified, the other user transactions can read the consensus
///    info by calling into the read method of that resource, which would thus give users the
///    information such as the current leader.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockMetadata {
    id: HashValue,
    epoch: u64,
    round: u64,
    proposer: AccountAddress,
    #[serde(with = "serde_bytes")]
    previous_block_votes_bitvec: Vec<u8>,
    failed_proposer_indices: Vec<u32>,
    timestamp_usecs: u64,
}
```

## Avalanche HyperVM SDK
### Controller
```golang
type Controller interface {
	Initialize(
		inner *VM,
		snowCtx *snow.Context,
		gatherer metrics.MultiGatherer,
		genesisBytes []byte,
		upgradeBytes []byte,
		configBytes []byte,
	) (
		config Config,
		genesis Genesis,
		builder builder.Builder,
		gossiper gossiper.Gossiper,
		blockDB KVDatabase,
		stateDB database.Database,
		handler Handlers,
		actionRegistry chain.ActionRegistry,
		authRegistry chain.AuthRegistry,
		err error,
	)

	Rules(t int64) chain.Rules

	Accepted(ctx context.Context, blk *chain.StatelessBlock) error
	Rejected(ctx context.Context, blk *chain.StatelessBlock) error
}
```

The controller interface is the key component that integrates Avalanche Network 
and Movement VM. 

You can view what this looks like in the indexvm by clicking this [`link`](https://github.com/ava-labs/indexvm/blob/main/controller/controller.go).

### Genesis
```golang
type Genesis interface {
	GetHRP() string
	Load(context.Context, atrace.Tracer, chain.Database) error
}
```

Genesis is typically the list of initial balances that accounts have at the start 
of the network and a list of default configurations that exist at the start of the 
network (fee price, enabled txs, etc.). The serialized genesis of any hyperchain is 
persisted on the P-Chain for anyone to see when the network is created.

You can view what this looks like in the indexvm by clicking this [`link`](https://github.com/ava-labs/indexvm/blob/main/genesis/genesis.go).

### Action
```golang
type Action interface {
	MaxUnits(Rules) uint64
	ValidRange(Rules) (start int64, end int64)

	StateKeys(Auth) [][]byte
	Execute(ctx context.Context, r Rules, db Database, timestamp int64, auth Auth, txID ids.ID) (result *Result, err error)

	Marshal(p *codec.Packer)
}
```

Actions are the heart of Movement VM. They define how users interact with the blockchain 
runtime. 

### Auth
```golang
type Auth interface {
	MaxUnits(Rules) uint64
	ValidRange(Rules) (start int64, end int64)

	StateKeys() [][]byte
	AsyncVerify(msg []byte) error
	Verify(ctx context.Context, r Rules, db Database, action Action) (units uint64, err error)

	Payer() []byte
	CanDeduct(ctx context.Context, db Database, amount uint64) error
	Deduct(ctx context.Context, db Database, amount uint64) error
	Refund(ctx context.Context, db Database, amount uint64) error

	Marshal(p *codec.Packer)
}
```

`Movement VM` will need to implement the `Auth` interface of `hypersdk` to bridge Aptos's ED25519
signature verification.

## Running the `Movement VM`

1. set up avalanch subnet env, you can refer to 
[avalanche-network-runner](https://github.com/ava-labs/avalanche-network-runner#network-runner-rpc-server-timestampvm-example) and [timestampvm-rs](https://github.com/ava-labs/timestampvm-rs)

2. clone this repo and build subnet binary
```
cd movement-subnet/vm/aptos-vm/subnet
cargo build 
```
3 start network and install subnet. move this subnet binary to /home/ubuntu/aavx-ruuner/plugins and create file name genesis.json

```
# start runner
avalanche-network-runner server --log-level debug --port=":8080" --grpc-gateway-port=":8081"

# install subnet
curl -X POST -k http://localhost:8081/v1/control/start -d '{"execPath":"'${AVALANCHEGO_EXEC_PATH}'","numNodes":5,"logLevel":"INFO","pluginDir":"/home/ubuntu/aavx-ruuner/plugins","blockchainSpecs":[{"vmName":"subnet","genesis":"/home/ubuntu/aavx-ruuner/genesis.json","blockchain_alias":"timestamp"}]}'

```

4 create account and faucet 
```
# create account
curl -X POST --data '{
  "jsonrpc": "2.0",
  "id"     : 1,
  "method" : "aptosvm.createAccount",
  "params" : [{"account":"0x61c66dea4265716facb3484ac5e2f366cf6c6e52e58120626f3434babb375eb1"}]
}' -H 'content-type:application/json;' 127.0.0.1:9650/ext/bc/241UUZZ1gqpynKomM7DPJP4sm91XT8zwi3ttexHMFs8DznzVDs/rpc


{"jsonrpc":"2.0","result":{"data":"99ddf6ae010fc534e848b5fdf9d3cb5d49407de99db36415c022e6e110e4b121"},"id":1}


# faucet aptos token
curl -X POST --data '{
  "jsonrpc": "2.0",
  "id"     : 1,
  "method" : "aptosvm.faucet",
  "params" : [{"account":"7e95b0c90bf89fab82a8f98fbf8062f7bed3ca721aaa2d91dbb712a5b7e8ea6a"}]
}' -H 'content-type:application/json;' 127.0.0.1:9650/ext/bc/241UUZZ1gqpynKomM7DPJP4sm91XT8zwi3ttexHMFs8DznzVDs/rpc

{"jsonrpc":"2.0","result":{"data":"3b1d120f5cb3c2ab25062541193b2e72dbbb4f2dafca0020c9375a68c33a918d"},"id":1}

# check balance
curl -X POST --data '{
  "jsonrpc": "2.0",
  "id"     : 1,
  "method" : "aptosvm.getBalance",
  "params" : [{"account":"7e95b0c90bf89fab82a8f98fbf8062f7bed3ca721aaa2d91dbb712a5b7e8ea6a"}]
}' -H 'content-type:application/json;' 127.0.0.1:9650/ext/bc/241UUZZ1gqpynKomM7DPJP4sm91XT8zwi3ttexHMFs8DznzVDs/rpc

{"jsonrpc":"2.0","result":{"data":100000000000},"id":1}

```
