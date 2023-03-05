use crate::cli::db;
use clap::{crate_version, Parser};
use reth::runner::CliContext;
use reth_db::{database::Database, tables, transaction::DbTx};
use reth_primitives::{
    rpc::{Bloom, H160, H256},
    rpc_utils::rlp::{Decodable, Rlp},
    Bytes, Header, SealedBlock, Signature, Transaction, TransactionKind, TransactionSigned,
    TxLegacy, U256,
};
use serde::Serialize;
use std::{fs, path::PathBuf};

/// A clone of erigon's block type
#[derive(Debug, Serialize)]
pub struct ErigonBlock {
    pub header: ErigonHeader,
    pub txs: Vec<LegacyTx>,
    pub uncles: Vec<ErigonHeader>,
}

/// Convert an [ErigonBlock] to a [SealedBlock]
impl From<ErigonBlock> for SealedBlock {
    fn from(block: ErigonBlock) -> Self {
        let header = Header::from(block.header).seal_slow();
        let txs = block.txs.into_iter().map(|tx| TransactionSigned::from(tx)).collect();
        let uncles =
            block.uncles.into_iter().map(|header| Header::from(header).seal_slow()).collect();
        Self { header, body: txs, ommers: uncles, withdrawals: None }
    }
}

/// RLP decoder for [ErigonBlock]
impl Decodable for ErigonBlock {
    fn decode(rlp: &Rlp) -> Result<Self, reth_primitives::rpc_utils::rlp::DecoderError> {
        let header: ErigonHeader = rlp.val_at(0)?;
        let txs = rlp.at(1)?.iter().map(|rlp| Decodable::decode(&rlp).unwrap()).collect();
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

/// Convert an [ErigonHeader] to a [Header]
impl From<ErigonHeader> for Header {
    fn from(header: ErigonHeader) -> Self {
        Self {
            parent_hash: reth_primitives::H256::from_slice(&header.parent_hash.0),
            ommers_hash: reth_primitives::H256::from_slice(&header.uncle_hash.0),
            beneficiary: reth_primitives::H160::from_slice(&header.coinbase.0),
            state_root: reth_primitives::H256::from_slice(&header.state_root.0),
            transactions_root: reth_primitives::H256::from_slice(&header.tx_hash.0),
            receipts_root: reth_primitives::H256::from_slice(&header.receipts_root.0),
            withdrawals_root: None,
            logs_bloom: reth_primitives::Bloom::from_slice(&header.logs_bloom.0),
            difficulty: header.difficulty,
            number: header.number,
            gas_limit: header.gas_limit,
            gas_used: header.gas_used,
            timestamp: header.timestamp,
            mix_hash: reth_primitives::H256::from_slice(&header.mix_hash.0),
            nonce: reth_primitives::U64::from_little_endian(&header.block_nonce.as_slice())
                .as_u64(),
            base_fee_per_gas: None,
            extra_data: Bytes::from(header.extra_data),
        }
    }
}

/// RLP Decoder for [ErigonHeader]
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

/// A legacy Ethereum transaction
/// l2geth was pre-berlin, so it only uses the legacy transaction schema
#[derive(Debug, Serialize)]
pub struct LegacyTx {
    pub nonce: u64,
    pub gas_price: u128,
    pub gas: u64,
    pub to: Option<H160>,
    pub value: u128,
    pub data: Vec<u8>,
    pub v: U256,
    pub r: U256,
    pub s: U256,
}

/// Convert a [LegacyTx] to a [TransactionSigned]
impl From<LegacyTx> for TransactionSigned {
    fn from(tx: LegacyTx) -> Self {
        let unsigned_tx = Transaction::Legacy(TxLegacy {
            chain_id: None,
            nonce: tx.nonce.into(),
            gas_price: tx.gas_price,
            gas_limit: tx.gas,
            to: if let Some(to) = tx.to {
                TransactionKind::Call(to.into())
            } else {
                TransactionKind::Create
            },
            value: tx.value.into(),
            input: Bytes::from(tx.data),
        });

        let signature = Signature {
            r: tx.r.into(),
            s: tx.s.into(),
            // An odd v means that the odd y-parity of the signature is true.
            odd_y_parity: (tx.v % U256::from(2)) == U256::from(1),
        };

        TransactionSigned::from_transaction_and_signature(unsigned_tx, signature)
    }
}

/// RLP Decoder for [LegacyTx]
impl Decodable for LegacyTx {
    fn decode(rlp: &Rlp) -> Result<Self, reth_primitives::rpc_utils::rlp::DecoderError> {
        let nonce = rlp.val_at(0)?;
        let gas_price = rlp.val_at(1)?;
        let gas = rlp.val_at(2)?;
        let to = rlp.at(3)?;
        let to = if to.is_empty() { None } else { Some(Decodable::decode(&to)?) };
        let value = rlp.val_at(4)?;
        let data = rlp.val_at(5)?;
        let v = rlp.val_at(6)?;
        let r = rlp.val_at(7)?;
        let s = rlp.val_at(8)?;

        Ok(Self { nonce, gas_price, gas, to, value, data, v, r, s })
    }
}

/// Block command
#[derive(Debug, Parser)]
pub struct Command {
    /// The path to the block dump file
    #[arg(
        long,
        value_name = "BLOCK_DUMP_PATH",
        verbatim_doc_comment,
        default_value = "blocks_export"
    )]
    path: String,

