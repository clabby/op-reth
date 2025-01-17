use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use clap::Parser;
use eyre::Result;
use reth::runner::CliContext;
use reth_db::{database::Database, mdbx::WriteMap, tables, transaction::DbTxMut};
use reth_primitives::{
    keccak256, Account as RethAccount, Address, Bytes, Header, SealedBlock, SealedHeader,
    StorageEntry, H256, U256,
};
use serde::{Deserialize, Serialize};

use crate::cli::db;

/// Genesis command
#[derive(Debug, Parser)]
pub struct Command {
    /// The path to the genesis file
    #[arg(long, value_name = "GENESIS", verbatim_doc_comment, default_value = "genesis.json")]
    path: String,
    /// The path to the database
    #[arg(long, value_name = "DATABASE_PATH", verbatim_doc_comment)]
    database: String,
}

/// Apply genesis state to the given database
pub async fn apply(db: &mut reth_db::mdbx::Env<WriteMap>, path: Option<&str>) -> Result<()> {
    let genesis = Genesis::from_file(path.unwrap_or("data/genesis.json"))?;
    db.create_tables()?;
    db.update(|tx| {
        let genesis_header: Header = genesis.to_header();
        let header: SealedHeader = genesis_header.seal_slow();
        let genesis_block = SealedBlock { header, body: vec![], ommers: vec![], withdrawals: None };
        let _ = reth_provider::insert_canonical_block(tx, &genesis_block, false);
    })?;

    db.update(|tx| {
        let _ = genesis.alloc.iter().try_for_each(|(address, account)| -> eyre::Result<()> {
            let has_code = !account.code.clone().unwrap_or_default().is_empty();
            let code_hash =
                if has_code { Some(keccak256(&account.code.clone().unwrap())) } else { None };
            tx.put::<tables::PlainAccountState>(
                *address,
                RethAccount {
                    balance: account.balance,
                    nonce: account.nonce.unwrap_or(0),
                    bytecode_hash: code_hash,
                },
            )?;
            if let Some(code_hash) = code_hash {
                tx.put::<tables::Bytecodes>(
                    code_hash,
                    account.code.clone().unwrap_or_default().to_vec(),
                )?;
            }
            if let Some(s) = &account.storage {
                s.iter().try_for_each(|(k, v)| {
                    tx.put::<tables::PlainStorageState>(
                        *address,
                        StorageEntry {
                            key: H256::from_slice(&k.to_fixed_bytes()),
                            value: U256::from_be_bytes(v.0),
                        },
                    )
                })?;
            }

            Ok(())
        });
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
}

/// Optimism Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Optimism {
    #[serde(rename = "eip1559Elasticity")]
    pub eip1559_elasticity: u64,
    #[serde(rename = "eip1559Denominator")]
    pub eip1559_denominator: u64,
}

/// The genesis inner config object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    #[serde(rename = "ChainName")]
    pub chain_name: String,
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    #[serde(rename = "homesteadBlock")]
    pub homestead_block: u64,
    #[serde(rename = "eip150Block")]
    pub eip150_block: u64,
    #[serde(rename = "eip150Hash")]
    pub eip150_hash: String,
    #[serde(rename = "eip155Block")]
    pub eip155_block: u64,
    #[serde(rename = "eip158Block")]
    pub eip158_block: u64,
    #[serde(rename = "byzantiumBlock")]
    pub byzantium_block: u64,
    #[serde(rename = "constantinopleBlock")]
    pub constantinople_block: u64,
    #[serde(rename = "petersburgBlock")]
    pub petersburg_block: u64,
    #[serde(rename = "istanbulBlock")]
    pub istanbul_block: u64,
    #[serde(rename = "muirGlacierBlock")]
    pub muir_glacier_block: u64,
    #[serde(rename = "berlinBlock")]
    pub berlin_block: u64,
    #[serde(rename = "londonBlock")]
    pub london_block: u64,
    #[serde(rename = "arrowGlacierBlock")]
    pub arrow_glacier_block: u64,
    #[serde(rename = "grayGlacierBlock")]
    pub gray_glacier_block: u64,
    #[serde(rename = "mergeNetsplitBlock")]
    pub merge_netsplit_block: u64,
    #[serde(rename = "bedrockBlock")]
    pub bedrock_block: u64,
    #[serde(rename = "terminalTotalDifficulty")]
    pub terminal_total_difficulty: u64,
    #[serde(rename = "terminalTotalDifficultyPassed")]
    pub terminal_total_difficulty_passed: bool,
    pub optimism: Optimism,
}

