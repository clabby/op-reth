use eyre::Result;

use crate::cli::db;

/// Run the op-reth node
pub async fn run() -> Result<()> {
    let _db = db::construct().await?;

    Ok(())
}
