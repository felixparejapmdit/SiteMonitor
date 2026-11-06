//! The VIP Action Center — decisions, sign-offs and updates waiting for the
//! owner. When there is nothing to do it collapses completely.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{Icon, Pill};
use crate::services::sync;
use crate::state::use_app;
use crate::util;
use crate::Route;

/// Keeps cards in a stable order and keeps a just-handled card on screen
/// (even after a live refresh removed it) until its fold-away animation ends,
/// so nothing ever jumps or vanishes abruptly.
fn stable<T: Clone>(store: &RefCell<Vec<T>>, live: Vec<T>, held: &HashSet<Uuid>, id: impl Fn(&T) -> Uuid) -> Vec<T> {
    let prev = store.borrow().clone();
    let mut out: Vec<T> = prev
        .into_iter()
        .filter_map(|p| {
            let pid = id(&p);
            match live.iter().find(|l| id(l) == pid) {
                Some(l) => Some(l.clone()),
                None if held.contains(&pid) => Some(p),
                None => None,
            }
        })
        .collect();
    for l in live {
        if !out.iter().any(|o| id(o) == id(&l)) {
            out.push(l);
        }
    }
    *store.borrow_mut() = out.clone();
    out
}

#[component]
pub fn VipActionCenter() -> Element {
    let app = use_app();
    let held = use_signal(HashSet::<Uuid>::new);
    let decision_store = use_hook(|| Rc::new(RefCell::new(Vec::<DecisionAlert>::new())));
    let photo_store = use_hook(|| Rc::new(RefCell::new(Vec::<PhotoUpdate>::new())));
    let notice_store = use_hook(|| Rc::new(RefCell::new(Vec::<Notification>::new())));
    let held_now = held.read().clone();

    let live_decisions: Vec<DecisionAlert> = app
        .decisions
        .read()
        .as_ref()
        .map(|d| d.iter().filter(|d| d.status == DecisionStatus::PublishedPendingClient).cloned().collect())
        .unwrap_or_default();
    let decisions = stable(&decision_store, live_decisions, &held_now, |d| d.id);
    let sign_offs: Vec<WorkItem> = app
        .work_items
        .read()
        .as_ref()
        .map(|w| w.iter().filter(|w| w.status == WorkStatus::PendingSignOff).cloned().collect())
        .unwrap_or_default();
    let photos = stable(&photo_store, app.photo_updates.read().clone(), &held_now, |p| p.photo.id);
    let notices = stable(&notice_store, app.notifications.read().clone(), &held_now, |n| n.id);

    let shown = decisions.len() + sign_offs.len() + photos.len() + notices.len();
    let total = shown
        - decisions.iter().filter(|d| held_now.contains(&d.id)).count()
        - photos.iter().filter(|p| held_now.contains(&p.photo.id)).count()
        - notices.iter().filter(|n| held_now.contains(&n.id)).count();
    if shown == 0 {
        // Nothing waiting: the section disappears so the page stays calm.
        return rsx! {};
    }
    let summary = util::plural(total, "item");

    rsx! {
        section { class: "rise-in", "data-testid": "action-center", "aria-label": "Waiting for you",
            div { class: "flex items-baseline justify-between mb-3",
                h2 { class: "section-title", "Waiting for you" }
                span { class: "text-sm text-muted", "{summary}" }
            }
            div { class: "space-y-3",
                for d in decisions {
                    DecisionCard { key: "{d.id}", decision: d, held }
                }
                for w in sign_offs {
                    SignOffCard { key: "{w.id}", item: w }
                }
                for p in photos {
                    PhotoUpdateCard { key: "{p.photo.id}", update: p, held }
                }
                for n in notices {
                    NoticeCard { key: "{n.id}", notice: n, held }
                }
            }
        }
    }
}

/// Wraps a card so it folds away smoothly once handled.
#[component]
fn Fold(folded: bool, children: Element) -> Element {
    rsx! {
        div { class: if folded { "fold folded" } else { "fold" },
            div { class: "min-h-0", {children} }
        }
    }
}

