//! The complete REST API router.

pub mod agenda;
pub mod assets;
pub mod decisions;
pub mod estate;
pub mod manpower;
pub mod photos;
pub mod settings;
pub mod workforce;
pub mod users;
pub mod work_items;

use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::StatusCode;
use axum::middleware;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, QuerySelect, Set, Statement};
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::convert::{self, enc};
use crate::entities::{notification, prelude::*, site_photo, timeline_event_record};
use crate::error::{AppError, AppResult};
use crate::services::scheduler;
use crate::state::{AppState, CurrentUser};

pub fn api(state: AppState) -> Router<AppState> {
    // Reachable without signing in.
    let public = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/auth/options", get(crate::auth::options))
        .route("/auth/login", post(crate::auth::login))
        .route("/auth/logout", post(crate::auth::logout))
        .route("/auth/me", get(crate::auth::me));
    // Everything else needs a session.
    let private = Router::new()
        .route("/auth/password", post(crate::auth::change_password))
        .route("/users", get(users).post(users::create))
        .route("/users/{id}", put(users::update))
        .route("/users/{id}/reset-password", post(users::reset_password))
        .route("/access", get(users::access_matrix).put(users::save_access))
        .route("/settings", get(settings::get).put(settings::update))
        .route("/settings/sample-data", post(settings::load_sample))
        .route("/settings/clear-data", post(settings::clear_data))
        .route("/workers", get(workforce::list).post(workforce::create))
        .route("/workers/{id}", put(workforce::update))
        .route("/workers/{id}/status", put(workforce::set_status))
        .route("/workers/{id}/photo", get(workforce::photo))
        .route("/workers/{id}/profile", get(workforce::profile).put(workforce::save_profile))
        .route("/workforce/summary", get(workforce::summary))
        .route("/workforce/family-roster", get(workforce::family_roster))
        .route("/workers/{id}/reassign", put(workforce::reassign))
        .route("/workers/{id}/archive", post(workforce::archive))
        .route("/workers/{id}/assignment", put(workforce::set_assignment))
        .route("/workers/{id}/reemploy", post(workforce::reemploy))
        .route("/workers/delete", post(workforce::delete_workers))
        .route("/workforce/start-day", post(workforce::start_day))
        .route("/workforce/project-history", get(workforce::project_history))
        .route("/workforce/dependability", get(workforce::dependability))
        .route("/workforce/groups", get(workforce::groups).put(workforce::link_group))
        .route("/workforce/records/{collection}", get(workforce::records))
        .route("/workforce/records/{collection}/{id}", put(workforce::put_record).delete(workforce::delete_record))
        // A live service5.db is ~90 MB and growing, so this one route takes big uploads.
        .route("/workforce/import", post(workforce::import).layer(DefaultBodyLimit::max(512 * 1024 * 1024)))
        .route("/workforce/export", get(workforce::export))
        .route("/workforce/import/preview", post(workforce::import_preview).layer(DefaultBodyLimit::max(512 * 1024 * 1024)))
        .route("/settings/backup", post(settings::backup_export))
        .route("/settings/restore", post(settings::backup_import).layer(DefaultBodyLimit::max(1024 * 1024 * 1024)))
        .route("/workforce/snapshots", get(workforce::snapshots).post(workforce::take_snapshot))
        .route("/workforce/snapshots/{id}/restore", post(workforce::restore_snapshot))
        // Estate map
        .route("/estate/map", get(estate::get_map))
        .route("/estate/areas", post(estate::create_area))
        .route("/estate/areas/{id}", put(estate::update_area))
        .route("/estate/base-maps", post(estate::upload_base_map))
        .route("/estate/base-maps/{id}", put(estate::update_base_map))
        .route("/estate/base-maps/{id}/activate", post(estate::activate_base_map))
        // Work items & phases
        .route("/work-items", get(work_items::list).post(work_items::create))
        .route("/work-items/{id}", get(work_items::detail).put(work_items::update).delete(work_items::remove))
        .route("/work-items/{id}/cancel", post(work_items::cancel))
        .route("/work-items/{id}/sign-off", post(work_items::sign_off))
        .route("/work-items/{id}/complete-task", post(work_items::complete_task))
        .route("/work-items/{id}/delay", post(work_items::log_delay))
        .route("/phases/{id}/complete", post(work_items::complete_phase))
        // Manpower
        .route("/manpower/reallocate", post(manpower::reallocate))
        // Assets
        .route("/assets", get(assets::list).post(assets::create))
        .route("/assets/{id}", put(assets::update))
        .route("/assets/{id}/relocate", post(assets::relocate))
        .route("/assets/{id}/link", post(assets::link))
        .route("/assets/{id}/report-issue", post(assets::report_issue))
        .route("/assets/{id}/mark-repaired", post(assets::mark_repaired))
        // Decisions
        .route("/decisions", get(decisions::list).post(decisions::create))
        .route("/decisions/{id}", put(decisions::update).delete(decisions::remove))
        .route("/decisions/{id}/publish", put(decisions::publish))
        .route("/decisions/{id}/review", post(decisions::review))
        .route("/decisions/{id}/respond", post(decisions::respond))
        // Photos & call-outs
        .route("/photos", post(photos::upload))
        .route("/photos/{id}", axum::routing::delete(photos::remove))
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
        .route("/agenda/{id}", put(agenda::update).delete(agenda::remove))
        // Notifications & automation
        .route("/activity", get(activity))
        .route("/notifications", get(notifications))
        .route("/notifications/{id}/dismiss", post(dismiss_notification))
        .route("/admin/jobs/{job}/run", post(run_job))
        .route_layer(middleware::from_fn_with_state(state, crate::auth::guard));
    // Full-resolution map pictures arrive base64-encoded (60 MB file ≈ 80 MB body).
    public.merge(private).layer(DefaultBodyLimit::max(90 * 1024 * 1024))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true, version: env!("CARGO_PKG_VERSION").into() })
}

