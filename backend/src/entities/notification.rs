//! SeaORM entity for the `notifications` table.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "notifications")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub audience: String,
    pub kind: String,
    pub work_item_id: Option<Uuid>,
    pub title: String,
    pub body: String,
    pub lines: Json,
    pub visible_from: DateTimeUtc,
    pub created_at: DateTimeUtc,
    pub dismissed: bool,
    pub dedupe_key: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
