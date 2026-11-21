//! People and the access matrix: the office adds, edits and deactivates
//! accounts, resets passwords, and decides what each person may do.

use axum::extract::{Path, State};
use axum::Json;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter, Set, Statement, TransactionTrait};
use shared::access::password_problem;
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::convert::{self, enc, to_json};
use crate::entities::{prelude::*, spatial_area, user};
use crate::error::{AppError, AppResult};
use crate::auth;
use crate::state::{AppState, CurrentUser};

/// A username: 3 to 40 characters, letters, digits, dot, dash or underscore.
pub fn username_ok(u: &str) -> bool {
    (3..=40).contains(&u.chars().count()) && u.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Trims and checks the form; returns the cleaned values.
async fn validate(state: &AppState, req: &SaveUserRequest, editing: Option<Uuid>) -> AppResult<(String, UserRole, String, String, Vec<Uuid>)> {
    let name = req.name.trim().to_string();
    let username = req.username.trim().to_lowercase();
    let phone = req.phone.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad("Please enter the person's name."));
    }
    let Some(role) = req.role else { return Err(AppError::bad("Please choose a role.")) };
    if !username_ok(&username) {
        return Err(AppError::bad("Please enter a username of 3 to 40 letters, digits, dots, dashes or underscores (no spaces)."));
    }
    let mut taken = Users::find().filter(user::Column::Username.eq(username.clone()));
    if let Some(id) = editing {
        taken = taken.filter(user::Column::Id.ne(id));
    }
    if taken.count(&state.db).await? > 0 {
        return Err(AppError::bad("Someone already uses that username."));
    }
    let areas = if role == UserRole::SiteMonitor { req.assigned_area_ids.clone() } else { vec![] };
    if !areas.is_empty() {
        let found = SpatialAreas::find().filter(spatial_area::Column::Id.is_in(areas.clone())).count(&state.db).await?;
        if found as usize != areas.len() {
            return Err(AppError::bad("One of the chosen zones no longer exists. Please refresh and try again."));
        }
    }
    Ok((name, role, phone, username, areas))
}

/// `POST /api/users` — add a person (Office Admin).
pub async fn create(State(state): State<AppState>, me: CurrentUser, Json(req): Json<SaveUserRequest>) -> AppResult<Json<User>> {
    me.require(Permission::ManagePeople)?;
    let (name, role, phone, username, areas) = validate(&state, &req, None).await?;
    let first = req.initial_password.clone().unwrap_or_default();
    if let Some(p) = password_problem(&first, &username) {
        return Err(AppError::bad(format!("First password: {p}")));
    }
    let hash = auth::hash_blocking(first.trim().to_string()).await?;
    let saved = user::ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(name),
        role: Set(enc(&role)),
        phone: Set(phone),
        username: Set(username),
        assigned_area_ids: Set(to_json(&areas)),
        is_active: Set(true),
        password_hash: Set(Some(hash)),
        must_change_password: Set(true),
        permission_overrides: Set(serde_json::json!({})),
        last_sign_in_at: Set(None),
    }
    .insert(&state.db)
    .await?;
    auth::audit(&state.db, me.0.id, saved.id, &format!("Account created as {}", role.label())).await?;
    state.hub.changed(&[DataScope::Users]).await;
    Ok(Json(convert::user(saved)))
}

