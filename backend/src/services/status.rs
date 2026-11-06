//! Computes the live health of every map polygon and builds the nested tree.

use std::collections::HashMap;

use sea_orm::{ConnectionTrait, EntityTrait};
use shared::api::AreaNode;
use shared::*;
use uuid::Uuid;

use crate::convert;
use crate::entities::prelude::*;
use crate::error::AppResult;

/// Everything needed to judge an area, gathered once per request.
pub struct EstateSnapshot {
    pub areas: Vec<SpatialArea>,
    pub items: Vec<WorkItem>,
    pub assets: Vec<Asset>,
    pub decisions: Vec<DecisionAlert>,
    pub open_callouts_by_item: HashMap<Uuid, usize>,
    pub flagged_agenda_by_area: HashMap<Uuid, usize>,
}

impl EstateSnapshot {
    pub async fn load<C: ConnectionTrait>(db: &C) -> AppResult<Self> {
        let areas = SpatialAreas::find().all(db).await?.into_iter().map(convert::area).collect();
        let items = WorkItems::find().all(db).await?.into_iter().map(convert::work_item).collect();
        let assets = Assets::find().all(db).await?.into_iter().map(convert::asset).collect();
        let decisions = DecisionAlerts::find().all(db).await?.into_iter().map(convert::decision).collect();

        let photos: HashMap<Uuid, Uuid> = SitePhotos::find()
            .all(db)
            .await?
            .into_iter()
            .map(|p| (p.id, p.work_item_id))
            .collect();
        let mut open_callouts_by_item = HashMap::new();
        for c in PhotoCallOuts::find().all(db).await?.into_iter().map(convert::callout) {
            if c.status != CallOutStatus::Resolved {
                if let Some(item) = photos.get(&c.photo_id) {
                    *open_callouts_by_item.entry(*item).or_insert(0) += 1;
                }
            }
        }
        let mut flagged_agenda_by_area = HashMap::new();
        for a in EstateAgendaItems::find().all(db).await?.into_iter().map(convert::agenda) {
            if a.is_flagged_conflict {
                if let Some(area) = a.area_id {
                    *flagged_agenda_by_area.entry(area).or_insert(0) += 1;
                }
            }
        }
        Ok(Self { areas, items, assets, decisions, open_callouts_by_item, flagged_agenda_by_area })
    }

    /// Status + plain-language reason for a single area, ignoring children.
    fn own_status(&self, area: &SpatialArea) -> (AreaStatus, Vec<String>) {
        if area.is_archived {
            return (AreaStatus::Archived, vec!["This location is archived".into()]);
        }
        let mut status = AreaStatus::Optimal;
        let mut reasons = Vec::new();
        let raise = |s: &mut AreaStatus, to: AreaStatus| {
            if severity(to) > severity(*s) {
                *s = to;
            }
        };

        for asset in self.assets.iter().filter(|a| a.current_area_id == area.id) {
            match asset.health {
                AssetHealth::BrokenDown => {
                    raise(&mut status, AreaStatus::CriticalAlert);
                    reasons.push(format!("{} has broken down", asset.name));
                }
                AssetHealth::NeedsMaintenance => {
                    raise(&mut status, AreaStatus::AttentionRequired);
                    reasons.push(format!("{} needs service", asset.name));
                }
                AssetHealth::Operational => {}
            }
            if asset.impact_level == ImpactLevel::FullWorkStoppage && asset.health != AssetHealth::Operational {
                raise(&mut status, AreaStatus::CriticalAlert);
            }
        }

        for item in self.items.iter().filter(|w| w.area_id == area.id) {
            let pending = self
                .decisions
                .iter()
                .filter(|d| d.work_item_id == item.id && d.status == DecisionStatus::PublishedPendingClient)
                .count();
            if pending > 0 {
                raise(&mut status, AreaStatus::AttentionRequired);
                reasons.push(plural(pending, "decision waiting for the owner", "decisions waiting for the owner"));
            }
            if let Some(n) = self.open_callouts_by_item.get(&item.id) {
                raise(&mut status, AreaStatus::AttentionRequired);
                reasons.push(plural(*n, "open photo note", "open photo notes"));
            }
            match item.status {
                WorkStatus::PendingSignOff => {
                    raise(&mut status, AreaStatus::AttentionRequired);
                    reasons.push(format!("{} is ready for sign-off", item.title));
                }
                WorkStatus::InProgress | WorkStatus::Halted => {
                    raise(&mut status, AreaStatus::ActiveWork);
                    if item.status == WorkStatus::Halted {
                        reasons.push(format!("{} is paused", item.title));
                    }
                }
                _ => {}
            }
        }
        if let Some(n) = self.flagged_agenda_by_area.get(&area.id) {
            raise(&mut status, AreaStatus::AttentionRequired);
            reasons.push(plural(*n, "task needs a new date", "tasks need new dates"));
        }
        let active = self
            .items
            .iter()
            .filter(|w| w.area_id == area.id && matches!(w.status, WorkStatus::InProgress))
            .count();
        if active > 0 {
            reasons.push(plural(active, "job in progress", "jobs in progress"));
        }
        (status, reasons)
    }

