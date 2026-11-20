//! People: the Office Admin adds, edits and deactivates the estate's users.

use dioxus::prelude::*;
use shared::api::SaveUserRequest;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{Avatar, EmptyState, Icon, Modal, Pill, Spinner};
use crate::services::sync;
use crate::state::use_app;
use crate::util;

#[derive(Clone, PartialEq)]
enum Editing {
    New,
    Existing(User),
}

#[component]
pub fn PeopleView() -> Element {
    let app = use_app();
    let mut editing = use_signal(|| None::<Editing>);
    let users = app.users.read().clone();
    let flat = app.flat_areas();
    let me = app.user_id();
    let inactive: Vec<User> = users.iter().filter(|u| !u.is_active).cloned().collect();

    rsx! {
        div { class: "space-y-6 rise-in",
            header { class: "flex flex-wrap items-end justify-between gap-4",
                div { class: "space-y-1",
                    h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "People" }
                    p { class: "text-[15px] text-muted", "Everyone who uses SiteMonitor — add new staff, change roles, assign zones or switch someone off." }
                }
                button { class: "btn-primary h-10 px-4", "data-testid": "add-person", onclick: move |_| editing.set(Some(Editing::New)),
                    Icon { name: "plus" }
                    "Add person"
                }
            }
            for role in UserRole::ALL {
                {
                    let group: Vec<User> = users.iter().filter(|u| u.is_active && u.role == role).cloned().collect();
                    rsx! {
                        section { key: "{role:?}",
                            h2 { class: "section-title mb-3", "{role.label()}s" span { class: "text-muted font-normal ml-2", "{group.len()}" } }
                            if group.is_empty() {
                                div { class: "card", EmptyState { icon: "users", title: "Nobody in this role yet" } }
                            } else {
                                ul { class: "card divide-y divide-line",
                                    for u in group {
                                        PersonRow { key: "{u.id}", user: u.clone(), zones: zone_names(&flat, &u), is_me: me == Some(u.id), on_edit: move |u| editing.set(Some(Editing::Existing(u))) }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if !inactive.is_empty() {
                section {
                    h2 { class: "section-title mb-3", "Deactivated" span { class: "text-muted font-normal ml-2", "{inactive.len()}" } }
                    ul { class: "card divide-y divide-line opacity-80",
                        for u in inactive {
                            PersonRow { key: "{u.id}", user: u.clone(), zones: zone_names(&flat, &u), is_me: false, on_edit: move |u| editing.set(Some(Editing::Existing(u))) }
                        }
                    }
                }
            }
        }
        if let Some(e) = editing() {
            PersonModal { editing: e, on_close: move |_| editing.set(None) }
        }
    }
}

fn zone_names(flat: &[util::FlatArea], u: &User) -> String {
    if u.role == UserRole::SiteMonitor && u.assigned_area_ids.is_empty() {
        return "⚠ No zones yet — can't update any job until you assign some".into();
    }
    u.assigned_area_ids.iter().map(|a| util::area_name(flat, *a)).collect::<Vec<_>>().join(", ")
}

#[component]
fn PersonRow(user: User, zones: String, is_me: bool, on_edit: EventHandler<User>) -> Element {
    let u = user.clone();
    rsx! {
        li { class: "flex flex-wrap sm:flex-nowrap items-center gap-x-4 gap-y-2 px-4 py-3", "data-testid": "person-row",
            Avatar { name: user.name.clone(), size: 36 }
            div { class: "flex-1 min-w-[12rem]",
                p { class: "text-[15px] font-medium flex items-center gap-2",
                    "{user.name}"
                    if is_me { Pill { tone: Tone::Blue, text: "You" } }
                    if !user.is_active { Pill { tone: Tone::Gray, text: "Deactivated" } }
                }
                p { class: "text-sm text-muted truncate", "{user.email} · {user.phone}" }
                if zones.starts_with('⚠') {
                    p { class: "text-xs text-tag-amber-text mt-0.5 truncate", "data-testid": "person-no-zones", "{zones}" }
                } else if !zones.is_empty() {
                    p { class: "text-xs text-muted mt-0.5 truncate", "Looks after: {zones}" }
                }
            }
            button { class: "btn-secondary h-10 px-4 ml-auto", "data-testid": "edit-person", onclick: move |_| on_edit.call(u.clone()),
                Icon { name: "edit" }
                "Edit"
            }
        }
    }
}

#[component]
fn PersonModal(editing: Editing, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let (id, start) = match &editing {
        Editing::New => (None, User { id: Uuid::nil(), name: String::new(), role: UserRole::SiteMonitor, phone: String::new(), email: String::new(), assigned_area_ids: vec![], is_active: true, access: Default::default(), permissions: vec![], must_change_password: false, last_sign_in_at: None }),
        Editing::Existing(u) => (Some(u.id), u.clone()),
    };
    let is_me = id.is_some() && id == app.user_id();
    let mut name = use_signal(|| start.name.clone());
    let mut role = use_signal(|| start.role);
    let mut phone = use_signal(|| start.phone.clone());
    let mut email = use_signal(|| start.email.clone());
    let mut zones = use_signal(|| start.assigned_area_ids.clone());
    let mut active = use_signal(|| start.is_active);
    let mut first_password = use_signal(String::new);
    let mut temp_password = use_signal(|| None::<String>);
    let mut confirm_reset = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let password_issue = if id.is_none() && !first_password().is_empty() { shared::access::password_problem(&first_password(), &email()) } else { None };
    let areas: Vec<util::FlatArea> = app.flat_areas().into_iter().filter(|a| !a.area.is_archived).collect();
    let can_save = !name().trim().is_empty() && !email().trim().is_empty() && !busy()
        && (id.is_some() || (!first_password().is_empty() && password_issue.is_none()));
    let title = if id.is_some() { format!("Edit {}", start.name) } else { "Add a person".to_string() };

    let save = move |_| async move {
        busy.set(true);
        let req = SaveUserRequest {
            name: name(),
            role: Some(role()),
            phone: phone(),
            email: email(),
            assigned_area_ids: zones(),
            is_active: active(),
            initial_password: id.is_none().then(|| first_password()),
        };
        let result: Result<User, _> = match id {
            Some(id) => api::put(&format!("/users/{id}"), &req).await,
            None => api::post("/users", &req).await,
        };
        match result {
            Ok(u) => {
                app.success(if id.is_some() { format!("{} updated", u.name) } else { format!("{} added", u.name) });
                sync::reload(app, vec![DataScope::Users]);
                on_close.call(());
            }
            Err(e) => app.error(e.message()),
        }
        busy.set(false);
    };

    rsx! {
        Modal { title, on_close: move |_| on_close.call(()),
            div { class: "space-y-4",
                div {
                    label { class: "label", r#for: "person-name", "Full name" }
                    input { id: "person-name", class: "field", "data-testid": "person-name", placeholder: "e.g. Liza Marquez", value: "{name}", oninput: move |e| name.set(e.value()) }
                }
                div {
                    label { class: "label", r#for: "person-role", "Role" }
                    select { id: "person-role", class: "field", "data-testid": "person-role", disabled: is_me,
                        onchange: move |e| if let Some(r) = UserRole::ALL.iter().find(|r| format!("{r:?}") == e.value()) { role.set(*r) },
                        for r in UserRole::ALL {
                            option { key: "{r:?}", value: "{r:?}", selected: role() == r, "{r.label()}" }
                        }
                    }
                    p { class: "hint",
                        match role() {
                            UserRole::VipClient => "Sees the calm owner portal: decisions, photos, agenda and sign-offs.",
                            UserRole::OfficeAdmin => "Runs the office: reviews reports, publishes decisions, edits the map and manages people.",
                            UserRole::SiteMonitor => "Works in the field: crew, equipment, progress photos and issue reports.",
                        }
                        " Fine-tune exactly what they can do on the Access page."
                    }
                }
                div { class: "grid sm:grid-cols-2 gap-4",
                    div {
                        label { class: "label", r#for: "person-email", "Email" }
                        input { id: "person-email", r#type: "email", class: "field", "data-testid": "person-email", placeholder: "name@example.com", value: "{email}", oninput: move |e| email.set(e.value()) }
                    }
                    div {
                        label { class: "label", r#for: "person-phone", "Phone" }
                        input { id: "person-phone", r#type: "tel", class: "field", "data-testid": "person-phone", placeholder: "+63 917 555 0000", value: "{phone}", oninput: move |e| phone.set(e.value()) }
                    }
                }
                if role() == UserRole::SiteMonitor {
                    div {
                        p { class: "label", "Zones they look after" }
                        div { class: "max-h-48 overflow-y-auto rounded-md border border-line divide-y divide-line",
                            for a in areas.iter() {
                                {
                                    let aid = a.area.id;
                                    rsx! {
                                        label { key: "{aid}", class: "flex items-center gap-3 pr-3 h-10 text-sm cursor-pointer hover:bg-hover",
                                            style: "padding-left:{0.75 + a.depth as f32}rem",
                                            input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", checked: zones().contains(&aid),
                                                onchange: move |e| {
                                                    let mut z = zones();
                                                    if e.checked() { if !z.contains(&aid) { z.push(aid) } } else { z.retain(|x| *x != aid) }
                                                    zones.set(z);
                                                },
                                            }
                                            span { class: "truncate", "{a.area.name}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if id.is_none() {
                    div {
                        label { class: "label", r#for: "person-password", "First password" }
                        input { id: "person-password", r#type: "text", class: "field font-mono", autocomplete: "off", "data-testid": "person-password",
                            placeholder: "At least 10 characters", value: "{first_password}", oninput: move |e| first_password.set(e.value()) }
                        p { class: if password_issue.is_some() { "hint text-tag-red-text" } else { "hint" },
                            {password_issue.unwrap_or("Give it to them in person. They'll be asked to choose their own the first time they sign in.")}
                        }
                    }
                }
                if let Some(uid) = id.filter(|_| !is_me) {
                    div { class: "rounded-lg border border-line p-3 space-y-2",
                        div { class: "flex items-center gap-3",
                            div { class: "flex-1 min-w-0",
                                p { class: "text-sm font-medium", "Password" }
                                p { class: "text-xs text-muted",
                                    match start.last_sign_in_at { Some(t) => format!("Last signed in {}", util::relative(t)), None => "Hasn't signed in yet".to_string() }
                                }
                            }
                            if temp_password().is_none() {
                                button { class: "btn-secondary h-9", "data-testid": "reset-password", disabled: busy(),
                                    onclick: move |_| confirm_reset.set(true), Icon { name: "key" } "Reset password" }
                            }
                        }
                        if confirm_reset() && temp_password().is_none() {
                            div { class: "rounded-md bg-tag-amber-bg text-tag-amber-text text-sm px-3 py-2 space-y-2",
                                p { "This signs them out on every device and gives them a one-time password. Continue?" }
                                div { class: "flex gap-2 justify-end",
                                    button { class: "btn-ghost h-8", onclick: move |_| confirm_reset.set(false), "Cancel" }
                                    button { class: "btn-primary h-8", "data-testid": "confirm-reset-password", disabled: busy(),
                                        onclick: move |_| async move {
                                            busy.set(true);
                                            match api::post::<_, shared::api::ResetPasswordResponse>(&format!("/users/{uid}/reset-password"), &()).await {
                                                Ok(r) => temp_password.set(Some(r.temporary_password)),
                                                Err(e) => app.error(e.message()),
                                            }
                                            confirm_reset.set(false);
                                            busy.set(false);
                                        },
                                        "Reset password"
                                    }
                                }
                            }
                        }
                        if let Some(t) = temp_password() {
                            div { class: "rounded-md bg-accent-soft px-3 py-2 space-y-1",
                                p { class: "text-xs text-muted", "One-time password — shown only now. Share it privately:" }
                                p { class: "font-mono text-lg text-accent-ink select-all", "data-testid": "temporary-password", "{t}" }
                            }
                        }
                    }
                }
                if id.is_some() {
                    label { class: if is_me { "flex items-start gap-3 text-sm opacity-60" } else { "flex items-start gap-3 text-sm cursor-pointer" },
                        input { r#type: "checkbox", class: "w-4 h-4 mt-0.5 accent-[#064e3b]", "data-testid": "person-active", disabled: is_me, checked: active(), onchange: move |e| active.set(e.checked()) }
                        span {
                            span { class: "font-medium", "Active" }
                            span { class: "block text-muted",
                                if is_me { "You can't switch yourself off." } else { "Untick to switch this person off. Their past work stays in the records." }
                            }
                        }
                    }
                }
                div { class: "modal-actions",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button { class: "btn-primary", disabled: !can_save, "data-testid": "save-person", onclick: save,
                        if busy() { Spinner {} }
                        if id.is_some() { "Save changes" } else { "Add person" }
                    }
                }
            }
        }
    }
}
