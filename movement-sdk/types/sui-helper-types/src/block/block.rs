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
        let mut groups: Vec<Vec<VerifiedExecutableTransaction>> = vec![];
        let mut group_id = 0;
        let mut processed_cnt = 0;
        // (objectId, is_mutable)
        let mut objects_in_groups: Vec<Vec<(ObjectID, bool)>> = vec![];
        let mut processed_digests: Vec<SenderSignedDataDigest> = vec![];

        let tx_cnt = self.clone().into_iter().len();
        loop {
            for executable_transaction in self.clone().into_iter() {
                let sender_signed_data = executable_transaction.clone().into_message();

                // check if transaction is already pushed in groups
                if let Some(_found) = processed_digests.iter().find(|&&tx_digest| tx_digest == sender_signed_data.full_message_digest()) {
                    break;
                }

                let transaction_data = sender_signed_data.transaction_data();
                let TransactionData::V1(tx_data_v1) = transaction_data;
                let shared_objects = tx_data_v1.shared_input_objects();
                    
                // check out if any shared objects are conflicted in the group
                let mut mut_obj_duplicated = false;
                for obj in shared_objects.iter() {
                    if let Some (_found) = objects_in_groups[group_id].iter().find(|&obj_in_group| obj_in_group.0 == obj.id && obj_in_group.1 && obj.mutable) {
                        mut_obj_duplicated = true;
                        break
                    }
                }

                // if no object conflicts, then the transaction can go in the group
                if !mut_obj_duplicated {
                    for obj in shared_objects.iter() {
                        objects_in_groups[group_id].push((obj.id, obj.mutable));
                    }
                    
                    // add a tx into group
                    groups[group_id].push(executable_transaction.clone());    

                    // set transaction as processed
                    processed_digests.push(sender_signed_data.full_message_digest());

                    // increase processed_count
                    processed_cnt += 1;
                }
            }
            // start new group
            group_id += 1;

            // if all transactions are included in groups, then exit from the loop
            if processed_cnt == tx_cnt {
                break
            }
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
