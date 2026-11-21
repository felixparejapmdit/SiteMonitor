//! Service 5's Print & Export dialog: a grouped field picker (Personal,
//! Contact, Family, Work & Residence, Project, Medical & Health Clearance),
//! print layouts (family roster cards / table, standard table, CV / biodata,
//! one volunteer per page) and CSV / Excel export of the chosen fields.

use std::collections::HashSet;

use dioxus::prelude::*;
use serde_json::{json, Value};
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{Icon, Modal, Spinner};
use crate::state::use_app;
use crate::views::personnel::{csv_cell, save_file};
use crate::views::profile::{age, excel_xml};
use crate::views::workforce_ops::{s, s5print};

/// (category id, label, fields (id, label)) — Service 5's `CATEGORIES`.
pub const CATEGORIES: &[(&str, &str, &[(&str, &str)])] = &[
    ("personal", "Personal info", &[("name", "Name"), ("aNumber", "A#"), ("photo", "Photo"), ("birthday", "Birthday & age"), ("civilStatus", "Civil status"), ("trade", "Trade")]),
    ("contact", "Contact", &[("phone", "Phone number"), ("address", "Home address"), ("emergencyContact", "Emergency contact name"), ("emergencyPhone", "Emergency phone")]),
    ("family", "Family", &[("spouseName", "Spouse name"), ("spouseBirthday", "Spouse birthday"), ("spouseChurchDuty", "Spouse church duty"), ("weddingDate", "Wedding anniversary"), ("children", "Children")]),
    ("work", "Work & residence", &[("tradeGroup", "Trade group"), ("tradeRole", "Trade role"), ("workResidence", "Work residence"), ("assignedUnit", "Assigned unit")]),
    ("project", "Project", &[("projectGroup", "Project group"), ("projectRole", "Project role"), ("visitationHousing", "Visitation housing")]),
    ("medical", "Medical & health clearance", &[("medical", "Medical condition"), ("bloodType", "Blood type"), ("allergies", "Allergies"), ("clearanceStatus", "Health clearance")]),
];

fn label_of(fid: &str) -> &'static str {
    CATEGORIES.iter().flat_map(|c| c.2.iter()).find(|f| f.0 == fid).map(|f| f.1).unwrap_or("")
}

/// Service 5's starting choice: Print starts with Personal and Contact, Export with everything.
pub fn default_fields(tab: &str, clinic: bool) -> HashSet<&'static str> {
    CATEGORIES
        .iter()
        .filter(|c| clinic || c.0 != "medical")
        .filter(|c| tab == "export" || c.0 == "personal" || c.0 == "contact")
        .flat_map(|c| c.2.iter().map(|f| f.0))
        .chain(std::iter::once("name"))
        .collect()
}

/// One field of a person as text (Service 5 `getFieldValue`).
pub fn field_value(p: &Value, fid: &str) -> String {
    match fid {
        "birthday" => {
            let b = s(p, "birthday");
            match age(&b) {
                Some(a) if !b.is_empty() => format!("{b} ({a} yrs)"),
                _ => b,
            }
        }
        "projectGroup" => if s(p, "projectGroup").is_empty() { s(p, "group") } else { s(p, "projectGroup") },
        "children" => p["children"].as_array().map(|c| c.iter().map(|x| s(x, "name")).filter(|n| !n.is_empty()).collect::<Vec<_>>().join("; ")).unwrap_or_default(),
        "photo" => if p["hasPhoto"].as_bool() == Some(true) { "on file".into() } else { String::new() },
        "allergies" => s(p, "healthClearance.allergies"),
        "clearanceStatus" => [s(p, "healthClearance.status"), s(p, "healthAndClearance.clearanceStatus")].into_iter().find(|x| !x.is_empty()).unwrap_or_default(),
        k => s(p, k),
    }
}