impl GenesisConfig {
    pub fn map(&self) -> HashMap<String, Vec<u8>> {
        let mut map = HashMap::new();
        let mut difficulty = vec![0u8];
        if self.terminal_total_difficulty_passed {
            difficulty = vec![1u8];
        }
        map.insert("ChainName".to_string(), self.chain_name.as_bytes().to_vec());
        map.insert("chainId".to_string(), self.chain_id.to_le_bytes().into());
        map.insert("homesteadBlock".to_string(), self.chain_id.to_le_bytes().into());
        map.insert("eip150Block".to_string(), self.eip150_block.to_le_bytes().into());
        map.insert("eip150Hash".to_string(), self.eip150_hash.as_bytes().to_vec());
        map.insert("eip155Block".to_string(), self.eip155_block.to_le_bytes().into());
        map.insert("eip158Block".to_string(), self.eip158_block.to_le_bytes().into());
        map.insert("byzantiumBlock".to_string(), self.byzantium_block.to_le_bytes().into());
        map.insert(
            "constantinopleBlock".to_string(),
            self.constantinople_block.to_le_bytes().into(),
        );
        map.insert("petersburgBlock".to_string(), self.petersburg_block.to_le_bytes().into());
        map.insert("istanbulBlock".to_string(), self.istanbul_block.to_le_bytes().into());
        map.insert("muirGlacierBlock".to_string(), self.muir_glacier_block.to_le_bytes().into());
        map.insert("berlinBlock".to_string(), self.berlin_block.to_le_bytes().into());
        map.insert("londonBlock".to_string(), self.london_block.to_le_bytes().into());
        map.insert("arrowGlacierBlock".to_string(), self.arrow_glacier_block.to_le_bytes().into());
        map.insert("grayGlacierBlock".to_string(), self.gray_glacier_block.to_le_bytes().into());
        map.insert(
            "mergeNetsplitBlock".to_string(),
            self.merge_netsplit_block.to_le_bytes().into(),
        );
        map.insert("bedrockBlock".to_string(), self.bedrock_block.to_le_bytes().into());
        map.insert(
            "terminalTotalDifficulty".to_string(),
            self.terminal_total_difficulty.to_le_bytes().into(),
        );
        map.insert("terminalTotalDifficultyPassed".to_string(), difficulty);
        map.insert(
            "eip1559Elasticity".to_string(),
            self.optimism.eip1559_elasticity.to_le_bytes().into(),
        );
        map.insert(
            "eip1559Denominator".to_string(),
            self.optimism.eip1559_denominator.to_le_bytes().into(),
        );
        map
    }
}

/// The genesis file object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genesis {
    pub config: GenesisConfig,
    pub difficulty: String,
    #[serde(rename = "gasLimit")]
    pub gas_limit: String,
    pub extradata: String,
    pub alloc: HashMap<Address, ErigonGenesisAccount>,
}

/// An Erigon Genesis Account
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErigonGenesisAccount {
    pub nonce: Option<u64>,
    pub balance: U256,
    pub code: Option<Bytes>,
    pub storage: Option<HashMap<H256, H256>>,
}

impl Genesis {
    pub fn to_header(&self) -> Header {
        Header {
            difficulty: self.difficulty.parse().unwrap(),
            gas_limit: self.gas_limit.parse().unwrap(),
            extra_data: reth_primitives::Bytes::from(
                hex::decode(self.extradata.strip_prefix("0x").unwrap_or(&self.extradata)).unwrap(),
            ),
            ..Default::default()
        }
    }
}

impl Genesis {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }
}
