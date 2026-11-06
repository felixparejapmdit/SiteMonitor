//! SiteMonitor — the estate's two-sided Progressive Web App.
//!
//! * Side A "The Showcase": a calm, document-like portal for the owner.
//! * Side B "The Engine": a fast field tool for the office and site team.

mod api;
mod components;
mod services;
mod state;
mod util;
mod views;

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use shared::*;
use uuid::Uuid;

use components::toast::ToastHost;
use components::ui::{Avatar, Icon};
use services::{live, sync};
use state::{use_app, AppState};
use views::{
    admin_map_studio::MapStudio, agenda_view::AgendaView, archive::ArchiveView, home::HomeView,
    staff::{asset_ops::EquipmentView, decision_pipeline::DecisionsView, manpower_manager::CrewView},
    vip::handover_page::HandoverPage, vip::project_document::{ProjectPage, ZonePage},
};

const TAILWIND: dioxus::prelude::Asset = asset!("/assets/tailwind.css");
const USER_KEY: &str = "sitemonitor.user";

#[rustfmt::skip]
#[derive(Routable, Clone, PartialEq, Debug)]
pub enum Route {
    #[layout(Shell)]
        #[route("/")]
        Home {},
        #[route("/zones/:id")]
        Zone { id: Uuid },
        #[route("/projects/:id")]
        Project { id: Uuid },
        #[route("/handover/:id")]
        Handover { id: Uuid },
        #[route("/agenda")]
        Agenda {},
        #[route("/crew")]
        Crew {},
        #[route("/equipment")]
        Equipment {},
        #[route("/decisions")]
        Decisions {},
        #[route("/map-studio")]
        Studio {},
        #[route("/archive")]
        Archive {},
        #[route("/:..segments")]
        NotFound { segments: Vec<String> },
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let app = use_context_provider(AppState::new);
    use_future(move || bootstrap(app));
    use_future(move || sync::connectivity_loop(app));
    use_future(move || live::live_loop(app));
    rsx! {
        document::Stylesheet { href: TAILWIND }
        Router::<Route> {}
    }
}

/// Loads the people list, restores the last chosen role and warms the cache.
async fn bootstrap(app: AppState) {
    let mut users = app.users;
    let mut current = app.current_user;
    if let Ok(list) = api::get::<Vec<User>>("/users").await {
        let saved = util::storage_get(USER_KEY).and_then(|s| Uuid::parse_str(&s).ok());
        let chosen = saved
            .and_then(|id| list.iter().find(|u| u.id == id).cloned())
            .or_else(|| list.iter().find(|u| u.role == UserRole::VipClient).cloned())
            .or_else(|| list.first().cloned());
        api::set_acting_user(chosen.as_ref().map(|u| u.id));
        users.set(list);
        current.set(chosen);
        sync::load_many(app, &sync::ALL_SCOPES).await;
    }
    // Give the stylesheet a frame to apply before revealing the app.
    TimeoutFuture::new(60).await;
    util::mark_app_ready();
}

pub fn switch_user(app: AppState, user: User) {
    let mut current = app.current_user;
    let mut details = app.details;
    let mut parent = app.active_parent_area;
    let mut layer = app.map_layer;
    api::set_acting_user(Some(user.id));
    util::storage_set(USER_KEY, &user.id.to_string());
    details.write().clear();
    parent.set(None);
    layer.set(state::MapLayer::FrontOfHouse);
    current.set(Some(user));
    sync::reload_all(app);
}

struct NavItem {
    route: Route,
    label: &'static str,
    icon: &'static str,
}

