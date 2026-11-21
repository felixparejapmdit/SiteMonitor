//! Service 5's multi-master sync (`SyncMergeService` in js/backup.js): before a
//! backup is merged in, compare it with what is here — new people, new
//! attendance, newer sections (field updates) and true collisions (both sides
//! changed the same section within a minute) — then merge each person section
//! by section, last write wins, with the user's choice for each collision
//! (keep local, accept incoming, smart merge).

use std::collections::{HashMap, HashSet};

use chrono::DateTime;
use serde_json::{json, Map, Value};

/// Two edits this close together are a collision, not an update (Service 5: 60 s).
const EPSILON_MS: i64 = 60_000;
/// History lists keep their latest entries (Service 5: 500).
const KEEP: usize = 500;
const SECTIONS: [(&str, &str, &str); 3] = [
    ("personalInfo", "pi", "Personal Information"),
    ("assignment", "asg", "Work Station & Assignment"),
    ("healthAndClearance", "hc", "Health & Medical Clearance"),
];

fn ms(v: &Value) -> i64 {
    match v {
        Value::Number(n) => n.as_i64().unwrap_or(0),
        Value::String(s) => DateTime::parse_from_rfc3339(s).map(|d| d.timestamp_millis()).or_else(|_| s.parse()).unwrap_or(0),
        _ => 0,
    }
}

fn text(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn obj(v: &Value, k: &str) -> Value {
    v.get(k).filter(|x| x.is_object()).cloned().unwrap_or_else(|| json!({}))
}

/// `{...a, ...b}`
fn spread(a: &Value, b: &Value) -> Value {
    let mut out = a.as_object().cloned().unwrap_or_default();
    for (k, v) in b.as_object().into_iter().flatten() {
        out.insert(k.clone(), v.clone());
    }
    Value::Object(out)
}

/// A section's time: its own `updatedAt`, else the person's.
fn section_time(w: &Value, section: &str) -> i64 {
    let own = ms(&w[section]["updatedAt"]);
    if own > 0 { own } else { ms(&w["updatedAt"]) }
}

fn id_of(w: &Value) -> String {
    [text(w, "workerId"), text(w, "aNumber"), text(w, "name")].into_iter().find(|x| !x.is_empty()).unwrap_or_default()
}

fn att_key(e: &Value) -> String {
    let c = text(e, "compositeId");
    if !c.is_empty() {
        return c;
    }
    let shift = text(e, "shift");
    format!("{}_{}", text(e, "date"), if shift.is_empty() { "Day".into() } else { shift })
}

fn att_status(e: &Value) -> String {
    let s = text(e, "status");
    if s.is_empty() { text(e, "newStatus") } else { s }
}

fn att_time(e: &Value) -> i64 {
    let t = ms(&e["updatedAt"]);
    if t > 0 { t } else { ms(&e["timestamp"]) }
}

/// Finds the local person an incoming one is: same workerId, else same A#, else same name.
pub fn match_local<'a>(local: &'a [Value], iw: &Value) -> Option<&'a Value> {
    let wid = text(iw, "workerId");
    let a = text(iw, "aNumber");
    let name = text(iw, "name").to_lowercase();
    local
        .iter()
        .find(|cw| !wid.is_empty() && text(cw, "workerId") == wid)
        .or_else(|| local.iter().find(|cw| !a.is_empty() && a != "0" && text(cw, "aNumber") == a))
        .or_else(|| local.iter().find(|cw| !name.is_empty() && text(cw, "name").to_lowercase() == name))
}

