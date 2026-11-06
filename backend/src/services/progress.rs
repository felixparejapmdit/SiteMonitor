//! Weighted progress & the 99% handover lock (Phase 2 §3.1).

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use shared::api::ProgressUpdate;
use shared::labels::{displayed_progress, weighted_progress};
use shared::ws::{DataScope, WsEvent};
use shared::*;
use uuid::Uuid;

use crate::convert::{self, enc, to_json};
use crate::entities::{executive_handover_report, prelude::*, project_phase, site_photo, timeline_event_record, work_item};
use crate::error::{AppError, AppResult};
use crate::services::{all_areas, log_timeline, subtree_ids};
use crate::state::AppState;

/// Marks a phase complete, recalculates weighted progress and, when every
/// phase is done, locks the project at 99% and compiles the handover report.
pub async fn complete_phase(
    state: &AppState,
    phase_id: Uuid,
    proof_photo_url: Option<String>,
    user: &User,
) -> AppResult<ProgressUpdate> {
    let txn = state.db.begin().await?;
    let phase = ProjectPhases::find_by_id(phase_id).one(&txn).await?.ok_or_else(|| AppError::not_found("phase"))?;
    let item_row = WorkItems::find_by_id(phase.work_item_id)
        .one(&txn)
        .await?
        .ok_or_else(|| AppError::not_found("project"))?;
    let item = convert::work_item(item_row.clone());
    if item.status == WorkStatus::CompletedAndArchived {
        return Err(AppError::bad("This project is already completed and archived."));
    }

    if !phase.is_completed {
        let phase_name = phase.phase_name.clone();
        let weight = phase.weight_percentage;
        let mut active = phase.into_active_model();
        active.is_completed = Set(true);
        active.completed_at = Set(Some(Utc::now()));
        active.proof_photo_url = Set(proof_photo_url.clone());
        active.update(&txn).await?;

        if let Some(url) = &proof_photo_url {
            site_photo::ActiveModel {
                id: Set(Uuid::new_v4()),
                work_item_id: Set(item.id),
                area_id: Set(item.area_id),
                image_url: Set(url.clone()),
                caption: Set(format!("Proof of completion: {phase_name}")),
                uploaded_by: Set(user.id),
                uploaded_at: Set(Utc::now()),
                client_acknowledged: Set(false),
            }
            .insert(&txn)
            .await?;
        }

        let phases: Vec<ProjectPhase> = ProjectPhases::find()
            .filter(project_phase::Column::WorkItemId.eq(item.id))
            .all(&txn)
            .await?
            .into_iter()
            .map(convert::phase)
            .collect();
        let raw = weighted_progress(&phases);
        let shown = displayed_progress(raw, false);
        log_timeline(
            &txn,
            item.id,
            format!("Phase completed: {phase_name}"),
            format!("{phase_name} ({weight}% of the work) was finished by {}. Overall progress is now {shown}%.", user.name),
            true,
            DelayCategory::None,
            0,
        )
        .await?;

        let all_done = !phases.is_empty() && phases.iter().all(|p| p.is_completed);
        let mut active = item_row.into_active_model();
        active.calculated_progress = Set(shown as i32);
        if all_done {
            active.status = Set(enc(&WorkStatus::PendingSignOff));
            let after = latest_photo_url(&txn, item.id).await?;
            if after.is_some() {
                active.after_photo_url = Set(after);
            }
        } else if item.status == WorkStatus::Planning {
            active.status = Set(enc(&WorkStatus::InProgress));
        }
        active.update(&txn).await?;

        if all_done {
            compile_handover(&txn, item.id).await?;
            log_timeline(
                &txn,
                item.id,
                "Ready for owner sign-off",
                "Every phase is finished. The project is held at 99% until the owner reviews the handover report.",
                true,
                DelayCategory::None,
                0,
            )
            .await?;
        }
    }
    txn.commit().await?;

    let updated = WorkItems::find_by_id(item.id).one(&state.db).await?.map(convert::work_item).ok_or_else(|| AppError::not_found("project"))?;
    let handover_created = updated.status == WorkStatus::PendingSignOff;
    state.hub.changed(&[DataScope::WorkItems, DataScope::Map, DataScope::Photos]).await;
    Ok(ProgressUpdate { item: updated, handover_created })
}

