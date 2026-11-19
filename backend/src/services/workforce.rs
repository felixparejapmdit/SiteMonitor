//! Workforce data merged in from Service 5 DB: reading its backups (every
//! version, plain or `.agwcz` encrypted) and its `service5.db`, importing them
//! without ever wiping populated data, exporting, and snapshots.
//!
//! Service 5 keeps everything as whole JSON arrays under keys. Workers get a
//! typed table; every other collection is stored one row per item in
//! `workforce_records` (lossless) until its screen gives it a typed table.

use std::collections::{BTreeMap, HashMap};

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement, TransactionTrait};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use shared::api::{ImportMode, WorkforceImportReport};
use shared::WorkerStatus;
use uuid::Uuid;

use crate::convert::enc;
use crate::error::{AppError, AppResult};

/// Per-device screen state in Service 5, not data.
const SKIP: &[&str] = &["dashboard_dismissed", "dashboard_dismissed_event_ids", "dashboard_dismissed_visit_ids"];
/// Top-level fields that describe the file rather than hold data.
const META: &[&str] = &["version", "schemaVersion", "exportedAt", "source", "syncMetadata", "formatVersion", "format", "_detectedVersion"];
/// Dictionary-shaped collections are stored as a single row with this id.
const DICT_ID: &str = "_";
/// Snapshots kept before the oldest are pruned.
const KEEP_SNAPSHOTS: i64 = 14;

/// What a backup file turned out to be, after decoding.
pub struct Normalized {
    pub label: String,
    pub workers: Vec<Value>,
    /// collection → (id, item); dictionary collections hold one `DICT_ID` item.
    pub collections: BTreeMap<String, Vec<(String, Value)>>,
    /// Service 5's own daily snapshots found in a service5.db: (date, data).
    pub legacy_snapshots: Vec<(String, Value)>,
}

// ------------------------------------------------------------------ reading

/// Turns uploaded bytes into normalized data: a `service5.db`, an encrypted
/// `.agwcz` backup, a Service 5 JSON backup of any version, or our own export.
pub fn read(bytes: &[u8], passphrase: Option<&str>) -> AppResult<Normalized> {
    if bytes.starts_with(b"SQLite format 3\0") {
        let (records, snapshots) = read_sqlite(bytes)?;
        let mut n = normalize(records)?;
        n.label = "Service 5 database (service5.db)".into();
        n.legacy_snapshots = snapshots;
        return Ok(n);
    }
    let raw: Value = serde_json::from_slice(bytes).map_err(|_| AppError::bad("This file isn't a Service 5 backup, a service5.db, or a SiteMonitor workforce backup."))?;
    if raw.get("_isAgwcEncrypted").and_then(Value::as_bool) == Some(true) || raw.get("format").and_then(Value::as_str) == Some("agwcz") {
        let pass = passphrase.filter(|p| !p.is_empty()).ok_or_else(|| AppError::bad("This backup is encrypted. Enter its passphrase to import it."))?;
        let mut n = normalize(decrypt_agwcz(&raw, pass)?)?;
        n.label = format!("Encrypted {}", n.label);
        return Ok(n);
    }
    normalize(raw)
}

/// `.agwcz` archives from Service 5's `js/crypto.js`: PBKDF2-HMAC-SHA256 →
/// AES-256-GCM, with a SHA-256 of the plaintext for an integrity check.
pub fn decrypt_agwcz(env: &Value, passphrase: &str) -> AppResult<Value> {
    let field = |k: &str| -> AppResult<Vec<u8>> {
        let s = env.get(k).and_then(Value::as_str).ok_or_else(|| AppError::bad(format!("The encrypted backup is missing its \"{k}\".")))?;
        B64.decode(s).map_err(|_| AppError::bad("The encrypted backup is damaged."))
    };
    let iterations = env.get("iterations").and_then(Value::as_u64).unwrap_or(600_000);
    // Bounds keep a hostile file from pinning the server for minutes.
    if !(1_000..=5_000_000).contains(&iterations) {
        return Err(AppError::bad("The encrypted backup has an unsupported key setting."));
    }
    let (salt, iv, data) = (field("salt")?, field("iv")?, field("data")?);
    if iv.len() != 12 {
        return Err(AppError::bad("The encrypted backup is damaged."));
    }
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), &salt, iterations as u32, &mut key);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| AppError::bad("The encrypted backup is damaged."))?;
    let plain = cipher
        .decrypt(Nonce::from_slice(&iv), data.as_ref())
        .map_err(|_| AppError::bad("Wrong passphrase, or the backup file is damaged."))?;
    if let Some(expected) = env.get("sha256").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        if hex(&Sha256::digest(&plain)) != expected.to_ascii_lowercase() {
            return Err(AppError::bad("The backup failed its integrity check (SHA-256 mismatch)."));
        }
    }
    serde_json::from_slice(&plain).map_err(|_| AppError::bad("The decrypted backup isn't valid JSON."))
}