/// `PUT /api/users/{id}` — edit details, change role, or (de)activate (Office Admin).
pub async fn update(
    State(state): State<AppState>,
    me: CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SaveUserRequest>,
) -> AppResult<Json<User>> {
    me.require(Permission::ManagePeople)?;
    let row = Users::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("person"))?;
    let (name, role, phone, username, areas) = validate(&state, &req, Some(id)).await?;
    let before = convert::user(row.clone());
    if id == me.0.id && (!req.is_active || role != before.role) {
        return Err(AppError::bad("You can't deactivate yourself or change your own role. Ask another Office Admin."));
    }
    let role_changed = role != before.role;
    // A new role starts from that role's defaults.
    let mut after = before.clone();
    after.role = role;
    after.is_active = req.is_active;
    if role_changed {
        after.access = AccessOverrides::default();
    }
    after.permissions = convert::permissions(&after.access, role);
    keep_an_access_admin(&state, &[after]).await?;
    let mut a = row.into_active_model();
    if role_changed {
        a.permission_overrides = Set(serde_json::json!({}));
    }
    a.name = Set(name);
    a.role = Set(enc(&role));
    a.phone = Set(phone);
    a.username = Set(username);
    a.assigned_area_ids = Set(to_json(&areas));
    a.is_active = Set(req.is_active);
    let saved = a.update(&state.db).await?;
    if role_changed {
        let msg = format!("Role changed from {} to {} (access reset to the role's defaults)", before.role.label(), role.label());
        auth::audit(&state.db, me.0.id, id, &msg).await?;
    }
    if before.is_active != req.is_active {
        auth::audit(&state.db, me.0.id, id, if req.is_active { "Account reactivated" } else { "Account deactivated and signed out" }).await?;
        if !req.is_active {
            auth::end_sessions(&state.db, id, None).await?;
        }
    }
    state.hub.changed(&[DataScope::Users]).await;
    Ok(Json(convert::user(saved)))
}

/// At least one active person must keep "Change access", or nobody could fix
/// access again. `replaced` are users as they would be after the change.
async fn keep_an_access_admin(state: &AppState, replaced: &[User]) -> AppResult<()> {
    let all: Vec<User> = Users::find().all(&state.db).await?.into_iter().map(convert::user).collect();
    let ok = all
        .iter()
        .map(|u| replaced.iter().find(|r| r.id == u.id).unwrap_or(u))
        .any(|u| u.is_active && u.can(Permission::ManageAccess));
    if ok {
        Ok(())
    } else {
        Err(AppError::bad(
            "At least one active person must keep \u{201c}Change access\u{201d}, or nobody could manage access any more.",
        ))
    }
}

/// `POST /api/users/{id}/reset-password` — issues a one-time password and signs the person out.
pub async fn reset_password(
    State(state): State<AppState>,
    me: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ResetPasswordResponse>> {
    me.require(Permission::ManagePeople)?;
    if id == me.0.id {
        return Err(AppError::bad("To change your own password, use \u{201c}Change password\u{201d} in your account menu."));
    }
    let row = Users::find_by_id(id).one(&state.db).await?.ok_or_else(|| AppError::not_found("person"))?;
    let temporary_password = auth::temporary_password();
    let mut a = row.into_active_model();
    a.password_hash = Set(Some(auth::hash_blocking(temporary_password.clone()).await?));
    a.must_change_password = Set(true);
    a.update(&state.db).await?;
    auth::end_sessions(&state.db, id, None).await?;
    auth::audit(&state.db, me.0.id, id, "Password reset; signed out everywhere").await?;
    state.hub.changed(&[DataScope::Users]).await;
    Ok(Json(ResetPasswordResponse { temporary_password }))
}

async fn matrix(state: &AppState) -> AppResult<AccessMatrixResponse> {
    let mut users: Vec<User> = Users::find().all(&state.db).await?.into_iter().map(convert::user).collect();
    users.sort_by_key(|u| (!u.is_active, UserRole::ALL.iter().position(|r| *r == u.role), u.name.clone()));
    let rows = state
        .db
        .query_all(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT a.id, a.at, coalesce(ac.name, 'Someone') AS actor, coalesce(t.name, 'Someone') AS target, a.summary
             FROM access_audit a LEFT JOIN users ac ON ac.id = a.actor_id LEFT JOIN users t ON t.id = a.target_id
             ORDER BY a.at DESC LIMIT 60",
        ))
        .await?;
    let audit = rows
        .into_iter()
        .map(|r| {
            Ok(AccessAuditEntry {
                id: r.try_get("", "id")?,
                at: r.try_get("", "at")?,
                actor_name: r.try_get("", "actor")?,
                target_name: r.try_get("", "target")?,
                summary: r.try_get("", "summary")?,
            })
        })
        .collect::<Result<Vec<_>, sea_orm::DbErr>>()?;
    Ok(AccessMatrixResponse { users, audit })
}

