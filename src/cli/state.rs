use std::{collections::HashMap, path::Path};

use clap::Parser;
use eyre::Result;
use reth::runner::CliContext;
use reth_primitives::{proofs::KeccakHasher, Address, GenesisAccount, H256};
use reth_rlp::Encodable;
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
}

impl Command {
    /// Execute the command
    pub async fn execute(self, _ctx: CliContext) -> eyre::Result<()> {
        tracing::info!(target: "reth::cli", "loading state file \"{}\"", self.path);
        let state = from_file(self.path)?;
        tracing::info!(target: "reth::cli", "completed state import");
        let state_root = state_root_hash(&state)?;
        println!("State root: {:#x}", state_root);
        println!("State root at block 4061224 0xbfe2b059bc76c33556870c292048f1d28c9d498462a02a3c7aadb6edf1c2d21c");
        tracing::info!(target: "reth::cli", "state root hash: {:#x}", state_root);
        Ok(())
    }
}

/// ## State
///
/// The world state trie is a key-value store that maps addresses to accounts.
pub type State = HashMap<Address, GenesisAccount>;

/// Decodes the world state from a json file
pub fn from_file(path: impl AsRef<Path>) -> Result<State> {
    let data = std::fs::read(&path)?;
    let state = serde_json::from_slice::<State>(&data)?;
    Ok(state)
}

/// Calculate the state root hash
pub fn state_root_hash(state: &State) -> Result<H256> {
    // Turn state into a vector of (address, account) tuples
    let tuples: Vec<(Address, Vec<u8>)> = state
        .iter()
        .map(|(address, account)| {
            let mut out = Vec::new();
            Encodable::encode(&account, &mut out);
            (*address, out)
        })
        .collect();
    let expected = H256(sec_trie_root::<KeccakHasher, _, _, _>(tuples).0);
    Ok(expected)
}

/// Calculate storage root hash
pub fn storage_root_hash(storage: HashMap<H256, H256>) -> Result<H256> {
    let tuples: Vec<(H256, H256)> = storage.iter().map(|(key, value)| (*key, *value)).collect();
    let expected = H256(sec_trie_root::<KeccakHasher, _, _, _>(tuples).0);
    Ok(expected)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use reth_primitives::hex;

    use super::*;

    #[test]
    fn test_state_root_hash() {
        let state = from_file("data/alloc_everything_4061224_final.json").unwrap();
        let state_root = state_root_hash(&state).unwrap();
        assert_eq!(
            state_root,
            H256::from_slice(
                &hex::decode("bfe2b059bc76c33556870c292048f1d28c9d498462a02a3c7aadb6edf1c2d21c")
                    .unwrap()
            )
        );
    }

    #[test]
    fn test_storage_hash() {
        let expected_storage_hash =
            "0x69fd5ad96fb5412504f44ab16140d2f0d35910fa9630cba6b9c68f277592248a";
        let account = "0x4200000000000000000000000000000000000006";
        let address = Address::from_str(account).unwrap();
        println!("Using address: {}", address);
        let state = from_file("data/alloc_everything_4061224_final.json").unwrap();
        let account = state.get(&address).unwrap();
        println!("Got genesis account for address: {:x?}", account);
        let storage = account.storage.clone().unwrap();
        // calculate the storage root hash
        let storage_root = storage_root_hash(storage).unwrap();
        assert_eq!(expected_storage_hash, format!("{:#x}", storage_root));
    }
}
