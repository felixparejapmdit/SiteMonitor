//! Small, reusable building blocks in the Notion-like visual language.

use dioxus::prelude::*;
use shared::Tone;

use crate::services::media_canvas;
use crate::state::use_app;

/// Line icons (24×24, 1.75 stroke).
#[component]
pub fn Icon(name: &'static str, #[props(default = 16)] size: u32, #[props(default)] class: String) -> Element {
    let paths: &[&str] = match name {
        "home" => &["M3 10.5 12 3l9 7.5", "M5 9.5V21h14V9.5", "M10 21v-6h4v6"],
        "calendar" => &["M4 6h16v15H4z", "M4 10h16", "M8 3v4", "M16 3v4"],
        "users" => &["M16 20v-1.5A3.5 3.5 0 0 0 12.5 15h-5A3.5 3.5 0 0 0 4 18.5V20", "M10 11a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z", "M20 20v-1.5a3.5 3.5 0 0 0-2.5-3.35", "M15.5 4.2a3.5 3.5 0 0 1 0 6.6"],
        "truck" => &["M3 6h11v10H3z", "M14 10h4l3 3v3h-7", "M7 19a2 2 0 1 0 0-4 2 2 0 0 0 0 4z", "M17 19a2 2 0 1 0 0-4 2 2 0 0 0 0 4z"],
        "alert" => &["M12 3 2 20h20L12 3z", "M12 10v4", "M12 17.5v.01"],
        "map" => &["M9 4 3 6v14l6-2 6 2 6-2V4l-6 2-6-2z", "M9 4v14", "M15 6v14"],
        "archive" => &["M3 5h18v4H3z", "M5 9v11h14V9", "M10 13h4"],
        "check" => &["M5 12.5 10 17l9-10"],
        "x" => &["M6 6l12 12", "M18 6 6 18"],
        "plus" => &["M12 5v14", "M5 12h14"],
        "minus" => &["M5 12h14"],
        "phone" => &["M5 4h4l2 5-2.5 1.5a11 11 0 0 0 5 5L15 13l5 2v4a2 2 0 0 1-2 2A16 16 0 0 1 3 6a2 2 0 0 1 2-2z"],
        "message" => &["M4 5h16v11H8l-4 4V5z"],
        "camera" => &["M4 8h3l2-3h6l2 3h3v11H4z", "M12 17a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z"],
        "chevron-right" => &["M9 6l6 6-6 6"],
        "chevron-left" => &["M15 6l-6 6 6 6"],
        "chevron-down" => &["M6 9l6 6 6-6"],
        "arrow-right" => &["M5 12h14", "M13 6l6 6-6 6"],
        "wifi-off" => &["M3 3l18 18", "M8.5 16.5a5 5 0 0 1 7 0", "M5 13a10 10 0 0 1 5-2.7", "M19 13a10 10 0 0 0-2.2-1.7", "M2 9.5a15 15 0 0 1 4.5-2.8", "M22 9.5A15 15 0 0 0 11 5.6", "M12 20h.01"],
        "refresh" => &["M20 11a8 8 0 0 0-14.9-3.5L4 9", "M4 4v5h5", "M4 13a8 8 0 0 0 14.9 3.5L20 15", "M20 20v-5h-5"],
        "zoom-in" => &["M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14z", "M20 20l-4-4", "M11 8v6", "M8 11h6"],
        "zoom-out" => &["M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14z", "M20 20l-4-4", "M8 11h6"],
        "maximize" => &["M4 9V4h5", "M20 9V4h-5", "M4 15v5h5", "M20 15v5h-5"],
        "layers" => &["M12 3 3 8l9 5 9-5-9-5z", "M3 13l9 5 9-5"],
        "pin" => &["M12 21s-6-5.5-6-11a6 6 0 0 1 12 0c0 5.5-6 11-6 11z", "M12 12a2 2 0 1 0 0-4 2 2 0 0 0 0 4z"],
        "clock" => &["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 7v5l3 2"],
        "wrench" => &["M14.5 6.5a4 4 0 0 0 5 5L12 19a2.1 2.1 0 0 1-3-3l7.5-7.5", "M14.5 6.5 17 4l3 3-2.5 2.5"],
        "battery" => &["M3 8h15v8H3z", "M21 11v2", "M6 11v2"],
        "sparkles" => &["M12 3l1.8 4.7L18.5 9.5l-4.7 1.8L12 16l-1.8-4.7L5.5 9.5l4.7-1.8z", "M19 15l.8 2 2 .8-2 .8-.8 2-.8-2-2-.8 2-.8z"],
        "search" => &["M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14z", "M20 20l-4-4"],
        "move" => &["M12 3v18", "M3 12h18", "M9 6l3-3 3 3", "M9 18l3 3 3-3", "M6 9l-3 3 3 3", "M18 9l3 3-3 3"],
        "link" => &["M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1", "M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1"],
        "unlink" => &["M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7", "M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7", "M4 4l16 16"],
        "upload" => &["M12 16V4", "M7 9l5-5 5 5", "M4 20h16"],
        "edit" => &["M4 20h4L19 9l-4-4L4 16v4z", "M14 6l4 4"],
        "bell" => &["M6 16V11a6 6 0 0 1 12 0v5l2 2H4l2-2z", "M10 20a2 2 0 0 0 4 0"],
        "info" => &["M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z", "M12 11v5", "M12 8h.01"],
        "flag" => &["M5 21V4", "M5 4h11l-2 4 2 4H5"],
        "image" => &["M4 5h16v14H4z", "M4 16l5-5 4 4 3-3 4 4", "M15 9h.01"],
        "hard-hat" => &["M3 18h18", "M5 18v-3a7 7 0 0 1 14 0v3", "M10 8V5h4v3"],
        "moon" => &["M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z"],
        "grip" => &["M9 6h.01", "M15 6h.01", "M9 12h.01", "M15 12h.01", "M9 18h.01", "M15 18h.01"],
        "undo" => &["M9 14 4 9l5-5", "M4 9h11a5 5 0 0 1 0 10h-3"],
        "eye" => &["M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z", "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z"],
        "pen-tool" => &["M12 19l7-7 3 3-7 7-3-3z", "M18 13l-1.5-7.5L2 2l3.5 14.5L13 18l5-5z", "M2 2l7.6 7.6", "M11 13a2 2 0 1 0 0-4 2 2 0 0 0 0 4z"],
        _ => &["M12 12h.01"],
    };
    rsx! {
        svg {
            class: "shrink-0 {class}",
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.75",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            for d in paths.iter() {
                path { d: "{d}" }
            }
        }
    }
}

