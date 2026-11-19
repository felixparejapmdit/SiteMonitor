//! The "Draft-to-VIP" decision pipeline.
//!
//! 1. A Site Monitor snaps a 16:9 photo, flags an issue and sends it to the
//!    Admin Review Queue.
//! 2. The Office Admin polishes the wording, defines Option A / Option B (or
//!    a simple acknowledgement) and clicks [Publish to VIP Client].

use dioxus::prelude::*;
use shared::api::*;
use shared::ws::DataScope;
use shared::*;
use uuid::Uuid;

use crate::components::ui::{Avatar, EmptyState, Icon, Modal, PhotoCapture, Pill, Spinner};
use crate::services::sync::{self, Outcome};
use crate::state::use_app;
use crate::util;
use crate::views::staff::asset_ops::ChoiceButton;
use crate::Route;

#[component]
pub fn DecisionsView() -> Element {
    let app = use_app();
    if app.can(Permission::PublishDecisions) {
        rsx! { OfficeInbox {} }
    } else {
        rsx! { ReporterView {} }
    }
}

/// Site team: file a report, then follow what the office did with it. The
/// server only sends this person's own reports.
#[component]
fn ReporterView() -> Element {
    let app = use_app();
    let mut mine = app.decisions.read().clone().unwrap_or_default();
    // What needs me first, then the most recent outcome.
    mine.sort_by_key(|d| (d.status != DecisionStatus::NeedsDetails, std::cmp::Reverse(d.resolved_at.unwrap_or(d.created_at))));
    let mut filter = use_signal(|| None::<Outcome4Reporter>);
    let shown: Vec<DecisionAlert> = mine.iter().filter(|d| filter().is_none_or(|f| f.holds(d.status))).cloned().collect();
    rsx! {
        div { class: "space-y-6 rise-in",
            header { class: "space-y-1",
                h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Report an issue" }
                p { class: "text-[15px] text-muted", "Spotted a problem? Tell the office in under a minute. It works without signal too." }
            }
            FieldReportForm {}
            section { class: "space-y-3",
                div { class: "flex flex-wrap items-center gap-2",
                    h2 { class: "section-title mr-2", "Your reports" }
                    FilterChip { label: "All".to_string(), count: mine.len(), active: filter().is_none(), on_click: move |_| filter.set(None) }
                    for f in Outcome4Reporter::ALL {
                        {
                            let n = mine.iter().filter(|d| f.holds(d.status)).count();
                            rsx! {
                                if n > 0 {
                                    FilterChip { key: "{f.label()}", label: f.label().to_string(), count: n, active: filter() == Some(f), on_click: move |_| filter.set(Some(f)) }
                                }
                            }
                        }
                    }
                }
                DecisionList { items: shown, empty: if mine.is_empty() { "You haven't sent any reports yet." } else { "No reports with this status." } }
            }
        }
    }
}

/// How a report stands, in the reporter's words.
#[derive(Clone, Copy, PartialEq)]
enum Outcome4Reporter {
    Waiting,
    NeedsDetails,
    Approved,
    Declined,
    OnHold,
    WithOwner,
}

impl Outcome4Reporter {
    const ALL: [Outcome4Reporter; 6] = [
        Outcome4Reporter::NeedsDetails,
        Outcome4Reporter::Waiting,
        Outcome4Reporter::Approved,
        Outcome4Reporter::Declined,
        Outcome4Reporter::OnHold,
        Outcome4Reporter::WithOwner,
    ];
    fn of(s: DecisionStatus) -> Self {
        match s {
            DecisionStatus::DraftInAdminQueue => Outcome4Reporter::Waiting,
            DecisionStatus::NeedsDetails => Outcome4Reporter::NeedsDetails,
            DecisionStatus::OfficeApproved => Outcome4Reporter::Approved,
            DecisionStatus::OfficeDeclined => Outcome4Reporter::Declined,
            DecisionStatus::OnHold => Outcome4Reporter::OnHold,
            _ => Outcome4Reporter::WithOwner,
        }
    }
    fn holds(self, s: DecisionStatus) -> bool {
        Self::of(s) == self
    }
    fn label(self) -> &'static str {
        match self {
            Outcome4Reporter::Waiting => "Waiting for office review",
            Outcome4Reporter::NeedsDetails => "Needs more details",
            Outcome4Reporter::Approved => "Approved",
            Outcome4Reporter::Declined => "Declined",
            Outcome4Reporter::OnHold => "On hold",
            Outcome4Reporter::WithOwner => "Sent to the owner",
        }
    }
    fn icon(self) -> &'static str {
        match self {
            Outcome4Reporter::Waiting => "clock",
            Outcome4Reporter::NeedsDetails => "message",
            Outcome4Reporter::Approved => "check",
            Outcome4Reporter::Declined => "x",
            Outcome4Reporter::OnHold => "pause",
            Outcome4Reporter::WithOwner => "send",
        }
    }
    /// (badge classes, left edge colour)
    fn style(self) -> (&'static str, &'static str) {
        match self {
            Outcome4Reporter::Waiting => ("bg-tag-gray-bg text-tag-gray-text", "border-l-line"),
            Outcome4Reporter::NeedsDetails => ("bg-tag-blue-bg text-tag-blue-text", "border-l-tag-blue-text"),
            Outcome4Reporter::Approved => ("bg-tag-green-bg text-tag-green-text", "border-l-tag-green-text"),
            Outcome4Reporter::Declined => ("bg-tag-red-bg text-tag-red-text", "border-l-tag-red-text"),
            Outcome4Reporter::OnHold => ("bg-tag-amber-bg text-tag-amber-text", "border-l-tag-amber-text"),
            Outcome4Reporter::WithOwner => ("bg-accent-soft text-accent-ink", "border-l-accent"),
        }
    }
}

