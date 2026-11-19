//! Personnel (merged from Service 5 DB): every worker and volunteer, plus the
//! workforce data tools (legacy import, backup download, snapshots).

use dioxus::prelude::*;
use shared::api::*;
use shared::*;
use uuid::Uuid;
use wasm_bindgen::JsCast;

use crate::api;
use crate::components::ui::{EmptyState, Icon, Modal, Pill, Spinner};
use crate::services::media_canvas;
use crate::state::use_app;
use crate::util;

/// Bumped after any change so the list reloads.
pub static REFRESH: GlobalSignal<u32> = Signal::global(|| 0);

#[derive(Clone, Copy, PartialEq)]
enum Filter {
    All,
    Status(WorkerStatus),
    Archived,
}

#[component]
pub fn PersonnelView() -> Element {
    let app = use_app();
    let can_edit = app.can(Permission::ManageWorkforce);
    let workers = use_resource(|| async {
        let _ = REFRESH();
        api::get::<Vec<Worker>>("/workers").await
    });
    let mut query = use_signal(String::new);
    let mut filter = use_signal(|| Filter::All);
    let mut editing = use_signal(|| None::<Option<Worker>>);
    let mut reassigning = use_signal(|| None::<Worker>);
    let mut profiling = use_signal(|| None::<Worker>);
    let mut f_trade = use_signal(String::new);
    let mut f_group = use_signal(String::new);
    let mut f_area = use_signal(String::new);
    let mut f_house = use_signal(String::new);
    let mut f_cat = use_signal(String::new);
    let mut hide_assigned = use_signal(|| false);
    let mut picked = use_signal(std::collections::HashSet::<Uuid>::new);
    let mut action = use_signal(|| None::<Bulk>);
    let mut new_day = use_signal(|| false);
    let mut show_cols = use_signal(|| false);
    // Extra details shown on each row (Service 5's "Columns"), remembered on this device.
    let mut cols = use_signal(|| util::storage_get(COLS_KEY).unwrap_or_else(|| "task,project".into()).split(',').filter(|s| !s.is_empty()).map(String::from).collect::<Vec<_>>());

    let all = match &*workers.read() {
        Some(Ok(w)) => w.clone(),
        Some(Err(e)) => return rsx! { div { class: "card", EmptyState { icon: "users", title: "Couldn't load personnel", body: e.message() } } },
        None => return rsx! { div { class: "card p-8 flex justify-center", Spinner { size: 20 } } },
    };
    let active: Vec<&Worker> = all.iter().filter(|w| !w.archived).collect();
    let count = |s: WorkerStatus| active.iter().filter(|w| w.status == s).count();
    let q = query().trim().to_lowercase();
    let choices = |get: fn(&Worker) -> &String| {
        let mut v: Vec<String> = all.iter().map(|w| get(w).trim().to_string()).filter(|s| !s.is_empty()).collect();
        v.sort_by_key(|s| s.to_lowercase());
        v.dedup();
        v
    };
    let (trades, groups, areas, housings) = (choices(|w| &w.trade), choices(|w| &w.project_group), choices(|w| &w.area), choices(|w| &w.housing));
    let shown: Vec<Worker> = all
        .iter()
        .filter(|w| match filter() {
            Filter::All => !w.archived,
            Filter::Status(s) => !w.archived && w.status == s,
            Filter::Archived => w.archived && (f_cat().is_empty() || w.archive_category == f_cat()),
        })
        .filter(|w| {
            q.is_empty()
                || [&w.name, &w.a_number, &w.trade, &w.area, &w.department, &w.project_group, &w.task, &w.phone].iter().any(|f| f.to_lowercase().contains(&q))
        })
        .filter(|w| (f_trade().is_empty() || w.trade.trim() == f_trade()) && (f_group().is_empty() || w.project_group.trim() == f_group()))
        .filter(|w| (f_area().is_empty() || w.area.trim() == f_area()) && (f_house().is_empty() || w.housing.trim() == f_house()))
        .filter(|w| !hide_assigned() || !w.assigned_today)
        .cloned()
        .collect();
    let chosen: Vec<Worker> = all.iter().filter(|w| picked().contains(&w.id)).cloned().collect();
    let all_picked = !shown.is_empty() && shown.iter().all(|w| picked().contains(&w.id));
    let shown_ids: Vec<Uuid> = shown.iter().map(|w| w.id).collect();
    let cols_now = cols();
    let clinic = app.can(Permission::ManageClinic);
    let column_choices: Vec<(&'static str, &'static str)> = EXTRA.iter().copied().filter(|(k, _)| *k != "medical" || clinic).collect();
    let csv_rows = if chosen.is_empty() { shown.clone() } else { chosen.clone() };
    let print_rows = csv_rows.clone();
    let export_csv = move |_| {
        let mut head = vec!["A#", "Name", "Status", "Trade", "Department", "Area", "Project group", "Role", "Project", "Task", "Remarks", "Phone", "Spouse", "Residence", "Housing"];
        if clinic {
            head.push("Medical");
        }
        let mut out = head.join(",") + "\n";
        for w in &csv_rows {
            let mut cells = vec![
                w.a_number.clone(), w.name.clone(), w.status.label().to_string(), w.trade.clone(), w.department.clone(), w.area.clone(), w.project_group.clone(),
                w.group_role.clone(), w.project.clone(), w.task.clone(), w.remarks.clone(), w.phone.clone(), w.spouse.clone(), w.residence.clone(), w.housing.clone(),
            ];
            if clinic {
                cells.push(w.medical.clone());
            }
            out += &cells.iter().map(|c| csv_cell(c)).collect::<Vec<_>>().join(",");
            out.push('\n');
        }
        save_file(&format!("personnel-{}.csv", chrono::Local::now().format("%Y-%m-%d")), &out);
    };
    let chips = [
        (Filter::All, "Total active", active.len()),
        (Filter::Status(WorkerStatus::Present), "Present", count(WorkerStatus::Present)),
        (Filter::Status(WorkerStatus::Absent), "Absent", count(WorkerStatus::Absent)),
        (Filter::Status(WorkerStatus::OnPass), "On pass", count(WorkerStatus::OnPass)),
        (Filter::Archived, "Archived", all.len() - active.len()),
    ];

    rsx! {
        div { class: "space-y-6 rise-in",
            header { class: "flex flex-wrap items-end justify-between gap-4",
                div { class: "space-y-1",
                    h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Personnel" }
                    p { class: "text-[15px] text-muted", "Every worker and volunteer, with today's status. Press Ctrl+K anywhere to search." }
                }
                div { class: "flex flex-wrap gap-2 no-print",
                    if can_edit {
                        button { class: "btn-secondary h-10", "data-testid": "start-day", title: "Tomorrow's assignments become today's; everyone else is unassigned", onclick: move |_| new_day.set(true),
                            Icon { name: "refresh" }
                            "Start new day"
                        }
                    }
                    button { class: "btn-secondary h-10", "data-testid": "export-csv", onclick: export_csv, Icon { name: "archive" } "Download CSV" }
                    button { class: "btn-secondary h-10", "data-testid": "print-list", onclick: move |_| crate::views::workforce_ops::print_with("size: landscape; margin: 8mm"), Icon { name: "archive" } "Print list" }
                    if can_edit {
                        button { class: "btn-primary h-10 px-4", "data-testid": "add-worker", onclick: move |_| editing.set(Some(None)),
                            Icon { name: "plus" }
                            "Add worker"
                        }
                    }
                }
            }
            div { class: "grid grid-cols-2 sm:grid-cols-5 gap-3", "data-testid": "workforce-counts",
                for (f, label, n) in chips {
                    button {
                        key: "{label}",
                        class: if filter() == f { "card p-3 text-left ring-2 ring-accent" } else { "card p-3 text-left hover:bg-hover transition-colors" },
                        "aria-pressed": filter() == f,
                        onclick: move |_| filter.set(f),
                        p { class: "text-xs text-muted", "{label}" }
                        p { class: "text-2xl font-semibold tabular", "data-testid": "count-{label}", "{n}" }
                    }
                }
            }
            input {
                id: "worker-search",
                class: "field",
                "data-testid": "worker-search",
                r#type: "search",
                autofocus: true,
                placeholder: "Search by name, A#, trade, area or group",
                value: "{query}",
                oninput: move |e| query.set(e.value()),
            }
            div { class: "flex flex-wrap items-center gap-2 no-print", "data-testid": "personnel-filters",
                FilterSelect { id: "f-trade", label: "Trade", all: "All trades", options: trades, value: f_trade(), on_change: move |v| f_trade.set(v) }
                FilterSelect { id: "f-group", label: "Project group", all: "All project groups", options: groups, value: f_group(), on_change: move |v| f_group.set(v) }
                FilterSelect { id: "f-area", label: "Area", all: "All areas", options: areas, value: f_area(), on_change: move |v| f_area.set(v) }
                FilterSelect { id: "f-house", label: "Housing", all: "All housing groups", options: housings, value: f_house(), on_change: move |v| f_house.set(v) }
                if filter() == Filter::Archived {
                    FilterSelect { id: "f-cat", label: "Archive reason", all: "All archived", options: ARCHIVE_CATEGORIES.iter().map(|c| c.to_string()).collect::<Vec<_>>(), value: f_cat(), on_change: move |v| f_cat.set(v) }
                }
                label { class: "flex items-center gap-2 text-sm px-1",
                    input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "hide-assigned", checked: hide_assigned(), onchange: move |e| hide_assigned.set(e.checked()) }
                    "Hide assigned"
                }
                div { class: "relative ml-auto",
                    button { class: "btn-secondary", "aria-expanded": show_cols(), "data-testid": "columns", onclick: move |_| show_cols.toggle(), Icon { name: "layers" } "Columns" }
                    if show_cols() {
                        div { class: "absolute right-0 top-11 z-30 w-56 card shadow-pop p-2 scale-in", "data-testid": "columns-menu",
                            for (k, label) in column_choices {
                                label { key: "{k}", class: "flex items-center gap-2 text-sm px-2 py-1.5 rounded hover:bg-hover",
                                    input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "col-{k}", checked: cols_now.iter().any(|c| c == k),
                                        onchange: move |e| {
                                            let mut c = cols.write();
                                            c.retain(|x| x != k);
                                            if e.checked() { c.push(k.to_string()); }
                                            util::storage_set(COLS_KEY, &c.join(","));
                                        }
                                    }
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }
            if can_edit && !chosen.is_empty() {
                BulkBar { count: chosen.len(), archived_view: filter() == Filter::Archived, on_pick: move |b| action.set(Some(b)), on_clear: move |_| picked.write().clear() }
            }
            PrintList { workers: print_rows, cols: cols_now.clone() }
            if all.is_empty() {
                div { class: "card",
                    EmptyState { icon: "users", title: "No personnel yet",
                        body: if can_edit { "Add people one by one, or import a Service 5 backup in Settings → Workforce data." } else { "The office hasn't added anyone yet." } }
                }
            } else if shown.is_empty() {
                div { class: "card", EmptyState { icon: "search", title: "Nobody matches", body: "Try another word, or another filter." } }
            } else {
                if can_edit {
                    label { class: "flex items-center gap-2 text-sm text-muted px-1 no-print",
                        input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "select-all", checked: all_picked,
                            onchange: move |e| {
                                let mut p = picked.write();
                                for id in &shown_ids { if e.checked() { p.insert(*id); } else { p.remove(id); } }
                            }
                        }
                        "Select all {shown.len()} shown"
                    }
                }
                ul { class: "card divide-y divide-line no-print", "data-testid": "worker-list",
                    for w in shown {
                        li {
                            key: "{w.id}",
                            class: "flex flex-wrap items-center gap-3 px-4 py-3 focus:outline-none focus-visible:bg-hover",
                            "data-testid": "worker-row",
                            tabindex: "0",
                            // Shift+R on a focused row reassigns that person (Service 5's shortcut).
                            onkeydown: {
                                let w = w.clone();
                                move |e: KeyboardEvent| {
                                    if e.modifiers().shift() && matches!(e.key(), Key::Character(ref c) if c.eq_ignore_ascii_case("r")) {
                                        e.prevent_default();
                                        reassigning.set(Some(w.clone()));
                                    }
                                }
                            },
                            if can_edit {
                                input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b] shrink-0", "aria-label": "Select {w.name}", "data-testid": "select-worker",
                                    checked: picked().contains(&w.id),
                                    onchange: { let id = w.id; move |e: FormEvent| { if e.checked() { picked.write().insert(id); } else { picked.write().remove(&id); } } }
                                }
                            }
                            if w.has_photo {
                                img { src: "/api/workers/{w.id}/photo", alt: "", loading: "lazy", class: "w-9 h-9 rounded-full object-cover shrink-0", "data-testid": "worker-photo" }
                            } else {
                                crate::components::ui::Avatar { name: w.name.clone(), size: 36 }
                            }
                            div { class: "flex-1 min-w-0",
                                p { class: "font-medium truncate",
                                    "{w.name}"
                                    if !w.a_number.is_empty() { span { class: "text-muted font-normal ml-2 tabular", "A# {w.a_number}" } }
                                }
                                p { class: "text-sm text-muted truncate",
                                    {[w.trade.as_str(), w.area.as_str(), w.project_group.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" · ")}
                                }
                                {
                                    let extras: Vec<String> = cols_now.iter().filter_map(|k| {
                                        let v = extra_value(&w, k);
                                        let label = EXTRA.iter().find(|(x, _)| x == k)?.1;
                                        (!v.trim().is_empty()).then(|| format!("{label}: {v}"))
                                    }).collect();
                                    rsx! { if !extras.is_empty() { p { class: "text-xs text-muted truncate", "data-testid": "worker-extras", {extras.join(" · ")} } } }
                                }
                            }
                            if !w.job_order {
                                span { class: "pill bg-tag-gray-bg text-tag-gray-text", title: "Job orders are turned off for this person", "No job order" }
                            }
                            if w.archived {
                                Pill { tone: Tone::Gray, text: if w.archive_category.is_empty() { "Archived".to_string() } else { w.archive_category.clone() } }
                            } else {
                                span { class: "sr-only", "{w.status.label()}" }
                                crate::views::workforce_ops::StatusButtons { worker: w.clone() }
                                {
                                    let w3 = w.clone();
                                    rsx! { button { class: "btn-icon btn-ghost", "aria-label": "Reassign {w.name} (Shift+R)", title: "Reassign (Shift+R)", "data-testid": "reassign-worker", onclick: move |_| reassigning.set(Some(w3.clone())), Icon { name: "move" } } }
                                }
                            }
                            if can_edit {
                                {
                                    let w2 = w.clone();
                                    let w4 = w.clone();
                                    rsx! {
                                        button { class: "btn-icon btn-ghost", "aria-label": "Profile of {w.name}", title: "Personal details", "data-testid": "profile-worker", onclick: move |_| profiling.set(Some(w4.clone())), Icon { name: "info" } }
                                        button { class: "btn-icon btn-ghost", "aria-label": "Edit {w.name}", "data-testid": "edit-worker", onclick: move |_| editing.set(Some(Some(w2.clone()))), Icon { name: "edit" } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(target) = editing() {
            WorkerModal { worker: target, on_close: move |_| editing.set(None) }
        }
        if let Some(w) = profiling() {
            crate::views::workforce_more::ProfileModal { worker: w, on_close: move |_| profiling.set(None) }
        }
        if let Some(w) = reassigning() {
            crate::views::workforce_ops::ReassignModal { worker: w, on_close: move |_| reassigning.set(None) }
        }
        if let Some(b) = action() {
            BulkModal { kind: b, workers: chosen.clone(), on_close: move |done: bool| {
                action.set(None);
                if done { picked.write().clear(); }
            } }
        }
        if new_day() {
            StartDayModal { on_close: move |_| new_day.set(false) }
        }
    }
}

const COLS_KEY: &str = "sm-personnel-cols";
const EXTRA: &[(&str, &str)] =
    &[("task", "Task"), ("project", "Project"), ("remarks", "Remarks"), ("medical", "Medical"), ("spouse", "Spouse"), ("residence", "Residence"), ("housing", "Housing")];
/// Service 5's archive reasons.
const ARCHIVE_CATEGORIES: &[&str] = &["Retired", "Dismissed", "Deceased"];

fn extra_value(w: &Worker, k: &str) -> String {
    match k {
        "task" => w.task.clone(),
        "project" => w.project.clone(),
        "remarks" => w.remarks.clone(),
        "medical" => w.medical.clone(),
        "spouse" => w.spouse.clone(),
        "residence" => w.residence.clone(),
        "housing" => w.housing.clone(),
        _ => String::new(),
    }
}

/// One CSV cell, quoted when it holds a comma, quote or line break.
pub fn csv_cell(v: &str) -> String {
    if v.contains([',', '"', '\n', '\r']) { format!("\"{}\"", v.replace('"', "\"\"")) } else { v.to_string() }
}

#[component]
fn FilterSelect(id: &'static str, label: &'static str, all: &'static str, options: Vec<String>, value: String, on_change: EventHandler<String>) -> Element {
    rsx! {
        select { id: "{id}", class: "field w-auto max-w-[14rem]", "aria-label": "{label}", "data-testid": "{id}", onchange: move |e| on_change.call(e.value()),
            option { value: "", selected: value.is_empty(), "{all}" }
            for o in options {
                option { key: "{o}", value: "{o}", selected: o == value, "{o}" }
            }
        }
    }
}

/// The current list as a printable table (landscape), with the chosen extra columns.
#[component]
fn PrintList(workers: Vec<Worker>, cols: Vec<String>) -> Element {
    let extra: Vec<(&'static str, &'static str)> = EXTRA.iter().copied().filter(|(k, _)| cols.iter().any(|c| c == k)).collect();
    rsx! {
        div { class: "print-only print-sheet",
            p { class: "font-semibold mb-2", "Personnel — {util::long_day(chrono::Utc::now())} — {util::plural(workers.len(), \"person\")}" }
            table { class: "w-full text-xs border-collapse",
                thead {
                    tr { class: "text-left border-b",
                        th { class: "p-1", "#" } th { class: "p-1", "A#" } th { class: "p-1", "Name" } th { class: "p-1", "Status" } th { class: "p-1", "Trade" } th { class: "p-1", "Project group" } th { class: "p-1", "Area" }
                        for (_, l) in extra.iter() { th { key: "{l}", class: "p-1", "{l}" } }
                    }
                }
                tbody {
                    for (i, w) in workers.iter().enumerate() {
                        tr { key: "{w.id}", class: "border-b",
                            td { class: "p-1", "{i + 1}" } td { class: "p-1", "{w.a_number}" } td { class: "p-1", "{w.name}" } td { class: "p-1", "{w.status.label()}" }
                            td { class: "p-1", "{w.trade}" } td { class: "p-1", "{w.project_group}" } td { class: "p-1", "{w.area}" }
                            for (k, _) in extra.iter() { td { key: "{k}", class: "p-1", {extra_value(w, k)} } }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Bulk {
    Status(WorkerStatus),
    Reassign,
    Archive,
    Restore,
    Delete,
}

/// Actions for the ticked people (Service 5's selection tray).
#[component]
fn BulkBar(count: usize, archived_view: bool, on_pick: EventHandler<Bulk>, on_clear: EventHandler<()>) -> Element {
    rsx! {
        div { class: "card px-4 py-2.5 flex flex-wrap items-center gap-2 no-print scale-in", "data-testid": "bulk-bar",
            span { class: "text-sm font-medium mr-2", "{count} selected" }
            if archived_view {
                button { class: "btn-secondary", "data-testid": "bulk-restore", onclick: move |_| on_pick.call(Bulk::Restore), "Restore to active" }
            } else {
                button { class: "btn-secondary", "data-testid": "bulk-present", onclick: move |_| on_pick.call(Bulk::Status(WorkerStatus::Present)), "Present" }
                button { class: "btn-secondary", "data-testid": "bulk-absent", onclick: move |_| on_pick.call(Bulk::Status(WorkerStatus::Absent)), "Absent" }
                button { class: "btn-secondary", "data-testid": "bulk-onpass", onclick: move |_| on_pick.call(Bulk::Status(WorkerStatus::OnPass)), "On pass" }
                button { class: "btn-secondary", "data-testid": "bulk-reassign", onclick: move |_| on_pick.call(Bulk::Reassign), Icon { name: "move" } "Reassign" }
                button { class: "btn-secondary", "data-testid": "bulk-archive", onclick: move |_| on_pick.call(Bulk::Archive), Icon { name: "archive" } "Archive" }
            }
            button { class: "btn-ghost text-tag-red-text", "data-testid": "bulk-delete", onclick: move |_| on_pick.call(Bulk::Delete), Icon { name: "trash" } "Delete" }
            button { class: "btn-ghost ml-auto", "data-testid": "bulk-clear", onclick: move |_| on_clear.call(()), "Clear selection" }
        }
    }
}

fn save_request(w: &Worker, archived: bool) -> SaveWorkerRequest {
    SaveWorkerRequest {
        a_number: w.a_number.clone(),
        name: w.name.clone(),
        phone: w.phone.clone(),
        trade: w.trade.clone(),
        department: w.department.clone(),
        area: w.area.clone(),
        project_group: w.project_group.clone(),
        group_role: w.group_role.clone(),
        status: w.status,
        archived,
    }
}

#[component]
fn BulkModal(kind: Bulk, workers: Vec<Worker>, on_close: EventHandler<bool>) -> Element {
    let app = use_app();
    let mut busy = use_signal(|| false);
    let mut category = use_signal(|| "Retired".to_string());
    let mut reason = use_signal(String::new);
    let mut group = use_signal(String::new);
    let mut area = use_signal(String::new);
    let n = workers.len();
    let names = workers.iter().take(8).map(|w| w.name.clone()).collect::<Vec<_>>().join(", ") + if n > 8 { " …" } else { "" };
    let (title, button, danger) = match kind {
        Bulk::Status(s) => (format!("Mark {} {}", util::plural(n, "person"), s.label().to_lowercase()), "Mark them".to_string(), false),
        Bulk::Reassign => (format!("Reassign {}", util::plural(n, "person")), "Reassign".to_string(), false),
        Bulk::Archive => (format!("Archive {}", util::plural(n, "person")), "Archive".to_string(), false),
        Bulk::Restore => (format!("Restore {}", util::plural(n, "person")), "Restore to active".to_string(), false),
        Bulk::Delete => (format!("Delete {} for good?", util::plural(n, "person")), format!("Delete {} for good", util::plural(n, "person")), true),
    };
    let run = move |_| {
        let workers = workers.clone();
        async move {
            busy.set(true);
            let mut failed = None;
            if kind == Bulk::Delete {
                let req = DeleteWorkersRequest { ids: workers.iter().map(|w| w.id).collect() };
                if let Err(e) = api::post::<_, serde_json::Value>("/workers/delete", &req).await {
                    failed = Some(e.message());
                }
            }
            for w in workers.iter().filter(|_| kind != Bulk::Delete) {
                let r = match kind {
                    Bulk::Status(s) => api::put::<_, Worker>(&format!("/workers/{}/status", w.id), &WorkerStatusRequest { status: s, reason: reason() }).await.map(|_| ()),
                    Bulk::Reassign => {
                        let pick = |new: String, old: &String| if new.trim().is_empty() { old.clone() } else { new.trim().to_string() };
                        let req = ReassignRequest { project_group: pick(group(), &w.project_group), area: pick(area(), &w.area), department: w.department.clone(), reason: reason() };
                        api::put::<_, Worker>(&format!("/workers/{}/reassign", w.id), &req).await.map(|_| ())
                    }
                    Bulk::Archive | Bulk::Restore => {
                        let archive = kind == Bulk::Archive;
                        match api::put::<_, Worker>(&format!("/workers/{}", w.id), &save_request(w, archive)).await {
                            Ok(_) => {
                                let patch = if archive { serde_json::json!({ "archiveCategory": category(), "archiveReason": reason() }) } else { serde_json::json!({ "archiveCategory": "", "archiveReason": "" }) };
                                api::put::<_, Worker>(&format!("/workers/{}/profile", w.id), &patch).await.map(|_| ())
                            }
                            Err(e) => Err(e),
                        }
                    }
                    Bulk::Delete => Ok(()),
                };
                if let Err(e) = r {
                    failed = Some(format!("{}: {}", w.name, e.message()));
                    break;
                }
            }
            *REFRESH.write() += 1;
            busy.set(false);
            match failed {
                Some(e) => app.error(e),
                None => {
                    app.success(match kind {
                        Bulk::Delete => format!("{} deleted (a snapshot was kept)", util::plural(n, "person")),
                        _ => format!("{} updated", util::plural(n, "person")),
                    });
                    on_close.call(true);
                }
            }
        }
    };
    rsx! {
        Modal { title: title.clone(), on_close: move |_| on_close.call(false),
            div { class: "space-y-4", "data-testid": "bulk-modal",
                p { class: "text-sm text-muted", "{names}" }
                match kind {
                    Bulk::Archive => rsx! {
                        div {
                            label { class: "label", r#for: "ba-cat", "Reason for archiving" }
                            select { id: "ba-cat", class: "field", "data-testid": "bulk-category", onchange: move |e| category.set(e.value()),
                                for c in ARCHIVE_CATEGORIES { option { key: "{c}", value: "{c}", selected: category() == *c, "{c}" } }
                            }
                        }
                    },
                    Bulk::Reassign => rsx! {
                        div { class: "grid sm:grid-cols-2 gap-3",
                            div { label { class: "label", r#for: "ba-group", "New project group" } input { id: "ba-group", class: "field", "data-testid": "bulk-group", placeholder: "Leave blank to keep", value: "{group}", oninput: move |e| group.set(e.value()) } }
                            div { label { class: "label", r#for: "ba-area", "New area" } input { id: "ba-area", class: "field", "data-testid": "bulk-area", placeholder: "Leave blank to keep", value: "{area}", oninput: move |e| area.set(e.value()) } }
                        }
                    },
                    Bulk::Delete => rsx! {
                        p { class: "text-sm rounded-md bg-tag-red-bg text-tag-red-text px-3 py-2",
                            "This erases their personal, family, health and assignment records. A snapshot is taken first, so it can be restored from Settings → Workforce data." }
                    },
                    _ => rsx! {},
                }
                if !matches!(kind, Bulk::Delete | Bulk::Restore) {
                    div {
                        label { class: "label", r#for: "ba-reason", "Reason (optional)" }
                        input { id: "ba-reason", class: "field", "data-testid": "bulk-reason", value: "{reason}", oninput: move |e| reason.set(e.value()) }
                    }
                }
                div { class: "modal-actions",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(false), "Cancel" }
                    button { class: if danger { "btn-danger" } else { "btn-primary" }, "data-testid": "bulk-confirm", disabled: busy(), onclick: run,
                        if busy() { Spinner {} }
                        "{button}"
                    }
                }
            }
        }
    }
}

/// Service 5's "Start New Day".
#[component]
fn StartDayModal(on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let mut busy = use_signal(|| false);
    let go = move |_| async move {
        busy.set(true);
        match api::post::<_, serde_json::Value>("/workforce/start-day", &serde_json::json!({})).await {
            Ok(v) => {
                let n = v["applied"].as_u64().unwrap_or(0) as usize;
                app.success(if n > 0 { format!("New day started: {} activated", util::plural(n, "staged assignment")) } else { "New day started".to_string() });
                *REFRESH.write() += 1;
                on_close.call(());
            }
            Err(e) => app.error(e.message()),
        }
        busy.set(false);
    };
    rsx! {
        Modal { title: "Start a new day?", on_close: move |_| on_close.call(()),
            div { class: "space-y-4",
                p { class: "text-sm", "Assignments staged for today on the Tomorrow tab become today's tasks, areas and projects. Everyone else goes back to unassigned; their previous areas and tasks are kept." }
                p { class: "text-sm text-muted", "A snapshot is taken first, so this can be undone from Settings → Workforce data." }
                div { class: "modal-actions",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button { class: "btn-primary", "data-testid": "start-day-confirm", disabled: busy(), onclick: go, if busy() { Spinner {} } "Start new day" }
                }
            }
        }
    }
}

#[component]
fn WorkerModal(worker: Option<Worker>, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let id = worker.as_ref().map(|w| w.id);
    let start = worker.map(|w| SaveWorkerRequest {
        a_number: w.a_number,
        name: w.name,
        phone: w.phone,
        trade: w.trade,
        department: w.department,
        area: w.area,
        project_group: w.project_group,
        group_role: w.group_role,
        status: w.status,
        archived: w.archived,
    });
    let mut form = use_signal(|| start.unwrap_or_default());
    let mut busy = use_signal(|| false);
    let f = form();
    let text = move |key: &'static str, label: &'static str, value: String, set: fn(&mut SaveWorkerRequest, String)| {
        rsx! {
            div {
                label { class: "label", r#for: "w-{key}", "{label}" }
                input { id: "w-{key}", class: "field", "data-testid": "worker-{key}", value: "{value}", oninput: move |e| set(&mut form.write(), e.value()) }
            }
        }
    };
    let save = move |_| async move {
        busy.set(true);
        let result: Result<Worker, _> = match id {
            Some(id) => api::put(&format!("/workers/{id}"), &form()).await,
            None => api::post("/workers", &form()).await,
        };
        busy.set(false);
        match result {
            Ok(w) => {
                app.success(format!("{} saved", w.name));
                *REFRESH.write() += 1;
                on_close.call(());
            }
            Err(e) => app.error(e.message()),
        }
    };
    rsx! {
        Modal { title: if id.is_some() { "Edit worker" } else { "Add worker" }, wide: true, on_close: move |_| on_close.call(()),
            div { class: "space-y-4",
                div { class: "grid sm:grid-cols-2 gap-4",
                    {text("name", "Name", f.name.clone(), |r, v| r.name = v)}
                    {text("a-number", "A# (worker number)", f.a_number.clone(), |r, v| r.a_number = v)}
                    {text("phone", "Phone", f.phone.clone(), |r, v| r.phone = v)}
                    {text("trade", "Trade", f.trade.clone(), |r, v| r.trade = v)}
                    {text("department", "Department", f.department.clone(), |r, v| r.department = v)}
                    {text("area", "Area", f.area.clone(), |r, v| r.area = v)}
                    {text("group", "Project group", f.project_group.clone(), |r, v| r.project_group = v)}
                    {text("role", "Role in group", f.group_role.clone(), |r, v| r.group_role = v)}
                }
                div {
                    p { class: "label", "Today" }
                    div { class: "flex flex-wrap gap-2",
                        for s in WorkerStatus::ALL {
                            button { key: "{s:?}", r#type: "button", class: if f.status == s { "btn-primary" } else { "btn-secondary" }, "data-testid": "worker-status-{s:?}",
                                onclick: move |_| form.write().status = s, "{s.label()}" }
                        }
                    }
                }
                label { class: "flex items-center gap-2 text-sm",
                    input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "worker-archived", checked: f.archived, onchange: move |e| form.write().archived = e.checked() }
                    "Archived (no longer working here; kept for the records)"
                }
                div { class: "modal-actions",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button { class: "btn-primary", "data-testid": "save-worker", disabled: busy() || f.name.trim().is_empty(), onclick: save,
                        if busy() { Spinner {} }
                        if id.is_some() { "Save changes" } else { "Add worker" }
                    }
                }
            }
        }
    }
}

/// Settings → Workforce data: import any Service 5 backup, download a backup,
/// take and restore snapshots.
#[component]
pub fn WorkforceData() -> Element {
    let app = use_app();
    let mut snapshots = use_resource(|| async { api::get::<Vec<WorkforceSnapshotInfo>>("/workforce/snapshots").await.unwrap_or_default() });
    let mut passphrase = use_signal(String::new);
    let mut replace = use_signal(|| false);
    let mut busy = use_signal(|| None::<&'static str>);
    let mut report = use_signal(|| None::<WorkforceImportReport>);
    let mut confirm_restore = use_signal(|| None::<Uuid>);

    let import = move |_| async move {
        let file = match media_canvas::backup_file("workforce-file", 500.0 * 1_048_576.0) {
            Ok(Some(f)) => f,
            Ok(None) => return,
            Err(e) => return app.error(e),
        };
        busy.set(Some("import"));
        let mode = if replace() { "Replace" } else { "Merge" };
        let path = format!("/workforce/import?mode={mode}&file_name={}", js_sys::encode_uri_component(&file.name()));
        let headers = api::passphrase_header(&passphrase());
        match api::upload(&path, &file, &headers).await.and_then(|t| {
            serde_json::from_str::<WorkforceImportReport>(&t).map_err(|_| api::ApiFail::Server(0, "Unexpected reply from the server.".into()))
        }) {
            Ok(r) => {
                app.success(format!("Imported {} workers", r.workers));
                report.set(Some(r));
                *REFRESH.write() += 1;
                snapshots.restart();
            }
            Err(e) => app.error(e.message()),
        }
        busy.set(None);
    };
    // Personnel data only leaves encrypted: every download needs the passphrase.
    let download = move |_| async move {
        if passphrase().chars().count() < 8 {
            return app.error("Type a passphrase of at least 8 characters first: the backup is encrypted with it.");
        }
        busy.set(Some("export"));
        match api::send_with("GET", "/workforce/export", &api::passphrase_header(&passphrase())).await {
            Ok(text) => save_file(&format!("sitemonitor-workforce-{}.agwcz", chrono::Local::now().format("%Y-%m-%d")), &text),
            Err(e) => app.error(e.message()),
        }
        busy.set(None);
    };
    let take = move |_| async move {
        busy.set(Some("snapshot"));
        match api::post::<_, Vec<WorkforceSnapshotInfo>>("/workforce/snapshots", &serde_json::json!({})).await {
            Ok(_) => {
                app.success("Snapshot saved");
                snapshots.restart();
            }
            Err(e) => app.error(e.message()),
        }
        busy.set(None);
    };

    let list = snapshots.read().clone().unwrap_or_default();
    rsx! {
        div { class: "space-y-5", "data-testid": "workforce-data",
            div { class: "space-y-3",
                p { class: "text-sm font-medium", "Import a Service 5 backup" }
                p { class: "text-sm text-muted", "Any Service 5 JSON backup (old or new), an encrypted .agwcz backup, a service5.db file, or a backup downloaded here. A snapshot is taken first, so an import can always be undone. For service5.db, close Service 5 first so its latest changes are saved into the file." }
                div { class: "grid sm:grid-cols-2 gap-3 max-w-xl",
                    div {
                        label { class: "label", r#for: "workforce-pass", "Backup passphrase (opens or encrypts a backup)" }
                        input { id: "workforce-pass", r#type: "password", class: "field", "data-testid": "workforce-passphrase", autocomplete: "off", value: "{passphrase}", oninput: move |e| passphrase.set(e.value()) }
                    }
                    label { class: "flex items-start gap-2 text-sm self-end pb-2",
                        input { r#type: "checkbox", class: "w-4 h-4 mt-0.5 accent-[#064e3b]", "data-testid": "workforce-replace", checked: replace(), onchange: move |e| replace.set(e.checked()) }
                        span { "Replace instead of merge" span { class: "block text-xs text-muted", "Parts that are empty in the file never wipe your data." } }
                    }
                }
                input { id: "workforce-file", r#type: "file", class: "hidden", accept: ".json,.agwcz,.db,application/json", "data-testid": "workforce-file", onchange: import }
                div { class: "flex flex-wrap gap-2",
                    label { r#for: "workforce-file", class: "btn-primary cursor-pointer", "data-testid": "workforce-import",
                        if busy() == Some("import") { Spinner {} } else { Icon { name: "upload" } }
                        "Choose backup file"
                    }
                    button { class: "btn-secondary", "data-testid": "workforce-export", disabled: busy().is_some(), onclick: download,
                        if busy() == Some("export") { Spinner {} } else { Icon { name: "archive" } }
                        "Download backup"
                    }
                }
                crate::views::workforce_extra::BackupPicker {
                    on_save: move |lists: String| async move {
                        if passphrase().chars().count() < 8 {
                            return app.error("Type a passphrase of at least 8 characters first: the backup is encrypted with it.");
                        }
                        match api::send_with("GET", &format!("/workforce/export?collections={lists}"), &api::passphrase_header(&passphrase())).await {
                            Ok(text) => save_file(&format!("sitemonitor-workforce-partial-{}.agwcz", chrono::Local::now().format("%Y-%m-%d")), &text),
                            Err(e) => app.error(e.message()),
                        }
                    }
                }
                if let Some(r) = report() {
                    div { class: "rounded-md bg-tag-green-bg text-tag-green-text text-sm px-4 py-3 space-y-1", "data-testid": "import-report",
                        p { class: "font-medium", "{r.detected}: {util::plural(r.workers, \"worker\")} imported" }
                        if !r.collections.is_empty() {
                            p { {r.collections.iter().map(|(c, n)| format!("{} {}", n, c.replace('_', " "))).collect::<Vec<_>>().join(" · ")} }
                        }
                        if !r.kept_existing.is_empty() {
                            p { "Kept as they were (empty in the file): " {r.kept_existing.join(", ").replace('_', " ")} }
                        }
                    }
                }
            }
            div { class: "space-y-3 border-t border-line pt-4",
                div { class: "flex flex-wrap items-center justify-between gap-2",
                    div {
                        p { class: "text-sm font-medium", "Snapshots" }
                        p { class: "text-sm text-muted", "Saved every night at the crew reset, before each import, and whenever you take one. The last 14 are kept." }
                    }
                    button { class: "btn-secondary", "data-testid": "take-snapshot", disabled: busy().is_some(), onclick: take,
                        if busy() == Some("snapshot") { Spinner {} } else { Icon { name: "plus" } }
                        "Take snapshot now"
                    }
                }
                if list.is_empty() {
                    p { class: "text-sm text-muted", "No snapshots yet." }
                }
                ul { class: "divide-y divide-line border-y border-line",
                    for s in list {
                        li { key: "{s.id}", class: "flex flex-wrap items-center gap-3 py-2 text-sm", "data-testid": "snapshot-row",
                            span { class: "flex-1 min-w-0", "{util::day(s.taken_at)} · {util::time(s.taken_at)}" span { class: "text-muted ml-2", "{s.reason} · {util::plural(s.workers as usize, \"worker\")}" } }
                            if confirm_restore() == Some(s.id) {
                                span { class: "text-tag-red-text", "Put the workforce data back as it was then?" }
                                button { class: "btn-ghost", onclick: move |_| confirm_restore.set(None), "Keep current" }
                                button { class: "btn-danger", "data-testid": "restore-confirm", disabled: busy().is_some(),
                                    onclick: move |_| async move {
                                        busy.set(Some("restore"));
                                        match api::post::<_, WorkforceImportReport>(&format!("/workforce/snapshots/{}/restore", s.id), &serde_json::json!({})).await {
                                            Ok(r) => {
                                                app.success(format!("Restored: {}", util::plural(r.workers, "worker")));
                                                *REFRESH.write() += 1;
                                                snapshots.restart();
                                            }
                                            Err(e) => app.error(e.message()),
                                        }
                                        confirm_restore.set(None);
                                        busy.set(None);
                                    },
                                    "Restore"
                                }
                            } else {
                                button { class: "btn-ghost", "data-testid": "restore-snapshot", onclick: move |_| confirm_restore.set(Some(s.id)), Icon { name: "undo" } "Restore" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Saves text as a downloaded file.
pub fn save_file(name: &str, text: &str) {
    let parts = js_sys::Array::of1(&wasm_bindgen::JsValue::from_str(text));
    let Ok(blob) = web_sys::Blob::new_with_str_sequence(&parts) else { return };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else { return };
    let doc = util::document();
    if let Some(a) = doc.create_element("a").ok().and_then(|a| a.dyn_into::<web_sys::HtmlElement>().ok()) {
        let _ = a.set_attribute("href", &url);
        let _ = a.set_attribute("download", name);
        a.click();
    }
    let _ = web_sys::Url::revoke_object_url(&url);
}
