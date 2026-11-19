//! SeaORM entity for the `decision_alerts` table.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "decision_alerts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub work_item_id: Uuid,
    pub area_id: Uuid,
    pub urgency: String,
    pub category: String,
    pub title: String,
    pub issue_description: String,
    pub mitigation_action_taken: String,
    pub option_a_label: Option<String>,
    pub option_b_label: Option<String>,
    pub status: String,
    pub created_at: DateTimeUtc,
    pub resolved_at: Option<DateTimeUtc>,
    pub photo_url: Option<String>,
    pub reported_by: Option<Uuid>,
    pub office_note: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
