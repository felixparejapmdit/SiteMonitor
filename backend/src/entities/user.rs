//! SeaORM entity for the `users` table.
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub role: String,
    pub phone: String,
    pub username: String,
    pub assigned_area_ids: Json,
    pub is_active: bool,
    /// Argon2 PHC string; never leaves the server.
    pub password_hash: Option<String>,
    pub must_change_password: bool,
    pub permission_overrides: Json,
    pub last_sign_in_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
