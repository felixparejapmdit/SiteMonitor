//! Projects, maintenance tasks and repairs.

use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::Json;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use shared::api::*;
use shared::*;
use uuid::Uuid;

use crate::convert;
use crate::entities::{
    decision_alert, executive_handover_report, manpower_ledger_entry, prelude::*, project_phase, site_photo,
    timeline_event_record,
};
use crate::error::{AppError, AppResult};
use crate::services::{all_areas, media, progress, subtree_ids};
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
    if user.0.role == UserRole::VipClient {
        decisions.retain(|d| d.status != DecisionStatus::DraftInAdminQueue);
    }

    let zone = subtree_ids(&areas, item.area_id);
    let zone_assets = Assets::find()
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::asset)
        .filter(|a| zone.contains(&a.current_area_id))
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
    user.require_staff("complete project phases")?;
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
    user.require_staff("mark tasks completed")?;
    let proof = match req.proof_data_url.as_deref().filter(|s| !s.is_empty()) {
        Some(data) => Some(media::save_data_url(&state.upload_dir, "photos", data).await?),
        None => None,
    };
    Ok(Json(progress::complete_task(&state, id, proof, req.note, &user.0).await?))
}

/// `POST /api/work-items/{id}/sign-off` — the owner accepts the handover.
pub async fn sign_off(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ExecutiveHandoverReport>> {
    user.require_vip("sign off projects")?;
    Ok(Json(progress::sign_off(&state, id).await?))
}
