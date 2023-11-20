# `canonical-move-natives`
This crate is used to create the move function tables for both the `AptosVm` and `SuiExecutor` by wrapping over the `CanonicalMoveResolver`.

In both cases, the natives table SHALL consist of
- Move stdlib natives (automatically compatible).
- Aptos natives (needs investigation).
- Sui natives (should be compatible via the resolver).