//! The sample workforce that "Load sample data" brings in with the sample
//! estate: Service 5-style people, groups, duty, attendance, overtime,
//! visitations, clinic records and signatories, linked to the sample zones
//! and jobs so every screen tells the same story.

use chrono::{Duration, Local, Utc};
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait};
use serde_json::{json, Value};

use crate::error::AppResult;
use crate::services::workforce;

/// (A#, name, trade, department, area, project group, role, status, housing)
const PEOPLE: &[(&str, &str, &str, &str, &str, &str, &str, &str, &str)] = &[
    ("101", "Ramon Dela Cruz", "Mason", "Job-site Construction", "Hall Area", "Hall | Masons", "Katiwala", "Present", "Staff House A"),
    ("102", "Jose Santos", "Mason", "Job-site Construction", "Hall Area", "Hall | Masons", "II Katiwala", "Present", "Staff House A"),
    ("103", "Mario Reyes", "Mason", "Job-site Construction", "Hall Area", "Hall | Masons", "Member", "Present", "Staff House A"),
    ("104", "Paolo Garcia", "Laborer", "Job-site Construction", "Hall Area", "Hall | Masons", "Member", "Absent", "Staff House B"),
    ("105", "Andres Villanueva", "Carpenter", "Job-site Construction", "Garden Area", "Garden | Builders", "Katiwala", "Present", "Staff House B"),
    ("106", "Eduardo Mendoza", "Carpenter", "Job-site Construction", "Garden Area", "Garden | Builders", "Member", "Present", "Staff House B"),
    ("107", "Felipe Bautista", "Electrician", "Job-site Construction", "Garden Area", "Garden | Builders", "Member", "Passes", "Villa Quarters"),
    ("108", "Lorenzo Aquino", "Steelman", "Job-site Construction", "Terrace Area", "Terrace | Concrete", "Katiwala", "Present", "Villa Quarters"),
    ("109", "Danilo Ramos", "Laborer", "Job-site Construction", "Terrace Area", "Terrace | Concrete", "Member", "Present", "Villa Quarters"),
    ("110", "Rogelio Castillo", "Laborer", "Job-site Construction", "Terrace Area", "Terrace | Concrete", "Member", "Present", "Staff House A"),
    ("111", "Teodoro Navarro", "Carpenter", "Woodland Shops", "Workshop Area", "Woodland | Carpenters", "Katiwala", "Present", "Staff House B"),
    ("112", "Victor Salazar", "Painter", "Woodland Shops", "Workshop Area", "Woodland | Carpenters", "Member", "Present", "Staff House B"),
    ("113", "Ernesto Flores", "Plumber", "Water System", "Utility Area", "Utility | Plumbers", "Katiwala", "Present", "Villa Quarters"),
    ("114", "Benigno Torres", "Plumber", "Water System", "Utility Area", "Utility | Plumbers", "Member", "Absent", "Villa Quarters"),
    ("115", "Gregorio Lim", "Gardener", "Garden", "Garden Area", "Garden | Builders", "Member", "Present", "Staff House A"),
    ("116", "Nestor Ocampo", "Helper", "Residential Only", "", "", "Member", "Present", "Villa Quarters"),
];

/// Service 5 area → the sample zone it lies in.
const AREA_ZONES: &[(&str, &str)] = &[
    ("Hall Area", "Multi-Purpose Hall (Parking Conversion)"),
    ("Garden Area", "Rose Garden & Gazebo"),
    ("Terrace Area", "Retaining Wall Terrace"),
    ("Workshop Area", "Contractor Workshop"),
    ("Utility Area", "Water Filtration Plant"),
];

/// Project group → the sample job it works on.
const GROUP_JOBS: &[(&str, &str)] = &[
    ("Hall | Masons", "Parking Conversion to Multi-Purpose Hall"),
    ("Garden | Builders", "Rose Garden Gazebo Build"),
    ("Terrace | Concrete", "Retaining Wall Leveling"),
    ("Utility | Plumbers", "Water Filtration Backwash"),
];

fn day(offset: i64) -> String {
    (Local::now().date_naive() + Duration::days(offset)).format("%Y-%m-%d").to_string()
}

fn ms(offset: i64, hour: u32) -> i64 {
    (Local::now().date_naive() + Duration::days(offset)).and_hms_opt(hour, 0, 0).map(|t| t.and_utc().timestamp_millis()).unwrap_or(0)
}

