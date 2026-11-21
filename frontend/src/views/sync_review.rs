//! Service 5's "Multi-Master Sync & Differential Review": before a backup is
//! merged in, show where it came from, what it adds (new people, attendance,
//! newer sections) and each true collision with a choice — keep local, accept
//! incoming, or smart merge — plus the pre-merge safety checkpoint.

use std::collections::HashMap;

use dioxus::prelude::*;
use serde_json::Value;

use crate::components::ui::{Icon, Modal};
use crate::views::workforce_ops::s;

/// Every collision starts as "accept incoming", as in Service 5.
pub fn default_choices(analysis: &Value) -> HashMap<String, String> {
    analysis["conflicts"].as_array().into_iter().flatten().map(|c| (s(c, "id"), "accept_incoming".to_string())).collect()
}

/// One side of a collision in a line (Service 5's summary per section).
fn summary(c: &Value, side: &str) -> String {
    let v = &c[side];
    let kind = if s(c, "type") == "attendance" { "attendance" } else { c["section"].as_str().unwrap_or("") };
    match kind {
        "healthAndClearance" => format!("Status: {}", or_none(s(v, "clearanceStatus"))),
        "attendance" => format!("Status: {}", or_none(if s(v, "status").is_empty() { s(v, "newStatus") } else { s(v, "status") })),
        "assignment" => format!("Station: {} · Trade: {}", or_none(s(v, "area")), or_none(s(v, "trade"))),
        _ => format!("Phone: {}", or_none(if s(v, "contact").is_empty() { s(v, "phone") } else { s(v, "contact") })),
    }
}

fn or_none(v: String) -> String {
    if v.is_empty() { "None".into() } else { v }
}

