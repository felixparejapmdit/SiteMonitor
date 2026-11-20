//! Estate settings: name, automation times and the editable pick lists.

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::NaiveTime;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use shared::ws::DataScope;
use shared::*;

use crate::entities::estate_setting;
use crate::error::{AppError, AppResult};
use crate::services::load_settings;
use crate::state::{AppState, CurrentUser};

/// `GET /api/settings`
pub async fn get(State(state): State<AppState>) -> AppResult<Json<EstateSettings>> {
    Ok(Json(load_settings(&state.db).await?))
}

/// Trims and checks the form; returns the cleaned settings.
pub fn clean(mut s: EstateSettings) -> Result<EstateSettings, AppError> {
    s.estate_name = s.estate_name.trim().to_string();
    if s.estate_name.is_empty() || s.estate_name.chars().count() > 80 {
        return Err(AppError::bad("Please enter the estate's name (up to 80 characters)."));
    }
    for (label, t) in [
        ("crew reset", &mut s.roster_reset_time),
        ("crew reminder", &mut s.roster_prompt_time),
        ("deadline check", &mut s.deadline_check_time),
        ("owner summary", &mut s.digest_time),
    ] {
        let parsed = NaiveTime::parse_from_str(t.trim(), "%H:%M")
            .map_err(|_| AppError::bad(format!("The {label} time must look like 07:00.")))?;
        *t = parsed.format("%H:%M").to_string();
    }
    let mut reasons: Vec<String> = Vec::new();
    for r in s.crew_move_reasons.iter().map(|r| r.trim()).filter(|r| !r.is_empty()) {
        if r.chars().count() > 80 {
            return Err(AppError::bad("Each crew move reason must be 80 characters or fewer."));
        }
        if !reasons.iter().any(|x| x.eq_ignore_ascii_case(r)) {
            reasons.push(r.to_string());
        }
    }
    if reasons.len() > 12 {
        return Err(AppError::bad("Keep the crew move reasons to 12 or fewer."));
    }
    s.crew_move_reasons = reasons;
    s.category_labels = s.category_labels.into_iter().map(|(k, v)| (k, v.trim().to_string())).filter(|(_, v)| !v.is_empty()).collect();
    s.event_type_labels = s.event_type_labels.into_iter().map(|(k, v)| (k, v.trim().to_string())).filter(|(_, v)| !v.is_empty()).collect();
    if s.category_labels.values().chain(s.event_type_labels.values()).any(|v| v.chars().count() > 40) {
        return Err(AppError::bad("Custom names must be 40 characters or fewer."));
    }
    Ok(s)
}

/// `PUT /api/settings` (Office Admin)
pub async fn update(State(state): State<AppState>, me: CurrentUser, Json(req): Json<EstateSettings>) -> AppResult<Json<EstateSettings>> {
    me.require(Permission::ManageSettings)?;
    let s = clean(req)?;
    let data = serde_json::to_value(&s).map_err(|e| AppError::bad(e.to_string()))?;
    let exists = estate_setting::Entity::find_by_id(1).one(&state.db).await?.is_some();
    let row = estate_setting::ActiveModel { id: Set(1), data: Set(data) };
    if exists {
        row.update(&state.db).await?;
    } else {
        row.insert(&state.db).await?;
    }
    // Switching a module on or off changes everyone's effective permissions.
    let was = crate::convert::CLINIC_ON.swap(s.clinic_enabled, std::sync::atomic::Ordering::Relaxed);
    if was != s.clinic_enabled {
        tracing::info!(by = %me.0.name, on = s.clinic_enabled, "clinic module switched");
        state.hub.changed(&[DataScope::Settings, DataScope::Users]).await;
    } else {
        state.hub.changed(&[DataScope::Settings]).await;
    }
    Ok(Json(s))
}