#[component]
fn DecisionCard(decision: DecisionAlert, held: Signal<HashSet<Uuid>>) -> Element {
    let app = use_app();
    let mut busy = use_signal(|| false);
    let mut approved = use_signal(|| None::<String>);
    let mut folded = use_signal(|| false);
    let mut held = held;
    let id = decision.id;
    let flat = app.flat_areas();
    let place = util::area_name(&flat, decision.area_id);
    let has_options = decision.option_a_label.is_some() && decision.option_b_label.is_some();

    let respond = move |response: DecisionResponse, label: String| async move {
        busy.set(true);
        held.write().insert(id);
        match api::post::<_, DecisionAlert>(&format!("/decisions/{id}/respond"), &RespondDecisionRequest { response }).await {
            Ok(_) => {
                approved.set(Some(label));
                TimeoutFuture::new(1400).await;
                folded.set(true);
                TimeoutFuture::new(380).await;
                sync::load_many(app, &[DataScope::Decisions, DataScope::Map, DataScope::WorkItems]).await;
                held.write().remove(&id);
            }
            Err(e) => {
                held.write().remove(&id);
                app.error(e.message());
                busy.set(false);
            }
        }
    };

    rsx! {
        Fold { folded: folded(),
            article {
                class: if approved().is_some() { "card p-5 border-[#cfe0cc] bg-[#f7fbf6] transition-colors duration-500" } else { "card p-5 transition-colors duration-500" },
                "data-testid": "decision-card",
                if let Some(label) = approved() {
                    div { class: "flex items-center gap-3 text-tag-green-text fade-in",
                        span { class: "w-8 h-8 rounded-full bg-tag-green-bg flex items-center justify-center", Icon { name: "check" } }
                        div {
                            p { class: "font-medium", "Approved — {decision.title}" }
                            p { class: "text-sm opacity-80", "{label} — the team has been told." }
                        }
                    }
                } else {
                    div { class: "flex flex-wrap items-center gap-2 mb-2",
                        Pill { tone: Tone::Amber, text: if has_options { "Decision needed".to_string() } else { "New update".to_string() } }
                        Pill { tone: Tone::Gray, text: decision.category.label().to_string() }
                        if decision.urgency != UrgencyLevel::Standard {
                            Pill { tone: decision.urgency.tone(), text: decision.urgency.label().to_string() }
                        }
                        span { class: "text-xs text-muted ml-auto", "{place} · {util::relative(decision.created_at)}" }
                    }
                    h3 { class: "text-lg font-semibold leading-snug", "{decision.title}" }
                    p { class: "text-[15px] leading-relaxed mt-1.5", "{decision.issue_description}" }
                    if !decision.mitigation_action_taken.is_empty() {
                        div { class: "mt-3 rounded-md bg-sidebar px-3 py-2.5",
                            p { class: "eyebrow mb-0.5", "What your team has already done" }
                            p { class: "text-sm leading-relaxed", "{decision.mitigation_action_taken}" }
                        }
                    }
                    div { class: "flex flex-col sm:flex-row gap-2 mt-4",
                        if has_options {
                            {
                                let a = decision.option_a_label.clone().unwrap_or_default();
                                let b = decision.option_b_label.clone().unwrap_or_default();
                                let a2 = a.clone();
                                let b2 = b.clone();
                                rsx! {
                                    button {
                                        class: "btn-primary h-auto min-h-10 py-2 whitespace-normal text-left justify-start",
                                        disabled: busy(),
                                        "data-testid": "approve-a",
                                        onclick: move |_| respond(DecisionResponse::ApproveOptionA, format!("Option A: {a2}")),
                                        "Approve Option A: {a}"
                                    }
                                    button {
                                        class: "btn-secondary h-auto min-h-10 py-2 whitespace-normal text-left justify-start",
                                        disabled: busy(),
                                        "data-testid": "approve-b",
                                        onclick: move |_| respond(DecisionResponse::ApproveOptionB, format!("Option B: {b2}")),
                                        "Approve Option B: {b}"
                                    }
                                }
                            }
                        } else {
                            button {
                                class: "btn-secondary",
                                disabled: busy(),
                                "data-testid": "acknowledge",
                                onclick: move |_| respond(DecisionResponse::AcknowledgeReceipt, "Acknowledged".to_string()),
                                Icon { name: "check" }
                                "Acknowledge Receipt"
                            }
                        }
                        Link { to: Route::Project { id: decision.work_item_id }, class: "btn-ghost sm:ml-auto", "View project" }
                    }
                }
            }
        }
    }
}

