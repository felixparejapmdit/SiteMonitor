//! Data loading, offline-aware writes and the background sync loop.

use chrono::Utc;
use dioxus::prelude::*;
use futures::future::join_all;
use gloo_timers::future::TimeoutFuture;
use serde::de::DeserializeOwned;
use serde::Serialize;
use shared::ws::DataScope;

use crate::api::{self, ApiFail};
use crate::services::offline_queue::{self, OfflineMutation};
use crate::state::{AppState, ToastTone};
use crate::util;

pub const ALL_SCOPES: [DataScope; 7] = [
    DataScope::Map,
    DataScope::WorkItems,
    DataScope::Assets,
    DataScope::Decisions,
    DataScope::Agenda,
    DataScope::Notifications,
    DataScope::Photos,
];

/// Fetches one slice of data into the cache. Failures keep the old data.
pub async fn load(app: AppState, scope: DataScope) {
    let AppState {
        mut map,
        mut work_items,
        mut assets,
        mut decisions,
        mut agenda,
        mut notifications,
        mut photo_updates,
        mut open_callouts,
        mut detail_epoch,
        ..
    } = app;
    match scope {
        DataScope::Map => {
            if let Ok(v) = api::get("/estate/map").await {
                if map.peek().as_ref() != Some(&v) {
                    map.set(Some(v));
                }
            }
        }
        DataScope::WorkItems => {
            if let Ok(v) = api::get("/work-items").await {
                if work_items.peek().as_ref() != Some(&v) {
                    work_items.set(Some(v));
                }
            }
            detail_epoch += 1;
        }
        DataScope::Assets => {
            if let Ok(v) = api::get("/assets").await {
                if assets.peek().as_ref() != Some(&v) {
                    assets.set(Some(v));
                }
            }
        }
        DataScope::Decisions => {
            if let Ok(v) = api::get("/decisions").await {
                if decisions.peek().as_ref() != Some(&v) {
                    decisions.set(Some(v));
                }
            }
        }
        DataScope::Agenda => {
            if let Ok(v) = api::get("/agenda").await {
                if agenda.peek().as_ref() != Some(&v) {
                    agenda.set(Some(v));
                }
            }
        }
        DataScope::Notifications => {
            if let Ok(v) = api::get("/notifications").await {
                if *notifications.peek() != v {
                    notifications.set(v);
                }
            }
        }
        DataScope::Photos => {
            let (updates, callouts) = futures::join!(api::get("/photos/updates"), api::get("/callouts/open"));
            if let Ok(v) = updates {
                if *photo_updates.peek() != v {
                    photo_updates.set(v);
                }
            }
            if let Ok(v) = callouts {
                if *open_callouts.peek() != v {
                    open_callouts.set(v);
                }
            }
            detail_epoch += 1;
        }
    }
}

/// Refreshes the given scopes in parallel.
pub async fn load_many(app: AppState, scopes: &[DataScope]) {
    join_all(scopes.iter().map(|s| load(app, *s))).await;
}

/// Fire-and-forget refresh (silent, keeps showing cached data meanwhile).
pub fn reload(app: AppState, scopes: Vec<DataScope>) {
    spawn(async move { load_many(app, &scopes).await });
}

pub fn reload_all(app: AppState) {
    reload(app, ALL_SCOPES.to_vec());
}

/// Result of a write that may have been queued for later.
pub enum Outcome<T> {
    Saved(T),
    /// No signal — saved on this device and will sync automatically.
    Queued,
    Failed(String),
}

/// Sends a write. Staff writes made without signal go to the IndexedDB
/// queue instead of failing.
pub async fn mutate<B: Serialize, T: DeserializeOwned>(
    app: AppState,
    label: impl Into<String>,
    method: &str,
    path: impl Into<String>,
    body: &B,
) -> Outcome<T> {
    let path = path.into();
    let label = label.into();
    let body = serde_json::to_value(body).unwrap_or(serde_json::Value::Null);
    let staff = app.is_staff();
    if staff && !*app.is_online.peek() {
        return queue(app, label, method, path, body).await;
    }
    match api::send_raw(method, &path, Some(&body), None).await {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(v) => Outcome::Saved(v),
            Err(_) => Outcome::Failed("Saved, but the reply was unexpected. Please refresh.".into()),
        },
        Err(ApiFail::Network) if staff => queue(app, label, method, path, body).await,
        Err(e) => Outcome::Failed(e.message()),
    }
}

async fn queue<T>(app: AppState, label: String, method: &str, path: String, body: serde_json::Value) -> Outcome<T> {
    let Some(user_id) = app.user_id() else {
        return Outcome::Failed("Please choose who you are first.".into());
    };
    let m = OfflineMutation { key: 0.0, label: label.clone(), method: method.into(), path, body, user_id, queued_at: Utc::now() };
    match offline_queue::enqueue(&m).await {
        Ok(()) => {
            refresh_pending(app).await;
            app.toast(
                ToastTone::Warning,
                "Saved on this device",
                Some(format!("No signal right now. \u{201c}{label}\u{201d} will sync automatically when you're back online.")),
            );
            Outcome::Queued
        }
        Err(e) => Outcome::Failed(format!("Couldn't save offline: {e}")),
    }
}

pub async fn refresh_pending(app: AppState) {
    let mut pending = app.pending_offline_mutations;
    if let Ok(list) = offline_queue::list().await {
        if *pending.peek() != list {
            pending.set(list);
        }
    }
}

/// Replays queued writes FIFO. Returns false if the network is still down.
pub async fn flush(app: AppState) -> bool {
    let mut syncing = app.syncing;
    if *syncing.peek() {
        return true;
    }
    syncing.set(true);
    let mut delivered = 0;
    let mut network_ok = true;
    if let Ok(list) = offline_queue::list().await {
        for m in list {
            match api::send_raw(&m.method, &m.path, Some(&m.body), Some(m.user_id)).await {
                Ok(_) => {
                    let _ = offline_queue::remove(m.key).await;
                    delivered += 1;
                }
                Err(ApiFail::Network) => {
                    network_ok = false;
                    break;
                }
                Err(ApiFail::Server(_, msg)) => {
                    let _ = offline_queue::remove(m.key).await;
                    app.toast(ToastTone::Error, format!("Couldn't sync \u{201c}{}\u{201d}", m.label), Some(msg));
                }
            }
            refresh_pending(app).await;
        }
    }
    refresh_pending(app).await;
    syncing.set(false);
    if delivered > 0 {
        app.toast(
            ToastTone::Success,
            format!("Synced {} saved offline", util::plural(delivered, "update")),
            Some("Everything you did without signal is now with the team.".into()),
        );
        load_many(app, &ALL_SCOPES).await;
    }
    network_ok
}

/// Watches connectivity and flushes the queue whenever signal returns.
pub async fn connectivity_loop(app: AppState) {
    let mut is_online = app.is_online;
    refresh_pending(app).await;
    let mut cool_down = 0u32;
    loop {
        let online = util::navigator_online();
        if online != *is_online.peek() {
            is_online.set(online);
            if online {
                app.toast(ToastTone::Success, "Back online", None);
                load_many(app, &ALL_SCOPES).await;
            }
        }
        if cool_down > 0 {
            cool_down -= 1;
        } else if online && !app.pending_offline_mutations.peek().is_empty() && !flush(app).await {
            // Signal is still too weak; try again in ~10 seconds.
            cool_down = 7;
        }
        TimeoutFuture::new(1500).await;
    }
}
