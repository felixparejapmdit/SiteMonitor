//! Sign-in: Argon2 password hashes, server-side sessions (an HttpOnly cookie
//! for browsers or a Bearer token for API clients), a sign-in throttle, and
//! the middleware that keeps every non-public `/api` route behind a session.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration as StdDuration, Instant};

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::Engine;
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseBackend, EntityTrait, IntoActiveModel, Set, Statement};
use sha2::{Digest, Sha256};
use shared::access::password_problem;
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::convert;
use crate::entities::{prelude::*, user};
use crate::error::{AppError, AppResult};
use crate::state::{AppState, CurrentUser};

/// How long a session lasts without use.
const SESSION_DAYS: i64 = 14;
/// Failed sign-ins allowed per username before a cool-down.
const MAX_FAILURES: u32 = 5;
const LOCKOUT: StdDuration = StdDuration::from_secs(15 * 60);
/// Password for the seeded demo accounts (shown on the sign-in page in demo mode).
pub const DEMO_PASSWORD: &str = "SiteMonitor-demo";

pub fn demo_mode() -> bool {
    std::env::var("SITEMONITOR_DEMO").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false)
}

fn secure_cookies() -> bool {
    std::env::var("SITEMONITOR_SECURE_COOKIES").map(|v| v == "1").unwrap_or(false)
}

// ------------------------------------------------------------------ passwords

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("hashing failed: {e}")))
}

fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).map(|h| Argon2::default().verify_password(password.as_bytes(), &h).is_ok()).unwrap_or(false)
}

/// Hashing is deliberately slow; keep it off the async workers.
pub async fn hash_blocking(password: String) -> AppResult<String> {
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
}

async fn verify_blocking(password: String, hash: Option<String>) -> bool {
    // Unknown accounts still pay for one verify so timing doesn't reveal which usernames exist.
    let hash = hash.unwrap_or_else(|| {
        "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHRzb21lc2FsdA$5V6ZpRMLEJdGJ0vKpFbLz0mZsXv3wCwq0V0FfE7rT3s".into()
    });
    tokio::task::spawn_blocking(move || verify_password(&password, &hash)).await.unwrap_or(false)
}

/// A readable one-time password, e.g. `kite-7342-amber`.
pub fn temporary_password() -> String {
    const WORDS: [&str; 16] = [
        "amber", "cedar", "delta", "ember", "fern", "harbor", "iris", "jade", "kite", "lotus", "maple", "nova", "orchid",
        "pine", "river", "sage",
    ];
    let mut b = [0u8; 4];
    OsRng.fill_bytes(&mut b);
    let n = u16::from_le_bytes([b[2], b[3]]) % 9000 + 1000;
    format!("{}-{n}-{}", WORDS[(b[0] % 16) as usize], WORDS[(b[1] % 16) as usize])
}

// ------------------------------------------------------------------ sessions

fn token_hash(token: &str) -> String {
    Sha256::digest(token.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// Session token from `Authorization: Bearer …` or the session cookie.
pub fn token_from(headers: &HeaderMap) -> Option<String> {
    if let Some(t) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        return Some(t.trim().to_string());
    }
    headers.get_all(header::COOKIE).iter().filter_map(|v| v.to_str().ok()).flat_map(|v| v.split(';')).find_map(|c| {
        let (k, v) = c.trim().split_once('=')?;
        (k == SESSION_COOKIE && !v.is_empty()).then(|| v.to_string())
    })
}

fn cookie(token: &str, max_age: i64) -> HeaderValue {
    let secure = if secure_cookies() { "; Secure" } else { "" };
    HeaderValue::from_str(&format!("{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}"))
        .expect("cookie is ASCII")
}

async fn start_session(db: &impl ConnectionTrait, user_id: Uuid, user_agent: &str) -> AppResult<String> {
    let mut raw = [0u8; 32];
    OsRng.fill_bytes(&mut raw);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);
    let now = Utc::now();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO sessions (id, user_id, token_hash, created_at, last_seen_at, expires_at, user_agent) VALUES ($1,$2,$3,$4,$4,$5,$6)",
        [
            Uuid::new_v4().into(),
            user_id.into(),
            token_hash(&token).into(),
            now.into(),
            (now + Duration::days(SESSION_DAYS)).into(),
            user_agent.chars().take(200).collect::<String>().into(),
        ],
    ))
    .await?;
    Ok(token)
}

/// The signed-in, active user for a token; slides the expiry while in use.
pub async fn user_for_token(db: &impl ConnectionTrait, token: &str) -> AppResult<Option<user::Model>> {
    let hash = token_hash(token);
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT user_id, last_seen_at FROM sessions WHERE token_hash = $1 AND expires_at > now()",
            [hash.clone().into()],
        ))
        .await?;
    let Some(row) = row else { return Ok(None) };
    let user_id: Uuid = row.try_get("", "user_id")?;
    let last_seen: chrono::DateTime<Utc> = row.try_get("", "last_seen_at")?;
    if Utc::now() - last_seen > Duration::hours(1) {
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE sessions SET last_seen_at = now(), expires_at = now() + make_interval(days => $2) WHERE token_hash = $1",
            [hash.into(), (SESSION_DAYS as i32).into()],
        ))
        .await?;
    }
    Ok(Users::find_by_id(user_id).one(db).await?.filter(|u| u.is_active))
}

