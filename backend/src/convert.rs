//! Conversions between database rows and the shared domain models.
//!
//! Enums are stored as their variant name (e.g. `"PendingSignOff"`), which is
//! exactly how serde serialises them, so the two always stay in sync.


use sea_orm::prelude::Json;
use serde::de::DeserializeOwned;
use serde::Serialize;
use shared::*;
use uuid::Uuid;

use crate::entities::*;

/// Enum -> its variant name.
pub fn enc<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => s,
        Ok(other) => other.to_string(),
        Err(_) => String::new(),
    }
}

/// Variant name -> enum. Panics only on corrupted data (guarded by CHECKs).
pub fn dec<T: DeserializeOwned>(raw: &str) -> T {
    serde_json::from_value(serde_json::Value::String(raw.to_string()))
        .unwrap_or_else(|_| panic!("unknown enum value in database: {raw}"))
}

pub fn to_json<T: Serialize>(value: &T) -> Json {
    serde_json::to_value(value).unwrap_or(Json::Null)
}

pub fn from_json<T: DeserializeOwned + Default>(value: &Json) -> T {
    serde_json::from_value(value.clone()).unwrap_or_default()
}

pub fn user(m: user::Model) -> User {
    User {
        id: m.id,
        name: m.name,
        role: dec(&m.role),
        phone: m.phone,
        email: m.email,
        assigned_area_ids: from_json::<Vec<Uuid>>(&m.assigned_area_ids),
    }
}

pub fn base_map(m: base_map_version::Model) -> BaseMapVersion {
    BaseMapVersion {
        id: m.id,
        version_name: m.version_name,
        orthographic_image_url: m.orthographic_image_url,
        is_active: m.is_active,
        created_at: m.created_at,
    }
}

pub fn area(m: spatial_area::Model) -> SpatialArea {
    SpatialArea {
        id: m.id,
        base_map_id: m.base_map_id,
        parent_id: m.parent_id,
        name: m.name,
        zone_class: dec(&m.zone_class),
        polygon_points: from_json::<Vec<Point2D>>(&m.polygon_points),
        is_archived: m.is_archived,
    }
}

pub fn work_item(m: work_item::Model) -> WorkItem {
    WorkItem {
        id: m.id,
        area_id: m.area_id,
        stream: dec(&m.stream),
        title: m.title,
        description: m.description,
        status: dec(&m.status),
        calculated_progress: m.calculated_progress.clamp(0, 100) as u8,
        original_deadline: m.original_deadline,
        current_target_deadline: m.current_target_deadline,
        personnel: ProjectPersonnel {
            group_leader_name: m.group_leader_name,
            group_leader_phone: m.group_leader_phone,
            site_qc_officer_name: m.site_qc_officer_name,
            site_qc_officer_phone: m.site_qc_officer_phone,
            admin_coordinator_name: m.admin_coordinator_name,
            admin_coordinator_phone: m.admin_coordinator_phone,
        },
        active_worker_count: m.active_worker_count.max(0) as u32,
        before_photo_url: m.before_photo_url,
        after_photo_url: m.after_photo_url,
    }
}

pub fn phase(m: project_phase::Model) -> ProjectPhase {
    ProjectPhase {
        id: m.id,
        work_item_id: m.work_item_id,
        phase_name: m.phase_name,
        order_index: m.order_index.clamp(0, 255) as u8,
        weight_percentage: m.weight_percentage.clamp(0, 100) as u8,
        is_completed: m.is_completed,
        completed_at: m.completed_at,
        proof_photo_url: m.proof_photo_url,
    }
}

pub fn ledger(m: manpower_ledger_entry::Model) -> ManpowerLedgerEntry {
    ManpowerLedgerEntry {
        id: m.id,
        work_item_id: m.work_item_id,
        source_work_item_id: m.source_work_item_id,
        headcount_delta: m.headcount_delta,
        resulting_headcount: m.resulting_headcount.max(0) as u32,
        reason: m.reason,
        logged_by: m.logged_by,
        timestamp: m.timestamp,
    }
}

pub fn decision(m: decision_alert::Model) -> DecisionAlert {
    DecisionAlert {
        id: m.id,
        work_item_id: m.work_item_id,
        area_id: m.area_id,
        urgency: dec(&m.urgency),
        category: dec(&m.category),
        title: m.title,
        issue_description: m.issue_description,
        mitigation_action_taken: m.mitigation_action_taken,
        option_a_label: m.option_a_label,
        option_b_label: m.option_b_label,
        status: dec(&m.status),
        created_at: m.created_at,
        resolved_at: m.resolved_at,
    }
}