/// The sample as a Service 5 v11 backup; `zone_of` / `job_of` give the ids of
/// the sample estate's zones and jobs by name (missing ones are left unlinked).
pub fn backup(zone_of: impl Fn(&str) -> Option<String>, job_of: impl Fn(&str) -> Option<String>) -> Value {
    let month = Local::now().format("%m").to_string();
    let today = Local::now().date_naive();
    let workers: Vec<Value> = PEOPLE
        .iter()
        .enumerate()
        .map(|(i, (a, name, trade, dept, area, group, role, status, housing))| {
            let married = i % 3 != 2;
            let mut w = json!({
                "aNumber": a, "name": name, "trade": trade, "department": dept, "area": area, "projectGroup": group, "groupRole": role,
                "dailyStatus": status, "status": status, "phone": format!("0917 555 0{a}"), "visitationHousing": housing,
                "workResidence": housing, "civilStatus": if married { "Married" } else { "Single" }, "language": "Tagalog",
                "address": "Brgy. San Isidro, Tanay, Rizal", "skillLevel": (["Lead", "Skilled", "Semi"][i % 3]), "hourlyRate": 78 + (i % 4) as i64,
                "jOrder": if *a == "116" { "NO" } else { "YES" },
                "birthday": format!("{}-{}-{:02}", 1975 + i, if i % 5 == 0 { month.clone() } else { format!("{:02}", (i % 12) + 1) }, 3 + i),
                "emergencyContact": format!("{} (family)", name.split(' ').next_back().unwrap_or("")), "emergencyPhone": format!("0918 444 0{a}"),
                "emergencyRelationship": if married { "Spouse" } else { "Parent" },
                "medical": (["Good condition", "Hypertension", "Good condition", "Back pain"][i % 4]),
                "visitationGroup": if i % 2 == 0 { "Dalaw Group 1" } else { "Dalaw Group 2" },
                "visitationRole": if i == 0 || i == 1 { "Katiwala" } else { "Member" },
                "project": if group.is_empty() { "" } else { group.split(" | ").next().unwrap_or("") },
                "specificTask": match *trade { "Mason" => "Block laying", "Carpenter" => "Formwork", "Plumber" => "Pipe fitting", "Painter" => "Priming", _ => "General work" },
                "isAssignedToday": *status == "Present",
                "taskStartedAt": format!("{}T07:00:00Z", day(if i % 4 == 1 { -5 } else { -1 })),
                "updatedAt": Utc::now().to_rfc3339(),
            });
            if married {
                w["spouseName"] = json!(format!("Maria {}", name.split(' ').next_back().unwrap_or("")));
                w["weddingDate"] = json!(format!("{}-06-15", 2000 + i));
                w["children"] = json!([{ "name": format!("Juan {}", name.split(' ').next_back().unwrap_or("")), "birthday": format!("{}-02-10", 2005 + i), "isStudent": true, "school": "Tanay National High School", "livesHere": false }]);
            }
            if *group == "Hall | Masons" {
                // Staged for tomorrow in the encoder (Start New Day turns it into the day's task).
                w["tomorrowAssignment"] = json!({ "specificTask": "Pour the hall slab", "area": area, "project": "Hall", "date": day(1), "isAssigned": true, "dailyStatus": "Present" });
            }
            w
        })
        .collect();

    let area_zones: Vec<Value> = AREA_ZONES.iter().filter_map(|(a, z)| zone_of(z).map(|id| json!({ "id": a, "zone_id": id }))).collect();
    let group_links: Vec<Value> = GROUP_JOBS.iter().filter_map(|(g, j)| job_of(j).map(|id| json!({ "id": g, "work_item_id": id }))).collect();
    let duty_areas = json!([
        { "id": "duty_gate", "name": "Main Gate", "color": "#064e3b", "shiftTime": "20:00", "workersNeeded": 1, "roster": [103, 106, 109, 112, 115], "lastRotationIndex": 0 },
        { "id": "duty_hall", "name": "Hall Site Watch", "color": "#b45309", "shiftTime": "21:00", "workersNeeded": 1, "roster": [102, 110, 113], "lastRotationIndex": 0 },
    ]);
    let duty_schedules: Vec<Value> = (-6..=0)
        .map(|d: i64| {
            let gate = [103, 106, 109, 112, 115][((d + 6) % 5) as usize];
            let hall = [102, 110, 113][((d + 6) % 3) as usize];
            json!({ "dateStr": day(d), "assignments": [{ "aNumber": gate, "areaId": "duty_gate" }, { "aNumber": hall, "areaId": "duty_hall" }] })
        })
        .collect();
    let att = |i: usize, a: &str, name: &str, d: i64, old: &str, new: &str, reason: &str| {
        json!({ "id": format!("att_sample_{i}"), "aNumber": a, "workerName": name, "date": day(d), "oldStatus": old, "newStatus": new, "reason": reason, "timestamp": ms(d, 7) })
    };
    let attendance_log = json!([
        att(1, "104", "Paolo Garcia", -9, "Present", "Absent", "Fever"),
        att(2, "104", "Paolo Garcia", -7, "Absent", "Present", ""),
        att(3, "114", "Benigno Torres", -5, "Present", "Absent", "Family matter"),
        att(4, "107", "Felipe Bautista", -3, "Present", "Passes", "Home visit"),
        att(5, "104", "Paolo Garcia", -1, "Present", "Absent", "Fever"),
        att(6, "114", "Benigno Torres", 0, "Present", "Absent", "Family matter"),
    ]);
    let assignment_log = json!([
        { "id": "asg_sample_1", "aNumber": "110", "workerName": "Rogelio Castillo", "date": day(-4), "field": "projectGroup", "oldValue": "Hall | Masons", "newValue": "Terrace | Concrete", "reason": "Concrete pour", "timestamp": ms(-4, 8), "by": "Office" },
        { "id": "asg_sample_2", "aNumber": "115", "workerName": "Gregorio Lim", "date": day(-2), "field": "area", "oldValue": "Utility Area", "newValue": "Garden Area", "reason": "Gazebo planting", "timestamp": ms(-2, 8), "by": "Office" },
    ]);
    let ot = |id: &str, d: i64, a: &str, title: &str, hours: f64, rate: f64| {
        json!({ "id": id, "date": day(d), "volunteerId": a, "title": title, "otArea": title, "otType": "regular", "hours": hours, "hourlyRate": rate, "computedPay": hours * rate, "status": "approved" })
    };
    let ot_entries = json!([
        ot("ot_sample_1", -3, "101", "Hall slab formwork", 2.0, 78.0),
        ot("ot_sample_2", -3, "102", "Hall slab formwork", 2.0, 79.0),
        ot("ot_sample_3", -2, "108", "Terrace rebar", 3.0, 78.0),
        ot("ot_sample_4", -1, "113", "Filtration valve repair", 1.5, 79.0),
        { "id": "ot_sample_5", "date": day(-1), "title": "Gazebo roofing", "otType": "regular", "hours": 2.0, "hourlyRate": 80.0, "computedPay": 480.0, "totalCount": 3,
          "volunteers": [{ "aNumber": "105", "area": "Garden Area" }, { "aNumber": "106", "area": "Garden Area" }, { "aNumber": "115", "area": "Garden Area" }] },
    ]);
    let visit = |id: &str, d: i64, a: &str, name: &str, minister: &str, status: &str| {
        let mut v = json!({ "id": id, "date": day(d), "aNumber": a, "workerId": a, "workerName": name, "ministerName": minister, "status": status, "notes": "", "createdAt": Utc::now().to_rfc3339() });
        if status == "completed" {
            v["completedAt"] = json!(Utc::now().to_rfc3339());
        }
        v
    };
    let visitations = json!([
        visit("visit_sample_1", -10, "101", "Ramon Dela Cruz", "Bro. Ernesto Cruz", "completed"),
        visit("visit_sample_2", -6, "105", "Andres Villanueva", "Bro. Samuel Reyes", "completed"),
        visit("visit_sample_3", -2, "108", "Lorenzo Aquino", "Bro. Ernesto Cruz", "completed"),
        visit("visit_sample_4", 2, "104", "Paolo Garcia", "Bro. Ernesto Cruz", "scheduled"),
        visit("visit_sample_5", 5, "114", "Benigno Torres", "Bro. Samuel Reyes", "scheduled"),
        visit("visit_sample_6", 9, "111", "Teodoro Navarro", "Bro. Samuel Reyes", "scheduled"),
    ]);
    let soon = (today + Duration::days(4)).format("%Y-%m-%d").to_string();
    json!({
        "version": 11,
        "workers": workers,
        "area_zones": area_zones,
        "group_links": group_links,
        "project_groups_meta": { "Hall | Masons": "Hall Area", "Garden | Builders": "Garden Area", "Terrace | Concrete": "Terrace Area", "Woodland | Carpenters": "Workshop Area", "Utility | Plumbers": "Utility Area" },
        "signatories": [
            { "id": "sig_sample_1", "name": "Engr. Carlos Medina", "projectGroups": ["Hall | Masons", "Terrace | Concrete", "Utility | Plumbers"], "signatureDate": day(-30) },
            { "id": "sig_sample_2", "name": "Arch. Liza Herrera", "projectGroups": ["Garden | Builders", "Woodland | Carpenters"], "signatureDate": day(-30) },
        ],
        "duty_areas": duty_areas,
        "duty_schedules": duty_schedules,
        "attendance_log": attendance_log,
        "assignment_log": assignment_log,
        "ot_entries": ot_entries,
        "ministers": [{ "id": "min_sample_1", "name": "Bro. Ernesto Cruz" }, { "id": "min_sample_2", "name": "Bro. Samuel Reyes" }],
        "visitation_groups": [
            { "id": "vg_sample_1", "name": "Dalaw Group 1", "housing": "Staff House A", "prayerDay": "Wednesday", "prayerTime": "19:00", "createdAt": Utc::now().to_rfc3339() },
            { "id": "vg_sample_2", "name": "Dalaw Group 2", "housing": "Staff House B", "prayerDay": "Thursday", "prayerTime": "19:00", "createdAt": Utc::now().to_rfc3339() },
        ],
        "visitations": visitations,
        "custom_datalists": { "woodlandShops": ["Carpenter Shop", "Painter Shop", "Tinsmith Shop"], "trades": ["Mason", "Carpenter", "Electrician", "Plumber", "Painter", "Steelman", "Laborer"] },
        "health_events": [
            { "id": "he_sample_1", "workerId": "104", "diagnosis": "Influenza", "dayZero": day(-1), "status": "active", "createdAt": Utc::now().to_rfc3339() },
            { "id": "he_sample_2", "workerId": "109", "diagnosis": "Sprained ankle", "dayZero": day(-20), "status": "cleared", "clearanceDate": day(-12), "createdAt": Utc::now().to_rfc3339() },
        ],
        "volunteer_health_logs": [
            { "logId": "hl_sample_1", "id": "hl_sample_1", "volunteerId": "104", "healthEventId": "he_sample_1", "logDateTime": format!("{}T09:00", day(-1)), "chiefComplaint": "Fever and body aches", "diagnosis": "Influenza", "physicianRecommendation": "Rest at home for 3 days", "advisedToStayHome": true, "createdAt": Utc::now().to_rfc3339() },
        ],
        "volunteer_vaccines": [
            { "id": "vx_sample_1", "volunteerId": "102", "vaccineName": "Tetanus", "dateAdministered": day(-300), "boosterDueDate": soon },
            { "id": "vx_sample_2", "volunteerId": "108", "vaccineName": "Flu", "dateAdministered": day(-60) },
        ],
        "volunteer_medications": [
            { "id": "med_sample_1", "volunteerId": "102", "medicationName": "Amlodipine 5 mg", "dosage": "Once a day", "startDate": day(-200) },
        ],
        "contact_traces": [
            { "id": "ct_sample_1", "contactWorkerId": "103", "healthEventId": "he_sample_1", "contactType": "workplace", "status": "monitoring" },
        ],
    })
}

