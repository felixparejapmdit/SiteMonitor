//! Office Admin: create a project, or edit its details, deadline, status and phases.

use chrono::{Duration, Utc};
use dioxus::prelude::*;
use shared::api::{CancelProjectRequest, PhaseInput, SaveWorkItemRequest, WorkItemDetail};
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{Icon, Modal, Pill, Spinner};
use crate::services::sync;
use crate::state::use_app;
use crate::util;
use crate::Route;

const PICKABLE: [WorkStatus; 3] = [WorkStatus::Planning, WorkStatus::InProgress, WorkStatus::Halted];

#[derive(Clone, PartialEq)]
struct PhaseRow {
    key: u32,
    id: Option<Uuid>,
    name: String,
    weight: u8,
    done: bool,
}

/// Cancel keeps the project in History with its reason; delete is only for one
/// made by mistake (nothing done yet, no photos, no reports — the server checks).
#[component]
fn StopProject(item: WorkItem, on_done: EventHandler<()>) -> Element {
    let app = use_app();
    let nav = use_navigator();
    let mut mode = use_signal(|| None::<&'static str>);
    let mut reason = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let id = item.id;
    let deletable = item.calculated_progress == 0;
    let finish = move |msg: String| {
        app.success(msg);
        sync::reload(app, vec![DataScope::WorkItems, DataScope::Map, DataScope::Agenda]);
        on_done.call(());
    };
    rsx! {
        section { class: "rounded-lg border border-line p-4 space-y-3", "data-testid": "stop-project",
            p { class: "label mb-0", "Stop this project" }
            match mode() {
                None => rsx! {
                    div { class: "flex flex-wrap gap-2",
                        button { class: "btn-secondary", "data-testid": "cancel-project", onclick: move |_| mode.set(Some("cancel")), "Cancel project" }
                        if deletable {
                            button { class: "btn-ghost text-tag-red-text", "data-testid": "delete-project", onclick: move |_| mode.set(Some("delete")), Icon { name: "trash" } "Delete project" }
                        }
                    }
                    p { class: "hint mt-0", if deletable { "Cancel keeps it in History. Delete removes it completely — only for a project made by mistake." } else { "Cancelled projects stay in History with the reason." } }
                },
                Some("cancel") => rsx! {
                    label { class: "label", r#for: "cancel-reason", "Why is it being cancelled?" }
                    textarea { id: "cancel-reason", class: "field", rows: "2", "data-testid": "cancel-reason", value: "{reason}", oninput: move |e| reason.set(e.value()) }
                    div { class: "flex justify-end gap-2",
                        button { class: "btn-ghost", onclick: move |_| mode.set(None), "Keep it" }
                        button { class: "btn-danger", "data-testid": "cancel-confirm", disabled: busy() || reason().trim().is_empty(),
                            onclick: move |_| async move {
                                busy.set(true);
                                match api::post::<_, WorkItem>(&format!("/work-items/{id}/cancel"), &CancelProjectRequest { reason: reason() }).await {
                                    Ok(w) => finish(format!("{} cancelled", w.title)),
                                    Err(e) => app.error(e.message()),
                                }
                                busy.set(false);
                            },
                            "Cancel project"
                        }
                    }
                },
                _ => rsx! {
                    p { class: "text-sm text-tag-red-text", "Delete “{item.title}” and its phases for good?" }
                    div { class: "flex justify-end gap-2",
                        button { class: "btn-ghost", onclick: move |_| mode.set(None), "Keep it" }
                        button { class: "btn-danger", "data-testid": "delete-confirm", disabled: busy(),
                            onclick: move |_| async move {
                                busy.set(true);
                                match api::send_raw("DELETE", &format!("/work-items/{id}"), None).await {
                                    Ok(_) => {
                                        finish("Project deleted".into());
                                        nav.push(Route::Home {});
                                    }
                                    Err(e) => app.error(e.message()),
                                }
                                busy.set(false);
                            },
                            "Delete for good"
                        }
                    }
                },
            }
        }
    }
}

/// `detail: None` creates a new project.
#[component]
pub fn ProjectModal(detail: Option<WorkItemDetail>, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let nav = use_navigator();
    let existing = detail.as_ref().map(|d| d.item.clone());
    let id = existing.as_ref().map(|w| w.id);
    let locked = existing.as_ref().is_some_and(|w| !PICKABLE.contains(&w.status));
    let me = app.current_user.peek().clone();

    let mut title = use_signal(|| existing.as_ref().map(|w| w.title.clone()).unwrap_or_default());
    let mut description = use_signal(|| existing.as_ref().map(|w| w.description.clone()).unwrap_or_default());
    let mut stream = use_signal(|| existing.as_ref().map(|w| w.stream).unwrap_or(WorkStream::CapitalProject));
    let mut status = use_signal(|| existing.as_ref().map(|w| w.status).unwrap_or(WorkStatus::Planning));
    let mut area = use_signal(|| existing.as_ref().map(|w| w.area_id).or(*app.active_parent_area.peek()));
    let original_deadline = existing.as_ref().map(|w| w.current_target_deadline);
    let mut date = use_signal(|| util::to_date_input(original_deadline.unwrap_or_else(|| Utc::now() + Duration::days(30))));
    let mut people = use_signal(|| {
        existing.as_ref().map(|w| w.personnel.clone()).unwrap_or_else(|| ProjectPersonnel {
            group_leader_name: String::new(),
            group_leader_phone: String::new(),
            site_qc_officer_name: String::new(),
            site_qc_officer_phone: String::new(),
            admin_coordinator_name: me.as_ref().map(|u| u.name.clone()).unwrap_or_default(),
            admin_coordinator_phone: me.as_ref().map(|u| u.phone.clone()).unwrap_or_default(),
        })
    });
    let mut phases = use_signal(|| match &detail {
        Some(d) => d
            .phases
            .iter()
            .enumerate()
            .map(|(i, p)| PhaseRow { key: i as u32, id: Some(p.id), name: p.phase_name.clone(), weight: p.weight_percentage, done: p.is_completed })
            .collect::<Vec<_>>(),
        None => [("Site preparation", 15), ("Main works", 65), ("Finishing and clean-up", 20)]
            .iter()
            .enumerate()
            .map(|(i, (n, w))| PhaseRow { key: i as u32, id: None, name: n.to_string(), weight: *w, done: false })
            .collect(),
    });
    let mut next_key = use_signal(|| 100u32);
    let mut busy = use_signal(|| false);

    let zones: Vec<util::FlatArea> = app.flat_areas().into_iter().filter(|a| !a.area.is_archived).collect();
    let capital = stream() == WorkStream::CapitalProject;
    let rows = phases();
    let total: u32 = rows.iter().map(|p| p.weight as u32).sum();
    let phases_ok = !capital || locked || (!rows.is_empty() && total == 100 && rows.iter().all(|p| !p.name.trim().is_empty() && p.weight >= 1));
    // Keep the exact original time when the date wasn't touched, so nothing is logged as "moved".
    let deadline = match original_deadline {
        Some(o) if util::to_date_input(o) == date() => Some(o),
        _ => util::from_input(&format!("{}T17:00", date())),
    };
    let can_save = !title().trim().is_empty() && area().is_some() && deadline.is_some() && phases_ok && !busy();

    let save = move |_| async move {
        let (Some(area_id), Some(target)) = (area(), deadline) else { return };
        busy.set(true);
        let req = SaveWorkItemRequest {
            area_id,
            stream: stream(),
            title: title(),
            description: description(),
            status: status(),
            target_deadline: target,
            personnel: people(),
            phases: if stream() == WorkStream::CapitalProject {
                phases().into_iter().map(|p| PhaseInput { id: p.id, phase_name: p.name, weight_percentage: p.weight }).collect()
            } else {
                vec![]
            },
        };
        let result: Result<WorkItem, _> = match id {
            Some(id) => api::put(&format!("/work-items/{id}"), &req).await,
            None => api::post("/work-items", &req).await,
        };
        busy.set(false);
        match result {
            Ok(w) => {
                app.success(if id.is_some() { format!("{} saved", w.title) } else { format!("{} created", w.title) });
                sync::reload(app, vec![DataScope::WorkItems, DataScope::Map, DataScope::Agenda]);
                on_close.call(());
                if id.is_none() {
                    nav.push(Route::Project { id: w.id });
                }
            }
            Err(e) => app.error(e.message()),
        }
    };

    // Picked from real accounts (People), so the owner only ever sees true names and phones.
    let staff: Vec<User> = app.users.read().iter().filter(|u| u.is_active && u.role.is_staff()).cloned().collect();
    // Service 5 group leaders (Katiwala, II Katiwala, Kalihim) can lead or check a job too.
    let workers = crate::views::workforce_ops::use_workers();
    let leaders: Vec<Worker> = workers
        .read()
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|w| !w.archived && !w.group_role.trim().is_empty() && !w.group_role.trim().eq_ignore_ascii_case("member"))
        .collect();
    let person = move |label: &'static str, key: &'static str, name: String, phone: String| {
        let known = staff.iter().any(|u| u.name == name) || leaders.iter().any(|w| w.name == name);
        let options = staff.clone();
        let leader_options = leaders.clone();
        let pick_from = staff.clone();
        let pick_leader = leaders.clone();
        rsx! {
            div {
                label { class: "label", r#for: "{key}-person", "{label}" }
                select { id: "{key}-person", class: "field", "data-testid": "person-{key}",
                    onchange: move |e| {
                        let v = e.value();
                        let (n, ph) = match v.strip_prefix("w:") {
                            Some(wid) => pick_leader.iter().find(|w| w.id.to_string() == wid).map(|w| (w.name.clone(), w.phone.clone())),
                            None => pick_from.iter().find(|u| u.id.to_string() == v).map(|u| (u.name.clone(), u.phone.clone())),
                        }
                        .unwrap_or_default();
                        let mut p = people.write();
                        match key {
                            "lead" => (p.group_leader_name, p.group_leader_phone) = (n, ph),
                            "qc" => (p.site_qc_officer_name, p.site_qc_officer_phone) = (n, ph),
                            _ => (p.admin_coordinator_name, p.admin_coordinator_phone) = (n, ph),
                        }
                    },
                    option { value: "", selected: name.is_empty(), "Nobody yet" }
                    if !name.is_empty() && !known {
                        option { value: "", selected: true, disabled: true, "{name} (not in People)" }
                    }
                    for u in options {
                        option { key: "{u.id}", value: "{u.id}", selected: u.name == name, "{u.name} · {u.role.label()}" }
                    }
                    if !leader_options.is_empty() {
                        optgroup { label: "Group leaders (Personnel)",
                            for w in leader_options {
                                option { key: "w{w.id}", value: "w:{w.id}", selected: w.name == name, "{w.name} · {w.group_role}, {w.project_group}" }
                            }
                        }
                    }
                }
                if !phone.is_empty() { p { class: "hint", "Phone {phone}" } }
            }
        }
    };
    let pp = people();

    rsx! {
        Modal { title: if id.is_some() { "Edit project".to_string() } else { "New project".to_string() }, wide: true, on_close: move |_| on_close.call(()),
            div { class: "space-y-5",
                div {
                    label { class: "label", r#for: "proj-title", "Name" }
                    input { id: "proj-title", class: "field", "data-testid": "proj-title", placeholder: "e.g. Orchid Greenhouse Build", value: "{title}", oninput: move |e| title.set(e.value()) }
                }
                div { class: "grid sm:grid-cols-2 gap-4",
                    div {
                        label { class: "label", r#for: "proj-stream", "Type" }
                        select { id: "proj-stream", class: "field", "data-testid": "proj-stream", disabled: id.is_some(),
                            onchange: move |e| if let Some(s) = WorkStream::ALL.iter().find(|s| format!("{s:?}") == e.value()) { stream.set(*s) },
                            for s in WorkStream::ALL {
                                option { key: "{s:?}", value: "{s:?}", selected: stream() == s, "{s.label()}" }
                            }
                        }
                        if id.is_some() { p { class: "hint", "The type can't change after a project is created." } }
                    }
                    div {
                        label { class: "label", r#for: "proj-zone", "Where" }
                        select { id: "proj-zone", class: "field", "data-testid": "proj-zone", onchange: move |e| area.set(Uuid::parse_str(&e.value()).ok()),
                            option { value: "", selected: area().is_none(), "Choose a zone…" }
                            for z in zones.iter() {
                                option { key: "{z.area.id}", value: "{z.area.id}", selected: area() == Some(z.area.id), "{\"\u{2003}\".repeat(z.depth)}{z.area.name}" }
                            }
                        }
                        if zones.is_empty() {
                            p { class: "hint", "data-testid": "no-zones-hint", "No zones yet. "
                                Link { to: Route::Studio {}, class: "text-accent-ink underline", onclick: move |_| on_close.call(()), "Draw one in Map Studio" }
                                " first."
                            }
                        }
                    }
                    div {
                        label { class: "label", r#for: "proj-date", "Target date" }
                        input { id: "proj-date", r#type: "date", class: "field", "data-testid": "proj-date", value: "{date}", oninput: move |e| date.set(e.value()) }
                    }
                    div {
                        label { class: "label", r#for: "proj-status", "Status" }
                        select { id: "proj-status", class: "field", "data-testid": "proj-status", disabled: locked,
                            onchange: move |e| if let Some(s) = PICKABLE.iter().find(|s| format!("{s:?}") == e.value()) { status.set(*s) },
                            if locked {
                                option { selected: true, "{status().staff_label()}" }
                            }
                            for s in PICKABLE {
                                option { key: "{s:?}", value: "{s:?}", selected: status() == s, "{s.staff_label()}" }
                            }
                        }
                        if locked { p { class: "hint", "Set by the handover workflow." } }
                    }
                }
                div {
                    label { class: "label", r#for: "proj-desc", "Description" }
                    textarea { id: "proj-desc", class: "field", rows: "3", placeholder: "What is being done, and why", value: "{description}", oninput: move |e| description.set(e.value()) }
                }

                if capital {
                    section { class: "space-y-2",
                        div { class: "flex items-end justify-between gap-3",
                            div {
                                p { class: "label mb-0", "Phases" }
                                p { class: "hint mt-0", "Each phase's share of the work. Progress is the total of finished shares." }
                            }
                            span {
                                class: if total == 100 { "pill bg-tag-green-bg text-tag-green-text tabular" } else { "pill bg-tag-amber-bg text-tag-amber-text tabular" },
                                "data-testid": "phase-total",
                                "Total {total}%"
                            }
                        }
                        if locked {
                            p { class: "text-sm text-muted", "Every phase is finished, so the phases are locked while the handover is under way." }
                        }
                        ol { class: "rounded-md border border-line divide-y divide-line",
                            for (i, row) in rows.iter().cloned().enumerate() {
                                li { key: "{row.key}", class: "flex items-center gap-2 px-2 py-2", "data-testid": "phase-row",
                                    div { class: "flex flex-col",
                                        button { class: "btn-ghost h-5 w-7 px-0", "aria-label": "Move up", disabled: locked || i == 0,
                                            onclick: move |_| phases.write().swap(i, i - 1), Icon { name: "chevron-up", size: 14 } }
                                        button { class: "btn-ghost h-5 w-7 px-0", "aria-label": "Move down", disabled: locked || i + 1 == rows.len(),
                                            onclick: move |_| phases.write().swap(i, i + 1), Icon { name: "chevron-down", size: 14 } }
                                    }
                                    input { class: "field flex-1 min-w-0", "data-testid": "phase-name", "aria-label": "Phase {i + 1} name", disabled: locked,
                                        value: "{row.name}", oninput: move |e| phases.write()[i].name = e.value() }
                                    div { class: "relative w-20 shrink-0",
                                        input { r#type: "number", min: "1", max: "100", class: "field pr-6 tabular", "data-testid": "phase-weight", "aria-label": "Phase {i + 1} share", disabled: locked,
                                            value: "{row.weight}", oninput: move |e| phases.write()[i].weight = e.value().parse::<u8>().unwrap_or(0).min(100) }
                                        span { class: "absolute right-2.5 top-1/2 -translate-y-1/2 text-sm text-muted pointer-events-none", "%" }
                                    }
                                    if row.done {
                                        span { class: "w-9 flex justify-center", title: "Finished phases can't be removed", Pill { tone: Tone::Green, text: "Done" } }
                                    } else {
                                        button { class: "btn-icon btn-ghost shrink-0", "aria-label": "Remove phase", disabled: locked,
                                            onclick: move |_| { phases.write().remove(i); }, Icon { name: "trash" } }
                                    }
                                }
                            }
                        }
                        if !locked {
                            div { class: "flex flex-wrap items-center gap-2",
                                button { class: "btn-secondary", "data-testid": "add-phase",
                                    onclick: move |_| {
                                        let k = next_key() + 1;
                                        next_key.set(k);
                                        let left = 100u32.saturating_sub(phases().iter().map(|p| p.weight as u32).sum());
                                        phases.write().push(PhaseRow { key: k, id: None, name: String::new(), weight: left.clamp(1, 100) as u8, done: false });
                                    },
                                    Icon { name: "plus" }
                                    "Add phase"
                                }
                                if total != 100 {
                                    p { class: "text-sm text-tag-amber-text", "The shares must add up to exactly 100%." }
                                }
                            }
                        }
                    }
                }

                section { class: "space-y-3",
                    div {
                        p { class: "label mb-0", "Who's responsible" }
                        p { class: "hint mt-0", "Chosen from People. Missing someone? Add them on the People page first." }
                    }
                    {person("Site team lead", "lead",pp.group_leader_name.clone(), pp.group_leader_phone.clone())}
                    {person("Quality checker", "qc", pp.site_qc_officer_name.clone(), pp.site_qc_officer_phone.clone())}
                    {person("Office contact", "office",pp.admin_coordinator_name.clone(), pp.admin_coordinator_phone.clone())}
                }

                if let Some(w) = existing.clone().filter(|w| w.status.is_active()) {
                    StopProject { item: w, on_done: move |_| on_close.call(()) }
                }

                div { class: "modal-actions",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button { class: "btn-primary", disabled: !can_save, "data-testid": "save-project", onclick: save,
                        if busy() { Spinner {} }
                        if id.is_some() { "Save changes" } else { "Create project" }
                    }
                }
            }
        }
    }
}
