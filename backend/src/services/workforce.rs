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

use crate::error::{AppError, AppResult};
use crate::services::vault;

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
    /// Where the backup came from and when (Service 5 `syncMetadata`), for the merge review.
    pub source_device: String,
    pub exported_at: String,
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
/// The reverse of `decrypt_agwcz`: an encrypted backup in the format Service 5
/// writes (PBKDF2-HMAC-SHA256, 600,000 rounds, AES-256-GCM, SHA-256 of the plain text).
pub fn encrypt_agwcz(data: &Value, passphrase: &str) -> AppResult<Value> {
    use aes_gcm::aead::{rand_core::RngCore, OsRng};
    if passphrase.chars().count() < 8 {
        return Err(AppError::bad("Use a passphrase of at least 8 characters."));
    }
    let plain = serde_json::to_vec(data).map_err(|e| AppError::bad(e.to_string()))?;
    let (mut salt, mut iv) = ([0u8; 16], [0u8; 12]);
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut iv);
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), &salt, 600_000, &mut key);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| AppError::bad("Could not encrypt the backup."))?;
    let ct = cipher.encrypt(Nonce::from_slice(&iv), plain.as_ref()).map_err(|_| AppError::bad("Could not encrypt the backup."))?;
    Ok(json!({
        "_isAgwcEncrypted": true, "format": "agwcz", "version": 1, "algo": "AES-GCM", "kdf": "PBKDF2-HMAC-SHA256", "iterations": 600_000,
        "salt": B64.encode(salt), "iv": B64.encode(iv), "sha256": hex(&Sha256::digest(&plain)), "data": B64.encode(ct),
    }))
}

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
fn custom_list_field(c: &str) -> Option<&'static str> {
    Some(match c {
        "custom_trades" => "trades",
        "custom_trade_groups" => "tradeGroups",
        "custom_housing_groups" => "housingGroups",
        "custom_work_residences" => "workResidences",
        "custom_woodland_shops" => "woodlandShops",
        _ => return None,
    })
}

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
    let source_device = obj.get("syncMetadata").map(|m| str_of(m, "sourceDeviceId")).unwrap_or_default();
    let exported_at = [obj.get("syncMetadata").map(|m| str_of(m, "exportTimestamp")).unwrap_or_default(), obj.get("exportedAt").and_then(scalar).unwrap_or_default()]
        .into_iter()
        .find(|x| !x.is_empty())
        .unwrap_or_default();
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
        // Service 5 also keeps each pick list under its own key (wc_custom_trades, …):
        // they become fields of the one custom_datalists record.
        if let Some(field) = custom_list_field(&c) {
            if let Value::Array(items) = value {
                let slot = dicts.entry("custom_datalists".into()).or_default();
                if !items.is_empty() && slot.get(field).is_none_or(|v| v.as_array().is_none_or(Vec::is_empty)) {
                    slot.insert(field.into(), Value::Array(items));
                }
            }
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
                    let id = list_key(&c, &item);
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
        let mut v: Vec<(String, Value)> = items.into_values().map(|item| (item_id(&item), item)).collect();
        v.sort_by(|a, b| a.0.cmp(&b.0));
        collections.insert(c, v);
    }
    for (c, map) in dicts {
        collections.entry(c).or_default().push((DICT_ID.into(), Value::Object(map)));
    }
    let mut workers: Vec<Value> = workers.into_values().collect();
    workers.sort_by_key(|w| str_of(w, "name").to_lowercase());
    let label = if ours { "SiteMonitor workforce backup".into() } else { format!("Service 5 backup (v{version})") };
    Ok(Normalized { label, workers, collections, legacy_snapshots: vec![], source_device, exported_at })
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

