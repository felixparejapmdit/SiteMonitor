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
pub fn fit(parent: Option<&FlatArea>) -> (f32, f32, f32) {
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
    // While the zoom animates the layer is GPU-scaled (smooth); once it settles
    // `moving` drops and the browser repaints the map sharp at the new zoom.
    let stage_style = format!("transform: translate({tx}%, {ty}%) scale({scale});");
    let mut last_style = use_signal(String::new);
    let mut moving = use_signal(|| false);
    if *last_style.peek() != stage_style {
        if !last_style.peek().is_empty() && !dragging {
            moving.set(true);
            // Matches the .55s transition in input.css.
            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(650).await;
                moving.set(false);
            });
        }
        last_style.set(stage_style.clone());
    }
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
                div { class: "inline-flex p-0.5 rounded-lg bg-hover text-sm", role: "tablist", "aria-label": "What to show on the map",
                    button {
                        role: "tab",
                        "aria-selected": current_layer == MapLayer::FrontOfHouse,
                        "data-testid": "layer-foh",
                        class: if current_layer == MapLayer::FrontOfHouse { "px-3 h-8 rounded-md bg-surface shadow-sm font-medium" } else { "px-3 h-8 rounded-md text-muted hover:text-ink" },
                        onclick: move |_| {
                            if layer() != MapLayer::FrontOfHouse {
                                layer.set(MapLayer::FrontOfHouse);
                                active_parent.set(None);
                                reset_view();
                            }
                        },
                        "Homes & gardens"
                    }
                    button {
                        role: "tab",
                        "aria-selected": current_layer == MapLayer::Infrastructure,
                        "data-testid": "layer-infra",
                        class: if current_layer == MapLayer::Infrastructure { "px-3 h-8 rounded-md bg-surface shadow-sm font-medium" } else { "px-3 h-8 rounded-md text-muted hover:text-ink" },
                        onclick: move |_| {
                            if layer() != MapLayer::Infrastructure {
                                layer.set(MapLayer::Infrastructure);
                                active_parent.set(None);
                                reset_view();
                            }
                        },
                        span { class: "hidden sm:inline", "Utilities & work areas" }
                        span { class: "sm:hidden", "Utilities" }
                    }
                }
            }

            // ------------------------------------------------ the map
            div {
                id: "{STAGE_ID}",
                class: "relative w-full aspect-[16/10] rounded-xl overflow-hidden border border-line map-bg select-none touch-none contain-paint",
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
                    class: if dragging { "map-stage dragging absolute inset-0" } else if moving() { "map-stage moving absolute inset-0" } else { "map-stage absolute inset-0" },
                    style: "{stage_style}",
                    BaseMapLayer { map: map.base_map.clone(), viewport: STAGE_ID, scale, tx, ty }
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
                                    span { class: "block max-w-[140px] text-center text-[11px] leading-tight font-semibold text-ink px-1.5 py-0.5 rounded bg-surface/85 shadow-sm",
                                        "{a.area.name}"
                                    }
                                }
                            }
                        }
                    }
                }
                // Hover info (fixed corner, never jitters)
                if let Some(h) = hovered_area {
                    div { class: "absolute top-3 left-3 max-w-[70%] bg-surface/95 rounded-lg shadow-float px-3 py-2 pointer-events-none fade-in",
                        p { class: "text-sm font-medium", "{h.area.name}" }
                        p { class: "text-xs text-muted mt-0.5 line-clamp-2", "{h.status.label()} — {h.status_reason}" }
                    }
                }
                // Map controls: zoom, whole estate, full screen. Greyed out when they'd do nothing.
                div { class: "absolute bottom-3 right-3 flex flex-col bg-surface/95 rounded-lg shadow-float overflow-hidden",
                    MapButton { label: "Zoom in", icon: "plus", disabled: zoom() >= 3.99, on_click: move |_| zoom.set((zoom() * 1.3).min(4.0)) }
                    div { class: "h-px bg-line" }
                    MapButton {
                        label: "Zoom out",
                        icon: "minus",
                        disabled: zoom() <= 1.01 && parent_id.is_none(),
                        on_click: move |_| {
                            if zoom() <= 1.01 {
                                // As far out as this area goes: step out to the bigger area.
                                let up = active_parent().and_then(|id| util::find(&util::flatten(app.map.peek().as_ref().unwrap()), id).and_then(|a| a.area.parent_id));
                                active_parent.set(up);
                                reset_view();
                            } else {
                                let next = (zoom() / 1.3).max(1.0);
                                zoom.set(next);
                                if next <= 1.01 {
                                    pan.set((0.0, 0.0));
                                }
                            }
                        },
                    }
                    div { class: "h-px bg-line" }
                    MapButton {
                        label: "Show the whole estate",
                        icon: "home",
                        disabled: parent_id.is_none() && zoom() <= 1.01 && pan() == (0.0, 0.0),
                        on_click: move |_| {
                            active_parent.set(None);
                            reset_view();
                        },
                    }
                    if util::can_fullscreen() {
                        div { class: "h-px bg-line" }
                        MapButton { label: "Full screen", icon: "maximize", on_click: move |_| util::toggle_fullscreen(STAGE_ID) }
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
                    p { class: "eyebrow mb-1", "In short" }
                    p { class: "text-sm leading-relaxed", "data-testid": "executive-summary", "{summary}" }
                }
            }
        }
    }
}

