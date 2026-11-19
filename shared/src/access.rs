//! Access matrix: what each person may do.
//!
//! Every role starts from a default set of permissions; the Office Admin can
//! then grant extra permissions to, or take defaults away from, one person.
//! The effective set is `role defaults + granted - revoked`, and the server
//! checks it on every action.

use serde::{Deserialize, Serialize};

use uuid::Uuid;

use crate::models::{DecisionAlert, User, UserRole};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Permission {
    // Projects
    ManageProjects,
    UpdateProgress,
    SignOffProjects,
    // Photos
    UploadPhotos,
    ManagePhotoNotes,
    CommentOnPhotos,
    // Crew & equipment
    ManageCrew,
    OperateEquipment,
    ManageEquipment,
    /// Service 5 workforce records: people, their personal details, backups and imports.
    ManageWorkforce,
    /// Clinic ledger: health episodes, clinic visits, vaccines, contact tracing.
    ManageClinic,
    // Issues & decisions
    RaiseIssues,
    PublishDecisions,
    AnswerDecisions,
    // Schedule
    ManageSchedule,
    BookQuietDays,
    // Estate map
    ManageMap,
    // Administration
    ManagePeople,
    ManageAccess,
    ManageSettings,
}

use Permission::*;

/// Permissions grouped the way the access matrix shows them.
pub const GROUPS: &[(&str, &[Permission])] = &[
    ("Projects", &[ManageProjects, UpdateProgress, SignOffProjects]),
    ("Site photos", &[UploadPhotos, ManagePhotoNotes, CommentOnPhotos]),
    ("Crew & equipment", &[ManageCrew, OperateEquipment, ManageEquipment, ManageWorkforce, ManageClinic]),
    ("Issues & decisions", &[RaiseIssues, PublishDecisions, AnswerDecisions]),
    ("Schedule", &[ManageSchedule, BookQuietDays]),
    ("Estate map", &[ManageMap]),
    ("Administration", &[ManagePeople, ManageAccess, ManageSettings]),
];

impl Permission {
    pub const ALL: [Permission; 20] = [
        ManageProjects, UpdateProgress, SignOffProjects, UploadPhotos, ManagePhotoNotes, CommentOnPhotos, ManageCrew,
        OperateEquipment, ManageEquipment, ManageWorkforce, ManageClinic, RaiseIssues, PublishDecisions, AnswerDecisions, ManageSchedule, BookQuietDays,
        ManageMap, ManagePeople, ManageAccess, ManageSettings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ManageProjects => "Create & edit projects",
            UpdateProgress => "Update progress",
            SignOffProjects => "Sign off finished projects",
            UploadPhotos => "Upload site photos",
            ManagePhotoNotes => "Answer & resolve photo notes",
            CommentOnPhotos => "Pin notes on photos",
            ManageCrew => "Move crew between jobs",
            OperateEquipment => "Move & report equipment",
            ManageEquipment => "Add, edit & retire equipment",
            ManageWorkforce => "Manage workers & volunteers",
            ManageClinic => "Clinic ledger (medical records)",
            RaiseIssues => "Report issues",
            PublishDecisions => "Send decisions to the owner",
            AnswerDecisions => "Answer decisions",
            ManageSchedule => "Plan & reschedule work",
            BookQuietDays => "Book Quiet Days",
            ManageMap => "Edit the estate map",
            ManagePeople => "Add people & reset passwords",
            ManageAccess => "Change access (this matrix)",
            ManageSettings => "Change estate settings",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            ManageProjects => "Start new projects and change their name, dates, phases and team.",
            UpdateProgress => "Finish phases and routine tasks, with proof photos.",
            SignOffProjects => "Give the final approval that closes a finished project.",
            UploadPhotos => "Add progress photos from the field.",
            ManagePhotoNotes => "Reply to and close notes the owner pinned on photos.",
            CommentOnPhotos => "Pin questions on photos and acknowledge updates.",
            ManageCrew => "Set and change how many workers are on each job.",
            OperateEquipment => "Move machines, attach parts, report and clear breakdowns.",
            ManageEquipment => "Add new machines, rename them or retire them.",
            ManageWorkforce => "See and edit worker records (personal details included), import Service 5 backups and restore snapshots.",
            ManageClinic => "See and record health episodes, clinic visits, vaccines and contact tracing. Every change is logged.",
            RaiseIssues => "Send problems from the field to the office.",
            PublishDecisions => "Turn field reports into decisions for the owner.",
            AnswerDecisions => "Choose an option on decisions the office sends.",
            ManageSchedule => "Add schedule items and move work to another day.",
            BookQuietDays => "Block days when no noisy work may happen.",
            ManageMap => "Draw and reshape zones, upload and switch base maps.",
            ManagePeople => "Add, edit and deactivate accounts, and reset passwords.",
            ManageAccess => "Grant or remove any permission for anyone.",
            ManageSettings => "Change job times, pick lists and automations, and load sample data or clear the estate.",
        }
    }

    /// Permissions that can lock people out or expose everything: shown with a
    /// warning in the matrix and confirmed before saving.
    pub fn is_sensitive(self) -> bool {
        matches!(self, ManagePeople | ManageAccess | ManageSettings | ManageWorkforce | ManageClinic)
    }
}

