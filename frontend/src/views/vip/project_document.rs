//! Notion-style document pages for a zone and for a single project.
//!
//! 1. Executive header (serif title, status, progress, target deadline)
//! 2. Work streams: Block A capital developments / Block B maintenance
//! 3. Live manpower & leadership roster (one-tap call / message)
//! 4. Zone asset & machinery inventory
//! 5. Milestone & delay audit-trail timeline

use chrono::{DateTime, Local, Timelike, Utc};
use dioxus::prelude::*;
use shared::api::*;
use shared::*;
use shared::Asset;
use uuid::Uuid;

use crate::api;
use crate::components::photo_annotator::PhotoAnnotator;
use crate::components::ui::{Avatar, EmptyState, Icon, Pill, ProgressBar, SkeletonPage};
use crate::state::{use_app, AppState};
use crate::util::{self, FlatArea};
use crate::views::staff::progress_updater::{PhotoUploader, ProgressUpdater, RoutineCompleter, StatusPill};
use crate::Route;

// ------------------------------------------------------------------ data

pub async fn fetch_detail(app: AppState, id: Uuid) {
    if let Ok(d) = api::get::<WorkItemDetail>(&format!("/work-items/{id}")).await {
        let mut details = app.details;
        if details.peek().get(&id) != Some(&d) {
            details.write().insert(id, d);
        }
    }
}

/// Keeps one project's dossier fresh (re-fetches after any related change).
pub fn use_detail(id: Uuid) -> Option<WorkItemDetail> {
    let app = use_app();
    use_effect(move || {
        let _epoch = (app.detail_epoch)();
        let _who = app.current_user.read().as_ref().map(|u| u.id);
        spawn(async move { fetch_detail(app, id).await });
    });
    let found = app.details.read().get(&id).cloned();
    found
}

// ------------------------------------------------------------------ timeline

