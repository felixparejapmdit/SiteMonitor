//! Encrypted database packages for moving a whole SiteMonitor between
//! machines without GitHub: the estate (map, zones, projects, reports, photos,
//! equipment, schedule, settings), the workforce (personnel, every list, the
//! clinic) and the uploaded files, in one file encrypted with a passphrase
//! (the `.agwcz` envelope: PBKDF2-HMAC-SHA256 600,000 rounds + AES-256-GCM).
//!
//! Login accounts travel too, so who reported or moved what still points at
//! someone, but restoring never replaces a local account: package accounts are
//! matched by id, then by username, and only missing ones are added.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chrono::Utc;
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement, TransactionTrait};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::services::workforce;

pub const FORMAT: &str = "sitemonitor-package";

/// Estate tables in the order they can be put back (parents before children).
const TABLES: &[&str] = &[
    "base_map_versions",
    "spatial_areas",
    "work_items",
    "project_phases",
    "assets",
    "estate_agenda_items",
    "equipment_reservations",
    "asset_transfer_logs",
    "manpower_ledger_entries",
    "decision_alerts",
    "site_photos",
    "photo_call_outs",
    "timeline_event_records",
    "executive_handover_reports",
    "notifications",
];

/// Columns that point at a login account.
const USER_REFS: &[(&str, &str)] = &[("manpower_ledger_entries", "logged_by"), ("decision_alerts", "reported_by"), ("asset_transfer_logs", "moved_by"), ("site_photos", "uploaded_by")];

fn stmt(sql: &str, values: Vec<sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DatabaseBackend::Postgres, sql, values)
}

async fn rows_of<C: ConnectionTrait>(db: &C, table: &str) -> AppResult<Value> {
    // Table names come only from the fixed lists above.
    let row = db.query_one(stmt(&format!("SELECT COALESCE(jsonb_agg(to_jsonb(x)), '[]'::jsonb) AS j FROM {table} x"), vec![])).await?;
    Ok(row.and_then(|r| r.try_get::<Value>("", "j").ok()).unwrap_or(json!([])))
}

async fn schema<C: ConnectionTrait>(db: &C) -> AppResult<String> {
    let row = db.query_one(stmt("SELECT version FROM seaql_migrations ORDER BY version DESC LIMIT 1", vec![])).await?;
    Ok(row.and_then(|r| r.try_get::<String>("", "version").ok()).unwrap_or_default())
}

fn walk(dir: &Path, base: &Path, out: &mut Map<String, Value>) -> std::io::Result<()> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Ok(()) };
    for e in entries {
        let p = e?.path();
        if p.is_dir() {
            walk(&p, base, out)?;
        } else if let Ok(rel) = p.strip_prefix(base) {
            let key = rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/");
            out.insert(key, json!(B64.encode(std::fs::read(&p)?)));
        }
    }
    Ok(())
}

/// Everything, unencrypted (the caller encrypts it with the passphrase).
pub async fn build(db: &DatabaseConnection, upload_dir: &Path) -> AppResult<Value> {
    let mut tables = Map::new();
    for t in TABLES.iter().chain(["estate_settings"].iter()) {
        tables.insert(t.to_string(), rows_of(db, t).await?);
    }
    let mut uploads = Map::new();
    walk(upload_dir, upload_dir, &mut uploads).map_err(|e| AppError::bad(format!("Could not read the uploaded files: {e}")))?;
    Ok(json!({
        "format": FORMAT, "version": 1, "createdAt": Utc::now().to_rfc3339(), "schema": schema(db).await?,
        "users": rows_of(db, "users").await?, "tables": tables,
        "workforce": workforce::export(db).await?, "uploads": uploads,
    }))
}

/// A relative upload path that stays inside the upload folder.
fn safe_path(base: &Path, rel: &str) -> Option<PathBuf> {
    let p = Path::new(rel);
    if rel.is_empty() || p.is_absolute() || p.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    Some(base.join(p))
}

/// What a restore did.
#[derive(serde::Serialize)]
pub struct Restored {
    pub tables: Vec<(String, usize)>,
    pub accounts_added: usize,
    pub workers: usize,
    pub files: usize,
}

