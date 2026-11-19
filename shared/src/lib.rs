//! `shared` — the single source of truth for SiteMonitor's domain.
//!
//! Compiled both natively (Axum backend) and to `wasm32-unknown-unknown`
//! (Dioxus frontend). Keep this crate free of platform-specific dependencies.

pub mod access;
pub mod api;
pub mod geometry;
pub mod labels;
pub mod models;
pub mod ws;

pub use access::{AccessOverrides, Permission};
pub use labels::Tone;
pub use models::*;