#[derive(Debug, Clone, PartialEq)]
pub enum EntryKind {
    Milestone,
    Delay(DelayCategory, u16),
    Crew(i32),
    Note,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineEntry {
    pub when: DateTime<Utc>,
    pub title: String,
    pub body: String,
    pub kind: EntryKind,
    pub project: Option<String>,
}

pub fn timeline_entries(detail: &WorkItemDetail, with_project: bool) -> Vec<TimelineEntry> {
    let project = with_project.then(|| detail.item.title.clone());
    let mut out: Vec<TimelineEntry> = detail
        .timeline
        .iter()
        .map(|t| TimelineEntry {
            when: t.timestamp,
            title: t.title.clone(),
            body: t.description.clone(),
            kind: if t.days_delayed > 0 {
                EntryKind::Delay(t.delay_category, t.days_delayed)
            } else if t.is_milestone {
                EntryKind::Milestone
            } else {
                EntryKind::Note
            },
            project: project.clone(),
        })
        .collect();
    for l in &detail.ledger {
        if l.headcount_delta == 0 || l.reason.starts_with("Midnight reset") {
            continue;
        }
        let sign = if l.headcount_delta > 0 { "+" } else { "" };
        out.push(TimelineEntry {
            when: l.timestamp,
            title: format!("{sign}{} workers · now {}", l.headcount_delta, l.resulting_headcount),
            body: l.reason.clone(),
            kind: EntryKind::Crew(l.headcount_delta),
            project: project.clone(),
        });
    }
    out
}

#[component]
pub fn TimelineBlock(mut entries: Vec<TimelineEntry>) -> Element {
    entries.sort_by(|a, b| b.when.cmp(&a.when));
    if entries.is_empty() {
        return rsx! { p { class: "text-sm text-muted", "Nothing has been logged yet." } };
    }
    rsx! {
        ol { class: "relative pl-6", "data-testid": "timeline",
            div { class: "absolute left-[9px] top-2 bottom-2 w-px bg-line" }
            for (i, e) in entries.iter().enumerate() {
                li { key: "{i}", class: "relative pb-5 last:pb-0",
                    span {
                        class: match e.kind {
                            EntryKind::Milestone => "absolute -left-6 top-0.5 w-[19px] h-[19px] rounded-full bg-tag-green-bg text-tag-green-text flex items-center justify-center ring-4 ring-white",
                            EntryKind::Delay(..) => "absolute -left-6 top-0.5 w-[19px] h-[19px] rounded-full bg-tag-amber-bg text-tag-amber-text flex items-center justify-center ring-4 ring-white",
                            EntryKind::Crew(_) => "absolute -left-6 top-0.5 w-[19px] h-[19px] rounded-full bg-tag-blue-bg text-tag-blue-text flex items-center justify-center ring-4 ring-white",
                            EntryKind::Note => "absolute -left-6 top-0.5 w-[19px] h-[19px] rounded-full bg-hover text-muted flex items-center justify-center ring-4 ring-white",
                        },
                        Icon {
                            name: match e.kind {
                                EntryKind::Milestone => "check",
                                EntryKind::Delay(..) => "clock",
                                EntryKind::Crew(_) => "users",
                                EntryKind::Note => "info",
                            },
                            size: 11,
                        }
                    }
                    div { class: "flex flex-wrap items-center gap-x-2 gap-y-1",
                        p { class: "text-[15px] font-medium", "{e.title}" }
                        if let EntryKind::Delay(cat, days) = e.kind {
                            Pill { tone: Tone::Amber, text: format!("{} · +{}", cat.label(), util::plural(days as usize, "day")) }
                        }
                    }
                    if !e.body.is_empty() {
                        p { class: "text-sm text-muted mt-0.5 leading-relaxed", "{e.body}" }
                    }
                    p { class: "text-xs text-muted/80 mt-1",
                        "{util::day_time(e.when)}"
                        if let Some(p) = e.project.clone() { " · {p}" }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ manpower & leaders

/// "(+6 transferred from Guest House this morning to accelerate leveling)"
pub fn transfer_note(details: &[WorkItemDetail], items: &[WorkItem]) -> Option<String> {
    let mut latest: Option<&ManpowerLedgerEntry> = None;
    for d in details {
        for l in &d.ledger {
            if l.source_work_item_id.is_some() && l.headcount_delta > 0 && util::same_local_day(l.timestamp, Utc::now()) {
                if latest.map(|x| l.timestamp > x.timestamp).unwrap_or(true) {
                    latest = Some(l);
                }
            }
        }
    }
    let l = latest?;
    let source = items
        .iter()
        .find(|w| Some(w.id) == l.source_work_item_id)
        .map(|w| w.title.clone())
        .unwrap_or_else(|| "another job".into());
    let when = if util::local(l.timestamp).hour() < 12 { "this morning" } else { "today" };
    let reason = l.reason.trim();
    let mut reason_text = reason.to_string();
    if let Some(first) = reason_text.get(0..1) {
        reason_text = format!("{}{}", first.to_lowercase(), &reason_text[1..]);
    }
    Some(format!("+{} transferred from {source} {when} — {reason_text}", l.headcount_delta))
}

#[component]
pub fn LeadershipBlock(personnel: ProjectPersonnel, headcount: u32, note: Option<String>) -> Element {
    let leaders = [
        ("Construction Group Leader", personnel.group_leader_name.clone(), personnel.group_leader_phone.clone()),
        ("On-Site Quality Control (QC) Officer", personnel.site_qc_officer_name.clone(), personnel.site_qc_officer_phone.clone()),
        ("Admin Office Coordinator", personnel.admin_coordinator_name.clone(), personnel.admin_coordinator_phone.clone()),
    ];
    rsx! {
        div { class: "space-y-4", "data-testid": "manpower-block",
            div { class: "flex flex-wrap items-baseline gap-x-3 gap-y-1",
                p { class: "text-2xl font-semibold tabular", "data-testid": "headcount", "{headcount} Active Personnel" }
                if let Some(n) = note {
                    span { class: "pill bg-tag-blue-bg text-tag-blue-text whitespace-normal", "({n})" }
                }
            }
            ul { class: "divide-y divide-line border-y border-line",
                for (role, name, phone) in leaders {
                    li { key: "{role}", class: "flex items-center gap-3 py-3",
                        Avatar { name: name.clone(), size: 34 }
                        div { class: "flex-1 min-w-0",
                            p { class: "text-[15px] font-medium truncate", "{name}" }
                            p { class: "text-xs text-muted", "{role}" }
                        }
                        a { href: "{util::tel(&phone)}", class: "btn-secondary h-10 w-10 px-0 sm:w-auto sm:px-3", "aria-label": "Call {name}",
                            Icon { name: "phone" }
                            span { class: "hidden sm:inline", "Call" }
                        }
                        a { href: "{util::sms(&phone)}", class: "btn-secondary h-10 w-10 px-0 sm:w-auto sm:px-3", "aria-label": "Message {name}",
                            Icon { name: "message" }
                            span { class: "hidden sm:inline", "Message" }
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ assets

#[component]
pub fn AssetBlock(assets: Vec<Asset>) -> Element {
    let fixed: Vec<Asset> = assets.iter().filter(|a| a.mobility != MobilityType::MobileMachinery && a.parent_asset_id.is_none()).cloned().collect();
    let machines: Vec<Asset> = assets.iter().filter(|a| a.mobility == MobilityType::MobileMachinery).cloned().collect();
    let children = assets.clone();
    if assets.is_empty() {
        return rsx! { p { class: "text-sm text-muted", "No equipment is recorded in this zone." } };
    }
    rsx! {
        div { class: "grid md:grid-cols-2 gap-6", "data-testid": "asset-block",
            div {
                p { class: "eyebrow mb-2", "Fixed Infrastructure" }
                if fixed.is_empty() {
                    p { class: "text-sm text-muted", "None in this zone." }
                }
                ul { class: "space-y-1",
                    for a in fixed {
                        AssetLine { key: "{a.id}", asset: a.clone(), nested: false }
                        for c in children.iter().filter(|c| c.parent_asset_id == Some(a.id)).cloned() {
                            AssetLine { key: "{c.id}", asset: c, nested: true }
                        }
                    }
                }
            }
            div {
                p { class: "eyebrow mb-2", "Deployed Heavy Machinery" }
                if machines.is_empty() {
                    p { class: "text-sm text-muted", "No machinery here right now." }
                }
                ul { class: "space-y-1",
                    for a in machines {
                        AssetLine { key: "{a.id}", asset: a, nested: false }
                    }
                }
            }
        }
    }
}

#[component]
fn AssetLine(asset: Asset, nested: bool) -> Element {
    rsx! {
        li { class: if nested { "flex items-start gap-2 py-1.5 pl-6 text-sm" } else { "flex items-start gap-2 py-1.5" },
            span { class: "text-muted mt-0.5", Icon { name: match asset.mobility { MobilityType::ModularComponent => "battery", MobilityType::MobileMachinery => "truck", MobilityType::FixedInfrastructure => "wrench" } } }
            div { class: "flex-1 min-w-0",
                p { class: "text-[15px]", "{asset.name}" }
                if asset.health != AssetHealth::Operational {
                    if let Some(eta) = asset.repair_eta {
                        p { class: "text-xs text-muted", "Expected back in service {util::day(eta)}" }
                    }
                }
            }
            Pill { tone: asset.health.tone(), text: asset.health.label().to_string() }
        }
    }
}

// ------------------------------------------------------------------ shared pieces

#[component]
fn Breadcrumbs(flat: Vec<FlatArea>, area_id: Uuid, #[props(default)] tail: Option<String>) -> Element {
    let path = util::path_to(&flat, area_id);
    let mut parent = use_app().active_parent_area;
    rsx! {
        nav { class: "flex flex-wrap items-center gap-1 text-sm text-muted mb-6", "aria-label": "Breadcrumb",
            Link { to: Route::Home {}, class: "hover:text-ink transition-colors", onclick: move |_| parent.set(None), "Overview" }
            for a in path {
                span { key: "{a.area.id}", class: "contents",
                    Icon { name: "chevron-right", size: 12, class: "text-muted/60" }
                    Link { to: Route::Zone { id: a.area.id }, class: "hover:text-ink transition-colors", "{a.area.name}" }
                }
            }
            if let Some(t) = tail {
                Icon { name: "chevron-right", size: 12, class: "text-muted/60" }
                span { class: "text-ink", "{t}" }
            }
        }
    }
}

#[component]
fn DocSection(#[props(into)] title: String, #[props(default, into)] subtitle: String, children: Element) -> Element {
    rsx! {
        section { class: "pt-8 mt-8 border-t border-line",
            h2 { class: "section-title", "{title}" }
            if !subtitle.is_empty() {
                p { class: "text-sm text-muted mt-0.5", "{subtitle}" }
            }
            div { class: "mt-4", {children} }
        }
    }
}

fn deadline_note(item: &WorkItem) -> Option<String> {
    let shift = (item.current_target_deadline - item.original_deadline).num_days();
    if shift > 0 {
        Some(format!("moved from {} (+{})", util::day(item.original_deadline), util::plural(shift as usize, "day")))
    } else if shift < 0 {
        Some(format!("brought forward from {}", util::day(item.original_deadline)))
    } else {
        None
    }
}

/// Block A card: capital project with weighted phase breakdown.
#[component]
pub fn CapitalCard(item: WorkItem, phases: Vec<ProjectPhase>) -> Element {
    let progress = item.calculated_progress;
    rsx! {
        Link { to: Route::Project { id: item.id }, class: "block card card-hover p-4", "data-testid": "capital-card",
            div { class: "flex flex-wrap items-start justify-between gap-2",
                p { class: "font-semibold text-[15px]", "{item.title}" }
                StatusPill { status: item.status }
            }
            div { class: "flex items-center gap-3 mt-3",
                div { class: "flex-1", ProgressBar { value: progress } }
                span { class: "text-sm tabular text-muted w-10 text-right", "{progress}%" }
            }
            if !phases.is_empty() {
                div { class: "flex flex-wrap gap-x-4 gap-y-1 mt-3 text-xs text-muted",
                    for p in phases.iter() {
                        span { key: "{p.id}", class: if p.is_completed { "text-tag-green-text" } else { "" },
                            if p.is_completed { "✓ " } else { "○ " }
                            "{p.phase_name} ({p.weight_percentage}%)"
                        }
                    }
                }
            }
            p { class: "text-xs text-muted mt-3", "Target: {util::day(item.current_target_deadline)} · {item.active_worker_count} on site today" }
        }
    }
}

/// Block B line: "✓ Pool Maintenance — Completed 8:00 AM".
#[component]
pub fn MaintenanceLine(item: WorkItem, detail: Option<WorkItemDetail>) -> Element {
    let app = use_app();
    let today_done = detail.as_ref().and_then(|d| {
        d.timeline.iter().rev().find(|t| t.is_milestone && util::same_local_day(t.timestamp, Utc::now())).cloned()
    });
    let latest_note = detail.as_ref().and_then(|d| d.timeline.iter().rev().find(|t| !t.is_milestone).cloned());
    let next_slot = app.agenda.read().as_ref().and_then(|a| {
        a.items
            .iter()
            .filter(|i| i.work_item_id == Some(item.id) && i.start_time > Utc::now())
            .min_by_key(|i| i.start_time)
            .map(|i| i.start_time)
    });
    let (icon_class, icon, text) = match (item.stream, item.status) {
        (_, WorkStatus::CompletedAndArchived) => ("text-tag-green-text", "✓", format!("{} — Completed", item.title)),
        (WorkStream::RoutineMaintenance, _) if today_done.is_some() => {
            let at = today_done.as_ref().map(|t| util::time(t.timestamp)).unwrap_or_default();
            ("text-tag-green-text", "✓", format!("{} — Completed {at}", item.title))
        }
        (WorkStream::RoutineMaintenance, _) => (
            "text-muted",
            "○",
            match next_slot {
                Some(t) => format!("{} — Next: {}", item.title, util::day_time(t)),
                None => format!("{} — {}", item.title, item.status.label()),
            },
        ),
        (_, _) => (
            "text-tag-amber-text",
            "⚠️",
            format!(
                "{} — {}",
                item.title,
                latest_note.map(|t| t.title).unwrap_or_else(|| item.status.label().to_string())
            ),
        ),
    };
    rsx! {
        Link { to: Route::Project { id: item.id }, class: "flex items-center gap-3 py-2.5 px-2 -mx-2 rounded-md hover:bg-hover transition-colors", "data-testid": "maintenance-line",
            span { class: "w-5 text-center {icon_class}", "{icon}" }
            span { class: "flex-1 text-[15px]", "{text}" }
            Icon { name: "chevron-right", size: 14, class: "text-muted/60" }
        }
    }
}

// ------------------------------------------------------------------ zone page

#[component]
pub fn ZonePage(id: Uuid) -> Element {
    let app = use_app();

    // Fetch the dossier of every project in this zone (and its sub-zones).
    use_effect(move || {
        let _epoch = (app.detail_epoch)();
        let flat = app.flat_areas();
        let zone = util::subtree(&flat, id);
        let ids: Vec<Uuid> = app
            .work_items
            .read()
            .as_ref()
            .map(|w| w.iter().filter(|w| zone.contains(&w.area_id)).map(|w| w.id).collect())
            .unwrap_or_default();
        for wid in ids {
            spawn(async move { fetch_detail(app, wid).await });
        }
    });

    let flat = app.flat_areas();
    let Some(area) = util::find(&flat, id).cloned() else {
        if app.map.read().is_none() {
            return rsx! { SkeletonPage {} };
        }
        return rsx! { EmptyState { icon: "map", title: "This zone no longer exists" } };
    };
    let zone = util::subtree(&flat, id);
    let items: Vec<WorkItem> = app.work_items.read().clone().unwrap_or_default();
    let in_zone: Vec<WorkItem> = items.iter().filter(|w| zone.contains(&w.area_id)).cloned().collect();
    let details_map = app.details.read().clone();
    let details: Vec<WorkItemDetail> = in_zone.iter().filter_map(|w| details_map.get(&w.id).cloned()).collect();

    let capital: Vec<WorkItem> = in_zone
        .iter()
        .filter(|w| w.stream == WorkStream::CapitalProject && w.status.is_active())
        .cloned()
        .collect();
    let maintenance: Vec<WorkItem> = in_zone
        .iter()
        .filter(|w| w.stream != WorkStream::CapitalProject && (w.status.is_active() || util::same_local_day(w.current_target_deadline, Utc::now())))
        .cloned()
        .collect();
    let archived: Vec<WorkItem> = in_zone.iter().filter(|w| w.status == WorkStatus::CompletedAndArchived).cloned().collect();
    let headcount: u32 = in_zone.iter().filter(|w| w.status.is_active()).map(|w| w.active_worker_count).sum();
    let note = transfer_note(&details, &items);
    let lead = in_zone
        .iter()
        .filter(|w| w.status.is_active())
        .max_by_key(|w| (w.active_worker_count, w.stream == WorkStream::CapitalProject))
        .or_else(|| in_zone.first())
        .map(|w| w.personnel.clone());
    let zone_assets: Vec<Asset> = app
        .assets
        .read()
        .as_ref()
        .map(|a| a.assets.iter().filter(|x| zone.contains(&x.current_area_id)).cloned().collect())
        .unwrap_or_default();
    let mut entries: Vec<TimelineEntry> = details.iter().flat_map(|d| timeline_entries(d, true)).collect();
    entries.truncate(400);
    let sub_zones: Vec<FlatArea> = area.child_ids.iter().filter_map(|c| util::find(&flat, *c).cloned()).filter(|a| !a.area.is_archived).collect();
    let mut equipment_zone = app.equipment_zone;
    let nav = use_navigator();

    rsx! {
        article { class: "max-w-doc mx-auto rise-in", "data-testid": "zone-page",
            Breadcrumbs { flat: flat.clone(), area_id: id }
            header { class: "space-y-3",
                p { class: "eyebrow", "{area.area.zone_class.label()}" }
                h1 { class: "doc-title", "{area.area.name}" }
                div { class: "flex flex-wrap items-center gap-2",
                    Pill { tone: area.status.tone(), text: area.status.label().to_string(), dot: true }
                    span { class: "text-sm text-muted", "{area.status_reason}" }
                    if area.area.is_archived {
                        Pill { tone: Tone::Gray, text: "Archived Locations" }
                    }
                }
                if let Some(summary) = area.executive_summary.clone() {
                    div { class: "rounded-lg bg-sidebar px-4 py-3 mt-2",
                        p { class: "eyebrow mb-1", "Executive summary" }
                        p { class: "text-[15px] leading-relaxed", "{summary}" }
                    }
                }
                if app.is_staff() {
                    div { class: "flex flex-wrap gap-2 pt-1",
                        button {
                            class: "btn-secondary",
                            onclick: move |_| {
                                equipment_zone.set(Some(id));
                                nav.push(Route::Equipment {});
                            },
                            Icon { name: "truck" }
                            "Move Equipment"
                        }
                        Link { to: Route::Decisions {}, class: "btn-secondary", Icon { name: "flag" } "Report an issue" }
                    }
                }
            }

            if !sub_zones.is_empty() {
                div { class: "flex flex-wrap gap-2 mt-6",
                    for s in sub_zones {
                        Link { key: "{s.area.id}", to: Route::Zone { id: s.area.id }, class: "pill bg-hover text-ink hover:bg-[#e8e8e5] transition-colors h-7 px-2.5",
                            span { class: "w-1.5 h-1.5 rounded-full", style: "background: currentColor; opacity:.4" }
                            "{s.area.name}"
                        }
                    }
                }
            }

            DocSection { title: "Active Capital Developments", subtitle: "Construction projects with their weighted phases.",
                if capital.is_empty() {
                    p { class: "text-sm text-muted", "No construction is underway here." }
                } else {
                    div { class: "space-y-3",
                        for w in capital {
                            CapitalCard { key: "{w.id}", item: w.clone(), phases: details_map.get(&w.id).map(|d| d.phases.clone()).unwrap_or_default() }
                        }
                    }
                }
            }

            DocSection { title: "Facility Maintenance & Repairs",
                if maintenance.is_empty() {
                    p { class: "text-sm text-muted", "No upkeep or repairs scheduled here." }
                } else {
                    div { class: "divide-y divide-line",
                        for w in maintenance {
                            MaintenanceLine { key: "{w.id}", item: w.clone(), detail: details_map.get(&w.id).cloned() }
                        }
                    }
                }
            }

            DocSection { title: "Live Manpower & Leadership", subtitle: "Today's crew on site and the people accountable.",
                if let Some(p) = lead {
                    LeadershipBlock { personnel: p, headcount, note }
                } else {
                    p { class: "text-sm text-muted", "No crew assigned here today." }
                }
            }

            DocSection { title: "Zone Assets & Machinery",
                AssetBlock { assets: zone_assets }
            }

            DocSection { title: "Milestones & Delay Audit Trail",
                TimelineBlock { entries }
            }

            if !archived.is_empty() {
                DocSection { title: "Completed & archived",
                    ul { class: "divide-y divide-line",
                        for w in archived {
                            li { key: "{w.id}",
                                Link { to: Route::Project { id: w.id }, class: "flex items-center gap-3 py-2.5 hover:bg-hover px-2 -mx-2 rounded-md transition-colors",
                                    span { class: "text-tag-green-text", "✓" }
                                    span { class: "flex-1", "{w.title}" }
                                    span { class: "text-xs text-muted", "{util::day(w.current_target_deadline)}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ project page

#[component]
pub fn ProjectPage(id: Uuid) -> Element {
    let app = use_app();
    let detail = use_detail(id);
    let mut photo_index = use_signal(|| 0usize);
    let Some(detail) = detail else {
        return rsx! { div { class: "max-w-doc mx-auto", SkeletonPage {} } };
    };
    let flat = app.flat_areas();
    let item = detail.item.clone();
    let items = app.work_items.read().clone().unwrap_or_default();
    let note = transfer_note(std::slice::from_ref(&detail), &items);
    let shift = deadline_note(&item);
    let is_staff = app.is_staff();
    let is_vip = app.is_vip();
    let entries = timeline_entries(&detail, false);
    let photos = detail.photos.clone();
    let idx = photo_index().min(photos.len().saturating_sub(1));
    let progress_tone = match item.status {
        WorkStatus::PendingSignOff => Some(Tone::Amber),
        WorkStatus::CompletedAndArchived => Some(Tone::Green),
        _ => None,
    };
    let completed_time = |t: DateTime<Utc>| t.with_timezone(&Local).format("%b %-d").to_string();

    rsx! {
        article { class: "max-w-doc mx-auto rise-in", "data-testid": "project-page",
            Breadcrumbs { flat: flat.clone(), area_id: item.area_id, tail: item.title.clone() }
            header { class: "space-y-4",
                p { class: "eyebrow", "{item.stream.label()}" }
                h1 { class: "doc-title", "data-testid": "project-title", "{item.title}" }
                div { class: "flex flex-wrap items-center gap-2",
                    StatusPill { status: item.status }
                    if detail.area.is_archived {
                        Pill { tone: Tone::Gray, text: "Archived Locations" }
                    }
                }
                div { class: "flex items-center gap-3 max-w-md",
                    div { class: "flex-1", ProgressBar { value: item.calculated_progress, tone: progress_tone } }
                    span { class: "text-sm font-medium tabular", "data-testid": "progress-value", "{item.calculated_progress}%" }
                }
                div { class: "text-[15px]",
                    span { class: "text-muted", "Current Target Deadline: " }
                    span { class: "font-medium", "{util::long_day(item.current_target_deadline)}" }
                    if let Some(s) = shift {
                        span { class: "text-sm text-muted", " — {s}" }
                    }
                }
                p { class: "text-[15px] leading-relaxed text-ink/90", "{item.description}" }
            }

            // ------------------------------------------- handover call-outs
            if item.status == WorkStatus::PendingSignOff {
                div { class: "mt-6 rounded-xl bg-tag-amber-bg px-5 py-4 flex flex-col sm:flex-row sm:items-center gap-3",
                    p { class: "flex-1 text-[15px] text-tag-amber-text",
                        if is_vip { "Every phase is finished. Please review the handover report to sign off." } else { "All phases are done. The owner is reviewing the handover report." }
                    }
                    Link { to: Route::Handover { id }, class: "btn-primary", "data-testid": "view-handover", "Open handover report" }
                }
            }
            if item.status == WorkStatus::CompletedAndArchived && detail.handover.is_some() {
                div { class: "mt-6 rounded-xl bg-tag-green-bg px-5 py-4 flex items-center gap-3",
                    p { class: "flex-1 text-[15px] text-tag-green-text", "Completed and signed off. The handover report is kept for reference." }
                    Link { to: Route::Handover { id }, class: "btn-secondary", "View report" }
                }
            }

            // ------------------------------------------- field tools (staff)
            if is_staff {
                section { class: "mt-8 space-y-3", "data-testid": "field-tools",
                    p { class: "eyebrow", "Field tools" }
                    if item.stream == WorkStream::CapitalProject {
                        ProgressUpdater { detail: detail.clone() }
                    } else {
                        RoutineCompleter { item: item.clone() }
                    }
                    div { class: "flex flex-wrap gap-2",
                        PhotoUploader { work_item_id: id }
                        Link { to: Route::Decisions {}, class: "btn-secondary", Icon { name: "flag" } "Report an issue" }
                        Link { to: Route::Equipment {}, class: "btn-secondary", Icon { name: "truck" } "Equipment" }
                    }
                }
            }

            // ------------------------------------------- phases (owner view)
            if item.stream == WorkStream::CapitalProject && !is_staff {
                DocSection { title: "Project phases", subtitle: "Progress is weighted by how much of the work each phase represents.",
                    ol { class: "divide-y divide-line border-y border-line",
                        for p in detail.phases.iter() {
                            li { key: "{p.id}", class: "flex items-center gap-3 py-2.5",
                                span { class: if p.is_completed { "text-tag-green-text w-5" } else { "text-muted w-5" }, if p.is_completed { "✓" } else { "○" } }
                                span { class: "flex-1 text-[15px]", "{p.order_index}. {p.phase_name} ({p.weight_percentage}%)" }
                                if let Some(at) = p.completed_at {
                                    span { class: "text-xs text-muted", "Done {completed_time(at)}" }
                                }
                            }
                        }
                    }
                }
            }

            // ------------------------------------------- photos
            DocSection { title: "Site photos", subtitle: if is_vip { "Tap a photo to point at anything and leave a note." } else { "Owner notes appear as pins — tap one to reply." },
                if photos.is_empty() {
                    p { class: "text-sm text-muted", "No photos yet." }
                } else {
                    div { class: "space-y-3",
                        PhotoAnnotator { key: "{photos[idx].photo.id}", photo: photos[idx].clone(), work_item_id: id }
                        if photos.len() > 1 {
                            div { class: "flex gap-2 overflow-x-auto no-scrollbar pb-1",
                                for (i, p) in photos.iter().enumerate() {
                                    button {
                                        key: "{p.photo.id}",
                                        class: if i == idx { "relative shrink-0 w-28 aspect-video rounded-md overflow-hidden ring-2 ring-ink" } else { "relative shrink-0 w-28 aspect-video rounded-md overflow-hidden opacity-75 hover:opacity-100 transition-opacity" },
                                        "aria-label": "Show photo: {p.photo.caption}",
                                        onclick: move |_| photo_index.set(i),
                                        img { src: "{p.photo.image_url}", alt: "", class: "w-full h-full object-cover", loading: "lazy" }
                                        if p.callouts.iter().any(|c| c.status != CallOutStatus::Resolved) {
                                            span { class: "absolute top-1 right-1 w-2.5 h-2.5 rounded-full bg-pin-amber ring-2 ring-white" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            DocSection { title: "Live Manpower & Leadership",
                LeadershipBlock { personnel: item.personnel.clone(), headcount: item.active_worker_count, note }
            }

            DocSection { title: "Zone Assets & Machinery",
                AssetBlock { assets: detail.zone_assets.clone() }
            }

            if !detail.decisions.is_empty() {
                DocSection { title: "Decisions & updates",
                    ul { class: "divide-y divide-line",
                        for d in detail.decisions.iter() {
                            li { key: "{d.id}", class: "py-3",
                                div { class: "flex flex-wrap items-center gap-2",
                                    p { class: "text-[15px] font-medium flex-1", "{d.title}" }
                                    Pill { tone: d.status.tone(), text: d.status.label().to_string() }
                                }
                                p { class: "text-sm text-muted mt-0.5", "{d.issue_description}" }
                            }
                        }
                    }
                }
            }

            DocSection { title: "Milestones & Delay Audit Trail",
                TimelineBlock { entries }
            }
        }
    }
}
