


# PHASE 1: FULL-STACK RUST WORKSPACE & SHARED DOMAIN MODELS

## 1. Objective
We are building a production-ready, two-sided Progressive Web App (PWA) for managing construction, maintenance, and asset logistics across a sprawling ultra-high-net-worth (UHNW) estate. 
- **Side A (The Showcase):** VIP Client Executive Portal (Notion-inspired minimalist document UI, interactive SVG estate map, decision approvals, photo call-out pins, quiet-day overrides, handover sign-offs).
- **Side B (The Engine):** Contractor & Maintenance Staff Portal (High-contrast mobile field tool with offline-first IndexedDB queuing, morning manpower allocation for an offline construction crew, context-aware machinery movement, and weighted phase updates).

Do NOT simplify or stub out any models. Implement the complete workspace scaffold and the entire `shared` crate first.

## 2. OS Compatibility & Tech Stack Constraints
- Must compile and run cleanly on both **macOS** and **Windows**.
- Avoid OS-specific native C dependencies (use `rustls` instead of `openssl`).
- **Workspace Structure:**
  - `Cargo.toml` (Workspace root resolving `shared`, `backend`, `frontend`)
  - `shared/` (Pure Rust library crate shared between Axum and Dioxus WASM)
  - `backend/` (Axum 0.7+, SeaORM, PostgreSQL, Tokio, WebSockets, Cron scheduler)
  - `frontend/` (Dioxus 0.5+ targeting `wasm32-unknown-unknown`, Tailwind CSS, `web-sys` for IndexedDB & Canvas API)

## 3. Notion-Aesthetic Tailwind Configuration (`frontend/tailwind.config.js`)
Configure Tailwind with these exact tokens:
- **Backgrounds:** Primary `#ffffff`, Secondary/Sidebar `#f7f7f5`, Hover block `#f1f1ef`
- **Text:** Primary `#37352f` (Deep Charcoal - never pure black), Muted `#787774`
- **Borders & Dividers:** `#ededed` (1px solid)
- **Typography:** Sans: `Inter, -apple-system, sans-serif`; Serif (Executive Titles): `Lyon Text, Georgia, serif`
- **Status Pastel Tags:**
  - Green (On Track / Operational / Resolved): bg `#edf3ec`, text `#346538`
  - Amber (Pending Decision / Minor Delay / Maintenance): bg `#fbf3db`, text `#8f6215`
  - Red (Critical / Work Stoppage / Broken Down): bg `#fdebec`, text `#9e2a2b`
  - Gray (Archived / Routine / Offline): bg `#f1f1ef`, text `#5a5a57`

## 4. Complete Shared Domain Models (`shared/src/models.rs`)
Implement all of the following structs and enums with `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]`:

### 4.1 Users & Role-Based Access Control
*(Note: The physical construction crew does not carry cellphones and has no user accounts; they are tracked via the Manpower Ledger).*
- `UserRole`: `VipClient`, `OfficeAdmin`, `SiteMonitor`
- `User`: `id: Uuid`, `name: String`, `role: UserRole`, `phone: String`, `email: String`, `assigned_area_ids: Vec<Uuid>`

### 4.2 Infinite Nesting Spatial Map & Base Layers
- `ZoneClass`:
  - `MajorResidential` (3 primary residences)
  - `MinorResidential` (4 secondary residences)
  - `MultiPurposeFacility` (buildings, event tents, gazebos, parking conversions)
  - `Recreational` (gardens, outdoor pools)
  - `UtilityInfrastructure` (Power House/Genset, Water Filtration, Active Deepwell)
  - `ContractorOperations` (Contractor Office, Warehouse, Workshop)
- `AreaStatus`: `Optimal`, `ActiveWork`, `AttentionRequired`, `CriticalAlert`, `Archived`
- `BaseMapVersion`: `id: Uuid`, `version_name: String`, `orthographic_image_url: String`, `is_active: bool`, `created_at: DateTime<Utc>`
- `Point2D`: `x: f32`, `y: f32` (Normalized 0.0 to 100.0 percentage coordinates relative to the base map)
- `SpatialArea`:
  - `id: Uuid`
  - `base_map_id: Uuid`
  - `parent_id: Option<Uuid>` (Self-referencing key enabling infinite nesting: Estate -> Zone -> Sub-Zone -> Room)
  - `name: String`
  - `zone_class: ZoneClass`
  - `polygon_points: Vec<Point2D>`
  - `is_archived: bool` (Soft-delete to preserve historical projects when facilities are demolished/repurposed)

### 4.3 Projects, Maintenance Taxonomy & Personnel Assignment
- `WorkStream`:
  - `CapitalProject` (e.g., Facility-to-Parking conversion, Gazebo build, Retaining wall leveling)
  - `RoutineMaintenance` (e.g., Swimming pool cleanup, Deepwell descaling, Water filtration backwash)
  - `ReactiveRepair` (e.g., Elevator repair, Generator recalibration)
