//! Dual-view calendar.
//!
//! * Owner: a calm chronological "Estate Agenda" — deadlines, disruptive
//!   work and their special occasions only — plus the Quiet Day override.
//! * Staff: the Master Scheduler with the Quiet Day conflict queue,
//!   machinery double-booking warnings and a drag-and-drop week board.

use std::collections::BTreeMap;

use chrono::{Duration, Local, NaiveDate, TimeZone, Utc};
use dioxus::prelude::*;
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use shared::Asset;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{EmptyState, Icon, Modal, Pill, SkeletonPage, Spinner};
use crate::services::sync::{self, Outcome};
use crate::state::use_app;
use crate::util;
use crate::Route;

#[component]
pub fn AgendaView() -> Element {
    let app = use_app();
    if app.agenda.read().is_none() {
        return rsx! { SkeletonPage {} };
    }
    if app.is_vip() {
        rsx! { VipAgenda {} }
    } else {
        rsx! { MasterScheduler {} }
    }
}

fn day_label(d: NaiveDate) -> String {
    let today = Local::now().date_naive();
    if d == today {
        "Today".into()
    } else if d == today + Duration::days(1) {
        "Tomorrow".into()
    } else {
        d.format("%A, %B %-d").to_string()
    }
}

fn agenda_tone(i: &EstateAgendaItem) -> Tone {
    match (i.event_type, i.intrusion_level) {
        (AgendaEventType::VipSpecialOccasion, _) => Tone::Blue,
        (AgendaEventType::TargetDeadline, _) => Tone::Green,
        (_, IntrusionLevel::NoiseHeavy) => Tone::Red,
        (_, IntrusionLevel::UtilityCut) => Tone::Amber,
        _ => Tone::Gray,
    }
}

// ================================================================== owner