/// Where two key generations hold the same item. Visitation groups are re-created
/// with new ids in each generation (Service 5 shows only `wc_visitation_groups`),
/// so a group is one per name; everything else is one per id.
fn list_key(collection: &str, item: &Value) -> String {
    match collection {
        "visitation_groups" if !str_of(item, "name").trim().is_empty() => format!("name:{}", str_of(item, "name").trim().to_lowercase()),
        _ => item_id(item),
    }
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
        // Service 5 wrote both 4 and 1.0.
        Value::Number(n) => n.as_u64().map(|n| n as u32).or_else(|| n.as_f64().filter(|f| *f >= 0.0).map(|f| f as u32)),
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

/// One person as stored: the full Service 5 record, decrypted.
#[derive(Clone, Debug)]
pub struct Person {
    pub id: Uuid,
    pub updated_at: DateTime<Utc>,
    pub p: Value,
}

fn flag(v: &Value) -> bool {
    v.as_bool() == Some(true) || v.as_str() == Some("true")
}

/// What the screens show for a person, worked out from the stored record
/// (the record is encrypted, so this happens here rather than in SQL).
pub fn worker_of(x: &Person) -> shared::Worker {
    let w = &x.p;
    let s = |k: &str| str_of(w, k);
    let first = |a: &str, b: &str| if s(a).is_empty() { s(b) } else { s(a) };
    let trade = match s("trade") {
        t if !t.is_empty() => t,
        _ => w.get("trades").and_then(Value::as_array).and_then(|a| a.first()).and_then(scalar).unwrap_or_default(),
    };
    let plan = |k: &str| w["tomorrowAssignment"].get(k).and_then(scalar).unwrap_or_default();
    let tomorrow = match &w["tomorrowAssignment"] {
        Value::Object(o) => o.get("specificTask").and_then(scalar).unwrap_or_default(),
        Value::String(t) => t.clone(),
        _ => String::new(),
    };
    shared::Worker {
        id: x.id,
        a_number: s("aNumber"),
        name: s("name"),
        phone: s("phone"),
        trade,
        department: s("department"),
        area: s("area"),
        project_group: s("projectGroup"),
        group_role: first("groupRole", "projectRole"),
        status: WorkerStatus::from_legacy(&first("dailyStatus", "status")),
        archived: flag(&w["archived"]) || flag(&w["isArchived"]),
        updated_at: x.updated_at,
        has_photo: s("photoUrl").starts_with("data:image/"),
        tomorrow,
        tomorrow_area: plan("area"),
        tomorrow_project: plan("project"),
        tomorrow_status: plan("dailyStatus"),
        tomorrow_reason: plan("absentReason"),
        residence: first("workResidence", "residence"),
        task: s("specificTask"),
        project: s("project"),
        remarks: s("absentReason"),
        spouse: s("spouseName"),
        housing: s("visitationHousing"),
        visit_group: s("visitationGroup"),
        visit_role: s("visitationRole"),
        trade_group: s("tradeGroup"),
        trade_role: s("tradeRole"),
        archive_category: s("archiveCategory"),
        medical: s("medical"),
        birthday: s("birthday"),
        hourly_rate: w["hourlyRate"].as_f64().or_else(|| s("hourlyRate").parse().ok()),
        skill_level: s("skillLevel"),
        job_order: !(s("jOrder") == "NO" || w["issueJobOrder"].as_bool() == Some(false) || s("issueJobOrder") == "false"),
        assigned_today: flag(&w["isAssignedToday"]),
        task_since: s("taskStartedAt"),
        stale_off: flag(&w["disableStaleAlerts"]),
    }
}

fn person_of(r: &sea_orm::QueryResult) -> AppResult<Person> {
    Ok(Person { id: r.try_get("", "id")?, updated_at: r.try_get("", "updated_at")?, p: vault::open(r.try_get("", "profile")?)? })
}

/// Everyone, decrypted.
pub async fn people<C: ConnectionTrait>(db: &C) -> AppResult<Vec<Person>> {
    let rows = db.query_all(Statement::from_string(DatabaseBackend::Postgres, "SELECT id, updated_at, profile FROM workers")).await?;
    rows.iter().map(person_of).collect()
}

pub async fn person<C: ConnectionTrait>(db: &C, id: Uuid) -> AppResult<Option<Person>> {
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres, "SELECT id, updated_at, profile FROM workers WHERE id = $1", [id.into()])).await?;
    row.as_ref().map(person_of).transpose()
}

