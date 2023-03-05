use futures::stream::FuturesUnordered;

use crate::cli::{genesis, block_headers, receipts, state};

/// Execute the full op-reth pipeline
pub async fn execute(ctx: &Context) -> eyre::Result<()> {
    let mut tasks = FuturesUnordered::new();

    let mut genesis = genesis::Command::new(ctx);
    let mut block_headers = block_headers::Command::new(ctx);
    let mut receipts = receipts::Command::new(ctx);
    let mut state = state::Command::new();

    tasks.push(genesis.execute());
    tasks.push(block_headers.execute());
    tasks.push(receipts.execute());
    tasks.push(state.execute());

    while let Some(result) = tasks.next().await {
        result?;
    }

    Ok(())
}
