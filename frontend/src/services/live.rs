//! Real-time WebSocket client: reconnects automatically and turns server
//! events into calm toasts plus silent background refreshes.

use dioxus::prelude::*;
use futures::future::{select, Either};
use futures::StreamExt;
use gloo_net::websocket::{futures::WebSocket, Message};
use gloo_timers::future::TimeoutFuture;
use shared::ws::{DataScope, WsEvent};
use shared::*;

use crate::services::sync::{self, ALL_SCOPES};
use crate::state::{AppState, ToastTone};
use crate::util;

fn ws_url(user: uuid::Uuid) -> String {
    let location = util::window().location();
    let protocol = if location.protocol().unwrap_or_default() == "https:" { "wss" } else { "ws" };
    let host = location.host().unwrap_or_else(|_| "localhost:8737".into());
    format!("{protocol}://{host}/ws?user_id={user}")
}

pub async fn live_loop(app: AppState) {
    let mut connected = app.ws_connected;
    let mut backoff = 1000u32;
    let mut first = true;
    loop {
        let Some(user) = app.user_id() else {
            TimeoutFuture::new(300).await;
            continue;
        };
        if let Ok(ws) = WebSocket::open(&ws_url(user)) {
            let (_sink, mut stream) = ws.split();
            loop {
                let next = Box::pin(stream.next());
                let tick = Box::pin(TimeoutFuture::new(1000));
                match select(next, tick).await {
                    Either::Left((Some(Ok(Message::Text(text))), _)) => {
                        if let Ok(event) = serde_json::from_str::<WsEvent>(&text) {
                            if matches!(event, WsEvent::Hello { .. }) {
                                connected.set(true);
                                backoff = 1000;
                                if !first {
                                    // Catch up on anything missed while disconnected.
                                    sync::reload(app, ALL_SCOPES.to_vec());
                                }
                                first = false;
                            }
                            handle(app, event);
                        }
                    }
                    Either::Left((Some(Ok(Message::Bytes(_))), _)) => {}
                    Either::Left((Some(Err(_)), _)) | Either::Left((None, _)) => break,
                    Either::Right(_) => {
                        // Role switched: reconnect as the new user.
                        if app.user_id() != Some(user) {
                            break;
                        }
                    }
                }
            }
        }
        if *connected.peek() {
            connected.set(false);
        }
        if app.user_id() == Some(user) {
            TimeoutFuture::new(backoff).await;
            backoff = (backoff * 2).min(15_000);
        }
    }
}

fn handle(app: AppState, event: WsEvent) {
    use DataScope::*;
    match event {
        WsEvent::Hello { .. } => {}
        WsEvent::DataChanged { scopes } => sync::reload(app, scopes),

        // ------------------------------------------------ owner (Tier 1)
        WsEvent::DecisionPublished(d) => {
            app.toast(ToastTone::Warning, "A decision needs your input", Some(d.title));
            sync::reload(app, vec![Decisions, Map]);
        }
        WsEvent::EquipmentStoppage { asset, work_item_title } => {
            let place = work_item_title.map(|t| format!(" on {t}")).unwrap_or_default();
            app.toast(
                ToastTone::Error,
                "Work has stopped",
                Some(format!("{} broke down{place}. The team is already on it.", asset.name)),
            );
            sync::reload(app, vec![Assets, Map, WorkItems]);
        }
        WsEvent::CallOutResolved(c) => {
            app.toast(ToastTone::Success, "The team resolved your photo note", c.staff_reply);
            sync::reload(app, vec![Photos, Map]);
        }
        WsEvent::DailyDigest(n) => {
            app.toast(ToastTone::Info, n.title, Some(n.body));
            sync::reload(app, vec![Notifications]);
        }

        // ------------------------------------------------ staff
        WsEvent::DecisionResponded(d) => {
            let answer = match d.status {
                DecisionStatus::ApprovedOptionA => d.option_a_label.clone().map(|l| format!("Approved Option A: {l}")),
                DecisionStatus::ApprovedOptionB => d.option_b_label.clone().map(|l| format!("Approved Option B: {l}")),
                _ => Some("Acknowledged".to_string()),
            };
            app.toast(ToastTone::Success, format!("Owner responded: {}", d.title), answer);
            sync::reload(app, vec![Decisions, Map, WorkItems]);
        }
        WsEvent::CallOutCreated { callout, .. } => {
            app.toast(ToastTone::Warning, "New photo note from the owner", Some(callout.client_comment));
            sync::reload(app, vec![Photos, Map, Notifications]);
        }
        WsEvent::QuietDayActivated { occasion, flagged } => {
            app.toast(
                ToastTone::Warning,
                format!("Quiet Day booked: {}", occasion.title),
                Some(if flagged.is_empty() {
                    "No noisy work was scheduled in that window.".into()
                } else {
                    format!("{} need a new date — open the Schedule.", util::plural(flagged.len(), "task"))
                }),
            );
            sync::reload(app, vec![Agenda, WorkItems, Map, Notifications]);
        }
        WsEvent::ProjectSignedOff(w) => {
            app.toast(ToastTone::Success, "Project signed off by the owner", Some(format!("{} is 100% complete.", w.title)));
            sync::reload(app, vec![WorkItems, Map]);
        }
        WsEvent::StaffNotice(n) => {
            app.toast(ToastTone::Info, n.title, Some(n.body));
            sync::reload(app, vec![Notifications]);
        }
    }
}