/// Stores a person encrypted. The plain columns of the table are left blank,
/// so nothing about the person can be read from the database without the key.
pub async fn save_person<C: ConnectionTrait>(db: &C, id: Uuid, p: &Value) -> AppResult<()> {
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO workers (id, a_number, name, phone, trade, department, area, project_group, group_role, daily_status, archived, profile, updated_at)
         VALUES ($1, '', '', '', '', '', '', '', '', 'Present', false, $2, now())
         ON CONFLICT (id) DO UPDATE SET a_number = '', name = '', phone = '', trade = '', department = '', area = '', project_group = '',
           group_role = '', daily_status = 'Present', archived = false, profile = EXCLUDED.profile, updated_at = now()",
        [id.into(), vault::seal(p)?.into()],
    ))
    .await?;
    Ok(())
}

/// Encrypts personnel data stored before encryption was switched on (runs at start-up).
pub async fn seal_existing(db: &DatabaseConnection) -> AppResult<usize> {
    let mut n = 0;
    let rows = db.query_all(Statement::from_string(DatabaseBackend::Postgres, "SELECT id, updated_at, profile FROM workers WHERE jsonb_typeof(profile) <> 'string'")).await?;
    for r in &rows {
        let x = person_of(r)?;
        save_person(db, x.id, &x.p).await?;
        n += 1;
    }
    let rows = db
        .query_all(Statement::from_string(DatabaseBackend::Postgres, "SELECT collection, id, data FROM workforce_records WHERE jsonb_typeof(data) <> 'string'"))
        .await?;
    for r in &rows {
        let (c, id, data): (String, String, Value) = (r.try_get("", "collection")?, r.try_get("", "id")?, r.try_get("", "data")?);
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE workforce_records SET data = $3 WHERE collection = $1 AND id = $2",
            [c.into(), id.into(), vault::seal(&data)?.into()],
        ))
        .await?;
        n += 1;
    }
    let rows = db.query_all(Statement::from_string(DatabaseBackend::Postgres, "SELECT id, data FROM workforce_snapshots WHERE jsonb_typeof(data) <> 'string'")).await?;
    for r in &rows {
        let (id, data): (Uuid, Value) = (r.try_get("", "id")?, r.try_get("", "data")?);
        db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres, "UPDATE workforce_snapshots SET data = $2 WHERE id = $1", [id.into(), vault::seal(&data)?.into()]))
            .await?;
        n += 1;
    }
    Ok(n)
}

/// How much personnel data could be read from the database without the key:
/// rows not encrypted, and plain columns that are not blank. All should be 0.
pub async fn plain_counts(db: &DatabaseConnection) -> AppResult<Value> {
    let n = |sql: &'static str| async move {
        let row = db.query_one(Statement::from_string(DatabaseBackend::Postgres, sql)).await?;
        AppResult::Ok(row.and_then(|r| r.try_get::<i64>("", "n").ok()).unwrap_or(0))
    };
    Ok(json!({
        "workers": n("SELECT COUNT(*) AS n FROM workers").await?,
        "workers_plain": n("SELECT COUNT(*) AS n FROM workers WHERE jsonb_typeof(profile) <> 'string' OR name <> '' OR a_number <> '' OR phone <> ''").await?,
        "records": n("SELECT COUNT(*) AS n FROM workforce_records").await?,
        "records_plain": n("SELECT COUNT(*) AS n FROM workforce_records WHERE jsonb_typeof(data) <> 'string'").await?,
        "snapshots_plain": n("SELECT COUNT(*) AS n FROM workforce_snapshots WHERE jsonb_typeof(data) <> 'string'").await?,
    }))
}

/// A snapshot, decrypted.
pub async fn snapshot_data<C: ConnectionTrait>(db: &C, id: Uuid) -> AppResult<Option<Value>> {
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres, "SELECT data FROM workforce_snapshots WHERE id = $1", [id.into()])).await?;
    row.map(|r| vault::open(r.try_get("", "data")?)).transpose()
}

/// Every snapshot as (taken at, decrypted data), oldest first.
pub async fn all_snapshots<C: ConnectionTrait>(db: &C) -> AppResult<Vec<(DateTime<Utc>, Value)>> {
    let rows = db.query_all(Statement::from_string(DatabaseBackend::Postgres, "SELECT taken_at, data FROM workforce_snapshots ORDER BY taken_at")).await?;
    rows.iter().map(|r| Ok((r.try_get("", "taken_at")?, vault::open(r.try_get("", "data")?)?))).collect()
}

