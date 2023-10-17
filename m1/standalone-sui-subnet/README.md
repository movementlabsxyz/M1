# Standalone Sui Subnet
The Standalone Sui Subnet is a sta

## Research Log
Ultimately, you need to bridge some kind of concept of working with `Vec<SenderSignedTransaction>` (as a block) with `execute_to_effects` or else rely on a custom block type.

- `TransactionOrchestrator::execute_transaction_block`: https://github.com/MystenLabs/sui/blob/85ed310e2771c5e0332b0900ee2394b86ad75600/crates/sui-core/src/transaction_orchestrator.rs#L155C37-L155C37
- `TransactionOrchestrator::execute_finalized_tx_locally_with_timeout`: https://github.com/MystenLabs/sui/blob/85ed310e2771c5e0332b0900ee2394b86ad75600/crates/sui-core/src/transaction_orchestrator.rs#L236C54-L236C54
    - Relies on a `VerifiedExecutableTransaction` which comes from the quorom of validators.
    - Under the hood, this wraps a `SignerSignedData` which itself wraps a `Vec<SenderSignedTransaction>`, thus forming a block: https://github.com/MystenLabs/sui/blob/main/crates/sui-types/src/transaction.rs#L1921

- Then, the validator state calls `fullnode_execute_certificate_with_effects`: https://github.com/MystenLabs/sui/blob/85ed310e2771c5e0332b0900ee2394b86ad75600/crates/sui-core/src/authority.rs#L811

- Relatively deep down in the executor logic, you have the execution loop: https://github.com/MystenLabs/sui/blob/85ed310e2771c5e0332b0900ee2394b86ad75600/sui-execution/v0/sui-adapter/src/execution_engine.rs#L375
    - This where most of what will be relevant to us if we abandon Sui blocks will actually take place. 
    - Here is the underlying `TransactionKind` enum: https://github.com/MystenLabs/sui/blob/main/crates/sui-types/src/transaction.rs#L227
    - A programmable transaction is a `Vec<CallArg>` and `Vec<Command>`: https://github.com/MystenLabs/sui/blob/85ed310e2771c5e0332b0900ee2394b86ad75600/crates/sui-types/src/transaction.rs#L555
        - Commands are quite special: https://github.com/MystenLabs/sui/blob/85ed310e2771c5e0332b0900ee2394b86ad75600/crates/sui-types/src/transaction.rs#L566
    - It gets executed like so: https://github.com/MystenLabs/sui/blob/85ed310e2771c5e0332b0900ee2394b86ad75600/sui-execution/v0/sui-adapter/src/programmable_transactions/execution.rs#L69