pub fn asset(m: asset::Model) -> Asset {
    Asset {
        id: m.id,
        name: m.name,
        mobility: dec(&m.mobility),
        current_area_id: m.current_area_id,
        parent_asset_id: m.parent_asset_id,
        health: dec(&m.health),
        active_issue_report: m.active_issue_report,
        impact_level: dec(&m.impact_level),
        repair_eta: m.repair_eta,
    }
}

pub fn transfer(m: asset_transfer_log::Model) -> AssetTransferLog {
    AssetTransferLog {
        id: m.id,
        asset_id: m.asset_id,
        from_area_id: m.from_area_id,
        to_area_id: m.to_area_id,
        moved_by: m.moved_by,
        timestamp: m.timestamp,
    }
}

pub fn photo(m: site_photo::Model) -> SitePhoto {
    SitePhoto {
        id: m.id,
        work_item_id: m.work_item_id,
        area_id: m.area_id,
        image_url: m.image_url,
        caption: m.caption,
        uploaded_by: m.uploaded_by,
        uploaded_at: m.uploaded_at,
        client_acknowledged: m.client_acknowledged,
    }
}

pub fn callout(m: photo_call_out::Model) -> PhotoCallOut {
    PhotoCallOut {
        id: m.id,
        photo_id: m.photo_id,
        x_percent: m.x_percent,
        y_percent: m.y_percent,
        client_comment: m.client_comment,
        staff_reply: m.staff_reply,
        status: dec(&m.status),
        created_at: m.created_at,
        resolved_at: m.resolved_at,
    }
}

pub fn timeline(m: timeline_event_record::Model) -> TimelineEventRecord {
    TimelineEventRecord {
        id: m.id,
        work_item_id: m.work_item_id,
        title: m.title,
        description: m.description,
        is_milestone: m.is_milestone,
        delay_category: dec(&m.delay_category),
        days_delayed: m.days_delayed.clamp(0, u16::MAX as i32) as u16,
        timestamp: m.timestamp,
    }
}

pub fn agenda(m: estate_agenda_item::Model) -> EstateAgendaItem {
    EstateAgendaItem {
        id: m.id,
        area_id: m.area_id,
        work_item_id: m.work_item_id,
        title: m.title,
        description: m.description,
        event_type: dec(&m.event_type),
        intrusion_level: dec(&m.intrusion_level),
        start_time: m.start_time,
        end_time: m.end_time,
        is_flagged_conflict: m.is_flagged_conflict,
    }
}

pub fn handover(m: executive_handover_report::Model) -> ExecutiveHandoverReport {
    ExecutiveHandoverReport {
        id: m.id,
        work_item_id: m.work_item_id,
        before_photo_url: m.before_photo_url,
        after_photo_url: m.after_photo_url,
        original_deadline: m.original_deadline,
        actual_completion_date: m.actual_completion_date,
        total_delay_days: m.total_delay_days.clamp(0, u16::MAX as i32) as u16,
        delay_audit_summary: from_json::<Vec<String>>(&m.delay_audit_summary),
        handed_over_assets: from_json::<Vec<Asset>>(&m.handed_over_assets),
        client_signed_off: m.client_signed_off,
    }
}

pub fn notification(m: notification::Model) -> Notification {
    Notification {
        id: m.id,
        audience: dec(&m.audience),
        kind: dec(&m.kind),
        work_item_id: m.work_item_id,
        title: m.title,
        body: m.body,
        lines: from_json::<Vec<String>>(&m.lines),
        visible_from: m.visible_from,
        created_at: m.created_at,
        dismissed: m.dismissed,
    }
}

pub fn reservation(m: equipment_reservation::Model) -> EquipmentReservation {
    EquipmentReservation { id: m.id, asset_id: m.asset_id, agenda_item_id: m.agenda_item_id }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_encoding_matches_serde() {
        assert_eq!(enc(&WorkStatus::PendingSignOff), "PendingSignOff");
        let back: WorkStatus = dec("PendingSignOff");
        assert_eq!(back, WorkStatus::PendingSignOff);
        assert_eq!(enc(&UserRole::VipClient), "VipClient");
    }
}
