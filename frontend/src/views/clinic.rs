//! Clinic ledger (merged from Service 5 DB): health episodes, clinic visits,
//! vaccines, contact tracing and the audit trail. Office Admin only — the
//! server checks "Clinic ledger" on every read and logs every change.

use chrono::{Duration, NaiveDate};
use dioxus::prelude::*;
use serde_json::{json, Value};

use crate::components::ui::Icon;
use crate::views::workforce_ops::{by_a_number, choice, f, name_of, new_id, print_with, s, today, use_records, use_workers, Field, Kind, RecordModal, Table, Tabs};

#[component]
pub fn ClinicView() -> Element {
    let tab = use_signal(|| "Health episodes");
    rsx! {
        div { class: "space-y-5 rise-in",
            header { class: "space-y-1 no-print",
                h1 { class: "text-2xl sm:text-3xl font-semibold tracking-tight", "Clinic ledger" }
                p { class: "text-[15px] text-muted", "Medical records are private: only people with \u{201c}Clinic ledger\u{201d} access see this page, and every change is logged." }
            }
            div { class: "no-print", Tabs { tabs: vec!["Health episodes", "Clinic visits", "Vaccines", "Contact tracing", "Audit trail"], current: tab } }
            match tab() {
                "Health episodes" => rsx! { Episodes {} },
                "Clinic visits" => rsx! { Visits {} },
                "Vaccines" => rsx! { Simple { list: "volunteer_vaccines", title: "Vaccine", head: vec!["Person", "Vaccine", "Given", "Lot", "Booster due"],
                    keys: vec!["volunteerId", "vaccineName", "dateAdministered", "lotNumber", "boosterDueDate"],
                    fields: vec![f("volunteerId", "Person", Kind::Worker), f("vaccineName", "Vaccine", Kind::Text), f("dateAdministered", "Given on", Kind::Date),
                                 f("lotNumber", "Lot number", Kind::Text), f("administeredBy", "Given by", Kind::Text), f("boosterDueDate", "Booster due", Kind::Date), f("notes", "Notes", Kind::Long)] } },
                "Contact tracing" => rsx! { Simple { list: "contact_traces", title: "Contact", head: vec!["Contact", "Episode", "How", "Status", "Traced"],
                    keys: vec!["contactWorkerId", "healthEventId", "contactType", "status", "tracedAt"],
                    fields: vec![f("contactWorkerId", "Person in contact", Kind::Worker), f("healthEventId", "Episode id", Kind::Text),
                                 f("contactType", "How", choice(&["household", "workplace", "roommate", "other"])), f("status", "Status", choice(&["monitoring", "cleared", "converted"])), f("tracedAt", "Traced on", Kind::Date)] } },
                _ => rsx! { Audit {} },
            }
        }
    }
}

/// Days after day 0 when Service 5 re-checks an episode.
const CHECKS: [i64; 3] = [8, 10, 14];

#[component]
fn Episodes() -> Element {
    let events = use_records("health_events");
    let workers = use_workers();
    let mut editing = use_signal(|| None::<(String, Value)>);
    let map = by_a_number(&workers.read().clone().unwrap_or_default());
    let mut list: Vec<Value> = events.read().clone().unwrap_or_default().into_iter().filter(|e| s(e, "status") != "deleted").collect();
    list.sort_by(|a, b| s(b, "dayZero").cmp(&s(a, "dayZero")));
    let open = list.iter().filter(|e| s(e, "status") != "cleared").count();
    let rows: Vec<(String, Vec<String>)> = list
        .iter()
        .map(|e| {
            let next = NaiveDate::parse_from_str(&s(e, "dayZero"), "%Y-%m-%d")
                .ok()
                .filter(|_| s(e, "status") != "cleared")
                .and_then(|d0| CHECKS.iter().map(|n| d0 + Duration::days(*n)).find(|d| d.format("%Y-%m-%d").to_string() >= today()))
                .map(|d| format!("Day check {}", d.format("%b %-d")))
                .unwrap_or_else(|| s(e, "clearanceDate"));
            (s(e, "id"), vec![name_of(&map, &s(e, "workerId")), s(e, "dayZero"), s(e, "diagnosis"), s(e, "status"), next])
        })
        .collect();
    let fields: Vec<Field> = vec![
        f("workerId", "Person", Kind::Worker),
        f("dayZero", "Day 0 (first symptoms or test)", Kind::Date),
        f("diagnosis", "Diagnosis", Kind::Text),
        f("status", "Status", choice(&["active", "monitoring", "cleared"])),
        f("clearanceDate", "Cleared on", Kind::Date),
        f("notes", "Notes", Kind::Long),
    ];
    rsx! {
        div { class: "flex flex-wrap items-center justify-between gap-3",
            p { class: "text-sm text-muted", "data-testid": "episodes-open", "{open} open · {list.len()} in all. Checks on day 8, 10 and 14." }
            button { class: "btn-primary", "data-testid": "episode-add", onclick: move |_| editing.set(Some((new_id("he"), json!({ "dayZero": today(), "status": "active", "createdAt": chrono::Utc::now().to_rfc3339() })))), Icon { name: "plus" } "New episode" }
        }
        Table { head: vec!["Person", "Day 0", "Diagnosis", "Status", "Next"], rows, on_edit: move |id: String| {
            if let Some(v) = list.iter().find(|e| s(e, "id") == id) { editing.set(Some((id, v.clone()))) }
        } }
        if let Some((id, v)) = editing() {
            RecordModal { title: "Health episode".to_string(), list: "health_events", id, fields, initial: v, on_close: move |_| editing.set(None) }
        }
    }
}

