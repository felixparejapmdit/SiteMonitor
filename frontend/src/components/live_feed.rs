//! The "live" layer of the owner's home: a pulse strip with today's numbers and
//! a running feed of what just happened on site. Times tick on their own.

use chrono::{DateTime, Duration, Utc};
use dioxus::prelude::*;
use shared::api::*;

use crate::api;
use crate::components::ui::Icon;
use crate::state::use_app;
use crate::util;
use crate::Route;

/// Bumped every 30 s (see `main.rs`) so "2 min ago" keeps moving without a reload.
pub static TICK: GlobalSignal<u64> = Signal::global(|| 0);

/// Latest happenings across the estate; fetched again whenever projects,
/// photos or decisions change through live updates.
pub fn use_activity() -> Resource<Vec<ActivityItem>> {
    let app = use_app();
    use_resource(move || async move {
        let _ = (app.work_items.read().clone(), app.photo_updates.read().len(), app.decisions.read().as_ref().map(|d| d.len()));
        api::get::<Vec<ActivityItem>>("/activity").await.unwrap_or_default()
    })
}

#[component]
pub fn LiveStrip(people: u32, jobs: usize, machines: usize, last: Option<DateTime<Utc>>) -> Element {
    let _ = TICK();
    let app = use_app();
    let online = (app.ws_connected)();
    rsx! {
        div { class: "card p-4 sm:p-5 flex flex-wrap items-center gap-x-10 gap-y-4", "data-testid": "live-strip",
            div { class: "flex items-center gap-3 min-w-[13rem] flex-1",
                span { class: if online { "live-dot" } else { "live-dot off" } }
                div {
                    p { class: "text-sm font-semibold", if online { "Live from the estate" } else { "Reconnecting…" } }
                    p { class: "text-xs text-muted",
                        match last {
                            Some(t) => format!("Last update {}", util::relative(t)),
                            None => "Waiting for the first update".to_string(),
                        }
                    }
                }
            }
            LiveStat { value: people, label: "people on site" }
            LiveStat { value: jobs as u32, label: "jobs underway" }
            LiveStat { value: machines as u32, label: "machines working" }
        }
    }
}

/// A number that counts up (or down) to its new value instead of jumping.
#[component]
fn LiveStat(value: u32, label: &'static str) -> Element {
    let mut shown = use_signal(|| value);
    let mut target = use_signal(|| value);
    if *target.peek() != value {
        target.set(value);
        spawn(async move {
            let from = *shown.peek() as f64;
            for step in 1..=12 {
                gloo_timers::future::TimeoutFuture::new(40).await;
                if *target.peek() != value {
                    return; // a newer value took over
                }
                shown.set((from + (value as f64 - from) * step as f64 / 12.0).round() as u32);
            }
        });
    }
    rsx! {
        div {
            p { class: if shown() != value { "text-2xl font-semibold tabular leading-none text-accent-ink transition-colors" } else { "text-2xl font-semibold tabular leading-none transition-colors" },
                "{shown}"
            }
            p { class: "text-xs text-muted mt-1", "{label}" }
        }
    }
}

