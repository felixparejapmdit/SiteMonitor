//! Context-aware equipment movement, component nesting and breakdown reports.

use axum::extract::{Path, State};
use axum::Json;
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};
use shared::api::*;
use shared::ws::{DataScope, WsEvent};
use shared::*;
use uuid::Uuid;

use crate::convert::{self, enc};
use crate::entities::{asset, asset_transfer_log, prelude::*, work_item};
use crate::error::{AppError, AppResult};
use crate::services::{create_notice, log_timeline, NewNotice};
use crate::state::{AppState, CurrentUser};

/// `GET /api/assets`
pub async fn list(State(state): State<AppState>) -> AppResult<Json<AssetsResponse>> {
    let mut assets: Vec<Asset> = Assets::find().all(&state.db).await?.into_iter().map(convert::asset).collect();
    assets.sort_by(|a, b| a.name.cmp(&b.name));
    let recent_transfers = AssetTransferLogs::find()
        .order_by_desc(asset_transfer_log::Column::Timestamp)
        .limit(50)
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::transfer)
        .collect();
    Ok(Json(AssetsResponse { assets, recent_transfers }))
}

/// Loads equipment that is still in use (retired equipment can't be moved or reported).
async fn active_asset(state: &AppState, id: Uuid) -> AppResult<asset::Model> {
    let row = Assets::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("equipment"))?;
    if row.is_retired {
        return Err(AppError::bad("This equipment has been retired. Bring it back into use first (Edit → In use)."));
    }
    Ok(row)
}

/// Checks the name and location from the equipment form.
async fn check_form(state: &AppState, req: &SaveAssetRequest) -> AppResult<String> {
    let name = req.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(AppError::bad("Please enter the equipment's name (up to 80 characters)."));
    }
    let area = SpatialAreas::find_by_id(req.current_area_id).one(&state.db).await?.ok_or_else(|| AppError::not_found("zone"))?;
    if area.is_archived {
        return Err(AppError::bad("That zone is archived. Pick an active location."));
    }
    Ok(name)
}

/// `POST /api/assets` — add equipment (Office Admin).
pub async fn create(State(state): State<AppState>, user: CurrentUser, Json(req): Json<SaveAssetRequest>) -> AppResult<Json<Asset>> {
    user.require(Permission::ManageEquipment)?;
    let name = check_form(&state, &req).await?;
    let saved = asset::ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(name),
        mobility: Set(enc(&req.mobility)),
        current_area_id: Set(req.current_area_id),
        parent_asset_id: Set(None),
        health: Set(enc(&AssetHealth::Operational)),
        active_issue_report: Set(None),
        impact_level: Set(enc(&ImpactLevel::NoImpact)),
        repair_eta: Set(None),
        is_retired: Set(false),
    }
    .insert(&state.db)
    .await?;
    state.hub.changed(&[DataScope::Assets, DataScope::Map]).await;
    Ok(Json(convert::asset(saved)))
}

/// `PUT /api/assets/{id}` — rename, change type or home zone, retire or bring back (Office Admin).
pub async fn update(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SaveAssetRequest>,
) -> AppResult<Json<Asset>> {
    user.require(Permission::ManageEquipment)?;
    let row = Assets::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("equipment"))?;
    let name = check_form(&state, &req).await?;
    let moving = row.current_area_id != req.current_area_id;
    if moving && row.parent_asset_id.is_some() {
        return Err(AppError::bad("This part is installed inside other equipment. Detach it first, then move it."));
    }
    let txn = state.db.begin().await?;
    if req.is_retired && !row.is_retired {
        let parts = Assets::find()
            .filter(asset::Column::ParentAssetId.eq(id))
            .filter(asset::Column::IsRetired.eq(false))
            .count(&txn)
            .await?;
        if parts > 0 {
            return Err(AppError::bad("Parts are still installed in this equipment. Detach or retire them first."));
        }
    }
    if moving {
        move_with_children(&txn, id, req.current_area_id, user.0.id).await?;
    }
    let mut a = Assets::find_by_id(id).one(&txn).await?.ok_or_else(|| AppError::not_found("equipment"))?.into_active_model();
    a.name = Set(name);
    a.mobility = Set(enc(&req.mobility));
    a.is_retired = Set(req.is_retired);
    if req.is_retired {
        // A retired part no longer sits inside anything.
        a.parent_asset_id = Set(None);
    }
    let saved = a.update(&txn).await?;
    txn.commit().await?;
    state.hub.changed(&[DataScope::Assets, DataScope::Map, DataScope::WorkItems]).await;
    Ok(Json(convert::asset(saved)))
}