#[component]
fn Visits() -> Element {
    let logs = use_records("volunteer_health_logs");
    let workers = use_workers();
    let mut editing = use_signal(|| None::<(String, Value)>);
    let map = by_a_number(&workers.read().clone().unwrap_or_default());
    let mut list: Vec<Value> = logs.read().clone().unwrap_or_default().into_iter().filter(|v| v["deleted"].as_bool() != Some(true)).collect();
    list.sort_by(|a, b| s(b, "logDateTime").cmp(&s(a, "logDateTime")));
    let rows: Vec<(String, Vec<String>)> = list
        .iter()
        .map(|v| {
            let who = if s(v, "volunteerName").is_empty() { name_of(&map, &s(v, "volunteerId")) } else { s(v, "volunteerName") };
            let vitals = [("BP", "bloodPressure"), ("T", "temperature"), ("HR", "heartRate"), ("O₂", "o2Saturation")]
                .iter()
                .filter(|(_, k)| !s(v, k).is_empty())
                .map(|(l, k)| format!("{l} {}", s(v, k)))
                .collect::<Vec<_>>()
                .join(" · ");
            let id = if s(v, "id").is_empty() { s(v, "logId") } else { s(v, "id") };
            (id, vec![s(v, "logDateTime").chars().take(16).collect(), who, s(v, "chiefComplaint"), s(v, "diagnosis"), vitals])
        })
        .collect();
    let fields: Vec<Field> = vec![
        f("volunteerId", "Person", Kind::Worker),
        f("logDateTime", "Date", Kind::Date),
        f("chiefComplaint", "Complaint", Kind::Text),
        f("diagnosis", "Diagnosis", Kind::Text),
        f("bloodPressure", "Blood pressure", Kind::Text),
        f("temperature", "Temperature (°C)", Kind::Number),
        f("heartRate", "Heart rate", Kind::Number),
        f("o2Saturation", "O₂ saturation (%)", Kind::Number),
        f("medicationsGiven", "Medicine given", Kind::Text),
        f("attendingPhysician", "Seen by", Kind::Text),
        f("returnToWorkDate", "Back to work on", Kind::Date),
        f("notes", "Notes", Kind::Long),
    ];
    rsx! {
        div { class: "flex flex-wrap items-center justify-between gap-3 no-print",
            p { class: "text-sm text-muted", "{list.len()} visits" }
            div { class: "flex gap-2",
                button { class: "btn-secondary", "data-testid": "print-clinical", onclick: move |_| print_with("size: A4; margin: 12mm"), Icon { name: "archive" } "Print clinical report" }
                button { class: "btn-primary", "data-testid": "visit-log-add", onclick: move |_| editing.set(Some((new_id("hl"), json!({ "logDateTime": today(), "createdAt": chrono::Utc::now().to_rfc3339() })))), Icon { name: "plus" } "Log a visit" }
            }
        }
        p { class: "font-semibold print-only", "Clinical report — {today()}" }
        Table { head: vec!["When", "Person", "Complaint", "Diagnosis", "Vital signs"], rows, on_edit: move |id: String| {
            if let Some(v) = list.iter().find(|e| s(e, "id") == id || s(e, "logId") == id) { editing.set(Some((id, v.clone()))) }
        } }
        if let Some((id, v)) = editing() {
            RecordModal { title: "Clinic visit".to_string(), list: "volunteer_health_logs", id, fields, initial: v, on_close: move |_| editing.set(None) }
        }
    }
}

/// A list with an add button and one form (vaccines, contact tracing).
#[component]
fn Simple(list: &'static str, title: &'static str, head: Vec<&'static str>, keys: Vec<&'static str>, fields: Vec<Field>) -> Element {
    let records = use_records(list);
    let workers = use_workers();
    let mut editing = use_signal(|| None::<(String, Value)>);
    let map = by_a_number(&workers.read().clone().unwrap_or_default());
    let all = records.read().clone().unwrap_or_default();
    let rows: Vec<(String, Vec<String>)> = all
        .iter()
        .map(|v| (s(v, "id"), keys.iter().map(|k| if k.ends_with("WorkerId") || *k == "volunteerId" { name_of(&map, &s(v, k)) } else { s(v, k) }).collect()))
        .collect();
    rsx! {
        div { class: "flex justify-end",
            button { class: "btn-primary", "data-testid": "simple-add", onclick: move |_| editing.set(Some((new_id(list), json!({ "createdAt": chrono::Utc::now().to_rfc3339() })))), Icon { name: "plus" } "Add {title.to_lowercase()}" }
        }
        Table { head, rows, on_edit: move |id: String| {
            if let Some(v) = all.iter().find(|e| s(e, "id") == id) { editing.set(Some((id, v.clone()))) }
        } }
        if let Some((id, v)) = editing() {
            RecordModal { title: title.to_string(), list, id, fields, initial: v, on_close: move |_| editing.set(None) }
        }
    }
}

#[component]
fn Audit() -> Element {
    let audit = use_records("medical_audit_logs");
    let mut list = audit.read().clone().unwrap_or_default();
    list.sort_by(|a, b| s(b, "timestamp").cmp(&s(a, "timestamp")));
    let rows: Vec<(String, Vec<String>)> =
        list.iter().map(|v| (s(v, "id"), vec![s(v, "timestamp").chars().take(16).collect(), s(v, "editedBy"), s(v, "action"), s(v, "details"), s(v, "recordId")])).collect();
    rsx! { Table { head: vec!["When", "Who", "Action", "Record type", "Record"], rows } }
}