/// The newest site photos as a swipeable strip.
#[component]
pub fn PhotoStrip(items: Vec<ActivityItem>) -> Element {
    let _ = TICK();
    let app = use_app();
    let photos: Vec<ActivityItem> = items.into_iter().filter(|a| a.image_url.is_some()).take(10).collect();
    if photos.is_empty() {
        return rsx! {};
    }
    rsx! {
        section { class: "space-y-3", "data-testid": "photo-strip",
            div { class: "flex items-baseline gap-2",
                h2 { class: "section-title", "Latest from the site" }
                span { class: "text-xs text-muted", "Swipe to see more" }
            }
            div { class: "flex gap-3 overflow-x-auto no-scrollbar snap-x snap-mandatory pb-1",
                for a in photos {
                    {
                        let job = app.work_item(a.work_item_id).map(|w| w.title).unwrap_or_default();
                        let fresh = Utc::now() - a.at < Duration::hours(2);
                        rsx! {
                            Link { key: "{a.id}", to: Route::Project { id: a.work_item_id },
                                class: "relative shrink-0 w-64 sm:w-72 aspect-video rounded-xl overflow-hidden snap-start group bg-hover border border-line",
                                img { src: "{a.image_url.clone().unwrap_or_default()}", alt: "{a.title}", class: "w-full h-full object-cover transition-transform duration-700 group-hover:scale-105" }
                                div { class: "absolute inset-x-0 bottom-0 px-3 py-2.5 bg-gradient-to-t from-black/75 via-black/30 to-transparent text-white",
                                    p { class: "text-sm font-medium truncate", "{job}" }
                                    p { class: "text-xs opacity-80", "{util::relative(a.at)}" }
                                }
                                if fresh {
                                    span { class: "absolute top-2 left-2 pill bg-white/90 text-ink", span { class: "live-dot" } "New" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One short card that answers "what's going on today?".
#[component]
pub fn TodayCard(items: Vec<ActivityItem>) -> Element {
    let _ = TICK();
    let app = use_app();
    let now = Utc::now();
    let today = |t: DateTime<Utc>| util::same_local_day(t, now);
    let finished = items.iter().filter(|a| a.kind == ActivityKind::Milestone && today(a.at)).count();
    let photos = items.iter().filter(|a| a.kind == ActivityKind::Photo && today(a.at)).count();
    let waiting = app
        .decisions
        .read()
        .as_ref()
        .map(|d| d.iter().filter(|d| d.status == shared::DecisionStatus::PublishedPendingClient).count())
        .unwrap_or(0);
    let next = app
        .agenda
        .read()
        .as_ref()
        .and_then(|a| a.items.iter().filter(|i| i.start_time > now && today(i.start_time)).min_by_key(|i| i.start_time).cloned());
    let mut lines: Vec<(&'static str, &'static str, String)> = vec![];
    lines.push(match waiting {
        0 => ("check", "bg-tag-green-bg text-tag-green-text", "Nothing needs your answer".to_string()),
        n => ("bell", "bg-tag-amber-bg text-tag-amber-text", format!("{} waiting for your answer", util::plural(n, "decision"))),
    });
    lines.push(match finished {
        0 => ("clock", "bg-hover text-muted", "No steps finished yet today".to_string()),
        n => ("check", "bg-tag-green-bg text-tag-green-text", format!("{} finished today", util::plural(n, "step"))),
    });
    if photos > 0 {
        lines.push(("image", "bg-tag-blue-bg text-tag-blue-text", format!("{} from the site today", util::plural(photos, "new photo"))));
    }
    lines.push(match next {
        Some(i) => ("calendar", "bg-accent-soft text-accent-ink", format!("Next: {} at {}", i.title, util::time(i.start_time))),
        None => ("calendar", "bg-hover text-muted", "Nothing else on the schedule today".to_string()),
    });
    rsx! {
        section { class: "card p-4 sm:p-5", "data-testid": "today-card",
            p { class: "eyebrow mb-3", "Today at a glance" }
            ul { class: "grid sm:grid-cols-2 gap-x-6 gap-y-3",
                for (i, (icon, tint, text)) in lines.into_iter().enumerate() {
                    li { key: "{i}", class: "flex items-center gap-3 text-sm",
                        span { class: "w-7 h-7 rounded-full flex items-center justify-center shrink-0 {tint}", Icon { name: icon, size: 14 } }
                        "{text}"
                    }
                }
            }
        }
    }
}

#[component]
pub fn LiveFeed(items: Vec<ActivityItem>) -> Element {
    let _ = TICK();
    let app = use_app();
    let mut more = use_signal(|| false);
    let shown = if more() { items.len() } else { items.len().min(6) };
    rsx! {
        section { class: "card overflow-hidden", "data-testid": "live-feed",
            div { class: "px-4 h-12 flex items-center gap-2 border-b border-line",
                span { class: "live-dot" }
                h2 { class: "text-sm font-semibold flex-1", "What's happening now" }
            }
            if items.is_empty() {
                p { class: "px-4 py-6 text-sm text-muted", "Updates from the site will show up here as they happen." }
            }
            ol { class: "divide-y divide-line",
                for a in items.iter().take(shown).cloned() {
                    {
                        let job = app.work_item(a.work_item_id).map(|w| w.title).unwrap_or_default();
                        let fresh = Utc::now() - a.at < Duration::minutes(15);
                        let (icon, tint) = match a.kind {
                            ActivityKind::Milestone => ("check", "bg-tag-green-bg text-tag-green-text"),
                            ActivityKind::Delay => ("clock", "bg-tag-amber-bg text-tag-amber-text"),
                            ActivityKind::Update => ("refresh", "bg-hover text-muted"),
                            ActivityKind::Photo => ("image", "bg-tag-blue-bg text-tag-blue-text"),
                        };
                        rsx! {
                            li { key: "{a.id}", class: "rise-in",
                                Link { to: Route::Project { id: a.work_item_id }, class: "flex gap-3 px-4 py-3 hover:bg-hover/50 transition-colors", "data-testid": "feed-item",
                                    match a.image_url.clone() {
                                        Some(src) => rsx! { img { src: "{src}", alt: "", class: "w-14 aspect-video object-cover rounded-md border border-line shrink-0" } },
                                        None => rsx! { span { class: "w-8 h-8 rounded-full flex items-center justify-center shrink-0 {tint}", Icon { name: icon, size: 14 } } },
                                    }
                                    div { class: "min-w-0 flex-1",
                                        p { class: "text-sm font-medium leading-snug line-clamp-2", "{a.title}" }
                                        p { class: "text-xs text-muted truncate mt-0.5", "{job}" }
                                    }
                                    span { class: if fresh { "text-xs font-medium text-accent-ink whitespace-nowrap" } else { "text-xs text-muted whitespace-nowrap" },
                                        "{util::relative(a.at)}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if items.len() > 6 {
                button { class: "w-full h-10 text-sm text-muted hover:bg-hover hover:text-ink border-t border-line transition-colors", onclick: move |_| more.toggle(),
                    if more() { "Show less" } else { "Show more" }
                }
            }
        }
    }
}
