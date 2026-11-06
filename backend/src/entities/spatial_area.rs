//! SeaORM entity for the `spatial_areas` table.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "spatial_areas")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub base_map_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub zone_class: String,
    pub polygon_points: Json,
    pub is_archived: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