    /// Short summary for utility / contractor zones shown to the owner.
    fn executive_summary(&self, area: &SpatialArea, subtree: &[Uuid]) -> Option<String> {
        if area.zone_class.is_front_of_house() || area.is_archived {
            return None;
        }
        let assets: Vec<&Asset> = self.assets.iter().filter(|a| subtree.contains(&a.current_area_id)).collect();
        let mut parts = Vec::new();
        if assets.is_empty() {
            parts.push("No equipment is stored here right now.".to_string());
        } else {
            let ok = assets.iter().filter(|a| a.health == AssetHealth::Operational).count();
            parts.push(format!("{ok} of {} pieces of equipment working normally.", assets.len()));
            let batteries: Vec<&&Asset> = assets.iter().filter(|a| a.mobility == MobilityType::ModularComponent).collect();
            if !batteries.is_empty() {
                let healthy = batteries.iter().filter(|a| a.health == AssetHealth::Operational).count();
                parts.push(format!("Battery array: {healthy} of {} batteries healthy.", batteries.len()));
            }
            for a in assets.iter().filter(|a| a.health != AssetHealth::Operational) {
                let eta = a
                    .repair_eta
                    .map(|t| format!(" — expected fixed {}", t.format("%b %-d")))
                    .unwrap_or_default();
                parts.push(format!("{}: {}{eta}.", a.name, a.health.label().to_lowercase()));
            }
            let machines: Vec<&str> = assets
                .iter()
                .filter(|a| a.mobility == MobilityType::MobileMachinery)
                .map(|a| a.name.as_str())
                .collect();
            if !machines.is_empty() {
                parts.push(format!("Machinery here: {}.", machines.join(", ")));
            }
        }
        if area.zone_class == ZoneClass::ContractorOperations {
            let delays = self
                .decisions
                .iter()
                .filter(|d| d.category == AlertCategory::MaterialDelay && !d.status.is_resolved())
                .count();
            parts.push(if delays == 0 {
                "Material deliveries: all on schedule.".into()
            } else {
                format!("Material deliveries: {}.", plural(delays, "delay being handled", "delays being handled"))
            });
        }
        Some(parts.join(" "))
    }

    fn node(&self, area: &SpatialArea) -> AreaNode {
        let children: Vec<AreaNode> = self
            .areas
            .iter()
            .filter(|a| a.parent_id == Some(area.id))
            .map(|a| self.node(a))
            .collect();
        let (mut status, mut reasons) = self.own_status(area);
        if status != AreaStatus::Archived {
            for c in children.iter().filter(|c| c.status != AreaStatus::Archived) {
                if severity(c.status) > severity(status) {
                    status = c.status;
                }
                if c.status != AreaStatus::Optimal && c.status != AreaStatus::ActiveWork {
                    let detail = c
                        .status_reason
                        .split(" · ")
                        .find(|r| !r.contains("in progress"))
                        .unwrap_or(c.status.label());
                    let detail = detail.split_once(" — ").map(|(_, d)| d).unwrap_or(detail);
                    reasons.push(format!("{} — {detail}", c.area.name));
                }
            }
        }
        let subtree = super::subtree_ids(&self.areas, area.id);
        let status_reason = if reasons.is_empty() {
            "Everything is running smoothly".to_string()
        } else {
            reasons.dedup();
            reasons.join(" · ")
        };
        AreaNode {
            area: area.clone(),
            status,
            status_reason,
            executive_summary: self.executive_summary(area, &subtree),
            children,
        }
    }

    pub fn tree(&self) -> Vec<AreaNode> {
        self.areas.iter().filter(|a| a.parent_id.is_none()).map(|a| self.node(a)).collect()
    }
}

fn severity(s: AreaStatus) -> u8 {
    match s {
        AreaStatus::Archived => 0,
        AreaStatus::Optimal => 1,
        AreaStatus::ActiveWork => 2,
        AreaStatus::AttentionRequired => 3,
        AreaStatus::CriticalAlert => 4,
    }
}

pub fn plural(n: usize, one: &str, many: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {many}")
    }
}
