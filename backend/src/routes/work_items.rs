//! Projects, maintenance tasks and repairs.

use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::Json;
use chrono::{Duration, Local};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, ModelTrait, PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use shared::labels::{displayed_progress, weighted_progress};
use shared::ws::DataScope;
use shared::api::*;
use shared::*;
use uuid::Uuid;

use crate::convert::{self, enc};
use crate::entities::{
    decision_alert, estate_agenda_item, executive_handover_report, manpower_ledger_entry, prelude::*, project_phase,
    site_photo, timeline_event_record, work_item,
};
use crate::error::{AppError, AppResult};
use crate::services::{all_areas, log_timeline, media, progress, subtree_ids};
use crate::state::{AppState, CurrentUser};

/// `GET /api/work-items`
pub async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<WorkItem>>> {
    let mut items: Vec<WorkItem> = WorkItems::find().all(&state.db).await?.into_iter().map(convert::work_item).collect();
    items.sort_by(|a, b| a.current_target_deadline.cmp(&b.current_target_deadline).then(a.title.cmp(&b.title)));
    Ok(Json(items))
}

/// `GET /api/work-items/{id}` — the full project dossier.
pub async fn detail(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<WorkItemDetail>> {
    let item = WorkItems::find_by_id(id).one(&state.db).await?.map(convert::work_item).ok_or_else(|| AppError::not_found("project"))?;
    let areas = all_areas(&state.db).await?;
    let area = areas.iter().find(|a| a.id == item.area_id).cloned().ok_or_else(|| AppError::not_found("zone"))?;
    let phases = ProjectPhases::find()
        .filter(project_phase::Column::WorkItemId.eq(id))
        .order_by_asc(project_phase::Column::OrderIndex)
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::phase)
        .collect();
    let ledger = ManpowerLedgerEntries::find()
        .filter(manpower_ledger_entry::Column::WorkItemId.eq(id))
        .order_by_asc(manpower_ledger_entry::Column::Timestamp)
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::ledger)
        .collect();
    let timeline = TimelineEventRecords::find()
        .filter(timeline_event_record::Column::WorkItemId.eq(id))
        .order_by_asc(timeline_event_record::Column::Timestamp)
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::timeline)
        .collect();

    let photo_rows: Vec<SitePhoto> = SitePhotos::find()
        .filter(site_photo::Column::WorkItemId.eq(id))
        .order_by_desc(site_photo::Column::UploadedAt)
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::photo)
        .collect();
    let photo_ids: Vec<Uuid> = photo_rows.iter().map(|p| p.id).collect();
    let mut callouts: HashMap<Uuid, Vec<PhotoCallOut>> = HashMap::new();
    if !photo_ids.is_empty() {
        for c in PhotoCallOuts::find()
            .filter(crate::entities::photo_call_out::Column::PhotoId.is_in(photo_ids))
            .order_by_asc(crate::entities::photo_call_out::Column::CreatedAt)
            .all(&state.db)
            .await?
            .into_iter()
            .map(convert::callout)
        {
            callouts.entry(c.photo_id).or_default().push(c);
        }
    }
    let photos = photo_rows
        .into_iter()
        .map(|p| {
            let c = callouts.remove(&p.id).unwrap_or_default();
            PhotoWithCallOuts { photo: p, callouts: c }
        })
        .collect();

    let mut decisions: Vec<DecisionAlert> = DecisionAlerts::find()
        .filter(decision_alert::Column::WorkItemId.eq(id))
        .order_by_desc(decision_alert::Column::CreatedAt)
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::decision)
        .collect();
    decisions.retain(|d| shared::access::can_see_report(&user.0, d));

    let zone = subtree_ids(&areas, item.area_id);
    let zone_assets = Assets::find()
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::asset)
        .filter(|a| zone.contains(&a.current_area_id) && !a.is_retired)
        .collect();
    let handover = ExecutiveHandoverReports::find()
        .filter(executive_handover_report::Column::WorkItemId.eq(id))
        .one(&state.db)
        .await?
        .map(convert::handover);

    Ok(Json(WorkItemDetail { item, area, phases, ledger, timeline, photos, decisions, zone_assets, handover }))
}

/// `POST /api/phases/{id}/complete`
pub async fn complete_phase(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CompletePhaseRequest>,
) -> AppResult<Json<ProgressUpdate>> {
    user.require(Permission::UpdateProgress)?;
    let phase = ProjectPhases::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("phase"))?;
    user.require_job(&state.db, phase.work_item_id).await?;
    let proof = match req.proof_data_url.as_deref().filter(|s| !s.is_empty()) {
        Some(data) => Some(media::save_data_url(&state.upload_dir, "photos", data).await?),
        None => None,
    };
    Ok(Json(progress::complete_phase(&state, id, proof, &user.0).await?))
}

