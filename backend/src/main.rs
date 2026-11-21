//! SiteMonitor API server.
//!
//! ```text
//! backend            # migrate, seed if empty, start crons, serve on PORT
//! backend reset      # drop everything, re-create the schema and re-seed
//! backend set-password <username> <password>   # recover an account
//! backend parity-check <service5.db>             # is everything in it held here?
//! ```

mod auth;
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
        // An empty estate with just the sign-in accounts (the default state).
        "reset" => {
            Migrator::fresh(&db).await?;
            seed::seed_empty(&db).await?;
            tracing::info!("database reset: sign-in accounts and an empty estate");
            return Ok(());
        }
        // The same accounts plus the sample estate (demos and the test suite).
        "reset-sample" => {
            Migrator::fresh(&db).await?;
            seed::seed(&db).await?;
            tracing::info!("database reset with the sample estate");
            return Ok(());
        }
        "set-password" => {
            let args: Vec<String> = std::env::args().skip(2).collect();
            let [username, password] = args.as_slice() else { anyhow::bail!("usage: backend set-password <username> <password>") };
            Migrator::up(&db, None).await?;
            auth::set_password_cli(&db, username, password).await?;
            println!("Password set for {username}.");
            return Ok(());
        }
        "audit-data" => {
            // Counts personnel data readable without the key (should all be 0).
            Migrator::up(&db, None).await?;
            println!("{}", services::workforce::plain_counts(&db).await?);
            return Ok(());
        }
        "parity-check" => {
            // Compares a Service 5 file with this database (names and counts only).
            let file = std::env::args().nth(2).ok_or_else(|| anyhow::anyhow!("usage: backend parity-check <service5.db or backup.json>"))?;
            Migrator::up(&db, None).await?;
            let key_file = PathBuf::from(std::env::var("DATA_KEY_FILE").unwrap_or_else(|_| "data/keys/personnel.key".into()));
            services::vault::init(&key_file)?;
            print!("{}", services::workforce::parity(&db, &std::fs::read(file)?).await?);
            return Ok(());
        }
        "" | "serve" => {}
        other => anyhow::bail!("unknown command `{other}` (use `serve` or `reset`)"),
    }

    Migrator::up(&db, None).await?;
    // Personnel data is encrypted at rest; the key lives outside the database (and outside Git).
    let key_file = PathBuf::from(std::env::var("DATA_KEY_FILE").unwrap_or_else(|_| "data/keys/personnel.key".into()));
    services::vault::init(&key_file)?;
    let sealed = services::workforce::seal_existing(&db).await?;
    if sealed > 0 {
        tracing::info!(records = sealed, "encrypted personnel data stored before encryption was on");
    }
    if seed::seed_if_empty(&db).await? {
        tracing::info!("seeded a fresh estate");
    }
    // Modules switched off in settings (e.g. the clinic) take their permissions with them.
    convert::CLINIC_ON.store(services::load_settings(&db).await?.clinic_enabled, std::sync::atomic::Ordering::Relaxed);
    auth::ensure_passwords(&db).await?;

    let state = AppState { db, hub: ws::Hub::default(), upload_dir: upload_dir.clone(), throttle: Default::default() };
    let _scheduler = services::scheduler::start(state.clone()).await?;
    tokio::spawn(routes::estate::backfill_tiles(state.clone()));

    let index = static_dir.join("index.html");
    let spa = ServeDir::new(&static_dir).fallback(ServeFile::new(&index));

    let app = Router::new()
        .nest("/api", routes::api(state.clone()))
        .route("/ws", get(ws::handler))
        .nest_service(
            "/uploads",
            Router::new()
                .fallback_service(ServeDir::new(&upload_dir))
                .layer(SetResponseHeaderLayer::overriding(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("private, max-age=31536000, immutable"),
                ))
                // Site photos and maps are private to signed-in people.
                .layer(axum::middleware::from_fn_with_state(state.clone(), auth::guard)),
        )
        // Browsers must re-check the app shell after every update instead of
        // guessing it's fresh; hashed bundles are cached by the service worker.
        .fallback_service(
            Router::new()
                .fallback_service(spa)
                .layer(SetResponseHeaderLayer::if_not_present(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"))),
        )
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("SiteMonitor is running at http://localhost:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