/// `service5.db`: the `records` table holds each collection as JSON under its key.
fn read_sqlite(bytes: &[u8]) -> AppResult<(Value, Vec<(String, Value)>)> {
    let path = std::env::temp_dir().join(format!("service5-import-{}.db", Uuid::new_v4()));
    std::fs::write(&path, bytes).map_err(anyhow::Error::from)?;
    let result = (|| -> rusqlite::Result<(Map<String, Value>, Vec<(String, Value)>)> {
        let conn = rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let mut stmt = conn.prepare("SELECT key, data FROM records")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut out = Map::new();
        for row in rows {
            let (k, d) = row?;
            if let Ok(v) = serde_json::from_str(&d) {
                out.insert(k, v);
            }
        }
        // Service 5's daily snapshots (older databases may not have the table).
        let mut snaps = vec![];
        if let Ok(mut s) = conn.prepare("SELECT snapshot_date, data FROM daily_snapshots ORDER BY snapshot_date") {
            let rows = s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            for row in rows.flatten() {
                if let Ok(v) = serde_json::from_str(&row.1) {
                    snaps.push((row.0, v));
                }
            }
        }
        Ok((out, snaps))
    })();
    let _ = std::fs::remove_file(&path);
    let (mut obj, snaps) = result.map_err(|_| AppError::bad("That SQLite file doesn't look like a Service 5 database (no \"records\" table)."))?;
    obj.insert("version".into(), json!(11));
    Ok((Value::Object(obj), snaps))
}

/// Service 5's `LEGACY_KEY_MAP` plus its `wc_` / `_data` storage names, so
/// every generation of key lands in one collection.
pub fn canonical(key: &str) -> String {
    let mapped = match key {
        "workersData" | "workers_data" | "wc_workers_data" => "workers",
        "signatoriesData" => "signatories",
        "ministersData" => "ministers",
        "visitationsData" => "visitations",
        "visitationGroups" | "visitationGroupsData" => "visitation_groups",
        "historyData" | "wc_history_logs" | "history_logs" => "history",
        "attendanceLog" => "attendance_log",
        "assignmentLog" => "assignment_log",
        "groupChanges" | "group_changes_log" => "group_changes",
        "dutyAreas" => "duty_areas",
        "dutyRosters" => "duty_rosters",
        "dutySchedules" => "duty_schedules",
        "healthEvents" => "health_events",
        "contactTraces" => "contact_traces",
        "vaccines" | "wc_vaccines" | "volunteerVaccines" => "volunteer_vaccines",
        "volunteerMedications" => "volunteer_medications",
        "volunteerHealthLogs" => "volunteer_health_logs",
        "medicalAuditLogs" => "medical_audit_logs",
        "otEntries" => "ot_entries",
        "dashboardDismissed" => "dashboard_dismissed",
        "projectGroupsMeta" => "project_groups_meta",
        "customDatalists" => "custom_datalists",
        "linkAuditLog" => "link_audit_log",
        _ => "",
    };
    if !mapped.is_empty() {
        return mapped.into();
    }
    let k = key.strip_prefix("wc_").unwrap_or(key);
    k.strip_suffix("_data").unwrap_or(k).to_string()
}

