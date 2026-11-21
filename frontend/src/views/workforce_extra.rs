//! The rest of Service 5's screens, merged after the audit: dashboard panels
//! (tonight's duty, deployability, housing, open health episodes), residences
//! and housing, signatory bulk assignment, project group management, project
//! roster history and person dossiers, visitations (schedule / history /
//! coverage), visitation group members and monthly sheets, the overtime
//! calculator, the duty month view and selective backups.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{Datelike, Duration, Local, NaiveDate};
use dioxus::prelude::*;
use serde_json::{json, Value};
use shared::api::*;
use shared::*;

use crate::api;
use crate::components::ui::{EmptyState, Icon, Modal, Spinner};
use crate::state::use_app;
use crate::util;
use crate::views::personnel::REFRESH as WORKERS_CHANGED;
use crate::views::workforce_ops::{by_a_number, choice, f, name_of, new_id, s, s5print, save_record, today, use_records, use_workers, Kind, RecordModal, S5Preview, Table};

/// A small switch between views inside a tab.
#[component]
pub fn Segmented(options: Vec<&'static str>, current: Signal<&'static str>, testid: &'static str) -> Element {
    let mut current = current;
    rsx! {
        div { class: "inline-flex flex-wrap rounded-md border border-line p-0.5 bg-surface no-print", role: "group",
            for o in options {
                button {
                    key: "{o}",
                    class: if current() == o { "px-3 h-8 rounded text-sm bg-accent-soft text-accent-ink font-medium" } else { "px-3 h-8 rounded text-sm text-muted hover:text-ink" },
                    "aria-pressed": current() == o,
                    "data-testid": "{testid}-{o}",
                    onclick: move |_| current.set(o),
                    "{o}"
                }
            }
        }
    }
}

fn active(workers: &Resource<Vec<Worker>>) -> Vec<Worker> {
    workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect()
}

fn date_of(d: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(d.get(..10).unwrap_or(""), "%Y-%m-%d").ok()
}

// ---------------------------------------------------------------- dashboard

/// Service 5's deployability tiers (`getWorkerDeployability`).
pub fn tier(w: &Worker) -> &'static str {
    if w.status != WorkerStatus::Present {
        "Off-site (absent or on pass)"
    } else if w.department == "Residential Only" {
        "Residential only"
    } else if w.department == "Job-site Construction" {
        "Job-site deployable"
    } else {
        "Shops, garden and offices"
    }
}

