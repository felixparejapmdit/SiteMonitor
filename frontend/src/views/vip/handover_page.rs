//! Route wrapper for the Executive Handover Report.

use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::handover_report::HandoverReport;
use crate::components::ui::{EmptyState, Icon, SkeletonPage};
use crate::views::vip::project_document::use_detail;
use crate::Route;

#[component]
pub fn HandoverPage(id: Uuid) -> Element {
    let Some(detail) = use_detail(id) else {
        return rsx! { div { class: "max-w-doc mx-auto", SkeletonPage {} } };
    };
    rsx! {
        div { class: "rise-in",
            div { class: "max-w-doc mx-auto mb-6",
                Link { to: Route::Project { id }, class: "inline-flex items-center gap-1 text-sm text-muted hover:text-ink transition-colors",
                    Icon { name: "chevron-left", size: 14 }
                    "Back to project"
                }
            }
            match detail.handover.clone() {
                Some(report) => rsx! { HandoverReport { detail: detail.clone(), report } },
                None => rsx! {
                    EmptyState {
                        icon: "archive",
                        title: "The handover report isn't ready yet",
                        body: "It is created automatically the moment the final phase is completed.",
                    }
                },
            }
        }
    }
}
