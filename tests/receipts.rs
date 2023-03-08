use std::{path::PathBuf, str::FromStr};

// use reth_db::{tables, transaction::DbTx, database::Database};
use reth_primitives::{rpc::H256, U256};

use op_reth::cli::{db, receipts};

const RECEIPTS_PATH: &str = "data/export_receipt_0_4061223";

#[test]
fn test_receipts_from_file() {
    let receipts = receipts::Receipt::from_file(RECEIPTS_PATH).unwrap();
    assert_eq!(0, receipts[0].ty);
    assert_eq!(1, receipts[0].status);
    assert_eq!(151191, receipts[0].cumulative_gas_used);
    assert_eq!(
        H256::from_str("0x7334ddc1f6beaf66892c25cffdecec275cdfabaf4def047f0c3ce20e6f6483e8")
            .unwrap(),
        receipts[0].tx_hash
    );
    assert_eq!(151191, receipts[0].gas_used);
    assert_eq!(
        H256::from_str("0x15d55041e8f7b0d1f303b6d4cefe2d2efc257d67acd9f17307261a8f7d786e0e")
            .unwrap(),
        receipts[0].block_hash
    );
    assert_eq!(U256::from(1), receipts[0].block_number);
    assert_eq!(0, receipts[0].transaction_index);
    assert_eq!(U256::from(1), receipts[0].l1_gas_price);
    assert_eq!(U256::from_str("0x1b62").unwrap(), receipts[0].l1_gas_used);
    assert_eq!(U256::from_str("0x2913").unwrap(), receipts[0].l1_fee);
    assert_eq!("1.5", receipts[0].l1_fee_scalar);
    assert_eq!(4029549, receipts.len());
}

#[tokio::test]
async fn test_read_write_receipts() {
    let db_path = PathBuf::from("temp-receipts");
    let mut db = db::open_rw_env(db_path.as_path()).unwrap();
    receipts::apply(&mut db, Some(RECEIPTS_PATH)).await.unwrap();

    // TODO: receipts::apply doesn't write anything to the database yet

    std::fs::remove_dir_all(db_path).unwrap();
}
