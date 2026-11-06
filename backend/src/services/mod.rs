//! Business logic and automation engines.

pub mod agenda;
pub mod media;
pub mod progress;
pub mod scheduler;
pub mod status;

use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use shared::*;
use uuid::Uuid;

use crate::convert::{self, enc, to_json};
use crate::entities::{notification, prelude::*, timeline_event_record};
use crate::error::AppResult;

/// Writes one entry to a work item's milestone & delay audit trail.
#[allow(clippy::too_many_arguments)]
pub async fn log_timeline<C: ConnectionTrait>(
    db: &C,
    work_item_id: Uuid,
    title: impl Into<String>,
    description: impl Into<String>,
    is_milestone: bool,
    delay_category: DelayCategory,
    days_delayed: u16,
) -> AppResult<TimelineEventRecord> {
    let row = timeline_event_record::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_item_id: Set(work_item_id),
        title: Set(title.into()),
        description: Set(description.into()),
        is_milestone: Set(is_milestone),
        delay_category: Set(enc(&delay_category)),
        days_delayed: Set(days_delayed as i32),
        timestamp: Set(Utc::now()),
    }
    .insert(db)
    .await?;
    Ok(convert::timeline(row))
}

/// Parameters for a stored in-app notice.
pub struct NewNotice {
    pub audience: UserRole,
    pub kind: NotificationKind,
    pub work_item_id: Option<Uuid>,
    pub title: String,
    pub body: String,
    pub lines: Vec<String>,
    pub visible_from: DateTime<Utc>,
    /// When set, the notice is only created once for this key.
    pub dedupe_key: Option<String>,
}

/// Stores a notice; returns `None` if one with the same dedupe key exists.
pub async fn create_notice<C: ConnectionTrait>(db: &C, notice: NewNotice) -> AppResult<Option<Notification>> {
    if let Some(key) = &notice.dedupe_key {
        let existing = Notifications::find()
            .filter(notification::Column::DedupeKey.eq(key.clone()))
            .one(db)
            .await?;
        if existing.is_some() {
            return Ok(None);
        }
    }
    let row = notification::ActiveModel {
        id: Set(Uuid::new_v4()),
        audience: Set(enc(&notice.audience)),
        kind: Set(enc(&notice.kind)),
        work_item_id: Set(notice.work_item_id),
        title: Set(notice.title),
        body: Set(notice.body),
        lines: Set(to_json(&notice.lines)),
        visible_from: Set(notice.visible_from),
        created_at: Set(Utc::now()),
        dismissed: Set(false),
        dedupe_key: Set(notice.dedupe_key),
    }
    .insert(db)
    .await?;
    Ok(Some(convert::notification(row)))
}

/// Ids of an area and every non-archived-or-archived descendant beneath it.
pub fn subtree_ids(all: &[SpatialArea], root: Uuid) -> Vec<Uuid> {
    let mut out = vec![root];
    let mut i = 0;
    while i < out.len() {
        let current = out[i];
        for a in all.iter().filter(|a| a.parent_id == Some(current)) {
            out.push(a.id);
        }
        i += 1;
    }
    out
}

/// Loads every area as shared models.
pub async fn all_areas<C: ConnectionTrait>(db: &C) -> AppResult<Vec<SpatialArea>> {
    Ok(SpatialAreas::find().all(db).await?.into_iter().map(convert::area).collect())
}

/// Picks the first Office Admin to attribute automated actions to.
pub async fn system_user<C: ConnectionTrait>(db: &C) -> AppResult<Uuid> {
    use crate::entities::user;
    let admin = Users::find()
        .filter(user::Column::Role.eq(enc(&UserRole::OfficeAdmin)))
        .one(db)
        .await?;
    match admin {
        Some(u) => Ok(u.id),
        None => Ok(Users::find().one(db).await?.map(|u| u.id).unwrap_or_default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(id: u128, parent: Option<u128>) -> SpatialArea {
        SpatialArea {
            id: Uuid::from_u128(id),
            base_map_id: Uuid::nil(),
            parent_id: parent.map(Uuid::from_u128),
            name: String::new(),
            zone_class: ZoneClass::Recreational,
            polygon_points: vec![],
            is_archived: false,
        }
    }

    #[test]
    fn subtree_walks_infinite_nesting() {
        let all = vec![area(1, None), area(2, Some(1)), area(3, Some(2)), area(4, Some(3)), area(5, None)];
        let ids = subtree_ids(&all, Uuid::from_u128(1));
        assert_eq!(ids.len(), 4);
        assert!(!ids.contains(&Uuid::from_u128(5)));
    }
}
