//! Access matrix: who may do what.
//!
//! Rows are permissions (grouped), columns are people. Each cell shows whether
//! the permission comes from the person's role, was added for them, or was
//! taken away. Edits are staged, reviewed in a summary, then saved together
//! (the server re-checks everything and keeps an audit trail).

use std::collections::HashMap;

use dioxus::prelude::*;
use shared::access::{role_defaults, GROUPS};
use shared::api::*;
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{Avatar, Icon, Modal, Spinner};
use crate::state::use_app;
use crate::util;

#[derive(Clone, Copy, PartialEq)]
enum Cell {
    FromRole,
    Added,
    Removed,
    None,
}

fn cell(u: &User, set: &[Permission], p: Permission) -> Cell {
    let has = set.contains(&p);
    let default = role_defaults(u.role).contains(&p);
    match (has, default) {
        (true, true) => Cell::FromRole,
        (true, false) => Cell::Added,
        (false, true) => Cell::Removed,
        (false, false) => Cell::None,
    }
}

fn sorted(mut v: Vec<Permission>) -> Vec<Permission> {
    v.sort();
    v.dedup();
    v
}

#[component]
pub fn AccessView() -> Element {
    let app = use_app();
    let mut data = use_signal(|| None::<AccessMatrixResponse>);
    let mut draft = use_signal(HashMap::<Uuid, Vec<Permission>>::new);
    let mut query = use_signal(String::new);
    let mut role_filter = use_signal(|| None::<UserRole>);
    let mut show_inactive = use_signal(|| false);
    let mut reviewing = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut phone_person = use_signal(|| None::<Uuid>);

    // Load, and reload whenever people change anywhere (live updates).
    use_effect(move || {
        let _ = app.users.read().len();
        spawn(async move {
            match api::get::<AccessMatrixResponse>("/access").await {
                Ok(m) => {
                    // Drop staged edits that the server now already matches.
                    draft.write().retain(|id, set| m.users.iter().find(|u| u.id == *id).map(|u| sorted(u.permissions.clone()) != *set).unwrap_or(false));
                    data.set(Some(m));
                }
                Err(e) => app.error(e.message()),
            }
        });
    });

    let Some(m) = data() else { return rsx! { crate::components::ui::SkeletonPage {} } };
    let me = app.user_id();
    let d = draft();
    let set_of = |u: &User| d.get(&u.id).cloned().unwrap_or_else(|| sorted(u.permissions.clone()));
    let q = query().trim().to_lowercase();
    let people: Vec<User> = m
        .users
        .iter()
        .filter(|u| show_inactive() || u.is_active)
        .filter(|u| role_filter().map(|r| r == u.role).unwrap_or(true))
        .filter(|u| q.is_empty() || u.name.to_lowercase().contains(&q) || u.email.to_lowercase().contains(&q))
        .cloned()
        .collect();
    let customised = m.users.iter().filter(|u| u.is_active && !u.access.is_default()).count();
    let changed_people = d.len();
    let changed_cells: usize = d
        .iter()
        .filter_map(|(id, set)| m.users.iter().find(|u| u.id == *id).map(|u| (u, set)))
        .map(|(u, set)| Permission::ALL.iter().filter(|p| set.contains(p) != u.permissions.contains(p)).count())
        .sum();

    // Handlers read people from the signal so they stay `Copy`.
    let mut toggle = move |id: Uuid, p: Permission| {
        let Some(u) = data.peek().as_ref().and_then(|m| m.users.iter().find(|u| u.id == id).cloned()) else { return };
        let mut set = draft.peek().get(&id).cloned().unwrap_or_else(|| sorted(u.permissions.clone()));
        if set.contains(&p) {
            set.retain(|x| *x != p);
        } else {
            set.push(p);
        }
        let set = sorted(set);
        if set == sorted(u.permissions.clone()) {
            draft.write().remove(&id);
        } else {
            draft.write().insert(id, set);
        }
    };
    let mut reset_person = move |id: Uuid| {
        let Some(u) = data.peek().as_ref().and_then(|m| m.users.iter().find(|u| u.id == id).cloned()) else { return };
        let defaults = sorted(role_defaults(u.role).to_vec());
        if defaults == sorted(u.permissions.clone()) {
            draft.write().remove(&id);
        } else {
            draft.write().insert(id, defaults);
        }
    };

    let save = move |_| {
        let Some(m) = data.peek().clone() else { return };
        let changes: Vec<AccessChange> = draft
            .peek()
            .iter()
            .filter_map(|(id, set)| {
                let u = m.users.iter().find(|u| u.id == *id)?;
                Some(AccessChange { user_id: *id, access: AccessOverrides::from_effective(u.role, set) })
            })
            .collect();
        spawn(async move {
            busy.set(true);
            match api::put::<_, AccessMatrixResponse>("/access", &SaveAccessRequest { changes }).await {
                Ok(fresh) => {
                    draft.write().clear();
                    data.set(Some(fresh));
                    reviewing.set(false);
                    app.success("Access saved. It applies from their next action.");
                }
                Err(e) => app.error(e.message()),
            }
            busy.set(false);
        });
    };

    let phone_selected = phone_person().or_else(|| people.first().map(|u| u.id));
    let phone_user = phone_selected.and_then(|id| people.iter().find(|u| u.id == id).cloned());

    rsx! {
        div { class: "space-y-5 rise-in pb-4",
            header { class: "flex flex-wrap items-end justify-between gap-3",
                div { class: "min-w-0",
                    h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Access" }
                    p { class: "text-[15px] text-muted mt-1 max-w-2xl", "Decide what each person can do. Everyone starts with their role's defaults; a change here applies to that one person, straight away." }
                }
                div { class: "flex flex-wrap gap-2 text-xs",
                    span { class: "inline-flex items-center gap-1.5 rounded-full border border-line px-2.5 h-6", span { class: "font-semibold text-ink", "{m.users.iter().filter(|u| u.is_active).count()}" } span { class: "text-muted", "active people" } }
                    span { class: "inline-flex items-center gap-1.5 rounded-full border border-line px-2.5 h-6", span { class: "font-semibold text-ink", "{customised}" } span { class: "text-muted", "with custom access" } }
                }
            }

            // ---- toolbar
            div { class: "flex flex-wrap items-center gap-2",
                div { class: "relative w-full sm:w-64",
                    span { class: "absolute left-3 top-1/2 -translate-y-1/2 text-muted pointer-events-none", Icon { name: "search" } }
                    input { class: "field pl-9", r#type: "search", placeholder: "Find a person", "aria-label": "Find a person", "data-testid": "access-search",
                        value: "{query}", oninput: move |e| query.set(e.value()) }
                }
                div { class: "inline-flex p-0.5 rounded-lg bg-hover text-sm overflow-x-auto no-scrollbar", role: "tablist", "aria-label": "Filter by role",
                    RoleChip { label: "Everyone", active: role_filter().is_none(), on_click: move |_| role_filter.set(None) }
                    for r in UserRole::ALL {
                        RoleChip { key: "{r:?}", label: r.label(), active: role_filter() == Some(r), on_click: move |_| role_filter.set(Some(r)) }
                    }
                }
                label { class: "inline-flex items-center gap-2 text-sm text-muted ml-auto cursor-pointer",
                    input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", checked: show_inactive(), onchange: move |e| show_inactive.set(e.checked()) }
                    "Show deactivated"
                }
            }

            // ---- legend
            div { class: "flex flex-wrap items-center gap-x-5 gap-y-2 text-xs text-muted",
                span { class: "inline-flex items-center gap-1.5", CellMark { state: Cell::FromRole } "From their role" }
                span { class: "inline-flex items-center gap-1.5", CellMark { state: Cell::Added } "Added for this person" }
                span { class: "inline-flex items-center gap-1.5", CellMark { state: Cell::Removed } "Taken away from their role" }
                span { class: "inline-flex items-center gap-1.5", CellMark { state: Cell::None } "No access" }
                span { class: "inline-flex items-center gap-1.5", span { class: "w-2 h-2 rounded-full bg-pin-amber" } "Unsaved change" }
            }

            if people.is_empty() {
                div { class: "card p-8 text-center text-sm text-muted", "Nobody matches that search." }
            }

            // ---- desktop / tablet: the matrix
            if !people.is_empty() {
                div { class: "hidden md:block card overflow-auto max-h-[70vh]", "data-testid": "access-matrix",
                    table { class: "border-separate border-spacing-0 text-sm min-w-full",
                        thead {
                            tr {
                                th { class: "sticky top-0 left-0 z-30 bg-surface border-b border-r border-line text-left px-4 py-3 min-w-[260px] align-bottom",
                                    p { class: "eyebrow", "Permission" }
                                }
                                for u in people.iter().cloned() {
                                    {
                                        let edited = d.contains_key(&u.id);
                                        let custom = !AccessOverrides::from_effective(u.role, &set_of(&u)).is_default();
                                        let id = u.id;
                                        rsx! {
                                            th { key: "h-{u.id}", class: "sticky top-0 z-20 bg-surface border-b border-line px-2 py-3 w-[120px] min-w-[120px] align-bottom font-normal",
                                                div { class: if u.is_active { "flex flex-col items-center gap-1 text-center" } else { "flex flex-col items-center gap-1 text-center opacity-50" },
                                                    div { class: "relative",
                                                        Avatar { name: u.name.clone(), size: 30 }
                                                        if edited { span { class: "absolute -top-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-pin-amber ring-2 ring-surface" } }
                                                    }
                                                    p { class: "text-[13px] font-medium leading-tight max-w-[110px] truncate", title: "{u.name}", "{u.name}" }
                                                    p { class: "text-[11px] text-muted leading-tight", if u.is_active { "{u.role.label()}" } else { "Deactivated" } }
                                                    if custom {
                                                        button { class: "text-[11px] text-accent-ink hover:underline", title: "Reset to the {u.role.label()} defaults", "data-testid": "reset-defaults",
                                                            onclick: move |_| reset_person(id), "Reset to defaults" }
                                                    } else {
                                                        span { class: "text-[11px] text-muted", "Role defaults" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        tbody {
                            for (group, perms) in GROUPS.iter() {
                                tr { key: "g-{group}",
                                    td { class: "sticky left-0 z-10 bg-sidebar border-b border-line px-4 py-2", p { class: "eyebrow", "{group}" } }
                                    td { class: "bg-sidebar border-b border-line", colspan: "{people.len()}" }
                                }
                                for p in perms.iter().copied() {
                                    tr { key: "{p:?}", class: "group",
                                        th { class: "sticky left-0 z-10 bg-surface group-hover:bg-hover border-b border-r border-line text-left font-normal px-4 py-2.5",
                                            p { class: "font-medium text-ink flex items-center gap-1.5",
                                                "{p.label()}"
                                                if p.is_sensitive() { span { class: "pill bg-tag-amber-bg text-tag-amber-text", title: "Gives admin powers", "Admin" } }
                                            }
                                            p { class: "text-xs text-muted mt-0.5 max-w-[320px]", "{p.description()}" }
                                        }
                                        for u in people.iter() {
                                            {
                                                let set = set_of(u);
                                                let state = cell(u, &set, p);
                                                let changed = set.contains(&p) != u.permissions.contains(&p);
                                                let locked = Some(u.id) == me && p == Permission::ManageAccess;
                                                let on = set.contains(&p);
                                                let id = u.id;
                                                let label = format!("{} — {}", u.name, p.label());
                                                rsx! {
                                                    td { key: "{u.id}-{p:?}", class: "border-b border-line text-center group-hover:bg-hover/60 p-1",
                                                        button {
                                                            class: if changed { "relative inline-flex items-center justify-center w-10 h-9 rounded-md bg-tag-amber-bg/50 ring-1 ring-pin-amber/60" } else { "relative inline-flex items-center justify-center w-10 h-9 rounded-md hover:bg-hover transition-colors" },
                                                            role: "switch",
                                                            "aria-checked": on,
                                                            "aria-label": "{label}",
                                                            title: if locked { "You can't remove your own access to this page".to_string() } else { label.clone() },
                                                            disabled: locked || !u.is_active,
                                                            "data-testid": "cell",
                                                            onclick: move |_| toggle(id, p),
                                                            CellMark { state }
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

                // ---- phone: one person at a time
                div { class: "md:hidden space-y-3",
                    select { class: "field", "aria-label": "Person", onchange: move |e| phone_person.set(Uuid::parse_str(&e.value()).ok()),
                        for u in people.iter() {
                            option { key: "{u.id}", value: "{u.id}", selected: Some(u.id) == phone_selected, "{u.name} · {u.role.label()}" }
                        }
                    }
                    if let Some(u) = phone_user {
                        for (group, perms) in GROUPS.iter() {
                            div { key: "pg-{group}", class: "card overflow-hidden",
                                p { class: "eyebrow px-4 pt-3 pb-1", "{group}" }
                                for p in perms.iter().copied() {
                                    {
                                        let set = set_of(&u);
                                        let state = cell(&u, &set, p);
                                        let on = set.contains(&p);
                                        let changed = on != u.permissions.contains(&p);
                                        let locked = Some(u.id) == me && p == Permission::ManageAccess;
                                        let id = u.id;
                                        rsx! {
                                            button { key: "{p:?}", class: if changed { "w-full flex items-center gap-3 px-4 py-3 text-left border-t border-line bg-tag-amber-bg/40" } else { "w-full flex items-center gap-3 px-4 py-3 text-left border-t border-line" },
                                                role: "switch", "aria-checked": on, disabled: locked || !u.is_active, onclick: move |_| toggle(id, p),
                                                div { class: "flex-1 min-w-0",
                                                    p { class: "text-sm font-medium", "{p.label()}" }
                                                    p { class: "text-xs text-muted", "{p.description()}" }
                                                }
                                                CellMark { state }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ---- audit trail
            div { class: "card overflow-hidden",
                div { class: "px-4 h-11 flex items-center justify-between border-b border-line",
                    p { class: "eyebrow", "Recent access changes" }
                    span { class: "text-xs text-muted", "Kept for every account change" }
                }
                if m.audit.is_empty() {
                    p { class: "px-4 py-6 text-sm text-muted", "No changes yet." }
                }
                ul { class: "divide-y divide-line max-h-72 overflow-y-auto", "data-testid": "access-audit",
                    for a in m.audit.iter() {
                        li { key: "{a.id}", class: "px-4 py-2.5 flex gap-3 text-sm",
                            span { class: "w-24 shrink-0 text-xs text-muted pt-0.5", title: "{util::day_time(a.at)}", "{util::relative(a.at)}" }
                            div { class: "min-w-0",
                                p { span { class: "font-medium", "{a.target_name}" } span { class: "text-muted", " · by {a.actor_name}" } }
                                p { class: "text-xs text-muted mt-0.5", "{a.summary}" }
                            }
                        }
                    }
                }
            }

            // ---- save bar
            if changed_people > 0 {
                div { class: "sticky bottom-16 md:bottom-0 z-30 -mx-4 sm:-mx-6 lg:-mx-10 px-4 sm:px-6 lg:px-10 py-3 glass rounded-none border-x-0 border-b-0 flex flex-wrap items-center gap-2 rise-in",
                    span { class: "text-sm mr-auto", "data-testid": "access-pending",
                        span { class: "font-semibold", "{util::plural(changed_cells, \"change\")}" }
                        span { class: "text-muted", if changed_people == 1 { " for 1 person" } else { " for {changed_people} people" } }
                    }
                    button { class: "btn-ghost h-10 px-4", onclick: move |_| draft.write().clear(), "Undo all" }
                    button { class: "btn-primary h-10 px-4", "data-testid": "review-access", onclick: move |_| reviewing.set(true), "Review & save" }
                }
            }
        }

        if reviewing() {
            Modal { title: "Review access changes", on_close: move |_| reviewing.set(false),
                div { class: "space-y-4",
                    p { class: "text-sm text-muted", "These take effect as soon as you save. Each change is recorded in the access log." }
                    for (id, set) in d.iter() {
                        if let Some(u) = m.users.iter().find(|u| u.id == *id) {
                            {
                                let gain: Vec<Permission> = set.iter().copied().filter(|p| !u.permissions.contains(p)).collect();
                                let lose: Vec<Permission> = u.permissions.iter().copied().filter(|p| !set.contains(p)).collect();
                                let risky: Vec<&str> = gain.iter().filter(|p| p.is_sensitive()).map(|p| p.label()).collect();
                                rsx! {
                                    div { key: "{id}", class: "rounded-lg border border-line p-3 space-y-2",
                                        div { class: "flex items-center gap-2",
                                            Avatar { name: u.name.clone(), size: 24 }
                                            p { class: "text-sm font-medium flex-1", "{u.name}" }
                                            span { class: "text-xs text-muted", "{u.role.label()}" }
                                        }
                                        if !gain.is_empty() {
                                            div { class: "flex flex-wrap gap-1.5",
                                                span { class: "text-xs text-muted w-full", "Will be able to" }
                                                for p in gain.iter() { span { key: "g{p:?}", class: "pill bg-tag-green-bg text-tag-green-text", "+ {p.label()}" } }
                                            }
                                        }
                                        if !lose.is_empty() {
                                            div { class: "flex flex-wrap gap-1.5",
                                                span { class: "text-xs text-muted w-full", "Will no longer be able to" }
                                                for p in lose.iter() { span { key: "l{p:?}", class: "pill bg-tag-red-bg text-tag-red-text", "\u{2212} {p.label()}" } }
                                            }
                                        }
                                        if !risky.is_empty() {
                                            p { class: "rounded-md bg-tag-amber-bg text-tag-amber-text text-xs px-3 py-2 flex gap-2",
                                                Icon { name: "alert", class: "shrink-0" }
                                                "Admin powers: {risky.join(\", \")}. Only give these to people you fully trust."
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "modal-actions",
                        button { class: "btn-ghost", onclick: move |_| reviewing.set(false), "Keep editing" }
                        button { class: "btn-primary", disabled: busy(), "data-testid": "confirm-access", onclick: save,
                            if busy() { Spinner {} }
                            "Save changes"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RoleChip(label: &'static str, active: bool, on_click: EventHandler<()>) -> Element {
    rsx! {
        button {
            role: "tab",
            "aria-selected": active,
            class: if active { "px-3 h-8 rounded-md bg-surface shadow-sm font-medium whitespace-nowrap" } else { "px-3 h-8 rounded-md text-muted hover:text-ink whitespace-nowrap" },
            onclick: move |_| on_click.call(()),
            "{label}"
        }
    }
}

#[component]
fn CellMark(state: Cell) -> Element {
    match state {
        Cell::FromRole => rsx! {
            span { class: "inline-flex w-6 h-6 rounded-md bg-accent-soft text-accent-ink items-center justify-center", "aria-hidden": "true", Icon { name: "check", size: 14 } }
        },
        Cell::Added => rsx! {
            span { class: "inline-flex w-6 h-6 rounded-md chip-on items-center justify-center", "aria-hidden": "true", Icon { name: "plus", size: 14 } }
        },
        Cell::Removed => rsx! {
            span { class: "inline-flex w-6 h-6 rounded-md border-[1.5px] border-dashed border-tag-red-text/60 text-tag-red-text items-center justify-center", "aria-hidden": "true", Icon { name: "minus", size: 14 } }
        },
        Cell::None => rsx! {
            span { class: "inline-flex w-6 h-6 rounded-md border border-line items-center justify-center", "aria-hidden": "true" }
        },
    }
}
