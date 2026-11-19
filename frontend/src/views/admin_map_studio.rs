//! Admin Canvas Manager & Polygon Drawing Studio.
//!
//! * Zones are SVG polygons drawn straight onto the base render; sub-zones
//!   must sit inside their parent's border (checked live and by the server).
//! * Versioned base maps: upload a new top-down render; older ones are
//!   archived and every zone carries over (coordinates are percentages).
//! * Vertex dragging to reshape, soft-delete archiving that keeps history.

use dioxus::prelude::*;
use shared::api::*;
use shared::geometry::corners_outside;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::estate_map::{fit, BaseMapLayer, MapButton};
use crate::components::ui::{Icon, Modal, Pill, SkeletonPage, Spinner};
use crate::services::{media_canvas, sync};
use crate::state::use_app;
use crate::util::{self, FlatArea};

const CANVAS_ID: &str = "studio-canvas";
/// The zoomable layer inside the canvas (clicks are measured against it).
const STAGE_ID: &str = "studio-stage";

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Select,
    Draw,
}

fn snap(v: f32) -> f32 {
    (v * 10.0).round() / 10.0
}

/// Even-odd path that shades everything outside `pts`.
fn outside_path(pts: &[Point2D]) -> String {
    let inner: Vec<String> = pts.iter().map(|p| format!("{} {}", p.x, p.y)).collect();
    format!("M0 0H100V100H0Z M{}Z", inner.join(" L"))
}

