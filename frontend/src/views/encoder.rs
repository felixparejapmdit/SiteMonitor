//! Service 5's Daily Assignment Encoder: choose today's assignments or
//! tomorrow's schedule, then encode by Group Slip (one project group's paper
//! slip at a time, with quick-fill, progress and job-order printing), Grid
//! (everyone, grouped, 50 per page) or Card (one person at a time).

use std::collections::{BTreeMap, HashMap, HashSet};

use dioxus::prelude::*;
use serde_json::{json, Value};
use shared::api::WorkerStatusRequest;
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{EmptyState, Icon};
use crate::services::sync::{self, Outcome};
use crate::state::{use_app, AppState};
use crate::views::personnel::REFRESH as WORKERS_CHANGED;
use crate::views::workforce_ops::{s, use_records, use_workers};
use crate::views::workforce_print::PrintTab;

const PAGE: usize = 50;
const FIELDS: [&str; 3] = ["area", "project", "specificTask"];

/// Service 5's names for the three statuses.
pub fn legacy(s: WorkerStatus) -> &'static str {
    match s {
        WorkerStatus::Present => "Present",
        WorkerStatus::Absent => "Absent",
        WorkerStatus::OnPass => "Passes",
    }
}

/// What the encoder shows for a person (Service 5 `getWorkerEncoderState`).
#[derive(Clone, PartialEq, Debug)]
pub struct EncState {
    pub status: String,
    pub reason: String,
    pub area: String,
    pub project: String,
    pub task: String,
    pub assigned: bool,
}

pub fn state_of(w: &Worker, tomorrow: bool) -> EncState {
    if tomorrow {
        if !w.tomorrow_status.is_empty() || !w.tomorrow.is_empty() || !w.tomorrow_area.is_empty() {
            return EncState {
                status: if w.tomorrow_status.is_empty() { "Present".into() } else { w.tomorrow_status.clone() },
                reason: w.tomorrow_reason.clone(),
                area: w.tomorrow_area.clone(),
                project: w.tomorrow_project.clone(),
                task: w.tomorrow.clone(),
                assigned: true,
            };
        }
        return EncState { status: legacy(w.status).into(), reason: w.remarks.clone(), area: w.area.clone(), project: w.project.clone(), task: w.task.clone(), assigned: false };
    }
    EncState { status: legacy(w.status).into(), reason: w.remarks.clone(), area: w.area.clone(), project: w.project.clone(), task: w.task.clone(), assigned: w.assigned_today }
}

fn get(st: &EncState, field: &str) -> String {
    match field {
        "area" => st.area.clone(),
        "project" => st.project.clone(),
        "specificTask" => st.task.clone(),
        "status" => st.status.clone(),
        _ => String::new(),
    }
}

fn next_status(s: &str) -> &'static str {
    match s {
        "Present" => "Absent",
        "Absent" => "Passes",
        _ => "Present",
    }
}

fn tomorrow_iso() -> String {
    (chrono::Local::now() + chrono::Duration::days(1)).format("%Y-%m-%d").to_string()
}

