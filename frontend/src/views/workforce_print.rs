//! Service 5's two print pages: the ID cards page (print the ticked people's
//! info cards, or a list of them) and the job order print view ("Generate
//! Forms": 4 per folio page, all workers, by project group or by signatory).
//! Every printout is Service 5's own template (frontend/public/print/s5print.js).

use std::collections::HashSet;

use dioxus::prelude::*;
use serde_json::json;
use shared::*;
use uuid::Uuid;

use crate::components::ui::{EmptyState, Icon};
use crate::views::workforce_ops::{s, s5print, use_records, use_workers, S5Preview};

/// Another page (e.g. Assignments) asks the Print tab to open a report, for some people.
pub static PRINT_REQUEST: GlobalSignal<Option<(&'static str, Vec<Uuid>)>> = Signal::global(|| None);

fn tomorrow() -> String {
    (chrono::Local::now().date_naive() + chrono::Duration::days(1)).format("%Y-%m-%d").to_string()
}

#[component]
pub fn PrintTab(
    #[props(default = "id")] initial: &'static str,
    /// Start with one project group (e.g. the encoder's current slip).
    #[props(default)]
    start_group: String,
    #[props(default = "group")] start_by: &'static str,
    /// The job orders' date; Service 5 starts on tomorrow.
    #[props(default)]
    date: String,
) -> Element {
    let mut kind = use_signal(|| initial);
    let mut ids = use_signal(Vec::<Uuid>::new);
    use_hook(move || {
        if let Some((k, list)) = PRINT_REQUEST.write().take() {
            kind.set(k);
            ids.set(list);
        }
    });
    if kind() == "jo" {
        rsx! { JobOrders { ids: ids(), start_group, start_by, date } }
    } else {
        rsx! { IdCards {} }
    }
}

/// Service 5's print view: date, print scope, default signatory, "Print Now".
#[component]
fn JobOrders(ids: Vec<Uuid>, start_group: String, start_by: &'static str, date: String) -> Element {
    let workers = use_workers();
    let sigs = use_records("signatories");
    let from_encoder = !date.is_empty();
    let mut day = use_signal(|| if date.is_empty() { tomorrow() } else { date.clone() });
    // The encoder's buttons: this group or all groups by project group, or all by signatory.
    let mut mode = use_signal(|| match start_by {
        "signatory" => "by-signatory",
        _ if from_encoder => "by-project-group",
        _ => "all",
    });
    let mut sig = use_signal(String::new);
    let mut chosen = use_signal(|| ids.clone());
    let group_ids: Vec<Uuid> = if start_group.is_empty() {
        vec![]
    } else {
        workers.read().clone().unwrap_or_default().into_iter().filter(|w| w.project_group == start_group).map(|w| w.id).collect()
    };
    let picked: Vec<String> = if chosen().is_empty() { group_ids } else { chosen() }.iter().map(Uuid::to_string).collect();
    let names: Vec<String> = sigs.read().clone().unwrap_or_default().iter().map(|v| s(v, "name")).filter(|n| !n.is_empty()).collect();
    let n = chosen().len();
    rsx! {
        div { class: "flex flex-wrap items-end gap-3 no-print", "data-testid": "jo-toolbar",
            div {
                label { class: "label", r#for: "print-date-input", "Date on the job orders" }
                input { id: "print-date-input", r#type: "date", class: "field", title: "Select Output Date", "data-testid": "print-date", value: "{day}", oninput: move |e| day.set(e.value()) }
            }
            div {
                label { class: "label", r#for: "print-batch-mode", "Print scope" }
                select { id: "print-batch-mode", class: "field", title: "Print Scope", "data-testid": "print-by",
                    onchange: move |e| mode.set(match e.value().as_str() { "by-project-group" => "by-project-group", "by-signatory" => "by-signatory", _ => "all" }),
                    option { value: "all", selected: mode() == "all", "🗂️ All Workers" }
                    option { value: "by-project-group", selected: mode() == "by-project-group", "👥 By Project Group (Batch)" }
                    option { value: "by-signatory", selected: mode() == "by-signatory", "✍️ By Signatory (Batch)" }
                }
            }
            div {
                label { class: "label", r#for: "print-global-sig-select", "Default signatory" }
                select { id: "print-global-sig-select", class: "field", title: "Select Default Signatory", "data-testid": "print-sig", onchange: move |e| sig.set(e.value()),
                    for nm in names { option { key: "{nm}", value: "{nm}", selected: sig() == nm, "{nm}" } }
                }
            }
            if n > 0 {
                span { class: "text-sm text-muted h-10 inline-flex items-center gap-2", "data-testid": "jo-selected",
                    "{n} selected"
                    button { class: "btn-ghost h-8", onclick: move |_| chosen.set(vec![]), "Print everyone instead" }
                }
            }
        }
        S5Preview { report: "jobOrders", args: json!([{ "ids": picked, "date": day(), "mode": mode(), "sig": sig() }]) }
    }
}

/// Service 5's cards page: tick people, then "Print Selected" (info cards)
/// or "Generate List" (a named list of them).
#[component]
fn IdCards() -> Element {
    let workers = use_workers();
    let mut group = use_signal(String::new);
    let mut trade = use_signal(String::new);
    let mut size = use_signal(|| "md");
    let mut sort = use_signal(|| "az");
    let mut picked = use_signal(HashSet::<Uuid>::new);
    let mut only_picked = use_signal(|| false);
    let mut list_name = use_signal(String::new);
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    let mut groups: Vec<String> = people.iter().map(|w| w.project_group.clone()).filter(|g| !g.is_empty()).collect();
    groups.sort();
    groups.dedup();
    let mut trades: Vec<String> = people.iter().map(|w| w.trade.clone()).filter(|t| !t.is_empty()).collect();
    trades.sort();
    trades.dedup();
    let mut chosen: Vec<Worker> = people
        .into_iter()
        .filter(|w| (group().is_empty() || w.project_group == group()) && (trade().is_empty() || w.trade == trade()))
        .filter(|w| !only_picked() || picked().contains(&w.id))
        .collect();
    match sort() {
        "za" => chosen.sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase())),
        "trade" => chosen.sort_by(|a, b| (a.trade.to_lowercase(), a.name.to_lowercase()).cmp(&(b.trade.to_lowercase(), b.name.to_lowercase()))),
        "status" => chosen.sort_by_key(|w| (w.status.label(), w.name.to_lowercase())),
        _ => chosen.sort_by_key(|w| w.name.to_lowercase()),
    }
    let card_class = match size() {
        "sm" => "grid grid-cols-3 sm:grid-cols-4 gap-3",
        "lg" => "grid grid-cols-1 sm:grid-cols-2 gap-4",
        _ => "grid grid-cols-2 sm:grid-cols-3 gap-4",
    };
    let n = picked().len();
    let ids = move || picked().iter().map(Uuid::to_string).collect::<Vec<_>>();
    rsx! {
        div { class: "flex flex-wrap items-end gap-3 no-print",
            div {
                label { class: "label", r#for: "print-group", "Project group" }
                select { id: "print-group", class: "field", "data-testid": "print-group", onchange: move |e| group.set(e.value()),
                    option { value: "", "Everyone" }
                    for g in groups { option { key: "{g}", value: "{g}", "{g}" } }
                }
            }
            div {
                label { class: "label", r#for: "print-trade", "Trade" }
                select { id: "print-trade", class: "field", "data-testid": "print-trade", onchange: move |e| trade.set(e.value()),
                    option { value: "", "All trades" }
                    for t in trades { option { key: "{t}", value: "{t}", "{t}" } }
                }
            }
            div {
                label { class: "label", r#for: "print-size", "Card size" }
                select { id: "print-size", class: "field", "data-testid": "print-size", onchange: move |e| size.set(match e.value().as_str() { "sm" => "sm", "lg" => "lg", _ => "md" }),
                    option { value: "sm", selected: size() == "sm", "Small" }
                    option { value: "md", selected: size() == "md", "Medium" }
                    option { value: "lg", selected: size() == "lg", "Large" }
                }
            }
            div {
                label { class: "label", r#for: "print-sort", "Sort" }
                select { id: "print-sort", class: "field", "data-testid": "print-sort", onchange: move |e| sort.set(match e.value().as_str() { "za" => "za", "trade" => "trade", "status" => "status", _ => "az" }),
                    option { value: "az", "A to Z" }
                    option { value: "za", "Z to A" }
                    option { value: "trade", "By trade" }
                    option { value: "status", "By status" }
                }
            }
        }
        // Service 5's selection tray.
        div { class: "card flex flex-wrap items-center gap-3 p-3 no-print", "data-testid": "cards-tray",
            span { class: "text-sm font-medium", "data-testid": "cards-selected-count", "{n} selected" }
            label { class: "flex items-center gap-2 text-sm",
                input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "print-only-picked", checked: only_picked(), onchange: move |e| only_picked.set(e.checked()) }
                "View Selected Only"
            }
            input { id: "cards-group-name", class: "field w-56", placeholder: "Group name for list...", "data-testid": "cards-group-name", value: "{list_name}", oninput: move |e| list_name.set(e.value()) }
            button { class: "btn-ghost", "data-testid": "cards-clear", onclick: move |_| picked.write().clear(), "Clear" }
            button { class: "btn-primary", "data-testid": "print-now", disabled: n == 0, onclick: move |_| s5print("idCards", json!([ids()])), Icon { name: "archive" } "Print Selected" }
            button { class: "btn-secondary", "data-testid": "print-make-list", disabled: n == 0, onclick: move |_| s5print("cardsList", json!([ids()])), "Generate List" }
        }
        if chosen.is_empty() {
            div { class: "card", EmptyState { icon: "users", title: "No one to show", body: "No people match these filters." } }
        }
        div { class: card_class, "data-testid": "id-cards",
            for w in chosen {
                label { key: "{w.id}", class: "id-card card p-4 flex gap-3 relative cursor-pointer",
                    input { r#type: "checkbox", class: "absolute top-2 right-2 w-4 h-4 accent-[#064e3b]", "aria-label": "Tick {w.name}", "data-testid": "card-pick",
                        checked: picked().contains(&w.id),
                        onchange: { let id = w.id; move |e: FormEvent| { if e.checked() { picked.write().insert(id); } else { picked.write().remove(&id); } } } }
                    if w.has_photo { img { src: "/api/workers/{w.id}/photo", alt: "", class: "w-16 h-20 object-cover rounded" } }
                    div { class: "space-y-1 min-w-0",
                        p { class: "text-lg font-semibold leading-tight", "{w.name}" }
                        p { class: "text-sm tabular", "A# {w.a_number}" }
                        p { class: "text-xs text-muted", "{w.trade} · {w.project_group}" }
                    }
                }
            }
        }
    }
}