/// What every person in a role can do unless the matrix says otherwise.
pub fn role_defaults(role: UserRole) -> &'static [Permission] {
    match role {
        UserRole::VipClient => &[SignOffProjects, CommentOnPhotos, AnswerDecisions, BookQuietDays],
        UserRole::SiteMonitor => &[UpdateProgress, UploadPhotos, ManagePhotoNotes, ManageCrew, OperateEquipment, RaiseIssues, ManageSchedule],
        UserRole::OfficeAdmin => &[
            ManageProjects, UpdateProgress, UploadPhotos, ManagePhotoNotes, ManageCrew, OperateEquipment, ManageEquipment,
            ManageWorkforce, ManageClinic, RaiseIssues, PublishDecisions, ManageSchedule, ManageMap, ManagePeople, ManageAccess,
            ManageSettings,
        ],
    }
}

/// One person's departures from their role's defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AccessOverrides {
    #[serde(default)]
    pub granted: Vec<Permission>,
    #[serde(default)]
    pub revoked: Vec<Permission>,
}

impl AccessOverrides {
    /// Drops no-op entries (granting a default, revoking a non-default) and
    /// sorts, so the same access always serialises the same way.
    pub fn normalized(&self, role: UserRole) -> Self {
        let defaults = role_defaults(role);
        let mut granted: Vec<Permission> = self.granted.iter().copied().filter(|p| !defaults.contains(p)).collect();
        let mut revoked: Vec<Permission> =
            self.revoked.iter().copied().filter(|p| defaults.contains(p) && !self.granted.contains(p)).collect();
        granted.sort();
        granted.dedup();
        revoked.sort();
        revoked.dedup();
        AccessOverrides { granted, revoked }
    }

    pub fn effective(&self, role: UserRole) -> Vec<Permission> {
        let mut out: Vec<Permission> = role_defaults(role).iter().copied().filter(|p| !self.revoked.contains(p)).collect();
        out.extend(self.granted.iter().copied());
        out.sort();
        out.dedup();
        out
    }

    /// Overrides that produce exactly `wanted` for this role.
    pub fn from_effective(role: UserRole, wanted: &[Permission]) -> Self {
        let defaults = role_defaults(role);
        AccessOverrides {
            granted: wanted.iter().copied().filter(|p| !defaults.contains(p)).collect(),
            revoked: defaults.iter().copied().filter(|p| !wanted.contains(p)).collect(),
        }
        .normalized(role)
    }

    pub fn is_default(&self) -> bool {
        self.granted.is_empty() && self.revoked.is_empty()
    }
}

/// Who may see a field report: the office sees every report, the owner sees
/// what was sent to them, and everyone else sees only the reports they wrote.
pub fn can_see_report(user: &User, report: &DecisionAlert) -> bool {
    if user.role == UserRole::VipClient {
        report.status.owner_can_see()
    } else if user.can(Permission::PublishDecisions) {
        true
    } else {
        report.reported_by == Some(user.id)
    }
}

/// Minimum rules for a password; `None` when it's acceptable.
pub fn password_problem(password: &str, email: &str) -> Option<&'static str> {
    let p = password.trim();
    if p.chars().count() < 10 {
        return Some("Use at least 10 characters.");
    }
    if p.chars().count() > 128 {
        return Some("Use at most 128 characters.");
    }
    if !email.is_empty() && p.eq_ignore_ascii_case(email.trim()) {
        return Some("Don't use your email address as your password.");
    }
    let first = p.chars().next().unwrap_or(' ');
    if p.chars().all(|c| c == first) {
        return Some("That password is too easy to guess.");
    }
    None
}

/// Whether this person may work in `area`. Site Monitors are limited to the
/// zones the office assigned them and every zone inside those; others aren't limited.
pub fn works_in(user: &User, area: Uuid, parent_of: impl Fn(Uuid) -> Option<Uuid>) -> bool {
    if user.role != UserRole::SiteMonitor {
        return true;
    }
    let mut cursor = Some(area);
    // The hop limit guards against a malformed parent loop.
    for _ in 0..64 {
        let Some(z) = cursor else { return false };
        if user.assigned_area_ids.contains(&z) {
            return true;
        }
        cursor = parent_of(z);
    }
    false
}