/// Pastel status tag.
#[component]
pub fn Pill(tone: Tone, #[props(into)] text: String, #[props(default)] dot: bool) -> Element {
    let class = tone.pill_class();
    rsx! {
        span { class: "pill {class}",
            if dot {
                span { class: "inline-block w-1.5 h-1.5 rounded-full bg-current opacity-80" }
            }
            "{text}"
        }
    }
}

#[component]
pub fn ProgressBar(value: u8, #[props(default)] tone: Option<Tone>) -> Element {
    let color = match tone {
        Some(Tone::Amber) => "background:#d97706",
        Some(Tone::Green) => "background:#346538",
        Some(Tone::Red) => "background:#9e2a2b",
        _ => "",
    };
    rsx! {
        div {
            class: "progress-track",
            role: "progressbar",
            "aria-valuenow": "{value}",
            "aria-valuemin": "0",
            "aria-valuemax": "100",
            div { class: "progress-fill", style: "width:{value}%;{color}" }
        }
    }
}

/// Centered dialog with a soft backdrop.
#[component]
pub fn Modal(#[props(into)] title: String, on_close: EventHandler<()>, children: Element, #[props(default)] wide: bool) -> Element {
    let width = if wide { "max-w-2xl" } else { "max-w-lg" };
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-end sm:items-center justify-center p-0 sm:p-6",
            role: "dialog",
            "aria-modal": "true",
            "aria-label": "{title}",
            onkeydown: move |e: KeyboardEvent| {
                if e.key() == Key::Escape {
                    on_close.call(());
                }
            },
            div {
                class: "absolute inset-0 bg-[#0f0f0f]/30 fade-in",
                onclick: move |_| on_close.call(()),
            }
            div { class: "relative w-full {width} max-h-[92vh] overflow-y-auto bg-white rounded-t-2xl sm:rounded-xl shadow-pop scale-in",
                div { class: "sticky top-0 z-10 flex items-center justify-between gap-4 px-5 h-14 bg-white/95 backdrop-blur border-b border-line",
                    h2 { class: "text-base font-semibold truncate", "{title}" }
                    button {
                        class: "btn-icon btn-ghost",
                        "aria-label": "Close",
                        onclick: move |_| on_close.call(()),
                        Icon { name: "x" }
                    }
                }
                div { class: "p-5 safe-bottom", {children} }
            }
        }
    }
}

