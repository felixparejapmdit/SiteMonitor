//! Service 5's "Daily Assignments": the whole workforce as one table, in three
//! views (task assignments, personnel directory, all columns), with Start New
//! Day, the encoder, print reports, bulk tools, filters, column choice, cards,
//! sorting, paging, and "Recording for" a past day (read-only, from the
//! attendance log).

use std::collections::HashSet;

use dioxus::prelude::*;
use serde_json::{json, Value};
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{EmptyState, Icon, Spinner, PAGE_TABS};
use crate::state::use_app;
use crate::util;
use crate::views::personnel::{extra_value, Bulk, BulkModal, FilterSelect, StartDayModal, WorkerModal, ARCHIVE_CATEGORIES, REFRESH};
use crate::views::workforce_ops::{s, today, ReassignModal, StatusButtons};
use crate::views::workforce_ops::s5print;
use crate::views::workforce_print::PRINT_REQUEST;
use crate::Route;

const HIDDEN_KEY: &str = "sm-assign-hidden";
const VIEW_KEY: &str = "sm-assign-view";

/// (key, heading)
const COLUMNS: &[(&str, &str)] = &[
    ("status", "Status"),
    ("a", "A#"),
    ("name", "Name"),
    ("medical", "Medical"),
    ("trade", "Trade group"),
    ("group", "Project group"),
    ("role", "Role"),
    ("department", "Department"),
    ("area", "Area"),
    ("project", "Project"),
    ("task", "Specific task"),
    ("phone", "Phone"),
    ("spouse", "Spouse"),
    ("residence", "Residence"),
    ("housing", "Housing"),
    ("remarks", "Remarks / reason"),
];

/// Service 5's three presets.
fn preset(view: &str) -> &'static [&'static str] {
    match view {
        "Task assignments" => &["status", "a", "name", "trade", "group", "area", "project", "task", "remarks"],
        "Personnel directory" => &["a", "name", "medical", "trade", "group", "status", "phone", "spouse", "residence", "housing"],
        _ => &["status", "a", "name", "medical", "trade", "group", "role", "department", "area", "project", "task", "phone", "spouse", "residence", "housing", "remarks"],
    }
}

fn cell(w: &Worker, k: &str) -> String {
    match k {
        "status" => w.status.label().to_string(),
        "a" => w.a_number.clone(),
        "name" => w.name.clone(),
        "trade" => w.trade.clone(),
        "group" => w.project_group.clone(),
        "role" => w.group_role.clone(),
        "department" => w.department.clone(),
        "area" => w.area.clone(),
        "phone" => w.phone.clone(),
        other => extra_value(w, other),
    }
}

/// The status someone had on a past day, from the attendance log: the last
/// change on or before that day, else the status before the first later change,
/// else today's status.
pub fn status_on(log: &[Value], a: &str, day: &str, current: WorkerStatus) -> WorkerStatus {
    let mut mine: Vec<(String, i64, String, String)> =
        log.iter().filter(|e| s(e, "aNumber") == a).map(|e| (s(e, "date"), e["timestamp"].as_i64().unwrap_or(0), s(e, "oldStatus"), s(e, "newStatus"))).collect();
    mine.sort();
    if let Some(e) = mine.iter().rev().find(|e| e.0.as_str() <= day) {
        return WorkerStatus::from_legacy(&e.3);
    }
    match mine.first() {
        Some(e) if !e.2.is_empty() => WorkerStatus::from_legacy(&e.2),
        _ => current,
    }
}

