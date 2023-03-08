use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::cli::db;
use bytes::BytesMut;
use clap::Parser;
use eyre::Result;
use reth::runner::CliContext;
use reth_db::{
    database::Database,
    mdbx::{Env, WriteMap},
    tables,
    transaction::DbTxMut,
};
use reth_primitives::{
    keccak256,
    proofs::{KeccakHasher, EMPTY_ROOT},
    Account, Address, Bytes, StorageEntry, H256, KECCAK_EMPTY, U256,
};
use reth_rlp::{Encodable, Header};
use triehash::sec_trie_root;

/// State command
#[derive(Debug, Parser)]
pub struct Command {
    /// The path to the state export
    #[arg(
        long,
        value_name = "STATE",
        verbatim_doc_comment,
        default_value = "data/alloc_everything_4061224_final.json"
    )]
    path: String,
    /// The path to the database
    #[arg(long, value_name = "DATABASE_PATH", verbatim_doc_comment)]
    database: String,
}

/// Apply world state to the given database
pub async fn apply(db: &mut Env<WriteMap>, path: Option<&str>) -> Result<()> {
    let file_path = path.unwrap_or("data/alloc_everything_4061224_final.json");
    let state = from_file(file_path)?;
    db.create_tables()?;
    db.update(|tx| {
        for (address, account) in &state {
            // Insert account
            let plain_account = Account {
                nonce: account.nonce.unwrap_or(0),
                balance: account.balance,
                bytecode_hash: account.code_hash,
            };
            tx.put::<tables::PlainAccountState>(*address, plain_account).unwrap();

            // Insert storage
            if let Some(storage) = &account.storage {
                for (key, value) in storage {
                    let storage_entry = StorageEntry { key: *key, value: *value };
                    tx.put::<tables::PlainStorageState>(*address, storage_entry).unwrap();
                }
            }

            // Insert bytecode
            if let Some(hash) = account.code_hash {
                let bytecode = if let Some(code) = &account.code {
                    Bytes::from(hex::decode(code).unwrap_or(vec![]))
                } else {
                    Bytes::from(vec![])
                };
                tx.put::<tables::Bytecodes>(hash, bytecode.to_vec()).unwrap();
            }
        }
    })?;
    Ok(())
}

impl Command {
    /// Execute the command
    pub async fn execute(self, _ctx: CliContext) -> Result<()> {
        let db_path = PathBuf::from(self.database);
        let mut db = db::open_rw_env(db_path.as_path())?;
        apply(&mut db, Some(&self.path)).await
    }

    /// Extract a portion of the state
    pub async fn export(&self, max: usize) -> eyre::Result<()> {
        let raw_data = std::fs::read(&self.path)?;
        let read_value = serde_json::from_slice::<serde_json::Value>(&raw_data)?;
        let mut ten_values = Vec::new();
        if let serde_json::Value::Object(map) = read_value {
            for (count, (key, value)) in map.into_iter().enumerate() {
                if count < max {
                    ten_values.push((key, value));
                } else {
                    break
                }
            }
        }
        let file = std::fs::File::create("temp_out.json")?;
        serde_json::to_writer(file, &ten_values)?;
        tracing::info!(target: "reth::cli", "Wrote to file temp_out.json");
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedAccount {
    pub balance: U256,
    pub code_hash: Option<H256>,
    pub code: Option<String>,
    pub nonce: Option<u64>,
    pub root: Option<H256>,
    pub storage: Option<HashMap<H256, U256>>,
}

/// ## State
///
/// The world state trie is a key-value store that maps addresses to accounts.
pub type State = HashMap<Address, ExportedAccount>;

pub fn exported_account_payload_len(ea: &ExportedAccount) -> usize {
    let mut len = 0;
    len += ea.nonce.unwrap_or_default().length();
    len += ea.balance.length();
    len += EMPTY_ROOT.length();
    len += ea.code_hash.as_ref().map_or(KECCAK_EMPTY, keccak256).length();
    len
}

pub fn encode_exported_account(ea: &ExportedAccount, out: &mut dyn bytes::BufMut) {
    let header = Header { list: true, payload_length: exported_account_payload_len(ea) };
    header.encode(out);
    ea.nonce.unwrap_or_default().encode(out);
    ea.balance.encode(out);
    ea.root.unwrap_or(EMPTY_ROOT).encode(out);
    ea.code_hash.as_ref().map_or(KECCAK_EMPTY, keccak256).encode(out);
}

/// Decodes the world state from a json file
pub fn from_file(path: impl AsRef<Path>) -> Result<State> {
    let data = std::fs::read(&path)?;
    let state = serde_json::from_slice::<State>(&data)?;
    Ok(state)
}

/// Calculate the state root hash
pub fn state_root_hash(state: &State) -> Result<H256> {
    let accounts = state.iter().map(|(address, account)| {
        let mut acc_rlp = BytesMut::new();
        encode_exported_account(account, &mut acc_rlp);
        (address, Bytes::from(acc_rlp.freeze()))
    });
    Ok(H256(sec_trie_root::<KeccakHasher, _, _, _>(accounts).0))
}
