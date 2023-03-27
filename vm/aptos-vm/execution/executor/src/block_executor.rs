// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    components::{block_tree::BlockTree, chunk_output::ChunkOutput},
    logging::{LogEntry, LogSchema},
    metrics::{
        APTOS_EXECUTOR_COMMIT_BLOCKS_SECONDS, APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS,
        APTOS_EXECUTOR_OTHER_TIMERS_SECONDS, APTOS_EXECUTOR_SAVE_TRANSACTIONS_SECONDS,
        APTOS_EXECUTOR_TRANSACTIONS_SAVED, APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS,
    },
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_executor_types::{BlockExecutorTrait, Error, StateComputeResult};
use aptos_infallible::RwLock;
use aptos_logger::prelude::*;
use aptos_scratchpad::SparseMerkleTree;
use aptos_state_view::StateViewId;
use aptos_storage_interface::{
    async_proof_fetcher::AsyncProofFetcher, cached_state_view::CachedStateView, DbReaderWriter,
};
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures, state_store::state_value::StateValue,
    transaction::Transaction,
};
use aptos_vm::AptosVM;
use fail::fail_point;
use std::{marker::PhantomData, sync::Arc};
use move_core_types::vm_status::{DiscardedVMStatus, StatusCode};
use aptos_types::transaction::{ExecutionStatus, TransactionStatus};

pub trait TransactionBlockExecutor<T>: Send + Sync {
    fn execute_transaction_block(
        transactions: Vec<T>,
        state_view: CachedStateView,
    ) -> Result<ChunkOutput>;
}

impl TransactionBlockExecutor<Transaction> for AptosVM {
    fn execute_transaction_block(
        transactions: Vec<Transaction>,
        state_view: CachedStateView,
    ) -> Result<ChunkOutput> {
        ChunkOutput::by_transaction_execution::<AptosVM>(transactions, state_view)
    }
}

pub struct BlockExecutor<V, T> {
    pub db: DbReaderWriter,
    inner: RwLock<Option<BlockExecutorInner<V, T>>>,
}

impl<V, T> BlockExecutor<V, T>
    where
        V: TransactionBlockExecutor<T>,
        T: Send + Sync,
{
    pub fn new(db: DbReaderWriter) -> Self {
        Self {
            db,
            inner: RwLock::new(None),
        }
    }

    pub fn root_smt(&self) -> SparseMerkleTree<StateValue> {
        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .root_smt()
    }

    fn maybe_initialize(&self) -> Result<()> {
        if self.inner.read().is_none() {
            self.reset()?;
        }
        Ok(())
    }
}

impl<V, T> BlockExecutorTrait<T> for BlockExecutor<V, T>
    where
        V: TransactionBlockExecutor<T>,
        T: Send + Sync,
{
    fn committed_block_id(&self) -> HashValue {
        self.maybe_initialize().expect("Failed to initialize.");
        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .committed_block_id()
    }

    fn reset(&self) -> Result<()> {
        *self.inner.write() = Some(BlockExecutorInner::new(self.db.clone())?);
        Ok(())
    }

    fn execute_block(
        &self,
        block: (HashValue, Vec<T>),
        parent_block_id: HashValue,
    ) -> Result<StateComputeResult, Error> {
        self.maybe_initialize()?;
        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .execute_block(block, parent_block_id)
    }

    fn commit_blocks_ext(
        &self,
        block_ids: Vec<HashValue>,
        ledger_info_with_sigs: LedgerInfoWithSignatures,
        save_state_snapshots: bool,
    ) -> Result<(), Error> {
        self.inner
            .read()
            .as_ref()
            .expect("BlockExecutor is not reset")
            .commit_blocks_ext(block_ids, ledger_info_with_sigs, save_state_snapshots)
    }

    fn finish(&self) {
        *self.inner.write() = None;
    }
}

struct BlockExecutorInner<V, T> {
    db: DbReaderWriter,
    block_tree: BlockTree,
    phantom: PhantomData<(V, T)>,
}

