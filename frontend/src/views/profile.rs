//! Service 5's personnel profile, field for field: five tabs (Personal, Family,
//! Residence & Work, Trade & Other, Attendance), previous / next / jump between
//! people, re-employment, children and maintenance medication rows, family
//! print / CSV / Excel, visit scheduling, the attendance record and its prints.

use dioxus::prelude::*;
use serde_json::{json, Value};
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{Icon, Modal, Spinner};
use crate::services::sync::{self, Outcome};
use crate::state::{use_app, AppState};
use crate::views::personnel::{csv_cell, save_file, REFRESH as WORKERS_CHANGED};
use crate::views::workforce_ops::{f, new_id, s, s5print, today, use_records, use_workers, Kind, RecordModal};

const TABS: &[(&str, &str)] = &[("personal", "Personal"), ("family", "Family"), ("residence", "Residence & Work"), ("trade", "Trade & Other"), ("attendance", "Attendance")];
const CIVIL: &[&str] = &["", "Single", "Married", "Widowed", "Separated"];
const ROLES: &[&str] = &["Member", "Kalihim", "II Katiwala", "Katiwala"];
const PROJECT_ROLES: &[&str] = &["Member", "Asst. Project Leader", "Project Leader", "Project Overseer"];
const DEPARTMENTS: &[&str] = &["Job-site Construction", "Woodland Shops", "Garden", "Residential Only", "Electrical Rm", "Water System", "Woodland Office"];
const SHOPS: &[&str] = &["Carpenter Shop", "Painter Shop", "Tinsmith Shop", "Steelman/Welder Shop", "Powder Coating Shop"];
const WORK_RESIDENCES: &[&str] = &[
    "Same as home address", "Centro Condo", "Pink Condo", "Quarry Condo", "Sagana Condo", "Green Condo", "Peach Condo", "Tagumpay I", "Tagumpay II",
    "Dasmarinas Cavite", "Non INC Owned Residence", "Woodland", "Others",
];
/// Critical-area access: stored value and label.
const CRITICAL_AREAS: &[(&str, &str)] = &[("CC", "CC Interior"), ("Station 1", "Station 1"), ("Mall", "Mall")];
const ADD_SHOP: &str = "__ADD_NEW_SHOP__";

/// Whole years since a `YYYY-MM-DD` date, if it is one.
pub fn age(birthday: &str) -> Option<i32> {
    let b = chrono::NaiveDate::parse_from_str(birthday.get(..10)?, "%Y-%m-%d").ok()?;
    let now = chrono::NaiveDate::parse_from_str(&today(), "%Y-%m-%d").ok()?;
    use chrono::Datelike;
    let mut years = now.year() - b.year();
    if (now.month(), now.day()) < (b.month(), b.day()) {
        years -= 1;
    }
    (years >= 0).then_some(years)
}

fn is_on(v: &Value) -> bool {
    v.as_bool() == Some(true) || matches!(v.as_str(), Some("true" | "YES" | "Yes" | "Oo" | "Opo"))
}

/// Options for a select, keeping a stored value that isn't one of them.
fn options_with(base: &[&str], extra: &[String], current: &str) -> Vec<String> {
    let mut v: Vec<String> = base.iter().map(|x| x.to_string()).collect();
    for x in extra.iter().chain(std::iter::once(&current.to_string())) {
        if !x.is_empty() && !v.contains(x) {
            v.push(x.clone());
        }
    }
    v
}

