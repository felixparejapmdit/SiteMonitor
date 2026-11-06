//! Small helpers: friendly dates, the flattened area tree, browser access.

use chrono::{DateTime, Duration, Local, NaiveDateTime, TimeZone, Utc};
use shared::api::{AreaNode, EstateMapResponse};
use shared::*;
use uuid::Uuid;

// ------------------------------------------------------------------ dates

pub fn local(dt: DateTime<Utc>) -> DateTime<Local> {
    dt.with_timezone(&Local)
}

/// "Mon, Nov 10"
pub fn day(dt: DateTime<Utc>) -> String {
    local(dt).format("%a, %b %-d").to_string()
}

/// "November 10, 2026"
pub fn long_day(dt: DateTime<Utc>) -> String {
    local(dt).format("%B %-d, %Y").to_string()
}

/// "8:00 AM"
pub fn time(dt: DateTime<Utc>) -> String {
    local(dt).format("%-I:%M %p").to_string()
}

/// "Mon, Nov 10 · 8:00 AM"
pub fn day_time(dt: DateTime<Utc>) -> String {
    format!("{} · {}", day(dt), time(dt))
}

/// "8:00 AM – 3:00 PM" or across days "Nov 10, 8:00 AM – Nov 11, 5:00 PM"
pub fn range(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    let (s, e) = (local(start), local(end));
    if s.date_naive() == e.date_naive() {
        format!("{} – {}", s.format("%-I:%M %p"), e.format("%-I:%M %p"))
    } else {
        format!("{} – {}", s.format("%b %-d, %-I:%M %p"), e.format("%b %-d, %-I:%M %p"))
    }
}

/// "in 2 days", "in 5 hours", "3 days ago", "just now"
pub fn relative(dt: DateTime<Utc>) -> String {
    let diff = dt - Utc::now();
    let future = diff > Duration::zero();
    let secs = diff.num_seconds().abs();
    let text = if secs < 60 {
        return "just now".into();
    } else if secs < 3600 {
        plural((secs / 60) as usize, "minute")
    } else if secs < 86_400 {
        plural((secs / 3600) as usize, "hour")
    } else {
        plural((secs / 86_400) as usize, "day")
    };
    if future {
        format!("in {text}")
    } else {
        format!("{text} ago")
    }
}

pub fn plural(n: usize, word: &str) -> String {
    if n == 1 {
        format!("1 {word}")
    } else {
        format!("{n} {word}s")
    }
}

/// Value for `<input type="datetime-local">`.
pub fn to_input(dt: DateTime<Utc>) -> String {
    local(dt).format("%Y-%m-%dT%H:%M").to_string()
}

pub fn from_input(value: &str) -> Option<DateTime<Utc>> {
    let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M").ok()?;
    Local.from_local_datetime(&naive).earliest().map(|d| d.with_timezone(&Utc))
}

/// Value for `<input type="date">`.
pub fn to_date_input(dt: DateTime<Utc>) -> String {
    local(dt).format("%Y-%m-%d").to_string()
}

pub fn greeting() -> &'static str {
    use chrono::Timelike;
    match Local::now().hour() {
        0..=11 => "Good morning",
        12..=16 => "Good afternoon",
        _ => "Good evening",
    }
}

pub fn same_local_day(a: DateTime<Utc>, b: DateTime<Utc>) -> bool {
    local(a).date_naive() == local(b).date_naive()
}

// ------------------------------------------------------------------ areas

#[derive(Debug, Clone, PartialEq)]
pub struct FlatArea {
    pub area: SpatialArea,
    pub status: AreaStatus,
    pub status_reason: String,
    pub executive_summary: Option<String>,
    pub depth: usize,
    pub child_ids: Vec<Uuid>,
}

pub fn flatten(map: &EstateMapResponse) -> Vec<FlatArea> {
    fn walk(n: &AreaNode, depth: usize, out: &mut Vec<FlatArea>) {
        out.push(FlatArea {
            area: n.area.clone(),
            status: n.status,
            status_reason: n.status_reason.clone(),
            executive_summary: n.executive_summary.clone(),
            depth,
            child_ids: n.children.iter().map(|c| c.area.id).collect(),
        });
        for c in &n.children {
            walk(c, depth + 1, out);
        }
    }
    let mut out = Vec::new();
    for n in &map.areas {
        walk(n, 0, &mut out);
    }
    out
}

pub fn find(flat: &[FlatArea], id: Uuid) -> Option<&FlatArea> {
    flat.iter().find(|a| a.area.id == id)
}

