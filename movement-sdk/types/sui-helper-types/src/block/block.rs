use std::collections::HashSet;

use sui_types::base_types::ObjectID;
use sui_types::digests::SenderSignedDataDigest;
use sui_types::message_envelope::VerifiedEnvelope;
use sui_types::{
    transaction::SenderSignedData,
    executable_transaction::VerifiedExecutableTransaction
};

use sui_types::transaction::{TransactionDataAPI, TransactionData};

/// A SuiBlock is a block as we would most often expect it to be constructed. 
/// It contains only user signed data. 
#[derive(Debug, Clone)]
pub struct Block(Vec<SenderSignedData>);

/// A VerifiedBlock is a block that has been verified by the SuiBlockExecutor; it contains `CertificateEnvelope`s for each transaction.
/// In most cases, this should be internally constructed.
#[derive(Debug, Clone)]
pub struct VerifiedExecutableBlock(Vec<VerifiedExecutableTransaction>);

#[derive(Debug, Clone)]
pub struct VerifiedExecutableExecutionGroups(Vec<Vec<VerifiedExecutableTransaction>>);

impl IntoIterator for VerifiedExecutableExecutionGroups {
    type Item = Vec<VerifiedExecutableTransaction>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl VerifiedExecutableBlock {

    pub fn new(transactions : Vec<VerifiedExecutableTransaction>) -> Self {
        Self(transactions)
    }

    pub fn get_max_parallel_groups(&self) -> VerifiedExecutableExecutionGroups {
        let mut groups: Vec<Vec<VerifiedExecutableTransaction>> = Vec::new();
        let mut objects_in_groups: Vec<HashSet<(ObjectID, bool)>> = Vec::new();
        let mut processed_digests: HashSet<SenderSignedDataDigest> = HashSet::new();
        for executable_transaction in self.clone().into_iter() {
            let sender_signed_data = executable_transaction.clone().into_message();

            // Skip if transaction is already processed
            if processed_digests.contains(&sender_signed_data.full_message_digest()) {
                continue;
            }

            let TransactionData::V1(tx_data_v1) = sender_signed_data.transaction_data();
            let shared_objects = tx_data_v1.shared_input_objects();

            // Find a group where the transaction can be added without conflict
            let mut group_id = 0;
            while group_id < objects_in_groups.len() {
                let is_conflict = shared_objects.iter().any(|obj| {
                    objects_in_groups[group_id].contains(&(obj.id, true)) && obj.mutable
                });

                if !is_conflict {
                    break;
                }
                group_id += 1;
            }

            // If no suitable group, create a new one
            if group_id == objects_in_groups.len() {
                groups.push(Vec::new());
                objects_in_groups.push(HashSet::new());
            }

            // Add the transaction to the group
            for obj in shared_objects {
                objects_in_groups[group_id].insert((obj.id, obj.mutable));
            }
            groups[group_id].push(executable_transaction.clone());
            processed_digests.insert(sender_signed_data.full_message_digest());
        }
        VerifiedExecutableExecutionGroups(groups)
    }
    


}

impl IntoIterator for VerifiedExecutableBlock {
    type Item = VerifiedExecutableTransaction;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