/// Brings any Service 5 backup (v1 bare worker array … v11) or our own export
/// into one shape. Duplicates across key generations merge by id, newest edit wins.
pub fn normalize(raw: Value) -> AppResult<Normalized> {
    let (version, obj) = match raw {
        Value::Array(workers) => (1, Map::from_iter([("workers".to_string(), Value::Array(workers))])),
        Value::Object(o) => {
            let v = ["schemaVersion", "version"].iter().find_map(|k| o.get(*k).and_then(leading_int)).unwrap_or(1);
            (v, o)
        }
        _ => return Err(AppError::bad("This file isn't a Service 5 backup.")),
    };
    let ours = obj.get("format").and_then(Value::as_str) == Some("sitemonitor-workforce");
    let mut workers: HashMap<String, Value> = HashMap::new();
    let mut lists: BTreeMap<String, HashMap<String, Value>> = BTreeMap::new();
    let mut dicts: BTreeMap<String, Map<String, Value>> = BTreeMap::new();

    for (key, value) in obj {
        if META.contains(&key.as_str()) {
            continue;
        }
        let c = canonical(&key);
        if SKIP.contains(&c.as_str()) {
            continue;
        }
        match value {
            Value::Array(items) if c == "workers" => {
                for w in items.into_iter().filter_map(clean_worker) {
                    let k = str_of(&w, "workerId");
                    let keep_old = workers.get(&k).is_some_and(|old| stamp(old) > stamp(&w));
                    if !keep_old {
                        workers.insert(k, w);
                    }
                }
            }
            Value::Array(items) => {
                let slot = lists.entry(c.clone()).or_default();
                for item in items.into_iter().filter(|v| !v.is_null()) {
                    let item = if c == "volunteer_vaccines" { migrate_vaccine(item) } else { item };
                    let id = item_id(&item);
                    let keep_old = slot.get(&id).is_some_and(|old| stamp(old) > stamp(&item));
                    if !keep_old {
                        slot.insert(id, item);
                    }
                }
            }
            Value::Object(map) => {
                let slot = dicts.entry(c).or_default();
                for (k, v) in map {
                    let k = k.trim().to_string();
                    let empty = |v: &Value| v.is_null() || v.as_str() == Some("") || v.as_array().is_some_and(|a| a.is_empty());
                    if !k.is_empty() && (!slot.contains_key(&k) || slot.get(&k).is_some_and(empty)) {
                        slot.insert(k, if let Value::String(s) = v { Value::String(s.trim().to_string()) } else { v });
                    }
                }
            }
            _ => {}
        }
    }

    let mut collections: BTreeMap<String, Vec<(String, Value)>> = BTreeMap::new();
    for (c, items) in lists {
        let mut v: Vec<(String, Value)> = items.into_iter().collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        collections.insert(c, v);
    }
    for (c, map) in dicts {
        collections.entry(c).or_default().push((DICT_ID.into(), Value::Object(map)));
    }
    let mut workers: Vec<Value> = workers.into_values().collect();
    workers.sort_by_key(|w| str_of(w, "name").to_lowercase());
    let label = if ours { "SiteMonitor workforce backup".into() } else { format!("Service 5 backup (v{version})") };
    Ok(Normalized { label, workers, collections, legacy_snapshots: vec![] })
}

/// Service 5's per-worker backfills (`normalizeBackupData`), minus random ids:
/// a missing `workerId` gets one derived from the A# so re-imports match.
fn clean_worker(w: Value) -> Option<Value> {
    let Value::Object(mut w) = w else { return None };
    let a = w.get("aNumber").and_then(scalar).or_else(|| w.get("id").and_then(scalar)).unwrap_or_default();
    let name = w.get("name").and_then(Value::as_str).unwrap_or_default().trim().to_string();
    if a.is_empty() && name.is_empty() {
        return None;
    }
    w.insert("aNumber".into(), json!(a));
    if name.is_empty() {
        w.insert("name".into(), json!(format!("Volunteer {a}")));
    }
    let status = w.get("dailyStatus").or_else(|| w.get("status")).and_then(Value::as_str).unwrap_or("Present").to_string();
    w.entry("dailyStatus").or_insert(json!(status));
    w.entry("status").or_insert(json!(status));
    for (k, d) in [("projectGroup", ""), ("projectRole", "Member"), ("visitationGroup", ""), ("visitationRole", "Member"), ("skillLevel", ""), ("subDepartment", "")] {
        w.entry(k).or_insert(json!(d));
    }
    for k in ["attendanceHistory", "assignmentHistory"] {
        if !w.get(k).is_some_and(Value::is_array) {
            w.insert(k.into(), json!([]));
        }
    }
    if w.get("workerId").and_then(scalar).is_none() {
        let seed = if a.is_empty() { format!("name:{}", w["name"].as_str().unwrap_or_default()) } else { format!("a#:{a}") };
        w.insert("workerId".into(), json!(stable_uuid(&seed).to_string()));
    }
    Some(Value::Object(w))
}

