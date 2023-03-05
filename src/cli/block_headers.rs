use crate::cli::db;
use clap::{crate_version, Parser};
use reth::runner::CliContext;
use reth_db::{database::Database, tables, transaction::DbTxMut};
use reth_primitives::{
    rpc::{Bloom, H160, H256},
    rpc_utils::rlp::{Decodable, Rlp},
    Bytes, Header, U256,
};
use serde::Serialize;
use std::{fs, path::PathBuf};

#[derive(Debug, Serialize)]
pub struct ErigonBlock {
    pub header: ErigonHeader,
    pub txs: Vec<LegacyTx>,
    pub uncles: Vec<ErigonHeader>,
}

impl Decodable for ErigonBlock {
    fn decode(rlp: &Rlp) -> Result<Self, reth_primitives::rpc_utils::rlp::DecoderError> {
        let header: ErigonHeader = rlp.val_at(0)?;

        // TODO: This is cursed, clean up
        let mut iter = rlp.iter();
        iter.next(); // skip over the header
        let txs: Vec<LegacyTx> =
            iter.next().unwrap().iter().map(|rlp| Decodable::decode(&rlp).unwrap()).collect();

        let uncles: Vec<ErigonHeader> = rlp.list_at(2)?;

        Ok(Self { header, uncles, txs })
    }
}

/// A clone of Erigon's block header type
#[derive(Debug, Serialize)]
pub struct ErigonHeader {
    pub parent_hash: H256,
    pub uncle_hash: H256,
    pub coinbase: H160,
    pub state_root: H256,
    pub tx_hash: H256,
    pub receipts_root: H256,
    pub logs_bloom: Bloom,
    pub difficulty: U256,
    pub number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub mix_hash: H256,
    pub block_nonce: Vec<u8>,
}

