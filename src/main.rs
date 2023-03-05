pub mod cli;

fn main() {
    register_shutdown_handler();
    if let Err(err) = cli::run() {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}

/// Registers a basic ctrl-c handler that will exit the process.
pub fn register_shutdown_handler() {
    ctrlc::set_handler(move || {
        tracing::info!(target: "reth::cli", "shutting down... received ctrl-c input");
        std::process::exit(0);
    })
    .expect("could not register shutdown handler");
    tracing::info!(target: "reth::cli", "registered shutdown handler, press ctrl-c to shutdown");
}
