//! Request / response contracts for the REST API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::*;

/// HTTP header the frontend uses to say which (dev-mode) user is acting.
pub const USER_HEADER: &str = "x-user-id";

/// Uniform error body returned by every failing endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiError {
    pub message: String,
}

// ----------------------------------------------------------------- estate map

/// One polygon in the nested estate tree, with its computed health.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AreaNode {
    pub area: SpatialArea,
    pub status: AreaStatus,
    /// Plain-language reason for the status ("1 decision waiting for you").
    pub status_reason: String,
    /// Short executive summary for utility / contractor zones.
    pub executive_summary: Option<String>,
    pub children: Vec<AreaNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EstateMapResponse {
    pub base_map: BaseMapVersion,
    /// All versions, newest first (previous ones are archived).
    pub base_map_history: Vec<BaseMapVersion>,
    /// Root-level areas (including archived ones, flagged as such).
    pub areas: Vec<AreaNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateAreaRequest {
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub zone_class: ZoneClass,
    pub polygon_points: Vec<Point2D>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UpdateAreaRequest {
    pub name: Option<String>,
    pub zone_class: Option<ZoneClass>,
    pub polygon_points: Option<Vec<Point2D>>,
    /// `Some(None)` moves the area to the top level.
    pub parent_id: Option<Option<Uuid>>,
    pub is_archived: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UploadBaseMapRequest {
    pub version_name: String,
    /// `data:image/...;base64,...`
    pub image_data_url: String,
}

// ----------------------------------------------------------------- work items

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhotoWithCallOuts {
    pub photo: SitePhoto,
    pub callouts: Vec<PhotoCallOut>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkItemDetail {
    pub item: WorkItem,
    pub area: SpatialArea,
    pub phases: Vec<ProjectPhase>,
    pub ledger: Vec<ManpowerLedgerEntry>,
    pub timeline: Vec<TimelineEventRecord>,
    pub photos: Vec<PhotoWithCallOuts>,
    pub decisions: Vec<DecisionAlert>,
    /// Assets currently inside this work item's area (and its sub-zones).
    pub zone_assets: Vec<Asset>,
    pub handover: Option<ExecutiveHandoverReport>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CompletePhaseRequest {
    /// 16:9 proof photo produced by the canvas cropper.
    pub proof_data_url: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CompleteTaskRequest {
    pub proof_data_url: Option<String>,
    pub note: Option<String>,
}

/// Result of completing a phase (so the UI can celebrate the 99% lock).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProgressUpdate {
    pub item: WorkItem,
    pub handover_created: bool,
}

// ----------------------------------------------------------------- manpower

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HeadcountAllocation {
    pub work_item_id: Uuid,
    pub headcount: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrewTransfer {
    pub source_work_item_id: Uuid,
    pub destination_work_item_id: Uuid,
    pub headcount: u32,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ReallocateRequest {
    /// Morning allocation: set each project's headcount directly.
    pub allocations: Vec<HeadcountAllocation>,
    /// Mid-day reshuffles between projects.
    pub transfers: Vec<CrewTransfer>,
    /// Reason recorded for the morning allocation entries.
    pub reason: Option<String>,
}

// ----------------------------------------------------------------- assets

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetsResponse {
    pub assets: Vec<Asset>,
    pub recent_transfers: Vec<AssetTransferLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RelocateAssetRequest {
    pub to_area_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LinkAssetRequest {
    /// `None` detaches the component from its parent.
    pub parent_asset_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportIssueRequest {
    pub health: AssetHealth,
    pub impact: ImpactLevel,
    pub note: String,
    pub repair_eta: Option<DateTime<Utc>>,
    /// The project affected by this breakdown, if any.
    pub work_item_id: Option<Uuid>,
    /// Filled when the deadline is affected; shifts the target deadline.
    pub days_delayed: Option<u16>,
}

// ----------------------------------------------------------------- decisions

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateDecisionRequest {
    pub work_item_id: Uuid,
    pub category: AlertCategory,
    pub urgency: UrgencyLevel,
    pub title: String,
    pub issue_description: String,
    pub mitigation_action_taken: String,
    /// Optional 16:9 field photo attached to the draft.
    pub photo_data_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublishDecisionRequest {
    pub title: String,
    pub issue_description: String,
    pub mitigation_action_taken: String,
    pub category: AlertCategory,
    pub urgency: UrgencyLevel,
    /// Leave both options empty for a simple "Acknowledge Receipt" update.
    pub option_a_label: Option<String>,
    pub option_b_label: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DecisionResponse {
    ApproveOptionA,
    ApproveOptionB,
    AcknowledgeReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RespondDecisionRequest {
    pub response: DecisionResponse,
}

// ----------------------------------------------------------------- photos

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UploadPhotoRequest {
    pub work_item_id: Uuid,
    pub caption: String,
    /// 16:9 JPEG / WebP data URL from the canvas cropper.
    pub image_data_url: String,
}

/// A photo the owner hasn't acknowledged yet (Action Center update card).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhotoUpdate {
    pub photo: SitePhoto,
    pub work_item_title: String,
}

/// An open owner note for the staff "Owner notes" queue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenCallOut {
    pub callout: PhotoCallOut,
    pub photo: SitePhoto,
    pub work_item_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateCallOutRequest {
    pub x_percent: f32,
    pub y_percent: f32,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolveCallOutRequest {
    pub reply: String,
}

// ----------------------------------------------------------------- agenda

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EquipmentConflict {
    pub asset_id: Uuid,
    pub asset_name: String,
    /// The calendar day (estate local time) of the double booking.
    pub day: String,
    pub agenda_item_ids: Vec<Uuid>,
    pub area_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgendaResponse {
    pub items: Vec<EstateAgendaItem>,
    pub reservations: Vec<EquipmentReservation>,
    pub equipment_conflicts: Vec<EquipmentConflict>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CreateAgendaItemRequest {
    pub area_id: Option<Uuid>,
    pub work_item_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub event_type: AgendaEventType,
    pub intrusion_level: IntrusionLevel,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    /// Mobile machinery booked for this slot.
    pub asset_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuietDayRequest {
    pub title: String,
    pub description: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuietDayResponse {
    pub occasion: EstateAgendaItem,
    pub flagged_items: Vec<EstateAgendaItem>,
    pub paused_work_item_ids: Vec<Uuid>,
    pub confirmation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RescheduleRequest {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

// ----------------------------------------------------------------- automation

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AutomationJob {
    MidnightRosterReset,
    DeadlineMonitor,
    VipDailyDigest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobRunResult {
    pub job: AutomationJob,
    pub summary: String,
}

/// Quick health probe used by the frontend and end-to-end tests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthResponse {
    pub ok: bool,
    pub version: String,
}
