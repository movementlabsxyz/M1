use movement_sdk::{ExecutionLayer};
use canonical_move_resolver::CanonicalMoveResolver;
use canonical_types::{Transaction, Block};

// todo: this will likely be a wrapper around some kind of 
// todo: type-state pattern structs that handle the bootstrapping
// todo: or else we will move that to a higher order
// todo: good thing to consider for the movement_sdk
pub struct CanonicalBlockExecutionLayer<'state> {
    move_resolver : CanonicalMoveResolver<'state>;
}

impl <'state> CanonicalBlockExecutionLayer<'state> {

    fn get_aptos_vm(){
        unimplemented!();
    }

    fn get_sui_executor(){
        unimplemented!();
    }

    /// Filters the block to just Aptos transactions.
    fn get_aptos_block(block : Block){
        unimplemented!();
    }

    /// Filters the block to just Sui transactions.
    /// Sui blocks are a fundamentally new construct.
    /// Sui does not have blocks in its original form.
    /// Concurrency considerations have not been made.
    fn get_sui_block(block : Block){
        unimplemented!();
    }

    /// Executes a canonical block.
    /// Canonical blocks in V1 are split into their respective transaction blocks.
    /// This is done to accommodate a transaction flow that is compatible with their respective RPCs.
    /// While Aptos has an original notion of blocks, Sui does not.
    /// Aptos transactions will thus be executed concurrently, insofar as Block-STM allows.
    /// Sui transactions will for now be executed sequentially.
    /// The order of Sui and Aptos execution must be deterministic.
    /// For now, we propose Sui transactions are executed first, then Aptos.
    /// We can opt for a more balance strategy at a later date.
    fn execute(block : Block){

        // 1. Execute the Sui block.
        let sui_block = self.get_sui_block(block);
        let sui_executor = self.get_sui_executor();

        // all sui transactions will be sequential for now
        for transaction in sui_block {
            
            sui_executor.execute_transaction_to_effects(
                transaction,
                ....
            );

        }

        // 2. Execute the Aptos block
        let aptos_block = self.get_aptos_block(block);
        let aptos_executor = self.get_aptos_executor();
        aptos_executor.execute_block(aptos_block);


    }

}