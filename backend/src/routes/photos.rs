//! 16:9 site photos and the bidirectional tap-to-pin call-out loop.

use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, Set, TransactionTrait};
use shared::api::*;
use shared::ws::{DataScope, WsEvent};
use shared::*;
use uuid::Uuid;

use crate::convert::{self, enc};
use crate::entities::{photo_call_out, prelude::*, site_photo};
use crate::error::{AppError, AppResult};
use crate::services::{create_notice, log_timeline, media, NewNotice};
use crate::state::{AppState, CurrentUser};

/// `POST /api/photos`
pub async fn upload(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<UploadPhotoRequest>,
) -> AppResult<Json<SitePhoto>> {
    user.require_staff("upload site photos")?;
    let item = WorkItems::find_by_id(req.work_item_id).one(&state.db).await?.ok_or_else(|| AppError::not_found("project"))?;
    let url = media::save_data_url(&state.upload_dir, "photos", &req.image_data_url).await?;
    let txn = state.db.begin().await?;
    let row = site_photo::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_item_id: Set(item.id),
        area_id: Set(item.area_id),
        image_url: Set(url.clone()),
        caption: Set(if req.caption.trim().is_empty() { "Site photo".into() } else { req.caption.trim().to_string() }),
        uploaded_by: Set(user.0.id),
        uploaded_at: Set(Utc::now()),
        client_acknowledged: Set(false),
    }
    .insert(&txn)
    .await?;
    if item.before_photo_url.is_none() {
        let mut a = item.into_active_model();
        a.before_photo_url = Set(Some(url));
        a.update(&txn).await?;
    }
    txn.commit().await?;
    state.hub.changed(&[DataScope::Photos, DataScope::WorkItems]).await;
    Ok(Json(convert::photo(row)))
}

/// `GET /api/photos/updates` — photos the owner hasn't acknowledged yet.
pub async fn unacknowledged(State(state): State<AppState>) -> AppResult<Json<Vec<PhotoUpdate>>> {
    use sea_orm::{ColumnTrait, QueryFilter, QueryOrder};
    let items: std::collections::HashMap<Uuid, String> =
        WorkItems::find().all(&state.db).await?.into_iter().map(|w| (w.id, w.title)).collect();
    let list = SitePhotos::find()
        .filter(site_photo::Column::ClientAcknowledged.eq(false))
        .order_by_desc(site_photo::Column::UploadedAt)
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::photo)
        .map(|p| PhotoUpdate { work_item_title: items.get(&p.work_item_id).cloned().unwrap_or_default(), photo: p })
        .collect();
    Ok(Json(list))
}

/// `GET /api/callouts/open` — every unresolved owner note, oldest first.
pub async fn open_callouts(State(state): State<AppState>) -> AppResult<Json<Vec<OpenCallOut>>> {
    use sea_orm::QueryOrder;
    let items: std::collections::HashMap<Uuid, String> =
        WorkItems::find().all(&state.db).await?.into_iter().map(|w| (w.id, w.title)).collect();
    let photos: std::collections::HashMap<Uuid, SitePhoto> =
        SitePhotos::find().all(&state.db).await?.into_iter().map(convert::photo).map(|p| (p.id, p)).collect();
    let list = PhotoCallOuts::find()
        .order_by_asc(photo_call_out::Column::CreatedAt)
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::callout)
        .filter(|c| c.status != CallOutStatus::Resolved)
        .filter_map(|c| {
            let photo = photos.get(&c.photo_id)?.clone();
            Some(OpenCallOut { work_item_title: items.get(&photo.work_item_id).cloned().unwrap_or_default(), photo, callout: c })
        })
        .collect();
    Ok(Json(list))
}

/// `POST /api/photos/{id}/acknowledge` — the owner clears a photo update.
pub async fn acknowledge(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>) -> AppResult<Json<SitePhoto>> {
    user.require_vip("acknowledge photo updates")?;
    let row = SitePhotos::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("photo"))?;
    let mut a = row.into_active_model();
    a.client_acknowledged = Set(true);
    let saved = a.update(&state.db).await?;
    state.hub.changed(&[DataScope::Photos]).await;
    Ok(Json(convert::photo(saved)))
}

