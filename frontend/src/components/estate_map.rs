//! Interactive estate map: zoomable / pannable base render, SVG polygon
//! overlay coloured by live status, infinite drill-down with breadcrumbs,
//! and the Front-of-House vs Infrastructure layer toggle.

use dioxus::prelude::*;
use shared::*;
use uuid::Uuid;

use crate::components::ui::{Icon, Pill};
use crate::state::{use_app, MapLayer};
use crate::util::{self, FlatArea};
use crate::Route;

const STAGE_ID: &str = "estate-map-stage";

fn poly_class(status: AreaStatus) -> &'static str {
    match status {
        AreaStatus::Optimal => "poly poly-optimal",
        AreaStatus::ActiveWork => "poly poly-active",
        AreaStatus::AttentionRequired => "poly poly-attention",
        AreaStatus::CriticalAlert => "poly poly-critical",
        AreaStatus::Archived => "poly poly-archived",
    }
}

fn in_layer(class: ZoneClass, layer: MapLayer) -> bool {
    match layer {
        MapLayer::FrontOfHouse => class.is_front_of_house(),
        MapLayer::Infrastructure => !class.is_front_of_house(),
    }
}

/// Base zoom/translate that fits the active parent's bounding box.
fn fit(parent: Option<&FlatArea>) -> (f32, f32, f32) {
    let Some(p) = parent else { return (1.0, 0.0, 0.0) };
    let (x1, y1, x2, y2) = util::bbox(&p.area.polygon_points);
    let (w, h) = ((x2 - x1).max(1.0), (y2 - y1).max(1.0));
    let s = (100.0 / w).min(100.0 / h) * 0.8;
    let s = s.clamp(1.0, 8.0);
    let (cx, cy) = ((x1 + x2) / 2.0, (y1 + y2) / 2.0);
    (s, 50.0 - cx * s, 50.0 - cy * s)
}

