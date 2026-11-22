//! Service 5's Attendance Sheet Generator and Semi-Annual Records Update Forms,
//! with every option of their dialogs: sheet format, grouping, sorting, page
//! breaks, photos, filters and selection, Excel export; form type, paper size,
//! blank copies, scope and photo.

use std::collections::{BTreeMap, HashSet};

use dioxus::prelude::*;
use serde_json::{json, Value};
use shared::*;
use uuid::Uuid;

use crate::api;
use crate::components::ui::{EmptyState, Icon};
use crate::views::personnel::save_file;
use crate::views::profile::{age, excel_xml};
use crate::views::workforce_ops::{s, s5print, today, use_workers, WorkerPhoto};

const FORMATS: &[(&str, &str)] = &[
    ("meeting", "Meeting attendance (name, trade, signature)"),
    ("attendance", "Daily work attendance (times & signatures)"),
    ("roster", "Project roster (tasks & contacts)"),
];
const GROUP_BY: &[(&str, &str)] = &[
    ("none", "None (continuous list)"),
    ("trade", "By trade / skill"),
    ("visitationGroup", "By visitation group"),
    ("projectGroup", "By project group"),
    ("area", "By duty area"),
    ("visitationHousing", "By housing / residence"),
];
const SORT_BY: &[(&str, &str)] = &[("name", "Name (A → Z)"), ("aNumber", "Assigned number (A#)"), ("trade", "Trade / skill"), ("age_asc", "Age (youngest first)"), ("age_desc", "Age (oldest first)")];

/// Service 5's group label for a person (empty values get its wording).
fn group_value(w: &Worker, by: &str) -> String {
    let or = |v: &str, none: &str| if v.trim().is_empty() { none.to_string() } else { v.trim().to_string() };
    match by {
        "trade" => or(&w.trade, "General / Unassigned Trade"),
        "visitationGroup" => or(&w.visit_group, "No Visitation Group"),
        "projectGroup" => or(&w.project_group, "No Project Group"),
        "area" => or(&w.area, "Unassigned Area"),
        "visitationHousing" => residence(w),
        _ => "General".into(),
    }
}

fn residence(w: &Worker) -> String {
    [&w.housing, &w.residence].into_iter().find(|x| !x.trim().is_empty()).cloned().unwrap_or_else(|| "-".into())
}

/// Service 5's sort orders; ties fall back to the name.
pub fn sort_people(list: &mut [Worker], by: &str) {
    let name = |w: &Worker| w.name.to_lowercase();
    match by {
        "aNumber" => list.sort_by(|a, b| {
            let (x, y) = (a.a_number.trim().parse::<i64>().ok(), b.a_number.trim().parse::<i64>().ok());
            match (x, y) {
                (Some(x), Some(y)) => x.cmp(&y).then(name(a).cmp(&name(b))),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                _ => name(a).cmp(&name(b)),
            }
        }),
        "trade" => list.sort_by(|a, b| a.trade.trim().cmp(b.trade.trim()).then(name(a).cmp(&name(b)))),
        "age_asc" => list.sort_by(|a, b| age(&a.birthday).unwrap_or(9999).cmp(&age(&b.birthday).unwrap_or(9999)).then(name(a).cmp(&name(b)))),
        "age_desc" => list.sort_by(|a, b| age(&b.birthday).unwrap_or(-1).cmp(&age(&a.birthday).unwrap_or(-1)).then(name(a).cmp(&name(b)))),
        _ => list.sort_by_key(name),
    }
}

fn status_kind(w: &Worker) -> &'static str {
    match w.status {
        WorkerStatus::Present => "present",
        WorkerStatus::Absent => "absent",
        WorkerStatus::OnPass => "passes",
    }
}

fn distinct(people: &[Worker], f: impl Fn(&Worker) -> String) -> Vec<String> {
    let mut v: Vec<String> = people.iter().map(f).filter(|x| !x.trim().is_empty()).collect();
    v.sort();
    v.dedup();
    v
}