/// v5 vaccine records (`workerId`, `dateGiven`, `nextDueDate`) → v6+ shape.
fn migrate_vaccine(v: Value) -> Value {
    let (Some(wid), true) = (v.get("workerId").and_then(scalar), v.get("volunteerId").is_none()) else { return v };
    let given = str_of(&v, "dateGiven");
    let given = if given.is_empty() { str_of(&v, "dateAdministered") } else { given };
    let due = str_of(&v, "nextDueDate");
    let interval = match (chrono::NaiveDate::parse_from_str(&given, "%Y-%m-%d"), chrono::NaiveDate::parse_from_str(&due, "%Y-%m-%d")) {
        (Ok(g), Ok(d)) => json!((d - g).num_days()),
        _ => Value::Null,
    };
    let name = str_of(&v, "vaccineName");
    json!({
        "id": v.get("id").cloned().unwrap_or_else(|| json!(format!("vx_{}", &hex(&Sha256::digest(v.to_string().as_bytes()))[..12]))),
        "volunteerId": wid,
        "vaccineName": if name.is_empty() { "Vaccine".to_string() } else { name },
        "dateAdministered": given,
        "lotNumber": str_of(&v, "lotNumber"),
        "administeredBy": str_of(&v, "administeredBy"),
        "requiresBooster": !due.is_empty(),
        "boosterIntervalDays": interval,
        "boosterDueDate": if due.is_empty() { v.get("boosterDueDate").cloned().unwrap_or(Value::Null) } else { json!(due) },
        "boosterDateReceived": str_of(&v, "boosterDateReceived"),
        "notes": str_of(&v, "notes"),
    })
}

/// Items keep their own id; id-less ones (e.g. duty rosters) are keyed by
/// content, so the same item from two key generations collapses into one.
fn item_id(v: &Value) -> String {
    for k in ["id", "logId", "recordId", "dateStr", "workerId"] {
        if let Some(s) = v.get(k).and_then(scalar) {
            return s;
        }
    }
    format!("h{}", &hex(&Sha256::digest(v.to_string().as_bytes()))[..20])
}

