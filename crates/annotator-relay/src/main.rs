//! `annotator-relay` daemon entrypoint.

use annotator_relay::{router, RelayState, SessionStore};
use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "annotator-relay",
    version,
    about = "PlausiDen-Annotator local relay daemon. Accepts JSON sessions from the bookmarklet, persists to disk."
)]
struct Args {
    /// Listen address. Defaults to 127.0.0.1:8788 (operator-local).
    #[arg(long, default_value = "127.0.0.1:8788", env = "ANNOTATOR_RELAY_BIND")]
    bind: String,

    /// Storage root for persisted sessions. Created if missing.
    #[arg(long, env = "ANNOTATOR_RELAY_STORE")]
    store_root: PathBuf,
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .init();

    let args = Args::parse();

    let store = match SessionStore::open(&args.store_root) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("annotator-relay: store open failed: {e}");
            return ExitCode::from(2);
        }
    };
    let app = router(RelayState::new(store));

    let listener = match tokio::net::TcpListener::bind(&args.bind).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("annotator-relay: bind {} failed: {e}", args.bind);
            return ExitCode::from(2);
        }
    };
    tracing::info!(target: "annotator-relay", bind = %args.bind, store = %args.store_root.display(), "listening");

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("annotator-relay: serve failed: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
