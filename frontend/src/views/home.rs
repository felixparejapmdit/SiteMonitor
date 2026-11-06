//! The landing page — the owner's calm briefing, or the team's control room.

use chrono::Local;
use dioxus::prelude::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::estate_map::EstateMap;
use crate::components::ui::{EmptyState, Icon, Pill, ProgressBar, SkeletonPage};
use crate::components::vip_action_center::VipActionCenter;
use crate::services::sync;
use crate::state::{use_app, AppState};
use crate::util;
use crate::views::staff::progress_updater::StatusPill;
use crate::views::vip::project_document::{fetch_detail, CapitalCard, MaintenanceLine};
use crate::Route;

#[component]
pub fn HomeView() -> Element {
    let app = use_app();
    if app.work_items.read().is_none() && app.map.read().is_none() {
        return rsx! { SkeletonPage {} };
    }
    if app.is_vip() {
        rsx! { VipHome {} }
    } else {
        rsx! { StaffHome {} }
    }
}

/// Work items inside the map's current drill-down selection.
fn filtered_items(app: AppState) -> (Vec<WorkItem>, Option<String>) {
    let items = app.work_items.read().clone().unwrap_or_default();
    let parent = (app.active_parent_area)();
    match parent {
        None => (items, None),
        Some(p) => {
            let flat = app.flat_areas();
            let zone = util::subtree(&flat, p);
            (items.into_iter().filter(|w| zone.contains(&w.area_id)).collect(), Some(util::area_name(&flat, p)))
        }
    }
}

// ================================================================== owner

