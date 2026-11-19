//! Workforce (merged from Service 5 DB): the people directory, legacy backup
//! import, backup download and snapshots.

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use sea_orm::{ConnectionTrait, DatabaseBackend, QueryResult, Statement};
use serde_json::{json, Value};
use shared::api::*;
use shared::*;
use uuid::Uuid;

use sea_orm::EntityTrait;
use shared::ws::DataScope;

use crate::convert::{dec, enc};
use crate::error::{AppError, AppResult};
use crate::services::workforce;
use crate::state::{AppState, CurrentUser};

const COLUMNS: &str = "id, a_number, name, phone, trade, department, area, project_group, group_role, daily_status, archived, updated_at,
    COALESCE(profile->>'photoUrl', '') LIKE 'data:image/%' AS has_photo,
    COALESCE(profile->>'tomorrowAssignment', '') AS tomorrow,
    COALESCE(NULLIF(profile->>'workResidence', ''), profile->>'residence', '') AS residence";

fn worker(r: &QueryResult) -> Option<Worker> {
    Some(Worker {
        id: r.try_get("", "id").ok()?,
        a_number: r.try_get("", "a_number").ok()?,
        name: r.try_get("", "name").ok()?,
        phone: r.try_get("", "phone").ok()?,
        trade: r.try_get("", "trade").ok()?,
        department: r.try_get("", "department").ok()?,
        area: r.try_get("", "area").ok()?,
        project_group: r.try_get("", "project_group").ok()?,
        group_role: r.try_get("", "group_role").ok()?,
        status: dec(&r.try_get::<String>("", "daily_status").ok()?),
        archived: r.try_get("", "archived").ok()?,
        updated_at: r.try_get("", "updated_at").ok()?,
        has_photo: r.try_get("", "has_photo").unwrap_or(false),
        tomorrow: r.try_get("", "tomorrow").unwrap_or_default(),
        residence: r.try_get("", "residence").unwrap_or_default(),
    })
}

/// Which workers a Site Monitor may see and update: those whose Service 5 area
/// is mapped to one of their zones, or whose project group works on a job in
/// those zones. Everyone else sees everyone. `None` = no limit.
async fn visible_ids(state: &AppState, user: &CurrentUser) -> AppResult<Option<std::collections::HashSet<Uuid>>> {
    if user.0.role != UserRole::SiteMonitor || user.0.can(Permission::ManageWorkforce) {
        return Ok(None);
    }
    let parents: std::collections::HashMap<Uuid, Option<Uuid>> =
        crate::entities::spatial_area::Entity::find().all(&state.db).await?.into_iter().map(|a| (a.id, a.parent_id)).collect();
    let mine = |zone: Uuid| shared::access::works_in(&user.0, zone, |z| parents.get(&z).copied().flatten());
    let zone_of = |v: &Value, k: &str| v[k].as_str().and_then(|s| Uuid::parse_str(s).ok());
    let areas: std::collections::HashSet<String> =
        workforce::list_records(&state.db, "area_zones").await?.into_iter().filter(|v| zone_of(v, "zone_id").is_some_and(mine)).filter_map(|v| v["id"].as_str().map(str::to_string)).collect();
    let job_zone: std::collections::HashMap<Uuid, Uuid> =
        crate::entities::work_item::Entity::find().all(&state.db).await?.into_iter().map(|w| (w.id, w.area_id)).collect();
    let groups: std::collections::HashSet<String> = workforce::list_records(&state.db, "group_links")
        .await?
        .into_iter()
        .filter(|v| zone_of(v, "work_item_id").and_then(|j| job_zone.get(&j).copied()).is_some_and(mine))
        .filter_map(|v| v["id"].as_str().map(str::to_string))
        .collect();
    let rows = state.db.query_all(stmt("SELECT id, area, project_group FROM workers", vec![])).await?;
    Ok(Some(
        rows.iter()
            .filter(|r| {
                let a: String = r.try_get("", "area").unwrap_or_default();
                let g: String = r.try_get("", "project_group").unwrap_or_default();
                areas.contains(&a) || groups.contains(&g)
            })
            .filter_map(|r| r.try_get::<Uuid>("", "id").ok())
            .collect(),
    ))
}

