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
use crate::components::ui::{EmptyState, Icon, Modal};
use crate::services::sync::{self, Outcome};
use crate::state::use_app;
use crate::util;
use crate::views::personnel::REFRESH as WORKERS_CHANGED;
use crate::views::workforce_ops::{by_a_number, choice, f, name_of, new_id, s, s5print, save_record, today, use_records, use_workers, Field, Kind, RecordModal, Table};

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

/// Tomorrow as Service 5 stages it (an object Start New Day turns into the task
/// of the day); a blank task clears it.
pub fn staged_tomorrow(w: Option<&Worker>, task: &str) -> Value {
    if task.trim().is_empty() {
        return Value::Null;
    }
    json!({
        "specificTask": task.trim(), "area": w.map(|w| w.area.clone()).unwrap_or_default(), "project": w.map(|w| w.project.clone()).unwrap_or_default(),
        "date": (chrono::Local::now() + Duration::days(1)).format("%Y-%m-%d").to_string(), "isAssigned": true, "dailyStatus": "Present",
        "encodedAt": chrono::Utc::now().to_rfc3339(),
    })
}

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
                    _ => save_profile(app, w.id, json!({ "tomorrowAssignment": staged_tomorrow(Some(w), &value()) })).await.is_ok(),
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
    // Service 5's separate pick-list keys are folded into this one record on import.
    let current = lists.read().clone().unwrap_or_default().into_iter().find(|v| s(v, "id").is_empty() || s(v, "id") == "_").unwrap_or(json!({}));
    let joined = |k: &str| current[k].as_array().map(|a| a.iter().filter_map(|x| x.as_str()).map(str::to_string).collect::<Vec<_>>()).unwrap_or_default().join("\n");
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
                    button { class: "btn-secondary", "data-testid": "vgroup-print-all-rosters", onclick: move |_| s5print("visitationSummary", json!([])), Icon { name: "archive" } "Print All Rosters" }
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