/// `GET /api/access` — everyone's effective permissions plus the change log.
pub async fn access_matrix(State(state): State<AppState>, me: CurrentUser) -> AppResult<Json<AccessMatrixResponse>> {
    me.require(Permission::ManageAccess)?;
    Ok(Json(matrix(&state).await?))
}

fn names(ps: &[Permission]) -> String {
    ps.iter().map(|p| p.label()).collect::<Vec<_>>().join(", ")
}

/// `PUT /api/access` — saves several people's access at once (all or nothing).
pub async fn save_access(
    State(state): State<AppState>,
    me: CurrentUser,
    Json(req): Json<SaveAccessRequest>,
) -> AppResult<Json<AccessMatrixResponse>> {
    me.require(Permission::ManageAccess)?;
    let mut planned: Vec<(User, String)> = vec![];
    for c in &req.changes {
        let row = Users::find_by_id(c.user_id).one(&state.db).await?.ok_or_else(|| AppError::not_found("person"))?;
        let before = convert::user(row);
        let mut access = c.access.normalized(before.role);
        // While the Clinic module is off its permission can't be seen or changed:
        // keep whatever each person had, so switching the module back on restores it.
        if !convert::CLINIC_ON.load(std::sync::atomic::Ordering::Relaxed) {
            access.granted.retain(|p| *p != Permission::ManageClinic);
            access.revoked.retain(|p| *p != Permission::ManageClinic);
            if before.access.granted.contains(&Permission::ManageClinic) {
                access.granted.push(Permission::ManageClinic);
            }
            if before.access.revoked.contains(&Permission::ManageClinic) {
                access.revoked.push(Permission::ManageClinic);
            }
            access = access.normalized(before.role);
        }
        let effective = convert::permissions(&access, before.role);
        if before.id == me.0.id && !effective.contains(&Permission::ManageAccess) {
            return Err(AppError::bad("You can't remove your own \u{201c}Change access\u{201d} permission. Ask another admin."));
        }
        let added: Vec<Permission> = effective.iter().copied().filter(|p| !before.permissions.contains(p)).collect();
        let removed: Vec<Permission> = before.permissions.iter().copied().filter(|p| !effective.contains(p)).collect();
        if added.is_empty() && removed.is_empty() && access == before.access {
            continue;
        }
        let mut parts = vec![];
        if !added.is_empty() {
            parts.push(format!("Granted: {}", names(&added)));
        }
        if !removed.is_empty() {
            parts.push(format!("Removed: {}", names(&removed)));
        }
        if parts.is_empty() {
            parts.push("Reset to the role's defaults".to_string());
        }
        let mut after = before;
        after.access = access;
        after.permissions = effective;
        planned.push((after, parts.join(" \u{b7} ")));
    }
    if planned.is_empty() {
        return Ok(Json(matrix(&state).await?));
    }
    keep_an_access_admin(&state, &planned.iter().map(|(u, _)| u.clone()).collect::<Vec<_>>()).await?;
    let txn = state.db.begin().await?;
    for (u, summary) in &planned {
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE users SET permission_overrides = $1 WHERE id = $2",
            [serde_json::to_value(&u.access).unwrap_or_default().into(), u.id.into()],
        ))
        .await?;
        auth::audit(&txn, me.0.id, u.id, summary).await?;
    }
    txn.commit().await?;
    state.hub.changed(&[DataScope::Users]).await;
    Ok(Json(matrix(&state).await?))
}

#[cfg(test)]
mod tests {
    use super::username_ok;

    #[test]
    fn usernames() {
        for ok in ["marco", "jun.dela-cruz", "ana_2", "abc"] {
            assert!(username_ok(ok), "{ok}");
        }
        for bad in ["ab", "has space", "name@example.com", "", &"x".repeat(41)] {
            assert!(!username_ok(bad), "{bad}");
        }
    }
}