#[component]
pub fn EmptyState(icon: &'static str, #[props(into)] title: String, #[props(default, into)] body: String) -> Element {
    rsx! {
        div { class: "flex flex-col items-center justify-center text-center py-10 px-6 fade-in",
            div { class: "w-10 h-10 rounded-full bg-hover text-muted flex items-center justify-center mb-3",
                Icon { name: icon, size: 18 }
            }
            p { class: "text-sm font-medium text-ink", "{title}" }
            if !body.is_empty() {
                p { class: "text-sm text-muted mt-1 max-w-sm", "{body}" }
            }
        }
    }
}

/// Placeholder blocks shown only on the very first load.
#[component]
pub fn SkeletonPage() -> Element {
    rsx! {
        div { class: "space-y-4 py-2",
            div { class: "skeleton h-8 w-64" }
            div { class: "skeleton h-4 w-96 max-w-full" }
            div { class: "skeleton h-64 w-full mt-6" }
            div { class: "grid sm:grid-cols-2 gap-3",
                div { class: "skeleton h-24" }
                div { class: "skeleton h-24" }
            }
        }
    }
}

#[component]
pub fn Spinner(#[props(default = 14)] size: u32) -> Element {
    rsx! {
        svg { class: "animate-spin", width: "{size}", height: "{size}", view_box: "0 0 24 24", fill: "none",
            circle { cx: "12", cy: "12", r: "9", stroke: "currentColor", stroke_opacity: "0.2", stroke_width: "3" }
            path { d: "M21 12a9 9 0 0 0-9-9", stroke: "currentColor", stroke_width: "3", stroke_linecap: "round" }
        }
    }
}

/// Camera / gallery button that returns a 16:9-cropped, compressed photo.
#[component]
pub fn PhotoCapture(
    id: String,
    photo: Signal<Option<String>>,
    #[props(default = "Take or choose a photo".to_string(), into)] label: String,
) -> Element {
    let app = use_app();
    let mut busy = use_signal(|| false);
    let input_id = id.clone();
    rsx! {
        div { class: "space-y-2",
            input {
                id: "{id}",
                r#type: "file",
                accept: "image/*",
                capture: "environment",
                class: "sr-only",
                "data-testid": "photo-input",
                onchange: move |_| {
                    let input_id = input_id.clone();
                    let mut photo = photo;
                    async move {
                        busy.set(true);
                        match media_canvas::crop_from_input(&input_id).await {
                            Ok(Some(data)) => photo.set(Some(data)),
                            Ok(None) => {}
                            Err(e) => app.error(e),
                        }
                        busy.set(false);
                    }
                },
            }
            match photo() {
                Some(src) => rsx! {
                    div { class: "relative aspect-video rounded-lg overflow-hidden border border-line bg-hover fade-in",
                        img { src: "{src}", alt: "Photo preview (cropped to 16:9)", class: "w-full h-full object-cover" }
                        div { class: "absolute bottom-2 right-2 flex gap-2",
                            label { r#for: "{id}", class: "btn-secondary h-8 cursor-pointer", Icon { name: "camera" } "Retake" }
                            button {
                                r#type: "button",
                                class: "btn-secondary h-8",
                                onclick: move |_| photo.set(None),
                                Icon { name: "x" }
                                "Remove"
                            }
                        }
                        span { class: "absolute top-2 left-2 pill bg-white/90 text-ink", "16:9" }
                    }
                },
                None => rsx! {
                    label {
                        r#for: "{id}",
                        class: "flex items-center justify-center gap-2 aspect-[16/6] rounded-lg border border-dashed border-[#d9d9d6] bg-sidebar text-sm text-muted cursor-pointer hover:bg-hover transition-colors",
                        if busy() {
                            Spinner {}
                            "Preparing photo…"
                        } else {
                            Icon { name: "camera", size: 18 }
                            "{label}"
                        }
                    }
                },
            }
        }
    }
}

/// Numbered avatar circle with initials.
#[component]
pub fn Avatar(#[props(into)] name: String, #[props(default = 28)] size: u32) -> Element {
    let initials: String = name
        .split_whitespace()
        .filter(|w| w.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false))
        .take(2)
        .filter_map(|w| w.chars().next())
        .collect();
    let font = size as f32 * 0.4;
    rsx! {
        span {
            class: "inline-flex items-center justify-center rounded-full bg-hover text-ink font-medium shrink-0",
            style: "width:{size}px;height:{size}px;font-size:{font}px",
            "{initials}"
        }
    }
}
