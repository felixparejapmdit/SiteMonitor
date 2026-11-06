//! Estate map: base map versions and the infinitely nested polygon tree.

use axum::extract::{Path, State};
use axum::Json;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set, TransactionTrait};
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::convert::{self, enc, to_json};
use crate::entities::{base_map_version, prelude::*, spatial_area};
use crate::error::{AppError, AppResult};
use crate::services::{all_areas, media, status::EstateSnapshot, subtree_ids};
use crate::state::{AppState, CurrentUser};

fn clean_polygon(points: &[Point2D]) -> AppResult<Vec<Point2D>> {
    if points.len() < 3 {
        return Err(AppError::bad("A zone needs at least 3 corner points. Click on the map to add more."));
    }
    Ok(points
        .iter()
        .map(|p| Point2D { x: p.x.clamp(0.0, 100.0), y: p.y.clamp(0.0, 100.0) })
        .collect())
}

/// `GET /api/estate/map`
pub async fn get_map(State(state): State<AppState>) -> AppResult<Json<EstateMapResponse>> {
    let history: Vec<BaseMapVersion> = BaseMapVersions::find()
        .order_by_desc(base_map_version::Column::CreatedAt)
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::base_map)
        .collect();
    let base_map = history
        .iter()
        .find(|b| b.is_active)
        .cloned()
        .ok_or_else(|| AppError::not_found("estate base map"))?;
    let snapshot = EstateSnapshot::load(&state.db).await?;
    Ok(Json(EstateMapResponse { base_map, base_map_history: history, areas: snapshot.tree() }))
}

/// `POST /api/estate/areas`
pub async fn create_area(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<CreateAreaRequest>,
) -> AppResult<Json<SpatialArea>> {
    user.require_admin("draw new zones")?;
    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::bad("Please give the zone a name."));
    }
    let points = clean_polygon(&req.polygon_points)?;
    let active = BaseMapVersions::find()
        .filter(base_map_version::Column::IsActive.eq(true))
        .one(&state.db)
        .await?
        .ok_or_else(|| AppError::not_found("estate base map"))?;
    if let Some(parent) = req.parent_id {
        SpatialAreas::find_by_id(parent).one(&state.db).await?.ok_or_else(|| AppError::not_found("parent zone"))?;
    }
    let row = spatial_area::ActiveModel {
        id: Set(Uuid::new_v4()),
        base_map_id: Set(active.id),
        parent_id: Set(req.parent_id),
        name: Set(name.to_string()),
        zone_class: Set(enc(&req.zone_class)),
        polygon_points: Set(to_json(&points)),
        is_archived: Set(false),
    }
    .insert(&state.db)
    .await?;
    state.hub.changed(&[DataScope::Map]).await;
    Ok(Json(convert::area(row)))
}

/// `PUT /api/estate/areas/{id}` — rename, reshape, re-nest or archive.
pub async fn update_area(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateAreaRequest>,
) -> AppResult<Json<SpatialArea>> {
    user.require_admin("edit zones")?;
    let row = SpatialAreas::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("zone"))?;
    let mut active = row.into_active_model();
    if let Some(name) = req.name {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::bad("Please give the zone a name."));
        }
        active.name = Set(name);
    }
    if let Some(class) = req.zone_class {
        active.zone_class = Set(enc(&class));
    }
    if let Some(points) = req.polygon_points {
        active.polygon_points = Set(to_json(&clean_polygon(&points)?));
    }
    if let Some(parent) = req.parent_id {
        if let Some(p) = parent {
            let areas = all_areas(&state.db).await?;
            if subtree_ids(&areas, id).contains(&p) {
                return Err(AppError::bad("A zone can't be placed inside itself or one of its own sub-zones."));
            }
        }
        active.parent_id = Set(parent);
    }
    if let Some(archived) = req.is_archived {
        active.is_archived = Set(archived);
    }
    let saved = active.update(&state.db).await?;
    state.hub.changed(&[DataScope::Map, DataScope::WorkItems]).await;
    Ok(Json(convert::area(saved)))
}

/// `POST /api/estate/base-maps` — upload a new render, archiving the old one.
pub async fn upload_base_map(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<UploadBaseMapRequest>,
) -> AppResult<Json<BaseMapVersion>> {
    user.require_admin("upload base maps")?;
    let name = req.version_name.trim();
    if name.is_empty() {
        return Err(AppError::bad("Please name this map version, e.g. \"2026 Q4 D5 Render\"."));
    }
    let url = media::save_data_url(&state.upload_dir, "maps", &req.image_data_url).await?;
    let txn = state.db.begin().await?;
    for old in BaseMapVersions::find().filter(base_map_version::Column::IsActive.eq(true)).all(&txn).await? {
        let mut a = old.into_active_model();
        a.is_active = Set(false);
        a.update(&txn).await?;
    }
    let row = base_map_version::ActiveModel {
        id: Set(Uuid::new_v4()),
        version_name: Set(name.to_string()),
        orthographic_image_url: Set(url),
        is_active: Set(true),
        created_at: Set(Utc::now()),
    }
    .insert(&txn)
    .await?;
    // Polygons use percentage coordinates, so they carry over to the new render.
    SpatialAreas::update_many()
        .col_expr(spatial_area::Column::BaseMapId, sea_orm::sea_query::Expr::value(row.id))
        .exec(&txn)
        .await?;
    txn.commit().await?;
    state.hub.changed(&[DataScope::Map]).await;
    Ok(Json(convert::base_map(row)))
}
