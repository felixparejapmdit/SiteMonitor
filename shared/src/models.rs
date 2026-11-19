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
    /// Deactivated people keep their history but can no longer sign in or be chosen.
    #[serde(default = "active_default")]
    pub is_active: bool,
    /// The Office Admin's departures from the role's default permissions.
    #[serde(default)]
    pub access: crate::access::AccessOverrides,
    /// What this person may do (role defaults + granted - revoked).
    #[serde(default)]
    pub permissions: Vec<crate::access::Permission>,
    /// Set after an admin creates the account or resets its password.
    #[serde(default)]
    pub must_change_password: bool,
    #[serde(default)]
    pub last_sign_in_at: Option<DateTime<Utc>>,
}

impl User {
    pub fn can(&self, p: crate::access::Permission) -> bool {
        self.permissions.contains(&p)
    }
}

fn active_default() -> bool {
    true
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
    /// Zoom tiles cut from an uploaded picture (`None` for drawings, which stay sharp anyway).
    #[serde(default)]
    pub tiles: Option<MapTiles>,
}

/// A big map picture cut into square tiles at every zoom level, so only the
/// part on screen is loaded, at the sharpness the zoom needs.
/// Level `max_level` is full size; each level below is half as big; level 0 fits one tile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MapTiles {
    pub width: u32,
    pub height: u32,
    pub tile_size: u32,
    pub max_level: u32,
    /// Folder URL: tiles are `{base_url}/{level}/{x}_{y}.jpg`, the overview is `{base_url}/preview.jpg`.
    pub base_url: String,
}

impl MapTiles {
    /// Width of the overview picture (`preview.jpg`); closer views use tiles.
    pub const PREVIEW_WIDTH: u32 = 2048;

    pub fn levels_for(width: u32, height: u32, tile_size: u32) -> u32 {
        let mut level = 0;
        while (width.max(height) >> level) > tile_size {
            level += 1;
        }
        level
    }

    /// Picture size in pixels at `level`.
    pub fn level_size(&self, level: u32) -> (u32, u32) {
        let d = 1u32 << (self.max_level - level.min(self.max_level));
        (self.width.div_ceil(d), self.height.div_ceil(d))
    }

    /// Columns and rows of tiles at `level`.
    pub fn grid(&self, level: u32) -> (u32, u32) {
        let (w, h) = self.level_size(level);
        (w.div_ceil(self.tile_size), h.div_ceil(self.tile_size))
    }

    /// The smallest level that is at least `needed_px` wide on screen.
    pub fn level_for(&self, needed_px: f64) -> u32 {
        (0..=self.max_level).find(|l| self.level_size(*l).0 as f64 >= needed_px).unwrap_or(self.max_level)
    }

    pub fn tile_url(&self, level: u32, x: u32, y: u32) -> String {
        format!("{}/{level}/{x}_{y}.jpg", self.base_url)
    }

    pub fn preview_url(&self) -> String {
        format!("{}/preview.jpg", self.base_url)
    }
}

impl BaseMapVersion {
    /// A light picture for thumbnails and the zoomed-out view.
    pub fn preview_url(&self) -> String {
        self.tiles.as_ref().map(|t| t.preview_url()).unwrap_or_else(|| self.orthographic_image_url.clone())
    }
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
    /// Stopped for good by the office; kept in History with its reason.
    Cancelled,
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
    // The office settles a field report itself (the owner never sees these):
    /// Parked for later.
    OnHold,
    /// Sent back to the reporter, who edits it and it returns to the office.
    NeedsDetails,
    OfficeApproved,
    OfficeDeclined,
}

/// What the office can do with a field report in one tap.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReviewAction {
    Approve,
    Decline,
    Hold,
    AskForDetails,
    /// Back to "to review".
    Reopen,
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
    /// The 16:9 field photo sent with the report.
    #[serde(default)]
    pub photo_url: Option<String>,
    /// Who wrote the report (they may edit it until the office sends it on).
    #[serde(default)]
    pub reported_by: Option<Uuid>,
    /// The office's reason when it declines a report or asks for more details.
    #[serde(default)]
    pub office_note: Option<String>,
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
    /// Retired equipment leaves the active list but stays in past records.
    #[serde(default)]
    pub is_retired: bool,
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

// ---------------------------------------------------------------------------
// Estate settings (editable by the Office Admin)
// ---------------------------------------------------------------------------

/// One settings record for the whole estate. Missing fields fall back to the defaults.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct EstateSettings {
    pub estate_name: String,
    /// Local "HH:MM" times for the automatic jobs.
    pub roster_reset_time: String,
    pub roster_prompt_time: String,
    pub deadline_check_time: String,
    pub digest_time: String,
    /// Quick reasons offered when moving workers between jobs.
    pub crew_move_reasons: Vec<String>,
    /// Custom wording for issue categories; blank or missing uses the built-in label.
    pub category_labels: std::collections::HashMap<AlertCategory, String>,
    /// Custom wording for schedule event types.
    pub event_type_labels: std::collections::HashMap<AgendaEventType, String>,
}

