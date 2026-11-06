//! "Quiet Day" special-occasion conflict engine (Phase 2 §3.2) and the
//! Master Scheduler's equipment double-booking detector (Phase 7 §2.3).

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Duration, Local, NaiveDate, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set, TransactionTrait};
use shared::api::{EquipmentConflict, QuietDayRequest, QuietDayResponse};
use shared::ws::{DataScope, WsEvent};
use shared::*;
use uuid::Uuid;

use crate::convert::{self, enc};
use crate::entities::{estate_agenda_item, prelude::*};
use crate::error::{AppError, AppResult};
use crate::services::{create_notice, log_timeline, NewNotice};
use crate::state::AppState;

pub const QUIET_DAY_CONFIRMATION: &str = "Confirmed. All heavy machinery (cranes, backhoes) and utility interruptions are automatically halted. Silent grounds detailing (pool & gardens) will proceed prior to your event.";

fn overlaps(a_start: DateTime<Utc>, a_end: DateTime<Utc>, b_start: DateTime<Utc>, b_end: DateTime<Utc>) -> bool {
    a_start < b_end && b_start < a_end
}

/// The owner books a Quiet Day: flag and pause every noisy or utility-cutting
/// task in the window, then alert staff instantly.
pub async fn schedule_quiet_day(state: &AppState, req: QuietDayRequest) -> AppResult<QuietDayResponse> {
    if req.title.trim().is_empty() {
        return Err(AppError::bad("Please give your occasion a name, e.g. \"Weekend Estate Gala\"."));
    }
    if req.end_time <= req.start_time {
        return Err(AppError::bad("The end time must be after the start time."));
    }
    let txn = state.db.begin().await?;
    let occasion = estate_agenda_item::ActiveModel {
        id: Set(Uuid::new_v4()),
        area_id: Set(None),
        work_item_id: Set(None),
        title: Set(req.title.trim().to_string()),
        description: Set(req.description.trim().to_string()),
        event_type: Set(enc(&AgendaEventType::VipSpecialOccasion)),
        intrusion_level: Set(enc(&IntrusionLevel::Silent)),
        start_time: Set(req.start_time),
        end_time: Set(req.end_time),
        is_flagged_conflict: Set(false),
    }
    .insert(&txn)
    .await?;
    let occasion = convert::agenda(occasion);

    let candidates = EstateAgendaItems::find()
        .filter(estate_agenda_item::Column::StartTime.lt(req.end_time))
        .filter(estate_agenda_item::Column::EndTime.gt(req.start_time))
        .all(&txn)
        .await?;
    let mut flagged = Vec::new();
    let mut paused = HashSet::new();
    for row in candidates {
        let item = convert::agenda(row.clone());
        if item.event_type == AgendaEventType::VipSpecialOccasion || !item.intrusion_level.is_intrusive() {
            continue;
        }
        let mut active = row.into_active_model();
        active.is_flagged_conflict = Set(true);
        let saved = convert::agenda(active.update(&txn).await?);
        if let Some(wid) = saved.work_item_id {
            if let Some(w) = WorkItems::find_by_id(wid).one(&txn).await? {
                let status: WorkStatus = convert::dec(&w.status);
                if matches!(status, WorkStatus::InProgress | WorkStatus::Planning) && paused.insert(wid) {
                    let mut wa = w.into_active_model();
                    wa.status = Set(enc(&WorkStatus::Halted));
                    wa.update(&txn).await?;
                    log_timeline(
                        &txn,
                        wid,
                        format!("Paused for Quiet Day: {}", occasion.title),
                        format!(
                            "\"{}\" was paused because it is {} and overlaps the owner's special occasion. It needs a new date.",
                            saved.title,
                            saved.intrusion_level.label().to_lowercase()
                        ),
                        false,
                        DelayCategory::None,
                        0,
                    )
                    .await?;
                }
            }
        }
        flagged.push(saved);
    }

    for role in [UserRole::OfficeAdmin, UserRole::SiteMonitor] {
        create_notice(
            &txn,
            NewNotice {
                audience: role,
                kind: NotificationKind::General,
                work_item_id: None,
                title: format!("Quiet Day booked: {}", occasion.title),
                body: if flagged.is_empty() {
                    "No noisy work or utility interruptions were scheduled in that window.".into()
                } else {
                    format!(
                        "{} need a new date. Open the Schedule to move them.",
                        super::status::plural(flagged.len(), "task", "tasks")
                    )
                },
                lines: flagged.iter().map(|f| f.title.clone()).collect(),
                visible_from: Utc::now(),
                dedupe_key: None,
            },
        )
        .await?;
    }
    txn.commit().await?;

    state
        .hub
        .to_staff(WsEvent::QuietDayActivated { occasion: occasion.clone(), flagged: flagged.clone() })
        .await;
    state
        .hub
        .changed(&[DataScope::Agenda, DataScope::WorkItems, DataScope::Map, DataScope::Notifications])
        .await;

    Ok(QuietDayResponse {
        occasion,
        flagged_items: flagged,
        paused_work_item_ids: paused.into_iter().collect(),
        confirmation: QUIET_DAY_CONFIRMATION.into(),
    })
}

