//! Global application state (`AppState`), shared through Dioxus context.
//!
//! Every screen reads from these cached signals, so switching pages is
//! instant and background refreshes never flash a loading state.

use std::collections::HashMap;

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use shared::api::*;
use shared::*;
use uuid::Uuid;

use crate::services::offline_queue::OfflineMutation;
use crate::util::{self, FlatArea};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastTone {
    Success,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub tone: ToastTone,
    pub title: String,
    pub body: Option<String>,
    pub leaving: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapLayer {
    /// Residences, multi-purpose facilities, gardens & pool (owner default).
    FrontOfHouse,
    /// Utilities and the contractor yard.
    Infrastructure,
}

#[derive(Clone, Copy)]
pub struct AppState {
    // --- who is using the app (dev-mode role switcher)
    pub users: Signal<Vec<User>>,
    pub current_user: Signal<Option<User>>,
    // --- connectivity
    pub is_online: Signal<bool>,
    pub pending_offline_mutations: Signal<Vec<OfflineMutation>>,
    pub ws_connected: Signal<bool>,
    pub syncing: Signal<bool>,
    // --- notifications
    pub toasts: Signal<Vec<Toast>>,
    toast_seq: Signal<u64>,
    // --- cached data
    pub map: Signal<Option<EstateMapResponse>>,
    pub work_items: Signal<Option<Vec<WorkItem>>>,
    pub assets: Signal<Option<AssetsResponse>>,
    pub decisions: Signal<Option<Vec<DecisionAlert>>>,
    pub agenda: Signal<Option<AgendaResponse>>,
    pub notifications: Signal<Vec<Notification>>,
    pub photo_updates: Signal<Vec<PhotoUpdate>>,
    pub open_callouts: Signal<Vec<OpenCallOut>>,
    pub details: Signal<HashMap<Uuid, WorkItemDetail>>,
    /// Bumped whenever project details may be stale.
    pub detail_epoch: Signal<u64>,
    // --- shared UI state
    pub active_parent_area: Signal<Option<Uuid>>,
    pub map_layer: Signal<MapLayer>,
    pub equipment_zone: Signal<Option<Uuid>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            users: Signal::new(Vec::new()),
            current_user: Signal::new(None),
            is_online: Signal::new(true),
            pending_offline_mutations: Signal::new(Vec::new()),
            ws_connected: Signal::new(false),
            syncing: Signal::new(false),
            toasts: Signal::new(Vec::new()),
            toast_seq: Signal::new(0),
            map: Signal::new(None),
            work_items: Signal::new(None),
            assets: Signal::new(None),
            decisions: Signal::new(None),
            agenda: Signal::new(None),
            notifications: Signal::new(Vec::new()),
            photo_updates: Signal::new(Vec::new()),
            open_callouts: Signal::new(Vec::new()),
            details: Signal::new(HashMap::new()),
            detail_epoch: Signal::new(0),
            active_parent_area: Signal::new(None),
            map_layer: Signal::new(MapLayer::FrontOfHouse),
            equipment_zone: Signal::new(None),
        }
    }

    pub fn role(&self) -> Option<UserRole> {
        self.current_user.read().as_ref().map(|u| u.role)
    }

    pub fn is_vip(&self) -> bool {
        self.role() == Some(UserRole::VipClient)
    }

    pub fn is_staff(&self) -> bool {
        self.role().map(|r| r.is_staff()).unwrap_or(false)
    }

    pub fn is_admin(&self) -> bool {
        self.role() == Some(UserRole::OfficeAdmin)
    }

    pub fn user_id(&self) -> Option<Uuid> {
        self.current_user.read().as_ref().map(|u| u.id)
    }

    pub fn flat_areas(&self) -> Vec<FlatArea> {
        self.map.read().as_ref().map(util::flatten).unwrap_or_default()
    }

    pub fn work_item(&self, id: Uuid) -> Option<WorkItem> {
        self.work_items.read().as_ref()?.iter().find(|w| w.id == id).cloned()
    }

    pub fn user_name(&self, id: Uuid) -> String {
        self.users.read().iter().find(|u| u.id == id).map(|u| u.name.clone()).unwrap_or_else(|| "The team".into())
    }

    /// Shows a calm toast that dismisses itself.
    pub fn toast(&self, tone: ToastTone, title: impl Into<String>, body: Option<String>) {
        let mut seq = self.toast_seq;
        let mut toasts = self.toasts;
        let id = *seq.peek() + 1;
        seq.set(id);
        toasts.write().push(Toast { id, tone, title: title.into(), body, leaving: false });
        // Keep at most four on screen.
        if toasts.peek().len() > 4 {
            toasts.write().remove(0);
        }
        let duration = match tone {
            ToastTone::Error => 6500,
            ToastTone::Warning => 6000,
            _ => 4200,
        };
        spawn(async move {
            TimeoutFuture::new(duration).await;
            dismiss_toast(toasts, id).await;
        });
    }

    pub fn success(&self, title: impl Into<String>) {
        self.toast(ToastTone::Success, title, None);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.toast(ToastTone::Error, "That didn't work", Some(message.into()));
    }
}

/// Animates a toast out, then removes it.
pub async fn dismiss_toast(mut toasts: Signal<Vec<Toast>>, id: u64) {
    if let Some(t) = toasts.write().iter_mut().find(|t| t.id == id) {
        t.leaving = true;
    }
    TimeoutFuture::new(220).await;
    toasts.write().retain(|t| t.id != id);
}

pub fn use_app() -> AppState {
    use_context::<AppState>()
}