/// The id plus every descendant.
pub fn subtree(flat: &[FlatArea], root: Uuid) -> Vec<Uuid> {
    let mut out = vec![root];
    let mut i = 0;
    while i < out.len() {
        if let Some(a) = find(flat, out[i]) {
            out.extend(a.child_ids.iter().copied());
        }
        i += 1;
    }
    out
}

/// Root → … → id, for breadcrumbs.
pub fn path_to(flat: &[FlatArea], id: Uuid) -> Vec<FlatArea> {
    let mut chain = Vec::new();
    let mut cursor = Some(id);
    while let Some(c) = cursor {
        match find(flat, c) {
            Some(a) => {
                chain.push(a.clone());
                cursor = a.area.parent_id;
            }
            None => break,
        }
        if chain.len() > 32 {
            break;
        }
    }
    chain.reverse();
    chain
}

pub fn area_name(flat: &[FlatArea], id: Uuid) -> String {
    find(flat, id).map(|a| a.area.name.clone()).unwrap_or_else(|| "Unknown location".into())
}

/// "Grand Garden › Retaining Wall Terrace"
pub fn area_path_label(flat: &[FlatArea], id: Uuid) -> String {
    path_to(flat, id).iter().map(|a| a.area.name.clone()).collect::<Vec<_>>().join(" › ")
}

pub fn bbox(points: &[Point2D]) -> (f32, f32, f32, f32) {
    let mut b = (100.0f32, 100.0f32, 0.0f32, 0.0f32);
    for p in points {
        b.0 = b.0.min(p.x);
        b.1 = b.1.min(p.y);
        b.2 = b.2.max(p.x);
        b.3 = b.3.max(p.y);
    }
    if points.is_empty() {
        (0.0, 0.0, 100.0, 100.0)
    } else {
        b
    }
}

pub fn centroid(points: &[Point2D]) -> (f32, f32) {
    let (x1, y1, x2, y2) = bbox(points);
    ((x1 + x2) / 2.0, (y1 + y2) / 2.0)
}

pub fn points_attr(points: &[Point2D]) -> String {
    points.iter().map(|p| format!("{:.2},{:.2}", p.x, p.y)).collect::<Vec<_>>().join(" ")
}

// ------------------------------------------------------------------ browser

pub fn window() -> web_sys::Window {
    web_sys::window().expect("running in a browser")
}

pub fn document() -> web_sys::Document {
    window().document().expect("document exists")
}

/// Bounding box of an element by id: (left, top, width, height).
pub fn rect_of(id: &str) -> Option<(f64, f64, f64, f64)> {
    let el = document().get_element_by_id(id)?;
    let r = el.get_bounding_client_rect();
    Some((r.left(), r.top(), r.width(), r.height()))
}

/// Converts a client point into 0–100 percentages of an element.
pub fn percent_in(id: &str, client_x: f64, client_y: f64) -> Option<(f32, f32)> {
    let (l, t, w, h) = rect_of(id)?;
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let x = ((client_x - l) / w * 100.0).clamp(0.0, 100.0) as f32;
    let y = ((client_y - t) / h * 100.0).clamp(0.0, 100.0) as f32;
    Some((x, y))
}

pub fn storage_get(key: &str) -> Option<String> {
    window().local_storage().ok().flatten()?.get_item(key).ok().flatten()
}

pub fn storage_set(key: &str, value: &str) {
    if let Some(s) = window().local_storage().ok().flatten() {
        let _ = s.set_item(key, value);
    }
}

pub fn navigator_online() -> bool {
    window().navigator().on_line()
}

/// Hides the boot splash once the first real screen is painted.
pub fn mark_app_ready() {
    if let Some(body) = document().body() {
        let _ = body.class_list().add_1("app-ready");
    }
}

pub fn scroll_top() {
    window().scroll_to_with_x_and_y(0.0, 0.0);
}

/// `tel:` link with spaces removed.
pub fn tel(phone: &str) -> String {
    format!("tel:{}", phone.chars().filter(|c| !c.is_whitespace()).collect::<String>())
}

pub fn sms(phone: &str) -> String {
    format!("sms:{}", phone.chars().filter(|c| !c.is_whitespace()).collect::<String>())
}

/// Next workday (Mon–Sat) after `after`, keeping the original time of day.
pub fn next_workday_like(original: DateTime<Utc>, after: DateTime<Utc>) -> DateTime<Utc> {
    use chrono::Datelike;
    let orig = local(original);
    let mut day = local(after).date_naive() + Duration::days(1);
    while day.weekday() == chrono::Weekday::Sun {
        day += Duration::days(1);
    }
    let naive = day.and_time(orig.time());
    Local.from_local_datetime(&naive).earliest().map(|d| d.with_timezone(&Utc)).unwrap_or(after + Duration::days(1))
}
