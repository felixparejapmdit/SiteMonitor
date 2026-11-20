//! Service 5 pages rebuilt to match their originals: the Signatories board
//! (cards with signature image, mapped groups with volunteer counts, unbind,
//! coverage stats) and Reassign Volunteers (move several people at once to a
//! project and area, logged, with a printable reallocation notice).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use dioxus::prelude::*;
use serde_json::{json, Value};
use shared::api::*;
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{EmptyState, Icon, Spinner};
use crate::services::media_canvas;
use crate::state::use_app;
use crate::util;
use crate::views::personnel::REFRESH as WORKERS_CHANGED;
use crate::views::workforce_ops::{new_id, print_with, s, save_record, today, use_records, use_workers};

fn groups_of(v: &Value) -> Vec<String> {
    v["projectGroups"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect()
}

// ---------------------------------------------------------------- signatories

/// Service 5's Authorized Signatories page.
#[component]
pub fn SignatoriesBoard() -> Element {
    let app = use_app();
    let sigs = use_records("signatories");
    let workers = use_workers();
    let meta = use_records("project_groups_meta");
    let mut assigning = use_signal(|| None::<String>);
    let mut confirm_delete = use_signal(|| None::<String>);
    let can_edit = app.can(Permission::ManageSignatories);
    let list = sigs.read().clone().unwrap_or_default();
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    let mut vols: HashMap<String, usize> = HashMap::new();
    for w in &people {
        *vols.entry(w.project_group.clone()).or_default() += 1;
    }
    let mut all_groups: BTreeSet<String> = people.iter().map(|w| w.project_group.clone()).filter(|g| !g.is_empty()).collect();
    if let Some(m) = meta.read().clone().unwrap_or_default().first().and_then(|v| v.as_object().cloned()) {
        all_groups.extend(m.keys().filter(|k| !matches!(k.as_str(), "id" | "updatedAt" | "_clientId")).cloned());
    }
    let mapped: HashSet<String> = list.iter().flat_map(groups_of).collect();
    let unassigned: Vec<String> = all_groups.iter().filter(|g| !mapped.contains(*g)).cloned().collect();
    let covered = all_groups.len() - unassigned.len();
    let pct = if all_groups.is_empty() { 100 } else { covered * 100 / all_groups.len() };
    let save = move |v: Value, msg: &'static str| async move {
        match save_record(app, "signatories", &s(&v, "id"), &v).await {
            Ok(_) => app.success(msg),
            Err(e) => app.error(e),
        }
    };
    let add = move |_| async move {
        let id = new_id("sig");
        if let Err(e) = save_record(app, "signatories", &id, &json!({ "name": "New signatory", "projectGroups": [], "signatureDate": Value::Null })).await {
            app.error(e);
        }
    };
    rsx! {
        div { class: "flex flex-wrap items-center gap-2 justify-end no-print",
            if can_edit {
                button { class: "btn-secondary", "data-testid": "sig-assign-open", onclick: move |_| assigning.set(Some(String::new())), Icon { name: "users" } "Bulk assign groups" }
                button { class: "btn-primary", "data-testid": "sig-add", onclick: add, Icon { name: "plus" } "Add signatory" }
            }
        }
        div { class: "grid md:grid-cols-3 gap-4", "data-testid": "sig-stats",
            div { class: "card p-4 flex items-center gap-3",
                span { class: "w-10 h-10 rounded-lg bg-blue-50 text-blue-700 flex items-center justify-center", Icon { name: "users", size: 18 } }
                div {
                    p { class: "text-[11px] font-semibold uppercase tracking-wide text-muted", "Active signatories" }
                    p { class: "text-2xl font-semibold tabular", "{list.len()}" }
                    p { class: "text-xs text-muted", "Job order releasing officers" }
                }
            }
            div { class: "card p-4 space-y-2",
                div { class: "flex items-center justify-between",
                    p { class: "text-[11px] font-semibold uppercase tracking-wide text-muted", "Group coverage" }
                    span { class: "text-xs font-semibold text-accent-ink", "{pct}%" }
                }
                p { class: "text-2xl font-semibold tabular", "{covered}" span { class: "text-sm text-muted font-normal", " / {all_groups.len()} mapped" } }
                div { class: "progress-track", div { class: "progress-fill", style: "width: {pct}%" } }
            }
            div { class: "card p-4 flex items-start gap-3",
                span { class: "w-10 h-10 rounded-lg bg-amber-50 text-amber-700 flex items-center justify-center", Icon { name: "alert", size: 18 } }
                div { class: "min-w-0",
                    p { class: "text-[11px] font-semibold uppercase tracking-wide text-muted", "Unassigned groups" }
                    p { class: "text-2xl font-semibold tabular", "data-testid": "sig-unassigned", "{unassigned.len()}" }
                    p { class: "text-xs text-muted truncate", {if unassigned.is_empty() { "All groups mapped to signatories".to_string() } else { unassigned.join(", ") }} }
                }
            }
        }
        if list.is_empty() {
            div { class: "card", EmptyState { icon: "pen-tool", title: "No signatories yet", body: "Add the officers who sign job orders, then give each their project groups." } }
        }
        div { class: "grid lg:grid-cols-2 gap-4", "data-testid": "sig-cards",
            for v in list.iter().cloned() {
                {
                    let id = s(&v, "id");
                    let groups = groups_of(&v);
                    let total: usize = groups.iter().map(|g| vols.get(g).copied().unwrap_or(0)).sum();
                    let initial = s(&v, "name").chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
                    let sig_url = s(&v, "signatureUrl");
                    let configured = s(&v, "signatureDate").get(..10).map(String::from).unwrap_or_default();
                    let file_id = format!("sig-png-{id}");
                    rsx! {
                        article { key: "{id}", class: "card p-5 space-y-4", "data-testid": "sig-card", "data-name": "{s(&v, \"name\")}",
                            div { class: "flex items-start gap-3",
                                span { class: "w-10 h-10 rounded-lg bg-blue-600 text-white font-semibold flex items-center justify-center shrink-0", "{initial}" }
                                div { class: "flex-1 min-w-0",
                                    input { class: "field font-semibold text-base", "aria-label": "Signatory name", "data-testid": "sig-name", value: "{s(&v, \"name\")}", disabled: !can_edit,
                                        onchange: { let v = v.clone(); move |e: FormEvent| { let mut n = v.clone(); n["name"] = json!(e.value().trim()); save(n, "Name saved") } } }
                                    p { class: "text-xs text-muted mt-1", "Authorized job order signatory" }
                                }
                                if can_edit {
                                    if confirm_delete() == Some(id.clone()) {
                                        button { class: "btn-danger h-9", "data-testid": "sig-delete-confirm",
                                            onclick: { let id = id.clone(); move |_| { let id = id.clone(); async move {
                                                match crate::views::workforce_ops::delete_record("signatories", &id).await { Ok(_) => app.success("Signatory removed"), Err(e) => app.error(e.message()) }
                                                confirm_delete.set(None);
                                            } } },
                                            "Remove"
                                        }
                                    } else {
                                        button { class: "btn-icon btn-ghost", "aria-label": "Remove signatory", "data-testid": "sig-delete", onclick: { let id = id.clone(); move |_| confirm_delete.set(Some(id.clone())) }, Icon { name: "trash" } }
                                    }
                                }
                            }
                            div { class: "flex justify-between rounded-md border border-line px-3 py-2 text-sm",
                                span { b { "{groups.len()}" } span { class: "text-muted", " groups assigned" } }
                                span { b { "{total}" } span { class: "text-muted", " total volunteers" } }
                            }
                            div { class: "rounded-md border border-line p-3 space-y-2",
                                div { class: "flex items-center justify-between",
                                    p { class: "text-xs font-semibold uppercase tracking-wide", "Digital signature" }
                                    if sig_url.starts_with("data:image/") {
                                        span { class: "pill bg-tag-green-bg text-tag-green-text", "Active digital seal" }
                                    } else {
                                        span { class: "pill bg-tag-gray-bg text-tag-gray-text", "No signature" }
                                    }
                                }
                                div { class: "h-20 rounded border border-dashed border-line flex items-center justify-center bg-surface",
                                    if sig_url.starts_with("data:image/") { img { src: "{sig_url}", alt: "Signature of {s(&v, \"name\")}", class: "max-h-16 object-contain", "data-testid": "sig-image" } }
                                    else { span { class: "text-xs text-muted", "Upload a PNG of the signature" } }
                                }
                                div { class: "flex items-center gap-2 text-xs text-muted",
                                    span { class: "flex-1", if configured.is_empty() { "Not configured" } else { "Configured: {configured}" } }
                                    if can_edit {
                                        input { id: "{file_id}", r#type: "file", accept: "image/png,image/jpeg,image/webp", class: "hidden", "data-testid": "sig-file",
                                            onchange: { let v = v.clone(); let file_id = file_id.clone(); move |_| { let v = v.clone(); let file_id = file_id.clone(); async move {
                                                match media_canvas::file_as_data_url(&file_id, 400_000.0).await {
                                                    Ok(Some(url)) => { let mut n = v.clone(); n["signatureUrl"] = json!(url); n["signatureDate"] = json!(chrono::Utc::now().to_rfc3339()); save(n, "Signature saved").await }
                                                    Ok(None) => {}
                                                    Err(e) => app.error(e),
                                                }
                                            } } } }
                                        label { r#for: "{file_id}", class: "btn-secondary h-8 cursor-pointer", "data-testid": "sig-change", if sig_url.is_empty() { "Upload PNG" } else { "Change PNG" } }
                                        if !sig_url.is_empty() {
                                            button { class: "btn-ghost h-8 text-tag-red-text", "data-testid": "sig-remove-png",
                                                onclick: { let v = v.clone(); move |_| { let mut n = v.clone(); n["signatureUrl"] = json!(""); n["signatureDate"] = Value::Null; save(n, "Signature removed") } },
                                                "Remove"
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "rounded-md border border-line p-3 space-y-2",
                                div { class: "flex items-center justify-between",
                                    p { class: "text-xs font-semibold uppercase tracking-wide", "Mapped project groups" }
                                    span { class: "text-xs text-muted",
                                        "{groups.len()} bound"
                                        if can_edit && !groups.is_empty() {
                                            button { class: "ml-2 underline text-tag-red-text", "data-testid": "sig-unbind-all",
                                                onclick: { let v = v.clone(); move |_| { let mut n = v.clone(); n["projectGroups"] = json!([]); save(n, "All groups unbound") } },
                                                "Unbind all"
                                            }
                                        }
                                    }
                                }
                                div { class: "flex flex-wrap gap-2",
                                    if groups.is_empty() { p { class: "text-sm text-muted", "No groups yet." } }
                                    for g in groups.iter().cloned() {
                                        span { key: "{g}", class: "inline-flex items-center gap-1.5 rounded-md border border-line px-2 py-1 text-sm", "data-testid": "sig-group",
                                            Icon { name: "check", size: 12, class: "text-accent-ink".to_string() }
                                            "{g}"
                                            span { class: "pill bg-blue-50 text-blue-700 text-[11px]", {util::plural(vols.get(&g).copied().unwrap_or(0), "vol")} }
                                            if can_edit {
                                                button { class: "text-muted hover:text-tag-red-text", "aria-label": "Unbind {g}", "data-testid": "sig-unbind",
                                                    onclick: { let v = v.clone(); let g = g.clone(); move |_| { let mut n = v.clone(); n["projectGroups"] = json!(groups_of(&v).into_iter().filter(|x| *x != g).collect::<Vec<_>>()); save(n, "Group unbound") } },
                                                    Icon { name: "x", size: 12 }
                                                }
                                            }
                                        }
                                    }
                                }
                                if can_edit {
                                    div { class: "flex items-center justify-between pt-1",
                                        button { class: "btn-secondary h-8", "data-testid": "sig-manage", onclick: { let id = id.clone(); move |_| assigning.set(Some(id.clone())) }, Icon { name: "edit" } "Assign / manage groups" }
                                        span { class: "text-xs text-muted", if unassigned.is_empty() { "All assigned" } else { "{unassigned.len()} unassigned" } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(who) = assigning() {
            crate::views::workforce_extra::SignatoryAssign { preselect: who, on_close: move |_| assigning.set(None) }
        }
    }
}

// ------------------------------------------------------------------ dashboard

/// Who is behind a number (Service 5's roster pop-ups on the dashboard).
#[component]
pub fn RosterModal(title: String, people: Vec<Worker>, on_close: EventHandler<()>) -> Element {
    rsx! {
        crate::components::ui::Modal { title: title.clone(), wide: true, on_close: move |_| on_close.call(()),
            div { class: "space-y-3", "data-testid": "roster-modal",
                div { class: "flex justify-between items-center",
                    p { class: "text-sm text-muted", {util::plural(people.len(), "person")} }
                    button { class: "btn-secondary no-print", onclick: move |_| print_with("size: A4; margin: 10mm"), Icon { name: "archive" } "Print" }
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
            RosterModal { title: t, people: list, on_close: move |_| roster.set(None) }
        }
    }
}

// ---------------------------------------------------------------------- duty

/// Change a night's duty roster by hand (Service 5's replace / remove / assign,
/// each with an override reason kept in the history log).
#[component]
pub fn DutyOverrides() -> Element {
    let app = use_app();
    let areas = use_records("duty_areas");
    let schedules = use_records("duty_schedules");
    let workers = use_workers();
    let mut date = use_signal(today);
    let mut reason = use_signal(String::new);
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived && !w.a_number.is_empty()).collect();
    let name = |a: &str| people.iter().find(|w| w.a_number == a).map(|w| w.name.clone()).unwrap_or_else(|| format!("A# {a}"));
    let day: Value = schedules.read().clone().unwrap_or_default().into_iter().find(|d| s(d, "dateStr") == date()).unwrap_or(json!({ "assignments": [] }));
    let assignments: Vec<Value> = day["assignments"].as_array().cloned().unwrap_or_default();
    let change = move |area: String, from: Option<String>, to: Option<String>, day: Value| async move {
        if reason().trim().is_empty() {
            return app.error("Write the reason for the change first.");
        }
        let d = date();
        let mut list: Vec<Value> = day["assignments"].as_array().cloned().unwrap_or_default();
        match (&from, &to) {
            (Some(f), Some(t)) => {
                for a in list.iter_mut().filter(|a| s(a, "areaId") == area && s(a, "aNumber") == *f) {
                    a["aNumber"] = json!(t);
                }
            }
            (Some(f), None) => list.retain(|a| !(s(a, "areaId") == area && s(a, "aNumber") == *f)),
            (None, Some(t)) => list.push(json!({ "aNumber": t, "areaId": area })),
            _ => return,
        }
        if let Err(e) = save_record(app, "duty_schedules", &d, &json!({ "dateStr": d, "assignments": list })).await {
            return app.error(e);
        }
        let id = new_id("asg");
        let entry = json!({ "aNumber": to.clone().or(from.clone()).unwrap_or_default(), "workerName": "", "date": d, "field": "duty",
            "oldValue": from.unwrap_or_default(), "newValue": to.unwrap_or_default(), "reason": reason().trim(), "timestamp": js_sys::Date::now() as i64 });
        let _ = save_record(app, "assignment_log", &id, &entry).await;
        app.success("Duty roster changed");
    };
    rsx! {
        section { class: "card p-4 space-y-3", "data-testid": "duty-overrides",
            div { class: "flex flex-wrap items-end gap-3",
                p { class: "font-semibold flex-1", "Change a night by hand" }
                div { label { class: "label", r#for: "do-date", "Night" } input { id: "do-date", r#type: "date", class: "field", "data-testid": "duty-override-date", value: "{date}", oninput: move |e| date.set(e.value()) } }
                div { class: "min-w-[16rem]", label { class: "label", r#for: "do-reason", "Reason for the change" } input { id: "do-reason", class: "field", "data-testid": "duty-override-reason", value: "{reason}", oninput: move |e| reason.set(e.value()) } }
            }
            for a in areas.read().clone().unwrap_or_default() {
                {
                    let aid = s(&a, "id");
                    let here: Vec<String> = assignments.iter().filter(|x| s(x, "areaId") == aid).map(|x| s(x, "aNumber")).collect();
                    rsx! {
                        div { key: "{aid}", class: "border-t border-line pt-2 space-y-1.5", "data-testid": "duty-override-area",
                            p { class: "text-sm font-medium", {s(&a, "name")} }
                            for who in here.iter().cloned() {
                                div { key: "{who}", class: "flex flex-wrap items-center gap-2 text-sm",
                                    span { class: "flex-1", {name(&who)} }
                                    select { class: "field w-auto h-8 text-sm", "aria-label": "Replace", "data-testid": "duty-replace",
                                        onchange: { let (aid, who, day) = (aid.clone(), who.clone(), day.clone()); move |e: FormEvent| change(aid.clone(), Some(who.clone()), Some(e.value()), day.clone()) },
                                        option { value: "", "Replace with…" }
                                        for p in people.iter().filter(|p| p.status == WorkerStatus::Present && p.a_number != who) { option { key: "{p.id}", value: "{p.a_number}", "{p.name}" } }
                                    }
                                    button { class: "btn-ghost h-8", "data-testid": "duty-remove",
                                        onclick: { let (aid, who, day) = (aid.clone(), who.clone(), day.clone()); move |_| change(aid.clone(), Some(who.clone()), None, day.clone()) },
                                        "Remove"
                                    }
                                }
                            }
                            select { class: "field w-auto h-8 text-sm", "aria-label": "Assign someone", "data-testid": "duty-assign",
                                onchange: { let (aid, day) = (aid.clone(), day.clone()); move |e: FormEvent| change(aid.clone(), None, Some(e.value()), day.clone()) },
                                option { value: "", "Assign someone…" }
                                for p in people.iter().filter(|p| p.status == WorkerStatus::Present && !here.contains(&p.a_number)) { option { key: "{p.id}", value: "{p.a_number}", "{p.name}" } }
                            }
                        }
                    }
                }
            }
        }
    }
}

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

/// A group's members with their role, changeable, and "remove from group".
#[component]
pub fn OrgMembers(people: Vec<Worker>) -> Element {
    let app = use_app();
    let set_role = move |w: Worker, role: String| async move {
        let mut req = crate::views::personnel::save_request(&w, w.archived);
        req.group_role = role;
        match api::put::<_, Worker>(&format!("/workers/{}", w.id), &req).await {
            Ok(_) => {
                *WORKERS_CHANGED.write() += 1;
                app.success("Role changed");
            }
            Err(e) => app.error(e.message()),
        }
    };
    let remove = move |w: Worker| async move {
        let req = ReassignRequest { project_group: String::new(), area: w.area.clone(), department: w.department.clone(), reason: "Removed from group (org chart)".into() };
        match api::put::<_, Worker>(&format!("/workers/{}/reassign", w.id), &req).await {
            Ok(_) => {
                *WORKERS_CHANGED.write() += 1;
                app.success(format!("{} removed from the group", w.name));
            }
            Err(e) => app.error(e.message()),
        }
    };
    rsx! {
        details { class: "no-print",
            summary { class: "text-sm text-accent-ink cursor-pointer", "data-testid": "org-members", "Members and roles" }
            ul { class: "pt-2 space-y-1",
                for w in people.iter().cloned() {
                    li { key: "{w.id}", class: "flex items-center gap-2 text-sm", "data-testid": "org-member",
                        span { class: "flex-1 truncate", "{w.name}" }
                        select { class: "field w-auto h-8 text-sm", "aria-label": "Role of {w.name}", "data-testid": "org-role",
                            onchange: { let w = w.clone(); move |e: FormEvent| set_role(w.clone(), e.value()) },
                            for r in ["Katiwala", "II Katiwala", "Kalihim", "Member"] {
                                option { key: "{r}", value: "{r}", selected: w.group_role == r || (r == "Member" && w.group_role.is_empty()), "{r}" }
                            }
                        }
                        button { class: "btn-icon btn-ghost", "aria-label": "Remove {w.name} from the group", "data-testid": "org-remove", onclick: { let w = w.clone(); move |_| remove(w.clone()) }, Icon { name: "x", size: 14 } }
                    }
                }
            }
        }
    }
}

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
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(|| None::<Notice>);
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    let projects: Vec<String> = people.iter().map(|w| w.project.clone()).filter(|p| !p.is_empty()).collect::<BTreeSet<_>>().into_iter().collect();
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
                let (to_project, to_area) = (project().trim().to_string(), area().trim().to_string());
                if to_project.is_empty() && to_area.is_empty() {
                    return app.error("Choose the project or area they move to.");
                }
                busy.set(true);
                let mut moved = vec![];
                let mut sources: BTreeMap<String, ()> = BTreeMap::new();
                for w in &picked {
                    let new_area = if to_area.is_empty() { w.area.clone() } else { to_area.clone() };
                    let req = ReassignRequest { project_group: w.project_group.clone(), area: new_area, department: w.department.clone(), reason: reason() };
                    let r1 = api::put::<_, Worker>(&format!("/workers/{}/reassign", w.id), &req).await;
                    let r2 = if to_project.is_empty() || to_project == w.project {
                        Ok(())
                    } else {
                        let r = api::put::<_, Worker>(&format!("/workers/{}/profile", w.id), &json!({ "project": to_project, "isAssignedToday": true, "taskStartedAt": chrono::Utc::now().to_rfc3339() })).await.map(|_| ());
                        if r.is_ok() {
                            // The project move goes in the assignment log too.
                            let id = new_id("asg");
                            let _ = save_record(app, "assignment_log", &id, &json!({ "aNumber": w.a_number, "workerName": w.name, "date": today(), "field": "project", "oldValue": w.project, "newValue": to_project, "reason": reason(), "timestamp": js_sys::Date::now() as i64, "by": app.current_user.peek().as_ref().map(|u| u.name.clone()).unwrap_or_default() })).await;
                        }
                        r
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
                p { class: "font-semibold", "Move volunteers" }
                div { class: "relative",
                    label { class: "label", r#for: "realloc-search", "Find and add people" }
                    input { id: "realloc-search", class: "field", "data-testid": "realloc-search", placeholder: "Name, A# or trade", value: "{q}", oninput: move |e| q.set(e.value()) }
                    if !matches.is_empty() {
                        ul { class: "absolute z-20 left-0 right-0 mt-1 card shadow-pop p-1 max-h-64 overflow-y-auto",
                            for w in matches {
                                li { key: "{w.id}",
                                    button { class: "w-full text-left px-3 py-2 rounded hover:bg-hover text-sm", "data-testid": "realloc-match",
                                        onclick: move |_| { chosen.write().push(w.id); q.set(String::new()); },
                                        "{w.name}" span { class: "text-muted ml-2", "A# {w.a_number} · {w.project}" }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "flex flex-wrap gap-2", "data-testid": "realloc-chips",
                    if picked.is_empty() { p { class: "text-sm text-muted", "Nobody added yet." } }
                    for w in picked.iter().cloned() {
                        span { key: "{w.id}", class: "inline-flex items-center gap-1.5 rounded-md bg-accent-soft text-accent-ink px-2 py-1 text-sm", "data-testid": "realloc-chip",
                            "{w.name}"
                            span { class: "text-xs opacity-70", {if w.project.is_empty() { "no project".to_string() } else { w.project.clone() }} }
                            button { "aria-label": "Remove {w.name}", onclick: move |_| chosen.write().retain(|x| *x != w.id), Icon { name: "x", size: 12 } }
                        }
                    }
                }
                div { class: "grid sm:grid-cols-2 gap-3",
                    div {
                        label { class: "label", r#for: "realloc-project", "To project" }
                        input { id: "realloc-project", class: "field", list: "realloc-projects", "data-testid": "realloc-project", value: "{project}", oninput: move |e| project.set(e.value()) }
                        datalist { id: "realloc-projects", for p in projects { option { key: "{p}", value: "{p}" } } }
                    }
                    div {
                        label { class: "label", r#for: "realloc-area", "To area" }
                        input { id: "realloc-area", class: "field", list: "realloc-areas", "data-testid": "realloc-area", value: "{area}", oninput: move |e| area.set(e.value()) }
                        datalist { id: "realloc-areas", for a in areas { option { key: "{a}", value: "{a}" } } }
                    }
                }
                div {
                    label { class: "label", r#for: "realloc-reason", "Reason" }
                    input { id: "realloc-reason", class: "field", "data-testid": "realloc-reason", placeholder: "Kept in the history log", value: "{reason}", oninput: move |e| reason.set(e.value()) }
                }
                button { class: "btn-primary", "data-testid": "realloc-apply", disabled: picked.is_empty() || busy(), onclick: apply,
                    if busy() { Spinner {} } else { Icon { name: "move" } }
                    "Reassign {util::plural(picked.len(), \"person\")}"
                }
            }
            section { class: "card p-5 space-y-3", "data-testid": "realloc-notice-card",
                div { class: "flex items-center justify-between",
                    p { class: "font-semibold", "Reallocation notice" }
                    if notice().is_some() {
                        button { class: "btn-secondary", "data-testid": "realloc-print", onclick: move |_| print_with("size: A5 landscape; margin: 10mm"), Icon { name: "archive" } "Print / share" }
                    }
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
