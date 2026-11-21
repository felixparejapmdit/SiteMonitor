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
use crate::components::ui::{current_tab, EmptyState, Icon, Modal, PageTabs, Spinner};
use crate::services::sync::{self, Outcome};
use crate::state::{use_app, AppState};
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

/// Saves a record. Without signal it is kept on this device and sent when the
/// connection is back (the office's and field edits never block each other).
pub async fn save_record(app: AppState, list: &str, id: &str, v: &Value) -> Result<Value, String> {
    let path = format!("/workforce/records/{list}/{}", js_sys::encode_uri_component(id));
    match sync::mutate::<_, Value>(app, "Workforce record", "PUT", path, v).await {
        Outcome::Saved(saved) => {
            *RECORDS.write() += 1;
            Ok(saved)
        }
        Outcome::Queued => Ok(v.clone()),
        Outcome::Failed(e) => Err(e),
    }
}

pub async fn delete_record(list: &str, id: &str) -> Result<(), api::ApiFail> {
    api::send_raw("DELETE", &format!("/workforce/records/{list}/{}", js_sys::encode_uri_component(id)), None).await?;
    *RECORDS.write() += 1;
    Ok(())
}

/// A field of a record as text (numbers too), "" when missing. `a.b` reaches
/// into a nested object when there is no field with that exact name.
pub fn s(v: &Value, k: &str) -> String {
    let found = v.get(k).or_else(|| if k.contains('.') { k.split('.').try_fold(v, |cur, part| cur.get(part)) } else { None });
    match found {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => if *b { "yes".into() } else { "no".into() },
        _ => String::new(),
    }
}

