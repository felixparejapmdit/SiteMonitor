//! Interactive 16:9 photo viewer with the tap-to-pin call-out loop.
//!
//! * The owner taps anywhere on the photo → an amber pin appears at
//!   (x%, y%) with an inline note box → staff are alerted instantly.
//! * Staff open the same photo, see the pin in the exact same place, reply
//!   and mark it resolved → the owner's pin turns green.

use dioxus::prelude::*;
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{Icon, Pill, Spinner};
use crate::services::sync::{self, Outcome};
use crate::state::{use_app, AppState};
use crate::util;

fn pin_class(status: CallOutStatus) -> &'static str {
    match status {
        CallOutStatus::Open => "pin pin-open",
        CallOutStatus::InReview => "pin pin-review",
        CallOutStatus::Resolved => "pin pin-resolved",
    }
}

/// Updates the cached project so a change shows instantly.
fn patch_callout(app: AppState, work_item_id: Uuid, callout: PhotoCallOut) {
    let mut details = app.details;
    let mut map = details.write();
    if let Some(detail) = map.get_mut(&work_item_id) {
        if let Some(p) = detail.photos.iter_mut().find(|p| p.photo.id == callout.photo_id) {
            match p.callouts.iter_mut().find(|c| c.id == callout.id) {
                Some(existing) => *existing = callout,
                None => p.callouts.push(callout),
            }
        }
    }
    drop(map);
}

