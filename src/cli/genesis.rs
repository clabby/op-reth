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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use reth_db::{tables, transaction::DbTx};
    use reth_primitives::{Bytes, StorageEntry, H160};

    use super::*;

    #[test]
    fn test_from_file() {
        let genesis = Genesis::from_file("data/genesis.json").unwrap();
        assert_eq!(genesis.config.chain_id, 420);
        assert_eq!(genesis.difficulty, "1");
        assert_eq!(genesis.gas_limit, "15000000");
        assert_eq!(genesis.extradata.len(), 236);
        assert_eq!(genesis.alloc.len(), 10);
    }

    #[tokio::test]
    async fn test_write_read_genesis_db() {
        let db_path = PathBuf::from("op-reth-db");
        let mut db = db::open_rw_env(db_path.as_path()).unwrap();
        apply(&mut db, Some("data/genesis.json")).await.unwrap();

        // Read account from genesis state
        let tx = db.tx().unwrap();
        let address = H160::from_str("0x4200000000000000000000000000000000000011").unwrap();
        let account = tx.get::<tables::PlainAccountState>(address).unwrap();
        assert_eq!(
            Some(
                H256::from_str(
                    "0x8b846c7bbf2a0a4e6d36d5b9fd759f8fd1d2887a1b6732460e86436c8dcefc4d"
                )
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
    }
}