/// Sets a field; `a.b` writes into a nested object (created when missing).
pub fn set_path(v: &mut Value, k: &str, val: Value) {
    if !k.contains('.') || v.get(k).is_some() {
        v[k] = val;
        return;
    }
    let mut cur = v;
    let parts: Vec<&str> = k.split('.').collect();
    for part in &parts[..parts.len() - 1] {
        if !cur.get(*part).is_some_and(Value::is_object) {
            cur[*part] = json!({});
        }
        cur = &mut cur[*part];
    }
    cur[parts[parts.len() - 1]] = val;
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
    /// A picture kept in the record as a data URL (e.g. visit photo evidence).
    Photo,
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
    /// Save here instead of the record list (e.g. `/workers/{id}/profile`).
    #[props(default)] endpoint: Option<String>,
    /// Offer Delete (existing records only).
    #[props(default = true)] deletable: bool,
) -> Element {
    let app = use_app();
    let workers = use_workers();
    let mut draft = use_signal(|| initial.clone());
    let mut busy = use_signal(|| false);
    let mut confirm_delete = use_signal(|| false);
    // New records start without an id; saved ones (and imported ones) have one.
    let existing = deletable && endpoint.is_none() && initial.get("id").is_some();
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
        let title = title.clone();
        move |_| {
            let id = id.clone();
            let title = title.clone();
            let endpoint = endpoint.clone();
            async move {
                busy.set(true);
                let result = match &endpoint {
                    // A different save target (e.g. a worker's profile) instead of a record list.
                    Some(path) => match sync::mutate::<_, Value>(app, title.clone(), "PUT", path.clone(), &draft()).await {
                        Outcome::Saved(v) => {
                            *WORKERS_CHANGED.write() += 1;
                            Ok(v)
                        }
                        Outcome::Queued => Ok(draft()),
                        Outcome::Failed(e) => Err(e),
                    },
                    None => save_record(app, list, &id, &draft()).await,
                };
                match result {
                    Ok(v) => {
                        app.success("Saved");
                        if let Some(h) = on_saved {
                            h.call(v);
                        }
                        on_close.call(());
                    }
                    Err(e) => app.error(e),
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
                                    set_path(&mut draft.write(), key, val);
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
                                    Kind::Photo => {
                                        let input_id = format!("rf-{}", key.replace('.', "-"));
                                        rsx! {
                                            div { class: "flex items-center gap-3",
                                                if value.starts_with("data:image/") { img { src: "{value}", alt: "", class: "w-16 h-16 rounded-lg object-cover border border-line" } }
                                                input { id: "{input_id}", r#type: "file", accept: "image/jpeg,image/png,image/webp", class: "text-sm", "data-testid": "rf-{key}",
                                                    onchange: move |_| {
                                                        let input_id = input_id.clone();
                                                        async move {
                                                            match crate::services::media_canvas::file_as_data_url(&input_id, 2.0 * 1_048_576.0).await {
                                                                Ok(Some(url)) => set_path(&mut draft.write(), key, json!(url)),
                                                                Ok(None) => {}
                                                                Err(e) => app.error(e),
                                                            }
                                                        }
                                                    }
                                                }
                                                if !value.is_empty() { button { r#type: "button", class: "btn-ghost h-8", onclick: move |_| set_path(&mut draft.write(), key, Value::Null), "Remove" } }
                                            }
                                        }
                                    }
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

/// Prints the page with a given paper set-up (e.g. folio or landscape).
pub fn print_with(page_css: &str) {
    let css = format!("@page {{ {page_css} }}");
    let _ = document::eval(&format!(
        "const s = document.createElement('style'); s.id = 'print-page'; s.textContent = {css:?};
         document.getElementById('print-page')?.remove(); document.head.appendChild(s);
         setTimeout(() => window.print(), 50);"
    ));
}

/// Runs one of Service 5's own print templates (frontend/public/print/s5print.js),
/// e.g. `s5print("personnelReport", json!(["trade"]))`; the args are the call's arguments.
pub fn s5print(report: &str, args: Value) {
    let _ = document::eval(&format!(
        "window.S5Print ? S5Print.{report}({}) : alert('The print templates are still loading. Please try again in a moment.');",
        call_args(&args)
    ));
}

fn call_args(args: &Value) -> String {
    args.as_array().map(|a| a.iter().map(Value::to_string).collect::<Vec<_>>().join(",")).unwrap_or_default()
}

/// Service 5's on-screen print view (job orders, visit sheets): its own page
/// layout in a frame, and "Print now" prints exactly that.
#[component]
pub fn S5Preview(report: &'static str, args: Value) -> Element {
    let call = format!("{report}({})", call_args(&args));
    let shown = call.clone();
    use_effect(use_reactive!(|shown| {
        let _ = document::eval(&format!(
            "(function go(n) {{
                if (!window.S5Print) {{ if (n < 100) setTimeout(function () {{ go(n + 1); }}, 100); return; }}
                S5Print.{shown}.then(function (html) {{ var f = document.getElementById('s5-preview'); if (f) f.srcdoc = html || ''; }});
            }})(0);"
        ));
    }));
    rsx! {
        div { class: "flex justify-end no-print",
            button { class: "btn-primary", "data-testid": "s5-print-now",
                onclick: move |_| { let _ = document::eval("S5Print.printFrame(document.getElementById('s5-preview'))"); },
                Icon { name: "archive" } "Print now"
            }
        }
        iframe { id: "s5-preview", title: "Print preview", "data-testid": "s5-preview", class: "w-full rounded-md border border-line bg-white", style: "height: 78vh;" }
    }
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
    let tabs = crate::views::page_tabs(use_app(), "workforce");
    let tab = current_tab("workforce", &tabs);
    rsx! {
        div { class: "space-y-5 rise-in",
            header { class: "space-y-1",
                h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Workforce" }
                p { class: "text-[15px] text-muted", "The workforce dashboard, departments, ID cards, project groups, visitations, reassignments, attendance lists, update forms and archived personnel." }
            }
            PageTabs { page: "workforce", tabs: tabs.clone() }
            match tab {
                "Dashboard" => rsx! { crate::views::workforce_more::DashboardTab {} crate::views::workforce_pages::DashboardCharts {} crate::views::workforce_extra::DashboardMore {} },
                "Departments" => rsx! { DepartmentsTab {} crate::views::workforce_extra::HousingTables {} },
                "ID Cards" => rsx! { crate::views::workforce_print::PrintTab { initial: "id" } },
                "Project Groups" => rsx! { GroupsTab {} },
                "Visitations" => rsx! { crate::views::workforce_extra::VisitationsHub {} },
                "Reassign Volunteers" => rsx! { crate::views::workforce_pages::ReallocationPanel {} },
                "Attendance Gen" => rsx! { crate::views::workforce_sheets::AttendanceGenerator {} },
                "Update Forms" => rsx! { crate::views::workforce_sheets::UpdateFormsPrint {} },
                "Archived Personnel" => rsx! { crate::views::assignments::AssignmentsView { initial_status: "archived" } },
                _ => rsx! { crate::views::workforce_more::AreasTab {} },
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
    let meta = use_records("project_groups_meta");
    let meta_map: Value = meta.read().clone().unwrap_or_default().into_iter().next().unwrap_or(json!({}));
    let mut editing = use_signal(|| None::<Option<String>>);
    let mut log = use_signal(|| false);
    let mut by_area = use_signal(|| false);
    let mut list = groups.read().clone().unwrap_or_default();
    if by_area() {
        list.sort_by_key(|g| (s(&meta_map, &g.name).to_lowercase(), g.name.to_lowercase()));
    }
    let tools = rsx! {
        div { class: "flex flex-wrap items-center gap-2 no-print",
            p { class: "text-sm text-muted flex-1 min-w-[16rem]", "Link each group to the job it works on: the Crew page then shows how many of its people are present today." }
            label { class: "flex items-center gap-2 text-sm",
                input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "groups-by-area", checked: by_area(), onchange: move |e| by_area.set(e.checked()) }
                "Grouped by area"
            }
            button { class: "btn-secondary", "data-testid": "groups-log", onclick: move |_| log.set(true), "Change log" }
            button { class: "btn-secondary", "data-testid": "groups-print", onclick: move |_| s5print("projectGroups", json!([{ "kind": "all" }])), Icon { name: "archive" } "Print All" }
            if can_link {
                button { class: "btn-primary", "data-testid": "group-create", onclick: move |_| editing.set(Some(None)), Icon { name: "plus" } "Create group" }
            }
        }
        crate::views::workforce_extra::GroupRosters { by_area: by_area() }
        if let Some(name) = editing() {
            crate::views::workforce_extra::GroupModal { name, on_close: move |_| { editing.set(None); groups.restart(); } }
        }
        if log() {
            crate::views::workforce_extra::GroupChangeLog { on_close: move |_| log.set(false) }
        }
    };
    if list.is_empty() {
        return rsx! { {tools} div { class: "card", EmptyState { icon: "users", title: "No project groups yet", body: "Create one, or set the project group of each person in Personnel." } } };
    }
    rsx! {
        {tools}
        ul { class: "card divide-y divide-line no-print", "data-testid": "group-list",
            for g in list {
                li { key: "{g.name}", class: "flex flex-wrap items-center gap-3 px-4 py-3", "data-testid": "group-row",
                    div { class: "flex-1 min-w-[12rem]",
                        p { class: "font-medium", "{g.name}" if !s(&meta_map, &g.name).is_empty() { span { class: "text-muted font-normal text-sm ml-2", {s(&meta_map, &g.name)} } } }
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
                    if can_link {
                        button { class: "btn-icon btn-ghost", "aria-label": "Print {g.name}", title: "Print", "data-testid": "group-print", onclick: { let n = g.name.clone(); move |_| s5print("projectGroups", json!([{ "group": n }])) }, Icon { name: "archive" } }
                        button { class: "btn-icon btn-ghost", "aria-label": "Edit {g.name}", "data-testid": "group-edit", onclick: { let n = g.name.clone(); move |_| editing.set(Some(Some(n.clone()))) }, Icon { name: "edit" } }
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
                    let everyone = people.clone();
                    let members = people.len() - leaders.len();
                    rsx! {
                        section { key: "{name}", class: "card p-4 space-y-2",
                            p { class: "font-semibold", "{name}" }
                            for l in leaders {
                                p { key: "{l.id}", class: "text-sm", span { class: "pill bg-accent-soft text-accent-ink mr-2", "{l.group_role}" } "{l.name}" }
                            }
                            p { class: "text-sm text-muted", "{util::plural(members, \"member\")}" }
                            crate::views::workforce_pages::OrgMembers { people: everyone }
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

/// Service 5's org chart buttons: print the charts, or job orders for everyone shown.
#[component]
fn OrgChartPrints() -> Element {
    let workers = use_workers();
    rsx! {
        div { class: "flex justify-end gap-2 no-print",
            button { class: "btn-secondary", "data-testid": "print-tab", onclick: move |_| print_with("size: auto; margin: 8mm 10mm"), Icon { name: "archive" } "Print Charts" }
            button { class: "btn-secondary", "data-testid": "org-print-jo", title: "Print Job Order slips for the volunteers shown below",
                onclick: move |_| {
                    let ids: Vec<uuid::Uuid> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).map(|w| w.id).collect();
                    *crate::views::workforce_print::PRINT_REQUEST.write() = Some(("jo", ids));
                    crate::components::ui::PAGE_TABS.write().insert("operations", "Print");
                },
                Icon { name: "archive" } "Print Job Orders"
            }
        }
    }
}

/// Service 5's "Analytics & History": the change log, project roster history, and person dossiers.
#[component]
fn HistoryHub() -> Element {
    let view = use_signal(|| "Change log");
    rsx! {
        crate::views::workforce_extra::Segmented { options: vec!["Change log", "Project roster history", "Person dossier"], current: view, testid: "history-view" }
        match view() {
            "Project roster history" => rsx! { crate::views::workforce_extra::ProjectHistory {} },
            "Person dossier" => rsx! { crate::views::workforce_extra::Dossier {} },
            _ => rsx! { HistoryTab {} },
        }
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
    let tabs = crate::views::page_tabs(use_app(), "operations");
    let tab = current_tab("operations", &tabs);
    rsx! {
        div { class: "space-y-5 rise-in",
            header { class: "space-y-1 no-print",
                h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Operations" }
                p { class: "text-[15px] text-muted", "Overnight duty, the encoder, attendance, overtime, the org chart, history, dependability, signatories and printing." }
            }
            PageTabs { page: "operations", tabs: tabs.clone() }
            match tab {
                "Overnight Duty" => rsx! { DutyTab {} crate::views::workforce_pages::DutyOverrides {} },
                "Duty History" => rsx! { crate::views::workforce_extra::DutyMonth {} crate::views::workforce_more::DutyHistoryTab {} },
                "Encoder" => rsx! { crate::views::encoder::Encoder {} },
                "Monthly Attendance" => rsx! { AttendanceTab {} },
                "OT Calculator" => rsx! { crate::views::workforce_extra::OtSummary {} },
                "OT Entries" => rsx! { OvertimeTab {} },
                "Org Chart" => rsx! { OrgChartPrints {} OrgChartTab {} },
                "History Log" => rsx! { HistoryHub {} },
                "Dependability" => rsx! {
                    div { class: "flex justify-end no-print",
                        button { class: "btn-secondary", "data-testid": "print-tab", onclick: move |_| s5print("analytics", json!([{ "kind": "roster", "days": 90 }])), Icon { name: "archive" } "Print Dependability Roster" }
                    }
                    DependabilityTab {}
                },
                "Signatories" => rsx! { crate::views::workforce_pages::SignatoriesBoard {} },
                "Bulk Update" => rsx! { crate::views::workforce_more::BulkTab {} },
                "Pick Lists" => rsx! { crate::views::workforce_more::PickListsTab {} },
                _ => rsx! { crate::views::workforce_print::PrintTab { initial: "jo" } },
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
    let mut clearing = use_signal(|| "");
    let can_edit = app.can(Permission::ManageDuty);
    let people = workers.read().clone().unwrap_or_default();
    let map = by_a_number(&people);
    let area_list = areas.read().clone().unwrap_or_default();
    // Service 5's Clear Day / Clear Week: empty the chosen night, or Monday to Sunday of its week.
    let clear = move |span: &'static str| async move {
        let Ok(d) = chrono::NaiveDate::parse_from_str(&date(), "%Y-%m-%d") else { return };
        use chrono::Datelike;
        let dates: Vec<String> = if span == "day" {
            vec![date()]
        } else {
            let monday = d - chrono::Duration::days(d.weekday().num_days_from_monday() as i64);
            (0..7).map(|i| (monday + chrono::Duration::days(i)).format("%Y-%m-%d").to_string()).collect()
        };
        let days: Vec<Value> = schedules.read().clone().unwrap_or_default().into_iter().filter(|x| dates.contains(&s(x, "dateStr"))).collect();
        let mut cleared = 0;
        for x in days {
            cleared += x["assignments"].as_array().map(Vec::len).unwrap_or(0);
            if let Err(e) = save_record(app, "duty_schedules", &s(&x, "dateStr"), &json!({ "dateStr": s(&x, "dateStr"), "assignments": [] })).await {
                return app.error(e);
            }
        }
        clearing.set("");
        if cleared == 0 { app.success("No assignments found to clear.") } else { app.success(format!("Cleared {cleared} duty assignments")) }
    };
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
                    if let Err(e) = save_record(app, "duty_areas", &s(a, "id"), &next).await {
                        return app.error(e);
                    }
                }
                let d = date();
                match save_record(app, "duty_schedules", &d, &json!({ "dateStr": d, "assignments": assignments })).await {
                    Ok(_) => app.success("Duty roster made"),
                    Err(e) => app.error(e),
                }
            }
        }
    };
    // Service 5 "Generate week": seven nights from the chosen date, the rotation carrying on.
    let week = {
        let area_list = area_list.clone();
        let present: HashMap<String, bool> = people.iter().map(|w| (w.a_number.clone(), !w.archived && w.status == WorkerStatus::Present)).collect();
        move |_| {
            let mut areas = area_list.clone();
            let present = present.clone();
            async move {
                let Ok(start) = NaiveDate::parse_from_str(&date(), "%Y-%m-%d") else { return };
                for n in 0..7 {
                    let d = (start + chrono::Duration::days(n)).format("%Y-%m-%d").to_string();
                    let mut assignments = vec![];
                    for a in areas.iter_mut() {
                        let (picked, idx) = rotate(a, &|x| present.get(x).copied().unwrap_or(false));
                        for p in picked {
                            assignments.push(json!({ "aNumber": p, "areaId": s(a, "id") }));
                        }
                        a["lastRotationIndex"] = json!(idx);
                    }
                    if let Err(e) = save_record(app, "duty_schedules", &d, &json!({ "dateStr": d, "assignments": assignments })).await {
                        return app.error(e);
                    }
                }
                for a in &areas {
                    if let Err(e) = save_record(app, "duty_areas", &s(a, "id"), a).await {
                        return app.error(e);
                    }
                }
                app.success("A week of duty made");
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
                button { class: "btn-secondary", "data-testid": "duty-week", onclick: week, "Make the week" }
                if clearing().is_empty() {
                    button { class: "btn-ghost text-tag-red-text", "data-testid": "duty-clear-day", onclick: move |_| clearing.set("day"), "Clear day" }
                    button { class: "btn-ghost text-tag-red-text", "data-testid": "duty-clear-week", onclick: move |_| clearing.set("week"), "Clear week" }
                    button { class: "btn-secondary", "data-testid": "duty-print", title: "Print schedule", onclick: move |_| s5print("duty", json!([{ "date": date() }])), Icon { name: "archive" } "Print" }
                } else {
                    span { class: "text-sm", if clearing() == "day" { "Clear every area on {date()}?" } else { "Clear every area for that whole week?" } }
                    button { class: "btn-danger", "data-testid": "duty-clear-confirm", onclick: move |_| clear(clearing()), "Clear" }
                    button { class: "btn-ghost", onclick: move |_| clearing.set(""), "Keep" }
                }
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
    let mut pasting = use_signal(|| false);
    let mut day = use_signal(today);
    let mut every_day = use_signal(|| false);
    let mut trade = use_signal(String::new);
    let mut who_q = use_signal(String::new);
    let mut show_rate = use_signal(|| true);
    let mut clearing = use_signal(|| false);
    let people = workers.read().clone().unwrap_or_default();
    let map = by_a_number(&people);
    let all = ot.read().clone().unwrap_or_default();
    // Service 5's OT Entries filters: trade and a volunteer name / A# search.
    let person_of = |e: &Value| map.get(&s(e, "volunteerId")).cloned();
    let trade_of = |e: &Value| if s(e, "trade").is_empty() { person_of(e).map(|w| w.trade).unwrap_or_default() } else { s(e, "trade") };
    let mut trades: Vec<String> = all.iter().map(trade_of).filter(|t| !t.is_empty()).collect();
    trades.sort();
    trades.dedup();
    let ql = who_q().to_lowercase();
    let list: Vec<Value> = all
        .iter()
        .filter(|e| every_day() || s(e, "date") == day())
        .filter(|e| trade().is_empty() || trade_of(e) == trade())
        .filter(|e| ql.is_empty() || name_of(&map, &s(e, "volunteerId")).to_lowercase().contains(&ql) || s(e, "volunteerId").contains(&ql))
        .cloned()
        .collect();
    let total: f64 = list.iter().filter_map(|e| e["computedPay"].as_f64()).sum();
    let mut sorted = list.clone();
    sorted.sort_by(|a, b| s(b, "date").cmp(&s(a, "date")));
    let mut step = move |n: i64| {
        if let Ok(d) = NaiveDate::parse_from_str(&day(), "%Y-%m-%d") {
            day.set((d + chrono::Duration::days(n)).format("%Y-%m-%d").to_string());
        }
    };
    // Service 5 "Load yesterday": copy the entries of the day before to this day, hours reset to 0.
    let copy_yesterday = {
        let all = all.clone();
        move |_| {
            let all = all.clone();
            async move {
                let Ok(d) = NaiveDate::parse_from_str(&day(), "%Y-%m-%d") else { return };
                let prev = (d - chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
                let from: Vec<Value> = all.iter().filter(|e| s(e, "date") == prev).cloned().collect();
                if from.is_empty() {
                    return app.error(format!("No overtime on {prev} to copy."));
                }
                for e in &from {
                    let mut next = e.clone();
                    next["id"] = json!(new_id("ot"));
                    next["date"] = json!(day());
                    next["hours"] = json!(0);
                    next["computedPay"] = json!(0);
                    if let Err(err) = save_record(app, "ot_entries", &s(&next, "id"), &next).await {
                        return app.error(err);
                    }
                }
                app.success(format!("{} copied from {prev}", util::plural(from.len(), "entry")));
            }
        }
    };
    let rows: Vec<(String, Vec<String>)> = sorted
        .iter()
        .map(|e| {
            let who = if !s(e, "volunteerId").is_empty() { name_of(&map, &s(e, "volunteerId")) } else { format!("{} people", s(e, "totalCount")) };
            let pay = e["computedPay"].as_f64().map(|p| format!("₱{p:.2}")).unwrap_or_default();
            let level = if s(e, "skillLevel").is_empty() { person_of(e).map(|w| w.skill_level).unwrap_or_default() } else { s(e, "skillLevel") };
            let area = if s(e, "otArea").is_empty() { s(e, "title") } else { s(e, "otArea") };
            let task = if s(e, "otTask").is_empty() { s(e, "description") } else { s(e, "otTask") };
            let mut cells = vec![s(e, "date"), who, trade_of(e), level, s(e, "hours"), area, task];
            if show_rate() {
                cells.push(s(e, "hourlyRate"));
            }
            cells.push(pay);
            (s(e, "id"), cells)
        })
        .collect();
    let head: Vec<&'static str> = if show_rate() {
        vec!["Date", "Name", "Trade", "Level", "Hours", "Area", "Task", "Rate", "Pay"]
    } else {
        vec!["Date", "Name", "Trade", "Level", "Hours", "Area", "Task", "Pay"]
    };
    // Service 5 "Clear": remove the entries shown (this day, or every day).
    let clear_ids: Vec<String> = list.iter().map(|e| s(e, "id")).collect();
    let clear = move |_| {
        let ids = clear_ids.clone();
        async move {
            for id in &ids {
                if let Err(e) = delete_record("ot_entries", id).await {
                    return app.error(e.message());
                }
            }
            clearing.set(false);
            app.success(format!("{} cleared", util::plural(ids.len(), "entry")));
        }
    };
    let fields = vec![
        f("date", "Date", Kind::Date),
        f("volunteerId", "Person", Kind::Worker),
        f("otArea", "Area (where)", Kind::Text),
        f("otTask", "Task / work done", Kind::Text),
        f("title", "Title", Kind::Text),
        f("otType", "Kind", choice(&["regular", "rest day", "holiday"])),
        f("hours", "Hours", Kind::Number),
        f("hourlyRate", "Rate per hour", Kind::Number),
        f("description", "Notes", Kind::Long),
    ];
    rsx! {
        div { class: "flex flex-wrap items-center gap-2 no-print",
            button { class: "btn-icon btn-ghost", "aria-label": "Previous day", "data-testid": "ot-prev", onclick: move |_| step(-1), Icon { name: "chevron-left" } }
            input { r#type: "date", class: "field w-auto", "aria-label": "Day", "data-testid": "ot-day", value: "{day}", oninput: move |e| day.set(e.value()) }
            button { class: "btn-icon btn-ghost", "aria-label": "Next day", "data-testid": "ot-next", onclick: move |_| step(1), Icon { name: "chevron-right" } }
            button { class: "btn-ghost", "data-testid": "ot-today", onclick: move |_| day.set(today()), "Today" }
            label { class: "flex items-center gap-2 text-sm",
                input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "ot-all-days", checked: every_day(), onchange: move |e| every_day.set(e.checked()) }
                "All days"
            }
            // Service 5's OT Entries prints: the schedule by area and the timesheet matrix.
            button { class: "btn-primary", "data-testid": "ot-schedule-print", title: "Print Overtime Schedule (By Area)",
                onclick: move |_| { let r = if every_day() { String::new() } else { day() }; s5print("overtime", json!([{ "kind": "schedule", "start": r, "end": r }])) }, Icon { name: "archive" } "OT Schedule" }
            button { class: "btn-secondary", "data-testid": "ot-matrix-print", title: "Timesheet Matrix (per volunteer)",
                onclick: move |_| { let r = if every_day() { String::new() } else { day() }; s5print("overtime", json!([{ "kind": "matrix", "start": r, "end": r }])) }, "Matrix" }
            if app.can(Permission::ManageOvertime) {
                button { class: "btn-secondary", "data-testid": "ot-copy-yesterday", onclick: copy_yesterday, "Load from yesterday" }
            }
        }
        div { class: "flex flex-wrap items-end gap-2 no-print",
            div {
                label { class: "label", r#for: "ot-trade", "Trade" }
                select { id: "ot-trade", class: "field w-auto", "data-testid": "ot-trade", onchange: move |e| trade.set(e.value()),
                    option { value: "", "All trades" }
                    for t in trades.iter() { option { key: "{t}", value: "{t}", selected: trade() == *t, "{t}" } }
                }
            }
            div {
                label { class: "label", r#for: "ot-who", "Volunteer" }
                input { id: "ot-who", r#type: "search", class: "field w-auto", placeholder: "Name or A#", "data-testid": "ot-who", value: "{who_q}", oninput: move |e| who_q.set(e.value()) }
            }
            label { class: "flex items-center gap-2 text-sm pb-2",
                input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "ot-show-rate", checked: show_rate(), onchange: move |e| show_rate.set(e.checked()) }
                "Rate"
            }
            if app.can(Permission::ManageOvertime) && !list.is_empty() {
                if clearing() {
                    span { class: "text-sm pb-2", "Remove the {list.len()} entries shown?" }
                    button { class: "btn-danger", "data-testid": "ot-clear-confirm", onclick: clear, "Clear" }
                    button { class: "btn-ghost", onclick: move |_| clearing.set(false), "Keep" }
                } else {
                    button { class: "btn-ghost text-tag-red-text", "data-testid": "ot-clear", onclick: move |_| clearing.set(true), "Clear" }
                }
            }
        }
        div { class: "flex flex-wrap items-center justify-between gap-3 no-print",
            p { class: "text-sm text-muted tabular", "data-testid": "ot-total", "{list.len()} entries · grand total ₱{total:.2}" }
            if app.can(Permission::ManageOvertime) {
                button { class: "btn-secondary", "data-testid": "ot-paste-open", onclick: move |_| pasting.set(true), Icon { name: "edit" } "From a list of names" }
                button { class: "btn-primary", "data-testid": "ot-add", onclick: move |_| editing.set(Some((new_id("ot"), json!({ "date": day(), "otType": "regular", "status": "approved" })))), Icon { name: "plus" } "Add overtime" }
            }
        }
        if pasting() { crate::views::workforce_more::OtPaste { on_close: move |_| pasting.set(false) } }
        if app.can(Permission::ManageOvertime) && !every_day() { crate::views::workforce_pages::OtEncoder { day: day() } }
        p { class: "font-semibold print-only", "Overtime schedule {day}" }
        Table { head, rows, on_edit: move |id: String| {
            if let Some(v) = sorted.iter().find(|e| s(e, "id") == id) { editing.set(Some((id, v.clone()))) }
        } }
        if let Some((id, v)) = editing() {
            RecordModal { title: "Overtime".to_string(), list: "ot_entries", id, fields, initial: v, on_close: move |_| editing.set(None),
                on_saved: move |saved: Value| {
                    // Pay = hours × rate, the Service 5 OT calculator rule.
                    // The rate defaults to the hourly rate on the person record.
                    let rate = saved["hourlyRate"].as_f64().or_else(|| map.get(&s(&saved, "volunteerId")).and_then(|w| w.hourly_rate)).unwrap_or(0.0);
                    let pay = ((saved["hours"].as_f64().unwrap_or(0.0) * rate) * 100.0).round() / 100.0;
                    let mut next = saved.clone();
                    next["hourlyRate"] = json!(rate);
                    next["computedPay"] = json!(pay);
                    let id = s(&saved, "id");
                    spawn(async move { let _ = save_record(app, "ot_entries", &id, &next).await; });
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
                            match sync::mutate::<_, Worker>(app, "Attendance", "PUT", format!("/workers/{id}/status"), &WorkerStatusRequest { status: st, reason: String::new() }).await {
                                Outcome::Saved(_) => *WORKERS_CHANGED.write() += 1,
                                Outcome::Queued => {}
                                Outcome::Failed(e) => app.error(e),
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
                                match sync::mutate::<_, Worker>(app, "Reassign", "PUT", format!("/workers/{id}/reassign"), &req()).await {
                                    Outcome::Saved(w) => {
                                        app.success(format!("{} reassigned", w.name));
                                        *WORKERS_CHANGED.write() += 1;
                                        on_close.call(());
                                    }
                                    Outcome::Queued => on_close.call(()),
                                    Outcome::Failed(e) => app.error(e),
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