/// Signs a person out everywhere, optionally keeping one session.
pub async fn end_sessions(db: &impl ConnectionTrait, user_id: Uuid, keep_token: Option<&str>) -> AppResult<()> {
    let keep = keep_token.map(token_hash).unwrap_or_default();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "DELETE FROM sessions WHERE user_id = $1 AND token_hash <> $2",
        [user_id.into(), keep.into()],
    ))
    .await?;
    Ok(())
}

/// Records an access or account change for the audit list.
pub async fn audit(db: &impl ConnectionTrait, actor: Uuid, target: Uuid, summary: &str) -> AppResult<()> {
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO access_audit (id, at, actor_id, target_id, summary) VALUES ($1, now(), $2, $3, $4)",
        [Uuid::new_v4().into(), actor.into(), target.into(), summary.to_string().into()],
    ))
    .await?;
    Ok(())
}

// ------------------------------------------------------------------ throttle

/// Per-username failure counter (in memory: resets when the server restarts).
#[derive(Clone, Default)]
pub struct LoginThrottle(Arc<Mutex<HashMap<String, (u32, Instant)>>>);

impl LoginThrottle {
    fn locked_for(&self, username: &str) -> Option<u64> {
        let map = self.0.lock().ok()?;
        let (fails, since) = map.get(username)?;
        let left = LOCKOUT.checked_sub(since.elapsed())?;
        (*fails >= MAX_FAILURES).then(|| left.as_secs() / 60 + 1)
    }
    fn fail(&self, username: &str) {
        if let Ok(mut map) = self.0.lock() {
            let e = map.entry(username.to_string()).or_insert((0, Instant::now()));
            if e.1.elapsed() > LOCKOUT {
                *e = (0, Instant::now());
            }
            e.0 += 1;
            e.1 = Instant::now();
        }
    }
    fn clear(&self, username: &str) {
        if let Ok(mut map) = self.0.lock() {
            map.remove(username);
        }
    }
}

// ------------------------------------------------------------------ middleware

/// Requires a valid session for every route it wraps. While a password change
/// is pending, only the account endpoints are allowed.
pub async fn guard(State(state): State<AppState>, mut req: Request, next: Next) -> Response {
    let Some(token) = token_from(req.headers()) else {
        return AppError::Unauthorized("Please sign in to continue.".into()).into_response();
    };
    let user = match user_for_token(&state.db, &token).await {
        Ok(Some(u)) => convert::user(u),
        Ok(None) => return AppError::Unauthorized("Your session has ended. Please sign in again.".into()).into_response(),
        Err(e) => return e.into_response(),
    };
    let path = req.uri().path().to_string();
    if user.must_change_password && !(path.ends_with("/auth/me") || path.ends_with("/auth/password")) {
        return AppError::forbidden("Please choose a new password before continuing.").into_response();
    }
    req.extensions_mut().insert(user);
    next.run(req).await
}

// ------------------------------------------------------------------ handlers

/// `GET /api/auth/options` — public: the demo accounts (demo mode only).
pub async fn options(State(state): State<AppState>) -> AppResult<Json<AuthOptions>> {
    if !demo_mode() {
        return Ok(Json(AuthOptions::default()));
    }
    let mut demo_accounts: Vec<DemoAccount> = Users::find()
        .all(&state.db)
        .await?
        .into_iter()
        .filter(|u| u.is_active)
        .map(|u| DemoAccount { name: u.name, role: convert::dec(&u.role), username: u.username })
        .collect();
    demo_accounts.sort_by_key(|a| (UserRole::ALL.iter().position(|r| *r == a.role), a.name.clone()));
    Ok(Json(AuthOptions { demo_accounts, demo_password: Some(DEMO_PASSWORD.into()) }))
}

/// `POST /api/auth/login`
pub async fn login(State(state): State<AppState>, headers: HeaderMap, Json(req): Json<LoginRequest>) -> AppResult<Response> {
    let username = req.username.trim().to_lowercase();
    if username.is_empty() || req.password.is_empty() {
        return Err(AppError::bad("Please enter your username and password."));
    }
    if let Some(mins) = state.throttle.locked_for(&username) {
        return Err(AppError::TooMany(format!(
            "Too many attempts. For your security, try again in {mins} minute{}.",
            if mins == 1 { "" } else { "s" }
        )));
    }
    let found = state
        .db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM users WHERE lower(username) = $1",
            [username.clone().into()],
        ))
        .await?;
    let row = match found {
        Some(r) => Users::find_by_id(r.try_get::<Uuid>("", "id")?).one(&state.db).await?,
        None => None,
    };
    let ok = verify_blocking(req.password.clone(), row.as_ref().and_then(|u| u.password_hash.clone())).await;
    let Some(row) = row.filter(|_| ok) else {
        state.throttle.fail(&username);
        return Err(AppError::Unauthorized("That username and password don't match. Please try again.".into()));
    };
    if !row.is_active {
        return Err(AppError::forbidden("This account has been deactivated. Please contact the Office Admin."));
    }
    state.throttle.clear(&username);
    let ua = headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).unwrap_or_default();
    let token = start_session(&state.db, row.id, ua).await?;
    let mut a = row.into_active_model();
    a.last_sign_in_at = Set(Some(Utc::now()));
    let user = convert::user(a.update(&state.db).await?);
    let mut res = Json(SessionResponse { user, token: token.clone() }).into_response();
    res.headers_mut().insert(header::SET_COOKIE, cookie(&token, SESSION_DAYS * 86_400));
    Ok(res)
}