async fn require_visible(state: &AppState, user: &CurrentUser, id: Uuid) -> AppResult<()> {
    match visible_ids(state, user).await? {
        Some(ids) if !ids.contains(&id) => Err(AppError::forbidden("That person isn't in your zones. Ask the Office Admin to map their area to a zone.")),
        _ => Ok(()),
    }
}

/// Personal details fields the profile editor may change. Health ones need "Clinic ledger".
const PROFILE_FIELDS: &[&str] = &[
    "birthday", "civilStatus", "address", "residence", "workResidence", "language", "fatherName", "motherName", "spouseName",
    "spouseBirthday", "spouseContact", "spouseChurchDuty", "weddingDate", "children", "emergencyContact", "emergencyPhone",
    "emergencyRelationship", "skillLevel", "hourlyRate", "subDepartment", "specificTask", "tomorrowAssignment", "assignment",
    "visitationGroup", "visitationHousing", "photoUrl",
];
const HEALTH_FIELDS: &[&str] = &["bloodType", "medicalNotes", "maintenanceMeds", "healthClearance", "medical", "healthAndClearance"];

/// `GET /api/workers/{id}/photo` — the photo kept in the worker's record.
pub async fn photo(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>) -> AppResult<axum::response::Response> {
    use axum::response::IntoResponse;
    can_read_workforce(&user)?;
    let row = state.db.query_one(stmt("SELECT profile->>'photoUrl' AS p FROM workers WHERE id = $1", vec![id.into()])).await?;
    let url: String = row.and_then(|r| r.try_get("", "p").ok()).unwrap_or_default();
    let (head, data) = url.split_once(',').ok_or_else(|| AppError::not_found("photo"))?;
    let mime = head.strip_prefix("data:").and_then(|h| h.strip_suffix(";base64")).filter(|m| m.starts_with("image/")).ok_or_else(|| AppError::not_found("photo"))?;
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(data).map_err(|_| AppError::not_found("photo"))?;
    Ok(([(axum::http::header::CONTENT_TYPE, mime.to_string()), (axum::http::header::CACHE_CONTROL, "private, max-age=300".to_string())], bytes).into_response())
}

/// `GET /api/workers/{id}/profile` — the personal details (health ones only with "Clinic ledger").
pub async fn profile(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>) -> AppResult<Json<Value>> {
    user.require(Permission::ManageWorkforce)?;
    let row = state.db.query_one(stmt("SELECT profile FROM workers WHERE id = $1", vec![id.into()])).await?;
    let mut p: Value = row.and_then(|r| r.try_get("", "profile").ok()).ok_or_else(|| AppError::not_found("worker"))?;
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
    let mut patch = serde_json::Map::new();
    for (k, v) in body.as_object().into_iter().flatten() {
        if PROFILE_FIELDS.contains(&k.as_str()) || (clinic && HEALTH_FIELDS.contains(&k.as_str())) {
            patch.insert(k.clone(), v.clone());
        }
    }
    if let Some(p) = patch.get("photoUrl").and_then(Value::as_str) {
        if !p.is_empty() && (!p.starts_with("data:image/") || p.len() > 3_000_000) {
            return Err(AppError::bad("The photo must be a picture under 2 MB."));
        }
    }
    one(&state, id).await?;
    state.db.execute(stmt("UPDATE workers SET profile = profile || $2::jsonb, updated_at = now() WHERE id = $1", vec![id.into(), Value::Object(patch).into()])).await?;
    announce(&state, None).await;
    Ok(Json(one(&state, id).await?))
}

