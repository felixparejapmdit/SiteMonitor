//! Request / response contracts for the REST API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::access::AccessOverrides;
use crate::models::*;

/// Name of the HttpOnly session cookie set at sign-in.
pub const SESSION_COOKIE: &str = "sm_session";

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

/// `POST /api/work-items/{id}/cancel`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CancelProjectRequest {
    pub reason: String,
}

/// `POST /api/work-items/{id}/delay` — pushes the target date back and records why.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogDelayRequest {
    pub category: DelayCategory,
    pub days: u16,
    pub title: String,
    pub note: String,
}

// ---------------------------------------------------------------- workforce

/// `POST /api/workers`, `PUT /api/workers/{id}`
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SaveWorkerRequest {
    pub a_number: String,
    pub name: String,
    pub phone: String,
    pub trade: String,
    pub department: String,
    pub area: String,
    pub project_group: String,
    pub group_role: String,
    pub status: WorkerStatus,
    pub archived: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ImportMode {
    /// Add new records and update matching ones; nothing is ever deleted.
    #[default]
    Merge,
    /// Make the workforce data exactly match the file (a snapshot is taken first).
    Replace,
}

/// `POST /api/workforce/import?file_name=…&mode=…` — the file itself is the raw
/// request body (a Service 5 backup of any version, an `.agwcz` encrypted
/// backup, a `service5.db`, or a SiteMonitor workforce backup). An encrypted
/// backup's passphrase goes in the `x-backup-passphrase` header as hex of its UTF-8 bytes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportOptions {
    #[serde(default)]
    pub file_name: String,
    /// The merge review's choices, as JSON: conflict id -> keep_local / accept_incoming / smart_merge.
    #[serde(default)]
    pub resolutions: String,
    #[serde(default)]
    pub mode: ImportMode,
    /// Lets a file with no workforce data through (anti-wipe override).
    #[serde(default)]
    pub allow_empty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WorkforceImportReport {
    /// e.g. "Service 5 backup (v4)", "Service 5 database", "Encrypted Service 5 backup (v11)".
    pub detected: String,
    pub workers: usize,
    /// (collection, rows) for everything else that was imported.
    pub collections: Vec<(String, usize)>,
    /// Collections that were empty in the file and so left untouched (anti-wipe).
    pub kept_existing: Vec<String>,
    /// The snapshot taken just before the import (restore it to undo).
    pub snapshot_id: Option<Uuid>,
}

/// `PUT /api/workers/{id}/status` — today's attendance (logged like Service 5).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkerStatusRequest {
    pub status: WorkerStatus,
    #[serde(default)]
    pub reason: String,
}

/// `PUT /api/workers/{id}/reassign` (Shift+R) — each changed field is logged.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReassignRequest {
    pub project_group: String,
    pub area: String,
    pub department: String,
    #[serde(default)]
    pub reason: String,
}

/// A Service 5 project group, its people today, and the job it works on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkGroup {
    pub name: String,
    pub total: u32,
    pub present: u32,
    pub absent: u32,
    pub on_pass: u32,
    pub work_item_id: Option<Uuid>,
}

/// `GET /api/workforce/summary` — the workforce dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct WorkforceSummary {
    pub trades: Vec<(String, u32)>,
    pub departments: Vec<(String, u32)>,
    /// Last 14 days: (date, people marked absent, people marked on pass).
    pub trend: Vec<(String, u32, u32)>,
    /// This month: (name, "YYYY-MM-DD" birthday).
    pub birthdays: Vec<(String, String)>,
}

/// `PUT /api/workforce/groups` — links a project group to a job (or unlinks it).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupLinkRequest {
    pub group: String,
    pub work_item_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkforceSnapshotInfo {
    pub id: Uuid,
    pub taken_at: DateTime<Utc>,
    /// "daily", "before import", "before restore" or "manual".
    pub reason: String,
    pub workers: i32,
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

/// What kind of thing happened on site (drives the feed's icon).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActivityKind {
    Milestone,
    Delay,
    Update,
    Photo,
}