/// Moves an agenda item; clears its Quiet-Day flag (and resumes paused work)
/// once it no longer overlaps any special occasion.
pub async fn reschedule(state: &AppState, id: Uuid, start: DateTime<Utc>, end: DateTime<Utc>) -> AppResult<EstateAgendaItem> {
    if end <= start {
        return Err(AppError::bad("The end time must be after the start time."));
    }
    let txn = state.db.begin().await?;
    let row = EstateAgendaItems::find_by_id(id).one(&txn).await?.ok_or_else(|| AppError::not_found("schedule item"))?;
    let before = convert::agenda(row.clone());
    if before.event_type == AgendaEventType::VipSpecialOccasion {
        return Err(AppError::bad("Only the owner can change their special occasion."));
    }
    let occasions: Vec<EstateAgendaItem> = EstateAgendaItems::find()
        .filter(estate_agenda_item::Column::EventType.eq(enc(&AgendaEventType::VipSpecialOccasion)))
        .all(&txn)
        .await?
        .into_iter()
        .map(convert::agenda)
        .collect();
    let still_conflicts = before.intrusion_level.is_intrusive()
        && occasions.iter().any(|o| overlaps(start, end, o.start_time, o.end_time));

    let mut active = row.into_active_model();
    active.start_time = Set(start);
    active.end_time = Set(end);
    active.is_flagged_conflict = Set(still_conflicts);
    let saved = convert::agenda(active.update(&txn).await?);

    if let Some(wid) = saved.work_item_id {
        let remaining = EstateAgendaItems::find()
            .filter(estate_agenda_item::Column::WorkItemId.eq(wid))
            .filter(estate_agenda_item::Column::IsFlaggedConflict.eq(true))
            .all(&txn)
            .await?
            .len();
        if remaining == 0 {
            if let Some(w) = WorkItems::find_by_id(wid).one(&txn).await? {
                let status: WorkStatus = convert::dec(&w.status);
                if status == WorkStatus::Halted && before.is_flagged_conflict {
                    let mut wa = w.into_active_model();
                    wa.status = Set(enc(&WorkStatus::InProgress));
                    wa.update(&txn).await?;
                    log_timeline(
                        &txn,
                        wid,
                        "Resumed after rescheduling",
                        format!(
                            "\"{}\" moved to {} so it no longer disturbs the owner's occasion.",
                            saved.title,
                            saved.start_time.with_timezone(&Local).format("%a %b %-d, %-I:%M %p")
                        ),
                        false,
                        DelayCategory::None,
                        0,
                    )
                    .await?;
                }
            }
        }
    }
    txn.commit().await?;
    state.hub.changed(&[DataScope::Agenda, DataScope::WorkItems, DataScope::Map]).await;
    Ok(saved)
}

/// Every local calendar day an agenda item touches.
fn days_covered(item: &EstateAgendaItem) -> Vec<NaiveDate> {
    let start = item.start_time.with_timezone(&Local).date_naive();
    // An item ending exactly at midnight does not occupy the next day.
    let end_instant = if item.end_time > item.start_time { item.end_time - Duration::seconds(1) } else { item.end_time };
    let end = end_instant.with_timezone(&Local).date_naive();
    let mut days = Vec::new();
    let mut d = start;
    while d <= end && days.len() < 366 {
        days.push(d);
        d = d.succ_opt().unwrap_or(d);
        if d == start {
            break;
        }
    }
    days
}