#[component]
pub fn MapStudio() -> Element {
    let app = use_app();
    let mut mode = use_signal(|| Mode::Select);
    let mut selected = use_signal(|| None::<Uuid>);
    let mut draft_points = use_signal(Vec::<Point2D>::new);
    let mut draw_parent = use_signal(|| None::<Uuid>);
    let mut edit_points = use_signal(|| None::<Vec<Point2D>>);
    let mut dragging = use_signal(|| None::<usize>);
    let mut show_upload = use_signal(|| false);
    let mut replacing = use_signal(|| None::<BaseMapVersion>);
    let mut show_archived = use_signal(|| false);
    let mut show_labels = use_signal(|| true);

    let Some(map) = app.map.read().clone() else { return rsx! { SkeletonPage {} } };
    let flat = util::flatten(&map);
    let drawing = mode() == Mode::Draw;
    let sel = if drawing { None } else { selected().and_then(|id| util::find(&flat, id).cloned()) };
    let sel_id = sel.as_ref().map(|s| s.area.id);
    let sel_points = edit_points().or_else(|| sel.as_ref().map(|s| s.area.polygon_points.clone()));
    let shape_changed = edit_points().is_some();

    // ---- live nesting checks
    let draft = draft_points();
    let into = if drawing { draw_parent().and_then(|id| util::find(&flat, id).cloned()) } else { None };
    let draft_bad = into.as_ref().map(|p| corners_outside(&draft, &p.area.polygon_points)).unwrap_or_default();
    let sel_parent = sel.as_ref().and_then(|s| s.area.parent_id).and_then(|id| util::find(&flat, id).cloned());
    let sel_bad = match (&sel_points, &sel_parent) {
        (Some(pts), Some(p)) => corners_outside(pts, &p.area.polygon_points),
        _ => vec![],
    };
    let stuck: Vec<Uuid> = match (sel_id, &sel_points) {
        (Some(id), Some(pts)) => flat
            .iter()
            .filter(|c| c.area.parent_id == Some(id) && !c.area.is_archived && !corners_outside(&c.area.polygon_points, pts).is_empty())
            .map(|c| c.area.id)
            .collect(),
        _ => vec![],
    };
    let stuck_name = stuck.first().and_then(|id| util::find(&flat, *id)).map(|a| a.area.name.clone());
    let parent_id = sel_parent.as_ref().map(|p| p.area.id).or(into.as_ref().map(|p| p.area.id));

    // ---- zoom: fly to the zone being edited or drawn into
    let mut view = use_signal(|| (1.0f32, 0.0f32, 0.0f32));
    let focus = if drawing { draw_parent() } else { selected() };
    let mut last_focus = use_signal(|| None::<Option<Uuid>>);
    if *last_focus.peek() != Some(focus) {
        last_focus.set(Some(focus));
        view.set(fit(focus.and_then(|id| util::find(&flat, id))));
    }
    let (zs, ztx, zty) = view();
    let inv = 1.0 / zs;
    let mut zoom_by = move |f: f32| {
        let (s, tx, ty) = view();
        let ns = (s * f).clamp(1.0, 12.0);
        let (cx, cy) = ((50.0 - tx) / s, (50.0 - ty) / s);
        let keep = |t: f32| t.clamp(100.0 - 100.0 * ns, 0.0);
        view.set((ns, keep(50.0 - cx * ns), keep(50.0 - cy * ns)));
    };

    // ---- layers: parents under children, the selection on top
    let mut layers: Vec<(FlatArea, String)> = flat
        .iter()
        .filter(|a| show_archived() || !a.area.is_archived)
        .map(|a| {
            let id = a.area.id;
            let mut c = String::from(if a.area.is_archived {
                "zone zone-archived"
            } else if Some(id) == sel_id {
                "zone zone-sel"
            } else if Some(id) == parent_id {
                "zone zone-parent"
            } else if a.depth == 0 {
                "zone zone-l0"
            } else {
                "zone zone-l1"
            });
            if (Some(id) == sel_id && !sel_bad.is_empty()) || stuck.contains(&id) {
                c.push_str(" zone-bad");
            }
            let related = Some(id) == sel_id || Some(id) == parent_id || (sel_id.is_some() && a.area.parent_id == sel_id);
            if sel_id.is_some() && !related {
                c.push_str(" zone-dim");
            }
            (a.clone(), c)
        })
        .collect();
    if let Some(i) = layers.iter().position(|(a, _)| Some(a.area.id) == sel_id) {
        let top = layers.remove(i);
        layers.push(top);
    }
    let live: Vec<&FlatArea> = flat.iter().filter(|a| !a.area.is_archived).collect();
    let (tops, subs) = (live.iter().filter(|a| a.depth == 0).count(), live.iter().filter(|a| a.depth > 0).count());
    let archived = flat.len() - live.len();

    // The header button always starts a new top-level zone on the whole map; to
    // draw inside a zone, select it and use Draw in the map toolbar.
    let start_drawing = move |_| {
        selected.set(None);
        draw_parent.set(None);
        mode.set(Mode::Draw);
        draft_points.set(vec![]);
        edit_points.set(None);
    };

    rsx! {
        div { class: "space-y-5 rise-in",
            header { class: "flex flex-wrap items-end justify-between gap-3",
                div { class: "min-w-0",
                    h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Map Studio" }
                    p { class: "text-[15px] text-muted mt-1", "Draw each zone on the map. A smaller zone always stays inside the zone it belongs to." }
                    div { class: "flex flex-wrap gap-2 mt-3 text-xs",
                        StatChip { value: tops, label: "zones" }
                        StatChip { value: subs, label: "sub-zones" }
                        if archived > 0 { StatChip { value: archived, label: "archived" } }
                    }
                }
                div { class: "flex gap-2",
                    button { class: "btn-secondary", "data-testid": "upload-base-map", onclick: move |_| show_upload.set(true), Icon { name: "upload" } span { class: "hidden sm:inline", "New map picture" } }
                    if !drawing {
                        button { class: "btn-primary", "data-testid": "draw-zone", onclick: start_drawing, Icon { name: "pen-tool" } "Draw a new zone" }
                    }
                }
            }

            div { class: "grid grid-cols-[minmax(0,1fr)] lg:grid-cols-[minmax(0,1fr)_340px] gap-5 items-start",
                // ------------------------------------------- canvas
                div {
                    id: "{CANVAS_ID}",
                    class: if drawing { "relative w-full aspect-[4/3] sm:aspect-[16/10] rounded-2xl overflow-hidden border border-line map-bg select-none touch-none cursor-crosshair shadow-float" } else { "relative w-full aspect-[4/3] sm:aspect-[16/10] rounded-2xl overflow-hidden border border-line map-bg select-none touch-none shadow-float" },
                    "data-testid": "studio-canvas",
                    onclick: move |e: MouseEvent| {
                        let c = e.client_coordinates();
                        if mode() == Mode::Draw {
                            if let Some((x, y)) = util::percent_in(STAGE_ID, c.x, c.y) {
                                draft_points.write().push(Point2D { x: snap(x), y: snap(y) });
                            }
                        } else if edit_points.peek().is_none() {
                            selected.set(None);
                        }
                    },
                    onpointermove: move |e: PointerEvent| {
                        let Some(i) = dragging() else { return };
                        let c = e.client_coordinates();
                        let Some((x, y)) = util::percent_in(STAGE_ID, c.x, c.y) else { return };
                        let base = edit_points.peek().clone().or_else(|| {
                            let id = (*selected.peek())?;
                            util::find(&app.flat_areas(), id).map(|a| a.area.polygon_points.clone())
                        });
                        if let Some(mut pts) = base {
                            if i < pts.len() {
                                pts[i] = Point2D { x: snap(x), y: snap(y) };
                                edit_points.set(Some(pts));
                            }
                        }
                    },
                    onpointerup: move |_| dragging.set(None),
                    onpointerleave: move |_| dragging.set(None),
                    div { id: "{STAGE_ID}", class: "map-stage absolute inset-0", style: "transform: translate({ztx}%, {zty}%) scale({zs});",
                    BaseMapLayer { map: map.base_map.clone(), viewport: CANVAS_ID, scale: zs, tx: ztx, ty: zty }
                    svg { class: "absolute inset-0 w-full h-full", view_box: "0 0 100 100", preserve_aspect_ratio: "none",
                        for (a, class) in layers.iter().cloned() {
                            {
                                let pts = if Some(a.area.id) == sel_id { sel_points.clone().unwrap_or_default() } else { a.area.polygon_points.clone() };
                                rsx! {
                                    polygon {
                                        key: "{a.area.id}",
                                        class,
                                        style: if drawing { "pointer-events:none" } else { "" },
                                        points: "{util::points_attr(&pts)}",
                                        "data-testid": "studio-zone-{a.area.name}",
                                        onclick: move |e: MouseEvent| {
                                            e.stop_propagation();
                                            if mode() == Mode::Select && selected() != Some(a.area.id) {
                                                selected.set(Some(a.area.id));
                                                edit_points.set(None);
                                            }
                                        },
                                    }
                                }
                            }
                        }
                        // Shade everything outside the zone being drawn into.
                        if let Some(p) = into.as_ref() {
                            path { d: "{outside_path(&p.area.polygon_points)}", fill: "rgba(4,10,8,.45)", style: "fill-rule:evenodd;pointer-events:none" }
                        }
                        if drawing && !draft.is_empty() {
                            if draft.len() >= 3 {
                                polygon { class: if draft_bad.is_empty() { "zone zone-draft" } else { "zone zone-draft zone-bad" }, points: "{util::points_attr(&draft)}" }
                            } else {
                                polyline { class: "zone zone-draft", style: "fill:none", points: "{util::points_attr(&draft)}" }
                            }
                        }
                    }
                    // Labels stay readable on any render.
                    if show_labels() {
                        for (a, class) in layers.iter().filter(|(a, _)| !a.area.is_archived) {
                            {
                                let (cx, cy) = util::centroid(&a.area.polygon_points);
                                let size = if a.depth == 0 { "text-[11px] sm:text-[12px]" } else { "text-[9.5px] sm:text-[10.5px] font-medium" };
                                let dim = if class.contains("zone-dim") || into.is_some() { "opacity-40" } else { "" };
                                rsx! {
                                    div { key: "l-{a.area.id}", class: "absolute pointer-events-none transition-opacity {dim}", style: "left:{cx}%;top:{cy}%;transform:translate(-50%,-50%) scale({inv})",
                                        span { class: "zone-label {size}", "{a.area.name}" }
                                    }
                                }
                            }
                        }
                    }
                    // Corner handles (HTML so they stay round at any aspect ratio).
                    if drawing {
                        for (i, p) in draft.iter().enumerate() {
                            span { key: "d{i}", class: if draft_bad.contains(&i) { "handle bad pointer-events-none" } else { "handle pointer-events-none" }, style: "left:{p.x}%;top:{p.y}%;transform:scale({inv})" }
                        }
                    } else if let Some(pts) = sel_points.clone() {
                        for (i, p) in pts.iter().enumerate() {
                            span {
                                key: "v{i}",
                                class: if sel_bad.contains(&i) { "handle bad cursor-move" } else { "handle cursor-move" },
                                style: "left:{p.x}%;top:{p.y}%;transform:scale({inv})",
                                "data-testid": "vertex",
                                onpointerdown: move |e: PointerEvent| {
                                    e.stop_propagation();
                                    e.prevent_default();
                                    dragging.set(Some(i));
                                },
                                onclick: move |e| e.stop_propagation(),
                            }
                        }
                    }

                    }

                    // ---- overlay: zoom
                    div {
                        class: "absolute bottom-3 right-3 glass flex flex-col overflow-hidden",
                        onclick: move |e| e.stop_propagation(),
                        onpointerdown: move |e| e.stop_propagation(),
                        MapButton { label: "Zoom in", icon: "plus", disabled: zs >= 11.9, on_click: move |_| zoom_by(1.5) }
                        div { class: "h-px bg-line" }
                        MapButton { label: "Zoom out", icon: "minus", disabled: zs <= 1.01, on_click: move |_| zoom_by(1.0 / 1.5) }
                        div { class: "h-px bg-line" }
                        MapButton { label: "Show the whole estate", icon: "home", disabled: zs <= 1.01 && ztx == 0.0 && zty == 0.0, on_click: move |_| view.set((1.0, 0.0, 0.0)) }
                        if util::can_fullscreen() {
                            div { class: "h-px bg-line" }
                            MapButton { label: "Full screen", icon: "maximize", on_click: move |_| util::toggle_fullscreen(CANVAS_ID) }
                        }
                    }

                    // ---- overlay: tools
                    div {
                        class: "absolute top-3 left-3 glass p-1 flex items-center gap-0.5 text-sm",
                        onclick: move |e| e.stop_propagation(),
                        onpointerdown: move |e| e.stop_propagation(),
                        ToolButton { active: !drawing, icon: "map", label: "Select", testid: "tool-select",
                            on_click: move |_| { mode.set(Mode::Select); draft_points.set(vec![]); } }
                        ToolButton { active: drawing, icon: "pen-tool", label: "Draw", testid: "tool-draw",
                            on_click: move |_| { if mode() != Mode::Draw { draw_parent.set(selected()); mode.set(Mode::Draw); draft_points.set(vec![]); edit_points.set(None); } } }
                        span { class: "w-px h-5 bg-line mx-1" }
                        ToolButton { active: show_labels(), icon: "eye", label: "Labels", testid: "tool-labels", on_click: move |_| show_labels.toggle() }
                        ToolButton { active: show_archived(), icon: "archive", label: "Archived", testid: "tool-archived", on_click: move |_| show_archived.toggle() }
                    }
                    // ---- overlay: which render is live
                    div { class: "absolute top-3 right-3 glass px-3 py-1.5 hidden sm:flex items-center gap-2 text-xs pointer-events-none max-w-[40%]",
                        span { class: "w-1.5 h-1.5 rounded-full bg-accent-ink shrink-0" }
                        span { class: "truncate", span { class: "text-muted", "Live map · " } "{map.base_map.version_name}" }
                    }
                    // ---- overlay: drawing guidance
                    if drawing {
                        div { class: "absolute bottom-3 left-1/2 -translate-x-1/2 w-[calc(100%-1.5rem)] sm:w-auto glass px-4 py-2 text-sm flex items-center gap-2 pointer-events-none fade-in",
                            if draft_bad.is_empty() {
                                Icon { name: "pin", class: "text-accent-ink shrink-0" }
                                span {
                                    "Tap to place corners · {draft.len()} placed"
                                    if let Some(p) = into.as_ref() { span { class: "text-muted", " · inside {p.area.name}" } }
                                }
                            } else {
                                Icon { name: "alert", class: "text-tag-red-text shrink-0" }
                                span { class: "text-tag-red-text font-medium", "{util::plural(draft_bad.len(), \"corner\")} outside {into.as_ref().map(|p| p.area.name.clone()).unwrap_or_default()} — undo and place inside" }
                            }
                        }
                    } else {
                        div { class: "absolute bottom-3 left-3 glass px-3 py-2 hidden md:flex items-center gap-3 text-[11px] text-muted pointer-events-none",
                            LegendSwatch { class: "zone-l0", label: "Zone" }
                            LegendSwatch { class: "zone-l1", label: "Sub-zone" }
                            LegendSwatch { class: "zone-sel", label: "Selected" }
                            LegendSwatch { class: "zone-bad", label: "Outside its zone" }
                        }
                    }
                }

                // ------------------------------------------- side panel
                aside { class: "min-w-0 space-y-4 lg:sticky lg:top-14",
                    if drawing {
                        NewZonePanel {
                            points: draft.clone(),
                            parent: draw_parent,
                            outside: draft_bad.len(),
                            flat: flat.clone(),
                            on_undo: move |_| { draft_points.write().pop(); },
                            on_cancel: move |_| { mode.set(Mode::Select); draft_points.set(vec![]); },
                            on_saved: move |id: Uuid| {
                                mode.set(Mode::Select);
                                draft_points.set(vec![]);
                                selected.set(Some(id));
                            },
                        }
                    } else if let Some(s) = sel.clone() {
                        EditZonePanel {
                            key: "{s.area.id}",
                            area: s.clone(),
                            flat: flat.clone(),
                            pending_points: edit_points(),
                            shape_changed,
                            outside: sel_bad.len(),
                            parent_name: sel_parent.as_ref().map(|p| p.area.name.clone()),
                            stuck_child: stuck_name.clone(),
                            on_reset_shape: move |_| edit_points.set(None),
                            on_shape_saved: move |_| edit_points.set(None),
                            on_close: move |_| { selected.set(None); edit_points.set(None); },
                        }
                    } else {
                        div { class: "card p-4 flex gap-3 text-sm",
                            span { class: "w-8 h-8 rounded-lg bg-accent-soft text-accent-ink flex items-center justify-center shrink-0", Icon { name: "map" } }
                            p { class: "text-muted", "Tap a zone on the map or in the list to rename it, change its shape or put it away. To add a smaller zone inside one, tap that zone first, then press Draw." }
                        }
                    }
                    ZoneTree { flat: flat.clone(), selected: if drawing { draw_parent() } else { selected() }, show_archived: show_archived(),
                        on_select: move |id| {
                            if mode() == Mode::Draw {
                                draw_parent.set(Some(id));
                            } else {
                                selected.set(Some(id));
                                edit_points.set(None);
                            }
                        } }
                    BaseMapHistory { history: map.base_map_history.clone(), on_replace: move |b| replacing.set(Some(b)) }
                }
            }
        }
        if show_upload() {
            UploadBaseMap { on_close: move |_| show_upload.set(false) }
        }
        if let Some(b) = replacing() {
            UploadBaseMap { replacing: b, on_close: move |_| replacing.set(None) }
        }
    }
}

