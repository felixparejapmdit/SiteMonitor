//! Pages.

pub mod access;
pub mod admin_map_studio;
pub mod auth;
pub mod agenda_view;
pub mod archive;
pub mod home;
pub mod people;
pub mod clinic;
pub mod personnel;
pub mod workforce_more;
pub mod workforce_ops;
pub mod workforce_print;
pub mod workforce_extra;
pub mod project_editor;
pub mod settings;
pub mod staff;
pub mod vip;

use shared::Permission;

use crate::state::AppState;

/// The tabs inside each tabbed page. The page's tab bar and the sidebar's
/// sub-items list the same tabs and share the open one (`ui::PAGE_TABS`).
pub const WORKFORCE_TABS: &[&str] = &["Dashboard", "Groups", "Areas & zones", "Departments", "Org chart", "Dependability", "History"];
pub const OPERATIONS_TABS: &[&str] =
    &["Overnight duty", "Duty history", "Tomorrow", "Attendance", "Overtime", "Visitations", "Visitation groups", "Signatories", "Bulk update", "Pick lists", "Print"];
pub const CLINIC_TABS: &[&str] = &["Health episodes", "Clearance", "Clinic visits", "Vaccines", "Medications", "Contact tracing", "Audit trail"];

pub fn settings_tabs(app: AppState) -> Vec<&'static str> {
    let mut t = vec!["General", "Crew", "Issues", "Schedule"];
    if app.can(Permission::ManageWorkforce) {
        t.push("Workforce data");
    }
    t.push("Estate data");
    if app.can(Permission::ManageWorkforce) && app.can(Permission::ManageClinic) {
        t.push("Backup & restore");
    }
    t
}

/// Tabs for a page key ("" = the page has none).
pub fn page_tabs(app: AppState, page: &str) -> Vec<&'static str> {
    match page {
        "workforce" => WORKFORCE_TABS.to_vec(),
        "operations" => OPERATIONS_TABS.to_vec(),
        "clinic" => CLINIC_TABS.to_vec(),
        "settings" => settings_tabs(app),
        _ => vec![],
    }
}
