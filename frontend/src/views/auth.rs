//! Sign-in, first-time / reset password change, and the account actions
//! (theme, change password, sign out).

use dioxus::prelude::*;
use shared::access::password_problem;
use shared::api::*;
use shared::*;

use crate::api;
use crate::components::ui::{Avatar, Icon, Modal, Spinner};
use crate::state::use_app;

#[component]
pub fn LoginPage() -> Element {
    let app = use_app();
    // `None` = the server couldn't be reached (the page itself may come from the offline cache).
    let mut options = use_resource(|| async { api::get::<AuthOptions>("/auth/options").await.ok() });
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut show = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let unreachable = matches!(*options.read(), Some(None));
    let opts = options.read().clone().flatten().unwrap_or_default();

    let submit = move || {
        if busy() {
            return;
        }
        spawn(async move {
            busy.set(true);
            error.set(None);
            let req = LoginRequest { email: email().trim().to_string(), password: password() };
            match api::post::<_, SessionResponse>("/auth/login", &req).await {
                Ok(s) => {
                    password.set(String::new());
                    crate::start_session(app, s.user).await;
                }
                Err(e) => error.set(Some(e.message())),
            }
            busy.set(false);
        });
    };

    rsx! {
        div { class: "min-h-screen bg-sidebar flex items-center justify-center px-4 py-10",
            div { class: "w-full max-w-[400px] space-y-5 rise-in",
                div { class: "flex flex-col items-center gap-3 text-center",
                    div { class: "w-11 h-11 rounded-xl chip-on flex items-center justify-center font-serif text-lg shadow-float", "S" }
                    h1 { class: "text-2xl font-semibold tracking-tight", "Sign in to SiteMonitor" }
                }
                div { class: "bg-surface border border-line rounded-2xl shadow-float p-6 sm:p-8", "data-testid": "login-box",
                    form {
                        class: "space-y-4",
                        "data-testid": "login-form",
                        onsubmit: move |e: FormEvent| {
                            e.prevent_default();
                            submit();
                        },
                        div {
                            label { class: "label", r#for: "login-email", "Email" }
                            input { id: "login-email", class: "field", r#type: "email", autocomplete: "username", required: true, autofocus: true,
                                "data-testid": "login-email", placeholder: "name@company.com", value: "{email}", oninput: move |e| email.set(e.value()) }
                        }
                        div {
                            label { class: "label", r#for: "login-password", "Password" }
                            div { class: "relative",
                                input { id: "login-password", class: "field pr-11", r#type: if show() { "text" } else { "password" }, autocomplete: "current-password", required: true,
                                    "data-testid": "login-password", value: "{password}", oninput: move |e| password.set(e.value()) }
                                button { r#type: "button", class: "absolute right-1 top-1 btn-icon btn-ghost h-8 w-8", "aria-label": if show() { "Hide password" } else { "Show password" },
                                    onclick: move |_| show.toggle(), Icon { name: "eye" } }
                            }
                        }
                        if unreachable {
                            div { class: "rounded-md bg-tag-amber-bg text-tag-amber-text text-sm px-3 py-2 flex items-center gap-2", role: "alert", "data-testid": "server-unreachable",
                                span { class: "flex-1", "We can't connect right now. Please check your internet connection and try again." }
                                button { r#type: "button", class: "btn h-7 px-2 bg-surface/60", onclick: move |_| options.restart(), "Try again" }
                            }
                        }
                        if let Some(msg) = error() {
                            p { class: "rounded-md bg-tag-red-bg text-tag-red-text text-sm px-3 py-2", role: "alert", "data-testid": "login-error", "{msg}" }
                        }
                        button { class: "btn-primary btn-lg w-full", r#type: "submit", disabled: busy(), "data-testid": "login-submit",
                            if busy() { Spinner {} }
                            "Sign in"
                        }
                    }
                }
                p { class: "text-xs text-muted text-center", "Forgot your password? Ask the Office Admin to reset it." }
                if !opts.demo_accounts.is_empty() {
                    div { class: "bg-surface border border-line rounded-2xl p-3 space-y-1", "data-testid": "demo-accounts",
                        div { class: "flex items-center justify-between px-2 pb-1",
                            p { class: "eyebrow", "Demo accounts" }
                            if let Some(p) = opts.demo_password.clone() { span { class: "text-xs text-muted", "password: ", code { class: "text-ink", "{p}" } } }
                        }
                        for a in opts.demo_accounts.iter().cloned() {
                            {
                                let pw = opts.demo_password.clone().unwrap_or_default();
                                rsx! {
                                    button {
                                        key: "{a.email}",
                                        class: "w-full flex items-center gap-3 rounded-lg px-2 py-2 text-left hover:bg-hover transition-colors",
                                        "data-testid": "demo-{a.role:?}",
                                        onclick: move |_| {
                                            email.set(a.email.clone());
                                            password.set(pw.clone());
                                            submit();
                                        },
                                        Avatar { name: a.name.clone(), size: 32 }
                                        div { class: "min-w-0 flex-1",
                                            p { class: "text-sm font-medium truncate", "{a.name}" }
                                            p { class: "text-xs text-muted truncate", "{a.email}" }
                                        }
                                        span { class: "pill bg-tag-gray-bg text-tag-gray-text", "{a.role.label()}" }
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

/// Shown instead of the app until a new or reset account picks its own password.
#[component]
pub fn ForcedPasswordChange() -> Element {
    let app = use_app();
    let name = app.current_user.read().as_ref().map(|u| u.name.split_whitespace().next().unwrap_or_default().to_string()).unwrap_or_default();
    rsx! {
        div { class: "min-h-screen flex items-center justify-center px-5 py-12",
            div { class: "w-full max-w-md card p-6 space-y-5 rise-in", "data-testid": "forced-password-change",
                div { class: "space-y-1.5",
                    span { class: "w-10 h-10 rounded-xl bg-accent-soft text-accent-ink flex items-center justify-center", Icon { name: "key", size: 20 } }
                    h1 { class: "text-xl font-semibold pt-2", "Welcome, {name}. Choose your own password" }
                    p { class: "text-sm text-muted", "You signed in with a one-time password from the office. Pick a new one only you know." }
                }
                PasswordForm {
                    on_done: move |u: User| { spawn(async move { crate::start_session(app, u).await; }); },
                }
                button { class: "btn-ghost w-full", onclick: move |_| { spawn(crate::sign_out()); }, Icon { name: "log-out" } "Sign out" }
            }
        }
    }
}

#[component]
fn PasswordForm(on_done: EventHandler<User>, #[props(default)] on_cancel: Option<EventHandler<()>>) -> Element {
    let app = use_app();
    let email = app.current_user.read().as_ref().map(|u| u.email.clone()).unwrap_or_default();
    let mut current = use_signal(String::new);
    let mut next = use_signal(String::new);
    let mut confirm = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let problem = if next().is_empty() { None } else { password_problem(&next(), &email) };
    let mismatch = !confirm().is_empty() && confirm() != next();
    let ready = !current().is_empty() && !next().is_empty() && problem.is_none() && confirm() == next();
    rsx! {
        form {
            class: "space-y-4",
            "data-testid": "password-form",
            onsubmit: move |e: FormEvent| {
                e.prevent_default();
                if !ready || busy() {
                    return;
                }
                spawn(async move {
                    busy.set(true);
                    error.set(None);
                    let req = ChangePasswordRequest { current_password: current(), new_password: next() };
                    match api::post::<_, User>("/auth/password", &req).await {
                        Ok(u) => {
                            app.success("Password changed. Your other devices were signed out.");
                            on_done.call(u);
                        }
                        Err(e) => error.set(Some(e.message())),
                    }
                    busy.set(false);
                });
            },
            // Lets password managers attach the new password to the right account.
            input { r#type: "email", class: "hidden", autocomplete: "username", value: "{email}", readonly: true }
            div {
                label { class: "label", r#for: "pw-current", "Current password" }
                input { id: "pw-current", class: "field", r#type: "password", autocomplete: "current-password", "data-testid": "pw-current", value: "{current}", oninput: move |e| current.set(e.value()) }
            }
            div {
                label { class: "label", r#for: "pw-new", "New password" }
                input { id: "pw-new", class: "field", r#type: "password", autocomplete: "new-password", "data-testid": "pw-new", value: "{next}", oninput: move |e| next.set(e.value()) }
                p { class: if problem.is_some() { "hint text-tag-red-text" } else { "hint" }, {problem.unwrap_or("At least 10 characters. A short phrase is easy to remember and hard to guess.")} }
            }
            div {
                label { class: "label", r#for: "pw-confirm", "Type it again" }
                input { id: "pw-confirm", class: "field", r#type: "password", autocomplete: "new-password", "data-testid": "pw-confirm", value: "{confirm}", oninput: move |e| confirm.set(e.value()) }
                if mismatch { p { class: "hint text-tag-red-text", "The two passwords don't match yet." } }
            }
            if let Some(msg) = error() {
                p { class: "rounded-md bg-tag-red-bg text-tag-red-text text-sm px-3 py-2", role: "alert", "{msg}" }
            }
            div { class: "flex gap-2",
                if let Some(c) = on_cancel {
                    button { r#type: "button", class: "btn-ghost flex-1 h-10", onclick: move |_| c.call(()), "Cancel" }
                }
                button { r#type: "submit", class: "btn-primary flex-1 h-10", disabled: !ready || busy(), "data-testid": "pw-save",
                    if busy() { Spinner {} }
                    "Save new password"
                }
            }
        }
    }
}

/// Change password and sign out — in the sidebar and the phone menu.
#[component]
pub fn AccountActions(#[props(default)] compact: bool) -> Element {
    let app = use_app();
    let mut changing = use_signal(|| false);
    rsx! {
        div { class: if compact { "flex flex-col items-center gap-1" } else { "flex items-center gap-1" },
            button { class: "btn-icon btn-ghost", title: "Change password", "aria-label": "Change password", "data-testid": "change-password",
                onclick: move |_| changing.set(true), Icon { name: "key" } }
            if compact {
                button { class: "btn-icon btn-ghost", title: "Sign out", "aria-label": "Sign out", "data-testid": "sign-out", onclick: move |_| { spawn(crate::sign_out()); },
                    Icon { name: "log-out" }
                }
            } else {
                button { class: "btn-ghost h-9 px-2.5 ml-auto", "data-testid": "sign-out", onclick: move |_| { spawn(crate::sign_out()); },
                    Icon { name: "log-out" }
                    "Sign out"
                }
            }
        }
        if changing() {
            Modal { title: "Change password", on_close: move |_| changing.set(false),
                PasswordForm {
                    on_done: move |u: User| {
                        let mut current = app.current_user;
                        current.set(Some(u));
                        changing.set(false);
                    },
                    on_cancel: move |_| changing.set(false),
                }
            }
        }
    }
}
