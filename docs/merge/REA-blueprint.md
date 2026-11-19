# REA Blueprint: merging Service 5 DB (`01_Rust_WC_DB`) into SiteMonitor

Evidence-based map of the source app, produced read-only from `../01_Rust_WC_DB`
(source code, `data/service5.db` opened `mode=ro&immutable=1`, `pb_schema.json`).
Counts below are from the live `service5.db` on 2026-11-19.

## 0. Size and shape of the source

| Part | Size | Notes |
|---|---|---|
| Frontend | ~40k lines vanilla JS in 27 modules (`js/`), ~11k lines CSS (`css/database.css` alone 11k), one 390 KB `database.html` | No build step; global functions; IndexedDB via LocalForage |
| Server | ~840 lines Rust (`rust/server/src`: axum + rusqlite) | A thin **key/value sync store**, not a relational backend |
| Database | SQLite, 3 tables | `records` (45 keys), `daily_snapshots` (2), `sync_audit_log` (46) |
| PocketBase (NAS) | `pb_schema.json` | `records`, `maps`, `zones`, `sub_zones`, `projects` — same key/value model plus an early estate map |

**Key finding:** all business logic lives in the browser. The server stores whole
arrays as JSON blobs under keys (`records.key` → `records.data`). There are **no
relational tables to copy** — every entity must be modelled fresh and filled by a
normalizer.

## 1. Data & schema map (`records` keys)

Two generations of keys coexist: plain (`workers`, `ot_entries`, …) and `wc_`-prefixed
(`wc_ot_entries`, …). Several pairs differ in count (e.g. `volunteer_health_logs` 133 vs
`wc_volunteer_health_logs` 151; `duty_schedules` 156 vs `wc_duty_schedules` 161), so the
importer must **merge both by `id`, newest `updatedAt`/`_lastModified` wins** — never pick one.