/// The columns of a sheet format, and one person's cells.
fn columns(format: &str) -> Vec<&'static str> {
    match format {
        "meeting" => vec!["#", "Volunteer name", "Trade / skill", "Signature"],
        "attendance" => vec!["#", "A-No.", "Volunteer name", "Age", "Trade / skill", "Residence / housing", "Time in", "Time out", "Signature"],
        _ => vec!["#", "A-No.", "Volunteer name", "Age", "Trade / skill", "Residence / housing", "Contact no.", "Task / role remarks"],
    }
}

fn cells(format: &str, n: usize, w: &Worker) -> Vec<String> {
    let a = age(&w.birthday).map(|a| a.to_string()).unwrap_or_default();
    match format {
        "meeting" => vec![n.to_string(), w.name.clone(), w.trade.clone(), String::new()],
        "attendance" => vec![n.to_string(), w.a_number.clone(), w.name.clone(), a, w.trade.clone(), residence(w), String::new(), String::new(), String::new()],
        _ => vec![n.to_string(), w.a_number.clone(), w.name.clone(), a, w.trade.clone(), residence(w), w.phone.clone(), [w.task.clone(), w.group_role.clone()].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ")],
    }
}

#[component]
pub fn AttendanceGenerator() -> Element {
    let workers = use_workers();
    let mut title = use_signal(|| "Attendance Sheet".to_string());
    let mut date = use_signal(today);
    let mut format = use_signal(|| "attendance".to_string());
    let mut facilitator = use_signal(String::new);
    let mut venue = use_signal(String::new);
    let mut group_by = use_signal(|| "none".to_string());
    let mut sort_by = use_signal(|| "name".to_string());
    let mut page_break = use_signal(|| false);
    let mut photos = use_signal(|| true);
    let mut q = use_signal(String::new);
    let f_pg = use_signal(String::new);
    let f_vg = use_signal(String::new);
    let f_area = use_signal(String::new);
    let f_trade = use_signal(String::new);
    let mut f_status = use_signal(|| "all".to_string());
    let mut picked = use_signal(HashSet::<Uuid>::new);
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    let ql = q().to_lowercase();
    let candidates: Vec<Worker> = people
        .iter()
        .filter(|w| ql.is_empty() || [&w.name, &w.a_number, &w.trade, &w.project_group].iter().any(|x| x.to_lowercase().contains(&ql)))
        .filter(|w| f_pg().is_empty() || w.project_group == f_pg())
        .filter(|w| f_vg().is_empty() || w.visit_group == f_vg())
        .filter(|w| f_area().is_empty() || w.area == f_area())
        .filter(|w| f_trade().is_empty() || w.trade == f_trade())
        .filter(|w| f_status() == "all" || status_kind(w) == f_status())
        .cloned()
        .collect();
    let all_in_view = !candidates.is_empty() && candidates.iter().all(|w| picked().contains(&w.id));
    // The sheet: chosen people, grouped and sorted the way the options say.
    let mut chosen: Vec<Worker> = people.iter().filter(|w| picked().contains(&w.id)).cloned().collect();
    let mut groups: BTreeMap<String, Vec<Worker>> = BTreeMap::new();
    for w in chosen.drain(..) {
        groups.entry(if group_by() == "none" { String::new() } else { group_value(&w, &group_by()) }).or_default().push(w);
    }
    for list in groups.values_mut() {
        sort_people(list, &sort_by());
    }
    let cols = columns(&format());
    let n = picked().len();
    let export = {
        let groups = groups.clone();
        let cols = cols.clone();
        move |_| {
            let mut rows = vec![];
            let mut i = 0;
            for (g, list) in &groups {
                for w in list {
                    i += 1;
                    let mut r = cells(&format(), i, w);
                    if !g.is_empty() {
                        r.insert(1, g.clone());
                    }
                    rows.push(r);
                }
            }
            let mut head = cols.clone();
            if group_by() != "none" {
                head.insert(1, "Group");
            }
            save_file(&format!("{}-{}.xls", title().replace(' ', "_"), date()), &excel_xml("Attendance", &head, &rows));
        }
    };
    let select = |id: &'static str, label: &'static str, value: String, all: &'static str, opts: Vec<String>, mut set: Signal<String>| {
        rsx! {
            div {
                label { class: "label", r#for: "{id}", "{label}" }
                select { id: "{id}", class: "field", "data-testid": "{id}", onchange: move |e| set.set(e.value()),
                    option { value: "", "{all}" }
                    for o in opts { option { key: "{o}", value: "{o}", selected: o == value, "{o}" } }
                }
            }
        }
    };
    rsx! {
        div { class: "space-y-4", "data-testid": "attendance-generator",
            div { class: "card p-4 grid sm:grid-cols-2 lg:grid-cols-3 gap-3 no-print",
                div { label { class: "label", r#for: "ag-title", "Sheet / meeting title" } input { id: "ag-title", class: "field", "data-testid": "ag-title", value: "{title}", oninput: move |e| title.set(e.value()) } }
                div { label { class: "label", r#for: "ag-date", "Date" } input { id: "ag-date", r#type: "date", class: "field", "data-testid": "ag-date", value: "{date}", oninput: move |e| date.set(e.value()) } }
                div {
                    label { class: "label", r#for: "ag-format", "Format style" }
                    select { id: "ag-format", class: "field", "data-testid": "ag-format", onchange: move |e| format.set(e.value()),
                        for (v, l) in FORMATS { option { key: "{v}", value: "{v}", selected: format() == *v, "{l}" } }
                    }
                }
                div { label { class: "label", r#for: "ag-fac", "Conducted / presided by" } input { id: "ag-fac", class: "field", "data-testid": "ag-facilitator", value: "{facilitator}", oninput: move |e| facilitator.set(e.value()) } }
                div { label { class: "label", r#for: "ag-venue", "Venue / location" } input { id: "ag-venue", class: "field", "data-testid": "ag-venue", value: "{venue}", oninput: move |e| venue.set(e.value()) } }
                div {
                    label { class: "label", r#for: "ag-group", "Group attendance by" }
                    select { id: "ag-group", class: "field", "data-testid": "ag-group-by", onchange: move |e| group_by.set(e.value()),
                        for (v, l) in GROUP_BY { option { key: "{v}", value: "{v}", selected: group_by() == *v, "{l}" } }
                    }
                }
                div {
                    label { class: "label", r#for: "ag-sort", "Sort within group by" }
                    select { id: "ag-sort", class: "field", "data-testid": "ag-sort-by", onchange: move |e| sort_by.set(e.value()),
                        for (v, l) in SORT_BY { option { key: "{v}", value: "{v}", selected: sort_by() == *v, "{l}" } }
                    }
                }
                div { class: "flex items-end gap-4 pb-2",
                    label { class: "flex items-center gap-2 text-sm", input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "ag-page-break", checked: page_break(), onchange: move |e| page_break.set(e.checked()) } "Separate page per group" }
                    label { class: "flex items-center gap-2 text-sm", input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "ag-photos", checked: photos(), onchange: move |e| photos.set(e.checked()) } "Photos in print" }
                }
            }
            div { class: "card p-4 space-y-3 no-print",
                div { class: "grid sm:grid-cols-3 lg:grid-cols-6 gap-3",
                    div { class: "sm:col-span-3 lg:col-span-1",
                        label { class: "label", r#for: "ag-q", "Search" }
                        input { id: "ag-q", r#type: "search", class: "field", placeholder: "Name, A#, trade…", "data-testid": "ag-search", value: "{q}", oninput: move |e| q.set(e.value()) }
                    }
                    {select("ag-filter-pg", "Project group", f_pg(), "All project groups", distinct(&people, |w| w.project_group.clone()), f_pg)}
                    {select("ag-filter-vg", "Visitation group", f_vg(), "All visitation groups", distinct(&people, |w| w.visit_group.clone()), f_vg)}
                    {select("ag-filter-area", "Area", f_area(), "All areas", distinct(&people, |w| w.area.clone()), f_area)}
                    {select("ag-filter-trade", "Trade", f_trade(), "All trades", distinct(&people, |w| w.trade.clone()), f_trade)}
                    div {
                        label { class: "label", r#for: "ag-filter-status", "Status" }
                        select { id: "ag-filter-status", class: "field", "data-testid": "ag-filter-status", onchange: move |e| f_status.set(e.value()),
                            for (v, l) in [("all", "All status"), ("present", "Present on site"), ("absent", "Absent"), ("passes", "Passes / on pass")] { option { key: "{v}", value: "{v}", selected: f_status() == v, "{l}" } }
                        }
                    }
                }
                div { class: "flex flex-wrap items-center gap-2",
                    label { class: "flex items-center gap-2 text-sm",
                        input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "ag-select-all-box", checked: all_in_view,
                            onchange: { let ids: Vec<Uuid> = candidates.iter().map(|w| w.id).collect(); move |e: FormEvent| { let mut p = picked.write(); for id in &ids { if e.checked() { p.insert(*id); } else { p.remove(id); } } } } }
                        "{candidates.len()} shown"
                    }
                    button { class: "btn-secondary h-8", "data-testid": "ag-select-all", onclick: { let ids: Vec<Uuid> = candidates.iter().map(|w| w.id).collect(); move |_| picked.write().extend(ids.iter().copied()) }, "Select all" }
                    button { class: "btn-ghost h-8", "data-testid": "ag-clear", onclick: move |_| picked.write().clear(), "Clear" }
                    span { class: "flex-1" }
                    button { class: "btn-secondary", "data-testid": "ag-export", disabled: n == 0, onclick: export, Icon { name: "archive" } "Export to Excel" }
                    button { class: "btn-primary", "data-testid": "ag-print", disabled: n == 0, onclick: move |_| s5print("attendanceSheet", json!([{
                        "ids": picked().iter().map(Uuid::to_string).collect::<Vec<_>>(), "title": title(), "date": date(), "facilitator": facilitator(), "venue": venue(),
                        "type": format(), "photos": photos(), "groupBy": group_by(), "sortBy": sort_by(), "pageBreak": page_break(),
                    }])), Icon { name: "archive" } "Generate & print ({n})" }
                }
                div { class: "max-h-72 overflow-y-auto grid sm:grid-cols-2 lg:grid-cols-3 gap-1", "data-testid": "ag-candidates",
                    if candidates.is_empty() { p { class: "text-sm text-muted", "No one matches these filters." } }
                    for w in candidates.iter() {
                        label { key: "{w.id}", class: "flex items-center gap-2 text-sm px-2 py-1 rounded hover:bg-hover",
                            input { r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "ag-pick", checked: picked().contains(&w.id),
                                onchange: { let id = w.id; move |e: FormEvent| { if e.checked() { picked.write().insert(id); } else { picked.write().remove(&id); } } } }
                            WorkerPhoto { id: w.id, name: w.name.clone(), has_photo: w.has_photo, size: 26 }
                            span { class: "truncate", "{w.name}" }
                            span { class: "text-xs text-muted truncate", "{w.trade}" }
                        }
                    }
                }
            }
            if n == 0 {
                div { class: "card no-print", EmptyState { icon: "users", title: "Pick the people for the sheet", body: "Use the filters and tick names, or Select all." } }
            }
        }
    }
}

#[component]
pub fn UpdateFormsPrint() -> Element {
    let workers = use_workers();
    let profiles = use_resource(|| async { api::get::<Vec<Value>>("/workforce/family-roster").await.unwrap_or_default() });
    let mut cycle = use_signal(|| {
        use chrono::Datelike;
        let now = chrono::Local::now();
        format!("Semi-Annual Update {} ({} Half)", now.year(), if now.month() <= 6 { "1st" } else { "2nd" })
    });
    let mut mode = use_signal(|| "prefilled".to_string());
    let mut paper = use_signal(|| "folio".to_string());
    let mut copies = use_signal(|| 10u32);
    let mut scope = use_signal(|| "active".to_string());
    let mut f_group = use_signal(String::new);
    let mut f_housing = use_signal(String::new);
    let mut single = use_signal(String::new);
    let mut photo = use_signal(|| true);
    let people: Vec<Worker> = workers.read().clone().unwrap_or_default().into_iter().filter(|w| !w.archived).collect();
    let list: Vec<Value> = profiles
        .read()
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| match scope().as_str() {
            "project-group" => s(p, "group") == f_group(),
            "housing" => s(p, "visitationHousing") == f_housing(),
            "single" => s(p, "id") == single(),
            _ => true,
        })
        .collect();
    let blank = mode() == "blank";
    let count = if blank { copies() as usize } else { list.len() };
    rsx! {
        div { class: "space-y-4", "data-testid": "update-forms-print",
            div { class: "card p-5 space-y-4 no-print max-w-3xl", "data-testid": "vru-panel",
                div {
                    p { class: "text-lg font-semibold", "Volunteer Records Update Form" }
                    p { class: "text-sm text-muted", "Semi-Annual Verification & Profiling Sheet Generator" }
                }
                p { class: "rounded-md bg-tag-blue-bg text-tag-blue-text text-sm px-3 py-2", "data-testid": "vru-notice",
                    strong { "Bi-Annual Census / Update Cycle: " }
                    "Distribute these 1-page individual verification sheets to volunteer workers. Volunteers review their current on-file data and write corrections directly on the lines provided."
                }
                div { label { class: "label", r#for: "vru-cycle", "Update Cycle / Title Banner" } input { id: "vru-cycle", class: "field", "data-testid": "vru-cycle", value: "{cycle}", oninput: move |e| cycle.set(e.value()) } }
                div { class: "grid sm:grid-cols-2 gap-3",
                    div {
                        label { class: "label", r#for: "vru-mode", "Form Type" }
                        select { id: "vru-mode", class: "field", "data-testid": "vru-mode", onchange: move |e| mode.set(e.value()),
                            option { value: "prefilled", selected: !blank, "Pre-filled (Verification / Dotted Correction Lines)" }
                            option { value: "blank", selected: blank, "Blank Profiling Sheet (For Walk-ins / New)" }
                        }
                    }
                    div {
                        label { class: "label", r#for: "vru-paper", "Target Paper Size" }
                        select { id: "vru-paper", class: "field", "data-testid": "vru-paper", onchange: move |e| paper.set(e.value()),
                            for (v, l) in [("folio", "Folio / Long (8.5 x 13 in) (Recommended)"), ("letter", "Letter (8.5 x 11 in)"), ("a4", "A4 (8.27 x 11.69 in)")] {
                                option { key: "{v}", value: "{v}", selected: paper() == v, "{l}" }
                            }
                        }
                    }
                }
                if blank {
                    div {
                        label { class: "label", r#for: "vru-copies", "Number of Blank Copies to Print" }
                        input { id: "vru-copies", r#type: "number", min: "1", max: "250", inputmode: "numeric", class: "field w-40", "data-testid": "vru-copies", value: "{copies}",
                            oninput: move |e| copies.set(e.value().parse::<u32>().unwrap_or(10).clamp(1, 250)) }
                    }
                } else {
                    div {
                        label { class: "label", r#for: "vru-scope", "Target Volunteers (Scope)" }
                        select { id: "vru-scope", class: "field", "data-testid": "vru-scope", onchange: move |e| scope.set(e.value()),
                            for (v, l) in [("active", "All Active Volunteers (Excludes archived)"), ("project-group", "Filter by Project Group"), ("housing", "Filter by Housing Cluster / Sitio"), ("single", "Single Volunteer Worker")] {
                                option { key: "{v}", value: "{v}", selected: scope() == v, "{l}" }
                            }
                        }
                    }
                    if scope() == "project-group" {
                        div {
                            label { class: "label", r#for: "vru-group", "Select Project Group" }
                            select { id: "vru-group", class: "field", "data-testid": "vru-group", onchange: move |e| f_group.set(e.value()),
                                option { value: "", "Choose…" }
                                for g in distinct(&people, |w| w.project_group.clone()) { option { key: "{g}", value: "{g}", selected: f_group() == g, "{g}" } }
                            }
                        }
                    }
                    if scope() == "housing" {
                        div {
                            label { class: "label", r#for: "vru-housing", "Select Housing Cluster" }
                            select { id: "vru-housing", class: "field", "data-testid": "vru-housing", onchange: move |e| f_housing.set(e.value()),
                                option { value: "", "Choose…" }
                                for h in distinct(&people, |w| w.housing.clone()) { option { key: "{h}", value: "{h}", selected: f_housing() == h, "{h}" } }
                            }
                        }
                    }
                    if scope() == "single" {
                        div {
                            label { class: "label", r#for: "vru-single", "Select Volunteer Worker" }
                            select { id: "vru-single", class: "field", "data-testid": "vru-single", onchange: move |e| single.set(e.value()),
                                option { value: "", "Choose…" }
                                for w in people.iter() { option { key: "{w.id}", value: "{w.id}", selected: single() == w.id.to_string(), "{w.name}" } }
                            }
                        }
                    }
                }
                label { class: "flex items-center gap-2 text-sm",
                    input { id: "vru-include-photo", r#type: "checkbox", class: "w-4 h-4 accent-[#064e3b]", "data-testid": "vru-photo", checked: photo(), onchange: move |e| photo.set(e.checked()) }
                    "Include worker photo in header (if available on file)"
                }
                div { class: "rounded-md border border-line p-3 flex flex-wrap justify-between gap-3 text-sm", "data-testid": "vru-summary",
                    div {
                        p { class: "text-xs uppercase tracking-wide text-muted", "Print Queue Summary" }
                        p { class: "font-semibold", "data-testid": "vru-count-preview",
                            {if blank { format!("Will print {} blank copies (1 page per sheet)", copies()) } else { format!("Will generate {} personalized update form(s) (1 page per volunteer)", count) }}
                        }
                    }
                    div { class: "text-xs text-muted text-right", p { "1-Page Layout" } p { "Auto Page-break" } }
                }
                div { class: "flex justify-end",
                    button { class: "btn-primary", "data-testid": "vru-print", disabled: count == 0, onclick: { let ids: Vec<String> = list.iter().map(|p| s(p, "id")).collect(); move |_| s5print("updateForms", json!([{
                        "ids": ids, "blank": copies(), "options": { "mode": mode(), "cycle": cycle(), "paperSize": paper(), "includePhoto": photo() },
                    }])) },
                        {if blank { format!("🖨️ Print {} Blank Forms", copies()) } else { format!("🖨️ Print {} Update Form{}", count, if count == 1 { "" } else { "s" }) }}
                    }
                }
            }
            if count == 0 {
                div { class: "card no-print", EmptyState { icon: "archive", title: "Nothing to print", body: "Choose who the forms are for." } }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(name: &str, a: &str, birthday: &str) -> Worker {
        serde_json::from_value(serde_json::json!({
            "id": Uuid::new_v4(), "a_number": a, "name": name, "phone": "", "trade": "", "department": "", "area": "", "project_group": "",
            "group_role": "", "status": "Present", "archived": false, "updated_at": chrono::Utc::now(), "birthday": birthday,
        }))
        .unwrap()
    }

    #[test]
    fn sorts_follow_service5() {
        let mut v = vec![w("Cruz", "12", "1990-01-01"), w("Abad", "", "1960-05-05"), w("Bata", "3", "")];
        sort_people(&mut v, "aNumber");
        assert_eq!(v.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["Bata", "Cruz", "Abad"]);
        sort_people(&mut v, "age_desc");
        assert_eq!(v[0].name, "Abad");
        sort_people(&mut v, "age_asc");
        assert_eq!(v[0].name, "Cruz");
        assert_eq!(v[2].name, "Bata"); // no birthday sorts last
    }

    #[test]
    fn group_labels_use_service5_wording_for_blanks() {
        let p = w("A", "1", "");
        assert_eq!(group_value(&p, "trade"), "General / Unassigned Trade");
        assert_eq!(group_value(&p, "projectGroup"), "No Project Group");
    }
}