/// Service 5's `compareWorker`: (field updates, new attendance, collisions).
fn compare(cw: &Value, iw: &Value) -> (usize, usize, Vec<Value>) {
    let wid = id_of(cw);
    let wname = if text(cw, "name").is_empty() { text(iw, "name") } else { text(cw, "name") };
    let (mut updates, mut new_att, mut conflicts) = (0, 0, vec![]);
    for (section, short, label) in SECTIONS {
        if obj(cw, section) == obj(iw, section) {
            continue;
        }
        let (ct, it) = (section_time(cw, section), section_time(iw, section));
        if (it - ct).abs() <= EPSILON_MS && ct > 0 && it > 0 {
            conflicts.push(json!({
                "id": format!("conflict_{short}_{wid}"), "type": "section", "section": section, "sectionLabel": label,
                "workerId": wid, "workerName": wname, "local": obj(cw, section), "incoming": obj(iw, section),
                "localTime": cw[section].get("updatedAt").cloned().unwrap_or(cw["updatedAt"].clone()),
                "incomingTime": iw[section].get("updatedAt").cloned().unwrap_or(iw["updatedAt"].clone()),
            }));
        } else if it > ct {
            updates += 1;
        }
    }
    let local_att: HashMap<String, &Value> = cw["attendanceHistory"].as_array().into_iter().flatten().filter(|e| e.is_object()).map(|e| (att_key(e), e)).collect();
    for ie in iw["attendanceHistory"].as_array().into_iter().flatten().filter(|e| e.is_object()) {
        let k = att_key(ie);
        match local_att.get(&k) {
            None => new_att += 1,
            Some(ce) if att_status(ce) != att_status(ie) => {
                let (ct, it) = (att_time(ce), att_time(ie));
                if (it - ct).abs() <= EPSILON_MS {
                    let shift = text(ie, "shift");
                    let date = text(ie, "date");
                    conflicts.push(json!({
                        "id": format!("conflict_att_{wid}_{k}"), "type": "attendance",
                        "sectionLabel": format!("Attendance ({} - {})", if date.is_empty() { "Shift".into() } else { date.clone() }, if shift.is_empty() { "Day".into() } else { shift }),
                        "workerId": wid, "workerName": wname, "compositeId": k, "date": date, "local": ce, "incoming": ie,
                        "localTime": ce.get("updatedAt").or(ce.get("timestamp")).cloned().unwrap_or(Value::Null),
                        "incomingTime": ie.get("updatedAt").or(ie.get("timestamp")).cloned().unwrap_or(Value::Null),
                    }));
                } else if it > ct {
                    updates += 1;
                }
            }
            _ => {}
        }
    }
    (updates, new_att, conflicts)
}

/// Service 5's `analyzeDiff` for the people in a backup.
pub fn analyze(local: &[Value], incoming: &[Value], source_device: &str, exported_at: &str) -> Value {
    let (mut new_workers, mut merged, mut updates, mut new_att, mut conflicts) = (0, 0, 0, 0, vec![]);
    for iw in incoming {
        match match_local(local, iw) {
            None => new_workers += 1,
            Some(cw) => {
                merged += 1;
                let (u, a, c) = compare(cw, iw);
                updates += u;
                new_att += a;
                conflicts.extend(c);
            }
        }
    }
    json!({
        "sourceDeviceId": if source_device.is_empty() { "External Workstation" } else { source_device },
        "exportTimestamp": exported_at,
        "summary": {
            "newWorkersCount": new_workers, "retainedWorkersCount": local.len(), "mergedWorkersCount": merged,
            "newAttendanceCount": new_att, "sectionUpdatesCount": updates, "conflictsCount": conflicts.len(),
        },
        "conflicts": conflicts,
    })
}

fn pick_section(cw: &Value, iw: &Value, section: &str, res: Option<&str>) -> Value {
    let (c, i) = (obj(cw, section), obj(iw, section));
    match res {
        Some("keep_local") => c,
        Some("accept_incoming") => i,
        Some("smart_merge") => spread(&c, &i),
        _ => {
            if section_time(iw, section) > section_time(cw, section) { spread(&c, &i) } else { spread(&i, &c) }
        }
    }
}

fn merge_attendance(cw: &Value, iw: &Value, wid: &str, res: &Map<String, Value>) -> Value {
    let mut order: Vec<String> = vec![];
    let mut map: HashMap<String, Value> = HashMap::new();
    for e in cw["attendanceHistory"].as_array().into_iter().flatten().filter(|e| e.is_object()) {
        let k = att_key(e);
        if map.insert(k.clone(), e.clone()).is_none() {
            order.push(k);
        }
    }
    for ie in iw["attendanceHistory"].as_array().into_iter().flatten().filter(|e| e.is_object()) {
        let k = att_key(ie);
        let merged = match map.get(&k) {
            None => {
                order.push(k.clone());
                ie.clone()
            }
            Some(ce) => match res.get(&format!("conflict_att_{wid}_{k}")).and_then(Value::as_str) {
                Some("keep_local") => ce.clone(),
                Some("accept_incoming") => ie.clone(),
                _ => if att_time(ie) >= att_time(ce) { spread(ce, ie) } else { spread(ie, ce) },
            },
        };
        map.insert(k, merged);
    }
    let mut out: Vec<Value> = order.into_iter().filter_map(|k| map.remove(&k)).collect();
    out.sort_by_key(|e| { let t = ms(&e["timestamp"]); if t > 0 { t } else { ms(&e["date"]) } });
    let skip = out.len().saturating_sub(KEEP);
    Value::Array(out.split_off(skip))
}

