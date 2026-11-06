//! SeaORM entity for the `asset_transfer_logs` table.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "asset_transfer_logs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub asset_id: Uuid,
    pub from_area_id: Option<Uuid>,
    pub to_area_id: Uuid,
    pub moved_by: Uuid,
    pub timestamp: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
