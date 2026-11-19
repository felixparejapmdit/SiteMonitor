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
use std::collections::{HashMap, HashSet};

use components::ui::{current_tab, Avatar, Icon, PAGE_TABS};
use services::{live, sync};
use state::{use_app, AppState};
use views::{
    access::AccessView, auth::{AccountActions, ForcedPasswordChange, LoginPage},
    admin_map_studio::MapStudio, agenda_view::AgendaView, archive::ArchiveView, home::HomeView, people::PeopleView, settings::SettingsView,
    staff::{asset_ops::EquipmentView, decision_pipeline::DecisionsView, manpower_manager::CrewView},
    vip::handover_page::HandoverPage, vip::project_document::{ProjectPage, ZonePage},
};

const TAILWIND: dioxus::prelude::Asset = asset!("/assets/tailwind.css");

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
        #[route("/people")]
        People {},
        #[route("/personnel")]
        Personnel {},
        #[route("/workforce")]
        Workforce {},
        #[route("/operations")]
        Operations {},
        #[route("/clinic")]
        Clinic {},
        #[route("/access")]
        Access {},
        #[route("/settings")]
        Settings {},
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
    // Keeps "2 min ago" moving everywhere it's shown.
    use_future(|| async {
        loop {
            TimeoutFuture::new(30_000).await;
            *components::live_feed::TICK.write() += 1;
        }
    });
    let ready = (app.auth_ready)();
    let user = app.current_user.read().clone();
    rsx! {
        document::Stylesheet { href: TAILWIND }
        if !ready {
            div {}
        } else if let Some(u) = user {
            if u.must_change_password {
                ForcedPasswordChange {}
            } else {
                Router::<Route> {}
            }
        } else {
            LoginPage {}
        }
        ToastHost {}
    }
}

/// Asks the server who is signed in (the session cookie), then warms the cache.
async fn bootstrap(app: AppState) {
    let current = app.current_user;
    let mut ready = app.auth_ready;
    // A 401 anywhere means the session ended: drop back to the sign-in page.
    api::on_signed_out(move || {
        let mut c = current;
        if c.peek().is_some() {
            c.set(None);
        }
    });
    if let Ok(Some(me)) = api::get::<Option<User>>("/auth/me").await {
        start_session(app, me).await;
    }
    ready.set(true);
    // Give the stylesheet a frame to apply before revealing the app.
    TimeoutFuture::new(60).await;
    util::mark_app_ready();
}

/// Called after sign-in (or when an existing session is found). Loads the
/// estate *before* setting the user: setting it swaps the sign-in screen for
/// the app, which would cancel this task if it were still running.
pub async fn start_session(app: AppState, user: User) {
    let mut current = app.current_user;
    let mut details = app.details;
    let mut parent = app.active_parent_area;
    details.write().clear();
    parent.set(None);
    if !user.must_change_password {
        sync::load_many(app, &sync::ALL_SCOPES).await;
    }
    current.set(Some(user));
}

/// Ends the session on the server and reloads to a clean sign-in page.
pub async fn sign_out() {
    let _ = api::post::<_, serde_json::Value>("/auth/logout", &()).await;
    let _ = util::window().location().set_href("/");
}

#[derive(Clone, PartialEq)]
struct NavItem {
    route: Route,
    label: &'static str,
    icon: &'static str,
    /// Sidebar section heading ("" = none).
    group: &'static str,
    hue: Hue,
    /// Tabbed pages: the key of their tabs (`views::page_tabs`), shown as sub-items.
    page: &'static str,
}

/// Each menu item has its own colour, so it's recognisable at a glance.
#[derive(Clone, Copy, PartialEq)]
enum Hue {
    Emerald,
    Sky,
    Amber,
    Rose,
    Violet,
    Teal,
    Stone,
    Blue,
    Indigo,
    Slate,
}

impl Hue {
    /// The icon colour (phone tab bar).
    fn ink(self) -> &'static str {
        match self {
            Hue::Emerald => "text-emerald-600",
            Hue::Sky => "text-sky-600",
            Hue::Amber => "text-amber-600",
            Hue::Rose => "text-rose-600",
            Hue::Violet => "text-violet-600",
            Hue::Teal => "text-teal-600",
            Hue::Stone => "text-stone-500",
            Hue::Blue => "text-blue-600",
            Hue::Indigo => "text-indigo-600",
            Hue::Slate => "text-slate-500",
        }
    }
}

