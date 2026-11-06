//! Notion-style toast notifications (the app never uses `alert()`).

use dioxus::prelude::*;

use crate::components::ui::Icon;
use crate::state::{dismiss_toast, use_app, ToastTone};

#[component]
pub fn ToastHost() -> Element {
    let app = use_app();
    let toasts = app.toasts;
    rsx! {
        div {
            class: "fixed z-[60] bottom-20 sm:bottom-5 right-3 left-3 sm:left-auto sm:right-5 flex flex-col gap-2 items-stretch sm:items-end pointer-events-none",
            "aria-live": "polite",
            for t in toasts.read().iter().cloned() {
                div {
                    key: "{t.id}",
                    class: if t.leaving { "toast leaving pointer-events-auto" } else { "toast pointer-events-auto" },
                    "data-testid": "toast",
                    div { class: "flex items-start gap-3 w-full sm:w-[360px] bg-white rounded-lg shadow-pop px-4 py-3",
                        span {
                            class: match t.tone {
                                ToastTone::Success => "mt-0.5 text-tag-green-text",
                                ToastTone::Warning => "mt-0.5 text-tag-amber-text",
                                ToastTone::Error => "mt-0.5 text-tag-red-text",
                                ToastTone::Info => "mt-0.5 text-muted",
                            },
                            Icon {
                                name: match t.tone {
                                    ToastTone::Success => "check",
                                    ToastTone::Warning => "bell",
                                    ToastTone::Error => "alert",
                                    ToastTone::Info => "info",
                                },
                                size: 16,
                            }
                        }
                        div { class: "flex-1 min-w-0",
                            p { class: "text-sm font-medium text-ink", "{t.title}" }
                            if let Some(body) = t.body.clone() {
                                p { class: "text-sm text-muted mt-0.5 leading-snug", "{body}" }
                            }
                        }
                        button {
                            class: "text-muted hover:text-ink transition-colors -mr-1",
                            "aria-label": "Dismiss",
                            onclick: move |_| {
                                let id = t.id;
                                spawn(async move { dismiss_toast(toasts, id).await });
                            },
                            Icon { name: "x", size: 14 }
                        }
                    }
                }
            }
        }
    }
}
