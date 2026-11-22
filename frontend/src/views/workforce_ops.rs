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

/// A person's photo beside their name (Service 5 shows one in every list), or their initials.
#[component]
pub fn WorkerPhoto(id: uuid::Uuid, name: String, has_photo: bool, #[props(default = 28)] size: u32) -> Element {
    if has_photo {
        rsx! { img { src: "/api/workers/{id}/photo", alt: "", loading: "lazy", class: "rounded-full object-cover shrink-0", style: "width: {size}px; height: {size}px;", "data-testid": "worker-photo" } }
    } else {
        rsx! { crate::components::ui::Avatar { name, size } }
    }
}

/// A Service 5 page shown as Service 5 draws it (frontend/public/print/s5view-*.html):
/// its own markup, styles and code over SiteMonitor's records. Read-only.
#[component]
pub fn S5Page(page: &'static str) -> Element {
    rsx! {
        iframe { src: "/print/s5view-{page}.html", title: "{page}", "data-testid": "s5-page-{page}", class: "w-full rounded-md border border-line bg-white", style: "height: calc(100vh - 210px); min-height: 520px;" }
    }
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
                "Assignments" => rsx! { crate::views::assignments::AssignmentsView {} },
                "Departments" => rsx! { S5Page { page: "departments" } },
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
    let workers = use_workers();
    let sigs = use_records("signatories");
    let jobs: Vec<WorkItem> = app.work_items.read().clone().unwrap_or_default().into_iter().filter(|w| w.status.is_active()).collect();
    let can_link = app.can(Permission::ManageWorkforce);
    let meta = use_records("project_groups_meta");
    let meta_map: Value = meta.read().clone().unwrap_or_default().into_iter().next().unwrap_or(json!({}));
    let mut editing = use_signal(|| None::<Option<String>>);
    let mut log = use_signal(|| false);
    let mut grouped = use_signal(|| true);
    let mut area_filter = use_signal(String::new);
    let mut q = use_signal(String::new);
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    let signatories = sigs.read().clone().unwrap_or_default();
    let links = groups.read().clone().unwrap_or_default();
    // Service 5's project group cards: area, members (overseer, leader, assistant first), signatory.
    let rank = |r: &str| match r {
        "Project Overseer" => 0,
        "Project Leader" => 1,
        "Asst. Project Leader" => 2,
        _ => 9,
    };
    let mut names: Vec<String> = links.iter().map(|g| g.name.clone()).collect();
    for w in &people {
        if !w.project_group.is_empty() && !names.contains(&w.project_group) {
            names.push(w.project_group.clone());
        }
    }
    names.sort();
    let cards: Vec<(String, String, Vec<Worker>, Option<uuid::Uuid>)> = names
        .into_iter()
        .map(|name| {
            let mut members: Vec<Worker> = people.iter().filter(|w| w.project_group == name).cloned().collect();
            members.sort_by_key(|w| (rank(&w.group_role), w.name.to_lowercase()));
            let job = links.iter().find(|g| g.name == name).and_then(|g| g.work_item_id);
            (name.clone(), group_area(&name, &members, &meta_map), members, job)
        })
        .collect();
    let mut areas: Vec<String> = cards.iter().map(|c| c.1.clone()).collect();
    areas.sort();
    areas.dedup();
    let total_people: usize = cards.iter().map(|c| c.2.len()).sum();
    let ql = q().to_lowercase();
    let shown: Vec<_> = cards
        .iter()
        .filter(|c| {
            (area_filter().is_empty() || c.1 == area_filter())
                && (ql.is_empty() || c.0.to_lowercase().contains(&ql) || c.1.to_lowercase().contains(&ql) || c.2.iter().any(|w| w.name.to_lowercase().contains(&ql) || w.a_number.contains(&ql)))
        })
        .cloned()
        .collect();
    let mut sections: Vec<(String, Vec<(String, String, Vec<Worker>, Option<uuid::Uuid>)>)> = vec![];
    for c in shown.iter().cloned() {
        let key = if grouped() { c.1.clone() } else { String::new() };
        match sections.iter_mut().find(|x| x.0 == key) {
            Some(x) => x.1.push(c),
            None => sections.push((key, vec![c])),
        }
    }
    sections.sort_by(|a, b| a.0.cmp(&b.0));
    let pill_count = |a: &str| cards.iter().filter(|c| c.1 == a).count();
    let card = move |(name, area, members, job): (String, String, Vec<Worker>, Option<uuid::Uuid>)| -> Element {
        let sig = signatories
            .iter()
            .find(|v| v["projectGroups"].as_array().is_some_and(|a| a.iter().any(|x| x.as_str() == Some(name.as_str()))))
            .map(|v| s(v, "name"))
            .unwrap_or_default();
        let n = members.len();
        let jobs = jobs.clone();
        rsx! {
            div { key: "{name}", class: "card overflow-hidden", "data-testid": "group-row",
                div { class: "p-4 border-b border-line flex justify-between gap-2",
                    div { class: "min-w-0",
                        button { class: "pill bg-tag-blue-bg text-tag-blue-text text-[11px]", title: "Click to filter by {area}", onclick: { let a = area.clone(); move |_| area_filter.set(a.clone()) }, "📍 Area: {area}" }
                        p { class: "font-bold mt-1", "{name}" }
                        p { class: "text-xs text-muted flex gap-2 items-center",
                            {util::plural(n, "member")}
                            span { class: if sig.is_empty() { "pill bg-hover text-[11px]" } else { "pill bg-tag-blue-bg text-tag-blue-text text-[11px]" },
                                "Signatory: " {if sig.is_empty() { "Default".to_string() } else { sig.clone() }}
                            }
                        }
                    }
                    if can_link {
                        div { class: "flex gap-1 shrink-0",
                            button { class: "btn-icon btn-ghost", title: "Reassign Members from this Group", "aria-label": "Reassign members of {name}", "data-testid": "group-reassign",
                                onclick: move |_| { crate::components::ui::PAGE_TABS.write().insert("workforce", "Reassign Volunteers"); }, Icon { name: "move" } }
                            button { class: "btn-icon btn-ghost", title: "Print Group", "aria-label": "Print {name}", "data-testid": "group-print", onclick: { let n = name.clone(); move |_| s5print("projectGroups", json!([{ "group": n }])) }, Icon { name: "archive" } }
                            button { class: "btn-icon btn-ghost", title: "Edit Group", "aria-label": "Edit {name}", "data-testid": "group-edit", onclick: { let n = name.clone(); move |_| editing.set(Some(Some(n.clone()))) }, Icon { name: "edit" } }
                            button { class: "btn-icon btn-ghost text-tag-red-text", title: "Delete Group", "aria-label": "Delete {name}", "data-testid": "group-delete", onclick: { let n = name.clone(); move |_| editing.set(Some(Some(n.clone()))) }, Icon { name: "trash" } }
                        }
                    }
                }
                div { class: "px-4 py-3 space-y-1",
                    if members.is_empty() { p { class: "text-xs text-muted italic text-center py-2", "No members assigned yet" } }
                    for w in members.iter() {
                        div { key: "{w.id}", class: "flex items-center gap-2 py-1", "data-testid": "group-member",
                            WorkerPhoto { id: w.id, name: w.name.clone(), has_photo: w.has_photo }
                            div { class: "flex-1 min-w-0 text-sm",
                                span { class: "font-semibold", "{w.name}" }
                                span { class: "text-xs text-muted ml-1", "#{w.a_number}" }
                                if rank(&w.group_role) < 9 {
                                    span { class: "pill bg-tag-blue-bg text-tag-blue-text text-[10px] ml-1.5",
                                        {match rank(&w.group_role) { 0 => "Overseer", 1 => "Leader", _ => "Asst. Lead" }}
                                    }
                                }
                            }
                            span { class: "text-xs text-muted whitespace-nowrap", "{w.trade}" }
                        }
                    }
                }
                // SiteMonitor: the job this group works on (the Crew page counts who is present).
                div { class: "px-4 pb-3",
                    select { class: "field h-8 text-sm", "aria-label": "Job for {name}", "data-testid": "group-job", disabled: !can_link,
                        onchange: {
                            let name = name.clone();
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
                        option { value: "", selected: job.is_none(), "Not linked to a job" }
                        for j in jobs.iter() { option { key: "{j.id}", value: "{j.id}", selected: job == Some(j.id), "{j.title}" } }
                    }
                }
            }
        }
    };
    rsx! {
        div { class: "card p-4 flex flex-wrap gap-8 no-print", "data-testid": "pg-stats",
            div { p { class: "text-xs text-muted uppercase", "Total Groups" } p { class: "text-2xl font-semibold tabular", "{cards.len()}" } }
            div { p { class: "text-xs text-muted uppercase", "Active Project Areas" } p { class: "text-2xl font-semibold tabular", "{areas.len()}" } }
            div { p { class: "text-xs text-muted uppercase", "Total Assigned Workforce" } p { class: "text-2xl font-semibold tabular", "{total_people} " span { class: "text-sm text-muted font-normal", "volunteers" } } }
        }
        div { class: "flex flex-wrap items-center gap-2 no-print",
            input { class: "field w-auto min-w-[16rem]", r#type: "search", placeholder: "Search groups, leaders, members...", "aria-label": "Search project groups", "data-testid": "pg-search", value: "{q}", oninput: move |e| q.set(e.value()) }
            button { class: if grouped() { "btn-primary h-8" } else { "btn-secondary h-8" }, "data-testid": "groups-by-area", onclick: move |_| grouped.set(true), "By Area" }
            button { class: if grouped() { "btn-secondary h-8" } else { "btn-primary h-8" }, "data-testid": "groups-grid", onclick: move |_| grouped.set(false), "Grid" }
            span { class: "flex-1" }
            button { class: "btn-secondary", "data-testid": "groups-log", onclick: move |_| log.set(true), "Change log" }
            button { class: "btn-secondary", "data-testid": "groups-print", onclick: move |_| s5print("projectGroups", json!([{ "kind": "all" }])), Icon { name: "archive" } "Print All" }
            if can_link {
                button { class: "btn-primary", "data-testid": "group-create", onclick: move |_| editing.set(Some(None)), Icon { name: "plus" } "Create Group" }
            }
        }
        div { class: "flex flex-wrap gap-1.5 no-print", "data-testid": "pg-area-pills",
            button { class: if area_filter().is_empty() { "btn-primary h-7 text-xs" } else { "btn-secondary h-7 text-xs" }, onclick: move |_| area_filter.set(String::new()), "All Areas ({cards.len()})" }
            for a in areas.iter().cloned() {
                button { key: "{a}", class: if area_filter() == a { "btn-primary h-7 text-xs" } else { "btn-secondary h-7 text-xs" },
                    onclick: { let a2 = a.clone(); move |_| area_filter.set(a2.clone()) },
                    "{a} ({pill_count(&a)})"
                }
            }
        }
        if cards.is_empty() {
            div { class: "card", EmptyState { icon: "users", title: "No Project Groups Yet", body: "Create a group to organize volunteers by project." } }
        } else if shown.is_empty() {
            div { class: "card p-6 text-center space-y-2",
                p { class: "text-sm text-muted", "No project groups match the selected filter" }
                button { class: "btn-secondary h-8", onclick: move |_| { area_filter.set(String::new()); q.set(String::new()); }, "Clear Filters" }
            }
        }
        for (area, list) in sections {
            section { key: "{area}", class: "space-y-2",
                if !area.is_empty() {
                    p { class: "font-bold",
                        "📍 {area} "
                        span { class: "text-xs text-muted font-normal", {format!("{} • {} Volunteers", util::plural(list.len(), "Group"), list.iter().map(|c| c.2.len()).sum::<usize>())} }
                    }
                }
                div { class: "grid sm:grid-cols-2 xl:grid-cols-3 gap-3",
                    for c in list { {card(c)} }
                }
            }
        }
        if let Some(name) = editing() {
            crate::views::workforce_extra::GroupModal { name, on_close: move |_| { editing.set(None); groups.restart(); } }
        }
        if log() {
            crate::views::workforce_extra::GroupChangeLog { on_close: move |_| log.set(false) }
        }
    }
}

/// Service 5's area of a project group: the set override, then the part before
/// "|", "-", "/", ":", then the area most of its people work in, else "General".
fn group_area(name: &str, members: &[Worker], meta: &Value) -> String {
    let custom = s(meta, name);
    if !custom.is_empty() {
        return custom;
    }
    if let Some(i) = name.find(['|', '-', '/', ':', '—']) {
        let head = name[..i].trim();
        let rest = &name[i + name[i..].chars().next().map_or(1, char::len_utf8)..];
        if !head.is_empty() && !rest.trim().is_empty() {
            return head.to_string();
        }
    }
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for w in members.iter().filter(|w| !w.area.trim().is_empty()) {
        *counts.entry(w.area.trim()).or_default() += 1;
    }
    counts.into_iter().max_by_key(|(_, n)| *n).map(|(a, _)| a.to_string()).unwrap_or_else(|| "General".into())
}

#[component]
/// Service 5's "Trade Org Charts": one tree per trade group — Katiwala on top,
/// II Katiwala and Kalihim below, then the members; each card can be edited
/// (trade group, trade role) or removed from the group.
fn OrgChartTab() -> Element {
    let app = use_app();
    let workers = use_workers();
    let mut q = use_signal(String::new);
    let mut editing = use_signal(|| None::<Worker>);
    let mut removing = use_signal(|| None::<Worker>);
    let can_edit = app.can(Permission::ManageWorkforce);
    // Service 5 leaves out the placeholder people a backup restore can create ("Volunteer 123" with no details).
    let ghost = |w: &Worker| {
        let n = w.name.trim();
        n.to_lowercase().starts_with("volunteer ") && !n[10..].contains(' ') && w.trade.is_empty() && w.area.is_empty() && w.project_group.is_empty() && w.trade_group.is_empty() && w.birthday.is_empty() && !w.has_photo
    };
    let everyone: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !ghost(w)).collect();
    let mut groups: BTreeMap<String, Vec<Worker>> = BTreeMap::new();
    for w in everyone.iter().cloned() {
        groups.entry(if w.trade_group.is_empty() { "General".into() } else { w.trade_group.clone() }).or_default().push(w);
    }
    let ql = q().to_lowercase();
    groups.retain(|g, m| ql.is_empty() || g.to_lowercase().contains(&ql) || m.iter().any(|w| w.name.to_lowercase().contains(&ql)));
    let shown_ids: Vec<uuid::Uuid> = groups.values().flatten().filter(|w| !w.archived).map(|w| w.id).collect();
    let known: Vec<String> = {
        let mut k: Vec<String> = everyone.iter().map(|w| w.trade_group.clone()).filter(|g| !g.is_empty()).collect();
        k.sort();
        k.dedup();
        k
    };
    let role = |w: &Worker| w.trade_role.trim().to_lowercase();
    let card = move |w: Worker, kind: &'static str| -> Element {
        let ring = match kind { "leader" => "border-2 border-accent", "officer" => "border border-accent/40", _ => "border border-line" };
        rsx! {
            div { key: "{w.id}", class: "card {ring} flex items-center gap-2 p-2 min-w-0", "data-testid": "org-member",
                WorkerPhoto { id: w.id, name: w.name.clone(), has_photo: w.has_photo, size: 36 }
                div { class: "flex-1 min-w-0",
                    p { class: "text-sm font-semibold truncate", title: "{w.name}", "{w.name}" }
                    p { class: "text-xs text-muted truncate", {if w.trade.is_empty() { "Volunteer".to_string() } else { w.trade.clone() }} }
                }
                if can_edit {
                    button { class: "btn-icon btn-ghost h-7 w-7", title: "Edit trade group / role", "aria-label": "Edit {w.name}", "data-testid": "org-edit", onclick: { let w = w.clone(); move |_| editing.set(Some(w.clone())) }, "✏️" }
                    button { class: "btn-icon btn-ghost h-7 w-7", title: "Remove from trade group", "aria-label": "Remove {w.name} from group", "data-testid": "org-remove", onclick: { let w = w.clone(); move |_| removing.set(Some(w.clone())) }, "🗑️" }
                }
            }
        }
    };
    let save = move |w: Worker, group: String, role: String| async move {
        match api::put::<_, Worker>(&format!("/workers/{}/profile", w.id), &json!({ "tradeGroup": group, "tradeRole": role })).await {
            Ok(_) => {
                *WORKERS_CHANGED.write() += 1;
                app.success("Saved");
            }
            Err(e) => app.error(e.message()),
        }
    };
    rsx! {
        div { class: "flex flex-wrap items-center gap-2 no-print",
            p { class: "font-semibold text-lg flex-1", "Trade Org Charts" }
            button { class: "btn-secondary", "data-testid": "org-print-jo", title: "Print Job Order slips for the volunteers shown below",
                onclick: move |_| {
                    *crate::views::workforce_print::PRINT_REQUEST.write() = Some(("jo", shown_ids.clone()));
                    crate::components::ui::PAGE_TABS.write().insert("operations", "Print");
                },
                Icon { name: "archive" } "Print Job Orders"
            }
            button { class: "btn-secondary", "data-testid": "print-tab", onclick: move |_| print_with("size: auto; margin: 8mm 10mm"), Icon { name: "archive" } "Print Charts" }
            input { class: "field w-auto min-w-[14rem]", r#type: "search", placeholder: "Search group or leader...", "aria-label": "Search group or leader", "data-testid": "org-search", value: "{q}", oninput: move |e| q.set(e.value()) }
        }
        if groups.is_empty() {
            div { class: "card p-6 text-center text-sm text-muted", "No trade groups found matching your search." }
        }
        div { class: "grid xl:grid-cols-2 gap-4", "data-testid": "org-chart",
            for (name, members) in groups {
                {
                    let katiwala = members.iter().find(|w| role(w) == "katiwala").cloned();
                    let ii = members.iter().find(|w| role(w).contains("ii katiwala")).cloned();
                    let kalihim = members.iter().find(|w| role(w).contains("kalihim")).cloned();
                    let mut rest: Vec<Worker> = members.iter().filter(|w| !role(w).contains("katiwala") && !role(w).contains("kalihim")).cloned().collect();
                    rest.sort_by(|a, b| a.name.cmp(&b.name));
                    let total = members.len();
                    rsx! {
                        section { key: "{name}", class: "card p-4 space-y-3 break-inside-avoid", "data-testid": "org-group",
                            div { class: "flex items-center justify-between gap-2 border-b border-line pb-2",
                                p { class: "font-bold tracking-wide", "📁 {name.to_uppercase()}" }
                                span { class: "pill bg-hover text-xs", "{total} Volunteers" }
                            }
                            div { class: "flex flex-col items-center gap-1",
                                p { class: "text-[11px] uppercase tracking-wide text-muted", "Katiwala" }
                                div { class: "w-full max-w-xs",
                                    match katiwala { Some(w) => card(w, "leader"), None => rsx! { p { class: "text-xs text-muted italic text-center border border-dashed border-line rounded p-2", "No Katiwala Assigned" } } }
                                }
                            }
                            div { class: "grid grid-cols-2 gap-3",
                                for (label, who) in [("II Katiwala", ii), ("Kalihim", kalihim)] {
                                    div { key: "{label}", class: "flex flex-col items-center gap-1",
                                        p { class: "text-[11px] uppercase tracking-wide text-muted", "{label}" }
                                        div { class: "w-full",
                                            match who { Some(w) => card(w, "officer"), None => rsx! { p { class: "text-xs text-muted italic text-center border border-dashed border-line rounded p-2", "Unassigned" } } }
                                        }
                                    }
                                }
                            }
                            div { class: "space-y-1",
                                p { class: "text-[11px] uppercase tracking-wide text-muted", "Members ({rest.len()})" }
                                div { class: "grid sm:grid-cols-2 gap-2",
                                    if rest.is_empty() { p { class: "text-xs text-muted italic", "No additional members" } }
                                    for w in rest { {card(w, "member")} }
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(w) = editing() {
            OrgEdit { worker: w, known: known.clone(), on_close: move |_| editing.set(None), on_save: move |(w, g, r): (Worker, String, String)| { editing.set(None); spawn(save(w, g, r)); } }
        }
        if let Some(w) = removing() {
            Modal { title: "Remove from trade group".to_string(), on_close: move |_| removing.set(None),
                div { class: "space-y-3",
                    p { class: "text-sm", {format!("Remove {} from \"{}\"? Their Trade Group will be cleared and Trade Role reset to Member.", w.name, if w.trade_group.is_empty() { "this group" } else { w.trade_group.as_str() })} }
                    div { class: "modal-actions",
                        button { class: "btn-ghost", onclick: move |_| removing.set(None), "Cancel" }
                        button { class: "btn-danger", "data-testid": "org-remove-confirm", onclick: { let w = w.clone(); move |_| { removing.set(None); spawn(save(w.clone(), String::new(), "Member".into())); } }, "Remove" }
                    }
                }
            }
        }
    }
}

/// Service 5's org chart edit popover: trade group (with the known ones) and trade role.
#[component]
fn OrgEdit(worker: Worker, known: Vec<String>, on_close: EventHandler<()>, on_save: EventHandler<(Worker, String, String)>) -> Element {
    let mut group = use_signal(|| worker.trade_group.clone());
    let mut role = use_signal(|| if worker.trade_role.is_empty() { "Member".to_string() } else { worker.trade_role.clone() });
    rsx! {
        Modal { title: format!("Edit {}", worker.name), on_close: move |_| on_close.call(()),
            div { class: "space-y-3", "data-testid": "org-edit-popover",
                div {
                    label { class: "label", r#for: "org-edit-tg-input", "Trade Group" }
                    input { id: "org-edit-tg-input", class: "field", list: "org-known-groups", placeholder: "e.g. Electrical Group", autocomplete: "off", "data-testid": "org-edit-group", value: "{group}", oninput: move |e| group.set(e.value()) }
                    datalist { id: "org-known-groups", for g in known { option { key: "{g}", value: "{g}" } } }
                }
                div {
                    label { class: "label", r#for: "org-edit-role", "Trade Role" }
                    select { id: "org-edit-role", class: "field", "data-testid": "org-edit-role", onchange: move |e| role.set(e.value()),
                        for r in ["Member", "Kalihim", "II Katiwala", "Katiwala"] { option { key: "{r}", value: "{r}", selected: role() == r, "{r}" } }
                    }
                }
                div { class: "modal-actions",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button { class: "btn-primary", "data-testid": "org-edit-save", onclick: { let w = worker.clone(); move |_| on_save.call((w.clone(), group().trim().to_string(), role())) }, "Save" }
                }
            }
        }
    }
}

#[component]
fn DependabilityTab() -> Element {
    let mut tab = use_signal(|| *ANALYTICS_TAB.peek());
    use_hook(|| *ANALYTICS_TAB.write() = "dependability");
    let worker = use_signal(String::new);
    let days = use_signal(|| 90u32);
    let data = use_resource(|| async { api::get::<Vec<Value>>("/workforce/dependability").await.unwrap_or_default() });
    rsx! {
        div { class: "flex flex-wrap items-center gap-2 no-print",
            p { class: "font-semibold text-lg mr-2", "Analytics & History" }
            button { class: if tab() == "dependability" { "btn-primary h-8" } else { "btn-ghost h-8" }, "data-testid": "analytics-tab-dependability", onclick: move |_| tab.set("dependability"), "👤 Worker Dependability" }
            button { class: if tab() == "project_history" { "btn-primary h-8" } else { "btn-ghost h-8" }, "data-testid": "analytics-tab-project", onclick: move |_| tab.set("project_history"), "🏗️ Project Roster History" }
            span { class: "flex-1" }
            button { class: "btn-secondary h-8", "data-testid": "print-tab", onclick: move |_| s5print("analytics", json!([{ "kind": "roster", "days": days() }])), "Print Dependability Roster" }
        }
        if tab() == "project_history" {
            crate::views::workforce_extra::ProjectHistory {}
        } else {
            match &*data.read() {
                None => rsx! { Spinner {} },
                Some(list) => rsx! { WorkerDependability { list: list.clone(), worker, days } },
            }
        }
    }
}

/// One attendance record, as Service 5's analytics reads it.
#[derive(Clone, PartialEq)]
struct DepRec {
    date: String,
    ts: i64,
    status: String,
    area: String,
    project: String,
    task: String,
}

/// Service 5's `computeStats`: reliability, versatility and what someone works on most.
#[derive(Clone, PartialEq, Default)]
struct DepStats {
    total: usize,
    present: usize,
    absent: usize,
    passes: usize,
    reliability: Option<u32>,
    versatility: u32,
    areas: Vec<(String, usize, u32)>,
    tasks: Vec<(String, usize, u32)>,
    recs: Vec<DepRec>,
}

/// Service 5's `_rankFreq`: values by how often they appear, with their share.
fn rank_freq(values: impl Iterator<Item = String>) -> Vec<(String, usize, u32)> {
    let vals: Vec<String> = values.filter(|v| !v.is_empty()).collect();
    let total = vals.len().max(1);
    let mut freq: Vec<(String, usize)> = vec![];
    for v in vals {
        match freq.iter_mut().find(|f| f.0 == v) {
            Some(f) => f.1 += 1,
            None => freq.push((v, 1)),
        }
    }
    freq.sort_by(|a, b| b.1.cmp(&a.1));
    freq.into_iter().map(|(v, n)| (v, n, ((n as f64 / total as f64) * 100.0).round() as u32)).collect()
}

fn dep_stats(records: &[Value], days: u32) -> DepStats {
    let cutoff = if days == 0 { i64::MIN } else { js_sys::Date::now() as i64 - days as i64 * 86_400_000 };
    let recs: Vec<DepRec> = records
        .iter()
        .map(|r| DepRec { date: s(r, "date"), ts: r["timestamp"].as_i64().unwrap_or(0), status: s(r, "dailyStatus"), area: s(r, "area"), project: s(r, "project"), task: s(r, "specificTask") })
        .filter(|r| r.ts >= cutoff)
        .collect();
    stats_of(recs)
}

fn stats_of(recs: Vec<DepRec>) -> DepStats {
    let st = |r: &DepRec| r.status.trim().to_lowercase();
    let is_present = |r: &DepRec| matches!(st(r).as_str(), "present" | "p");
    let present = recs.iter().filter(|r| is_present(r)).count();
    let absent = recs.iter().filter(|r| matches!(st(r).as_str(), "absent" | "a")).count();
    let passes = recs.iter().filter(|r| ["pass", "excused", "clinic", "quarantine"].iter().any(|k| st(r).contains(k))).count();
    let total = recs.len();
    let areas = rank_freq(recs.iter().filter(|r| is_present(r)).map(|r| r.area.clone()));
    let tasks = rank_freq(recs.iter().filter(|r| is_present(r)).map(|r| r.task.clone()));
    let versatility = ((areas.len() as f64 / 4.0).min(1.0) * 40.0 + (tasks.len() as f64 / 5.0).min(1.0) * 60.0).round() as u32;
    DepStats {
        total,
        present,
        absent,
        passes,
        reliability: (total > 0).then(|| ((present as f64 / total as f64) * 100.0).round() as u32),
        versatility,
        areas,
        tasks,
        recs,
    }
}

fn reliability_badge(pct: Option<u32>) -> Element {
    match pct {
        None => rsx! { span { class: "text-xs text-muted", "No data" } },
        Some(p) => {
            let (c, l) = if p >= 90 { ("bg-tag-green-bg text-tag-green-text", "Excellent") } else if p >= 70 { ("bg-tag-amber-bg text-tag-amber-text", "Good") } else { ("bg-tag-red-bg text-tag-red-text", "Low") };
            rsx! { span { class: "pill {c} text-[11px]", "{p}% {l}" } }
        }
    }
}

/// Service 5's "Worker Dependability" tab.
#[component]
fn WorkerDependability(list: Vec<Value>, worker: Signal<String>, days: Signal<u32>) -> Element {
    let mut worker = worker;
    let mut days = days;
    let mut search = use_signal(String::new);
    let mut sort = use_signal(|| ("date", true));
    let ql = search().to_lowercase();
    let mut all: Vec<(Value, DepStats)> = list.iter().map(|v| (v.clone(), dep_stats(v["records"].as_array().map(Vec::as_slice).unwrap_or(&[]), days()))).filter(|(_, st)| st.total > 0).collect();
    all.sort_by(|a, b| b.1.reliability.cmp(&a.1.reliability).then(b.1.versatility.cmp(&a.1.versatility)));
    let matches = |v: &Value| ql.is_empty() || [s(v, "name"), s(v, "trade"), s(v, "aNumber")].iter().any(|x| x.to_lowercase().contains(&ql));
    let filtered: Vec<(Value, DepStats)> = all.iter().filter(|(v, _)| matches(v)).cloned().collect();
    let mut names: Vec<String> = list.iter().filter(|v| v["archived"] != json!(true) && matches(v)).map(|v| s(v, "name")).filter(|n| !n.is_empty()).collect();
    names.sort();
    names.dedup();
    let cur = if names.contains(&worker()) { worker() } else { names.first().cloned().unwrap_or_default() };
    let idx = names.iter().position(|n| *n == cur).unwrap_or(0);
    let me = list.iter().find(|v| s(v, "name") == cur).cloned().unwrap_or(json!({}));
    let st = dep_stats(me["records"].as_array().map(Vec::as_slice).unwrap_or(&[]), days());
    let mut recs = st.recs.clone();
    let (col, desc) = sort();
    recs.sort_by(|a, b| {
        let o = match col {
            "status" => a.status.cmp(&b.status),
            "area" => a.area.cmp(&b.area),
            "project" => a.project.cmp(&b.project),
            "task" => a.task.cmp(&b.task),
            _ => a.ts.cmp(&b.ts),
        };
        if desc { o.reverse() } else { o }
    });
    let n = names.len();
    let (prev, next) = (names.get(idx.saturating_sub(1)).cloned(), names.get((idx + 1).min(n.saturating_sub(1))).cloned());
    let rel_color = match st.reliability { Some(p) if p >= 90 => "text-tag-green-text", Some(p) if p >= 70 => "text-tag-amber-text", Some(_) => "text-tag-red-text", None => "text-muted" };
    let id = uuid::Uuid::parse_str(&s(&me, "id")).unwrap_or_default();
    let has_photo = me["hasPhoto"] == json!(true);
    rsx! {
        div { class: "card p-3 flex flex-wrap items-center gap-3 no-print", "data-testid": "dep-filter",
            div { class: "flex items-center gap-1",
                button { class: "btn-secondary h-8 px-2", title: "Previous Worker", "data-testid": "dep-prev", disabled: idx == 0, onclick: move |_| if let Some(p) = prev.clone() { worker.set(p) }, "◀" }
                select { class: "field h-8 w-auto min-w-[11rem] font-semibold", "aria-label": "Worker", "data-testid": "dep-worker", onchange: move |e| worker.set(e.value()),
                    for nm in names.iter() { option { key: "{nm}", value: "{nm}", selected: *nm == cur, "{nm}" } }
                }
                button { class: "btn-secondary h-8 px-2", title: "Next Worker", "data-testid": "dep-next", disabled: idx + 1 >= n, onclick: move |_| if let Some(p) = next.clone() { worker.set(p) }, "▶" }
                span { class: "text-xs text-muted min-w-[3rem] text-center", {if n > 0 { format!("{} of {}", idx + 1, n) } else { "0".into() }} }
            }
            input { class: "field h-8 w-auto min-w-[13rem]", r#type: "search", placeholder: "Search worker, trade, ID...", "aria-label": "Search worker", "data-testid": "dep-search", value: "{search}", oninput: move |e| search.set(e.value()) }
            label { class: "text-sm font-semibold text-muted", r#for: "dep-period", "Period:" }
            select { id: "dep-period", class: "field h-8 w-auto", "data-testid": "dep-period", onchange: move |e| days.set(e.value().parse().unwrap_or(90)),
                for (l, d) in [("Last 30 Days", 30u32), ("Last 90 Days", 90), ("Last 6 Months", 180), ("Last 1 Year", 365), ("All Time", 0)] {
                    option { key: "{d}", value: "{d}", selected: days() == d, "{l}" }
                }
            }
            button { class: "btn-secondary h-8 ml-auto", "data-testid": "dossier-print", onclick: { let c = cur.clone(); move |_| s5print("analytics", json!([{ "worker": c, "days": days() }])) }, Icon { name: "archive" } "Print Dossier" }
        }
        if !cur.is_empty() {
            div { class: "card p-4 flex flex-wrap items-center gap-3", "data-testid": "dep-banner",
                WorkerPhoto { id, name: cur.clone(), has_photo, size: 48 }
                div {
                    p { class: "font-bold text-lg", "{cur}" }
                    p { class: "text-sm text-muted", {if s(&me, "trade").is_empty() { "No trade on file".to_string() } else { s(&me, "trade") }} }
                }
                div { class: "ml-auto flex gap-2",
                    if let Some(a) = st.areas.first() { span { class: "pill bg-tag-blue-bg text-tag-blue-text", "Primary: {a.0}" } }
                    if let Some(t) = st.tasks.first() { span { class: "pill bg-tag-green-bg text-tag-green-text", "Task: {t.0}" } }
                }
            }
            div { class: "grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-3", "data-testid": "dep-kpis",
                for (l, v, sub, c) in [
                    ("Snapshot Days", st.total.to_string(), "recorded appearances".to_string(), ""),
                    ("Present", st.present.to_string(), "days on duty".into(), "text-tag-green-text"),
                    ("Absent", st.absent.to_string(), "days absent".into(), "text-tag-red-text"),
                    ("Passes", st.passes.to_string(), "official passes".into(), "text-tag-amber-text"),
                    ("Reliability Index", st.reliability.map(|p| format!("{p}%")).unwrap_or_else(|| "-".into()), if st.total > 0 { format!("{}/{} present", st.present, st.total) } else { "no data".into() }, rel_color),
                    ("Versatility Score", format!("{}/100", st.versatility), format!("{} areas, {} tasks", st.areas.len(), st.tasks.len()), "text-tag-blue-text"),
                ] {
                    div { key: "{l}", class: "card p-4",
                        p { class: "text-[11px] uppercase tracking-wide font-bold text-muted", "{l}" }
                        p { class: "text-2xl font-extrabold {c}", "{v}" }
                        p { class: "text-xs text-muted", "{sub}" }
                    }
                }
            }
            div { class: "grid md:grid-cols-2 gap-4",
                for (title, items, bar) in [("Top Assigned Areas", st.areas.clone(), "bg-tag-blue-text"), ("Top Specific Tasks", st.tasks.clone(), "bg-tag-green-text")] {
                    div { key: "{title}", class: "card p-4 space-y-2",
                        p { class: "text-xs font-extrabold uppercase tracking-wide text-muted", "{title}" }
                        if items.is_empty() { p { class: "text-sm text-muted italic", {if title.contains("Area") { "No area data" } else { "No task data" }} } }
                        for (v, c, p) in items.into_iter().take(5) {
                            div { key: "{v}", class: "flex items-center gap-3 text-sm",
                                span { class: "w-40 truncate font-semibold", title: "{v}", "{v}" }
                                div { class: "flex-1 h-2.5 rounded-full bg-line overflow-hidden", div { class: "h-full rounded-full {bar}", style: "width: {p}%" } }
                                span { class: "w-20 text-right text-xs text-muted", "{c}x ({p}%)" }
                            }
                        }
                    }
                }
            }
            div { class: "card overflow-hidden", "data-testid": "dep-timeline",
                div { class: "px-4 py-3 border-b border-line flex flex-wrap justify-between gap-2",
                    div {
                        p { class: "font-extrabold text-sm", "📅 Attendance Timeline" }
                        p { class: "text-xs text-muted", "Audit trail of all recorded appearances in selected period • Click headers to sort" }
                    }
                    span { class: "text-xs text-muted", "{st.recs.len()} snapshot record(s)" }
                }
                div { class: "overflow-x-auto",
                    table { class: "w-full text-sm min-w-[680px]",
                        thead { tr { class: "text-left text-muted border-b border-line",
                            for (k, l) in [("date", "Date"), ("status", "Status"), ("area", "Area"), ("project", "Project"), ("task", "Specific Task")] {
                                th { key: "{k}", class: "px-3 py-2 font-medium cursor-pointer select-none", onclick: move |_| { let (c, d) = sort(); sort.set((k, if c == k { !d } else { true })); },
                                    "{l}" if col == k { {if desc { " ▼" } else { " ▲" }} }
                                }
                            }
                        } }
                        tbody { class: "divide-y divide-line",
                            if recs.is_empty() { tr { td { colspan: "5", class: "px-3 py-6 text-center text-muted italic", "No snapshot data in this period." } } }
                            for (i, r) in recs.iter().enumerate() {
                                tr { key: "{i}",
                                    td { class: "px-3 py-2 font-semibold whitespace-nowrap", "{r.date}" }
                                    td { class: "px-3 py-2", span { class: match r.status.as_str() { "Present" => "pill bg-tag-green-bg text-tag-green-text", "Absent" => "pill bg-tag-red-bg text-tag-red-text", "Passes" => "pill bg-tag-amber-bg text-tag-amber-text", _ => "pill bg-hover" }, "{r.status}" } }
                                    td { class: "px-3 py-2", {if r.area.is_empty() { "-".to_string() } else { r.area.clone() }} }
                                    td { class: "px-3 py-2", {if r.project.is_empty() { "-".to_string() } else { r.project.clone() }} }
                                    td { class: "px-3 py-2", {if r.task.is_empty() { "-".to_string() } else { r.task.clone() }} }
                                }
                            }
                        }
                    }
                }
            }
        }
        div { class: "card overflow-hidden", "data-testid": "dep-roster",
            div { class: "px-4 py-3 border-b border-line flex justify-between gap-2",
                p { class: "font-extrabold text-sm", "🎖️ All-Workers Dependability Roster " span { class: "font-normal text-muted", "(click a row to inspect)" } }
                span { class: "text-xs text-muted", {format!("{} worker(s){}", filtered.len(), if ql.is_empty() { " with history" } else { " matching filter" })} }
            }
            div { class: "overflow-x-auto",
                table { class: "w-full text-sm min-w-[900px]",
                    thead { tr { class: "text-left text-muted border-b border-line",
                        for h in ["#", "Name", "Trade", "Reliability", "Versatility", "Primary Area", "Primary Task", "Days"] { th { key: "{h}", class: "px-3 py-2 font-medium", "{h}" } }
                    } }
                    tbody { class: "divide-y divide-line",
                        if filtered.is_empty() { tr { td { colspan: "8", class: "px-3 py-8 text-center text-muted italic", "No personnel found matching the query." } } }
                        for (i, (v, st)) in filtered.into_iter().enumerate() {
                            tr { key: "{s(&v, \"id\")}", class: if s(&v, "name") == cur { "cursor-pointer bg-accent-soft" } else { "cursor-pointer hover:bg-hover" }, "data-testid": "dep-row",
                                onclick: { let nm = s(&v, "name"); move |_| worker.set(nm.clone()) },
                                td { class: "px-3 py-2 text-xs", {format!("#{}", i + 1)} }
                                td { class: "px-3 py-2 font-bold",
                                    span { class: "inline-flex items-center gap-2",
                                        WorkerPhoto { id: uuid::Uuid::parse_str(&s(&v, "id")).unwrap_or_default(), name: s(&v, "name"), has_photo: v["hasPhoto"] == json!(true) }
                                        {s(&v, "name")}
                                    }
                                }
                                td { class: "px-3 py-2 text-muted", {s(&v, "trade")} }
                                td { class: "px-3 py-2", {reliability_badge(st.reliability)} }
                                td { class: "px-3 py-2 font-semibold text-accent-ink", {format!("{}/100", st.versatility)} }
                                td { class: "px-3 py-2", {st.areas.first().map(|a| a.0.clone()).unwrap_or_else(|| "-".into())} }
                                td { class: "px-3 py-2", {st.tasks.first().map(|a| a.0.clone()).unwrap_or_else(|| "-".into())} }
                                td { class: "px-3 py-2 text-muted", "{st.total}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod dependability_tests {
    use super::*;

    fn rec(status: &str, area: &str, task: &str) -> DepRec {
        DepRec { date: String::new(), ts: 0, status: status.into(), area: area.into(), project: String::new(), task: task.into() }
    }

    #[test]
    fn service5_reliability_and_versatility() {
        let st = stats_of(vec![rec("Present", "Hall", "Pour"), rec("Present", "Gate", "Paint"), rec("Absent", "", ""), rec("Passes", "", "")]);
        assert_eq!((st.total, st.present, st.absent, st.passes), (4, 2, 1, 1));
        assert_eq!(st.reliability, Some(50));
        // 2 of 4 areas → 20, 2 of 5 tasks → 24.
        assert_eq!(st.versatility, 44);
        assert_eq!(st.areas[0].2, 50);
        assert_eq!(stats_of(vec![]).reliability, None);
    }
}

/// Which Dependability sub-tab opens next ("Project Roster History" lands on its own tab).
pub static ANALYTICS_TAB: GlobalSignal<&'static str> = Signal::global(|| "dependability");

fn snap_time(v: &Value) -> String {
    match &v["timestamp"] {
        Value::Number(n) => n.as_i64().and_then(chrono::DateTime::from_timestamp_millis).map(|d| d.with_timezone(&Local).format("%-I:%M:%S %p").to_string()).unwrap_or_default(),
        Value::String(x) => chrono::DateTime::parse_from_rfc3339(x).map(|d| d.with_timezone(&Local).format("%-I:%M:%S %p").to_string()).unwrap_or_default(),
        _ => String::new(),
    }
}

fn snap_count(v: &Value, key: &str, present_only: bool) -> usize {
    v[key].as_u64().map(|n| n as usize).unwrap_or_else(|| v["data"].as_array().map(|d| d.iter().filter(|w| !present_only || s(w, "dailyStatus") == "Present").count()).unwrap_or(0))
}

/// Service 5's History page: archived daily snapshots on the left; the chosen
/// one's assignments by area (and who was absent or on pass) on the right.
#[component]
fn HistoryHub() -> Element {
    let snaps = use_records("history");
    let workers = use_workers();
    let mut pick = use_signal(|| None::<usize>);
    let mut list = snaps.read().clone().unwrap_or_default();
    list.sort_by(|a, b| s(b, "dateStr").cmp(&s(a, "dateStr")).then(b["timestamp"].to_string().cmp(&a["timestamp"].to_string())));
    let people = workers.read().clone().unwrap_or_default();
    let chosen = pick().and_then(|i| list.get(i).cloned());
    rsx! {
        div { class: "flex flex-wrap items-center gap-2 no-print",
            p { class: "font-semibold text-lg flex-1", "History" }
            button { class: "btn-secondary h-9", "data-testid": "history-project-roster",
                onclick: move |_| {
                    *ANALYTICS_TAB.write() = "project_history";
                    crate::components::ui::PAGE_TABS.write().insert("operations", "Dependability");
                },
                "🏗️ Project Roster History"
            }
        }
        div { class: "grid md:grid-cols-[18rem_1fr] gap-4", "data-testid": "history-split",
            div { class: "card overflow-hidden",
                p { class: "px-4 py-3 font-semibold border-b border-line", "Archived Snapshots" }
                div { class: "max-h-[70vh] overflow-y-auto", "data-testid": "history-list",
                    if list.is_empty() { p { class: "p-6 text-center text-sm text-muted italic", "No snapshots recorded." } }
                    for (i, v) in list.iter().enumerate() {
                        button { key: "{i}", "data-testid": "history-item",
                            class: if pick() == Some(i) { "w-full text-left px-4 py-3 border-b border-line bg-accent-soft border-l-4 border-l-accent" } else { "w-full text-left px-4 py-3 border-b border-line hover:bg-hover" },
                            onclick: move |_| pick.set(Some(i)),
                            p { class: "font-semibold", {s(v, "dateStr")} }
                            p { class: "text-xs text-muted",
                                span { class: "text-tag-green-text font-medium", {format!("{} Present", snap_count(v, "activeCount", true))} }
                                {format!(" • {} Total", snap_count(v, "totalCount", false))}
                            }
                            p { class: "text-xs text-muted", {snap_time(v)} }
                        }
                    }
                }
            }
            div { class: "card p-4 space-y-4 min-h-[20rem]", "data-testid": "history-viewer",
                if let Some(snap) = chosen {
                    SnapshotView { snap, people }
                } else {
                    p { class: "text-sm text-muted text-center py-16", "Select a snapshot from the left to view assignments." }
                }
            }
        }
    }
}

#[component]
fn SnapshotView(snap: Value, people: Vec<Worker>) -> Element {
    let mut groups: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut away: Vec<Value> = vec![];
    for w in snap["data"].as_array().cloned().unwrap_or_default() {
        let live = people.iter().find(|p| p.a_number == s(&w, "aNumber"));
        let mut h = w.clone();
        if s(&h, "name").is_empty() {
            h["name"] = json!(live.map(|p| p.name.clone()).unwrap_or_else(|| "Unknown Worker".into()));
        }
        if s(&h, "trade").is_empty() {
            h["trade"] = json!(live.map(|p| p.trade.clone()).unwrap_or_else(|| "Unknown Trade".into()));
        }
        if s(&h, "phone").is_empty() {
            h["phone"] = json!(live.map(|p| p.phone.clone()).unwrap_or_default());
        }
        if matches!(s(&h, "dailyStatus").as_str(), "Absent" | "Passes") {
            away.push(h);
        } else {
            groups.entry(s(&h, "area")).or_default().push(h);
        }
    }
    let most = groups.values().map(Vec::len).max().unwrap_or(1).max(1);
    rsx! {
        div { class: "border-b border-line pb-2",
            p { class: "text-xl font-bold", {format!("Snapshot: {}", s(&snap, "dateStr"))} }
            p { class: "text-sm text-muted", {format!("Recorded at {}", snap_time(&snap))} }
        }
        if groups.is_empty() && away.is_empty() { p { class: "text-sm text-muted italic", "Empty snapshot." } }
        for (area, ws) in groups {
            section { key: "{area}", class: "space-y-2", "data-testid": "history-area",
                div { class: "flex items-center gap-3",
                    p { class: "font-bold", {if area.is_empty() { "Unassigned Area".to_string() } else { area.clone() }} }
                    span { class: "pill bg-hover tabular", "{ws.len()}" }
                    div { class: "flex-1 h-1.5 rounded bg-hover", div { class: "h-1.5 rounded bg-accent", style: "width: {ws.len() * 100 / most}%" } }
                }
                div { class: "grid sm:grid-cols-2 gap-2",
                    for w in ws.iter().cloned() {
                        div { key: "{s(&w, \"aNumber\")}", class: "card p-2 flex justify-between gap-2 text-sm",
                            div {
                                p { class: "font-semibold", {s(&w, "name")} span { class: "text-xs text-muted ml-1", {format!("(#{})", s(&w, "aNumber"))} } }
                                p { class: "text-xs text-muted", if s(&w, "specificTask").is_empty() { i { "General Tasking" } } else { {s(&w, "specificTask")} } }
                            }
                            p { class: "text-xs text-muted whitespace-nowrap", {s(&w, "trade")} }
                        }
                    }
                }
            }
        }
        if !away.is_empty() {
            section { class: "space-y-2", "data-testid": "history-away",
                div { class: "flex items-center gap-3",
                    p { class: "font-bold text-tag-red-text", "Absent and On Passes" }
                    span { class: "pill bg-tag-red-bg text-tag-red-text tabular", "{away.len()}" }
                }
                div { class: "grid sm:grid-cols-2 gap-2",
                    for w in away.iter().cloned() {
                        div { key: "{s(&w, \"aNumber\")}", class: "card p-2 flex justify-between gap-2 text-sm",
                            div {
                                p { class: "font-semibold", {s(&w, "name")} span { class: "text-xs text-muted ml-1", {format!("(#{})", s(&w, "aNumber"))} } }
                                p { class: "text-xs text-muted", {format!("{}{}", s(&w, "dailyStatus"), if s(&w, "absentReason").is_empty() { String::new() } else { format!(" - {}", s(&w, "absentReason")) })} }
                                p { class: "text-xs text-muted", {format!("📞 {}", if s(&w, "phone").is_empty() { "N/A".to_string() } else { s(&w, "phone") })} }
                            }
                            p { class: "text-xs text-muted whitespace-nowrap", {s(&w, "trade")} }
                        }
                    }
                }
            }
        }
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
                "Overnight Duty" => rsx! { S5Page { page: "duty" } },
                "Duty History" => rsx! { crate::views::workforce_extra::DutyMonth {} crate::views::workforce_more::DutyHistoryTab {} },
                "Encoder" => rsx! { crate::views::encoder::Encoder {} },
                "Monthly Attendance" => rsx! { AttendanceTab {} },
                "OT Calculator" => rsx! { S5Page { page: "ot-calc" } },
                "OT Entries" => rsx! { S5Page { page: "ot-entries" } },
                "Org Chart" => rsx! { OrgChartTab {} },
                "History Log" => rsx! { HistoryHub {} },
                "Dependability" => rsx! { DependabilityTab {} },
                "Signatories" => rsx! { S5Page { page: "signatories" } },
                "Bulk Update" => rsx! { crate::views::workforce_more::BulkTab {} },
                "Pick Lists" => rsx! { crate::views::workforce_more::PickListsTab {} },
                _ => rsx! { crate::views::workforce_print::PrintTab { initial: "jo" } },
            }
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
