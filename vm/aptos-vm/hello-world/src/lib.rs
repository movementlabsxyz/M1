use std::sync::Arc;
use rand::SeedableRng;
use aptos_crypto::HashValue;
use aptos_db::AptosDB;
use aptos_executor::block_executor::BlockExecutor;
use aptos_executor::db_bootstrapper::{generate_waypoint, maybe_bootstrap};
use aptos_executor_types::{BlockExecutorTrait, StateComputeResult};
use aptos_logger::info;
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_sdk::types::{AccountKey, LocalAccount};
use aptos_state_view::account_with_state_view::{AccountWithStateView, AsAccountWithStateView};
use aptos_storage_interface::DbReaderWriter;
use aptos_storage_interface::state_view::DbStateViewAtVersion;
use aptos_types::{
    account_config::aptos_test_root_address,
    account_view::AccountView,
    block_metadata::BlockMetadata,
    chain_id::ChainId,
    event::EventKey,
    transaction::{
        Transaction, Transaction::UserTransaction, TransactionListWithProof, TransactionWithProof,
        WriteSetPayload,
    },
    trusted_state::{TrustedState, TrustedStateChange},
    waypoint::Waypoint,
};
use aptos_crypto;
use aptos_vm_genesis::*;
use aptos_temppath::TempPath;
use aptos_types::block_info::BlockInfo;
use aptos_types::ledger_info::{generate_ledger_info_with_sig, LedgerInfo, LedgerInfoWithSignatures};
use aptos_types::validator_signer::ValidatorSigner;
use aptos_vm::{AptosVM, VMExecutor};

pub fn exe_transaction() {
    const B: u64 = 1_000_000_000;
    let (genesis, validators) = test_genesis_change_set_and_validators(Some(1));
    let genesis_txn = Transaction::GenesisTransaction(WriteSetPayload::Direct(genesis));
    let mut core_resources_account: LocalAccount = LocalAccount::new(
        aptos_test_root_address(),
        AccountKey::from_private_key(GENESIS_KEYPAIR.0.clone()),
        0,
    );
    let path = TempPath::new();
    path.create_as_dir().unwrap();
    let (aptos_db,
        db,
        executor,
        waypoint) = create_db_and_executor(path.path(), &genesis_txn);
    let signer = ValidatorSigner::new(
        validators[0].data.owner_address,
        validators[0].consensus_key.clone(),
    );
    // This generates accounts that do not overlap with genesis
    let seed = [3u8; 32];
    let mut rng = ::rand::rngs::StdRng::from_seed(seed);

    let mut account1 = LocalAccount::generate(&mut rng);
    let mut account2 = LocalAccount::generate(&mut rng);

    let account1_address = account1.address();
    let account2_address = account2.address();
    let txn_factory = TransactionFactory::new(ChainId::test());
    let block1_id = gen_block_id(1);
    let block1_meta = Transaction::BlockMetadata(BlockMetadata::new(
        block1_id,
        1,
        0,
        signer.author(),
        vec![0],
        vec![],
        1,
    ));
    let acc_tx1 = core_resources_account
        .sign_with_transaction_builder(txn_factory.create_user_account(account1.public_key()));
    let token_acc1 = core_resources_account
        .sign_with_transaction_builder(txn_factory.mint(account1.address(), 1_000 * B));


    let acc_tx2 = core_resources_account
        .sign_with_transaction_builder(txn_factory.create_user_account(account2.public_key()));
    let token_acc2 = core_resources_account
        .sign_with_transaction_builder(txn_factory.mint(account2.address(), 2_000 * B));
    // let _reconfig1 = core_resources_account
    //     .sign_with_transaction_builder(txn_factory.payload(aptos_stdlib::version_set_version(100)));
    let block1: Vec<_> = vec![
        block1_meta,
        UserTransaction(acc_tx1),
        UserTransaction(token_acc1),
        UserTransaction(acc_tx2),
        UserTransaction(token_acc2),
        Transaction::StateCheckpoint(HashValue::random()),
    ];
    let parent_block_id = executor.committed_block_id();
    let output1 = executor
        .execute_block((block1_id, block1.clone()), parent_block_id)
        .unwrap();
    let li1 = gen_ledger_info_with_sigs(1, &output1, block1_id, &[signer.clone()]);
    executor.commit_blocks(vec![block1_id], li1).unwrap();
    let state_proof = db.reader.get_state_proof(0).unwrap();
    let current_version = state_proof.latest_ledger_info().version();
    info!("--current_version---{}-", current_version);
    let db_state_view = db.reader.state_view_at_version(Some(2)).unwrap();
    let account1_view = db_state_view.as_account_with_state_view(&account1_address);
    let bal = get_account_balance(&account1_view);
    info!("--account 1 bal at version 2 ---{}-", bal);

    let db_state_view = db.reader.state_view_at_version(Some(3)).unwrap();
    let account1_view = db_state_view.as_account_with_state_view(&account1_address);
    let bal = get_account_balance(&account1_view);
    info!("--account 1 bal at version 3 ---{}-", bal);

    let block2_id = gen_block_id(2);
    let block2_meta = Transaction::BlockMetadata(BlockMetadata::new(
        block2_id,
        2,
        0,
        signer.author(),
        vec![0],
        vec![],
        2,
    ));

    let tx_transfer = account1.sign_with_transaction_builder(
        txn_factory.transfer(account2_address, 99 * B));
    let block2: Vec<_> = vec![
        block2_meta,
        UserTransaction(tx_transfer),
        Transaction::StateCheckpoint(HashValue::random()),
    ];

    let parent_block_id = executor.committed_block_id();
    let output2 = executor
        .execute_block((block2_id, block2.clone()), parent_block_id)
        .unwrap();
    let li2 = gen_ledger_info_with_sigs(1, &output2, block2_id, &[signer.clone()]);
    executor.commit_blocks(vec![block2_id], li2).unwrap();

    let state_proof = db.reader.get_state_proof(0).unwrap();
    let current_version = state_proof.latest_ledger_info().version();
    info!("--current_version- after--{}-", current_version);
    let db_state_view = db.reader.state_view_at_version(Some(9)).unwrap();
    let account1_view = db_state_view.as_account_with_state_view(&account1_address);
    let bal = get_account_balance(&account1_view);
    info!("--account 1 bal ---{}-", bal);

    let account2_view = db_state_view.as_account_with_state_view(&account2_address);
    let bal = get_account_balance(&account2_view);
    info!("--account 2 bal  ---{}-", bal);
}

