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

/// Sub-zones must sit inside their parent, and a reshaped zone must still hold
/// all of its live sub-zones.
fn check_nesting(areas: &[SpatialArea], id: Option<Uuid>, parent: Option<Uuid>, points: &[Point2D]) -> AppResult<()> {
    if let Some(pid) = parent {
        let p = areas.iter().find(|a| a.id == pid).ok_or_else(|| AppError::not_found("parent zone"))?;
        let out = shared::geometry::corners_outside(points, &p.polygon_points).len();
        if out > 0 {
            return Err(AppError::bad(format!(
                "{out} corner{} of this zone {} outside “{}”. A sub-zone has to sit inside its parent zone's border.",
                if out == 1 { "" } else { "s" },
                if out == 1 { "falls" } else { "fall" },
                p.name
            )));
        }
    }
    if let Some(id) = id {
        let child = areas
            .iter()
            .filter(|a| a.parent_id == Some(id) && !a.is_archived)
            .find(|a| !shared::geometry::corners_outside(&a.polygon_points, points).is_empty());
        if let Some(c) = child {
            return Err(AppError::bad(format!(
                "This shape would leave “{}” sticking out. Shrink or move that sub-zone first.",
                c.name
            )));
        }
    }
    Ok(())
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
    user.require(Permission::ManageMap)?;
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
    check_nesting(&all_areas(&state.db).await?, None, req.parent_id, &points)?;
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
    user.require(Permission::ManageMap)?;
    let row = SpatialAreas::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("zone"))?;
    if req.polygon_points.is_some() || req.parent_id.is_some() {
        let current = convert::area(row.clone());
        let points = match &req.polygon_points {
            Some(p) => clean_polygon(p)?,
            None => current.polygon_points.clone(),
        };
        check_nesting(&all_areas(&state.db).await?, Some(id), req.parent_id.unwrap_or(current.parent_id), &points)?;
    }
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

/// Cuts an uploaded map picture (`/uploads/maps/<file>`) into zoom tiles.
async fn tile_upload(upload_dir: &std::path::Path, url: &str, id: Uuid) -> AppResult<MapTiles> {
    let src = upload_dir.join(url.trim_start_matches("/uploads/"));
    let out = upload_dir.join("maps").join("tiles").join(id.to_string());
    let base = format!("/uploads/maps/tiles/{id}");
    tokio::task::spawn_blocking(move || media::make_tiles(&src, &out, &base))
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
        .map_err(|e| {
            let msg = e.to_string();
            if msg.starts_with("That picture") {
                AppError::bad(msg)
            } else {
                tracing::warn!(error = %msg, "could not tile base map");
                AppError::bad("We couldn't read that picture. Please save it as JPEG or PNG and try again.")
            }
        })
}

/// Start-up: prepares zoom tiles for uploaded maps that don't have them yet.
pub async fn backfill_tiles(state: AppState) {
    let Ok(rows) = BaseMapVersions::find().all(&state.db).await else { return };
    for row in rows.into_iter().filter(|r| r.tiles.is_none() && r.orthographic_image_url.starts_with("/uploads/")) {
        match tile_upload(&state.upload_dir, &row.orthographic_image_url.clone(), row.id).await {
            Ok(t) => {
                let mut a = row.into_active_model();
                a.tiles = Set(Some(to_json(&t)));
                if a.update(&state.db).await.is_ok() {
                    tracing::info!("prepared zoom tiles for a base map ({} × {})", t.width, t.height);
                    state.hub.changed(&[DataScope::Map]).await;
                }
            }
            Err(e) => tracing::warn!(error = %e, "skipped zoom tiles for {}", row.orthographic_image_url),
        }
    }
}

