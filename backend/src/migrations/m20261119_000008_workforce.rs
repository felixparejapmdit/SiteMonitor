//! Workforce data merged in from Service 5 DB: people (workers/volunteers),
//! every other Service 5 collection kept losslessly as JSON rows, and snapshots.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261119_000008_workforce"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
CREATE TABLE IF NOT EXISTS workers (
    id            UUID PRIMARY KEY,
    a_number      TEXT NOT NULL DEFAULT '',
    name          TEXT NOT NULL,
    phone         TEXT NOT NULL DEFAULT '',
    trade         TEXT NOT NULL DEFAULT '',
    department    TEXT NOT NULL DEFAULT '',
    area          TEXT NOT NULL DEFAULT '',
    project_group TEXT NOT NULL DEFAULT '',
    group_role    TEXT NOT NULL DEFAULT '',
    daily_status  TEXT NOT NULL DEFAULT 'Present',
    archived      BOOLEAN NOT NULL DEFAULT FALSE,
    -- The full Service 5 record, kept verbatim so nothing is lost.
    profile       JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS workers_a_number_idx ON workers (a_number);

-- Every other Service 5 collection (attendance log, duty, OT, visitations, clinic…),
-- one row per item until its screen gives it a typed table.
CREATE TABLE IF NOT EXISTS workforce_records (
    collection TEXT NOT NULL,
    id         TEXT NOT NULL,
    data       JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (collection, id)
);

CREATE TABLE IF NOT EXISTS workforce_snapshots (
    id       UUID PRIMARY KEY,
    taken_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    reason   TEXT NOT NULL,
    workers  INTEGER NOT NULL DEFAULT 0,
    data     JSONB NOT NULL
);
"#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS workforce_snapshots; DROP TABLE IF EXISTS workforce_records; DROP TABLE IF EXISTS workers;")
            .await?;
        Ok(())
    }
}