/// Moves an asset and every component nested inside it, logging each move.
async fn move_with_children<C: ConnectionTrait>(db: &C, root: Uuid, to: Uuid, by: Uuid) -> AppResult<()> {
    let mut queue = vec![root];
    while let Some(id) = queue.pop() {
        let row = Assets::find_by_id(id).one(db).await?.ok_or_else(|| AppError::not_found("equipment"))?;
        let from = row.current_area_id;
        if from != to {
            asset_transfer_log::ActiveModel {
                id: Set(Uuid::new_v4()),
                asset_id: Set(id),
                from_area_id: Set(Some(from)),
                to_area_id: Set(to),
                moved_by: Set(by),
                timestamp: Set(Utc::now()),
            }
            .insert(db)
            .await?;
            let mut a = row.into_active_model();
            a.current_area_id = Set(to);
            a.update(db).await?;
        }
        for child in Assets::find().filter(asset::Column::ParentAssetId.eq(id)).all(db).await? {
            queue.push(child.id);
        }
    }
    Ok(())
}

/// `POST /api/assets/{id}/relocate`
pub async fn relocate(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RelocateAssetRequest>,
) -> AppResult<Json<Asset>> {
    user.require(Permission::OperateEquipment)?;
    let dest = SpatialAreas::find_by_id(req.to_area_id).one(&state.db).await?.ok_or_else(|| AppError::not_found("destination zone"))?;
    if dest.is_archived {
        return Err(AppError::bad("That zone is archived. Pick an active location."));
    }
    let row = active_asset(&state, id).await?;
    user.require_zone(&state.db, row.current_area_id).await?;
    user.require_zone(&state.db, dest.id).await?;
    if row.parent_asset_id.is_some() {
        return Err(AppError::bad(
            "This part is installed inside other equipment. Detach it first, then move it.",
        ));
    }
    let txn = state.db.begin().await?;
    move_with_children(&txn, id, req.to_area_id, user.0.id).await?;
    txn.commit().await?;
    state.hub.changed(&[DataScope::Assets, DataScope::Map]).await;
    let saved = Assets::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("equipment"))?;
    Ok(Json(convert::asset(saved)))
}

/// `POST /api/assets/{id}/link` — attach to / detach from a parent asset.
pub async fn link(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<LinkAssetRequest>,
) -> AppResult<Json<Asset>> {
    user.require(Permission::OperateEquipment)?;
    let row = active_asset(&state, id).await?;
    user.require_zone(&state.db, row.current_area_id).await?;
    let txn = state.db.begin().await?;
    match req.parent_asset_id {
        Some(parent_id) => {
            if parent_id == id {
                return Err(AppError::bad("A part can't be installed inside itself."));
            }
            // Walk up from the new parent to make sure we don't create a loop.
            let mut cursor = Some(parent_id);
            let mut parent_area = None;
            while let Some(c) = cursor {
                if c == id {
                    return Err(AppError::bad("That would place the equipment inside one of its own parts."));
                }
                let p = Assets::find_by_id(c).one(&txn).await?.ok_or_else(|| AppError::not_found("parent equipment"))?;
                if p.is_retired {
                    return Err(AppError::bad("That equipment has been retired. Pick one that's still in use."));
                }
                if parent_area.is_none() {
                    parent_area = Some(p.current_area_id);
                }
                cursor = p.parent_asset_id;
            }
            let mut a = row.into_active_model();
            a.parent_asset_id = Set(Some(parent_id));
            a.update(&txn).await?;
            if let Some(area) = parent_area {
                move_with_children(&txn, id, area, user.0.id).await?;
            }
        }
        None => {
            let mut a = row.into_active_model();
            a.parent_asset_id = Set(None);
            a.update(&txn).await?;
        }
    }
    txn.commit().await?;
    state.hub.changed(&[DataScope::Assets, DataScope::Map]).await;
    let saved = Assets::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("equipment"))?;
    Ok(Json(convert::asset(saved)))
}