fn nav_for(role: Option<UserRole>) -> Vec<NavItem> {
    match role {
        Some(UserRole::VipClient) | None => vec![
            NavItem { route: Route::Home {}, label: "Overview", icon: "home" },
            NavItem { route: Route::Agenda {}, label: "Agenda", icon: "calendar" },
            NavItem { route: Route::Archive {}, label: "History", icon: "archive" },
        ],
        Some(UserRole::OfficeAdmin) => vec![
            NavItem { route: Route::Home {}, label: "Overview", icon: "home" },
            NavItem { route: Route::Crew {}, label: "Crew", icon: "users" },
            NavItem { route: Route::Equipment {}, label: "Equipment", icon: "truck" },
            NavItem { route: Route::Decisions {}, label: "Issues", icon: "flag" },
            NavItem { route: Route::Agenda {}, label: "Schedule", icon: "calendar" },
            NavItem { route: Route::Studio {}, label: "Map Studio", icon: "pen-tool" },
            NavItem { route: Route::Archive {}, label: "History", icon: "archive" },
        ],
        Some(UserRole::SiteMonitor) => vec![
            NavItem { route: Route::Home {}, label: "Today", icon: "home" },
            NavItem { route: Route::Crew {}, label: "Crew", icon: "users" },
            NavItem { route: Route::Equipment {}, label: "Equipment", icon: "truck" },
            NavItem { route: Route::Decisions {}, label: "Report", icon: "flag" },
            NavItem { route: Route::Agenda {}, label: "Schedule", icon: "calendar" },
        ],
    }
}