#[component]
pub fn DashboardMore() -> Element {
    let app = use_app();
    let workers = use_workers();
    let schedules = use_records("duty_schedules");
    let areas = use_records("duty_areas");
    let clinic = app.can(Permission::ManageClinic);
    let episodes = use_resource(move || async move {
        if clinic { api::get::<Vec<Value>>("/workforce/records/health_events").await.unwrap_or_default() } else { vec![] }
    });
    let people = active(&workers);
    let map = by_a_number(&people);
    let area_name: HashMap<String, String> = areas.read().clone().unwrap_or_default().iter().map(|a| (s(a, "id"), s(a, "name"))).collect();
    let tonight: Vec<(String, String)> = schedules
        .read()
        .clone()
        .unwrap_or_default()
        .iter()
        .find(|d| s(d, "dateStr") == today())
        .and_then(|d| d["assignments"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .map(|a| (area_name.get(&s(a, "areaId")).cloned().unwrap_or_default(), name_of(&map, &s(a, "aNumber"))))
        .collect();
    let mut tiers: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut housing: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for w in &people {
        *tiers.entry(tier(w)).or_default() += 1;
        let h = if !w.housing.is_empty() { w.housing.clone() } else if !w.residence.is_empty() { w.residence.clone() } else { "Not set".into() };
        let e = housing.entry(h).or_default();
        e.1 += 1;
        if w.status == WorkerStatus::Present {
            e.0 += 1;
        }
    }
    let active_eps: Vec<Value> = episodes.read().clone().unwrap_or_default().into_iter().filter(|e| s(e, "status") != "cleared").collect();
    let open = active_eps.len();
    let mut by_diagnosis: BTreeMap<String, usize> = BTreeMap::new();
    for e in &active_eps {
        *by_diagnosis.entry(if s(e, "diagnosis").is_empty() { "Not recorded".into() } else { s(e, "diagnosis") }).or_default() += 1;
    }
    // Service 5 stale tasks: present, on the same task for 3 days or more, alerts not turned off.
    let now = Local::now().date_naive();
    let stale: Vec<(Worker, i64)> = people
        .iter()
        .filter(|w| w.status == WorkerStatus::Present && !w.task.is_empty() && !w.stale_off)
        .filter_map(|w| date_of(&w.task_since).map(|d| (w.clone(), (now - d).num_days())))
        .filter(|(_, days)| *days >= 3)
        .collect();
    let dismiss = move |w: Worker| async move {
        match api::put::<_, Worker>(&format!("/workers/{}/profile", w.id), &json!({ "disableStaleAlerts": true })).await {
            Ok(_) => *WORKERS_CHANGED.write() += 1,
            Err(e) => app.error(e.message()),
        }
    };
    rsx! {
        div { class: "grid lg:grid-cols-3 gap-4",
            section { class: "card p-4 space-y-2", "data-testid": "wf-tonight",
                p { class: "font-semibold", "Tonight's duty" }
                if tonight.is_empty() { p { class: "text-sm text-muted", "No duty roster for tonight yet." } }
                for (i, (area, who)) in tonight.iter().enumerate() {
                    p { key: "{i}", class: "text-sm flex justify-between gap-2", span { class: "text-muted truncate", "{area}" } span { "{who}" } }
                }
            }
            section { class: "card p-4 space-y-2", "data-testid": "wf-deploy",
                p { class: "font-semibold", "Deployability today" }
                for (t, n) in tiers { p { key: "{t}", class: "text-sm flex justify-between", span { "{t}" } span { class: "tabular", "{n}" } } }
                if clinic {
                    p { class: "text-sm flex justify-between pt-2 border-t border-line", span { "Open health episodes" } span { class: "tabular", "data-testid": "wf-health", "{open}" } }
                    for (d, n) in by_diagnosis { p { key: "{d}", class: "text-xs text-muted flex justify-between pl-3", span { "{d}" } span { class: "tabular", "{n}" } } }
                }
            }
            section { class: "card p-4 space-y-2", "data-testid": "wf-stale",
                p { class: "font-semibold", "Same task for 3+ days" span { class: "text-muted font-normal", " · {stale.len()} flagged" } }
                if !stale.is_empty() {
                    // Service 5: mark everyone flagged at once, or dismiss the whole list.
                    div { class: "flex flex-wrap gap-1",
                        for (st, label) in [(WorkerStatus::Present, "All present"), (WorkerStatus::Absent, "All absent"), (WorkerStatus::OnPass, "All passes")] {
                            button { key: "{label}", class: "btn-secondary h-7 px-2 text-xs", "data-testid": "stale-all-{st:?}",
                                onclick: { let list: Vec<Worker> = stale.iter().map(|x| x.0.clone()).collect(); move |_| { let list = list.clone(); async move {
                                    for w in list {
                                        if let Err(e) = api::put::<_, Worker>(&format!("/workers/{}/status", w.id), &shared::api::WorkerStatusRequest { status: st, reason: String::new() }).await { return app.error(e.message()); }
                                    }
                                    *WORKERS_CHANGED.write() += 1;
                                } } }, "{label}" }
                        }
                        button { class: "btn-ghost h-7 px-2 text-xs", "data-testid": "stale-dismiss-all",
                            onclick: { let list: Vec<Worker> = stale.iter().map(|x| x.0.clone()).collect(); move |_| { let list = list.clone(); async move { for w in list { dismiss(w).await; } } } }, "Dismiss page" }
                    }
                }
                if stale.is_empty() { p { class: "text-sm text-muted", "Nobody is stuck on an old task." } }
                for (w, days) in stale.iter().cloned() {
                    div { key: "{w.id}", class: "text-sm flex items-center gap-2",
                        span { class: "flex-1 truncate", "{w.name}" span { class: "text-muted", " · {w.task}" } }
                        span { class: "tabular text-muted", "{days} days" }
                        button { class: "btn-ghost h-7 px-2 text-xs", "aria-label": "Stop alerts for {w.name}", "data-testid": "stale-dismiss", onclick: move |_| dismiss(w.clone()), "Dismiss" }
                    }
                }
            }
            section { class: "card p-4 space-y-2", "data-testid": "wf-housing",
                p { class: "font-semibold", "Housing" }
                for (h, (p, t)) in housing.into_iter().take(10) {
                    p { key: "{h}", class: "text-sm flex justify-between gap-2", span { class: "truncate", "{h}" } span { class: "tabular text-muted", "{p} of {t} present" } }
                }
            }
        }
    }
}

/// Service 5's "Work residences & housing" (on the Departments tab).
#[component]
pub fn HousingTables() -> Element {
    let workers = use_workers();
    let people = active(&workers);
    let table = |key: fn(&Worker) -> &String| -> Vec<(String, Vec<String>)> {
        let mut by: BTreeMap<String, [usize; 4]> = BTreeMap::new();
        for w in &people {
            let k = if key(w).trim().is_empty() { "Not set".to_string() } else { key(w).trim().to_string() };
            let e = by.entry(k).or_default();
            e[0] += 1;
            e[match w.status {
                WorkerStatus::Present => 1,
                WorkerStatus::Absent => 2,
                WorkerStatus::OnPass => 3,
            }] += 1;
        }
        by.into_iter().map(|(k, n)| (k.clone(), vec![k, n[0].to_string(), n[1].to_string(), n[2].to_string(), n[3].to_string()])).collect()
    };
    rsx! {
        p { class: "font-semibold pt-2", "Work residences" }
        Table { head: vec!["Residence", "People", "Present", "Absent", "On pass"], rows: table(|w| &w.residence) }
        p { class: "font-semibold pt-2", "Housing groups" }
        Table { head: vec!["Housing", "People", "Present", "Absent", "On pass"], rows: table(|w| &w.housing) }
    }
}

// ------------------------------------------------------------- signatories

/// Service 5's "Bulk assign groups": tick the project groups one signatory signs
/// for. A group has one signatory, so it leaves anyone else's list.
#[component]
pub fn SignatoryAssign(on_close: EventHandler<()>, #[props(default)] preselect: String) -> Element {
    let app = use_app();
    let sigs = use_records("signatories");
    let workers = use_workers();
    let mut who = use_signal(String::new);
    let mut ticked = use_signal(HashSet::<String>::new);
    let mut busy = use_signal(|| false);
    let list = sigs.read().clone().unwrap_or_default();
    // Opened from one signatory's card: start on that signatory and its groups.
    if !preselect.is_empty() && who.peek().is_empty() {
        if let Some(v) = list.iter().find(|v| s(v, "id") == preselect) {
            ticked.set(v["projectGroups"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect());
            who.set(preselect.clone());
        }
    }
    let mut groups: Vec<String> = active(&workers).into_iter().map(|w| w.project_group).filter(|g| !g.is_empty()).collect();
    for v in &list {
        groups.extend(v["projectGroups"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)));
    }
    groups.sort();
    groups.dedup();
    let owner: HashMap<String, String> =
        list.iter().flat_map(|v| v["projectGroups"].as_array().into_iter().flatten().filter_map(|x| x.as_str()).map(|g| (g.to_string(), s(v, "name"))).collect::<Vec<_>>()).collect();
    let pick = {
        let list = list.clone();
        move |e: FormEvent| {
            let id = e.value();
            let mine: HashSet<String> = list
                .iter()
                .find(|v| s(v, "id") == id)
                .and_then(|v| v["projectGroups"].as_array().cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect();
            ticked.set(mine);
            who.set(id);
        }
    };
    let save = move |_| {
        let list = list.clone();
        async move {
            busy.set(true);
            let mine = ticked();
            for v in list {
                let id = s(&v, "id");
                let before: Vec<String> = v["projectGroups"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect();
                let mut after: Vec<String> = if id == who() { mine.iter().cloned().collect() } else { before.iter().filter(|g| !mine.contains(*g)).cloned().collect() };
                after.sort();
                if after != before {
                    let mut next = v.clone();
                    next["projectGroups"] = json!(after);
                    if let Err(e) = save_record(app, "signatories", &id, &next).await {
                        app.error(e);
                        busy.set(false);
                        return;
                    }
                }
            }
            app.success("Groups assigned");
            busy.set(false);
            on_close.call(());
        }
    };
    rsx! {
        Modal { title: "Assign project groups to a signatory", wide: true, on_close: move |_| on_close.call(()),
            div { class: "space-y-4", "data-testid": "sig-assign",
                div {
                    label { class: "label", r#for: "sa-who", "Signatory" }
                    select { id: "sa-who", class: "field", "data-testid": "sig-assign-who", onchange: pick,
                        option { value: "", "Choose…" }
                        for v in sigs.read().clone().unwrap_or_default() { option { key: "{s(&v, \"id\")}", value: "{s(&v, \"id\")}", selected: s(&v, "id") == who(), {s(&v, "name")} } }
                    }
                }
                if !who().is_empty() {
                    div { class: "max-h-80 overflow-y-auto grid sm:grid-cols-2 gap-1",
                        for g in groups {
                            label { key: "{g}", class: "flex items-center gap-2 text-sm py-1",
                                input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "sig-assign-group", checked: ticked().contains(&g),
                                    onchange: { let g = g.clone(); move |e: FormEvent| { if e.checked() { ticked.write().insert(g.clone()); } else { ticked.write().remove(&g); } } } }
                                span { class: "flex-1", "{g}" }
                                if let Some(o) = owner.get(&g) { span { class: "text-xs text-muted truncate", "{o}" } }
                            }
                        }
                    }
                }
                div { class: "modal-actions",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button { class: "btn-primary", "data-testid": "sig-assign-save", disabled: who().is_empty() || busy(), onclick: save, if busy() { Spinner {} } "Save" }
                }
            }
        }
    }
}

// ----------------------------------------------------------- project groups

async fn move_people(people: &[Worker], to: &str, reason: &str) -> Result<(), String> {
    for w in people {
        let req = ReassignRequest { project_group: to.to_string(), area: w.area.clone(), department: w.department.clone(), reason: reason.to_string() };
        api::put::<_, Worker>(&format!("/workers/{}/reassign", w.id), &req).await.map_err(|e| format!("{}: {}", w.name, e.message()))?;
    }
    Ok(())
}

/// Service 5's project groups: create, rename, set the area and signatory,
/// add or remove members (each move logged with a reason), delete.
#[component]
pub fn GroupModal(name: Option<String>, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let workers = use_workers();
    let meta = use_records("project_groups_meta");
    let sigs = use_records("signatories");
    let old = name.clone().unwrap_or_default();
    let meta_map: Value = meta.read().clone().unwrap_or_default().into_iter().next().unwrap_or(json!({}));
    let mut title = use_signal(|| old.clone());
    let mut area = use_signal(String::new);
    let mut signer = use_signal(String::new);
    let mut reason = use_signal(String::new);
    let mut members = use_signal(|| None::<HashSet<uuid::Uuid>>);
    let mut confirm_delete = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let people = active(&workers);
    if members.peek().is_none() && !people.is_empty() {
        members.set(Some(people.iter().filter(|w| !old.is_empty() && w.project_group == old).map(|w| w.id).collect()));
        area.set(s(&meta_map, &old));
        signer.set(
            sigs.peek().clone().unwrap_or_default().iter().find(|v| v["projectGroups"].as_array().is_some_and(|a| a.iter().any(|x| x.as_str() == Some(&old)))).map(|v| s(v, "id")).unwrap_or_default(),
        );
    }
    let ticked = members().unwrap_or_default();
    let sig_list = sigs.read().clone().unwrap_or_default();
    let save = {
        let people = people.clone();
        let old = old.clone();
        let meta_map = meta_map.clone();
        let sig_list = sig_list.clone();
        move |delete: bool| {
            let people = people.clone();
            let old = old.clone();
            let mut meta_map = meta_map.clone();
            let sig_list = sig_list.clone();
            async move {
                let new = if delete { String::new() } else { title().trim().to_string() };
                if !delete && new.is_empty() {
                    return app.error("Give the group a name.");
                }
                busy.set(true);
                let ticked = members().unwrap_or_default();
                let why = if reason().trim().is_empty() { if delete { "Group deleted".to_string() } else { "Group change".to_string() } } else { reason().trim().to_string() };
                let joining: Vec<Worker> = if delete { vec![] } else { people.iter().filter(|w| ticked.contains(&w.id) && w.project_group != new).cloned().collect() };
                let leaving: Vec<Worker> = people.iter().filter(|w| !old.is_empty() && w.project_group == old && (delete || !ticked.contains(&w.id))).cloned().collect();
                let r = async {
                    move_people(&joining, &new, &why).await?;
                    move_people(&leaving, "", &why).await?;
                    if let Some(m) = meta_map.as_object_mut() {
                        if !old.is_empty() {
                            m.remove(&old);
                        }
                        if !new.is_empty() {
                            m.insert(new.clone(), json!(area().trim()));
                        }
                    }
                    save_record(app, "project_groups_meta", "_", &meta_map).await?;
                    for v in &sig_list {
                        let before: Vec<String> = v["projectGroups"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect();
                        let mut after: Vec<String> = before.iter().filter(|g| **g != old && **g != new).cloned().collect();
                        if !new.is_empty() && s(v, "id") == signer() {
                            after.push(new.clone());
                        }
                        after.sort();
                        if after != before {
                            let mut next = v.clone();
                            next["projectGroups"] = json!(after);
                            save_record(app, "signatories", &s(v, "id"), &next).await?;
                        }
                    }
                    Ok::<_, String>(())
                }
                .await;
                *WORKERS_CHANGED.write() += 1;
                busy.set(false);
                match r {
                    Ok(_) => {
                        app.success(if delete { "Group deleted" } else { "Group saved" });
                        on_close.call(());
                    }
                    Err(e) => app.error(e),
                }
            }
        }
    };
    let save2 = save.clone();
    let mut q = use_signal(String::new);
    let ql = q().to_lowercase();
    rsx! {
        Modal { title: if old.is_empty() { "New project group".to_string() } else { format!("Project group: {old}") }, wide: true, on_close: move |_| on_close.call(()),
            div { class: "space-y-4", "data-testid": "group-form",
                div { class: "grid sm:grid-cols-2 gap-3",
                    div { label { class: "label", r#for: "gm-name", "Group name" } input { id: "gm-name", class: "field", "data-testid": "gm-name", placeholder: "e.g. CC | Electricians", value: "{title}", oninput: move |e| title.set(e.value()) } }
                    div { label { class: "label", r#for: "gm-area", "Area" } input { id: "gm-area", class: "field", "data-testid": "gm-area", value: "{area}", oninput: move |e| area.set(e.value()) } }
                    div {
                        label { class: "label", r#for: "gm-sig", "Signatory" }
                        select { id: "gm-sig", class: "field", "data-testid": "gm-sig", onchange: move |e| signer.set(e.value()),
                            option { value: "", selected: signer().is_empty(), "None" }
                            for v in sig_list.iter() { option { key: "{s(v, \"id\")}", value: "{s(v, \"id\")}", selected: s(v, "id") == signer(), {s(v, "name")} } }
                        }
                    }
                    div { label { class: "label", r#for: "gm-reason", "Reason for the change" } input { id: "gm-reason", class: "field", "data-testid": "gm-reason", placeholder: "Kept in the history log", value: "{reason}", oninput: move |e| reason.set(e.value()) } }
                }
                div { class: "space-y-2",
                    p { class: "label", "Members ({ticked.len()})" }
                    input { class: "field", placeholder: "Find a person", "aria-label": "Find a person", value: "{q}", oninput: move |e| q.set(e.value()) }
                    div { class: "max-h-72 overflow-y-auto grid sm:grid-cols-2 gap-1", "data-testid": "gm-members",
                        for w in people.iter().filter(|w| ql.is_empty() || w.name.to_lowercase().contains(&ql) || w.a_number.contains(&ql)).cloned() {
                            label { key: "{w.id}", class: "flex items-center gap-2 text-sm py-1",
                                input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "gm-member", checked: ticked.contains(&w.id),
                                    onchange: move |e| { if let Some(m) = members.write().as_mut() { if e.checked() { m.insert(w.id); } else { m.remove(&w.id); } } } }
                                span { class: "flex-1 truncate", "{w.name}" }
                                if !w.project_group.is_empty() && w.project_group != old { span { class: "text-xs text-muted truncate", "{w.project_group}" } }
                            }
                        }
                    }
                }
                div { class: "modal-actions",
                    if !old.is_empty() {
                        if confirm_delete() {
                            button { class: "btn-danger mr-auto", "data-testid": "gm-delete-confirm", disabled: busy(), onclick: move |_| save2(true), "Delete the group (members become ungrouped)" }
                        } else {
                            button { class: "btn-ghost text-tag-red-text mr-auto", "data-testid": "gm-delete", onclick: move |_| confirm_delete.set(true), Icon { name: "trash" } "Delete group" }
                        }
                    }
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button { class: "btn-primary", "data-testid": "gm-save", disabled: busy(), onclick: move |_| save(false), if busy() { Spinner {} } "Save" }
                }
            }
        }
    }
}

/// Every group's roster on paper (print-only), grouped by area when asked.
#[component]
pub fn GroupRosters(by_area: bool) -> Element {
    let workers = use_workers();
    let meta = use_records("project_groups_meta");
    let meta_map: Value = meta.read().clone().unwrap_or_default().into_iter().next().unwrap_or(json!({}));
    let mut groups: BTreeMap<(String, String), Vec<Worker>> = BTreeMap::new();
    for w in active(&workers).into_iter().filter(|w| !w.project_group.is_empty()) {
        let area = if by_area { s(&meta_map, &w.project_group) } else { String::new() };
        groups.entry((area, w.project_group.clone())).or_default().push(w);
    }
    rsx! {
        div { class: "print-only space-y-4",
            p { class: "font-semibold", "Project group rosters — {util::long_day(chrono::Utc::now())}" }
            for ((area, g), list) in groups {
                section { key: "{area}{g}", class: "break-inside-avoid",
                    p { class: "font-medium text-sm", "{g}" if !area.is_empty() { span { class: "text-muted", " · {area}" } } }
                    table { class: "w-full text-xs border-collapse",
                        tbody {
                            for w in list {
                                tr { key: "{w.id}", class: "border-b",
                                    td { class: "p-1 w-16", "{w.a_number}" } td { class: "p-1", "{w.name}" } td { class: "p-1", "{w.group_role}" } td { class: "p-1", "{w.trade}" } td { class: "p-1", "{w.status.label()}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Group membership changes from the assignment log (Service 5's group change log).
#[component]
pub fn GroupChangeLog(on_close: EventHandler<()>) -> Element {
    let asg = use_records("assignment_log");
    let mut list: Vec<Value> = asg.read().clone().unwrap_or_default().into_iter().filter(|e| s(e, "field") == "projectGroup").collect();
    list.sort_by(|a, b| s(b, "date").cmp(&s(a, "date")).then(b["timestamp"].as_i64().cmp(&a["timestamp"].as_i64())));
    let rows: Vec<(String, Vec<String>)> = list
        .iter()
        .enumerate()
        .map(|(i, e)| (format!("{i}"), vec![s(e, "date"), s(e, "workerName"), s(e, "oldValue"), s(e, "newValue"), s(e, "reason"), s(e, "by")]))
        .collect();
    rsx! {
        Modal { title: "Group change log", wide: true, on_close: move |_| on_close.call(()),
            div { class: "space-y-3", "data-testid": "group-log",
                div { class: "flex justify-end no-print", button { class: "btn-secondary", "data-testid": "group-log-print", onclick: move |_| s5print("projectGroups", json!([{ "kind": "changes", "range": "all" }])), Icon { name: "archive" } "Print" } }
                Table { head: vec!["Date", "Person", "From", "To", "Reason", "By"], rows, empty: "No group changes logged yet." }
            }
        }
    }
}

// ------------------------------------------------------- analytics & history

/// Service 5's "Project Roster History": who was on a project, day by day.
#[component]
pub fn ProjectHistory() -> Element {
    let rows = use_resource(|| async { api::get::<Vec<Value>>("/workforce/project-history").await.unwrap_or_default() });
    let mut pick = use_signal(String::new);
    let all = rows.read().clone().unwrap_or_default();
    let mut projects: Vec<String> = all.iter().map(|r| s(r, "project")).collect();
    projects.sort();
    projects.dedup();
    let project = if pick().is_empty() { projects.first().cloned().unwrap_or_default() } else { pick() };
    let mut days: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for r in all.iter().filter(|r| s(r, "project") == project) {
        days.entry(s(r, "date")).or_default().push(r.clone());
    }
    let people: HashSet<String> = days.values().flatten().map(|r| s(r, "aNumber")).collect();
    rsx! {
        if projects.is_empty() {
            div { class: "card", EmptyState { icon: "archive", title: "No project history yet", body: "Daily snapshots record who works on which project. History builds up each day." } }
        } else {
            div { class: "flex flex-wrap items-end gap-3 no-print",
                div {
                    label { class: "label", r#for: "ph-project", "Project" }
                    select { id: "ph-project", class: "field", "data-testid": "ph-project", onchange: move |e| pick.set(e.value()),
                        for p in projects.iter() { option { key: "{p}", value: "{p}", selected: *p == project, "{p}" } }
                    }
                }
                button { class: "btn-secondary", "data-testid": "ph-print", onclick: { let project = project.clone(); move |_| s5print("analytics", json!([{ "kind": "project", "project": project, "days": 90 }])) }, Icon { name: "archive" } "Print Investigation Dossier" }
            }
            p { class: "font-semibold", "{project}" span { class: "text-muted font-normal", " · {days.len()} days · {util::plural(people.len(), \"person\")} in all" } }
            Table { head: vec!["Date", "People", "Present", "Who"],
                rows: days.into_iter().rev().map(|(d, list)| {
                    let present = list.iter().filter(|r| s(r, "status").eq_ignore_ascii_case("present")).count();
                    (d.clone(), vec![d, list.len().to_string(), present.to_string(), list.iter().map(|r| s(r, "name")).collect::<Vec<_>>().join(", ")])
                }).collect() }
        }
    }
}

/// One person's full record on one page (Service 5's investigation dossier):
/// attendance and assignment history, duty nights, overtime and visits.
#[component]
pub fn Dossier() -> Element {
    let workers = use_workers();
    let att = use_records("attendance_log");
    let asg = use_records("assignment_log");
    let duty = use_records("duty_schedules");
    let ot = use_records("ot_entries");
    let visits = use_records("visitations");
    let mut who = use_signal(String::new);
    let people = active(&workers);
    let Some(w) = people.iter().find(|w| w.a_number == who()).cloned() else {
        return rsx! {
            div { class: "max-w-md",
                label { class: "label", r#for: "dos-who", "Person" }
                select { id: "dos-who", class: "field", "data-testid": "dossier-who", onchange: move |e| who.set(e.value()),
                    option { value: "", "Choose a person…" }
                    for w in people.iter().filter(|w| !w.a_number.is_empty()) { option { key: "{w.id}", value: "{w.a_number}", "{w.name} · A# {w.a_number}" } }
                }
            }
        };
    };
    let a = w.a_number.clone();
    let mine = |list: &Resource<Vec<Value>>| -> Vec<Value> {
        let mut v: Vec<Value> = list.read().clone().unwrap_or_default().into_iter().filter(|e| s(e, "aNumber") == a).collect();
        v.sort_by(|x, y| s(y, "date").cmp(&s(x, "date")));
        v
    };
    let att_rows: Vec<(String, Vec<String>)> = mine(&att).iter().enumerate().map(|(i, e)| (format!("a{i}"), vec![s(e, "date"), format!("{} → {}", s(e, "oldStatus"), s(e, "newStatus")), s(e, "reason")])).collect();
    let asg_rows: Vec<(String, Vec<String>)> =
        mine(&asg).iter().enumerate().map(|(i, e)| (format!("b{i}"), vec![s(e, "date"), s(e, "field"), format!("{} → {}", s(e, "oldValue"), s(e, "newValue")), s(e, "reason")])).collect();
    let nights: Vec<String> = duty
        .read()
        .clone()
        .unwrap_or_default()
        .iter()
        .filter(|d| d["assignments"].as_array().is_some_and(|x| x.iter().any(|a2| s(a2, "aNumber") == a)))
        .map(|d| s(d, "dateStr"))
        .collect();
    let ot_rows: Vec<(String, Vec<String>)> = ot
        .read()
        .clone()
        .unwrap_or_default()
        .iter()
        .filter(|e| s(e, "volunteerId") == a || e["volunteers"].as_array().is_some_and(|v| v.iter().any(|x| s(x, "aNumber") == a)))
        .map(|e| (s(e, "id"), vec![s(e, "date"), s(e, "title"), s(e, "hours")]))
        .collect();
    let visit_rows: Vec<(String, Vec<String>)> = visits
        .read()
        .clone()
        .unwrap_or_default()
        .iter()
        .filter(|v| s(v, "aNumber") == a || s(v, "workerId") == a)
        .map(|v| (s(v, "id"), vec![s(v, "date"), s(v, "ministerName"), s(v, "status")]))
        .collect();
    let absent = att_rows.iter().filter(|r| r.1[1].ends_with("Absent")).count();
    rsx! {
        div { class: "flex flex-wrap items-end gap-3 no-print",
            div { class: "max-w-md flex-1",
                label { class: "label", r#for: "dos-who", "Person" }
                select { id: "dos-who", class: "field", "data-testid": "dossier-who", onchange: move |e| who.set(e.value()),
                    for p in people.iter().filter(|w| !w.a_number.is_empty()) { option { key: "{p.id}", value: "{p.a_number}", selected: p.a_number == a, "{p.name} · A# {p.a_number}" } }
                }
            }
            button { class: "btn-secondary", "data-testid": "dossier-print", onclick: { let name = w.name.clone(); move |_| s5print("analytics", json!([{ "worker": name, "days": 90 }])) }, Icon { name: "archive" } "Print dossier" }
        }
        div { class: "space-y-4", "data-testid": "dossier",
            div { class: "card p-4",
                p { class: "text-lg font-semibold", "{w.name}" span { class: "text-muted font-normal text-sm ml-2", "A# {w.a_number}" } }
                p { class: "text-sm text-muted", {[w.trade.as_str(), w.department.as_str(), w.project_group.as_str(), w.area.as_str()].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" · ")} }
                p { class: "text-sm", "Today: {w.status.label()}" if !w.task.is_empty() { " · {w.task}" } }
                p { class: "text-sm text-muted tabular", "{absent} times marked absent · {nights.len()} duty nights · {ot_rows.len()} overtime entries · {visit_rows.len()} visits" }
            }
            p { class: "font-semibold", "Attendance changes" }
            Table { head: vec!["Date", "Change", "Reason"], rows: att_rows, empty: "No attendance changes." }
            p { class: "font-semibold", "Assignment changes" }
            Table { head: vec!["Date", "What", "Change", "Reason"], rows: asg_rows, empty: "No assignment changes." }
            p { class: "font-semibold", "Overtime" }
            Table { head: vec!["Date", "Work", "Hours"], rows: ot_rows, empty: "No overtime." }
            p { class: "font-semibold", "Visits" }
            Table { head: vec!["Date", "Minister", "Status"], rows: visit_rows, empty: "No visits." }
            if !nights.is_empty() { p { class: "text-sm", span { class: "font-semibold", "Duty nights: " } {nights.join(", ")} } }
        }
    }
}

// --------------------------------------------------------------- visitations

fn in_timeframe(date: &str, frame: &str) -> bool {
    let (Some(d), now) = (date_of(date), Local::now().date_naive()) else { return frame == "All dates" };
    match frame {
        "This week" => d >= now - Duration::days(now.weekday().num_days_from_monday() as i64) && d < now + Duration::days(7 - now.weekday().num_days_from_monday() as i64),
        "Next 14 days" => d >= now && d <= now + Duration::days(14),
        "This month" => d.year() == now.year() && d.month() == now.month(),
        _ => true,
    }
}

/// Service 5's visitations: the schedule, the history, and coverage (who
/// hasn't been visited, by housing). Visits are marked done or cancelled here.
#[component]
pub fn VisitationsHub() -> Element {
    let app = use_app();
    let visits = use_records("visitations");
    let ministers = use_records("ministers");
    let workers = use_workers();
    let view = use_signal(|| "Schedule");
    let mut editing = use_signal(|| None::<(String, Value)>);
    let mut q = use_signal(String::new);
    let mut minister = use_signal(String::new);
    let mut frame = use_signal(|| "All dates".to_string());
    let mut status = use_signal(String::new);
    let people = active(&workers);
    let map = by_a_number(&people);
    let can_edit = app.can(Permission::ManageVisitations);
    let mut list = visits.read().clone().unwrap_or_default();
    list.sort_by(|a, b| s(a, "date").cmp(&s(b, "date")));
    let names: Vec<String> = ministers.read().clone().unwrap_or_default().iter().map(|m| s(m, "name")).filter(|n| !n.is_empty()).collect();
    let who = |v: &Value| if s(v, "workerName").is_empty() { name_of(&map, &s(v, "aNumber")) } else { s(v, "workerName") };
    let ql = q().to_lowercase();
    let matches = |v: &Value| (ql.is_empty() || [who(v), s(v, "ministerName"), s(v, "notes")].iter().any(|x| x.to_lowercase().contains(&ql))) && (minister().is_empty() || s(v, "ministerName") == minister());
    let scheduled: Vec<Value> = list.iter().filter(|v| s(v, "status") == "scheduled" && matches(v) && in_timeframe(&s(v, "date"), &frame())).cloned().collect();
    let mut history: Vec<Value> = list.iter().filter(|v| s(v, "status") != "scheduled" && matches(v) && (status().is_empty() || s(v, "status").starts_with(&status()))).cloned().collect();
    history.reverse();
    // Coverage: each person's last completed visit; never-visited people by housing.
    let mut last: HashMap<String, String> = HashMap::new();
    for v in list.iter().filter(|v| s(v, "status") == "completed") {
        let a = if s(v, "aNumber").is_empty() { s(v, "workerId") } else { s(v, "aNumber") };
        let e = last.entry(a).or_default();
        if s(v, "date") > *e {
            *e = s(v, "date");
        }
    }
    let mut cover: Vec<(String, Worker)> = people.iter().map(|w| (last.get(&w.a_number).cloned().unwrap_or_default(), w.clone())).collect();
    cover.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.name.cmp(&b.1.name)));
    let mut unvisited: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (d, w) in cover.iter().filter(|c| c.0.is_empty()) {
        let _ = d;
        unvisited.entry(if w.housing.is_empty() { "No housing group".into() } else { w.housing.clone() }).or_default().push(w.name.clone());
    }
    let visited = cover.iter().filter(|c| !c.0.is_empty()).count();
    let set_status = move |v: Value, to: &'static str| async move {
        let mut next = v.clone();
        next["status"] = json!(to);
        let now = chrono::Utc::now().to_rfc3339();
        next["completedAt"] = json!(now);
        let mut details = next.get("completionDetails").filter(|d| d.is_object()).cloned().unwrap_or_else(|| json!({}));
        details["completedAt"] = json!(now);
        next["completionDetails"] = details;
        match save_record(app, "visitations", &s(&v, "id"), &next).await {
            Ok(_) => app.success(if to == "completed" { "Visit marked done" } else { "Visit cancelled" }),
            Err(e) => app.error(e),
        }
    };
    let fields = vec![
        f("date", "Date", Kind::Date),
        f("aNumber", "Person visited", Kind::Worker),
        f("ministerName", "Minister", if names.is_empty() { Kind::Text } else { Kind::Choice(names.clone()) }),
        f("status", "Status", choice(&["scheduled", "completed", "canceled"])),
        f("completionDetails.members", "Members present", Kind::Text),
        f("completionDetails.concerns", "Issues or concerns raised", Kind::Long),
        f("completionDetails.cancelReason", "Reason for cancelation", Kind::Text),
        f("completionDetails.photoDataUrl", "Photo evidence", Kind::Photo),
        f("notes", "Notes / purpose", Kind::Long),
    ];
    let print_rows = |rows: &[Value]| -> Vec<(String, Vec<String>)> { rows.iter().map(|v| (s(v, "id"), vec![s(v, "date"), who(v), s(v, "ministerName"), s(v, "status"), s(v, "notes")])).collect() };
    // Service 5's visit sheets: "scheduled", "week", or one visit's id.
    let mut printing = use_signal(String::new);
    rsx! {
        div { class: "flex flex-wrap items-center gap-3 no-print",
            Segmented { options: vec!["Schedule", "History", "Coverage", "Groups"], current: view, testid: "visit-view" }
            if can_edit {
                button { class: "btn-primary ml-auto", "data-testid": "visit-add", onclick: move |_| editing.set(Some((new_id("visit"), json!({ "date": today(), "status": "scheduled", "createdAt": chrono::Utc::now().to_rfc3339() })))), Icon { name: "plus" } "Schedule a visit" }
                button { class: "btn-secondary", "data-testid": "visit-past", onclick: move |_| editing.set(Some((new_id("visit"), json!({ "date": today(), "status": "completed", "completedAt": chrono::Utc::now().to_rfc3339(), "createdAt": chrono::Utc::now().to_rfc3339() })))), "Record a past visit" }
            }
            button { class: "btn-secondary", "data-testid": "visit-print-scheduled", onclick: move |_| printing.set("scheduled".into()), Icon { name: "archive" } "Print scheduled" }
            button { class: "btn-secondary", "data-testid": "visit-print-week", onclick: move |_| printing.set("week".into()), Icon { name: "archive" } "Print last 7 days" }
        }
        if view() == "Groups" {
            crate::views::workforce_more::VisitGroupsTab {}
            VisitGroupMembers {}
        }
        if view() == "Schedule" || view() == "History" {
            div { class: "flex flex-wrap gap-2 no-print",
                input { class: "field w-auto min-w-[14rem]", "aria-label": "Search visits", "data-testid": "visit-search", placeholder: "Search person, minister, notes", value: "{q}", oninput: move |e| q.set(e.value()) }
                select { class: "field w-auto", "aria-label": "Minister", "data-testid": "visit-minister", onchange: move |e| minister.set(e.value()),
                    option { value: "", "All ministers" }
                    for n in names.iter() { option { key: "{n}", value: "{n}", "{n}" } }
                }
                if view() == "Schedule" {
                    select { class: "field w-auto", "aria-label": "When", "data-testid": "visit-frame", onchange: move |e| frame.set(e.value()),
                        for o in ["All dates", "This week", "Next 14 days", "This month"] { option { key: "{o}", value: "{o}", "{o}" } }
                    }
                } else {
                    select { class: "field w-auto", "aria-label": "Status", "data-testid": "visit-status", onchange: move |e| status.set(e.value()),
                        option { value: "", "All statuses" } option { value: "completed", "Completed" } option { value: "cancel", "Cancelled" }
                    }
                }
            }
        }
        match view() {
            "Schedule" => rsx! {
                p { class: "text-sm text-muted no-print", "{scheduled.len()} scheduled" }
                if scheduled.is_empty() { div { class: "card no-print", EmptyState { icon: "calendar", title: "No visits scheduled", body: "Schedule one, or change the filters." } } }
                ul { class: "card divide-y divide-line no-print", "data-testid": "visit-schedule",
                    for v in scheduled.iter().cloned() {
                        li { key: "{s(&v, \"id\")}", class: "flex flex-wrap items-center gap-3 px-4 py-2.5", "data-testid": "visit-row",
                            span { class: "tabular text-sm w-24", {s(&v, "date")} }
                            span { class: "flex-1 min-w-[10rem]", {who(&v)} span { class: "text-muted text-sm ml-2", {s(&v, "ministerName")} } }
                            if can_edit {
                                button { class: "btn-secondary h-8", "data-testid": "visit-done", onclick: { let v = v.clone(); move |_| set_status(v.clone(), "completed") }, Icon { name: "check" } "Done" }
                                button { class: "btn-ghost h-8", "data-testid": "visit-cancel", onclick: { let v = v.clone(); move |_| set_status(v.clone(), "canceled") }, "Cancel" }
                                button { class: "btn-ghost h-8", "data-testid": "visit-print-form", onclick: { let id = s(&v, "id"); move |_| printing.set(id.clone()) }, Icon { name: "archive" } "Print form" }
                                button { class: "btn-icon btn-ghost", "aria-label": "Edit visit", onclick: { let v = v.clone(); move |_| editing.set(Some((s(&v, "id"), v.clone()))) }, Icon { name: "edit" } }
                            }
                        }
                    }
                }
            },
            "History" => rsx! {
                div { class: "no-print",
                    Table { head: vec!["Date", "Person", "Minister", "Status", "Notes"], rows: print_rows(&history), empty: "No past visits.",
                        on_edit: move |id: String| { if can_edit { if let Some(v) = list.iter().find(|e| s(e, "id") == id) { editing.set(Some((id, v.clone()))) } } } }
                }
            },
            "Groups" => rsx! {},
            _ => rsx! {
                div { class: "no-print space-y-4", "data-testid": "visit-coverage",
                    p { class: "text-sm", "{visited} of {people.len()} people have had a visit." }
                    div { class: "grid lg:grid-cols-2 gap-4",
                        section { class: "space-y-2",
                            p { class: "font-semibold", "Last visit, oldest first" }
                            Table { head: vec!["Person", "Housing", "Last visit"], rows: cover.iter().map(|(d, w)| (w.id.to_string(), vec![w.name.clone(), w.housing.clone(), if d.is_empty() { "Never".to_string() } else { d.clone() }])).collect() }
                        }
                        section { class: "space-y-2", "data-testid": "visit-unvisited",
                            p { class: "font-semibold", "Not visited yet, by housing" }
                            if unvisited.is_empty() { p { class: "text-sm text-muted", "Everyone has had a visit." } }
                            for (h, names) in unvisited { div { key: "{h}", class: "card p-3", p { class: "text-sm font-medium", "{h} · {names.len()}" } p { class: "text-sm text-muted", {names.join(", ")} } } }
                        }
                    }
                }
            },
        }
        if !printing().is_empty() {
            Modal { title: "Visitation print view".to_string(), wide: true, on_close: move |_| printing.set(String::new()),
                {
                    let kind = printing();
                    let args = match kind.as_str() { "scheduled" | "week" => json!([{ "kind": kind }]), id => json!([{ "kind": "one", "id": id }]) };
                    rsx! { S5Preview { report: "visitSheets", args } }
                }
            }
        }
        if let Some((id, v)) = editing() {
            RecordModal { title: "Visitation".to_string(), list: "visitations", id, fields, initial: v, on_close: move |_| editing.set(None),
                on_saved: move |saved: Value| {
                    let mut next = saved.clone();
                    if let Some(w) = map.get(&s(&saved, "aNumber")) { next["workerName"] = json!(w.name); next["workerId"] = json!(w.a_number); }
                    if s(&saved, "status") == "completed" && s(&saved, "completedAt").is_empty() { next["completedAt"] = json!(s(&saved, "date")); }
                    if s(&saved, "status") == "scheduled" { next["completedAt"] = Value::Null; next["completionDetails"] = Value::Null; }
                    let id = s(&saved, "id");
                    spawn(async move { let _ = save_record(app, "visitations", &id, &next).await; });
                }
            }
        }
    }
}

/// A visitation group's members (Service 5 keeps them on each person:
/// `visitationGroup` / `visitationRole`), its roster and monthly sheet.
#[component]
pub fn VisitGroupMembers() -> Element {
    let app = use_app();
    let groups = use_records("visitation_groups");
    let workers = use_workers();
    let mut current = use_signal(String::new);
    let mut adding = use_signal(HashSet::<uuid::Uuid>::new);
    let mut month = use_signal(|| Local::now().format("%Y-%m").to_string());
    // Service 5's add-member dialog: who to list, and filters.
    let mut pool = use_signal(|| "unassigned");
    let mut pool_q = use_signal(String::new);
    let mut pool_trade = use_signal(String::new);
    let mut pool_housing = use_signal(String::new);
    // Monthly sheet options.
    let mut paper = use_signal(|| "folio".to_string());
    let mut officers = use_signal(|| true);
    let glist = groups.read().clone().unwrap_or_default();
    let names: Vec<String> = glist.iter().map(|g| s(g, "name")).filter(|n| !n.is_empty()).collect();
    let people = active(&workers);
    let g = if current().is_empty() { names.first().cloned().unwrap_or_default() } else { current() };
    let ginfo = glist.iter().find(|x| s(x, "name") == g).cloned().unwrap_or(json!({}));
    let rank = |r: &str| match r { "Katiwala" => 0, "II Katiwala" => 1, "Kalihim" => 2, _ => 9 };
    let mut members: Vec<Worker> = people.iter().filter(|w| w.visit_group == g).cloned().collect();
    members.sort_by_key(|w| (rank(&w.visit_role), w.name.to_lowercase()));
    let ql = pool_q().to_lowercase();
    let unassigned: Vec<Worker> = people
        .iter()
        .filter(|w| match pool() {
            "same" => w.visit_group != g && !s(&ginfo, "housing").is_empty() && w.housing == s(&ginfo, "housing"),
            "other" => !w.visit_group.is_empty() && w.visit_group != g,
            _ => w.visit_group.is_empty(),
        })
        .filter(|w| ql.is_empty() || w.name.to_lowercase().contains(&ql) || w.a_number.contains(&ql))
        .filter(|w| pool_trade().is_empty() || w.trade == pool_trade())
        .filter(|w| pool_housing().is_empty() || w.housing == pool_housing())
        .cloned()
        .collect();
    let distinct = |f: fn(&Worker) -> String| {
        let mut v: Vec<String> = people.iter().map(f).filter(|x| !x.is_empty()).collect();
        v.sort();
        v.dedup();
        v
    };
    let trades = distinct(|w| w.trade.clone());
    let housings = distinct(|w| w.housing.clone());
    // Monthly sheet rows for a group (officers first, or left out).
    let sheet_rows = {
        let people = people.clone();
        move |group: &str| -> Vec<Worker> {
            let mut v: Vec<Worker> = people.iter().filter(|w| w.visit_group == group && (officers() || rank(&w.visit_role) == 9)).cloned().collect();
            v.sort_by_key(|w| (rank(&w.visit_role), w.name.to_lowercase()));
            v
        }
    };
    let set = move |w: Worker, group: String, role: String| async move {
        match api::put::<_, Worker>(&format!("/workers/{}/profile", w.id), &json!({ "visitationGroup": group, "visitationRole": role })).await {
            Ok(_) => *WORKERS_CHANGED.write() += 1,
            Err(e) => app.error(e.message()),
        }
    };
    let assign = {
        let g = g.clone();
        let all = people.clone();
        move |_| {
            let g = g.clone();
            let chosen: Vec<Worker> = all.iter().filter(|w| adding().contains(&w.id)).cloned().collect();
            async move {
                for w in chosen {
                    if let Err(e) = api::put::<_, Worker>(&format!("/workers/{}/profile", w.id), &json!({ "visitationGroup": g, "visitationRole": "Member" })).await {
                        return app.error(e.message());
                    }
                }
                adding.write().clear();
                *WORKERS_CHANGED.write() += 1;
                app.success("Members added");
            }
        }
    };
    let (y, m) = month().split_once('-').and_then(|(y, m)| Some((y.parse::<i32>().ok()?, m.parse::<u32>().ok()?))).unwrap_or((Local::now().year(), Local::now().month()));
    let days = NaiveDate::from_ymd_opt(if m == 12 { y + 1 } else { y }, if m == 12 { 1 } else { m + 1 }, 1).zip(NaiveDate::from_ymd_opt(y, m, 1)).map(|(a, b)| (a - b).num_days()).unwrap_or(30);
    if names.is_empty() {
        return rsx! {};
    }
    rsx! {
        section { class: "space-y-3 pt-2", "data-testid": "vgroup-members",
            div { class: "flex flex-wrap items-end gap-3 no-print",
                div {
                    label { class: "label", r#for: "vg-pick", "Group" }
                    select { id: "vg-pick", class: "field", "data-testid": "vgroup-pick", onchange: move |e| current.set(e.value()),
                        for n in names.iter() { option { key: "{n}", value: "{n}", selected: *n == g, "{n}" } }
                    }
                }
                div {
                    label { class: "label", r#for: "vg-month", "Month" }
                    input { id: "vg-month", r#type: "month", class: "field", value: "{month}", oninput: move |e| month.set(e.value()) }
                }
                button { class: "btn-secondary", "data-testid": "vgroup-print-roster", onclick: { let g = g.clone(); move |_| s5print("visitationRoster", json!([g])) }, Icon { name: "archive" } "Print roster" }
                div {
                    label { class: "label", r#for: "vg-paper", "Paper size" }
                    select { id: "vg-paper", class: "field", "data-testid": "vgroup-paper", onchange: move |e| paper.set(e.value()),
                        for (v, l) in [("folio", "Folio / Long (8.5 x 13 in)"), ("a4", "A4 (8.27 x 11.69 in)"), ("letter", "Letter (8.5 x 11 in)")] { option { key: "{v}", value: "{v}", selected: paper() == v, "{l}" } }
                    }
                }
                label { class: "flex items-center gap-2 text-sm pb-2",
                    input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "vgroup-officers", checked: officers(), onchange: move |e| officers.set(e.checked()) }
                    "Include officers in rows"
                }
                button { class: "btn-secondary", "data-testid": "vgroup-print-month", onclick: { let g = g.clone(); move |_| s5print("monthlySheet", json!([{ "group": g, "year": y, "month": m - 1, "paperSize": paper(), "officers": officers() }])) }, Icon { name: "archive" } "Print active sheet" }
                button { class: "btn-secondary", "data-testid": "vgroup-print-all", onclick: move |_| s5print("monthlySheet", json!([{ "all": true, "year": y, "month": m - 1, "paperSize": paper(), "officers": officers() }])), Icon { name: "archive" } "Batch print all groups" }
                button { class: "btn-secondary", "data-testid": "vgroup-csv",
                    onclick: { let rows = sheet_rows.clone(); let g = g.clone(); move |_| {
                        let mut out = format!("Name,Role,Housing{}\n", (1..=days).map(|d| format!(",{d}")).collect::<String>());
                        for w in rows(&g) {
                            out += &format!("{},{},{}{}\n", crate::views::personnel::csv_cell(&w.name), if w.visit_role.is_empty() { "Member" } else { &w.visit_role }, crate::views::personnel::csv_cell(&w.housing), ",".repeat(days as usize));
                        }
                        crate::views::personnel::save_file(&format!("{}-{}.csv", g.replace(' ', "_"), month()), &out);
                    } }, "Export CSV" }
            }
            p { class: "font-semibold no-print", "{g}" span { class: "text-muted font-normal", " · {util::plural(members.len(), \"member\")} · {s(&ginfo, \"housing\")}" } }
            ul { class: "card divide-y divide-line no-print",
                if members.is_empty() { li { class: "px-4 py-3 text-sm text-muted", "No members yet. Add people below." } }
                for w in members.iter().cloned() {
                    li { key: "{w.id}", class: "flex flex-wrap items-center gap-3 px-4 py-2", "data-testid": "vgroup-member",
                        span { class: "flex-1", "{w.name}" span { class: "text-muted text-sm ml-2", "{w.housing}" } }
                        select { class: "field w-auto", "aria-label": "Role of {w.name}", "data-testid": "vgroup-role",
                            onchange: { let w = w.clone(); let g = g.clone(); move |e: FormEvent| set(w.clone(), g.clone(), e.value()) },
                            for r in ["Katiwala", "II Katiwala", "Kalihim", "Member"] { option { key: "{r}", value: "{r}", selected: w.visit_role == r || (r == "Member" && w.visit_role.is_empty()), "{r}" } }
                        }
                        select { class: "field w-auto h-8", "aria-label": "Move {w.name} to another group", "data-testid": "vgroup-transfer",
                            onchange: { let w = w.clone(); move |e: FormEvent| { if e.value() != "__stay" { spawn(set(w.clone(), e.value(), if e.value().is_empty() { String::new() } else { "Member".into() })); } } },
                            option { value: "__stay", selected: true, "Move to…" }
                            option { value: "", "-- Unassigned (remove from group) --" }
                            for n in names.iter().filter(|n| **n != g) { option { key: "{n}", value: "{n}", "{n}" } }
                        }
                        button { class: "btn-ghost h-8", "data-testid": "vgroup-remove", onclick: { let w = w.clone(); move |_| set(w.clone(), String::new(), String::new()) }, "Remove" }
                    }
                }
            }
            div {
                details { class: "card p-3 no-print",
                    summary { class: "text-sm font-medium cursor-pointer", "Add members ({unassigned.len()})" }
                    div { class: "flex flex-wrap gap-2 pt-2",
                        for (k, l) in [("unassigned", "Unassigned only"), ("same", "Same housing"), ("other", "Other groups")] {
                            button { key: "{k}", class: if pool() == k { "btn-primary h-8" } else { "btn-secondary h-8" }, "data-testid": "vgroup-pool-{k}", onclick: move |_| pool.set(k), "{l}" }
                        }
                        input { class: "field h-8 w-40", r#type: "search", placeholder: "Search…", "data-testid": "vgroup-pool-search", value: "{pool_q}", oninput: move |e| pool_q.set(e.value()) }
                        select { class: "field h-8 w-auto", "aria-label": "Trade", "data-testid": "vgroup-pool-trade", onchange: move |e| pool_trade.set(e.value()),
                            option { value: "", "All trades / skills" }
                            for x in trades.iter() { option { key: "{x}", value: "{x}", "{x}" } }
                        }
                        select { class: "field h-8 w-auto", "aria-label": "Housing", "data-testid": "vgroup-pool-housing", onchange: move |e| pool_housing.set(e.value()),
                            option { value: "", "All residences / housing" }
                            for x in housings.iter() { option { key: "{x}", value: "{x}", "{x}" } }
                        }
                        button { class: "btn-ghost h-8", "data-testid": "vgroup-select-filtered", onclick: { let ids: Vec<uuid::Uuid> = unassigned.iter().map(|w| w.id).collect(); move |_| adding.write().extend(ids.iter().copied()) }, "Select all filtered" }
                    }
                    if unassigned.is_empty() { p { class: "text-sm text-muted pt-2", "No one matches." } }
                    div { class: "max-h-64 overflow-y-auto grid sm:grid-cols-2 gap-1 pt-2",
                        for w in unassigned.iter().cloned() {
                            label { key: "{w.id}", class: "flex items-center gap-2 text-sm py-1",
                                input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "vgroup-unassigned", checked: adding().contains(&w.id),
                                    onchange: move |e| { if e.checked() { adding.write().insert(w.id); } else { adding.write().remove(&w.id); } } }
                                span { class: "flex-1 truncate", "{w.name}" }
                                span { class: "text-xs text-muted truncate", "{w.housing}" }
                            }
                        }
                    }
                    button { class: "btn-primary mt-2", "data-testid": "vgroup-assign", disabled: adding().is_empty(), onclick: assign, "Add selected ({adding().len()}) to {g}" }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ overtime

/// One overtime entry's share per person: (A#, hours, pay, area). Service 5 has
/// single entries (one person) and group sessions (a list of volunteers).
pub fn ot_shares(e: &Value) -> Vec<(String, f64, f64, String)> {
    let hours = e["hours"].as_f64().unwrap_or(0.0);
    let pay = e["computedPay"].as_f64().unwrap_or(0.0);
    let area = [s(e, "otArea"), s(e, "title")].into_iter().find(|x| !x.is_empty()).unwrap_or_default();
    if !s(e, "volunteerId").is_empty() {
        return vec![(s(e, "volunteerId"), hours, pay, area)];
    }
    let vols = e["volunteers"].as_array().cloned().unwrap_or_default();
    let n = vols.len().max(1) as f64;
    vols.iter().map(|v| (s(v, "aNumber"), hours, pay / n, if s(v, "area").is_empty() { area.clone() } else { s(v, "area") })).collect()
}

/// Service 5's OT calculator over a date range: per volunteer, by area, by day,
/// and the volunteer × day matrix.
#[component]
pub fn OtSummary() -> Element {
    let ot = use_records("ot_entries");
    let workers = use_workers();
    let map = by_a_number(&workers.read().clone().unwrap_or_default());
    let mut from = use_signal(|| Local::now().format("%Y-%m-01").to_string());
    let mut to = use_signal(today);
    let view = use_signal(|| "Per volunteer");
    let app = use_app();
    // Service 5 "Load from Yesterday": yesterday's people, area and task copied to today
    // with hours reset to 0, and the range set to today.
    let load_yesterday = move |_| async move {
        let yesterday = (Local::now().date_naive() - Duration::days(1)).format("%Y-%m-%d").to_string();
        let from_list: Vec<Value> = ot.read().clone().unwrap_or_default().into_iter().filter(|e| s(e, "date") == yesterday).collect();
        if from_list.is_empty() {
            return app.error(format!("No entries from {yesterday} to copy."));
        }
        for e in &from_list {
            let mut next = e.clone();
            next["id"] = json!(new_id("ot"));
            next["date"] = json!(today());
            next["hours"] = json!(0);
            next["computedPay"] = json!(0);
            next["status"] = json!("Planned");
            next["createdAt"] = json!(chrono::Utc::now().to_rfc3339());
            if let Err(err) = save_record(app, "ot_entries", &s(&next, "id"), &next).await {
                return app.error(err);
            }
        }
        from.set(today());
        to.set(today());
        app.success(format!("Loaded {} entries from yesterday ({yesterday}) to today. Hours reset to 0.", from_list.len()));
    };
    let entries: Vec<Value> = ot.read().clone().unwrap_or_default().into_iter().filter(|e| s(e, "date") >= from() && s(e, "date") <= to()).collect();
    let mut per: BTreeMap<String, (usize, f64, f64)> = BTreeMap::new();
    let mut area: BTreeMap<String, (HashSet<String>, f64, f64)> = BTreeMap::new();
    let mut daily: BTreeMap<String, (usize, f64, f64)> = BTreeMap::new();
    let mut matrix: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    for e in &entries {
        let d = s(e, "date");
        let shares = ot_shares(e);
        let day = daily.entry(d.clone()).or_default();
        day.0 += 1;
        for (a, h, p, ar) in shares {
            let x = per.entry(a.clone()).or_default();
            x.0 += 1;
            x.1 += h;
            x.2 += p;
            let y = area.entry(if ar.is_empty() { "—".into() } else { ar }).or_default();
            y.0.insert(a.clone());
            y.1 += h;
            y.2 += p;
            day.1 += h;
            day.2 += p;
            *matrix.entry(a).or_default().entry(d.clone()).or_default() += h;
        }
    }
    let total: f64 = per.values().map(|x| x.2).sum();
    let dates: Vec<String> = daily.keys().cloned().collect();
    let money = |p: f64| format!("₱{p:.2}");
    rsx! {
        div { class: "flex flex-wrap items-end gap-3 no-print",
            div { label { class: "label", r#for: "ots-from", "From" } input { id: "ots-from", r#type: "date", class: "field", "data-testid": "ots-from", value: "{from}", oninput: move |e| from.set(e.value()) } }
            div { label { class: "label", r#for: "ots-to", "To" } input { id: "ots-to", r#type: "date", class: "field", "data-testid": "ots-to", value: "{to}", oninput: move |e| to.set(e.value()) } }
            Segmented { options: vec!["Per volunteer", "By area", "Daily", "Matrix"], current: view, testid: "ots-view" }
            if app.can(Permission::ManageOvertime) { button { class: "btn-secondary", "data-testid": "ots-load-yesterday", onclick: load_yesterday, "Load from yesterday" } }
            // Service 5's three OT prints: schedule by area, timesheet matrix, daily breakdown.
            button { class: "btn-primary", "data-testid": "ots-print", title: "Print Overtime Schedule (By Area)", onclick: move |_| s5print("overtime", json!([{ "kind": "schedule", "start": from(), "end": to() }])), Icon { name: "archive" } "OT Schedule" }
            button { class: "btn-secondary", "data-testid": "ots-print-matrix", title: "Timesheet Matrix (per volunteer)", onclick: move |_| s5print("overtime", json!([{ "kind": "matrix", "start": from(), "end": to() }])), "Matrix" }
            button { class: "btn-secondary", "data-testid": "ots-print-daily", title: "Daily Breakdown (by Date)", onclick: move |_| s5print("overtime", json!([{ "kind": "daily", "start": from(), "end": to() }])), "Daily Breakdown" }
        }
        p { class: "text-sm tabular", "data-testid": "ots-total", "{entries.len()} entries · {util::plural(per.len(), \"person\")} · total {money(total)}" }
        div { class: "print-sheet",
            p { class: "font-semibold print-only", "Overtime {view()} — {from()} to {to()}" }
            match view() {
                "By area" => rsx! { Table { head: vec!["Area / work", "People", "Hours", "Pay"], rows: area.into_iter().map(|(a, (p, h, m))| (a.clone(), vec![a, p.len().to_string(), format!("{h:.1}"), money(m)])).collect() } },
                "Daily" => rsx! { Table { head: vec!["Date", "Entries", "Hours", "Pay"], rows: daily.into_iter().map(|(d, (n, h, m))| (d.clone(), vec![d, n.to_string(), format!("{h:.1}"), money(m)])).collect() } },
                "Matrix" => rsx! {
                    div { class: "card overflow-x-auto",
                        table { class: "text-xs tabular", "data-testid": "ots-matrix",
                            thead { tr { th { class: "px-2 py-1 text-left", "Person" } for d in dates.iter() { th { key: "{d}", class: "px-1 py-1", {d.get(5..).unwrap_or_default().to_string()} } } th { class: "px-2", "Total" } } }
                            tbody {
                                for (a, row) in matrix.iter() {
                                    tr { key: "{a}", class: "border-t border-line",
                                        td { class: "px-2 py-1 whitespace-nowrap", {name_of(&map, a)} }
                                        for d in dates.iter() { td { key: "{d}", class: "px-1 text-center", {row.get(d).map(|h| format!("{h:.1}")).unwrap_or_default()} } }
                                        td { class: "px-2 font-medium", {format!("{:.1}", row.values().sum::<f64>())} }
                                    }
                                }
                            }
                        }
                    }
                },
                _ => rsx! { Table { head: vec!["Person", "Entries", "Hours", "Pay"], rows: per.iter().map(|(a, (n, h, m))| (a.clone(), vec![name_of(&map, a), n.to_string(), format!("{h:.1}"), money(*m)])).collect() } },
            }
        }
    }
}

// ---------------------------------------------------------------------- duty

/// The duty roster for a whole month (dates × duty areas), printable.
#[component]
pub fn DutyMonth() -> Element {
    let schedules = use_records("duty_schedules");
    let areas = use_records("duty_areas");
    let workers = use_workers();
    let mut month = use_signal(|| Local::now().format("%Y-%m").to_string());
    let map = by_a_number(&workers.read().clone().unwrap_or_default());
    let area_list = areas.read().clone().unwrap_or_default();
    let days: Vec<Value> = {
        let mut d: Vec<Value> = schedules.read().clone().unwrap_or_default().into_iter().filter(|d| s(d, "dateStr").starts_with(&month())).collect();
        d.sort_by_key(|x| s(x, "dateStr"));
        d
    };
    rsx! {
        div { class: "flex flex-wrap items-end gap-3 no-print",
            div { label { class: "label", r#for: "dm-month", "Month" } input { id: "dm-month", r#type: "month", class: "field", "data-testid": "duty-month", value: "{month}", oninput: move |e| month.set(e.value()) } }
        }
        div { class: "card overflow-x-auto print-sheet", "data-testid": "duty-month-table",
            p { class: "font-semibold px-3 pt-3 print-only", "Overnight duty — {month}" }
            table { class: "text-sm w-full",
                thead { tr { class: "text-left border-b border-line", th { class: "px-3 py-2", "Date" } for a in area_list.iter() { th { key: "{s(a, \"id\")}", class: "px-3 py-2", {s(a, "name")} } } } }
                tbody {
                    if days.is_empty() { tr { td { class: "px-3 py-3 text-muted", colspan: "99", "No rosters this month." } } }
                    for d in days.iter() {
                        tr { key: "{s(d, \"dateStr\")}", class: "border-b border-line",
                            td { class: "px-3 py-1.5 tabular whitespace-nowrap", {s(d, "dateStr")} }
                            for a in area_list.iter() {
                                td { key: "{s(a, \"id\")}", class: "px-3 py-1.5",
                                    {d["assignments"].as_array().cloned().unwrap_or_default().iter().filter(|x| s(x, "areaId") == s(a, "id")).map(|x| name_of(&map, &s(x, "aNumber"))).collect::<Vec<_>>().join(", ")}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// -------------------------------------------------------------------- backup

/// The lists a backup can hold (Service 5's selective backup picker).
pub const BACKUP_LISTS: &[(&str, &str)] = &[
    ("workers", "People"),
    ("attendance_log", "Attendance log"),
    ("assignment_log", "Assignment log"),
    ("duty_areas", "Duty areas"),
    ("duty_schedules", "Duty rosters"),
    ("ot_entries", "Overtime"),
    ("visitations", "Visitations"),
    ("visitation_groups", "Visitation groups"),
    ("ministers", "Ministers"),
    ("signatories", "Signatories"),
    ("project_groups_meta", "Project group areas"),
    ("custom_datalists", "Pick lists"),
    ("area_zones", "Area to zone map"),
    ("health_events", "Health episodes"),
    ("volunteer_health_logs", "Clinic visits"),
    ("volunteer_vaccines", "Vaccines"),
    ("volunteer_medications", "Medications"),
    ("contact_traces", "Contact tracing"),
    ("medical_audit_logs", "Clinic audit trail"),
];

/// Download only some lists (e.g. just the people, or just the clinic).
#[component]
pub fn BackupPicker(on_save: EventHandler<String>) -> Element {
    let mut picked = use_signal(|| HashSet::<&'static str>::from(["workers"]));
    let chosen: Vec<&str> = BACKUP_LISTS.iter().map(|x| x.0).filter(|k| picked().contains(k)).collect();
    let query = chosen.join(",");
    rsx! {
        details { class: "card p-3", "data-testid": "backup-picker",
            summary { class: "text-sm font-medium cursor-pointer", "Download only some lists" }
            div { class: "grid sm:grid-cols-3 gap-1 pt-2",
                for (k, label) in BACKUP_LISTS.iter().copied() {
                    label { key: "{k}", class: "flex items-center gap-2 text-sm py-1",
                        input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "backup-pick-{k}", checked: picked().contains(k),
                            onchange: move |e| { if e.checked() { picked.write().insert(k); } else { picked.write().remove(k); } } }
                        "{label}"
                    }
                }
            }
            button { class: "btn-secondary mt-2", "data-testid": "backup-selected", disabled: chosen.is_empty(), onclick: move |_| on_save.call(query.clone()), Icon { name: "archive" } "Download {chosen.len()} selected" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_sessions_split_pay_and_single_entries_keep_it() {
        let single = json!({ "volunteerId": "7", "hours": 2.0, "computedPay": 200.0, "otArea": "Hall" });
        assert_eq!(ot_shares(&single), vec![("7".to_string(), 2.0, 200.0, "Hall".to_string())]);
        let session = json!({ "title": "Pool", "hours": 3.0, "computedPay": 300.0, "volunteers": [{ "aNumber": "1" }, { "aNumber": "2", "area": "Deck" }] });
        assert_eq!(ot_shares(&session), vec![("1".to_string(), 3.0, 150.0, "Pool".to_string()), ("2".to_string(), 3.0, 150.0, "Deck".to_string())]);
    }

    #[test]
    fn timeframes() {
        let now = Local::now().date_naive();
        assert!(in_timeframe(&now.format("%Y-%m-%d").to_string(), "Next 14 days"));
        assert!(!in_timeframe(&(now + Duration::days(20)).format("%Y-%m-%d").to_string(), "Next 14 days"));
        assert!(in_timeframe("2001-01-01", "All dates"));
    }
}