/// `POST /api/work-items/{id}/complete-task` — routine upkeep & repairs.
pub async fn complete_task(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CompleteTaskRequest>,
) -> AppResult<Json<WorkItem>> {
    user.require(Permission::UpdateProgress)?;
    user.require_job(&state.db, id).await?;
    let proof = match req.proof_data_url.as_deref().filter(|s| !s.is_empty()) {
        Some(data) => Some(media::save_data_url(&state.upload_dir, "photos", data).await?),
        None => None,
    };
    Ok(Json(progress::complete_task(&state, id, proof, req.note, &user.0).await?))
}

/// `POST /api/work-items/{id}/delay` — the target date moves back by `days`
/// and the reason goes on the timeline the owner reads.
pub async fn log_delay(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<LogDelayRequest>,
) -> AppResult<Json<WorkItem>> {
    user.require(Permission::UpdateProgress)?;
    user.require_job(&state.db, id).await?;
    if req.category == DelayCategory::None {
        return Err(AppError::bad("Choose what caused the delay."));
    }
    if !(1..=90).contains(&req.days) {
        return Err(AppError::bad("A delay must be between 1 and 90 days."));
    }
    let note = req.note.trim();
    if note.is_empty() {
        return Err(AppError::bad("Please say briefly what happened."));
    }
    let title = match req.title.trim() {
        "" => format!("{} delay", req.category.label()),
        t => t.to_string(),
    };
    let txn = state.db.begin().await?;
    let row = WorkItems::find_by_id(id).one(&txn).await?.ok_or_else(|| AppError::not_found("project"))?;
    if !convert::work_item(row.clone()).status.is_active() {
        return Err(AppError::bad("This project is finished, so its date can't move."));
    }
    let target = row.current_target_deadline + Duration::days(req.days as i64);
    let mut a = row.into_active_model();
    a.current_target_deadline = Set(target);
    a.update(&txn).await?;
    for e in EstateAgendaItems::find()
        .filter(estate_agenda_item::Column::WorkItemId.eq(id))
        .filter(estate_agenda_item::Column::EventType.eq(enc(&AgendaEventType::TargetDeadline)))
        .all(&txn)
        .await?
    {
        let mut ea = e.into_active_model();
        ea.start_time = Set(target - Duration::hours(1));
        ea.end_time = Set(target);
        ea.update(&txn).await?;
    }
    log_timeline(&txn, id, title, format!("{note} (logged by {})", user.0.name), false, req.category, req.days).await?;
    txn.commit().await?;
    state.hub.changed(&[DataScope::WorkItems, DataScope::Map, DataScope::Agenda]).await;
    let saved = WorkItems::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("project"))?;
    Ok(Json(convert::work_item(saved)))
}

/// `POST /api/work-items/{id}/cancel` — stops a project for good. It stays in
/// History with the reason; its upcoming schedule items (and bookings) go.
pub async fn cancel(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CancelProjectRequest>,
) -> AppResult<Json<WorkItem>> {
    user.require(Permission::ManageProjects)?;
    let reason = req.reason.trim();
    if reason.is_empty() {
        return Err(AppError::bad("Please say why the project is being cancelled."));
    }
    let txn = state.db.begin().await?;
    let row = WorkItems::find_by_id(id).one(&txn).await?.ok_or_else(|| AppError::not_found("project"))?;
    if !convert::work_item(row.clone()).status.is_active() {
        return Err(AppError::bad("Only an ongoing project can be cancelled."));
    }
    let mut a = row.into_active_model();
    a.status = Set(enc(&WorkStatus::Cancelled));
    a.active_worker_count = Set(0);
    a.update(&txn).await?;
    EstateAgendaItems::delete_many()
        .filter(estate_agenda_item::Column::WorkItemId.eq(id))
        .filter(estate_agenda_item::Column::StartTime.gt(chrono::Utc::now()))
        .exec(&txn)
        .await?;
    log_timeline(&txn, id, "Project cancelled", format!("{} cancelled the project: {reason}", user.0.name), false, DelayCategory::None, 0).await?;
    txn.commit().await?;
    state.hub.changed(&[DataScope::WorkItems, DataScope::Map, DataScope::Agenda]).await;
    let saved = WorkItems::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("project"))?;
    Ok(Json(convert::work_item(saved)))
}