#[component]
fn VipHome() -> Element {
    let app = use_app();
    let first_name = app.current_user.read().as_ref().map(|u| u.name.split_whitespace().next().unwrap_or("").to_string()).unwrap_or_default();
    let today = Local::now().format("%A, %B %-d").to_string();
    let (items, scope_name) = filtered_items(app);
    let mut parent = app.active_parent_area;

    // Phase breakdowns for the capital cards.
    use_effect(move || {
        let _epoch = (app.detail_epoch)();
        let ids: Vec<Uuid> = app
            .work_items
            .read()
            .as_ref()
            .map(|w| w.iter().filter(|w| w.status.is_active()).map(|w| w.id).collect())
            .unwrap_or_default();
        for id in ids {
            spawn(async move { fetch_detail(app, id).await });
        }
    });
    let details = app.details.read().clone();

    let capital: Vec<WorkItem> = items.iter().filter(|w| w.stream == WorkStream::CapitalProject && w.status.is_active()).cloned().collect();
    let upkeep: Vec<WorkItem> = items.iter().filter(|w| w.stream != WorkStream::CapitalProject && w.status.is_active()).cloned().collect();
    let all = app.work_items.read().clone().unwrap_or_default();
    let underway = all.iter().filter(|w| w.stream == WorkStream::CapitalProject && w.status.is_active()).count();
    let on_site: u32 = all.iter().filter(|w| w.status.is_active()).map(|w| w.active_worker_count).sum();

    rsx! {
        div { class: "space-y-10 rise-in",
            header { class: "space-y-2",
                p { class: "eyebrow", "{today}" }
                h1 { class: "doc-title", "{util::greeting()}, {first_name}" }
                p { class: "text-[15px] text-muted",
                    "{util::plural(underway, \"project\")} underway · {on_site} people working on the estate today."
                }
            }

            VipActionCenter {}

            section { class: "space-y-3",
                h2 { class: "section-title", "Your estate" }
                EstateMap {}
            }

            section { class: "space-y-8",
                div { class: "flex flex-wrap items-end justify-between gap-2",
                    div {
                        h2 { class: "section-title", "Projects & maintenance" }
                        p { class: "text-sm text-muted mt-0.5",
                            if let Some(n) = scope_name.clone() { "Showing {n} and everything inside it." } else { "Across the whole estate." }
                        }
                    }
                    if scope_name.is_some() {
                        button { class: "btn-ghost h-8", onclick: move |_| parent.set(None), Icon { name: "x" } "Show everything" }
                    }
                }
                div {
                    p { class: "eyebrow mb-3", "Active Capital Developments" }
                    if capital.is_empty() {
                        p { class: "text-sm text-muted", "No construction underway here." }
                    } else {
                        div { class: "grid md:grid-cols-2 gap-3",
                            for w in capital {
                                CapitalCard { key: "{w.id}", item: w.clone(), phases: details.get(&w.id).map(|d| d.phases.clone()).unwrap_or_default() }
                            }
                        }
                    }
                }
                div {
                    p { class: "eyebrow mb-2", "Facility Maintenance & Repairs" }
                    if upkeep.is_empty() {
                        p { class: "text-sm text-muted", "Nothing scheduled here." }
                    } else {
                        div { class: "divide-y divide-line",
                            for w in upkeep {
                                MaintenanceLine { key: "{w.id}", item: w.clone(), detail: details.get(&w.id).cloned() }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ================================================================== staff

#[component]
fn StaffHome() -> Element {
    let app = use_app();
    let is_admin = app.is_admin();
    let user = app.current_user.read().clone();
    let first_name = user.as_ref().map(|u| u.name.split_whitespace().next().unwrap_or("").to_string()).unwrap_or_default();
    let all = app.work_items.read().clone().unwrap_or_default();
    let (items, scope_name) = filtered_items(app);
    let mut parent = app.active_parent_area;
    let notices = app.notifications.read().clone();
    let roster_prompt = notices.iter().find(|n| n.kind == NotificationKind::MorningRosterPrompt).cloned();
    let other_notices: Vec<Notification> = notices.into_iter().filter(|n| n.kind != NotificationKind::MorningRosterPrompt).collect();
    let open_notes = app.open_callouts.read().clone();
    let drafts = app
        .decisions
        .read()
        .as_ref()
        .map(|d| d.iter().filter(|d| d.status == DecisionStatus::DraftInAdminQueue).count())
        .unwrap_or(0);
    let waiting_owner = app
        .decisions
        .read()
        .as_ref()
        .map(|d| d.iter().filter(|d| d.status == DecisionStatus::PublishedPendingClient).count())
        .unwrap_or(0);
    let down = app
        .assets
        .read()
        .as_ref()
        .map(|a| a.assets.iter().filter(|a| a.health != AssetHealth::Operational).count())
        .unwrap_or(0);
    let crew: u32 = all.iter().filter(|w| w.status.is_active()).map(|w| w.active_worker_count).sum();
    let active_jobs = all.iter().filter(|w| w.status.is_active()).count();
    let flagged = app
        .agenda
        .read()
        .as_ref()
        .map(|a| a.items.iter().filter(|i| i.is_flagged_conflict).count())
        .unwrap_or(0);
    let flat = app.flat_areas();
    let today = Local::now().format("%A, %B %-d").to_string();

    let mut active: Vec<WorkItem> = items.into_iter().filter(|w| w.status.is_active()).collect();
    active.sort_by_key(|w| (w.status != WorkStatus::Halted, w.current_target_deadline));

    rsx! {
        div { class: "space-y-8 rise-in",
            header { class: "space-y-1",
                p { class: "eyebrow", "{today}" }
                h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight",
                    if is_admin { "Operations overview" } else { "Good to see you, {first_name}" }
                }
                p { class: "text-[15px] text-muted",
                    if is_admin { "Everything happening across the estate, in one place." } else { "Your jobs for today. Updates save even without signal." }
                }
            }

            if is_admin {
                if let Some(p) = roster_prompt {
                    div { class: "rounded-xl bg-tag-amber-bg border border-[#f0e2b6] px-5 py-4 flex flex-col sm:flex-row sm:items-center gap-3 rise-in", "data-testid": "roster-banner",
                        div { class: "flex-1",
                            p { class: "font-semibold text-tag-amber-text", "{p.title}" }
                            p { class: "text-sm text-tag-amber-text/90 mt-0.5", "{p.body}" }
                        }
                        Link { to: Route::Crew {}, class: "btn-primary", "Allocate crew" Icon { name: "arrow-right" } }
                    }
                }
            }

            // -------------------------------------------- at a glance
            div { class: "grid grid-cols-2 lg:grid-cols-4 gap-3",
                StatTile { label: "Crew on site", value: crew.to_string(), hint: format!("across {}", util::plural(active_jobs, "job")), to: Route::Crew {} }
                StatTile { label: "Owner notes to answer", value: open_notes.len().to_string(), hint: "pinned on photos".to_string(), to: Route::Home {}, alert: !open_notes.is_empty() }
                StatTile {
                    label: if is_admin { "Reports to review" } else { "Waiting for the owner" },
                    value: if is_admin { drafts.to_string() } else { waiting_owner.to_string() },
                    hint: if is_admin { "from the field".to_string() } else { "decisions sent".to_string() },
                    to: Route::Decisions {},
                    alert: is_admin && drafts > 0,
                }
                StatTile { label: "Equipment down", value: down.to_string(), hint: "needs service or broken".to_string(), to: Route::Equipment {}, alert: down > 0 }
            }

            // -------------------------------------------- quick actions (field)
            div { class: "grid grid-cols-2 sm:grid-cols-4 gap-3",
                QuickAction { icon: "truck", label: "Move equipment", to: Route::Equipment {} }
                QuickAction { icon: "wrench", label: "Report breakdown", to: Route::Equipment {} }
                QuickAction { icon: "flag", label: "Report an issue", to: Route::Decisions {} }
                QuickAction { icon: "users", label: "Move workers", to: Route::Crew {} }
            }

            if flagged > 0 {
                Link { to: Route::Agenda {}, class: "flex items-center gap-3 rounded-xl bg-tag-red-bg text-tag-red-text px-5 py-4 hover:opacity-90 transition-opacity", "data-testid": "quiet-day-alert",
                    Icon { name: "moon", size: 18 }
                    p { class: "flex-1 text-[15px] font-medium", "{util::plural(flagged, \"task\")} must move for the owner's Quiet Day" }
                    span { class: "text-sm", "Open schedule" }
                    Icon { name: "arrow-right" }
                }
            }

            // -------------------------------------------- owner notes
            if !open_notes.is_empty() {
                section { "data-testid": "owner-notes",
                    h2 { class: "section-title mb-3", "Owner notes to answer" }
                    div { class: "space-y-2",
                        for n in open_notes {
                            Link { key: "{n.callout.id}", to: Route::Project { id: n.photo.work_item_id }, class: "card card-hover p-3 flex items-center gap-3",
                                div { class: "relative w-24 aspect-video rounded-md overflow-hidden bg-hover shrink-0",
                                    img { src: "{n.photo.image_url}", alt: "", class: "w-full h-full object-cover", loading: "lazy" }
                                    span { class: "absolute w-2.5 h-2.5 -ml-[5px] -mt-[5px] rounded-full bg-pin-amber ring-2 ring-white", style: "left:{n.callout.x_percent}%;top:{n.callout.y_percent}%" }
                                }
                                div { class: "flex-1 min-w-0",
                                    p { class: "text-[15px] line-clamp-2", "\u{201c}{n.callout.client_comment}\u{201d}" }
                                    p { class: "text-xs text-muted mt-0.5", "{n.work_item_title} · {util::relative(n.callout.created_at)}" }
                                }
                                Pill { tone: n.callout.status.tone(), text: n.callout.status.label().to_string() }
                            }
                        }
                    }
                }
            }

            // -------------------------------------------- notices
            if !other_notices.is_empty() {
                section {
                    h2 { class: "section-title mb-3", "Notices" }
                    div { class: "space-y-2",
                        for n in other_notices.into_iter().take(6) {
                            NoticeRow { key: "{n.id}", notice: n }
                        }
                    }
                }
            }

            // -------------------------------------------- map + jobs
            section { class: "space-y-3",
                h2 { class: "section-title", "Estate map" }
                EstateMap {}
            }

            section { class: "space-y-3",
                div { class: "flex flex-wrap items-end justify-between gap-2",
                    div {
                        h2 { class: "section-title", "Active jobs" }
                        p { class: "text-sm text-muted mt-0.5",
                            if let Some(n) = scope_name.clone() { "In {n}" } else { "Across the whole estate" }
                        }
                    }
                    if scope_name.is_some() {
                        button { class: "btn-ghost h-8", onclick: move |_| parent.set(None), Icon { name: "x" } "Show everything" }
                    }
                }
                if active.is_empty() {
                    EmptyState { icon: "check", title: "No active jobs here" }
                } else {
                    div { class: "card divide-y divide-line overflow-hidden",
                        for w in active {
                            Link { key: "{w.id}", to: Route::Project { id: w.id }, class: "flex items-center gap-3 px-4 py-3 hover:bg-[#fbfbfa] transition-colors", "data-testid": "job-row",
                                div { class: "flex-1 min-w-0",
                                    div { class: "flex flex-wrap items-center gap-2",
                                        p { class: "text-[15px] font-medium truncate", "{w.title}" }
                                        StatusPill { status: w.status }
                                    }
                                    p { class: "text-xs text-muted mt-0.5 truncate",
                                        "{w.stream.label()} · {util::area_path_label(&flat, w.area_id)} · due {util::day(w.current_target_deadline)}"
                                    }
                                }
                                if w.stream == WorkStream::CapitalProject {
                                    div { class: "hidden sm:flex items-center gap-2 w-40",
                                        div { class: "flex-1", ProgressBar { value: w.calculated_progress } }
                                        span { class: "text-xs text-muted tabular w-8 text-right", "{w.calculated_progress}%" }
                                    }
                                } else {
                                    span { class: "hidden sm:block w-40 text-xs text-muted text-right",
                                        if w.stream == WorkStream::RoutineMaintenance { "Recurring upkeep" } else { "Repair" }
                                    }
                                }
                                span { class: "pill bg-hover text-ink tabular", Icon { name: "users", size: 12 } "{w.active_worker_count}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatTile(#[props(into)] label: String, value: String, hint: String, to: Route, #[props(default)] alert: bool) -> Element {
    rsx! {
        Link { to, class: "card card-hover p-4 block",
            p { class: "text-xs text-muted", "{label}" }
            p { class: if alert { "text-2xl font-semibold mt-1 tabular text-tag-amber-text" } else { "text-2xl font-semibold mt-1 tabular" }, "{value}" }
            p { class: "text-xs text-muted mt-0.5", "{hint}" }
        }
    }
}

#[component]
fn QuickAction(icon: &'static str, label: &'static str, to: Route) -> Element {
    rsx! {
        Link { to, class: "card card-hover h-20 flex flex-col items-center justify-center gap-1.5 text-sm font-medium",
            Icon { name: icon, size: 20 }
            "{label}"
        }
    }
}

#[component]
fn NoticeRow(notice: Notification) -> Element {
    let app = use_app();
    let id = notice.id;
    let target = notice.work_item_id;
    rsx! {
        div { class: "card px-4 py-3 flex items-start gap-3 fade-in", "data-testid": "notice-row",
            span { class: "mt-0.5 text-muted", Icon { name: match notice.kind { NotificationKind::DeadlineWarning => "clock", _ => "bell" } } }
            div { class: "flex-1 min-w-0",
                p { class: "text-[15px] font-medium", "{notice.title}" }
                p { class: "text-sm text-muted mt-0.5", "{notice.body}" }
                if !notice.lines.is_empty() {
                    p { class: "text-xs text-muted mt-1", "{notice.lines.join(\" · \")}" }
                }
            }
            if let Some(w) = target {
                Link { to: Route::Project { id: w }, class: "btn-ghost h-8", "Open" }
            }
            button {
                class: "btn-icon btn-ghost h-8",
                "aria-label": "Dismiss notice",
                onclick: move |_| async move {
                    if api::post::<_, Notification>(&format!("/notifications/{id}/dismiss"), &serde_json::json!({})).await.is_ok() {
                        sync::load_many(app, &[DataScope::Notifications]).await;
                    }
                },
                Icon { name: "x", size: 14 }
            }
        }
    }
}
