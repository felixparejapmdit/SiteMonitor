//! `shared` — the single source of truth for SiteMonitor's domain.
//!
//! Compiled both natively (Axum backend) and to `wasm32-unknown-unknown`
//! (Dioxus frontend). Keep this crate free of platform-specific dependencies.

pub mod api;
pub mod labels;
pub mod models;
pub mod ws;

pub use labels::Tone;
pub use models::*;