/// `DELETE /api/work-items/{id}` — only for a project created by mistake:
/// nothing done yet, no photos and no reports. Anything else is cancelled instead.
pub async fn remove(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>) -> AppResult<axum::http::StatusCode> {
    user.require(Permission::ManageProjects)?;
    let txn = state.db.begin().await?;
    let row = WorkItems::find_by_id(id).one(&txn).await?.ok_or_else(|| AppError::not_found("project"))?;
    let photos = SitePhotos::find().filter(site_photo::Column::WorkItemId.eq(id)).count(&txn).await?;
    let reports = DecisionAlerts::find().filter(decision_alert::Column::WorkItemId.eq(id)).count(&txn).await?;
    let done = ProjectPhases::find()
        .filter(project_phase::Column::WorkItemId.eq(id))
        .filter(project_phase::Column::IsCompleted.eq(true))
        .count(&txn)
        .await?;
    if row.calculated_progress > 0 || photos > 0 || reports > 0 || done > 0 {
        return Err(AppError::bad("This project already has progress, photos or reports, so it can't be deleted. Cancel it instead — it stays in History."));
    }
    // Crew once moved from this job keeps its history; it just no longer points here.
    ManpowerLedgerEntries::update_many()
        .col_expr(manpower_ledger_entry::Column::SourceWorkItemId, sea_orm::sea_query::Expr::value(Option::<Uuid>::None))
        .filter(manpower_ledger_entry::Column::SourceWorkItemId.eq(id))
        .exec(&txn)
        .await?;
    row.delete(&txn).await?;
    txn.commit().await?;
    state.hub.changed(&[DataScope::WorkItems, DataScope::Map, DataScope::Agenda]).await;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// `POST /api/work-items/{id}/sign-off` — the owner accepts the handover.
pub async fn sign_off(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ExecutiveHandoverReport>> {
    user.require(Permission::SignOffProjects)?;
    Ok(Json(progress::sign_off(&state, id).await?))
}

// ------------------------------------------------------------ managing projects

/// Statuses the office may pick by hand; sign-off and completion only come from the workflow.
const HAND_PICKED: [WorkStatus; 3] = [WorkStatus::Planning, WorkStatus::InProgress, WorkStatus::Halted];

/// Checks the phase list: names present, each weight 1–100, total exactly 100.
pub fn check_phases(phases: &[PhaseInput]) -> Result<(), AppError> {
    if phases.is_empty() {
        return Err(AppError::bad("Add at least one phase for a construction project."));
    }
    if phases.len() > 30 {
        return Err(AppError::bad("Keep it to 30 phases or fewer."));
    }
    for p in phases {
        if p.phase_name.trim().is_empty() || p.phase_name.trim().chars().count() > 80 {
            return Err(AppError::bad("Every phase needs a name (up to 80 characters)."));
        }
        if !(1..=100).contains(&p.weight_percentage) {
            return Err(AppError::bad(format!("\u{201c}{}\u{201d} needs a share between 1% and 100%.", p.phase_name.trim())));
        }
    }
    let total: u32 = phases.iter().map(|p| p.weight_percentage as u32).sum();
    if total != 100 {
        return Err(AppError::bad(format!("The phase shares add up to {total}%. They must add up to exactly 100%.")));
    }
    Ok(())
}

/// Validates the shared parts of the project form; returns the trimmed title.
async fn check_form(state: &AppState, req: &SaveWorkItemRequest) -> AppResult<String> {
    let title = req.title.trim().to_string();
    if title.is_empty() || title.chars().count() > 120 {
        return Err(AppError::bad("Please enter a project name (up to 120 characters)."));
    }
    let area = SpatialAreas::find_by_id(req.area_id).one(&state.db).await?.ok_or_else(|| AppError::not_found("zone"))?;
    if area.is_archived {
        return Err(AppError::bad("That zone is archived. Pick an active location."));
    }
    Ok(title)
}

fn set_personnel(a: &mut work_item::ActiveModel, p: &ProjectPersonnel) {
    a.group_leader_name = Set(p.group_leader_name.trim().into());
    a.group_leader_phone = Set(p.group_leader_phone.trim().into());
    a.site_qc_officer_name = Set(p.site_qc_officer_name.trim().into());
    a.site_qc_officer_phone = Set(p.site_qc_officer_phone.trim().into());
    a.admin_coordinator_name = Set(p.admin_coordinator_name.trim().into());
    a.admin_coordinator_phone = Set(p.admin_coordinator_phone.trim().into());
}

fn handover_title(title: &str) -> String {
    format!("{title} \u{2014} target handover")
}

/// `POST /api/work-items` — start a new project, upkeep task or repair (Office Admin).
pub async fn create(State(state): State<AppState>, user: CurrentUser, Json(req): Json<SaveWorkItemRequest>) -> AppResult<Json<WorkItem>> {
    user.require(Permission::ManageProjects)?;
    let title = check_form(&state, &req).await?;
    if !HAND_PICKED.contains(&req.status) {
        return Err(AppError::bad("A new project starts as Planning, In progress or Paused."));
    }
    let capital = req.stream == WorkStream::CapitalProject;
    if capital {
        check_phases(&req.phases)?;
    }
    let id = Uuid::new_v4();
    let txn = state.db.begin().await?;
    let mut a = work_item::ActiveModel {
        id: Set(id),
        area_id: Set(req.area_id),
        stream: Set(enc(&req.stream)),
        title: Set(title.clone()),
        description: Set(req.description.trim().into()),
        status: Set(enc(&req.status)),
        calculated_progress: Set(0),
        original_deadline: Set(req.target_deadline),
        current_target_deadline: Set(req.target_deadline),
        active_worker_count: Set(0),
        before_photo_url: Set(None),
        after_photo_url: Set(None),
        ..Default::default()
    };
    set_personnel(&mut a, &req.personnel);
    a.insert(&txn).await?;
    if capital {
        for (i, p) in req.phases.iter().enumerate() {
            project_phase::ActiveModel {
                id: Set(Uuid::new_v4()),
                work_item_id: Set(id),
                phase_name: Set(p.phase_name.trim().into()),
                order_index: Set(i as i32 + 1),
                weight_percentage: Set(p.weight_percentage as i32),
                is_completed: Set(false),
                completed_at: Set(None),
                proof_photo_url: Set(None),
            }
            .insert(&txn)
            .await?;
        }
        estate_agenda_item::ActiveModel {
            id: Set(Uuid::new_v4()),
            area_id: Set(Some(req.area_id)),
            work_item_id: Set(Some(id)),
            title: Set(handover_title(&title)),
            description: Set("Planned completion and handover.".into()),
            event_type: Set(enc(&AgendaEventType::TargetDeadline)),
            intrusion_level: Set(enc(&IntrusionLevel::Silent)),
            start_time: Set(req.target_deadline - Duration::hours(1)),
            end_time: Set(req.target_deadline),
            is_flagged_conflict: Set(false),
        }
        .insert(&txn)
        .await?;
    }
    log_timeline(
        &txn,
        id,
        "Project created",
        format!("{} set up this {}.", user.0.name, req.stream.label().to_lowercase()),
        false,
        DelayCategory::None,
        0,
    )
    .await?;
    txn.commit().await?;
    state.hub.changed(&[DataScope::WorkItems, DataScope::Map, DataScope::Agenda]).await;
    let saved = WorkItems::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("project"))?;
    Ok(Json(convert::work_item(saved)))
}

/// `PUT /api/work-items/{id}` — edit details, deadline, status and phases (Office Admin).
pub async fn update(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SaveWorkItemRequest>,
) -> AppResult<Json<WorkItem>> {
    user.require(Permission::ManageProjects)?;
    let row = WorkItems::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("project"))?;
    let current = convert::work_item(row.clone());
    let title = check_form(&state, &req).await?;
    if req.stream != current.stream {
        return Err(AppError::bad("A project's type can't be changed after it's created."));
    }
    // Once every phase is done the handover workflow owns the status and phases.
    let locked = !HAND_PICKED.contains(&current.status);
    if !locked && !HAND_PICKED.contains(&req.status) {
        return Err(AppError::bad("Choose Planning, In progress or Paused."));
    }
    let txn = state.db.begin().await?;
    let mut notes: Vec<String> = Vec::new();
    let mut progress = current.calculated_progress;

    if current.stream == WorkStream::CapitalProject && !locked {
        check_phases(&req.phases)?;
        let existing: Vec<ProjectPhase> = ProjectPhases::find()
            .filter(project_phase::Column::WorkItemId.eq(id))
            .all(&txn)
            .await?
            .into_iter()
            .map(convert::phase)
            .collect();
        if req.phases.iter().filter_map(|p| p.id).any(|pid| !existing.iter().any(|e| e.id == pid)) {
            return Err(AppError::bad("One of the phases no longer exists. Please refresh and try again."));
        }
        let removed: Vec<&ProjectPhase> = existing.iter().filter(|e| !req.phases.iter().any(|p| p.id == Some(e.id))).collect();
        if let Some(done) = removed.iter().find(|e| e.is_completed) {
            return Err(AppError::bad(format!("\u{201c}{}\u{201d} is already finished, so it can't be removed.", done.phase_name)));
        }
        let after: Vec<ProjectPhase> = req
            .phases
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let old = p.id.and_then(|pid| existing.iter().find(|e| e.id == pid));
                ProjectPhase {
                    id: p.id.unwrap_or_else(Uuid::new_v4),
                    work_item_id: id,
                    phase_name: p.phase_name.trim().into(),
                    order_index: (i + 1) as u8,
                    weight_percentage: p.weight_percentage,
                    is_completed: old.is_some_and(|o| o.is_completed),
                    completed_at: old.and_then(|o| o.completed_at),
                    proof_photo_url: old.and_then(|o| o.proof_photo_url.clone()),
                }
            })
            .collect();
        if after.iter().all(|p| p.is_completed) {
            return Err(AppError::bad("Keep at least one unfinished phase. To finish the project, complete its last phase."));
        }
        let edited = after.iter().filter(|a| !existing.contains(a)).count();
        if !removed.is_empty() || edited > 0 {
            for r in &removed {
                ProjectPhases::delete_by_id(r.id).exec(&txn).await?;
            }
            for p in &after {
                let model = project_phase::ActiveModel {
                    id: Set(p.id),
                    work_item_id: Set(id),
                    phase_name: Set(p.phase_name.clone()),
                    order_index: Set(p.order_index as i32),
                    weight_percentage: Set(p.weight_percentage as i32),
                    is_completed: Set(p.is_completed),
                    completed_at: Set(p.completed_at),
                    proof_photo_url: Set(p.proof_photo_url.clone()),
                };
                if existing.iter().any(|e| e.id == p.id) {
                    model.update(&txn).await?;
                } else {
                    model.insert(&txn).await?;
                }
            }
            progress = displayed_progress(weighted_progress(&after), false);
            notes.push(format!("updated the phases ({} in total); progress is now {progress}%.", after.len()));
        }
    }

    if req.target_deadline != current.current_target_deadline {
        notes.push(format!(
            "moved the target date from {} to {}.",
            current.current_target_deadline.with_timezone(&Local).format("%B %-d"),
            req.target_deadline.with_timezone(&Local).format("%B %-d")
        ));
    }
    let status = if locked { current.status } else { req.status };
    if status != current.status {
        notes.push(format!("changed the status to {}.", status.staff_label()));
    }

    let mut a = row.into_active_model();
    a.area_id = Set(req.area_id);
    a.title = Set(title.clone());
    a.description = Set(req.description.trim().into());
    a.status = Set(enc(&status));
    a.calculated_progress = Set(progress as i32);
    a.current_target_deadline = Set(req.target_deadline);
    set_personnel(&mut a, &req.personnel);
    a.update(&txn).await?;

    // Keep the schedule's handover entry in step with the project.
    for e in EstateAgendaItems::find()
        .filter(estate_agenda_item::Column::WorkItemId.eq(id))
        .filter(estate_agenda_item::Column::EventType.eq(enc(&AgendaEventType::TargetDeadline)))
        .all(&txn)
        .await?
    {
        let mut ea = e.into_active_model();
        ea.title = Set(handover_title(&title));
        ea.area_id = Set(Some(req.area_id));
        ea.start_time = Set(req.target_deadline - Duration::hours(1));
        ea.end_time = Set(req.target_deadline);
        ea.update(&txn).await?;
    }

    if !notes.is_empty() {
        log_timeline(&txn, id, "Project details updated", format!("{} {}", user.0.name, notes.join(" Also ")), false, DelayCategory::None, 0).await?;
    }
    txn.commit().await?;
    state.hub.changed(&[DataScope::WorkItems, DataScope::Map, DataScope::Agenda]).await;
    let saved = WorkItems::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("project"))?;
    Ok(Json(convert::work_item(saved)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(name: &str, w: u8) -> PhaseInput {
        PhaseInput { id: None, phase_name: name.into(), weight_percentage: w }
    }

    #[test]
    fn phases_must_total_100() {
        assert!(check_phases(&[p("Dig", 40), p("Pour", 60)]).is_ok());
        assert!(check_phases(&[p("Dig", 40), p("Pour", 50)]).is_err());
        assert!(check_phases(&[p("Dig", 0), p("Pour", 100)]).is_err());
        assert!(check_phases(&[p(" ", 100)]).is_err());
        assert!(check_phases(&[]).is_err());
    }
}