/// One line in the "What's happening now" feed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActivityItem {
    pub id: Uuid,
    pub kind: ActivityKind,
    pub title: String,
    pub detail: String,
    pub work_item_id: Uuid,
    pub image_url: Option<String>,
    pub at: DateTime<Utc>,
}

/// One-tap office decision on a field report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReviewDecisionRequest {
    pub action: ReviewAction,
    /// Reason (decline) or what is missing (ask for details).
    #[serde(default)]
    pub note: Option<String>,
}

/// Changes a report that is still waiting for office review.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpdateDecisionRequest {
    pub work_item_id: Uuid,
    pub category: AlertCategory,
    pub urgency: UrgencyLevel,
    pub title: String,
    pub issue_description: String,
    pub mitigation_action_taken: String,
    /// A new photo (`data:image/...`); `None` keeps the current one.
    pub photo_data_url: Option<String>,
    /// Take the photo off the report.
    #[serde(default)]
    pub remove_photo: bool,
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

// ----------------------------------------------------------------- people

/// `POST /api/users` and `PUT /api/users/{id}` (Office Admin).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SaveUserRequest {
    pub name: String,
    pub role: Option<UserRole>,
    pub phone: String,
    pub username: String,
    /// Zones a Site Monitor looks after (ignored for other roles).
    pub assigned_area_ids: Vec<Uuid>,
    pub is_active: bool,
    /// New accounts only: the first password, which the person must change.
    #[serde(default)]
    pub initial_password: Option<String>,
}

// ----------------------------------------------------------------- sign-in

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Returned by sign-in. Browsers use the cookie; `token` is for API clients
/// (sent as `Authorization: Bearer <token>`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionResponse {
    pub user: User,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResetPasswordResponse {
    /// Shown to the admin once; the person must change it at next sign-in.
    pub temporary_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DemoAccount {
    pub name: String,
    pub role: UserRole,
    pub username: String,
}

/// What the sign-in page may show before anyone is signed in.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AuthOptions {
    /// Only filled in demo mode (`SITEMONITOR_DEMO=1`).
    pub demo_accounts: Vec<DemoAccount>,
    pub demo_password: Option<String>,
}

// ----------------------------------------------------------------- access matrix

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccessAuditEntry {
    pub id: Uuid,
    pub at: DateTime<Utc>,
    pub actor_name: String,
    pub target_name: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccessMatrixResponse {
    pub users: Vec<User>,
    pub audit: Vec<AccessAuditEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccessChange {
    pub user_id: Uuid,
    pub access: AccessOverrides,
}

/// Saves several people's access in one go (all or nothing).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SaveAccessRequest {
    pub changes: Vec<AccessChange>,
}

// ----------------------------------------------------------------- projects

/// One phase row in the project editor. `id` is `None` for a new phase;
/// the list order becomes the phase order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhaseInput {
    pub id: Option<Uuid>,
    pub phase_name: String,
    pub weight_percentage: u8,
}

/// `POST /api/work-items` and `PUT /api/work-items/{id}` (Office Admin).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SaveWorkItemRequest {
    pub area_id: Uuid,
    pub stream: WorkStream,
    pub title: String,
    pub description: String,
    /// Only Planning, In progress or Paused can be chosen by hand.
    pub status: WorkStatus,
    pub target_deadline: DateTime<Utc>,
    pub personnel: ProjectPersonnel,
    /// Construction projects only; weights must add up to 100.
    pub phases: Vec<PhaseInput>,
}

// ----------------------------------------------------------------- equipment

/// `POST /api/assets` and `PUT /api/assets/{id}` (Office Admin).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SaveAssetRequest {
    pub name: String,
    pub mobility: MobilityType,
    pub current_area_id: Uuid,
    pub is_retired: bool,
}

/// `POST /api/workers/delete` — permanent removal of the chosen people.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeleteWorkersRequest {
    pub ids: Vec<Uuid>,
}