/// `POST /api/settings/sample-data` (Office Admin) — the sample estate replaces
/// the current one. People, sign-ins and settings stay.
pub async fn load_sample(State(state): State<AppState>, me: CurrentUser) -> AppResult<StatusCode> {
    me.require(Permission::ManageSettings)?;
    crate::seed::load_sample(&state.db, me.0.id).await?;
    // The workforce sample comes with it, linked to the sample zones and jobs.
    crate::services::workforce_sample::load(&state.db).await?;
    after_reset(&state).await;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/settings/clear-data` (Office Admin) — an empty estate on a blank
/// map. People, sign-ins and settings stay.
pub async fn clear_data(State(state): State<AppState>, me: CurrentUser) -> AppResult<StatusCode> {
    me.require(Permission::ManageSettings)?;
    crate::seed::clear(&state.db).await?;
    after_reset(&state).await;
    Ok(StatusCode::NO_CONTENT)
}

/// Drops the uploads that belonged to the old estate and refreshes every open screen.
async fn after_reset(state: &AppState) {
    for folder in ["photos", "maps"] {
        if let Err(e) = tokio::fs::remove_dir_all(state.upload_dir.join(folder)).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(error = %e, folder, "could not remove old uploads");
            }
        }
    }
    use DataScope::*;
    state.hub.changed(&[Map, WorkItems, Assets, Decisions, Agenda, Notifications, Photos, Users, Settings, Workforce]).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_normalises_and_rejects() {
        let mut s = EstateSettings::default();
        s.roster_reset_time = "7:05".into();
        s.crew_move_reasons = vec!["  Rain ".into(), "rain".into(), "".into()];
        s.category_labels.insert(AlertCategory::MaterialDelay, "  ".into());
        let c = clean(s).unwrap();
        assert_eq!(c.roster_reset_time, "07:05");
        assert_eq!(c.crew_move_reasons, vec!["Rain".to_string()]);
        assert!(c.category_labels.is_empty());

        let mut bad = EstateSettings::default();
        bad.digest_time = "5pm".into();
        assert!(clean(bad).is_err());
    }

    #[test]
    fn settings_round_trip_with_enum_keys() {
        let mut s = EstateSettings::default();
        s.category_labels.insert(AlertCategory::MaterialDelay, "Late delivery".into());
        let back: EstateSettings = serde_json::from_value(serde_json::to_value(&s).unwrap()).unwrap();
        assert_eq!(back.category_label(AlertCategory::MaterialDelay), "Late delivery");
        assert_eq!(back.category_label(AlertCategory::DesignDecision), "Design choice");
    }
}

/// Creating or restoring a database package needs settings and backup access
/// (in practice, the Office Admin). Clinic records go along even while the
/// Clinic module is off, so a restore never loses them.
fn require_package(me: &CurrentUser) -> AppResult<()> {
    me.require(Permission::ManageSettings)?;
    me.require(Permission::BackupWorkforce)
}

/// `POST /api/settings/backup` — the whole database (estate, workforce, uploaded
/// files) as one package encrypted with the passphrase in `x-backup-passphrase`.
pub async fn backup_export(State(state): State<AppState>, me: CurrentUser, headers: HeaderMap) -> AppResult<Json<serde_json::Value>> {
    require_package(&me)?;
    let pass = crate::routes::workforce::passphrase(&headers).ok_or_else(|| AppError::bad("Choose a passphrase to protect the package."))?;
    let pkg = crate::services::package::build(&state.db, &state.upload_dir).await?;
    let sealed = tokio::task::spawn_blocking(move || crate::services::workforce::encrypt_agwcz(&pkg, &pass))
        .await
        .map_err(|e| AppError::bad(format!("The package stopped unexpectedly: {e}")))??;
    tracing::info!(by = %me.0.name, "database package created");
    Ok(Json(sealed))
}

/// `POST /api/settings/restore` — puts a package back (the raw file is the body).
/// Local login accounts are kept; a workforce snapshot is taken first.
pub async fn backup_import(State(state): State<AppState>, me: CurrentUser, headers: HeaderMap, body: axum::body::Bytes) -> AppResult<Json<crate::services::package::Restored>> {
    require_package(&me)?;
    let pass = crate::routes::workforce::passphrase(&headers).ok_or_else(|| AppError::bad("Enter the passphrase the package was made with."))?;
    let env: serde_json::Value = serde_json::from_slice(&body).map_err(|_| AppError::bad("This file isn't a SiteMonitor database package."))?;
    if env.get("_isAgwcEncrypted").and_then(|v| v.as_bool()) != Some(true) {
        return Err(AppError::bad("Only encrypted packages can be restored."));
    }
    let pkg = tokio::task::spawn_blocking(move || crate::services::workforce::decrypt_agwcz(&env, &pass))
        .await
        .map_err(|e| AppError::bad(format!("The restore stopped unexpectedly: {e}")))??;
    let report = crate::services::package::restore(&state.db, &state.upload_dir, pkg).await?;
    use DataScope::*;
    state.hub.changed(&[Map, WorkItems, Assets, Decisions, Agenda, Notifications, Photos, Users, Settings, Workforce]).await;
    tracing::info!(by = %me.0.name, workers = report.workers, files = report.files, "database package restored");
    Ok(Json(report))
}