#[component]
fn FilterChip(label: String, count: usize, active: bool, on_click: EventHandler<()>) -> Element {
    rsx! {
        button {
            class: if active { "pill h-8 px-3 chip-on" } else { "pill h-8 px-3 bg-hover text-ink hover:bg-line" },
            "aria-pressed": active,
            "data-testid": "report-filter",
            onclick: move |_| on_click.call(()),
            "{label}"
            span { class: "opacity-70 tabular", "{count}" }
        }
    }
}

/// The office's inbox tabs.
#[derive(Clone, Copy, PartialEq)]
enum Inbox {
    Review,
    Hold,
    Details,
    Owner,
    Done,
}

const INBOXES: [Inbox; 5] = [Inbox::Review, Inbox::Hold, Inbox::Details, Inbox::Owner, Inbox::Done];

impl Inbox {
    fn label(self) -> &'static str {
        match self {
            Inbox::Review => "To review",
            Inbox::Hold => "On hold",
            Inbox::Details => "Waiting for details",
            Inbox::Owner => "With the owner",
            Inbox::Done => "Done",
        }
    }
    fn holds(self, s: DecisionStatus) -> bool {
        match self {
            Inbox::Review => s == DecisionStatus::DraftInAdminQueue,
            Inbox::Hold => s == DecisionStatus::OnHold,
            Inbox::Details => s == DecisionStatus::NeedsDetails,
            Inbox::Owner => s == DecisionStatus::PublishedPendingClient,
            Inbox::Done => s.is_resolved() || matches!(s, DecisionStatus::OfficeApproved | DecisionStatus::OfficeDeclined),
        }
    }
    fn empty(self) -> &'static str {
        match self {
            Inbox::Review => "All caught up. New reports from the site will appear here.",
            Inbox::Hold => "Nothing is on hold.",
            Inbox::Details => "No reports are waiting for more details.",
            Inbox::Owner => "Nothing is waiting for the owner.",
            Inbox::Done => "Nothing finished yet.",
        }
    }
}

/// Reports in a tab: the most urgent first and, among those, the one waiting longest.
fn inbox_list(all: &[DecisionAlert], tab: Inbox) -> Vec<DecisionAlert> {
    let mut v: Vec<DecisionAlert> = all.iter().filter(|d| tab.holds(d.status)).cloned().collect();
    let rank = |u: UrgencyLevel| match u {
        UrgencyLevel::Urgent => 0,
        UrgencyLevel::High => 1,
        UrgencyLevel::Standard => 2,
    };
    if matches!(tab, Inbox::Review | Inbox::Hold | Inbox::Details) {
        v.sort_by_key(|d| (rank(d.urgency), d.created_at));
    } else {
        v.sort_by_key(|d| std::cmp::Reverse(d.resolved_at.unwrap_or(d.created_at)));
    }
    v
}

/// Office Admin: a two-pane inbox. Quick decisions on every row; the selected
/// report opens beside the list, and the next one opens after each decision.
#[component]
fn OfficeInbox() -> Element {
    let app = use_app();
    let all = app.decisions.read().clone().unwrap_or_default();
    let mut tab = use_signal(|| Inbox::Review);
    let mut selected = use_signal(|| None::<Uuid>);
    // Phones show the details in a dialog, only after a tap.
    let mut opened = use_signal(|| None::<Uuid>);
    let mut new_report = use_signal(|| false);
    let items = inbox_list(&all, tab());
    let current = selected().and_then(|id| items.iter().find(|d| d.id == id).cloned()).or_else(|| items.first().cloned());
    let phone = opened().and_then(|id| all.iter().find(|d| d.id == id).cloned());

    // After a decision, open the next report in the same tab.
    let advance = use_callback(move |id: Uuid| {
        let list = inbox_list(&app.decisions.peek().clone().unwrap_or_default(), *tab.peek());
        let next = list.iter().position(|d| d.id == id).and_then(|i| list.get(i + 1).or_else(|| i.checked_sub(1).and_then(|j| list.get(j)))).map(|d| d.id);
        selected.set(next);
        if opened.peek().is_some() {
            opened.set(next);
        }
    });
    let pick = move |id: Uuid| {
        selected.set(Some(id));
        // Below the two-pane layout (Tailwind `lg`, 1024 px) the details open in a dialog.
        let narrow = util::window().inner_width().ok().and_then(|w| w.as_f64()).is_some_and(|w| w < 1024.0);
        if narrow {
            opened.set(Some(id));
        }
    };

    rsx! {
        div { class: "space-y-5 rise-in",
            header { class: "flex flex-wrap items-end justify-between gap-3",
                div { class: "space-y-1",
                    h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Issues" }
                    p { class: "text-[15px] text-muted", "Reports from the site team. Approve, decline or hold them here, or send them to the owner." }
                }
                button { class: "btn-secondary", "data-testid": "office-new-report", onclick: move |_| new_report.set(true), Icon { name: "plus" } "New report" }
            }

            nav { class: "flex gap-1 overflow-x-auto no-scrollbar border-b border-line", role: "tablist", "aria-label": "Report lists",
                for t in INBOXES {
                    {
                        let n = all.iter().filter(|d| t.holds(d.status)).count();
                        let on = tab() == t;
                        rsx! {
                            button {
                                key: "{t.label()}",
                                role: "tab",
                                "aria-selected": on,
                                "data-testid": "inbox-tab",
                                class: if on { "flex items-center gap-2 px-3 h-10 -mb-px border-b-2 border-accent text-ink font-medium whitespace-nowrap" } else { "flex items-center gap-2 px-3 h-10 -mb-px border-b-2 border-transparent text-muted hover:text-ink whitespace-nowrap" },
                                onclick: move |_| { tab.set(t); selected.set(None); },
                                "{t.label()}"
                                if n > 0 {
                                    span { class: if t == Inbox::Review { "pill chip-on" } else { "pill bg-hover text-muted" }, "{n}" }
                                }
                            }
                        }
                    }
                }
            }

            if items.is_empty() {
                EmptyState { icon: "check", title: tab().label(), body: tab().empty() }
            } else {
                div { class: "grid lg:grid-cols-[minmax(0,1fr)_minmax(0,1.1fr)] gap-5 items-start",
                    ul { class: "card divide-y divide-line overflow-hidden", "data-testid": "issue-list",
                        for d in items.iter().cloned() {
                            IssueRow { key: "{d.id}", active: current.as_ref().map(|c| c.id) == Some(d.id), d, on_select: pick, on_done: advance }
                        }
                    }
                    div { class: "hidden lg:block lg:sticky lg:top-6",
                        if let Some(d) = current.clone() {
                            IssueDetail { key: "{d.id}", d, on_done: advance }
                        }
                    }
                }
            }
        }
        if let Some(d) = phone {
            div { class: "lg:hidden",
                Modal { title: "Report", on_close: move |_| opened.set(None),
                    IssueDetail { key: "{d.id}", d, on_done: advance }
                }
            }
        }
        if new_report() {
            Modal { title: "New report", on_close: move |_| new_report.set(false),
                FieldReportForm { on_done: move |_| new_report.set(false) }
            }
        }
    }
}