/// Finds mobile machinery booked in two different zones on the same day.
pub fn equipment_conflicts(
    items: &[EstateAgendaItem],
    reservations: &[EquipmentReservation],
    assets: &[Asset],
) -> Vec<EquipmentConflict> {
    let by_id: HashMap<Uuid, &EstateAgendaItem> = items.iter().map(|i| (i.id, i)).collect();
    // (asset, day) -> agenda items
    let mut grid: BTreeMap<(Uuid, NaiveDate), Vec<&EstateAgendaItem>> = BTreeMap::new();
    for r in reservations {
        if let Some(item) = by_id.get(&r.agenda_item_id) {
            for day in days_covered(item) {
                grid.entry((r.asset_id, day)).or_default().push(item);
            }
        }
    }
    let mut out = Vec::new();
    for ((asset_id, day), booked) in grid {
        let zones: HashSet<Uuid> = booked.iter().filter_map(|i| i.area_id).collect();
        if zones.len() >= 2 {
            let asset_name = assets
                .iter()
                .find(|a| a.id == asset_id)
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "Equipment".into());
            let mut area_ids: Vec<Uuid> = zones.into_iter().collect();
            area_ids.sort();
            out.push(EquipmentConflict {
                asset_id,
                asset_name,
                day: day.format("%Y-%m-%d").to_string(),
                agenda_item_ids: booked.iter().map(|i| i.id).collect(),
                area_ids,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(area: u128, start: DateTime<Utc>, hours: i64) -> EstateAgendaItem {
        EstateAgendaItem {
            id: Uuid::new_v4(),
            area_id: Some(Uuid::from_u128(area)),
            work_item_id: None,
            title: "t".into(),
            description: String::new(),
            event_type: AgendaEventType::IntrusiveMaintenance,
            intrusion_level: IntrusionLevel::NoiseHeavy,
            start_time: start,
            end_time: start + Duration::hours(hours),
            is_flagged_conflict: false,
        }
    }

    fn crane() -> Asset {
        Asset {
            id: Uuid::from_u128(99),
            name: "2-Ton Mini Crane".into(),
            mobility: MobilityType::MobileMachinery,
            current_area_id: Uuid::nil(),
            parent_asset_id: None,
            health: AssetHealth::Operational,
            active_issue_report: None,
            impact_level: ImpactLevel::NoImpact,
            repair_eta: None,
        }
    }

    #[test]
    fn detects_double_booked_crane() {
        let day = Local::now().date_naive().and_hms_opt(9, 0, 0).unwrap().and_local_timezone(Local).unwrap().with_timezone(&Utc);
        let a = item(1, day, 2);
        let b = item(2, day + Duration::hours(4), 2);
        let res = vec![
            EquipmentReservation { id: Uuid::new_v4(), asset_id: crane().id, agenda_item_id: a.id },
            EquipmentReservation { id: Uuid::new_v4(), asset_id: crane().id, agenda_item_id: b.id },
        ];
        let conflicts = equipment_conflicts(&[a, b], &res, &[crane()]);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].asset_name, "2-Ton Mini Crane");
    }

    #[test]
    fn same_zone_is_not_a_conflict() {
        let day = Local::now().date_naive().and_hms_opt(9, 0, 0).unwrap().and_local_timezone(Local).unwrap().with_timezone(&Utc);
        let a = item(1, day, 2);
        let b = item(1, day + Duration::hours(4), 2);
        let res = vec![
            EquipmentReservation { id: Uuid::new_v4(), asset_id: crane().id, agenda_item_id: a.id },
            EquipmentReservation { id: Uuid::new_v4(), asset_id: crane().id, agenda_item_id: b.id },
        ];
        assert!(equipment_conflicts(&[a, b], &res, &[crane()]).is_empty());
    }

    #[test]
    fn overlap_rules() {
        let t = Utc::now();
        assert!(overlaps(t, t + Duration::hours(2), t + Duration::hours(1), t + Duration::hours(3)));
        assert!(!overlaps(t, t + Duration::hours(1), t + Duration::hours(1), t + Duration::hours(2)));
    }
}