/// Last-write-wins clock: the newest of the edit timestamps Service 5 uses.
fn stamp(v: &Value) -> i64 {
    ["updatedAt", "_lastModified", "updated_at", "savedAt", "timestamp", "createdAt"]
        .iter()
        .filter_map(|k| v.get(*k))
        .filter_map(|t| match t {
            Value::Number(n) => n.as_i64(),
            Value::String(s) => DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp_millis()).or_else(|| s.parse().ok()),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

fn leading_int(v: &Value) -> Option<u32> {
    match v {
        Value::Number(n) => n.as_u64().map(|n| n as u32),
        Value::String(s) => s.trim().split(|c: char| !c.is_ascii_digit()).next().and_then(|d| d.parse().ok()),
        _ => None,
    }
}

fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn str_of(v: &Value, k: &str) -> String {
    v.get(k).and_then(scalar).unwrap_or_default()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A UUID that is always the same for the same text.
pub fn stable_uuid(text: &str) -> Uuid {
    let h = Sha256::digest(text.as_bytes());
    let mut b = [0u8; 16];
    b.copy_from_slice(&h[..16]);
    uuid::Builder::from_random_bytes(b).into_uuid()
}

/// The typed columns of one worker row, taken from a Service 5 record.
fn worker_row(w: &Value) -> Value {
    let wid = str_of(w, "workerId");
    let id = Uuid::parse_str(&wid).unwrap_or_else(|_| stable_uuid(&format!("wid:{wid}")));
    let trade = match str_of(w, "trade") {
        t if !t.is_empty() => t,
        _ => w.get("trades").and_then(Value::as_array).and_then(|a| a.first()).and_then(scalar).unwrap_or_default(),
    };
    let role = match str_of(w, "groupRole") {
        r if !r.is_empty() => r,
        _ => str_of(w, "projectRole"),
    };
    let status = WorkerStatus::from_legacy(&match str_of(w, "dailyStatus") {
        s if !s.is_empty() => s,
        _ => str_of(w, "status"),
    });
    let archived = [w.get("archived"), w.get("isArchived")].into_iter().flatten().any(|v| v.as_bool() == Some(true));
    json!({
        "id": id, "a_number": str_of(w, "aNumber"), "name": str_of(w, "name"), "phone": str_of(w, "phone"),
        "trade": trade, "department": str_of(w, "department"), "area": str_of(w, "area"),
        "project_group": str_of(w, "projectGroup"), "group_role": role, "daily_status": enc(&status),
        "archived": archived, "profile": w,
    })
}

// ------------------------------------------------------------------ writing

/// Imports normalized data. Merge adds and updates (never deletes). Replace
/// swaps each collection the file has for the file's copy. Either way, a
/// collection that is empty in the file never wipes one that has data.
pub async fn import(db: &DatabaseConnection, data: Normalized, mode: ImportMode, allow_empty: bool) -> AppResult<WorkforceImportReport> {
    if data.workers.is_empty() && data.collections.values().all(Vec::is_empty) && !allow_empty {
        return Err(AppError::bad("This file has no workforce data in it, so nothing was changed."));
    }
    let txn = db.begin().await?;
    let existing_workers = count(&txn, "SELECT COUNT(*) AS n FROM workers").await?;
    let snapshot_id = Some(snapshot_in(&txn, "before import").await?);
    let mut report = WorkforceImportReport { detected: data.label.clone(), snapshot_id, ..Default::default() };

    // Workers.
    if data.workers.is_empty() {
        if existing_workers > 0 {
            report.kept_existing.push("workers".into());
        }
    } else {
        // A# matches keep the ids of workers that were added in SiteMonitor first.
        let by_a: HashMap<String, Uuid> = query_pairs(&txn, "SELECT a_number, id::text FROM workers WHERE a_number <> ''")
            .await?
            .into_iter()
            .filter_map(|(a, id)| Uuid::parse_str(&id).ok().map(|id| (a, id)))
            .collect();
        let rows: Vec<Value> = data
            .workers
            .iter()
            .map(|w| {
                let mut r = worker_row(w);
                if let Some(id) = by_a.get(&str_of(w, "aNumber")) {
                    r["id"] = json!(id);
                    r["profile"]["workerId"] = json!(id);
                }
                r
            })
            .collect();
        if mode == ImportMode::Replace {
            txn.execute_unprepared("DELETE FROM workers").await?;
        }
        exec(&txn, UPSERT_WORKERS, Value::Array(rows)).await?;
        report.workers = data.workers.len();
    }

    // Everything else.
    let stored: HashMap<String, i64> = query_pairs(&txn, "SELECT collection, COUNT(*)::text FROM workforce_records GROUP BY collection")
        .await?
        .into_iter()
        .map(|(c, n)| (c, n.parse().unwrap_or(0)))
        .collect();
    for (c, items) in &data.collections {
        if items.is_empty() {
            if stored.get(c).copied().unwrap_or(0) > 0 {
                report.kept_existing.push(c.clone());
            }
            continue;
        }
        if mode == ImportMode::Replace {
            exec(&txn, "DELETE FROM workforce_records WHERE collection = ($1::jsonb ->> 0)", json!([c])).await?;
        }
        let rows: Vec<Value> = items.iter().map(|(id, v)| json!({ "c": c, "id": id, "v": v })).collect();
        exec(&txn, UPSERT_RECORDS, Value::Array(rows)).await?;
        report.collections.push((c.clone(), items.len()));
    }
    // Service 5's own daily snapshots become restorable snapshots here (once each).
    for (date, snap) in &data.legacy_snapshots {
        let reason = format!("Service 5 daily snapshot {date}");
        let workers = snap["workers"].as_array().map(Vec::len).unwrap_or(0) as i32;
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO workforce_snapshots (id, taken_at, reason, workers, data)
             SELECT $1, ($2 || 'T23:59:00')::timestamptz, $3, $4, $5
             WHERE NOT EXISTS (SELECT 1 FROM workforce_snapshots WHERE reason = $3)",
            [Uuid::new_v4().into(), date.clone().into(), reason.into(), workers.into(), snap.clone().into()],
        ))
        .await?;
    }
    txn.commit().await?;
    Ok(report)
}

const UPSERT_WORKERS: &str = r#"
INSERT INTO workers (id, a_number, name, phone, trade, department, area, project_group, group_role, daily_status, archived, profile, updated_at)
SELECT (e->>'id')::uuid, e->>'a_number', e->>'name', e->>'phone', e->>'trade', e->>'department', e->>'area',
       e->>'project_group', e->>'group_role', e->>'daily_status', (e->>'archived')::boolean, e->'profile', now()
FROM jsonb_array_elements($1::jsonb) e
ON CONFLICT (id) DO UPDATE SET a_number = EXCLUDED.a_number, name = EXCLUDED.name, phone = EXCLUDED.phone,
    trade = EXCLUDED.trade, department = EXCLUDED.department, area = EXCLUDED.area, project_group = EXCLUDED.project_group,
    group_role = EXCLUDED.group_role, daily_status = EXCLUDED.daily_status, archived = EXCLUDED.archived,
    profile = EXCLUDED.profile, updated_at = now()"#;

const UPSERT_RECORDS: &str = r#"
INSERT INTO workforce_records (collection, id, data, updated_at)
SELECT e->>'c', e->>'id', e->'v', now() FROM jsonb_array_elements($1::jsonb) e
ON CONFLICT (collection, id) DO UPDATE SET data = EXCLUDED.data, updated_at = now()"#;

/// Everything as one file in Service 5's own layout (so it stays readable by
/// the same importer and by Service 5-style tools), marked as ours.
pub async fn export<C: ConnectionTrait>(db: &C) -> AppResult<Value> {
    let mut out = Map::new();
    out.insert("format".into(), json!("sitemonitor-workforce"));
    out.insert("version".into(), json!(11));
    out.insert("exportedAt".into(), json!(Utc::now().to_rfc3339()));
    let workers = query_json(db, "SELECT COALESCE(jsonb_agg(profile || jsonb_build_object('workerId', id::text) ORDER BY name), '[]'::jsonb) AS j FROM workers").await?;
    out.insert("workers".into(), workers);
    let per = query_json(
        db,
        "SELECT COALESCE(jsonb_object_agg(collection, items), '{}'::jsonb) AS j FROM (
           SELECT collection, jsonb_agg(data ORDER BY id) AS items FROM workforce_records GROUP BY collection) t",
    )
    .await?;
    if let Value::Object(per) = per {
        for (c, items) in per {
            // A dictionary collection is stored as its single "_" row.
            let v = match items {
                Value::Array(mut a) if is_dict(&c) && a.len() == 1 && a[0].is_object() => a.remove(0),
                other => other,
            };
            out.insert(c, v);
        }
    }
    Ok(Value::Object(out))
}