#[cfg(test)]
mod tests {
    #[test]
    fn monitors_work_only_in_their_zones_and_the_zones_inside() {
        use super::*;
        let (garden, rose, pool) = (Uuid::from_u128(1), Uuid::from_u128(2), Uuid::from_u128(3));
        let parent = |z: Uuid| (z == rose).then_some(garden);
        let mut u: User = serde_json::from_value(serde_json::json!({
            "id": Uuid::from_u128(9), "name": "Ana", "role": "SiteMonitor", "phone": "", "email": "a@x",
            "assigned_area_ids": [garden], "is_active": true
        }))
        .unwrap();
        assert!(works_in(&u, garden, parent));
        assert!(works_in(&u, rose, parent));
        assert!(!works_in(&u, pool, parent));
        u.assigned_area_ids.clear();
        assert!(!works_in(&u, garden, parent));
        u.role = UserRole::OfficeAdmin;
        assert!(works_in(&u, pool, parent));
    }

    use super::*;

    #[test]
    fn overrides_round_trip_through_effective() {
        let role = UserRole::SiteMonitor;
        let wanted = vec![UpdateProgress, UploadPhotos, ManageMap];
        let o = AccessOverrides::from_effective(role, &wanted);
        assert_eq!(o.granted, vec![ManageMap]);
        assert!(o.revoked.contains(&ManageCrew) && !o.revoked.contains(&UploadPhotos));
        let mut eff = o.effective(role);
        eff.sort();
        let mut w = wanted.clone();
        w.sort();
        assert_eq!(eff, w);
        assert!(AccessOverrides::from_effective(role, role_defaults(role)).is_default());
    }

    #[test]
    fn every_permission_is_in_one_group() {
        for p in Permission::ALL {
            assert_eq!(GROUPS.iter().filter(|(_, ps)| ps.contains(&p)).count(), 1, "{p:?}");
        }
    }

    #[test]
    fn site_team_only_sees_their_own_reports() {
        use crate::models::*;
        use chrono::Utc;
        use uuid::Uuid;
        let person = |role: UserRole| {
            let access = AccessOverrides::default();
            User {
                id: Uuid::new_v4(),
                name: "x".into(),
                role,
                phone: String::new(),
                email: String::new(),
                assigned_area_ids: vec![],
                is_active: true,
                permissions: access.effective(role),
                access,
                must_change_password: false,
                last_sign_in_at: None,
            }
        };
        let (ana, jun, office, owner) =
            (person(UserRole::SiteMonitor), person(UserRole::SiteMonitor), person(UserRole::OfficeAdmin), person(UserRole::VipClient));
        let report = |by: Option<Uuid>, status: DecisionStatus| DecisionAlert {
            id: Uuid::new_v4(),
            work_item_id: Uuid::new_v4(),
            area_id: Uuid::new_v4(),
            urgency: UrgencyLevel::Standard,
            category: AlertCategory::MaterialDelay,
            title: "t".into(),
            issue_description: "d".into(),
            mitigation_action_taken: String::new(),
            option_a_label: None,
            option_b_label: None,
            status,
            created_at: Utc::now(),
            resolved_at: None,
            photo_url: None,
            reported_by: by,
            office_note: None,
        };
        let anas = report(Some(ana.id), DecisionStatus::OfficeApproved);
        assert!(can_see_report(&ana, &anas));
        assert!(!can_see_report(&jun, &anas), "another site monitor");
        assert!(can_see_report(&office, &anas));
        assert!(!can_see_report(&owner, &anas), "office decisions stay with the office");
        // Even once it's with the owner, other site staff don't see it; reports with no recorded author neither.
        assert!(!can_see_report(&jun, &report(Some(ana.id), DecisionStatus::PublishedPendingClient)));
        assert!(!can_see_report(&jun, &report(None, DecisionStatus::DraftInAdminQueue)));
        assert!(can_see_report(&owner, &report(Some(ana.id), DecisionStatus::PublishedPendingClient)));
    }

    #[test]
    fn password_rules() {
        assert!(password_problem("short", "").is_some());
        assert!(password_problem("aaaaaaaaaaaa", "").is_some());
        assert!(password_problem("ana@estate.ph", "ana@estate.ph").is_some());
        assert!(password_problem("correct horse battery", "ana@estate.ph").is_none());
    }
}
