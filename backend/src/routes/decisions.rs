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
use crate::entities::{decision_alert, prelude::*, site_photo};
use crate::error::{AppError, AppResult};
use crate::services::{log_timeline, media};
use crate::state::{AppState, CurrentUser};

/// `GET /api/decisions` — the owner never sees unpublished drafts.
pub async fn list(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Vec<DecisionAlert>>> {
    let mut all: Vec<DecisionAlert> = DecisionAlerts::find()
        .order_by_desc(decision_alert::Column::CreatedAt)
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::decision)
        .collect();
    if user.0.role == UserRole::VipClient {
        all.retain(|d| d.status != DecisionStatus::DraftInAdminQueue);
    }
    Ok(Json(all))
}

/// `POST /api/decisions` — a Site Monitor (or Admin) files a field draft.
pub async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<CreateDecisionRequest>,
) -> AppResult<Json<DecisionAlert>> {
    user.require_staff("raise issues")?;
    if req.title.trim().is_empty() || req.issue_description.trim().is_empty() {
        return Err(AppError::bad("Please add a short title and describe the issue."));
    }
    let item = WorkItems::find_by_id(req.work_item_id).one(&state.db).await?.ok_or_else(|| AppError::not_found("project"))?;
    let photo_url = match req.photo_data_url.as_deref().filter(|s| !s.is_empty()) {
        Some(data) => Some(media::save_data_url(&state.upload_dir, "photos", data).await?),
        None => None,
    };
    let txn = state.db.begin().await?;
    if let Some(url) = photo_url {
        site_photo::ActiveModel {
            id: Set(Uuid::new_v4()),
            work_item_id: Set(item.id),
            area_id: Set(item.area_id),
            image_url: Set(url),
            caption: Set(format!("Field report: {}", req.title.trim())),
            uploaded_by: Set(user.0.id),
            uploaded_at: Set(Utc::now()),
            client_acknowledged: Set(false),
        }
        .insert(&txn)
        .await?;
    }
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
    }
    .insert(&txn)
    .await?;
    txn.commit().await?;
    state.hub.changed(&[DataScope::Decisions, DataScope::Photos]).await;
    Ok(Json(convert::decision(row)))
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
    user.require_admin("publish to the owner")?;
    let row = DecisionAlerts::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("decision"))?;
    let current: DecisionStatus = convert::dec(&row.status);
    if current != DecisionStatus::DraftInAdminQueue {
        return Err(AppError::bad("This item has already been sent to the owner."));
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
    user.require_vip("answer decisions")?;
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

    state.hub.to_staff(WsEvent::DecisionResponded(saved.clone())).await;
    state.hub.changed(&[DataScope::Decisions, DataScope::Map, DataScope::WorkItems]).await;
    Ok(Json(saved))
}