#[component]
pub fn SyncReview(
    analysis: Value,
    on_apply: EventHandler<HashMap<String, String>>,
    on_cancel: EventHandler<()>,
    /// Download the pre-merge checkpoint (an encrypted backup of what is here now).
    on_checkpoint: EventHandler<()>,
) -> Element {
    let mut choices = use_signal(|| default_choices(&analysis));
    let sum = &analysis["summary"];
    let n = |k: &str| sum[k].as_u64().unwrap_or(0);
    let conflicts: Vec<Value> = analysis["conflicts"].as_array().cloned().unwrap_or_default();
    let ids: Vec<String> = conflicts.iter().map(|c| s(c, "id")).collect();
    let ids2 = ids.clone();
    rsx! {
        div { class: "contents", "data-testid": "sync-review",
            Modal { title: "Multi-master sync & differential review", wide: true, on_close: move |_| on_cancel.call(()),
                div { class: "space-y-4",
                    p { class: "text-sm text-muted", "Review incoming changes, check the merge is non-destructive, and resolve collisions." }
                    div { class: "card p-3 flex flex-wrap justify-between gap-2 text-sm",
                        div { p { class: "text-xs text-muted", "Incoming station" } p { class: "font-medium", "data-testid": "sync-source", {s(&analysis, "sourceDeviceId")} } }
                        div { class: "text-right", p { class: "text-xs text-muted", "Export time" } p { class: "font-medium", {if s(&analysis, "exportTimestamp").is_empty() { "Not recorded".to_string() } else { s(&analysis, "exportTimestamp") }} } }
                    }
                    div { class: "grid grid-cols-2 sm:grid-cols-4 gap-2", "data-testid": "sync-stats",
                        for (label, value, testid) in [("New workers", format!("+{}", n("newWorkersCount")), "sync-new"), ("Attendance added", format!("+{}", n("newAttendanceCount")), "sync-att"), ("Field updates", n("sectionUpdatesCount").to_string(), "sync-updates"), ("True collisions", n("conflictsCount").to_string(), "sync-conflicts")] {
                            div { key: "{label}", class: if testid == "sync-conflicts" && n("conflictsCount") > 0 { "card p-3 border-tag-red-text" } else { "card p-3" },
                                p { class: "text-xs text-muted", "{label}" }
                                p { class: "text-lg font-semibold tabular", "data-testid": "{testid}", "{value}" }
                            }
                        }
                    }
                    if conflicts.is_empty() {
                        p { class: "rounded-md bg-tag-green-bg text-tag-green-text text-sm px-3 py-2", "data-testid": "sync-clean",
                            "Clean convergence: every record is new, additive, or resolved by section-level last-write-wins. Nothing here will be wiped or overwritten by older data." }
                    } else {
                        div { class: "space-y-2", "data-testid": "sync-conflict-list",
                            div { class: "flex flex-wrap items-center gap-2",
                                p { class: "font-semibold text-sm flex-1", "Collisions needing a decision ({conflicts.len()})" }
                                button { class: "btn-secondary h-8", "data-testid": "sync-all-local", onclick: move |_| { let mut c = choices.write(); for id in &ids { c.insert(id.clone(), "keep_local".into()); } }, "Keep all local" }
                                button { class: "btn-secondary h-8", "data-testid": "sync-all-incoming", onclick: move |_| { let mut c = choices.write(); for id in &ids2 { c.insert(id.clone(), "accept_incoming".into()); } }, "Accept all incoming" }
                            }
                            for c in conflicts.iter() {
                                {
                                    let id = s(c, "id");
                                    let attendance = s(c, "type") == "attendance";
                                    let safety = s(c, "section") == "healthAndClearance" && matches!(s(&c["incoming"], "clearanceStatus").as_str(), "Medical Hold" | "Restricted");
                                    rsx! {
                                        div { key: "{id}", class: "card p-3 space-y-2", "data-testid": "sync-conflict",
                                            p { class: "text-sm font-semibold", {s(c, "workerName")} span { class: "text-muted font-normal", " · {s(c, \"sectionLabel\")}" } }
                                            div { class: "grid sm:grid-cols-2 gap-2 text-sm",
                                                div { class: "rounded border border-line p-2", p { class: "text-xs text-muted", "Current local state" } p { {summary(c, "local")} } p { class: "text-xs text-muted", {s(c, "localTime")} } }
                                                div { class: "rounded border border-line p-2", p { class: "text-xs text-accent-ink", "Incoming file state" } p { {summary(c, "incoming")} } p { class: "text-xs text-muted", {s(c, "incomingTime")} } }
                                            }
                                            div { class: "flex flex-wrap gap-4 text-sm", role: "radiogroup", "aria-label": "Resolution",
                                                for (v, l) in [("keep_local", "Keep local"), ("accept_incoming", if safety { "Accept incoming (safety recommendation)" } else { "Accept incoming" }), ("smart_merge", "Smart merge")].into_iter().filter(|(v, _)| !(attendance && *v == "smart_merge")) {
                                                    {
                                                        let id = id.clone();
                                                        let on = choices().get(&id).map(String::as_str) == Some(v);
                                                        rsx! {
                                                            label { key: "{v}", class: "flex items-center gap-1.5",
                                                                input { r#type: "radio", name: "res_{id}", class: "accent-[#064e3b]", "data-testid": "sync-res-{v}", checked: on,
                                                                    onchange: move |_| { choices.write().insert(id.clone(), v.to_string()); } }
                                                                "{l}"
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
                    div { class: "flex flex-wrap items-center justify-between gap-2 rounded-md bg-hover px-3 py-2 text-sm",
                        span { Icon { name: "shield", size: 14 } " Safety checkpoint: what is here now is snapshotted automatically before the merge (restore it from Daily Snapshot)." }
                        button { class: "btn-secondary h-8", "data-testid": "sync-checkpoint", onclick: move |_| on_checkpoint.call(()), "Download checkpoint" }
                    }
                    div { class: "modal-actions",
                        button { class: "btn-ghost", "data-testid": "sync-cancel", onclick: move |_| on_cancel.call(()), "Cancel" }
                        button { class: "btn-primary", "data-testid": "sync-apply", onclick: move |_| on_apply.call(choices()), "Apply differential merge" }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collisions_default_to_accept_incoming() {
        let a = serde_json::json!({ "conflicts": [{ "id": "conflict_pi_w1" }, { "id": "conflict_att_w1_2026-11-01_Day" }] });
        let c = default_choices(&a);
        assert_eq!(c.len(), 2);
        assert!(c.values().all(|v| v == "accept_incoming"));
    }
}