/// `POST /api/assets/{id}/report-issue` — the machinery breakdown flow.
pub async fn report_issue(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ReportIssueRequest>,
) -> AppResult<Json<Asset>> {
    user.require(Permission::OperateEquipment)?;
    if req.health == AssetHealth::Operational {
        return Err(AppError::bad("Choose \"Needs service\" or \"Broken down\" to report a problem."));
    }
    if req.note.trim().is_empty() {
        return Err(AppError::bad("Please describe what's wrong and what you've done about it."));
    }
    let row = active_asset(&state, id).await?;
    user.require_zone(&state.db, row.current_area_id).await?;
    let asset_name = row.name.clone();
    let txn = state.db.begin().await?;
    let mut a = row.into_active_model();
    a.health = Set(enc(&req.health));
    a.impact_level = Set(enc(&req.impact));
    a.active_issue_report = Set(Some(req.note.trim().to_string()));
    a.repair_eta = Set(req.repair_eta);
    let saved = convert::asset(a.update(&txn).await?);

    let mut work_item_title = None;
    if let Some(wid) = req.work_item_id {
        let w = WorkItems::find_by_id(wid).one(&txn).await?.ok_or_else(|| AppError::not_found("project"))?;
        work_item_title = Some(w.title.clone());
        let days = req.days_delayed.unwrap_or(0);
        let mut wa: work_item::ActiveModel = w.clone().into_active_model();
        if days > 0 {
            wa.current_target_deadline = Set(w.current_target_deadline + Duration::days(days as i64));
        }
        if req.impact == ImpactLevel::FullWorkStoppage {
            wa.status = Set(enc(&WorkStatus::Halted));
        }
        wa.update(&txn).await?;
        log_timeline(
            &txn,
            wid,
            format!("{asset_name}: {}", req.health.label().to_lowercase()),
            format!("{} {}", req.note.trim(), req.impact.label().to_lowercase().replace("work has", "— work has")),
            false,
            if days > 0 { DelayCategory::EquipmentFailure } else { DelayCategory::None },
            days,
        )
        .await?;
    }
    for role in [UserRole::OfficeAdmin, UserRole::SiteMonitor] {
        create_notice(
            &txn,
            NewNotice {
                audience: role,
                kind: NotificationKind::General,
                work_item_id: req.work_item_id,
                title: format!("{asset_name} reported: {}", req.health.label().to_lowercase()),
                body: format!("{} Reported by {}.", req.note.trim(), user.0.name),
                lines: vec![],
                visible_from: Utc::now(),
                dedupe_key: None,
            },
        )
        .await?;
    }
    txn.commit().await?;

    if req.impact == ImpactLevel::FullWorkStoppage {
        state.hub.to_vip(WsEvent::EquipmentStoppage { asset: saved.clone(), work_item_title }).await;
    }
    state
        .hub
        .changed(&[DataScope::Assets, DataScope::Map, DataScope::WorkItems, DataScope::Notifications])
        .await;
    Ok(Json(saved))
}

/// `POST /api/assets/{id}/mark-repaired` — back in service; resumes work it stopped.
pub async fn mark_repaired(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Asset>> {
    user.require(Permission::OperateEquipment)?;
    let row = Assets::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("equipment"))?;
    user.require_zone(&state.db, row.current_area_id).await?;
    let was_stoppage = row.impact_level == enc(&ImpactLevel::FullWorkStoppage);
    let area = row.current_area_id;
    let name = row.name.clone();
    let txn = state.db.begin().await?;
    let mut a = row.into_active_model();
    a.health = Set(enc(&AssetHealth::Operational));
    a.impact_level = Set(enc(&ImpactLevel::NoImpact));
    a.active_issue_report = Set(None);
    a.repair_eta = Set(None);
    let saved = convert::asset(a.update(&txn).await?);
    if was_stoppage {
        let flagged: Vec<Uuid> = EstateAgendaItems::find()
            .filter(crate::entities::estate_agenda_item::Column::IsFlaggedConflict.eq(true))
            .all(&txn)
            .await?
            .into_iter()
            .filter_map(|i| i.work_item_id)
            .collect();
        for w in WorkItems::find()
            .filter(work_item::Column::Status.eq(enc(&WorkStatus::Halted)))
            .all(&txn)
            .await?
        {
            if flagged.contains(&w.id) {
                continue;
            }
            let related = w.area_id == area
                || TimelineEventRecords::find()
                    .filter(crate::entities::timeline_event_record::Column::WorkItemId.eq(w.id))
                    .filter(crate::entities::timeline_event_record::Column::Title.starts_with(format!("{name}:")))
                    .one(&txn)
                    .await?
                    .is_some();
            if related {
                let wid = w.id;
                let mut wa = w.into_active_model();
                wa.status = Set(enc(&WorkStatus::InProgress));
                wa.update(&txn).await?;
                log_timeline(&txn, wid, "Work resumed", format!("{name} is repaired and back in service."), false, DelayCategory::None, 0).await?;
            }
        }
    }
    txn.commit().await?;
    state.hub.changed(&[DataScope::Assets, DataScope::Map, DataScope::WorkItems]).await;
    Ok(Json(saved))
}
