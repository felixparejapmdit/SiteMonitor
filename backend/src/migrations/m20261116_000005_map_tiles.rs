//! Zoom tiles for uploaded base map pictures.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261116_000005_map_tiles"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("ALTER TABLE base_map_versions ADD COLUMN IF NOT EXISTS tiles JSONB;").await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("ALTER TABLE base_map_versions DROP COLUMN IF EXISTS tiles;").await?;
        Ok(())
    }
}