fn is_dict(c: &str) -> bool {
    matches!(c, "project_groups_meta" | "custom_datalists")
}

// ------------------------------------------------------------------ records

/// Health data: only people with "Clinic ledger" may read or change these.
pub const CLINIC: &[&str] = &["health_events", "volunteer_health_logs", "volunteer_vaccines", "volunteer_medications", "contact_traces", "medical_audit_logs"];

/// Collections the screens may read and edit (the audit trail is read-only).
pub fn known(c: &str) -> bool {
    CLINIC.contains(&c)
        || matches!(
            c,
            "attendance_log" | "assignment_log" | "duty_areas" | "duty_schedules" | "ot_entries" | "visitations" | "visitation_groups"
                | "ministers" | "signatories" | "history" | "group_links" | "project_groups_meta" | "custom_datalists" | "area_zones"
        )
}

pub async fn list_records<C: ConnectionTrait>(db: &C, collection: &str) -> AppResult<Vec<Value>> {
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT data FROM workforce_records WHERE collection = $1 ORDER BY updated_at DESC, id",
            [collection.into()],
        ))
        .await?;
    Ok(rows.into_iter().filter_map(|r| r.try_get::<Value>("", "data").ok()).collect())
}

/// Writes one record. Last write wins; the record is stamped with the time and
/// the device that wrote it (so that device can ignore its own echo).
pub async fn put_record<C: ConnectionTrait>(db: &C, collection: &str, id: &str, mut data: Value, client_id: Option<&str>) -> AppResult<Value> {
    if !data.is_object() {
        return Err(AppError::bad("A record must be an object."));
    }
    if id.is_empty() || id.len() > 200 {
        return Err(AppError::bad("That record id isn't valid."));
    }
    if id != DICT_ID {
        data["id"] = json!(id);
    }
    data["updatedAt"] = json!(Utc::now().to_rfc3339());
    if let Some(c) = client_id {
        data["_clientId"] = json!(c);
    }
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO workforce_records (collection, id, data, updated_at) VALUES ($1, $2, $3, now())
         ON CONFLICT (collection, id) DO UPDATE SET data = EXCLUDED.data, updated_at = now()",
        [collection.into(), id.into(), data.clone().into()],
    ))
    .await?;
    Ok(data)
}

