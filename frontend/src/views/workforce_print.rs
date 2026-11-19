//! Service 5's print centre: ID cards, job orders (by group or by signatory,
//! for a chosen date, only for people who can get one), tomorrow's and
//! residential job orders, the list & attendance sheet, the family roster and
//! the semi-annual records update forms.

use std::collections::HashSet;

use dioxus::prelude::*;
use serde_json::Value;
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{EmptyState, Icon};
use crate::util;
use crate::views::workforce_ops::{print_with, s, today, use_records, use_workers, Table};

/// Why someone gets no job order for a date (Service 5's `checkJobOrderEligibility`).
pub fn no_job_order(w: &Worker, tomorrow: bool) -> Option<&'static str> {
    if w.archived {
        Some("archived")
    } else if !w.job_order {
        Some("job orders turned off")
    } else if !tomorrow && w.status != WorkerStatus::Present {
        Some(if w.status == WorkerStatus::OnPass { "on pass" } else { "absent" })
    } else {
        None
    }
}

fn age(birthday: &str) -> String {
    use chrono::Datelike;
    let Ok(b) = chrono::NaiveDate::parse_from_str(birthday.get(..10).unwrap_or(""), "%Y-%m-%d") else { return String::new() };
    let now = chrono::Local::now().date_naive();
    let mut y = now.year() - b.year();
    if (now.month(), now.day()) < (b.month(), b.day()) {
        y -= 1;
    }
    y.to_string()
}

