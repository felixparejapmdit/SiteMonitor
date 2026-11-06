//! One error type for every handler, rendered as a friendly JSON message.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use shared::api::ApiError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("database error: {0}")]
    Db(#[from] sea_orm::DbErr),
    #[error("{0}")]
    Internal(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn not_found(what: &str) -> Self {
        AppError::NotFound(format!("We couldn't find that {what}. It may have been removed."))
    }
    pub fn bad(msg: impl Into<String>) -> Self {
        AppError::BadRequest(msg.into())
    }
    pub fn forbidden(msg: impl Into<String>) -> Self {
        AppError::Forbidden(msg.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.clone()),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            AppError::Db(e) => {
                tracing::error!(error = %e, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong saving or loading data. Please try again.".into())
            }
            AppError::Internal(e) => {
                tracing::error!(error = %e, "internal error");
                (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong on our side. Please try again.".into())
            }
        };
        (status, Json(ApiError { message })).into_response()
    }
}