pub async fn delete_record<C: ConnectionTrait>(db: &C, collection: &str, id: &str) -> AppResult<()> {
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM workforce_records WHERE collection = $1 AND id = $2",
        [collection.into(), id.into()],
    ))
    .await?;
    Ok(())
}

/// Appends a log entry (attendance, assignment, medical audit) with a fresh id.
pub async fn append<C: ConnectionTrait>(db: &C, collection: &str, entry: Value) -> AppResult<()> {
    let id = format!("{}_{}", collection.split('_').next().unwrap_or("log"), Uuid::new_v4().simple());
    put_record(db, collection, &id, entry, None).await.map(|_| ())
}

/// Saves the current workforce data so it can be restored later.
pub async fn snapshot(db: &DatabaseConnection, reason: &str) -> AppResult<Uuid> {
    let txn = db.begin().await?;
    let id = snapshot_in(&txn, reason).await?;
    txn.commit().await?;
    Ok(id)
}

async fn snapshot_in<C: ConnectionTrait>(db: &C, reason: &str) -> AppResult<Uuid> {
    let id = Uuid::new_v4();
    let data = export(db).await?;
    let workers = data["workers"].as_array().map(Vec::len).unwrap_or(0) as i32;
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO workforce_snapshots (id, reason, workers, data) VALUES ($1, $2, $3, $4)",
        [id.into(), reason.into(), workers.into(), data.into()],
    ))
    .await?;
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM workforce_snapshots WHERE id NOT IN (SELECT id FROM workforce_snapshots ORDER BY taken_at DESC LIMIT $1)",
        [KEEP_SNAPSHOTS.into()],
    ))
    .await?;
    Ok(id)
}

async fn exec<C: ConnectionTrait>(db: &C, sql: &str, arg: Value) -> AppResult<()> {
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, [arg.into()])).await?;
    Ok(())
}

async fn count<C: ConnectionTrait>(db: &C, sql: &str) -> AppResult<i64> {
    let row = db.query_one(Statement::from_string(DatabaseBackend::Postgres, sql)).await?;
    Ok(row.and_then(|r| r.try_get::<i64>("", "n").ok()).unwrap_or(0))
}

async fn query_json<C: ConnectionTrait>(db: &C, sql: &str) -> AppResult<Value> {
    let row = db.query_one(Statement::from_string(DatabaseBackend::Postgres, sql)).await?;
    Ok(row.and_then(|r| r.try_get::<Value>("", "j").ok()).unwrap_or(Value::Null))
}