fn worker_id(w: &Value) -> Uuid {
    let wid = str_of(w, "workerId");
    Uuid::parse_str(&wid).unwrap_or_else(|_| stable_uuid(&format!("wid:{wid}")))
}

// ------------------------------------------------------------------ writing

/// Imports normalized data. Merge adds and updates (never deletes). Replace
/// swaps each collection the file has for the file's copy. Either way, a
/// collection that is empty in the file never wipes one that has data.
pub async fn import(db: &DatabaseConnection, data: Normalized, mode: ImportMode, allow_empty: bool, resolutions: &Map<String, Value>) -> AppResult<WorkforceImportReport> {
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
        let here = people(&txn).await?;
        let by_a: HashMap<String, Uuid> = here.iter().map(|x| (str_of(&x.p, "aNumber"), x.id)).filter(|(a, _)| !a.is_empty()).collect();
        if mode == ImportMode::Replace {
            txn.execute_unprepared("DELETE FROM workers").await?;
        }
        // Merge: someone already here is merged section by section (Service 5's sync merge).
        let local: Vec<Value> = here.iter().map(|x| { let mut p = x.p.clone(); p["workerId"] = json!(x.id.to_string()); p }).collect();
        let now = Utc::now().to_rfc3339();
        for w in &data.workers {
            let existing = (mode == ImportMode::Merge).then(|| crate::services::sync_merge::match_local(&local, w)).flatten();
            let (id, mut w) = match existing {
                Some(cw) => (Uuid::parse_str(&str_of(cw, "workerId")).unwrap_or_else(|_| worker_id(w)), crate::services::sync_merge::merge_pair(cw, w, resolutions, &now)),
                None => (by_a.get(&str_of(w, "aNumber")).copied().unwrap_or_else(|| worker_id(w)), w.clone()),
            };
            w["workerId"] = json!(id);
            save_person(&txn, id, &w).await?;
        }
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
        let rows: Vec<Value> = items.iter().map(|(id, v)| Ok(json!({ "c": c, "id": id, "v": vault::seal(v)? }))).collect::<AppResult<_>>()?;
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
            [Uuid::new_v4().into(), date.clone().into(), reason.into(), workers.into(), vault::seal(snap)?.into()],
        ))
        .await?;
    }
    txn.commit().await?;
    Ok(report)
}

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
    let mut workers: Vec<Value> = people(db)
        .await?
        .into_iter()
        .map(|x| {
            let mut p = x.p;
            p["workerId"] = json!(x.id.to_string());
            p
        })
        .collect();
    workers.sort_by_key(|w| str_of(w, "name").to_lowercase());
    out.insert("workers".into(), Value::Array(workers));
    let rows = db.query_all(Statement::from_string(DatabaseBackend::Postgres, "SELECT collection, data FROM workforce_records ORDER BY collection, id")).await?;
    let mut per: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for r in &rows {
        per.entry(r.try_get("", "collection")?).or_default().push(vault::open(r.try_get("", "data")?)?);
    }
    {
        for (c, items) in per {
            let items = Value::Array(items);
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
    rows.iter().map(|r| vault::open(r.try_get("", "data")?)).collect()
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
        [collection.into(), id.into(), vault::seal(&data)?.into()],
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
        [id.into(), reason.into(), workers.into(), vault::seal(&data)?.into()],
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

async fn query_pairs<C: ConnectionTrait>(db: &C, sql: &str) -> AppResult<Vec<(String, String)>> {
    let rows = db.query_all(Statement::from_string(DatabaseBackend::Postgres, sql)).await?;
    Ok(rows.into_iter().filter_map(|r| Some((r.try_get_by_index::<String>(0).ok()?, r.try_get_by_index::<String>(1).ok()?))).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key in Service 5's LEGACY_KEY_MAP (js/backup.js) lands where Service 5 puts it.
    #[test]
    fn service5_legacy_key_map_is_honoured() {
        let map = [
            ("workersData", "workers"), ("wc_workers_data", "workers"), ("workers_data", "workers"),
            ("signatoriesData", "signatories"), ("wc_signatories", "signatories"), ("signatories_data", "signatories"), ("wc_signatories_data", "signatories"),
            ("ministersData", "ministers"), ("wc_ministers", "ministers"), ("ministers_data", "ministers"), ("wc_ministers_data", "ministers"),
            ("visitationsData", "visitations"), ("wc_visitations", "visitations"), ("visitations_data", "visitations"), ("wc_visitations_data", "visitations"),
            ("visitationGroups", "visitation_groups"), ("visitationGroupsData", "visitation_groups"), ("wc_visitation_groups", "visitation_groups"), ("visitation_groups_data", "visitation_groups"),
            ("historyData", "history"), ("wc_history_data", "history"), ("history_data", "history"), ("wc_history_logs", "history"), ("history_logs", "history"),
            ("attendanceLog", "attendance_log"), ("wc_attendance_log", "attendance_log"), ("attendance_log_data", "attendance_log"),
            ("assignmentLog", "assignment_log"), ("wc_assignment_log", "assignment_log"), ("assignment_log_data", "assignment_log"),
            ("groupChanges", "group_changes"), ("group_changes_log", "group_changes"), ("group_changes_data", "group_changes"), ("wc_group_changes", "group_changes"),
            ("dutyAreas", "duty_areas"), ("wc_duty_areas", "duty_areas"), ("duty_areas_data", "duty_areas"),
            ("dutyRosters", "duty_rosters"), ("wc_duty_rosters", "duty_rosters"), ("duty_rosters_data", "duty_rosters"),
            ("dutySchedules", "duty_schedules"), ("wc_duty_schedules", "duty_schedules"), ("duty_schedules_data", "duty_schedules"),
            ("healthEvents", "health_events"), ("wc_health_events", "health_events"), ("health_events_data", "health_events"),
            ("contactTraces", "contact_traces"), ("wc_contact_traces", "contact_traces"), ("contact_traces_data", "contact_traces"),
            ("vaccines", "volunteer_vaccines"), ("wc_vaccines", "volunteer_vaccines"), ("volunteerVaccines", "volunteer_vaccines"), ("wc_volunteer_vaccines", "volunteer_vaccines"), ("volunteer_vaccines_data", "volunteer_vaccines"),
            ("volunteerMedications", "volunteer_medications"), ("wc_volunteer_medications", "volunteer_medications"),
            ("wc_volunteer_health_logs", "volunteer_health_logs"), ("wc_medical_audit_logs", "medical_audit_logs"), ("wc_ot_entries", "ot_entries"),
            ("wc_project_groups_meta", "project_groups_meta"), ("wc_custom_trades", "custom_trades"),
        ];
        for (from, to) in map {
            assert_eq!(canonical(from), to, "{from}");
        }
    }

    /// One backup per Service 5 generation, each with a person, a log entry and a list item.
    #[test]
    fn every_service5_backup_generation_imports() {
        let person = |a: &str| json!({ "aNumber": a, "name": format!("Person {a}"), "dailyStatus": "Present" });
        let generations = [
            ("v1 bare worker array", json!([person("1")])),
            ("v2 camelCase stores", json!({ "version": 2, "workersData": [person("1")], "attendanceLog": [{ "id": "att1", "aNumber": "1", "date": "2026-01-02", "newStatus": "Absent" }], "ministersData": [{ "id": "m1", "name": "Bro. A" }] })),
            ("v3 string version, v5 vaccine shape", json!({ "version": "3.0", "workers": [person("1")], "vaccines": [{ "workerId": "1", "vaccineName": "Flu", "dateGiven": "2026-01-01", "nextDueDate": "2026-01-31" }] })),
            ("v4 canonical stores with syncMetadata", json!({ "version": 4, "schemaVersion": 2, "syncMetadata": { "sourceDeviceId": "Station-7", "exportTimestamp": "2026-05-01T00:00:00Z" }, "workers": [person("1")], "duty_areas": [{ "id": "d1", "name": "Gate" }] })),
            ("v11 storage keys (service5.db records)", json!({ "version": 11, "workers": [person("1")], "wc_ot_entries": [{ "id": "ot1", "volunteerId": "1", "hours": 2 }], "wc_custom_woodland_shops": ["Glass Shop"] })),
            ("daily snapshot file (version 1.0)", json!({ "version": 1.0, "exportedAt": "2026-11-19T23:59:00Z", "workers": [person("1")], "dashboard_dismissed": { "eventIds": ["x"] }, "history": [{ "id": "h1", "dateStr": "Nov 19" }] })),
        ];
        for (label, raw) in generations {
            let n = normalize(raw).unwrap_or_else(|e| panic!("{label}: {e:?}"));
            assert_eq!(n.workers.len(), 1, "{label}");
            assert_eq!(str_of(&n.workers[0], "aNumber"), "1", "{label}");
            assert!(!str_of(&n.workers[0], "workerId").is_empty(), "{label}: a stable workerId is backfilled");
            assert_eq!(str_of(&n.workers[0], "projectRole"), "Member", "{label}: Service 5 backfills");
            assert!(n.workers[0]["attendanceHistory"].is_array(), "{label}");
            assert!(!n.collections.contains_key("dashboard_dismissed"), "{label}: per-device state is skipped");
        }
        let v2 = normalize(json!({ "version": 2, "workersData": [person("1")], "attendanceLog": [{ "id": "att1" }], "ministersData": [{ "id": "m1" }] })).unwrap();
        assert_eq!(v2.collections["attendance_log"].len(), 1);
        assert_eq!(v2.collections["ministers"].len(), 1);
        let v3 = normalize(json!({ "version": "3.0", "vaccines": [{ "workerId": "1", "vaccineName": "Flu", "dateGiven": "2026-01-01", "nextDueDate": "2026-01-31" }] })).unwrap();
        let vx = &v3.collections["volunteer_vaccines"][0].1;
        assert_eq!((vx["volunteerId"].as_str(), vx["boosterIntervalDays"].as_i64()), (Some("1"), Some(30)));
        let v4 = normalize(json!({ "version": 4, "syncMetadata": { "sourceDeviceId": "Station-7", "exportTimestamp": "2026-05-01T00:00:00Z" }, "workers": [] })).unwrap();
        assert_eq!((v4.source_device.as_str(), v4.exported_at.as_str()), ("Station-7", "2026-05-01T00:00:00Z"));
        assert_eq!(v4.label, "Service 5 backup (v4)");
        let v11 = normalize(json!({ "version": 11, "wc_custom_woodland_shops": ["Glass Shop"] })).unwrap();
        assert_eq!(v11.collections["custom_datalists"][0].1["woodlandShops"], json!(["Glass Shop"]));
        assert_eq!(normalize(json!({ "version": 4.0, "workers": [] })).unwrap().label, "Service 5 backup (v4)");
    }

    /// Service 5's encrypted .agwcz backups (js/crypto.js) open with the passphrase, and only with it.
    #[test]
    fn encrypted_agwcz_backups_round_trip() {
        let backup = json!({ "version": 4, "workers": [{ "aNumber": "9", "name": "Enc Person" }] });
        let env = encrypt_agwcz(&backup, "correct horse").unwrap();
        assert_eq!((env["_isAgwcEncrypted"].as_bool(), env["format"].as_str()), (Some(true), Some("agwcz")));
        let bytes = serde_json::to_vec(&env).unwrap();
        let n = read(&bytes, Some("correct horse")).unwrap();
        assert_eq!(n.workers.len(), 1);
        assert!(read(&bytes, Some("wrong pass")).is_err());
        assert!(read(&bytes, None).is_err());
    }

    #[test]
    fn visitation_groups_from_two_key_generations_are_one_per_name() {
        let n = normalize(json!({
            "visitation_groups": [{ "id": "vg_old", "name": "Group A", "createdAt": "2026-08-17T09:56:30Z", "notes": "old" }],
            "wc_visitation_groups": [{ "id": "vg_new", "name": "group a ", "createdAt": "2026-11-17T07:48:02Z", "notes": "new" }, { "id": "vg_b", "name": "Group B" }],
        }))
        .unwrap();
        let groups = &n.collections["visitation_groups"];
        assert_eq!(groups.len(), 2);
        assert!(groups.iter().any(|(_, g)| g["id"] == "vg_new" && g["notes"] == "new"));
    }

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
        let w = worker_of(&Person { id: Uuid::nil(), updated_at: Utc::now(), p: v1.workers[0].clone() });
        assert_eq!(w.a_number, "7");
        assert_eq!(w.status, WorkerStatus::OnPass);
    }

    #[test]
    fn old_vaccine_records_are_upgraded() {
        let v = migrate_vaccine(json!({ "id": "v1", "workerId": "w9", "dateGiven": "2026-01-01", "nextDueDate": "2026-01-31" }));
        assert_eq!(v["volunteerId"], "w9");
        assert_eq!(v["boosterIntervalDays"], 30);
        assert_eq!(v["requiresBooster"], true);
    }

    #[test]
    fn separate_pick_lists_fold_into_custom_datalists() {
        let n = normalize(json!({ "version": 11, "workers": [{ "aNumber": "1", "name": "A" }],
            "custom_datalists": { "trades": [] }, "wc_custom_trades": ["Mason"], "wc_custom_woodland_shops": ["Paint Shop"] }))
        .unwrap();
        let lists = &n.collections["custom_datalists"][0].1;
        assert_eq!(lists["trades"], json!(["Mason"]));
        assert_eq!(lists["woodlandShops"], json!(["Paint Shop"]));
        assert!(!n.collections.contains_key("custom_trades"));
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

// ------------------------------------------------------------------ parity check

/// Every leaf of `v` as (path, value); arrays index by position.
fn leaves(v: &Value, path: String, out: &mut Vec<(String, Value)>) {
    match v {
        Value::Object(m) if !m.is_empty() => m.iter().for_each(|(k, x)| leaves(x, if path.is_empty() { k.clone() } else { format!("{path}.{k}") }, out)),
        Value::Array(a) if !a.is_empty() => a.iter().enumerate().for_each(|(i, x)| leaves(x, format!("{path}[{i}]"), out)),
        _ => out.push((path, v.clone())),
    }
}

/// Follows a `a.b[2].c` path.
fn at<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(v, |cur, seg| {
        let (key, rest) = match seg.find('[') {
            Some(i) => (&seg[..i], &seg[i..]),
            None => (seg, ""),
        };
        let mut cur = if key.is_empty() { cur } else { cur.get(key)? };
        for n in rest.split(['[', ']']).filter(|s| !s.is_empty()) {
            cur = cur.get(n.parse::<usize>().ok()?)?;
        }
        Some(cur)
    })
}

fn blank(v: &Value) -> bool {
    v.is_null() || v.as_str() == Some("") || v.as_array().is_some_and(Vec::is_empty) || v.as_object().is_some_and(Map::is_empty)
}

/// Paths of `src` whose value `stored` doesn't hold (strings compared trimmed).
fn lost_paths(src: &Value, stored: &Value, skip: &[&str]) -> Vec<String> {
    let mut l = vec![];
    leaves(src, String::new(), &mut l);
    l.into_iter()
        .filter(|(p, _)| !skip.contains(&p.as_str()))
        .filter(|(p, v)| match (at(stored, p), v) {
            (Some(Value::String(a)), Value::String(b)) => a.trim() != b.trim(),
            // The importer keeps an A# as text ("123"); the number is the same.
            (Some(Value::String(a)), Value::Number(n)) => a.trim() != n.to_string(),
            (Some(g), v) => g != v,
            (None, v) => !blank(v),
        })
        .map(|(p, _)| p)
        .collect()
}

fn sample(paths: &[String]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    format!(" - fields: {}", paths.iter().take(12).cloned().collect::<Vec<_>>().join(", "))
}

/// Checks that everything in a Service 5 file (service5.db or JSON backup) is held
/// in this database: per collection, items missing and fields lost or changed.
/// Prints collection names, counts and field paths only, never personal values.
pub async fn parity(db: &DatabaseConnection, bytes: &[u8]) -> AppResult<String> {
    let raw = if bytes.starts_with(b"SQLite format 3\0") {
        read_sqlite(bytes)?.0
    } else {
        serde_json::from_slice(bytes).map_err(|_| AppError::bad("not a Service 5 file"))?
    };
    let stored = export(db).await?;
    let mut report = String::new();
    let mut total_lost = 0;
    // Every key generation of a collection is checked against where it lands; when two
    // generations hold the same item, only the newest copy has to be kept intact.
    let mut groups: BTreeMap<String, Vec<(String, Value)>> = BTreeMap::new();
    for (key, value) in raw.as_object().into_iter().flatten() {
        let c = canonical(key);
        if META.contains(&key.as_str()) || SKIP.contains(&c.as_str()) {
            continue;
        }
        groups.entry(c).or_default().push((key.clone(), value.clone()));
    }
    for (c, sources) in groups {
        let (target, field) = match custom_list_field(&c) {
            Some(f) => ("custom_datalists".to_string(), Some(f)),
            None => (c.clone(), None),
        };
        let have = stored.get(&target).cloned().unwrap_or(Value::Null);
        let keys = sources.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>().join(", ");
        let (msg, lost) = if let Some(field) = field {
            let src: Vec<Value> = sources.iter().filter_map(|(_, v)| v.as_array()).flatten().cloned().collect();
            let held = have[field].as_array().cloned().unwrap_or_default();
            (format!("{} items, {} held", src.len(), held.len()), src.iter().filter(|v| !held.contains(v)).count())
        } else if sources.iter().all(|(_, v)| v.is_object()) {
            let mut lost: Vec<String> = sources.iter().flat_map(|(_, v)| lost_paths(v, &have, &[])).collect();
            lost.sort();
            lost.dedup();
            let n = sources.iter().filter_map(|(_, v)| v.as_object()).map(Map::len).max().unwrap_or(0);
            (format!("{n} keys{}", sample(&lost)), lost.len())
        } else {
            let is_workers = c == "workers";
            let id_of = |v: &Value| if is_workers { v.get("aNumber").and_then(scalar).unwrap_or_default() } else { list_key(&c, v) };
            let mut newest: HashMap<String, Value> = HashMap::new();
            for item in sources.iter().filter_map(|(_, v)| v.as_array()).flatten().filter(|v| !v.is_null()) {
                let item = if c == "volunteer_vaccines" { migrate_vaccine(item.clone()) } else { item.clone() };
                let id = id_of(&item);
                if newest.get(&id).is_none_or(|old| stamp(old) <= stamp(&item)) {
                    newest.insert(id, item);
                }
            }
            let held: HashMap<String, &Value> = have.as_array().into_iter().flatten().map(|v| (id_of(v), v)).collect();
            let (mut missing, mut changed, mut paths) = (0, 0, vec![]);
            for (id, item) in &newest {
                match held.get(id) {
                    None => missing += 1,
                    Some(h) => {
                        let lost = lost_paths(item, h, if is_workers { &["workerId"] } else { &[] });
                        if !lost.is_empty() {
                            changed += 1;
                            paths.extend(lost.into_iter().map(|p| p.split('[').next().unwrap_or_default().to_string()));
                        }
                    }
                }
            }
            paths.sort();
            paths.dedup();
            (format!("{} items, {} held, {missing} missing, {changed} changed{}", newest.len(), held.len(), sample(&paths)), missing + changed)
        };
        total_lost += lost;
        report.push_str(&format!("{} {target:<24} <- {keys:<50} {msg}\n", if lost == 0 { "ok  " } else { "LOST" }));
    }
    report.push_str(if total_lost == 0 { "\nPARITY OK: every item and field in the file is held.\n" } else { "\nPARITY FAILED: see the LOST lines.\n" });
    Ok(report)
}

#[cfg(test)]
mod parity_tests {
    use super::*;

    #[test]
    fn lost_paths_finds_dropped_and_changed_fields() {
        let src = json!({ "a": 1, "b": { "c": "x ", "d": [1, 2] }, "e": "" });
        assert!(lost_paths(&src, &json!({ "a": 1, "b": { "c": "x", "d": [1, 2] }, "extra": 3 }), &[]).is_empty());
        assert_eq!(lost_paths(&src, &json!({ "a": 2, "b": { "d": [1] } }), &[]), vec!["a", "b.c", "b.d[1]"]);
    }
}
