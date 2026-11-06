//! SeaORM entity for the `executive_handover_reports` table.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "executive_handover_reports")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub work_item_id: Uuid,
    pub before_photo_url: String,
    pub after_photo_url: String,
    pub original_deadline: DateTimeUtc,
    pub actual_completion_date: DateTimeUtc,
    pub total_delay_days: i32,
    pub delay_audit_summary: Json,
    pub handed_over_assets: Json,
    pub client_signed_off: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
