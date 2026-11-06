//! Auto-generated Executive Handover Report with the 99% sign-off lock.

use dioxus::prelude::*;
use shared::api::*;
use shared::ws::DataScope;
use shared::*;

use crate::api;
use crate::components::photo_annotator::PhotoAnnotator;
use crate::components::ui::{Icon, Pill, Spinner};
use crate::services::sync;
use crate::state::use_app;
use crate::util;

/// Draggable vertical split comparing Day 1 with completion.
#[component]
pub fn BeforeAfterSlider(before: String, after: String) -> Element {
    let mut pos = use_signal(|| 50.0f64);
    let p = pos();
    let clip = 100.0 - p;
    rsx! {
        div { class: "relative aspect-video rounded-xl overflow-hidden bg-hover select-none", "data-testid": "before-after",
            img { src: "{after}", alt: "After — at completion", class: "absolute inset-0 w-full h-full object-cover", draggable: "false" }
            img {
                src: "{before}",
                alt: "Before — day 1",
                class: "absolute inset-0 w-full h-full object-cover",
                style: "clip-path: inset(0 {clip}% 0 0);",
                draggable: "false",
            }
            div { class: "absolute inset-y-0 w-0.5 bg-white shadow-[0_0_0_1px_rgba(0,0,0,.08)] pointer-events-none", style: "left:{p}%;",
                div { class: "absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-9 h-9 rounded-full bg-white shadow-pop flex items-center justify-center text-ink",
                    Icon { name: "chevron-left", size: 14 }
                    Icon { name: "chevron-right", size: 14 }
                }
            }
            span { class: "absolute top-3 left-3 pill bg-white/90 text-ink pointer-events-none", "Before · Day 1" }
            span { class: "absolute top-3 right-3 pill bg-white/90 text-ink pointer-events-none", "After · Completion" }
            input {
                r#type: "range",
                min: "0",
                max: "100",
                step: "0.5",
                value: "{p}",
                class: "compare-range",
                "aria-label": "Drag to compare before and after",
                oninput: move |e| {
                    if let Ok(v) = e.value().parse::<f64>() {
                        pos.set(v);
                    }
                },
            }
        }
    }
}

