//! Phases 7–10 of the Service 5 merge: the workforce dashboard, area-to-zone
//! mapping, tomorrow's assignments (encoder), bulk updates, pick lists, duty
//! history and swaps, overtime from a pasted list, visitation groups and
//! ministers, and the full worker profile editor.

use std::collections::{BTreeMap, HashMap};

use chrono::Duration;
use dioxus::prelude::*;
use serde_json::{json, Value};
use shared::api::*;
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{EmptyState, Icon, Modal, Spinner};
use crate::services::sync::{self, Outcome};
use crate::state::use_app;
use crate::util;
use crate::views::personnel::REFRESH as WORKERS_CHANGED;
use crate::views::workforce_ops::{by_a_number, choice, f, name_of, new_id, s, save_record, today, use_records, use_workers, Field, Kind, RecordModal, Table};

// ---------------------------------------------------------------- dashboard

#[component]
pub fn DashboardTab() -> Element {
    let workers = use_workers();
    let summary = use_resource(|| async {
        let _ = WORKERS_CHANGED();
        api::get::<WorkforceSummary>("/workforce/summary").await.unwrap_or_default()
    });
    let visits = use_records("visitations");
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    let count = |st: WorkerStatus| people.iter().filter(|w| w.status == st).count();
    let sm = summary.read().clone().unwrap_or_default();
    let max = sm.trend.iter().map(|t| t.1 + t.2).max().unwrap_or(1).max(1);
    let upcoming = visits.read().clone().unwrap_or_default().into_iter().filter(|v| s(v, "status") == "scheduled" && s(v, "date") >= today()).count();
    let bar = |items: Vec<(String, u32)>| {
        let top = items.iter().map(|i| i.1).max().unwrap_or(1).max(1);
        rsx! {
            ul { class: "space-y-1.5",
                for (name, n) in items.into_iter().take(8) {
                    li { key: "{name}", class: "text-sm",
                        div { class: "flex justify-between", span { class: "truncate", "{name}" } span { class: "tabular text-muted", "{n}" } }
                        div { class: "h-1.5 rounded bg-hover", div { class: "h-1.5 rounded bg-accent", style: "width: {n * 100 / top}%" } }
                    }
                }
            }
        }
    };
    rsx! {
        div { class: "grid grid-cols-2 sm:grid-cols-5 gap-3", "data-testid": "wf-dashboard",
            for (label, n) in [("Total active", people.len()), ("Present", count(WorkerStatus::Present)), ("Absent", count(WorkerStatus::Absent)), ("On pass", count(WorkerStatus::OnPass)), ("Visits coming up", upcoming)] {
                div { key: "{label}", class: "card p-3", p { class: "text-xs text-muted", "{label}" } p { class: "text-2xl font-semibold tabular", "{n}" } }
            }
        }
        div { class: "grid lg:grid-cols-3 gap-4",
            section { class: "card p-4 space-y-2 lg:col-span-2",
                p { class: "font-semibold", "Absent and on pass, last 14 days" }
                if sm.trend.is_empty() { p { class: "text-sm text-muted", "No attendance changes logged yet." } }
                div { class: "flex items-end gap-1 h-32", "data-testid": "wf-trend",
                    for (d, a, p) in sm.trend.clone() {
                        div { key: "{d}", class: "flex-1 flex flex-col justify-end", title: "{d}: {a} absent, {p} on pass",
                            div { class: "bg-tag-amber-text/70 rounded-t", style: "height: {p * 120 / max}px" }
                            div { class: "bg-tag-red-text/80", style: "height: {a * 120 / max}px" }
                            p { class: "text-[10px] text-muted text-center tabular", {d.get(8..).unwrap_or_default().to_string()} }
                        }
                    }
                }
            }
            section { class: "card p-4 space-y-2",
                p { class: "font-semibold", "Birthdays this month" }
                if sm.birthdays.is_empty() { p { class: "text-sm text-muted", "None this month." } }
                for (n, b) in sm.birthdays.clone() { p { key: "{n}{b}", class: "text-sm flex justify-between", span { "{n}" } span { class: "text-muted tabular", {b.get(5..).unwrap_or_default().to_string()} } } }
            }
            section { class: "card p-4 space-y-2", p { class: "font-semibold", "Trades" } {bar(sm.trades.clone())} }
            section { class: "card p-4 space-y-2", p { class: "font-semibold", "Departments" } {bar(sm.departments.clone())} }
        }
    }
}

