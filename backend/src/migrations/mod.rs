//! Versioned SQL migrations, applied automatically at start-up.

use sea_orm_migration::prelude::*;

mod m20261107_000001_create_estate_schema;
mod m20261115_000002_user_active_flag;
mod m20261115_000003_settings_and_retired_assets;
mod m20261116_000004_auth_and_access;
mod m20261116_000005_map_tiles;
mod m20261116_000006_report_photo_and_author;
mod m20261116_000007_report_office_note;
mod m20261119_000008_workforce;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261107_000001_create_estate_schema::Migration),
            Box::new(m20261115_000002_user_active_flag::Migration),
            Box::new(m20261115_000003_settings_and_retired_assets::Migration),
            Box::new(m20261116_000004_auth_and_access::Migration),
            Box::new(m20261116_000005_map_tiles::Migration),
            Box::new(m20261116_000006_report_photo_and_author::Migration),
            Box::new(m20261116_000007_report_office_note::Migration),
            Box::new(m20261119_000008_workforce::Migration),
        ]
    }
}
