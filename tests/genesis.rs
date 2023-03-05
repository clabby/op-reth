use std::{path::PathBuf, str::FromStr};

use reth_db::{database::Database, tables, transaction::DbTx};
use reth_primitives::*;

use op_reth::cli::{db, genesis};

#[test]
fn test_from_file() {
    let genesis = genesis::Genesis::from_file("data/genesis.json").unwrap();
    assert_eq!(genesis.config.chain_id, 420);
    assert_eq!(genesis.difficulty, "1");
    assert_eq!(genesis.gas_limit, "15000000");
    assert_eq!(genesis.extradata.len(), 236);
    assert_eq!(genesis.alloc.len(), 10);
}

#[tokio::test]
async fn test_write_read_genesis_db() {
    let db_path = PathBuf::from("temp-genesis-db");
    let mut db = db::open_rw_env(db_path.as_path()).unwrap();
    genesis::apply(&mut db, Some("data/genesis.json")).await.unwrap();

    // Read account from genesis state
    let tx = db.tx().unwrap();
    let address = H160::from_str("0x4200000000000000000000000000000000000011").unwrap();
    let account = tx.get::<tables::PlainAccountState>(address).unwrap();
    assert_eq!(
        Some(
            H256::from_str("0x8b846c7bbf2a0a4e6d36d5b9fd759f8fd1d2887a1b6732460e86436c8dcefc4d")
                .unwrap()
        ),
        account.unwrap().bytecode_hash
    );
    assert_eq!(U256::ZERO, account.unwrap().balance);
    assert_eq!(0, account.unwrap().nonce);

    // Read bytecode from genesis state
    let tx = db.tx().unwrap();
    let code = "0x6080604052600436106100385760003560e01c80633ccfd60b14610044578063d3e5792b1461005b578063d4ff92181461008a57600080fd5b3661003f57005b600080fd5b34801561005057600080fd5b506100596100dc565b005b34801561006757600080fd5b5061007767d02ab486cedc000081565b6040519081526020015b60405180910390f35b34801561009657600080fd5b506000546100b79073ffffffffffffffffffffffffffffffffffffffff1681565b60405173ffffffffffffffffffffffffffffffffffffffff9091168152602001610081565b67d02ab486cedc000047101561019e576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152605760248201527f4f564d5f53657175656e6365724665655661756c743a2077697468647261776160448201527f6c20616d6f756e74206d7573742062652067726561746572207468616e206d6960648201527f6e696d756d207769746864726177616c20616d6f756e74000000000000000000608482015260a40160405180910390fd5b600080546040805160208101825283815290517fa3a795480000000000000000000000000000000000000000000000000000000081527342000000000000000000000000000000000000109363a3a79548936102309373deaddeaddeaddeaddeaddeaddeaddeaddead00009373ffffffffffffffffffffffffffffffffffffffff909216924792909190600401610264565b600060405180830381600087803b15801561024a57600080fd5b505af115801561025e573d6000803e3d6000fd5b50505050565b600073ffffffffffffffffffffffffffffffffffffffff808816835260208188168185015286604085015263ffffffff8616606085015260a06080850152845191508160a085015260005b828110156102cb5785810182015185820160c0015281016102af565b828111156102dd57600060c084870101525b5050601f017fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe0169190910160c001969550505050505056fea2646970667358221220387a6116dde263ea48767352a397053c8cffa776aecb43cded2f25a4a9cfbdc264736f6c63430008090033";
    let code_bytes = Bytes::from_str(code).unwrap();
    let code_hash = keccak256(code_bytes.clone());
    let code = tx.get::<tables::Bytecodes>(code_hash).unwrap().unwrap();
    assert_eq!(code_bytes.to_vec(), code);

    // Check the tables::PlainStorageState
    let tx = db.tx().unwrap();
    let address = H160::from_str("0x4200000000000000000000000000000000000011").unwrap();
    let storage_key =
        H256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();
    let storage_value =
        U256::from_str("0x000000000000000000000000fd1d2e729ae8eee2e146c033bf4400fe75284301")
            .unwrap();
    let storage_entry = tx.get::<tables::PlainStorageState>(address).unwrap().unwrap();
    assert_eq!(StorageEntry { key: storage_key, value: storage_value }, storage_entry);

    let tx = db.tx().unwrap();
    let block_hash = tx.get::<tables::CanonicalHeaders>(0u64).unwrap().unwrap();
    let expected_block_hash =
        H256::from_str("0xad7e4e683df9b4b187b52e921d9d88e380c879b3956f6fb4d183baec76012bd0")
            .unwrap();
    assert_eq!(expected_block_hash, block_hash);

    let tx = db.tx().unwrap();
    let expected_difficulty = U256::from_str("0x01").unwrap();
    let expected_gas_limit = 15000000;
    let expected_extra_data = Bytes::from_str("0x000000000000000000000000000000000000000000000000000000000000000027770a9694e4B4b1E130Ab91Bc327C36855f612E0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let header = tx.get::<tables::Headers>(0u64).unwrap().unwrap();
    assert_eq!(
        Header {
            difficulty: expected_difficulty,
            gas_limit: expected_gas_limit,
            extra_data: expected_extra_data,
            ..Default::default()
        },
        header
    );

    std::fs::remove_dir_all(db_path).unwrap();
}