#[component]
fn VipAgenda() -> Element {
    let app = use_app();
    let mut booking = use_signal(|| false);
    let mut confirmation = use_signal(|| None::<QuietDayResponse>);
    let data = app.agenda.read().clone().unwrap_or(AgendaResponse { items: vec![], reservations: vec![], equipment_conflicts: vec![] });
    let flat = app.flat_areas();
    let now = Utc::now();
    let horizon = now + Duration::days(45);
    let shown: Vec<EstateAgendaItem> = data
        .items
        .iter()
        .filter(|i| i.end_time >= now && i.start_time <= horizon)
        .filter(|i| match i.event_type {
            AgendaEventType::TargetDeadline | AgendaEventType::VipSpecialOccasion => true,
            _ => i.intrusion_level.is_intrusive(),
        })
        .cloned()
        .collect();
    let mut by_day: BTreeMap<NaiveDate, Vec<EstateAgendaItem>> = BTreeMap::new();
    for i in shown {
        by_day.entry(util::local(i.start_time).date_naive()).or_default().push(i);
    }
    let cards: Vec<Notification> = app.notifications.read().iter().filter(|n| n.kind == NotificationKind::AnticipationCard).cloned().collect();
    let items = app.work_items.read().clone().unwrap_or_default();

    rsx! {
        div { class: "max-w-doc mx-auto space-y-8 rise-in",
            header { class: "flex flex-col sm:flex-row sm:items-end justify-between gap-4",
                div {
                    p { class: "eyebrow", "Next six weeks" }
                    h1 { class: "doc-title mt-1", "Estate Agenda" }
                    p { class: "text-[15px] text-muted mt-1", "Only what affects you: handovers, noisy work, water or power interruptions, and your occasions." }
                }
                button { class: "btn-primary btn-lg shrink-0", "data-testid": "schedule-quiet-day", onclick: move |_| booking.set(true),
                    Icon { name: "plus", size: 18 }
                    "Schedule Quiet Day / Special Occasion"
                }
            }

            if let Some(c) = confirmation() {
                div { class: "rounded-xl bg-tag-green-bg px-5 py-4 text-tag-green-text space-y-2 rise-in", "data-testid": "quiet-day-confirmation",
                    div { class: "flex items-start gap-3",
                        Icon { name: "check", size: 18, class: "mt-0.5" }
                        div { class: "flex-1",
                            p { class: "font-semibold", "{c.occasion.title} — {util::range(c.occasion.start_time, c.occasion.end_time)}" }
                            p { class: "text-[15px] mt-1", "{c.confirmation}" }
                            if !c.flagged_items.is_empty() {
                                p { class: "text-sm mt-2 opacity-90", "Paused and being moved: {c.flagged_items.iter().map(|f| f.title.clone()).collect::<Vec<_>>().join(\", \")}." }
                            }
                        }
                        button { class: "btn-icon btn-ghost text-tag-green-text", "aria-label": "Dismiss", onclick: move |_| confirmation.set(None), Icon { name: "x" } }
                    }
                }
            }

            if !cards.is_empty() {
                div { class: "space-y-3",
                    for n in cards {
                        div { key: "{n.id}", class: "card p-5 border-[#cfe0cc] bg-[#f7fbf6]", "data-testid": "anticipation-card",
                            p { class: "eyebrow text-tag-green-text", "Coming up" }
                            p { class: "font-semibold mt-0.5", "{n.title}" }
                            p { class: "text-[15px] text-muted mt-1", "{n.body}" }
                        }
                    }
                }
            }

            if by_day.is_empty() {
                EmptyState { icon: "calendar", title: "A quiet few weeks ahead", body: "Nothing disruptive is planned." }
            }
            div { class: "space-y-8",
                for (day, list) in by_day {
                    section { key: "{day}",
                        h2 { class: "text-sm font-semibold text-muted mb-2", "{day_label(day)}" }
                        ul { class: "border-l-2 border-line pl-5 space-y-4",
                            for i in list {
                                li { key: "{i.id}", class: "relative", "data-testid": "agenda-item",
                                    span { class: "absolute -left-[27px] top-1.5 w-3 h-3 rounded-full ring-4 ring-white {agenda_tone(&i).pill_class()}" }
                                    div { class: "flex flex-wrap items-center gap-2",
                                        p { class: "text-[15px] font-medium", "{i.title}" }
                                        Pill { tone: agenda_tone(&i), text: match i.event_type {
                                            AgendaEventType::VipSpecialOccasion => "Your occasion".to_string(),
                                            AgendaEventType::TargetDeadline => "Handover".to_string(),
                                            _ => i.intrusion_level.label().to_string(),
                                        } }
                                        if i.is_flagged_conflict {
                                            Pill { tone: Tone::Gray, text: "Paused for your occasion" }
                                        }
                                    }
                                    p { class: "text-sm text-muted mt-0.5",
                                        "{util::range(i.start_time, i.end_time)}"
                                        if let Some(a) = i.area_id { " · {util::area_name(&flat, a)}" }
                                    }
                                    if !i.description.is_empty() {
                                        p { class: "text-[15px] mt-1", "{i.description}" }
                                    }
                                    if let Some(w) = i.work_item_id.and_then(|w| items.iter().find(|x| x.id == w)) {
                                        if i.event_type == AgendaEventType::TargetDeadline {
                                            Link { to: Route::Project { id: w.id }, class: "inline-flex items-center gap-1 text-sm text-muted hover:text-ink mt-1", "{w.calculated_progress}% complete · view project" Icon { name: "arrow-right", size: 12 } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if booking() {
            QuietDayModal {
                on_close: move |_| booking.set(false),
                on_done: move |r: QuietDayResponse| {
                    confirmation.set(Some(r));
                    booking.set(false);
                },
            }
        }
    }
}

#[component]
fn QuietDayModal(on_close: EventHandler<()>, on_done: EventHandler<QuietDayResponse>) -> Element {
    let app = use_app();
    let default_start = Local::now().date_naive() + Duration::days(4);
    let start_default = Local.from_local_datetime(&default_start.and_hms_opt(8, 0, 0).unwrap()).earliest().unwrap().with_timezone(&Utc);
    let mut title = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut start = use_signal(|| util::to_input(start_default));
    let mut end = use_signal(|| util::to_input(start_default + Duration::hours(14)));
    let mut busy = use_signal(|| false);
    let valid = !title().trim().is_empty()
        && util::from_input(&start()).zip(util::from_input(&end())).map(|(s, e)| e > s).unwrap_or(false);
    rsx! {
        Modal { title: "Schedule a Quiet Day", on_close: move |_| on_close.call(()),
            div { class: "space-y-5",
                p { class: "text-[15px] text-muted", "During this time all heavy machinery and any water or power interruptions are stopped automatically. Quiet garden and pool care can still happen beforehand." }
                div {
                    label { class: "label", r#for: "qd-title", "What's the occasion?" }
                    input { id: "qd-title", class: "field", "data-testid": "qd-title", placeholder: "e.g. Weekend Estate Gala", value: "{title}", oninput: move |e| title.set(e.value()) }
                }
                div { class: "grid sm:grid-cols-2 gap-4",
                    div {
                        label { class: "label", r#for: "qd-start", "Starts" }
                        input { id: "qd-start", r#type: "datetime-local", class: "field", "data-testid": "qd-start", value: "{start}", oninput: move |e| start.set(e.value()) }
                    }
                    div {
                        label { class: "label", r#for: "qd-end", "Ends" }
                        input { id: "qd-end", r#type: "datetime-local", class: "field", "data-testid": "qd-end", value: "{end}", oninput: move |e| end.set(e.value()) }
                    }
                }
                div {
                    label { class: "label", r#for: "qd-notes", "Anything the team should know? (optional)" }
                    textarea { id: "qd-notes", class: "field", rows: "2", value: "{description}", oninput: move |e| description.set(e.value()) }
                }
                div { class: "flex justify-end gap-2",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button {
                        class: "btn-primary btn-lg",
                        disabled: !valid || busy(),
                        "data-testid": "qd-submit",
                        onclick: move |_| async move {
                            let (Some(s), Some(e)) = (util::from_input(&start()), util::from_input(&end())) else { return };
                            busy.set(true);
                            let req = QuietDayRequest { title: title(), description: description(), start_time: s, end_time: e };
                            match api::post::<_, QuietDayResponse>("/agenda/quiet-day", &req).await {
                                Ok(r) => {
                                    sync::reload(app, vec![DataScope::Agenda, DataScope::WorkItems, DataScope::Map]);
                                    on_done.call(r);
                                }
                                Err(e) => app.error(e.message()),
                            }
                            busy.set(false);
                        },
                        if busy() { Spinner {} }
                        "Confirm Quiet Day"
                    }
                }
            }
        }
    }
}

// ================================================================== staff

#[component]
fn MasterScheduler() -> Element {
    let app = use_app();
    let mut week_offset = use_signal(|| 0i64);
    let mut open_item = use_signal(|| None::<EstateAgendaItem>);
    let mut adding = use_signal(|| false);
    let mut drag_over = use_signal(|| None::<NaiveDate>);
    let data = app.agenda.read().clone().unwrap_or(AgendaResponse { items: vec![], reservations: vec![], equipment_conflicts: vec![] });
    let flat = app.flat_areas();
    let assets = app.assets.read().as_ref().map(|a| a.assets.clone()).unwrap_or_default();
    let is_admin = app.is_admin();

    let today = Local::now().date_naive();
    let monday = today + Duration::weeks(week_offset());
    let days: Vec<NaiveDate> = (0..7).map(|i| monday + Duration::days(i)).collect();
    let flagged: Vec<EstateAgendaItem> = data.items.iter().filter(|i| i.is_flagged_conflict).cloned().collect();
    let occasions: Vec<EstateAgendaItem> = data.items.iter().filter(|i| i.event_type == AgendaEventType::VipSpecialOccasion && i.end_time > Utc::now()).cloned().collect();

    let move_item = move |item: EstateAgendaItem, start: chrono::DateTime<Utc>| async move {
        let length = item.end_time - item.start_time;
        let req = RescheduleRequest { start_time: start, end_time: start + length };
        let outcome: Outcome<EstateAgendaItem> = sync::mutate(app, format!("Reschedule: {}", item.title), "PUT", format!("/agenda/{}/reschedule", item.id), &req).await;
        match outcome {
            Outcome::Saved(saved) => {
                if saved.is_flagged_conflict {
                    app.toast(crate::state::ToastTone::Warning, "Still clashes with the Quiet Day", Some("Pick a day after the owner's occasion.".into()));
                } else {
                    app.success(format!("{} moved to {}", saved.title, util::day_time(saved.start_time)));
                }
                sync::reload(app, vec![DataScope::Agenda, DataScope::WorkItems, DataScope::Map]);
            }
            Outcome::Queued => {}
            Outcome::Failed(e) => app.error(e),
        }
    };

    rsx! {
        div { class: "space-y-6 rise-in",
            header { class: "flex flex-wrap items-end justify-between gap-3",
                div {
                    h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Master Schedule" }
                    p { class: "text-[15px] text-muted mt-1", "All upkeep, repairs and machinery bookings. Drag a task onto another day to move it." }
                }
                if is_admin {
                    button { class: "btn-primary", "data-testid": "add-schedule", onclick: move |_| adding.set(true), Icon { name: "plus" } "Add to schedule" }
                }
            }

            // -------------------------------------------- quiet day queue
            if !flagged.is_empty() {
                section { class: "rounded-xl border border-[#f3c9cb] bg-tag-red-bg/70 overflow-hidden", "data-testid": "conflict-queue",
                    div { class: "px-5 py-3 flex items-center gap-2 text-tag-red-text",
                        Icon { name: "moon" }
                        p { class: "font-semibold flex-1", "Quiet Day conflicts — {util::plural(flagged.len(), \"task\")} must move" }
                    }
                    ul { class: "divide-y divide-[#f3c9cb] bg-white/60",
                        for f in flagged {
                            {
                                let blocking = occasions.iter().filter(|o| o.start_time < f.end_time && f.start_time < o.end_time).max_by_key(|o| o.end_time).cloned();
                                let after = blocking.as_ref().map(|o| o.end_time).unwrap_or(f.end_time);
                                let target = util::next_workday_like(f.start_time, after);
                                let f2 = f.clone();
                                rsx! {
                                    li {
                                        key: "{f.id}",
                                        class: "px-5 py-3 flex flex-col sm:flex-row sm:items-center gap-3 cursor-grab",
                                        draggable: "true",
                                        "data-testid": "conflict-item",
                                        ondragstart: move |e: DragEvent| { let _ = e.data_transfer().set_data("text/plain", &f2.id.to_string()); },
                                        div { class: "flex-1 min-w-0",
                                            p { class: "text-[15px] font-medium text-tag-red-text", "{f.title}" }
                                            p { class: "text-sm text-muted",
                                                "{util::day(f.start_time)} · {util::range(f.start_time, f.end_time)}"
                                                if let Some(a) = f.area_id { " · {util::area_name(&flat, a)}" }
                                                " · {f.intrusion_level.label()}"
                                            }
                                            if let Some(o) = blocking {
                                                p { class: "text-xs text-muted mt-0.5", "Clashes with \u{201c}{o.title}\u{201d}" }
                                            }
                                        }
                                        {
                                            let f3 = f.clone();
                                            rsx! {
                                                button {
                                                    class: "btn-primary",
                                                    "data-testid": "move-next-workday",
                                                    onclick: move |_| move_item(f3.clone(), target),
                                                    "Move to {util::day(target)}"
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

            // -------------------------------------------- equipment conflicts
            if !data.equipment_conflicts.is_empty() {
                section { class: "rounded-xl border border-[#f0e2b6] bg-tag-amber-bg/70 px-5 py-3 space-y-2", "data-testid": "equipment-conflicts",
                    p { class: "font-semibold text-tag-amber-text flex items-center gap-2", Icon { name: "alert" } "Machinery double-booked" }
                    for c in data.equipment_conflicts.iter() {
                        {
                            let day = NaiveDate::parse_from_str(&c.day, "%Y-%m-%d").map(day_label).unwrap_or(c.day.clone());
                            let zones = c.area_ids.iter().map(|a| util::area_name(&flat, *a)).collect::<Vec<_>>().join(" and ");
                            rsx! {
                                p { key: "{c.asset_id}-{c.day}", class: "text-[15px] text-tag-amber-text",
                                    span { class: "font-medium", "{c.asset_name}" }
                                    " is booked in {zones} on {day}. Move one of the jobs or bring in another machine."
                                }
                            }
                        }
                    }
                }
            }

            // -------------------------------------------- week board
            section { class: "space-y-3",
                div { class: "flex items-center gap-2",
                    button { class: "btn-icon btn-secondary", "aria-label": "Previous week", onclick: move |_| week_offset.set(week_offset() - 1), Icon { name: "chevron-left" } }
                    button { class: "btn-icon btn-secondary", "aria-label": "Next week", onclick: move |_| week_offset.set(week_offset() + 1), Icon { name: "chevron-right" } }
                    p { class: "font-medium ml-1", "{monday.format(\"%B %-d\")} – {(monday + Duration::days(6)).format(\"%B %-d\")}" }
                    if week_offset() != 0 {
                        button { class: "btn-ghost h-8 ml-auto", onclick: move |_| week_offset.set(0), "Today" }
                    }
                }
                div { class: "grid grid-flow-col auto-cols-[minmax(190px,1fr)] lg:grid-cols-7 lg:auto-cols-auto gap-2 overflow-x-auto no-scrollbar pb-2", "data-testid": "week-board",
                    for d in days {
                        {
                            let list: Vec<EstateAgendaItem> = data.items.iter().filter(|i| util::local(i.start_time).date_naive() == d).cloned().collect();
                            let is_today = d == today;
                            let quiet = occasions.iter().any(|o| util::local(o.start_time).date_naive() <= d && util::local(o.end_time).date_naive() >= d);
                            let items_for_drop = data.items.clone();
                            rsx! {
                                div {
                                    key: "{d}",
                                    class: if drag_over() == Some(d) { "rounded-lg border-2 border-ink bg-hover min-h-48 transition-colors" } else if quiet { "rounded-lg border border-[#c9d8e8] bg-tag-blue-bg/50 min-h-48 transition-colors" } else { "rounded-lg border border-line bg-sidebar min-h-48 transition-colors" },
                                    "data-testid": "day-{d}",
                                    ondragover: move |e: DragEvent| { e.prevent_default(); drag_over.set(Some(d)); },
                                    ondragleave: move |_| { if drag_over() == Some(d) { drag_over.set(None); } },
                                    ondrop: move |e: DragEvent| {
                                        e.prevent_default();
                                        drag_over.set(None);
                                        let id = e.data_transfer().get_data("text/plain").and_then(|s| Uuid::parse_str(&s).ok());
                                        if let Some(item) = id.and_then(|id| items_for_drop.iter().find(|i| i.id == id).cloned()) {
                                            if item.event_type == AgendaEventType::VipSpecialOccasion {
                                                app.error("Only the owner can change their occasion.");
                                                return;
                                            }
                                            let t = util::local(item.start_time).time();
                                            if let Some(start) = Local.from_local_datetime(&d.and_time(t)).earliest() {
                                                spawn(move_item(item, start.with_timezone(&Utc)));
                                            }
                                        }
                                    },
                                    div { class: "px-3 py-2 border-b border-line/80 flex items-baseline justify-between",
                                        p { class: if is_today { "text-sm font-semibold" } else { "text-sm font-medium text-muted" }, "{d.format(\"%a %-d\")}" }
                                        if quiet { span { class: "text-[11px] text-tag-blue-text font-medium", "Quiet Day" } }
                                    }
                                    div { class: "p-2 space-y-1.5",
                                        for i in list {
                                            {
                                                let i2 = i.clone();
                                                let i3 = i.clone();
                                                let movable = i.event_type != AgendaEventType::VipSpecialOccasion;
                                                let machinery: Vec<String> = data.reservations.iter().filter(|r| r.agenda_item_id == i.id).filter_map(|r| assets.iter().find(|a| a.id == r.asset_id).map(|a| a.name.clone())).collect();
                                                rsx! {
                                                    button {
                                                        key: "{i.id}",
                                                        class: if i.is_flagged_conflict { "w-full text-left rounded-md bg-white border-l-4 border-tag-red-text px-2 py-1.5 shadow-sm hover:shadow transition-shadow" } else { "w-full text-left rounded-md bg-white border-l-4 border-transparent px-2 py-1.5 shadow-sm hover:shadow transition-shadow" },
                                                        style: if !i.is_flagged_conflict { match agenda_tone(&i) { Tone::Red => "border-left-color:#e7a9ab", Tone::Amber => "border-left-color:#e9c66c", Tone::Green => "border-left-color:#9cc298", Tone::Blue => "border-left-color:#8fb3d6", _ => "border-left-color:#d6d6d3" } } else { "" },
                                                        draggable: if movable { "true" } else { "false" },
                                                        "data-testid": "board-item",
                                                        ondragstart: move |e: DragEvent| { let _ = e.data_transfer().set_data("text/plain", &i2.id.to_string()); },
                                                        onclick: move |_| open_item.set(Some(i3.clone())),
                                                        p { class: "text-xs text-muted tabular", "{util::time(i.start_time)}" }
                                                        p { class: "text-[13px] font-medium leading-snug", "{i.title}" }
                                                        if !machinery.is_empty() {
                                                            p { class: "text-[11px] text-muted mt-0.5 flex items-center gap-1", Icon { name: "truck", size: 11 } "{machinery.join(\", \")}" }
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
                div { class: "flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted",
                    span { class: "inline-flex items-center gap-1.5", span { class: "w-3 h-1 rounded bg-[#e7a9ab]" } "Noisy" }
                    span { class: "inline-flex items-center gap-1.5", span { class: "w-3 h-1 rounded bg-[#e9c66c]" } "Water / power interruption" }
                    span { class: "inline-flex items-center gap-1.5", span { class: "w-3 h-1 rounded bg-[#9cc298]" } "Handover" }
                    span { class: "inline-flex items-center gap-1.5", span { class: "w-3 h-1 rounded bg-[#8fb3d6]" } "Owner's occasion" }
                    span { class: "inline-flex items-center gap-1.5", span { class: "w-3 h-1 rounded bg-[#d6d6d3]" } "Quiet / routine" }
                }
            }
        }

        if let Some(item) = open_item() {
            ItemModal { item, on_close: move |_| open_item.set(None) }
        }
        if adding() {
            AddScheduleModal { on_close: move |_| adding.set(false) }
        }
    }
}

#[component]
fn ItemModal(item: EstateAgendaItem, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let flat = app.flat_areas();
    let mut start = use_signal(|| util::to_input(item.start_time));
    let mut end = use_signal(|| util::to_input(item.end_time));
    let mut busy = use_signal(|| false);
    let id = item.id;
    let editable = item.event_type != AgendaEventType::VipSpecialOccasion;
    let machinery: Vec<String> = app
        .agenda
        .read()
        .as_ref()
        .map(|a| a.reservations.iter().filter(|r| r.agenda_item_id == id).map(|r| r.asset_id).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|aid| app.assets.read().as_ref().and_then(|a| a.assets.iter().find(|x| x.id == aid).map(|x| x.name.clone())))
        .collect();
    rsx! {
        Modal { title: item.title.clone(), on_close: move |_| on_close.call(()),
            div { class: "space-y-4",
                div { class: "flex flex-wrap gap-2",
                    Pill { tone: agenda_tone(&item), text: item.event_type.label() }
                    Pill { tone: Tone::Gray, text: item.intrusion_level.label() }
                    if item.is_flagged_conflict {
                        Pill { tone: Tone::Red, text: "Clashes with a Quiet Day" }
                    }
                }
                p { class: "text-[15px]", "{item.description}" }
                p { class: "text-sm text-muted",
                    "{util::day(item.start_time)} · {util::range(item.start_time, item.end_time)}"
                    if let Some(a) = item.area_id { " · {util::area_name(&flat, a)}" }
                }
                if !machinery.is_empty() {
                    p { class: "text-sm", span { class: "text-muted", "Machinery: " } "{machinery.join(\", \")}" }
                }
                if let Some(w) = item.work_item_id {
                    Link { to: Route::Project { id: w }, class: "btn-secondary", onclick: move |_| on_close.call(()), "Open project" }
                }
                if editable {
                    div { class: "border-t border-line pt-4 space-y-3",
                        p { class: "label", "Reschedule" }
                        div { class: "grid sm:grid-cols-2 gap-3",
                            input { r#type: "datetime-local", class: "field", "aria-label": "New start", value: "{start}", oninput: move |e| start.set(e.value()) }
                            input { r#type: "datetime-local", class: "field", "aria-label": "New end", value: "{end}", oninput: move |e| end.set(e.value()) }
                        }
                        div { class: "flex justify-end",
                            button {
                                class: "btn-primary",
                                disabled: busy(),
                                "data-testid": "save-reschedule",
                                onclick: move |_| async move {
                                    let (Some(s), Some(e)) = (util::from_input(&start()), util::from_input(&end())) else { app.error("Please pick valid dates."); return };
                                    busy.set(true);
                                    let outcome: Outcome<EstateAgendaItem> = sync::mutate(app, "Reschedule", "PUT", format!("/agenda/{id}/reschedule"), &RescheduleRequest { start_time: s, end_time: e }).await;
                                    match outcome {
                                        Outcome::Saved(_) => {
                                            app.success("Rescheduled");
                                            sync::reload(app, vec![DataScope::Agenda, DataScope::WorkItems, DataScope::Map]);
                                            on_close.call(());
                                        }
                                        Outcome::Queued => on_close.call(()),
                                        Outcome::Failed(e) => app.error(e),
                                    }
                                    busy.set(false);
                                },
                                "Save new time"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn AddScheduleModal(on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let flat: Vec<util::FlatArea> = app.flat_areas().into_iter().filter(|a| !a.area.is_archived).collect();
    let items: Vec<WorkItem> = app.work_items.read().clone().unwrap_or_default().into_iter().filter(|w| w.status.is_active()).collect();
    let machines: Vec<Asset> = app.assets.read().as_ref().map(|a| a.assets.iter().filter(|a| a.mobility == MobilityType::MobileMachinery).cloned().collect()).unwrap_or_default();
    let tomorrow = Local::now().date_naive() + Duration::days(1);
    let start_default = Local.from_local_datetime(&tomorrow.and_hms_opt(8, 0, 0).unwrap()).earliest().unwrap().with_timezone(&Utc);
    let mut title = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut area = use_signal(|| None::<Uuid>);
    let mut project = use_signal(|| None::<Uuid>);
    let mut kind = use_signal(|| AgendaEventType::IntrusiveMaintenance);
    let mut level = use_signal(|| IntrusionLevel::NoiseHeavy);
    let mut start = use_signal(|| util::to_input(start_default));
    let mut end = use_signal(|| util::to_input(start_default + Duration::hours(3)));
    let mut booked = use_signal(Vec::<Uuid>::new);
    let mut busy = use_signal(|| false);
    rsx! {
        Modal { title: "Add to schedule", wide: true, on_close: move |_| on_close.call(()),
            div { class: "space-y-4",
                div {
                    label { class: "label", "Title" }
                    input { class: "field", "data-testid": "sched-title", placeholder: "e.g. Crane lift — gazebo roof beams", value: "{title}", oninput: move |e| title.set(e.value()) }
                }
                div { class: "grid sm:grid-cols-2 gap-4",
                    div {
                        label { class: "label", "Where" }
                        select { class: "field", "data-testid": "sched-area", onchange: move |e| area.set(Uuid::parse_str(&e.value()).ok()),
                            option { value: "", "Choose a zone…" }
                            for a in flat.iter() {
                                option { key: "{a.area.id}", value: "{a.area.id}", "{\"\u{2003}\".repeat(a.depth)}{a.area.name}" }
                            }
                        }
                    }
                    div {
                        label { class: "label", "Project (optional)" }
                        select { class: "field", onchange: move |e| project.set(Uuid::parse_str(&e.value()).ok()),
                            option { value: "", "None" }
                            for w in items.iter() {
                                option { key: "{w.id}", value: "{w.id}", "{w.title}" }
                            }
                        }
                    }
                    div {
                        label { class: "label", "Type" }
                        select { class: "field", onchange: move |e| { if let Some(k) = AgendaEventType::ALL.iter().find(|k| format!("{k:?}") == e.value()) { kind.set(*k); } },
                            for k in AgendaEventType::ALL.iter().filter(|k| **k != AgendaEventType::VipSpecialOccasion) {
                                option { key: "{k:?}", value: "{k:?}", selected: kind() == *k, "{k.label()}" }
                            }
                        }
                    }
                    div {
                        label { class: "label", "How disruptive?" }
                        select { class: "field", "data-testid": "sched-level", onchange: move |e| { if let Some(l) = IntrusionLevel::ALL.iter().find(|l| format!("{l:?}") == e.value()) { level.set(*l); } },
                            for l in IntrusionLevel::ALL {
                                option { key: "{l:?}", value: "{l:?}", selected: level() == l, "{l.label()}" }
                            }
                        }
                    }
                    div {
                        label { class: "label", "Starts" }
                        input { r#type: "datetime-local", class: "field", "data-testid": "sched-start", value: "{start}", oninput: move |e| start.set(e.value()) }
                    }
                    div {
                        label { class: "label", "Ends" }
                        input { r#type: "datetime-local", class: "field", "data-testid": "sched-end", value: "{end}", oninput: move |e| end.set(e.value()) }
                    }
                }
                div {
                    p { class: "label", "Machinery needed" }
                    div { class: "flex flex-wrap gap-2",
                        for m in machines {
                            {
                                let on = booked().contains(&m.id);
                                rsx! {
                                    button {
                                        key: "{m.id}",
                                        r#type: "button",
                                        class: if on { "pill h-8 px-3 bg-ink text-white" } else { "pill h-8 px-3 bg-hover text-ink hover:bg-[#e8e8e5]" },
                                        "aria-pressed": on,
                                        onclick: move |_| {
                                            let mut b = booked.write();
                                            if let Some(pos) = b.iter().position(|x| *x == m.id) { b.remove(pos); } else { b.push(m.id); }
                                        },
                                        "{m.name}"
                                    }
                                }
                            }
                        }
                    }
                }
                div {
                    label { class: "label", "Notes" }
                    textarea { class: "field", rows: "2", value: "{description}", oninput: move |e| description.set(e.value()) }
                }
                div { class: "flex justify-end gap-2",
                    button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                    button {
                        class: "btn-primary",
                        disabled: busy() || title().trim().is_empty(),
                        "data-testid": "sched-save",
                        onclick: move |_| async move {
                            let (Some(s), Some(e)) = (util::from_input(&start()), util::from_input(&end())) else { app.error("Please pick valid dates."); return };
                            busy.set(true);
                            let req = CreateAgendaItemRequest {
                                area_id: area(),
                                work_item_id: project(),
                                title: title(),
                                description: description(),
                                event_type: kind(),
                                intrusion_level: level(),
                                start_time: s,
                                end_time: e,
                                asset_ids: booked(),
                            };
                            let outcome: Outcome<EstateAgendaItem> = sync::mutate(app, "Add schedule item", "POST", "/agenda", &req).await;
                            match outcome {
                                Outcome::Saved(saved) => {
                                    if saved.is_flagged_conflict {
                                        app.toast(crate::state::ToastTone::Warning, "Added — but it clashes with a Quiet Day", Some("It's flagged at the top so you can move it.".into()));
                                    } else {
                                        app.success("Added to the schedule");
                                    }
                                    sync::reload(app, vec![DataScope::Agenda]);
                                    on_close.call(());
                                }
                                Outcome::Queued => on_close.call(()),
                                Outcome::Failed(e) => app.error(e),
                            }
                            busy.set(false);
                        },
                        if busy() { Spinner {} }
                        "Add"
                    }
                }
            }
        }
    }
}