// ------------------------------------------------------------ areas & zones

/// Service 5 keeps a worker's area as free text; mapping each one to a zone lets
/// Site Monitors see the people working in their zones.
#[component]
pub fn AreasTab() -> Element {
    let app = use_app();
    let workers = use_workers();
    let links = use_records("area_zones");
    let flat = app.flat_areas();
    let mut areas: BTreeMap<String, usize> = BTreeMap::new();
    for w in workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived && !w.area.is_empty()) {
        *areas.entry(w.area).or_default() += 1;
    }
    let mapped: HashMap<String, String> = links.read().clone().unwrap_or_default().iter().map(|v| (s(v, "id"), s(v, "zone_id"))).collect();
    if areas.is_empty() {
        return rsx! { div { class: "card", EmptyState { icon: "map", title: "No areas yet", body: "Areas come from each person's area in Personnel." } } };
    }
    rsx! {
        p { class: "text-sm text-muted", "Map each Service 5 area to a zone. Site Monitors then see (and can mark) only the people working in their zones." }
        ul { class: "card divide-y divide-line", "data-testid": "area-list",
            for (area, n) in areas {
                li { key: "{area}", class: "flex flex-wrap items-center gap-3 px-4 py-2.5", "data-testid": "area-row",
                    div { class: "flex-1 min-w-[10rem]", p { class: "font-medium", "{area}" } p { class: "text-xs text-muted", "{util::plural(n, \"person\")}" } }
                    select {
                        class: "field w-auto min-w-[14rem]",
                        "aria-label": "Zone for {area}",
                        "data-testid": "area-zone",
                        onchange: {
                            let area = area.clone();
                            move |e: FormEvent| {
                                let area = area.clone();
                                async move {
                                    match save_record(app, "area_zones", &area, &json!({ "zone_id": e.value() })).await {
                                        Ok(_) => app.success("Area mapped"),
                                        Err(err) => app.error(err),
                                    }
                                }
                            }
                        },
                        option { value: "", "Not mapped" }
                        for z in flat.iter().filter(|z| !z.area.is_archived) {
                            option { key: "{z.area.id}", value: "{z.area.id}", selected: mapped.get(&area) == Some(&z.area.id.to_string()), "{\"\u{2003}\".repeat(z.depth)}{z.area.name}" }
                        }
                    }
                }
            }
        }
    }
}

// --------------------------------------------------------- tomorrow (encoder)

async fn save_profile(app: AppState, id: Uuid, patch: Value) -> Result<(), String> {
    match sync::mutate::<_, Value>(app, "Profile", "PUT", format!("/workers/{id}/profile"), &patch).await {
        Outcome::Saved(_) => {
            *WORKERS_CHANGED.write() += 1;
            Ok(())
        }
        Outcome::Queued => Ok(()),
        Outcome::Failed(e) => Err(e),
    }
}
use crate::state::AppState;