/// `POST /api/photos/{id}/callouts` — the owner drops a pin with a note.
pub async fn create_callout(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateCallOutRequest>,
) -> AppResult<Json<PhotoCallOut>> {
    user.require_vip("add photo notes")?;
    if req.comment.trim().is_empty() {
        return Err(AppError::bad("Please write a short note for the site team."));
    }
    if !(0.0..=100.0).contains(&req.x_percent) || !(0.0..=100.0).contains(&req.y_percent) {
        return Err(AppError::bad("Please tap inside the photo."));
    }
    let photo = SitePhotos::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("photo"))?;
    let item = WorkItems::find_by_id(photo.work_item_id).one(&state.db).await?.map(convert::work_item).ok_or_else(|| AppError::not_found("project"))?;
    let txn = state.db.begin().await?;
    let row = photo_call_out::ActiveModel {
        id: Set(Uuid::new_v4()),
        photo_id: Set(id),
        x_percent: Set(req.x_percent),
        y_percent: Set(req.y_percent),
        client_comment: Set(req.comment.trim().to_string()),
        staff_reply: Set(None),
        status: Set(enc(&CallOutStatus::Open)),
        created_at: Set(Utc::now()),
        resolved_at: Set(None),
    }
    .insert(&txn)
    .await?;
    let callout = convert::callout(row);
    let on_hold = item.status == WorkStatus::PendingSignOff;
    if on_hold {
        log_timeline(
            &txn,
            item.id,
            "Owner added a final note — held at 99%",
            format!("\"{}\" The project stays at 99% until the site team resolves this.", callout.client_comment),
            false,
            DelayCategory::None,
            0,
        )
        .await?;
    }
    for role in [UserRole::OfficeAdmin, UserRole::SiteMonitor] {
        create_notice(
            &txn,
            NewNotice {
                audience: role,
                kind: NotificationKind::General,
                work_item_id: Some(item.id),
                title: if on_hold {
                    format!("Final note on {} — project held at 99%", item.title)
                } else {
                    format!("New photo note on {}", item.title)
                },
                body: format!("\"{}\"", callout.client_comment),
                lines: vec![],
                visible_from: Utc::now(),
                dedupe_key: None,
            },
        )
        .await?;
    }
    txn.commit().await?;
    state.hub.to_staff(WsEvent::CallOutCreated { callout: callout.clone(), work_item_id: item.id }).await;
    state.hub.changed(&[DataScope::Photos, DataScope::Map, DataScope::Notifications]).await;
    Ok(Json(callout))
}

/// `PUT /api/callouts/{id}/review` — staff say "we're looking at it".
pub async fn review_callout(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>) -> AppResult<Json<PhotoCallOut>> {
    user.require_staff("update photo notes")?;
    let row = PhotoCallOuts::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("photo note"))?;
    let current: CallOutStatus = convert::dec(&row.status);
    if current == CallOutStatus::Resolved {
        return Ok(Json(convert::callout(row)));
    }
    let mut a = row.into_active_model();
    a.status = Set(enc(&CallOutStatus::InReview));
    let saved = convert::callout(a.update(&state.db).await?);
    state.hub.changed(&[DataScope::Photos]).await;
    Ok(Json(saved))
}

/// `PUT /api/callouts/{id}/resolve` — staff reply and mark it resolved.
pub async fn resolve_callout(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ResolveCallOutRequest>,
) -> AppResult<Json<PhotoCallOut>> {
    user.require_staff("resolve photo notes")?;
    if req.reply.trim().is_empty() {
        return Err(AppError::bad("Please tell the owner what was done."));
    }
    let row = PhotoCallOuts::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("photo note"))?;
    let mut a = row.into_active_model();
    a.status = Set(enc(&CallOutStatus::Resolved));
    a.staff_reply = Set(Some(req.reply.trim().to_string()));
    a.resolved_at = Set(Some(Utc::now()));
    let saved = convert::callout(a.update(&state.db).await?);
    state.hub.to_vip(WsEvent::CallOutResolved(saved.clone())).await;
    state.hub.changed(&[DataScope::Photos, DataScope::Map]).await;
    Ok(Json(saved))
}