impl<V, T> BlockExecutorInner<V, T>
    where
        V: TransactionBlockExecutor<T>,
        T: Send + Sync,
{
    pub fn new(db: DbReaderWriter) -> Result<Self> {
        let block_tree = BlockTree::new(&db.reader)?;
        Ok(Self {
            db,
            block_tree,
            phantom: PhantomData,
        })
    }

    fn root_smt(&self) -> SparseMerkleTree<StateValue> {
        self.block_tree
            .root_block()
            .output
            .result_view
            .state()
            .current
            .clone()
    }
}

impl<V, T> BlockExecutorInner<V, T>
    where
        V: TransactionBlockExecutor<T>,
        T: Send + Sync,
{
    fn committed_block_id(&self) -> HashValue {
        self.block_tree.root_block().id
    }

    fn execute_block(
        &self,
        block: (HashValue, Vec<T>),
        parent_block_id: HashValue,
    ) -> Result<StateComputeResult, Error> {
        let (block_id, transactions) = block;
        let committed_block = self.block_tree.root_block();
        let mut block_vec = self
            .block_tree
            .get_blocks_opt(&[block_id, parent_block_id])?;
        let parent_block = block_vec
            .pop()
            .expect("Must exist.")
            .ok_or(Error::BlockNotFound(parent_block_id))?;
        let parent_output = &parent_block.output;
        let parent_view = &parent_output.result_view;
        let parent_accumulator = parent_view.txn_accumulator();

        if let Some(b) = block_vec.pop().expect("Must exist") {
            // this is a retry
            parent_block.ensure_has_child(block_id)?;
            return Ok(b.output.as_state_compute_result(parent_accumulator));
        }

        let output = if parent_block_id != committed_block.id && parent_output.has_reconfiguration()
        {
            info!(
                LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
                "reconfig_descendant_block_received"
            );
            parent_output.reconfig_suffix()
        } else {
            info!(
                LogSchema::new(LogEntry::BlockExecutor).block_id(block_id),
                "execute_block"
            );
            let _timer = APTOS_EXECUTOR_EXECUTE_BLOCK_SECONDS.start_timer();
            let state_view = {
                let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                    .with_label_values(&["verified_state_view"])
                    .start_timer();
                parent_view.verified_state_view(
                    StateViewId::BlockExecution { block_id },
                    Arc::clone(&self.db.reader),
                    Arc::new(AsyncProofFetcher::new(self.db.reader.clone())),
                )?
            };

            let chunk_output = {
                let _timer = APTOS_EXECUTOR_VM_EXECUTE_BLOCK_SECONDS.start_timer();
                fail_point!("executor::vm_execute_block", |_| {
                    Err(Error::from(anyhow::anyhow!(
                        "Injected error in vm_execute_block"
                    )))
                });
                V::execute_transaction_block(transactions, state_view)?
            };

            for out in &chunk_output.transaction_outputs {
                for event in out.events() {
                    println!("------contract event----{}------", event);
                }
                let status = out.status();

                match status {
                    TransactionStatus::Keep(x) => {
                        match ExecutionStatus::Success {
                            ExecutionStatus::Success => {
                                println!("------contract status---Success------");
                            }
                            ExecutionStatus::OutOfGas => {
                                println!("------contract status---OutOfGas------");
                            }
                            ExecutionStatus::MoveAbort { .. } => {
                                println!("------contract status---MoveAbort------");
                            }
                            ExecutionStatus::ExecutionFailure { .. } => {
                                println!("------contract status---ExecutionFailure------");
                            }
                            ExecutionStatus::MiscellaneousError(code) => {
                                let c = code.unwrap();
                                println!("------contract status---ExecutionFailure----");
                            }
                        }
                    }
                    TransactionStatus::Discard(status) => {
                        println!("------contract status------Discard----");
                        match status {
                            DiscardedVMStatus::UNKNOWN_STATUS => { println!("------contract status------Discard--UNKNOWN_STATUS--"); }
                            DiscardedVMStatus::UNKNOWN_VALIDATION_STATUS => { println!("------contract status------Discard--UNKNOWN_VALIDATION_STATUS--"); }
                            DiscardedVMStatus::INVALID_SIGNATURE => { println!("------contract status------Discard--INVALID_SIGNATURE--"); }
                            DiscardedVMStatus::INVALID_AUTH_KEY => { println!("------contract status------Discard--INVALID_AUTH_KEY--"); }
                            DiscardedVMStatus::SEQUENCE_NUMBER_TOO_OLD => { println!("------contract status------Discard--SEQUENCE_NUMBER_TOO_OLD--"); }
                            DiscardedVMStatus::SEQUENCE_NUMBER_TOO_NEW => { println!("------contract status------Discard--SEQUENCE_NUMBER_TOO_NEW--"); }
                            DiscardedVMStatus::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE => { println!("------contract status------Discard--INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE--"); }
                            DiscardedVMStatus::TRANSACTION_EXPIRED => { println!("------contract status------Discard--TRANSACTION_EXPIRED--"); }
                            DiscardedVMStatus::SENDING_ACCOUNT_DOES_NOT_EXIST => {}
                            DiscardedVMStatus::REJECTED_WRITE_SET => {}
                            DiscardedVMStatus::INVALID_WRITE_SET => {}
                            DiscardedVMStatus::EXCEEDED_MAX_TRANSACTION_SIZE => {}
                            DiscardedVMStatus::UNKNOWN_SCRIPT => {}
                            DiscardedVMStatus::UNKNOWN_MODULE => {}
                            DiscardedVMStatus::MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND => {}
                            DiscardedVMStatus::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS => {}
                            DiscardedVMStatus::GAS_UNIT_PRICE_BELOW_MIN_BOUND => {}
                            DiscardedVMStatus::GAS_UNIT_PRICE_ABOVE_MAX_BOUND => {}
                            DiscardedVMStatus::INVALID_GAS_SPECIFIER => {}
                            DiscardedVMStatus::SENDING_ACCOUNT_FROZEN => {}
                            DiscardedVMStatus::UNABLE_TO_DESERIALIZE_ACCOUNT => {}
                            DiscardedVMStatus::CURRENCY_INFO_DOES_NOT_EXIST => {}
                            DiscardedVMStatus::INVALID_MODULE_PUBLISHER => {}
                            DiscardedVMStatus::NO_ACCOUNT_ROLE => {}
                            DiscardedVMStatus::BAD_CHAIN_ID => {}
                            DiscardedVMStatus::SEQUENCE_NUMBER_TOO_BIG => {}
                            DiscardedVMStatus::BAD_TRANSACTION_FEE_CURRENCY => {}
                            DiscardedVMStatus::FEATURE_UNDER_GATING => {}
                            DiscardedVMStatus::SECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH => {}
                            DiscardedVMStatus::SIGNERS_CONTAIN_DUPLICATES => {}
                            DiscardedVMStatus::SEQUENCE_NONCE_INVALID => {}
                            DiscardedVMStatus::CHAIN_ACCOUNT_INFO_DOES_NOT_EXIST => {}
                            DiscardedVMStatus::ACCOUNT_NOT_MULTISIG => {}
                            DiscardedVMStatus::NOT_MULTISIG_OWNER => {}
                            DiscardedVMStatus::MULTISIG_TRANSACTION_NOT_FOUND => {}
                            DiscardedVMStatus::MULTISIG_TRANSACTION_INSUFFICIENT_APPROVALS => {}
                            DiscardedVMStatus::MULTISIG_TRANSACTION_PAYLOAD_DOES_NOT_MATCH_HASH => {}
                            DiscardedVMStatus::RESERVED_VALIDATION_ERROR_1 => {}
                            DiscardedVMStatus::RESERVED_VALIDATION_ERROR_2 => {}
                            DiscardedVMStatus::RESERVED_VALIDATION_ERROR_3 => {}
                            DiscardedVMStatus::RESERVED_VALIDATION_ERROR_4 => {}
                            DiscardedVMStatus::RESERVED_VALIDATION_ERROR_5 => {}
                            DiscardedVMStatus::UNKNOWN_VERIFICATION_ERROR => {}
                            DiscardedVMStatus::INDEX_OUT_OF_BOUNDS => {}
                            DiscardedVMStatus::INVALID_SIGNATURE_TOKEN => {}
                            DiscardedVMStatus::RECURSIVE_STRUCT_DEFINITION => {}
                            DiscardedVMStatus::FIELD_MISSING_TYPE_ABILITY => {}
                            DiscardedVMStatus::INVALID_FALL_THROUGH => {}
                            DiscardedVMStatus::NEGATIVE_STACK_SIZE_WITHIN_BLOCK => {}
                            DiscardedVMStatus::INVALID_MAIN_FUNCTION_SIGNATURE => {}
                            DiscardedVMStatus::DUPLICATE_ELEMENT => {}
                            DiscardedVMStatus::INVALID_MODULE_HANDLE => {}
                            DiscardedVMStatus::UNIMPLEMENTED_HANDLE => {}
                            DiscardedVMStatus::LOOKUP_FAILED => {}
                            DiscardedVMStatus::TYPE_MISMATCH => {}
                            DiscardedVMStatus::MISSING_DEPENDENCY => {}
                            DiscardedVMStatus::POP_WITHOUT_DROP_ABILITY => {}
                            DiscardedVMStatus::BR_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::ABORT_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::STLOC_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::STLOC_UNSAFE_TO_DESTROY_ERROR => {}
                            DiscardedVMStatus::UNSAFE_RET_LOCAL_OR_RESOURCE_STILL_BORROWED => {}
                            DiscardedVMStatus::RET_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::RET_BORROWED_MUTABLE_REFERENCE_ERROR => {}
                            DiscardedVMStatus::FREEZEREF_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::FREEZEREF_EXISTS_MUTABLE_BORROW_ERROR => {}
                            DiscardedVMStatus::BORROWFIELD_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::BORROWFIELD_BAD_FIELD_ERROR => {}
                            DiscardedVMStatus::BORROWFIELD_EXISTS_MUTABLE_BORROW_ERROR => {}
                            DiscardedVMStatus::COPYLOC_UNAVAILABLE_ERROR => {}
                            DiscardedVMStatus::COPYLOC_WITHOUT_COPY_ABILITY => {}
                            DiscardedVMStatus::COPYLOC_EXISTS_BORROW_ERROR => {}
                            DiscardedVMStatus::MOVELOC_UNAVAILABLE_ERROR => {}
                            DiscardedVMStatus::MOVELOC_EXISTS_BORROW_ERROR => {}
                            DiscardedVMStatus::BORROWLOC_REFERENCE_ERROR => {}
                            DiscardedVMStatus::BORROWLOC_UNAVAILABLE_ERROR => {}
                            DiscardedVMStatus::BORROWLOC_EXISTS_BORROW_ERROR => {}
                            DiscardedVMStatus::CALL_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::CALL_BORROWED_MUTABLE_REFERENCE_ERROR => {}
                            DiscardedVMStatus::PACK_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::UNPACK_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::READREF_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::READREF_WITHOUT_COPY_ABILITY => {}
                            DiscardedVMStatus::READREF_EXISTS_MUTABLE_BORROW_ERROR => {}
                            DiscardedVMStatus::WRITEREF_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::WRITEREF_WITHOUT_DROP_ABILITY => {}
                            DiscardedVMStatus::WRITEREF_EXISTS_BORROW_ERROR => {}
                            DiscardedVMStatus::WRITEREF_NO_MUTABLE_REFERENCE_ERROR => {}
                            DiscardedVMStatus::INTEGER_OP_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::BOOLEAN_OP_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::EQUALITY_OP_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::EXISTS_WITHOUT_KEY_ABILITY_OR_BAD_ARGUMENT => {}
                            DiscardedVMStatus::BORROWGLOBAL_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::BORROWGLOBAL_WITHOUT_KEY_ABILITY => {}
                            DiscardedVMStatus::MOVEFROM_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::MOVEFROM_WITHOUT_KEY_ABILITY => {}
                            DiscardedVMStatus::MOVETO_TYPE_MISMATCH_ERROR => {}
                            DiscardedVMStatus::MOVETO_WITHOUT_KEY_ABILITY => {}
                            DiscardedVMStatus::MODULE_ADDRESS_DOES_NOT_MATCH_SENDER => {}
                            DiscardedVMStatus::NO_MODULE_HANDLES => {}
                            DiscardedVMStatus::POSITIVE_STACK_SIZE_AT_BLOCK_END => {}
                            DiscardedVMStatus::MISSING_ACQUIRES_ANNOTATION => {}
                            DiscardedVMStatus::EXTRANEOUS_ACQUIRES_ANNOTATION => {}
                            DiscardedVMStatus::DUPLICATE_ACQUIRES_ANNOTATION => {}
                            DiscardedVMStatus::INVALID_ACQUIRES_ANNOTATION => {}
                            DiscardedVMStatus::GLOBAL_REFERENCE_ERROR => {}
                            DiscardedVMStatus::CONSTRAINT_NOT_SATISFIED => {}
                            DiscardedVMStatus::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH => {}
                            DiscardedVMStatus::LOOP_IN_INSTANTIATION_GRAPH => {}
                            DiscardedVMStatus::ZERO_SIZED_STRUCT => {}
                            DiscardedVMStatus::LINKER_ERROR => {}
                            DiscardedVMStatus::INVALID_CONSTANT_TYPE => {}
                            DiscardedVMStatus::MALFORMED_CONSTANT_DATA => {}
                            DiscardedVMStatus::EMPTY_CODE_UNIT => {}
                            DiscardedVMStatus::INVALID_LOOP_SPLIT => {}
                            DiscardedVMStatus::INVALID_LOOP_BREAK => {}
                            DiscardedVMStatus::INVALID_LOOP_CONTINUE => {}
                            DiscardedVMStatus::UNSAFE_RET_UNUSED_VALUES_WITHOUT_DROP => {}
                            DiscardedVMStatus::TOO_MANY_LOCALS => {}
                            DiscardedVMStatus::GENERIC_MEMBER_OPCODE_MISMATCH => {}
                            DiscardedVMStatus::FUNCTION_RESOLUTION_FAILURE => {}
                            DiscardedVMStatus::INVALID_OPERATION_IN_SCRIPT => {}
                            DiscardedVMStatus::DUPLICATE_MODULE_NAME => {}
                            DiscardedVMStatus::BACKWARD_INCOMPATIBLE_MODULE_UPDATE => {}
                            DiscardedVMStatus::CYCLIC_MODULE_DEPENDENCY => {}
                            DiscardedVMStatus::NUMBER_OF_ARGUMENTS_MISMATCH => {}
                            DiscardedVMStatus::INVALID_PARAM_TYPE_FOR_DESERIALIZATION => {}
                            DiscardedVMStatus::FAILED_TO_DESERIALIZE_ARGUMENT => {}
                            DiscardedVMStatus::NUMBER_OF_SIGNER_ARGUMENTS_MISMATCH => {}
                            DiscardedVMStatus::CALLED_SCRIPT_VISIBLE_FROM_NON_SCRIPT_VISIBLE => {}
                            DiscardedVMStatus::EXECUTE_ENTRY_FUNCTION_CALLED_ON_NON_ENTRY_FUNCTION => {}
                            DiscardedVMStatus::INVALID_FRIEND_DECL_WITH_SELF => {}
                            DiscardedVMStatus::INVALID_FRIEND_DECL_WITH_MODULES_OUTSIDE_ACCOUNT_ADDRESS => {}
                            DiscardedVMStatus::INVALID_FRIEND_DECL_WITH_MODULES_IN_DEPENDENCIES => {}
                            DiscardedVMStatus::CYCLIC_MODULE_FRIENDSHIP => {}
                            DiscardedVMStatus::INVALID_PHANTOM_TYPE_PARAM_POSITION => {}
                            DiscardedVMStatus::VEC_UPDATE_EXISTS_MUTABLE_BORROW_ERROR => {}
                            DiscardedVMStatus::VEC_BORROW_ELEMENT_EXISTS_MUTABLE_BORROW_ERROR => {}
                            DiscardedVMStatus::LOOP_MAX_DEPTH_REACHED => {}
                            DiscardedVMStatus::TOO_MANY_TYPE_PARAMETERS => {}
                            DiscardedVMStatus::TOO_MANY_PARAMETERS => {}
                            DiscardedVMStatus::TOO_MANY_BASIC_BLOCKS => {}
                            DiscardedVMStatus::VALUE_STACK_OVERFLOW => {}
                            DiscardedVMStatus::TOO_MANY_TYPE_NODES => {}
                            DiscardedVMStatus::VALUE_STACK_PUSH_OVERFLOW => {}
                            DiscardedVMStatus::MAX_DEPENDENCY_DEPTH_REACHED => {}
                            DiscardedVMStatus::MAX_FUNCTION_DEFINITIONS_REACHED => {}
                            DiscardedVMStatus::MAX_STRUCT_DEFINITIONS_REACHED => {}
                            DiscardedVMStatus::MAX_FIELD_DEFINITIONS_REACHED => {}
                            DiscardedVMStatus::TOO_MANY_BACK_EDGES => {}
                            DiscardedVMStatus::RESERVED_VERIFICATION_ERROR_1 => {}
                            DiscardedVMStatus::RESERVED_VERIFICATION_ERROR_2 => {}
                            DiscardedVMStatus::RESERVED_VERIFICATION_ERROR_3 => {}
                            DiscardedVMStatus::RESERVED_VERIFICATION_ERROR_4 => {}
                            DiscardedVMStatus::RESERVED_VERIFICATION_ERROR_5 => {}
                            DiscardedVMStatus::UNKNOWN_INVARIANT_VIOLATION_ERROR => {}
                            DiscardedVMStatus::EMPTY_VALUE_STACK => {}
                            DiscardedVMStatus::PC_OVERFLOW => {}
                            DiscardedVMStatus::VERIFICATION_ERROR => {}
                            DiscardedVMStatus::STORAGE_ERROR => {}
                            DiscardedVMStatus::INTERNAL_TYPE_ERROR => {}
                            DiscardedVMStatus::EVENT_KEY_MISMATCH => {}
                            DiscardedVMStatus::UNREACHABLE => {}
                            DiscardedVMStatus::VM_STARTUP_FAILURE => {}
                            DiscardedVMStatus::UNEXPECTED_ERROR_FROM_KNOWN_MOVE_FUNCTION => {}
                            DiscardedVMStatus::VERIFIER_INVARIANT_VIOLATION => {}
                            DiscardedVMStatus::UNEXPECTED_VERIFIER_ERROR => {}
                            DiscardedVMStatus::UNEXPECTED_DESERIALIZATION_ERROR => {}
                            DiscardedVMStatus::FAILED_TO_SERIALIZE_WRITE_SET_CHANGES => {}
                            DiscardedVMStatus::FAILED_TO_DESERIALIZE_RESOURCE => {}
                            DiscardedVMStatus::TYPE_RESOLUTION_FAILURE => {}
                            DiscardedVMStatus::DUPLICATE_NATIVE_FUNCTION => {}
                            DiscardedVMStatus::RESERVED_INVARIANT_VIOLATION_ERROR_1 => {}
                            DiscardedVMStatus::RESERVED_INVARIANT_VIOLATION_ERROR_2 => {}
                            DiscardedVMStatus::RESERVED_INVARIANT_VIOLATION_ERROR_3 => {}
                            DiscardedVMStatus::RESERVED_INVARIANT_VIOLATION_ERROR_4 => {}
                            DiscardedVMStatus::RESERVED_INVARIANT_VIOLATION_ERROR_5 => {}
                            DiscardedVMStatus::UNKNOWN_BINARY_ERROR => {}
                            DiscardedVMStatus::MALFORMED => {}
                            DiscardedVMStatus::BAD_MAGIC => {}
                            DiscardedVMStatus::UNKNOWN_VERSION => {}
                            DiscardedVMStatus::UNKNOWN_TABLE_TYPE => {}
                            DiscardedVMStatus::UNKNOWN_SIGNATURE_TYPE => {}
                            DiscardedVMStatus::UNKNOWN_SERIALIZED_TYPE => {}
                            DiscardedVMStatus::UNKNOWN_OPCODE => {}
                            DiscardedVMStatus::BAD_HEADER_TABLE => {}
                            DiscardedVMStatus::UNEXPECTED_SIGNATURE_TYPE => {}
                            DiscardedVMStatus::DUPLICATE_TABLE => {}
                            DiscardedVMStatus::UNKNOWN_ABILITY => {}
                            DiscardedVMStatus::UNKNOWN_NATIVE_STRUCT_FLAG => {}
                            DiscardedVMStatus::BAD_U16 => {}
                            DiscardedVMStatus::BAD_U32 => {}
                            DiscardedVMStatus::BAD_U64 => {}
                            DiscardedVMStatus::BAD_U128 => {}
                            DiscardedVMStatus::BAD_U256 => {}
                            DiscardedVMStatus::VALUE_SERIALIZATION_ERROR => {}
                            DiscardedVMStatus::VALUE_DESERIALIZATION_ERROR => {}
                            DiscardedVMStatus::CODE_DESERIALIZATION_ERROR => {}
                            DiscardedVMStatus::INVALID_FLAG_BITS => {}
                            DiscardedVMStatus::RESERVED_DESERIALIZAION_ERROR_1 => {}
                            DiscardedVMStatus::RESERVED_DESERIALIZAION_ERROR_2 => {}
                            DiscardedVMStatus::RESERVED_DESERIALIZAION_ERROR_3 => {}
                            DiscardedVMStatus::RESERVED_DESERIALIZAION_ERROR_4 => {}
                            DiscardedVMStatus::RESERVED_DESERIALIZAION_ERROR_5 => {}
                            DiscardedVMStatus::UNKNOWN_RUNTIME_STATUS => {}
                            DiscardedVMStatus::EXECUTED => {}
                            DiscardedVMStatus::OUT_OF_GAS => {}
                            DiscardedVMStatus::RESOURCE_DOES_NOT_EXIST => {}
                            DiscardedVMStatus::RESOURCE_ALREADY_EXISTS => {}
                            DiscardedVMStatus::MISSING_DATA => {}
                            DiscardedVMStatus::DATA_FORMAT_ERROR => {}
                            DiscardedVMStatus::ABORTED => {}
                            DiscardedVMStatus::ARITHMETIC_ERROR => {}
                            DiscardedVMStatus::VECTOR_OPERATION_ERROR => {}
                            DiscardedVMStatus::EXECUTION_STACK_OVERFLOW => {}
                            DiscardedVMStatus::CALL_STACK_OVERFLOW => {}
                            DiscardedVMStatus::VM_MAX_TYPE_DEPTH_REACHED => {}
                            DiscardedVMStatus::VM_MAX_VALUE_DEPTH_REACHED => {}
                            DiscardedVMStatus::VM_EXTENSION_ERROR => {}
                            DiscardedVMStatus::STORAGE_WRITE_LIMIT_REACHED => {}
                            DiscardedVMStatus::MEMORY_LIMIT_EXCEEDED => {}
                            DiscardedVMStatus::VM_MAX_TYPE_NODES_REACHED => {}
                            DiscardedVMStatus::EXECUTION_LIMIT_REACHED => {}
                            DiscardedVMStatus::IO_LIMIT_REACHED => {}
                            DiscardedVMStatus::STORAGE_LIMIT_REACHED => {}
                            DiscardedVMStatus::RESERVED_RUNTIME_ERROR_1 => {}
                            DiscardedVMStatus::RESERVED_RUNTIME_ERROR_2 => {}
                            DiscardedVMStatus::RESERVED_RUNTIME_ERROR_3 => {}
                            DiscardedVMStatus::RESERVED_RUNTIME_ERROR_4 => {}
                            DiscardedVMStatus::RESERVED_RUNTIME_ERROR_5 => {}
                        }
                    }
                    TransactionStatus::Retry => {
                        println!("------contract status---retry------");
                    }
                };
            }

            chunk_output.trace_log_transaction_status();

            let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                .with_label_values(&["apply_to_ledger"])
                .start_timer();
            let (output, _, _) = chunk_output.apply_to_ledger(parent_view)?;
            output
        };
        output.ensure_ends_with_state_checkpoint()?;

        let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
            .with_label_values(&["as_state_compute_result"])
            .start_timer();
        let block = self
            .block_tree
            .add_block(parent_block_id, block_id, output)?;
        Ok(block.output.as_state_compute_result(parent_accumulator))
    }

    fn commit_blocks_ext(
        &self,
        block_ids: Vec<HashValue>,
        ledger_info_with_sigs: LedgerInfoWithSignatures,
        sync_commit: bool,
    ) -> Result<(), Error> {
        let _timer = APTOS_EXECUTOR_COMMIT_BLOCKS_SECONDS.start_timer();

        // Ensure the block ids are not empty
        if block_ids.is_empty() {
            return Err(anyhow::anyhow!("Cannot commit 0 blocks!").into());
        }

        // Check for any potential retries
        let committed_block = self.block_tree.root_block();
        if committed_block.num_persisted_transactions()
            == ledger_info_with_sigs.ledger_info().version() + 1
        {
            return Ok(());
        }

        // Ensure the last block id matches the ledger info block id to commit
        let block_id_to_commit = ledger_info_with_sigs.ledger_info().consensus_block_id();
        info!(
            LogSchema::new(LogEntry::BlockExecutor).block_id(block_id_to_commit),
            "commit_block"
        );
        let last_block_id = *block_ids.last().unwrap();
        if last_block_id != block_id_to_commit {
            // This should not happen. If it does, we need to panic!
            panic!(
                "Block id to commit ({:?}) does not match last block id ({:?})!",
                block_id_to_commit, last_block_id
            );
        }

        let blocks = self.block_tree.get_blocks(&block_ids)?;
        let txns_to_commit: Vec<_> = {
            let _timer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                .with_label_values(&["get_txns_to_commit"])
                .start_timer();
            blocks
                .into_iter()
                .map(|block| block.output.transactions_to_commit())
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .flatten()
                .collect()
        };
        let first_version = committed_block
            .output
            .result_view
            .txn_accumulator()
            .num_leaves();
        let to_commit = txns_to_commit.len();
        let target_version = ledger_info_with_sigs.ledger_info().version();
        if first_version + txns_to_commit.len() as u64 != target_version + 1 {
            return Err(Error::BadNumTxnsToCommit {
                first_version,
                to_commit,
                target_version,
            });
        }

        let _timer = APTOS_EXECUTOR_SAVE_TRANSACTIONS_SECONDS.start_timer();
        APTOS_EXECUTOR_TRANSACTIONS_SAVED.observe(to_commit as f64);

        fail_point!("executor::commit_blocks", |_| {
            Err(anyhow::anyhow!("Injected error in commit_blocks.").into())
        });
        let result_in_memory_state = self
            .block_tree
            .get_block(block_id_to_commit)?
            .output
            .result_view
            .state()
            .clone();
        self.db.writer.save_transactions(
            &txns_to_commit,
            first_version,
            committed_block.output.result_view.state().base_version,
            Some(&ledger_info_with_sigs),
            sync_commit,
            result_in_memory_state,
        )?;
        self.block_tree
            .prune(ledger_info_with_sigs.ledger_info())
            .expect("Failure pruning block tree.");

        Ok(())
    }
}
