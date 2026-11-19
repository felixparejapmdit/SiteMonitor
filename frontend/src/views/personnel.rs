//! Personnel (merged from Service 5 DB): every worker and volunteer, plus the
//! workforce data tools (legacy import, backup download, snapshots).

use dioxus::prelude::*;
use shared::api::*;
use shared::*;
use uuid::Uuid;
use wasm_bindgen::JsCast;

use crate::api;
use crate::components::ui::{EmptyState, Icon, Modal, Pill, Spinner};
use crate::services::media_canvas;
use crate::state::use_app;
use crate::util;

/// Bumped after any change so the list reloads.
pub static REFRESH: GlobalSignal<u32> = Signal::global(|| 0);

#[derive(Clone, Copy, PartialEq)]
enum Filter {
    All,
    Status(WorkerStatus),
    Archived,
}

#[component]
pub fn PersonnelView() -> Element {
    let app = use_app();
    let can_edit = app.can(Permission::ManageWorkforce);
    let workers = use_resource(|| async {
        let _ = REFRESH();
        api::get::<Vec<Worker>>("/workers").await
    });
    let mut query = use_signal(String::new);
    let mut filter = use_signal(|| Filter::All);
    let mut editing = use_signal(|| None::<Option<Worker>>);
    let mut reassigning = use_signal(|| None::<Worker>);

    let all = match &*workers.read() {
        Some(Ok(w)) => w.clone(),
        Some(Err(e)) => return rsx! { div { class: "card", EmptyState { icon: "users", title: "Couldn't load personnel", body: e.message() } } },
        None => return rsx! { div { class: "card p-8 flex justify-center", Spinner { size: 20 } } },
    };
    let active: Vec<&Worker> = all.iter().filter(|w| !w.archived).collect();
    let count = |s: WorkerStatus| active.iter().filter(|w| w.status == s).count();
    let q = query().trim().to_lowercase();
    let shown: Vec<Worker> = all
        .iter()
        .filter(|w| match filter() {
            Filter::All => !w.archived,
            Filter::Status(s) => !w.archived && w.status == s,
            Filter::Archived => w.archived,
        })
        .filter(|w| {
            q.is_empty()
                || [&w.name, &w.a_number, &w.trade, &w.area, &w.department, &w.project_group].iter().any(|f| f.to_lowercase().contains(&q))
        })
        .cloned()
        .collect();
    let chips = [
        (Filter::All, "Total active", active.len()),
        (Filter::Status(WorkerStatus::Present), "Present", count(WorkerStatus::Present)),
        (Filter::Status(WorkerStatus::Absent), "Absent", count(WorkerStatus::Absent)),
        (Filter::Status(WorkerStatus::OnPass), "On pass", count(WorkerStatus::OnPass)),
        (Filter::Archived, "Archived", all.len() - active.len()),
    ];

    rsx! {
        div { class: "space-y-6 rise-in",
            header { class: "flex flex-wrap items-end justify-between gap-4",
                div { class: "space-y-1",
                    h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Personnel" }
                    p { class: "text-[15px] text-muted", "Every worker and volunteer, with today's status. Press Ctrl+K anywhere to search." }
                }
                if can_edit {
                    button { class: "btn-primary h-10 px-4", "data-testid": "add-worker", onclick: move |_| editing.set(Some(None)),
                        Icon { name: "plus" }
                        "Add worker"
                    }
                }
            }
            div { class: "grid grid-cols-2 sm:grid-cols-5 gap-3", "data-testid": "workforce-counts",
                for (f, label, n) in chips {
                    button {
                        key: "{label}",
                        class: if filter() == f { "card p-3 text-left ring-2 ring-accent" } else { "card p-3 text-left hover:bg-hover transition-colors" },
                        "aria-pressed": filter() == f,
                        onclick: move |_| filter.set(f),
                        p { class: "text-xs text-muted", "{label}" }
                        p { class: "text-2xl font-semibold tabular", "data-testid": "count-{label}", "{n}" }
                    }
                }
            }
            input {
                id: "worker-search",
                class: "field",
                "data-testid": "worker-search",
                r#type: "search",
                autofocus: true,
                placeholder: "Search by name, A#, trade, area or group",
                value: "{query}",
                oninput: move |e| query.set(e.value()),
            }
            if all.is_empty() {
                div { class: "card",
                    EmptyState { icon: "users", title: "No personnel yet",
                        body: if can_edit { "Add people one by one, or import a Service 5 backup in Settings → Workforce data." } else { "The office hasn't added anyone yet." } }
                }
            } else if shown.is_empty() {
                div { class: "card", EmptyState { icon: "search", title: "Nobody matches", body: "Try another word, or another filter." } }
            } else {
                ul { class: "card divide-y divide-line", "data-testid": "worker-list",
                    for w in shown {
                        li {
                            key: "{w.id}",
                            class: "flex flex-wrap items-center gap-3 px-4 py-3 focus:outline-none focus-visible:bg-hover",
                            "data-testid": "worker-row",
                            tabindex: "0",
                            // Shift+R on a focused row reassigns that person (Service 5's shortcut).
                            onkeydown: {
                                let w = w.clone();
                                move |e: KeyboardEvent| {
                                    if e.modifiers().shift() && matches!(e.key(), Key::Character(ref c) if c.eq_ignore_ascii_case("r")) {
                                        e.prevent_default();
                                        reassigning.set(Some(w.clone()));
                                    }
                                }
                            },
                            div { class: "flex-1 min-w-0",
                                p { class: "font-medium truncate",
                                    "{w.name}"
                                    if !w.a_number.is_empty() { span { class: "text-muted font-normal ml-2 tabular", "A# {w.a_number}" } }
                                }
                                p { class: "text-sm text-muted truncate",
                                    {[w.trade.as_str(), w.area.as_str(), w.project_group.as_str()].iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" · ")}
                                }
                            }
                            if w.archived {
                                Pill { tone: Tone::Gray, text: "Archived" }
                            } else {
                                span { class: "sr-only", "{w.status.label()}" }
                                crate::views::workforce_ops::StatusButtons { worker: w.clone() }
                                {
                                    let w3 = w.clone();
                                    rsx! { button { class: "btn-icon btn-ghost", "aria-label": "Reassign {w.name} (Shift+R)", title: "Reassign (Shift+R)", "data-testid": "reassign-worker", onclick: move |_| reassigning.set(Some(w3.clone())), Icon { name: "move" } } }
                                }
                            }
                            if can_edit {
                                {
                                    let w2 = w.clone();
                                    rsx! { button { class: "btn-icon btn-ghost", "aria-label": "Edit {w.name}", "data-testid": "edit-worker", onclick: move |_| editing.set(Some(Some(w2.clone()))), Icon { name: "edit" } } }
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(target) = editing() {
            WorkerModal { worker: target, on_close: move |_| editing.set(None) }
        }
        if let Some(w) = reassigning() {
            crate::views::workforce_ops::ReassignModal { worker: w, on_close: move |_| reassigning.set(None) }
        }
    }
}

#[component]
fn WorkerModal(worker: Option<Worker>, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let id = worker.as_ref().map(|w| w.id);
    let start = worker.map(|w| SaveWorkerRequest {
        a_number: w.a_number,
        name: w.name,
        phone: w.phone,
        trade: w.trade,
        department: w.department,
        area: w.area,
        project_group: w.project_group,
        group_role: w.group_role,
        status: w.status,
        archived: w.archived,
    });
    let mut form = use_signal(|| start.unwrap_or_default());
    let mut busy = use_signal(|| false);
    let f = form();
    let text = move |key: &'static str, label: &'static str, value: String, set: fn(&mut SaveWorkerRequest, String)| {
        rsx! {
            div {
                label { class: "label", r#for: "w-{key}", "{label}" }
                input { id: "w-{key}", class: "field", "data-testid": "worker-{key}", value: "{value}", oninput: move |e| set(&mut form.write(), e.value()) }
            }
        }
    };
    let save = move |_| async move {
        busy.set(true);
        let result: Result<Worker, _> = match id {
            Some(id) => api::put(&format!("/workers/{id}"), &form()).await,
            None => api::post("/workers", &form()).await,
        };
        busy.set(false);
        match result {
            Ok(w) => {
                app.success(format!("{} saved", w.name));
                *REFRESH.write() += 1;
                on_close.call(());
            }
            Err(e) => app.error(e.message()),
        }
    };
    rsx! {
        Modal { title: if id.is_some() { "Edit worker" } else { "Add worker" }, wide: true, on_close: move |_| on_close.call(()),
            div { class: "space-y-4",
                div { class: "grid sm:grid-cols-2 gap-4",
                    {text("name", "Name", f.name.clone(), |r, v| r.name = v)}
                    {text("a-number", "A# (worker number)", f.a_number.clone(), |r, v| r.a_number = v)}
                    {text("phone", "Phone", f.phone.clone(), |r, v| r.phone = v)}
                    {text("trade", "Trade", f.trade.clone(), |r, v| r.trade = v)}
                    {text("department", "Department", f.department.clone(), |r, v| r.department = v)}
                    {text("area", "Area", f.area.clone(), |r, v| r.area = v)}
                    {text("group", "Project group", f.project_group.clone(), |r, v| r.project_group = v)}
                    {text("role", "Role in group", f.group_role.clone(), |r, v| r.group_role = v)}
                }
                div {
                    p { class: "label", "Today" }
                    div { class: "flex flex-wrap gap-2",
                        for s in WorkerStatus::ALL {
                            button { key: "{s:?}", r#type: "button", class: if f.status == s { "btn-primary" } else { "btn-secondary" }, "data-testid": "worker-status-{s:?}",
                                onclick: move |_| form.write().status = s, "{s.label()}" }
                        }
                    }
                }
                label { class: "flex items-center gap-2 text-sm",
                    input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "worker-archived", checked: f.archived, onchange: move |e| form.write().archived = e.checked() }
                    "Archived (no longer working here; kept for the records)"
                }
                div { class: "modal-actions",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button { class: "btn-primary", "data-testid": "save-worker", disabled: busy() || f.name.trim().is_empty(), onclick: save,
                        if busy() { Spinner {} }
                        if id.is_some() { "Save changes" } else { "Add worker" }
                    }
                }
            }
        }
    }
}

/// Settings → Workforce data: import any Service 5 backup, download a backup,
/// take and restore snapshots.
#[component]
pub fn WorkforceData() -> Element {
    let app = use_app();
    let mut snapshots = use_resource(|| async { api::get::<Vec<WorkforceSnapshotInfo>>("/workforce/snapshots").await.unwrap_or_default() });
    let mut passphrase = use_signal(String::new);
    let mut replace = use_signal(|| false);
    let mut busy = use_signal(|| None::<&'static str>);
    let mut report = use_signal(|| None::<WorkforceImportReport>);
    let mut confirm_restore = use_signal(|| None::<Uuid>);

    let import = move |_| async move {
        let file = match media_canvas::backup_file("workforce-file", 500.0 * 1_048_576.0) {
            Ok(Some(f)) => f,
            Ok(None) => return,
            Err(e) => return app.error(e),
        };
        busy.set(Some("import"));
        let mode = if replace() { "Replace" } else { "Merge" };
        let path = format!("/workforce/import?mode={mode}&file_name={}", js_sys::encode_uri_component(&file.name()));
        let pass = passphrase();
        // Hex keeps any passphrase (accents, emoji) safe inside an HTTP header.
        let headers: Vec<(&str, String)> =
            if pass.is_empty() { vec![] } else { vec![("x-backup-passphrase", pass.bytes().map(|b| format!("{b:02x}")).collect())] };
        match api::upload(&path, &file, &headers).await.and_then(|t| {
            serde_json::from_str::<WorkforceImportReport>(&t).map_err(|_| api::ApiFail::Server(0, "Unexpected reply from the server.".into()))
        }) {
            Ok(r) => {
                app.success(format!("Imported {} workers", r.workers));
                report.set(Some(r));
                *REFRESH.write() += 1;
                snapshots.restart();
            }
            Err(e) => app.error(e.message()),
        }
        busy.set(None);
    };
    let download = move |_| async move {
        busy.set(Some("export"));
        match api::send_raw("GET", "/workforce/export", None).await {
            Ok(text) => save_file(&format!("sitemonitor-workforce-{}.json", chrono::Local::now().format("%Y-%m-%d")), &text),
            Err(e) => app.error(e.message()),
        }
        busy.set(None);
    };
    let take = move |_| async move {
        busy.set(Some("snapshot"));
        match api::post::<_, Vec<WorkforceSnapshotInfo>>("/workforce/snapshots", &serde_json::json!({})).await {
            Ok(_) => {
                app.success("Snapshot saved");
                snapshots.restart();
            }
            Err(e) => app.error(e.message()),
        }
        busy.set(None);
    };

    let list = snapshots.read().clone().unwrap_or_default();
    rsx! {
        div { class: "space-y-5", "data-testid": "workforce-data",
            div { class: "space-y-3",
                p { class: "text-sm font-medium", "Import a Service 5 backup" }
                p { class: "text-sm text-muted", "Any Service 5 JSON backup (old or new), an encrypted .agwcz backup, a service5.db file, or a backup downloaded here. A snapshot is taken first, so an import can always be undone. For service5.db, close Service 5 first so its latest changes are saved into the file." }
                div { class: "grid sm:grid-cols-2 gap-3 max-w-xl",
                    div {
                        label { class: "label", r#for: "workforce-pass", "Passphrase (encrypted backups only)" }
                        input { id: "workforce-pass", r#type: "password", class: "field", "data-testid": "workforce-passphrase", autocomplete: "off", value: "{passphrase}", oninput: move |e| passphrase.set(e.value()) }
                    }
                    label { class: "flex items-start gap-2 text-sm self-end pb-2",
                        input { r#type: "checkbox", class: "w-4 h-4 mt-0.5 accent-[#064e3b]", "data-testid": "workforce-replace", checked: replace(), onchange: move |e| replace.set(e.checked()) }
                        span { "Replace instead of merge" span { class: "block text-xs text-muted", "Parts that are empty in the file never wipe your data." } }
                    }
                }
                input { id: "workforce-file", r#type: "file", class: "hidden", accept: ".json,.agwcz,.db,application/json", "data-testid": "workforce-file", onchange: import }
                div { class: "flex flex-wrap gap-2",
                    label { r#for: "workforce-file", class: "btn-primary cursor-pointer", "data-testid": "workforce-import",
                        if busy() == Some("import") { Spinner {} } else { Icon { name: "upload" } }
                        "Choose backup file"
                    }
                    button { class: "btn-secondary", "data-testid": "workforce-export", disabled: busy().is_some(), onclick: download,
                        if busy() == Some("export") { Spinner {} } else { Icon { name: "archive" } }
                        "Download backup"
                    }
                }
                if let Some(r) = report() {
                    div { class: "rounded-md bg-tag-green-bg text-tag-green-text text-sm px-4 py-3 space-y-1", "data-testid": "import-report",
                        p { class: "font-medium", "{r.detected}: {util::plural(r.workers, \"worker\")} imported" }
                        if !r.collections.is_empty() {
                            p { {r.collections.iter().map(|(c, n)| format!("{} {}", n, c.replace('_', " "))).collect::<Vec<_>>().join(" · ")} }
                        }
                        if !r.kept_existing.is_empty() {
                            p { "Kept as they were (empty in the file): " {r.kept_existing.join(", ").replace('_', " ")} }
                        }
                    }
                }
            }
            div { class: "space-y-3 border-t border-line pt-4",
                div { class: "flex flex-wrap items-center justify-between gap-2",
                    div {
                        p { class: "text-sm font-medium", "Snapshots" }
                        p { class: "text-sm text-muted", "Saved every night at the crew reset, before each import, and whenever you take one. The last 14 are kept." }
                    }
                    button { class: "btn-secondary", "data-testid": "take-snapshot", disabled: busy().is_some(), onclick: take,
                        if busy() == Some("snapshot") { Spinner {} } else { Icon { name: "plus" } }
                        "Take snapshot now"
                    }
                }
                if list.is_empty() {
                    p { class: "text-sm text-muted", "No snapshots yet." }
                }
                ul { class: "divide-y divide-line border-y border-line",
                    for s in list {
                        li { key: "{s.id}", class: "flex flex-wrap items-center gap-3 py-2 text-sm", "data-testid": "snapshot-row",
                            span { class: "flex-1 min-w-0", "{util::day(s.taken_at)} · {util::time(s.taken_at)}" span { class: "text-muted ml-2", "{s.reason} · {util::plural(s.workers as usize, \"worker\")}" } }
                            if confirm_restore() == Some(s.id) {
                                span { class: "text-tag-red-text", "Put the workforce data back as it was then?" }
                                button { class: "btn-ghost", onclick: move |_| confirm_restore.set(None), "Keep current" }
                                button { class: "btn-danger", "data-testid": "restore-confirm", disabled: busy().is_some(),
                                    onclick: move |_| async move {
                                        busy.set(Some("restore"));
                                        match api::post::<_, WorkforceImportReport>(&format!("/workforce/snapshots/{}/restore", s.id), &serde_json::json!({})).await {
                                            Ok(r) => {
                                                app.success(format!("Restored: {}", util::plural(r.workers, "worker")));
                                                *REFRESH.write() += 1;
                                                snapshots.restart();
                                            }
                                            Err(e) => app.error(e.message()),
                                        }
                                        confirm_restore.set(None);
                                        busy.set(None);
                                    },
                                    "Restore"
                                }
                            } else {
                                button { class: "btn-ghost", "data-testid": "restore-snapshot", onclick: move |_| confirm_restore.set(Some(s.id)), Icon { name: "undo" } "Restore" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Saves text as a downloaded file.
fn save_file(name: &str, text: &str) {
    let parts = js_sys::Array::of1(&wasm_bindgen::JsValue::from_str(text));
    let Ok(blob) = web_sys::Blob::new_with_str_sequence(&parts) else { return };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else { return };
    let doc = util::document();
    if let Some(a) = doc.create_element("a").ok().and_then(|a| a.dyn_into::<web_sys::HtmlElement>().ok()) {
        let _ = a.set_attribute("href", &url);
        let _ = a.set_attribute("download", name);
        a.click();
    }
    let _ = web_sys::Url::revoke_object_url(&url);
}
