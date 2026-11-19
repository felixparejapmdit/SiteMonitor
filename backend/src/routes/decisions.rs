//! The Draft-to-VIP decision pipeline.

use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, QueryOrder, Set, TransactionTrait};

use shared::api::*;
use shared::ws::{DataScope, WsEvent};
use shared::*;
use uuid::Uuid;

use crate::convert::{self, enc};
use crate::entities::{decision_alert, prelude::*};
use crate::error::{AppError, AppResult};
use crate::services::{log_timeline, media};
use crate::state::{AppState, CurrentUser};

/// `GET /api/decisions` — the office sees every report, the owner what was sent
/// to them, and each member of the site team only the reports they wrote.
pub async fn list(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Vec<DecisionAlert>>> {
    let mut all: Vec<DecisionAlert> = DecisionAlerts::find()
        .order_by_desc(decision_alert::Column::CreatedAt)
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::decision)
        .collect();
    all.retain(|d| shared::access::can_see_report(&user.0, d));
    Ok(Json(all))
}

/// `POST /api/decisions` — a Site Monitor (or Admin) files a field draft.
pub async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<CreateDecisionRequest>,
) -> AppResult<Json<DecisionAlert>> {
    user.require(Permission::RaiseIssues)?;
    if req.title.trim().is_empty() || req.issue_description.trim().is_empty() {
        return Err(AppError::bad("Please add a short title and describe the issue."));
    }
    let item = WorkItems::find_by_id(req.work_item_id).one(&state.db).await?.ok_or_else(|| AppError::not_found("project"))?;
    user.require_zone(&state.db, item.area_id).await?;
    let photo_url = match req.photo_data_url.as_deref().filter(|s| !s.is_empty()) {
        Some(data) => Some(media::save_data_url(&state.upload_dir, "photos", data).await?),
        None => None,
    };
    // The photo stays with the report (not the shared project gallery), so only
    // the reporter, the office and, once sent, the owner see it.
    let row = decision_alert::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_item_id: Set(item.id),
        area_id: Set(item.area_id),
        urgency: Set(enc(&req.urgency)),
        category: Set(enc(&req.category)),
        title: Set(req.title.trim().to_string()),
        issue_description: Set(req.issue_description.trim().to_string()),
        mitigation_action_taken: Set(req.mitigation_action_taken.trim().to_string()),
        option_a_label: Set(None),
        option_b_label: Set(None),
        status: Set(enc(&DecisionStatus::DraftInAdminQueue)),
        created_at: Set(Utc::now()),
        resolved_at: Set(None),
        photo_url: Set(photo_url),
        reported_by: Set(Some(user.0.id)),
        office_note: Set(None),
    }
    .insert(&state.db)
    .await?;
    state.hub.changed(&[DataScope::Decisions]).await;
    Ok(Json(convert::decision(row)))
}

/// `PUT /api/decisions/{id}` — change a report that is still waiting for office
/// review. The office can change any report; the person who wrote it, their own.
pub async fn update(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDecisionRequest>,
) -> AppResult<Json<DecisionAlert>> {
    let row = DecisionAlerts::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("report"))?;
    let d = convert::decision(row.clone());
    let office = user.0.can(Permission::PublishDecisions);
    let author = user.0.can(Permission::RaiseIssues) && d.reported_by == Some(user.0.id);
    if !office && !author {
        return Err(AppError::forbidden("You can only change reports you wrote."));
    }
    // The office can change any report it still holds; the reporter, while it's
    // waiting for review or when the office asked them for more details.
    let editable = if office { d.status.is_open() } else { matches!(d.status, DecisionStatus::DraftInAdminQueue | DecisionStatus::NeedsDetails) };
    if !editable {
        return Err(AppError::bad("This report has already been dealt with, so it can't be changed any more."));
    }
    if req.title.trim().is_empty() || req.issue_description.trim().is_empty() {
        return Err(AppError::bad("Please add a short title and describe the issue."));
    }
    let item = WorkItems::find_by_id(req.work_item_id).one(&state.db).await?.ok_or_else(|| AppError::not_found("project"))?;
    let new_photo = match req.photo_data_url.as_deref().filter(|s| !s.is_empty()) {
        Some(data) => Some(media::save_data_url(&state.upload_dir, "photos", data).await?),
        None => None,
    };
    let mut a = row.into_active_model();
    a.work_item_id = Set(item.id);
    a.area_id = Set(item.area_id);
    a.urgency = Set(enc(&req.urgency));
    a.category = Set(enc(&req.category));
    a.title = Set(req.title.trim().to_string());
    a.issue_description = Set(req.issue_description.trim().to_string());
    a.mitigation_action_taken = Set(req.mitigation_action_taken.trim().to_string());
    if new_photo.is_some() {
        a.photo_url = Set(new_photo);
    } else if req.remove_photo {
        a.photo_url = Set(None);
    }
    // The reporter added the details the office asked for: back to the office.
    if !office && d.status == DecisionStatus::NeedsDetails {
        a.status = Set(enc(&DecisionStatus::DraftInAdminQueue));
    }
    let saved = convert::decision(a.update(&state.db).await?);
    state.hub.changed(&[DataScope::Decisions]).await;
    Ok(Json(saved))
}

