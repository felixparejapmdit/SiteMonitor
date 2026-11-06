//! The "Draft-to-VIP" decision pipeline.
//!
//! 1. A Site Monitor snaps a 16:9 photo, flags an issue and sends it to the
//!    Admin Review Queue.
//! 2. The Office Admin polishes the wording, defines Option A / Option B (or
//!    a simple acknowledgement) and clicks [Publish to VIP Client].

use dioxus::prelude::*;
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::components::ui::{EmptyState, Icon, PhotoCapture, Pill, Spinner};
use crate::services::sync::{self, Outcome};
use crate::state::use_app;
use crate::util;
use crate::views::staff::asset_ops::ChoiceButton;
use crate::Route;

#[component]
pub fn DecisionsView() -> Element {
    let app = use_app();
    let is_admin = app.is_admin();
    let decisions = app.decisions.read().clone().unwrap_or_default();
    let drafts: Vec<DecisionAlert> = decisions.iter().filter(|d| d.status == DecisionStatus::DraftInAdminQueue).cloned().collect();
    let waiting: Vec<DecisionAlert> = decisions.iter().filter(|d| d.status == DecisionStatus::PublishedPendingClient).cloned().collect();
    let answered: Vec<DecisionAlert> = decisions.iter().filter(|d| d.status.is_resolved()).cloned().collect();
    let mut tab = use_signal(move || if is_admin { "queue" } else { "report" });

    rsx! {
        div { class: "space-y-6 rise-in",
            header { class: "space-y-1",
                h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", if is_admin { "Issues & Decisions" } else { "Report an issue" } }
                p { class: "text-[15px] text-muted",
                    if is_admin { "Review reports from the field, polish them and send them to the owner." } else { "Spotted a problem? Tell the office in under a minute — it works without signal too." }
                }
            }

            div { class: "inline-flex p-0.5 rounded-lg bg-hover text-sm", role: "tablist",
                if is_admin {
                    TabButton { active: tab() == "queue", onclick: move |_| tab.set("queue"), label: format!("Review queue ({})", drafts.len()) }
                }
                TabButton { active: tab() == "report", onclick: move |_| tab.set("report"), label: "New report".to_string() }
                TabButton { active: tab() == "history", onclick: move |_| tab.set("history"), label: format!("With the owner ({})", waiting.len()) }
            }

            match tab() {
                "queue" => rsx! {
                    if drafts.is_empty() {
                        EmptyState { icon: "check", title: "The review queue is empty", body: "New field reports will appear here." }
                    } else {
                        div { class: "space-y-4",
                            for d in drafts {
                                PublishEditor { key: "{d.id}", draft: d }
                            }
                        }
                    }
                },
                "history" => rsx! {
                    section { class: "space-y-6",
                        DecisionList { title: "Waiting for the owner", items: waiting, empty: "Nothing is waiting for the owner right now." }
                        DecisionList { title: "Answered", items: answered, empty: "No answers yet." }
                    }
                },
                _ => rsx! {
                    FieldReportForm {}
                    if !is_admin {
                        section { class: "mt-8",
                            DecisionList { title: "Reports waiting for office review", items: drafts, empty: "None — the office is up to date." }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn TabButton(active: bool, label: String, onclick: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            role: "tab",
            "aria-selected": active,
            class: if active { "px-3 h-8 rounded-md bg-white shadow-sm font-medium" } else { "px-3 h-8 rounded-md text-muted hover:text-ink" },
            onclick: move |e| onclick.call(e),
            "{label}"
        }
    }
}

/// Site Monitor field input.
#[component]
fn FieldReportForm() -> Element {
    let app = use_app();
    let items: Vec<WorkItem> = app.work_items.read().clone().unwrap_or_default().into_iter().filter(|w| w.status.is_active()).collect();
    let mut project = use_signal(|| None::<Uuid>);
    let mut category = use_signal(|| AlertCategory::MaterialDelay);
    let mut urgency = use_signal(|| UrgencyLevel::Standard);
    let mut title = use_signal(String::new);
    let mut issue = use_signal(String::new);
    let mut mitigation = use_signal(String::new);
    let photo = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let flat = app.flat_areas();
    let can_submit = project().is_some() && !title().trim().is_empty() && !issue().trim().is_empty();

    rsx! {
        form {
            class: "card p-5 space-y-5 max-w-2xl",
            "data-testid": "field-report",
            onsubmit: move |e| e.prevent_default(),
            div {
                label { class: "label", r#for: "report-project", "Which job is affected?" }
                select { id: "report-project", class: "field", "data-testid": "report-project", onchange: move |e| project.set(Uuid::parse_str(&e.value()).ok()),
                    option { value: "", selected: project().is_none(), "Choose a job…" }
                    for w in items.iter() {
                        option { key: "{w.id}", value: "{w.id}", selected: project() == Some(w.id), "{w.title} — {util::area_name(&flat, w.area_id)}" }
                    }
                }
            }
            div {
                p { class: "label", "What kind of issue?" }
                div { class: "flex flex-wrap gap-2",
                    for c in AlertCategory::ALL {
                        button {
                            key: "{c:?}",
                            r#type: "button",
                            class: if category() == c { "pill h-8 px-3 bg-ink text-white" } else { "pill h-8 px-3 bg-hover text-ink hover:bg-[#e8e8e5]" },
                            onclick: move |_| category.set(c),
                            "{c.label()}"
                        }
                    }
                }
            }
            div {
                p { class: "label", "How urgent?" }
                div { class: "grid grid-cols-3 gap-2",
                    for u in UrgencyLevel::ALL {
                        ChoiceButton { key: "{u:?}", active: urgency() == u, label: u.label(), hint: "", onclick: move |_| urgency.set(u) }
                    }
                }
            }
            PhotoCapture { id: "report-photo", photo, label: "Snap a photo of the issue" }
            div {
                label { class: "label", r#for: "report-title", "Short title" }
                input { id: "report-title", class: "field", "data-testid": "report-title", placeholder: "e.g. Imported paving stones delayed by 2 weeks", value: "{title}", oninput: move |e| title.set(e.value()) }
            }
            div {
                label { class: "label", r#for: "report-issue", "What happened?" }
                textarea { id: "report-issue", class: "field", rows: "3", "data-testid": "report-issue", value: "{issue}", oninput: move |e| issue.set(e.value()) }
            }
            div {
                label { class: "label", r#for: "report-mitigation", "What have you already done about it?" }
                textarea { id: "report-mitigation", class: "field", rows: "2", placeholder: "Optional", value: "{mitigation}", oninput: move |e| mitigation.set(e.value()) }
            }
            div { class: "flex justify-end",
                button {
                    r#type: "button",
                    class: "btn-primary btn-lg",
                    disabled: !can_submit || busy(),
                    "data-testid": "submit-report",
                    onclick: move |_| async move {
                        let Some(work_item_id) = project() else { return };
                        busy.set(true);
                        let req = CreateDecisionRequest {
                            work_item_id,
                            category: category(),
                            urgency: urgency(),
                            title: title(),
                            issue_description: issue(),
                            mitigation_action_taken: mitigation(),
                            photo_data_url: photo(),
                        };
                        let outcome: Outcome<DecisionAlert> = sync::mutate(app, format!("Issue report: {}", title()), "POST", "/decisions", &req).await;
                        match outcome {
                            Outcome::Saved(_) => {
                                app.success("Sent to the office for review");
                                sync::reload(app, vec![DataScope::Decisions, DataScope::Photos]);
                                title.set(String::new());
                                issue.set(String::new());
                                mitigation.set(String::new());
                                let mut p = photo;
                                p.set(None);
                            }
                            Outcome::Queued => {
                                title.set(String::new());
                                issue.set(String::new());
                                mitigation.set(String::new());
                                let mut p = photo;
                                p.set(None);
                            }
                            Outcome::Failed(e) => app.error(e),
                        }
                        busy.set(false);
                    },
                    if busy() { Spinner {} }
                    Icon { name: "upload" }
                    "Send to Admin Review Queue"
                }
            }
        }
    }
}

/// Office Admin polish & publish.
#[component]
fn PublishEditor(draft: DecisionAlert) -> Element {
    let app = use_app();
    let id = draft.id;
    let mut title = use_signal(|| draft.title.clone());
    let mut summary = use_signal(|| draft.issue_description.clone());
    let mut mitigation = use_signal(|| draft.mitigation_action_taken.clone());
    let mut category = use_signal(|| draft.category);
    let mut urgency = use_signal(|| draft.urgency);
    let mut needs_choice = use_signal(|| draft.category != AlertCategory::RoutineUpdate);
    let mut option_a = use_signal(String::new);
    let mut option_b = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let project = app.work_item(draft.work_item_id).map(|w| w.title).unwrap_or_default();
    let place = util::area_name(&app.flat_areas(), draft.area_id);
    let options_ok = !needs_choice() || (!option_a().trim().is_empty() && !option_b().trim().is_empty());
    let can_publish = options_ok && !title().trim().is_empty() && !summary().trim().is_empty();

    rsx! {
        article { class: "card overflow-hidden", "data-testid": "publish-editor",
            div { class: "px-5 py-3 bg-sidebar border-b border-line flex flex-wrap items-center gap-2 text-sm",
                Pill { tone: Tone::Gray, text: "Field draft" }
                span { class: "font-medium text-ink", "data-testid": "draft-original-title", "{draft.title}" }
                span { class: "text-muted", "{project} · {place} · {util::relative(draft.created_at)}" }
                Link { to: Route::Project { id: draft.work_item_id }, class: "ml-auto text-muted hover:text-ink", "Open project" }
            }
            div { class: "p-5 space-y-4",
                div {
                    label { class: "label", "Title the owner will see" }
                    input { class: "field", "data-testid": "publish-title", value: "{title}", oninput: move |e| title.set(e.value()) }
                }
                div {
                    label { class: "label", "Executive summary" }
                    textarea { class: "field", rows: "3", "data-testid": "publish-summary", value: "{summary}", oninput: move |e| summary.set(e.value()) }
                }
                div {
                    label { class: "label", "What the team has already done" }
                    textarea { class: "field", rows: "2", value: "{mitigation}", oninput: move |e| mitigation.set(e.value()) }
                }
                div { class: "grid sm:grid-cols-2 gap-4",
                    div {
                        label { class: "label", "Type" }
                        select { class: "field", onchange: move |e| {
                                if let Some(c) = AlertCategory::ALL.iter().find(|c| format!("{c:?}") == e.value()) {
                                    category.set(*c);
                                    if *c == AlertCategory::RoutineUpdate { needs_choice.set(false); }
                                }
                            },
                            for c in AlertCategory::ALL {
                                option { key: "{c:?}", value: "{c:?}", selected: category() == c, "{c.label()}" }
                            }
                        }
                    }
                    div {
                        label { class: "label", "Urgency" }
                        select { class: "field", onchange: move |e| {
                                if let Some(u) = UrgencyLevel::ALL.iter().find(|u| format!("{u:?}") == e.value()) { urgency.set(*u); }
                            },
                            for u in UrgencyLevel::ALL {
                                option { key: "{u:?}", value: "{u:?}", selected: urgency() == u, "{u.label()}" }
                            }
                        }
                    }
                }
                div {
                    p { class: "label", "What should the owner do?" }
                    div { class: "grid sm:grid-cols-2 gap-2",
                        ChoiceButton { active: needs_choice(), label: "Choose between two options", hint: "Shows [Approve Option A] / [Approve Option B]", onclick: move |_| needs_choice.set(true) }
                        ChoiceButton { active: !needs_choice(), label: "Just keep them informed", hint: "Shows [Acknowledge Receipt]", onclick: move |_| needs_choice.set(false) }
                    }
                }
                if needs_choice() {
                    div { class: "grid sm:grid-cols-2 gap-4 rise-in",
                        div {
                            label { class: "label", "Option A" }
                            input { class: "field", "data-testid": "option-a", placeholder: "e.g. Approve Local Paving Supplier - No Delay", value: "{option_a}", oninput: move |e| option_a.set(e.value()) }
                        }
                        div {
                            label { class: "label", "Option B" }
                            input { class: "field", "data-testid": "option-b", placeholder: "e.g. Wait for Imported Stones - +14 Days Delay", value: "{option_b}", oninput: move |e| option_b.set(e.value()) }
                        }
                    }
                }
                div { class: "flex justify-end pt-1",
                    button {
                        class: "btn-primary btn-lg",
                        disabled: !can_publish || busy(),
                        "data-testid": "publish-to-vip",
                        onclick: move |_| async move {
                            busy.set(true);
                            let choice = needs_choice();
                            let req = PublishDecisionRequest {
                                title: title(),
                                issue_description: summary(),
                                mitigation_action_taken: mitigation(),
                                category: if choice || category() != AlertCategory::RoutineUpdate { category() } else { AlertCategory::RoutineUpdate },
                                urgency: urgency(),
                                option_a_label: if choice { Some(option_a()) } else { None },
                                option_b_label: if choice { Some(option_b()) } else { None },
                            };
                            let outcome: Outcome<DecisionAlert> = sync::mutate(app, "Publish to owner", "PUT", format!("/decisions/{id}/publish"), &req).await;
                            match outcome {
                                Outcome::Saved(_) => {
                                    app.success("Published — the owner was notified instantly");
                                    sync::reload(app, vec![DataScope::Decisions, DataScope::Map]);
                                }
                                Outcome::Queued => {}
                                Outcome::Failed(e) => app.error(e),
                            }
                            busy.set(false);
                        },
                        if busy() { Spinner {} }
                        "Publish to VIP Client"
                    }
                }
            }
        }
    }
}

#[component]
fn DecisionList(#[props(into)] title: String, items: Vec<DecisionAlert>, #[props(into)] empty: String) -> Element {
    let app = use_app();
    rsx! {
        section {
            h2 { class: "section-title mb-3", "{title}" }
            if items.is_empty() {
                p { class: "text-sm text-muted", "{empty}" }
            } else {
                ul { class: "card divide-y divide-line",
                    for d in items {
                        li { key: "{d.id}", class: "px-4 py-3",
                            div { class: "flex flex-wrap items-center gap-2",
                                p { class: "text-[15px] font-medium flex-1 min-w-[12rem]", "{d.title}" }
                                Pill { tone: d.status.tone(), text: d.status.label().to_string() }
                            }
                            p { class: "text-xs text-muted mt-0.5",
                                "{app.work_item(d.work_item_id).map(|w| w.title).unwrap_or_default()} · {d.category.label()} · {util::relative(d.created_at)}"
                            }
                            match d.status {
                                DecisionStatus::ApprovedOptionA => rsx! { p { class: "text-sm mt-1 text-tag-green-text", "Chose: {d.option_a_label.clone().unwrap_or_default()}" } },
                                DecisionStatus::ApprovedOptionB => rsx! { p { class: "text-sm mt-1 text-tag-green-text", "Chose: {d.option_b_label.clone().unwrap_or_default()}" } },
                                _ => rsx! {},
                            }
                        }
                    }
                }
            }
        }
    }
}
