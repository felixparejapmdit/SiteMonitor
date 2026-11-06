//! Offline crew manpower: morning allocation and mid-day transfers.

use std::collections::HashMap;

use axum::extract::State;
use axum::Json;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set, TransactionTrait};
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::convert::{self, enc};
use crate::entities::{manpower_ledger_entry, notification, prelude::*, work_item};
use crate::error::{AppError, AppResult};
use crate::state::{AppState, CurrentUser};

/// `POST /api/manpower/reallocate`
pub async fn reallocate(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<ReallocateRequest>,
) -> AppResult<Json<Vec<WorkItem>>> {
    user.require_staff("move crew between jobs")?;
    if req.allocations.is_empty() && req.transfers.is_empty() {
        return Err(AppError::bad("Nothing to save — change at least one crew number."));
    }
    let txn = state.db.begin().await?;
    let mut rows: HashMap<Uuid, work_item::Model> =
        WorkItems::find().all(&txn).await?.into_iter().map(|w| (w.id, w)).collect();
    let mut touched = Vec::new();
    let now = Utc::now();
    let morning_reason = req
        .reason
        .clone()
        .filter(|r| !r.trim().is_empty())
        .unwrap_or_else(|| "Morning crew allocation".into());

    for alloc in &req.allocations {
        let row = rows.get_mut(&alloc.work_item_id).ok_or_else(|| AppError::not_found("job"))?;
        let delta = alloc.headcount as i32 - row.active_worker_count;
        if delta != 0 {
            manpower_ledger_entry::ActiveModel {
                id: Set(Uuid::new_v4()),
                work_item_id: Set(row.id),
                source_work_item_id: Set(None),
                headcount_delta: Set(delta),
                resulting_headcount: Set(alloc.headcount as i32),
                reason: Set(morning_reason.clone()),
                logged_by: Set(user.0.id),
                timestamp: Set(now),
            }
            .insert(&txn)
            .await?;
            row.active_worker_count = alloc.headcount as i32;
            touched.push(row.id);
        }
    }

    for t in &req.transfers {
        if t.source_work_item_id == t.destination_work_item_id {
            return Err(AppError::bad("Pick two different jobs to move workers between."));
        }
        if t.headcount == 0 {
            return Err(AppError::bad("Move at least 1 worker."));
        }
        let reason = if t.reason.trim().is_empty() { "Crew reshuffle".to_string() } else { t.reason.trim().to_string() };
        let (src_title, src_count) = {
            let src = rows.get(&t.source_work_item_id).ok_or_else(|| AppError::not_found("source job"))?;
            (src.title.clone(), src.active_worker_count)
        };
        if src_count < t.headcount as i32 {
            return Err(AppError::bad(format!(
                "{src_title} only has {src_count} workers today, so you can't move {}.",
                t.headcount
            )));
        }
        let dst_title = rows
            .get(&t.destination_work_item_id)
            .ok_or_else(|| AppError::not_found("destination job"))?
            .title
            .clone();
        let n = t.headcount as i32;

        let src = rows.get_mut(&t.source_work_item_id).expect("checked above");
        src.active_worker_count -= n;
        manpower_ledger_entry::ActiveModel {
            id: Set(Uuid::new_v4()),
            work_item_id: Set(src.id),
            source_work_item_id: Set(None),
            headcount_delta: Set(-n),
            resulting_headcount: Set(src.active_worker_count),
            reason: Set(format!("Moved to {dst_title}: {reason}")),
            logged_by: Set(user.0.id),
            timestamp: Set(now),
        }
        .insert(&txn)
        .await?;
        touched.push(src.id);

        let dst = rows.get_mut(&t.destination_work_item_id).expect("checked above");
        dst.active_worker_count += n;
        manpower_ledger_entry::ActiveModel {
            id: Set(Uuid::new_v4()),
            work_item_id: Set(dst.id),
            source_work_item_id: Set(Some(t.source_work_item_id)),
            headcount_delta: Set(n),
            resulting_headcount: Set(dst.active_worker_count),
            reason: Set(reason),
            logged_by: Set(user.0.id),
            timestamp: Set(now),
        }
        .insert(&txn)
        .await?;
        touched.push(dst.id);
    }

    touched.sort();
    touched.dedup();
    for id in &touched {
        let row = rows.get(id).expect("touched rows exist").clone();
        let count = row.active_worker_count;
        let mut active = row.into_active_model();
        active.active_worker_count = Set(count);
        active.reset(work_item::Column::ActiveWorkerCount);
        active.update(&txn).await?;
    }

    // The morning prompt is answered once today's crew has been allocated.
    if !req.allocations.is_empty() {
        for n in Notifications::find()
            .filter(notification::Column::Kind.eq(enc(&NotificationKind::MorningRosterPrompt)))
            .filter(notification::Column::Dismissed.eq(false))
            .all(&txn)
            .await?
        {
            let mut a = n.into_active_model();
            a.dismissed = Set(true);
            a.update(&txn).await?;
        }
    }
    txn.commit().await?;
    state.hub.changed(&[DataScope::WorkItems, DataScope::Notifications]).await;

    let items = WorkItems::find().all(&state.db).await?.into_iter().map(convert::work_item).collect();
    Ok(Json(items))
}
