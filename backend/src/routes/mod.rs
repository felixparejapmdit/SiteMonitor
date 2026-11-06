//! The complete REST API router.

pub mod agenda;
pub mod assets;
pub mod decisions;
pub mod estate;
pub mod manpower;
pub mod photos;
pub mod work_items;

use axum::extract::{DefaultBodyLimit, Path, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set};
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::convert::{self, enc};
use crate::entities::{notification, prelude::*};
use crate::error::{AppError, AppResult};
use crate::services::scheduler;
use crate::state::{AppState, CurrentUser};

pub fn api() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/users", get(users))
        // Estate map
        .route("/estate/map", get(estate::get_map))
        .route("/estate/areas", post(estate::create_area))
        .route("/estate/areas/{id}", put(estate::update_area))
        .route("/estate/base-maps", post(estate::upload_base_map))
        // Work items & phases
        .route("/work-items", get(work_items::list))
        .route("/work-items/{id}", get(work_items::detail))
        .route("/work-items/{id}/sign-off", post(work_items::sign_off))
        .route("/work-items/{id}/complete-task", post(work_items::complete_task))
        .route("/phases/{id}/complete", post(work_items::complete_phase))
        // Manpower
        .route("/manpower/reallocate", post(manpower::reallocate))
        // Assets
        .route("/assets", get(assets::list))
        .route("/assets/{id}/relocate", post(assets::relocate))
        .route("/assets/{id}/link", post(assets::link))
        .route("/assets/{id}/report-issue", post(assets::report_issue))
        .route("/assets/{id}/mark-repaired", post(assets::mark_repaired))
        // Decisions
        .route("/decisions", get(decisions::list).post(decisions::create))
        .route("/decisions/{id}/publish", put(decisions::publish))
        .route("/decisions/{id}/respond", post(decisions::respond))
        // Photos & call-outs
        .route("/photos", post(photos::upload))
        .route("/photos/updates", get(photos::unacknowledged))
        .route("/callouts/open", get(photos::open_callouts))
        .route("/photos/{id}/acknowledge", post(photos::acknowledge))
        .route("/photos/{id}/callouts", post(photos::create_callout))
        .route("/callouts/{id}/review", put(photos::review_callout))
        .route("/callouts/{id}/resolve", put(photos::resolve_callout))
        // Agenda
        .route("/agenda", get(agenda::list).post(agenda::create))
        .route("/agenda/quiet-day", post(agenda::quiet_day))
        .route("/agenda/{id}/reschedule", put(agenda::reschedule))
        // Notifications & automation
        .route("/notifications", get(notifications))
        .route("/notifications/{id}/dismiss", post(dismiss_notification))
        .route("/admin/jobs/{job}/run", post(run_job))
        .layer(DefaultBodyLimit::max(25 * 1024 * 1024))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true, version: env!("CARGO_PKG_VERSION").into() })
}

/// `GET /api/users` — powers the dev-mode role switcher.
async fn users(State(state): State<AppState>) -> AppResult<Json<Vec<User>>> {
    let mut list: Vec<User> = Users::find().all(&state.db).await?.into_iter().map(convert::user).collect();
    list.sort_by_key(|u| (UserRole::ALL.iter().position(|r| *r == u.role), u.name.clone()));
    Ok(Json(list))
}

/// `GET /api/notifications` — visible, undismissed notices for my role.
async fn notifications(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Vec<Notification>>> {
    let list = Notifications::find()
        .filter(notification::Column::Audience.eq(enc(&user.0.role)))
        .filter(notification::Column::Dismissed.eq(false))
        .filter(notification::Column::VisibleFrom.lte(Utc::now()))
        .order_by_desc(notification::Column::CreatedAt)
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::notification)
        .collect();
    Ok(Json(list))
}

async fn dismiss_notification(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Notification>> {
    let row = Notifications::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("notice"))?;
    if row.audience != enc(&user.0.role) {
        return Err(AppError::forbidden("That notice belongs to someone else."));
    }
    let mut a = row.into_active_model();
    a.dismissed = Set(true);
    let saved = a.update(&state.db).await?;
    state.hub.changed(&[DataScope::Notifications]).await;
    Ok(Json(convert::notification(saved)))
}

/// `POST /api/admin/jobs/{job}/run` — run an automation now (Office Admin).
async fn run_job(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(job): Path<String>,
) -> AppResult<Json<JobRunResult>> {
    user.require_admin("run automations")?;
    let job = match job.as_str() {
        "midnight-roster-reset" => AutomationJob::MidnightRosterReset,
        "deadline-monitor" => AutomationJob::DeadlineMonitor,
        "vip-daily-digest" => AutomationJob::VipDailyDigest,
        _ => return Err(AppError::not_found("automation")),
    };
    Ok(Json(scheduler::run(&state, job).await?))
}