#[component]
pub fn AssignmentsView(#[props(default = "active")] initial_status: &'static str) -> Element {
    let app = use_app();
    let nav = use_navigator();
    let can_edit = app.can(Permission::ManageWorkforce);
    let clinic = app.can(Permission::ManageClinic);
    let workers = use_resource(|| async {
        let _ = REFRESH();
        api::get::<Vec<Worker>>("/workers").await
    });
    let mut day = use_signal(today);
    let log = use_resource(move || async move {
        if day() == today() { vec![] } else { api::get::<Vec<Value>>("/workforce/records/attendance_log").await.unwrap_or_default() }
    });
    let mut view = use_signal(|| util::storage_get(VIEW_KEY).unwrap_or_else(|| "Task assignments".into()));
    let mut hidden = use_signal(|| util::storage_get(HIDDEN_KEY).unwrap_or_default().split(',').filter(|x| !x.is_empty()).map(String::from).collect::<HashSet<String>>());
    let mut query = use_signal(String::new);
    let mut status = use_signal(|| initial_status.to_string());
    let mut f_trade = use_signal(String::new);
    let mut f_group = use_signal(String::new);
    let mut f_area = use_signal(String::new);
    let mut f_house = use_signal(String::new);
    let mut hide_assigned = use_signal(|| false);
    let mut cards = use_signal(|| false);
    let mut wrap = use_signal(|| false);
    let mut show_cols = use_signal(|| false);
    let mut sort = use_signal(|| ("name".to_string(), true));
    let mut per_page = use_signal(|| 15usize);
    let mut page = use_signal(|| 0usize);
    let mut picked = use_signal(HashSet::<Uuid>::new);
    let mut action = use_signal(|| None::<Bulk>);
    let mut new_day = use_signal(|| false);
    let mut editing = use_signal(|| None::<Option<Worker>>);
    let mut reassigning = use_signal(|| None::<Worker>);
    let mut profiling = use_signal(|| None::<Worker>);

    let all = match &*workers.read() {
        Some(Ok(w)) => w.clone(),
        Some(Err(e)) => return rsx! { div { class: "card", EmptyState { icon: "users", title: "Couldn't load the workforce", body: e.message() } } },
        None => return rsx! { div { class: "card p-8 flex justify-center", Spinner { size: 20 } } },
    };
    let past = day() != today();
    let entries = log.read().clone().unwrap_or_default();
    // On a past day, everyone's status as it was then (read-only).
    let all: Vec<Worker> = all
        .into_iter()
        .map(|mut w| {
            if past {
                w.status = status_on(&entries, &w.a_number, &day(), w.status);
            }
            w
        })
        .collect();
    let choices = |get: fn(&Worker) -> &String| {
        let mut v: Vec<String> = all.iter().map(|w| get(w).trim().to_string()).filter(|s| !s.is_empty()).collect();
        v.sort_by_key(|s| s.to_lowercase());
        v.dedup();
        v
    };
    let (trades, groups, areas, housings) = (choices(|w| &w.trade), choices(|w| &w.project_group), choices(|w| &w.area), choices(|w| &w.housing));
    let q = query().trim().to_lowercase();
    let mut shown: Vec<Worker> = all
        .iter()
        .filter(|w| match status().as_str() {
            "active" => !w.archived,
            "Present" => !w.archived && w.status == WorkerStatus::Present,
            "Absent" => !w.archived && w.status == WorkerStatus::Absent,
            "OnPass" => !w.archived && w.status == WorkerStatus::OnPass,
            "archived" => w.archived,
            "all" => true,
            cat => w.archived && w.archive_category == cat,
        })
        .filter(|w| q.is_empty() || [&w.name, &w.a_number, &w.trade, &w.phone, &w.task, &w.area, &w.project_group].iter().any(|f| f.to_lowercase().contains(&q)))
        .filter(|w| (f_trade().is_empty() || w.trade.trim() == f_trade()) && (f_group().is_empty() || w.project_group.trim() == f_group()))
        .filter(|w| (f_area().is_empty() || w.area.trim() == f_area()) && (f_house().is_empty() || w.housing.trim() == f_house()))
        .filter(|w| !hide_assigned() || !w.assigned_today)
        .cloned()
        .collect();
    let (sort_key, asc) = sort();
    let chevron: &'static str = if asc { "chevron-up" } else { "chevron-down" };
    shown.sort_by(|a, b| {
        let o = if sort_key == "a" {
            a.a_number.parse::<u64>().unwrap_or(u64::MAX).cmp(&b.a_number.parse::<u64>().unwrap_or(u64::MAX)).then(a.a_number.cmp(&b.a_number))
        } else {
            cell(a, &sort_key).to_lowercase().cmp(&cell(b, &sort_key).to_lowercase())
        };
        if asc { o } else { o.reverse() }
    });
    let total = shown.len();
    let pages = total.div_ceil(per_page()).max(1);
    let pg = page().min(pages - 1);
    let first = pg * per_page();
    let rows: Vec<Worker> = shown.iter().skip(first).take(per_page()).cloned().collect();
    let cols: Vec<(&'static str, &'static str)> =
        COLUMNS.iter().copied().filter(|(k, _)| preset(&view()).contains(k) && !hidden().contains(*k) && (*k != "medical" || clinic)).collect();
    let chosen: Vec<Worker> = all.iter().filter(|w| picked().contains(&w.id)).cloned().collect();
    let n_picked = chosen.len();
    let showing = format!("Showing {} to {} of {total} entries", if total == 0 { 0 } else { first + 1 }, first + rows.len());
    let page_ids: Vec<Uuid> = rows.iter().map(|w| w.id).collect();
    let all_on_page = !rows.is_empty() && rows.iter().all(|w| picked().contains(&w.id));
    let cell_class = if wrap() { "px-3 py-2 align-top whitespace-normal break-words" } else { "px-3 py-2 align-top whitespace-nowrap truncate max-w-[16rem]" };
    let open_print = move |kind: &'static str, ids: Vec<Uuid>| {
        *PRINT_REQUEST.write() = Some((kind, ids));
        PAGE_TABS.write().insert("operations", "Print");
        nav.push(Route::Operations {});
    };
    let picked_ids: Vec<Uuid> = chosen.iter().map(|w| w.id).collect();
    let picked_ids2 = picked_ids.clone();
    let bulk_btn = |label: &'static str, testid: &'static str, icon: &'static str, b: Bulk| {
        rsx! {
            button { class: "btn-secondary", "data-testid": "{testid}", disabled: n_picked == 0, onclick: move |_| action.set(Some(b)), Icon { name: icon } "{label}" }
        }
    };
    rsx! {
        div { class: "space-y-4 rise-in",
            header { class: "flex flex-wrap items-center gap-3 no-print",
                h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight mr-2", "Daily assignments" }
                div { class: "inline-flex rounded-md border border-line p-0.5 bg-surface", role: "group", "aria-label": "View",
                    for v in ["Task assignments", "Personnel directory", "All columns"] {
                        button {
                            key: "{v}",
                            class: if view() == v { "px-3 h-8 rounded text-sm bg-accent-soft text-accent-ink font-medium" } else { "px-3 h-8 rounded text-sm text-muted hover:text-ink" },
                            "aria-pressed": view() == v,
                            "data-testid": "assign-view-{v}",
                            onclick: move |_| { view.set(v.to_string()); util::storage_set(VIEW_KEY, v); },
                            "{v}"
                        }
                    }
                }
                div { class: "flex items-center gap-2 ml-auto text-sm",
                    label { class: "text-muted", r#for: "assign-day", "Recording for" }
                    input { id: "assign-day", r#type: "date", class: "field w-auto h-9", "data-testid": "assign-day", value: "{day}", max: "{today()}", oninput: move |e| day.set(e.value()) }
                    if past {
                        button { class: "btn-ghost h-9", "data-testid": "assign-today", onclick: move |_| day.set(today()), "Today" }
                    } else {
                        span { class: "pill bg-accent-soft text-accent-ink", "Today" }
                    }
                }
            }
            if past {
                p { class: "rounded-md bg-tag-amber-bg text-tag-amber-text text-sm px-3 py-2 no-print", "data-testid": "assign-past",
                    "Showing statuses as they were on {day} (from the attendance log). Changes can only be made for today."
                }
            }
            if can_edit {
                div { class: "flex flex-wrap items-center gap-2 no-print", "data-testid": "assign-toolbar",
                    button { class: "btn-secondary", "data-testid": "assign-start-day", onclick: move |_| new_day.set(true), Icon { name: "calendar" } "Start new day" }
                    button { class: "btn-primary", "data-testid": "assign-encoder",
                        onclick: move |_| { PAGE_TABS.write().insert("operations", "Tomorrow"); nav.push(Route::Operations {}); },
                        Icon { name: "edit" } "Encoder mode"
                    }
                    PrintReportsMenu { ids: picked_ids.clone(), testid: "assign" }
                    button { class: "btn-secondary", "data-testid": "assign-add", onclick: move |_| editing.set(Some(None)), Icon { name: "plus" } "Add volunteer" }
                    span { class: "w-px h-6 bg-line mx-1" }
                    {bulk_btn("Reassign", "assign-bulk-reassign", "move", Bulk::Reassign)}
                    {bulk_btn("Bulk edit", "assign-bulk-edit", "edit", Bulk::Edit)}
                    {bulk_btn("Bulk group", "assign-bulk-group", "users", Bulk::Reassign)}
                    if status() == "archived" || ARCHIVE_CATEGORIES.contains(&status().as_str()) {
                        {bulk_btn("Restore", "assign-bulk-restore", "undo", Bulk::Restore)}
                    } else {
                        {bulk_btn("Archive", "assign-bulk-archive", "archive", Bulk::Archive)}
                    }
                    button { class: "btn-ghost text-tag-red-text", "data-testid": "assign-bulk-delete", disabled: n_picked == 0, onclick: move |_| action.set(Some(Bulk::Delete)), Icon { name: "trash" } "Delete" }
                    // Service 5's "Generate Forms": the job order print view for the ticked people.
                    button { class: "btn-primary", "data-testid": "assign-forms", disabled: n_picked == 0,
                        onclick: move |_| open_print("jo", picked_ids2.clone()),
                        Icon { name: "archive" } "Generate forms"
                        span { class: "pill bg-surface/30 ml-1 tabular", "{n_picked}" }
                    }
                }
            }
            div { class: "flex flex-wrap items-center gap-2 no-print", "data-testid": "assign-filters",
                input { class: "field w-auto min-w-[15rem] flex-1 max-w-sm", r#type: "search", "aria-label": "Search", "data-testid": "assign-search",
                    placeholder: "Search by name, A#, trade, phone, task…", value: "{query}", oninput: move |e| { query.set(e.value()); page.set(0); } }
                label { class: "flex items-center gap-2 text-sm px-1",
                    input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "assign-hide-assigned", checked: hide_assigned(), onchange: move |e| { hide_assigned.set(e.checked()); page.set(0); } }
                    "Hide assigned"
                }
                select { class: "field w-auto", "aria-label": "Status", "data-testid": "assign-status", onchange: move |e| { status.set(e.value()); page.set(0); },
                    for (v, l) in [("active", "All active"), ("Present", "Present"), ("Absent", "Absent"), ("OnPass", "On pass"), ("archived", "Archived: all"), ("Deceased", "Archived: deceased"), ("Dismissed", "Archived: dismissed"), ("Retired", "Archived: retired"), ("all", "All (active and archived)")] {
                        option { key: "{v}", value: "{v}", selected: status() == v, "{l}" }
                    }
                }
                FilterSelect { id: "assign-trade", label: "Trade", all: "All trades", options: trades, value: f_trade(), on_change: move |v| { f_trade.set(v); page.set(0); } }
                FilterSelect { id: "assign-group", label: "Project group", all: "All project groups", options: groups, value: f_group(), on_change: move |v| { f_group.set(v); page.set(0); } }
                FilterSelect { id: "assign-area", label: "Area", all: "All areas", options: areas, value: f_area(), on_change: move |v| { f_area.set(v); page.set(0); } }
                FilterSelect { id: "assign-house", label: "Housing group", all: "All housing groups", options: housings, value: f_house(), on_change: move |v| { f_house.set(v); page.set(0); } }
                div { class: "relative ml-auto",
                    button { class: "btn-secondary", "aria-expanded": show_cols(), "data-testid": "assign-columns", onclick: move |_| show_cols.toggle(), Icon { name: "layers" } "Columns" }
                    if show_cols() {
                        div { class: "absolute right-0 top-11 z-30 w-60 card shadow-pop p-2 scale-in", "data-testid": "assign-columns-menu",
                            for (k, l) in COLUMNS.iter().copied().filter(|(k, _)| preset(&view()).contains(k) && *k != "name" && (*k != "medical" || clinic)) {
                                label { key: "{k}", class: "flex items-center gap-2 text-sm px-2 py-1.5 rounded hover:bg-hover",
                                    input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "assign-col-{k}", checked: !hidden().contains(k),
                                        onchange: move |e| {
                                            let mut h = hidden.write();
                                            if e.checked() { h.remove(k); } else { h.insert(k.to_string()); }
                                            util::storage_set(HIDDEN_KEY, &h.iter().cloned().collect::<Vec<_>>().join(","));
                                        }
                                    }
                                    "{l}"
                                }
                            }
                        }
                    }
                }
                button { class: if cards() { "btn-secondary ring-2 ring-accent" } else { "btn-secondary" }, "aria-pressed": cards(), "data-testid": "assign-cards", onclick: move |_| cards.toggle(), Icon { name: "grip" } "Cards" }
            }
            if !cards() {
                label { class: "inline-flex items-center gap-2 text-sm text-muted no-print",
                    input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "assign-wrap", checked: wrap(), onchange: move |e| wrap.set(e.checked()) }
                    "Wrap cells"
                }
            }
            if total == 0 {
                div { class: "card",
                    EmptyState { icon: "search", title: "No workers found", body: "Nobody matches your filters or search." }
                    div { class: "flex justify-center pb-6",
                        button { class: "btn-secondary", "data-testid": "assign-reset",
                            onclick: move |_| { query.set(String::new()); status.set("active".into()); f_trade.set(String::new()); f_group.set(String::new()); f_area.set(String::new()); f_house.set(String::new()); hide_assigned.set(false); },
                            "Reset filters"
                        }
                    }
                }
            } else if cards() {
                div { class: "grid sm:grid-cols-2 xl:grid-cols-3 gap-3", "data-testid": "assign-card-list",
                    for w in rows.iter().cloned() {
                        div { key: "{w.id}", class: "card p-4 space-y-2", "data-testid": "assign-card",
                            div { class: "flex items-start gap-2",
                                if can_edit {
                                    input { r#type: "checkbox", class: "w-4 h-4 mt-1 accent-[#064e3b]", "aria-label": "Select {w.name}", checked: picked().contains(&w.id),
                                        onchange: { let id = w.id; move |e: FormEvent| { if e.checked() { picked.write().insert(id); } else { picked.write().remove(&id); } } } }
                                }
                                div { class: "flex-1 min-w-0",
                                    p { class: "font-medium truncate", "{w.name}" span { class: "text-muted font-normal text-sm ml-2 tabular", "A# {w.a_number}" } }
                                    p { class: "text-sm text-muted truncate", {[w.trade.as_str(), w.project_group.as_str()].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" · ")} }
                                }
                            }
                            for (k, l) in cols.iter().copied().filter(|(k, _)| !matches!(*k, "name" | "a" | "status")) {
                                if !cell(&w, k).is_empty() { p { key: "{k}", class: "text-sm", span { class: "text-muted", "{l}: " } {cell(&w, k)} } }
                            }
                            if !w.archived && !past { StatusButtons { worker: w.clone() } } else { p { class: "text-sm", "{w.status.label()}" } }
                        }
                    }
                }
            } else {
                div { class: "card overflow-x-auto", "data-testid": "assign-table",
                    table { class: "w-full text-sm",
                        thead {
                            tr { class: "text-left text-xs uppercase tracking-wide text-muted border-b border-line bg-hover/40",
                                if can_edit {
                                    th { class: "px-3 py-2 w-8",
                                        input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "aria-label": "Select this page", "data-testid": "assign-select-page", checked: all_on_page,
                                            onchange: move |e| { let mut p = picked.write(); for id in &page_ids { if e.checked() { p.insert(*id); } else { p.remove(id); } } } }
                                    }
                                }
                                for (k, l) in cols.iter().copied() {
                                    th { key: "{k}", class: "px-3 py-2 font-medium whitespace-nowrap",
                                        "aria-sort": if sort_key == k { if asc { "ascending" } else { "descending" } } else { "none" },
                                        button { class: "inline-flex items-center gap-1 hover:text-ink", "data-testid": "assign-sort-{k}",
                                            onclick: move |_| { let (cur, a) = sort(); sort.set((k.to_string(), if cur == k { !a } else { true })); },
                                            "{l}"
                                            if sort_key == k { Icon { name: chevron, size: 12 } }
                                        }
                                    }
                                }
                                th { class: "px-3 py-2 text-right", "Actions" }
                            }
                        }
                        tbody { class: "divide-y divide-line",
                            for w in rows.iter().cloned() {
                                tr { key: "{w.id}", class: if picked().contains(&w.id) { "bg-accent-soft/50" } else { "hover:bg-hover/50" }, "data-testid": "assign-row",
                                    if can_edit {
                                        td { class: "px-3 py-2",
                                            input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "aria-label": "Select {w.name}", "data-testid": "assign-select", checked: picked().contains(&w.id),
                                                onchange: { let id = w.id; move |e: FormEvent| { if e.checked() { picked.write().insert(id); } else { picked.write().remove(&id); } } } }
                                        }
                                    }
                                    for (k, _) in cols.iter().copied() {
                                        td { key: "{k}", class: cell_class,
                                            if k == "status" && !w.archived && !past {
                                                StatusButtons { worker: w.clone() }
                                            } else if k == "status" && w.archived {
                                                span { class: "pill bg-tag-gray-bg text-tag-gray-text", {if w.archive_category.is_empty() { "Archived".to_string() } else { w.archive_category.clone() }} }
                                            } else if k == "name" {
                                                span { class: "font-medium", "{w.name}" }
                                                if !w.job_order { span { class: "ml-2 pill bg-tag-gray-bg text-tag-gray-text text-[11px]", "No JO" } }
                                            } else {
                                                {cell(&w, k)}
                                            }
                                        }
                                    }
                                    td { class: "px-2 py-1 text-right whitespace-nowrap",
                                        if can_edit && !past {
                                            button { class: "btn-icon btn-ghost", "aria-label": "Reassign {w.name}", title: "Reassign (Shift+R)", onclick: { let w = w.clone(); move |_| reassigning.set(Some(w.clone())) }, Icon { name: "move" } }
                                            button { class: "btn-icon btn-ghost", "aria-label": "Profile of {w.name}", onclick: { let w = w.clone(); move |_| profiling.set(Some(w.clone())) }, Icon { name: "info" } }
                                            button { class: "btn-icon btn-ghost", "aria-label": "Edit {w.name}", "data-testid": "assign-edit", onclick: { let w = w.clone(); move |_| editing.set(Some(Some(w.clone()))) }, Icon { name: "edit" } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "flex flex-wrap items-center gap-3 text-sm text-muted no-print", "data-testid": "assign-paging",
                span { "data-testid": "assign-count", "{showing}" }
                div { class: "flex items-center gap-2 ml-auto",
                    label { r#for: "assign-per-page", "Rows per page" }
                    select { id: "assign-per-page", class: "field w-auto h-9", "data-testid": "assign-per-page",
                        onchange: move |e| { per_page.set(e.value().parse().unwrap_or(15)); page.set(0); },
                        for n in [15usize, 25, 50, 100] { option { key: "{n}", value: "{n}", selected: per_page() == n, "{n}" } }
                    }
                    button { class: "btn-icon btn-ghost", "aria-label": "Previous page", "data-testid": "assign-prev", disabled: pg == 0, onclick: move |_| page.set(pg.saturating_sub(1)), Icon { name: "chevron-left" } }
                    span { class: "tabular", "Page {pg + 1} of {pages}" }
                    button { class: "btn-icon btn-ghost", "aria-label": "Next page", "data-testid": "assign-next", disabled: pg + 1 >= pages, onclick: move |_| page.set(pg + 1), Icon { name: "chevron-right" } }
                }
            }
        }
        if let Some(b) = action() {
            BulkModal { kind: b, workers: chosen.clone(), on_close: move |done: bool| {
                action.set(None);
                if done { picked.write().clear(); }
            } }
        }
        if new_day() { StartDayModal { on_close: move |_| new_day.set(false) } }
        if let Some(target) = editing() { WorkerModal { worker: target, on_close: move |_| editing.set(None) } }
        if let Some(w) = reassigning() { ReassignModal { worker: w, on_close: move |_| reassigning.set(None) } }
        if let Some(w) = profiling() { crate::views::profile::ProfileModal { worker: w, on_close: move |_| profiling.set(None) } }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn past_status_comes_from_the_log() {
        let log = vec![
            json!({ "aNumber": "1", "date": "2026-11-10", "timestamp": 1, "oldStatus": "Present", "newStatus": "Absent" }),
            json!({ "aNumber": "1", "date": "2026-11-12", "timestamp": 2, "oldStatus": "Absent", "newStatus": "Passes" }),
        ];
        assert_eq!(status_on(&log, "1", "2026-11-09", WorkerStatus::Present), WorkerStatus::Present); // before the first change
        assert_eq!(status_on(&log, "1", "2026-11-11", WorkerStatus::Present), WorkerStatus::Absent);
        assert_eq!(status_on(&log, "1", "2026-11-20", WorkerStatus::Present), WorkerStatus::OnPass);
        assert_eq!(status_on(&log, "2", "2026-11-11", WorkerStatus::Absent), WorkerStatus::Absent); // no history: today
    }
}

/// Service 5's "Print Reports" menu (Assignments and Personnel), in its order
/// and wording; the family roster uses the ticked people, or everyone.
#[component]
pub fn PrintReportsMenu(ids: Vec<Uuid>, testid: &'static str) -> Element {
    let nav = use_navigator();
    let mut open = use_signal(|| false);
    let n = ids.len();
    rsx! {
        div { class: "relative",
            button { class: "btn-secondary", "aria-expanded": open(), "data-testid": "{testid}-reports", onclick: move |_| open.toggle(), Icon { name: "archive" } "Print Reports" }
            if open() {
                div { class: "absolute left-0 top-11 z-30 w-72 card shadow-pop p-1.5 scale-in", "data-testid": "{testid}-reports-menu",
                    p { class: "px-3 pt-1 pb-1.5 text-xs font-semibold uppercase tracking-wide text-muted", "Select Grouping" }
                    button { class: "w-full text-left px-3 py-2 rounded text-sm hover:bg-hover", "data-testid": "{testid}-report-family",
                        onclick: move |_| { open.set(false); s5print("familyRoster", json!([{ "ids": ids.iter().map(Uuid::to_string).collect::<Vec<_>>() }])); },
                        "📋 Volunteer & Family Roster"
                        if n > 0 { span { class: "text-muted", " ({n} ticked)" } }
                    }
                    button { class: "w-full text-left px-3 py-2 rounded text-sm hover:bg-hover", "data-testid": "{testid}-report-list",
                        onclick: move |_| { PAGE_TABS.write().insert("workforce", "Attendance Gen"); nav.push(Route::Workforce {}); },
                        "📋 List & Attendance Generator"
                    }
                    button { class: "w-full text-left px-3 py-2 rounded text-sm hover:bg-hover", "data-testid": "{testid}-report-forms",
                        onclick: move |_| { PAGE_TABS.write().insert("workforce", "Update Forms"); nav.push(Route::Workforce {}); },
                        "📝 Semi-Annual Records Update Forms"
                    }
                    div { class: "my-1 border-t border-line" }
                    for (k, l) in [("trade", "By Trade"), ("group", "By Project Group"), ("housing", "By Housing")] {
                        button { key: "{k}", class: "w-full text-left px-3 py-2 rounded text-sm hover:bg-hover", "data-testid": "{testid}-report-by-{k}",
                            onclick: move |_| { open.set(false); s5print("personnelReport", json!([k])); },
                            "{l}"
                        }
                    }
                }
            }
        }
    }
}