/// `PUT /api/estate/base-maps/{id}` — give an existing map a new picture
/// (and/or name). Zones stay where they are; the old picture and tiles are removed.
pub async fn update_base_map(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UploadBaseMapRequest>,
) -> AppResult<Json<BaseMapVersion>> {
    user.require(Permission::ManageMap)?;
    let row = BaseMapVersions::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("map"))?;
    let name = req.version_name.trim();
    if name.is_empty() {
        return Err(AppError::bad("Please give this map a name."));
    }
    let old = convert::base_map(row.clone());
    let mut a = row.into_active_model();
    a.version_name = Set(name.to_string());
    let replaced = !req.image_data_url.is_empty();
    if replaced {
        let url = media::save_data_url(&state.upload_dir, "maps", &req.image_data_url).await?;
        // A fresh tile folder: browsers keep tiles cached forever, so old URLs must not be reused.
        let tiles = tile_upload(&state.upload_dir, &url, Uuid::new_v4()).await?;
        a.orthographic_image_url = Set(url);
        a.tiles = Set(Some(to_json(&tiles)));
    }
    let saved = a.update(&state.db).await?;
    if replaced {
        // Best effort: leftover files only waste disk space. Built-in drawings (/seed/…) are never touched.
        if let Some(file) = old.orthographic_image_url.strip_prefix("/uploads/") {
            let _ = tokio::fs::remove_file(state.upload_dir.join(file)).await;
        }
        if let Some(dir) = old.tiles.as_ref().and_then(|t| t.base_url.strip_prefix("/uploads/").map(str::to_string)) {
            let _ = tokio::fs::remove_dir_all(state.upload_dir.join(dir)).await;
        }
    }
    state.hub.changed(&[DataScope::Map]).await;
    Ok(Json(convert::base_map(saved)))
}

/// `POST /api/estate/base-maps` — upload a new render, archiving the old one.
pub async fn upload_base_map(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(req): Json<UploadBaseMapRequest>,
) -> AppResult<Json<BaseMapVersion>> {
    user.require(Permission::ManageMap)?;
    let name = req.version_name.trim();
    if name.is_empty() {
        return Err(AppError::bad("Please name this map version, e.g. \"2026 Q4 D5 Render\"."));
    }
    let url = media::save_data_url(&state.upload_dir, "maps", &req.image_data_url).await?;
    let id = Uuid::new_v4();
    let tiles = tile_upload(&state.upload_dir, &url, id).await?;
    let txn = state.db.begin().await?;
    let row = base_map_version::ActiveModel {
        id: Set(id),
        version_name: Set(name.to_string()),
        orthographic_image_url: Set(url),
        is_active: Set(false),
        created_at: Set(Utc::now()),
        tiles: Set(Some(to_json(&tiles))),
    }
    .insert(&txn)
    .await?;
    let row = make_active(&txn, row).await?;
    txn.commit().await?;
    state.hub.changed(&[DataScope::Map]).await;
    Ok(Json(convert::base_map(row)))
}

/// Archives every other version, activates `row` and points all zones at it.
async fn make_active(txn: &sea_orm::DatabaseTransaction, row: base_map_version::Model) -> AppResult<base_map_version::Model> {
    BaseMapVersions::update_many()
        .col_expr(base_map_version::Column::IsActive, sea_orm::sea_query::Expr::value(false))
        .filter(base_map_version::Column::Id.ne(row.id))
        .exec(txn)
        .await?;
    // Polygons use percentage coordinates, so they carry over to any render.
    SpatialAreas::update_many()
        .col_expr(spatial_area::Column::BaseMapId, sea_orm::sea_query::Expr::value(row.id))
        .exec(txn)
        .await?;
    let mut a = row.into_active_model();
    a.is_active = Set(true);
    Ok(a.update(txn).await?)
}

/// `POST /api/estate/base-maps/{id}/activate` — switch back (or forward) to another map version.
pub async fn activate_base_map(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<BaseMapVersion>> {
    user.require(Permission::ManageMap)?;
    let row = BaseMapVersions::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("map version"))?;
    let txn = state.db.begin().await?;
    let row = make_active(&txn, row).await?;
    txn.commit().await?;
    state.hub.changed(&[DataScope::Map]).await;
    Ok(Json(convert::base_map(row)))
}