#[component]
fn StatChip(value: usize, label: &'static str) -> Element {
    rsx! {
        span { class: "inline-flex items-center gap-1.5 rounded-full border border-line px-2.5 h-6",
            span { class: "font-semibold tabular text-ink", "{value}" }
            span { class: "text-muted", "{label}" }
        }
    }
}

#[component]
fn ToolButton(active: bool, icon: &'static str, label: &'static str, testid: &'static str, on_click: EventHandler<()>) -> Element {
    rsx! {
        button {
            class: if active { "h-8 px-2.5 rounded-md flex items-center gap-1.5 chip-on font-medium" } else { "h-8 px-2.5 rounded-md flex items-center gap-1.5 text-muted hover:bg-hover hover:text-ink transition-colors" },
            "aria-pressed": active,
            "aria-label": label,
            "data-testid": testid,
            onclick: move |_| on_click.call(()),
            Icon { name: icon }
            span { class: "hidden sm:inline", "{label}" }
        }
    }
}

#[component]
fn LegendSwatch(class: &'static str, label: &'static str) -> Element {
    rsx! {
        span { class: "inline-flex items-center gap-1.5",
            svg { class: "w-4 h-3", view_box: "0 0 16 12", rect { class: "zone {class}", x: "1", y: "1", width: "14", height: "10", rx: "2" } }
            "{label}"
        }
    }
}