#[component]
pub fn PrintExportModal(people: Vec<Worker>, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let clinic = app.can(Permission::ManageClinic);
    let profiles = use_resource(|| async { api::get::<Vec<Value>>("/workforce/family-roster").await.map_err(|e| e.message()) });
    let mut tab = use_signal(|| "print");
    let mut fields = use_signal(|| default_fields("print", clinic));
    let mut layout = use_signal(|| "roster");
    let mut individual = use_signal(|| false);
    let ids: HashSet<Uuid> = people.iter().map(|w| w.id).collect();
    let has_photo: HashSet<Uuid> = people.iter().filter(|w| w.has_photo).map(|w| w.id).collect();
    let list: Vec<Value> = match &*profiles.read() {
        Some(Ok(all)) => all
            .iter()
            .filter_map(|p| {
                let id = Uuid::parse_str(&s(p, "id")).ok()?;
                ids.contains(&id).then(|| {
                    let mut p = p.clone();
                    p["hasPhoto"] = Value::Bool(has_photo.contains(&id));
                    p
                })
            })
            .collect(),
        _ => vec![],
    };
    let loading = profiles.read().is_none();
    // Chosen fields in Service 5's order.
    let chosen: Vec<&'static str> = CATEGORIES.iter().flat_map(|c| c.2.iter().map(|f| f.0)).filter(|f| fields().contains(f)).collect();
    let table = {
        let list = list.clone();
        let chosen = chosen.clone();
        move || -> (Vec<&'static str>, Vec<Vec<String>>) {
            let head: Vec<&'static str> = chosen.iter().map(|f| label_of(f)).collect();
            (head, list.iter().map(|p| chosen.iter().map(|f| field_value(p, f)).collect()).collect())
        }
    };
    let stamp = chrono::Local::now().format("%Y-%m-%d").to_string();
    rsx! {
        div { class: "contents", "data-testid": "print-export",
            Modal { title: format!("Print & export — {} volunteers selected", people.len()), wide: true, on_close: move |_| on_close.call(()),
                div { class: "space-y-4 no-print",
                    div { class: "flex border-b border-line", role: "tablist",
                        for (k, l) in [("print", "Print options"), ("export", "Export options")] {
                            button { key: "{k}", role: "tab", "aria-selected": tab() == k, "data-testid": "pe-tab-{k}",
                                class: if tab() == k { "px-3 py-2 text-sm font-semibold border-b-2 border-accent" } else { "px-3 py-2 text-sm text-muted border-b-2 border-transparent" },
                                onclick: move |_| { tab.set(k); fields.set(default_fields(k, clinic)); }, "{l}" }
                        }
                    }
                    if tab() == "print" {
                        div { class: "flex flex-wrap gap-4",
                            for (v, l) in [("roster", "Family roster (cards)"), ("family-table", "Family roster (table)"), ("table", "Standard table"), ("profile", "CV / biodata profile")] {
                                label { key: "{v}", class: "flex items-center gap-2 text-sm",
                                    input { r#type: "radio", name: "pe-layout", class: "accent-[#064e3b]", "data-testid": "pe-layout-{v}", checked: layout() == v, onchange: move |_| layout.set(v) }
                                    "{l}"
                                }
                            }
                        }
                        if layout() == "profile" {
                            label { class: "flex items-center gap-2 text-sm",
                                input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "pe-individual", checked: individual(), onchange: move |e| individual.set(e.checked()) }
                                "Print each volunteer on a separate page"
                            }
                        }
                    }
                    div { class: "flex gap-2",
                        button { class: "btn-secondary h-8", "data-testid": "pe-select-all", onclick: move |_| fields.set(CATEGORIES.iter().filter(|c| clinic || c.0 != "medical").flat_map(|c| c.2.iter().map(|f| f.0)).collect()), "✓ Select all fields" }
                        button { class: "btn-secondary h-8", "data-testid": "pe-clear-all", onclick: move |_| fields.set(["name"].into_iter().collect()), "✕ Clear all fields" }
                    }
                    div { class: "grid sm:grid-cols-2 gap-3", "data-testid": "pe-tree",
                        for (cid, clabel, cfields) in CATEGORIES.iter().filter(|c| clinic || c.0 != "medical") {
                            {
                                let on = cfields.iter().filter(|f| fields().contains(f.0)).count();
                                let all_on = on == cfields.len();
                                rsx! {
                                    fieldset { key: "{cid}", class: "card p-3 space-y-1",
                                        label { class: "flex items-center gap-2 text-sm font-semibold",
                                            input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "pe-cat-{cid}", checked: all_on,
                                                onchange: move |e| { let mut f = fields.write(); for (id, _) in cfields.iter() { if e.checked() { f.insert(id); } else if *id != "name" { f.remove(id); } } } }
                                            "{clabel}"
                                            span { class: "text-xs text-muted font-normal ml-auto tabular", "{on}/{cfields.len()}" }
                                        }
                                        for (id, l) in cfields.iter() {
                                            label { key: "{id}", class: "flex items-center gap-2 text-sm pl-6",
                                                input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "pe-field-{id}", checked: fields().contains(id), disabled: *id == "name",
                                                    onchange: move |e| { if e.checked() { fields.write().insert(id); } else { fields.write().remove(id); } } }
                                                "{l}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if loading { Spinner {} }
                    div { class: "modal-actions",
                        button { class: "btn-ghost", onclick: move |_| on_close.call(()), "Cancel" }
                        if tab() == "print" {
                            button { class: "btn-primary", "data-testid": "pe-print", disabled: list.is_empty(),
                                // Service 5's own Print & Export templates (opens its print page).
                                onclick: { let ids: Vec<String> = list.iter().map(|p| s(p, "id")).collect(); let chosen = chosen.clone(); move |_| {
                                    s5print("printExport", json!([{ "ids": ids, "layout": layout(), "fields": chosen, "individual": individual() }]));
                                } },
                                Icon { name: "archive" } "Print" }
                        } else {
                            button { class: "btn-secondary", "data-testid": "pe-csv", disabled: list.is_empty(),
                                onclick: { let table = table.clone(); let stamp = stamp.clone(); move |_| {
                                    let (head, rows) = table();
                                    let mut out = head.join(",") + "\n";
                                    for r in rows { out += &(r.iter().map(|c| csv_cell(c)).collect::<Vec<_>>().join(",") + "\n"); }
                                    save_file(&format!("volunteers-{stamp}.csv"), &out);
                                } }, "Export CSV" }
                            button { class: "btn-primary", "data-testid": "pe-xlsx", disabled: list.is_empty(),
                                onclick: { let table = table.clone(); let stamp = stamp.clone(); move |_| { let (head, rows) = table(); save_file(&format!("volunteers-{stamp}.xls"), &excel_xml("Volunteers", &head, &rows)); } }, "Export Excel" }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_service5() {
        let print = default_fields("print", true);
        assert!(print.contains("phone") && print.contains("name") && !print.contains("spouseName"));
        let export = default_fields("export", false);
        assert!(export.contains("spouseName") && !export.contains("bloodType"));
    }

    #[test]
    fn values_read_nested_and_derived_fields() {
        let p = serde_json::json!({ "group": "Team P", "children": [{ "name": "A" }, { "name": "B" }], "healthClearance": { "allergies": "Dust" } });
        assert_eq!(field_value(&p, "projectGroup"), "Team P");
        assert_eq!(field_value(&p, "children"), "A; B");
        assert_eq!(field_value(&p, "allergies"), "Dust");
    }
}