/// Replaces the workforce with the sample (a snapshot is taken first, so the
/// previous workforce can be restored from Settings → Workforce data).
pub async fn load(db: &DatabaseConnection) -> AppResult<()> {
    workforce::snapshot(db, "before loading sample data").await?;
    let zones: Vec<(String, String)> = crate::entities::spatial_area::Entity::find().all(db).await?.into_iter().map(|a| (a.name, a.id.to_string())).collect();
    let jobs: Vec<(String, String)> = crate::entities::work_item::Entity::find().all(db).await?.into_iter().map(|w| (w.title, w.id.to_string())).collect();
    let find = |list: &Vec<(String, String)>, name: &str| list.iter().find(|(n, _)| n == name).map(|(_, id)| id.clone());
    let raw = backup(|z| find(&zones, z), |j| find(&jobs, j));
    db.execute_unprepared("DELETE FROM workforce_records; DELETE FROM workers").await?;
    workforce::import(db, workforce::normalize(raw)?, shared::api::ImportMode::Merge, true).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn the_sample_is_a_complete_service5_backup_linked_to_the_estate() {
        let raw = backup(|z| Some(format!("zone:{z}")), |j| Some(format!("job:{j}")));
        let n = workforce::normalize(raw).unwrap();
        assert_eq!(n.workers.len(), PEOPLE.len());
        for c in ["area_zones", "group_links", "signatories", "duty_areas", "duty_schedules", "attendance_log", "ot_entries", "visitations", "health_events", "volunteer_vaccines"] {
            assert!(n.collections.get(c).is_some_and(|v| !v.is_empty()), "{c} is empty");
        }
        let links = &n.collections["group_links"];
        assert!(links.iter().any(|(_, v)| v["work_item_id"] == "job:Parking Conversion to Multi-Purpose Hall"));
        // Dates in the sample are real calendar dates.
        assert!(n.collections["duty_schedules"].iter().all(|(_, v)| NaiveDate::parse_from_str(v["dateStr"].as_str().unwrap(), "%Y-%m-%d").is_ok()));
    }
}