/// One field of the profile, bound to a key of the draft.
#[component]
fn PField(draft: Signal<Value>, k: &'static str, label: &'static str, #[props(default)] kind: &'static str, #[props(default)] options: Vec<String>, #[props(default)] wide: bool) -> Element {
    let mut draft = draft;
    let value = s(&draft.read(), k);
    let id = format!("pf-{k}");
    let mut set = move |v: String| {
        draft.write()[k] = if kind == "number" { v.parse::<f64>().map(|n| json!(n)).unwrap_or(json!(v)) } else { json!(v) };
    };
    rsx! {
        div { class: if wide { "sm:col-span-2" } else { "" },
            label { class: "label", r#for: "{id}", "{label}" }
            match kind {
                "long" => rsx! { textarea { id: "{id}", class: "field", rows: "3", "data-testid": "{id}", value: "{value}", oninput: move |e| set(e.value()) } },
                "select" => rsx! {
                    select { id: "{id}", class: "field", "data-testid": "{id}", onchange: move |e| set(e.value()),
                        for o in options { option { key: "{o}", value: "{o}", selected: o == value, if o.is_empty() { "-- Select --" } else { "{o}" } } }
                    }
                },
                "date" | "number" | "tel" => rsx! { input { id: "{id}", class: "field", r#type: kind, step: "any", "data-testid": "{id}", value: "{value}", oninput: move |e| set(e.value()) } },
                _ => rsx! { input { id: "{id}", class: "field", "data-testid": "{id}", value: "{value}", oninput: move |e| set(e.value()) } },
            }
        }
    }
}

#[component]
fn Check(draft: Signal<Value>, k: &'static str, label: &'static str, #[props(default)] inverted: bool) -> Element {
    let mut draft = draft;
    let on = is_on(&draft.read()[k]) != inverted;
    rsx! {
        label { class: "flex items-center gap-2 text-sm cursor-pointer",
            input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "pf-{k}", checked: on,
                onchange: move |e| draft.write()[k] = if k == "jOrder" { json!(if e.checked() { "YES" } else { "NO" }) } else { json!(e.checked() != inverted) } }
            "{label}"
        }
    }
}

/// A repeatable row list (children, maintenance medication) kept as an array of objects.
#[component]
fn Rows(draft: Signal<Value>, k: &'static str, title: &'static str, add: &'static str, cols: Vec<(&'static str, &'static str, &'static str)>) -> Element {
    let mut draft = draft;
    let items: Vec<Value> = draft.read()[k].as_array().cloned().unwrap_or_default();
    rsx! {
        div { class: "space-y-2", "data-testid": "rows-{k}",
            div { class: "flex items-center justify-between",
                p { class: "font-semibold text-sm", "{title}" }
                button { r#type: "button", class: "btn-secondary h-8", "data-testid": "add-{k}",
                    onclick: move |_| {
                        let mut d = draft.write();
                        if !d[k].is_array() { d[k] = json!([]); }
                        if let Some(a) = d[k].as_array_mut() { a.push(json!({})); }
                    },
                    Icon { name: "plus" } "{add}"
                }
            }
            if items.is_empty() { p { class: "text-sm text-muted", "None yet." } }
            for (i, item) in items.into_iter().enumerate() {
                div { key: "{k}-{i}", class: "card p-3 grid sm:grid-cols-3 gap-2 relative",
                    for (key, label, kind) in cols.clone() {
                        {
                            let v = item.get(key).cloned().unwrap_or(Value::Null);
                            let id = format!("{k}-{i}-{key}");
                            rsx! {
                                div { key: "{id}",
                                    if kind == "check" {
                                        label { class: "flex items-center gap-2 text-sm pt-6",
                                            input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "{id}", checked: is_on(&v),
                                                onchange: move |e| draft.write()[k][i][key] = json!(e.checked()) }
                                            "{label}"
                                        }
                                    } else if kind == "age" {
                                        label { class: "label", "{label}" }
                                        input { class: "field", disabled: true, "data-testid": "{id}", value: {age(&s(&draft.read()[k][i], "birthday")).map(|a| a.to_string()).unwrap_or_else(|| s(&draft.read()[k][i], "age"))} }
                                    } else {
                                        label { class: "label", r#for: "{id}", "{label}" }
                                        input { id: "{id}", class: "field", r#type: if kind == "date" { "date" } else { "text" }, "data-testid": "{id}", value: {s(&draft.read()[k][i], key)},
                                            oninput: move |e| {
                                                let val = e.value();
                                                let mut d = draft.write();
                                                d[k][i][key] = json!(val);
                                                // Service 5 keeps a child's age beside the birthday.
                                                if key == "birthday" { if let Some(a) = age(&val) { d[k][i]["age"] = json!(a); } }
                                            } }
                                    }
                                }
                            }
                        }
                    }
                    button { r#type: "button", class: "absolute top-2 right-2 btn-icon btn-ghost h-7 w-7 text-tag-red-text", "aria-label": "Remove", "data-testid": "remove-{k}-{i}",
                        onclick: move |_| { if let Some(a) = draft.write()[k].as_array_mut() { if i < a.len() { a.remove(i); } } },
                        Icon { name: "x" } }
                }
            }
        }
    }
}

async fn save(app: AppState, id: Uuid, draft: &Value) -> Result<(), String> {
    match sync::mutate::<_, Value>(app, "Profile", "PUT", format!("/workers/{id}/profile"), draft).await {
        Outcome::Saved(_) | Outcome::Queued => {
            *WORKERS_CHANGED.write() += 1;
            Ok(())
        }
        Outcome::Failed(e) => Err(e),
    }
}

/// Family rows for print / CSV / Excel: spouse first, then each child.
fn family_rows(p: &Value) -> Vec<Vec<String>> {
    let mut rows = vec![];
    if !s(p, "spouseName").is_empty() {
        rows.push(vec!["Spouse".into(), s(p, "spouseName"), s(p, "spouseBirthday"), age(&s(p, "spouseBirthday")).map(|a| a.to_string()).unwrap_or_default(), String::new(), s(p, "spouseChurchDuty"), String::new()]);
    }
    for c in p["children"].as_array().into_iter().flatten() {
        let doing = if is_on(&c["isStudent"]) || !s(c, "school").is_empty() {
            format!("Studying: {}{}", s(c, "school"), if s(c, "course").is_empty() { String::new() } else { format!(" ({})", s(c, "course")) })
        } else {
            [s(c, "job"), s(c, "workplace")].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" at ")
        };
        let a = age(&s(c, "birthday")).map(|a| a.to_string()).unwrap_or_else(|| s(c, "age"));
        rows.push(vec!["Child".into(), s(c, "name"), s(c, "birthday"), a, doing, s(c, "churchDuty"), if is_on(&c["livesHere"]) { "Yes".into() } else { "No".into() }]);
    }
    rows
}
const FAMILY_HEAD: [&str; 7] = ["Relation", "Name", "Birthday", "Age", "School / work", "Church duty", "Lives here"];

