//! SeaORM entity for the `timeline_event_records` table.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "timeline_event_records")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub work_item_id: Uuid,
    pub title: String,
    pub description: String,
    pub is_milestone: bool,
    pub delay_category: String,
    pub days_delayed: i32,
    pub timestamp: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