#[component]
pub fn EstateMap() -> Element {
    let app = use_app();
    let mut active_parent = app.active_parent_area;
    let mut layer = app.map_layer;
    let mut zoom = use_signal(|| 1.0f32);
    let mut pan = use_signal(|| (0.0f32, 0.0f32));
    let mut drag = use_signal(|| None::<(f64, f64, f32, f32)>);
    let mut moved = use_signal(|| false);
    let mut hovered = use_signal(|| None::<Uuid>);

    let Some(map) = app.map.read().clone() else {
        return rsx! { div { class: "skeleton aspect-[16/10] w-full" } };
    };
    let flat = util::flatten(&map);
    let parent_id = active_parent();
    let parent = parent_id.and_then(|id| util::find(&flat, id).cloned());
    let current_layer = layer();

    let mut visible: Vec<FlatArea> = flat
        .iter()
        .filter(|a| a.area.parent_id == parent_id && !a.area.is_archived)
        .filter(|a| parent_id.is_some() || in_layer(a.area.zone_class, current_layer))
        .cloned()
        .collect();
    let leaf_focus = parent.is_some() && visible.is_empty();
    if leaf_focus {
        if let Some(p) = parent.clone() {
            visible.push(p);
        }
    }

    let (base_s, base_tx, base_ty) = fit(parent.as_ref());
    let z = zoom();
    let (px, py) = pan();
    let scale = base_s * z;
    let tx = 50.0 + z * (base_tx - 50.0) + px;
    let ty = 50.0 + z * (base_ty - 50.0) + py;
    let label_scale = 1.0 / scale;
    let dragging = drag().is_some();
    let breadcrumbs = parent_id.map(|id| util::path_to(&flat, id)).unwrap_or_default();
    let hovered_area = hovered().and_then(|id| util::find(&flat, id).cloned());

    let mut reset_view = move || {
        zoom.set(1.0);
        pan.set((0.0, 0.0));
    };

    rsx! {
        div { class: "space-y-3",
            // ------------------------------------------------ breadcrumbs + layers
            div { class: "flex flex-wrap items-center justify-between gap-3",
                nav { class: "flex items-center flex-wrap gap-1 text-sm min-h-8", "aria-label": "Map location",
                    button {
                        class: if parent_id.is_none() { "px-1.5 py-1 rounded font-medium text-ink" } else { "px-1.5 py-1 rounded text-muted hover:bg-hover hover:text-ink transition-colors" },
                        "data-testid": "crumb-root",
                        onclick: move |_| {
                            active_parent.set(None);
                            reset_view();
                        },
                        "Entire Estate"
                    }
                    for (i, crumb) in breadcrumbs.iter().cloned().enumerate() {
                        span { key: "{crumb.area.id}", class: "contents",
                            span { class: "text-muted/60", "/" }
                            button {
                                class: if i + 1 == breadcrumbs.len() { "px-1.5 py-1 rounded font-medium text-ink" } else { "px-1.5 py-1 rounded text-muted hover:bg-hover hover:text-ink transition-colors" },
                                onclick: move |_| {
                                    active_parent.set(Some(crumb.area.id));
                                    reset_view();
                                },
                                "{crumb.area.name}"
                            }
                        }
                    }
                }
                div { class: "inline-flex p-0.5 rounded-lg bg-hover text-sm", role: "tablist", "aria-label": "Map layer",
                    button {
                        role: "tab",
                        "aria-selected": current_layer == MapLayer::FrontOfHouse,
                        "data-testid": "layer-foh",
                        class: if current_layer == MapLayer::FrontOfHouse { "px-3 h-8 rounded-md bg-white shadow-sm font-medium" } else { "px-3 h-8 rounded-md text-muted hover:text-ink" },
                        onclick: move |_| {
                            if layer() != MapLayer::FrontOfHouse {
                                layer.set(MapLayer::FrontOfHouse);
                                active_parent.set(None);
                                reset_view();
                            }
                        },
                        "Front-of-House View"
                    }
                    button {
                        role: "tab",
                        "aria-selected": current_layer == MapLayer::Infrastructure,
                        "data-testid": "layer-infra",
                        class: if current_layer == MapLayer::Infrastructure { "px-3 h-8 rounded-md bg-white shadow-sm font-medium" } else { "px-3 h-8 rounded-md text-muted hover:text-ink" },
                        onclick: move |_| {
                            if layer() != MapLayer::Infrastructure {
                                layer.set(MapLayer::Infrastructure);
                                active_parent.set(None);
                                reset_view();
                            }
                        },
                        span { class: "hidden sm:inline", "Infrastructure & Operations Layer" }
                        span { class: "sm:hidden", "Infrastructure" }
                    }
                }
            }

            // ------------------------------------------------ the map
            div {
                id: "{STAGE_ID}",
                class: "relative w-full aspect-[16/10] rounded-xl overflow-hidden border border-line bg-[#e9ede4] select-none touch-none contain-paint",
                style: if dragging { "cursor:grabbing" } else { "cursor:grab" },
                "data-testid": "estate-map",
                onpointerdown: move |e: PointerEvent| {
                    let c = e.client_coordinates();
                    let (px, py) = pan();
                    drag.set(Some((c.x, c.y, px, py)));
                    moved.set(false);
                },
                onpointermove: move |e: PointerEvent| {
                    if let Some((sx, sy, ox, oy)) = drag() {
                        let c = e.client_coordinates();
                        let (dx, dy) = (c.x - sx, c.y - sy);
                        if dx.abs() + dy.abs() > 4.0 {
                            moved.set(true);
                        }
                        if let Some((_, _, w, h)) = util::rect_of(STAGE_ID) {
                            let limit = 50.0 * zoom();
                            let nx = (ox + (dx / w * 100.0) as f32).clamp(-limit, limit);
                            let ny = (oy + (dy / h * 100.0) as f32).clamp(-limit, limit);
                            pan.set((nx, ny));
                        }
                    }
                },
                onpointerup: move |_| drag.set(None),
                onpointerleave: move |_| {
                    drag.set(None);
                    hovered.set(None);
                },
                onwheel: move |e: WheelEvent| {
                    let m = e.modifiers();
                    if m.ctrl() || m.meta() {
                        e.prevent_default();
                        let dy = e.delta().strip_units().y;
                        let next = if dy < 0.0 { zoom() * 1.12 } else { zoom() / 1.12 };
                        zoom.set(next.clamp(1.0, 4.0));
                    }
                },
                div {
                    class: if dragging { "map-stage dragging absolute inset-0" } else { "map-stage absolute inset-0" },
                    style: "transform: translate({tx}%, {ty}%) scale({scale});",
                    img {
                        src: "{map.base_map.orthographic_image_url}",
                        alt: "Top-down render of the estate ({map.base_map.version_name})",
                        class: "absolute inset-0 w-full h-full object-fill pointer-events-none",
                        draggable: "false",
                    }
                    svg {
                        class: "absolute inset-0 w-full h-full",
                        view_box: "0 0 100 100",
                        preserve_aspect_ratio: "none",
                        if let Some(p) = parent.clone() {
                            if !leaf_focus {
                                polygon { class: "poly poly-focus", points: "{util::points_attr(&p.area.polygon_points)}" }
                            }
                        }
                        for a in visible.iter().cloned() {
                            polygon {
                                key: "{a.area.id}",
                                class: poly_class(a.status),
                                points: "{util::points_attr(&a.area.polygon_points)}",
                                "data-testid": "zone-{a.area.name}",
                                role: "button",
                                "aria-label": "{a.area.name}: {a.status.label()}",
                                onpointerenter: move |_| hovered.set(Some(a.area.id)),
                                onclick: move |_| {
                                    if moved() {
                                        return;
                                    }
                                    if active_parent() != Some(a.area.id) {
                                        active_parent.set(Some(a.area.id));
                                        zoom.set(1.0);
                                        pan.set((0.0, 0.0));
                                    }
                                },
                            }
                        }
                    }
                    // Labels stay crisp and readable at any zoom level.
                    for a in visible.iter() {
                        {
                            let (cx, cy) = util::centroid(&a.area.polygon_points);
                            rsx! {
                                div {
                                    key: "label-{a.area.id}",
                                    class: "absolute pointer-events-none",
                                    style: "left:{cx}%;top:{cy}%;transform:translate(-50%,-50%) scale({label_scale});transform-origin:center;",
                                    span { class: "block max-w-[140px] text-center text-[11px] leading-tight font-semibold text-ink px-1.5 py-0.5 rounded bg-white/85 shadow-sm",
                                        "{a.area.name}"
                                    }
                                }
                            }
                        }
                    }
                }
                // Hover info (fixed corner, never jitters)
                if let Some(h) = hovered_area {
                    div { class: "absolute top-3 left-3 max-w-[70%] bg-white/95 rounded-lg shadow-float px-3 py-2 pointer-events-none fade-in",
                        p { class: "text-sm font-medium", "{h.area.name}" }
                        p { class: "text-xs text-muted mt-0.5 line-clamp-2", "{h.status.label()} — {h.status_reason}" }
                    }
                }
                // Zoom controls
                div { class: "absolute bottom-3 right-3 flex flex-col bg-white/95 rounded-lg shadow-float overflow-hidden",
                    button {
                        class: "w-9 h-9 flex items-center justify-center hover:bg-hover transition-colors",
                        "aria-label": "Zoom in",
                        onpointerdown: move |e| e.stop_propagation(),
                        onclick: move |_| zoom.set((zoom() * 1.3).min(4.0)),
                        Icon { name: "plus" }
                    }
                    div { class: "h-px bg-line" }
                    button {
                        class: "w-9 h-9 flex items-center justify-center hover:bg-hover transition-colors",
                        "aria-label": "Zoom out",
                        onpointerdown: move |e| e.stop_propagation(),
                        onclick: move |_| {
                            let next = (zoom() / 1.3).max(1.0);
                            zoom.set(next);
                            if next <= 1.01 {
                                pan.set((0.0, 0.0));
                            }
                        },
                        Icon { name: "minus" }
                    }
                    div { class: "h-px bg-line" }
                    button {
                        class: "w-9 h-9 flex items-center justify-center hover:bg-hover transition-colors",
                        "aria-label": "Reset view",
                        onpointerdown: move |e| e.stop_propagation(),
                        onclick: move |_| reset_view(),
                        Icon { name: "maximize" }
                    }
                }
                if parent_id.is_some() {
                    button {
                        class: "absolute top-3 right-3 btn-secondary h-8 shadow-float",
                        onpointerdown: move |e| e.stop_propagation(),
                        onclick: move |_| {
                            let up = active_parent().and_then(|id| util::find(&util::flatten(app.map.peek().as_ref().unwrap()), id).and_then(|a| a.area.parent_id));
                            active_parent.set(up);
                            reset_view();
                        },
                        Icon { name: "chevron-left" }
                        "Back"
                    }
                }
            }

            // ------------------------------------------------ legend
            div { class: "flex flex-wrap items-center gap-x-4 gap-y-1.5 text-xs text-muted",
                LegendDot { color: "#346538", label: "All good" }
                LegendDot { color: "#476585", label: "Work in progress" }
                LegendDot { color: "#d97706", label: "Needs attention" }
                LegendDot { color: "#9e2a2b", label: "Urgent issue" }
                span { class: "hidden sm:inline ml-auto", "Tap a zone to look inside · drag to move" }
            }

            if let Some(p) = parent {
                ZoneSummary { area: p }
            }
        }
    }
}