fn create_db_and_executor<P: AsRef<std::path::Path>>(
    path: P,
    genesis: &Transaction,
) -> (
    Arc<AptosDB>,
    DbReaderWriter,
    BlockExecutor<AptosVM, Transaction>,
    Waypoint,
) {
    let (db, dbrw) = DbReaderWriter::wrap(AptosDB::new_for_test(&path));
    let waypoint = bootstrap_genesis::<AptosVM>(&dbrw, genesis).unwrap();
    let executor = BlockExecutor::new(dbrw.clone());
    (db, dbrw, executor, waypoint)
}

fn get_account_balance(account_state_view: &AccountWithStateView) -> u64 {
    account_state_view
        .get_coin_store_resource()
        .unwrap()
        .map(|b| b.coin())
        .unwrap_or(0)
}

fn gen_ledger_info_with_sigs(
    epoch: u64,
    output: &StateComputeResult,
    commit_block_id: HashValue,
    signer: &[ValidatorSigner],
) -> LedgerInfoWithSignatures {
    let ledger_info = LedgerInfo::new(
        BlockInfo::new(
            epoch,
            0, /* round */
            commit_block_id,
            output.root_hash(),
            output.version(),
            0, /* timestamp */
            output.epoch_state().clone(),
        ),
        HashValue::zero(),
    );
    generate_ledger_info_with_sig(signer, ledger_info)
}

fn gen_block_id(index: u8) -> HashValue {
    HashValue::new([index; HashValue::LENGTH])
}

fn bootstrap_genesis<V: VMExecutor>(
    db: &DbReaderWriter,
    genesis_txn: &Transaction,
) -> anyhow::Result<Waypoint> {
    let waypoint = generate_waypoint::<V>(db, genesis_txn)?;
    maybe_bootstrap::<V>(db, genesis_txn, waypoint)?;
    Ok(waypoint)
}

pub fn block(mut user_txns: Vec<Transaction>) -> Vec<Transaction> {
    user_txns.push(Transaction::StateCheckpoint(HashValue::random()));
    user_txns
}