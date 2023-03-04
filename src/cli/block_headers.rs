use clap::{crate_version, Parser};
use reth::runner::CliContext;
use reth_primitives::{
    rpc_utils::rlp::{Decodable, Rlp},
    U256,
};
use serde::Serialize;
use std::fs;

#[derive(Debug, Serialize)]
pub struct ErigonBlock {
    pub header: Header,
    pub txs: Vec<LegacyTx>,
    pub uncles: Vec<Header>,
}

impl Decodable for ErigonBlock {
    fn decode(rlp: &Rlp) -> Result<Self, reth_primitives::rpc_utils::rlp::DecoderError> {
        let header: Header = rlp.val_at(0)?;

        // TODO: This is cursed, clean up
        let mut iter = rlp.iter();
        iter.next(); // skip over the header
        let txs: Vec<LegacyTx> =
            iter.next().unwrap().iter().map(|rlp| Decodable::decode(&rlp).unwrap()).collect();

        let uncles: Vec<Header> = rlp.list_at(2)?;

        Ok(Self { header, uncles, txs })
    }
}

/// A clone of Erigon's block header type
#[derive(Debug, Serialize)]
pub struct Header {
    pub parent_hash: U256,
    pub uncle_hash: U256,
    pub coinbase: U256,
    pub state_root: U256,
    pub tx_hash: U256,
    pub receipts_root: U256,
    pub logs_bloom: Vec<u8>,
    pub difficulty: U256,
    pub number: U256,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub mix_hash: U256,
    pub block_nonce: Vec<u8>,
}

/// Decodable trait implementation for Header
impl Decodable for Header {
    fn decode(rlp: &Rlp) -> Result<Self, reth_primitives::rpc_utils::rlp::DecoderError> {
        let parent_hash = rlp.val_at(0)?;
        let uncle_hash = rlp.val_at(1)?;
        let coinbase = rlp.val_at(2)?;
        let state_root = rlp.val_at(3)?;
        let tx_hash = rlp.val_at(4)?;
        let receipts_root = rlp.val_at(5)?;
        let logs_bloom = rlp.list_at(6)?;
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
}

impl Command {
    /// Execute `node` command
    pub async fn execute(self, _ctx: CliContext) -> eyre::Result<()> {
        tracing::info!(target: "op-reth::block_headers", "loading block_headers file {}", crate_version!());

        // Load the genesis file from the specified path
        let contents = fs::read(self.path)?;

        // Create a new Rlp stream from the block header dump file
        let rlp = Rlp::new(&contents);

        // Iterate over the Rlp stream
        let mut iter = rlp.iter();

        // Decode the block headers and store them on the heap
        let mut headers: Vec<Header> = Vec::new();
        while let Some(header) = iter.next() {
            if let Ok(header) = Decodable::decode(&header) {
                headers.push(header);
            }
        }

        // TODO: Move the intermediate block header types into RETH primitives and insert them
        // into the RETH MDBX database

        tracing::debug!(target: "op-reth::block_headers", "block_headers file decoded");

        Ok(())
    }
}
