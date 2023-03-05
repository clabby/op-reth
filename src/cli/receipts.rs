use std::path::{Path, PathBuf};

use clap::Parser;
use eyre::Result;
use reth::runner::CliContext;
use reth_db::{
    database::Database,
    mdbx::{Env, WriteMap},
    Error,
};
use reth_primitives::{rpc::H256, U256};
use rlp::Decodable;
use serde::{Deserialize, Serialize};

use super::db;

/// Receipts command
#[derive(Debug, Parser)]
pub struct Command {
    /// The path to the receipts export
    #[arg(
        long,
        value_name = "RECEIPTS",
        verbatim_doc_comment,
        default_value = "data/export_receipt_0_4061223"
    )]
    path: String,
    /// The path to the database
    #[arg(long, value_name = "DATABASE_PATH", verbatim_doc_comment)]
    database: String,
}

/// Apply receipts to the given database
pub async fn apply(db: &mut Env<WriteMap>, path: Option<&str>) -> Result<()> {
    let _receipts = Receipt::from_file(path.unwrap_or("data/export_receipt_0_4061223"))?;
    db.create_tables()?;
    match db.update(|_tx| {
        // TODO: apply receipts to db
        Ok::<(), Error>(())
    })? {
        Ok(_) => tracing::info!(target: "reth::cli", "Receipts inserted! 🎉"),
        Err(err) => {
            tracing::error!(target: "reth::cli", "Error inserting receipts into MDBX: {}", err)
        }
    }
    Ok(())
}

impl Command {
    /// Execute the command
    pub async fn execute(self, _ctx: CliContext) -> Result<()> {
        let db_path = PathBuf::from(self.database);
        let mut db = db::open_rw_env(db_path.as_path())?;
        apply(&mut db, Some(&self.path)).await
    }
}

/// ## Receipt
///
/// This is a receipt types based on the [HackReceipt](https://github.com/testinprod-io/erigon/blob/pcw109550/state-import/core/types/receipt.go#L81)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    /// The receipt type
    #[serde(rename = "type")]
    pub ty: u8,
    /// The post state root
    #[serde(rename = "root")]
    pub post_state: Vec<u8>,
    /// The tx receipt status
    pub status: u64,
    /// The cumulative gas used
    #[serde(rename = "cumulativeGasUsed")]
    pub cumulative_gas_used: u64,
    /// The bloom filter
    #[serde(rename = "logsBloom")]
    pub bloom: Vec<u8>,
    /// Receipt logs
    pub logs: Vec<u8>,
    /// The transaction hash
    #[serde(rename = "transactionHash")]
    pub tx_hash: H256,
    /// The contract address
    #[serde(rename = "contractAddress")]
    pub contract_address: String,
    /// The gas used
    #[serde(rename = "gasUsed")]
    pub gas_used: u64,
    /// The block hash
    #[serde(rename = "blockHash")]
    pub block_hash: H256,
    /// The block number
    #[serde(rename = "blockNumber")]
    pub block_number: U256,
    /// The transaction index
    #[serde(rename = "transactionIndex")]
    pub transaction_index: u64,
    /// The L1 gas price
    #[serde(rename = "l1GasPrice")]
    pub l1_gas_price: U256,
    /// The L1 gas used
    #[serde(rename = "l1GasUsed")]
    pub l1_gas_used: U256,
    /// The L1 fee
    #[serde(rename = "l1Fee")]
    pub l1_fee: U256,
    /// The L1 fee scalar
    #[serde(rename = "l1FeeScalar")]
    pub l1_fee_scalar: String,
}

