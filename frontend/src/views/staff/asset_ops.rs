//! Context-aware asset mover, component nesting & breakdown reporter.
//!
//! No QR codes: the screen remembers which zone you're standing in and
//! lists only the equipment checked into it, with a smart search fallback.

use chrono::{Duration, Utc};
use dioxus::prelude::*;
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use shared::Asset;
use uuid::Uuid;

use crate::components::ui::{EmptyState, Icon, Modal, Pill, Spinner};
use crate::services::sync::{self, Outcome};
use crate::state::use_app;
use crate::util::{self, FlatArea};

#[derive(Clone, Copy, PartialEq)]
enum Dialog {
    Move(Option<Uuid>),
    Breakdown(Option<Uuid>),
    Attach(Uuid),
}

#[component]
pub fn EquipmentView() -> Element {
    let app = use_app();
    let mut zone = app.equipment_zone;
    let mut dialog = use_signal(|| None::<Dialog>);
    let mut search = use_signal(String::new);

    // Field staff start in their first assigned zone.
    use_hook(move || {
        if zone.peek().is_none() {
            if let Some(u) = app.current_user.peek().as_ref() {
                if u.role == UserRole::SiteMonitor {
                    zone.set(u.assigned_area_ids.first().copied());
                }
            }
        }
    });

    let flat = app.flat_areas();
    let Some(data) = app.assets.read().clone() else {
        return rsx! { crate::components::ui::SkeletonPage {} };
    };
    let assets = data.assets.clone();
    let zone_id = zone();
    let scope: Vec<Uuid> = zone_id.map(|z| util::subtree(&flat, z)).unwrap_or_default();
    let here: Vec<Asset> = match zone_id {
        Some(_) => assets.iter().filter(|a| scope.contains(&a.current_area_id)).cloned().collect(),
        None => assets.clone(),
    };
    let q = search().to_lowercase();
    let elsewhere: Vec<Asset> = if q.is_empty() {
        vec![]
    } else {
        assets
            .iter()
            .filter(|a| !here.iter().any(|h| h.id == a.id))
            .filter(|a| a.name.to_lowercase().contains(&q) || util::area_name(&flat, a.current_area_id).to_lowercase().contains(&q))
            .cloned()
            .collect()
    };
    let zone_name = zone_id.map(|z| util::area_name(&flat, z)).unwrap_or_else(|| "the whole estate".into());
    let tops: Vec<Asset> = here.iter().filter(|a| a.parent_asset_id.map(|p| !here.iter().any(|h| h.id == p)).unwrap_or(true)).cloned().collect();
    let active_zones: Vec<FlatArea> = flat.iter().filter(|a| !a.area.is_archived).cloned().collect();

    rsx! {
        div { class: "space-y-6 rise-in",
            header { class: "space-y-1",
                h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Equipment" }
                p { class: "text-[15px] text-muted", "Machinery, built-in systems and swappable parts — where they are and how they're doing." }
            }

            div { class: "card p-4 space-y-4",
                div {
                    label { class: "label", r#for: "zone-context", "Where are you?" }
                    select {
                        id: "zone-context",
                        class: "field",
                        "data-testid": "zone-context",
                        onchange: move |e| zone.set(Uuid::parse_str(&e.value()).ok()),
                        option { value: "", selected: zone_id.is_none(), "Whole estate" }
                        for a in active_zones.iter() {
                            option { key: "{a.area.id}", value: "{a.area.id}", selected: zone_id == Some(a.area.id), "{\"\u{2003}\".repeat(a.depth)}{a.area.name}" }
                        }
                    }
                    p { class: "hint", "We'll show the equipment checked into this zone first." }
                }
                div { class: "grid grid-cols-2 gap-2",
                    button { class: "btn-primary btn-lg", "data-testid": "move-equipment", onclick: move |_| dialog.set(Some(Dialog::Move(None))), Icon { name: "truck", size: 18 } "Move Equipment" }
                    button { class: "btn-red btn-lg", "data-testid": "report-breakdown", onclick: move |_| dialog.set(Some(Dialog::Breakdown(None))), Icon { name: "alert", size: 18 } "Report Breakdown" }
                }
            }

            section {
                h2 { class: "section-title mb-3", "At {zone_name}" }
                if tops.is_empty() {
                    EmptyState { icon: "truck", title: "No equipment checked into this zone", body: "Use search below to find something elsewhere." }
                } else {
                    div { class: "space-y-2",
                        for a in tops {
                            AssetCard { key: "{a.id}", asset: a.clone(), parts: assets.iter().filter(|c| c.parent_asset_id == Some(a.id)).cloned().collect::<Vec<_>>(), on_action: move |d| dialog.set(Some(d)) }
                        }
                    }
                }
            }

            section {
                h2 { class: "section-title mb-3", "Find something elsewhere" }
                div { class: "relative",
                    span { class: "absolute left-3 top-1/2 -translate-y-1/2 text-muted", Icon { name: "search" } }
                    input { class: "field pl-9", placeholder: "Search by name or location, e.g. crane", "data-testid": "asset-search", value: "{search}", oninput: move |e| search.set(e.value()) }
                }
                if !q.is_empty() {
                    div { class: "space-y-2 mt-3",
                        if elsewhere.is_empty() {
                            p { class: "text-sm text-muted", "Nothing else matches \u{201c}{search}\u{201d}." }
                        }
                        for a in elsewhere {
                            AssetCard { key: "{a.id}", asset: a.clone(), parts: vec![], on_action: move |d| dialog.set(Some(d)) }
                        }
                    }
                }
            }

            section {
                h2 { class: "section-title mb-3", "Recent movements" }
                if data.recent_transfers.is_empty() {
                    p { class: "text-sm text-muted", "No equipment has moved yet." }
                } else {
                    ul { class: "divide-y divide-line border-y border-line",
                        for t in data.recent_transfers.iter().take(12) {
                            li { key: "{t.id}", class: "py-2.5 text-sm flex flex-wrap gap-x-2",
                                span { class: "font-medium", "{assets.iter().find(|a| a.id == t.asset_id).map(|a| a.name.clone()).unwrap_or_default()}" }
                                span { class: "text-muted",
                                    "moved "
                                    if let Some(f) = t.from_area_id { "from {util::area_name(&flat, f)} " }
                                    "to {util::area_name(&flat, t.to_area_id)}"
                                }
                                span { class: "text-muted/80 ml-auto", "{app.user_name(t.moved_by)} · {util::relative(t.timestamp)}" }
                            }
                        }
                    }
                }
            }
        }

        match dialog() {
            Some(Dialog::Move(pre)) => rsx! { MoveDialog { preselect: pre, here: here.clone(), all: assets.clone(), zones: active_zones.clone(), on_close: move |_| dialog.set(None) } },
            Some(Dialog::Breakdown(pre)) => rsx! { BreakdownDialog { preselect: pre, here: here.clone(), all: assets.clone(), zone: zone_id, on_close: move |_| dialog.set(None) } },
            Some(Dialog::Attach(id)) => rsx! { AttachDialog { asset_id: id, all: assets.clone(), on_close: move |_| dialog.set(None) } },
            None => rsx! {},
        }
    }
}

#[component]
fn AssetCard(asset: Asset, parts: Vec<Asset>, on_action: EventHandler<Dialog>) -> Element {
    let app = use_app();
    let flat = app.flat_areas();
    let id = asset.id;
    let mut busy = use_signal(|| false);
    let repaired = move |target: Uuid| async move {
        busy.set(true);
        let outcome: Outcome<Asset> = sync::mutate(app, "Equipment back in service", "POST", format!("/assets/{target}/mark-repaired"), &serde_json::json!({})).await;
        match outcome {
            Outcome::Saved(a) => {
                app.success(format!("{} is back in service", a.name));
                sync::reload(app, vec![DataScope::Assets, DataScope::Map, DataScope::WorkItems]);
            }
            Outcome::Queued => {}
            Outcome::Failed(e) => app.error(e),
        }
        busy.set(false);
    };
    let detach = move |target: Uuid| async move {
        let outcome: Outcome<Asset> = sync::mutate(app, "Detach part", "POST", format!("/assets/{target}/link"), &LinkAssetRequest { parent_asset_id: None }).await;
        match outcome {
            Outcome::Saved(a) => {
                app.success(format!("{} detached — you can move it now", a.name));
                sync::reload(app, vec![DataScope::Assets]);
            }
            Outcome::Queued => {}
            Outcome::Failed(e) => app.error(e),
        }
    };
    rsx! {
        div { class: "card overflow-hidden", "data-testid": "asset-card",
            div { class: "p-4 flex flex-col sm:flex-row sm:items-start gap-3",
                div { class: "flex-1 min-w-0",
                    div { class: "flex flex-wrap items-center gap-2",
                        p { class: "text-[15px] font-semibold", "{asset.name}" }
                        Pill { tone: asset.health.tone(), text: asset.health.label().to_string(), dot: true }
                        if asset.impact_level == ImpactLevel::FullWorkStoppage && asset.health != AssetHealth::Operational {
                            Pill { tone: Tone::Red, text: "Work stopped" }
                        }
                    }
                    p { class: "text-xs text-muted mt-0.5", "{asset.mobility.label()} · {util::area_name(&flat, asset.current_area_id)}" }
                    if let Some(issue) = asset.active_issue_report.clone() {
                        p { class: "text-sm mt-2", "{issue}" }
                    }
                    if let Some(eta) = asset.repair_eta {
                        p { class: "text-xs text-muted mt-1", "Repair expected {util::day(eta)}" }
                    }
                }
                div { class: "flex flex-wrap gap-2",
                    if asset.parent_asset_id.is_none() {
                        button { class: "btn-secondary h-10", onclick: move |_| on_action.call(Dialog::Move(Some(id))), Icon { name: "truck" } "Move" }
                    }
                    if asset.health == AssetHealth::Operational {
                        button { class: "btn-secondary h-10", onclick: move |_| on_action.call(Dialog::Breakdown(Some(id))), Icon { name: "alert" } "Report issue" }
                    } else {
                        button { class: "btn-green h-10", disabled: busy(), "data-testid": "mark-repaired", onclick: move |_| repaired(id), Icon { name: "check" } "Mark repaired" }
                    }
                    if asset.parent_asset_id.is_some() {
                        button { class: "btn-secondary h-10", "data-testid": "detach", onclick: move |_| detach(id), Icon { name: "unlink" } "Detach" }
                    } else if asset.mobility == MobilityType::ModularComponent {
                        button { class: "btn-secondary h-10", onclick: move |_| on_action.call(Dialog::Attach(id)), Icon { name: "link" } "Install in…" }
                    }
                }
            }
            if !parts.is_empty() {
                div { class: "border-t border-line bg-sidebar/60 divide-y divide-line",
                    for c in parts {
                        {
                            let cid = c.id;
                            rsx! {
                                div { key: "{cid}", class: "pl-8 pr-4 py-2.5 flex flex-wrap items-center gap-2 text-sm", "data-testid": "child-asset",
                                    Icon { name: "battery", class: "text-muted" }
                                    span { class: "flex-1 min-w-[10rem]", "{c.name}" }
                                    Pill { tone: c.health.tone(), text: c.health.label().to_string() }
                                    if c.health == AssetHealth::Operational {
                                        button { class: "btn-ghost h-8", onclick: move |_| on_action.call(Dialog::Breakdown(Some(cid))), "Report issue" }
                                    } else {
                                        button { class: "btn-ghost h-8", onclick: move |_| repaired(cid), "Mark repaired" }
                                    }
                                    button { class: "btn-secondary h-8", "data-testid": "detach-child", onclick: move |_| detach(cid), Icon { name: "unlink", size: 14 } "Detach" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Asset picker: this zone's equipment first, plus a search fallback.
#[component]
fn AssetPicker(here: Vec<Asset>, all: Vec<Asset>, selected: Signal<Option<Uuid>>, #[props(default)] only_movable: bool) -> Element {
    let mut query = use_signal(String::new);
    let mut selected = selected;
    let q = query().to_lowercase();
    let filter = |a: &&Asset| !only_movable || a.parent_asset_id.is_none();
    let list: Vec<Asset> = if q.is_empty() {
        here.iter().filter(filter).cloned().collect()
    } else {
        all.iter().filter(filter).filter(|a| a.name.to_lowercase().contains(&q)).cloned().collect()
    };
    rsx! {
        div { class: "space-y-2",
            select {
                class: "field",
                "data-testid": "asset-picker",
                onchange: move |e| selected.set(Uuid::parse_str(&e.value()).ok()),
                option { value: "", selected: selected().is_none(), if q.is_empty() { "Equipment in this zone…" } else { "Search results…" } }
                for a in list.iter() {
                    option { key: "{a.id}", value: "{a.id}", selected: selected() == Some(a.id), "{a.name}" }
                }
            }
            input { class: "field", placeholder: "Not listed? Search all equipment", value: "{query}", oninput: move |e| query.set(e.value()) }
        }
    }
}

#[component]
fn MoveDialog(preselect: Option<Uuid>, here: Vec<Asset>, all: Vec<Asset>, zones: Vec<FlatArea>, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let selected = use_signal(|| preselect);
    let mut dest = use_signal(|| None::<Uuid>);
    let mut busy = use_signal(|| false);
    let current_area = selected().and_then(|id| all.iter().find(|a| a.id == id)).map(|a| a.current_area_id);
    let child_count = selected().map(|id| all.iter().filter(|a| a.parent_asset_id == Some(id)).count()).unwrap_or(0);
    rsx! {
        Modal { title: "Move equipment", on_close: move |_| on_close.call(()),
            div { class: "space-y-5",
                div {
                    p { class: "label", "Which equipment?" }
                    AssetPicker { here: here.clone(), all: all.clone(), selected, only_movable: true }
                    if child_count > 0 {
                        p { class: "hint", "Its {util::plural(child_count, \"installed part\")} will move with it." }
                    }
                }
                div {
                    label { class: "label", r#for: "move-dest", "Move to" }
                    select { id: "move-dest", class: "field", "data-testid": "move-dest", onchange: move |e| dest.set(Uuid::parse_str(&e.value()).ok()),
                        option { value: "", selected: dest().is_none(), "Choose a location…" }
                        for z in zones.iter().filter(|z| Some(z.area.id) != current_area) {
                            option { key: "{z.area.id}", value: "{z.area.id}", selected: dest() == Some(z.area.id), "{\"\u{2003}\".repeat(z.depth)}{z.area.name}" }
                        }
                    }
                }
                div { class: "flex justify-end gap-2",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button {
                        class: "btn-primary btn-lg",
                        disabled: busy() || selected().is_none() || dest().is_none(),
                        "data-testid": "confirm-move",
                        onclick: move |_| async move {
                            let (Some(a), Some(d)) = (selected(), dest()) else { return };
                            busy.set(true);
                            let outcome: Outcome<Asset> = sync::mutate(app, "Move equipment", "POST", format!("/assets/{a}/relocate"), &RelocateAssetRequest { to_area_id: d }).await;
                            match outcome {
                                Outcome::Saved(asset) => {
                                    let flat = app.flat_areas();
                                    app.success(format!("{} moved to {}", asset.name, util::area_name(&flat, d)));
                                    sync::reload(app, vec![DataScope::Assets, DataScope::Map]);
                                    on_close.call(());
                                }
                                Outcome::Queued => on_close.call(()),
                                Outcome::Failed(e) => app.error(e),
                            }
                            busy.set(false);
                        },
                        if busy() { Spinner {} }
                        "Move"
                    }
                }
            }
        }
    }
}

#[component]
fn BreakdownDialog(preselect: Option<Uuid>, here: Vec<Asset>, all: Vec<Asset>, zone: Option<Uuid>, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let selected = use_signal(|| preselect);
    let mut health = use_signal(|| AssetHealth::BrokenDown);
    let mut impact = use_signal(|| ImpactLevel::MinorDelay);
    let mut note = use_signal(String::new);
    let mut eta = use_signal(|| util::to_date_input(Utc::now() + Duration::days(2)));
    let mut affects_deadline = use_signal(|| false);
    let mut days = use_signal(|| 1u16);
    let mut project = use_signal(|| None::<Uuid>);
    let mut busy = use_signal(|| false);

    let flat = app.flat_areas();
    let asset_area = selected().and_then(|id| all.iter().find(|a| a.id == id)).map(|a| a.current_area_id).or(zone);
    let items: Vec<WorkItem> = app.work_items.read().clone().unwrap_or_default().into_iter().filter(|w| w.status.is_active()).collect();
    let nearby: Vec<Uuid> = asset_area
        .map(|a| {
            // Jobs in the asset's zone, its parents and its sub-zones.
            let mut ids = util::subtree(&flat, a);
            ids.extend(util::path_to(&flat, a).iter().map(|p| p.area.id));
            ids
        })
        .unwrap_or_default();
    let mut sorted = items.clone();
    sorted.sort_by_key(|w| !nearby.contains(&w.area_id));
    // Default the affected project to the closest one.
    use_effect(move || {
        let _ = selected();
        if project.peek().is_none() {
            let flat = app.flat_areas();
            let area = selected().and_then(|id| app.assets.peek().as_ref().and_then(|d| d.assets.iter().find(|a| a.id == id).map(|a| a.current_area_id)));
            if let Some(area) = area {
                let near = util::subtree(&flat, area);
                let first = app.work_items.peek().as_ref().and_then(|w| w.iter().find(|w| w.status.is_active() && near.contains(&w.area_id)).map(|w| w.id));
                if first.is_some() {
                    project.set(first);
                }
            }
        }
    });

    let can_submit = selected().is_some() && !note().trim().is_empty() && (!affects_deadline() || project().is_some());

    rsx! {
        Modal { title: "Report a breakdown", on_close: move |_| on_close.call(()),
            div { class: "space-y-5", "data-testid": "breakdown-form",
                div {
                    p { class: "label", "Which equipment?" }
                    AssetPicker { here: here.clone(), all: all.clone(), selected }
                }
                div {
                    p { class: "label", "1. How bad is it?" }
                    div { class: "grid grid-cols-2 gap-2",
                        ChoiceButton { active: health() == AssetHealth::NeedsMaintenance, label: "Needs service", hint: "Still usable", onclick: move |_| health.set(AssetHealth::NeedsMaintenance) }
                        ChoiceButton { active: health() == AssetHealth::BrokenDown, label: "Broken down", hint: "Can't be used", onclick: move |_| health.set(AssetHealth::BrokenDown) }
                    }
                }
                div {
                    p { class: "label", "2. Effect on the work" }
                    div { class: "grid grid-cols-1 sm:grid-cols-3 gap-2",
                        for level in ImpactLevel::ALL {
                            ChoiceButton {
                                key: "{level:?}",
                                active: impact() == level,
                                label: level.label(),
                                hint: match level { ImpactLevel::NoImpact => "Work carries on", ImpactLevel::MinorDelay => "Slower than planned", ImpactLevel::FullWorkStoppage => "The owner is told now" },
                                onclick: move |_| impact.set(level),
                            }
                        }
                    }
                }
                div {
                    label { class: "label", r#for: "diag-note", "3. Diagnostic & Mitigation Note" }
                    textarea { id: "diag-note", class: "field", rows: "3", "data-testid": "diag-note", placeholder: "What's wrong, and what have you done so far?", value: "{note}", oninput: move |e| note.set(e.value()) }
                }
                div {
                    label { class: "label", r#for: "repair-eta", "Repair ETA" }
                    input { id: "repair-eta", r#type: "date", class: "field", value: "{eta}", oninput: move |e| eta.set(e.value()) }
                }
                div { class: "rounded-lg bg-sidebar p-4 space-y-3",
                    p { class: "text-[15px] font-medium", "Does this impact the project's target deadline?" }
                    div { class: "grid grid-cols-2 gap-2",
                        ChoiceButton { active: !affects_deadline(), label: "No", hint: "", onclick: move |_| affects_deadline.set(false) }
                        ChoiceButton { active: affects_deadline(), label: "Yes", hint: "", onclick: move |_| affects_deadline.set(true) }
                    }
                    if affects_deadline() || impact() == ImpactLevel::FullWorkStoppage {
                        div { class: "space-y-3 rise-in",
                            div {
                                label { class: "label", r#for: "affected-project", "Which project?" }
                                select { id: "affected-project", class: "field", "data-testid": "affected-project", onchange: move |e| project.set(Uuid::parse_str(&e.value()).ok()),
                                    option { value: "", selected: project().is_none(), "Choose a project…" }
                                    for w in sorted.iter() {
                                        option { key: "{w.id}", value: "{w.id}", selected: project() == Some(w.id), "{w.title}" }
                                    }
                                }
                            }
                            if affects_deadline() {
                                div {
                                    p { class: "label", "How many days of delay?" }
                                    div { class: "flex items-center gap-2",
                                        button { class: "btn-secondary h-11 w-12 px-0", disabled: days() <= 1, onclick: move |_| days.set(days().saturating_sub(1).max(1)), Icon { name: "minus" } }
                                        span { class: "w-14 text-center text-2xl font-semibold tabular", "data-testid": "delay-days", "{days}" }
                                        button { class: "btn-secondary h-11 w-12 px-0", "data-testid": "delay-plus", onclick: move |_| days.set(days() + 1), Icon { name: "plus" } }
                                    }
                                    p { class: "hint", "The target deadline moves automatically and the delay is logged as Equipment Failure." }
                                }
                            }
                        }
                    }
                }
                div { class: "flex justify-end gap-2",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button {
                        class: "btn-red btn-lg",
                        disabled: !can_submit || busy(),
                        "data-testid": "submit-breakdown",
                        onclick: move |_| async move {
                            let Some(asset_id) = selected() else { return };
                            busy.set(true);
                            let repair_eta = util::from_input(&format!("{}T16:00", eta()));
                            let affected = if affects_deadline() || impact() == ImpactLevel::FullWorkStoppage { project() } else { None };
                            let req = ReportIssueRequest {
                                health: health(),
                                impact: impact(),
                                note: note(),
                                repair_eta,
                                work_item_id: affected,
                                days_delayed: if affects_deadline() { Some(days()) } else { None },
                            };
                            let outcome: Outcome<Asset> = sync::mutate(app, "Breakdown report", "POST", format!("/assets/{asset_id}/report-issue"), &req).await;
                            match outcome {
                                Outcome::Saved(a) => {
                                    app.success(format!("{} reported — the team has been notified", a.name));
                                    sync::reload(app, vec![DataScope::Assets, DataScope::Map, DataScope::WorkItems, DataScope::Notifications]);
                                    on_close.call(());
                                }
                                Outcome::Queued => on_close.call(()),
                                Outcome::Failed(e) => app.error(e),
                            }
                            busy.set(false);
                        },
                        if busy() { Spinner {} }
                        "Send report"
                    }
                }
            }
        }
    }
}

#[component]
fn AttachDialog(asset_id: Uuid, all: Vec<Asset>, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let flat = app.flat_areas();
    let mut parent = use_signal(|| None::<Uuid>);
    let mut busy = use_signal(|| false);
    let part = all.iter().find(|a| a.id == asset_id).cloned();
    let hosts: Vec<Asset> = all.iter().filter(|a| a.id != asset_id && a.mobility != MobilityType::ModularComponent).cloned().collect();
    rsx! {
        Modal { title: "Install part", on_close: move |_| on_close.call(()),
            div { class: "space-y-5",
                if let Some(p) = part {
                    p { class: "text-[15px]", "Install ", span { class: "font-medium", "{p.name}" }, " inside:" }
                }
                select { class: "field", "data-testid": "attach-parent", onchange: move |e| parent.set(Uuid::parse_str(&e.value()).ok()),
                    option { value: "", selected: parent().is_none(), "Choose equipment…" }
                    for h in hosts.iter() {
                        option { key: "{h.id}", value: "{h.id}", "{h.name} — {util::area_name(&flat, h.current_area_id)}" }
                    }
                }
                p { class: "hint", "The part moves to wherever that equipment is." }
                div { class: "flex justify-end gap-2",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button {
                        class: "btn-primary",
                        disabled: parent().is_none() || busy(),
                        "data-testid": "confirm-attach",
                        onclick: move |_| async move {
                            busy.set(true);
                            let outcome: Outcome<Asset> = sync::mutate(app, "Install part", "POST", format!("/assets/{asset_id}/link"), &LinkAssetRequest { parent_asset_id: parent() }).await;
                            match outcome {
                                Outcome::Saved(a) => {
                                    app.success(format!("{} installed", a.name));
                                    sync::reload(app, vec![DataScope::Assets, DataScope::Map]);
                                    on_close.call(());
                                }
                                Outcome::Queued => on_close.call(()),
                                Outcome::Failed(e) => app.error(e),
                            }
                            busy.set(false);
                        },
                        "Install"
                    }
                }
            }
        }
    }
}

#[component]
pub fn ChoiceButton(active: bool, #[props(into)] label: String, #[props(into)] hint: String, onclick: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: if active { "rounded-lg border-2 border-ink bg-white px-3 py-2.5 text-left transition-colors" } else { "rounded-lg border-2 border-line bg-white px-3 py-2.5 text-left hover:border-[#d6d6d3] transition-colors" },
            "aria-pressed": active,
            onclick: move |e| onclick.call(e),
            p { class: "text-sm font-medium", "{label}" }
            if !hint.is_empty() {
                p { class: "text-xs text-muted", "{hint}" }
            }
        }
    }
}
