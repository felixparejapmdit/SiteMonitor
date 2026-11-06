//! Estate agenda, Quiet Day override and Master Scheduler.

use axum::extract::{Path, State};
use axum::Json;
use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, Set, TransactionTrait};
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
    user.require_staff("add schedule items")?;
    if req.title.trim().is_empty() {
        return Err(AppError::bad("Please give this schedule item a title."));
    }
    if req.end_time <= req.start_time {
        return Err(AppError::bad("The end time must be after the start time."));
    }
    if req.event_type == AgendaEventType::VipSpecialOccasion {
        return Err(AppError::bad("Only the owner can book a Quiet Day."));
    }
    let txn = state.db.begin().await?;
    let occasions: Vec<EstateAgendaItem> = EstateAgendaItems::find()
        .all(&txn)
        .await?
        .into_iter()
        .map(convert::agenda)
        .filter(|i| i.event_type == AgendaEventType::VipSpecialOccasion)
        .collect();
    let conflict = req.intrusion_level.is_intrusive()
        && occasions.iter().any(|o| req.start_time < o.end_time && o.start_time < req.end_time);
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
    for asset_id in &req.asset_ids {
        Assets::find_by_id(*asset_id).one(&txn).await?.ok_or_else(|| AppError::not_found("equipment"))?;
        equipment_reservation::ActiveModel {
            id: Set(Uuid::new_v4()),
            asset_id: Set(*asset_id),
            agenda_item_id: Set(row.id),
        }
        .insert(&txn)
        .await?;
    }
    txn.commit().await?;
    state.hub.changed(&[DataScope::Agenda, DataScope::Map]).await;
    Ok(Json(convert::agenda(row)))
}

/// `POST /api/agenda/quiet-day`
pub async fn quiet_day(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<QuietDayRequest>,
) -> AppResult<Json<QuietDayResponse>> {
    user.require_vip("book a Quiet Day")?;
    Ok(Json(agenda::schedule_quiet_day(&state, req).await?))
}

/// `PUT /api/agenda/{id}/reschedule`
pub async fn reschedule(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<RescheduleRequest>,
) -> AppResult<Json<EstateAgendaItem>> {
    user.require_staff("reschedule work")?;
    Ok(Json(agenda::reschedule(&state, id, req.start_time, req.end_time).await?))
}
