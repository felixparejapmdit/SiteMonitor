//! SeaORM entity for the `work_items` table.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "work_items")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub area_id: Uuid,
    pub stream: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub calculated_progress: i32,
    pub original_deadline: DateTimeUtc,
    pub current_target_deadline: DateTimeUtc,
    pub group_leader_name: String,
    pub group_leader_phone: String,
    pub site_qc_officer_name: String,
    pub site_qc_officer_phone: String,
    pub admin_coordinator_name: String,
    pub admin_coordinator_phone: String,
    pub active_worker_count: i32,
    pub before_photo_url: Option<String>,
    pub after_photo_url: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
