//! Pages.

pub mod access;
pub mod admin_map_studio;
pub mod auth;
pub mod agenda_view;
pub mod archive;
pub mod assignments;
pub mod home;
pub mod people;
pub mod clinic;
pub mod encoder;
pub mod personnel;
pub mod profile;
pub mod print_export;
pub mod workforce_more;
pub mod workforce_ops;
pub mod workforce_print;
pub mod workforce_sheets;
pub mod workforce_pages;
pub mod workforce_extra;
pub mod project_editor;
pub mod settings;
pub mod staff;
pub mod sync_review;
pub mod vip;

use shared::Permission;

use crate::state::AppState;

/// The tabs inside each tabbed page. The page's tab bar and the sidebar's
/// sub-items list the same tabs and share the open one (`ui::PAGE_TABS`).
/// Named after the Service 5 pages they replace.
pub const WORKFORCE_TABS: &[&str] =
    &["Dashboard", "Departments", "ID Cards", "Project Groups", "Visitations", "Reassign Volunteers", "Attendance Gen", "Update Forms", "Archived Personnel", "Areas & zones"];
pub const OPERATIONS_TABS: &[&str] = &[
    "Overnight Duty", "Duty History", "Encoder", "Monthly Attendance", "OT Calculator", "OT Entries", "Org Chart", "History Log", "Dependability", "Signatories",
    "Bulk Update", "Pick Lists", "Print",
];
pub const CLINIC_TABS: &[&str] = &["Health episodes", "Clearance", "Clinic visits", "Vaccines", "Medications", "Contact tracing", "Audit trail"];

pub fn settings_tabs(app: AppState) -> Vec<&'static str> {
    let mut t = vec!["General", "Crew", "Issues", "Schedule"];
    // Service 5's Admin & tools: Backup DB, Restore DB, Daily Snapshot.
    if app.can(Permission::BackupWorkforce) {
        t.extend(["Backup DB", "Restore DB", "Daily Snapshot"]);
    }
    t.push("Estate data");
    t
}

/// The permission (Access page) that opens a Workforce or Operations tab.
pub fn tab_permission(tab: &str) -> Permission {
    match tab {
        "Dashboard" | "Departments" | "Org Chart" | "History Log" | "Dependability" => Permission::ViewWorkforceReports,
        "ID Cards" | "Attendance Gen" | "Update Forms" | "Print" => Permission::PrintWorkforce,
        "Visitations" => Permission::ManageVisitations,
        "Overnight Duty" | "Duty History" => Permission::ManageDuty,
        "OT Calculator" | "OT Entries" => Permission::ManageOvertime,
        "Signatories" => Permission::ManageSignatories,
        _ => Permission::ManageWorkforce,
    }
}

/// Tabs for a page key ("" = the page has none).
pub fn page_tabs(app: AppState, page: &str) -> Vec<&'static str> {
    match page {
        "workforce" => WORKFORCE_TABS.iter().copied().filter(|t| app.can(tab_permission(t))).collect(),
        "operations" => OPERATIONS_TABS.iter().copied().filter(|t| app.can(tab_permission(t))).collect(),
        "clinic" => CLINIC_TABS.to_vec(),
        "settings" => settings_tabs(app),
        _ => vec![],
    }
}
