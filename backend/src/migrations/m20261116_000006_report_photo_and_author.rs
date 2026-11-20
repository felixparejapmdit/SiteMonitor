//! Field reports keep their photo and who wrote them (so the office sees the
//! photo and the author can edit their own report).

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261116_000006_report_photo_and_author"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE decision_alerts ADD COLUMN IF NOT EXISTS photo_url TEXT;
                 ALTER TABLE decision_alerts ADD COLUMN IF NOT EXISTS reported_by UUID REFERENCES users(id) ON DELETE SET NULL;
                 -- Older reports: their photo went to the project gallery captioned 'Field report: <title>'.
                 UPDATE decision_alerts d SET photo_url = p.image_url, reported_by = p.uploaded_by
                 FROM (SELECT DISTINCT ON (work_item_id, caption) work_item_id, caption, image_url, uploaded_by
                       FROM site_photos ORDER BY work_item_id, caption, uploaded_at DESC) p
                 WHERE p.work_item_id = d.work_item_id AND p.caption = 'Field report: ' || d.title AND d.photo_url IS NULL;",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE decision_alerts DROP COLUMN IF EXISTS reported_by, DROP COLUMN IF EXISTS photo_url;")
            .await?;
        Ok(())
    }
}