/// Marks a routine-maintenance or repair task done (one tap + proof photo).
pub async fn complete_task(
    state: &AppState,
    work_item_id: Uuid,
    proof_photo_url: Option<String>,
    note: Option<String>,
    user: &User,
) -> AppResult<WorkItem> {
    let row = WorkItems::find_by_id(work_item_id).one(&state.db).await?.ok_or_else(|| AppError::not_found("task"))?;
    let item = convert::work_item(row.clone());
    if item.stream == WorkStream::CapitalProject {
        return Err(AppError::bad("Construction projects are completed phase by phase."));
    }
    let txn = state.db.begin().await?;
    if let Some(url) = &proof_photo_url {
        site_photo::ActiveModel {
            id: Set(Uuid::new_v4()),
            work_item_id: Set(item.id),
            area_id: Set(item.area_id),
            image_url: Set(url.clone()),
            caption: Set(format!("{} — completed", item.title)),
            uploaded_by: Set(user.id),
            uploaded_at: Set(Utc::now()),
            client_acknowledged: Set(false),
        }
        .insert(&txn)
        .await?;
    }
    let local_time = Utc::now().with_timezone(&chrono::Local).format("%-I:%M %p");
    log_timeline(
        &txn,
        item.id,
        format!("{} — Completed {local_time}", item.title),
        note.filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| format!("Marked completed by {}.", user.name)),
        true,
        DelayCategory::None,
        0,
    )
    .await?;
    let mut active = row.into_active_model();
    match item.stream {
        // Routine upkeep recurs: it stays active and simply logs today's visit.
        WorkStream::RoutineMaintenance => {
            active.status = Set(enc(&WorkStatus::InProgress));
            active.calculated_progress = Set(100);
            if let Some(url) = proof_photo_url {
                active.after_photo_url = Set(Some(url));
            }
        }
        // A repair is finished for good.
        _ => {
            active.status = Set(enc(&WorkStatus::CompletedAndArchived));
            active.calculated_progress = Set(100);
            if let Some(url) = proof_photo_url {
                active.after_photo_url = Set(Some(url));
            }
        }
    }
    let saved = active.update(&txn).await?;
    txn.commit().await?;
    state.hub.changed(&[DataScope::WorkItems, DataScope::Map, DataScope::Photos]).await;
    Ok(convert::work_item(saved))
}

async fn latest_photo_url<C: sea_orm::ConnectionTrait>(db: &C, work_item_id: Uuid) -> AppResult<Option<String>> {
    Ok(SitePhotos::find()
        .filter(site_photo::Column::WorkItemId.eq(work_item_id))
        .order_by_desc(site_photo::Column::UploadedAt)
        .one(db)
        .await?
        .map(|p| p.image_url))
}

/// Builds (or rebuilds) the Executive Handover Report for a work item.
pub async fn compile_handover<C: sea_orm::ConnectionTrait>(db: &C, work_item_id: Uuid) -> AppResult<ExecutiveHandoverReport> {
    let item = WorkItems::find_by_id(work_item_id).one(db).await?.map(convert::work_item).ok_or_else(|| AppError::not_found("project"))?;
    let photos: Vec<SitePhoto> = SitePhotos::find()
        .filter(site_photo::Column::WorkItemId.eq(work_item_id))
        .order_by_asc(site_photo::Column::UploadedAt)
        .all(db)
        .await?
        .into_iter()
        .map(convert::photo)
        .collect();
    let before = item
        .before_photo_url
        .clone()
        .or_else(|| photos.first().map(|p| p.image_url.clone()))
        .unwrap_or_default();
    let after = item
        .after_photo_url
        .clone()
        .or_else(|| photos.last().map(|p| p.image_url.clone()))
        .unwrap_or_else(|| before.clone());

    let delays: Vec<TimelineEventRecord> = TimelineEventRecords::find()
        .filter(timeline_event_record::Column::WorkItemId.eq(work_item_id))
        .filter(timeline_event_record::Column::DaysDelayed.gt(0))
        .order_by_asc(timeline_event_record::Column::Timestamp)
        .all(db)
        .await?
        .into_iter()
        .map(convert::timeline)
        .collect();
    let total_delay_days: u16 = delays.iter().map(|d| d.days_delayed).sum();
    let summary: Vec<String> = delays
        .iter()
        .map(|d| {
            let unit = if d.days_delayed == 1 { "Day" } else { "Days" };
            format!(
                "+{} {unit}: {} — {} ({})",
                d.days_delayed,
                d.delay_category.label(),
                d.description,
                d.timestamp.format("%b %-d")
            )
        })
        .collect();

    let areas = all_areas(db).await?;
    let zone = subtree_ids(&areas, item.area_id);
    let assets: Vec<Asset> = Assets::find()
        .all(db)
        .await?
        .into_iter()
        .map(convert::asset)
        .filter(|a| a.mobility == MobilityType::FixedInfrastructure && zone.contains(&a.current_area_id))
        .collect();

    let existing = ExecutiveHandoverReports::find()
        .filter(executive_handover_report::Column::WorkItemId.eq(work_item_id))
        .one(db)
        .await?;
    let saved = match existing {
        Some(row) => {
            let mut active = row.into_active_model();
            active.before_photo_url = Set(before);
            active.after_photo_url = Set(after);
            active.original_deadline = Set(item.original_deadline);
            active.actual_completion_date = Set(Utc::now());
            active.total_delay_days = Set(total_delay_days as i32);
            active.delay_audit_summary = Set(to_json(&summary));
            active.handed_over_assets = Set(to_json(&assets));
            active.update(db).await?
        }
        None => {
            executive_handover_report::ActiveModel {
                id: Set(Uuid::new_v4()),
                work_item_id: Set(work_item_id),
                before_photo_url: Set(before),
                after_photo_url: Set(after),
                original_deadline: Set(item.original_deadline),
                actual_completion_date: Set(Utc::now()),
                total_delay_days: Set(total_delay_days as i32),
                delay_audit_summary: Set(to_json(&summary)),
                handed_over_assets: Set(to_json(&assets)),
                client_signed_off: Set(false),
            }
            .insert(db)
            .await?
        }
    };
    Ok(convert::handover(saved))
}

