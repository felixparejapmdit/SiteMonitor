//! SeaORM entity for the `manpower_ledger_entries` table.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "manpower_ledger_entries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub work_item_id: Uuid,
    pub source_work_item_id: Option<Uuid>,
    pub headcount_delta: i32,
    pub resulting_headcount: i32,
    pub reason: String,
    pub logged_by: Uuid,
    pub timestamp: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