fn parent_options(flat: &[FlatArea], exclude: Option<Uuid>) -> Vec<FlatArea> {
    let blocked = exclude.map(|id| util::subtree(flat, id)).unwrap_or_default();
    flat.iter().filter(|a| !a.area.is_archived && !blocked.contains(&a.area.id)).cloned().collect()
}

#[component]
fn NewZonePanel(
    points: Vec<Point2D>,
    parent: Signal<Option<Uuid>>,
    outside: usize,
    flat: Vec<FlatArea>,
    on_undo: EventHandler<()>,
    on_cancel: EventHandler<()>,
    on_saved: EventHandler<Uuid>,
) -> Element {
    let app = use_app();
    let mut name = use_signal(String::new);
    let mut class = use_signal(|| {
        parent.peek().and_then(|p| util::find(&flat, p).map(|a| a.area.zone_class)).unwrap_or(ZoneClass::Recreational)
    });
    let mut busy = use_signal(|| false);
    let ready = points.len() >= 3 && !name().trim().is_empty() && outside == 0;
    let options = parent_options(&flat, None);
    rsx! {
        div { class: "card p-4 space-y-4 rise-in", "data-testid": "new-zone-panel",
            div { class: "flex items-center gap-2",
                span { class: "w-7 h-7 rounded-lg chip-on flex items-center justify-center", Icon { name: "pen-tool", size: 14 } }
                p { class: "font-semibold", "New zone" }
                span { class: "ml-auto text-xs text-muted tabular", "{points.len()} corners" }
            }
            div {
                label { class: "label", "Inside of" }
                select { class: "field", "data-testid": "zone-parent", onchange: move |e| parent.set(Uuid::parse_str(&e.value()).ok()),
                    option { value: "", selected: parent().is_none(), "Nothing — top level of the estate" }
                    for a in options.iter() {
                        option { key: "{a.area.id}", value: "{a.area.id}", selected: parent() == Some(a.area.id), "{\"\u{2003}\".repeat(a.depth)}{a.area.name}" }
                    }
                }
                p { class: "hint", "The map shades everything outside that zone so you draw in the right place." }
            }
            div {
                label { class: "label", "Name" }
                input { class: "field", "data-testid": "zone-name", placeholder: "e.g. Orchid Greenhouse", value: "{name}", oninput: move |e| name.set(e.value()) }
            }
            div {
                label { class: "label", "Type" }
                select { class: "field", "data-testid": "zone-class", onchange: move |e| { if let Some(c) = ZoneClass::ALL.iter().find(|c| format!("{c:?}") == e.value()) { class.set(*c); } },
                    for c in ZoneClass::ALL {
                        option { key: "{c:?}", value: "{c:?}", selected: class() == c, "{c.label()}" }
                    }
                }
            }
            if outside > 0 {
                p { class: "rounded-md bg-tag-red-bg text-tag-red-text text-sm px-3 py-2", "{util::plural(outside, \"corner\")} outside the parent zone. Undo them and place them inside." }
            }
            div { class: "flex flex-wrap gap-2",
                button { class: "btn-ghost", disabled: points.is_empty(), onclick: move |_| on_undo.call(()), Icon { name: "undo" } "Undo corner" }
                button { class: "btn-ghost", onclick: move |_| on_cancel.call(()), "Cancel" }
                {
                    let pts = points.clone();
                    rsx! {
                        button {
                            class: "btn-primary ml-auto",
                            disabled: !ready || busy(),
                            "data-testid": "save-zone",
                            onclick: move |_| {
                                let pts = pts.clone();
                                async move {
                                    busy.set(true);
                                    let req = CreateAreaRequest { parent_id: parent(), name: name(), zone_class: class(), polygon_points: pts };
                                    match api::post::<_, SpatialArea>("/estate/areas", &req).await {
                                        Ok(a) => {
                                            app.success(format!("{} added to the map", a.name));
                                            sync::load_many(app, &[DataScope::Map]).await;
                                            on_saved.call(a.id);
                                        }
                                        Err(e) => app.error(e.message()),
                                    }
                                    busy.set(false);
                                }
                            },
                            if busy() { Spinner {} }
                            "Save zone"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn EditZonePanel(
    area: FlatArea,
    flat: Vec<FlatArea>,
    pending_points: Option<Vec<Point2D>>,
    shape_changed: bool,
    outside: usize,
    parent_name: Option<String>,
    stuck_child: Option<String>,
    on_reset_shape: EventHandler<()>,
    on_shape_saved: EventHandler<()>,
    on_close: EventHandler<()>,
) -> Element {
    let app = use_app();
    let id = area.area.id;
    let mut name = use_signal(|| area.area.name.clone());
    let mut class = use_signal(|| area.area.zone_class);
    let mut parent = use_signal(|| area.area.parent_id);
    let mut busy = use_signal(|| false);
    let archived = area.area.is_archived;
    let options = parent_options(&flat, Some(id));
    let details_changed = name() != area.area.name || class() != area.area.zone_class || parent() != area.area.parent_id;
    let children = flat.iter().filter(|c| c.area.parent_id == Some(id) && !c.area.is_archived).count();
    let shape_problem = if outside > 0 {
        Some(format!("{} outside {}.", util::plural(outside, "corner"), parent_name.clone().unwrap_or_default()))
    } else {
        stuck_child.as_ref().map(|c| format!("{c} would stick out of this shape."))
    };

    let save = move |req: UpdateAreaRequest, done: &'static str| async move {
        busy.set(true);
        match api::put::<_, SpatialArea>(&format!("/estate/areas/{id}"), &req).await {
            Ok(_) => {
                app.success(done);
                sync::load_many(app, &[DataScope::Map, DataScope::WorkItems]).await;
            }
            Err(e) => app.error(e.message()),
        }
        busy.set(false);
    };

    rsx! {
        div { class: "card p-4 space-y-4 rise-in", "data-testid": "edit-zone-panel",
            div { class: "flex items-start justify-between gap-2",
                div { class: "min-w-0",
                    p { class: "eyebrow", if let Some(p) = parent_name.clone() { "Inside {p}" } else { "Top-level zone" } }
                    p { class: "font-semibold text-lg leading-tight truncate mt-0.5", "{area.area.name}" }
                    div { class: "flex flex-wrap gap-1.5 mt-1.5",
                        Pill { tone: area.status.tone(), text: area.status.label() }
                        if children > 0 { Pill { tone: Tone::Gray, text: util::plural(children, "sub-zone") } }
                        if archived { Pill { tone: Tone::Gray, text: "No longer in use" } }
                    }
                }
                button { class: "btn-icon btn-ghost", "aria-label": "Close", onclick: move |_| on_close.call(()), Icon { name: "x" } }
            }
            div {
                label { class: "label", "Name" }
                input { class: "field", value: "{name}", oninput: move |e| name.set(e.value()) }
            }
            div { class: "grid grid-cols-2 gap-3",
                div {
                    label { class: "label", "Type" }
                    select { class: "field", onchange: move |e| { if let Some(c) = ZoneClass::ALL.iter().find(|c| format!("{c:?}") == e.value()) { class.set(*c); } },
                        for c in ZoneClass::ALL {
                            option { key: "{c:?}", value: "{c:?}", selected: class() == c, "{c.label()}" }
                        }
                    }
                }
                div {
                    label { class: "label", "Inside of" }
                    select { class: "field", onchange: move |e| parent.set(Uuid::parse_str(&e.value()).ok()),
                        option { value: "", selected: parent().is_none(), "Top level" }
                        for a in options.iter() {
                            option { key: "{a.area.id}", value: "{a.area.id}", selected: parent() == Some(a.area.id), "{\"\u{2003}\".repeat(a.depth)}{a.area.name}" }
                        }
                    }
                }
            }
            if details_changed {
                button {
                    class: "btn-primary w-full",
                    disabled: busy(),
                    onclick: move |_| save(UpdateAreaRequest { name: Some(name()), zone_class: Some(class()), parent_id: Some(parent()), ..Default::default() }, "Zone details saved"),
                    "Save details"
                }
            }
            div { class: "rounded-lg border border-line p-3 space-y-2",
                p { class: "text-sm flex items-center gap-2", Icon { name: "pen-tool", class: "text-accent-ink" } "Drag the corner dots on the map to reshape." }
                if let Some(msg) = shape_problem.clone() {
                    p { class: "rounded-md bg-tag-red-bg text-tag-red-text text-sm px-3 py-2", "data-testid": "shape-problem", "{msg} Move the red corners back before saving." }
                }
                if shape_changed {
                    div { class: "flex gap-2",
                        button { class: "btn-ghost flex-1", onclick: move |_| on_reset_shape.call(()), "Reset" }
                        {
                            let pts = pending_points.clone().unwrap_or_default();
                            rsx! {
                                button {
                                    class: "btn-primary flex-1",
                                    disabled: busy() || shape_problem.is_some(),
                                    "data-testid": "save-shape",
                                    onclick: move |_| {
                                        let pts = pts.clone();
                                        async move {
                                            save(UpdateAreaRequest { polygon_points: Some(pts), ..Default::default() }, "New shape saved").await;
                                            on_shape_saved.call(());
                                        }
                                    },
                                    "Save shape"
                                }
                            }
                        }
                    }
                }
            }
            if archived {
                button {
                    class: "btn-secondary w-full",
                    disabled: busy(),
                    "data-testid": "restore-zone",
                    onclick: move |_| save(UpdateAreaRequest { is_archived: Some(false), ..Default::default() }, "Zone restored to the live map"),
                    Icon { name: "undo" }
                    "Restore zone"
                }
            } else {
                button {
                    class: "btn-red w-full",
                    disabled: busy(),
                    "data-testid": "archive-zone",
                    onclick: move |_| save(UpdateAreaRequest { is_archived: Some(true), ..Default::default() }, "Zone archived — its history is kept"),
                    Icon { name: "archive" }
                    "Archive Zone"
                }
                p { class: "hint", "Archiving hides a demolished or repurposed structure from the live map. Its projects stay searchable under History." }
            }
        }
    }
}

#[component]
fn ZoneTree(flat: Vec<FlatArea>, selected: Option<Uuid>, show_archived: bool, on_select: EventHandler<Uuid>) -> Element {
    let rows: Vec<FlatArea> = flat.iter().filter(|a| show_archived || !a.area.is_archived).cloned().collect();
    rsx! {
        div { class: "card overflow-hidden",
            div { class: "flex items-center justify-between px-4 h-11 border-b border-line",
                p { class: "eyebrow", "All zones" }
                span { class: "text-xs text-muted tabular", "{rows.len()}" }
            }
            div { class: "p-1.5 max-h-80 overflow-y-auto",
                for a in rows {
                    {
                        let kids = flat.iter().filter(|c| c.area.parent_id == Some(a.area.id) && !c.area.is_archived).count();
                        let on = selected == Some(a.area.id);
                        rsx! {
                            button {
                                key: "{a.area.id}",
                                class: if on { "w-full flex items-center gap-2 rounded-md h-9 pr-2 text-sm text-left bg-accent-soft text-accent-ink font-medium" } else { "w-full flex items-center gap-2 rounded-md h-9 pr-2 text-sm text-left text-ink hover:bg-hover transition-colors" },
                                style: "padding-left:{0.6 + a.depth as f32 * 1.0}rem",
                                "data-testid": "tree-zone",
                                onclick: move |_| on_select.call(a.area.id),
                                span { class: if a.depth == 0 { "w-2.5 h-2.5 rounded-sm bg-accent-ink/80 shrink-0" } else { "w-2.5 h-2.5 rounded-sm border-[1.5px] border-accent-ink/70 border-dashed shrink-0" } }
                                span { class: if a.area.is_archived { "flex-1 truncate line-through opacity-60" } else { "flex-1 truncate" }, "{a.area.name}" }
                                if kids > 0 { span { class: "text-[11px] text-muted tabular", "{kids}" } }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn BaseMapHistory(history: Vec<BaseMapVersion>, on_replace: EventHandler<BaseMapVersion>) -> Element {
    let app = use_app();
    let mut switching = use_signal(|| None::<Uuid>);
    let activate = move |b: BaseMapVersion| async move {
        switching.set(Some(b.id));
        match api::post::<_, BaseMapVersion>(&format!("/estate/base-maps/{}/activate", b.id), &()).await {
            Ok(_) => {
                app.success(format!("\u{201c}{}\u{201d} is now the live map", b.version_name));
                sync::reload(app, vec![DataScope::Map]);
            }
            Err(e) => app.error(e.message()),
        }
        switching.set(None);
    };
    rsx! {
        div { class: "card overflow-hidden",
            div { class: "px-4 pt-3 pb-2",
                p { class: "eyebrow", "Map pictures" }
                p { class: "text-xs text-muted mt-0.5", "Your zones show on whichever picture is in use. If buildings have moved, adjust the zones to match." }
            }
            ul { class: "px-2 pb-2 space-y-1",
                for b in history {
                    li { key: "{b.id}", class: if b.is_active { "flex items-center gap-3 rounded-lg p-2 bg-accent-soft" } else { "flex items-center gap-3 rounded-lg p-2" },
                        img { src: "{b.preview_url()}", alt: "", class: "w-14 aspect-[16/10] object-cover rounded border border-line" }
                        div { class: "flex-1 min-w-0",
                            p { class: "text-sm truncate", title: "{b.version_name}", "{b.version_name}" }
                            p { class: "text-xs text-muted", "{util::day(b.created_at)}" }
                        }
                        button {
                            class: "btn-icon btn-ghost h-8 w-8 shrink-0",
                            title: "Update picture",
                            "aria-label": "Update the picture for {b.version_name}",
                            "data-testid": "replace-base-map",
                            onclick: {
                                let b = b.clone();
                                move |_| on_replace.call(b.clone())
                            },
                            Icon { name: "upload" }
                        }
                        if b.is_active {
                            Pill { tone: Tone::Green, text: "Live" }
                        } else {
                            button {
                                class: "btn-secondary h-8 px-2.5 text-xs shrink-0",
                                "data-testid": "activate-base-map",
                                title: "Make this the live map",
                                disabled: switching().is_some(),
                                onclick: {
                                    let b = b.clone();
                                    move |_| activate(b.clone())
                                },
                                if switching() == Some(b.id) { Spinner { size: 12 } }
                                "Use this map"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
/// Adds a new map picture, or (with `replacing`) gives an existing map a new one.
fn UploadBaseMap(#[props(default)] replacing: Option<BaseMapVersion>, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let editing = replacing.clone();
    let mut name = use_signal(|| replacing.as_ref().map(|b| b.version_name.clone()).unwrap_or_default());
    let mut image = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let title = if editing.is_some() { "Update the map picture" } else { "Add a new map picture" };
    rsx! {
        Modal { title, on_close: move |_| on_close.call(()),
            div { class: "space-y-4",
                if let Some(b) = editing.clone() {
                    div { class: "flex items-center gap-3 rounded-lg bg-sidebar p-2",
                        img { src: "{b.preview_url()}", alt: "The picture in use now", class: "w-20 aspect-[16/10] object-cover rounded border border-line" }
                        p { class: "text-sm text-muted", "Choose the new picture for this map. Your zones stay where they are, so use a picture that shows the estate from the same angle." }
                    }
                } else {
                    p { class: "text-sm text-muted", "Choose a picture of the whole estate taken from straight above, like a drone photo or an architect's 3D picture. The bigger and sharper, the better: everyone can zoom right in. Your zones stay where they are." }
                }
                div {
                    label { class: "label", "Name" }
                    input { class: "field", "data-testid": "base-map-name", placeholder: "e.g. December drone photo", value: "{name}", oninput: move |e| name.set(e.value()) }
                }
                input {
                    id: "base-map-file",
                    r#type: "file",
                    accept: "image/jpeg,image/png,image/webp",
                    class: "sr-only",
                    "data-testid": "base-map-file",
                    onchange: move |_| async move {
                        match media_canvas::file_as_data_url("base-map-file", 60.0 * 1024.0 * 1024.0).await {
                            Ok(Some(d)) => image.set(Some(d)),
                            Ok(None) => {}
                            Err(e) => app.error(e),
                        }
                    },
                }
                match image() {
                    Some(src) => rsx! {
                        div { class: "relative",
                            img { src: "{src}", alt: "The picture you chose", class: "w-full rounded-lg border border-line" }
                            label { r#for: "base-map-file", class: "absolute top-2 right-2 btn-secondary h-8 cursor-pointer", "Choose another" }
                        }
                    },
                    None => rsx! {
                        label { r#for: "base-map-file", class: "flex flex-col items-center justify-center gap-1.5 aspect-[16/7] rounded-lg border border-dashed border-muted/30 bg-sidebar text-sm text-muted cursor-pointer hover:bg-hover transition-colors",
                            Icon { name: "upload", size: 20 }
                            span { class: "text-ink font-medium", "Choose a picture" }
                            span { class: "text-xs", "JPEG, PNG or WebP, up to 60 MB" }
                        }
                    },
                }
                if busy() {
                    p { class: "rounded-md bg-accent-soft text-accent-ink text-sm px-3 py-2 flex items-center gap-2", role: "status",
                        Spinner {}
                        "Preparing your map so it stays sharp when you zoom in. Big pictures can take a minute."
                    }
                }
                div { class: "modal-actions",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button {
                        class: "btn-primary",
                        disabled: busy() || image().is_none() || name().trim().is_empty(),
                        "data-testid": "confirm-base-map",
                        onclick: move |_| {
                            let editing = editing.clone();
                            async move {
                            let Some(data) = image() else { return };
                            busy.set(true);
                            let req = UploadBaseMapRequest { version_name: name(), image_data_url: data };
                            let result = match &editing {
                                Some(b) => api::put::<_, BaseMapVersion>(&format!("/estate/base-maps/{}", b.id), &req).await,
                                None => api::post::<_, BaseMapVersion>("/estate/base-maps", &req).await,
                            };
                            match result {
                                Ok(_) => {
                                    app.success(if editing.is_some() { "The map picture is updated" } else { "Your new map is ready" });
                                    sync::load_many(app, &[DataScope::Map]).await;
                                    on_close.call(());
                                }
                                Err(e) => app.error(e.message()),
                            }
                            busy.set(false);
                            }
                        },
                        if busy() { Spinner {} }
                        if busy() { "Preparing…" } else if replacing.is_some() { "Update picture" } else { "Use this map" }
                    }
                }
            }
        }
    }
}