/// Decodable trait implementation for Header
impl Decodable for ErigonHeader {
    fn decode(rlp: &Rlp) -> Result<Self, reth_primitives::rpc_utils::rlp::DecoderError> {
        let parent_hash = rlp.val_at(0)?;
        let uncle_hash = rlp.val_at(1)?;
        let coinbase = rlp.val_at(2)?;
        let state_root = rlp.val_at(3)?;
        let tx_hash = rlp.val_at(4)?;
        let receipts_root = rlp.val_at(5)?;
        let logs_bloom = rlp.val_at(6)?;
        let difficulty = rlp.val_at(7)?;
        let number = rlp.val_at(8)?;
        let gas_limit = rlp.val_at(9)?;
        let gas_used = rlp.val_at(10)?;
        let timestamp = rlp.val_at(11)?;
        let extra_data = rlp.val_at(12)?;
        let mix_hash = rlp.val_at(13)?;
        let block_nonce = rlp.list_at(14)?;

        Ok(Self {
            parent_hash,
            uncle_hash,
            coinbase,
            state_root,
            tx_hash,
            receipts_root,
            logs_bloom,
            difficulty,
            number,
            gas_limit,
            gas_used,
            timestamp,
            extra_data,
            mix_hash,
            block_nonce,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct LegacyTx {
    pub nonce: U256,
    pub gas_price: U256,
    pub gas: U256,
    pub to: Vec<u8>,
    pub value: U256,
    pub data: Vec<u8>,
    pub v: U256,
    pub r: U256,
    pub s: U256,
}

impl Decodable for LegacyTx {
    fn decode(rlp: &Rlp) -> Result<Self, reth_primitives::rpc_utils::rlp::DecoderError> {
        let nonce = rlp.val_at(0)?;
        let gas_price = rlp.val_at(1)?;
        let gas = rlp.val_at(2)?;
        let to = rlp.val_at(3)?;
        let value = rlp.val_at(4)?;
        let data = rlp.val_at(5)?;
        let v = rlp.val_at(6)?;
        let r = rlp.val_at(7)?;
        let s = rlp.val_at(8)?;

        Ok(Self { nonce, gas_price, gas, to, value, data, v, r, s })
    }
}

/// Genesis command
#[derive(Debug, Parser)]
pub struct Command {
    /// The path to the block header dump file
    #[arg(
        long,
        value_name = "BLOCK_HEADERS",
        verbatim_doc_comment,
        default_value = "headers_export"
    )]
    path: String,

    /// The path to the database
    #[arg(long, value_name = "DATABASE_PATH", verbatim_doc_comment)]
    database: String,
}

impl Command {
    /// Execute `node` command
    pub async fn execute(self, _ctx: CliContext) -> eyre::Result<()> {
        tracing::debug!(target: "reth::cli", "loading block_headers file {}", crate_version!());

        // Load the genesis file from the specified path
        let contents = fs::read(self.path)?;

        // Create a new Rlp stream from the block header dump file
        let rlp = Rlp::new(&contents);

        tracing::debug!(target: "reth::cli", "loaded block headers file");

        // Iterate over the Rlp stream
        let mut iter = rlp.iter();

        // Decode the block headers and store them on the heap
        let mut headers: Vec<Header> = Vec::with_capacity(4_061_227);
        while let Some(block) = iter.next() {
            let erigon_block: Result<ErigonBlock, _> = Decodable::decode(&block);
            if let Ok(erigon_block) = erigon_block {
                headers.push(Header {
                    parent_hash: reth_primitives::H256::from_slice(
                        &erigon_block.header.parent_hash.0,
                    ),
                    ommers_hash: reth_primitives::H256::from_slice(
                        &erigon_block.header.uncle_hash.0,
                    ),
                    beneficiary: reth_primitives::H160::from_slice(&erigon_block.header.coinbase.0),
                    state_root: reth_primitives::H256::from_slice(
                        &erigon_block.header.state_root.0,
                    ),
                    transactions_root: reth_primitives::H256::from_slice(
                        &erigon_block.header.tx_hash.0,
                    ),
                    receipts_root: reth_primitives::H256::from_slice(
                        &erigon_block.header.receipts_root.0,
                    ),
                    withdrawals_root: None,
                    logs_bloom: reth_primitives::Bloom::from_slice(
                        &erigon_block.header.logs_bloom.0,
                    ),
                    difficulty: erigon_block.header.difficulty,
                    number: erigon_block.header.number,
                    gas_limit: erigon_block.header.gas_limit,
                    gas_used: erigon_block.header.gas_used,
                    timestamp: erigon_block.header.timestamp,
                    mix_hash: reth_primitives::H256::from_slice(&erigon_block.header.mix_hash.0),
                    nonce: reth_primitives::U64::from_little_endian(
                        &erigon_block.header.block_nonce.as_slice(),
                    )
                    .as_u64(),
                    base_fee_per_gas: None,
                    extra_data: Bytes::from(erigon_block.header.extra_data),
                });
            }
        }

        tracing::debug!(target: "reth::cli", "block_headers file decoded, opening DB");

        // Open the database at the given path
        let db_path = PathBuf::from(self.database);
        let db = db::open_rw_env(db_path.as_path())?;

        // Create the tables for the db (if necessary)
        tracing::debug!(target: "reth::cli", "DB opened. Creating tables (if they're not present)");
        db.create_tables()?;

        tracing::debug!(target: "reth::cli", "Inserting block headers");

        // Insert all block headers into MDBX
        db.update(|tx| {
            for header in &headers {
                tx.put::<tables::Headers>(header.number, header.clone()).unwrap();
                let block_hash = header.hash_slow();
                tx.put::<tables::CanonicalHeaders>(header.number, block_hash).unwrap();
                tx.put::<tables::HeaderNumbers>(block_hash, header.number).unwrap();
            }
        })?;

        tracing::debug!(target: "reth::cli", "Block headers inserted!");

        // Sanity checking
        // db.view(|tx| {
        //     let headers_cursor = tx.cursor_read::<tables::Headers>().unwrap();
        //     let header = tx.get::<tables::Headers>(1);
        //     dbg!(header);
        // });

        Ok(())
    }
}
