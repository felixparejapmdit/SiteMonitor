//! Sign-in (password hashes, sessions) and the per-person access matrix.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261116_000004_auth_and_access"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE users ADD COLUMN IF NOT EXISTS password_hash TEXT;
                 ALTER TABLE users ADD COLUMN IF NOT EXISTS must_change_password BOOLEAN NOT NULL DEFAULT FALSE;
                 ALTER TABLE users ADD COLUMN IF NOT EXISTS permission_overrides JSONB NOT NULL DEFAULT '{}'::jsonb;
                 ALTER TABLE users ADD COLUMN IF NOT EXISTS last_sign_in_at TIMESTAMPTZ;
                 CREATE UNIQUE INDEX IF NOT EXISTS users_email_lower_idx ON users (lower(email));
                 CREATE TABLE IF NOT EXISTS sessions (
                     id UUID PRIMARY KEY,
                     user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                     token_hash TEXT NOT NULL UNIQUE,
                     created_at TIMESTAMPTZ NOT NULL,
                     last_seen_at TIMESTAMPTZ NOT NULL,
                     expires_at TIMESTAMPTZ NOT NULL,
                     user_agent TEXT NOT NULL DEFAULT ''
                 );
                 CREATE INDEX IF NOT EXISTS sessions_user_idx ON sessions (user_id);
                 CREATE TABLE IF NOT EXISTS access_audit (
                     id UUID PRIMARY KEY,
                     at TIMESTAMPTZ NOT NULL,
                     actor_id UUID REFERENCES users(id) ON DELETE SET NULL,
                     target_id UUID REFERENCES users(id) ON DELETE SET NULL,
                     summary TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS access_audit_at_idx ON access_audit (at DESC);",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS access_audit; DROP TABLE IF EXISTS sessions; DROP INDEX IF EXISTS users_email_lower_idx;
                 ALTER TABLE users DROP COLUMN IF EXISTS password_hash, DROP COLUMN IF EXISTS must_change_password,
                   DROP COLUMN IF EXISTS permission_overrides, DROP COLUMN IF EXISTS last_sign_in_at;",
            )
            .await?;
        Ok(())
    }
}
