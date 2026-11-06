//! Admin Canvas Manager & Polygon Drawing Studio.
//!
//! * Versioned base maps: upload a new top-down render; older ones are
//!   archived and every zone carries over (coordinates are percentages).
//! * Point-and-click polygon creator with parent nesting and zone class.
//! * Vertex dragging to reshape existing zones.
//! * Soft-delete archiving that keeps all history.

use dioxus::prelude::*;
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{Icon, Modal, Pill, SkeletonPage, Spinner};
use crate::services::{media_canvas, sync};
use crate::state::use_app;
use crate::util::{self, FlatArea};

const CANVAS_ID: &str = "studio-canvas";

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Select,
    Draw,
}

#[component]
pub fn MapStudio() -> Element {
    let app = use_app();
    let mut mode = use_signal(|| Mode::Select);
    let mut selected = use_signal(|| None::<Uuid>);
    let mut draft_points = use_signal(Vec::<Point2D>::new);
    let mut edit_points = use_signal(|| None::<Vec<Point2D>>);
    let mut dragging = use_signal(|| None::<usize>);
    let mut show_upload = use_signal(|| false);
    let mut show_archived = use_signal(|| false);

    let Some(map) = app.map.read().clone() else { return rsx! { SkeletonPage {} } };
    let flat = util::flatten(&map);
    let sel = selected().and_then(|id| util::find(&flat, id).cloned());
    let shown: Vec<FlatArea> = flat.iter().filter(|a| show_archived() || !a.area.is_archived).cloned().collect();
    let sel_points = edit_points().or_else(|| sel.as_ref().map(|s| s.area.polygon_points.clone()));
    let shape_changed = edit_points().is_some();

    let canvas_click = move |e: MouseEvent| {
        if mode() != Mode::Draw {
            return;
        }
        let c = e.client_coordinates();
        if let Some((x, y)) = util::percent_in(CANVAS_ID, c.x, c.y) {
            draft_points.write().push(Point2D { x: (x * 10.0).round() / 10.0, y: (y * 10.0).round() / 10.0 });
        }
    };

    rsx! {
        div { class: "space-y-6 rise-in",
            header { class: "flex flex-wrap items-end justify-between gap-3",
                div {
                    h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Map Studio" }
                    p { class: "text-[15px] text-muted mt-1", "Upload new estate renders and draw, reshape, nest or archive zones." }
                }
                div { class: "flex gap-2",
                    button { class: "btn-secondary", "data-testid": "upload-base-map", onclick: move |_| show_upload.set(true), Icon { name: "upload" } "New base map" }
                    if mode() == Mode::Select {
                        button {
                            class: "btn-primary",
                            "data-testid": "draw-zone",
                            onclick: move |_| {
                                mode.set(Mode::Draw);
                                draft_points.set(vec![]);
                                edit_points.set(None);
                            },
                            Icon { name: "pen-tool" }
                            "Draw a new zone"
                        }
                    }
                }
            }

            div { class: "grid lg:grid-cols-[1fr_320px] gap-6 items-start",
                // ------------------------------------------- canvas
                div { class: "space-y-2",
                    if mode() == Mode::Draw {
                        div { class: "rounded-lg bg-tag-blue-bg text-tag-blue-text px-4 py-2.5 text-sm flex flex-wrap items-center gap-2 fade-in",
                            Icon { name: "pin" }
                            span { class: "flex-1", "Click on the map to place each corner. {draft_points.read().len()} placed — at least 3 needed." }
                        }
                    }
                    div {
                        id: "{CANVAS_ID}",
                        class: if mode() == Mode::Draw { "relative w-full aspect-[16/10] rounded-xl overflow-hidden border border-line bg-[#e9ede4] select-none cursor-crosshair touch-none" } else { "relative w-full aspect-[16/10] rounded-xl overflow-hidden border border-line bg-[#e9ede4] select-none touch-none" },
                        "data-testid": "studio-canvas",
                        onclick: canvas_click,
                        onpointermove: move |e: PointerEvent| {
                            if let Some(i) = dragging() {
                                let c = e.client_coordinates();
                                if let Some((x, y)) = util::percent_in(CANVAS_ID, c.x, c.y) {
                                    let base = edit_points.peek().clone().or_else(|| sel_points_for(app, selected()));
                                    if let Some(mut pts) = base {
                                        if i < pts.len() {
                                            pts[i] = Point2D { x: (x * 10.0).round() / 10.0, y: (y * 10.0).round() / 10.0 };
                                            edit_points.set(Some(pts));
                                        }
                                    }
                                }
                            }
                        },
                        onpointerup: move |_| dragging.set(None),
                        onpointerleave: move |_| dragging.set(None),
                        img { src: "{map.base_map.orthographic_image_url}", alt: "Current base map", class: "absolute inset-0 w-full h-full object-fill pointer-events-none", draggable: "false" }
                        svg { class: "absolute inset-0 w-full h-full", view_box: "0 0 100 100", preserve_aspect_ratio: "none",
                            for a in shown.iter().cloned() {
                                {
                                    let is_sel = Some(a.area.id) == selected();
                                    let pts = if is_sel { sel_points.clone().unwrap_or_default() } else { a.area.polygon_points.clone() };
                                    let class = if a.area.is_archived { "poly poly-archived" } else if is_sel { "poly poly-attention" } else if a.depth == 0 { "poly poly-optimal" } else { "poly poly-active" };
                                    rsx! {
                                        polygon {
                                            key: "{a.area.id}",
                                            class,
                                            style: if mode() == Mode::Draw { "pointer-events:none;animation:none" } else { "animation:none" },
                                            points: "{util::points_attr(&pts)}",
                                            onclick: move |e: MouseEvent| {
                                                if mode() == Mode::Select {
                                                    e.stop_propagation();
                                                    selected.set(Some(a.area.id));
                                                    edit_points.set(None);
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                            if mode() == Mode::Draw && !draft_points.read().is_empty() {
                                polyline {
                                    points: "{util::points_attr(&draft_points.read())}",
                                    fill: "rgba(43,90,133,0.15)",
                                    stroke: "#2b5a85",
                                    stroke_width: "2",
                                    stroke_dasharray: "4 3",
                                    vector_effect: "non-scaling-stroke",
                                    style: "pointer-events:none",
                                }
                            }
                        }
                        // Corner handles (HTML so they stay round).
                        if mode() == Mode::Draw {
                            for (i, p) in draft_points.read().iter().enumerate() {
                                span { key: "d{i}", class: "absolute w-3 h-3 -ml-1.5 -mt-1.5 rounded-full bg-white border-2 border-[#2b5a85] pointer-events-none", style: "left:{p.x}%;top:{p.y}%" }
                            }
                        }
                        if mode() == Mode::Select {
                            if let Some(pts) = sel_points.clone() {
                                for (i, p) in pts.iter().enumerate() {
                                    span {
                                        key: "v{i}",
                                        class: "absolute w-4 h-4 -ml-2 -mt-2 rounded-full bg-white border-2 border-pin-amber shadow cursor-move hover:scale-125 transition-transform",
                                        style: "left:{p.x}%;top:{p.y}%",
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
                    }
                    div { class: "flex flex-wrap items-center gap-3 text-xs text-muted",
                        span { "Active map: {map.base_map.version_name}" }
                        label { class: "inline-flex items-center gap-1.5 ml-auto cursor-pointer",
                            input { r#type: "checkbox", checked: show_archived(), onchange: move |e| show_archived.set(e.checked()) }
                            "Show archived zones"
                        }
                    }
                }

                // ------------------------------------------- side panel
                aside { class: "space-y-4",
                    if mode() == Mode::Draw {
                        NewZonePanel {
                            points: draft_points(),
                            default_parent: selected(),
                            flat: flat.clone(),
                            on_undo: move |_| { draft_points.write().pop(); },
                            on_cancel: move |_| { mode.set(Mode::Select); draft_points.set(vec![]); },
                            on_saved: move |id: Uuid| {
                                mode.set(Mode::Select);
                                draft_points.set(vec![]);
                                selected.set(Some(id));
                            },
                        }
                    } else if let Some(s) = sel {
                        EditZonePanel {
                            key: "{s.area.id}",
                            area: s.clone(),
                            flat: flat.clone(),
                            pending_points: edit_points(),
                            shape_changed,
                            on_reset_shape: move |_| edit_points.set(None),
                            on_shape_saved: move |_| edit_points.set(None),
                            on_close: move |_| { selected.set(None); edit_points.set(None); },
                        }
                    } else {
                        div { class: "card p-4 text-sm text-muted", "Click a zone on the map to rename, reshape, nest or archive it — or draw a new one." }
                    }
                    ZoneTree { flat: flat.clone(), selected: selected(), on_select: move |id| { selected.set(Some(id)); edit_points.set(None); mode.set(Mode::Select); } }
                    BaseMapHistory { history: map.base_map_history.clone() }
                }
            }
        }
        if show_upload() {
            UploadBaseMap { on_close: move |_| show_upload.set(false) }
        }
    }
}

fn sel_points_for(app: crate::state::AppState, id: Option<Uuid>) -> Option<Vec<Point2D>> {
    let id = id?;
    let flat = app.flat_areas();
    util::find(&flat, id).map(|a| a.area.polygon_points.clone())
}

fn parent_options(flat: &[FlatArea], exclude: Option<Uuid>) -> Vec<FlatArea> {
    let blocked = exclude.map(|id| util::subtree(flat, id)).unwrap_or_default();
    flat.iter().filter(|a| !a.area.is_archived && !blocked.contains(&a.area.id)).cloned().collect()
}

#[component]
fn NewZonePanel(
    points: Vec<Point2D>,
    default_parent: Option<Uuid>,
    flat: Vec<FlatArea>,
    on_undo: EventHandler<()>,
    on_cancel: EventHandler<()>,
    on_saved: EventHandler<Uuid>,
) -> Element {
    let app = use_app();
    let mut name = use_signal(String::new);
    let mut class = use_signal(|| {
        default_parent.and_then(|p| util::find(&flat, p).map(|a| a.area.zone_class)).unwrap_or(ZoneClass::Recreational)
    });
    let mut parent = use_signal(|| default_parent);
    let mut busy = use_signal(|| false);
    let ready = points.len() >= 3 && !name().trim().is_empty();
    let options = parent_options(&flat, None);
    rsx! {
        div { class: "card p-4 space-y-4 rise-in", "data-testid": "new-zone-panel",
            p { class: "font-semibold", "New zone" }
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
            div {
                label { class: "label", "Inside of" }
                select { class: "field", "data-testid": "zone-parent", onchange: move |e| parent.set(Uuid::parse_str(&e.value()).ok()),
                    option { value: "", selected: parent().is_none(), "Nothing — top level of the estate" }
                    for a in options.iter() {
                        option { key: "{a.area.id}", value: "{a.area.id}", selected: parent() == Some(a.area.id), "{\"\u{2003}\".repeat(a.depth)}{a.area.name}" }
                    }
                }
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
                div {
                    p { class: "font-semibold", "{area.area.name}" }
                    div { class: "flex gap-1.5 mt-1",
                        Pill { tone: area.status.tone(), text: area.status.label() }
                        if archived { Pill { tone: Tone::Gray, text: "Archived Locations" } }
                    }
                }
                button { class: "btn-icon btn-ghost", "aria-label": "Close", onclick: move |_| on_close.call(()), Icon { name: "x" } }
            }
            div {
                label { class: "label", "Name" }
                input { class: "field", value: "{name}", oninput: move |e| name.set(e.value()) }
            }
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
                    option { value: "", selected: parent().is_none(), "Nothing — top level of the estate" }
                    for a in options.iter() {
                        option { key: "{a.area.id}", value: "{a.area.id}", selected: parent() == Some(a.area.id), "{\"\u{2003}\".repeat(a.depth)}{a.area.name}" }
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
            div { class: "rounded-lg bg-sidebar p-3 space-y-2",
                p { class: "text-sm", "Drag the corner dots on the map to reshape this zone." }
                if shape_changed {
                    div { class: "flex gap-2",
                        button { class: "btn-ghost flex-1", onclick: move |_| on_reset_shape.call(()), "Reset" }
                        {
                            let pts = pending_points.clone().unwrap_or_default();
                            rsx! {
                                button {
                                    class: "btn-primary flex-1",
                                    disabled: busy(),
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
fn ZoneTree(flat: Vec<FlatArea>, selected: Option<Uuid>, on_select: EventHandler<Uuid>) -> Element {
    rsx! {
        div { class: "card p-2 max-h-80 overflow-y-auto",
            p { class: "eyebrow px-2 py-1.5", "All zones" }
            for a in flat {
                button {
                    key: "{a.area.id}",
                    class: if selected == Some(a.area.id) { "w-full text-left nav-link active" } else { "w-full text-left nav-link" },
                    style: "padding-left:{0.6 + a.depth as f32 * 0.9}rem",
                    onclick: move |_| on_select.call(a.area.id),
                    span { class: if a.area.is_archived { "flex-1 truncate line-through opacity-60" } else { "flex-1 truncate" }, "{a.area.name}" }
                }
            }
        }
    }
}

#[component]
fn BaseMapHistory(history: Vec<BaseMapVersion>) -> Element {
    rsx! {
        div { class: "card p-4 space-y-2",
            p { class: "eyebrow", "Base map versions" }
            ul { class: "space-y-2",
                for b in history {
                    li { key: "{b.id}", class: "flex items-center gap-3",
                        img { src: "{b.orthographic_image_url}", alt: "", class: "w-14 aspect-[16/10] object-cover rounded border border-line" }
                        div { class: "flex-1 min-w-0",
                            p { class: "text-sm truncate", "{b.version_name}" }
                            p { class: "text-xs text-muted", "{util::day(b.created_at)}" }
                        }
                        if b.is_active { Pill { tone: Tone::Green, text: "Active" } } else { Pill { tone: Tone::Gray, text: "Archived" } }
                    }
                }
            }
        }
    }
}

#[component]
fn UploadBaseMap(on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let mut name = use_signal(String::new);
    let mut image = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    rsx! {
        Modal { title: "Upload a new base map", on_close: move |_| on_close.call(()),
            div { class: "space-y-4",
                p { class: "text-sm text-muted", "Use a straight top-down render (e.g. from V-Ray or D5 Render) of the whole estate. The current map is archived and all zones carry over." }
                div {
                    label { class: "label", "Version name" }
                    input { class: "field", "data-testid": "base-map-name", placeholder: "e.g. 2027 Q1 D5 Render", value: "{name}", oninput: move |e| name.set(e.value()) }
                }
                input {
                    id: "base-map-file",
                    r#type: "file",
                    accept: "image/*",
                    class: "sr-only",
                    "data-testid": "base-map-file",
                    onchange: move |_| async move {
                        match media_canvas::image_from_input("base-map-file", 3200.0).await {
                            Ok(Some(d)) => image.set(Some(d)),
                            Ok(None) => {}
                            Err(e) => app.error(e),
                        }
                    },
                }
                match image() {
                    Some(src) => rsx! { img { src: "{src}", alt: "New base map preview", class: "w-full rounded-lg border border-line" } },
                    None => rsx! {
                        label { r#for: "base-map-file", class: "flex items-center justify-center gap-2 aspect-[16/7] rounded-lg border border-dashed border-[#d9d9d6] bg-sidebar text-sm text-muted cursor-pointer hover:bg-hover transition-colors",
                            Icon { name: "upload" }
                            "Choose render image"
                        }
                    },
                }
                div { class: "flex justify-end gap-2",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button {
                        class: "btn-primary",
                        disabled: busy() || image().is_none() || name().trim().is_empty(),
                        "data-testid": "confirm-base-map",
                        onclick: move |_| async move {
                            let Some(data) = image() else { return };
                            busy.set(true);
                            match api::post::<_, BaseMapVersion>("/estate/base-maps", &UploadBaseMapRequest { version_name: name(), image_data_url: data }).await {
                                Ok(_) => {
                                    app.success("New base map is live");
                                    sync::load_many(app, &[DataScope::Map]).await;
                                    on_close.call(());
                                }
                                Err(e) => app.error(e.message()),
                            }
                            busy.set(false);
                        },
                        if busy() { Spinner {} }
                        "Make it the active map"
                    }
                }
            }
        }
    }
}
