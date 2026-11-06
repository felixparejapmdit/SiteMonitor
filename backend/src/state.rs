//! Shared application state and the "who is acting" extractor.

use std::path::PathBuf;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use sea_orm::{DatabaseConnection, EntityTrait};
use shared::api::USER_HEADER;
use shared::{User, UserRole};
use uuid::Uuid;

use crate::convert;
use crate::entities::prelude::Users;
use crate::error::AppError;
use crate::ws::Hub;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub hub: Hub,
    /// Where uploaded photos and base maps are written.
    pub upload_dir: PathBuf,
}

/// The user making the request (sent by the frontend's role switcher).
pub struct CurrentUser(pub User);

impl CurrentUser {
    pub fn require(&self, roles: &[UserRole], action: &str) -> Result<(), AppError> {
        if roles.contains(&self.0.role) {
            Ok(())
        } else {
            Err(AppError::forbidden(format!(
                "As {} you can't {action}. Switch role using the bar at the top.",
                self.0.role.label()
            )))
        }
    }
    pub fn require_staff(&self, action: &str) -> Result<(), AppError> {
        self.require(&[UserRole::OfficeAdmin, UserRole::SiteMonitor], action)
    }
    pub fn require_admin(&self, action: &str) -> Result<(), AppError> {
        self.require(&[UserRole::OfficeAdmin], action)
    }
    pub fn require_vip(&self, action: &str) -> Result<(), AppError> {
        self.require(&[UserRole::VipClient], action)
    }
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let raw = parts
            .headers
            .get(USER_HEADER)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Please choose who you are using the bar at the top.".into()))?;
        let id = Uuid::parse_str(raw).map_err(|_| AppError::Unauthorized("That user is not recognised.".into()))?;
        let user = Users::find_by_id(id)
            .one(&state.db)
            .await?
            .ok_or_else(|| AppError::Unauthorized("That user is not recognised.".into()))?;
        Ok(CurrentUser(convert::user(user)))
    }
}
