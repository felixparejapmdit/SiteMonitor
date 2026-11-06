//! SiteMonitor API server.
//!
//! ```text
//! backend            # migrate, seed if empty, start crons, serve on PORT
//! backend reset      # drop everything, re-create the schema and re-seed
//! ```

mod convert;
mod entities;
mod error;
mod migrations;
mod routes;
mod seed;
mod services;
mod state;
mod ws;

use std::net::SocketAddr;
use std::path::PathBuf;

use axum::http::{header, HeaderValue};
use axum::routing::get;
use axum::Router;
use sea_orm::{ConnectOptions, Database};
use sea_orm_migration::MigratorTrait;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

use crate::migrations::Migrator;
use crate::state::AppState;

const DEFAULT_DATABASE_URL: &str = "postgres://sitemonitor:sitemonitor_dev@localhost:54329/sitemonitor";
const DEFAULT_PORT: u16 = 8737;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "backend=info,tower_http=warn,sea_orm=warn,sqlx=warn".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.into());
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(DEFAULT_PORT);
    let static_dir = PathBuf::from(std::env::var("STATIC_DIR").unwrap_or_else(|_| "target/dx/frontend/release/web/public".into()));
    let upload_dir = PathBuf::from(std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "data/uploads".into()));
    tokio::fs::create_dir_all(&upload_dir).await?;

    let mut opts = ConnectOptions::new(database_url);
    opts.max_connections(20).sqlx_logging(false);
    let db = Database::connect(opts).await?;

    let command = std::env::args().nth(1).unwrap_or_default();
    match command.as_str() {
        "reset" => {
            Migrator::fresh(&db).await?;
            seed::seed(&db).await?;
            tracing::info!("database reset and re-seeded");
            return Ok(());
        }
        "" | "serve" => {}
        other => anyhow::bail!("unknown command `{other}` (use `serve` or `reset`)"),
    }

    Migrator::up(&db, None).await?;
    if seed::seed_if_empty(&db).await? {
        tracing::info!("seeded a fresh estate");
    }

    let state = AppState { db, hub: ws::Hub::default(), upload_dir: upload_dir.clone() };
    let _scheduler = services::scheduler::start(state.clone()).await?;

    let index = static_dir.join("index.html");
    let spa = ServeDir::new(&static_dir).fallback(ServeFile::new(&index));

    let app = Router::new()
        .nest("/api", routes::api())
        .route("/ws", get(ws::handler))
        .nest_service(
            "/uploads",
            Router::new()
                .fallback_service(ServeDir::new(&upload_dir))
                .layer(SetResponseHeaderLayer::overriding(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                )),
        )
        .fallback_service(spa)
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("SiteMonitor is running at http://localhost:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
