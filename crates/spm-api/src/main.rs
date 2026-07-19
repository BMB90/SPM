mod error;
mod platform_collectors;
mod routes;
mod state;

use std::net::SocketAddr;
use std::path::PathBuf;

use spm_storage::Storage;
use state::AppState;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();

    let db_path: PathBuf = std::env::var("SPM_DB_PATH").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("./data/spm.db"));
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let storage = Storage::open(&db_path)?;
    tracing::info!(path = %db_path.display(), "opened database");

    let state = AppState::new(storage);
    let app = routes::build_router(state).layer(CorsLayer::permissive()).layer(TraceLayer::new_for_http());

    let port: u16 = std::env::var("SPM_API_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(7878);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!(%addr, "starting SPM API server");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
