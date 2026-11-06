//! History & search: completed projects and archived (demolished or
//! repurposed) locations, with every linked record still searchable.

use dioxus::prelude::*;
use shared::*;

use crate::components::ui::{EmptyState, Icon, Pill};
use crate::state::use_app;
use crate::util;
use crate::Route;

#[component]
pub fn ArchiveView() -> Element {
    let app = use_app();
    let mut query = use_signal(String::new);
    let flat = app.flat_areas();
    let items = app.work_items.read().clone().unwrap_or_default();
    let q = query().trim().to_lowercase();

    let archived_zone_ids: Vec<uuid::Uuid> = flat
        .iter()
        .filter(|a| a.area.is_archived)
        .flat_map(|a| util::subtree(&flat, a.area.id))
        .collect();
    let matches = |w: &WorkItem| {
        q.is_empty()
            || w.title.to_lowercase().contains(&q)
            || w.description.to_lowercase().contains(&q)
            || util::area_path_label(&flat, w.area_id).to_lowercase().contains(&q)
    };
    let mut results: Vec<WorkItem> = items
        .iter()
        .filter(|w| if q.is_empty() { w.status == WorkStatus::CompletedAndArchived || archived_zone_ids.contains(&w.area_id) } else { true })
        .filter(|w| matches(w))
        .cloned()
        .collect();
    results.sort_by(|a, b| b.current_target_deadline.cmp(&a.current_target_deadline));
    let archived_zones: Vec<util::FlatArea> = flat.iter().filter(|a| a.area.is_archived).cloned().collect();

    rsx! {
        div { class: "max-w-doc mx-auto space-y-8 rise-in",
            header {
                h1 { class: "doc-title", "History" }
                p { class: "text-[15px] text-muted mt-1", "Finished projects and places that have been demolished or repurposed — nothing is ever lost." }
            }
            div { class: "relative",
                span { class: "absolute left-3 top-1/2 -translate-y-1/2 text-muted", Icon { name: "search" } }
                input { class: "field pl-9 h-11", "data-testid": "history-search", placeholder: "Search every project, past and present", value: "{query}", oninput: move |e| query.set(e.value()) }
            }

            if q.is_empty() && !archived_zones.is_empty() {
                section {
                    h2 { class: "section-title mb-3", "Archived Locations" }
                    div { class: "flex flex-wrap gap-2",
                        for a in archived_zones {
                            Link { key: "{a.area.id}", to: Route::Zone { id: a.area.id }, class: "pill h-8 px-3 bg-tag-gray-bg text-tag-gray-text hover:bg-[#e8e8e5] transition-colors",
                                Icon { name: "archive", size: 12 }
                                "{a.area.name}"
                            }
                        }
                    }
                }
            }

            section {
                h2 { class: "section-title mb-3", if q.is_empty() { "Completed & archived projects" } else { "Results" } }
                if results.is_empty() {
                    EmptyState { icon: "search", title: "Nothing found", body: "Try another word, like a place or a type of work." }
                } else {
                    ul { class: "divide-y divide-line border-y border-line", "data-testid": "history-results",
                        for w in results {
                            {
                                let in_archived = archived_zone_ids.contains(&w.area_id);
                                rsx! {
                                    li { key: "{w.id}",
                                        Link { to: Route::Project { id: w.id }, class: "flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-3 py-3 px-2 -mx-2 rounded-md hover:bg-hover transition-colors",
                                            div { class: "flex-1 min-w-0",
                                                p { class: "text-[15px] font-medium", "{w.title}" }
                                                p { class: "text-xs text-muted", "{util::area_path_label(&flat, w.area_id)} · {w.stream.label()} · {util::day(w.current_target_deadline)}" }
                                            }
                                            div { class: "flex gap-1.5",
                                                if in_archived {
                                                    Pill { tone: Tone::Gray, text: "Archived Locations" }
                                                }
                                                Pill { tone: w.status.tone(), text: w.status.label() }
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
