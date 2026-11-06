//! Context-aware equipment movement, component nesting and breakdown reports.

use axum::extract::{Path, State};
use axum::Json;
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, QuerySelect, Set,
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
    user.require_staff("move equipment")?;
    let dest = SpatialAreas::find_by_id(req.to_area_id).one(&state.db).await?.ok_or_else(|| AppError::not_found("destination zone"))?;
    if dest.is_archived {
        return Err(AppError::bad("That zone is archived. Pick an active location."));
    }
    let row = Assets::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("equipment"))?;
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
    user.require_staff("attach or detach parts")?;
    let row = Assets::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("equipment"))?;
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
    user.require_staff("report breakdowns")?;
    if req.health == AssetHealth::Operational {
        return Err(AppError::bad("Choose \"Needs service\" or \"Broken down\" to report a problem."));
    }
    if req.note.trim().is_empty() {
        return Err(AppError::bad("Please describe what's wrong and what you've done about it."));
    }
    let row = Assets::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("equipment"))?;
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
    user.require_staff("mark equipment as repaired")?;
    let row = Assets::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("equipment"))?;
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
