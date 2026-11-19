//! Service 5 workforce and operations screens: groups, departments, org chart,
//! dependability, history, overnight duty, attendance sheet, overtime,
//! visitations, signatories and print (ID cards, 4-up job orders).
//!
//! Most lists are Service 5's own records, read and written through
//! `/api/workforce/records/{list}` with one shared form and table.

use std::collections::{BTreeMap, HashMap};

use chrono::{Datelike, Local, NaiveDate};
use dioxus::prelude::*;
use serde_json::{json, Value};
use shared::api::*;
use shared::*;

use crate::api;
use crate::components::ui::{EmptyState, Icon, Modal, Spinner};
use crate::state::use_app;
use crate::util;
use crate::views::personnel::REFRESH as WORKERS_CHANGED;

/// Bumped after any record change so every list reloads.
pub static RECORDS: GlobalSignal<u32> = Signal::global(|| 0);

pub fn use_workers() -> Resource<Vec<Worker>> {
    use_resource(|| async {
        let _ = WORKERS_CHANGED();
        api::get::<Vec<Worker>>("/workers").await.unwrap_or_default()
    })
}

pub fn use_records(list: &'static str) -> Resource<Vec<Value>> {
    use_resource(move || async move {
        let _ = RECORDS();
        api::get::<Vec<Value>>(&format!("/workforce/records/{list}")).await.unwrap_or_default()
    })
}

pub async fn save_record(list: &str, id: &str, v: &Value) -> Result<Value, api::ApiFail> {
    let r = api::put::<_, Value>(&format!("/workforce/records/{list}/{}", js_sys::encode_uri_component(id)), v).await;
    if r.is_ok() {
        *RECORDS.write() += 1;
    }
    r
}

pub async fn delete_record(list: &str, id: &str) -> Result<(), api::ApiFail> {
    api::send_raw("DELETE", &format!("/workforce/records/{list}/{}", js_sys::encode_uri_component(id)), None).await?;
    *RECORDS.write() += 1;
    Ok(())
}

/// A field of a record as text (numbers too), "" when missing.
pub fn s(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => if *b { "yes".into() } else { "no".into() },
        _ => String::new(),
    }
}

pub fn new_id(prefix: &str) -> String {
    format!("{prefix}_{:x}{:08x}", js_sys::Date::now() as u64, (js_sys::Math::random() * 4_294_967_295.0) as u64)
}

pub fn today() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// Service 5 links records to people by A# (sometimes stored as a number).
pub fn by_a_number(workers: &[Worker]) -> HashMap<String, Worker> {
    workers.iter().filter(|w| !w.a_number.is_empty()).map(|w| (w.a_number.clone(), w.clone())).collect()
}

pub fn name_of(map: &HashMap<String, Worker>, a: &str) -> String {
    map.get(a).map(|w| w.name.clone()).unwrap_or_else(|| if a.is_empty() { "—".into() } else { format!("A# {a}") })
}

// --------------------------------------------------------------- shared UI

#[derive(Clone, PartialEq)]
pub enum Kind {
    Text,
    Long,
    Date,
    Number,
    /// Stores the worker's A#.
    Worker,
    Choice(Vec<String>),
}

pub fn choice(opts: &[&str]) -> Kind {
    Kind::Choice(opts.iter().map(|o| o.to_string()).collect())
}

#[derive(Clone, PartialEq)]
pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: Kind,
}

pub fn f(key: &'static str, label: &'static str, kind: Kind) -> Field {
    Field { key, label, kind }
}

