//! Estate-wide settings record and retirable equipment.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261115_000003_settings_and_retired_assets"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE assets ADD COLUMN IF NOT EXISTS is_retired BOOLEAN NOT NULL DEFAULT FALSE;
                 CREATE TABLE IF NOT EXISTS estate_settings (id INTEGER PRIMARY KEY CHECK (id = 1), data JSONB NOT NULL);",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS estate_settings; ALTER TABLE assets DROP COLUMN IF EXISTS is_retired;")
            .await?;
        Ok(())
    }
}