- `WorkStatus`: `Planning`, `InProgress`, `Halted`, `PendingSignOff` (Held at 99%), `CompletedAndArchived`
- `ProjectPersonnel`:
  - `group_leader_name: String`, `group_leader_phone: String`
  - `site_qc_officer_name: String`, `site_qc_officer_phone: String`
  - `admin_coordinator_name: String`, `admin_coordinator_phone: String`
- `WorkItem`:
  - `id: Uuid`, `area_id: Uuid`, `stream: WorkStream`, `title: String`, `description: String`
  - `status: WorkStatus`, `calculated_progress: u8` (0-100, derived automatically from weighted phases)
  - `original_deadline: DateTime<Utc>`, `current_target_deadline: DateTime<Utc>`
  - `personnel: ProjectPersonnel`
  - `active_worker_count: u32` (Current day's allocated offline crew count)
  - `before_photo_url: Option<String>`, `after_photo_url: Option<String>`

### 4.4 Weighted Project Phases
- `ProjectPhase`:
  - `id: Uuid`, `work_item_id: Uuid`, `phase_name: String`, `order_index: u8`
  - `weight_percentage: u8` (All phases in a `WorkItem` must sum to 100)
  - `is_completed: bool`, `completed_at: Option<DateTime<Utc>>`, `proof_photo_url: Option<String>`

### 4.5 Offline Crew Manpower Ledger (Reshuffling & Daily Reset)
- `ManpowerLedgerEntry`:
  - `id: Uuid`, `work_item_id: Uuid`, `source_work_item_id: Option<Uuid>` (If transferred from another site)
  - `headcount_delta: i32` (e.g., `+6` or `-4`), `resulting_headcount: u32`
  - `reason: String`, `logged_by: Uuid`, `timestamp: DateTime<Utc>`

### 4.6 Decisions, Material Delays & Action Center
- `UrgencyLevel`: `Standard`, `High`, `Urgent`
- `AlertCategory`: `MaterialDelay`, `DesignDecision`, `ScheduleChange`, `EquipmentBreakdown`, `RoutineUpdate`
- `DecisionStatus`: `DraftInAdminQueue`, `PublishedPendingClient`, `ApprovedOptionA`, `ApprovedOptionB`, `AcknowledgedReceipt`
- `DecisionAlert`:
  - `id: Uuid`, `work_item_id: Uuid`, `area_id: Uuid`, `urgency: UrgencyLevel`, `category: AlertCategory`
  - `title: String`, `issue_description: String`, `mitigation_action_taken: String`
  - `option_a_label: Option<String>` (e.g., "Approve Local Paving Supplier - No Delay")
  - `option_b_label: Option<String>` (e.g., "Wait for Imported Stones - +14 Days Delay")
  - `status: DecisionStatus`, `created_at: DateTime<Utc>`, `resolved_at: Option<DateTime<Utc>>`

### 4.7 Assets, Heavy Machinery & Digital Passports
- `MobilityType`: `MobileMachinery` (Backhoe, Dump Truck, 2-Ton Mini Crane), `FixedInfrastructure` (Elevator, Pool Pump, HVAC, 500 kW Generator, Deepwell Pump), `ModularComponent` (12.8V 200Ah LiFePO4 Battery)
- `AssetHealth`: `Operational`, `NeedsMaintenance`, `BrokenDown`
- `ImpactLevel`: `NoImpact`, `MinorDelay`, `FullWorkStoppage`
- `Asset`:
  - `id: Uuid`, `name: String`, `mobility: MobilityType`
  - `current_area_id: Uuid` (Where it is currently deployed or stored, e.g., Warehouse vs. South Wing)
  - `parent_asset_id: Option<Uuid>` (Supports nested assets, e.g., LiFePO4 battery inside a Solar/Power Generator array)
  - `health: AssetHealth`, `active_issue_report: Option<String>`, `impact_level: ImpactLevel`, `repair_eta: Option<DateTime<Utc>>`
- `AssetTransferLog`:
  - `id: Uuid`, `asset_id: Uuid`, `from_area_id: Option<Uuid>`, `to_area_id: Uuid`, `moved_by: Uuid`, `timestamp: DateTime<Utc>`

### 4.8 Standardized Site Photos & VIP Pin Call-Outs
- `CallOutStatus`: `Open`, `InReview`, `Resolved`
- `SitePhoto`:
  - `id: Uuid`, `work_item_id: Uuid`, `area_id: Uuid`, `image_url: String` (Strictly 16:9 cropped)
  - `caption: String`, `uploaded_by: Uuid`, `uploaded_at: DateTime<Utc>`, `client_acknowledged: bool`
- `PhotoCallOut`:
  - `id: Uuid`, `photo_id: Uuid`, `x_percent: f32`, `y_percent: f32` (0.0 to 100.0 relative to 16:9 canvas)
  - `client_comment: String`, `staff_reply: Option<String>`, `status: CallOutStatus`
  - `created_at: DateTime<Utc>`, `resolved_at: Option<DateTime<Utc>>`

### 4.9 Milestones, Delay Audit Trail & Estate Agenda (Quiet Day Engine)
- `DelayCategory`: `None`, `Weather`, `EquipmentFailure`, `MaterialShortage`, `ClientDecision`, `SiteInstability`
- `TimelineEventRecord`:
  - `id: Uuid`, `work_item_id: Uuid`, `title: String`, `description: String`
  - `is_milestone: bool`, `delay_category: DelayCategory`, `days_delayed: u16`, `timestamp: DateTime<Utc>`
- `IntrusionLevel`: `Silent`, `VisualDisturbance`, `NoiseHeavy`, `UtilityCut`
- `AgendaEventType`: `TargetDeadline`, `IntrusiveMaintenance`, `RoutineMaintenance`, `VipSpecialOccasion`
- `EstateAgendaItem`:
  - `id: Uuid`, `area_id: Option<Uuid>`, `work_item_id: Option<Uuid>`, `title: String`, `description: String`
  - `event_type: AgendaEventType`, `intrusion_level: IntrusionLevel`
  - `start_time: DateTime<Utc>`, `end_time: DateTime<Utc>`, `is_flagged_conflict: bool`

### 4.10 Auto-Generated Executive Handover Report
- `ExecutiveHandoverReport`:
  - `id: Uuid`, `work_item_id: Uuid`, `before_photo_url: String`, `after_photo_url: String`
  - `original_deadline: DateTime<Utc>`, `actual_completion_date: DateTime<Utc>`
  - `total_delay_days: u16`, `delay_audit_summary: Vec<String>`
  - `handed_over_assets: Vec<Asset>`, `client_signed_off: bool`

## Deliverable for Phase 1
Generate the Cargo workspace files and the complete `shared` crate (`Cargo.toml`, `src/lib.rs`, `src/models.rs`), and verify that `cargo check --workspace` passes with zero errors.


# PHASE 2: AXUM BACKEND, SEAORM MIGRATIONS, AUTOMATION CRONS & WEBSOCKETS

## 1. Objective
Build the `backend` crate using Axum, SeaORM (PostgreSQL), and Tokio. Implement all database migrations matching `shared::models`, REST endpoints, automated background tasks, and a real-time WebSocket hub that routes events differently based on VIP Attention Protection rules.

## 2. SeaORM Entities & Migrations (`backend/src/entities/` & `backend/src/migrations/`)
Create SeaORM entity definitions and SQL migrations for all models defined in Phase 1:
- `users`, `base_map_versions`, `spatial_areas`, `work_items`, `project_phases`, `manpower_ledger_entries`, `decision_alerts`, `assets`, `asset_transfer_logs`, `site_photos`, `photo_call_outs`, `timeline_event_records`, `estate_agenda_items`, `executive_handover_reports`.
- Create a seed script (`backend/src/seed.rs`) that populates realistic initial estate data:
  - 3 Major Residential Zones, 4 Minor Residential Zones, Multi-Purpose Facility (Parking Conversion), Gardens, Outdoor Pool, Power House (with 500 kW Generator & nested 12.8V 200Ah LiFePO4 Batteries), Deepwell, Water Filtration, and Contractor Warehouse/Workshop (with 2-Ton Mini Crane, Backhoe, Dump Truck).

## 3. Core Backend Business Logic & Automated Engines

### 3.1 Weighted Progress & 99% Handover Lock (`backend/src/services/progress.rs`)
- When a `ProjectPhase` is marked complete via `POST /api/phases/:id/complete`:
  1. Sum `weight_percentage` of all completed phases for that `work_item_id`.
  2. Automatically create a `TimelineEventRecord` with `is_milestone = true`.
  3. **The 99% Rule:** If all phases are 100% complete, set `WorkItem.calculated_progress = 99` and `WorkItem.status = WorkStatus::PendingSignOff`. Automatically compile and insert the `ExecutiveHandoverReport` (pulling first/last photos, summarizing all `TimelineEventRecord` delays, and listing fixed assets in that `area_id`).
  4. The project only hits `100%` and `WorkStatus::CompletedAndArchived` when the VIP Client calls `POST /api/work-items/:id/sign-off`. If the VIP instead submits a final `PhotoCallOut` on the handover report, keep the project at `99%` and alert the staff.

### 3.2 "Quiet Day" Special Occasion Conflict Engine (`backend/src/services/agenda.rs`)
- When the VIP Client creates a `VipSpecialOccasion` via `POST /api/agenda/quiet-day`:
  1. Query all `EstateAgendaItem` and active `WorkItem` tasks overlapping that date range.
  2. Any item with `IntrusionLevel::NoiseHeavy` (e.g., crane/backhoe work, jackhammering) or `IntrusionLevel::UtilityCut` (e.g., generator/deepwell shutdown) is automatically set to `is_flagged_conflict = true` and its associated `WorkItem.status` is temporarily flagged for rescheduling.
  3. Push an immediate WebSocket alert to `OfficeAdmin` and `SiteMonitor` connections to drag-and-drop those tasks to a new date.

### 3.3 Automated Cron Jobs (`backend/src/services/scheduler.rs`)
Implement three background Tokio cron tasks:
1. **Midnight Manpower Roster Reset (00:00 Daily):** Resets `active_worker_count = 0` across all `WorkItem` rows and queues a `MorningRosterPrompt` flag for the Office Admin at 07:30 AM so yesterday's crew allocations never become "ghost workers."
2. **Dual-Warning Deadline Monitor (07:00 Daily):**
   - Scans `WorkItem.current_target_deadline`.
   - At **7 Days** and **48 Hours** out: Pushes a staff warning to `OfficeAdmin`/`SiteMonitor` prompting: *"Target deadline approaching in X hours. Current progress: Y%. Reallocate manpower?"*
   - At **48 Hours** out: Adds a polished anticipation card to the VIP Client's Agenda feed.
3. **5:00 PM VIP Daily Digest (17:00 Daily):**
   - Batches all non-urgent milestones, routine maintenance completions (e.g., pool cleaned), and standard photo uploads from the day into a single summary notification payload for the VIP Client.

## 4. WebSocket Real-Time Hub (`backend/src/ws.rs`)
Maintain active WebSocket connections mapped by `UserRole`:
- **Tier 1 Instant Push to VIP (`VipClient`):** Triggered ONLY when an `OfficeAdmin` publishes a `DecisionAlert` (`PublishedPendingClient`), an emergency `EquipmentBreakdown` causes a `FullWorkStoppage`, or a staff member resolves a VIP's `PhotoCallOut`.
- **Instant Push to Staff (`OfficeAdmin` & `SiteMonitor`):** Triggered immediately whenever the VIP Client clicks `[Approve Option A]`, `[Approve Option B]`, `[Acknowledge Receipt]`, drops a new `PhotoCallOut` pin on a photo, or activates a `[Quiet Day Override]`.

## 5. Complete REST API Router (`backend/src/routes/`)
Implement handlers for:
- `GET /api/estate/map` (Returns active `BaseMapVersion`, nested `SpatialArea` tree, and computed health status per polygon)
- `POST /api/estate/areas` & `PUT /api/estate/areas/:id` (Admin polygon creation, coordinate updates, and soft-delete archiving)
- `GET /api/work-items` & `GET /api/work-items/:id` (Full project details, phases, personnel, manpower ledger, timeline events, photos)
- `POST /api/manpower/reallocate` (Batch morning allocation + mid-day transfers between projects)
- `GET /api/assets` & `POST /api/assets/:id/relocate` & `POST /api/assets/:id/report-issue` (Context-aware asset movement, parent/child battery nesting, and breakdown reporting)
- `POST /api/decisions` (Monitor creates draft -> Admin publishes to VIP) & `POST /api/decisions/:id/respond` (VIP approves Option A/B or acknowledges)
- `POST /api/photos` & `POST /api/photos/:id/callouts` & `PUT /api/callouts/:id/resolve` (16:9 photo upload, VIP pin drop, Staff reply/resolution)
- `GET /api/agenda` & `POST /api/agenda/quiet-day` & `PUT /api/agenda/:id/reschedule`

## Deliverable for Phase 2
Implement all backend modules, migrations, seed data, cron services, and WebSocket handlers. Ensure `cargo check -p backend` compiles cleanly.


# PHASE 3: DIOXUS WASM FOUNDATION, OFFLINE SYNC ENGINE & 16:9 MEDIA PIPELINE

## 1. Objective
Initialize the `frontend` Dioxus WASM crate, configure PWA offline capabilities, build the IndexedDB offline mutation queue for field staff working in cellular dead zones, and implement the client-side HTML5 Canvas image cropper so every uploaded site photo is locked to a 16:9 aspect ratio before upload.

## 2. PWA Configuration (`frontend/index.html`, `manifest.json`, `sw.js`)
- Configure `manifest.json` with `"display": "standalone"`, `"orientation": "any"`, theme color `#ffffff`, and background `#f7f7f5`.
- Implement a Service Worker (`sw.js`) that caches the compiled WASM bundle, CSS, fonts, and the active estate 3D base map image for instant loading on poor cellular connections.

## 3. Global State & Role Switcher (`frontend/src/state.rs`)
- Create a global Dioxus Context (`AppState`) holding:
  - `current_user: User` (Include a persistent dev-mode top bar switch allowing instant toggling between `VipClient`, `OfficeAdmin`, and `SiteMonitor` to test all workflows seamlessly).
  - `is_online: bool` (Listens to browser `online`/`offline` events).
  - `pending_offline_mutations: Vec<OfflineMutation>`
  - `ws_connected: bool` and real-time toast notification queue (Custom Notion-style toast notifications; NEVER use browser `alert()`).

## 4. Offline-First IndexedDB Queue (`frontend/src/services/offline_queue.rs`)
- Field staff frequently enter dead zones (deepwell pump rooms, basements, distant gardens).
- Wrap `web-sys` IndexedDB APIs to create an object store named `staff_mutation_queue`.
- Whenever a `SiteMonitor` or `OfficeAdmin` submits a form (photo upload, phase completion, equipment breakdown, manpower shift, or call-out reply):
  1. If `navigator.onLine == false` or the HTTP request fails due to network timeout, serialize the request payload into `staff_mutation_queue`.
  2. Display a persistent badge in the Staff header: `"Offline Mode — X Updates Queued"`.
  3. Run a background async loop in Dioxus (`use_coroutine`) that listens for network restoration and flushes `staff_mutation_queue` FIFO to the Axum backend, updating the UI automatically.

## 5. Client-Side 16:9 Auto-Cropper & Compressor (`frontend/src/services/media_canvas.rs`)
- To guarantee that VIP Client `x_percent` and `y_percent` call-out pins point to the exact same physical object on an iPhone, iPad, Mac, or Windows screen, all photos must share an identical 16:9 aspect ratio.
- Using `web-sys::HtmlCanvasElement` and `CanvasRenderingContext2d`:
  1. Intercept `<input type="file" accept="image/*" capture="environment">`.
  2. Load the image into an offscreen HTML5 Canvas.
  3. Calculate a center-crop rectangle with a strict `16:9` aspect ratio (`1920x1080` max resolution).
  4. Draw the cropped region onto the canvas and export as a compressed WebP/JPEG data URL (`quality = 0.82`).
  5. Pass this standardized 16:9 payload to the API or offline queue.

## Deliverable for Phase 3
Build the Dioxus app shell, routing structure, custom toast system, IndexedDB offline sync service, and the 16:9 HTML5 Canvas processor. Verify compilation targeting `wasm32-unknown-unknown`.



# PHASE 4: MACRO ESTATE COMMAND CENTER & INFINITE SPATIAL MAP ENGINE

## 1. Objective
Build the interactive spatial mapping engine used by both the VIP Client (to visually monitor the estate and drill down into zones) and the Office Admin (to upload new 3D orthographic renders and draw/edit infinitely nested polygon boundaries).

## 2. The Interactive Map Viewer (`frontend/src/components/estate_map.rs`)
- **Base Layer:** Render the active `BaseMapVersion.orthographic_image_url` inside a responsive, zoomable/pannable container.
- **SVG Polygon Overlay:** Layer an `<svg viewBox="0 0 100 100" preserveAspectRatio="none">` directly over the base map.
- **Dynamic Polygon Styling:**
  - Render only the `SpatialArea` polygons whose `parent_id` matches the currently selected `active_parent_area_id` (and where `is_archived == false`).
  - Color-code polygons based on their computed `AreaStatus`:
    - `Optimal`: Subtle translucent green fill (`rgba(52, 101, 56, 0.12)`), crisp border.
    - `ActiveWork`: Subtle slate/blue fill.
    - `AttentionRequired` (Pending VIP Decision or Equipment Maintenance): Soft pulsing amber glow (`rgba(217, 119, 6, 0.25)`).
    - `CriticalAlert` (Work Stoppage / Broken Generator): Pulsing crimson border.
- **Infinite Drill-Down Breadcrumbs:**
  - Tapping a macro polygon (e.g., `"Major Residential 1"` or `"Massive Garden Block"`) sets it as `active_parent_area_id`, fluidly zooming into that bounding box and revealing its child sub-zones (e.g., `"South Wing"`, `"Guest Annex"`, `"Orchid Greenhouse"`).
  - Render a clean Notion-style breadcrumb bar at the top: `Entire Estate / Major Residential 1 / South Wing`.
  - Simultaneously filter the adjacent Project & Maintenance blocks to show only items within the currently selected spatial hierarchy.

## 3. Front-of-House vs. Back-of-House Layer Toggles
- Implement a clean pill toggle bar above the map:
  - **[Front-of-House View] (Default for VIP):** Displays `MajorResidential`, `MinorResidential`, `MultiPurposeFacility`, and `Recreational` (Gardens, Outdoor Pool).
  - **[Infrastructure & Operations Layer]:** Reveals `UtilityInfrastructure` (Power House/Genset, Water Filtration, Active Deepwell) and `ContractorOperations` (Office, Warehouse, Workshop).
  - When the VIP clicks a Utility or Contractor zone, display a concise executive summary block (e.g., Power House fuel/battery array health, Deepwell pressure status, or Warehouse material delivery status).

## 4. Admin Canvas Manager & Polygon Drawing Studio (`frontend/src/views/admin_map_studio.rs`)
- Accessible to `OfficeAdmin` to ensure the PWA is 100% future-proof when the client modifies the property:
  - **Versioned Base Maps:** Upload a new top-down 3D render from V-Ray/D5 Render while archiving previous versions.
  - **Point-and-Click Polygon Creator:** Click on the map canvas to drop `(x, y)` vertices, forming a closed SVG polygon. Select `parent_id` (to nest inside an existing zone), name the zone, and assign its `ZoneClass`.
  - **Vertex Dragging:** Drag existing polygon corner nodes to expand or reshape a garden or building footprint.
  - **Soft-Delete Archiving:** An `[Archive Zone]` button that hides demolished or repurposed structures from the live map while preserving all linked historical `WorkItem` records in search results under an `"Archived Locations"` badge.

## Deliverable for Phase 4
Implement the complete `estate_map.rs` viewer, breadcrumb drill-down state machine, layer toggles, and `admin_map_studio.rs` polygon editor.




# PHASE 5: SIDE B ("THE ENGINE") — STAFF & ADMIN OPERATIONAL PORTAL

## 1. Objective
Build the utilitarian, high-speed interface for `SiteMonitor` (mobile field view) and `OfficeAdmin` (desktop command view). Everything must be optimized for big tap targets, zero unnecessary typing, and full compatibility with the Phase 3 offline queue.

## 2. The 7:30 AM Morning Roster Reset & Manpower Reallocator (`frontend/src/views/staff/manpower_manager.rs`)
- **Morning Reset Banner:** If today's offline crew roster has not been initialized after the midnight reset, display a prominent banner for the `OfficeAdmin`: `"Morning Roster Reset: Allocate Today's Crew"`.
- **Unified Allocation Board:** List all active `CapitalProject`, `RoutineMaintenance`, and `ReactiveRepair` items with fast `-5`, `-1`, `+1`, `+5` stepper buttons to distribute the offline workforce in under 60 seconds.
- **Mid-Day Reshuffling Modal:** When a monitor or admin moves workers mid-day, they select `Source Project`, `Destination Project`, `Headcount`, and a quick-tag reason (e.g., `"Accelerating leveling before rain"`). This writes directly to `ManpowerLedgerEntry` and updates both projects' live headcounts.

## 3. Context-Aware Asset Mover & Breakdown Reporter (`frontend/src/views/staff/asset_ops.rs`)
- **No QR Codes — Context-First Filtering:**
  - When a `SiteMonitor` is viewing a specific zone (e.g., `"Retaining Wall"` or `"Contractor Warehouse"`) and taps **[Move Equipment]**, the dropdown defaults to listing **only the assets currently checked into that zone** (plus a fallback smart-search bar).
  - Selecting an asset (e.g., `"2-Ton Mini Crane"`) and choosing a destination zone immediately logs an `AssetTransferLog` and updates `current_area_id`.
- **Nested Asset Management:**
  - Allow linking/unlinking modular components via `parent_asset_id` (e.g., detaching a faulty `"12.8V 200Ah LiFePO4 Battery"` from the `"Solar/Backup Generator Array"` and moving the battery to `"Contractor Workshop"` while the generator remains on site).
- **Machinery Breakdown Flow:**
  - Tapping **[Report Breakdown]** on any asset (e.g., Backhoe, Dump Truck, 500 kW Generator, Elevator) prompts for:
    1. `AssetHealth` (`NeedsMaintenance` vs `BrokenDown`)
    2. `ImpactLevel` (`NoImpact`, `MinorDelay`, `FullWorkStoppage`)
    3. `Diagnostic & Mitigation Note` + `Repair ETA`
    4. *Conditional Prompt:* `"Does this impact the project's target deadline?"` If Yes, prompts for `days_delayed`, automatically shifts `WorkItem.current_target_deadline`, and logs a `TimelineEventRecord` with `DelayCategory::EquipmentFailure`.

## 4. Weighted Phase Updater & Routine Maintenance Checklists (`frontend/src/views/staff/progress_updater.rs`)
- **Capital Projects:** Display the ordered list of `ProjectPhase` items with their assigned weights (e.g., `1. Demolition (10%)`, `2. Leveling (20%)`, `3. Paving (60%)`, `4. Painting (10%)`). Tapping **[Complete Phase]** opens the 16:9 camera cropper for proof of completion and triggers the backend weighted progress calculation.
- **Routine & Reactive Maintenance:** For tasks like `"Swimming Pool Cleanup"` or `"Elevator 2 Repair"`, render a one-tap **[Mark Completed + Snap Proof Photo]** button.

## 5. The "Draft-to-VIP" Decision Pipeline (`frontend/src/views/staff/decision_pipeline.rs`)
- **Site Monitor Field Input:** Monitor snaps a 16:9 photo, flags an issue (e.g., `"Imported paving stones delayed by 2 weeks"`), selects `AlertCategory::MaterialDelay`, and submits it to the **Admin Review Queue**.
- **Office Admin Polish & Publish:** The `OfficeAdmin` reviews the field draft, refines the executive summary, defines clear `Option A` and `Option B` mitigation buttons (or sets it as `RoutineUpdate` requiring simple acknowledgment), and clicks **[Publish to VIP Client]**.

## Deliverable for Phase 5
Implement all Side B staff views, forms, context-aware dropdowns, and the Admin review queue.


# PHASE 6: SIDE A ("THE SHOWCASE") — VIP CLIENT PORTAL & PHOTO CALL-OUT ENGINE

## 1. Objective
Build the serene, Notion-inspired executive interface for the `VipClient`. Every view must feel like a high-end, ultra-clean briefing document (`#ffffff` background, `#37352f` typography, generous whitespace, 1px `#ededed` dividers).

## 2. The VIP Action Center (`frontend/src/components/vip_action_center.rs`)
- Positioned at the top of the VIP Master View. **Rule:** If there are zero pending decisions or unacknowledged updates, this section collapses completely so the interface stays calm.
- **Decision Cards (Amber Pastel Tag):**
  - Displays the issue description and your team's active mitigation plan.
  - Renders interactive action buttons: **[Approve Option A: ...]** and **[Approve Option B: ...]**.
  - Clicking an option immediately updates the card to a calming green `"Approved"` state, folds it away, and sends a real-time WebSocket alert to the staff.
- **New Update Cards (Routine Photo / Milestone):**
  - Renders a single **[Acknowledge Receipt]** button so the client can clear items they have reviewed.

## 3. The Notion-Style Project Document View (`frontend/src/views/vip/project_document.rs`)
When the VIP clicks a zone on the map or selects a project, render a structured document page containing:
1. **Executive Header:** Serif title (`Lyon Text`), current status pill, sleek progress bar (`calculated_progress%`), and `Current Target Deadline` (with a subtle note if shifted from original deadline).
2. **Work Stream Separation:**
   - **Block A — Active Capital Developments:** Full progress cards with weighted phase breakdowns.
   - **Block B — Facility Maintenance & Repairs:** Minimalist checklist showing routine completions (`"✓ Pool Maintenance — Completed 8:00 AM"`) and reactive repairs (`"⚠️ Elevator 2 Repair — Technician on site"`).
3. **Live Manpower & Leadership Roster Block:**
   - Displays today's live headcount: e.g., `"14 Active Personnel"` with a live delta badge: `"(+6 transferred from Guest House this morning to accelerate leveling)"`.
   - Lists the three key accountable leaders with one-tap `tel:` call/message links:
     - **Construction Group Leader**
     - **On-Site Quality Control (QC) Officer**
     - **Admin Office Coordinator**
4. **Zone Asset & Machinery Inventory Block:**
   - Split cleanly into **Fixed Infrastructure** (Generators, Deepwell Pumps, HVAC) and **Deployed Heavy Machinery** (2-Ton Mini Crane, Backhoe), displaying their live health pill and repair ETA if down.
5. **Milestone & Delay Audit Trail Timeline:**
   - Chronological vertical timeline combining completed milestones, manpower shifts, and logged delays (explicitly showing root cause tag like `Weather` or `Equipment Failure` and days impacted).

## 4. Interactive 16:9 Photo Viewer & Tap-to-Pin Call-Out Engine (`frontend/src/components/photo_annotator.rs`)
- **VIP Tap-to-Pin Interaction:**
  - Render uploaded site photos in a locked `aspect-video` (16:9) container.
  - When the VIP clicks/taps anywhere on the image, calculate `x_percent = (click_x / width) * 100.0` and `y_percent = (click_y / height) * 100.0`.
  - Render a glowing gold/amber pin (`#d97706`) at `left: {x_percent}%; top: {y_percent}%`.
  - Open a minimalist inline popover below the photo: `"Add a call-out note for the site team..."` with a **[Submit Call-Out]** button.
- **Staff Resolution Loop (Connected to Side B):**
  - Submitting a call-out fires an instant WebSocket alert to `SiteMonitor` and `OfficeAdmin`.
  - Staff opens the exact same 16:9 photo on their phone, sees the pin at `(x_percent, y_percent)`, inspects the physical area on site, types a reply (e.g., `"Just runoff from the tool wash station, redirecting drainage now"`), and clicks **[Mark Resolved]**.
  - On the VIP's screen, the pin smoothly transitions from amber to green (`#346538`) and displays the staff's resolution note on tap.

## Deliverable for Phase 6
Implement the VIP Action Center, the Notion-style Project Document View, the Leadership & Manpower block, and the bidirectional 16:9 Photo Call-Out Annotator.


# PHASE 7: SMART "ESTATE AGENDA", QUIET DAY OVERRIDE & EXECUTIVE HANDOVER REPORTS

## 1. Objective
Complete the final two pillars of the platform: the dual-view Calendar/Agenda system with automated "Quiet Day" noise suppression, and the Auto-Generated Executive Handover Report with the 99% client sign-off lock.

## 2. Dual-View Calendar Architecture (`frontend/src/views/agenda_view.rs`)

### 2.1 Side A: The VIP "Estate Agenda" (Curated Chronological Timeline)
- Never show a cluttered 30-day grid to the VIP Client. Render a clean, Notion-style chronological timeline showing only:
  1. **Capital Target Deadlines** (including 48-hour approaching handover cards).
  2. **Intrusive Maintenance Alerts** (`IntrusionLevel::UtilityCut` or `NoiseHeavy`, such as *"Deepwell Descaling — Water pressure reduced 1:00 PM to 3:00 PM"*). Routine silent tasks remain hidden.
  3. **VIP Special Occasions ("Quiet Days").**

### 2.2 The VIP "[Quiet Day Override]" Action
- Include a prominent **[+ Schedule Quiet Day / Special Occasion]** button on the VIP Agenda.
- The VIP selects a date/time range and enters a label (e.g., *"Weekend Estate Gala"*).
- Upon submission:
  - Display immediate confirmation to the VIP: *"Confirmed. All heavy machinery (cranes, backhoes) and utility interruptions are automatically halted. Silent grounds detailing (pool & gardens) will proceed prior to your event."*
  - Trigger the backend conflict engine to flag all overlapping `NoiseHeavy` and `UtilityCut` tasks across adjacent zones.

### 2.3 Side B: The Admin "Master Scheduler" & Conflict Resolver
- For `OfficeAdmin`, render a full operational scheduler showing all routine maintenance recurrences (Mon/Wed/Fri pool cleaning, filtration backwash) and machinery deployments.
- **Equipment Conflict Detection:** Warn the admin if a piece of mobile machinery (e.g., `2-Ton Mini Crane`) is double-booked across two zones on the same day.
- **Quiet Day Conflict Queue:** When the VIP books a Quiet Day, highlight all halted `NoiseHeavy`/`UtilityCut` tasks in red at the top of the Admin Scheduler so the admin can one-click reschedule them to the following workdays.

## 3. Auto-Generated Executive Handover Report (`frontend/src/components/handover_report.rs`)
When all weighted phases of a `CapitalProject` are checked off by staff, the project pauses at **99% (`PendingSignOff`)** and presents this auto-generated report to the VIP Client:
1. **Interactive Before & After Slider:** A draggable vertical split-screen slider comparing `before_photo_url` (Day 1) and `after_photo_url` (Completion).
2. **Schedule & Delay Audit Summary:** Compares `original_deadline` vs `actual_completion_date`. Automatically lists every logged `TimelineEventRecord` delay reason (e.g., *"+2 Days: Torrential rain delayed crane anchoring on Nov 4"*) for complete transparency.
3. **Asset Handover Ledger:** Lists all new fixed infrastructure assets installed in that zone during the project, automatically transitioning them into the zone's permanent maintenance inventory.
4. **Final Client Sign-Off Controls:**
   - **[Accept & Archive Project (100%)]:** Marks the project `100% CompletedAndArchived`, removes the active work badge from the estate map, and stores the report in the historical archive.
   - **[Add Final Call-Out (Hold at 99%)]:** Allows the VIP to tap the "After" photo, drop a coordinate pin (e.g., *"Move this debris bin out of the corner"*), and bounce the project back to the `SiteMonitor` queue at 99% until resolved.

## 4. Final End-to-End Verification
- Verify that the entire workspace compiles cleanly (`cargo check --workspace` and Dioxus WASM build).
- Ensure zero placeholder functions or unimplemented routes remain across `shared`, `backend`, and `frontend`.