/// `DELETE /api/decisions/{id}` — the office removes a report the owner hasn't
/// answered. Answered ones stay as part of the project history.
pub async fn remove(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>) -> AppResult<Json<serde_json::Value>> {
    user.require(Permission::PublishDecisions)?;
    let row = DecisionAlerts::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("report"))?;
    let status: DecisionStatus = convert::dec(&row.status);
    if status.is_resolved() {
        return Err(AppError::bad("The owner has already answered this, so it stays in the project history."));
    }
    DecisionAlerts::delete_by_id(id).exec(&state.db).await?;
    state.hub.changed(&[DataScope::Decisions, DataScope::Map]).await;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// `POST /api/decisions/{id}/review` — one-tap office decision on a field report:
/// approve, decline, hold, ask the reporter for details, or move it back to review.
pub async fn review(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ReviewDecisionRequest>,
) -> AppResult<Json<DecisionAlert>> {
    user.require(Permission::PublishDecisions)?;
    let row = DecisionAlerts::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("report"))?;
    let current: DecisionStatus = convert::dec(&row.status);
    let next = current
        .after_review(req.action)
        .ok_or_else(|| AppError::bad(format!("This report is \u{201c}{}\u{201d}, so that can't be done now.", current.label())))?;
    let note = req.note.as_deref().map(str::trim).filter(|n| !n.is_empty()).map(|n| n.chars().take(500).collect::<String>());
    if req.action == ReviewAction::AskForDetails && note.is_none() {
        return Err(AppError::bad("Tell the reporter what you need, e.g. \u{201c}Please add a photo of the crack\u{201d}."));
    }
    let mut a = row.into_active_model();
    a.status = Set(enc(&next));
    a.office_note = Set(match req.action {
        ReviewAction::Decline | ReviewAction::AskForDetails => note,
        _ => None,
    });
    a.resolved_at = Set(matches!(next, DecisionStatus::OfficeApproved | DecisionStatus::OfficeDeclined).then(Utc::now));
    let saved = convert::decision(a.update(&state.db).await?);
    state.hub.changed(&[DataScope::Decisions]).await;
    Ok(Json(saved))
}

fn clean_option(o: &Option<String>) -> Option<String> {
    o.as_ref().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// `PUT /api/decisions/{id}/publish` — the Office Admin polishes & publishes.
pub async fn publish(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<PublishDecisionRequest>,
) -> AppResult<Json<DecisionAlert>> {
    user.require(Permission::PublishDecisions)?;
    let row = DecisionAlerts::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("decision"))?;
    let current: DecisionStatus = convert::dec(&row.status);
    if !current.is_open() {
        return Err(AppError::bad("This item has already been dealt with."));
    }
    if req.title.trim().is_empty() || req.issue_description.trim().is_empty() {
        return Err(AppError::bad("Please add a clear title and summary for the owner."));
    }
    let a = clean_option(&req.option_a_label);
    let b = clean_option(&req.option_b_label);
    if a.is_some() != b.is_some() {
        return Err(AppError::bad("Give both Option A and Option B, or leave both empty for a simple update."));
    }
    if req.category != AlertCategory::RoutineUpdate && a.is_none() && req.category == AlertCategory::DesignDecision {
        return Err(AppError::bad("A design choice needs Option A and Option B for the owner to pick from."));
    }
    let mut active = row.into_active_model();
    active.title = Set(req.title.trim().to_string());
    active.issue_description = Set(req.issue_description.trim().to_string());
    active.mitigation_action_taken = Set(req.mitigation_action_taken.trim().to_string());
    active.category = Set(enc(&req.category));
    active.urgency = Set(enc(&req.urgency));
    active.option_a_label = Set(a);
    active.option_b_label = Set(b);
    active.status = Set(enc(&DecisionStatus::PublishedPendingClient));
    active.office_note = Set(None);
    let saved = convert::decision(active.update(&state.db).await?);

    state.hub.to_vip(WsEvent::DecisionPublished(saved.clone())).await;
    state.hub.changed(&[DataScope::Decisions, DataScope::Map]).await;
    Ok(Json(saved))
}

/// `POST /api/decisions/{id}/respond` — the owner chooses or acknowledges.
pub async fn respond(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RespondDecisionRequest>,
) -> AppResult<Json<DecisionAlert>> {
    user.require(Permission::AnswerDecisions)?;
    let row = DecisionAlerts::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("decision"))?;
    let d = convert::decision(row.clone());
    if d.status != DecisionStatus::PublishedPendingClient {
        return Err(AppError::bad("This decision has already been answered."));
    }
    let (status, summary) = match req.response {
        DecisionResponse::ApproveOptionA => {
            let label = d.option_a_label.clone().ok_or_else(|| AppError::bad("This item has no Option A."))?;
            (DecisionStatus::ApprovedOptionA, format!("The owner approved Option A: {label}"))
        }
        DecisionResponse::ApproveOptionB => {
            let label = d.option_b_label.clone().ok_or_else(|| AppError::bad("This item has no Option B."))?;
            (DecisionStatus::ApprovedOptionB, format!("The owner approved Option B: {label}"))
        }
        DecisionResponse::AcknowledgeReceipt => (DecisionStatus::AcknowledgedReceipt, "The owner acknowledged this update.".to_string()),
    };
    let txn = state.db.begin().await?;
    let mut active = row.into_active_model();
    active.status = Set(enc(&status));
    active.resolved_at = Set(Some(Utc::now()));
    let saved = convert::decision(active.update(&txn).await?);
    log_timeline(&txn, d.work_item_id, format!("Decision: {}", d.title), summary, false, DelayCategory::None, 0).await?;
    txn.commit().await?;

    // The reporter hears about it through their own report list.
    state.hub.to_office(WsEvent::DecisionResponded(saved.clone())).await;
    state.hub.changed(&[DataScope::Decisions, DataScope::Map, DataScope::WorkItems]).await;
    Ok(Json(saved))
}
