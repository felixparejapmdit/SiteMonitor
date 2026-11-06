//! Real-time WebSocket events.
//!
//! The backend routes each event by role ("VIP Attention Protection"):
//! the VIP only ever receives the few Tier-1 events that truly need them,
//! while staff receive every client action instantly.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::*;

/// Which slice of cached data changed, so clients can refresh quietly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DataScope {
    Map,
    WorkItems,
    Assets,
    Decisions,
    Agenda,
    Notifications,
    Photos,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum WsEvent {
    // ---------------------------------------------- Tier 1: instant to VIP
    /// An Office Admin published a decision for the client.
    DecisionPublished(DecisionAlert),
    /// An equipment breakdown caused a full work stoppage.
    EquipmentStoppage { asset: Asset, work_item_title: Option<String> },
    /// Staff resolved one of the client's photo call-outs.
    CallOutResolved(PhotoCallOut),
    /// The 5:00 PM digest.
    DailyDigest(Notification),

    // ---------------------------------------------- instant to staff
    /// The client approved an option or acknowledged an update.
    DecisionResponded(DecisionAlert),
    /// The client dropped a new pin on a photo.
    CallOutCreated { callout: PhotoCallOut, work_item_id: Uuid },
    /// The client booked a Quiet Day; these tasks need a new date.
    QuietDayActivated { occasion: EstateAgendaItem, flagged: Vec<EstateAgendaItem> },
    /// The client signed off a project.
    ProjectSignedOff(WorkItem),
    /// Deadline warnings, roster prompts and other staff notices.
    StaffNotice(Notification),

    // ---------------------------------------------- everyone, silent
    /// Data changed; refresh in the background without interrupting.
    DataChanged { scopes: Vec<DataScope> },
    /// Server greeting after a successful connection.
    Hello { role: UserRole },
}
