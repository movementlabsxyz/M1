# Canonical Execution Layer and Supporting Layers
<!-- Provide a 1-3 sentence description of the project. For example: "A web application designed to streamline the workflow of remote teams by integrating task management, communication, and reporting tools." -->
© 2023, Movement Labs. All rights reserved.

## Summary
Provide a Canonical Execution layer that supports behavior analogous to popular Move networks and idioms including Sui and the, currently, internal Movement VM. In addition to the execution layer, provide supporting logic which enables inter-execution-layer compatibility and analogous behavior to the execution layer upstream.  

## Motivation
To create an execution layer that is (a) capable of supporting popular Move idioms, (b) interoperating between different idioms without the need for messaging or bridging, and (c) portable to other consensus and settlement layers via the `movement-sdk`.


## Requirements
At the time of the writing, Canonical Execution Layer will support the following "VMs":
- Sui
- Movement

The support for Sui SHALL include an analogous RPC to the Sui upstream.

The support for Movement SHALL include an analogous RPC to the Movement upstream.

The Sui and Movement execution paths SHALL be interoperable.

The initial deployment of Canonical Execution Layer SHALL be as an Avalanche subnet. That is, consensus and settlement SHALL be handled by the Avalanche consensus engine.

## User Journeys

### Canonical Subnet
- Developer can Submit a Sui Transaction to the network for execution and expect that accepted Sui transactions will be executed in a manner analogous to the Sui upstream.
- Developer can Submit a Movement Transaction to the network for execution and expect that accepted Movement transactions are executed in a manner analogous to the Movement upstream.
- Developer can make requests to Sui routes within the Canonical Subnet RPC and received responses analogous to the Sui upstream.
- Developer can make requests to Movement routes within the Canonical Subnet RPC and received responses analogous to the Movement upstream.
- Developer can operate on states altered by Movement transactions via Sui transactions and vice versa.

## Appendices

### [A1] Stage 1: Standalone Sui Subnet
#### Summary
To better understand the intricacies of Sui while delivering a new network, this project SHALL begin with the construction of a Sui Avalanche subnet. This subnet shall feature the Sui Block Execution Layer which will ideally later be used to compose the Canonical Execution Layer.

The requirements for creating analogous structures to the Sui upstream stipulated for the canonical remain firmly in place. 

#### Motivation
- Better understand Sui execution. 
- Provide Sui-compatible network.
- Be able to include a Sui execution layer within the `movement-sdk`.
- With the successful extraction of Sui into a Sui Block Executor and other analogous structure, we may be able offer Mysten Labs an implementation of Sui as L2--a goal reported to us via the BD team. 

### [A2] Interoperation via Common Storage
One way to achieve interoperation between different execution layers is to have them share a common move resolver or storage layer. This would allow for the execution layers to operate on the same data. 

**N1**: Synchronization between the execution layers would be required to ensure that the data is consistent.
**N2**: This proposal is conceptually similar to the "transaction multiplexing" first suggested in [MIP-3](https://github.com/movemntdev/MIPs/blob/main/MIPs/mip-3.md)

![Common Storage](./rsc/canonical_shared_resolver.png)


### [A3] Sui Block Execution Layer
In order to achieve Sui-like execution in a block-based consensus network, it has been recommended a point of contact at Mysten Labs that we utilize the checkpoint execution path in the current Sui source. This appears as below:
![Sui Block Executor](./rsc/sui_block_executor.png)

### [A4] Implementation of Canonical Subnet
#### Summary
The first deployment of the Canonical Execution Layer will be as an Avalanche subnet. The subnet Execution Layer will be composed of the Sui Block Executor and the Movement Block Executor. The subnet will be configured to use the Avalanche consensus engine for consensus and settlement.

### [A5] Stage 3: Integration of Sui Block Executor and Canonical Execution Layer with Movement-SDK
WIP