    /// The path to the database
    #[arg(long, value_name = "DATABASE_PATH", verbatim_doc_comment)]
    database: String,
}

impl Command {
    /// Execute `node` command
    pub async fn execute(self, _ctx: CliContext) -> eyre::Result<()> {
        tracing::debug!(target: "reth::cli", "loading block dump file. CLI version: {}", crate_version!());

        // Load the block dump file from the specified path
        let contents = fs::read(self.path)?;

        // Create a new Rlp stream from the block dump file
        let rlp = Rlp::new(&contents);

        tracing::debug!(target: "reth::cli", "loaded block file");

        // Iterate over the Rlp stream
        let mut rlp_iter = rlp.iter();

        // Decode the block headers and store them on the heap
        // TODO: Clean
        let mut blocks: Vec<SealedBlock> = Vec::with_capacity(4_061_227);
        while let Some(block) = rlp_iter.next() {
            let erigon_block: Result<ErigonBlock, _> = Decodable::decode(&block);
            if let Ok(erigon_block) = erigon_block {
                blocks.push(erigon_block.into());
            }
        }

        tracing::debug!(target: "reth::cli", "block dump file decoded, opening DB");

        // Open the database at the given path
        let db_path = PathBuf::from(self.database);
        let db = db::open_rw_env(db_path.as_path())?;

        // Create the tables for the db (if necessary)
        // TODO: Remove
        tracing::debug!(target: "reth::cli", "DB opened. Creating tables (if they're not present)");
        db.create_tables()?;

        tracing::debug!(target: "reth::cli", "Inserting blocks");

        // Insert all block headers into MDBX
        match db.update(|tx| {
            // The following operation requires the genesis block to be present in the database
            if let Ok(None) = tx.get::<tables::Headers>(0) {
                eyre::bail!("Genesis block not found! Please insert it before using this command.");
            }

            dbg!(&blocks[0]);
            // TODO: Why is there no signature attached to the transaction within block #1?
            for sealed_block in &blocks[1..] {
                // TODO: Parent tx num transition
                // I think we just need the genesis block inserted first?

                // We have no block rewards pre-merge
                reth_provider::insert_canonical_block(tx, sealed_block, false).unwrap();
            }

            Ok(())
        })? {
            Ok(_) => tracing::info!(target: "reth::cli", "Blocks inserted! 🎉"),
            Err(err) => {
                tracing::error!(target: "reth::cli", "Error inserting blocks into DB: {}", err)
            }
        }

        Ok(())
    }
}