#[component]
fn LegendDot(color: &'static str, label: &'static str) -> Element {
    rsx! {
        span { class: "inline-flex items-center gap-1.5",
            span { class: "w-2.5 h-2.5 rounded-sm", style: "background:{color};opacity:.75" }
            "{label}"
        }
    }
}

/// Card under the map for the selected zone, with the executive summary
/// for utility and contractor zones.
#[component]
fn ZoneSummary(area: FlatArea) -> Element {
    let app = use_app();
    let mut equipment_zone = app.equipment_zone;
    let nav = use_navigator();
    let id = area.area.id;
    rsx! {
        div { class: "card p-4 rise-in", "data-testid": "zone-summary",
            div { class: "flex flex-wrap items-start justify-between gap-3",
                div { class: "min-w-0",
                    p { class: "eyebrow", "{area.area.zone_class.label()}" }
                    h3 { class: "text-lg font-semibold mt-0.5", "{area.area.name}" }
                    div { class: "flex flex-wrap items-center gap-2 mt-1.5",
                        Pill { tone: area.status.tone(), text: area.status.label().to_string(), dot: true }
                        span { class: "text-sm text-muted", "{area.status_reason}" }
                    }
                }
                div { class: "flex gap-2",
                    if app.is_staff() {
                        button {
                            class: "btn-secondary",
                            onclick: move |_| {
                                equipment_zone.set(Some(id));
                                nav.push(Route::Equipment {});
                            },
                            Icon { name: "truck" }
                            "Equipment here"
                        }
                    }
                    Link { to: Route::Zone { id }, class: "btn-primary", "data-testid": "open-zone", "Open zone page" Icon { name: "arrow-right" } }
                }
            }
            if let Some(summary) = area.executive_summary.clone() {
                div { class: "mt-4 rounded-lg bg-sidebar px-4 py-3",
                    p { class: "eyebrow mb-1", "Executive summary" }
                    p { class: "text-sm leading-relaxed", "data-testid": "executive-summary", "{summary}" }
                }
            }
        }
    }
}
