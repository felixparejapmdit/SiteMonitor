//! SeaORM entity for the `project_phases` table.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "project_phases")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub work_item_id: Uuid,
    pub phase_name: String,
    pub order_index: i32,
    pub weight_percentage: i32,
    pub is_completed: bool,
    pub completed_at: Option<DateTimeUtc>,
    pub proof_photo_url: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