async fn query_pairs<C: ConnectionTrait>(db: &C, sql: &str) -> AppResult<Vec<(String, String)>> {
    let rows = db.query_all(Statement::from_string(DatabaseBackend::Postgres, sql)).await?;
    Ok(rows.into_iter().filter_map(|r| Some((r.try_get_by_index::<String>(0).ok()?, r.try_get_by_index::<String>(1).ok()?))).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_generation_lands_in_one_collection() {
        for k in ["workers", "workersData", "wc_workers_data"] {
            assert_eq!(canonical(k), "workers");
        }
        assert_eq!(canonical("wc_volunteer_health_logs"), "volunteer_health_logs");
        assert_eq!(canonical("wc_ministers_data"), "ministers");
        assert_eq!(canonical("wc_history_logs"), "history");
        assert_eq!(canonical("otEntries"), "ot_entries");
        assert_eq!(canonical("vaccines"), "volunteer_vaccines");
    }

    #[test]
    fn duplicates_merge_newest_wins_and_v1_arrays_work() {
        let n = normalize(json!({
            "version": "4.0",
            "ot_entries": [{ "id": "a", "hours": 1, "updatedAt": "2026-01-01T00:00:00Z" }],
            "wc_ot_entries": [{ "id": "a", "hours": 3, "updatedAt": "2026-02-01T00:00:00Z" }, { "id": "b", "hours": 2 }],
            "project_groups_meta": { " CC | Tent ": " Site A " },
            "wc_dashboard_dismissed_event_ids": ["x"],
            "workers": [{ "aNumber": "A1", "name": "Ana" }, { "name": "" }],
        }))
        .unwrap();
        let ot = &n.collections["ot_entries"];
        assert_eq!(ot.len(), 2);
        assert_eq!(ot.iter().find(|(id, _)| id == "a").unwrap().1["hours"], 3);
        assert_eq!(n.collections["project_groups_meta"][0].1["CC | Tent"], "Site A");
        assert!(!n.collections.contains_key("dashboard_dismissed_event_ids"));
        assert_eq!(n.workers.len(), 1, "a worker with no name and no A# is dropped");
        assert_eq!(n.label, "Service 5 backup (v4)");
        // Same A#, same id on every import.
        assert_eq!(n.workers[0]["workerId"], json!(stable_uuid("a#:A1").to_string()));

        let v1 = normalize(json!([{ "id": "7", "name": "Jun", "status": "Passes" }])).unwrap();
        assert_eq!(v1.label, "Service 5 backup (v1)");
        let row = worker_row(&v1.workers[0]);
        assert_eq!(row["a_number"], "7");
        assert_eq!(row["daily_status"], "OnPass");
    }

    #[test]
    fn old_vaccine_records_are_upgraded() {
        let v = migrate_vaccine(json!({ "id": "v1", "workerId": "w9", "dateGiven": "2026-01-01", "nextDueDate": "2026-01-31" }));
        assert_eq!(v["volunteerId"], "w9");
        assert_eq!(v["boosterIntervalDays"], 30);
        assert_eq!(v["requiresBooster"], true);
    }

    #[test]
    fn agwcz_round_trip_wrong_passphrase_and_tampering() {
        // Encrypted the way Service 5's js/crypto.js does (fewer iterations to keep the test fast).
        let plain = r#"{"version":11,"workers":[{"aNumber":"A1","name":"Ana"}]}"#;
        let (salt, iv) = ([7u8; 16], [9u8; 12]);
        let mut key = [0u8; 32];
        pbkdf2::pbkdf2_hmac::<Sha256>(b"open sesame", &salt, 2_000, &mut key);
        let data = Aes256Gcm::new_from_slice(&key).unwrap().encrypt(Nonce::from_slice(&iv), plain.as_bytes()).unwrap();
        let env = json!({
            "_isAgwcEncrypted": true, "format": "agwcz", "version": 1, "iterations": 2000,
            "salt": B64.encode(salt), "iv": B64.encode(iv), "data": B64.encode(&data),
            "sha256": hex(&Sha256::digest(plain.as_bytes())),
        });
        let n = read(env.to_string().as_bytes(), Some("open sesame")).unwrap();
        assert_eq!(n.workers.len(), 1);
        assert!(n.label.starts_with("Encrypted"));
        assert!(read(env.to_string().as_bytes(), Some("wrong")).is_err());
        assert!(read(env.to_string().as_bytes(), None).is_err());
        let mut bad = env.clone();
        bad["sha256"] = json!("00");
        assert!(read(bad.to_string().as_bytes(), Some("open sesame")).is_err());
    }
}
