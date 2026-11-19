//! The office's reason when it declines a field report or asks for more details.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261116_000007_report_office_note"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("ALTER TABLE decision_alerts ADD COLUMN IF NOT EXISTS office_note TEXT;").await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("ALTER TABLE decision_alerts DROP COLUMN IF EXISTS office_note;").await?;
        Ok(())
    }
}