/// Saves fields of one person the way Service 5's `encoderSaveField` does (several
/// at once from quick-fill, so they arrive together).
async fn save_fields(app: AppState, w: Worker, tomorrow: bool, changes: Vec<(&'static str, String)>, current: EncState) -> Result<(), String> {
    if tomorrow {
        let mut plan = json!({
            "date": tomorrow_iso(), "dailyStatus": current.status, "absentReason": current.reason, "area": current.area,
            "project": current.project, "specificTask": current.task, "isAssigned": true, "encodedAt": chrono::Utc::now().to_rfc3339(),
        });
        for (field, val) in &changes {
            plan[if *field == "status" { "dailyStatus" } else { *field }] = json!(val);
        }
        return match sync::mutate::<_, Value>(app, "Tomorrow's schedule", "PUT", format!("/workers/{}/profile", w.id), &json!({ "tomorrowAssignment": plan })).await {
            Outcome::Saved(_) | Outcome::Queued => Ok(()),
            Outcome::Failed(e) => Err(e),
        };
    }
    let mut body = serde_json::Map::new();
    for (field, val) in changes {
        if field == "status" {
            let req = WorkerStatusRequest { status: WorkerStatus::from_legacy(&val), reason: current.reason.clone() };
            api::put::<_, Worker>(&format!("/workers/{}/status", w.id), &req).await.map_err(|e| e.message())?;
        } else {
            body.insert(if field == "specificTask" { "specific_task".into() } else { field.into() }, json!(val));
        }
    }
    if body.is_empty() {
        return Ok(());
    }
    api::put::<_, Worker>(&format!("/workers/{}/assignment", w.id), &Value::Object(body)).await.map(|_| ()).map_err(|e| e.message())
}

#[component]
pub fn Encoder() -> Element {
    let app = use_app();
    let workers = use_workers();
    let signatories = use_records("signatories");
    let mut mode = use_signal(|| None::<&'static str>);
    let mut view = use_signal(|| "slip");
    let mut group_filter = use_signal(String::new);
    let mut slip = use_signal(String::new);
    let mut slip_q = use_signal(String::new);
    let mut page = use_signal(|| 0usize);
    let mut card = use_signal(|| 0usize);
    let mut modified = use_signal(HashSet::<Uuid>::new);
    // Edits shown at once, before the list reloads: (person, field) → value.
    let mut local = use_signal(HashMap::<(Uuid, &'static str), String>::new);
    let mut m_area = use_signal(String::new);
    let mut m_project = use_signal(String::new);
    let mut m_task = use_signal(String::new);
    let mut overwrite = use_signal(|| false);
    let mut printing = use_signal(|| None::<(&'static str, String, &'static str)>);

    let Some(md) = mode() else {
        return rsx! {
            div { class: "card p-8 flex flex-col items-center gap-5 text-center", "data-testid": "encoder-choice",
                p { class: "text-lg font-semibold", "What are you encoding today?" }
                div { class: "flex flex-wrap justify-center gap-3",
                    button { class: "btn-primary btn-lg", "data-testid": "encoder-today", onclick: move |_| mode.set(Some("today")), Icon { name: "edit" } "Update today's assignments" }
                    button { class: "btn-secondary btn-lg", "data-testid": "encoder-tomorrow", onclick: move |_| mode.set(Some("tomorrow")), Icon { name: "calendar" } "Encode tomorrow's schedule" }
                }
            }
        };
    };
    let tomorrow = md == "tomorrow";
    let mut people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    people.sort_by_key(|w| (w.project_group.to_lowercase(), w.name.to_lowercase()));
    let state = move |w: &Worker| {
        let mut st = state_of(w, tomorrow);
        let l = local.read();
        for f in FIELDS.iter().chain(["status"].iter()) {
            if let Some(v) = l.get(&(w.id, *f)) {
                match *f {
                    "area" => st.area = v.clone(),
                    "project" => st.project = v.clone(),
                    "specificTask" => st.task = v.clone(),
                    _ => st.status = v.clone(),
                }
            }
        }
        st
    };
    let mut save_many = move |w: Worker, changes: Vec<(&'static str, String)>| {
        let current = state(&w);
        let changes: Vec<(&'static str, String)> = changes.into_iter().filter(|(f, v)| get(&current, f) != *v).collect();
        if changes.is_empty() {
            return;
        }
        for (f, v) in &changes {
            local.write().insert((w.id, *f), v.clone());
        }
        modified.write().insert(w.id);
        spawn(async move {
            match save_fields(app, w, tomorrow, changes, current).await {
                Ok(_) => *WORKERS_CHANGED.write() += 1,
                Err(e) => app.error(e),
            }
        });
    };
    let mut save = move |w: Worker, field: &'static str, val: String| save_many(w, vec![(field, val)]);
    let mut groups: Vec<String> = people.iter().map(|w| w.project_group.clone()).filter(|g| !g.trim().is_empty()).collect();
    groups.sort_by_key(|g| g.to_lowercase());
    groups.dedup();
    let filtered: Vec<Worker> = people.iter().filter(|w| group_filter().is_empty() || w.project_group == group_filter()).cloned().collect();
    let mut areas: Vec<String> = people.iter().map(|w| w.area.clone()).filter(|x| !x.is_empty()).collect();
    areas.sort();
    areas.dedup();
    let mut projects: Vec<String> = people.iter().map(|w| w.project.clone()).filter(|x| !x.is_empty()).collect();
    projects.sort();
    projects.dedup();
    let job_kind: &'static str = if tomorrow { "tomorrow" } else { "jo" };

    // ----- Group slip
    let active = if slip().is_empty() || !groups.contains(&slip()) { groups.first().cloned().unwrap_or_default() } else { slip() };
    let encoded = |g: &str| -> (usize, usize) {
        let m: Vec<&Worker> = people.iter().filter(|w| w.project_group == g).collect();
        let done = m.iter().filter(|w| { let st = state(w); modified.read().contains(&w.id) || st.status != "Present" || (st.assigned && !st.task.is_empty()) }).count();
        (done, m.len())
    };
    let done_groups = groups.iter().filter(|g| { let (d, t) = encoded(g); t > 0 && d == t }).count();
    let slip_idx = groups.iter().position(|g| *g == active).unwrap_or(0);
    let members: Vec<Worker> = people.iter().filter(|w| w.project_group == active).cloned().collect();
    let sig_name = signatories
        .read()
        .clone()
        .unwrap_or_default()
        .iter()
        .find(|sg| sg["projectGroups"].as_array().is_some_and(|a| a.iter().any(|x| x.as_str() == Some(active.as_str()))))
        .map(|sg| s(sg, "name"))
        .unwrap_or_else(|| "Not assigned".into());
    let fill = {
        let members = members.clone();
        move || {
            let values = [("area", m_area()), ("project", m_project()), ("specificTask", m_task())];
            for w in &members {
                let st = state(w);
                if st.status == "Absent" {
                    continue;
                }
                let changes: Vec<(&'static str, String)> = values
                    .iter()
                    .filter(|(f, v)| !v.trim().is_empty() && (overwrite() || get(&st, f).trim().is_empty()))
                    .map(|(f, v)| (*f, v.trim().to_string()))
                    .collect();
                save_many(w.clone(), changes);
            }
        }
    };
    let next_slip = {
        let groups = groups.clone();
        move |step: i32| {
            if groups.is_empty() {
                return;
            }
            let i = (slip_idx as i32 + step).clamp(0, groups.len() as i32 - 1) as usize;
            slip.set(groups[i].clone());
            m_area.set(String::new());
            m_project.set(String::new());
            m_task.set(String::new());
        }
    };

    let field_input = move |w: Worker, f: &'static str, st: &EncState, list: &'static str| {
        let v = get(st, f);
        let disabled = st.status == "Absent";
        rsx! {
            input { class: "field h-8 text-sm", list: "{list}", value: "{v}", disabled, "aria-label": "{f} for {w.name}", "data-testid": "enc-{f}",
                onchange: move |e: FormEvent| save(w.clone(), f, e.value()) }
        }
    };

    rsx! {
        div { class: "space-y-3", "data-testid": "encoder",
            onkeydown: {
                let mut fill = fill.clone();
                let mut next_slip = next_slip.clone();
                move |e: KeyboardEvent| {
                    let m = e.modifiers();
                    if m.alt() && e.key() == Key::Character("a".into()) { e.prevent_default(); fill(); }
                    if m.alt() && e.key() == Key::ArrowRight { next_slip(1); }
                    if m.alt() && e.key() == Key::ArrowLeft { next_slip(-1); }
                    if m.ctrl() && e.key() == Key::Enter { next_slip(1); }
                }
            },
            datalist { id: "enc-areas", for a in areas.iter() { option { key: "{a}", value: "{a}" } } }
            datalist { id: "enc-projects", for p in projects.iter() { option { key: "{p}", value: "{p}" } } }
            div { class: "flex flex-wrap items-center gap-2",
                p { class: "font-semibold", if tomorrow { "Encoding tomorrow's schedule" } else { "Updating today's assignments" } }
                span { class: "pill bg-tag-green-bg text-tag-green-text", "data-testid": "enc-updated", "Updated: {modified().len()}" }
                button { class: "btn-ghost h-8", onclick: move |_| { mode.set(None); local.write().clear(); modified.write().clear(); }, "Change" }
                span { class: "flex-1" }
                select { class: "field w-auto h-9", "aria-label": "Group", "data-testid": "enc-group-filter", onchange: move |e| { group_filter.set(e.value()); page.set(0); card.set(0); },
                    option { value: "", "All groups" }
                    for g in groups.iter() { option { key: "{g}", value: "{g}", selected: group_filter() == *g, "{g}" } }
                }
                div { class: "flex rounded-lg border border-line overflow-hidden", role: "group",
                    for (v, l) in [("slip", "Group slip"), ("grid", "Grid"), ("card", "Card")] {
                        button { key: "{v}", class: if view() == v { "px-3 h-9 text-sm bg-accent text-white" } else { "px-3 h-9 text-sm hover:bg-hover" }, "aria-pressed": view() == v, "data-testid": "enc-view-{v}", onclick: move |_| view.set(v), "{l}" }
                    }
                }
            }
            match view() {
                "slip" => rsx! {
                    if groups.is_empty() {
                        div { class: "card", EmptyState { icon: "users", title: "No project groups", body: "Assign people to project groups to use the group slip encoder." } }
                    } else {
                        div { class: "grid lg:grid-cols-[16rem_1fr] gap-3",
                            aside { class: "card p-3 space-y-2", "data-testid": "enc-slip-list",
                                input { class: "field h-8", r#type: "search", placeholder: "Filter group slips…", "data-testid": "enc-slip-search", value: "{slip_q}", oninput: move |e| slip_q.set(e.value()) }
                                p { class: "text-xs text-muted", "Groups encoded: {done_groups} / {groups.len()}" }
                                div { class: "h-1.5 rounded bg-hover overflow-hidden", div { class: "h-full bg-accent", style: "width: {done_groups * 100 / groups.len().max(1)}%" } }
                                div { class: "max-h-[28rem] overflow-y-auto space-y-0.5",
                                    for g in groups.iter().filter(|g| slip_q().is_empty() || g.to_lowercase().contains(&slip_q().to_lowercase())) {
                                        {
                                            let (d, t) = encoded(g);
                                            let g2 = g.clone();
                                            rsx! {
                                                button { key: "{g}", class: if *g == active { "w-full flex justify-between gap-2 px-2 py-1.5 rounded text-sm text-left bg-accent-soft font-medium" } else { "w-full flex justify-between gap-2 px-2 py-1.5 rounded text-sm text-left hover:bg-hover" },
                                                    "data-testid": "enc-slip-item", onclick: move |_| slip.set(g2.clone()),
                                                    span { class: "truncate", "{g}" }
                                                    span { class: if d == t { "text-tag-green-text tabular" } else { "text-muted tabular" }, if d == t { "✓ " } "{d}/{t}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            section { class: "card p-4 space-y-3 min-w-0", "data-testid": "enc-slip",
                                div { class: "flex flex-wrap items-center gap-2",
                                    p { class: "font-semibold", "{active}" span { class: "text-muted font-normal", " ({members.len()} members)" } }
                                    span { class: "pill bg-tag-gray-bg text-tag-gray-text", "Signatory: {sig_name}" }
                                    span { class: "flex-1" }
                                    button { class: "btn-secondary h-8", "data-testid": "enc-print-this", onclick: { let a = active.clone(); move |_| printing.set(Some((job_kind, a.clone(), "group"))) }, Icon { name: "archive" } "This group" }
                                    button { class: "btn-secondary h-8", "data-testid": "enc-print-groups", onclick: move |_| printing.set(Some((job_kind, String::new(), "group"))), Icon { name: "archive" } "Print group JOs" }
                                    button { class: "btn-primary h-8", "data-testid": "enc-print-signatory", onclick: move |_| printing.set(Some((job_kind, String::new(), "signatory"))), Icon { name: "archive" } "Print all (by signatory)" }
                                }
                                div { class: "rounded-lg bg-accent-soft/50 p-3 space-y-2",
                                    p { class: "text-sm font-medium", "Group quick-fill " span { class: "text-muted font-normal", "(set the common area, project and task once; adjust people below)" } }
                                    div { class: "grid sm:grid-cols-[1fr_1fr_1fr_auto_auto] gap-2 items-center",
                                        input { class: "field h-9", list: "enc-areas", placeholder: "Common area", "data-testid": "enc-master-area", value: "{m_area}", oninput: move |e| m_area.set(e.value()) }
                                        input { class: "field h-9", list: "enc-projects", placeholder: "Common project", "data-testid": "enc-master-project", value: "{m_project}", oninput: move |e| m_project.set(e.value()) }
                                        input { class: "field h-9", placeholder: "Common task", "data-testid": "enc-master-task", value: "{m_task}", oninput: move |e| m_task.set(e.value()) }
                                        button { class: "btn-primary h-9", "data-testid": "enc-fill", onclick: { let mut fill = fill.clone(); move |_| fill() }, "Fill group" kbd { class: "ml-1 text-[10px] opacity-70", "Alt+A" } }
                                        label { class: "flex items-center gap-1.5 text-sm", input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "enc-overwrite", checked: overwrite(), onchange: move |e| overwrite.set(e.checked()) } "Overwrite existing" }
                                    }
                                }
                                div { class: "overflow-x-auto",
                                    table { class: "w-full text-sm",
                                        thead { tr { class: "text-left text-muted", th { class: "p-1.5", "Status" } th { class: "p-1.5", "Worker & trade" } th { class: "p-1.5", "Area" } th { class: "p-1.5", "Project" } th { class: "p-1.5", "Specific task" } th { class: "p-1.5", "✓" } } }
                                        tbody {
                                            for w in members.iter().cloned() {
                                                {
                                                    let st = state(&w);
                                                    let done = modified().contains(&w.id) || (st.assigned && !st.task.is_empty());
                                                    let tone = match st.status.as_str() { "Present" => "bg-tag-green-bg text-tag-green-text", "Absent" => "bg-tag-red-bg text-tag-red-text", _ => "bg-tag-amber-bg text-tag-amber-text" };
                                                    let cur = st.status.clone();
                                                    rsx! {
                                                        tr { key: "{w.id}", class: "border-t border-line align-top", "data-testid": "enc-slip-row",
                                                            td { class: "p-1.5",
                                                                button { class: "pill {tone} cursor-pointer", "data-testid": "enc-status", "aria-label": "Status of {w.name}: {cur} (click to change)",
                                                                    onclick: { let w = w.clone(); let cur = cur.clone(); move |_| save(w.clone(), "status", next_status(&cur).to_string()) }, "{cur}" }
                                                            }
                                                            td { class: "p-1.5", p { class: "font-medium", "{w.name}" } p { class: "text-xs text-muted", "#{w.a_number} · {w.trade}" } }
                                                            td { class: "p-1.5", {field_input(w.clone(), "area", &st, "enc-areas")} }
                                                            td { class: "p-1.5", {field_input(w.clone(), "project", &st, "enc-projects")} }
                                                            td { class: "p-1.5", {field_input(w.clone(), "specificTask", &st, "")} }
                                                            td { class: "p-1.5 text-center", if done { span { class: "text-tag-green-text", "✓" } } else { span { class: "text-muted", "-" } } }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "flex items-center justify-between gap-2 pt-2 border-t border-line",
                                    button { class: "btn-secondary h-9", "data-testid": "enc-slip-prev", disabled: slip_idx == 0, onclick: { let mut n = next_slip.clone(); move |_| n(-1) }, "← Previous slip" }
                                    span { class: "text-xs text-muted", "Slip {slip_idx + 1} of {groups.len()} · {active}" }
                                    button { class: "btn-primary h-9", "data-testid": "enc-slip-next", onclick: { let mut n = next_slip.clone(); move |_| n(1) },
                                        if slip_idx + 1 >= groups.len() { "Save & finish" } else { "Save & next slip →" } }
                                }
                            }
                        }
                    }
                },
                "grid" => {
                    let total_pages = filtered.len().div_ceil(PAGE).max(1);
                    let pg = page().min(total_pages - 1);
                    let mut grouped: BTreeMap<String, Vec<Worker>> = BTreeMap::new();
                    for w in filtered.iter().skip(pg * PAGE).take(PAGE) {
                        grouped.entry(if w.project_group.is_empty() { "Unassigned".into() } else { w.project_group.clone() }).or_default().push(w.clone());
                    }
                    rsx! {
                        div { class: "card overflow-x-auto", "data-testid": "enc-grid",
                            if filtered.is_empty() { p { class: "p-4 text-sm text-muted", "No workers match the current filter." } }
                            table { class: "w-full text-sm",
                                thead { tr { class: "text-left text-muted", th { class: "p-1.5", "Name" } th { class: "p-1.5", "Trade" } th { class: "p-1.5", "Area" } th { class: "p-1.5", "Project" } th { class: "p-1.5", "Task" } th { class: "p-1.5" } } }
                                for (g, list) in grouped {
                                    tbody { key: "{g}",
                                        tr { td { colspan: "6", class: "px-1.5 pt-3 pb-1 font-semibold", "{g} " span { class: "text-muted font-normal", "({list.len()})" } } }
                                        for w in list {
                                            {
                                                let st = state(&w);
                                                let badge = if modified().contains(&w.id) { "✓" } else if st.assigned && !st.area.is_empty() && !st.project.is_empty() && !st.task.is_empty() { "📋" } else { "" };
                                                rsx! {
                                                    tr { key: "{w.id}", class: if modified().contains(&w.id) { "bg-tag-green-bg/40" } else { "" }, "data-testid": "enc-grid-row",
                                                        td { class: "p-1.5 whitespace-nowrap", "{w.name}" }
                                                        td { class: "p-1.5 text-muted", "{w.trade}" }
                                                        td { class: "p-1.5", {field_input(w.clone(), "area", &st, "enc-areas")} }
                                                        td { class: "p-1.5", {field_input(w.clone(), "project", &st, "enc-projects")} }
                                                        td { class: "p-1.5", {field_input(w.clone(), "specificTask", &st, "")} }
                                                        td { class: "p-1.5", "{badge}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "flex items-center justify-center gap-3 p-3 text-sm",
                                button { class: "btn-secondary h-8", "data-testid": "enc-page-prev", disabled: pg == 0, onclick: move |_| page.set(pg.saturating_sub(1)), "← Prev" }
                                span { "Page {pg + 1} of {total_pages}" }
                                button { class: "btn-secondary h-8", "data-testid": "enc-page-next", disabled: pg + 1 >= total_pages, onclick: move |_| page.set(pg + 1), "Next →" }
                            }
                        }
                    }
                },
                _ => {
                    let total = filtered.len();
                    let i = card().min(total.saturating_sub(1));
                    match filtered.get(i).cloned() {
                        None => rsx! { div { class: "card p-4 text-sm text-muted", "No workers match the current filter." } },
                        Some(w) => {
                            let st = state(&w);
                            rsx! {
                                div { class: "card p-5 max-w-xl mx-auto space-y-4", "data-testid": "enc-card",
                                    div {
                                        p { class: "flex justify-between gap-2", strong { class: "text-lg", "{w.name}" } span { class: "text-sm text-muted", {if w.project_group.is_empty() { "Unassigned".to_string() } else { w.project_group.clone() }} } }
                                        p { class: "text-sm text-muted", {if w.trade.is_empty() { "No trade".to_string() } else { w.trade.clone() }} " · #{w.a_number}" }
                                    }
                                    if modified().contains(&w.id) { p { class: "text-sm text-tag-green-text", "✅ Saved" } } else if st.assigned { p { class: "text-sm text-muted", "📋 Already has an assignment" } }
                                    div { label { class: "label", "Area" } {field_input(w.clone(), "area", &st, "enc-areas")} }
                                    div { label { class: "label", "Project" } {field_input(w.clone(), "project", &st, "enc-projects")} }
                                    div { label { class: "label", "Task" } {field_input(w.clone(), "specificTask", &st, "")} }
                                    div { class: "flex items-center justify-between",
                                        button { class: "btn-secondary", "data-testid": "enc-card-prev", disabled: i == 0, onclick: move |_| card.set(i.saturating_sub(1)), "← Previous" }
                                        span { class: "text-sm text-muted", "{i + 1} of {total}" }
                                        button { class: "btn-primary", "data-testid": "enc-card-next", disabled: i + 1 >= total, onclick: move |_| card.set(i + 1), "Next →" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            p { class: "text-xs text-muted no-print", "Alt+A fill group · Ctrl+Enter save & next slip · Alt+← / Alt+→ switch slip · Tab moves between fields. Every change saves as you leave the field." }
            if let Some((kind, g, by)) = printing() {
                section { class: "space-y-2 pt-4 border-t border-line", "data-testid": "enc-print",
                    div { class: "flex justify-between no-print", p { class: "font-semibold", "Job orders" } button { class: "btn-ghost h-8", onclick: move |_| printing.set(None), "Close" } }
                    PrintTab { key: "{kind}-{g}-{by}", initial: "jo", start_group: g, start_by: by,
                        date: (chrono::Local::now().date_naive() + chrono::Duration::days(if kind == "tomorrow" { 1 } else { 0 })).format("%Y-%m-%d").to_string() }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_pill_cycles_like_service5() {
        assert_eq!(next_status("Present"), "Absent");
        assert_eq!(next_status("Absent"), "Passes");
        assert_eq!(next_status("Passes"), "Present");
    }
}
