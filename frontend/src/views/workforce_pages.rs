//! Service 5 pages rebuilt to match their originals: the Signatories board
//! (cards with signature image, mapped groups with volunteer counts, unbind,
//! coverage stats) and Reassign Volunteers (move several people at once to a
//! project and area, logged, with a printable reallocation notice).

use std::collections::{BTreeMap, BTreeSet, HashSet};

use dioxus::prelude::*;
use serde_json::{json, Value};
use shared::api::*;
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{Icon, Spinner};
use crate::state::use_app;
use crate::util;
use crate::views::personnel::REFRESH as WORKERS_CHANGED;
use crate::views::workforce_ops::{new_id, s, s5print, save_record, use_records, use_workers, WorkerPhoto};

// ---------------------------------------------------------------- signatories

// ------------------------------------------------------------------ dashboard

/// Who is behind a number (Service 5's roster pop-ups on the dashboard).
#[component]
pub fn RosterModal(title: String, people: Vec<Worker>, on_close: EventHandler<()>, #[props(default)] present: bool) -> Element {
    let ids: Vec<String> = people.iter().map(|w| w.id.to_string()).collect();
    rsx! {
        crate::components::ui::Modal { title: title.clone(), wide: true, on_close: move |_| on_close.call(()),
            div { class: "space-y-3", "data-testid": "roster-modal",
                div { class: "flex justify-between items-center",
                    p { class: "text-sm text-muted", {util::plural(people.len(), "person")} }
                    if present {
                        button { class: "btn-primary no-print", "data-testid": "roster-print", onclick: move |_| s5print("presentRoster", json!([ids.clone()])), "🖨️ Print All Present" }
                    }
                }
                table { class: "w-full text-sm",
                    thead { tr { class: "text-left text-muted border-b border-line", th { class: "py-1.5", "Name" } th { "A#" } th { "Trade" } th { "Project group" } th { "Status" } } }
                    tbody { class: "divide-y divide-line",
                        for w in people.iter() {
                            tr { key: "{w.id}", "data-testid": "roster-row",
                                td { class: "py-1.5", "{w.name}" } td { "{w.a_number}" } td { "{w.trade}" } td { "{w.project_group}" } td { "{w.status.label()}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Service 5's dashboard charts: status donut, people by area and by
/// department, each opening the roster behind it.
#[component]
pub fn DashboardCharts() -> Element {
    let workers = use_workers();
    let mut roster = use_signal(|| None::<(String, Vec<Worker>)>);
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    let total = people.len().max(1) as f64;
    let parts = [("Present", WorkerStatus::Present, "#16a34a"), ("Absent", WorkerStatus::Absent, "#dc2626"), ("On pass", WorkerStatus::OnPass, "#d97706")];
    let mut offset = 0.0;
    let arcs: Vec<(&str, WorkerStatus, &str, usize, f64, f64)> = parts
        .iter()
        .map(|(l, st, c)| {
            let n = people.iter().filter(|w| w.status == *st).count();
            let len = n as f64 / total * 100.0;
            let a = (*l, *st, *c, n, len, offset);
            offset += len;
            a
        })
        .collect();
    let by = |key: fn(&Worker) -> &String| {
        let mut m: BTreeMap<String, Vec<Worker>> = BTreeMap::new();
        for w in &people {
            m.entry(if key(w).trim().is_empty() { "Not set".into() } else { key(w).trim().to_string() }).or_default().push(w.clone());
        }
        let mut v: Vec<(String, Vec<Worker>)> = m.into_iter().collect();
        v.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        v
    };
    let areas = by(|w| &w.area);
    let depts = by(|w| &w.department);
    let max_area = areas.first().map(|a| a.1.len()).unwrap_or(1).max(1);
    rsx! {
        div { class: "grid lg:grid-cols-3 gap-4", "data-testid": "wf-charts",
            section { class: "card p-4 space-y-3",
                p { class: "font-semibold", "Today's status" }
                div { class: "flex items-center gap-4",
                    svg { width: "120", height: "120", view_box: "0 0 42 42", role: "img", "aria-label": "Status of active people today",
                        circle { cx: "21", cy: "21", r: "15.9155", fill: "none", stroke: "#eee", stroke_width: "6" }
                        for (l, _, c, _, len, off) in arcs.iter().cloned() {
                            circle { key: "{l}", cx: "21", cy: "21", r: "15.9155", fill: "none", stroke: "{c}", stroke_width: "6",
                                stroke_dasharray: "{len} {100.0 - len}", stroke_dashoffset: "{25.0 - off}" }
                        }
                        text { x: "21", y: "23", text_anchor: "middle", font_size: "7", font_weight: "600", fill: "currentColor", "{people.len()}" }
                    }
                    ul { class: "space-y-1.5 text-sm flex-1",
                        for (l, st, c, n, _, _) in arcs.iter().cloned() {
                            li { key: "{l}",
                                button { class: "w-full flex items-center gap-2 rounded px-1.5 py-1 hover:bg-hover", "data-testid": "wf-donut-{l}",
                                    onclick: { let list: Vec<Worker> = people.iter().filter(|w| w.status == st).cloned().collect(); move |_| roster.set(Some((l.to_string(), list.clone()))) },
                                    span { class: "w-2.5 h-2.5 rounded-full", style: "background: {c}" }
                                    span { class: "flex-1 text-left", "{l}" }
                                    span { class: "tabular", "{n}" }
                                }
                            }
                        }
                    }
                }
            }
            section { class: "card p-4 space-y-2",
                p { class: "font-semibold", "Where people are" }
                for (a, list) in areas.iter().take(8).cloned() {
                    button { key: "{a}", class: "w-full text-left text-sm hover:bg-hover rounded px-1 py-0.5", "data-testid": "wf-area",
                        onclick: { let a = a.clone(); let l = list.clone(); move |_| roster.set(Some((a.clone(), l.clone()))) },
                        div { class: "flex justify-between", span { class: "truncate", "{a}" } span { class: "tabular text-muted", "{list.len()}" } }
                        div { class: "h-1.5 rounded bg-hover", div { class: "h-1.5 rounded bg-accent", style: "width: {list.len() * 100 / max_area}%" } }
                    }
                }
            }
            section { class: "card p-4 space-y-1",
                p { class: "font-semibold", "Departments" }
                for (d, list) in depts.iter().cloned() {
                    button { key: "{d}", class: "w-full flex justify-between text-sm hover:bg-hover rounded px-1 py-1", "data-testid": "wf-dept",
                        onclick: { let d = d.clone(); let l = list.clone(); move |_| roster.set(Some((d.clone(), l.clone()))) },
                        span { class: "truncate", "{d}" }
                        span { class: "tabular text-muted", "{list.iter().filter(|w| w.status == WorkerStatus::Present).count()} / {list.len()} present" }
                    }
                }
            }
        }
        if let Some((t, list)) = roster() {
            // Service 5 prints only the Present roster ("Print All Present").
            RosterModal { present: arcs.iter().any(|a| a.1 == WorkerStatus::Present && a.0 == t.as_str()), title: t, people: list, on_close: move |_| roster.set(None) }
        }
    }
}

// ---------------------------------------------------------------------- duty

// ------------------------------------------------------------------ overtime

/// Service 5's OT encoder: one quick row per entry, the rate taken from the
/// person, and "copy to the next day".
#[component]
pub fn OtEncoder(day: String) -> Element {
    let app = use_app();
    let workers = use_workers();
    let ot = use_records("ot_entries");
    let mut who = use_signal(String::new);
    let mut area = use_signal(String::new);
    let mut task = use_signal(String::new);
    let mut hours = use_signal(String::new);
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived && !w.a_number.is_empty()).collect();
    let add = {
        let people = people.clone();
        let day = day.clone();
        move |_| {
            let people = people.clone();
            let day = day.clone();
            async move {
                let Some(w) = people.iter().find(|w| w.a_number == who()) else { return app.error("Choose the person.") };
                let h: f64 = hours().trim().parse().unwrap_or(0.0);
                if h <= 0.0 {
                    return app.error("Enter the hours worked.");
                }
                let rate = w.hourly_rate.unwrap_or(0.0);
                let id = new_id("ot");
                let entry = json!({ "date": day, "volunteerId": w.a_number, "title": task().trim(), "otTask": task().trim(), "otArea": area().trim(), "otType": "regular",
                    "hours": h, "hourlyRate": rate, "computedPay": ((h * rate) * 100.0).round() / 100.0, "trade": w.trade, "skillLevel": w.skill_level, "status": "approved" });
                match save_record(app, "ot_entries", &id, &entry).await {
                    Ok(_) => {
                        app.success(format!("Overtime added for {}", w.name));
                        who.set(String::new());
                        hours.set(String::new());
                    }
                    Err(e) => app.error(e),
                }
            }
        }
    };
    let copy_next = {
        let day = day.clone();
        move |_| {
            let day = day.clone();
            let all = ot.peek().clone().unwrap_or_default();
            async move {
                let Ok(d) = chrono::NaiveDate::parse_from_str(&day, "%Y-%m-%d") else { return };
                let next = (d + chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
                let from: Vec<Value> = all.into_iter().filter(|e| s(e, "date") == day).collect();
                if from.is_empty() {
                    return app.error("No overtime on this day to copy.");
                }
                for e in &from {
                    let mut n = e.clone();
                    let id = new_id("ot");
                    n["id"] = json!(id);
                    n["date"] = json!(next);
                    if let Err(err) = save_record(app, "ot_entries", &id, &n).await {
                        return app.error(err);
                    }
                }
                app.success(format!("{} copied to {next}", util::plural(from.len(), "entry")));
            }
        }
    };
    rsx! {
        div { class: "card p-3 flex flex-wrap items-end gap-2 no-print", "data-testid": "ot-encoder",
            div { class: "min-w-[14rem] flex-1",
                label { class: "label", r#for: "ote-who", "Volunteer" }
                select { id: "ote-who", class: "field", "data-testid": "ote-who", onchange: move |e| who.set(e.value()),
                    option { value: "", selected: who().is_empty(), "Choose…" }
                    for w in people.iter() { option { key: "{w.id}", value: "{w.a_number}", selected: who() == w.a_number, "{w.name} · {w.trade}" } }
                }
            }
            div { label { class: "label", r#for: "ote-area", "Area" } input { id: "ote-area", class: "field", "data-testid": "ote-area", placeholder: "e.g. Hall", value: "{area}", oninput: move |e| area.set(e.value()) } }
            div { label { class: "label", r#for: "ote-task", "Task" } input { id: "ote-task", class: "field", "data-testid": "ote-task", placeholder: "e.g. Painting", value: "{task}", oninput: move |e| task.set(e.value()) } }
            div { class: "w-24", label { class: "label", r#for: "ote-hours", "Hours" } input { id: "ote-hours", r#type: "number", step: "0.5", class: "field tabular", "data-testid": "ote-hours", value: "{hours}", oninput: move |e| hours.set(e.value()) } }
            button { class: "btn-primary", "data-testid": "ote-add", onclick: add, Icon { name: "plus" } "Add entry" }
            button { class: "btn-secondary", "data-testid": "ote-copy-next", onclick: copy_next, "Copy to next day" }
        }
    }
}

// ------------------------------------------------------------------ org chart

// --------------------------------------------------------- reassign volunteers

/// What a reallocation did, for the printable notice.
#[derive(Clone, PartialEq)]
struct Notice {
    moved: Vec<(String, String, String)>,
    project: String,
    area: String,
    reason: String,
    remaining: Vec<(String, Vec<String>)>,
    at: String,
}

/// Service 5's Reassign Volunteers: pick people, a project and an area; every
/// move is logged and a reallocation notice is ready to print or share.
#[component]
pub fn ReallocationPanel() -> Element {
    let app = use_app();
    let workers = use_workers();
    let mut q = use_signal(String::new);
    let mut chosen = use_signal(Vec::<Uuid>::new);
    let mut project = use_signal(String::new);
    let mut area = use_signal(String::new);
    let mut reason = use_signal(String::new);
    let mut task = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(|| None::<Notice>);
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    let projects: Vec<String> = people.iter().flat_map(|w| [w.project.clone(), w.project_group.clone()]).filter(|p| !p.is_empty()).collect::<BTreeSet<_>>().into_iter().collect();
    let areas: Vec<String> = people.iter().map(|w| w.area.clone()).filter(|p| !p.is_empty()).collect::<BTreeSet<_>>().into_iter().collect();
    let ql = q().trim().to_lowercase();
    let matches: Vec<Worker> = if ql.is_empty() {
        vec![]
    } else {
        people.iter().filter(|w| !chosen().contains(&w.id) && (w.name.to_lowercase().contains(&ql) || w.a_number.to_lowercase() == ql || w.trade.to_lowercase().contains(&ql))).take(8).cloned().collect()
    };
    let picked: Vec<Worker> = chosen().iter().filter_map(|id| people.iter().find(|w| w.id == *id).cloned()).collect();
    let apply = {
        let people = people.clone();
        let picked = picked.clone();
        move |_| {
            let people = people.clone();
            let picked = picked.clone();
            async move {
                let (to_project, to_area, to_task) = (project().trim().to_string(), area().trim().to_string(), task().trim().to_string());
                // Service 5 needs both the target project group and the area.
                if to_project.is_empty() {
                    return app.error("Target project is required.");
                }
                if to_area.is_empty() {
                    return app.error("Target area is required.");
                }
                busy.set(true);
                let mut moved = vec![];
                let mut sources: BTreeMap<String, ()> = BTreeMap::new();
                for w in &picked {
                    // As in Service 5: project group and project become the target; area and task (kept if blank) are set and logged.
                    let body = json!({ "area": to_area, "project": to_project, "specific_task": if to_task.is_empty() { w.task.clone() } else { to_task.clone() } });
                    let r1 = api::put::<_, Worker>(&format!("/workers/{}/assignment", w.id), &body).await.map(|_| ());
                    let r2 = if to_project == w.project_group {
                        Ok(())
                    } else {
                        let why = if reason().trim().is_empty() { "Workforce Reallocation".to_string() } else { reason() };
                        let req = ReassignRequest { project_group: to_project.clone(), area: to_area.clone(), department: w.department.clone(), reason: why };
                        api::put::<_, Worker>(&format!("/workers/{}/reassign", w.id), &req).await.map(|_| ())
                    };
                    if let Err(e) = r1.and(r2) {
                        app.error(format!("{}: {}", w.name, e.message()));
                        busy.set(false);
                        return;
                    }
                    if !w.project.is_empty() {
                        sources.insert(w.project.clone(), ());
                    }
                    moved.push((w.name.clone(), if w.project.is_empty() { w.area.clone() } else { w.project.clone() }, w.a_number.clone()));
                }
                let moved_ids: HashSet<Uuid> = picked.iter().map(|w| w.id).collect();
                let remaining = sources
                    .keys()
                    .map(|p| (p.clone(), people.iter().filter(|w| &w.project == p && !moved_ids.contains(&w.id)).map(|w| w.name.clone()).collect()))
                    .collect();
                notice.set(Some(Notice { moved, project: to_project, area: to_area, reason: reason(), remaining, at: util::long_day(chrono::Utc::now()) }));
                chosen.write().clear();
                *WORKERS_CHANGED.write() += 1;
                app.success(format!("{} reassigned", util::plural(picked.len(), "person")));
                busy.set(false);
            }
        }
    };
    rsx! {
        div { class: "grid lg:grid-cols-2 gap-5 no-print", "data-testid": "realloc",
            section { class: "card p-5 space-y-4",
                div {
                    p { class: "font-semibold", "Quick Reassign Volunteers" }
                    p { class: "text-sm text-muted", "Reallocate selected volunteer workers from their current project, area, and tasking to new assignments for Today's shift." }
                }
                div { class: "space-y-2",
                    p { class: "label", "Selected Volunteers (" span { "data-testid": "realloc-count", "{picked.len()}" } ")" }
                    div { class: "flex flex-wrap gap-2", "data-testid": "realloc-chips",
                        if picked.is_empty() { p { class: "text-sm text-muted", "No volunteers selected yet." } }
                        for w in picked.iter().cloned() {
                            span { key: "{w.id}", class: "inline-flex items-center gap-1.5 rounded-md bg-accent-soft text-accent-ink px-2 py-1 text-sm", "data-testid": "realloc-chip",
                                WorkerPhoto { id: w.id, name: w.name.clone(), has_photo: w.has_photo, size: 20 }
                                "{w.name}"
                                span { class: "text-xs opacity-70", {if w.project_group.is_empty() { "Unassigned".to_string() } else { w.project_group.clone() }} }
                                button { "aria-label": "Remove {w.name}", onclick: move |_| chosen.write().retain(|x| *x != w.id), Icon { name: "x", size: 12 } }
                            }
                        }
                    }
                }
                div { class: "relative",
                    label { class: "label", r#for: "realloc-search", "Add another volunteer to queue:" }
                    input { id: "realloc-search", class: "field", "data-testid": "realloc-search", autocomplete: "off", placeholder: "Type name, A#, trade or project to add...", value: "{q}", oninput: move |e| q.set(e.value()) }
                    if !matches.is_empty() {
                        ul { class: "absolute z-20 left-0 right-0 mt-1 card shadow-pop p-1 max-h-64 overflow-y-auto",
                            for w in matches {
                                li { key: "{w.id}",
                                    button { class: "w-full text-left px-3 py-2 rounded hover:bg-hover text-sm flex items-center gap-2", "data-testid": "realloc-match",
                                        onclick: move |_| { chosen.write().push(w.id); q.set(String::new()); },
                                        WorkerPhoto { id: w.id, name: w.name.clone(), has_photo: w.has_photo, size: 22 }
                                        "{w.name}" span { class: "text-muted ml-1", "A# {w.a_number} · {w.trade} · {w.project_group}" }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "grid sm:grid-cols-2 gap-3",
                    div {
                        label { class: "label", r#for: "reassign-target-project", "Target Project Group *" }
                        input { id: "reassign-target-project", class: "field", list: "realloc-projects", required: true, autocomplete: "off", placeholder: "e.g. House 2 Renovation", "data-testid": "realloc-project", value: "{project}", oninput: move |e| project.set(e.value()) }
                        datalist { id: "realloc-projects", for p in projects { option { key: "{p}", value: "{p}" } } }
                    }
                    div {
                        label { class: "label", r#for: "reassign-target-area", "Target Area / Site *" }
                        input { id: "reassign-target-area", class: "field", list: "realloc-areas", required: true, autocomplete: "off", placeholder: "e.g. House 2", "data-testid": "realloc-area", value: "{area}", oninput: move |e| area.set(e.value()) }
                        datalist { id: "realloc-areas", for a in areas { option { key: "{a}", value: "{a}" } } }
                    }
                }
                div {
                    label { class: "label", r#for: "reassign-target-task", "New Tasking / Specific Task" }
                    input { id: "reassign-target-task", class: "field", autocomplete: "off", placeholder: "e.g. Subfloor reinforcement and electrical rough-in", "data-testid": "realloc-task", value: "{task}", oninput: move |e| task.set(e.value()) }
                }
                div {
                    label { class: "label", r#for: "reassign-reason", "Reallocation Note / Reason (Optional)" }
                    input { id: "reassign-reason", class: "field", autocomplete: "off", placeholder: "e.g. Urgent manpower support for concrete pour deadline", "data-testid": "realloc-reason", value: "{reason}", oninput: move |e| reason.set(e.value()) }
                }
                div { class: "flex justify-end gap-2",
                    button { class: "btn-secondary", "data-testid": "realloc-cancel",
                        onclick: move |_| { chosen.write().clear(); project.set(String::new()); area.set(String::new()); task.set(String::new()); reason.set(String::new()); },
                        "Cancel"
                    }
                    button { class: "btn-primary", "data-testid": "realloc-apply", disabled: picked.is_empty() || busy(), onclick: apply,
                        if busy() { Spinner {} } else { Icon { name: "move" } }
                        "Reassign Volunteers"
                    }
                }
            }
            section { class: "card p-5 space-y-3", "data-testid": "realloc-notice-card",
                div { class: "flex items-center justify-between",
                    p { class: "font-semibold", "Reallocation notice" }
                }
                if notice().is_none() { p { class: "text-sm text-muted", "After a move, the notice to share with the team appears here." } }
            }
        }
        if let Some(n) = notice() {
            article { class: "card p-6 space-y-3 max-w-2xl print-sheet", "data-testid": "realloc-notice",
                p { class: "text-[10px] uppercase tracking-wider text-muted text-center", "Service section · Workforce command" }
                p { class: "text-lg font-semibold text-center", "Workforce reallocation notice" }
                p { class: "text-sm text-center text-muted", "{n.at}" }
                p { class: "text-sm", span { class: "font-semibold", "To: " } {[n.project.as_str(), n.area.as_str()].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" · ")} }
                if !n.reason.is_empty() { p { class: "text-sm", span { class: "font-semibold", "Reason: " } "{n.reason}" } }
                table { class: "w-full text-sm border-collapse",
                    thead { tr { class: "text-left border-b", th { class: "p-1", "Volunteer" } th { class: "p-1", "A#" } th { class: "p-1", "From" } th { class: "p-1", "" } } }
                    tbody {
                        for (name, from, a) in n.moved.iter() {
                            tr { key: "{a}{name}", class: "border-b",
                                td { class: "p-1 font-medium", "{name}" } td { class: "p-1", "{a}" } td { class: "p-1", "{from}" }
                                td { class: "p-1", span { class: "pill bg-tag-green-bg text-tag-green-text", "Reassigned" } }
                            }
                        }
                    }
                }
                for (p, names) in n.remaining.iter() {
                    p { key: "{p}", class: "text-sm",
                        span { class: "font-semibold", "Still on {p}: " }
                        {if names.is_empty() { "No remaining members assigned to this project.".to_string() } else { names.join(", ") }}
                    }
                }
            }
        }
    }
}