fn merge_assignment_history(cw: &Value, iw: &Value) -> Value {
    let key = |e: &Value| {
        let r = [text(e, "recordId"), text(e, "id")].into_iter().find(|x| !x.is_empty());
        r.unwrap_or_else(|| format!("{}_{}_{}_{}", text(e, "date"), text(e, "timestamp"), text(e, "field"), text(e, "newValue")))
    };
    let mut seen = HashSet::new();
    let mut out: Vec<Value> = vec![];
    for e in cw["assignmentHistory"].as_array().into_iter().flatten().chain(iw["assignmentHistory"].as_array().into_iter().flatten()).filter(|e| e.is_object()) {
        if seen.insert(key(e)) {
            out.push(e.clone());
        }
    }
    out.sort_by_key(|e| ms(&e["timestamp"]));
    let skip = out.len().saturating_sub(KEEP);
    Value::Array(out.split_off(skip))
}

fn union(cw: &Value, iw: &Value, k: &str) -> Value {
    let mut out: Vec<Value> = vec![];
    for v in cw[k].as_array().into_iter().flatten().chain(iw[k].as_array().into_iter().flatten()) {
        let empty = v.is_null() || v.as_str() == Some("");
        if !empty && !out.contains(v) {
            out.push(v.clone());
        }
    }
    Value::Array(out)
}

