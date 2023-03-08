use std::{path::PathBuf, str::FromStr};

use reth_db::{database::Database, tables, transaction::DbTx};
use reth_primitives::{Bytes, Header, TxHash, U256};

use op_reth::cli::{blocks, db};

const BLOCKS_PATH: &str = "data/export_0_4061224";

#[test]
fn test_blocks_from_file() {
    let blocks = blocks::read_blocks(BLOCKS_PATH).unwrap();
    let unsealed_first_header = blocks[0].header.clone().unseal();
    assert_eq!(
        TxHash::from_str("0x7334ddc1f6beaf66892c25cffdecec275cdfabaf4def047f0c3ce20e6f6483e8")
            .unwrap(),
        unsealed_first_header.parent_hash
    );
    assert_eq!(
        TxHash::from_str("0x7334ddc1f6beaf66892c25cffdecec275cdfabaf4def047f0c3ce20e6f6483e8")
            .unwrap(),
        unsealed_first_header.ommers_hash
    );

    assert_eq!(0, blocks[0].body.len());
    assert_eq!(0, blocks[0].ommers.len());
}

#[tokio::test]
async fn test_read_write_blocks() {
    let db_path = PathBuf::from("temp-blocks");
    let mut db = db::open_rw_env(db_path.as_path()).unwrap();
    blocks::apply(&mut db, Some(BLOCKS_PATH)).await.unwrap();

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
