//! SeaORM entity for the `site_photos` table.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "site_photos")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub work_item_id: Uuid,
    pub area_id: Uuid,
    pub image_url: String,
    pub caption: String,
    pub uploaded_by: Uuid,
    pub uploaded_at: DateTimeUtc,
    pub client_acknowledged: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