/// The owner accepts the handover: 100% and archived (Phase 2 §3.1 step 4).
pub async fn sign_off(state: &AppState, work_item_id: Uuid) -> AppResult<ExecutiveHandoverReport> {
    let row = WorkItems::find_by_id(work_item_id).one(&state.db).await?.ok_or_else(|| AppError::not_found("project"))?;
    let item = convert::work_item(row.clone());
    if item.status != WorkStatus::PendingSignOff {
        return Err(AppError::bad("This project isn't ready for sign-off yet."));
    }
    let photo_ids: Vec<Uuid> = SitePhotos::find()
        .filter(site_photo::Column::WorkItemId.eq(work_item_id))
        .all(&state.db)
        .await?
        .into_iter()
        .map(|p| p.id)
        .collect();
    let open = PhotoCallOuts::find()
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::callout)
        .filter(|c| photo_ids.contains(&c.photo_id) && c.status != CallOutStatus::Resolved)
        .count();
    if open > 0 {
        return Err(AppError::bad(format!(
            "There {} still {open} open photo note{} on this project. The site team will resolve {} first.",
            if open == 1 { "is" } else { "are" },
            if open == 1 { "" } else { "s" },
            if open == 1 { "it" } else { "them" },
        )));
    }

    let txn = state.db.begin().await?;
    let report = match ExecutiveHandoverReports::find()
        .filter(executive_handover_report::Column::WorkItemId.eq(work_item_id))
        .one(&txn)
        .await?
    {
        Some(r) => r,
        None => {
            compile_handover(&txn, work_item_id).await?;
            ExecutiveHandoverReports::find()
                .filter(executive_handover_report::Column::WorkItemId.eq(work_item_id))
                .one(&txn)
                .await?
                .ok_or_else(|| AppError::not_found("handover report"))?
        }
    };
    let mut r = report.into_active_model();
    r.client_signed_off = Set(true);
    let report = r.update(&txn).await?;

    let mut active: work_item::ActiveModel = row.into_active_model();
    active.calculated_progress = Set(100);
    active.status = Set(enc(&WorkStatus::CompletedAndArchived));
    active.active_worker_count = Set(0);
    let saved = active.update(&txn).await?;
    log_timeline(
        &txn,
        work_item_id,
        "Signed off by the owner",
        "The owner accepted the handover report. The project is 100% complete and archived.",
        true,
        DelayCategory::None,
        0,
    )
    .await?;
    txn.commit().await?;

    state.hub.to_staff(WsEvent::ProjectSignedOff(convert::work_item(saved))).await;
    state.hub.changed(&[DataScope::WorkItems, DataScope::Map]).await;
    Ok(convert::handover(report))
}
