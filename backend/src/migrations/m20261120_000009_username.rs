//! People sign in with a username instead of an email address. Existing
//! accounts keep their sign-in: the part of the email before the "@" becomes
//! the username (lowercased; a number is added if two would clash).

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261120_000009_username"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
DROP INDEX IF EXISTS users_email_lower_idx;
ALTER TABLE users RENAME COLUMN email TO username;
WITH base AS (
    SELECT id, lower(regexp_replace(split_part(username, '@', 1), '[^a-zA-Z0-9._-]', '', 'g')) AS b FROM users
), fixed AS (
    SELECT id, CASE WHEN length(b) < 3 THEN b || '_user' ELSE b END AS b FROM base
), ranked AS (
    SELECT id, b, row_number() OVER (PARTITION BY b ORDER BY id) AS n FROM fixed
)
UPDATE users u SET username = CASE WHEN r.n = 1 THEN r.b ELSE r.b || r.n END FROM ranked r WHERE u.id = r.id;
CREATE UNIQUE INDEX IF NOT EXISTS users_username_lower_idx ON users (lower(username));
"#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "DROP INDEX IF EXISTS users_username_lower_idx;
                 ALTER TABLE users RENAME COLUMN username TO email;
                 CREATE UNIQUE INDEX IF NOT EXISTS users_email_lower_idx ON users (lower(email));",
            )
            .await?;
        Ok(())
    }
}