const SIDEBAR_KEY: &str = "sm-sidebar";
const NAV_CLOSED_KEY: &str = "sm-nav-closed";

/// Collapsed sidebar categories (remembered on this device), shared by the
/// desktop sidebar and the phone drawer.
static NAV_CLOSED: GlobalSignal<HashSet<String>> =
    Signal::global(|| util::storage_get(NAV_CLOSED_KEY).unwrap_or_default().split('|').filter(|s| !s.is_empty()).map(String::from).collect());
/// Tabbed pages whose tabs were shown or hidden by hand (otherwise: shown while on that page).
static TABS_OPEN: GlobalSignal<HashMap<&'static str, bool>> = Signal::global(HashMap::new);

fn nav_groups(nav: &[NavItem]) -> Vec<(&'static str, Vec<NavItem>)> {
    let mut groups: Vec<(&'static str, Vec<NavItem>)> = vec![];
    for it in nav.iter().cloned() {
        match groups.last_mut() {
            Some((g, items)) if *g == it.group => items.push(it),
            _ => groups.push((it.group, vec![it])),
        }
    }
    groups
}

/// The menu: each role's everyday pages, plus admin pages for whoever the
/// access matrix allows.
fn nav_for(app: AppState) -> Vec<NavItem> {
    let item = |route: Route, label: &'static str, icon: &'static str, group: &'static str, hue: Hue| {
        let page = match route {
            Route::Workforce {} => "workforce",
            Route::Operations {} => "operations",
            Route::Clinic {} => "clinic",
            Route::Settings {} => "settings",
            _ => "",
        };
        NavItem { route, label, icon, group, hue, page }
    };
    let mut v = vec![];
    match app.role() {
        Some(UserRole::VipClient) | None => {
            v.push(item(Route::Home {}, "Overview", "home", "", Hue::Emerald));
            v.push(item(Route::Agenda {}, "Agenda", "calendar", "Your estate", Hue::Violet));
            if app.can(Permission::ManageMap) {
                v.push(item(Route::Studio {}, "Map Studio", "pen-tool", "Your estate", Hue::Teal));
            }
            v.push(item(Route::Archive {}, "History", "archive", "Your estate", Hue::Stone));
        }
        Some(r) => {
            // One platform: the estate, the workforce, operations, and the system.
            let admin = r == UserRole::OfficeAdmin;
            v.push(item(Route::Home {}, if admin { "Overview" } else { "Today" }, "home", "", Hue::Emerald));
            const ESTATE: &str = "Estate command";
            if app.can(Permission::ManageMap) {
                v.push(item(Route::Studio {}, "Map Studio", "pen-tool", ESTATE, Hue::Teal));
            }
            v.push(item(Route::Decisions {}, if admin { "Issues" } else { "Report" }, "flag", ESTATE, Hue::Rose));
            v.push(item(Route::Agenda {}, "Schedule", "calendar", ESTATE, Hue::Violet));
            v.push(item(Route::Equipment {}, "Equipment", "truck", ESTATE, Hue::Amber));
            v.push(item(Route::Archive {}, "History", "archive", ESTATE, Hue::Stone));
            const CREW: &str = "Workforce & crew";
            v.push(item(Route::Crew {}, "Crew", "hard-hat", CREW, Hue::Sky));
            if app.can(Permission::ManageWorkforce) || app.can(Permission::ManageCrew) {
                v.push(item(Route::Personnel {}, "Personnel", "users", CREW, Hue::Teal));
            }
            if app.can(Permission::ManageWorkforce) {
                v.push(item(Route::Workforce {}, "Groups & records", "layers", CREW, Hue::Indigo));
                v.push(item(Route::Operations {}, "Duty & operations", "clock", "Operations & duty", Hue::Blue));
            }
            if app.can(Permission::ManageClinic) {
                v.push(item(Route::Clinic {}, "Clinic ledger", "info", "Operations & duty", Hue::Rose));
            }
        }
    }
    const ADMIN: &str = "System & admin";
    if app.can(Permission::ManagePeople) {
        v.push(item(Route::People {}, "People", "key", ADMIN, Hue::Blue));
    }
    if app.can(Permission::ManageAccess) {
        v.push(item(Route::Access {}, "Access", "shield", ADMIN, Hue::Indigo));
    }
    if app.can(Permission::ManageSettings) {
        v.push(item(Route::Settings {}, "Settings", "settings", ADMIN, Hue::Slate));
    }
    v
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
    let nav = nav_for(app);
    // Sidebar open (names) or folded to an icon strip; remembered on this device.
    let mut folded = use_signal(|| util::storage_get(SIDEBAR_KEY).as_deref() == Some("folded"));
    let fold = folded();
    let side_w = if fold { "72px" } else { "240px" };
    let toggle_icon = if fold { "chevron-right" } else { "chevron-left" };
    let toggle_label = if fold { "Open the menu" } else { "Fold the menu" };
    let live_class = if fold { "justify-center pb-2 text-xs text-muted" } else { "px-4 pb-2 text-xs text-muted" };
    // The category holding the open page starts open.
    let active_group = nav.iter().find(|i| is_active(&route, &i.route)).map(|i| i.group).unwrap_or("");
    use_hook(move || {
        NAV_CLOSED.write().remove(active_group);
    });
    // Phone: the slide-out menu and the "More" sheet.
    let mut drawer = use_signal(|| false);
    let mut more = use_signal(|| false);
    let pending = app.pending_offline_mutations.read().len();
    let user = app.current_user.read().clone();
    let bar: Vec<NavItem> = nav.iter().take(4).cloned().collect();
    let more_active = !bar.iter().any(|i| is_active(&route, &i.route));

    // Ctrl+K (⌘K) anywhere: find a worker (Service 5's worker lookup).
    let router = use_navigator();
    let lookup = app.can(Permission::ManageWorkforce) || app.can(Permission::ManageCrew);
    use_future(move || async move {
        if !lookup {
            return;
        }
        let mut keys = document::eval(
            "document.addEventListener('keydown', e => {
               if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
                 e.preventDefault();
                 const box = document.getElementById('worker-search');
                 if (box) box.focus(); else dioxus.send('open');
               }
             });",
        );
        while keys.recv::<String>().await.is_ok() {
            router.push(Route::Personnel {});
        }
    });

    // Scroll to the top whenever the page changes.
    let mut last_route = use_signal(|| route.clone());
    if *last_route.peek() != route {
        last_route.set(route.clone());
        util::scroll_top();
        if NAV_CLOSED.peek().contains(active_group) {
            NAV_CLOSED.write().remove(active_group);
        }
        drawer.set(false);
        more.set(false);
    }

    rsx! {
        div { class: "min-h-screen flex flex-col", style: "--sidebar-w: {side_w}",
            MobileTopBar { menu_open: drawer(), on_menu: move |_| drawer.set(true) }
            if drawer() {
                MobileDrawer { on_close: move |_| drawer.set(false) }
            }
            if more() {
                MoreSheet { on_close: move |_| more.set(false) }
            }
            div { class: "flex flex-1 min-h-0",
                // ---------------------------------------------- sidebar
                aside {
                    class: if fold { "hidden md:flex w-[72px] shrink-0 flex-col bg-sidebar border-r border-line sticky top-0 h-screen transition-[width] duration-200" } else { "hidden md:flex w-60 shrink-0 flex-col bg-sidebar border-r border-line sticky top-0 h-screen transition-[width] duration-200" },
                    "data-testid": "sidebar",
                    div { class: if fold { "pt-4 pb-1 flex flex-col items-center gap-2" } else { "pl-4 pr-2 pt-4 pb-1 flex items-center gap-2.5" },
                        div { class: "w-8 h-8 rounded-lg chip-on flex items-center justify-center font-serif text-sm shrink-0", "S" }
                        if !fold {
                            p { class: "text-sm font-semibold flex-1", "SiteMonitor" }
                        }
                        button {
                            class: "btn-icon btn-ghost h-8 w-8",
                            title: "{toggle_label}",
                            "aria-label": "{toggle_label}",
                            "aria-expanded": !fold,
                            "data-testid": "sidebar-toggle",
                            onclick: move |_| {
                                let next = !folded();
                                folded.set(next);
                                util::storage_set(SIDEBAR_KEY, if next { "folded" } else { "open" });
                            },
                            Icon { name: toggle_icon }
                        }
                    }
                    NavTree { fold, prefix: "nav" }
                    LiveStatus { class: live_class, compact: fold }
                    if let Some(u) = user.clone() {
                        if fold {
                            div { class: "p-2 mb-2 flex flex-col items-center gap-1", title: "{u.name} · {u.role.label()}",
                                Avatar { name: u.name.clone(), size: 34 }
                                AccountActions { compact: true }
                            }
                        } else {
                            div { class: "p-2 m-2 rounded-lg border border-line space-y-2",
                                div { class: "flex items-center gap-2.5 px-1 pt-1",
                                    Avatar { name: u.name.clone(), size: 34 }
                                    div { class: "min-w-0 flex-1",
                                        p { class: "text-sm font-medium truncate", "data-testid": "signed-in-as", "{u.name}" }
                                        p { class: "text-xs text-muted", "{u.role.label()}" }
                                    }
                                }
                                AccountActions {}
                            }
                        }
                    }
                }
                // ---------------------------------------------- page
                main { class: "flex-1 min-w-0",
                    if app.is_staff() && (pending > 0 || !(app.is_online)()) {
                        OfflineBanner {}
                    }
                    div { class: "w-full px-4 sm:px-6 lg:px-10 pt-6 pb-28 md:pb-16",
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
                class: "md:hidden fixed bottom-0 inset-x-0 z-40 bg-surface/95 backdrop-blur border-t border-line safe-bottom",
                "aria-label": "Quick",
                "data-testid": "bottom-bar",
                div { class: "flex h-16",
                    for item in bar {
                        Link {
                            key: "{item.label}",
                            to: item.route.clone(),
                            class: if is_active(&route, &item.route) { "tab-link active" } else { "tab-link" },
                            Icon { name: item.icon, size: 20, class: item.hue.ink().to_string() }
                            span { class: "truncate max-w-full px-1", "{item.label}" }
                        }
                    }
                    // The last slot opens every other page, in the sidebar's categories.
                    button {
                        class: if more_active || more() { "tab-link active" } else { "tab-link" },
                        "aria-haspopup": "dialog",
                        "aria-expanded": more(),
                        "data-testid": "bottom-more",
                        onclick: move |_| more.toggle(),
                        Icon { name: "more", size: 20 }
                        span { "More" }
                    }
                }
            }
        }
    }
}

/// The sidebar's menu: categories as dropdowns, tabbed pages opening into their tabs.
/// Used by the desktop sidebar and the phone drawer.
#[component]
fn NavTree(fold: bool, prefix: &'static str, #[props(default)] on_navigate: Option<EventHandler<()>>) -> Element {
    let app = use_app();
    let route: Route = use_route();
    let nav = nav_for(app);
    let decisions = app.decisions.read().clone().unwrap_or_default();
    let me = app.user_id();
    let draft_count = decisions.iter().filter(|d| d.status == DecisionStatus::DraftInAdminQueue).count();
    // Reports the office sent back to me for more details.
    let asked_me = decisions.iter().filter(|d| d.status == DecisionStatus::NeedsDetails && d.reported_by == me).count();
    rsx! {
        nav { class: "px-2 pb-2 flex-1 overflow-y-auto no-scrollbar", "aria-label": "Main",
            for (gi, (group, items)) in nav_groups(&nav).into_iter().enumerate() {
                div { key: "{gi}",
                    if group.is_empty() || fold {
                        if fold && !group.is_empty() {
                            div { class: "h-px bg-line mx-2 my-2" }
                        }
                        div { class: "space-y-0.5",
                            for item in items {
                                {
                                    let (badge, badge_tone) = nav_badge(item.label, draft_count, asked_me);
                                    let active = is_active(&route, &item.route);
                                    rsx! { NavLink { key: "{item.label}", item, active, folded: fold, badge, badge_tone, on_navigate } }
                                }
                            }
                        }
                    } else {
                        {
                            let open = !NAV_CLOSED.read().contains(group);
                            rsx! {
                                // A category: a dropdown row whose chevron turns, over its pages.
                                button {
                                    class: "nav-parent mt-1",
                                    "aria-expanded": open,
                                    "aria-controls": "{prefix}-group-{gi}",
                                    "data-testid": "nav-group",
                                    onclick: move |_| {
                                        let mut c = NAV_CLOSED.write();
                                        if !c.remove(group) {
                                            c.insert(group.to_string());
                                        }
                                        util::storage_set(NAV_CLOSED_KEY, &c.iter().cloned().collect::<Vec<_>>().join("|"));
                                    },
                                    Icon { name: group_icon(group), size: 17, class: "text-muted".to_string() }
                                    span { class: "flex-1 truncate", "{group}" }
                                    Icon { name: "chevron-down", size: 14, class: if open { "nav-chev open".to_string() } else { "nav-chev".to_string() } }
                                }
                                if open {
                                    ul { id: "{prefix}-group-{gi}", class: "nav-sub",
                                        for item in items {
                                            {
                                                let (badge, badge_tone) = nav_badge(item.label, draft_count, asked_me);
                                                let active = is_active(&route, &item.route);
                                                let page = item.page;
                                                let open = TABS_OPEN.read().get(page).copied().unwrap_or(active);
                                                rsx! {
                                                    SubItem { key: "{item.label}", item, active, badge, badge_tone, open, on_navigate,
                                                        on_toggle: move |_| { TABS_OPEN.write().insert(page, !open); } }
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
}

/// Moves focus into a just-opened panel so Escape closes it straight away.
fn focus_panel(id: &str) {
    document::eval(&format!("setTimeout(() => document.getElementById('{id}')?.focus(), 0)"));
}

/// Phone and small tablet: the whole sidebar as a slide-out panel.
#[component]
fn MobileDrawer(on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let user = app.current_user.read().clone();
    rsx! {
        div {
            id: "mobile-drawer",
            class: "md:hidden fixed inset-0 z-50",
            role: "dialog",
            "aria-modal": "true",
            "aria-label": "Menu",
            tabindex: "-1",
            "data-testid": "mobile-drawer",
            onmounted: move |_| focus_panel("mobile-drawer"),
            onkeydown: move |e: KeyboardEvent| { if e.key() == Key::Escape { on_close.call(()); } },
            div { class: "absolute inset-0 bg-[#0f0f0f]/30 fade-in", onclick: move |_| on_close.call(()) }
            aside { class: "absolute inset-y-0 left-0 w-[290px] max-w-[85vw] bg-sidebar border-r border-line shadow-pop flex flex-col drawer-in",
                div { class: "pl-4 pr-2 pt-4 pb-1 flex items-center gap-2.5",
                    div { class: "w-8 h-8 rounded-lg chip-on flex items-center justify-center font-serif text-sm shrink-0", "S" }
                    p { class: "text-sm font-semibold flex-1", "SiteMonitor" }
                    button { class: "btn-icon btn-ghost h-9 w-9", "aria-label": "Close the menu", "data-testid": "menu-close", onclick: move |_| on_close.call(()), Icon { name: "x" } }
                }
                NavTree { fold: false, prefix: "drawer", on_navigate: move |_| on_close.call(()) }
                LiveStatus { class: "px-4 pb-2 text-xs text-muted" }
                if let Some(u) = user {
                    div { class: "p-2 m-2 rounded-lg border border-line space-y-2 safe-bottom",
                        div { class: "flex items-center gap-2.5 px-1 pt-1",
                            Avatar { name: u.name.clone(), size: 34 }
                            div { class: "min-w-0 flex-1",
                                p { class: "text-sm font-medium truncate", "{u.name}" }
                                p { class: "text-xs text-muted", "{u.role.label()}" }
                            }
                        }
                        AccountActions {}
                    }
                }
            }
        }
    }
}

/// The bottom bar's "More": every page, in the same categories as the sidebar.
#[component]
fn MoreSheet(on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let route: Route = use_route();
    let nav = nav_for(app);
    rsx! {
        div {
            id: "more-sheet",
            class: "md:hidden fixed inset-0 z-50",
            role: "dialog",
            "aria-modal": "true",
            "aria-label": "More pages",
            tabindex: "-1",
            "data-testid": "more-sheet",
            onmounted: move |_| focus_panel("more-sheet"),
            onkeydown: move |e: KeyboardEvent| { if e.key() == Key::Escape { on_close.call(()); } },
            div { class: "absolute inset-0 bg-[#0f0f0f]/30 fade-in", onclick: move |_| on_close.call(()) }
            div { class: "absolute inset-x-0 bottom-0 max-h-[80vh] overflow-y-auto bg-surface rounded-t-2xl shadow-pop sheet-in safe-bottom",
                div { class: "sticky top-0 bg-surface/95 backdrop-blur flex items-center justify-between px-5 h-14 border-b border-line",
                    p { class: "font-semibold", "All pages" }
                    button { class: "btn-icon btn-ghost", "aria-label": "Close", "data-testid": "more-close", onclick: move |_| on_close.call(()), Icon { name: "x" } }
                }
                div { class: "px-4 pt-2 pb-24 space-y-4",
                    for (group, items) in nav_groups(&nav) {
                        section { key: "{group}", "data-testid": "more-group",
                            p { class: "flex items-center gap-2 px-1 pt-2 pb-1.5 text-xs font-semibold uppercase tracking-wide text-muted",
                                Icon { name: if group.is_empty() { "home" } else { group_icon(group) }, size: 14 }
                                if group.is_empty() { "Start" } else { "{group}" }
                            }
                            div { class: "grid grid-cols-3 gap-2",
                                for item in items {
                                    Link {
                                        key: "{item.label}",
                                        to: item.route.clone(),
                                        class: if is_active(&route, &item.route) { "more-tile active" } else { "more-tile" },
                                        "data-testid": "more-link",
                                        onclick: move |_| on_close.call(()),
                                        Icon { name: item.icon, size: 20, class: item.hue.ink().to_string() }
                                        span { class: "text-xs text-center leading-tight", "{item.label}" }
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

/// "Offline Mode — X Updates Queued" (Phase 3 §4).
#[component]
fn OfflineBanner() -> Element {
    let app = use_app();
    let pending = app.pending_offline_mutations.read().len();
    let online = (app.is_online)();
    let syncing = (app.syncing)();
    let text = if !online {
        format!("You're offline — {} saved on this device", util::plural(pending, "update"))
    } else if syncing {
        format!("Back online — sending {}…", util::plural(pending, "update"))
    } else {
        format!("{} waiting to send", util::plural(pending, "update"))
    };
    rsx! {
        div {
            class: "sticky top-12 md:top-0 z-30 bg-tag-amber-bg text-tag-amber-text border-b border-tag-amber-text/20 fade-in",
            "data-testid": "offline-badge",
            div { class: "w-full px-4 sm:px-6 lg:px-10 h-10 flex items-center gap-2 text-sm font-medium",
                Icon { name: match online { true => "refresh", false => "wifi-off" } }
                span { class: "flex-1", "{text}" }
                if !online {
                    span { class: "hidden sm:inline font-normal opacity-80", "Keep working — everything syncs automatically." }
                } else if pending > 0 && !syncing {
                    button {
                        class: "btn h-7 bg-surface/60 hover:bg-surface text-tag-amber-text",
                        onclick: move |_| { spawn(async move { sync::flush(app).await; }); },
                        "Send now"
                    }
                }
            }
        }
    }
}

/// Phone header: estate name, live status and the account menu.
#[component]
fn MobileTopBar(menu_open: bool, on_menu: EventHandler<()>) -> Element {
    let app = use_app();
    let mut open = use_signal(|| false);
    let user = app.current_user.read().clone();
    rsx! {
        div { class: "md:hidden sticky top-0 z-40 h-12 glass rounded-none border-x-0 border-t-0 shadow-none",
            div { class: "h-full px-2 flex items-center gap-2",
                button { class: "btn-icon btn-ghost h-10 w-10", "aria-label": "Open the menu", "aria-expanded": menu_open, "aria-haspopup": "dialog", "data-testid": "menu-open",
                    onclick: move |_| on_menu.call(()),
                    svg { width: "20", height: "20", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "1.75", stroke_linecap: "round", "aria-hidden": "true",
                        path { d: "M4 7h16M4 12h16M4 17h16" }
                    }
                }
                div { class: "w-7 h-7 rounded-md chip-on flex items-center justify-center font-serif text-sm shrink-0", "S" }
                p { class: "text-sm font-semibold truncate flex-1", "SiteMonitor" }
                LiveStatus { class: "text-xs text-muted" }
                button { class: "btn-icon btn-ghost", "aria-label": "Account", "data-testid": "account-menu", onclick: move |_| open.toggle(),
                    Avatar { name: user.as_ref().map(|u| u.name.clone()).unwrap_or_default(), size: 26 }
                }
            }
            if open() {
                div { class: "absolute right-3 top-12 w-64 card shadow-pop p-3 space-y-2 scale-in",
                    if let Some(u) = user {
                        div { class: "px-1",
                            p { class: "text-sm font-medium truncate", "{u.name}" }
                            p { class: "text-xs text-muted", "{u.role.label()}" }
                        }
                    }
                    AccountActions {}
                }
            }
        }
    }
}

/// "Live" / "Offline" dot for the live-update connection.
#[component]
fn LiveStatus(class: &'static str, #[props(default)] compact: bool) -> Element {
    let app = use_app();
    let connected = (app.ws_connected)();
    let online = (app.is_online)();
    rsx! {
        span {
            class: "flex items-center gap-1.5 {class}",
            title: if connected { "Live updates are on" } else { "Reconnecting to live updates…" },
            span { class: if !online { "w-1.5 h-1.5 rounded-full bg-[#e5a33d]" } else if connected { "w-1.5 h-1.5 rounded-full bg-[#34c38f]" } else { "w-1.5 h-1.5 rounded-full bg-muted/50 animate-pulse" } }
            if !compact {
                if !online { "Offline" } else if connected { "Live" } else { "Connecting…" }
            }
        }
    }
}

/// One menu entry: a coloured icon tile, the name, and a count if something is waiting.
#[component]
fn NavLink(item: NavItem, active: bool, folded: bool, badge: usize, badge_tone: &'static str, #[props(default)] on_navigate: Option<EventHandler<()>>) -> Element {
    let class = match (active, folded) {
        (true, true) => "nav-link active h-10 justify-center px-0",
        (false, true) => "nav-link h-10 justify-center px-0",
        (true, false) => "nav-link active h-9",
        (false, false) => "nav-link h-9",
    };
    let name = if badge > 0 { format!("{}, {badge} new", item.label) } else { item.label.to_string() };
    rsx! {
        Link {
            to: item.route.clone(),
            class,
            onclick: move |_| { if let Some(h) = on_navigate { h.call(()) } },
            title: if folded { name.clone() } else { String::new() },
            "aria-label": "{name}",
            span { class: "relative w-5 h-5 flex items-center justify-center shrink-0",
                Icon { name: item.icon, size: 17 }
                if folded && badge > 0 {
                    span { class: "absolute -top-1.5 -right-1.5 min-w-[16px] h-4 px-1 rounded-full bg-rose-600 text-white text-[10px] leading-4 font-semibold text-center", "{badge}" }
                }
            }
            if !folded {
                span { class: "flex-1 truncate", "{item.label}" }
                if badge > 0 {
                    span { class: "pill {badge_tone}", "data-testid": "nav-badge", "{badge}" }
                }
            }
        }
    }
}

fn nav_badge(label: &str, drafts: usize, asked: usize) -> (usize, &'static str) {
    match label {
        "Issues" => (drafts, "bg-tag-amber-bg text-tag-amber-text"),
        "Report" => (asked, "bg-tag-blue-bg text-tag-blue-text"),
        _ => (0, ""),
    }
}

fn group_icon(group: &str) -> &'static str {
    match group {
        "Workforce & crew" => "users",
        "Operations & duty" => "clock",
        "System & admin" => "settings",
        _ => "map",
    }
}

/// A page inside a sidebar category. A page with tabs opens into those tabs,
/// which match the tab bar on the page: pick either one and both follow.
#[component]
fn SubItem(item: NavItem, active: bool, badge: usize, badge_tone: &'static str, open: bool, on_toggle: EventHandler<()>, #[props(default)] on_navigate: Option<EventHandler<()>>) -> Element {
    let app = use_app();
    let page = item.page;
    let tabs = views::page_tabs(app, page);
    let tab = current_tab(page, &tabs);
    let name = if badge > 0 { format!("{}, {badge} new", item.label) } else { item.label.to_string() };
    let class = match (active, tabs.is_empty()) {
        (true, true) => "nav-sub-link active",
        (true, false) => "nav-sub-link current",
        _ => "nav-sub-link",
    };
    rsx! {
        li { class: "relative",
            Link { to: item.route.clone(), class, "aria-label": "{name}", onclick: move |_| { if let Some(h) = on_navigate { h.call(()) } },
                span { class: "flex-1 truncate", "{item.label}" }
                if badge > 0 {
                    span { class: "pill {badge_tone}", "data-testid": "nav-badge", "{badge}" }
                }
                if !tabs.is_empty() {
                    span { class: "w-5 shrink-0" }
                }
            }
            if !tabs.is_empty() {
                button {
                    class: "nav-chev-btn",
                    "aria-label": if open { format!("Hide the {} sections", item.label) } else { format!("Show the {} sections", item.label) },
                    "aria-expanded": open,
                    "data-testid": "nav-expand",
                    onclick: move |_| on_toggle.call(()),
                    Icon { name: "chevron-down", size: 14, class: if open { "nav-chev open".to_string() } else { "nav-chev".to_string() } }
                }
                if open {
                    ul { class: "nav-sub",
                        for t in tabs.iter().copied() {
                            li { key: "{t}",
                                Link {
                                    to: item.route.clone(),
                                    class: if active && tab == t { "nav-sub-link active" } else { "nav-sub-link" },
                                    "data-testid": "nav-tab-{t}",
                                    onclick: move |_| {
                                        PAGE_TABS.write().insert(page, t);
                                        if let Some(h) = on_navigate { h.call(()) }
                                    },
                                    "{t}"
                                }
                            }
                        }
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
    if !app.can(Permission::ManageMap) {
        return rsx! { components::ui::EmptyState { icon: "pen-tool", title: "You don't have access to Map Studio", body: "Ask the Office Admin to give you \u{201c}Edit the estate map\u{201d}." } };
    }
    rsx! { MapStudio {} }
}

#[component]
fn People() -> Element {
    let app = use_app();
    if !app.can(Permission::ManagePeople) {
        return rsx! { components::ui::EmptyState { icon: "users", title: "You don't have access to People", body: "Ask the Office Admin to give you \u{201c}Add people & reset passwords\u{201d}." } };
    }
    rsx! { PeopleView {} }
}

#[component]
fn Personnel() -> Element {
    let app = use_app();
    if !app.can(Permission::ManageWorkforce) && !app.can(Permission::ManageCrew) {
        return rsx! { components::ui::EmptyState { icon: "users", title: "You don't have access to Personnel", body: "Ask the Office Admin to give you \u{201c}Manage workers & volunteers\u{201d}." } };
    }
    rsx! { views::personnel::PersonnelView {} }
}

fn no_access(icon: &'static str, page: &str, permission: &str) -> Element {
    rsx! { components::ui::EmptyState { icon, title: format!("You don't have access to {page}"), body: format!("Ask the Office Admin to give you \u{201c}{permission}\u{201d}.") } }
}

#[component]
fn Workforce() -> Element {
    if !use_app().can(Permission::ManageWorkforce) {
        return no_access("layers", "workforce records", "Manage workers & volunteers");
    }
    rsx! { views::workforce_ops::WorkforceView {} }
}

#[component]
fn Operations() -> Element {
    if !use_app().can(Permission::ManageWorkforce) {
        return no_access("clock", "duty & operations", "Manage workers & volunteers");
    }
    rsx! { views::workforce_ops::OperationsView {} }
}

#[component]
fn Clinic() -> Element {
    if !use_app().can(Permission::ManageClinic) {
        return no_access("info", "the clinic ledger", "Clinic ledger (medical records)");
    }
    rsx! { views::clinic::ClinicView {} }
}

#[component]
fn Settings() -> Element {
    let app = use_app();
    if !app.can(Permission::ManageSettings) {
        return rsx! { components::ui::EmptyState { icon: "settings", title: "You don't have access to Settings", body: "Ask the Office Admin to give you \u{201c}Change estate settings\u{201d}." } };
    }
    rsx! { SettingsView {} }
}

#[component]
fn Access() -> Element {
    let app = use_app();
    if !app.can(Permission::ManageAccess) {
        return rsx! { components::ui::EmptyState { icon: "shield", title: "You don't have access to the access matrix", body: "Ask the Office Admin to give you \u{201c}Change access\u{201d}." } };
    }
    rsx! { AccessView {} }
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
                body: "It's for the Office Admin and Site Monitors. Sign in with a staff account to use it.",
            }
            div { class: "flex justify-center",
                Link { to: Route::Home {}, class: "btn-secondary", "Back to overview" }
            }
        }
    }
}
