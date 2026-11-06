//! Real-time WebSocket hub with role-based routing ("VIP Attention Protection").
//!
//! * Tier-1 events go to the VIP Client instantly (published decisions, full
//!   work stoppages, resolved call-outs) — nothing else interrupts them.
//! * Every client action goes to Office Admins and Site Monitors instantly.
//! * Silent `DataChanged` refreshes go to everyone so screens stay current.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use sea_orm::EntityTrait;
use serde::Deserialize;
use shared::ws::{DataScope, WsEvent};
use shared::UserRole;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::convert;
use crate::entities::prelude::Users;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

const STAFF: [UserRole; 2] = [UserRole::OfficeAdmin, UserRole::SiteMonitor];

#[derive(Clone, Default)]
pub struct Hub {
    connections: Arc<RwLock<HashMap<Uuid, (UserRole, mpsc::UnboundedSender<String>)>>>,
}

impl Hub {
    async fn register(&self, role: UserRole) -> (Uuid, mpsc::UnboundedReceiver<String>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let id = Uuid::new_v4();
        self.connections.write().await.insert(id, (role, tx));
        (id, rx)
    }

    async fn unregister(&self, id: Uuid) {
        self.connections.write().await.remove(&id);
    }

    pub async fn send_to(&self, roles: &[UserRole], event: &WsEvent) {
        let Ok(text) = serde_json::to_string(event) else { return };
        let conns = self.connections.read().await;
        for (role, tx) in conns.values() {
            if roles.contains(role) {
                let _ = tx.send(text.clone());
            }
        }
    }

    /// Tier-1 instant push to the VIP Client.
    pub async fn to_vip(&self, event: WsEvent) {
        self.send_to(&[UserRole::VipClient], &event).await;
    }

    /// Instant push to Office Admin and Site Monitor connections.
    pub async fn to_staff(&self, event: WsEvent) {
        self.send_to(&STAFF, &event).await;
    }

    /// Silent background refresh for everyone.
    pub async fn changed(&self, scopes: &[DataScope]) {
        self.send_to(&UserRole::ALL, &WsEvent::DataChanged { scopes: scopes.to_vec() }).await;
    }
}

#[derive(Deserialize)]
pub struct WsParams {
    user_id: Uuid,
}

pub async fn handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<WsParams>,
) -> AppResult<Response> {
    let user = Users::find_by_id(params.user_id)
        .one(&state.db)
        .await?
        .map(convert::user)
        .ok_or_else(|| AppError::Unauthorized("Please choose who you are before connecting.".into()))?;
    let hub = state.hub.clone();
    Ok(ws.on_upgrade(move |socket| serve(socket, hub, user.role)))
}

async fn serve(socket: WebSocket, hub: Hub, role: UserRole) {
    let (id, mut rx) = hub.register(role).await;
    let (mut sink, mut stream) = socket.split();

    let hello = serde_json::to_string(&WsEvent::Hello { role }).unwrap_or_default();
    if sink.send(Message::Text(hello.into())).await.is_err() {
        hub.unregister(id).await;
        return;
    }

    let mut send_task = tokio::spawn(async move {
        while let Some(text) = rx.recv().await {
            if sink.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            if matches!(msg, Message::Close(_)) {
                break;
            }
        }
    });
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
    hub.unregister(id).await;
}
