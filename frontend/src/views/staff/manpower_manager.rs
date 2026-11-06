//! Morning roster reset & manpower reallocator.
//!
//! The physical crew carry no phones, so the office allocates them here:
//! fast −5 / −1 / +1 / +5 steppers for the morning roster, and a mid-day
//! reshuffle that moves people between jobs with a quick reason.

use std::collections::HashMap;

use dioxus::prelude::*;
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::components::ui::{EmptyState, Icon, Modal, Spinner};
use crate::services::sync::{self, Outcome};
use crate::state::use_app;
use crate::util;

const QUICK_REASONS: [&str; 6] = [
    "Accelerating leveling before rain",
    "Urgent repair needs extra hands",
    "Materials just arrived",
    "Finished early here",
    "Rain — moving to indoor work",
    "Owner requested priority",
];

#[component]
pub fn CrewView() -> Element {
    let app = use_app();
    let mut draft = use_signal(HashMap::<Uuid, u32>::new);
    let mut busy = use_signal(|| false);
    let mut show_move = use_signal(|| false);
    let is_admin = app.is_admin();

    let items: Vec<WorkItem> = app
        .work_items
        .read()
        .as_ref()
        .map(|w| w.iter().filter(|w| matches!(w.status, WorkStatus::Planning | WorkStatus::InProgress | WorkStatus::Halted)).cloned().collect())
        .unwrap_or_default();
    let flat = app.flat_areas();
    let roster_prompt = app.notifications.read().iter().find(|n| n.kind == NotificationKind::MorningRosterPrompt).cloned();
    let saved: HashMap<Uuid, u32> = items.iter().map(|w| (w.id, w.active_worker_count)).collect();
    let current = |id: Uuid| -> u32 { draft.read().get(&id).copied().unwrap_or_else(|| saved.get(&id).copied().unwrap_or(0)) };
    let total: u32 = items.iter().map(|w| current(w.id)).sum();
    let changes: Vec<HeadcountAllocation> = items
        .iter()
        .filter_map(|w| {
            let v = current(w.id);
            (v != w.active_worker_count).then_some(HeadcountAllocation { work_item_id: w.id, headcount: v })
        })
        .collect();
    let change_count = changes.len();
    let saved_total: u32 = items.iter().map(|w| w.active_worker_count).sum();

    let save = move |_| {
        let changes = changes.clone();
        async move {
            busy.set(true);
            let req = ReallocateRequest { allocations: changes, transfers: vec![], reason: Some("Morning crew allocation".into()) };
            let outcome: Outcome<Vec<WorkItem>> = sync::mutate(app, "Crew allocation", "POST", "/manpower/reallocate", &req).await;
            match outcome {
                Outcome::Saved(_) => {
                    app.success("Today's crew is saved");
                    sync::load_many(app, &[DataScope::WorkItems, DataScope::Notifications]).await;
                    draft.write().clear();
                }
                Outcome::Queued => {}
                Outcome::Failed(e) => app.error(e),
            }
            busy.set(false);
        }
    };

    rsx! {
        div { class: "space-y-6 rise-in pb-20",
            header { class: "flex flex-wrap items-end justify-between gap-3",
                div {
                    h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Crew" }
                    p { class: "text-[15px] text-muted mt-1", "{saved_total} workers on site today across {util::plural(items.len(), \"job\")}." }
                }
                button { class: "btn-secondary", "data-testid": "open-move-workers", onclick: move |_| show_move.set(true), Icon { name: "move" } "Move workers mid-day" }
            }

            if is_admin {
                if let Some(p) = roster_prompt {
                    div { class: "rounded-xl bg-tag-amber-bg border border-[#f0e2b6] px-5 py-4", "data-testid": "roster-banner",
                        p { class: "font-semibold text-tag-amber-text", "Morning Roster Reset: Allocate Today's Crew" }
                        p { class: "text-sm text-tag-amber-text/90 mt-0.5", "{p.body} Use the + and − buttons below, then press Save." }
                    }
                }
            }

            if items.is_empty() {
                EmptyState { icon: "users", title: "No active jobs", body: "Jobs appear here once they're planned or in progress." }
            }

            for stream in WorkStream::ALL {
                {
                    let rows: Vec<WorkItem> = items.iter().filter(|w| w.stream == stream).cloned().collect();
                    let flat = flat.clone();
                    rsx! {
                        if !rows.is_empty() {
                            section { key: "{stream:?}",
                                p { class: "eyebrow mb-2", "{stream.label()}" }
                                div { class: "card divide-y divide-line overflow-hidden",
                                    for w in rows {
                                        {
                                            let id = w.id;
                                            let value = current(id);
                                            let changed = value != w.active_worker_count;
                                            let place = util::area_name(&flat, w.area_id);
                                            rsx! {
                                                div { key: "{id}", class: if changed { "flex flex-col sm:flex-row sm:items-center gap-3 px-4 py-3 bg-[#fbfaf5]" } else { "flex flex-col sm:flex-row sm:items-center gap-3 px-4 py-3" }, "data-testid": "crew-row",
                                                    div { class: "flex-1 min-w-0",
                                                        p { class: "text-[15px] font-medium truncate", "{w.title}" }
                                                        p { class: "text-xs text-muted truncate",
                                                            "{place}"
                                                            if changed { " · was {w.active_worker_count}" }
                                                        }
                                                    }
                                                    div { class: "flex items-center gap-1.5 self-end sm:self-auto",
                                                        for step in [-5i32, -1] {
                                                            button {
                                                                key: "{step}",
                                                                class: "btn-secondary h-11 w-12 px-0 tabular",
                                                                disabled: value == 0,
                                                                "aria-label": "Remove {-step} from {w.title}",
                                                                onclick: move |_| {
                                                                    let next = (value as i32 + step).max(0) as u32;
                                                                    draft.write().insert(id, next);
                                                                },
                                                                "{step}"
                                                            }
                                                        }
                                                        span { class: "w-12 text-center text-xl font-semibold tabular", "data-testid": "crew-count", "{value}" }
                                                        for step in [1i32, 5] {
                                                            button {
                                                                key: "{step}",
                                                                class: "btn-secondary h-11 w-12 px-0 tabular",
                                                                "aria-label": "Add {step} to {w.title}",
                                                                "data-testid": "crew-plus-{step}",
                                                                onclick: move |_| {
                                                                    draft.write().insert(id, value + step as u32);
                                                                },
                                                                "+{step}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ------------------------------------------------ sticky save bar
        if change_count > 0 {
            div { class: "fixed bottom-16 md:bottom-0 inset-x-0 md:left-60 z-30 border-t border-line bg-white/95 backdrop-blur rise-in",
                div { class: "max-w-6xl mx-auto px-4 sm:px-8 h-16 flex items-center gap-3",
                    p { class: "flex-1 text-sm",
                        span { class: "font-semibold tabular", "{total} workers" }
                        span { class: "text-muted", " · {util::plural(change_count, \"change\")}" }
                    }
                    button { class: "btn-ghost", onclick: move |_| draft.write().clear(), "Undo" }
                    button { class: "btn-primary btn-lg", disabled: busy(), "data-testid": "save-allocation", onclick: save,
                        if busy() { Spinner {} }
                        "Save today's crew"
                    }
                }
            }
        }

        if show_move() {
            MoveWorkersModal { items: items.clone(), on_close: move |_| show_move.set(false) }
        }
    }
}

#[component]
fn MoveWorkersModal(items: Vec<WorkItem>, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let with_crew: Vec<WorkItem> = items.iter().filter(|w| w.active_worker_count > 0).cloned().collect();
    let mut source = use_signal(|| with_crew.first().map(|w| w.id));
    let mut dest = use_signal(|| None::<Uuid>);
    let mut count = use_signal(|| 1u32);
    let mut reason = use_signal(|| QUICK_REASONS[0].to_string());
    let mut busy = use_signal(|| false);

    let available = source().and_then(|s| items.iter().find(|w| w.id == s)).map(|w| w.active_worker_count).unwrap_or(0);
    let n = count().min(available.max(1));
    let can_submit = source().is_some() && dest().is_some() && source() != dest() && available > 0 && !reason().trim().is_empty();

    rsx! {
        Modal { title: "Move workers between jobs", on_close: move |_| on_close.call(()),
            div { class: "space-y-5",
                div {
                    label { class: "label", r#for: "move-from", "From" }
                    select {
                        id: "move-from",
                        class: "field",
                        "data-testid": "move-from",
                        onchange: move |e| {
                            source.set(Uuid::parse_str(&e.value()).ok());
                            count.set(1);
                        },
                        for w in with_crew.iter() {
                            option { key: "{w.id}", value: "{w.id}", selected: source() == Some(w.id), "{w.title} — {w.active_worker_count} on site" }
                        }
                    }
                    if with_crew.is_empty() {
                        p { class: "hint", "No job has workers yet. Allocate today's crew first." }
                    }
                }
                div {
                    label { class: "label", r#for: "move-to", "To" }
                    select {
                        id: "move-to",
                        class: "field",
                        "data-testid": "move-to",
                        onchange: move |e| dest.set(Uuid::parse_str(&e.value()).ok()),
                        option { value: "", selected: dest().is_none(), "Choose a job…" }
                        for w in items.iter().filter(|w| Some(w.id) != source()) {
                            option { key: "{w.id}", value: "{w.id}", selected: dest() == Some(w.id), "{w.title} — {w.active_worker_count} on site" }
                        }
                    }
                }
                div {
                    p { class: "label", "How many people?" }
                    div { class: "flex items-center gap-2",
                        button { class: "btn-secondary h-11 w-12 px-0", disabled: n <= 1, onclick: move |_| count.set(n.saturating_sub(1).max(1)), Icon { name: "minus" } }
                        span { class: "w-14 text-center text-2xl font-semibold tabular", "data-testid": "move-count", "{n}" }
                        button { class: "btn-secondary h-11 w-12 px-0", "data-testid": "move-plus", disabled: n >= available, onclick: move |_| count.set((n + 1).min(available)), Icon { name: "plus" } }
                        span { class: "text-sm text-muted ml-2", "of {available} available" }
                    }
                }
                div {
                    p { class: "label", "Why?" }
                    div { class: "flex flex-wrap gap-2 mb-2",
                        for r in QUICK_REASONS {
                            button {
                                key: "{r}",
                                class: if reason() == r { "pill h-8 px-3 bg-ink text-white" } else { "pill h-8 px-3 bg-hover text-ink hover:bg-[#e8e8e5]" },
                                onclick: move |_| reason.set(r.to_string()),
                                "{r}"
                            }
                        }
                    }
                    input { class: "field", placeholder: "Or type a reason", value: "{reason}", oninput: move |e| reason.set(e.value()) }
                }
                div { class: "flex justify-end gap-2 pt-1",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button {
                        class: "btn-primary btn-lg",
                        disabled: !can_submit || busy(),
                        "data-testid": "confirm-move-workers",
                        onclick: move |_| async move {
                            let (Some(s), Some(d)) = (source(), dest()) else { return };
                            busy.set(true);
                            let req = ReallocateRequest {
                                allocations: vec![],
                                transfers: vec![CrewTransfer { source_work_item_id: s, destination_work_item_id: d, headcount: n, reason: reason() }],
                                reason: None,
                            };
                            let outcome: Outcome<Vec<WorkItem>> = sync::mutate(app, format!("Move {n} workers"), "POST", "/manpower/reallocate", &req).await;
                            match outcome {
                                Outcome::Saved(_) => {
                                    app.success(format!("Moved {}", util::plural(n as usize, "worker")));
                                    sync::load_many(app, &[DataScope::WorkItems]).await;
                                    on_close.call(());
                                }
                                Outcome::Queued => on_close.call(()),
                                Outcome::Failed(e) => app.error(e),
                            }
                            busy.set(false);
                        },
                        if busy() { Spinner {} }
                        "Move {util::plural(n as usize, \"worker\")}"
                    }
                }
            }
        }
    }
}
