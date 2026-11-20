//! Estate agenda, Quiet Day override and Master Scheduler.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, IntoActiveModel, ModelTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::convert::{self, enc};
use crate::entities::{equipment_reservation, estate_agenda_item, prelude::*};
use crate::error::{AppError, AppResult};
use crate::services::agenda;
use crate::state::{AppState, CurrentUser};

/// `GET /api/agenda`
pub async fn list(State(state): State<AppState>) -> AppResult<Json<AgendaResponse>> {
    let items: Vec<EstateAgendaItem> = EstateAgendaItems::find()
        .order_by_asc(estate_agenda_item::Column::StartTime)
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::agenda)
        .collect();
    let reservations: Vec<EquipmentReservation> =
        EquipmentReservations::find().all(&state.db).await?.into_iter().map(convert::reservation).collect();
    let assets: Vec<Asset> = Assets::find().all(&state.db).await?.into_iter().map(convert::asset).collect();
    let equipment_conflicts = agenda::equipment_conflicts(&items, &reservations, &assets);
    Ok(Json(AgendaResponse { items, reservations, equipment_conflicts }))
}

/// `POST /api/agenda` — staff schedule work (optionally booking machinery).
pub async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<CreateAgendaItemRequest>,
) -> AppResult<Json<EstateAgendaItem>> {
    user.require(Permission::ManageSchedule)?;
    check(&req)?;
    if let Some(area) = req.area_id {
        user.require_zone(&state.db, area).await?;
    }
    let txn = state.db.begin().await?;
    let conflict = clashes_with_quiet_day(&txn, &req).await?;
    let row = estate_agenda_item::ActiveModel {
        id: Set(Uuid::new_v4()),
        area_id: Set(req.area_id),
        work_item_id: Set(req.work_item_id),
        title: Set(req.title.trim().to_string()),
        description: Set(req.description.trim().to_string()),
        event_type: Set(enc(&req.event_type)),
        intrusion_level: Set(enc(&req.intrusion_level)),
        start_time: Set(req.start_time),
        end_time: Set(req.end_time),
        is_flagged_conflict: Set(conflict),
    }
    .insert(&txn)
    .await?;
    book(&txn, row.id, &req.asset_ids).await?;
    txn.commit().await?;
    state.hub.changed(&[DataScope::Agenda, DataScope::Map]).await;
    Ok(Json(convert::agenda(row)))
}

/// `PUT /api/agenda/{id}` — change any detail of a staff-planned item.
pub async fn update(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateAgendaItemRequest>,
) -> AppResult<Json<EstateAgendaItem>> {
    user.require(Permission::ManageSchedule)?;
    check(&req)?;
    let row = editable(&state, &user, id).await?;
    if let Some(area) = req.area_id {
        user.require_zone(&state.db, area).await?;
    }
    let txn = state.db.begin().await?;
    let conflict = clashes_with_quiet_day(&txn, &req).await?;
    let mut a = row.into_active_model();
    a.area_id = Set(req.area_id);
    a.work_item_id = Set(req.work_item_id);
    a.title = Set(req.title.trim().to_string());
    a.description = Set(req.description.trim().to_string());
    a.event_type = Set(enc(&req.event_type));
    a.intrusion_level = Set(enc(&req.intrusion_level));
    a.start_time = Set(req.start_time);
    a.end_time = Set(req.end_time);
    a.is_flagged_conflict = Set(conflict);
    let saved = a.update(&txn).await?;
    EquipmentReservations::delete_many().filter(equipment_reservation::Column::AgendaItemId.eq(id)).exec(&txn).await?;
    book(&txn, id, &req.asset_ids).await?;
    txn.commit().await?;
    state.hub.changed(&[DataScope::Agenda, DataScope::Map]).await;
    Ok(Json(convert::agenda(saved)))
}

/// `DELETE /api/agenda/{id}` — removes the item and frees its machinery bookings.
pub async fn remove(State(state): State<AppState>, user: CurrentUser, Path(id): Path<Uuid>) -> AppResult<StatusCode> {
    user.require(Permission::ManageSchedule)?;
    let row = editable(&state, &user, id).await?;
    row.delete(&state.db).await?;
    state.hub.changed(&[DataScope::Agenda, DataScope::Map]).await;
    Ok(StatusCode::NO_CONTENT)
}

fn check(req: &CreateAgendaItemRequest) -> Result<(), AppError> {
    if req.title.trim().is_empty() {
        return Err(AppError::bad("Please give this schedule item a title."));
    }
    if req.end_time <= req.start_time {
        return Err(AppError::bad("The end time must be after the start time."));
    }
    if req.event_type == AgendaEventType::VipSpecialOccasion {
        return Err(AppError::bad("Only the owner can book a Quiet Day."));
    }
    Ok(())
}

/// Owner's Quiet Days and project handover dates aren't edited here.
async fn editable(state: &AppState, user: &CurrentUser, id: Uuid) -> AppResult<estate_agenda_item::Model> {
    let row = EstateAgendaItems::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("schedule item"))?;
    match convert::agenda(row.clone()).event_type {
        AgendaEventType::VipSpecialOccasion => return Err(AppError::bad("This is the owner's Quiet Day. Only the owner can change it.")),
        AgendaEventType::TargetDeadline => {
            return Err(AppError::bad("This is a project's handover date. Change the project's target date instead."))
        }
        _ => {}
    }
    if let Some(area) = row.area_id {
        user.require_zone(&state.db, area).await?;
    }
    Ok(row)
}

async fn clashes_with_quiet_day(txn: &DatabaseTransaction, req: &CreateAgendaItemRequest) -> AppResult<bool> {
    if !req.intrusion_level.is_intrusive() {
        return Ok(false);
    }
    Ok(EstateAgendaItems::find()
        .all(txn)
        .await?
        .into_iter()
        .map(convert::agenda)
        .any(|o| o.event_type == AgendaEventType::VipSpecialOccasion && req.start_time < o.end_time && o.start_time < req.end_time))
}

async fn book(txn: &DatabaseTransaction, item: Uuid, asset_ids: &[Uuid]) -> AppResult<()> {
    for asset_id in asset_ids {
        Assets::find_by_id(*asset_id).one(txn).await?.ok_or_else(|| AppError::not_found("equipment"))?;
        equipment_reservation::ActiveModel { id: Set(Uuid::new_v4()), asset_id: Set(*asset_id), agenda_item_id: Set(item) }.insert(txn).await?;
    }
    Ok(())
}

/// `POST /api/agenda/quiet-day`
pub async fn quiet_day(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<QuietDayRequest>,
) -> AppResult<Json<QuietDayResponse>> {
    user.require(Permission::BookQuietDays)?;
    Ok(Json(agenda::schedule_quiet_day(&state, req).await?))
}

/// `PUT /api/agenda/{id}/reschedule`
pub async fn reschedule(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RescheduleRequest>,
) -> AppResult<Json<EstateAgendaItem>> {
    user.require(Permission::ManageSchedule)?;
    let item = EstateAgendaItems::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("schedule item"))?;
    if let Some(area) = item.area_id {
        user.require_zone(&state.db, area).await?;
    }
    Ok(Json(agenda::reschedule(&state, id, req.start_time, req.end_time).await?))
}
