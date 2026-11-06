//! SeaORM entity for the `estate_agenda_items` table.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "estate_agenda_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub area_id: Option<Uuid>,
    pub work_item_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub event_type: String,
    pub intrusion_level: String,
    pub start_time: DateTimeUtc,
    pub end_time: DateTimeUtc,
    pub is_flagged_conflict: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
