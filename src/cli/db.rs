use eyre::Result;
use reth_db::mdbx::{Env, EnvKind, WriteMap};
use std::path::Path;

/// Helper that opens a read/write MDBX db at the given path
pub fn open_rw_env(path: &Path) -> Result<Env<WriteMap>> {
    Env::open(path, EnvKind::RW).map_err(|e| eyre::eyre!(e))
}