/// One report in the office inbox, with one-tap decisions.
#[component]
fn IssueRow(d: DecisionAlert, active: bool, on_select: EventHandler<Uuid>, on_done: Callback<Uuid>) -> Element {
    let app = use_app();
    let id = d.id;
    let reporter = d.reported_by.and_then(|r| app.users.read().iter().find(|u| u.id == r).map(|u| u.name.clone()));
    let job = app.work_item(d.work_item_id).map(|w| w.title).unwrap_or_default();
    let category = app.settings.read().category_label(d.category);
    rsx! {
        li { class: if active { "bg-accent-soft/50" } else { "hover:bg-hover/40 transition-colors" }, "data-testid": "issue-row",
            div { class: "flex gap-3 p-3 cursor-pointer", role: "button", "aria-pressed": active, onclick: move |_| on_select.call(id),
                match d.photo_url.clone() {
                    Some(src) => rsx! { img { src: "{src}", alt: "", class: "w-20 aspect-video object-cover rounded-md border border-line shrink-0" } },
                    None => rsx! { div { class: "w-20 aspect-video rounded-md bg-hover text-muted flex items-center justify-center shrink-0", Icon { name: "flag" } } },
                }
                div { class: "min-w-0 flex-1",
                    div { class: "flex items-center gap-2",
                        p { class: "font-medium truncate flex-1", "{d.title}" }
                        if d.urgency != UrgencyLevel::Standard {
                            Pill { tone: if d.urgency == UrgencyLevel::Urgent { Tone::Red } else { Tone::Amber }, text: d.urgency.label() }
                        }
                        if !d.status.is_open() {
                            Pill { tone: d.status.tone(), text: d.status.label() }
                        }
                    }
                    div { class: "flex items-center gap-2 mt-1 min-w-0", "data-testid": "issue-reporter",
                        match reporter.clone() {
                            Some(r) => rsx! {
                                span { class: "inline-flex items-center gap-1.5 text-xs font-medium text-ink shrink-0",
                                    Avatar { name: r.clone(), size: 18 }
                                    "{r}"
                                }
                            },
                            None => rsx! { span { class: "text-xs text-muted shrink-0", "Reporter not recorded" } },
                        }
                        span { class: "text-xs text-muted truncate", "· {job} · {category} · {util::relative(d.created_at)}" }
                    }
                    if let Some(n) = d.office_note.clone() {
                        p { class: "text-xs mt-1 text-tag-blue-text truncate", "Office: {n}" }
                    }
                }
            }
            QuickActions { d: d.clone(), on_done, on_open: move |_| on_select.call(id) }
        }
    }
}

