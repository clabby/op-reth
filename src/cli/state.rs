use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::cli::db;
use bytes::BytesMut;
use clap::Parser;
use eyre::Result;
use reth::runner::CliContext;
use reth_db::{database::Database, tables, transaction::DbTxMut};
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

impl Command {
    /// Execute the command
    pub async fn execute(self, _ctx: CliContext) -> eyre::Result<()> {
        // Load the state
        tracing::info!(target: "reth::cli", "Loading State from \"{}\"", self.path);
        let state = from_file(&self.path)?;
        tracing::info!(target: "reth::cli", "World State Import Complete");

        // Open the database at the given path
        let db_path = PathBuf::from(self.database);
        let db = db::open_rw_env(db_path.as_path())?;

        // Create the tables for the db (if necessary)
        tracing::debug!(target: "reth::cli", "DB opened. Creating tables if not present");
        db.create_tables()?;

        // Insert world state into MDBX
        tracing::debug!(target: "reth::cli", "Inserting block world state into MDBX");
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
        tracing::debug!(target: "reth::cli", "World State inserted!");

        Ok(())
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

#[cfg(test)]
mod tests {
    // use std::str::FromStr;

    // use reth_primitives::{hex, Bytes, U256, GenesisAccount};

    // use super::*;

    // #[test]
    // fn test_mem_storage_hash() {
    //     let expected_storage_hash =
    //         "0x69fd5ad96fb5412504f44ab16140d2f0d35910fa9630cba6b9c68f277592248a";
    //     let account = GenesisAccount {
    //         nonce: Some(0),
    //         balance:
    // U256::from_str("0000000000000000000000000000000000000000009667929385620327791008").unwrap(),
    //         code: Some(Bytes::from(
    //             
    // hex::decode("
    // 6080604052600436106100bc5760003560e01c8063313ce56711610074578063a9059cbb1161004e578063a9059cbb146102cb578063d0e30db0146100bc578063dd62ed3e14610311576100bc565b8063313ce5671461024b57806370a082311461027657806395d89b41146102b6576100bc565b806318160ddd116100a557806318160ddd146101aa57806323b872dd146101d15780632e1a7d4d14610221576100bc565b806306fdde03146100c6578063095ea7b314610150575b6100c4610359565b005b3480156100d257600080fd5b506100db6103a8565b6040805160208082528351818301528351919283929083019185019080838360005b838110156101155781810151838201526020016100fd565b50505050905090810190601f1680156101425780820380516001836020036101000a031916815260200191505b509250505060405180910390f35b34801561015c57600080fd5b506101966004803603604081101561017357600080fd5b5073ffffffffffffffffffffffffffffffffffffffff8135169060200135610454565b604080519115158252519081900360200190f35b3480156101b657600080fd5b506101bf6104c7565b60408051918252519081900360200190f35b3480156101dd57600080fd5b50610196600480360360608110156101f457600080fd5b5073ffffffffffffffffffffffffffffffffffffffff8135811691602081013590911690604001356104cb565b34801561022d57600080fd5b506100c46004803603602081101561024457600080fd5b503561066b565b34801561025757600080fd5b50610260610700565b6040805160ff9092168252519081900360200190f35b34801561028257600080fd5b506101bf6004803603602081101561029957600080fd5b503573ffffffffffffffffffffffffffffffffffffffff16610709565b3480156102c257600080fd5b506100db61071b565b3480156102d757600080fd5b50610196600480360360408110156102ee57600080fd5b5073ffffffffffffffffffffffffffffffffffffffff8135169060200135610793565b34801561031d57600080fd5b506101bf6004803603604081101561033457600080fd5b5073ffffffffffffffffffffffffffffffffffffffff813581169160200135166107a7565b33600081815260036020908152604091829020805434908101909155825190815291517fe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c9281900390910190a2565b6000805460408051602060026001851615610100027fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff0190941693909304601f8101849004840282018401909252818152929183018282801561044c5780601f106104215761010080835404028352916020019161044c565b820191906000526020600020905b81548152906001019060200180831161042f57829003601f168201915b505050505081565b33600081815260046020908152604080832073ffffffffffffffffffffffffffffffffffffffff8716808552908352818420869055815186815291519394909390927f8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925928290030190a350600192915050565b4790565b73ffffffffffffffffffffffffffffffffffffffff83166000908152600360205260408120548211156104fd57600080fd5b73ffffffffffffffffffffffffffffffffffffffff84163314801590610573575073ffffffffffffffffffffffffffffffffffffffff841660009081526004602090815260408083203384529091529020547fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff14155b156105ed5773ffffffffffffffffffffffffffffffffffffffff841660009081526004602090815260408083203384529091529020548211156105b557600080fd5b73ffffffffffffffffffffffffffffffffffffffff841660009081526004602090815260408083203384529091529020805483900390555b73ffffffffffffffffffffffffffffffffffffffff808516600081815260036020908152604080832080548890039055938716808352918490208054870190558351868152935191937fddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef929081900390910190a35060019392505050565b3360009081526003602052604090205481111561068757600080fd5b33600081815260036020526040808220805485900390555183156108fc0291849190818181858888f193505050501580156106c6573d6000803e3d6000fd5b5060408051828152905133917f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65919081900360200190a250565b60025460ff1681565b60036020526000908152604090205481565b60018054604080516020600284861615610100027fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff0190941693909304601f8101849004840282018401909252818152929183018282801561044c5780601f106104215761010080835404028352916020019161044c565b60006107a03384846104cb565b9392505050565b60046020908152600092835260408084209091529082529020548156fea265627a7a7231582091c18790e0cca5011d2518024840ee00fecc67e11f56fd746f2cf84d5b583e0064736f6c63430005110032"
    // ).unwrap()         )),
    //         storage: None
    //     };
    //     println!("Got genesis account for address: {:x?}", account);
    //     let storage = account.storage.unwrap_or_default();
    //     println!("Default storage: {:x?}", storage);
    //     let storage_root = storage_root_hash(storage).unwrap();
    //     assert_eq!(expected_storage_hash, format!("{:#x}", storage_root));
    // }

    // #[test]
    // fn test_storage_hash() {
    //     let expected_storage_hash =
    //         "0x69fd5ad96fb5412504f44ab16140d2f0d35910fa9630cba6b9c68f277592248a";
    //     let account = "0x4200000000000000000000000000000000000006";
    //     let address = Address::from_str(account).unwrap();
    //     println!("Using address: {}", address);
    //     let state = from_file("data/alloc_everything_4061224_final.json").unwrap();
    //     let account = state.get(&address).unwrap();
    //     println!("Got genesis account for address: {:x?}", account);
    //     let storage = account.root.clone().unwrap();
    //     // calculate the storage root hash
    //     let storage_root = storage_root_hash(storage).unwrap();
    //     assert_eq!(expected_storage_hash, format!("{:#x}", storage_root));
    // }
}
