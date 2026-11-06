//! Weighted phase updater & routine maintenance one-tap completion.

use chrono::Utc;
use dioxus::prelude::*;
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::components::ui::{Icon, Modal, PhotoCapture, Pill, Spinner};
use crate::services::sync::{self, Outcome};
use crate::state::{use_app, AppState};
use crate::util;

/// Optimistically marks a phase done in the cache (offline-friendly).
fn patch_phase_done(app: AppState, work_item_id: Uuid, phase_id: Uuid) {
    let mut details = app.details;
    let mut map = details.write();
    if let Some(d) = map.get_mut(&work_item_id) {
        if let Some(p) = d.phases.iter_mut().find(|p| p.id == phase_id) {
            p.is_completed = true;
            p.completed_at = Some(Utc::now());
        }
        let raw = shared::labels::weighted_progress(&d.phases);
        d.item.calculated_progress = shared::labels::displayed_progress(raw, false);
    }
    drop(map);
}

/// Capital projects: ordered phases with weights and [Complete Phase].
#[component]
pub fn ProgressUpdater(detail: WorkItemDetail) -> Element {
    let app = use_app();
    let mut completing = use_signal(|| None::<ProjectPhase>);
    let mut proof = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let item_id = detail.item.id;
    let locked = matches!(detail.item.status, WorkStatus::PendingSignOff | WorkStatus::CompletedAndArchived);
    let next_open = detail.phases.iter().find(|p| !p.is_completed).map(|p| p.id);

    let submit = move |phase: ProjectPhase| async move {
        busy.set(true);
        let req = CompletePhaseRequest { proof_data_url: proof() };
        let outcome: Outcome<ProgressUpdate> =
            sync::mutate(app, format!("Phase completed: {}", phase.phase_name), "POST", format!("/phases/{}/complete", phase.id), &req).await;
        match outcome {
            Outcome::Saved(update) => {
                if update.handover_created {
                    app.toast(
                        crate::state::ToastTone::Success,
                        "All phases done — sent for sign-off",
                        Some("The project is held at 99% and the owner now has the handover report.".into()),
                    );
                } else {
                    app.success(format!("{} completed — progress is now {}%", phase.phase_name, update.item.calculated_progress));
                }
                patch_phase_done(app, item_id, phase.id);
                sync::reload(app, vec![DataScope::WorkItems, DataScope::Map, DataScope::Photos]);
                completing.set(None);
                proof.set(None);
            }
            Outcome::Queued => {
                patch_phase_done(app, item_id, phase.id);
                completing.set(None);
                proof.set(None);
            }
            Outcome::Failed(e) => app.error(e),
        }
        busy.set(false);
    };

    rsx! {
        section { class: "card", "data-testid": "progress-updater",
            div { class: "px-4 py-3 border-b border-line flex items-center justify-between",
                h3 { class: "font-semibold", "Update progress" }
                span { class: "text-sm text-muted tabular", "{detail.item.calculated_progress}% done" }
            }
            ol { class: "divide-y divide-line",
                for p in detail.phases.iter().cloned() {
                    li { key: "{p.id}", class: "flex items-center gap-3 px-4 py-3 min-h-14",
                        span {
                            class: if p.is_completed { "w-7 h-7 rounded-full bg-tag-green-bg text-tag-green-text flex items-center justify-center shrink-0" } else { "w-7 h-7 rounded-full border border-line text-muted flex items-center justify-center text-xs shrink-0 tabular" },
                            if p.is_completed { Icon { name: "check", size: 14 } } else { "{p.order_index}" }
                        }
                        div { class: "flex-1 min-w-0",
                            p { class: if p.is_completed { "text-[15px] text-muted" } else { "text-[15px] font-medium" }, "{p.order_index}. {p.phase_name} ({p.weight_percentage}%)" }
                            if let Some(at) = p.completed_at {
                                p { class: "text-xs text-muted", "Completed {util::day(at)}" }
                            }
                        }
                        if !p.is_completed && !locked {
                            button {
                                class: if Some(p.id) == next_open { "btn-primary h-10" } else { "btn-secondary h-10" },
                                "data-testid": "complete-phase-{p.order_index}",
                                onclick: {
                                    let target = p.clone();
                                    move |_| {
                                        proof.set(None);
                                        completing.set(Some(target.clone()));
                                    }
                                },
                                Icon { name: "camera" }
                                "Complete Phase"
                            }
                        }
                    }
                }
            }
            if locked {
                p { class: "px-4 py-3 text-sm text-muted border-t border-line",
                    if detail.item.status == WorkStatus::PendingSignOff { "All phases are done. Waiting for the owner's sign-off." } else { "This project is finished and archived." }
                }
            }
        }
        if let Some(phase) = completing() {
            Modal { title: format!("Complete \u{201c}{}\u{201d}", phase.phase_name), on_close: move |_| completing.set(None),
                div { class: "space-y-4",
                    p { class: "text-sm text-muted", "Take a photo of the finished work as proof. It is cropped to 16:9 automatically so the owner sees exactly what you see." }
                    PhotoCapture { id: "phase-proof", photo: proof, label: "Take proof photo" }
                    div { class: "flex flex-col-reverse sm:flex-row sm:justify-end gap-2 pt-2",
                        button { class: "btn-ghost", onclick: move |_| completing.set(None), "Cancel" }
                        {
                            let ph = phase.clone();
                            rsx! {
                                button {
                                    class: "btn-primary btn-lg",
                                    disabled: busy(),
                                    "data-testid": "confirm-complete-phase",
                                    onclick: move |_| submit(ph.clone()),
                                    if busy() { Spinner {} }
                                    if proof().is_some() { "Complete phase with photo" } else { "Complete without photo" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Routine upkeep & repairs: [Mark Completed + Snap Proof Photo].
#[component]
pub fn RoutineCompleter(item: WorkItem) -> Element {
    let app = use_app();
    let mut open = use_signal(|| false);
    let mut proof = use_signal(|| None::<String>);
    let mut note = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let id = item.id;
    let title = item.title.clone();
    if item.status == WorkStatus::CompletedAndArchived {
        return rsx! {
            div { class: "card px-4 py-3 text-sm text-muted flex items-center gap-2", Icon { name: "check", class: "text-tag-green-text" } "This repair is finished." }
        };
    }
    rsx! {
        button {
            class: "btn-primary btn-lg w-full sm:w-auto",
            "data-testid": "mark-completed",
            onclick: move |_| open.set(true),
            Icon { name: "camera", size: 18 }
            "Mark Completed + Snap Proof Photo"
        }
        if open() {
            Modal { title: format!("Complete \u{201c}{title}\u{201d}"), on_close: move |_| open.set(false),
                div { class: "space-y-4",
                    PhotoCapture { id: "routine-proof", photo: proof, label: "Snap proof photo" }
                    div {
                        label { class: "label", r#for: "routine-note", "Note for the record (optional)" }
                        textarea { id: "routine-note", class: "field", rows: "2", placeholder: "e.g. Chemicals balanced, filter basket emptied", value: "{note}", oninput: move |e| note.set(e.value()) }
                    }
                    div { class: "flex justify-end gap-2",
                        button { class: "btn-ghost", onclick: move |_| open.set(false), "Cancel" }
                        button {
                            class: "btn-primary btn-lg",
                            disabled: busy(),
                            "data-testid": "confirm-mark-completed",
                            onclick: move |_| async move {
                                busy.set(true);
                                let req = CompleteTaskRequest { proof_data_url: proof(), note: Some(note()).filter(|n| !n.trim().is_empty()) };
                                let outcome: Outcome<WorkItem> = sync::mutate(app, "Task completed", "POST", format!("/work-items/{id}/complete-task"), &req).await;
                                match outcome {
                                    Outcome::Saved(_) => {
                                        app.success("Marked completed");
                                        sync::reload(app, vec![DataScope::WorkItems, DataScope::Map, DataScope::Photos]);
                                        open.set(false);
                                        proof.set(None);
                                        note.set(String::new());
                                    }
                                    Outcome::Queued => {
                                        open.set(false);
                                        proof.set(None);
                                    }
                                    Outcome::Failed(e) => app.error(e),
                                }
                                busy.set(false);
                            },
                            if busy() { Spinner {} }
                            "Mark completed"
                        }
                    }
                }
            }
        }
    }
}

/// Adds a standard 16:9 site photo to a project.
#[component]
pub fn PhotoUploader(work_item_id: Uuid) -> Element {
    let app = use_app();
    let mut open = use_signal(|| false);
    let mut photo = use_signal(|| None::<String>);
    let mut caption = use_signal(String::new);
    let mut busy = use_signal(|| false);
    rsx! {
        button { class: "btn-secondary", "data-testid": "add-photo", onclick: move |_| open.set(true), Icon { name: "image" } "Add site photo" }
        if open() {
            Modal { title: "Add a site photo", on_close: move |_| open.set(false),
                div { class: "space-y-4",
                    PhotoCapture { id: "site-photo", photo }
                    div {
                        label { class: "label", r#for: "photo-caption", "What does it show?" }
                        input { id: "photo-caption", class: "field", placeholder: "e.g. Paving laid along the drop-off", value: "{caption}", oninput: move |e| caption.set(e.value()) }
                    }
                    div { class: "flex justify-end gap-2",
                        button { class: "btn-ghost", onclick: move |_| open.set(false), "Cancel" }
                        button {
                            class: "btn-primary",
                            disabled: busy() || photo().is_none(),
                            "data-testid": "upload-photo",
                            onclick: move |_| async move {
                                let Some(data) = photo() else { return };
                                busy.set(true);
                                let req = UploadPhotoRequest { work_item_id, caption: caption(), image_data_url: data };
                                let outcome: Outcome<SitePhoto> = sync::mutate(app, "Site photo upload", "POST", "/photos", &req).await;
                                match outcome {
                                    Outcome::Saved(_) => {
                                        app.success("Photo added");
                                        sync::reload(app, vec![DataScope::Photos, DataScope::WorkItems]);
                                        open.set(false);
                                        photo.set(None);
                                        caption.set(String::new());
                                    }
                                    Outcome::Queued => {
                                        open.set(false);
                                        photo.set(None);
                                        caption.set(String::new());
                                    }
                                    Outcome::Failed(e) => app.error(e),
                                }
                                busy.set(false);
                            },
                            if busy() { Spinner {} }
                            "Upload photo"
                        }
                    }
                }
            }
        }
    }
}

/// Quick status label shown next to a project on staff lists.
#[component]
pub fn StatusPill(status: WorkStatus) -> Element {
    let app = use_app();
    let text = if app.is_vip() { status.label() } else { status.staff_label() };
    rsx! { Pill { tone: status.tone(), text: text.to_string() } }
}