/// Replaces the estate, the workforce and the uploaded files with a package's.
/// Local login accounts are kept; a workforce snapshot is taken first.
pub async fn restore(db: &DatabaseConnection, upload_dir: &Path, pkg: Value) -> AppResult<Restored> {
    if pkg["format"] != FORMAT {
        return Err(AppError::bad("This file isn't a SiteMonitor database package."));
    }
    let here = schema(db).await?;
    if pkg["schema"].as_str() != Some(here.as_str()) {
        return Err(AppError::bad("This package was made by a different version of SiteMonitor. Update both machines to the same version first."));
    }
    let txn = db.begin().await?;
    // Accounts: keep every local one; match package ones by id, then username.
    let local = txn.query_all(stmt("SELECT id, lower(username) AS username FROM users", vec![])).await?;
    let by_id: Vec<Uuid> = local.iter().filter_map(|r| r.try_get("", "id").ok()).collect();
    let by_username: HashMap<String, Uuid> = local.iter().filter_map(|r| Some((r.try_get::<String>("", "username").ok()?, r.try_get::<Uuid>("", "id").ok()?))).collect();
    let mut map: HashMap<String, String> = HashMap::new();
    let mut added = 0;
    for u in pkg["users"].as_array().into_iter().flatten() {
        let id = u["id"].as_str().and_then(|s| Uuid::parse_str(s).ok()).ok_or_else(|| AppError::bad("The package has a damaged account."))?;
        // Packages made before usernames carry an email: its first part is the username.
        let mut u = u.clone();
        if u.get("username").is_none() {
            let from_email = u["email"].as_str().unwrap_or("").split('@').next().unwrap_or("").to_lowercase();
            u["username"] = json!(from_email);
        }
        let u = &u;
        let username = u["username"].as_str().unwrap_or("").to_lowercase();
        if by_id.contains(&id) {
            // Same account: its zones come with the restored map.
            txn.execute(stmt("UPDATE users SET assigned_area_ids = $2 WHERE id = $1", vec![id.into(), u["assigned_area_ids"].clone().into()])).await?;
        } else if let Some(local_id) = by_username.get(&username) {
            map.insert(id.to_string(), local_id.to_string());
            txn.execute(stmt("UPDATE users SET assigned_area_ids = $2 WHERE id = $1", vec![(*local_id).into(), u["assigned_area_ids"].clone().into()])).await?;
        } else {
            txn.execute(stmt("INSERT INTO users SELECT * FROM jsonb_populate_record(NULL::users, $1)", vec![u.clone().into()])).await?;
            added += 1;
        }
    }
    txn.execute_unprepared(&format!("TRUNCATE {}", TABLES.iter().rev().cloned().collect::<Vec<_>>().join(", "))).await?;
    let mut report = vec![];
    for t in TABLES {
        let mut rows = pkg["tables"][*t].as_array().cloned().unwrap_or_default();
        for (table, col) in USER_REFS.iter().filter(|(table, _)| table == t) {
            let _ = table;
            for r in rows.iter_mut() {
                if let Some(to) = r[*col].as_str().and_then(|from| map.get(from)) {
                    r[*col] = json!(to);
                }
            }
        }
        if !rows.is_empty() {
            txn.execute(stmt(&format!("INSERT INTO {t} SELECT * FROM jsonb_populate_recordset(NULL::{t}, $1)"), vec![Value::Array(rows.clone()).into()])).await?;
        }
        report.push((t.to_string(), rows.len()));
    }
    if let Some(s) = pkg["tables"]["estate_settings"].as_array().and_then(|a| a.first()) {
        txn.execute_unprepared("DELETE FROM estate_settings").await?;
        txn.execute(stmt("INSERT INTO estate_settings SELECT * FROM jsonb_populate_record(NULL::estate_settings, $1)", vec![s.clone().into()])).await?;
    }
    txn.commit().await?;

    // The workforce, through the usual importer (snapshot first; stored encrypted).
    workforce::snapshot(db, "before restoring a database package").await?;
    db.execute_unprepared("DELETE FROM workforce_records; DELETE FROM workers").await?;
    let n = workforce::normalize(pkg["workforce"].clone())?;
    let workers = n.workers.len();
    workforce::import(db, n, shared::api::ImportMode::Merge, true, &serde_json::Map::new()).await?;

    // Uploaded files.
    let mut files = 0;
    if upload_dir.exists() {
        for entry in std::fs::read_dir(upload_dir).map_err(|e| AppError::bad(e.to_string()))? {
            let p = entry.map_err(|e| AppError::bad(e.to_string()))?.path();
            let _ = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
        }
    }
    for (rel, data) in pkg["uploads"].as_object().into_iter().flatten() {
        let Some(path) = safe_path(upload_dir, rel) else { continue };
        let bytes = B64.decode(data.as_str().unwrap_or("")).map_err(|_| AppError::bad("The package has a damaged file."))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| AppError::bad(e.to_string()))?;
        }
        std::fs::write(&path, bytes).map_err(|e| AppError::bad(e.to_string()))?;
        files += 1;
    }
    Ok(Restored { tables: report, accounts_added: added, workers, files })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_paths_cannot_escape_the_folder() {
        let base = Path::new("/data/uploads");
        assert!(safe_path(base, "photos/a.jpg").is_some());
        assert!(safe_path(base, "../keys/personnel.key").is_none());
        assert!(safe_path(base, "/etc/passwd").is_none());
        assert!(safe_path(base, "photos/../../x").is_none());
        assert!(safe_path(base, "").is_none());
    }
}