/// Service 5's `mergeWorkerPair`: one person from here (`cw`) and from the file (`iw`).
pub fn merge_pair(cw: &Value, iw: &Value, res: &Map<String, Value>, now: &str) -> Value {
    let mut m = cw.as_object().cloned().unwrap_or_default();
    let wid = id_of(cw);
    let r = |short: &str| res.get(&format!("conflict_{short}_{wid}")).and_then(Value::as_str);

    // 1. Personal information, and the top-level fields it feeds.
    let pi = pick_section(cw, iw, "personalInfo", r("pi"));
    for (from, to) in [("fullName", "name"), ("contact", "phone"), ("address", "address"), ("birthday", "birthday"), ("civilStatus", "civilStatus"), ("spouseName", "spouseName"), ("emergencyContact", "emergencyContact"), ("emergencyPhone", "emergencyPhone"), ("department", "department")] {
        if let Some(v) = pi.get(from).filter(|v| !v.is_null() && v.as_str() != Some("")) {
            m.insert(to.into(), v.clone());
        }
    }
    for k in ["subDepartment", "hourlyRate"] {
        if let Some(v) = pi.get(k).filter(|v| !v.is_null()) {
            m.insert(k.into(), v.clone());
        }
    }
    m.insert("personalInfo".into(), pi);

    // 2. Assignment, and the fields it feeds.
    let mut asg = pick_section(cw, iw, "assignment", r("asg"));
    for k in ["area", "trade", "projectGroup", "specificTask", "jOrder", "project"] {
        if let Some(v) = asg.get(k).filter(|v| !v.is_null()) {
            m.insert(k.into(), v.clone());
        }
    }

    // 2b. Daily status: the later status change wins; a tie keeps an absence over "Present".
    let st = |w: &Value| {
        let s = [text(w, "dailyStatus"), text(w, "status"), text(&w["assignment"], "dailyStatus")].into_iter().find(|x| !x.is_empty());
        s.unwrap_or_else(|| "Present".into())
    };
    let stime = |w: &Value| { let t = ms(&w["statusUpdatedAt"]); if t > 0 { t } else { ms(&w["updatedAt"]) } };
    let (ct, it) = (stime(cw), stime(iw));
    let (status, reason, when) = if it > ct {
        (st(iw), text(iw, "absentReason"), iw.get("statusUpdatedAt").cloned().unwrap_or(json!(now)))
    } else if ct > it {
        (st(cw), text(cw, "absentReason"), cw.get("statusUpdatedAt").cloned().unwrap_or(json!(now)))
    } else if st(iw) != "Present" && st(cw) == "Present" {
        (st(iw), text(iw, "absentReason"), iw.get("statusUpdatedAt").cloned().unwrap_or(json!(now)))
    } else if st(cw) != "Present" && st(iw) == "Present" {
        (st(cw), text(cw, "absentReason"), cw.get("statusUpdatedAt").cloned().unwrap_or(json!(now)))
    } else {
        let s = [text(iw, "dailyStatus"), text(cw, "dailyStatus")].into_iter().find(|x| !x.is_empty()).unwrap_or_else(|| "Present".into());
        let reason = [text(iw, "absentReason"), text(cw, "absentReason")].into_iter().find(|x| !x.is_empty()).unwrap_or_default();
        (s, reason, iw.get("statusUpdatedAt").or(cw.get("statusUpdatedAt")).cloned().unwrap_or(json!(now)))
    };
    let reason = if status == "Present" { String::new() } else { reason };
    m.insert("dailyStatus".into(), json!(status));
    m.insert("status".into(), json!(status));
    m.insert("absentReason".into(), json!(reason));
    m.insert("statusUpdatedAt".into(), when);
    asg["dailyStatus"] = json!(status);
    asg["absentReason"] = json!(reason);

    // Project role follows the newer assignment.
    let role = |w: &Value| [text(w, "projectRole"), text(&w["assignment"], "projectRole")].into_iter().find(|x| !x.is_empty()).unwrap_or_else(|| "Member".into());
    let pr = if section_time(iw, "assignment") > section_time(cw, "assignment") { role(iw) } else { role(cw) };
    m.insert("projectRole".into(), json!(pr));
    asg["projectRole"] = json!(pr);
    m.insert("assignment".into(), asg);

    // 3. Health and clearance.
    let hc = pick_section(cw, iw, "healthAndClearance", r("hc"));
    if let Some(n) = hc.get("medicalNotes").filter(|v| v.as_str().is_some_and(|s| !s.is_empty())) {
        m.insert("medical".into(), n.clone());
        m.insert("medicalNotes".into(), n.clone());
    }
    if let Some(b) = hc.get("bloodType").filter(|v| v.as_str().is_some_and(|s| !s.is_empty() && s != "UNKNOWN")) {
        m.insert("bloodType".into(), b.clone());
    }
    m.insert("healthAndClearance".into(), hc);

    // 4. A photo is never lost.
    let has = |w: &Map<String, Value>| ["photo", "photoUrl"].iter().any(|k| w.get(*k).and_then(Value::as_str).is_some_and(|s| !s.is_empty()));
    if !has(&m) {
        for k in ["photo", "photoUrl"] {
            if let Some(v) = cw.get(k) {
                m.insert(k.into(), v.clone());
            }
        }
    }

    // 5. Append-only histories, 6. set unions.
    m.insert("attendanceHistory".into(), merge_attendance(cw, iw, &wid, res));
    m.insert("assignmentHistory".into(), merge_assignment_history(cw, iw));
    m.insert("trades".into(), union(cw, iw, "trades"));
    m.insert("groups".into(), union(cw, iw, "groups"));
    m.insert("updatedAt".into(), json!(now));
    Value::Object(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(a: &str, phone: &str, at: &str) -> Value {
        json!({ "workerId": format!("w{a}"), "aNumber": a, "name": format!("P {a}"), "personalInfo": { "contact": phone, "updatedAt": at }, "updatedAt": at })
    }

    #[test]
    fn analysis_counts_new_people_updates_and_collisions() {
        let local = vec![w("1", "111", "2026-11-01T08:00:00Z"), w("2", "222", "2026-11-01T08:00:00Z")];
        let incoming = vec![
            w("1", "999", "2026-11-05T08:00:00Z"),    // newer: an update
            w("2", "333", "2026-11-01T08:00:30Z"),    // 30 s apart: a collision
            w("3", "444", "2026-11-01T08:00:00Z"),    // new person
        ];
        let a = analyze(&local, &incoming, "", "");
        assert_eq!(a["summary"]["newWorkersCount"], 1);
        assert_eq!(a["summary"]["sectionUpdatesCount"], 1);
        assert_eq!(a["summary"]["conflictsCount"], 1);
        assert_eq!(a["conflicts"][0]["id"], "conflict_pi_w2");
    }

    #[test]
    fn merge_follows_resolutions_and_last_write_wins() {
        let cw = w("2", "222", "2026-11-01T08:00:00Z");
        let iw = w("2", "333", "2026-11-01T08:00:30Z");
        let mut res = Map::new();
        res.insert("conflict_pi_w2".into(), json!("keep_local"));
        assert_eq!(merge_pair(&cw, &iw, &res, "now")["phone"], "222");
        res.insert("conflict_pi_w2".into(), json!("accept_incoming"));
        assert_eq!(merge_pair(&cw, &iw, &res, "now")["phone"], "333");
        // No choice: the newer section wins.
        assert_eq!(merge_pair(&cw, &iw, &Map::new(), "now")["phone"], "333");
    }

    #[test]
    fn merge_keeps_both_attendance_histories_and_absences_on_ties() {
        let cw = json!({ "workerId": "w1", "dailyStatus": "Absent", "absentReason": "Sick", "attendanceHistory": [{ "date": "2026-11-01", "status": "Present" }] });
        let iw = json!({ "workerId": "w1", "dailyStatus": "Present", "attendanceHistory": [{ "date": "2026-11-02", "status": "Absent" }] });
        let m = merge_pair(&cw, &iw, &Map::new(), "now");
        assert_eq!(m["attendanceHistory"].as_array().unwrap().len(), 2);
        assert_eq!((m["dailyStatus"].as_str(), m["absentReason"].as_str()), (Some("Absent"), Some("Sick")));
    }
}
