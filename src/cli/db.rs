use std::path::{Path, PathBuf};

use eyre::Result;
use reth_db::mdbx::{Env, EnvKind, WriteMap};

use crate::cli::{blocks, genesis, receipts, state};

/// Helper that opens a read/write MDBX db at the given path
pub fn open_rw_env(path: &Path) -> Result<Env<WriteMap>> {
    Env::open(path, EnvKind::RW).map_err(|e| eyre::eyre!(e))
}

/// Construct the full op-reth database
pub async fn construct() -> eyre::Result<Env<WriteMap>> {
    // Create a database at a new location
    let db_path = PathBuf::from("op-reth-db");
    let mut db = open_rw_env(db_path.as_path())?;

    // Apply genesis state
    if let Err(e) = genesis::apply(&mut db, None).await {
        eprintln!("Error while applying genesis to mdbx: {}", e);
        return Err(e)
    }

    // Apply blocks
    if let Err(e) = blocks::apply(&mut db, None).await {
        eprintln!("Error while applying blocks to mdbx: {}", e);
        return Err(e)
    }

    // Apply receipts
    if let Err(e) = receipts::apply(&mut db, None).await {
        eprintln!("Error while applying receips to mdbx: {}", e);
        return Err(e)
    }

    // Apply state
    if let Err(e) = state::apply(&mut db, None).await {
        eprintln!("Error while applying state to mdbx: {}", e);
        return Err(e)
    }

    Ok(db)
}
