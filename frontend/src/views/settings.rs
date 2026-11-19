//! Settings: when the automatic jobs run, and the wording
//! of the pick lists people choose from.

use dioxus::prelude::*;
use shared::*;

use crate::api;
use crate::components::ui::{Icon, Modal, Spinner};
use crate::services::sync;
use crate::state::use_app;
use crate::Route;

#[component]
pub fn SettingsView() -> Element {
    let app = use_app();
    let mut draft = use_signal(|| app.settings.peek().clone());
    let mut busy = use_signal(|| false);
    let mut confirm = use_signal(|| None::<Reset>);
    let saved = app.settings.read().clone();
    let d = draft();
    let dirty = d != saved;

    let save = move |_| async move {
        busy.set(true);
        match api::put::<_, EstateSettings>("/settings", &draft()).await {
            Ok(v) => {
                let mut settings = app.settings;
                settings.set(v.clone());
                draft.set(v);
                app.success("Settings saved");
            }
            Err(e) => app.error(e.message()),
        }
        busy.set(false);
    };

    rsx! {
        div { class: "space-y-6 rise-in pb-4",
            header { class: "space-y-1",
                h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Settings" }
                p { class: "text-[15px] text-muted", "When the automatic jobs run, and the choices people pick from." }
            }

            Section { title: "Automatic jobs", hint: "Times are in the estate's local time. Changes start from the next minute.",
                div { class: "grid sm:grid-cols-2 gap-4",
                    TimeField { id: "t-reset", label: "Crew reset", hint: "Clears yesterday's crew numbers.", value: d.roster_reset_time.clone(),
                        on_change: move |v| draft.write().roster_reset_time = v }
                    TimeField { id: "t-prompt", label: "Crew reminder", hint: "Asks the office to set today's crew.", value: d.roster_prompt_time.clone(),
                        on_change: move |v| draft.write().roster_prompt_time = v }
                    TimeField { id: "t-deadline", label: "Deadline check", hint: "Warns the team 7 days and 48 hours before a target date.", value: d.deadline_check_time.clone(),
                        on_change: move |v| draft.write().deadline_check_time = v }
                    TimeField { id: "t-digest", label: "Owner's daily summary", hint: "Sends the owner one calm round-up of the day.", value: d.digest_time.clone(),
                        on_change: move |v| draft.write().digest_time = v }
                }
            }

            Section { title: "Reasons for moving workers", hint: "Offered as quick choices on the Crew page. People can still type their own.",
                div { class: "space-y-2 max-w-xl",
                    for (i, r) in d.crew_move_reasons.iter().enumerate() {
                        div { key: "{i}", class: "flex items-center gap-2",
                            input { class: "field", "data-testid": "reason-input", value: "{r}", "aria-label": "Reason {i + 1}",
                                oninput: move |e| draft.write().crew_move_reasons[i] = e.value() }
                            button { class: "btn-icon btn-ghost shrink-0", "aria-label": "Remove reason", onclick: move |_| { draft.write().crew_move_reasons.remove(i); },
                                Icon { name: "trash" }
                            }
                        }
                    }
                    button { class: "btn-secondary", "data-testid": "add-reason", disabled: d.crew_move_reasons.len() >= 12,
                        onclick: move |_| draft.write().crew_move_reasons.push(String::new()),
                        Icon { name: "plus" }
                        "Add reason"
                    }
                }
            }

            Section { title: "Issue categories", hint: "Rename the categories used when reporting issues. Leave blank to keep the built-in name.",
                div { class: "divide-y divide-line border-y border-line max-w-xl",
                    for c in AlertCategory::ALL {
                        div { key: "{c:?}", class: "grid grid-cols-[minmax(8rem,12rem)_1fr] items-center gap-3 py-2",
                            span { class: "text-sm text-muted", "{c.label()}" }
                            input { class: "field", placeholder: "{c.label()}", "aria-label": "Name for {c.label()}",
                                value: "{d.category_labels.get(&c).cloned().unwrap_or_default()}",
                                oninput: move |e| { draft.write().category_labels.insert(c, e.value()); } }
                        }
                    }
                }
            }

            Section { title: "Schedule event types", hint: "Rename the event types on the schedule. Leave blank to keep the built-in name.",
                div { class: "divide-y divide-line border-y border-line max-w-xl",
                    for k in AgendaEventType::ALL {
                        div { key: "{k:?}", class: "grid grid-cols-[minmax(8rem,12rem)_1fr] items-center gap-3 py-2",
                            span { class: "text-sm text-muted", "{k.label()}" }
                            input { class: "field", placeholder: "{k.label()}", "aria-label": "Name for {k.label()}",
                                value: "{d.event_type_labels.get(&k).cloned().unwrap_or_default()}",
                                oninput: move |e| { draft.write().event_type_labels.insert(k, e.value()); } }
                        }
                    }
                }
            }

            Section { title: "Estate data", hint: "Try SiteMonitor with a ready-made estate, or wipe it and set up your own. People, their sign-ins and these settings are always kept.",
                div { class: "divide-y divide-line border-y border-line", "data-testid": "estate-data",
                    div { class: "flex flex-wrap items-center gap-x-6 gap-y-3 py-3",
                        div { class: "flex-1 min-w-[16rem]",
                            p { class: "text-sm font-medium", "Load sample data" }
                            p { class: "text-sm text-muted", "Replaces the map, zones, projects, reports, photos, equipment and schedule with a sample estate, worked by your current team." }
                        }
                        button { class: "btn-secondary", "data-testid": "load-sample", onclick: move |_| confirm.set(Some(Reset::Sample)),
                            Icon { name: "sparkles" }
                            "Load sample data"
                        }
                    }
                    div { class: "flex flex-wrap items-center gap-x-6 gap-y-3 py-3",
                        div { class: "flex-1 min-w-[16rem]",
                            p { class: "text-sm font-medium", "Clear data" }
                            p { class: "text-sm text-muted", "Removes the map picture and every zone, project, report, photo, piece of equipment and schedule entry, so you can start fresh." }
                        }
                        button { class: "btn-secondary text-tag-red-text", "data-testid": "clear-data", onclick: move |_| confirm.set(Some(Reset::Clear)),
                            Icon { name: "trash" }
                            "Clear data"
                        }
                    }
                }
            }
            if let Some(kind) = confirm() {
                ConfirmReset { kind, on_close: move |_| confirm.set(None) }
            }

            div { class: "sticky bottom-16 md:bottom-0 z-20 -mx-4 sm:-mx-6 lg:-mx-10 px-4 sm:px-6 lg:px-10 py-3 bg-page/95 backdrop-blur border-t border-line flex items-center justify-end gap-2",
                if dirty { span { class: "text-sm text-muted mr-auto", "You have unsaved changes." } }
                button { class: "btn-ghost h-10 px-4", disabled: !dirty || busy(), onclick: move |_| draft.set(app.settings.peek().clone()), "Undo changes" }
                button { class: "btn-primary h-10 px-4", "data-testid": "save-settings", disabled: !dirty || busy(), onclick: save,
                    if busy() { Spinner {} }
                    "Save settings"
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Reset {
    Sample,
    Clear,
}

/// Both actions replace the whole estate, so the office types a word to confirm.
#[component]
fn ConfirmReset(kind: Reset, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let nav = use_navigator();
    let mut typed = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let (title, body, word, action, path, done) = match kind {
        Reset::Sample => (
            "Load sample data?",
            "Everything in the estate (the map, zones, projects, reports, photos, equipment and schedule) is replaced by the sample estate. People, their sign-ins and settings stay.",
            "SAMPLE",
            "Replace with sample data",
            "/settings/sample-data",
            "Sample data loaded",
        ),
        Reset::Clear => (
            "Clear all estate data?",
            "This removes the map picture and every zone, project, report, photo, piece of equipment and schedule entry. People, their sign-ins and settings stay.",
            "CLEAR",
            "Clear everything",
            "/settings/clear-data",
            "All estate data cleared",
        ),
    };
    let ready = typed().trim().eq_ignore_ascii_case(word) && !busy();
    let run = move |_| async move {
        busy.set(true);
        match api::send_raw("POST", path, Some(&serde_json::json!({}))).await {
            Ok(_) => {
                let mut parent = app.active_parent_area;
                parent.set(None);
                sync::load_many(app, &sync::ALL_SCOPES).await;
                app.success(done);
                on_close.call(());
                nav.push(Route::Home {});
            }
            Err(e) => {
                busy.set(false);
                app.error(e.message());
            }
        }
    };
    rsx! {
        Modal { title, on_close: move |_| if !busy() { on_close.call(()) },
            div { class: "space-y-4", "data-testid": "reset-dialog",
                p { class: "text-sm", "{body}" }
                p { class: "text-sm font-medium text-tag-red-text", "This can't be undone." }
                div {
                    label { class: "label", r#for: "reset-word", "Type {word} to confirm" }
                    input { id: "reset-word", class: "field", "data-testid": "reset-word", autocomplete: "off", spellcheck: "false", autofocus: true,
                        value: "{typed}", oninput: move |e| typed.set(e.value()) }
                }
                div { class: "modal-actions",
                    button { class: "btn-ghost", disabled: busy(), onclick: move |_| on_close.call(()), "Cancel" }
                    button { class: if kind == Reset::Clear { "btn-danger" } else { "btn-primary" }, "data-testid": "reset-confirm", disabled: !ready, onclick: run,
                        if busy() { Spinner {} }
                        "{action}"
                    }
                }
            }
        }
    }
}

#[component]
fn Section(#[props(into)] title: String, #[props(default, into)] hint: String, children: Element) -> Element {
    rsx! {
        section { class: "card p-5 space-y-3",
            div {
                h2 { class: "section-title", "{title}" }
                if !hint.is_empty() { p { class: "text-sm text-muted mt-0.5", "{hint}" } }
            }
            {children}
        }
    }
}

#[component]
fn TimeField(id: &'static str, label: &'static str, hint: &'static str, value: String, on_change: EventHandler<String>) -> Element {
    rsx! {
        div {
            label { class: "label", r#for: id, "{label}" }
            input { id, r#type: "time", class: "field", "data-testid": id, value: "{value}", oninput: move |e| on_change.call(e.value()) }
            p { class: "hint", "{hint}" }
        }
    }
}