/// One form for every record type: edits the given fields, keeps the rest.
#[component]
pub fn RecordModal(
    title: String,
    list: &'static str,
    id: String,
    fields: Vec<Field>,
    initial: Value,
    on_close: EventHandler<()>,
    #[props(default)] on_saved: Option<EventHandler<Value>>,
) -> Element {
    let app = use_app();
    let workers = use_workers();
    let mut draft = use_signal(|| initial.clone());
    let mut busy = use_signal(|| false);
    let mut confirm_delete = use_signal(|| false);
    // New records start without an id; saved ones (and imported ones) have one.
    let existing = initial.get("id").is_some();
    let people = workers.read().clone().unwrap_or_default();
    let remove = {
        let id = id.clone();
        move |_| {
            let id = id.clone();
            async move {
                busy.set(true);
                match delete_record(list, &id).await {
                    Ok(_) => {
                        app.success("Deleted");
                        on_close.call(());
                    }
                    Err(e) => app.error(e.message()),
                }
                busy.set(false);
            }
        }
    };
    let save = {
        let id = id.clone();
        move |_| {
            let id = id.clone();
            async move {
                busy.set(true);
                match save_record(list, &id, &draft()).await {
                    Ok(v) => {
                        app.success("Saved");
                        if let Some(h) = on_saved {
                            h.call(v);
                        }
                        on_close.call(());
                    }
                    Err(e) => app.error(e.message()),
                }
                busy.set(false);
            }
        }
    };
    rsx! {
        Modal { title, wide: true, on_close: move |_| on_close.call(()),
            div { class: "space-y-4", "data-testid": "record-form",
                div { class: "grid sm:grid-cols-2 gap-4",
                    for fl in fields {
                        div { key: "{fl.key}", class: if matches!(fl.kind, Kind::Long) { "sm:col-span-2" } else { "" },
                            label { class: "label", r#for: "rf-{fl.key}", "{fl.label}" }
                            {
                                let key = fl.key;
                                let value = s(&draft.read(), key);
                                let mut set = move |v: String, num: bool| {
                                    let val = if num { v.parse::<f64>().map(|n| json!(n)).unwrap_or(json!(v)) } else { json!(v) };
                                    draft.write()[key] = val;
                                };
                                match fl.kind.clone() {
                                    Kind::Long => rsx! { textarea { id: "rf-{key}", class: "field", rows: "3", "data-testid": "rf-{key}", value: "{value}", oninput: move |e| set(e.value(), false) } },
                                    Kind::Date => rsx! { input { id: "rf-{key}", class: "field", r#type: "date", "data-testid": "rf-{key}", value: "{value}", oninput: move |e| set(e.value(), false) } },
                                    Kind::Number => rsx! { input { id: "rf-{key}", class: "field tabular", r#type: "number", step: "any", "data-testid": "rf-{key}", value: "{value}", oninput: move |e| set(e.value(), true) } },
                                    Kind::Worker => rsx! {
                                        select { id: "rf-{key}", class: "field", "data-testid": "rf-{key}", onchange: move |e| set(e.value(), false),
                                            option { value: "", selected: value.is_empty(), "Choose a person…" }
                                            for w in people.iter().filter(|w| !w.archived && !w.a_number.is_empty()) {
                                                option { key: "{w.id}", value: "{w.a_number}", selected: w.a_number == value, "{w.name} · A# {w.a_number}" }
                                            }
                                        }
                                    },
                                    Kind::Choice(opts) => rsx! {
                                        select { id: "rf-{key}", class: "field", "data-testid": "rf-{key}", onchange: move |e| set(e.value(), false),
                                            for o in opts {
                                                option { key: "{o}", value: "{o}", selected: *o == value, "{o}" }
                                            }
                                        }
                                    },
                                    Kind::Text => rsx! { input { id: "rf-{key}", class: "field", "data-testid": "rf-{key}", value: "{value}", oninput: move |e| set(e.value(), false) } },
                                }
                            }
                        }
                    }
                }
                div { class: "modal-actions",
                    if existing {
                        if confirm_delete() {
                            button { class: "btn-danger mr-auto", "data-testid": "rf-delete-confirm", disabled: busy(), onclick: remove, "Delete for good" }
                        } else {
                            button { class: "btn-ghost text-tag-red-text mr-auto", "data-testid": "rf-delete", onclick: move |_| confirm_delete.set(true), Icon { name: "trash" } "Delete" }
                        }
                    }
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button { class: "btn-primary", "data-testid": "rf-save", disabled: busy(), onclick: save,
                        if busy() { Spinner {} }
                        "Save"
                    }
                }
            }
        }
    }
}

/// A plain table: one row of text cells per record, with optional row actions.
#[component]
pub fn Table(head: Vec<&'static str>, rows: Vec<(String, Vec<String>)>, #[props(default)] on_edit: Option<EventHandler<String>>, #[props(default)] empty: String) -> Element {
    if rows.is_empty() {
        return rsx! { div { class: "card", EmptyState { icon: "archive", title: if empty.is_empty() { "Nothing here yet".to_string() } else { empty } } } };
    }
    rsx! {
        div { class: "card overflow-x-auto",
            table { class: "w-full text-sm", "data-testid": "record-table",
                thead {
                    tr { class: "text-left text-muted border-b border-line",
                        for h in head { th { key: "{h}", class: "px-3 py-2 font-medium whitespace-nowrap", "{h}" } }
                        if on_edit.is_some() { th { class: "w-10" } }
                    }
                }
                tbody { class: "divide-y divide-line",
                    for (id, cells) in rows {
                        tr { key: "{id}", "data-testid": "record-row",
                            for (i, c) in cells.iter().enumerate() { td { key: "{i}", class: "px-3 py-2 align-top", "{c}" } }
                            if let Some(h) = on_edit {
                                td { class: "px-1",
                                    button { class: "btn-icon btn-ghost", "aria-label": "Edit", "data-testid": "record-edit", onclick: move |_| h.call(id.clone()), Icon { name: "edit" } }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Tabs(tabs: Vec<&'static str>, current: Signal<&'static str>) -> Element {
    let mut current = current;
    rsx! {
        div { class: "flex flex-wrap gap-1 border-b border-line", role: "tablist",
            for t in tabs {
                button {
                    key: "{t}",
                    role: "tab",
                    "aria-selected": current() == t,
                    "data-testid": "tab-{t}",
                    class: if current() == t { "px-3 h-9 text-sm font-medium border-b-2 border-accent text-accent-ink" } else { "px-3 h-9 text-sm text-muted hover:text-ink" },
                    onclick: move |_| current.set(t),
                    "{t}"
                }
            }
        }
    }
}

/// Prints the page with a given paper set-up (e.g. folio or landscape).
pub fn print_with(page_css: &str) {
    let css = format!("@page {{ {page_css} }}");
    let _ = document::eval(&format!(
        "const s = document.createElement('style'); s.id = 'print-page'; s.textContent = {css:?};
         document.getElementById('print-page')?.remove(); document.head.appendChild(s);
         setTimeout(() => window.print(), 50);"
    ));
}

fn status_tone(s: WorkerStatus) -> Tone {
    match s {
        WorkerStatus::Present => Tone::Green,
        WorkerStatus::Absent => Tone::Red,
        WorkerStatus::OnPass => Tone::Amber,
    }
}

// ------------------------------------------------------------ workforce hub

#[component]
pub fn WorkforceView() -> Element {
    let tab = use_signal(|| "Groups");
    rsx! {
        div { class: "space-y-5 rise-in",
            header { class: "space-y-1",
                h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Workforce" }
                p { class: "text-[15px] text-muted", "Project groups and the jobs they work on, departments, the org chart, dependability and the change history." }
            }
            Tabs { tabs: vec!["Groups", "Departments", "Org chart", "Dependability", "History"], current: tab }
            match tab() {
                "Groups" => rsx! { GroupsTab {} },
                "Departments" => rsx! { DepartmentsTab {} },
                "Org chart" => rsx! { OrgChartTab {} },
                "Dependability" => rsx! { DependabilityTab {} },
                _ => rsx! { HistoryTab {} },
            }
        }
    }
}

#[component]
fn GroupsTab() -> Element {
    let app = use_app();
    let mut groups = use_resource(|| async {
        let _ = WORKERS_CHANGED();
        api::get::<Vec<WorkGroup>>("/workforce/groups").await.unwrap_or_default()
    });
    let jobs: Vec<WorkItem> = app.work_items.read().clone().unwrap_or_default().into_iter().filter(|w| w.status.is_active()).collect();
    let can_link = app.can(Permission::ManageWorkforce);
    let list = groups.read().clone().unwrap_or_default();
    if list.is_empty() {
        return rsx! { div { class: "card", EmptyState { icon: "users", title: "No project groups yet", body: "Groups come from each person's project group in Personnel." } } };
    }
    rsx! {
        p { class: "text-sm text-muted", "Link each group to the job it works on: the Crew page then shows how many of its people are present today." }
        ul { class: "card divide-y divide-line", "data-testid": "group-list",
            for g in list {
                li { key: "{g.name}", class: "flex flex-wrap items-center gap-3 px-4 py-3", "data-testid": "group-row",
                    div { class: "flex-1 min-w-[12rem]",
                        p { class: "font-medium", "{g.name}" }
                        p { class: "text-sm text-muted tabular", "{g.present} present · {g.absent} absent · {g.on_pass} on pass · {g.total} in all" }
                    }
                    select {
                        class: "field w-auto min-w-[14rem]",
                        "aria-label": "Job for {g.name}",
                        "data-testid": "group-job",
                        disabled: !can_link,
                        onchange: {
                            let name = g.name.clone();
                            move |e: FormEvent| {
                                let req = GroupLinkRequest { group: name.clone(), work_item_id: uuid::Uuid::parse_str(&e.value()).ok() };
                                async move {
                                    match api::put::<_, Vec<WorkGroup>>("/workforce/groups", &req).await {
                                        Ok(_) => {
                                            app.success("Group linked");
                                            groups.restart();
                                        }
                                        Err(e) => app.error(e.message()),
                                    }
                                }
                            }
                        },
                        option { value: "", selected: g.work_item_id.is_none(), "Not linked to a job" }
                        for j in jobs.iter() {
                            option { key: "{j.id}", value: "{j.id}", selected: g.work_item_id == Some(j.id), "{j.title}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DepartmentsTab() -> Element {
    let workers = use_workers();
    let mut by: BTreeMap<String, [usize; 4]> = BTreeMap::new();
    for w in workers.read().clone().unwrap_or_default().iter().filter(|w| !w.archived) {
        let d = if w.department.is_empty() { "No department".to_string() } else { w.department.clone() };
        let e = by.entry(d).or_default();
        e[0] += 1;
        e[match w.status {
            WorkerStatus::Present => 1,
            WorkerStatus::Absent => 2,
            WorkerStatus::OnPass => 3,
        }] += 1;
    }
    let rows: Vec<(String, Vec<String>)> = by.into_iter().map(|(d, n)| (d.clone(), vec![d, n[0].to_string(), n[1].to_string(), n[2].to_string(), n[3].to_string()])).collect();
    rsx! { Table { head: vec!["Department", "People", "Present", "Absent", "On pass"], rows } }
}

#[component]
fn OrgChartTab() -> Element {
    let workers = use_workers();
    let mut groups: BTreeMap<String, Vec<Worker>> = BTreeMap::new();
    for w in workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived) {
        groups.entry(if w.project_group.is_empty() { "No group".into() } else { w.project_group.clone() }).or_default().push(w);
    }
    // Service 5's ranks: Katiwala leads, II Katiwala assists, Kalihim keeps records.
    let rank = |r: &str| match r.trim().to_lowercase().as_str() {
        "katiwala" => 0,
        "ii katiwala" => 1,
        "kalihim" => 2,
        "" | "member" => 9,
        _ => 5,
    };
    rsx! {
        div { class: "grid sm:grid-cols-2 xl:grid-cols-3 gap-4", "data-testid": "org-chart",
            for (name, mut people) in groups {
                {
                    people.sort_by_key(|w| (rank(&w.group_role), w.name.to_lowercase()));
                    let leaders: Vec<Worker> = people.iter().filter(|w| rank(&w.group_role) < 9).cloned().collect();
                    let members = people.len() - leaders.len();
                    rsx! {
                        section { key: "{name}", class: "card p-4 space-y-2",
                            p { class: "font-semibold", "{name}" }
                            for l in leaders {
                                p { key: "{l.id}", class: "text-sm", span { class: "pill bg-accent-soft text-accent-ink mr-2", "{l.group_role}" } "{l.name}" }
                            }
                            p { class: "text-sm text-muted", "{util::plural(members, \"member\")}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DependabilityTab() -> Element {
    let workers = use_workers();
    let log = use_records("attendance_log");
    let since = (Local::now().date_naive() - chrono::Duration::days(30)).format("%Y-%m-%d").to_string();
    // Days in the last 30 on which someone was marked absent or on pass.
    let mut missed: HashMap<String, (std::collections::BTreeSet<String>, std::collections::BTreeSet<String>)> = HashMap::new();
    for e in log.read().clone().unwrap_or_default() {
        let date = s(&e, "date");
        if date < since {
            continue;
        }
        let slot = missed.entry(s(&e, "aNumber")).or_default();
        match s(&e, "newStatus").as_str() {
            "Absent" => {
                slot.0.insert(date);
            }
            "Passes" | "On Pass" => {
                slot.1.insert(date);
            }
            _ => {}
        }
    }
    let mut rows: Vec<(i64, String, Vec<String>)> = workers
        .read()
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|w| !w.archived)
        .map(|w| {
            let (a, p) = missed.get(&w.a_number).map(|(a, p)| (a.len(), p.len())).unwrap_or((0, 0));
            let score = (100 - (a as i64 * 100 / 30)).clamp(0, 100);
            (score, w.id.to_string(), vec![w.name, w.a_number, a.to_string(), p.to_string(), format!("{score}%")])
        })
        .collect();
    rows.sort_by_key(|r| (r.0, r.2[0].clone()));
    rsx! {
        p { class: "text-sm text-muted", "Last 30 days: days marked absent or on pass. Lowest dependability first." }
        Table { head: vec!["Name", "A#", "Absent days", "Pass days", "Dependability"], rows: rows.into_iter().map(|(_, id, c)| (id, c)).collect() }
    }
}

#[component]
fn HistoryTab() -> Element {
    let att = use_records("attendance_log");
    let asg = use_records("assignment_log");
    let mut query = use_signal(String::new);
    let q = query().to_lowercase();
    let mut all: Vec<(i64, String, Vec<String>)> = vec![];
    for e in att.read().clone().unwrap_or_default() {
        let detail = format!("{} → {}{}", s(&e, "oldStatus"), s(&e, "newStatus"), if s(&e, "reason").is_empty() { String::new() } else { format!(" ({})", s(&e, "reason")) });
        all.push((e["timestamp"].as_i64().unwrap_or(0), s(&e, "id"), vec![s(&e, "date"), s(&e, "workerName"), "Attendance".into(), detail]));
    }
    for e in asg.read().clone().unwrap_or_default() {
        let detail = format!("{}: {} → {}", s(&e, "field"), s(&e, "oldValue"), s(&e, "newValue"));
        all.push((e["timestamp"].as_i64().unwrap_or(0), s(&e, "id"), vec![s(&e, "date"), s(&e, "workerName"), "Assignment".into(), detail]));
    }
    all.retain(|r| q.is_empty() || r.2.iter().any(|c| c.to_lowercase().contains(&q)));
    all.sort_by(|a, b| b.2[0].cmp(&a.2[0]).then(b.0.cmp(&a.0)));
    let total = all.len();
    rsx! {
        input { class: "field", "data-testid": "history-search", placeholder: "Search by name, date or change", value: "{query}", oninput: move |e| query.set(e.value()) }
        p { class: "text-sm text-muted", "Showing the newest {total.min(300)} of {total} changes." }
        Table { head: vec!["Date", "Person", "Kind", "Change"], rows: all.into_iter().take(300).map(|(_, id, c)| (id, c)).collect() }
    }
}

// ---------------------------------------------------------- operations hub

#[component]
pub fn OperationsView() -> Element {
    let tab = use_signal(|| "Overnight duty");
    rsx! {
        div { class: "space-y-5 rise-in",
            header { class: "space-y-1 no-print",
                h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Operations & duty" }
                p { class: "text-[15px] text-muted", "Overnight duty, the monthly attendance sheet, overtime, visitations, signatories and printing." }
            }
            div { class: "no-print", Tabs { tabs: vec!["Overnight duty", "Attendance", "Overtime", "Visitations", "Signatories", "Print"], current: tab } }
            match tab() {
                "Overnight duty" => rsx! { DutyTab {} },
                "Attendance" => rsx! { AttendanceTab {} },
                "Overtime" => rsx! { OvertimeTab {} },
                "Visitations" => rsx! { VisitationsTab {} },
                "Signatories" => rsx! { SignatoriesTab {} },
                _ => rsx! { PrintTab {} },
            }
        }
    }
}

/// Fills each duty area for a date by rotating through its roster, skipping
/// anyone not present today (Service 5's `lastRotationIndex` round-robin).
pub fn rotate(area: &Value, present: &dyn Fn(&str) -> bool) -> (Vec<String>, i64) {
    let roster: Vec<String> = area["roster"].as_array().map(|r| r.iter().map(|x| s(&json!({ "v": x }), "v")).collect()).unwrap_or_default();
    let need = area["workersNeeded"].as_i64().unwrap_or(1).max(0) as usize;
    let mut idx = area["lastRotationIndex"].as_i64().unwrap_or(-1);
    let mut picked = vec![];
    for _ in 0..roster.len() {
        if picked.len() >= need {
            break;
        }
        idx = (idx + 1).rem_euclid(roster.len() as i64);
        let a = &roster[idx as usize];
        if present(a) && !picked.contains(a) {
            picked.push(a.clone());
        }
    }
    (picked, idx)
}

#[component]
fn DutyTab() -> Element {
    let app = use_app();
    let areas = use_records("duty_areas");
    let schedules = use_records("duty_schedules");
    let workers = use_workers();
    let mut date = use_signal(today);
    let mut editing = use_signal(|| None::<(String, Value)>);
    let can_edit = app.can(Permission::ManageWorkforce);
    let people = workers.read().clone().unwrap_or_default();
    let map = by_a_number(&people);
    let area_list = areas.read().clone().unwrap_or_default();
    let day = schedules.read().clone().unwrap_or_default().into_iter().find(|d| s(d, "dateStr") == date());
    let assigned = |area_id: &str| -> Vec<String> {
        day.as_ref()
            .and_then(|d| d["assignments"].as_array().cloned())
            .unwrap_or_default()
            .iter()
            .filter(|a| s(a, "areaId") == area_id)
            .map(|a| name_of(&map, &s(a, "aNumber")))
            .collect()
    };
    let generate = {
        let area_list = area_list.clone();
        let present: HashMap<String, bool> = people.iter().map(|w| (w.a_number.clone(), !w.archived && w.status == WorkerStatus::Present)).collect();
        move |_| {
            let area_list = area_list.clone();
            let present = present.clone();
            async move {
                let mut assignments = vec![];
                for a in &area_list {
                    let (picked, idx) = rotate(a, &|x| present.get(x).copied().unwrap_or(false));
                    for p in picked {
                        assignments.push(json!({ "aNumber": p, "areaId": s(a, "id") }));
                    }
                    let mut next = a.clone();
                    next["lastRotationIndex"] = json!(idx);
                    if let Err(e) = save_record("duty_areas", &s(a, "id"), &next).await {
                        return app.error(e.message());
                    }
                }
                let d = date();
                match save_record("duty_schedules", &d, &json!({ "dateStr": d, "assignments": assignments })).await {
                    Ok(_) => app.success("Duty roster made"),
                    Err(e) => app.error(e.message()),
                }
            }
        }
    };
    let fields = vec![
        f("name", "Duty area", Kind::Text),
        f("shiftTime", "Shift time", Kind::Text),
        f("workersNeeded", "People needed", Kind::Number),
        f("color", "Colour", Kind::Text),
    ];
    rsx! {
        div { class: "flex flex-wrap items-end gap-3",
            div {
                label { class: "label", r#for: "duty-date", "Date" }
                input { id: "duty-date", r#type: "date", class: "field", "data-testid": "duty-date", value: "{date}", oninput: move |e| date.set(e.value()) }
            }
            if can_edit {
                button { class: "btn-primary", "data-testid": "duty-generate", onclick: generate, Icon { name: "refresh" } "Make the roster for this date" }
                button { class: "btn-secondary", "data-testid": "duty-add-area",
                    onclick: move |_| editing.set(Some((new_id("duty"), json!({ "roster": [], "workersNeeded": 1, "lastRotationIndex": -1, "shiftTime": "20:00" })))),
                    Icon { name: "plus" } "Add duty area"
                }
            }
        }
        if area_list.is_empty() {
            div { class: "card", EmptyState { icon: "calendar", title: "No duty areas yet", body: "Add the places that need someone overnight." } }
        }
        div { class: "grid sm:grid-cols-2 xl:grid-cols-3 gap-4", "data-testid": "duty-areas",
            for a in area_list.iter().cloned() {
                {
                    let names = assigned(&s(&a, "id"));
                    let roster = a["roster"].as_array().map(Vec::len).unwrap_or(0);
                    let a2 = a.clone();
                    rsx! {
                        section { key: "{s(&a, \"id\")}", class: "card p-4 space-y-2", "data-testid": "duty-area",
                            div { class: "flex items-start gap-2",
                                span { class: "w-3 h-3 rounded-full mt-1.5 shrink-0", style: "background: {s(&a, \"color\")}" }
                                div { class: "flex-1",
                                    p { class: "font-semibold", "{s(&a, \"name\")}" }
                                    p { class: "text-xs text-muted", "{s(&a, \"shiftTime\")} · needs {s(&a, \"workersNeeded\")} · {roster} on the roster" }
                                }
                                if can_edit {
                                    button { class: "btn-icon btn-ghost", "aria-label": "Edit duty area", onclick: move |_| editing.set(Some((s(&a2, "id"), a2.clone()))), Icon { name: "edit" } }
                                }
                            }
                            if names.is_empty() {
                                p { class: "text-sm text-muted", "Nobody on duty this date." }
                            }
                            for n in names { p { key: "{n}", class: "text-sm", "{n}" } }
                        }
                    }
                }
            }
        }
        if let Some((id, v)) = editing() {
            DutyAreaModal { id, initial: v, fields, on_close: move |_| editing.set(None) }
        }
    }
}

/// Duty area form plus its roster (who takes turns).
#[component]
fn DutyAreaModal(id: String, initial: Value, fields: Vec<Field>, on_close: EventHandler<()>) -> Element {
    let workers = use_workers();
    let mut roster = use_signal(|| initial["roster"].as_array().map(|r| r.iter().map(|x| s(&json!({ "v": x }), "v")).collect::<Vec<_>>()).unwrap_or_default());
    let mut show = use_signal(|| false);
    let people = workers.read().clone().unwrap_or_default();
    let mut base = initial.clone();
    base["roster"] = json!(roster());
    if !show() {
        return rsx! {
            Modal { title: "Who takes turns", wide: true, on_close: move |_| on_close.call(()),
                div { class: "space-y-3",
                    p { class: "text-sm text-muted", "Tick the people on this area's rotation. They are picked in this order, skipping anyone not present." }
                    div { class: "max-h-80 overflow-y-auto grid sm:grid-cols-2 gap-1", "data-testid": "duty-roster",
                        for w in people.into_iter().filter(|w| !w.archived && !w.a_number.is_empty()) {
                            label { key: "{w.id}", class: "flex items-center gap-2 text-sm py-1",
                                input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", checked: roster().contains(&w.a_number),
                                    onchange: move |e| {
                                        let mut r = roster.write();
                                        if e.checked() { r.push(w.a_number.clone()) } else { r.retain(|x| x != &w.a_number) }
                                    }
                                }
                                "{w.name}"
                            }
                        }
                    }
                    div { class: "modal-actions",
                        button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                        button { class: "btn-primary", "data-testid": "duty-roster-next", onclick: move |_| show.set(true), "Next" }
                    }
                }
            }
        };
    }
    rsx! { RecordModal { title: "Duty area".to_string(), list: "duty_areas", id, fields, initial: base, on_close: move |_| on_close.call(()) } }
}

/// The status in force on each day of the month, from the attendance log.
pub fn month_grid(log: &[Value], a: &str, current: WorkerStatus, year: i32, month: u32) -> Vec<char> {
    let mut entries: Vec<(String, i64, String)> =
        log.iter().filter(|e| s(e, "aNumber") == a).map(|e| (s(e, "date"), e["timestamp"].as_i64().unwrap_or(0), s(e, "newStatus"))).collect();
    entries.sort();
    let days = (NaiveDate::from_ymd_opt(if month == 12 { year + 1 } else { year }, if month == 12 { 1 } else { month + 1 }, 1).unwrap()
        - NaiveDate::from_ymd_opt(year, month, 1).unwrap())
    .num_days();
    let letter = |st: &str| match st {
        "Absent" => 'A',
        "Passes" | "On Pass" => 'L',
        _ => 'P',
    };
    let first = format!("{year:04}-{month:02}-01");
    // Status before the month began (or today's if there is no history at all).
    let mut cur = entries.iter().rev().find(|e| e.0 < first).map(|e| letter(&e.2)).unwrap_or(if entries.is_empty() {
        match current {
            WorkerStatus::Present => 'P',
            WorkerStatus::Absent => 'A',
            WorkerStatus::OnPass => 'L',
        }
    } else {
        'P'
    });
    let today = today();
    (1..=days)
        .map(|d| {
            let date = format!("{year:04}-{month:02}-{d:02}");
            if let Some(e) = entries.iter().filter(|e| e.0 == date).last() {
                cur = letter(&e.2);
            }
            if date > today { ' ' } else { cur }
        })
        .collect()
}

#[component]
fn AttendanceTab() -> Element {
    let workers = use_workers();
    let log = use_records("attendance_log");
    let mut month = use_signal(|| Local::now().format("%Y-%m").to_string());
    let (y, m) = month().split_once('-').and_then(|(y, m)| Some((y.parse::<i32>().ok()?, m.parse::<u32>().ok()?))).unwrap_or((Local::now().year(), Local::now().month()));
    let entries = log.read().clone().unwrap_or_default();
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    let grid: Vec<(Worker, Vec<char>)> = people.into_iter().map(|w| {
        let g = month_grid(&entries, &w.a_number, w.status, y, m);
        (w, g)
    }).collect();
    let days = grid.first().map(|g| g.1.len()).unwrap_or(30);
    rsx! {
        div { class: "flex flex-wrap items-end gap-3 no-print",
            div {
                label { class: "label", r#for: "att-month", "Month" }
                input { id: "att-month", r#type: "month", class: "field", "data-testid": "att-month", value: "{month}", oninput: move |e| month.set(e.value()) }
            }
            button { class: "btn-secondary", "data-testid": "print-attendance", onclick: move |_| print_with("size: landscape; margin: 8mm"), Icon { name: "archive" } "Print sheet (landscape)" }
            p { class: "text-sm text-muted", "P present · A absent · L on pass" }
        }
        div { class: "card overflow-x-auto print-sheet", "data-testid": "attendance-sheet",
            p { class: "font-semibold px-3 pt-3 print-only", "Monthly attendance — {month}" }
            table { class: "text-xs tabular",
                thead {
                    tr {
                        th { class: "px-2 py-1 text-left sticky left-0 bg-surface", "Name" }
                        for d in 1..=days { th { key: "{d}", class: "px-1 py-1 w-6 text-center", "{d}" } }
                    }
                }
                tbody {
                    for (w, cells) in grid {
                        tr { key: "{w.id}", class: "border-t border-line",
                            td { class: "px-2 py-1 whitespace-nowrap sticky left-0 bg-surface", "{w.name}" }
                            for (i, c) in cells.into_iter().enumerate() {
                                td { key: "{i}", class: match c { 'A' => "text-center text-tag-red-text font-semibold", 'L' => "text-center text-tag-amber-text font-semibold", _ => "text-center text-muted" }, "{c}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn OvertimeTab() -> Element {
    let app = use_app();
    let ot = use_records("ot_entries");
    let workers = use_workers();
    let mut editing = use_signal(|| None::<(String, Value)>);
    let map = by_a_number(&workers.read().clone().unwrap_or_default());
    let list = ot.read().clone().unwrap_or_default();
    let total: f64 = list.iter().filter_map(|e| e["computedPay"].as_f64()).sum();
    let mut sorted = list.clone();
    sorted.sort_by(|a, b| s(b, "date").cmp(&s(a, "date")));
    let rows: Vec<(String, Vec<String>)> = sorted
        .iter()
        .map(|e| {
            let who = if !s(e, "volunteerId").is_empty() { name_of(&map, &s(e, "volunteerId")) } else { format!("{} people", s(e, "totalCount")) };
            let pay = e["computedPay"].as_f64().map(|p| format!("₱{p:.2}")).unwrap_or_default();
            (s(e, "id"), vec![s(e, "date"), s(e, "title"), who, s(e, "otType"), s(e, "hours"), s(e, "hourlyRate"), pay])
        })
        .collect();
    let fields = vec![
        f("date", "Date", Kind::Date),
        f("title", "Work done", Kind::Text),
        f("volunteerId", "Person", Kind::Worker),
        f("otType", "Kind", choice(&["regular", "rest day", "holiday"])),
        f("hours", "Hours", Kind::Number),
        f("hourlyRate", "Rate per hour", Kind::Number),
        f("description", "Notes", Kind::Long),
    ];
    rsx! {
        div { class: "flex flex-wrap items-center justify-between gap-3",
            p { class: "text-sm text-muted tabular", "data-testid": "ot-total", "{list.len()} entries · total ₱{total:.2}" }
            if app.can(Permission::ManageWorkforce) {
                button { class: "btn-primary", "data-testid": "ot-add", onclick: move |_| editing.set(Some((new_id("ot"), json!({ "date": today(), "otType": "regular", "status": "approved" })))), Icon { name: "plus" } "Add overtime" }
            }
        }
        Table { head: vec!["Date", "Work", "Who", "Kind", "Hours", "Rate", "Pay"], rows, on_edit: move |id: String| {
            if let Some(v) = sorted.iter().find(|e| s(e, "id") == id) { editing.set(Some((id, v.clone()))) }
        } }
        if let Some((id, v)) = editing() {
            RecordModal { title: "Overtime".to_string(), list: "ot_entries", id, fields, initial: v, on_close: move |_| editing.set(None),
                on_saved: move |saved: Value| {
                    // Pay = hours × rate, the Service 5 OT calculator rule.
                    let pay = ((saved["hours"].as_f64().unwrap_or(0.0) * saved["hourlyRate"].as_f64().unwrap_or(0.0)) * 100.0).round() / 100.0;
                    let mut next = saved.clone();
                    next["computedPay"] = json!(pay);
                    let id = s(&saved, "id");
                    spawn(async move { let _ = save_record("ot_entries", &id, &next).await; });
                }
            }
        }
    }
}

#[component]
fn VisitationsTab() -> Element {
    let app = use_app();
    let visits = use_records("visitations");
    let ministers = use_records("ministers");
    let workers = use_workers();
    let mut editing = use_signal(|| None::<(String, Value)>);
    let map = by_a_number(&workers.read().clone().unwrap_or_default());
    let mut list = visits.read().clone().unwrap_or_default();
    list.sort_by(|a, b| s(b, "date").cmp(&s(a, "date")));
    let names: Vec<String> = ministers.read().clone().unwrap_or_default().iter().map(|m| s(m, "name")).filter(|n| !n.is_empty()).collect();
    let can_edit = app.can(Permission::ManageWorkforce);
    let rows: Vec<(String, Vec<String>)> = list
        .iter()
        .map(|v| {
            let who = if s(v, "workerName").is_empty() { name_of(&map, &s(v, "aNumber")) } else { s(v, "workerName") };
            (s(v, "id"), vec![s(v, "date"), who, s(v, "ministerName"), s(v, "status"), s(v, "notes")])
        })
        .collect();
    let fields = vec![
        f("date", "Date", Kind::Date),
        f("aNumber", "Person visited", Kind::Worker),
        f("ministerName", "Minister", if names.is_empty() { Kind::Text } else { Kind::Choice(names) }),
        f("status", "Status", choice(&["scheduled", "completed", "cancelled"])),
        f("notes", "Notes", Kind::Long),
    ];
    let scheduled = list.iter().filter(|v| s(v, "status") == "scheduled").count();
    rsx! {
        div { class: "flex flex-wrap items-center justify-between gap-3",
            p { class: "text-sm text-muted", "{scheduled} scheduled · {list.len()} in all" }
            if can_edit {
                button { class: "btn-primary", "data-testid": "visit-add", onclick: move |_| editing.set(Some((new_id("visit"), json!({ "date": today(), "status": "scheduled", "createdAt": chrono::Utc::now().to_rfc3339() })))), Icon { name: "plus" } "Schedule a visit" }
            }
        }
        Table { head: vec!["Date", "Person", "Minister", "Status", "Notes"], rows, on_edit: move |id: String| {
            if can_edit { if let Some(v) = list.iter().find(|e| s(e, "id") == id) { editing.set(Some((id, v.clone()))) } }
        } }
        if let Some((id, v)) = editing() {
            RecordModal { title: "Visitation".to_string(), list: "visitations", id, fields, initial: v, on_close: move |_| editing.set(None),
                on_saved: move |saved: Value| {
                    // Keep the visited person's name on the record, as Service 5 does.
                    let mut next = saved.clone();
                    if let Some(w) = map.get(&s(&saved, "aNumber")) { next["workerName"] = json!(w.name); }
                    if s(&saved, "status") == "completed" && s(&saved, "completedAt").is_empty() { next["completedAt"] = json!(chrono::Utc::now().to_rfc3339()); }
                    let id = s(&saved, "id");
                    spawn(async move { let _ = save_record("visitations", &id, &next).await; });
                }
            }
        }
    }
}

#[component]
fn SignatoriesTab() -> Element {
    let app = use_app();
    let sigs = use_records("signatories");
    let mut editing = use_signal(|| None::<(String, Value)>);
    let list = sigs.read().clone().unwrap_or_default();
    let join = |v: &Value, k: &str| v[k].as_array().map(|a| a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_else(|| s(v, k));
    let rows: Vec<(String, Vec<String>)> = list.iter().map(|v| (s(v, "id"), vec![s(v, "name"), join(v, "trades"), join(v, "projectGroups"), s(v, "signatureDate")])).collect();
    let fields = vec![f("name", "Name", Kind::Text), f("trades", "Trades (comma separated)", Kind::Text), f("projectGroups", "Project groups (comma separated)", Kind::Text), f("signatureDate", "Signed", Kind::Date)];
    rsx! {
        div { class: "flex justify-end",
            if app.can(Permission::ManageWorkforce) {
                button { class: "btn-primary", "data-testid": "sig-add", onclick: move |_| editing.set(Some((new_id("sig"), json!({ "signatureDate": today() })))), Icon { name: "plus" } "Add signatory" }
            }
        }
        Table { head: vec!["Name", "Trades", "Project groups", "Signed"], rows, on_edit: move |id: String| {
            if let Some(v) = list.iter().find(|e| s(e, "id") == id) {
                let mut v = v.clone();
                for k in ["trades", "projectGroups"] { v[k] = json!(join(&v, k)); }
                editing.set(Some((id, v)));
            }
        } }
        if let Some((id, v)) = editing() {
            RecordModal { title: "Signatory".to_string(), list: "signatories", id, fields, initial: v, on_close: move |_| editing.set(None),
                on_saved: move |saved: Value| {
                    // Service 5 keeps trades and groups as lists.
                    let mut next = saved.clone();
                    for k in ["trades", "projectGroups"] {
                        next[k] = json!(s(&saved, k).split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect::<Vec<_>>());
                    }
                    let id = s(&saved, "id");
                    spawn(async move { let _ = save_record("signatories", &id, &next).await; });
                }
            }
        }
    }
}

#[component]
fn PrintTab() -> Element {
    let workers = use_workers();
    let sigs = use_records("signatories");
    let mut kind = use_signal(|| "id");
    let mut group = use_signal(String::new);
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    let mut groups: Vec<String> = people.iter().map(|w| w.project_group.clone()).filter(|g| !g.is_empty()).collect();
    groups.sort();
    groups.dedup();
    let chosen: Vec<Worker> = people.into_iter().filter(|w| group().is_empty() || w.project_group == group()).collect();
    let signer = sigs.read().clone().unwrap_or_default().into_iter().find(|v| {
        let g = group();
        g.is_empty() || v["projectGroups"].as_array().is_some_and(|a| a.iter().any(|x| x.as_str() == Some(g.as_str())))
    });
    let signer = signer.map(|v| s(&v, "name")).unwrap_or_default();
    let date = util::long_day(chrono::Utc::now());
    rsx! {
        div { class: "flex flex-wrap items-end gap-3 no-print",
            div {
                label { class: "label", r#for: "print-kind", "Print" }
                select { id: "print-kind", class: "field", "data-testid": "print-kind", onchange: move |e| kind.set(if e.value() == "jo" { "jo" } else { "id" }),
                    option { value: "id", selected: kind() == "id", "ID cards" }
                    option { value: "jo", selected: kind() == "jo", "Job orders (4 per folio page)" }
                }
            }
            div {
                label { class: "label", r#for: "print-group", "Project group" }
                select { id: "print-group", class: "field", "data-testid": "print-group", onchange: move |e| group.set(e.value()),
                    option { value: "", "Everyone" }
                    for g in groups { option { key: "{g}", value: "{g}", "{g}" } }
                }
            }
            button { class: "btn-primary", "data-testid": "print-now",
                onclick: move |_| print_with(if kind() == "jo" { "size: 8.5in 13in; margin: 0.4in" } else { "size: A4; margin: 10mm" }),
                Icon { name: "archive" } "Print {chosen.len()}"
            }
        }
        if kind() == "id" {
            div { class: "grid grid-cols-2 sm:grid-cols-3 gap-4 print-grid-ids", "data-testid": "id-cards",
                for w in chosen {
                    div { key: "{w.id}", class: "id-card card p-4 space-y-1 break-inside-avoid",
                        p { class: "text-[10px] uppercase tracking-wider text-muted", "SiteMonitor · Workforce ID" }
                        p { class: "text-lg font-semibold leading-tight", "{w.name}" }
                        p { class: "text-sm tabular", "A# {w.a_number}" }
                        p { class: "text-xs text-muted", "{w.trade} · {w.project_group}" }
                    }
                }
            }
        } else {
            div { class: "grid sm:grid-cols-2 gap-4 job-orders", "data-testid": "job-orders",
                for w in chosen {
                    div { key: "{w.id}", class: "job-order card p-4 space-y-2 break-inside-avoid",
                        p { class: "font-semibold text-center", "JOB ORDER" }
                        p { class: "text-sm", "Name: " b { "{w.name}" } " · A# {w.a_number}" }
                        p { class: "text-sm", "Trade: {w.trade}" }
                        p { class: "text-sm", "Project group: {w.project_group}" }
                        p { class: "text-sm", "Area: {w.area}" }
                        p { class: "text-sm", "Date: {date}" }
                        p { class: "text-sm pt-6 border-t border-line text-center", "{signer}" br {} span { class: "text-xs text-muted", "Signatory" } }
                    }
                }
            }
        }
    }
}

/// Today's attendance buttons used on the Personnel list.
#[component]
pub fn StatusButtons(worker: Worker) -> Element {
    let app = use_app();
    rsx! {
        div { class: "flex gap-1", role: "group", "aria-label": "Today for {worker.name}",
            for st in WorkerStatus::ALL {
                button {
                    key: "{st:?}",
                    class: if worker.status == st { format!("pill h-7 px-2 {}", status_tone(st).pill_class()) } else { "pill h-7 px-2 bg-hover text-muted hover:text-ink".to_string() },
                    "aria-pressed": worker.status == st,
                    "data-testid": "mark-{st:?}",
                    onclick: move |_| {
                        let id = worker.id;
                        async move {
                            match api::put::<_, Worker>(&format!("/workers/{id}/status"), &WorkerStatusRequest { status: st, reason: String::new() }).await {
                                Ok(_) => *WORKERS_CHANGED.write() += 1,
                                Err(e) => app.error(e.message()),
                            }
                        }
                    },
                    {match st { WorkerStatus::Present => "P", WorkerStatus::Absent => "A", WorkerStatus::OnPass => "L" }}
                }
            }
        }
    }
}

/// Shift+R: move someone to another group, area or department.
#[component]
pub fn ReassignModal(worker: Worker, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let mut req = use_signal(|| ReassignRequest { project_group: worker.project_group.clone(), area: worker.area.clone(), department: worker.department.clone(), reason: String::new() });
    let r = req();
    rsx! {
        Modal { title: format!("Reassign {}", worker.name), on_close: move |_| on_close.call(()),
            div { class: "space-y-4", "data-testid": "reassign-form",
                div { label { class: "label", r#for: "ra-group", "Project group" } input { id: "ra-group", class: "field", "data-testid": "ra-group", value: "{r.project_group}", oninput: move |e| req.write().project_group = e.value() } }
                div { label { class: "label", r#for: "ra-area", "Area" } input { id: "ra-area", class: "field", value: "{r.area}", oninput: move |e| req.write().area = e.value() } }
                div { label { class: "label", r#for: "ra-dept", "Department" } input { id: "ra-dept", class: "field", value: "{r.department}", oninput: move |e| req.write().department = e.value() } }
                div { label { class: "label", r#for: "ra-reason", "Why (kept in the history)" } input { id: "ra-reason", class: "field", value: "{r.reason}", oninput: move |e| req.write().reason = e.value() } }
                div { class: "modal-actions",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button { class: "btn-primary", "data-testid": "ra-save",
                        onclick: move |_| {
                            let id = worker.id;
                            async move {
                                match api::put::<_, Worker>(&format!("/workers/{id}/reassign"), &req()).await {
                                    Ok(w) => {
                                        app.success(format!("{} reassigned", w.name));
                                        *WORKERS_CHANGED.write() += 1;
                                        on_close.call(());
                                    }
                                    Err(e) => app.error(e.message()),
                                }
                            }
                        },
                        "Save"
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_skips_absent_people_and_wraps() {
        let area = json!({ "roster": [1, 2, 3], "workersNeeded": 2, "lastRotationIndex": 1 });
        let (picked, idx) = rotate(&area, &|a| a != "1");
        assert_eq!(picked, vec!["3", "2"]);
        assert_eq!(idx, 1);
    }

    #[test]
    fn month_grid_carries_status_forward() {
        let log = vec![
            json!({ "aNumber": "7", "date": "2026-01-03", "newStatus": "Absent", "timestamp": 1 }),
            json!({ "aNumber": "7", "date": "2026-01-05", "newStatus": "Present", "timestamp": 2 }),
        ];
        let g = month_grid(&log, "7", WorkerStatus::Present, 2026, 1);
        assert_eq!(g.len(), 31);
        assert_eq!(&g[..6].iter().collect::<String>(), "PPAAPP");
    }
}