fn is_active(current: &Route, target: &Route) -> bool {
    match (current, target) {
        (Route::Home {}, Route::Home {}) => true,
        (Route::Zone { .. } | Route::Project { .. } | Route::Handover { .. }, Route::Home {}) => true,
        (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
    }
}

#[component]
fn Shell() -> Element {
    let app = use_app();
    let route: Route = use_route();
    let role = app.role();
    let nav = nav_for(role);
    let pending = app.pending_offline_mutations.read().len();
    let user = app.current_user.read().clone();
    let draft_count = app
        .decisions
        .read()
        .as_ref()
        .map(|d| d.iter().filter(|d| d.status == DecisionStatus::DraftInAdminQueue).count())
        .unwrap_or(0);

    // Scroll to the top whenever the page changes.
    let mut last_route = use_signal(|| route.clone());
    if *last_route.peek() != route {
        last_route.set(route.clone());
        util::scroll_top();
    }

    rsx! {
        div { class: "min-h-screen flex flex-col bg-page",
            RoleBar {}
            div { class: "flex flex-1 min-h-0",
                // ---------------------------------------------- sidebar
                aside { class: "hidden md:flex w-60 shrink-0 flex-col bg-sidebar border-r border-line sticky top-10 h-[calc(100vh-2.5rem)]",
                    div { class: "px-4 pt-5 pb-4 flex items-center gap-2.5",
                        div { class: "w-7 h-7 rounded-md bg-ink text-white flex items-center justify-center font-serif text-sm", "S" }
                        div {
                            p { class: "text-sm font-semibold leading-tight", "SiteMonitor" }
                            p { class: "text-xs text-muted leading-tight", "Ashford Estate" }
                        }
                    }
                    nav { class: "px-2 space-y-0.5 flex-1", "aria-label": "Main",
                        for item in nav.iter() {
                            Link {
                                key: "{item.label}",
                                to: item.route.clone(),
                                class: if is_active(&route, &item.route) { "nav-link active" } else { "nav-link" },
                                Icon { name: item.icon }
                                span { class: "flex-1", "{item.label}" }
                                if item.label == "Issues" && draft_count > 0 {
                                    span { class: "pill bg-tag-amber-bg text-tag-amber-text", "{draft_count}" }
                                }
                            }
                        }
                    }
                    if let Some(u) = user.clone() {
                        div { class: "p-3 m-2 rounded-lg flex items-center gap-2.5",
                            Avatar { name: u.name.clone() }
                            div { class: "min-w-0",
                                p { class: "text-sm font-medium truncate", "{u.name}" }
                                p { class: "text-xs text-muted", "{u.role.label()}" }
                            }
                        }
                    }
                }
                // ---------------------------------------------- page
                main { class: "flex-1 min-w-0",
                    if app.is_staff() && (pending > 0 || !(app.is_online)()) {
                        OfflineBanner {}
                    }
                    div { class: "max-w-6xl mx-auto px-4 sm:px-8 pt-6 pb-28 md:pb-16",
                        if user.is_some() {
                            Outlet::<Route> {}
                        } else {
                            components::ui::SkeletonPage {}
                        }
                    }
                }
            }
            // ---------------------------------------------- mobile tabs
            nav {
                class: "md:hidden fixed bottom-0 inset-x-0 z-40 bg-white/95 backdrop-blur border-t border-line safe-bottom",
                "aria-label": "Main",
                div { class: "flex h-16",
                    for item in nav.iter().take(5) {
                        Link {
                            key: "{item.label}",
                            to: item.route.clone(),
                            class: if is_active(&route, &item.route) { "tab-link active" } else { "tab-link" },
                            Icon { name: item.icon, size: 20 }
                            span { "{item.label}" }
                        }
                    }
                }
            }
            ToastHost {}
        }
    }
}

/// "Offline Mode — X Updates Queued" (Phase 3 §4).
#[component]
fn OfflineBanner() -> Element {
    let app = use_app();
    let pending = app.pending_offline_mutations.read().len();
    let online = (app.is_online)();
    let syncing = (app.syncing)();
    let text = if !online {
        format!("Offline Mode — {} Queued", util::plural(pending, "Update"))
    } else if syncing {
        format!("Back online — syncing {}…", util::plural(pending, "update"))
    } else {
        format!("{} waiting to sync", util::plural(pending, "update"))
    };
    rsx! {
        div {
            class: "sticky top-10 z-30 bg-tag-amber-bg text-tag-amber-text border-b border-[#f0e2b6] fade-in",
            "data-testid": "offline-badge",
            div { class: "max-w-6xl mx-auto px-4 sm:px-8 h-10 flex items-center gap-2 text-sm font-medium",
                Icon { name: match online { true => "refresh", false => "wifi-off" } }
                span { class: "flex-1", "{text}" }
                if !online {
                    span { class: "hidden sm:inline font-normal opacity-80", "Keep working — everything syncs automatically." }
                } else if pending > 0 && !syncing {
                    button {
                        class: "btn h-7 bg-white/60 hover:bg-white text-tag-amber-text",
                        onclick: move |_| { spawn(async move { sync::flush(app).await; }); },
                        "Sync now"
                    }
                }
            }
        }
    }
}

/// Persistent dev-mode bar to switch between the three roles instantly.
#[component]
fn RoleBar() -> Element {
    let app = use_app();
    let nav = use_navigator();
    let users = app.users.read().clone();
    let current = app.current_user.read().clone();
    let connected = (app.ws_connected)();
    let online = (app.is_online)();
    rsx! {
        div { class: "sticky top-0 z-40 h-10 bg-[#2f2e2a] text-white/90 text-xs",
            div { class: "h-full px-3 sm:px-4 flex items-center gap-3",
                span { class: "hidden lg:inline text-white/50 uppercase tracking-wider text-[10px] font-semibold", "Demo · View as" }
                // Desktop: segmented buttons
                div { class: "hidden sm:flex items-center gap-1 overflow-x-auto no-scrollbar", role: "tablist",
                    for u in users.iter().cloned() {
                        button {
                            key: "{u.id}",
                            role: "tab",
                            "data-testid": "role-{u.role:?}-{u.name}",
                            "aria-selected": current.as_ref().map(|c| c.id == u.id).unwrap_or(false),
                            class: if current.as_ref().map(|c| c.id == u.id).unwrap_or(false) {
                                "h-7 px-2.5 rounded-md bg-white text-ink font-medium transition-colors whitespace-nowrap"
                            } else {
                                "h-7 px-2.5 rounded-md text-white/75 hover:bg-white/10 hover:text-white transition-colors whitespace-nowrap"
                            },
                            onclick: move |_| {
                                switch_user(app, u.clone());
                                nav.push(Route::Home {});
                            },
                            "{u.role.label()} · {u.name.split_whitespace().next().unwrap_or_default()}"
                        }
                    }
                }
                // Mobile: compact select
                select {
                    class: "sm:hidden h-7 rounded-md bg-white/10 text-white px-2 text-xs border-0",
                    "aria-label": "View as",
                    "data-testid": "role-select",
                    onchange: move |e| {
                        if let Ok(id) = Uuid::parse_str(&e.value()) {
                            if let Some(u) = app.users.peek().iter().find(|u| u.id == id).cloned() {
                                switch_user(app, u);
                                nav.push(Route::Home {});
                            }
                        }
                    },
                    for u in users.iter() {
                        option {
                            key: "{u.id}",
                            value: "{u.id}",
                            selected: current.as_ref().map(|c| c.id == u.id).unwrap_or(false),
                            "{u.role.label()} · {u.name}"
                        }
                    }
                }
                div { class: "flex-1" }
                span {
                    class: "flex items-center gap-1.5 text-white/70",
                    title: if connected { "Live updates are on" } else { "Reconnecting to live updates…" },
                    span {
                        class: if !online { "w-1.5 h-1.5 rounded-full bg-[#e5a33d]" } else if connected { "w-1.5 h-1.5 rounded-full bg-[#6fbf73]" } else { "w-1.5 h-1.5 rounded-full bg-white/40 animate-pulse" },
                    }
                    span { class: "hidden sm:inline",
                        if !online { "Offline" } else if connected { "Live" } else { "Connecting…" }
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------- route views

#[component]
fn Home() -> Element {
    rsx! { HomeView {} }
}

#[component]
fn Zone(id: Uuid) -> Element {
    rsx! { ZonePage { key: "{id}", id } }
}

#[component]
fn Project(id: Uuid) -> Element {
    rsx! { ProjectPage { key: "{id}", id } }
}

#[component]
fn Handover(id: Uuid) -> Element {
    rsx! { HandoverPage { key: "{id}", id } }
}

#[component]
fn Agenda() -> Element {
    rsx! { AgendaView {} }
}

#[component]
fn Crew() -> Element {
    rsx! { StaffOnly { CrewView {} } }
}

#[component]
fn Equipment() -> Element {
    rsx! { StaffOnly { EquipmentView {} } }
}

#[component]
fn Decisions() -> Element {
    rsx! { StaffOnly { DecisionsView {} } }
}

#[component]
fn Studio() -> Element {
    let app = use_app();
    if !app.is_admin() {
        return rsx! {
            components::ui::EmptyState {
                icon: "pen-tool",
                title: "Map Studio is for the Office Admin",
                body: "Switch to Office Admin in the bar at the top to draw and edit zones.",
            }
        };
    }
    rsx! { MapStudio {} }
}

#[component]
fn Archive() -> Element {
    rsx! { ArchiveView {} }
}

#[component]
fn NotFound(segments: Vec<String>) -> Element {
    rsx! {
        div { class: "py-16",
            components::ui::EmptyState { icon: "map", title: "This page doesn't exist", body: "It may have moved. Head back to the overview." }
            div { class: "flex justify-center",
                Link { to: Route::Home {}, class: "btn-secondary", "Back to overview" }
            }
        }
    }
}

#[component]
fn StaffOnly(children: Element) -> Element {
    let app = use_app();
    if app.is_staff() {
        return children;
    }
    rsx! {
        div { class: "py-16",
            components::ui::EmptyState {
                icon: "hard-hat",
                title: "This page is for the site team",
                body: "Switch to Office Admin or Site Monitor in the bar at the top to use it.",
            }
            div { class: "flex justify-center",
                Link { to: Route::Home {}, class: "btn-secondary", "Back to overview" }
            }
        }
    }
}