#[component]
pub fn PrintTab() -> Element {
    let workers = use_workers();
    let sigs = use_records("signatories");
    let mut kind = use_signal(|| "id");
    let mut group = use_signal(String::new);
    let mut by = use_signal(|| "group");
    let mut date = use_signal(today);
    let mut default_sig = use_signal(String::new);
    let mut size = use_signal(|| "md");
    let mut sort = use_signal(|| "az");
    let mut picked = use_signal(HashSet::<Uuid>::new);
    let mut only_picked = use_signal(|| false);
    let mut title = use_signal(String::new);
    let profiles = use_resource(move || async move {
        if matches!(kind(), "family" | "forms") { api::get::<Vec<Value>>("/workforce/family-roster").await.unwrap_or_default() } else { vec![] }
    });
    let k = kind();
    let job_kinds = matches!(k, "jo" | "tomorrow" | "residential");
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    let mut groups: Vec<String> = people.iter().map(|w| w.project_group.clone()).filter(|g| !g.is_empty()).collect();
    groups.sort();
    groups.dedup();
    let mut chosen: Vec<Worker> = people.into_iter().filter(|w| group().is_empty() || w.project_group == group()).collect();
    if only_picked() && !picked().is_empty() {
        chosen.retain(|w| picked().contains(&w.id));
    }
    if k == "tomorrow" {
        chosen.retain(|w| !w.tomorrow.trim().is_empty());
    }
    // Job orders only go to people who can get one; the rest are listed, not printed.
    let mut left_out: Vec<(String, &'static str)> = vec![];
    if job_kinds {
        chosen.retain(|w| match no_job_order(w, k == "tomorrow") {
            Some(why) => {
                left_out.push((w.name.clone(), why));
                false
            }
            None => true,
        });
    }
    let signatories = sigs.read().clone().unwrap_or_default();
    let sig_names: Vec<String> = signatories.iter().map(|v| s(v, "name")).filter(|n| !n.is_empty()).collect();
    let fallback = default_sig();
    let signer_for = {
        let signatories = signatories.clone();
        move |g: &str| -> (String, String) {
            signatories
                .iter()
                .find(|v| v["projectGroups"].as_array().is_some_and(|a| a.iter().any(|x| x.as_str() == Some(g))))
                .or_else(|| signatories.iter().find(|v| s(v, "name") == fallback))
                .or_else(|| signatories.first())
                .map(|v| (s(v, "name"), s(v, "signatureUrl")))
                .unwrap_or_default()
        }
    };
    // How the sheets are grouped (Service 5: by project group, trade, housing, or signatory batch).
    let key_of = {
        let signer_for = signer_for.clone();
        move |w: &Worker| -> String {
            let v = match (k, by()) {
                ("residential", _) => w.residence.clone(),
                (_, "trade") => w.trade.clone(),
                (_, "housing") => w.housing.clone(),
                (_, "signatory") => signer_for(&w.project_group).0,
                _ => w.project_group.clone(),
            };
            if v.trim().is_empty() { "—".to_string() } else { v }
        }
    };
    match sort() {
        "za" => chosen.sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase())),
        "trade" => chosen.sort_by(|a, b| (a.trade.to_lowercase(), a.name.to_lowercase()).cmp(&(b.trade.to_lowercase(), b.name.to_lowercase()))),
        "status" => chosen.sort_by_key(|w| (w.status.label(), w.name.to_lowercase())),
        _ => chosen.sort_by_key(|w| w.name.to_lowercase()),
    }
    if k != "id" {
        chosen.sort_by(|a, b| key_of(a).to_lowercase().cmp(&key_of(b).to_lowercase()).then(a.project_group.cmp(&b.project_group)).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    }
    let mut sections: Vec<(String, Vec<Worker>)> = vec![];
    for w in chosen.iter().cloned() {
        let key = key_of(&w);
        match sections.last_mut() {
            Some((k2, list)) if *k2 == key => list.push(w),
            _ => sections.push((key, vec![w])),
        }
    }
    let day = chrono::NaiveDate::parse_from_str(&date(), "%Y-%m-%d").ok();
    let date_label = match (k, day) {
        ("tomorrow", _) => util::long_day(chrono::Utc::now() + chrono::Duration::days(1)),
        (_, Some(d)) => d.format("%A, %B %-d, %Y").to_string(),
        _ => util::long_day(chrono::Utc::now()),
    };
    let paper = match k {
        "id" | "family" | "forms" | "list" => "size: A4; margin: 10mm",
        _ => "size: 8.5in 13in; margin: 0.4in",
    };
    let card_class = match size() {
        "sm" => "grid grid-cols-3 sm:grid-cols-4 gap-3 print-grid-ids",
        "lg" => "grid grid-cols-1 sm:grid-cols-2 gap-4 print-grid-ids",
        _ => "grid grid-cols-2 sm:grid-cols-3 gap-4 print-grid-ids",
    };
    let profile_list: Vec<Value> = profiles.read().clone().unwrap_or_default().into_iter().filter(|p| group().is_empty() || s(p, "group") == group()).collect();
    let n_picked = picked().len();
    rsx! {
        div { class: "flex flex-wrap items-end gap-3 no-print",
            div {
                label { class: "label", r#for: "print-kind", "Print" }
                select { id: "print-kind", class: "field", "data-testid": "print-kind",
                    onchange: move |e| kind.set(match e.value().as_str() {
                        "jo" => "jo", "tomorrow" => "tomorrow", "residential" => "residential", "family" => "family", "list" => "list", "forms" => "forms", _ => "id",
                    }),
                    option { value: "id", selected: k == "id", "ID cards" }
                    option { value: "jo", selected: k == "jo", "Job orders (4 per folio page)" }
                    option { value: "tomorrow", selected: k == "tomorrow", "Tomorrow's job orders" }
                    option { value: "residential", selected: k == "residential", "Residential job orders" }
                    option { value: "list", selected: k == "list", "List & attendance sheet" }
                    option { value: "family", selected: k == "family", "Volunteer & family roster" }
                    option { value: "forms", selected: k == "forms", "Semi-annual records update forms" }
                }
            }
            div {
                label { class: "label", r#for: "print-group", "Project group" }
                select { id: "print-group", class: "field", "data-testid": "print-group", onchange: move |e| group.set(e.value()),
                    option { value: "", "Everyone" }
                    for g in groups { option { key: "{g}", value: "{g}", "{g}" } }
                }
            }
            if !matches!(k, "id" | "residential" | "forms" | "family") {
                div {
                    label { class: "label", r#for: "print-by", "Grouped by" }
                    select { id: "print-by", class: "field", "data-testid": "print-by",
                        onchange: move |e| by.set(match e.value().as_str() { "trade" => "trade", "housing" => "housing", "signatory" => "signatory", _ => "group" }),
                        option { value: "group", selected: by() == "group", "Project group" }
                        option { value: "trade", selected: by() == "trade", "Trade" }
                        option { value: "housing", selected: by() == "housing", "Housing" }
                        if job_kinds { option { value: "signatory", selected: by() == "signatory", "Signatory (batch)" } }
                    }
                }
            }
            if matches!(k, "jo" | "residential" | "list") {
                div {
                    label { class: "label", r#for: "print-date", "Date on the sheets" }
                    input { id: "print-date", r#type: "date", class: "field", "data-testid": "print-date", value: "{date}", oninput: move |e| date.set(e.value()) }
                }
            }
            if job_kinds && !sig_names.is_empty() {
                div {
                    label { class: "label", r#for: "print-sig", "Default signatory" }
                    select { id: "print-sig", class: "field", "data-testid": "print-sig", onchange: move |e| default_sig.set(e.value()),
                        option { value: "", "First on the list" }
                        for n in sig_names { option { key: "{n}", value: "{n}", "{n}" } }
                    }
                }
            }
            if k == "id" {
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
            if matches!(k, "id" | "list") && n_picked > 0 {
                label { class: "flex items-center gap-2 text-sm h-10",
                    input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "print-only-picked", checked: only_picked(), onchange: move |e| only_picked.set(e.checked()) }
                    "Only the {n_picked} ticked"
                }
            }
            if k == "list" {
                div {
                    label { class: "label", r#for: "print-title", "List title" }
                    input { id: "print-title", class: "field", "data-testid": "print-title", placeholder: "e.g. Concrete pour team", value: "{title}", oninput: move |e| title.set(e.value()) }
                }
            }
            button { class: "btn-primary", "data-testid": "print-now", onclick: move |_| print_with(paper), Icon { name: "archive" } "Print" }
        }
        if !left_out.is_empty() {
            p { class: "text-sm text-muted no-print", "data-testid": "print-left-out",
                "Not printed ({left_out.len()}): "
                {left_out.iter().map(|(n, why)| format!("{n} ({why})")).collect::<Vec<_>>().join(", ")}
            }
        }
        match k {
            "id" => rsx! {
                if n_picked > 0 {
                    div { class: "flex items-center gap-2 no-print",
                        span { class: "text-sm", "{n_picked} ticked" }
                        button { class: "btn-secondary", "data-testid": "print-make-list", onclick: move |_| { only_picked.set(true); kind.set("list"); }, "Make a list of the ticked people" }
                        button { class: "btn-ghost", onclick: move |_| picked.write().clear(), "Clear" }
                    }
                }
                div { class: card_class, "data-testid": "id-cards",
                    for w in chosen {
                        div { key: "{w.id}", class: "id-card card p-4 flex gap-3 break-inside-avoid relative",
                            input { r#type: "checkbox", class: "absolute top-2 right-2 w-4 h-4 accent-[#064e3b] no-print", "aria-label": "Tick {w.name}", "data-testid": "card-pick",
                                checked: picked().contains(&w.id),
                                onchange: { let id = w.id; move |e: FormEvent| { if e.checked() { picked.write().insert(id); } else { picked.write().remove(&id); } } } }
                            if w.has_photo { img { src: "/api/workers/{w.id}/photo", alt: "", class: "w-16 h-20 object-cover rounded" } }
                            div { class: "space-y-1 min-w-0",
                                p { class: "text-[10px] uppercase tracking-wider text-muted", "SiteMonitor · Workforce ID" }
                                p { class: "text-lg font-semibold leading-tight", "{w.name}" }
                                p { class: "text-sm tabular", "A# {w.a_number}" }
                                p { class: "text-xs text-muted", "{w.trade} · {w.project_group}" }
                            }
                        }
                    }
                }
            },
            "family" => rsx! {
                p { class: "font-semibold print-only", "Volunteer & family roster — {date_label}" }
                Table { head: vec!["Name", "A#", "Group", "Spouse", "Children", "Father", "Mother", "Emergency contact"],
                    rows: profile_list.iter().map(|v| (s(v, "id"), vec![
                        s(v, "name"), s(v, "aNumber"), s(v, "group"), s(v, "spouseName"),
                        v["children"].as_array().map(|c| c.iter().map(|x| s(x, "name")).filter(|n| !n.is_empty()).collect::<Vec<_>>().join(", ")).unwrap_or_default(),
                        s(v, "fatherName"), s(v, "motherName"),
                        format!("{} {}", s(v, "emergencyContact"), s(v, "emergencyPhone")).trim().to_string(),
                    ])).collect() }
            },
            "forms" => rsx! {
                p { class: "text-sm text-muted no-print", "One form per person: their details as on record, with space to correct them." }
                if profile_list.is_empty() {
                    div { class: "card", EmptyState { icon: "archive", title: "Nothing to print", body: "No people match." } }
                }
                div { class: "space-y-6", "data-testid": "update-forms",
                    for p in profile_list.iter().cloned() { UpdateForm { key: "{s(&p, \"id\")}", p } }
                }
            },
            "list" => rsx! {
                if sections.is_empty() {
                    div { class: "card", EmptyState { icon: "archive", title: "Nothing to print", body: "No people match." } }
                }
                div { class: "print-sheet space-y-4", "data-testid": "print-list",
                    p { class: "font-semibold", {if title().trim().is_empty() { "List & attendance".to_string() } else { title() }} " — {date_label}" }
                    for (key, list) in sections {
                        section { key: "{key}", class: "break-inside-avoid",
                            p { class: "font-medium text-sm mb-1", "{key} · {util::plural(list.len(), \"person\")}" }
                            table { class: "w-full text-xs border-collapse",
                                thead {
                                    tr { class: "text-left border-b",
                                        th { class: "p-1 w-6", "#" }
                                        th { class: "p-1", "Name" }
                                        th { class: "p-1", "A#" }
                                        th { class: "p-1", "Trade" }
                                        th { class: "p-1", "Residence" }
                                        for d in 1..=5 { th { key: "{d}", class: "p-1 w-16 text-center", "Day {d}" } }
                                    }
                                }
                                tbody {
                                    for (i, w) in list.iter().enumerate() {
                                        tr { key: "{w.id}", class: "border-b", "data-testid": "list-row",
                                            td { class: "p-1", "{i + 1}" }
                                            td { class: "p-1", "{w.name}" }
                                            td { class: "p-1", "{w.a_number}" }
                                            td { class: "p-1", "{w.trade}" }
                                            td { class: "p-1", "{w.residence}" }
                                            for d in 1..=5 { td { key: "{d}", class: "p-1 border-l" } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "grid grid-cols-2 gap-10 pt-8 text-xs",
                        div { class: "border-t pt-1", "Prepared by" }
                        div { class: "border-t pt-1", "Noted by" }
                    }
                }
            },
            _ => rsx! {
                if sections.is_empty() {
                    div { class: "card", EmptyState { icon: "archive", title: "Nothing to print", body: if k == "tomorrow" { "Set tomorrow's assignments on the Tomorrow tab first." } else { "No people match." } } }
                }
                for (key, list) in sections {
                    {
                        let (signer, _) = signer_for(&list[0].project_group);
                        rsx! {
                            section { key: "{key}", class: "space-y-2",
                                p { class: "font-semibold no-print", "{key}" span { class: "text-muted font-normal ml-2", "signed by {signer}" } }
                                div { class: "grid sm:grid-cols-2 gap-4 job-orders", "data-testid": "job-orders",
                                    for w in list {
                                        {
                                            let (signer, signature) = signer_for(&w.project_group);
                                            rsx! {
                                                div { key: "{w.id}", class: "job-order card p-4 space-y-2 break-inside-avoid",
                                                    p { class: "font-semibold text-center", "JOB ORDER" }
                                                    p { class: "text-sm", "Name: " b { "{w.name}" } " · A# {w.a_number}" }
                                                    p { class: "text-sm", "Trade: {w.trade}" }
                                                    p { class: "text-sm", "Project group: {w.project_group}" }
                                                    if k == "residential" { p { class: "text-sm", "Residence: {w.residence}" } } else { p { class: "text-sm", "Area: {w.area}" } }
                                                    if k == "tomorrow" && !w.tomorrow.is_empty() { p { class: "text-sm", "Task: {w.tomorrow}" } }
                                                    if k != "tomorrow" && !w.task.is_empty() { p { class: "text-sm", "Task: {w.task}" } }
                                                    p { class: "text-sm", "Date: {date_label}" }
                                                    div { class: "pt-2 border-t border-line text-center",
                                                        if signature.starts_with("data:image/") { img { src: "{signature}", alt: "Signature", class: "h-10 mx-auto object-contain" } }
                                                        p { class: "text-sm", "{signer}" }
                                                        p { class: "text-xs text-muted", "Signatory" }
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
            },
        }
    }
}

/// One person's semi-annual records update form (Service 5 SS-VRU): the record
/// as it stands, with a blank beside each line for corrections.
#[component]
fn UpdateForm(p: Value) -> Element {
    let row = |label: &'static str, value: String| {
        rsx! {
            tr { class: "border-b align-top",
                td { class: "p-1 w-44 text-muted", "{label}" }
                td { class: "p-1 w-1/3 font-medium", "{value}" }
                td { class: "p-1 text-muted", "→ ________________________________" }
            }
        }
    };
    let list = |k: &str| {
        p[k].as_array()
            .map(|a| a.iter().map(|x| if x.is_string() { x.as_str().unwrap_or("").to_string() } else { s(x, "name") }).filter(|x| !x.is_empty()).collect::<Vec<_>>().join(", "))
            .unwrap_or_else(|| s(&p, k))
    };
    let children: Vec<Value> = p["children"].as_array().cloned().unwrap_or_default();
    let bday = s(&p, "birthday");
    rsx! {
        article { class: "card p-5 text-xs space-y-3 break-after-page", "data-testid": "update-form",
            div { class: "text-center space-y-0.5",
                p { class: "text-[10px] uppercase tracking-wider text-muted", "Service section · Volunteer workforce administration" }
                p { class: "font-semibold text-sm", "Semi-annual volunteer records update form" }
                p { class: "text-muted", "Date issued: {util::long_day(chrono::Utc::now())}" }
            }
            p { class: "text-muted", "Please check each line. Write the correct detail in the blank where something has changed, then sign below." }
            p { class: "font-semibold", "1. Personal & contact profile" }
            table { class: "w-full border-collapse",
                tbody {
                    {row("A#", s(&p, "aNumber"))}
                    {row("Name", s(&p, "name"))}
                    {row("Civil status", s(&p, "civilStatus"))}
                    {row("Birthday / age", if bday.is_empty() { String::new() } else { format!("{bday} / {}", age(&bday)) })}
                    {row("Mobile phone", s(&p, "phone"))}
                    {row("Language / dialect", s(&p, "language"))}
                    {row("Work residence", s(&p, "workResidence"))}
                    {row("Permanent home address", s(&p, "address"))}
                    {row("Housing unit / room", s(&p, "assignedUnit"))}
                }
            }
            p { class: "font-semibold", "2. Trade, skills & work assignment" }
            table { class: "w-full border-collapse",
                tbody {
                    {row("Primary trade", s(&p, "trade"))}
                    {row("Skill level", s(&p, "skillLevel"))}
                    {row("Project group / role", format!("{} {}", s(&p, "group"), s(&p, "groupRole")).trim().to_string())}
                    {row("Other skills", list("trades"))}
                }
            }
            p { class: "font-semibold", "3. Health & emergency" }
            table { class: "w-full border-collapse",
                tbody {
                    {row("Blood type", s(&p, "bloodType"))}
                    {row("Medical conditions / allergies", s(&p, "medical"))}
                    {row("Maintenance medicines", list("maintenanceMeds"))}
                    {row("Emergency contact", s(&p, "emergencyContact"))}
                    {row("Relationship / phone", format!("{} {}", s(&p, "emergencyRelationship"), s(&p, "emergencyPhone")).trim().to_string())}
                }
            }
            p { class: "font-semibold", "4. Spouse & family" }
            table { class: "w-full border-collapse",
                tbody {
                    {row("Spouse", s(&p, "spouseName"))}
                    {row("Spouse's birthday", s(&p, "spouseBirthday"))}
                    {row("Spouse's phone", s(&p, "spouseContact"))}
                    {row("Wedding date", s(&p, "weddingDate"))}
                    {row("Father", s(&p, "fatherName"))}
                    {row("Mother", s(&p, "motherName"))}
                }
            }
            table { class: "w-full border-collapse",
                thead {
                    tr { class: "text-left border-b",
                        th { class: "p-1", "Child" }
                        th { class: "p-1", "Birthday" }
                        th { class: "p-1", "Duty" }
                        th { class: "p-1", "School / work" }
                        th { class: "p-1", "Lives here" }
                        th { class: "p-1", "Correction" }
                    }
                }
                tbody {
                    for (i, c) in children.iter().enumerate() {
                        tr { key: "{i}", class: "border-b",
                            td { class: "p-1", {s(c, "name")} }
                            td { class: "p-1", {s(c, "birthday")} }
                            td { class: "p-1", {s(c, "churchDuty")} }
                            td { class: "p-1", {if c["isStudent"].as_bool() == Some(true) { s(c, "school") } else { s(c, "workplace") }} }
                            td { class: "p-1", {if c["livesHere"].as_bool() == Some(true) { "Yes" } else { "No" }} }
                            td { class: "p-1 text-muted", "______________" }
                        }
                    }
                    for i in 0..2 {
                        tr { key: "new{i}", class: "border-b",
                            td { class: "p-1 text-muted", colspan: "6", "New child: ______________________  Birthday: ____________  School / work: ______________" }
                        }
                    }
                }
            }
            div { class: "grid grid-cols-2 gap-10 pt-6",
                div { class: "border-t pt-1", "Signature over printed name" }
                div { class: "border-t pt-1", "Date" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(status: WorkerStatus, job_order: bool) -> Worker {
        serde_json::from_value(serde_json::json!({
            "id": Uuid::nil(), "a_number": "1", "name": "A", "phone": "", "trade": "", "department": "", "area": "", "project_group": "",
            "group_role": "", "status": status, "archived": false, "updated_at": chrono::Utc::now(), "job_order": job_order,
        }))
        .unwrap()
    }

    #[test]
    fn job_orders_skip_people_off_today_or_turned_off() {
        assert_eq!(no_job_order(&w(WorkerStatus::Present, true), false), None);
        assert_eq!(no_job_order(&w(WorkerStatus::OnPass, true), false), Some("on pass"));
        assert_eq!(no_job_order(&w(WorkerStatus::Absent, true), false), Some("absent"));
        assert_eq!(no_job_order(&w(WorkerStatus::Present, false), false), Some("job orders turned off"));
        // Tomorrow's job orders don't depend on today's attendance.
        assert_eq!(no_job_order(&w(WorkerStatus::Absent, true), true), None);
    }
}
