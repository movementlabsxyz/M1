use sui_helper_types::{
    providers::object_version::ObjectVersionProvider,
    block::VerifiedExecutableExecutionGroups
};
use sui_types::{digests::TransactionDigest, base_types::{ObjectID, SequenceNumber}, transaction::{TransactionData, TransactionDataAPI, SharedInputObject, SenderSignedData}, executable_transaction::VerifiedExecutableTransaction, message_envelope::{Message, VerifiedEnvelope, Envelope}};
use typed_store::{rocks::DBMap, Map};

#[derive(Debug, Clone)] // would like for this to remain debug clone
pub struct ObjectVersionRocksDB {
    shared_object_versions: DBMap<TransactionDigest, Vec<(ObjectID, SequenceNumber)>>
}

#[async_trait::async_trait]
impl ObjectVersionProvider for ObjectVersionRocksDB {
    
    // todo: implement against rocksdb store
    // todo: add methods to the trait in the `sui-helper-types` crate

    async fn assign_shared_object_versions(&self, transaction_groups : VerifiedExecutableExecutionGroups) -> Result<VerifiedExecutableExecutionGroups, anyhow::Error> {

        let mut new_groups: Vec<Vec<VerifiedExecutableTransaction>> = Vec::new();
/*
        for tx_group in transaction_groups {
            
            let mut new_group: Vec<VerifiedExecutableTransaction> = Vec::new();

            for transaction in tx_group {
                let modified_transaction = transaction.clone();
                let transaction_data = &mut modified_transaction.inner_mut().intent_message.value;

                let TransactionData::V1(tx_data_v1) = transaction_data;

                // get SharedObjects from transaction data
                let mut shared_objects: Vec<SharedInputObject> = tx_data_v1.kind_mut().shared_input_objects().collect();
                let digest = modified_transaction.digest();
                
                // update sequence_number of shared_objects
                if let Ok(shared_object_versions) = self.shared_object_versions.get(&digest) {
                    if shared_object_versions.is_some() {
                        for version in shared_object_versions.unwrap() {
                            if let Some(obj) = shared_objects.iter_mut().find(|obj| obj.id == version.0) {
                                obj.initial_shared_version = version.1;
                            }
                        }
                    }
                }
                
                new_group.push(modified_transaction);
            }

            new_groups.push(new_group);
        }
 */
        let verified_executable_execution_groups = VerifiedExecutableExecutionGroups(new_groups);
        Ok(verified_executable_execution_groups)
    }

}