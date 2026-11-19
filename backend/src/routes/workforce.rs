//! Workforce (merged from Service 5 DB): the people directory, legacy backup
//! import, backup download and snapshots.
//!
//! Personnel records are encrypted at rest (`services::vault`), so every
//! filter, count and lookup here works on the decrypted records in Rust.

use std::collections::{BTreeMap, HashMap, HashSet};

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use sea_orm::{ConnectionTrait, DatabaseBackend, EntityTrait, Statement};
use serde_json::{json, Map, Value};
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::services::workforce::{self, Person};
use crate::state::{AppState, CurrentUser};

/// Health details stay with the clinic: the Medical column is blank without "Clinic ledger".
fn for_user(mut w: Worker, user: &CurrentUser) -> Worker {
    if !user.0.can(Permission::ManageClinic) {
        w.medical.clear();
    }
    w
}

fn text(p: &Value, k: &str) -> String {
    match p.get(k) {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// Which workers a Site Monitor may see and update: those whose Service 5 area
/// is mapped to one of their zones, or whose project group works on a job in
/// those zones. Everyone else sees everyone. `None` = no limit.
async fn visible_ids(state: &AppState, user: &CurrentUser, people: &[Person]) -> AppResult<Option<HashSet<Uuid>>> {
    if user.0.role != UserRole::SiteMonitor || user.0.can(Permission::ManageWorkforce) {
        return Ok(None);
    }
    let parents: HashMap<Uuid, Option<Uuid>> = crate::entities::spatial_area::Entity::find().all(&state.db).await?.into_iter().map(|a| (a.id, a.parent_id)).collect();
    let mine = |zone: Uuid| shared::access::works_in(&user.0, zone, |z| parents.get(&z).copied().flatten());
    let zone_of = |v: &Value, k: &str| v[k].as_str().and_then(|s| Uuid::parse_str(s).ok());
    let areas: HashSet<String> =
        workforce::list_records(&state.db, "area_zones").await?.into_iter().filter(|v| zone_of(v, "zone_id").is_some_and(mine)).filter_map(|v| v["id"].as_str().map(str::to_string)).collect();
    let job_zone: HashMap<Uuid, Uuid> = crate::entities::work_item::Entity::find().all(&state.db).await?.into_iter().map(|w| (w.id, w.area_id)).collect();
    let groups: HashSet<String> = workforce::list_records(&state.db, "group_links")
        .await?
        .into_iter()
        .filter(|v| zone_of(v, "work_item_id").and_then(|j| job_zone.get(&j).copied()).is_some_and(mine))
        .filter_map(|v| v["id"].as_str().map(str::to_string))
        .collect();
    Ok(Some(people.iter().filter(|x| areas.contains(&text(&x.p, "area")) || groups.contains(&text(&x.p, "projectGroup"))).map(|x| x.id).collect()))
}

async fn require_visible(state: &AppState, user: &CurrentUser, id: Uuid) -> AppResult<()> {
    let people = workforce::people(&state.db).await?;
    match visible_ids(state, user, &people).await? {
        Some(ids) if !ids.contains(&id) => Err(AppError::forbidden("That person isn't in your zones. Ask the Office Admin to map their area to a zone.")),
        _ => Ok(()),
    }
}

/// Personal details fields the profile editor may change. Health ones need "Clinic ledger".
const PROFILE_FIELDS: &[&str] = &[
    "birthday", "civilStatus", "address", "residence", "workResidence", "language", "fatherName", "motherName", "spouseName",
    "spouseBirthday", "spouseContact", "spouseChurchDuty", "weddingDate", "children", "emergencyContact", "emergencyPhone",
    "emergencyRelationship", "skillLevel", "hourlyRate", "subDepartment", "specificTask", "tomorrowAssignment", "assignment",
    "visitationGroup", "visitationHousing", "visitationRole", "photoUrl", "project", "absentReason", "archiveCategory", "archiveReason",
    "jOrder", "trades", "tradeGroup", "assignedUnit", "disableStaleAlerts",
];
const HEALTH_FIELDS: &[&str] = &["bloodType", "medicalNotes", "maintenanceMeds", "healthClearance", "medical", "healthAndClearance"];

async fn person(state: &AppState, id: Uuid) -> AppResult<Person> {
    workforce::person(&state.db, id).await?.ok_or_else(|| AppError::not_found("worker"))
}

async fn one(state: &AppState, user: &CurrentUser, id: Uuid) -> AppResult<Worker> {
    Ok(for_user(workforce::worker_of(&person(state, id).await?), user))
}

/// Merges fields into a stored person (`profile || patch`) and saves it encrypted.
async fn patch(state: &AppState, id: Uuid, patch: &Value) -> AppResult<Person> {
    let mut x = person(state, id).await?;
    if let (Some(p), Some(add)) = (x.p.as_object_mut(), patch.as_object()) {
        for (k, v) in add {
            p.insert(k.clone(), v.clone());
        }
    }
    workforce::save_person(&state.db, id, &x.p).await?;
    Ok(x)
}

/// `GET /api/workers/{id}/photo` — the photo kept in the worker's record.
pub async fn photo(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>) -> AppResult<axum::response::Response> {
    use axum::response::IntoResponse;
    can_read_workforce(&user)?;
    let url = text(&person(&state, id).await?.p, "photoUrl");
    let (head, data) = url.split_once(',').ok_or_else(|| AppError::not_found("photo"))?;
    let mime = head.strip_prefix("data:").and_then(|h| h.strip_suffix(";base64")).filter(|m| m.starts_with("image/")).ok_or_else(|| AppError::not_found("photo"))?;
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(data).map_err(|_| AppError::not_found("photo"))?;
    Ok(([(axum::http::header::CONTENT_TYPE, mime.to_string()), (axum::http::header::CACHE_CONTROL, "private, max-age=300".to_string())], bytes).into_response())
}

/// `GET /api/workers/{id}/profile` — the personal details (health ones only with "Clinic ledger").
pub async fn profile(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    user.require(Permission::ManageWorkforce)?;
    let mut p = person(&state, id).await?.p;
    if let Some(m) = p.as_object_mut() {
        m.remove("attendanceHistory");
        m.remove("assignmentHistory");
        if !user.0.can(Permission::ManageClinic) {
            for k in HEALTH_FIELDS {
                m.remove(*k);
            }
        }
    }
    Ok(Json(p))
}

/// `PUT /api/workers/{id}/profile` — saves personal details; other fields are ignored.
pub async fn save_profile(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>, Json(body): Json<Value>) -> AppResult<Json<Worker>> {
    user.require(Permission::ManageWorkforce)?;
    let clinic = user.0.can(Permission::ManageClinic);
    let mut fields = Map::new();
    for (k, v) in body.as_object().into_iter().flatten() {
        if PROFILE_FIELDS.contains(&k.as_str()) || (clinic && HEALTH_FIELDS.contains(&k.as_str())) {
            fields.insert(k.clone(), v.clone());
        }
    }
    if let Some(p) = fields.get("photoUrl").and_then(Value::as_str) {
        if !p.is_empty() && (!p.starts_with("data:image/") || p.len() > 3_000_000) {
            return Err(AppError::bad("The photo must be a picture under 2 MB."));
        }
    }
    // Health details changed through a profile are kept in the clinic audit trail too.
    let health: Vec<String> = fields.keys().filter(|k| HEALTH_FIELDS.contains(&k.as_str())).cloned().collect();
    let x = patch(&state, id, &Value::Object(fields)).await?;
    if !health.is_empty() {
        workforce::append(
            &state.db,
            "medical_audit_logs",
            json!({ "action": "saved", "recordId": id, "details": format!("profile: {}", health.join(", ")), "editedBy": user.0.name, "timestamp": chrono::Utc::now().to_rfc3339() }),
        )
        .await?;
    }
    announce(&state, None).await;
    Ok(Json(for_user(workforce::worker_of(&x), &user)))
}

fn counts(items: impl Iterator<Item = String>, blank: &str) -> Vec<(String, u32)> {
    let mut m: HashMap<String, u32> = HashMap::new();
    for i in items {
        *m.entry(if i.is_empty() { blank.to_string() } else { i }).or_default() += 1;
    }
    let mut v: Vec<(String, u32)> = m.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v
}

/// `GET /api/workforce/summary` — the workforce dashboard.
pub async fn summary(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<WorkforceSummary>> {
    can_read_workforce(&user)?;
    let active: Vec<Worker> = workforce::people(&state.db).await?.iter().map(workforce::worker_of).filter(|w| !w.archived).collect();
    let trades = counts(active.iter().map(|w| w.trade.clone()), "No trade");
    let departments = counts(active.iter().map(|w| w.department.clone()), "No department");
    let since = (chrono::Local::now().date_naive() - chrono::Duration::days(13)).format("%Y-%m-%d").to_string();
    let mut days: BTreeMap<String, (HashSet<String>, HashSet<String>)> = BTreeMap::new();
    for e in workforce::list_records(&state.db, "attendance_log").await? {
        let d = text(&e, "date");
        if d < since {
            continue;
        }
        let slot = days.entry(d).or_default();
        match text(&e, "newStatus").as_str() {
            "Absent" => {
                slot.0.insert(text(&e, "aNumber"));
            }
            "Passes" | "On Pass" => {
                slot.1.insert(text(&e, "aNumber"));
            }
            _ => {}
        }
    }
    let trend = days.into_iter().map(|(d, (a, p))| (d, a.len() as u32, p.len() as u32)).collect();
    let month = chrono::Local::now().format("-%m-").to_string();
    let mut birthdays: Vec<(String, String)> = active.iter().filter(|w| w.birthday.contains(&month)).map(|w| (w.name.clone(), w.birthday.clone())).collect();
    birthdays.sort_by(|a, b| a.1.get(8..10).cmp(&b.1.get(8..10)));
    Ok(Json(WorkforceSummary { trades, departments, trend, birthdays }))
}

/// `GET /api/workforce/family-roster` — everyone's personal details for the
/// printed reports (family roster, records update forms). Health details only
/// with "Clinic ledger".
pub async fn family_roster(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Vec<Value>>> {
    user.require(Permission::ManageWorkforce)?;
    let clinic = user.0.can(Permission::ManageClinic);
    let mut out: Vec<Value> = workforce::people(&state.db)
        .await?
        .into_iter()
        .filter_map(|x| {
            let w = workforce::worker_of(&x);
            if w.archived {
                return None;
            }
            let mut p = x.p;
            let m = p.as_object_mut()?;
            for k in ["attendanceHistory", "assignmentHistory", "photoUrl"] {
                m.remove(k);
            }
            if !clinic {
                for k in HEALTH_FIELDS {
                    m.remove(*k);
                }
            }
            m.insert("id".into(), json!(x.id));
            m.insert("name".into(), json!(w.name));
            m.insert("aNumber".into(), json!(w.a_number));
            m.insert("group".into(), json!(w.project_group));
            m.insert("trade".into(), json!(w.trade));
            Some(p)
        })
        .collect();
    out.sort_by_key(|p| (text(p, "group"), text(p, "name").to_lowercase()));
    Ok(Json(out))
}

/// `GET /api/workers` — everyone who manages crew can see the list.
pub async fn list(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Vec<Worker>>> {
    if !user.0.can(Permission::ManageWorkforce) {
        user.require(Permission::ManageCrew)?;
    }
    let people = workforce::people(&state.db).await?;
    let visible = visible_ids(&state, &user, &people).await?;
    let mut out: Vec<Worker> =
        people.iter().filter(|x| visible.as_ref().is_none_or(|v| v.contains(&x.id))).map(|x| for_user(workforce::worker_of(x), &user)).collect();
    out.sort_by_key(|w| (w.archived, w.name.to_lowercase()));
    Ok(Json(out))
}

fn legacy(s: WorkerStatus) -> &'static str {
    match s {
        WorkerStatus::Present => "Present",
        WorkerStatus::Absent => "Absent",
        WorkerStatus::OnPass => "Passes",
    }
}

/// Checks the form and returns the Service 5-style fields kept in the record.
async fn check(state: &AppState, req: &SaveWorkerRequest, id: Option<Uuid>) -> AppResult<Value> {
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(AppError::bad("Please enter the person's name (up to 120 characters)."));
    }
    let a = req.a_number.trim();
    if !a.is_empty() {
        if let Some(other) = workforce::people(&state.db).await?.into_iter().find(|x| Some(x.id) != id && text(&x.p, "aNumber") == a) {
            return Err(AppError::bad(format!("A# {a} already belongs to {}.", text(&other.p, "name"))));
        }
    }
    Ok(json!({
        "aNumber": a, "name": name, "phone": req.phone.trim(), "trade": req.trade.trim(), "department": req.department.trim(),
        "area": req.area.trim(), "projectGroup": req.project_group.trim(), "groupRole": req.group_role.trim(),
        "dailyStatus": legacy(req.status), "status": legacy(req.status), "archived": req.archived, "isArchived": req.archived,
    }))
}

/// `POST /api/workers`
pub async fn create(State(state): State<AppState>, user: CurrentUser, Json(req): Json<SaveWorkerRequest>) -> AppResult<Json<Worker>> {
    user.require(Permission::ManageWorkforce)?;
    let id = Uuid::new_v4();
    let mut p = check(&state, &req, None).await?;
    p["workerId"] = json!(id);
    workforce::save_person(&state.db, id, &p).await?;
    announce(&state, None).await;
    Ok(Json(one(&state, &user, id).await?))
}

/// `PUT /api/workers/{id}` — the Service 5 fields stay in step.
pub async fn update(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>, Json(req): Json<SaveWorkerRequest>) -> AppResult<Json<Worker>> {
    user.require(Permission::ManageWorkforce)?;
    person(&state, id).await?;
    let fields = check(&state, &req, Some(id)).await?;
    let x = patch(&state, id, &fields).await?;
    announce(&state, None).await;
    Ok(Json(for_user(workforce::worker_of(&x), &user)))
}

/// Tells every open screen that workforce data changed; the sending device
/// (its `x-client-id`) ignores its own echo.
async fn announce(state: &AppState, headers: Option<&HeaderMap>) {
    let origin = headers.and_then(|h| h.get("x-client-id")).and_then(|v| v.to_str().ok());
    state.hub.changed_from(&[DataScope::Workforce], origin).await;
}

fn can_read_workforce(user: &CurrentUser) -> AppResult<()> {
    if user.0.can(Permission::ManageWorkforce) {
        return Ok(());
    }
    user.require(Permission::ManageCrew)
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// `PUT /api/workers/{id}/status` — marks today's attendance and logs it.
pub async fn set_status(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>, headers: HeaderMap, Json(req): Json<WorkerStatusRequest>) -> AppResult<Json<Worker>> {
    can_read_workforce(&user)?;
    require_visible(&state, &user, id).await?;
    let w = one(&state, &user, id).await?;
    if w.status == req.status {
        return Ok(Json(w));
    }
    let reason = req.reason.trim().to_string();
    let x = patch(&state, id, &json!({ "dailyStatus": legacy(req.status), "status": legacy(req.status), "absentReason": reason })).await?;
    workforce::append(
        &state.db,
        "attendance_log",
        json!({ "aNumber": w.a_number, "workerName": w.name, "date": today(), "oldStatus": legacy(w.status), "newStatus": legacy(req.status),
                "reason": reason, "timestamp": chrono::Utc::now().timestamp_millis(), "by": user.0.name }),
    )
    .await?;
    announce(&state, Some(&headers)).await;
    Ok(Json(for_user(workforce::worker_of(&x), &user)))
}

/// `PUT /api/workers/{id}/reassign` (Shift+R) — moves someone to another group,
/// area or department; each change goes in the assignment log.
pub async fn reassign(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>, Json(req): Json<ReassignRequest>) -> AppResult<Json<Worker>> {
    can_read_workforce(&user)?;
    require_visible(&state, &user, id).await?;
    let w = one(&state, &user, id).await?;
    let changes = [("projectGroup", &w.project_group, req.project_group.trim()), ("area", &w.area, req.area.trim()), ("department", &w.department, req.department.trim())];
    if changes.iter().all(|(_, old, new)| old.as_str() == *new) {
        return Ok(Json(w));
    }
    let x = patch(&state, id, &json!({ "projectGroup": req.project_group.trim(), "area": req.area.trim(), "department": req.department.trim() })).await?;
    for (field, old, new) in changes.iter().filter(|(_, old, new)| old.as_str() != *new) {
        workforce::append(
            &state.db,
            "assignment_log",
            json!({ "aNumber": w.a_number, "workerName": w.name, "date": today(), "field": field, "oldValue": old, "newValue": new,
                    "reason": req.reason.trim(), "timestamp": chrono::Utc::now().timestamp_millis(), "by": user.0.name }),
        )
        .await?;
    }
    announce(&state, None).await;
    Ok(Json(for_user(workforce::worker_of(&x), &user)))
}

/// `GET /api/workforce/groups` — every project group with today's head-count
/// and the job it is linked to.
pub async fn groups(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Vec<WorkGroup>>> {
    can_read_workforce(&user)?;
    let links: HashMap<String, Uuid> = workforce::list_records(&state.db, "group_links")
        .await?
        .into_iter()
        .filter_map(|v| Some((v["id"].as_str()?.to_string(), Uuid::parse_str(v["work_item_id"].as_str()?).ok()?)))
        .collect();
    let mut by: BTreeMap<String, WorkGroup> = BTreeMap::new();
    for w in workforce::people(&state.db).await?.iter().map(workforce::worker_of).filter(|w| !w.archived && !w.project_group.is_empty()) {
        let g = by.entry(w.project_group.clone()).or_insert_with(|| WorkGroup { name: w.project_group.clone(), total: 0, present: 0, absent: 0, on_pass: 0, work_item_id: None });
        g.total += 1;
        match w.status {
            WorkerStatus::Present => g.present += 1,
            WorkerStatus::Absent => g.absent += 1,
            WorkerStatus::OnPass => g.on_pass += 1,
        }
    }
    let mut out: Vec<WorkGroup> = by.into_values().collect();
    // Groups Service 5 knows about but nobody is in right now still show up.
    for meta in workforce::list_records(&state.db, "project_groups_meta").await? {
        for name in meta.as_object().into_iter().flat_map(|m| m.keys()).filter(|k| !matches!(k.as_str(), "id" | "updatedAt" | "_clientId")) {
            if !out.iter().any(|g| &g.name == name) {
                out.push(WorkGroup { name: name.clone(), total: 0, present: 0, absent: 0, on_pass: 0, work_item_id: None });
            }
        }
    }
    for g in out.iter_mut() {
        g.work_item_id = links.get(&g.name).copied();
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(Json(out))
}

/// `PUT /api/workforce/groups` — links a project group to a job.
pub async fn link_group(State(state): State<AppState>, user: CurrentUser, Json(req): Json<GroupLinkRequest>) -> AppResult<Json<Vec<WorkGroup>>> {
    user.require(Permission::ManageWorkforce)?;
    let name = req.group.trim();
    if name.is_empty() {
        return Err(AppError::bad("Choose a project group."));
    }
    match req.work_item_id {
        Some(job) => {
            workforce::put_record(&state.db, "group_links", name, json!({ "work_item_id": job }), None).await?;
        }
        None => workforce::delete_record(&state.db, "group_links", name).await?,
    }
    announce(&state, None).await;
    groups(State(state), user).await
}

/// Who may touch a collection: clinic lists need "Clinic ledger"; the rest
/// can be read by crew managers and changed by workforce managers.
fn check_collection(user: &CurrentUser, collection: &str, write: bool) -> AppResult<()> {
    if !workforce::known(collection) {
        return Err(AppError::not_found("list"));
    }
    if workforce::CLINIC.contains(&collection) {
        if write && collection == "medical_audit_logs" {
            return Err(AppError::bad("The medical audit trail can't be edited."));
        }
        return user.require(Permission::ManageClinic);
    }
    if write {
        user.require(Permission::ManageWorkforce)
    } else {
        can_read_workforce(user)
    }
}

/// Every clinic change leaves a line in the medical audit trail.
async fn audit_clinic(state: &AppState, user: &CurrentUser, collection: &str, id: &str, action: &str) -> AppResult<()> {
    if workforce::CLINIC.contains(&collection) {
        workforce::append(
            &state.db,
            "medical_audit_logs",
            json!({ "action": action, "recordId": id, "details": collection, "editedBy": user.0.name, "timestamp": chrono::Utc::now().to_rfc3339() }),
        )
        .await?;
    }
    Ok(())
}

/// `GET /api/workforce/records/{collection}`
pub async fn records(State(state): State<AppState>, user: CurrentUser, Path(collection): Path<String>) -> AppResult<Json<Vec<Value>>> {
    check_collection(&user, &collection, false)?;
    Ok(Json(workforce::list_records(&state.db, &collection).await?))
}

/// `PUT /api/workforce/records/{collection}/{id}` — last write wins.
pub async fn put_record(
    State(state): State<AppState>,
    user: CurrentUser,
    Path((collection, id)): Path<(String, String)>,
    headers: HeaderMap,
    Json(data): Json<Value>,
) -> AppResult<Json<Value>> {
    check_collection(&user, &collection, true)?;
    let client = headers.get("x-client-id").and_then(|v| v.to_str().ok());
    let saved = workforce::put_record(&state.db, &collection, &id, data, client).await?;
    audit_clinic(&state, &user, &collection, &id, "saved").await?;
    announce(&state, Some(&headers)).await;
    Ok(Json(saved))
}

/// `DELETE /api/workforce/records/{collection}/{id}`
pub async fn delete_record(State(state): State<AppState>, user: CurrentUser, Path((collection, id)): Path<(String, String)>) -> AppResult<axum::http::StatusCode> {
    check_collection(&user, &collection, true)?;
    workforce::delete_record(&state.db, &collection, &id).await?;
    audit_clinic(&state, &user, &collection, &id, "deleted").await?;
    announce(&state, None).await;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// The passphrase arrives hex-encoded (UTF-8 bytes) so any characters survive the header.
pub fn passphrase(headers: &HeaderMap) -> Option<String> {
    headers.get("x-backup-passphrase").and_then(|v| v.to_str().ok()).and_then(|h| {
        let bytes: Option<Vec<u8>> = (0..h.len()).step_by(2).map(|i| h.get(i..i + 2).and_then(|b| u8::from_str_radix(b, 16).ok())).collect();
        String::from_utf8(bytes?).ok()
    })
}

/// `POST /api/workforce/import` — any Service 5 backup, `.agwcz`, `service5.db`,
/// or our own backup, as the raw body. A snapshot is taken first, so it can be undone.
pub async fn import(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(req): Query<ImportOptions>,
    headers: HeaderMap,
    bytes: Bytes,
) -> AppResult<Json<WorkforceImportReport>> {
    user.require(Permission::ManageWorkforce)?;
    if bytes.is_empty() {
        return Err(AppError::bad("The file was empty. Please choose the backup again."));
    }
    let pass = passphrase(&headers);
    // Key derivation is deliberately slow (600,000 rounds), so keep it off the async workers.
    let data = tokio::task::spawn_blocking(move || workforce::read(&bytes, pass.as_deref()))
        .await
        .map_err(|e| AppError::bad(format!("The import stopped unexpectedly: {e}")))??;
    let report = workforce::import(&state.db, data, req.mode, req.allow_empty).await?;
    announce(&state, None).await;
    tracing::info!(by = %user.0.name, file = %req.file_name, detected = %report.detected, workers = report.workers, "workforce import");
    Ok(Json(report))
}

/// `GET /api/workforce/export` — one file with every workforce record, encrypted
/// with the passphrase in `x-backup-passphrase` (`.agwcz`, the format Service 5 uses).
pub async fn export(State(state): State<AppState>, user: CurrentUser, Query(q): Query<ExportQuery>, headers: HeaderMap) -> AppResult<Json<Value>> {
    user.require(Permission::ManageWorkforce)?;
    let mut all = workforce::export(&state.db).await?;
    // Selective backup (Service 5's backup picker): keep only the chosen lists.
    if let (Some(keep), Some(m)) = (q.collections.as_deref().filter(|c| !c.is_empty()), all.as_object_mut()) {
        let keep: Vec<&str> = keep.split(',').map(str::trim).collect();
        m.retain(|k, _| matches!(k.as_str(), "format" | "version" | "exportedAt") || keep.contains(&k.as_str()));
    }
    // Personnel data only leaves the server encrypted.
    let pass = passphrase(&headers).ok_or_else(|| AppError::bad("Choose a passphrase (at least 8 characters) to encrypt the backup."))?;
    let sealed = tokio::task::spawn_blocking(move || workforce::encrypt_agwcz(&all, &pass)).await.map_err(|e| AppError::bad(e.to_string()))??;
    Ok(Json(sealed))
}

#[derive(serde::Deserialize)]
pub struct ExportQuery {
    collections: Option<String>,
}

/// `POST /api/workforce/start-day` — Service 5's "Start New Day": staged
/// assignments (tomorrow's) become today's; everyone else is unassigned.
/// A snapshot is taken first so it can be undone.
pub async fn start_day(State(state): State<AppState>, user: CurrentUser, headers: HeaderMap) -> AppResult<Json<Value>> {
    user.require(Permission::ManageWorkforce)?;
    workforce::snapshot(&state.db, "before Start New Day").await?;
    let today = today();
    let mut applied = 0;
    for mut x in workforce::people(&state.db).await? {
        if workforce::worker_of(&x).archived {
            continue;
        }
        let area = text(&x.p, "area");
        let tm = x.p.get("tomorrowAssignment").cloned().unwrap_or(Value::Null);
        let due = staged(&tm, &today);
        let fields = match &due {
            Some(st) => {
                applied += 1;
                let field = |k: &str, cur: &str| st.get(k).and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| cur.to_string());
                let legacy = st.get("dailyStatus").and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or("Present").to_string();
                json!({ "area": field("area", &area), "project": field("project", ""), "specificTask": field("specificTask", ""), "dailyStatus": legacy, "status": legacy,
                        "absentReason": field("absentReason", ""), "isAssignedToday": st.get("isAssigned").and_then(Value::as_bool).unwrap_or(true),
                        "taskStartedAt": st.get("encodedAt").cloned().unwrap_or(json!(chrono::Utc::now().to_rfc3339())) })
            }
            None => json!({ "isAssignedToday": false }),
        };
        if let Some(p) = x.p.as_object_mut() {
            // A staging dated after today stays put for its own day.
            if due.is_some() {
                p.remove("tomorrowAssignment");
            }
            for (k, v) in fields.as_object().into_iter().flatten() {
                p.insert(k.clone(), v.clone());
            }
        }
        workforce::save_person(&state.db, x.id, &x.p).await?;
    }
    announce(&state, Some(&headers)).await;
    Ok(Json(json!({ "applied": applied })))
}

/// Tomorrow's assignment if it is due today (or undated): Service 5's object
/// form, or the plain task text older SiteMonitor builds saved.
fn staged(tm: &Value, today: &str) -> Option<Map<String, Value>> {
    match tm {
        Value::Object(o) if o.get("date").and_then(Value::as_str).is_none_or(|d| d.is_empty() || d <= today) => Some(o.clone()),
        Value::String(task) if !task.trim().is_empty() => Some(Map::from_iter([("specificTask".to_string(), json!(task))])),
        _ => None,
    }
}

/// `POST /api/workers/delete` — permanent removal (Service 5's "Delete selected").
/// A snapshot is taken first so a mistake can be restored.
pub async fn delete_workers(State(state): State<AppState>, user: CurrentUser, headers: HeaderMap, Json(req): Json<DeleteWorkersRequest>) -> AppResult<Json<Value>> {
    user.require(Permission::ManageWorkforce)?;
    if req.ids.is_empty() {
        return Err(AppError::bad("Choose who to delete."));
    }
    let who = if req.ids.len() == 1 { "1 person".to_string() } else { format!("{} people", req.ids.len()) };
    workforce::snapshot(&state.db, &format!("before deleting {who}")).await?;
    let mut deleted = 0;
    for id in &req.ids {
        deleted += state.db.execute(stmt("DELETE FROM workers WHERE id = $1", vec![(*id).into()])).await?.rows_affected();
    }
    announce(&state, Some(&headers)).await;
    Ok(Json(json!({ "deleted": deleted })))
}

fn stmt(sql: &str, values: Vec<sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

/// Who was on which project each day: (date, project, A#, name, status, task),
/// the newest record per person and day winning.
fn roster_rows(days: Vec<(String, Vec<Value>)>) -> Vec<Value> {
    let mut best: BTreeMap<(String, String), Value> = BTreeMap::new();
    for (d, workers) in days {
        for w in workers {
            let (p, a) = (text(&w, "project"), text(&w, "aNumber"));
            if p.is_empty() || a.is_empty() {
                continue;
            }
            let status = if text(&w, "dailyStatus").is_empty() { text(&w, "status") } else { text(&w, "dailyStatus") };
            best.insert((d.clone(), a.clone()), json!({ "date": d, "project": p, "aNumber": a, "name": text(&w, "name"), "status": status, "task": text(&w, "specificTask") }));
        }
    }
    let mut out: Vec<Value> = best.into_values().collect();
    out.sort_by(|a, b| text(b, "date").cmp(&text(a, "date")).then(text(a, "project").cmp(&text(b, "project"))).then(text(a, "name").cmp(&text(b, "name"))));
    out
}

/// `GET /api/workforce/project-history` — who was on each project, day by day,
/// from the daily snapshots and today (Service 5's "Project Roster History").
pub async fn project_history(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Vec<Value>>> {
    user.require(Permission::ManageWorkforce)?;
    let mut days: Vec<(String, Vec<Value>)> = workforce::all_snapshots(&state.db)
        .await?
        .into_iter()
        .map(|(at, data)| (at.with_timezone(&chrono::Local).format("%Y-%m-%d").to_string(), data["workers"].as_array().cloned().unwrap_or_default()))
        .collect();
    let now: Vec<Value> = workforce::people(&state.db).await?.into_iter().filter(|x| !workforce::worker_of(x).archived).map(|x| x.p).collect();
    days.push((today(), now));
    Ok(Json(roster_rows(days)))
}

/// `GET /api/workforce/snapshots`
pub async fn snapshots(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Vec<WorkforceSnapshotInfo>>> {
    user.require(Permission::ManageWorkforce)?;
    let rows = state.db.query_all(stmt("SELECT id, taken_at, reason, workers FROM workforce_snapshots ORDER BY taken_at DESC", vec![])).await?;
    Ok(Json(
        rows.iter()
            .filter_map(|r| {
                Some(WorkforceSnapshotInfo {
                    id: r.try_get("", "id").ok()?,
                    taken_at: r.try_get("", "taken_at").ok()?,
                    reason: r.try_get("", "reason").ok()?,
                    workers: r.try_get("", "workers").ok()?,
                })
            })
            .collect(),
    ))
}

/// `POST /api/workforce/snapshots` — "take a snapshot now".
pub async fn take_snapshot(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Vec<WorkforceSnapshotInfo>>> {
    user.require(Permission::ManageWorkforce)?;
    workforce::snapshot(&state.db, "manual").await?;
    snapshots(State(state), user).await
}

/// `POST /api/workforce/snapshots/{id}/restore` — puts the workforce data back
/// as it was (the current data is snapshotted first, and empty parts never wipe).
pub async fn restore_snapshot(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>) -> AppResult<Json<WorkforceImportReport>> {
    user.require(Permission::ManageWorkforce)?;
    let data = workforce::snapshot_data(&state.db, id).await?.ok_or_else(|| AppError::not_found("snapshot"))?;
    let mut n = workforce::normalize(data)?;
    n.label = "Snapshot restore".into();
    let report = workforce::import(&state.db, n, ImportMode::Replace, false).await?;
    announce(&state, None).await;
    Ok(Json(report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_day_applies_due_or_undated_stagings_only() {
        let today = "2026-11-20";
        assert!(staged(&json!({ "specificTask": "Paint", "date": "2026-11-20" }), today).is_some());
        assert!(staged(&json!({ "specificTask": "Paint" }), today).is_some());
        assert!(staged(&json!({ "specificTask": "Paint", "date": "2026-11-19" }), today).is_some());
        assert!(staged(&json!({ "specificTask": "Paint", "date": "2026-11-21" }), today).is_none());
        // Plain text from older builds still counts; blanks and nothing do not.
        assert_eq!(staged(&json!("Sweep"), today).unwrap()["specificTask"], "Sweep");
        assert!(staged(&json!(" "), today).is_none());
        assert!(staged(&Value::Null, today).is_none());
    }

    #[test]
    fn roster_keeps_the_newest_record_per_person_and_day() {
        let rows = roster_rows(vec![
            ("2026-11-19".into(), vec![json!({ "aNumber": 1, "name": "A", "project": "Hall", "dailyStatus": "Absent" })]),
            ("2026-11-19".into(), vec![json!({ "aNumber": "1", "name": "A", "project": "Hall", "dailyStatus": "Present" }), json!({ "aNumber": "2", "name": "B", "project": "" })]),
            ("2026-11-20".into(), vec![json!({ "aNumber": "1", "name": "A", "project": "Pool", "status": "Passes" })]),
        ]);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["date"], "2026-11-20");
        assert_eq!(rows[0]["status"], "Passes");
        assert_eq!(rows[1]["status"], "Present");
    }
}