/// `POST /api/auth/logout` — ends this session and clears the cookie.
pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    if let Some(t) = token_from(&headers) {
        state
            .db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "DELETE FROM sessions WHERE token_hash = $1",
                [token_hash(&t).into()],
            ))
            .await?;
    }
    let mut res = Json(serde_json::json!({ "ok": true })).into_response();
    res.headers_mut().insert(header::SET_COOKIE, cookie("", 0));
    Ok(res)
}

/// `GET /api/auth/me` — public: the signed-in person, or `null` when nobody
/// is (a normal answer on the sign-in page, so not an error).
pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Json<Option<User>>> {
    let Some(token) = token_from(&headers) else { return Ok(Json(None)) };
    Ok(Json(user_for_token(&state.db, &token).await?.map(convert::user)))
}

/// `POST /api/auth/password` — change my own password; signs out my other devices.
pub async fn change_password(
    State(state): State<AppState>,
    me: CurrentUser,
    headers: HeaderMap,
    Json(req): Json<ChangePasswordRequest>,
) -> AppResult<Json<User>> {
    let row = Users::find_by_id(me.0.id).one(&state.db).await?.ok_or_else(|| AppError::not_found("account"))?;
    if !verify_blocking(req.current_password.clone(), row.password_hash.clone()).await {
        return Err(AppError::bad("Your current password isn't right."));
    }
    if let Some(problem) = password_problem(&req.new_password, &row.username) {
        return Err(AppError::bad(problem));
    }
    if req.new_password == req.current_password {
        return Err(AppError::bad("Choose a password you haven't just used."));
    }
    let hash = hash_blocking(req.new_password.trim().to_string()).await?;
    let mut a = row.into_active_model();
    a.password_hash = Set(Some(hash));
    a.must_change_password = Set(false);
    let saved = a.update(&state.db).await?;
    end_sessions(&state.db, saved.id, token_from(&headers).as_deref()).await?;
    audit(&state.db, saved.id, saved.id, "Changed their own password").await?;
    state.hub.changed(&[DataScope::Users]).await;
    Ok(Json(convert::user(saved)))
}

// ------------------------------------------------------------------ bootstrap

/// Gives accounts that have never had a password the demo password (demo mode
/// only), so an existing database stays usable after sign-in arrives.
pub async fn ensure_passwords(db: &impl ConnectionTrait) -> AppResult<()> {
    let missing = db
        .query_one(Statement::from_string(DatabaseBackend::Postgres, "SELECT count(*) AS n FROM users WHERE password_hash IS NULL"))
        .await?
        .map(|r| r.try_get::<i64>("", "n"))
        .transpose()?
        .unwrap_or(0);
    if missing == 0 {
        return Ok(());
    }
    if demo_mode() {
        let hash = hash_blocking(DEMO_PASSWORD.into()).await?;
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE users SET password_hash = $1 WHERE password_hash IS NULL",
            [hash.into()],
        ))
        .await?;
        tracing::info!("gave {missing} account(s) the demo password");
    } else {
        tracing::warn!(
            "{missing} account(s) have no password and can't sign in. Set one with: backend set-password <username> <password>"
        );
    }
    Ok(())
}

/// `backend set-password <username> <password>` — recovery from the command line.
pub async fn set_password_cli(db: &impl ConnectionTrait, username: &str, password: &str) -> anyhow::Result<()> {
    if let Some(p) = password_problem(password, username) {
        anyhow::bail!("{p}");
    }
    let hash = hash_password(password).map_err(|e| anyhow::anyhow!("{e}"))?;
    let res = db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE users SET password_hash = $1, must_change_password = FALSE, is_active = TRUE WHERE lower(username) = lower($2)",
            [hash.into(), username.to_string().into()],
        ))
        .await?;
    if res.rows_affected() == 0 {
        anyhow::bail!("no account uses {username}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_verify_and_tokens_parse() {
        let h = hash_password("correct horse battery").unwrap();
        assert!(verify_password("correct horse battery", &h));
        assert!(!verify_password("wrong horse battery", &h));
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, HeaderValue::from_static("theme=dark; sm_session=abc123"));
        assert_eq!(token_from(&headers).as_deref(), Some("abc123"));
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer xyz"));
        assert_eq!(token_from(&headers).as_deref(), Some("xyz"));
        let t = temporary_password();
        assert!(password_problem(&t, "").is_none(), "{t}");
    }
}