| Entity | Key(s) | Rows | Main fields | Target in SiteMonitor |
|---|---|---|---|---|
| **Workers / volunteers** | `workers` | 174 | 92 fields: `workerId` (UUID, v11), `aNumber` (A#), `name`, `phone`, `photoUrl`, `trade`/`trades`/`tradeGroup`/`tradeRole`, `skillLevel`, `hourlyRate`, `department`/`subDepartment`, `area` (38 distinct), `zone_id`/`sub_zone_id`/`sub_zone_code`, `projectGroup`/`project`/`projectRole`, `groupName`/`groupRole` (Member, Katiwala, II Katiwala, Kalihim), `dailyStatus` (Present/Absent/Passes), `absentReason`, `assignment`/`tomorrowAssignment`/`specificTask`, `archived`/`archiveCategory`/`archiveReason`/`archivedAt`, `reemployedAt`, family & spouse block (12 fields), emergency contact, `birthday`, `bloodType`, `civilStatus`, `medical`/`medicalNotes`/`maintenanceMeds`/`healthClearance`, visitation block (`visitationGroup`, `lastVisited*`, `upcomingVisit*`), `attendanceHistory`, `assignmentHistory` (embedded logs since v11), `version_vector`, `_firestoreId` | new `workers` table; status enum `Present / Absent / OnPass` + `archived` flag; personal & family fields in a JSONB `profile` column (kept verbatim, typed later) |
| Attendance log | `attendance_log` | 10,791 | `aNumber`, `date`, `oldStatus`, `newStatus`, `reason`, `timestamp`, `workerName` | `worker_attendance` (worker_id FK, date, status, reason) |
| Assignment log | `assignment_log` | 1,605 | `aNumber`, `date`, `field`, `oldValue`, `newValue` | `worker_assignment_log` |
| Departments / areas / trades | derived from workers + `custom_datalists` (`housingGroups`, `tradeGroups`, `trades`, `workResidences`) | — | free-text values | `departments` + pick lists in `estate_settings` |
| Project groups | `project_groups_meta` / `wc_project_groups_meta` | 18 / 25 | dict keyed `"Site \| Group"` → meta | `project_groups` (name, site, optional `area_id` → `SpatialArea`, optional `work_item_id`) |
| Signatories | `signatories` / `wc_signatories_data` | 2 | `name`, `trades`, `projectGroups`, `signatureUrl` (image), `signatureDate` | `signatories` (+ signature image in uploads) |
| Overnight duty areas | `duty_areas` / `wc_duty_areas` | 18 | `name`, `color`, `shiftTime`, `workersNeeded`, `roster`, `lastRotationIndex` | `duty_areas` |
| Duty rosters / schedules | `duty_rosters` 30, `duty_schedules` 156/161 | | `dateStr`, `assignments[]` | `duty_assignments` (date, duty_area, worker) |
| Visitations | `visitations` / `wc_visitations_data` | 62 | `workerId`, `date`, `ministerName`, `status`, `completionDetails` | `visitations` |
| Visitation groups | `visitation_groups` | 23 | `name`, `housing`, `prayerDay`, `prayerTime` | `visitation_groups` |
| Ministers | `ministers` | 3 | `name` | `ministers` |
| OT entries | `ot_entries` / `wc_ot_entries` | 70 | `date`, `hours`, `hourlyRate`, `computedPay`, `otType`, `otArea`, `otTask`, `skillLevel`, `trade`, `volunteers[]`, `status` | `ot_entries` + `ot_entry_workers` |
| **Clinic: health events** | `health_events` / `wc_health_events` | 84 | episode state machine (`dayZero`, day 8/10/14 antigen & symptoms, `clearanceDate`, `respiratoryBlock`), `covid*`, `diagnosis` | `health_events` — **sensitive** |
| Clinic: visit logs | `volunteer_health_logs` | 133/151 | vitals (BP, HR, temp, O₂, RR), complaint, diagnosis, meds, physician, return-to-work, photos | `clinic_visits` — **sensitive** |
| Clinic: vaccines / meds | `volunteer_vaccines` 1, `volunteer_medications` 0 | | lot, dates, boosters | `vaccinations`, `medications` |
| Contact tracing | `contact_traces` / `wc_contact_traces` | 69 | `healthEventId`, `contactWorkerId`, `linkType`, conversion to event | `contact_traces` |
| Medical audit | `medical_audit_logs` | 50/51 | who/what/why on medical edits | `clinic_audit` (append-only) |
| History log | `history` | 556 | daily auto-snapshots (`activeCount`, `totalCount`) + actions | `workforce_history` |
| Daily snapshots | table `daily_snapshots` | 2 | full JSON per date | `daily_snapshots` (same shape) |
| Sync audit | table `sync_audit_log` | 46 | key, action, client_id | not migrated (replaced by SiteMonitor's audit) |
| Dashboard dismissals | `dashboard_dismissed` / `wc_dashboard_dismissed_*` | | id lists | per-user UI state — not migrated |

## 2. Legacy backups and encryption

**JSON backup** (`js/backup.js`, `normalizeBackupData`, line 271):
- Version detection: object with `version` → that number; bare array / no version → v1;
  default v4. `formatVersion` 1 (< v4) or 2 (≥ v4). Envelope `syncMetadata` holds
  `exportTimestamp`, `formatVersion`, `recordCount`, `sourceDeviceId`, `storesExported`.
- Schema steps (`js/core.js` `migrateData`, `SCHEMA_VERSION = 11`):
  v1→2 duty stores · v2→3 health stores · v3→4 duty colour/shift/rotation · v4→5 vaccines
  & custom groups · v5→6 health logs/medications/isolation · v6→7 isolation → health
  events, strip medical strings from workers · v7→8 clinic logs for orphan events ·
  v8→9 OT stores + `skillLevel` backfill · v9→10 removed (no-op) · v10→11 `workerId`
  UUID backfill + attendance/assignment logs embedded in workers.
- The SiteMonitor importer replays exactly these steps server-side (Rust), then maps to
  the tables above. Pre-import checkpoint (the source keeps one in `sessionStorage`)
  becomes a server-side DB snapshot taken before every import.

**Encrypted backup** (`js/crypto.js` `encryptBackupArchive`): envelope
`{ _isAgwcEncrypted: true, format: "agwcz", version: 1, algo: "AES-GCM",
kdf: "PBKDF2-HMAC-SHA256", iterations: 600000, salt (16 B, base64), iv (12 B, base64),
sha256 (hex of the plaintext JSON), data (ciphertext + GCM tag, base64), timestamp }`,
AES-256-GCM with a 256-bit key. Decrypt server-side with the `aes-gcm` + `pbkdf2`
crates (pure Rust, no OpenSSL — fine on Windows and macOS). Wrong passphrase → GCM tag
failure → clear "wrong passphrase" error, nothing written.

**Anti-wipe** (`js/pocketbase_sync.js:883`): an empty array never overwrites a populated
store. Ported as a server rule: an import or sync that would replace a non-empty table
with zero rows is rejected unless the caller passes an explicit typed confirmation.

## 3. Business logic to carry over

- **Offline sync**: `wc_offline_sync_queue` + optimistic last-write-wins per key with a
  `CLIENT_ID` stamped on every write; the client ignores WebSocket echoes carrying its own
  id. SiteMonitor already has an IndexedDB mutation queue (`services/offline_queue`) and a
  WS hub; add `client_id` to mutations and `updated_at` checks per row (row-level LWW,
  finer than the source's whole-array LWW).
- **Keyboard**: `Shift+R` reassign, `Ctrl+K` worker lookup (`js/worker_lookup.js`).
- **Print** (`css/print.css`, 475 lines): 4-up folio job orders on 8.5×13 in, landscape
  monthly attendance sheets, ID cards, clinical reports, with Safari guards. Port the CSS
  as-is into a print stylesheet; render print pages as dedicated routes.
- **Duty rotation**: `lastRotationIndex` round-robin over each area's roster.
- **OT pay**: `computedPay = hours × hourlyRate` by skill level and OT type.
- **Health episode state machine**: day 0 → day 8/10/14 checks → cleared / blocked.

## 4. Integration with SiteMonitor

| Source | SiteMonitor | How |
|---|---|---|
| Workers `dailyStatus` | Crew page, Morning Roster | Crew shows Total active / Present / Absent / On pass from workers; headcount per job becomes the number of assigned present workers (manual headcount stays as a fallback for contractors without records) |
| Worker → project group → area | `WorkItem` + `SpatialArea` | `project_groups.area_id` / `work_item_id`; assigning a group to a job assigns its present members |
| Mid-day reallocation (`js/reallocation.js`) | Move workers | Moves named workers (logged to `worker_assignment_log`) instead of anonymous counts |
| Signatories / leaders (`groupRole` Katiwala…) | Project team pickers | Pickers list People accounts **and** workers with leader roles, phone auto-filled |
| Areas / departments | Zones | One-time mapping screen: legacy `area` text → zone; unmapped values kept as-is |
| `zones`, `sub_zones`, `maps` in `pb_schema.json` | Map Studio | Early estate map — import as zones only if present on the NAS; the SQLite db has none |

## 5. Decisions (owner, 2026-11-19)

1. **Dark mode** — stays light-only (the earlier decision stands).
2. **Medical data** — Clinic Ledger is Office Admin only: a `ManageClinic` permission
   (grantable per person in Access), never visible to the owner, every read/write logged.
3. **Scale** — built in the phases below, each tested and committed. Phase 1 first.

## 5a. Findings while building Phase 1

- A live `service5.db` is ~90 MB (plus a WAL file of recent writes), so imports upload the
  raw file (no base64/JSON) on a route allowing 512 MB. **Close Service 5 (or use its own
  Backup button) before copying `service5.db`**: changes still in `service5.db-wal` are not
  in the main file.
- Service 5 gives workers without a `workerId` a random one on every import; SiteMonitor
  derives it from the A# instead, so importing the same file twice never duplicates people.
- Attendance and assignment logs stay as their own collections (10,791 / 1,605 rows) rather
  than being embedded per worker with Service 5's 500-entry cap, so no history is dropped.

## 6. Phased plan

| Phase | Delivers | Depends on |
|---|---|---|
| **1. Data foundation** | Tables + migrations for every entity above; legacy importer (JSON v1–v11, `agwcz` encrypted, `service5.db`), anti-wipe, pre-import snapshot; Backup/Restore/Daily Snapshot in Settings; Personnel Directory (list, search, Ctrl+K, add/edit/archive) | — |
| **2. Crew integration** | Present/Absent/On pass counts, worker-based morning roster and mid-day moves (Shift+R), project groups ↔ zones/jobs, team pickers from workers | 1 |
| **3. Workforce views** | Assignments, Departments, Project Groups, Archived personnel, History log, Org chart, Dependability | 1 |
| **4. Operations & print** | Overnight duty + rotation, Attendance generator + landscape sheet, OT calculator & entries, Visitations & groups, Update forms, ID cards, 4-up job orders | 1 |
| **5. Clinic ledger** | Health events state machine, visit logs, vaccines/meds, contact tracing, clinical reports, audit | 1, permission decision |
| **6. Sync & sidebar** | client_id + row LWW, unified sidebar (Estate Command / Workforce & Crew / Operations & Duty / System & Admin) with role scoping, full e2e incl. legacy import test | all |
