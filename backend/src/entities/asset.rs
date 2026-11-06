//! SeaORM entity for the `assets` table.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "assets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub mobility: String,
    pub current_area_id: Uuid,
    pub parent_asset_id: Option<Uuid>,
    pub health: String,
    pub active_issue_report: Option<String>,
    pub impact_level: String,
    pub repair_eta: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