/// A minimal Excel 2003 XML workbook (opens in Excel and LibreOffice).
pub fn excel_xml(sheet: &str, head: &[&str], rows: &[Vec<String>]) -> String {
    let esc = |x: &str| x.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
    let row = |cells: Vec<String>| format!("<Row>{}</Row>", cells.iter().map(|c| format!("<Cell><Data ss:Type=\"String\">{}</Data></Cell>", esc(c))).collect::<String>());
    format!(
        "<?xml version=\"1.0\"?>\n<Workbook xmlns=\"urn:schemas-microsoft-com:office:spreadsheet\" xmlns:ss=\"urn:schemas-microsoft-com:office:spreadsheet\"><Worksheet ss:Name=\"{}\"><Table>{}{}</Table></Worksheet></Workbook>",
        esc(sheet),
        row(head.iter().map(|h| h.to_string()).collect()),
        rows.iter().cloned().map(row).collect::<String>()
    )
}

/// One attendance entry as shown on the Attendance tab.
#[derive(Clone, PartialEq)]
struct Att {
    date: String,
    status: String,
    shift: String,
    details: String,
    verified: String,
    notes: String,
}

fn attendance(p: &Value, log: &[Value]) -> Vec<Att> {
    let mut out: Vec<Att> = p["attendanceHistory"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|h| {
            let date = if s(h, "date").is_empty() { s(h, "timestamp").chars().take(10).collect() } else { s(h, "date") };
            let status = [s(h, "status"), s(h, "newStatus")].into_iter().find(|x| !x.is_empty()).unwrap_or_default();
            let shift = [s(h, "shift"), s(h, "type")].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
            let details = [s(h, "specificTask"), s(h, "project"), s(h, "area")].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
            let notes = [s(h, "reason"), s(h, "notes")].into_iter().find(|x| !x.is_empty()).unwrap_or_default();
            Att { date, status, shift, details, verified: s(h, "verifiedBy"), notes }
        })
        .collect();
    // Service 5 also folds in the session attendance log, without repeating a day's status.
    let a = s(p, "aNumber");
    for l in log.iter().filter(|l| !a.is_empty() && s(l, "aNumber") == a || s(l, "workerName") == s(p, "name")) {
        let date = if s(l, "date").is_empty() { s(l, "timestamp").chars().take(10).collect() } else { s(l, "date") };
        let status = if s(l, "newStatus").is_empty() { "Present".to_string() } else { s(l, "newStatus") };
        if !out.iter().any(|e| e.date == date && e.status == status) {
            out.push(Att { date, status, shift: "Day".into(), details: String::new(), verified: "Encoder".into(), notes: s(l, "reason") });
        }
    }
    out.sort_by(|x, y| y.date.cmp(&x.date));
    out
}

fn kind_of(status: &str) -> &'static str {
    let l = status.to_lowercase();
    if l.contains("pass") || l.contains("excused") || l.contains("medical") {
        "passes"
    } else if l.contains("present") || l == "p" {
        "present"
    } else if l.contains("absent") || l == "a" {
        "absent"
    } else {
        "passes"
    }
}

