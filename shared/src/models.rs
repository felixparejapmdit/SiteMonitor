//! Complete SiteMonitor domain model.
//!
//! Every struct and enum in this file is shared verbatim between the Axum
//! backend and the Dioxus WASM frontend, so both sides always agree on the
//! exact shape of the data that travels over HTTP, WebSockets and the
//! offline IndexedDB queue.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// 4.1 Users & Role-Based Access Control
// ---------------------------------------------------------------------------

/// The three kinds of people who sign in to SiteMonitor.
///
/// The physical construction crew does not carry phones and has no user
/// accounts; their daily headcount is tracked through the Manpower Ledger.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UserRole {
    VipClient,
    OfficeAdmin,
    SiteMonitor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub role: UserRole,
    pub phone: String,
    pub email: String,
    pub assigned_area_ids: Vec<Uuid>,
}

// ---------------------------------------------------------------------------
// 4.2 Infinite Nesting Spatial Map & Base Layers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ZoneClass {
    /// The 3 primary residences.
    MajorResidential,
    /// The 4 secondary residences.
    MinorResidential,
    /// Buildings, event tents, gazebos, parking conversions.
    MultiPurposeFacility,
    /// Gardens and outdoor pools.
    Recreational,
    /// Power House / Genset, Water Filtration, Active Deepwell.
    UtilityInfrastructure,
    /// Contractor Office, Warehouse, Workshop.
    ContractorOperations,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AreaStatus {
    Optimal,
    ActiveWork,
    AttentionRequired,
    CriticalAlert,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaseMapVersion {
    pub id: Uuid,
    pub version_name: String,
    pub orthographic_image_url: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// Normalized 0.0 – 100.0 percentage coordinates relative to the base map.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point2D {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SpatialArea {
    pub id: Uuid,
    pub base_map_id: Uuid,
    /// Self-referencing key enabling infinite nesting:
    /// Estate -> Zone -> Sub-Zone -> Room.
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub zone_class: ZoneClass,
    pub polygon_points: Vec<Point2D>,
    /// Soft-delete that preserves historical projects when a facility is
    /// demolished or repurposed.
    pub is_archived: bool,
}

// ---------------------------------------------------------------------------
// 4.3 Projects, Maintenance Taxonomy & Personnel Assignment
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WorkStream {
    /// e.g. Facility-to-Parking conversion, Gazebo build, Retaining wall leveling.
    CapitalProject,
    /// e.g. Swimming pool cleanup, Deepwell descaling, Water filtration backwash.
    RoutineMaintenance,
    /// e.g. Elevator repair, Generator recalibration.
    ReactiveRepair,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WorkStatus {
    Planning,
    InProgress,
    Halted,
    /// Held at 99% until the VIP Client signs off.
    PendingSignOff,
    CompletedAndArchived,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectPersonnel {
    pub group_leader_name: String,
    pub group_leader_phone: String,
    pub site_qc_officer_name: String,
    pub site_qc_officer_phone: String,
    pub admin_coordinator_name: String,
    pub admin_coordinator_phone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkItem {
    pub id: Uuid,
    pub area_id: Uuid,
    pub stream: WorkStream,
    pub title: String,
    pub description: String,
    pub status: WorkStatus,
    /// 0 – 100, derived automatically from weighted phases.
    pub calculated_progress: u8,
    pub original_deadline: DateTime<Utc>,
    pub current_target_deadline: DateTime<Utc>,
    pub personnel: ProjectPersonnel,
    /// Current day's allocated offline crew count.
    pub active_worker_count: u32,
    pub before_photo_url: Option<String>,
    pub after_photo_url: Option<String>,
}

// ---------------------------------------------------------------------------
// 4.4 Weighted Project Phases
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectPhase {
    pub id: Uuid,
    pub work_item_id: Uuid,
    pub phase_name: String,
    pub order_index: u8,
    /// All phases of a `WorkItem` must sum to 100.
    pub weight_percentage: u8,
    pub is_completed: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub proof_photo_url: Option<String>,
}

// ---------------------------------------------------------------------------
// 4.5 Offline Crew Manpower Ledger (Reshuffling & Daily Reset)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManpowerLedgerEntry {
    pub id: Uuid,
    pub work_item_id: Uuid,
    /// Set when workers were transferred from another site.
    pub source_work_item_id: Option<Uuid>,
    /// e.g. `+6` or `-4`.
    pub headcount_delta: i32,
    pub resulting_headcount: u32,
    pub reason: String,
    pub logged_by: Uuid,
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// 4.6 Decisions, Material Delays & Action Center
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum UrgencyLevel {
    Standard,
    High,
    Urgent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AlertCategory {
    MaterialDelay,
    DesignDecision,
    ScheduleChange,
    EquipmentBreakdown,
    RoutineUpdate,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DecisionStatus {
    DraftInAdminQueue,
    PublishedPendingClient,
    ApprovedOptionA,
    ApprovedOptionB,
    AcknowledgedReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionAlert {
    pub id: Uuid,
    pub work_item_id: Uuid,
    pub area_id: Uuid,
    pub urgency: UrgencyLevel,
    pub category: AlertCategory,
    pub title: String,
    pub issue_description: String,
    pub mitigation_action_taken: String,
    /// e.g. "Approve Local Paving Supplier - No Delay".
    pub option_a_label: Option<String>,
    /// e.g. "Wait for Imported Stones - +14 Days Delay".
    pub option_b_label: Option<String>,
    pub status: DecisionStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// 4.7 Assets, Heavy Machinery & Digital Passports
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MobilityType {
    /// Backhoe, Dump Truck, 2-Ton Mini Crane.
    MobileMachinery,
    /// Elevator, Pool Pump, HVAC, 500 kW Generator, Deepwell Pump.
    FixedInfrastructure,
    /// 12.8V 200Ah LiFePO4 Battery.
    ModularComponent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AssetHealth {
    Operational,
    NeedsMaintenance,
    BrokenDown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ImpactLevel {
    NoImpact,
    MinorDelay,
    FullWorkStoppage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Asset {
    pub id: Uuid,
    pub name: String,
    pub mobility: MobilityType,
    /// Where it is currently deployed or stored (e.g. Warehouse vs. South Wing).
    pub current_area_id: Uuid,
    /// Supports nested assets, e.g. a LiFePO4 battery inside a generator array.
    pub parent_asset_id: Option<Uuid>,
    pub health: AssetHealth,
    pub active_issue_report: Option<String>,
    pub impact_level: ImpactLevel,
    pub repair_eta: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssetTransferLog {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub from_area_id: Option<Uuid>,
    pub to_area_id: Uuid,
    pub moved_by: Uuid,
    pub timestamp: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// 4.8 Standardized Site Photos & VIP Pin Call-Outs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CallOutStatus {
    Open,
    InReview,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SitePhoto {
    pub id: Uuid,
    pub work_item_id: Uuid,
    pub area_id: Uuid,
    /// Strictly 16:9 cropped.
    pub image_url: String,
    pub caption: String,
    pub uploaded_by: Uuid,
    pub uploaded_at: DateTime<Utc>,
    pub client_acknowledged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhotoCallOut {
    pub id: Uuid,
    pub photo_id: Uuid,
    /// 0.0 – 100.0 relative to the 16:9 canvas.
    pub x_percent: f32,
    pub y_percent: f32,
    pub client_comment: String,
    pub staff_reply: Option<String>,
    pub status: CallOutStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// 4.9 Milestones, Delay Audit Trail & Estate Agenda (Quiet Day Engine)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DelayCategory {
    None,
    Weather,
    EquipmentFailure,
    MaterialShortage,
    ClientDecision,
    SiteInstability,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimelineEventRecord {
    pub id: Uuid,
    pub work_item_id: Uuid,
    pub title: String,
    pub description: String,
    pub is_milestone: bool,
    pub delay_category: DelayCategory,
    pub days_delayed: u16,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IntrusionLevel {
    Silent,
    VisualDisturbance,
    NoiseHeavy,
    UtilityCut,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AgendaEventType {
    TargetDeadline,
    IntrusiveMaintenance,
    RoutineMaintenance,
    VipSpecialOccasion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EstateAgendaItem {
    pub id: Uuid,
    pub area_id: Option<Uuid>,
    pub work_item_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub event_type: AgendaEventType,
    pub intrusion_level: IntrusionLevel,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub is_flagged_conflict: bool,
}

// ---------------------------------------------------------------------------
// 4.10 Auto-Generated Executive Handover Report
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutiveHandoverReport {
    pub id: Uuid,
    pub work_item_id: Uuid,
    pub before_photo_url: String,
    pub after_photo_url: String,
    pub original_deadline: DateTime<Utc>,
    pub actual_completion_date: DateTime<Utc>,
    pub total_delay_days: u16,
    pub delay_audit_summary: Vec<String>,
    pub handed_over_assets: Vec<Asset>,
    pub client_signed_off: bool,
}

// ---------------------------------------------------------------------------
// Supporting records required by the automation engines (Phase 2 & 7).
// These extend — never replace — the models above.
// ---------------------------------------------------------------------------

/// What kind of in-app notice a [`Notification`] represents.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NotificationKind {
    /// Queued by the midnight reset; shown to the Office Admin from 07:30.
    MorningRosterPrompt,
    /// 7-day / 48-hour staff deadline warning.
    DeadlineWarning,
    /// Polished 48-hour "handover approaching" card on the VIP agenda.
    AnticipationCard,
    /// The 5:00 PM VIP daily digest.
    DailyDigest,
    /// Anything else worth telling someone.
    General,
}

/// A stored, dismissible in-app notice for everyone with a given role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Notification {
    pub id: Uuid,
    pub audience: UserRole,
    pub kind: NotificationKind,
    pub work_item_id: Option<Uuid>,
    pub title: String,
    pub body: String,
    /// Extra lines (used by the daily digest).
    pub lines: Vec<String>,
    /// Notices are hidden until this moment (e.g. 07:30 roster prompt).
    pub visible_from: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub dismissed: bool,
}

/// Books a piece of mobile machinery to an agenda slot so the Master
/// Scheduler can warn about double-bookings across zones.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EquipmentReservation {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub agenda_item_id: Uuid,
}