/// Service 5's encoder: set everyone's assignment for tomorrow, group by group,
/// with "fill the whole group". Job orders for tomorrow print from the Print tab.
#[component]
pub fn TomorrowTab() -> Element {
    let app = use_app();
    let workers = use_workers();
    let mut drafts = use_signal(HashMap::<Uuid, String>::new);
    let mut fill = use_signal(HashMap::<String, String>::new);
    let mut busy = use_signal(|| false);
    let mut groups: BTreeMap<String, Vec<Worker>> = BTreeMap::new();
    for w in workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived) {
        groups.entry(if w.project_group.is_empty() { "No group".into() } else { w.project_group.clone() }).or_default().push(w);
    }
    let all: Vec<Worker> = groups.values().flatten().cloned().collect();
    let changed: Vec<(Uuid, String)> = drafts().into_iter().filter(|(id, v)| all.iter().any(|w| w.id == *id && w.tomorrow != *v)).collect();
    let n = changed.len();
    let save = move |_| {
        let changed = changed.clone();
        async move {
            busy.set(true);
            for (id, v) in changed {
                if let Err(e) = save_profile(app, id, json!({ "tomorrowAssignment": v })).await {
                    app.error(e);
                    busy.set(false);
                    return;
                }
            }
            drafts.write().clear();
            app.success("Tomorrow's assignments saved");
            busy.set(false);
        }
    };
    rsx! {
        div { class: "flex flex-wrap items-center justify-between gap-3",
            p { class: "text-sm text-muted", "Tomorrow ({util::day(chrono::Utc::now() + Duration::days(1))}): what each person will do. Fill a whole group at once, then adjust." }
            button { class: "btn-primary", "data-testid": "tomorrow-save", disabled: n == 0 || busy(), onclick: save,
                if busy() { Spinner {} }
                "Save {n} changes"
            }
        }
        for (group, members) in groups {
            section { key: "{group}", class: "card p-4 space-y-2", "data-testid": "tomorrow-group",
                div { class: "flex flex-wrap items-center gap-2",
                    p { class: "font-semibold flex-1", "{group}" }
                    input { class: "field w-auto min-w-[14rem]", "aria-label": "Assignment for all of {group}", "data-testid": "tomorrow-fill-text", placeholder: "Same task for the whole group",
                        value: "{fill().get(&group).cloned().unwrap_or_default()}",
                        oninput: { let g = group.clone(); move |e: FormEvent| { fill.write().insert(g.clone(), e.value()); } } }
                    button { class: "btn-secondary", "data-testid": "tomorrow-fill",
                        onclick: { let g = group.clone(); let ids: Vec<Uuid> = members.iter().map(|w| w.id).collect(); move |_| {
                            let v = fill().get(&g).cloned().unwrap_or_default();
                            let mut d = drafts.write();
                            for id in &ids { d.insert(*id, v.clone()); }
                        } },
                        "Fill group"
                    }
                }
                for w in members.clone() {
                    div { key: "{w.id}", class: "grid sm:grid-cols-[14rem_1fr] items-center gap-2",
                        label { class: "text-sm truncate", r#for: "tm-{w.id}", "{w.name}" }
                        input { id: "tm-{w.id}", class: "field", "data-testid": "tomorrow-input",
                            value: "{drafts().get(&w.id).cloned().unwrap_or_else(|| w.tomorrow.clone())}",
                            oninput: move |e| { drafts.write().insert(w.id, e.value()); } }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------- bulk update

/// Paste names or A# numbers (one per line) and apply one change to all of them.
#[component]
pub fn BulkTab() -> Element {
    let app = use_app();
    let workers = use_workers();
    let mut text = use_signal(String::new);
    let mut action = use_signal(|| "Absent".to_string());
    let mut value = use_signal(String::new);
    let mut result = use_signal(|| None::<(usize, Vec<String>)>);
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    let run = move |_| {
        let people = people.clone();
        async move {
            let (mut done, mut missing) = (0, vec![]);
            for line in text().lines().map(str::trim).filter(|l| !l.is_empty()) {
                let l = line.to_lowercase();
                let Some(w) = people.iter().find(|w| w.a_number.to_lowercase() == l || w.name.to_lowercase() == l) else {
                    missing.push(line.to_string());
                    continue;
                };
                let ok = match action().as_str() {
                    a @ ("Present" | "Absent" | "OnPass") => {
                        let status = match a { "Absent" => WorkerStatus::Absent, "OnPass" => WorkerStatus::OnPass, _ => WorkerStatus::Present };
                        !matches!(sync::mutate::<_, Worker>(app, "Attendance", "PUT", format!("/workers/{}/status", w.id), &WorkerStatusRequest { status, reason: "Bulk update".into() }).await, Outcome::Failed(_))
                    }
                    "group" => !matches!(
                        sync::mutate::<_, Worker>(app, "Reassign", "PUT", format!("/workers/{}/reassign", w.id), &ReassignRequest { project_group: value(), area: w.area.clone(), department: w.department.clone(), reason: "Bulk update".into() }).await,
                        Outcome::Failed(_)
                    ),
                    _ => save_profile(app, w.id, json!({ "tomorrowAssignment": value() })).await.is_ok(),
                };
                if ok { done += 1 } else { missing.push(line.to_string()) }
            }
            *WORKERS_CHANGED.write() += 1;
            result.set(Some((done, missing)));
        }
    };
    rsx! {
        div { class: "grid lg:grid-cols-2 gap-4",
            div { class: "space-y-3",
                label { class: "label", r#for: "bulk-text", "Names or A# numbers, one per line" }
                textarea { id: "bulk-text", class: "field font-mono text-sm", rows: "10", "data-testid": "bulk-text", value: "{text}", oninput: move |e| text.set(e.value()) }
            }
            div { class: "space-y-3",
                label { class: "label", r#for: "bulk-action", "What to do" }
                select { id: "bulk-action", class: "field", "data-testid": "bulk-action", onchange: move |e| action.set(e.value()),
                    option { value: "Absent", "Mark absent" }
                    option { value: "Present", "Mark present" }
                    option { value: "OnPass", "Mark on pass" }
                    option { value: "group", "Move to project group…" }
                    option { value: "tomorrow", "Set tomorrow's assignment…" }
                }
                if matches!(action().as_str(), "group" | "tomorrow") {
                    input { class: "field", "data-testid": "bulk-value", placeholder: if action() == "group" { "Project group" } else { "Tomorrow's task" }, value: "{value}", oninput: move |e| value.set(e.value()) }
                }
                button { class: "btn-primary", "data-testid": "bulk-apply", onclick: run, "Apply" }
                if let Some((done, missing)) = result() {
                    div { class: "rounded-md bg-hover px-3 py-2 text-sm", "data-testid": "bulk-result",
                        p { "{util::plural(done, \"person\")} updated." }
                        if !missing.is_empty() { p { class: "text-tag-red-text", "Not found: " {missing.join(", ")} } }
                    }
                }
            }
        }
    }
}

// --------------------------------------------------------------- pick lists

/// Service 5's custom pick lists (trades, housing, residences, Woodland shops).
#[component]
pub fn PickListsTab() -> Element {
    let app = use_app();
    let lists = use_records("custom_datalists");
    let shops = use_records("custom_woodland_shops");
    let current = lists.read().clone().unwrap_or_default().into_iter().find(|v| s(v, "id").is_empty() || s(v, "id") == "_").unwrap_or(json!({}));
    let legacy_shops: Vec<String> = shops.read().clone().unwrap_or_default().iter().filter_map(|v| v.as_str().map(str::to_string)).collect();
    let joined = |k: &str| {
        let a = current[k].as_array().map(|a| a.iter().filter_map(|x| x.as_str()).map(str::to_string).collect::<Vec<_>>()).unwrap_or_default();
        if k == "woodlandShops" && a.is_empty() { legacy_shops.join("\n") } else { a.join("\n") }
    };
    let keys = [("trades", "Trades"), ("tradeGroups", "Trade groups"), ("housingGroups", "Housing groups"), ("workResidences", "Work residences"), ("woodlandShops", "Woodland shops")];
    let mut draft = use_signal(HashMap::<&'static str, String>::new);
    let save = {
        let current = current.clone();
        let base: HashMap<&'static str, String> = keys.iter().map(|(k, _)| (*k, joined(k))).collect();
        move |_| {
            let mut next = current.clone();
            let base = base.clone();
            async move {
                for (k, v) in base {
                    let text = draft().get(k).cloned().unwrap_or(v);
                    next[k] = json!(text.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>());
                }
                match save_record(app, "custom_datalists", "_", &next).await {
                    Ok(_) => app.success("Pick lists saved"),
                    Err(e) => app.error(e),
                }
            }
        }
    };
    rsx! {
        p { class: "text-sm text-muted", "One per line. These are the choices offered when filling in people's details." }
        div { class: "grid sm:grid-cols-2 lg:grid-cols-3 gap-4",
            for (k, label) in keys {
                div { key: "{k}",
                    label { class: "label", r#for: "pl-{k}", "{label}" }
                    textarea { id: "pl-{k}", class: "field text-sm", rows: "8", "data-testid": "picklist-{k}",
                        value: "{draft().get(k).cloned().unwrap_or_else(|| joined(k))}", oninput: move |e| { draft.write().insert(k, e.value()); } }
                }
            }
        }
        div { class: "flex justify-end", button { class: "btn-primary", "data-testid": "picklist-save", onclick: save, "Save pick lists" } }
    }
}

// ------------------------------------------------------ duty history & swaps

#[component]
pub fn DutyHistoryTab() -> Element {
    let app = use_app();
    let schedules = use_records("duty_schedules");
    let areas = use_records("duty_areas");
    let workers = use_workers();
    let people = workers.read().clone().unwrap_or_default();
    let map = by_a_number(&people);
    let area_name: HashMap<String, String> = areas.read().clone().unwrap_or_default().iter().map(|a| (s(a, "id"), s(a, "name"))).collect();
    let mut list = schedules.read().clone().unwrap_or_default();
    list.sort_by(|a, b| s(b, "dateStr").cmp(&s(a, "dateStr")));
    let present: Vec<Worker> = people.iter().filter(|w| !w.archived && w.status == WorkerStatus::Present && !w.a_number.is_empty()).cloned().collect();
    if list.is_empty() {
        return rsx! { div { class: "card", EmptyState { icon: "calendar", title: "No duty rosters yet", body: "Make one on the Overnight duty tab." } } };
    }
    let swap = move |day: Value, idx: usize, to: String| async move {
        let mut next = day.clone();
        if let Some(a) = next["assignments"].get_mut(idx) {
            a["aNumber"] = json!(to);
        }
        match save_record(app, "duty_schedules", &s(&day, "dateStr"), &next).await {
            Ok(_) => app.success("Swapped"),
            Err(e) => app.error(e),
        }
    };
    rsx! {
        p { class: "text-sm text-muted", "Every roster made, newest first. Swap someone on today's or a future roster." }
        div { class: "space-y-3", "data-testid": "duty-history",
            for day in list.into_iter().take(60) {
                {
                    let editable = s(&day, "dateStr") >= today();
                    let assignments = day["assignments"].as_array().cloned().unwrap_or_default();
                    rsx! {
                        section { key: "{s(&day, \"dateStr\")}", class: "card p-4 space-y-1.5",
                            p { class: "font-semibold", "{s(&day, \"dateStr\")}" }
                            for (i, a) in assignments.into_iter().enumerate() {
                                div { key: "{i}", class: "flex flex-wrap items-center gap-2 text-sm", "data-testid": "duty-history-row",
                                    span { class: "text-muted w-40 truncate", {area_name.get(&s(&a, "areaId")).cloned().unwrap_or_default()} }
                                    span { class: "flex-1", {name_of(&map, &s(&a, "aNumber"))} }
                                    if editable {
                                        select { class: "field w-auto text-sm", "aria-label": "Swap with", "data-testid": "duty-swap",
                                            onchange: { let day = day.clone(); move |e: FormEvent| swap(day.clone(), i, e.value()) },
                                            option { value: "", "Swap with…" }
                                            for w in present.iter().filter(|w| w.a_number != s(&a, "aNumber")) {
                                                option { key: "{w.id}", value: "{w.a_number}", "{w.name}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------ overtime from pasted names

/// Service 5's OT parser: paste a list of names, match them to people, and log
/// one group overtime entry (pay = hours × rate × people).
#[component]
pub fn OtPaste(on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let workers = use_workers();
    let mut text = use_signal(String::new);
    let mut title = use_signal(String::new);
    let mut date = use_signal(today);
    let mut hours = use_signal(|| "2".to_string());
    let mut rate = use_signal(String::new);
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    // A line matches someone by A#, full name, or all of its words appearing in the name.
    let matched: Vec<(String, Option<Worker>)> = text()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| {
            let low = l.to_lowercase();
            let words: Vec<&str> = low.split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() > 1).collect();
            let hit = people
                .iter()
                .find(|w| w.a_number.to_lowercase() == low || w.name.to_lowercase() == low)
                .or_else(|| people.iter().find(|w| !words.is_empty() && words.iter().all(|x| w.name.to_lowercase().contains(x))))
                .cloned();
            (l.to_string(), hit)
        })
        .collect();
    let found: Vec<Worker> = matched.iter().filter_map(|m| m.1.clone()).collect();
    let (h, r) = (hours().parse::<f64>().unwrap_or(0.0), rate().parse::<f64>().unwrap_or(0.0));
    let pay = ((h * r * found.len() as f64) * 100.0).round() / 100.0;
    let (count, none_found) = (found.len(), found.is_empty());
    let save = move |_| {
        let found = found.clone();
        async move {
            let vols: Vec<Value> = found.iter().map(|w| json!({ "aNumber": w.a_number, "parsedName": w.name, "trade": w.trade, "matchStatus": "matched" })).collect();
            let entry = json!({ "date": date(), "title": title(), "otType": "regular", "hours": h, "hourlyRate": r, "computedPay": pay,
                                "totalCount": vols.len(), "volunteers": vols, "status": "approved", "savedAt": chrono::Utc::now().to_rfc3339() });
            match save_record(app, "ot_entries", &new_id("ot"), &entry).await {
                Ok(_) => {
                    app.success("Overtime logged");
                    on_close.call(());
                }
                Err(e) => app.error(e),
            }
        }
    };
    rsx! {
        Modal { title: "Overtime from a list of names", wide: true, on_close: move |_| on_close.call(()),
            div { class: "space-y-4", "data-testid": "ot-paste",
                div { class: "grid sm:grid-cols-4 gap-3",
                    div { class: "sm:col-span-2", label { class: "label", r#for: "otp-title", "Work done" } input { id: "otp-title", class: "field", "data-testid": "otp-title", value: "{title}", oninput: move |e| title.set(e.value()) } }
                    div { label { class: "label", r#for: "otp-date", "Date" } input { id: "otp-date", r#type: "date", class: "field", value: "{date}", oninput: move |e| date.set(e.value()) } }
                    div { label { class: "label", r#for: "otp-hours", "Hours" } input { id: "otp-hours", r#type: "number", step: "any", class: "field", "data-testid": "otp-hours", value: "{hours}", oninput: move |e| hours.set(e.value()) } }
                    div { label { class: "label", r#for: "otp-rate", "Rate per hour" } input { id: "otp-rate", r#type: "number", step: "any", class: "field", "data-testid": "otp-rate", value: "{rate}", oninput: move |e| rate.set(e.value()) } }
                }
                div {
                    label { class: "label", r#for: "otp-names", "Names (one per line, as written)" }
                    textarea { id: "otp-names", class: "field text-sm", rows: "6", "data-testid": "otp-names", value: "{text}", oninput: move |e| text.set(e.value()) }
                }
                ul { class: "text-sm space-y-0.5", "data-testid": "otp-matches",
                    for (i, (line, hit)) in matched.into_iter().enumerate() {
                        li { key: "{i}", class: if hit.is_some() { "" } else { "text-tag-red-text" },
                            "{line} → "
                            {hit.as_ref().map(|w| format!("{} (A# {})", w.name, w.a_number)).unwrap_or_else(|| "not found".into())}
                        }
                    }
                }
                div { class: "modal-actions",
                    span { class: "mr-auto text-sm tabular", "data-testid": "otp-pay", "{util::plural(count, \"person\")} · ₱{pay:.2}" }
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button { class: "btn-primary", "data-testid": "otp-save", disabled: none_found || title().trim().is_empty(), onclick: save, "Log overtime" }
                }
            }
        }
    }
}

// ------------------------------------------- visitation groups and ministers

#[component]
pub fn VisitGroupsTab() -> Element {
    let groups = use_records("visitation_groups");
    let ministers = use_records("ministers");
    let workers = use_workers();
    let mut editing = use_signal(|| None::<(&'static str, String, Value)>);
    let people = workers.read().clone().unwrap_or_default();
    let glist = groups.read().clone().unwrap_or_default();
    let mlist = ministers.read().clone().unwrap_or_default();
    let grows: Vec<(String, Vec<String>)> = glist.iter().map(|g| (s(g, "id"), vec![s(g, "name"), s(g, "housing"), s(g, "prayerDay"), s(g, "prayerTime")])).collect();
    let mrows: Vec<(String, Vec<String>)> = mlist.iter().map(|m| (s(m, "id"), vec![s(m, "name")])).collect();
    let _ = people;
    let gfields: Vec<Field> = vec![
        f("name", "Group name", Kind::Text),
        f("housing", "Housing", Kind::Text),
        f("prayerDay", "Prayer day", choice(&["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"])),
        f("prayerTime", "Prayer time", Kind::Text),
        f("notes", "Notes", Kind::Long),
    ];
    let mfields: Vec<Field> = vec![f("name", "Minister's name", Kind::Text)];
    rsx! {
        div { class: "grid lg:grid-cols-[2fr_1fr] gap-4",
            div { class: "space-y-2",
                div { class: "flex justify-between items-center",
                    p { class: "font-semibold", "Visitation groups" }
                    button { class: "btn-secondary", "data-testid": "vgroup-add", onclick: move |_| editing.set(Some(("visitation_groups", new_id("vg"), json!({ "createdAt": chrono::Utc::now().to_rfc3339() })))), Icon { name: "plus" } "Add group" }
                }
                Table { head: vec!["Group", "Housing", "Prayer day", "Time"], rows: grows, on_edit: move |id: String| {
                    if let Some(v) = glist.iter().find(|g| s(g, "id") == id) { editing.set(Some(("visitation_groups", id, v.clone()))) }
                } }
            }
            div { class: "space-y-2",
                div { class: "flex justify-between items-center",
                    p { class: "font-semibold", "Ministers" }
                    button { class: "btn-secondary", "data-testid": "minister-add", onclick: move |_| editing.set(Some(("ministers", new_id("min"), json!({})))), Icon { name: "plus" } "Add minister" }
                }
                Table { head: vec!["Name"], rows: mrows, on_edit: move |id: String| {
                    if let Some(v) = mlist.iter().find(|m| s(m, "id") == id) { editing.set(Some(("ministers", id, v.clone()))) }
                } }
            }
        }
        if let Some((list, id, v)) = editing() {
            RecordModal { title: if list == "ministers" { "Minister".to_string() } else { "Visitation group".to_string() }, list, id, fields: if list == "ministers" { mfields.clone() } else { gfields.clone() }, initial: v, on_close: move |_| editing.set(None) }
        }
    }
}

// ------------------------------------------------------------ profile editor

/// Everything Service 5 records about a person beyond the basics. Health fields
/// appear only for people with "Clinic ledger" (the server enforces it too).
#[component]
pub fn ProfileModal(worker: Worker, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let id = worker.id;
    let profile = use_resource(move || async move { api::get::<Value>(&format!("/workers/{id}/profile")).await });
    let mut photo_busy = use_signal(|| false);
    let clinic = app.can(Permission::ManageClinic);
    let mut fields = vec![
        f("birthday", "Birthday", Kind::Date),
        f("civilStatus", "Civil status", choice(&["", "Single", "Married", "Widowed", "Separated"])),
        f("address", "Home address", Kind::Text),
        f("residence", "Residence", Kind::Text),
        f("workResidence", "Work residence", Kind::Text),
        f("language", "Language", Kind::Text),
        f("fatherName", "Father's name", Kind::Text),
        f("motherName", "Mother's name", Kind::Text),
        f("spouseName", "Spouse's name", Kind::Text),
        f("spouseBirthday", "Spouse's birthday", Kind::Date),
        f("spouseContact", "Spouse's phone", Kind::Text),
        f("weddingDate", "Wedding date", Kind::Date),
        f("emergencyContact", "Emergency contact", Kind::Text),
        f("emergencyRelationship", "Relationship", Kind::Text),
        f("emergencyPhone", "Emergency phone", Kind::Text),
        f("skillLevel", "Skill level", Kind::Text),
        f("hourlyRate", "Hourly rate", Kind::Number),
        f("subDepartment", "Sub-department", Kind::Text),
        f("specificTask", "Specific task", Kind::Text),
        f("tomorrowAssignment", "Tomorrow's assignment", Kind::Text),
        f("visitationGroup", "Visitation group", Kind::Text),
    ];
    if clinic {
        fields.extend([f("bloodType", "Blood type", choice(&["", "A+", "A-", "B+", "B-", "AB+", "AB-", "O+", "O-"])), f("healthClearance", "Health clearance", Kind::Text), f("maintenanceMeds", "Maintenance medicine", Kind::Text), f("medicalNotes", "Medical notes", Kind::Long)]);
    }
    let upload = move |_| async move {
        let file = match crate::services::media_canvas::file_as_data_url("profile-photo", 2.0 * 1_048_576.0).await {
            Ok(Some(f)) => f,
            Ok(None) => return,
            Err(e) => return app.error(e),
        };
        photo_busy.set(true);
        match save_profile(app, id, json!({ "photoUrl": file })).await {
            Ok(_) => app.success("Photo saved"),
            Err(e) => app.error(e),
        }
        photo_busy.set(false);
    };
    let loaded: Option<Result<Value, String>> = profile.read().as_ref().map(|r| r.clone().map_err(|e| e.message()));
    match loaded {
        None => rsx! { Modal { title: worker.name.clone(), on_close: move |_| on_close.call(()), div { class: "p-6 flex justify-center", Spinner { size: 20 } } } },
        Some(Err(e)) => rsx! { Modal { title: worker.name.clone(), on_close: move |_| on_close.call(()), p { class: "text-sm text-tag-red-text", "{e}" } } },
        Some(Ok(p)) => rsx! {
            div { class: "contents", "data-testid": "profile-modal",
                RecordModal { title: format!("{} — profile", worker.name), list: "workers", id: id.to_string(), fields, initial: p,
                    endpoint: Some(format!("/workers/{id}/profile")), deletable: false, on_close: move |_| on_close.call(()) }
            }
            div { class: "fixed bottom-6 left-6 z-[60] card p-3 flex items-center gap-3 shadow-float",
                if worker.has_photo { img { src: "/api/workers/{id}/photo", alt: "", class: "w-12 h-12 rounded-full object-cover" } }
                input { id: "profile-photo", r#type: "file", accept: "image/jpeg,image/png,image/webp", class: "hidden", onchange: upload }
                label { r#for: "profile-photo", class: "btn-secondary cursor-pointer", "data-testid": "profile-photo-upload",
                    if photo_busy() { Spinner {} } else { Icon { name: "camera" } }
                    if worker.has_photo { "Change photo" } else { "Add photo" }
                }
            }
        },
    }
}