#[component]
pub fn ProfileModal(worker: Worker, on_close: EventHandler<()>) -> Element {
    let app = use_app();
    let workers = use_workers();
    let lists = use_records("custom_datalists");
    let att_log = use_records("attendance_log");
    let clinic = app.can(Permission::ManageClinic);
    let archived = worker.archived;
    let mut cur = use_signal(|| worker.id);
    let mut tab = use_signal(|| "personal");
    let profile = use_resource(move || async move {
        let id = cur();
        (id, api::get::<Value>(&format!("/workers/{id}/profile")).await.map_err(|e| e.message()))
    });
    let mut draft = use_signal(|| Value::Null);
    let mut base = use_signal(|| Value::Null);
    let mut loaded = use_signal(|| None::<Uuid>);
    let mut busy = use_signal(|| false);
    let mut adding_shop = use_signal(|| false);
    let mut new_shop = use_signal(String::new);
    let mut visit = use_signal(|| None::<Value>);
    let mut reemploying = use_signal(|| false);
    let mut att_filter = use_signal(|| "all");
    let mut att_q = use_signal(String::new);
    use_effect(move || {
        if let Some((id, Ok(p))) = &*profile.read() {
            if *loaded.peek() != Some(*id) {
                draft.set(p.clone());
                base.set(p.clone());
                loaded.set(Some(*id));
            }
        }
    });
    // Previous / next walk the same list (active or archived), by name.
    let order = use_memo(move || {
        let mut p: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| w.archived == archived).collect();
        p.sort_by_key(|w| w.name.to_lowercase());
        p
    });
    let people = order();
    let pos = people.iter().position(|w| w.id == cur());
    let custom = lists.read().clone().unwrap_or_default().into_iter().next().unwrap_or_else(|| json!({}));
    let custom_list = |k: &str| custom[k].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect::<Vec<_>>()).unwrap_or_default();
    let shops = custom_list("woodlandShops");
    let residences = custom_list("workResidences");
    let d = draft();
    let ready = loaded() == Some(cur()) && d.is_object();
    let dirty = ready && d != base();
    let title = if ready { format!("{} — profile", s(&d, "name")) } else { worker.name.clone() };

    // Save, then optionally move to the previous / next person.
    let save_then = move |step: i32| {
        let ppl = order();
        async move {
            let id = cur();
            if loaded() == Some(id) && draft() != base() {
                busy.set(true);
                let result = save(app, id, &draft()).await;
                busy.set(false);
                match result {
                    Ok(_) => {
                        base.set(draft());
                        app.success("Saved");
                    }
                    Err(e) => return app.error(e),
                }
            }
            if step != 0 {
                if let Some(i) = ppl.iter().position(|w| w.id == id) {
                    let j = (i as i32 + step).rem_euclid(ppl.len() as i32) as usize;
                    cur.set(ppl[j].id);
                }
            }
        }
    };

    let body = if !ready {
        match &*profile.read() {
            Some((_, Err(e))) => rsx! { p { class: "text-sm text-tag-red-text", "{e}" } },
            _ => rsx! { div { class: "p-6 flex justify-center", Spinner { size: 20 } } },
        }
    } else {
        let id = cur();
        let photo = s(&d, "photoUrl");
        let a = s(&d, "aNumber");
        let ca: Vec<String> = d["caEligible"].as_array().map(|x| x.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
        let rows = attendance(&d, &att_log.read().clone().unwrap_or_default());
        let (present, passes, absent) = rows.iter().fold((0, 0, 0), |acc, r| match kind_of(&r.status) {
            "present" => (acc.0 + 1, acc.1, acc.2),
            "absent" => (acc.0, acc.1, acc.2 + 1),
            _ => (acc.0, acc.1 + 1, acc.2),
        });
        let reliability = if rows.is_empty() { 100.0 } else { present as f64 * 100.0 / rows.len() as f64 };
        let ql = att_q().to_lowercase();
        let shown: Vec<Att> = rows
            .iter()
            .filter(|r| att_filter() == "all" || kind_of(&r.status) == att_filter())
            .filter(|r| ql.is_empty() || [&r.date, &r.status, &r.shift, &r.details, &r.verified, &r.notes].iter().any(|x| x.to_lowercase().contains(&ql)))
            .cloned()
            .collect();
        let fam = family_rows(&d);
        let name = s(&d, "name");
        rsx! {
            // Tabs
            div { class: "flex border-b border-line -mx-5 px-5 overflow-x-auto no-print", role: "tablist",
                for (i, (key, label)) in TABS.iter().enumerate() {
                    button { key: "{key}", r#type: "button", role: "tab", "aria-selected": tab() == *key, "data-testid": "ptab-{key}",
                        class: if tab() == *key { "px-3 py-2 text-sm font-semibold border-b-2 border-accent whitespace-nowrap" } else { "px-3 py-2 text-sm text-muted border-b-2 border-transparent whitespace-nowrap hover:text-ink" },
                        onclick: move |_| tab.set(key), "{i + 1} {label}" }
                }
            }
            div { class: "pt-4 space-y-4 no-print",
                match tab() {
                    "personal" => rsx! {
                        div { class: "flex items-center gap-4",
                            if photo.starts_with("data:image/") {
                                img { src: "{photo}", alt: "", class: "w-20 h-20 rounded-xl object-cover border border-line" }
                            } else {
                                div { class: "w-20 h-20 rounded-xl bg-hover flex items-center justify-center text-muted", Icon { name: "camera", size: 22 } }
                            }
                            div { class: "flex flex-wrap gap-2",
                                input { id: "profile-photo", r#type: "file", accept: "image/jpeg,image/png,image/webp", class: "hidden",
                                    onchange: move |_| async move {
                                        match crate::services::media_canvas::file_as_data_url("profile-photo", 2.0 * 1_048_576.0).await {
                                            Ok(Some(url)) => draft.write()["photoUrl"] = json!(url),
                                            Ok(None) => {}
                                            Err(e) => app.error(e),
                                        }
                                    } }
                                label { r#for: "profile-photo", class: "btn-secondary cursor-pointer", "data-testid": "profile-photo-upload", Icon { name: "camera" } if photo.is_empty() { "Add photo" } else { "Change photo" } }
                                if !photo.is_empty() { button { r#type: "button", class: "btn-ghost", "data-testid": "profile-photo-remove", onclick: move |_| draft.write()["photoUrl"] = json!(""), "Remove photo" } }
                            }
                        }
                        div { class: "grid sm:grid-cols-2 gap-4",
                            PField { draft, k: "name", label: "Full name", wide: true }
                            PField { draft, k: "aNumber", label: "Assigned No. (A#)" }
                            PField { draft, k: "birthday", label: "Birthday", kind: "date" }
                            div { label { class: "label", "Age" } input { class: "field", disabled: true, "data-testid": "pf-age", value: {age(&s(&d, "birthday")).map(|a| a.to_string()).unwrap_or_default()} } }
                            PField { draft, k: "civilStatus", label: "Civil status", kind: "select", options: options_with(CIVIL, &[], &s(&d, "civilStatus")) }
                            PField { draft, k: "weddingDate", label: "Wedding date", kind: "date" }
                            PField { draft, k: "phone", label: "Phone number", kind: "tel" }
                            PField { draft, k: "language", label: "Language" }
                            PField { draft, k: "address", label: "Home address", kind: "long", wide: true }
                        }
                        if clinic {
                            div { class: "grid sm:grid-cols-2 gap-4",
                                PField { draft, k: "medical", label: "Health & medical history", kind: "long", wide: true }
                                PField { draft, k: "bloodType", label: "Blood type", kind: "select", options: options_with(&["", "A+", "A-", "B+", "B-", "AB+", "AB-", "O+", "O-"], &[], &s(&d, "bloodType")) }
                                PField { draft, k: "medicalNotes", label: "Medical notes", kind: "long", wide: true }
                            }
                            Rows { draft, k: "maintenanceMeds", title: "Maintenance medication", add: "Add medication",
                                cols: vec![("name", "Medication name", "text"), ("dosage", "Dosage", "text"), ("prescribedBy", "Prescribed by", "text"),
                                           ("prescribedUntil", "Prescribed until", "date"), ("nextCheckup", "Next check-up / update", "date"), ("purpose", "Purpose (what is it for)", "text")] }
                        } else {
                            p { class: "text-sm text-muted", "Health & medical history is kept with the clinic ledger." }
                        }
                        p { class: "font-semibold text-sm", "Emergency contact" }
                        div { class: "grid sm:grid-cols-3 gap-4",
                            PField { draft, k: "emergencyContact", label: "Contact name" }
                            PField { draft, k: "emergencyRelationship", label: "Relationship" }
                            PField { draft, k: "emergencyPhone", label: "Phone number", kind: "tel" }
                        }
                    },
                    "family" => rsx! {
                        div { class: "flex flex-wrap gap-2",
                            button { r#type: "button", class: "btn-secondary", "data-testid": "family-print", onclick: move |_| s5print("familyInfo", json!([cur().to_string()])), Icon { name: "archive" } "Print family info" }
                            button { r#type: "button", class: "btn-secondary", "data-testid": "family-csv",
                                onclick: { let fam = fam.clone(); let name = name.clone(); move |_| {
                                    let mut out = FAMILY_HEAD.join(",") + "\n";
                                    for r in &fam { out += &(r.iter().map(|c| csv_cell(c)).collect::<Vec<_>>().join(",") + "\n"); }
                                    save_file(&format!("family-{}.csv", name.replace(' ', "_")), &out);
                                } }, "Export CSV" }
                            button { r#type: "button", class: "btn-secondary", "data-testid": "family-excel",
                                onclick: { let fam = fam.clone(); let name = name.clone(); move |_| save_file(&format!("family-{}.xls", name.replace(' ', "_")), &excel_xml("Family", &FAMILY_HEAD, &fam)) }, "Export Excel" }
                        }
                        div { class: "grid sm:grid-cols-2 gap-4",
                            PField { draft, k: "spouseName", label: "Spouse name" }
                            PField { draft, k: "spouseBirthday", label: "Spouse birthday", kind: "date" }
                            PField { draft, k: "spouseChurchDuty", label: "Spouse church duty" }
                            PField { draft, k: "spouseContact", label: "Spouse phone", kind: "tel" }
                            PField { draft, k: "spouseKasalSaIglesia", label: "Kasal sa Iglesia?", kind: "select", options: options_with(&["", "Oo", "Hindi"], &[], &s(&d, "spouseKasalSaIglesia")) }
                            PField { draft, k: "spouseNagsasama", label: "Nagsasama?", kind: "select", options: options_with(&["", "Oo", "Hindi"], &[], &s(&d, "spouseNagsasama")) }
                            PField { draft, k: "fatherName", label: "Father's name" }
                            PField { draft, k: "motherName", label: "Mother's name" }
                        }
                        Rows { draft, k: "children", title: "Children & dependents", add: "Add child",
                            cols: vec![("name", "Name", "text"), ("birthday", "Birthday", "date"), ("age", "Age", "age"), ("isStudent", "Student?", "check"),
                                       ("school", "School", "text"), ("course", "Course", "text"), ("workplace", "Workplace", "text"), ("job", "Occupation / work", "text"),
                                       ("churchDuty", "Church duty", "text"), ("livesHere", "Lives here?", "check")] }
                    },
                    "residence" => rsx! {
                        div { class: "grid sm:grid-cols-2 gap-4",
                            PField { draft, k: "workResidence", label: "Work residence", kind: "select", options: options_with(&[""], &[WORK_RESIDENCES.iter().map(|x| x.to_string()).collect::<Vec<_>>(), residences.clone()].concat(), &s(&d, "workResidence")) }
                            PField { draft, k: "assignedUnit", label: "Assigned unit" }
                            PField { draft, k: "visitationHousing", label: "Visitation housing group" }
                            PField { draft, k: "visitationGroup", label: "Visitation group" }
                            PField { draft, k: "visitationRole", label: "Visitation role", kind: "select", options: options_with(ROLES, &[], &s(&d, "visitationRole")) }
                            PField { draft, k: "projectGroup", label: "Project group" }
                            PField { draft, k: "projectRole", label: "Project group role", kind: "select", options: options_with(PROJECT_ROLES, &[], &s(&d, "projectRole")) }
                        }
                        div {
                            p { class: "label", "Critical area access" }
                            div { class: "flex flex-wrap gap-4",
                                for (v, l) in CRITICAL_AREAS.iter() {
                                    label { key: "{v}", class: "flex items-center gap-2 text-sm",
                                        input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "pf-ca-{l}", checked: ca.iter().any(|x| x == v),
                                            onchange: move |e| {
                                                let mut list: Vec<String> = draft.read()["caEligible"].as_array().map(|x| x.iter().filter_map(|v| v.as_str().map(str::to_string)).collect()).unwrap_or_default();
                                                list.retain(|x| x != v);
                                                if e.checked() { list.push(v.to_string()); }
                                                draft.write()["caEligible"] = json!(list);
                                            } }
                                        "{l}"
                                    }
                                }
                            }
                        }
                        div { class: "flex flex-wrap gap-2 items-center",
                            p { class: "label w-full", "Pastoral visitation record" }
                            if !s(&d, "lastVisitedDate").is_empty() { span { class: "text-sm text-muted", "Last visit {s(&d, \"lastVisitedDate\")} · {s(&d, \"lastVisitedMinister\")}" } }
                            if !s(&d, "upcomingVisitDate").is_empty() { span { class: "text-sm text-muted", "Next visit {s(&d, \"upcomingVisitDate\")} · {s(&d, \"upcomingVisitMinister\")}" } }
                            if app.can(Permission::ManageVisitations) {
                                button { r#type: "button", class: "btn-secondary", "data-testid": "profile-schedule-visit",
                                    onclick: { let a = a.clone(); let name = name.clone(); move |_| visit.set(Some(json!({ "aNumber": a, "workerName": name, "status": "scheduled", "date": today() }))) },
                                    Icon { name: "calendar" } "Schedule visit" }
                                button { r#type: "button", class: "btn-secondary", "data-testid": "profile-past-visit",
                                    onclick: { let a = a.clone(); let name = name.clone(); move |_| visit.set(Some(json!({ "aNumber": a, "workerName": name, "status": "completed", "date": today() }))) },
                                    Icon { name: "plus" } "Log past visit" }
                            }
                        }
                    },
                    "trade" => rsx! {
                        div { class: "grid sm:grid-cols-2 gap-4",
                            PField { draft, k: "department", label: "Department / category", kind: "select", options: options_with(&[""], &DEPARTMENTS.iter().map(|x| x.to_string()).collect::<Vec<_>>(), &s(&d, "department")) }
                            div {
                                label { class: "label", r#for: "pf-subDepartment", "Shop specialty" }
                                select { id: "pf-subDepartment", class: "field", "data-testid": "pf-subDepartment",
                                    onchange: move |e| { if e.value() == ADD_SHOP { adding_shop.set(true) } else { draft.write()["subDepartment"] = json!(e.value()) } },
                                    for o in options_with(&[""], &[SHOPS.iter().map(|x| x.to_string()).collect::<Vec<_>>(), shops.clone()].concat(), &s(&d, "subDepartment")) {
                                        option { key: "{o}", value: "{o}", selected: o == s(&d, "subDepartment"), if o.is_empty() { "-- None --" } else { "{o}" } }
                                    }
                                    option { value: ADD_SHOP, "＋ Add new shop…" }
                                }
                                if adding_shop() {
                                    div { class: "flex gap-2 mt-2",
                                        input { class: "field", placeholder: "Shop name", "data-testid": "new-shop", value: "{new_shop}", oninput: move |e| new_shop.set(e.value()) }
                                        button { r#type: "button", class: "btn-primary", "data-testid": "add-shop", disabled: new_shop().trim().is_empty(),
                                            onclick: { let custom = custom.clone(); move |_| {
                                                let shop = new_shop().trim().to_string();
                                                let mut next = custom.clone();
                                                let mut all: Vec<Value> = next["woodlandShops"].as_array().cloned().unwrap_or_default();
                                                if !all.iter().any(|x| x.as_str() == Some(shop.as_str())) { all.push(json!(shop)); }
                                                next["woodlandShops"] = json!(all);
                                                draft.write()["subDepartment"] = json!(shop);
                                                adding_shop.set(false);
                                                new_shop.set(String::new());
                                                spawn(async move { if let Err(e) = crate::views::workforce_ops::save_record(app, "custom_datalists", "_", &next).await { app.error(e) } });
                                            } }, "Add shop" }
                                    }
                                }
                            }
                            PField { draft, k: "trade", label: "Individual trade / skill" }
                            PField { draft, k: "skillLevel", label: "Skill level" }
                            PField { draft, k: "tradeGroup", label: "Trade group name" }
                            PField { draft, k: "tradeRole", label: "Trade role", kind: "select", options: options_with(ROLES, &[], &s(&d, "tradeRole")) }
                            PField { draft, k: "hourlyRate", label: "Regular hourly rate (₱)", kind: "number" }
                        }
                        div { class: "flex flex-wrap gap-6",
                            Check { draft, k: "jOrder", label: "Issue job order?" }
                            Check { draft, k: "disableStaleAlerts", label: "Enable stale task alerts?", inverted: true }
                        }
                    },
                    _ => rsx! {
                        div { class: "grid grid-cols-2 sm:grid-cols-5 gap-2", "data-testid": "profile-att-kpis",
                            for (label, n) in [("Total duty", rows.len().to_string()), ("Present", present.to_string()), ("Passes", passes.to_string()), ("Absent", absent.to_string()), ("Reliability", format!("{reliability:.1}%"))] {
                                div { key: "{label}", class: "card p-3", p { class: "text-xs text-muted", "{label}" } p { class: "text-lg font-semibold tabular", "{n}" } }
                            }
                        }
                        div { class: "flex flex-wrap gap-2 items-center",
                            for (k, l) in [("all", "All"), ("present", "Present"), ("passes", "Passes"), ("absent", "Absent")] {
                                button { key: "{k}", r#type: "button", class: if att_filter() == k { "btn-primary h-8" } else { "btn-secondary h-8" }, "data-testid": "att-filter-{k}", onclick: move |_| att_filter.set(k), "{l}" }
                            }
                            input { class: "field h-8 flex-1 min-w-[10rem]", placeholder: "Search date, status, task…", "data-testid": "att-search", value: "{att_q}", oninput: move |e| att_q.set(e.value()) }
                            button { r#type: "button", class: "btn-secondary h-8", "data-testid": "att-print-dossier", onclick: move |_| s5print("attendanceDossier", json!([cur().to_string()])), Icon { name: "archive" } "Print official dossier" }
                            button { r#type: "button", class: "btn-secondary h-8", "data-testid": "att-assignment-history", onclick: move |_| s5print("historyPage", json!([{ "id": cur().to_string(), "kind": "assignment" }])), "View Assignment History" }
                            button { r#type: "button", class: "btn-secondary h-8", "data-testid": "att-print-update", onclick: move |_| s5print("updateForms", json!([{ "ids": [cur().to_string()], "options": { "mode": "prefilled", "paperSize": "folio", "includePhoto": true } }])), Icon { name: "edit" } "Print update form" }
                        }
                        div { class: "overflow-x-auto",
                            table { class: "w-full text-sm", "data-testid": "profile-att-table",
                                thead { tr { class: "text-left text-muted", for h in ["Date", "Status", "Shift / type", "Task / details / area", "Verified by", "Notes / reason"] { th { key: "{h}", class: "p-2 font-medium", "{h}" } } } }
                                tbody {
                                    if shown.is_empty() { tr { td { class: "p-2 text-muted", colspan: "6", "No attendance entries." } } }
                                    for (i, r) in shown.iter().enumerate() {
                                        tr { key: "{i}", class: "border-t border-line",
                                            td { class: "p-2 tabular whitespace-nowrap", "{r.date}" } td { class: "p-2", "{r.status}" } td { class: "p-2", "{r.shift}" }
                                            td { class: "p-2", "{r.details}" } td { class: "p-2", "{r.verified}" } td { class: "p-2", "{r.notes}" }
                                        }
                                    }
                                }
                            }
                        }
                    },
                }
            }
            // Footer: navigation and save.
            div { class: "modal-actions no-print flex-wrap",
                button { r#type: "button", class: "btn-ghost", "data-testid": "profile-prev", "aria-label": "Previous person", onclick: move |_| save_then(-1), Icon { name: "chevron-left" } "Prev" }
                select { class: "field h-9 max-w-[14rem]", "aria-label": "Jump to volunteer", "data-testid": "profile-jump",
                    onchange: move |e| { if let Ok(id) = e.value().parse::<Uuid>() { cur.set(id) } },
                    for w in people.iter() { option { key: "{w.id}", value: "{w.id}", selected: Some(w.id) == Some(id), "{w.name}" } }
                }
                button { r#type: "button", class: "btn-ghost", "data-testid": "profile-next", "aria-label": "Next person", onclick: move |_| save_then(1), "Next" Icon { name: "chevron-right" } }
                if archived { button { r#type: "button", class: "btn-secondary", "data-testid": "profile-reemploy", onclick: move |_| reemploying.set(true), "Re-employ volunteer" } }
                span { class: "flex-1" }
                if let Some(i) = pos { span { class: "text-xs text-muted tabular", "{i + 1} of {people.len()}" } }
                button { r#type: "button", class: "btn-secondary", "data-testid": "profile-save-next", disabled: busy(), onclick: move |_| save_then(1), "Save & next" }
                button { r#type: "button", class: "btn-primary", "data-testid": "profile-save", disabled: busy() || !dirty,
                    onclick: move |_| async move {
                        busy.set(true);
                        let r = save(app, cur(), &draft()).await;
                        busy.set(false);
                        match r { Ok(_) => { app.success("Saved"); base.set(draft()); on_close.call(()); } Err(e) => app.error(e) }
                    },
                    if busy() { Spinner {} } "Save"
                }
            }
        }
    };

    rsx! {
        div { class: "contents", "data-testid": "profile-modal",
            Modal { title, wide: true, on_close: move |_| on_close.call(()), {body} }
        }
        if let Some(v) = visit() {
            RecordModal { title: if s(&v, "status") == "scheduled" { "Schedule visit".to_string() } else { "Log past visit".to_string() }, list: "visitations", id: new_id("v"),
                fields: vec![
                    f("date", if s(&v, "status") == "scheduled" { "Visitation date" } else { "Date of visit" }, Kind::Date),
                    f("ministerName", "Visiting minister", Kind::Text),
                    f("status", "Status", crate::views::workforce_ops::choice(&["scheduled", "completed", "canceled"])),
                    f("completionDetails.members", "Members present", Kind::Text),
                    f("completionDetails.concerns", "Issues or concerns raised", Kind::Long),
                    f("completionDetails.cancelReason", "Reason for cancelation", Kind::Text),
                    f("completionDetails.photoDataUrl", "Photo evidence", Kind::Photo),
                    f("notes", "Notes / purpose", Kind::Long),
                ],
                initial: v.clone(), deletable: false, on_close: move |_| visit.set(None) }
        }
        if reemploying() {
            crate::views::personnel::BulkModal { kind: crate::views::personnel::Bulk::Restore, workers: vec![worker.clone()],
                on_close: move |done: bool| { reemploying.set(false); if done { on_close.call(()) } } }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_keep_a_stored_value_that_is_not_listed() {
        assert_eq!(options_with(&["A"], &["B".into()], "C"), vec!["A", "B", "C"]);
        assert_eq!(options_with(&["A"], &[], "A"), vec!["A"]);
    }

    #[test]
    fn family_rows_describe_spouse_and_children() {
        let p = json!({ "spouseName": "S", "children": [{ "name": "K", "isStudent": true, "school": "U", "course": "BS", "livesHere": true }, { "name": "J", "job": "Driver", "workplace": "Co" }] });
        let r = family_rows(&p);
        assert_eq!(r.len(), 3);
        assert_eq!(r[1][4], "Studying: U (BS)");
        assert_eq!(r[2][4], "Driver at Co");
        assert_eq!(r[1][6], "Yes");
    }

    #[test]
    fn attendance_kinds_follow_service5() {
        assert_eq!(kind_of("Present"), "present");
        assert_eq!(kind_of("Passes"), "passes");
        assert_eq!(kind_of("Medical"), "passes");
        assert_eq!(kind_of("Absent"), "absent");
    }
}