/// `GET /api/workforce/summary` — the workforce dashboard.
pub async fn summary(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<WorkforceSummary>> {
    can_read_workforce(&user)?;
    let pairs = |sql: &str| {
        let db = state.db.clone();
        let sql = sql.to_string();
        async move {
            let rows = db.query_all(stmt(&sql, vec![])).await?;
            AppResult::Ok(rows.iter().filter_map(|r| Some((r.try_get_by_index::<String>(0).ok()?, r.try_get_by_index::<i64>(1).ok()? as u32))).collect::<Vec<_>>())
        }
    };
    let trades = pairs("SELECT COALESCE(NULLIF(trade, ''), 'No trade'), COUNT(*) FROM workers WHERE NOT archived GROUP BY 1 ORDER BY 2 DESC, 1").await?;
    let departments = pairs("SELECT COALESCE(NULLIF(department, ''), 'No department'), COUNT(*) FROM workers WHERE NOT archived GROUP BY 1 ORDER BY 2 DESC, 1").await?;
    let since = (chrono::Local::now().date_naive() - chrono::Duration::days(13)).format("%Y-%m-%d").to_string();
    let rows = state
        .db
        .query_all(stmt(
            "SELECT data->>'date' AS d,
                    COUNT(DISTINCT data->>'aNumber') FILTER (WHERE data->>'newStatus' = 'Absent') AS a,
                    COUNT(DISTINCT data->>'aNumber') FILTER (WHERE data->>'newStatus' IN ('Passes', 'On Pass')) AS p
             FROM workforce_records WHERE collection = 'attendance_log' AND data->>'date' >= $1 GROUP BY 1 ORDER BY 1",
            vec![since.into()],
        ))
        .await?;
    let trend = rows
        .iter()
        .filter_map(|r| Some((r.try_get::<String>("", "d").ok()?, r.try_get::<i64>("", "a").ok()? as u32, r.try_get::<i64>("", "p").ok()? as u32)))
        .collect();
    let month = chrono::Local::now().format("-%m-").to_string();
    let rows = state
        .db
        .query_all(stmt(
            "SELECT name, profile->>'birthday' AS b FROM workers WHERE NOT archived AND COALESCE(profile->>'birthday', '') LIKE '%' || $1 || '%'
             ORDER BY substring(profile->>'birthday' from 9 for 2)",
            vec![month.into()],
        ))
        .await?;
    let birthdays = rows.iter().filter_map(|r| Some((r.try_get::<String>("", "name").ok()?, r.try_get::<String>("", "b").ok()?))).collect();
    Ok(Json(WorkforceSummary { trades, departments, trend, birthdays }))
}

/// `GET /api/workforce/family-roster` — family and emergency details for printing.
pub async fn family_roster(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Vec<Value>>> {
    user.require(Permission::ManageWorkforce)?;
    let rows = state
        .db
        .query_all(stmt(
            "SELECT jsonb_build_object('name', name, 'aNumber', a_number, 'group', project_group,
               'spouseName', profile->>'spouseName', 'children', profile->'children', 'fatherName', profile->>'fatherName',
               'motherName', profile->>'motherName', 'emergencyContact', profile->>'emergencyContact',
               'emergencyPhone', profile->>'emergencyPhone', 'address', profile->>'address') AS j
             FROM workers WHERE NOT archived ORDER BY project_group, lower(name)",
            vec![],
        ))
        .await?;
    Ok(Json(rows.iter().filter_map(|r| r.try_get::<Value>("", "j").ok()).collect()))
}

fn stmt(sql: &str, values: Vec<sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

async fn one(state: &AppState, id: Uuid) -> AppResult<Worker> {
    let row = state.db.query_one(stmt(&format!("SELECT {COLUMNS} FROM workers WHERE id = $1"), vec![id.into()])).await?;
    row.as_ref().and_then(worker).ok_or_else(|| AppError::not_found("worker"))
}

/// `GET /api/workers` — everyone who manages crew can see the list.
pub async fn list(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Vec<Worker>>> {
    if !user.0.can(Permission::ManageWorkforce) {
        user.require(Permission::ManageCrew)?;
    }
    let rows = state.db.query_all(stmt(&format!("SELECT {COLUMNS} FROM workers ORDER BY archived, lower(name)"), vec![])).await?;
    let visible = visible_ids(&state, &user).await?;
    Ok(Json(rows.iter().filter_map(worker).filter(|w| visible.as_ref().is_none_or(|v| v.contains(&w.id))).collect()))
}

/// Checks the form and returns the Service 5-style fields kept in `profile`.
async fn check(state: &AppState, req: &SaveWorkerRequest, id: Option<Uuid>) -> AppResult<Value> {
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(AppError::bad("Please enter the person's name (up to 120 characters)."));
    }
    let a = req.a_number.trim();
    if !a.is_empty() {
        let clash = state
            .db
            .query_one(stmt("SELECT name FROM workers WHERE a_number = $1 AND id <> $2", vec![a.into(), id.unwrap_or_default().into()]))
            .await?;
        if let Some(r) = clash {
            let other: String = r.try_get_by_index(0).unwrap_or_default();
            return Err(AppError::bad(format!("A# {a} already belongs to {other}.")));
        }
    }
    let legacy = match req.status {
        WorkerStatus::Present => "Present",
        WorkerStatus::Absent => "Absent",
        WorkerStatus::OnPass => "Passes",
    };
    Ok(json!({
        "aNumber": a, "name": name, "phone": req.phone.trim(), "trade": req.trade.trim(), "department": req.department.trim(),
        "area": req.area.trim(), "projectGroup": req.project_group.trim(), "groupRole": req.group_role.trim(),
        "dailyStatus": legacy, "status": legacy, "archived": req.archived, "isArchived": req.archived,
    }))
}

fn typed(req: &SaveWorkerRequest) -> Vec<sea_orm::Value> {
    vec![
        req.a_number.trim().into(),
        req.name.trim().into(),
        req.phone.trim().into(),
        req.trade.trim().into(),
        req.department.trim().into(),
        req.area.trim().into(),
        req.project_group.trim().into(),
        req.group_role.trim().into(),
        enc(&req.status).into(),
        req.archived.into(),
    ]
}

/// `POST /api/workers`
pub async fn create(State(state): State<AppState>, user: CurrentUser, Json(req): Json<SaveWorkerRequest>) -> AppResult<Json<Worker>> {
    user.require(Permission::ManageWorkforce)?;
    let id = Uuid::new_v4();
    let mut profile = check(&state, &req, None).await?;
    profile["workerId"] = json!(id);
    let mut values = vec![id.into()];
    values.extend(typed(&req));
    values.push(profile.into());
    state
        .db
        .execute(stmt(
            "INSERT INTO workers (id, a_number, name, phone, trade, department, area, project_group, group_role, daily_status, archived, profile)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
            values,
        ))
        .await?;
    announce(&state, None).await;
    Ok(Json(one(&state, id).await?))
}

/// `PUT /api/workers/{id}` — the Service 5 fields in `profile` stay in step.
pub async fn update(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>, Json(req): Json<SaveWorkerRequest>) -> AppResult<Json<Worker>> {
    user.require(Permission::ManageWorkforce)?;
    one(&state, id).await?;
    let patch = check(&state, &req, Some(id)).await?;
    let mut values = vec![id.into()];
    values.extend(typed(&req));
    values.push(patch.into());
    state
        .db
        .execute(stmt(
            "UPDATE workers SET a_number = $2, name = $3, phone = $4, trade = $5, department = $6, area = $7, project_group = $8,
               group_role = $9, daily_status = $10, archived = $11, profile = profile || $12::jsonb, updated_at = now()
             WHERE id = $1",
            values,
        ))
        .await?;
    announce(&state, None).await;
    Ok(Json(one(&state, id).await?))
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
    let w = one(&state, id).await?;
    if w.status == req.status {
        return Ok(Json(w));
    }
    let legacy = |s: WorkerStatus| match s {
        WorkerStatus::Present => "Present",
        WorkerStatus::Absent => "Absent",
        WorkerStatus::OnPass => "Passes",
    };
    let reason = req.reason.trim().to_string();
    state
        .db
        .execute(stmt(
            "UPDATE workers SET daily_status = $2, profile = profile || $3::jsonb, updated_at = now() WHERE id = $1",
            vec![id.into(), enc(&req.status).into(), json!({ "dailyStatus": legacy(req.status), "status": legacy(req.status), "absentReason": reason }).into()],
        ))
        .await?;
    workforce::append(
        &state.db,
        "attendance_log",
        json!({ "aNumber": w.a_number, "workerName": w.name, "date": today(), "oldStatus": legacy(w.status), "newStatus": legacy(req.status),
                "reason": reason, "timestamp": chrono::Utc::now().timestamp_millis(), "by": user.0.name }),
    )
    .await?;
    announce(&state, Some(&headers)).await;
    Ok(Json(one(&state, id).await?))
}

/// `PUT /api/workers/{id}/reassign` (Shift+R) — moves someone to another group,
/// area or department; each change goes in the assignment log.
pub async fn reassign(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>, Json(req): Json<ReassignRequest>) -> AppResult<Json<Worker>> {
    can_read_workforce(&user)?;
    require_visible(&state, &user, id).await?;
    let w = one(&state, id).await?;
    let changes = [("projectGroup", &w.project_group, req.project_group.trim()), ("area", &w.area, req.area.trim()), ("department", &w.department, req.department.trim())];
    if changes.iter().all(|(_, old, new)| old.as_str() == *new) {
        return Ok(Json(w));
    }
    state
        .db
        .execute(stmt(
            "UPDATE workers SET project_group = $2, area = $3, department = $4, profile = profile || $5::jsonb, updated_at = now() WHERE id = $1",
            vec![
                id.into(),
                req.project_group.trim().into(),
                req.area.trim().into(),
                req.department.trim().into(),
                json!({ "projectGroup": req.project_group.trim(), "area": req.area.trim(), "department": req.department.trim() }).into(),
            ],
        ))
        .await?;
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
    Ok(Json(one(&state, id).await?))
}

/// `GET /api/workforce/groups` — every project group with today's head-count
/// and the job it is linked to.
pub async fn groups(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Vec<WorkGroup>>> {
    can_read_workforce(&user)?;
    let rows = state
        .db
        .query_all(stmt(
            "SELECT project_group AS name, COUNT(*)::int AS total,
                    COUNT(*) FILTER (WHERE daily_status = 'Present')::int AS present,
                    COUNT(*) FILTER (WHERE daily_status = 'Absent')::int AS absent,
                    COUNT(*) FILTER (WHERE daily_status = 'OnPass')::int AS on_pass
             FROM workers WHERE NOT archived AND project_group <> '' GROUP BY project_group",
            vec![],
        ))
        .await?;
    let links: std::collections::HashMap<String, Uuid> = workforce::list_records(&state.db, "group_links")
        .await?
        .into_iter()
        .filter_map(|v| Some((v["id"].as_str()?.to_string(), Uuid::parse_str(v["work_item_id"].as_str()?).ok()?)))
        .collect();
    let mut out: Vec<WorkGroup> = rows
        .iter()
        .filter_map(|r| {
            let n = |k: &str| r.try_get::<i32>("", k).ok().map(|v| v as u32);
            let name: String = r.try_get("", "name").ok()?;
            Some(WorkGroup { work_item_id: links.get(&name).copied(), total: n("total")?, present: n("present")?, absent: n("absent")?, on_pass: n("on_pass")?, name })
        })
        .collect();
    // Groups Service 5 knows about but nobody is in right now still show up.
    for meta in workforce::list_records(&state.db, "project_groups_meta").await? {
        for name in meta.as_object().into_iter().flat_map(|m| m.keys()).filter(|k| !matches!(k.as_str(), "id" | "updatedAt" | "_clientId")) {
            if !out.iter().any(|g| &g.name == name) {
                out.push(WorkGroup { name: name.clone(), total: 0, present: 0, absent: 0, on_pass: 0, work_item_id: links.get(name).copied() });
            }
        }
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
    // The passphrase arrives hex-encoded (UTF-8 bytes) so any characters survive the header.
    let pass = headers.get("x-backup-passphrase").and_then(|v| v.to_str().ok()).and_then(|h| {
        let bytes: Option<Vec<u8>> = (0..h.len()).step_by(2).map(|i| h.get(i..i + 2).and_then(|b| u8::from_str_radix(b, 16).ok())).collect();
        String::from_utf8(bytes?).ok()
    });
    // Key derivation is deliberately slow (600,000 rounds), so keep it off the async workers.
    let data = tokio::task::spawn_blocking(move || workforce::read(&bytes, pass.as_deref()))
        .await
        .map_err(|e| AppError::bad(format!("The import stopped unexpectedly: {e}")))??;
    let report = workforce::import(&state.db, data, req.mode, req.allow_empty).await?;
    announce(&state, None).await;
    tracing::info!(by = %user.0.name, file = %req.file_name, detected = %report.detected, workers = report.workers, "workforce import");
    Ok(Json(report))
}

/// `GET /api/workforce/export` — one file with every workforce record.
pub async fn export(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Value>> {
    user.require(Permission::ManageWorkforce)?;
    Ok(Json(workforce::export(&state.db).await?))
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
    let row = state.db.query_one(stmt("SELECT data FROM workforce_snapshots WHERE id = $1", vec![id.into()])).await?;
    let data: Value = row.and_then(|r| r.try_get("", "data").ok()).ok_or_else(|| AppError::not_found("snapshot"))?;
    let mut n = workforce::normalize(data)?;
    n.label = "Snapshot restore".into();
    let report = workforce::import(&state.db, n, ImportMode::Replace, false).await?;
    announce(&state, None).await;
    Ok(Json(report))
}