impl Default for EstateSettings {
    fn default() -> Self {
        Self {
            estate_name: "Ashford Estate".into(),
            roster_reset_time: "00:00".into(),
            roster_prompt_time: "07:30".into(),
            deadline_check_time: "07:00".into(),
            digest_time: "17:00".into(),
            crew_move_reasons: [
                "Accelerating leveling before rain",
                "Urgent repair needs extra hands",
                "Materials just arrived",
                "Finished early here",
                "Rain \u{2014} moving to indoor work",
                "Owner requested priority",
            ]
            .map(String::from)
            .to_vec(),
            category_labels: Default::default(),
            event_type_labels: Default::default(),
        }
    }
}

impl EstateSettings {
    pub fn category_label(&self, c: AlertCategory) -> String {
        self.category_labels.get(&c).map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or(c.label()).to_string()
    }
    pub fn event_type_label(&self, k: AgendaEventType) -> String {
        self.event_type_labels.get(&k).map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or(k.label()).to_string()
    }
}

// ---------------------------------------------------------------------------
// Workforce (merged from Service 5 DB)
// ---------------------------------------------------------------------------

/// Today's status of a worker or volunteer.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum WorkerStatus {
    #[default]
    Present,
    Absent,
    /// "Passes" in Service 5: away with permission.
    OnPass,
}

impl WorkerStatus {
    pub const ALL: [WorkerStatus; 3] = [WorkerStatus::Present, WorkerStatus::Absent, WorkerStatus::OnPass];

    pub fn label(self) -> &'static str {
        match self {
            WorkerStatus::Present => "Present",
            WorkerStatus::Absent => "Absent",
            WorkerStatus::OnPass => "On pass",
        }
    }

    /// Reads Service 5's free-text status ("Present", "Absent", "Passes", …).
    pub fn from_legacy(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "absent" => WorkerStatus::Absent,
            "passes" | "pass" | "on pass" | "onpass" => WorkerStatus::OnPass,
            _ => WorkerStatus::Present,
        }
    }
}

/// A worker or volunteer (Service 5's "worker"). The full original record is
/// kept on the server; these are the fields every screen uses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Worker {
    pub id: Uuid,
    /// Service 5's "A#" worker number.
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
    pub updated_at: DateTime<Utc>,
    /// A photo is on file (served by `GET /api/workers/{id}/photo`).
    #[serde(default)]
    pub has_photo: bool,
    /// Tomorrow's assignment (Service 5's encoder), "" when not set.
    #[serde(default)]
    pub tomorrow: String,
    /// Where the person stays (Service 5 `workResidence` / `residence`).
    #[serde(default)]
    pub residence: String,
    /// Today's task, project and absence reason (Service 5 `specificTask`, `project`, `absentReason`).
    #[serde(default)]
    pub task: String,
    #[serde(default)]
    pub project: String,
    #[serde(default)]
    pub remarks: String,
    #[serde(default)]
    pub spouse: String,
    /// Visitation housing cluster and group.
    #[serde(default)]
    pub housing: String,
    #[serde(default)]
    pub visit_group: String,
    #[serde(default)]
    pub visit_role: String,
    /// Retired, Dismissed or Deceased when archived.
    #[serde(default)]
    pub archive_category: String,
    /// Medical conditions — blank unless the viewer has "Clinic ledger".
    #[serde(default)]
    pub medical: String,
    #[serde(default)]
    pub birthday: String,
    #[serde(default)]
    pub hourly_rate: Option<f64>,
    #[serde(default)]
    pub skill_level: String,
    /// Service 5 "Issue a job order" (`jOrder` YES/NO).
    #[serde(default = "yes")]
    pub job_order: bool,
    /// Given a task today (cleared by Start New Day).
    #[serde(default)]
    pub assigned_today: bool,
    /// When the current task started (Service 5 `taskStartedAt`), for stale-task alerts.
    #[serde(default)]
    pub task_since: String,
    /// Stale-task alerts turned off for this person.
    #[serde(default)]
    pub stale_off: bool,
}

fn yes() -> bool {
    true
}

#[cfg(test)]
mod tile_tests {
    use super::MapTiles;

    #[test]
    fn tile_levels_halve_down_to_one_tile() {
        let t = MapTiles { width: 6000, height: 4000, tile_size: 512, max_level: MapTiles::levels_for(6000, 4000, 512), base_url: "/m".into() };
        assert_eq!(t.max_level, 4); // 6000 -> 3000 -> 1500 -> 750 -> 375
        assert_eq!(t.level_size(4), (6000, 4000));
        assert_eq!(t.level_size(0), (375, 250));
        assert_eq!(t.grid(0), (1, 1));
        assert_eq!(t.grid(4), (12, 8));
        assert_eq!(t.level_for(1400.0), 2); // 1500 wide is the first that's enough
        assert_eq!(t.level_for(99_999.0), 4);
        assert_eq!(t.tile_url(2, 1, 0), "/m/2/1_0.jpg");
    }
}