impl rlp::Decodable for Receipt {
    fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
        let ty = rlp.val_at(0)?;
        let post_state = rlp.val_at(1)?;
        let status = rlp.val_at(2)?;
        let cumulative_gas_used = rlp.val_at(3)?;
        let bloom = rlp.val_at(4)?;
        let logs = rlp.at(5)?.as_raw();
        let tx_hash = rlp.val_at(6)?;
        let contract_address = rlp.val_at(7)?;
        let gas_used = rlp.val_at(8)?;
        let block_hash = rlp.val_at(9)?;
        let block_number = rlp.val_at(10)?;
        let transaction_index = rlp.val_at(11)?;
        let l1_gas_price = rlp.val_at(12)?;
        let l1_gas_used = rlp.val_at(13)?;
        let l1_fee = rlp.val_at(14)?;
        let l1_fee_scalar = rlp.val_at(15)?;

        let r = Receipt {
            ty,
            post_state,
            status,
            cumulative_gas_used,
            bloom,
            logs: logs.to_vec(),
            tx_hash,
            contract_address,
            gas_used,
            block_hash,
            block_number,
            transaction_index,
            l1_gas_price,
            l1_gas_used,
            l1_fee,
            l1_fee_scalar,
        };
        Ok(r)
    }
}

impl Receipt {
    fn decode_receipt_vec(rlp: &rlp::Rlp) -> Result<Vec<Receipt>, rlp::DecoderError> {
        let mut receipts = Vec::new();
        for (_, item) in rlp.iter().enumerate() {
            if item.is_empty() {
                continue
            }
            let r = if let Ok(r) = Receipt::decode(&item) {
                r
            } else {
                let mut inner_vec = Receipt::decode_receipt_vec(&item)?;
                receipts.append(&mut inner_vec);
                continue
            };
            receipts.push(r);
        }
        Ok(receipts)
    }

    /// Decodes receipts from an rlp-encoded list of receipts file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Vec<Receipt>> {
        let data = std::fs::read(&path)?;
        let rlp_data = rlp::Rlp::new(&data[1..]);
        if rlp_data.is_empty() {
            tracing::warn!(target: "reth::cli", "rlp data is empty!");
        }
        if rlp_data.is_null() {
            tracing::warn!(target: "reth::cli", "rlp data is null!");
        }
        if rlp_data.is_list() {
            tracing::debug!(target: "reth::cli", "decoding rlp data as list");
        }
        let receipts = Receipt::decode_receipt_vec(&rlp_data).map_err(|e| eyre::eyre!(e))?;
        Ok(receipts)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    // use reth_db::{tables, transaction::DbTx};
    // use reth_primitives::{Bytes, StorageEntry, H160};

    use super::*;

    const RECEIPTS_PATH: &str = "data/export_receipt_0_4061223";

    #[test]
    fn test_receipts_from_file() {
        let receipts = Receipt::from_file(RECEIPTS_PATH).unwrap();
        assert_eq!(0, receipts[0].ty);
        assert_eq!(1, receipts[0].status);
        assert_eq!(151191, receipts[0].cumulative_gas_used);
        assert_eq!(H256::from_str("0x7334ddc1f6beaf66892c25cffdecec275cdfabaf4def047f0c3ce20e6f6483e8").unwrap(), receipts[0].tx_hash);
        assert_eq!(151191, receipts[0].gas_used);
        assert_eq!(H256::from_str("0x15d55041e8f7b0d1f303b6d4cefe2d2efc257d67acd9f17307261a8f7d786e0e").unwrap(), receipts[0].block_hash);
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
        apply(&mut db, Some(RECEIPTS_PATH)).await.unwrap();

        std::fs::remove_dir_all(db_path).unwrap();

        // Read receipts
        // let tx = db.tx().unwrap();
        // let address = H160::from_str("0x4200000000000000000000000000000000000011").unwrap();
        // let account = tx.get::<tables::PlainAccountState>(address).unwrap();
        // assert_eq!(
        //     Some(
        //         H256::from_str(
        //             "0x8b846c7bbf2a0a4e6d36d5b9fd759f8fd1d2887a1b6732460e86436c8dcefc4d"
        //         )
        //         .unwrap()
        //     ),
        //     account.unwrap().bytecode_hash
        // );
        // assert_eq!(U256::ZERO, account.unwrap().balance);
        // assert_eq!(0, account.unwrap().nonce);
    }
}