/// Approve / Decline / Hold, plus "more". Decline and "ask for details" take a
/// short note right here, without leaving the list.
#[component]
fn QuickActions(d: DecisionAlert, #[props(default)] large: bool, on_done: Callback<Uuid>, #[props(default)] on_open: Option<EventHandler<()>>) -> Element {
    let app = use_app();
    let id = d.id;
    let status = d.status;
    let mut asking = use_signal(|| None::<ReviewAction>);
    let mut note = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut more = use_signal(|| false);
    let size = if large { "h-10 px-4" } else { "h-8 px-2.5 text-xs" };

    let act = move |action: ReviewAction, text: Option<String>| {
        spawn(async move {
            busy.set(true);
            let req = ReviewDecisionRequest { action, note: text };
            let (label, done) = match action {
                ReviewAction::Approve => ("Approve report", "Approved"),
                ReviewAction::Decline => ("Decline report", "Marked as not approved"),
                ReviewAction::Hold => ("Hold report", "Put on hold"),
                ReviewAction::AskForDetails => ("Ask for details", "Asked the reporter for more details"),
                ReviewAction::Reopen => ("Move back to review", "Moved back to review"),
            };
            let outcome: Outcome<DecisionAlert> = sync::mutate(app, label, "POST", format!("/decisions/{id}/review"), &req).await;
            match outcome {
                Outcome::Saved(_) => {
                    app.success(done);
                    asking.set(None);
                    more.set(false);
                    note.set(String::new());
                    on_done.call(id);
                    sync::reload(app, vec![DataScope::Decisions]);
                }
                Outcome::Queued => {}
                Outcome::Failed(e) => app.error(e),
            }
            busy.set(false);
        });
    };

    rsx! {
        div { class: if large { "space-y-3" } else { "px-3 pb-3 sm:pl-[6.5rem] space-y-2" },
            div { class: "flex flex-wrap items-center gap-1.5",
                if status.is_open() {
                    button { class: "btn-green {size}", "data-testid": "quick-approve", disabled: busy(), onclick: move |_| act(ReviewAction::Approve, None),
                        Icon { name: "check" } "Approve" }
                    button { class: "btn-red {size}", "data-testid": "quick-decline", disabled: busy(), onclick: move |_| { asking.set(Some(ReviewAction::Decline)); more.set(false); },
                        Icon { name: "x" } "Decline" }
                    if status != DecisionStatus::OnHold {
                        button { class: "btn-amber {size}", "data-testid": "quick-hold", disabled: busy(), onclick: move |_| act(ReviewAction::Hold, None),
                            Icon { name: "pause" } "Hold" }
                    }
                    div { class: "relative",
                        button { class: "btn-ghost {size}", "aria-label": "More actions", "aria-expanded": more(), "data-testid": "quick-more", onclick: move |_| more.toggle(),
                            Icon { name: "more" } if large { "More" } }
                        if more() {
                            div { class: "absolute z-20 mt-1 left-0 w-56 card shadow-pop p-1 scale-in", role: "menu",
                                if status != DecisionStatus::NeedsDetails {
                                    button { class: "w-full text-left nav-link", role: "menuitem", "data-testid": "quick-ask", onclick: move |_| { asking.set(Some(ReviewAction::AskForDetails)); more.set(false); },
                                        Icon { name: "message" } "Ask for more details" }
                                }
                                if let Some(open) = on_open {
                                    button { class: "w-full text-left nav-link", role: "menuitem", onclick: move |_| { more.set(false); open.call(()); },
                                        Icon { name: "send" } "Send to the owner…" }
                                }
                                if status != DecisionStatus::DraftInAdminQueue {
                                    button { class: "w-full text-left nav-link", role: "menuitem", onclick: move |_| act(ReviewAction::Reopen, None),
                                        Icon { name: "undo" } "Move back to review" }
                                }
                            }
                        }
                    }
                } else if matches!(status, DecisionStatus::OfficeApproved | DecisionStatus::OfficeDeclined) {
                    button { class: "btn-secondary {size}", "data-testid": "quick-reopen", disabled: busy(), onclick: move |_| act(ReviewAction::Reopen, None),
                        Icon { name: "undo" } "Move back to review" }
                }
            }
            if let Some(action) = asking() {
                form { class: "flex flex-wrap gap-2 fade-in",
                    onsubmit: move |e: FormEvent| {
                        e.prevent_default();
                        let text = note().trim().to_string();
                        if action == ReviewAction::AskForDetails && text.is_empty() { return; }
                        act(action, (!text.is_empty()).then_some(text));
                    },
                    input { class: "field h-9 flex-1 min-w-[12rem]", autofocus: true, "data-testid": "quick-note",
                        placeholder: if action == ReviewAction::Decline { "Why not? (optional, shown to the reporter)" } else { "What do you need from them?" },
                        value: "{note}", oninput: move |e| note.set(e.value()) }
                    button { r#type: "button", class: "btn-ghost h-9", onclick: move |_| { asking.set(None); note.set(String::new()); }, "Cancel" }
                    button { r#type: "submit", class: if action == ReviewAction::Decline { "btn-red h-9" } else { "btn-primary h-9" }, "data-testid": "quick-confirm",
                        disabled: busy() || (action == ReviewAction::AskForDetails && note().trim().is_empty()),
                        if action == ReviewAction::Decline { "Decline" } else { "Send request" }
                    }
                }
            }
        }
    }
}

/// The selected report: quick decisions on top, then the full report to edit or send to the owner.
#[component]
fn IssueDetail(d: DecisionAlert, on_done: Callback<Uuid>) -> Element {
    let app = use_app();
    let reporter = d.reported_by.and_then(|r| app.users.read().iter().find(|u| u.id == r).map(|u| u.name.clone()));
    if d.status.is_open() {
        return rsx! {
            div { class: "space-y-3", "data-testid": "issue-detail",
                div { class: "card p-4 space-y-2",
                    div { class: "flex items-center gap-2",
                        p { class: "eyebrow flex-1", "Quick decision" }
                        if let Some(r) = reporter { span { class: "text-xs text-muted", "Reported by {r}" } }
                    }
                    if let Some(n) = d.office_note.clone() {
                        p { class: "text-sm rounded-md bg-tag-blue-bg text-tag-blue-text px-3 py-2", "You asked: {n}" }
                    }
                    QuickActions { d: d.clone(), large: true, on_done }
                }
                PublishEditor { key: "{d.id}", draft: d }
            }
        };
    }
    let job = app.work_item(d.work_item_id).map(|w| w.title).unwrap_or_default();
    rsx! {
        article { class: "card overflow-hidden", "data-testid": "issue-detail",
            if let Some(src) = d.photo_url.clone() {
                a { href: "{src}", target: "_blank", rel: "noopener", img { src: "{src}", alt: "Photo for {d.title}", class: "w-full aspect-video object-cover" } }
            }
            div { class: "p-5 space-y-3",
                div { class: "flex flex-wrap items-center gap-2",
                    Pill { tone: d.status.tone(), text: d.status.label() }
                    span { class: "text-xs text-muted", "{job} · {util::relative(d.created_at)}" }
                    span { class: "text-xs text-muted ml-auto", if let Some(r) = reporter.clone() { "Reported by {r}" } else { "Reporter not recorded" } }
                }
                h2 { class: "text-lg font-semibold", "{d.title}" }
                p { class: "text-sm leading-relaxed whitespace-pre-line", "{d.issue_description}" }
                if !d.mitigation_action_taken.is_empty() {
                    p { class: "text-sm text-muted", "Already done: {d.mitigation_action_taken}" }
                }
                if let Some(n) = d.office_note.clone() {
                    p { class: "text-sm rounded-md bg-tag-red-bg text-tag-red-text px-3 py-2", "Reason: {n}" }
                }
                match d.status {
                    DecisionStatus::ApprovedOptionA => rsx! { p { class: "text-sm text-tag-green-text", "The owner chose: {d.option_a_label.clone().unwrap_or_default()}" } },
                    DecisionStatus::ApprovedOptionB => rsx! { p { class: "text-sm text-tag-green-text", "The owner chose: {d.option_b_label.clone().unwrap_or_default()}" } },
                    _ => rsx! {},
                }
                QuickActions { d: d.clone(), large: true, on_done }
            }
        }
    }
}

/// Site Monitor field input. With `editing`, the same form changes an existing
/// report that is still waiting for office review.
#[component]
fn FieldReportForm(#[props(default)] editing: Option<DecisionAlert>, #[props(default)] on_done: Option<EventHandler<()>>) -> Element {
    let app = use_app();
    let current_job = editing.as_ref().map(|d| d.work_item_id);
    let items: Vec<WorkItem> = app
        .work_items
        .read()
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|w| (w.status.is_active() && app.can_work_in(w.area_id)) || Some(w.id) == current_job)
        .collect();
    let mut project = use_signal(|| current_job);
    let mut category = use_signal(|| editing.as_ref().map(|d| d.category).unwrap_or(AlertCategory::MaterialDelay));
    let mut urgency = use_signal(|| editing.as_ref().map(|d| d.urgency).unwrap_or(UrgencyLevel::Standard));
    let mut title = use_signal(|| editing.as_ref().map(|d| d.title.clone()).unwrap_or_default());
    let mut issue = use_signal(|| editing.as_ref().map(|d| d.issue_description.clone()).unwrap_or_default());
    let mut mitigation = use_signal(|| editing.as_ref().map(|d| d.mitigation_action_taken.clone()).unwrap_or_default());
    // Holds the current photo's address, or a new photo's data once one is taken.
    let photo = use_signal(|| editing.as_ref().and_then(|d| d.photo_url.clone()));
    let had_photo = editing.as_ref().is_some_and(|d| d.photo_url.is_some());
    let edit_id = editing.as_ref().map(|d| d.id);
    let editing_status = editing.as_ref().map(|d| d.status);
    let form_id = if edit_id.is_some() { "edit-report-photo" } else { "report-photo" };
    let mut busy = use_signal(|| false);
    let flat = app.flat_areas();
    let can_submit = project().is_some() && !title().trim().is_empty() && !issue().trim().is_empty();

    rsx! {
        form {
            class: if edit_id.is_some() { "space-y-5" } else { "card p-5 space-y-5" },
            "data-testid": if edit_id.is_some() { "edit-report" } else { "field-report" },
            onsubmit: move |e| e.prevent_default(),
            div {
                label { class: "label", r#for: "report-project", "Which job is affected?" }
                select { id: "report-project", class: "field", "data-testid": "report-project", onchange: move |e| project.set(Uuid::parse_str(&e.value()).ok()),
                    option { value: "", selected: project().is_none(), "Choose a job…" }
                    for w in items.iter() {
                        option { key: "{w.id}", value: "{w.id}", selected: project() == Some(w.id), "{w.title} — {util::area_name(&flat, w.area_id)}" }
                    }
                }
            }
            div {
                p { class: "label", "What kind of issue?" }
                div { class: "flex flex-wrap gap-2",
                    for c in AlertCategory::ALL {
                        button {
                            key: "{c:?}",
                            r#type: "button",
                            class: if category() == c { "pill h-8 px-3 chip-on" } else { "pill h-8 px-3 bg-hover text-ink hover:bg-line" },
                            onclick: move |_| category.set(c),
                            "{app.settings.read().category_label(c)}"
                        }
                    }
                }
            }
            div {
                p { class: "label", "How urgent?" }
                div { class: "grid grid-cols-3 gap-2",
                    for u in UrgencyLevel::ALL {
                        ChoiceButton { key: "{u:?}", active: urgency() == u, label: u.label(), hint: "", onclick: move |_| urgency.set(u) }
                    }
                }
            }
            PhotoCapture { id: form_id, photo, label: "Snap a photo of the issue" }
            div {
                label { class: "label", r#for: "report-title", "Short title" }
                input { id: "report-title", class: "field", "data-testid": "report-title", placeholder: "e.g. Imported paving stones delayed by 2 weeks", value: "{title}", oninput: move |e| title.set(e.value()) }
            }
            div {
                label { class: "label", r#for: "report-issue", "What happened?" }
                textarea { id: "report-issue", class: "field", rows: "3", "data-testid": "report-issue", value: "{issue}", oninput: move |e| issue.set(e.value()) }
            }
            div {
                label { class: "label", r#for: "report-mitigation", "What have you already done about it?" }
                textarea { id: "report-mitigation", class: "field", rows: "2", placeholder: "Optional", value: "{mitigation}", oninput: move |e| mitigation.set(e.value()) }
            }
            div { class: "flex justify-end",
                button {
                    r#type: "button",
                    class: "btn-primary btn-lg",
                    disabled: !can_submit || busy(),
                    "data-testid": "submit-report",
                    onclick: move |_| async move {
                        let Some(work_item_id) = project() else { return };
                        busy.set(true);
                        if let Some(id) = edit_id {
                            let new_photo = photo().filter(|p| p.starts_with("data:"));
                            let req = UpdateDecisionRequest {
                                work_item_id,
                                category: category(),
                                urgency: urgency(),
                                title: title(),
                                issue_description: issue(),
                                mitigation_action_taken: mitigation(),
                                remove_photo: had_photo && photo().is_none(),
                                photo_data_url: new_photo,
                            };
                            let outcome: Outcome<DecisionAlert> = sync::mutate(app, format!("Report changes: {}", title()), "PUT", format!("/decisions/{id}"), &req).await;
                            match outcome {
                                Outcome::Saved(_) => {
                                    app.success(if editing_status == Some(DecisionStatus::NeedsDetails) { "Sent back to the office" } else { "Report updated" });
                                    sync::reload(app, vec![DataScope::Decisions, DataScope::Photos]);
                                    if let Some(done) = on_done { done.call(()); }
                                }
                                Outcome::Queued => if let Some(done) = on_done { done.call(()); },
                                Outcome::Failed(e) => app.error(e),
                            }
                            busy.set(false);
                            return;
                        }
                        let req = CreateDecisionRequest {
                            work_item_id,
                            category: category(),
                            urgency: urgency(),
                            title: title(),
                            issue_description: issue(),
                            mitigation_action_taken: mitigation(),
                            photo_data_url: photo(),
                        };
                        let outcome: Outcome<DecisionAlert> = sync::mutate(app, format!("Issue report: {}", title()), "POST", "/decisions", &req).await;
                        match outcome {
                            Outcome::Saved(_) => {
                                app.success("Sent to the office for review");
                                sync::reload(app, vec![DataScope::Decisions, DataScope::Photos]);
                                if let Some(done) = on_done { done.call(()); }
                                title.set(String::new());
                                issue.set(String::new());
                                mitigation.set(String::new());
                                let mut p = photo;
                                p.set(None);
                            }
                            Outcome::Queued => {
                                title.set(String::new());
                                issue.set(String::new());
                                mitigation.set(String::new());
                                let mut p = photo;
                                p.set(None);
                            }
                            Outcome::Failed(e) => app.error(e),
                        }
                        busy.set(false);
                    },
                    if busy() { Spinner {} }
                    if edit_id.is_some() {
                        "Save changes"
                    } else {
                        Icon { name: "upload" }
                        "Send to the office"
                    }
                }
            }
        }
    }
}

/// Office Admin polish & publish.
#[component]
fn PublishEditor(draft: DecisionAlert) -> Element {
    let app = use_app();
    let id = draft.id;
    let mut title = use_signal(|| draft.title.clone());
    let mut summary = use_signal(|| draft.issue_description.clone());
    let mut mitigation = use_signal(|| draft.mitigation_action_taken.clone());
    let mut category = use_signal(|| draft.category);
    let mut urgency = use_signal(|| draft.urgency);
    let mut needs_choice = use_signal(|| draft.category != AlertCategory::RoutineUpdate);
    let mut option_a = use_signal(String::new);
    let mut option_b = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let photo = use_signal(|| draft.photo_url.clone());
    let had_photo = draft.photo_url.is_some();
    let mut confirm_delete = use_signal(|| false);
    let work_item_id = draft.work_item_id;
    let project = app.work_item(draft.work_item_id).map(|w| w.title).unwrap_or_default();
    let place = util::area_name(&app.flat_areas(), draft.area_id);
    let options_ok = !needs_choice() || (!option_a().trim().is_empty() && !option_b().trim().is_empty());
    let can_publish = options_ok && !title().trim().is_empty() && !summary().trim().is_empty();

    rsx! {
        article { class: "card overflow-hidden", "data-testid": "publish-editor",
            div { class: "px-5 py-3 bg-sidebar border-b border-line flex flex-wrap items-center gap-2 text-sm",
                Pill { tone: Tone::Gray, text: "Field draft" }
                span { class: "font-medium text-ink", "data-testid": "draft-original-title", "{draft.title}" }
                span { class: "text-muted", "{project} · {place} · {util::relative(draft.created_at)}" }
                Link { to: Route::Project { id: draft.work_item_id }, class: "ml-auto text-muted hover:text-ink", "Open project" }
            }
            div { class: "p-5 space-y-4",
                div { class: "max-w-sm",
                    p { class: "label", "Photo from the field" }
                    PhotoCapture { id: format!("draft-photo-{id}"), photo, label: "No photo yet — add one" }
                }
                div {
                    label { class: "label", "Title the owner will see" }
                    input { class: "field", "data-testid": "publish-title", value: "{title}", oninput: move |e| title.set(e.value()) }
                }
                div {
                    label { class: "label", "In short" }
                    textarea { class: "field", rows: "3", "data-testid": "publish-summary", value: "{summary}", oninput: move |e| summary.set(e.value()) }
                }
                div {
                    label { class: "label", "What the team has already done" }
                    textarea { class: "field", rows: "2", value: "{mitigation}", oninput: move |e| mitigation.set(e.value()) }
                }
                div { class: "grid sm:grid-cols-2 gap-4",
                    div {
                        label { class: "label", "Type" }
                        select { class: "field", onchange: move |e| {
                                if let Some(c) = AlertCategory::ALL.iter().find(|c| format!("{c:?}") == e.value()) {
                                    category.set(*c);
                                    if *c == AlertCategory::RoutineUpdate { needs_choice.set(false); }
                                }
                            },
                            for c in AlertCategory::ALL {
                                option { key: "{c:?}", value: "{c:?}", selected: category() == c, "{app.settings.read().category_label(c)}" }
                            }
                        }
                    }
                    div {
                        label { class: "label", "Urgency" }
                        select { class: "field", onchange: move |e| {
                                if let Some(u) = UrgencyLevel::ALL.iter().find(|u| format!("{u:?}") == e.value()) { urgency.set(*u); }
                            },
                            for u in UrgencyLevel::ALL {
                                option { key: "{u:?}", value: "{u:?}", selected: urgency() == u, "{u.label()}" }
                            }
                        }
                    }
                }
                div {
                    p { class: "label", "What should the owner do?" }
                    div { class: "grid sm:grid-cols-2 gap-2",
                        ChoiceButton { active: needs_choice(), label: "Choose between two options", hint: "Shows [Approve Option A] / [Approve Option B]", onclick: move |_| needs_choice.set(true) }
                        ChoiceButton { active: !needs_choice(), label: "Just keep them informed", hint: "Shows [Got it]", onclick: move |_| needs_choice.set(false) }
                    }
                }
                if needs_choice() {
                    div { class: "grid sm:grid-cols-2 gap-4 rise-in",
                        div {
                            label { class: "label", "Option A" }
                            input { class: "field", "data-testid": "option-a", placeholder: "e.g. Approve Local Paving Supplier - No Delay", value: "{option_a}", oninput: move |e| option_a.set(e.value()) }
                        }
                        div {
                            label { class: "label", "Option B" }
                            input { class: "field", "data-testid": "option-b", placeholder: "e.g. Wait for Imported Stones - +14 Days Delay", value: "{option_b}", oninput: move |e| option_b.set(e.value()) }
                        }
                    }
                }
                if confirm_delete() {
                    div { class: "rounded-md bg-tag-red-bg text-tag-red-text text-sm px-3 py-2 flex flex-wrap items-center gap-2", role: "alert",
                        span { class: "flex-1", "Delete this report? This can't be undone." }
                        button { class: "btn-ghost h-8", onclick: move |_| confirm_delete.set(false), "Keep it" }
                        button { class: "btn-red h-8", "data-testid": "confirm-delete-report", disabled: busy(),
                            onclick: move |_| async move {
                                busy.set(true);
                                let outcome: Outcome<serde_json::Value> = sync::mutate(app, "Delete report", "DELETE", format!("/decisions/{id}"), &()).await;
                                match outcome {
                                    Outcome::Saved(_) => {
                                        app.success("Report deleted");
                                        sync::reload(app, vec![DataScope::Decisions]);
                                    }
                                    Outcome::Queued => {}
                                    Outcome::Failed(e) => app.error(e),
                                }
                                busy.set(false);
                            },
                            "Delete"
                        }
                    }
                }
                div { class: "flex flex-wrap justify-end gap-2 pt-1",
                    button { class: "btn-ghost h-12 px-4 mr-auto text-tag-red-text", "data-testid": "delete-report", disabled: busy(),
                        onclick: move |_| confirm_delete.set(true), Icon { name: "trash" } "Delete" }
                    button {
                        class: "btn-secondary btn-lg",
                        "data-testid": "save-draft",
                        disabled: busy() || title().trim().is_empty() || summary().trim().is_empty(),
                        onclick: move |_| async move {
                            busy.set(true);
                            let req = UpdateDecisionRequest {
                                work_item_id,
                                category: category(),
                                urgency: urgency(),
                                title: title(),
                                issue_description: summary(),
                                mitigation_action_taken: mitigation(),
                                remove_photo: had_photo && photo().is_none(),
                                photo_data_url: photo().filter(|p| p.starts_with("data:")),
                            };
                            let outcome: Outcome<DecisionAlert> = sync::mutate(app, "Save report", "PUT", format!("/decisions/{id}"), &req).await;
                            match outcome {
                                Outcome::Saved(_) => {
                                    app.success("Changes saved");
                                    sync::reload(app, vec![DataScope::Decisions, DataScope::Photos]);
                                }
                                Outcome::Queued => {}
                                Outcome::Failed(e) => app.error(e),
                            }
                            busy.set(false);
                        },
                        "Save changes"
                    }
                    button {
                        class: "btn-primary btn-lg",
                        disabled: !can_publish || busy(),
                        "data-testid": "publish-to-vip",
                        onclick: move |_| async move {
                            busy.set(true);
                            let choice = needs_choice();
                            let req = PublishDecisionRequest {
                                title: title(),
                                issue_description: summary(),
                                mitigation_action_taken: mitigation(),
                                category: if choice || category() != AlertCategory::RoutineUpdate { category() } else { AlertCategory::RoutineUpdate },
                                urgency: urgency(),
                                option_a_label: if choice { Some(option_a()) } else { None },
                                option_b_label: if choice { Some(option_b()) } else { None },
                            };
                            let outcome: Outcome<DecisionAlert> = sync::mutate(app, "Send to the owner", "PUT", format!("/decisions/{id}/publish"), &req).await;
                            match outcome {
                                Outcome::Saved(_) => {
                                    app.success("Sent — the owner was notified");
                                    sync::reload(app, vec![DataScope::Decisions, DataScope::Map]);
                                }
                                Outcome::Queued => {}
                                Outcome::Failed(e) => app.error(e),
                            }
                            busy.set(false);
                        },
                        if busy() { Spinner {} }
                        "Send to the owner"
                    }
                }
            }
        }
    }
}

/// The reporter's own reports, each with a clear result from the office.
#[component]
fn DecisionList(items: Vec<DecisionAlert>, #[props(into)] empty: String) -> Element {
    let app = use_app();
    let mut editing = use_signal(|| None::<DecisionAlert>);
    rsx! {
        if items.is_empty() {
            p { class: "text-sm text-muted", "{empty}" }
        } else {
            ul { class: "space-y-2", "data-testid": "my-reports",
                for d in items {
                    {
                        let outcome = Outcome4Reporter::of(d.status);
                        let (badge, edge) = outcome.style();
                        let can_edit = matches!(d.status, DecisionStatus::DraftInAdminQueue | DecisionStatus::NeedsDetails);
                        let owner_answer = match d.status {
                            DecisionStatus::ApprovedOptionA => d.option_a_label.clone().map(|l| format!("The owner chose: {l}")),
                            DecisionStatus::ApprovedOptionB => d.option_b_label.clone().map(|l| format!("The owner chose: {l}")),
                            DecisionStatus::AcknowledgedReceipt => Some("The owner has seen it.".to_string()),
                            DecisionStatus::PublishedPendingClient => Some("Waiting for the owner's answer.".to_string()),
                            _ => None,
                        };
                        rsx! {
                            li { key: "{d.id}", class: "card border-l-4 {edge} p-3 sm:p-4 flex gap-3", "data-testid": "my-report",
                                if let Some(src) = d.photo_url.clone() {
                                    a { href: "{src}", target: "_blank", rel: "noopener", class: "shrink-0", title: "Open the photo",
                                        img { src: "{src}", alt: "Photo for {d.title}", class: "w-20 sm:w-24 aspect-video object-cover rounded-md border border-line", "data-testid": "report-photo-thumb" }
                                    }
                                }
                                div { class: "flex-1 min-w-0 space-y-1.5",
                                    div { class: "flex flex-wrap items-start gap-2",
                                        div { class: "flex-1 min-w-[10rem]",
                                            p { class: "text-[15px] font-medium", "{d.title}" }
                                            p { class: "text-xs text-muted mt-0.5",
                                                "{app.work_item(d.work_item_id).map(|w| w.title).unwrap_or_default()} · {app.settings.read().category_label(d.category)} · sent {util::relative(d.created_at)}"
                                            }
                                        }
                                        span { class: "inline-flex items-center gap-1.5 rounded-md px-2.5 h-7 text-sm font-semibold {badge}", "data-testid": "report-status",
                                            Icon { name: outcome.icon(), size: 14 }
                                            "{outcome.label()}"
                                        }
                                    }
                                    if let Some(n) = d.office_note.clone() {
                                        p { class: if d.status == DecisionStatus::OfficeDeclined { "text-sm rounded-md bg-tag-red-bg text-tag-red-text px-2.5 py-1.5" } else { "text-sm rounded-md bg-tag-blue-bg text-tag-blue-text px-2.5 py-1.5" },
                                            "data-testid": "office-note",
                                            if d.status == DecisionStatus::OfficeDeclined { "Reason: {n}" } else { "The office asks: {n}" }
                                        }
                                    }
                                    if let Some(a) = owner_answer {
                                        p { class: "text-sm text-accent-ink", "{a}" }
                                    }
                                    if can_edit {
                                        button { class: if d.status == DecisionStatus::NeedsDetails { "btn-primary h-8" } else { "btn-secondary h-8" }, "data-testid": "edit-report",
                                            onclick: {
                                                let d = d.clone();
                                                move |_| editing.set(Some(d.clone()))
                                            },
                                            Icon { name: "edit" }
                                            if d.status == DecisionStatus::NeedsDetails { "Add details" } else { "Edit" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(d) = editing() {
            Modal { title: "Edit report", on_close: move |_| editing.set(None),
                FieldReportForm { editing: d, on_done: move |_| editing.set(None) }
            }
        }
    }
}
