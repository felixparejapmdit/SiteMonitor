//! Versioned SQL migrations, applied automatically at start-up.

use sea_orm_migration::prelude::*;

mod m20261107_000001_create_estate_schema;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20261107_000001_create_estate_schema::Migration)]
    }
}