/// The map picture. Uploaded pictures are drawn from zoom tiles: only the
/// tiles on screen, at the sharpness the current zoom needs, laid over a light
/// overview so the map is never blank while they load. `scale`/`tx`/`ty` are the
/// stage transform (`translate(tx%, ty%) scale(scale)` from the top-left).
#[component]
pub fn BaseMapLayer(map: BaseMapVersion, viewport: &'static str, scale: f32, tx: f32, ty: f32) -> Element {
    // Width of the map on screen; unknown on the very first draw, so measure once it's shown.
    let mut measured = use_signal(|| 0.0f64);
    let Some(t) = map.tiles.clone() else {
        // Drawings (SVG) stay sharp at any zoom on their own.
        return rsx! {
            img { src: "{map.orthographic_image_url}", alt: "Map of the estate", class: "absolute inset-0 w-full h-full object-fill pointer-events-none", draggable: "false" }
        };
    };
    let view_w = util::rect_of(viewport).map(|r| r.2).filter(|w| *w > 0.0).unwrap_or(measured());
    let dpr = util::window().device_pixel_ratio().max(1.0);
    let level = t.level_for(view_w * scale as f64 * dpr);
    let (lw, lh) = t.level_size(level);
    let (cols, rows) = t.grid(level);
    // The part of the map on screen, in % of the map.
    let (x0, x1) = (-tx / scale, (100.0 - tx) / scale);
    let (y0, y1) = (-ty / scale, (100.0 - ty) / scale);
    let ts = t.tile_size as f32;
    let mut tiles = vec![];
    if view_w > 0.0 && lw > MapTiles::PREVIEW_WIDTH {
        for y in 0..rows {
            for x in 0..cols {
                let left = x as f32 * ts / lw as f32 * 100.0;
                let right = ((x + 1) as f32 * ts).min(lw as f32) / lw as f32 * 100.0;
                let top = y as f32 * ts / lh as f32 * 100.0;
                let bottom = ((y + 1) as f32 * ts).min(lh as f32) / lh as f32 * 100.0;
                if right >= x0 && left <= x1 && bottom >= y0 && top <= y1 {
                    tiles.push((x, y, left, top, right - left, bottom - top));
                }
            }
        }
    }
    rsx! {
        img { src: "{t.preview_url()}", alt: "Map of the estate", class: "absolute inset-0 w-full h-full object-fill pointer-events-none", draggable: "false",
            onmounted: move |_| {
                if let Some(r) = util::rect_of(viewport) {
                    measured.set(r.2);
                }
            },
        }
        for (x, y, l, tp, w, h) in tiles {
            // A hair of overlap hides seams between neighbouring tiles.
            img { key: "{level}-{x}-{y}", src: "{t.tile_url(level, x, y)}", alt: "", decoding: "async", draggable: "false",
                class: "absolute pointer-events-none", style: "left:{l}%;top:{tp}%;width:{w + 0.04}%;height:{h + 0.04}%" }
        }
    }
}

/// One square map control with a tooltip; greyed out when it would do nothing.
#[component]
pub fn MapButton(#[props(into)] label: String, icon: &'static str, #[props(default)] disabled: bool, on_click: EventHandler<()>) -> Element {
    rsx! {
        button {
            class: "w-9 h-9 flex items-center justify-center hover:bg-hover transition-colors disabled:opacity-30 disabled:cursor-not-allowed disabled:hover:bg-transparent",
            "aria-label": "{label}",
            title: "{label}",
            disabled,
            onpointerdown: move |e| e.stop_propagation(),
            onclick: move |e| {
                e.stop_propagation();
                on_click.call(());
            },
            Icon { name: icon }
        }
    }
}
