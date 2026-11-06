//! SeaORM entity for the `photo_call_outs` table.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "photo_call_outs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub photo_id: Uuid,
    pub x_percent: f32,
    pub y_percent: f32,
    pub client_comment: String,
    pub staff_reply: Option<String>,
    pub status: String,
    pub created_at: DateTimeUtc,
    pub resolved_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