/// Readiness includes the database, without exposing connection errors publicly.
async fn ready(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let query = state.db.query_one(Statement::from_string(DatabaseBackend::Postgres, "SELECT 1"));
    let ok = matches!(tokio::time::timeout(std::time::Duration::from_secs(3), query).await, Ok(Ok(Some(_))));
    let status = if ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (status, Json(HealthResponse { ok, version: env!("CARGO_PKG_VERSION").into() }))
}

/// `GET /api/users` — everyone on the estate team (for pickers and contact cards).
async fn users(State(state): State<AppState>) -> AppResult<Json<Vec<User>>> {
    let mut list: Vec<User> = Users::find().all(&state.db).await?.into_iter().map(convert::user).collect();
    list.sort_by_key(|u| (UserRole::ALL.iter().position(|r| *r == u.role), u.name.clone()));
    Ok(Json(list))
}

/// `GET /api/activity` — the latest happenings across the estate (project
/// history and site photos), newest first, for the live feed.
async fn activity(State(state): State<AppState>, _user: CurrentUser) -> AppResult<Json<Vec<ActivityItem>>> {
    let now = Utc::now();
    let events = TimelineEventRecords::find()
        .filter(timeline_event_record::Column::Timestamp.lte(now))
        .order_by_desc(timeline_event_record::Column::Timestamp)
        .limit(30)
        .all(&state.db)
        .await?;
    let photos = SitePhotos::find()
        .filter(site_photo::Column::UploadedAt.lte(now))
        // Field-report photos are private to the office and the reporter.
        .filter(site_photo::Column::Caption.not_like("Field report:%"))
        .order_by_desc(site_photo::Column::UploadedAt)
        .limit(30)
        .all(&state.db)
        .await?;
    let mut out: Vec<ActivityItem> = events
        .into_iter()
        .map(convert::timeline)
        .map(|t| ActivityItem {
            id: t.id,
            kind: if t.days_delayed > 0 {
                ActivityKind::Delay
            } else if t.is_milestone {
                ActivityKind::Milestone
            } else {
                ActivityKind::Update
            },
            title: t.title,
            detail: t.description,
            work_item_id: t.work_item_id,
            image_url: None,
            at: t.timestamp,
        })
        .collect();
    out.extend(photos.into_iter().map(convert::photo).map(|p| ActivityItem {
        id: p.id,
        kind: ActivityKind::Photo,
        title: if p.caption.trim().is_empty() { "New photo from the site".into() } else { format!("New photo: {}", p.caption.trim()) },
        detail: String::new(),
        work_item_id: p.work_item_id,
        image_url: Some(p.image_url),
        at: p.uploaded_at,
    }));
    out.sort_by_key(|a| std::cmp::Reverse(a.at));
    out.truncate(30);
    Ok(Json(out))
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
    user.require(Permission::ManageSettings)?;
    let job = match job.as_str() {
        "midnight-roster-reset" => AutomationJob::MidnightRosterReset,
        "deadline-monitor" => AutomationJob::DeadlineMonitor,
        "vip-daily-digest" => AutomationJob::VipDailyDigest,
        _ => return Err(AppError::not_found("automation")),
    };
    Ok(Json(scheduler::run(&state, job).await?))
}