#[component]
pub fn PhotoAnnotator(
    photo: PhotoWithCallOuts,
    work_item_id: Uuid,
    /// Hint shown to the owner above the photo.
    #[props(default)]
    owner_hint: Option<String>,
) -> Element {
    let app = use_app();
    let is_vip = app.is_vip();
    let is_staff = app.is_staff();
    let mut draft = use_signal(|| None::<(f32, f32)>);
    let mut comment = use_signal(String::new);
    let mut selected = use_signal(|| None::<Uuid>);
    let mut reply = use_signal(String::new);
    let mut busy = use_signal(|| false);

    let photo_id = photo.photo.id;
    let stage_id = format!("photo-{photo_id}");
    let stage_for_click = stage_id.clone();
    let callouts = photo.callouts.clone();
    let open_count = callouts.iter().filter(|c| c.status != CallOutStatus::Resolved).count();
    let selected_callout = selected().and_then(|id| callouts.iter().find(|c| c.id == id).cloned());
    let uploader = app.user_name(photo.photo.uploaded_by);

    let submit_callout = move |_| async move {
        let Some((x, y)) = draft() else { return };
        let text = comment().trim().to_string();
        if text.is_empty() {
            app.error("Please write a short note for the site team.");
            return;
        }
        busy.set(true);
        let req = CreateCallOutRequest { x_percent: x, y_percent: y, comment: text };
        match api::post::<_, PhotoCallOut>(&format!("/photos/{photo_id}/callouts"), &req).await {
            Ok(c) => {
                let id = c.id;
                patch_callout(app, work_item_id, c);
                draft.set(None);
                comment.set(String::new());
                selected.set(Some(id));
                app.success("Note sent to the site team");
                sync::reload(app, vec![DataScope::Photos, DataScope::Map]);
            }
            Err(e) => app.error(e.message()),
        }
        busy.set(false);
    };

    let resolve = move |callout: PhotoCallOut| async move {
        let text = reply().trim().to_string();
        if text.is_empty() {
            app.error("Please tell the owner what was done.");
            return;
        }
        busy.set(true);
        let req = ResolveCallOutRequest { reply: text.clone() };
        let outcome: Outcome<PhotoCallOut> =
            sync::mutate(app, format!("Reply to owner: {}", callout.client_comment), "PUT", format!("/callouts/{}/resolve", callout.id), &req).await;
        match outcome {
            Outcome::Saved(c) => {
                patch_callout(app, work_item_id, c);
                reply.set(String::new());
                app.success("Marked resolved — the owner can see your reply");
                sync::reload(app, vec![DataScope::Photos, DataScope::Map]);
            }
            Outcome::Queued => {
                let mut c = callout.clone();
                c.status = CallOutStatus::Resolved;
                c.staff_reply = Some(text);
                patch_callout(app, work_item_id, c);
                reply.set(String::new());
            }
            Outcome::Failed(e) => app.error(e),
        }
        busy.set(false);
    };

    let mark_review = move |callout: PhotoCallOut| async move {
        let outcome: Outcome<PhotoCallOut> =
            sync::mutate(app, "Photo note: checking", "PUT", format!("/callouts/{}/review", callout.id), &serde_json::json!({})).await;
        if let Outcome::Saved(c) = outcome {
            patch_callout(app, work_item_id, c);
        }
    };

    rsx! {
        figure { class: "space-y-3", "data-testid": "photo-annotator",
            if is_vip {
                p { class: "text-sm text-muted flex items-center gap-1.5",
                    Icon { name: "pin" }
                    {owner_hint.clone().unwrap_or_else(|| "Tap anywhere on the photo to leave a note for the site team.".into())}
                }
            }
            div {
                id: "{stage_id}",
                class: if is_vip { "relative aspect-video rounded-lg overflow-hidden bg-hover cursor-crosshair select-none" } else { "relative aspect-video rounded-lg overflow-hidden bg-hover select-none" },
                "data-testid": "photo-stage",
                onclick: move |e: MouseEvent| {
                    if !is_vip {
                        return;
                    }
                    let c = e.client_coordinates();
                    if let Some((x, y)) = util::percent_in(&stage_for_click, c.x, c.y) {
                        draft.set(Some((x, y)));
                        selected.set(None);
                    }
                },
                img {
                    src: "{photo.photo.image_url}",
                    alt: "{photo.photo.caption}",
                    class: "absolute inset-0 w-full h-full object-cover pointer-events-none",
                    draggable: "false",
                    loading: "lazy",
                }
                for c in callouts.iter().cloned() {
                    button {
                        key: "{c.id}",
                        class: if selected() == Some(c.id) { format!("{} ring-2 ring-white scale-110", pin_class(c.status)) } else { pin_class(c.status).to_string() },
                        style: "left:{c.x_percent}%;top:{c.y_percent}%;",
                        "data-testid": "pin-{c.status:?}",
                        "aria-label": "Photo note: {c.client_comment}",
                        onclick: move |e: MouseEvent| {
                            e.stop_propagation();
                            draft.set(None);
                            selected.set(if selected() == Some(c.id) { None } else { Some(c.id) });
                        },
                    }
                }
                if let Some((x, y)) = draft() {
                    span { class: "pin pin-draft pointer-events-none", style: "left:{x}%;top:{y}%;" }
                }
            }
            figcaption { class: "flex flex-wrap items-center gap-x-3 gap-y-1 text-sm",
                span { class: "font-medium", "{photo.photo.caption}" }
                span { class: "text-muted", "{uploader} · {util::day_time(photo.photo.uploaded_at)}" }
                if open_count > 0 {
                    Pill { tone: Tone::Amber, text: util::plural(open_count, "open note") }
                }
            }

            // ------------------------------------------------ owner: new note
            if draft().is_some() {
                div { class: "card p-4 scale-in", "data-testid": "callout-popover",
                    textarea {
                        class: "field",
                        rows: "3",
                        placeholder: "Add a call-out note for the site team...",
                        "data-testid": "callout-text",
                        autofocus: true,
                        value: "{comment}",
                        oninput: move |e| comment.set(e.value()),
                    }
                    div { class: "flex justify-end gap-2 mt-3",
                        button { class: "btn-ghost", onclick: move |_| { draft.set(None); comment.set(String::new()); }, "Cancel" }
                        button {
                            class: "btn-primary",
                            disabled: busy() || comment().trim().is_empty(),
                            "data-testid": "submit-callout",
                            onclick: submit_callout,
                            if busy() { Spinner {} }
                            "Submit Call-Out"
                        }
                    }
                }
            }

            // ------------------------------------------------ selected note
            if let Some(c) = selected_callout {
                div { class: "card p-4 scale-in", "data-testid": "callout-detail",
                    div { class: "flex items-center gap-2 mb-2",
                        Pill { tone: c.status.tone(), text: c.status.label().to_string(), dot: true }
                        span { class: "text-xs text-muted ml-auto", "{util::relative(c.created_at)}" }
                    }
                    p { class: "eyebrow", "Owner's note" }
                    p { class: "text-[15px] mt-0.5", "{c.client_comment}" }
                    if let Some(r) = c.staff_reply.clone() {
                        div { class: "mt-3 rounded-md bg-tag-green-bg/60 px-3 py-2.5 fade-in",
                            p { class: "eyebrow text-tag-green-text", "Site team's reply" }
                            p { class: "text-[15px] mt-0.5", "data-testid": "staff-reply", "{r}" }
                        }
                    }
                    if is_staff && c.status != CallOutStatus::Resolved {
                        div { class: "mt-4 space-y-3",
                            textarea {
                                class: "field",
                                rows: "3",
                                placeholder: "What did you find and what was done? e.g. \"Just runoff from the tool wash station, redirecting drainage now\"",
                                "data-testid": "reply-text",
                                value: "{reply}",
                                oninput: move |e| reply.set(e.value()),
                            }
                            div { class: "flex flex-wrap justify-end gap-2",
                                if c.status == CallOutStatus::Open {
                                    {
                                        let c2 = c.clone();
                                        rsx! {
                                            button { class: "btn-secondary", onclick: move |_| mark_review(c2.clone()), Icon { name: "eye" } "We're checking" }
                                        }
                                    }
                                }
                                {
                                    let c3 = c.clone();
                                    rsx! {
                                        button {
                                            class: "btn-green",
                                            disabled: busy() || reply().trim().is_empty(),
                                            "data-testid": "mark-resolved",
                                            onclick: move |_| resolve(c3.clone()),
                                            Icon { name: "check" }
                                            "Mark Resolved"
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