#[component]
fn SignOffCard(item: WorkItem) -> Element {
    rsx! {
        article { class: "card p-5", "data-testid": "signoff-card",
            div { class: "flex flex-wrap items-center gap-2 mb-2",
                Pill { tone: Tone::Amber, text: "Ready for your sign-off" }
                span { class: "text-xs text-muted ml-auto", "Held at 99%" }
            }
            h3 { class: "text-lg font-semibold", "{item.title}" }
            p { class: "text-[15px] text-muted mt-1", "Every phase is finished. Review the before-and-after, the schedule summary and the equipment handed over, then accept or leave a final note." }
            div { class: "mt-4",
                Link { to: Route::Handover { id: item.id }, class: "btn-primary", "data-testid": "open-handover", "Review handover report" Icon { name: "arrow-right" } }
            }
        }
    }
}

#[component]
fn PhotoUpdateCard(update: PhotoUpdate, held: Signal<HashSet<Uuid>>) -> Element {
    let app = use_app();
    let mut held = held;
    let mut busy = use_signal(|| false);
    let mut folded = use_signal(|| false);
    let id = update.photo.id;
    rsx! {
        Fold { folded: folded(),
            article { class: "card p-3 sm:p-4 flex gap-4 items-center", "data-testid": "photo-update-card",
                img { src: "{update.photo.image_url}", alt: "{update.photo.caption}", class: "w-28 sm:w-36 aspect-video object-cover rounded-md bg-hover shrink-0", loading: "lazy" }
                div { class: "flex-1 min-w-0",
                    p { class: "eyebrow", "New photo · {util::relative(update.photo.uploaded_at)}" }
                    p { class: "font-medium truncate mt-0.5", "{update.photo.caption}" }
                    p { class: "text-sm text-muted truncate", "{update.work_item_title}" }
                    div { class: "flex flex-wrap gap-2 mt-2",
                        button {
                            class: "btn-secondary h-8",
                            disabled: busy(),
                            "data-testid": "acknowledge-photo",
                            onclick: move |_| async move {
                                busy.set(true);
                                held.write().insert(id);
                                match api::post::<_, SitePhoto>(&format!("/photos/{id}/acknowledge"), &serde_json::json!({})).await {
                                    Ok(_) => {
                                        folded.set(true);
                                        TimeoutFuture::new(380).await;
                                        sync::load_many(app, &[DataScope::Photos]).await;
                                        held.write().remove(&id);
                                    }
                                    Err(e) => {
                                        held.write().remove(&id);
                                        app.error(e.message());
                                        busy.set(false);
                                    }
                                }
                            },
                            Icon { name: "check" }
                            "Acknowledge Receipt"
                        }
                        Link { to: Route::Project { id: update.photo.work_item_id }, class: "btn-ghost h-8", "Open" }
                    }
                }
            }
        }
    }
}

#[component]
fn NoticeCard(notice: Notification, held: Signal<HashSet<Uuid>>) -> Element {
    let app = use_app();
    let mut held = held;
    let mut busy = use_signal(|| false);
    let mut folded = use_signal(|| false);
    let id: Uuid = notice.id;
    let (tone, tag) = match notice.kind {
        NotificationKind::DailyDigest => (Tone::Gray, "Daily summary"),
        NotificationKind::AnticipationCard => (Tone::Green, "Coming up"),
        _ => (Tone::Gray, "Update"),
    };
    rsx! {
        Fold { folded: folded(),
            article { class: "card p-5", "data-testid": "notice-card",
                div { class: "flex items-center gap-2 mb-2",
                    Pill { tone, text: tag }
                    span { class: "text-xs text-muted ml-auto", "{util::relative(notice.created_at)}" }
                }
                h3 { class: "font-semibold", "{notice.title}" }
                p { class: "text-[15px] text-muted mt-1", "{notice.body}" }
                if !notice.lines.is_empty() {
                    ul { class: "mt-3 space-y-1.5",
                        for line in notice.lines.iter() {
                            li { class: "text-sm flex gap-2", span { class: "text-muted", "•" } "{line}" }
                        }
                    }
                }
                button {
                    class: "btn-secondary mt-4",
                    disabled: busy(),
                    onclick: move |_| async move {
                        busy.set(true);
                        held.write().insert(id);
                        match api::post::<_, Notification>(&format!("/notifications/{id}/dismiss"), &serde_json::json!({})).await {
                            Ok(_) => {
                                folded.set(true);
                                TimeoutFuture::new(380).await;
                                sync::load_many(app, &[DataScope::Notifications]).await;
                                held.write().remove(&id);
                            }
                            Err(e) => {
                                held.write().remove(&id);
                                app.error(e.message());
                                busy.set(false);
                            }
                        }
                    },
                    Icon { name: "check" }
                    "Acknowledge Receipt"
                }
            }
        }
    }
}