#[component]
pub fn HandoverReport(detail: WorkItemDetail, report: ExecutiveHandoverReport) -> Element {
    let app = use_app();
    let nav = use_navigator();
    let is_vip = app.is_vip();
    let mut pin_mode = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let item = detail.item.clone();
    let id = item.id;

    let open_notes: usize = detail
        .photos
        .iter()
        .flat_map(|p| p.callouts.iter())
        .filter(|c| c.status != CallOutStatus::Resolved)
        .count();
    let after_photo = detail
        .photos
        .iter()
        .find(|p| p.photo.image_url == report.after_photo_url)
        .or_else(|| detail.photos.first())
        .cloned();
    let days_vs_plan = (report.actual_completion_date - report.original_deadline).num_days();
    let schedule_line = if days_vs_plan > 0 {
        format!("Finished {} after the original target.", util::plural(days_vs_plan as usize, "day"))
    } else if days_vs_plan < 0 {
        format!("Finished {} ahead of the original target.", util::plural((-days_vs_plan) as usize, "day"))
    } else {
        "Finished on the original target date.".to_string()
    };
    let signed = report.client_signed_off || item.status == WorkStatus::CompletedAndArchived;
    let progress = item.calculated_progress;

    rsx! {
        article { class: "max-w-doc mx-auto space-y-10", "data-testid": "handover-report",
            header { class: "space-y-3",
                p { class: "eyebrow", "Executive Handover Report" }
                h1 { class: "doc-title", "{item.title}" }
                div { class: "flex flex-wrap items-center gap-2",
                    if signed {
                        Pill { tone: Tone::Green, text: "Signed off · 100% complete", dot: true }
                    } else {
                        Pill { tone: Tone::Amber, text: "Waiting for sign-off · held at {progress}%", dot: true }
                    }
                    span { class: "text-sm text-muted", "{detail.area.name}" }
                }
            }

            section { class: "space-y-3",
                h2 { class: "section-title", "Before & after" }
                BeforeAfterSlider { before: report.before_photo_url.clone(), after: report.after_photo_url.clone() }
                p { class: "text-sm text-muted", "Drag the handle to compare the first day with the finished result." }
            }

            section { class: "space-y-4",
                h2 { class: "section-title", "Schedule & delays" }
                div { class: "grid grid-cols-1 sm:grid-cols-3 gap-3",
                    Stat { label: "Original target", value: util::long_day(report.original_deadline) }
                    Stat { label: "Actually finished", value: util::long_day(report.actual_completion_date) }
                    Stat { label: "Days of delay logged", value: format!("{}", report.total_delay_days) }
                }
                p { class: "text-[15px]", "{schedule_line}" }
                if report.delay_audit_summary.is_empty() {
                    p { class: "text-sm text-muted", "No delays were logged on this project." }
                } else {
                    ul { class: "divide-y divide-line border-y border-line", "data-testid": "delay-audit",
                        for line in report.delay_audit_summary.iter() {
                            li { class: "py-2.5 text-[15px] flex gap-3",
                                span { class: "text-tag-amber-text mt-0.5", Icon { name: "clock" } }
                                "{line}"
                            }
                        }
                    }
                }
            }

            section { class: "space-y-3",
                h2 { class: "section-title", "Equipment handed over" }
                p { class: "text-sm text-muted", "These built-in systems now join this zone's permanent maintenance schedule." }
                if report.handed_over_assets.is_empty() {
                    p { class: "text-sm text-muted", "No new built-in equipment was installed on this project." }
                } else {
                    ul { class: "divide-y divide-line border-y border-line",
                        for a in report.handed_over_assets.iter() {
                            li { class: "py-2.5 flex items-center gap-3",
                                Icon { name: "wrench", class: "text-muted" }
                                span { class: "flex-1 text-[15px]", "{a.name}" }
                                Pill { tone: a.health.tone(), text: a.health.label().to_string() }
                            }
                        }
                    }
                }
            }

            if signed {
                div { class: "rounded-xl bg-tag-green-bg px-5 py-4 text-tag-green-text flex items-center gap-3",
                    Icon { name: "check", size: 18 }
                    p { class: "text-[15px]", "Accepted and archived. This report is kept in the estate's history." }
                }
            } else if is_vip {
                section { class: "space-y-4 rounded-xl border border-line p-5 bg-sidebar", "data-testid": "signoff-controls",
                    h2 { class: "section-title", "Your sign-off" }
                    if open_notes > 0 {
                        p { class: "text-[15px] text-tag-amber-text",
                            "You have {util::plural(open_notes, \"open note\")} on this project. It stays at 99% until the site team resolves them."
                        }
                    } else {
                        p { class: "text-[15px] text-muted", "Accept to close the project at 100%, or leave a final note on the finished photo and the team will take care of it first." }
                    }
                    div { class: "flex flex-col sm:flex-row gap-2",
                        button {
                            class: "btn-primary btn-lg",
                            disabled: busy() || open_notes > 0,
                            "data-testid": "accept-archive",
                            onclick: move |_| async move {
                                busy.set(true);
                                match api::post::<_, ExecutiveHandoverReport>(&format!("/work-items/{id}/sign-off"), &serde_json::json!({})).await {
                                    Ok(_) => {
                                        app.success("Project accepted and archived at 100%");
                                        sync::load_many(app, &[DataScope::WorkItems, DataScope::Map]).await;
                                        let mut epoch = app.detail_epoch;
                                        epoch += 1;
                                        nav.push(crate::Route::Home {});
                                    }
                                    Err(e) => app.error(e.message()),
                                }
                                busy.set(false);
                            },
                            if busy() { Spinner {} }
                            "Accept & Archive Project (100%)"
                        }
                        button {
                            class: if pin_mode() { "btn-amber btn-lg" } else { "btn-secondary btn-lg" },
                            "data-testid": "add-final-callout",
                            onclick: move |_| pin_mode.set(!pin_mode()),
                            Icon { name: "pin" }
                            "Add Final Call-Out (Hold at 99%)"
                        }
                    }
                    if pin_mode() {
                        if let Some(p) = after_photo.clone() {
                            div { class: "pt-2 rise-in",
                                PhotoAnnotator {
                                    photo: p,
                                    work_item_id: id,
                                    owner_hint: "Tap the spot on the finished photo that needs attention, then describe it.".to_string(),
                                }
                            }
                        } else {
                            p { class: "text-sm text-muted", "There's no completion photo to mark yet." }
                        }
                    }
                }
            } else {
                div { class: "rounded-xl bg-tag-amber-bg px-5 py-4 text-tag-amber-text",
                    p { class: "text-[15px]", "Waiting for the owner to review and sign off. Any final notes they leave will appear on the project photos." }
                }
                if let Some(p) = after_photo.clone() {
                    if open_notes > 0 {
                        PhotoAnnotator { photo: p, work_item_id: id }
                    }
                }
            }
        }
    }
}

#[component]
fn Stat(label: String, value: String) -> Element {
    rsx! {
        div { class: "rounded-lg border border-line px-4 py-3",
            p { class: "text-xs text-muted", "{label}" }
            p { class: "text-[15px] font-medium mt-0.5 tabular", "{value}" }
        }
    }
}
