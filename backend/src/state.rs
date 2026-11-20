//! Shared application state and the "who is acting" extractor.

use std::collections::HashMap;
use std::path::PathBuf;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait};
use shared::{Permission, User, UserRole};
use uuid::Uuid;

use crate::auth::{self, LoginThrottle};
use crate::convert;
use crate::entities::{spatial_area, work_item};
use crate::error::AppError;
use crate::ws::Hub;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub hub: Hub,
    /// Where uploaded photos and base maps are written.
    pub upload_dir: PathBuf,
    pub throttle: LoginThrottle,
}

/// The signed-in person making the request.
pub struct CurrentUser(pub User);

impl CurrentUser {
    /// Checks the access matrix for one permission.
    pub fn require(&self, p: Permission) -> Result<(), AppError> {
        if self.0.can(p) {
            Ok(())
        } else {
            Err(AppError::forbidden(format!(
                "You don't have access to \u{201c}{}\u{201d}. Ask the Office Admin if you need it.",
                p.label()
            )))
        }
    }

    /// Site Monitors work only in their assigned zones (and the zones inside them).
    pub async fn require_zone(&self, db: &impl ConnectionTrait, area: Uuid) -> Result<(), AppError> {
        if self.0.role != UserRole::SiteMonitor {
            return Ok(());
        }
        let parents: HashMap<Uuid, Option<Uuid>> =
            spatial_area::Entity::find().all(db).await?.into_iter().map(|a| (a.id, a.parent_id)).collect();
        if shared::access::works_in(&self.0, area, |z| parents.get(&z).copied().flatten()) {
            Ok(())
        } else if self.0.assigned_area_ids.is_empty() {
            Err(AppError::forbidden("You don't have any zones yet. Ask the Office Admin to assign your zones in People."))
        } else {
            Err(AppError::forbidden("That's outside your zones. Ask the Office Admin if you need to work there."))
        }
    }

    /// `require_zone` for the zone a project is in.
    pub async fn require_job(&self, db: &impl ConnectionTrait, work_item: Uuid) -> Result<(), AppError> {
        if self.0.role != UserRole::SiteMonitor {
            return Ok(());
        }
        let job = work_item::Entity::find_by_id(work_item).one(db).await?.ok_or_else(|| AppError::not_found("project"))?;
        self.require_zone(db, job.area_id).await
    }
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    /// Uses the user the `auth::guard` middleware already loaded, or (for the
    /// WebSocket and public routes) looks the session up itself.
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        if let Some(u) = parts.extensions.get::<User>() {
            return Ok(CurrentUser(u.clone()));
        }
        let token = auth::token_from(&parts.headers).ok_or_else(|| AppError::Unauthorized("Please sign in to continue.".into()))?;
        let user = auth::user_for_token(&state.db, &token)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Your session has ended. Please sign in again.".into()))?;
        Ok(CurrentUser(convert::user(user)))
    }
}
