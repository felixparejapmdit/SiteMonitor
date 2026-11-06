//! SeaORM entity definitions — one module per table.
pub mod asset;
pub mod asset_transfer_log;
pub mod base_map_version;
pub mod decision_alert;
pub mod equipment_reservation;
pub mod estate_agenda_item;
pub mod executive_handover_report;
pub mod manpower_ledger_entry;
pub mod notification;
pub mod photo_call_out;
pub mod project_phase;
pub mod site_photo;
pub mod spatial_area;
pub mod timeline_event_record;
pub mod user;
pub mod work_item;

pub mod prelude {
    pub use super::asset::Entity as Assets;
    pub use super::asset_transfer_log::Entity as AssetTransferLogs;
    pub use super::base_map_version::Entity as BaseMapVersions;
    pub use super::decision_alert::Entity as DecisionAlerts;
    pub use super::equipment_reservation::Entity as EquipmentReservations;
    pub use super::estate_agenda_item::Entity as EstateAgendaItems;
    pub use super::executive_handover_report::Entity as ExecutiveHandoverReports;
    pub use super::manpower_ledger_entry::Entity as ManpowerLedgerEntries;
    pub use super::notification::Entity as Notifications;
    pub use super::photo_call_out::Entity as PhotoCallOuts;
    pub use super::project_phase::Entity as ProjectPhases;
    pub use super::site_photo::Entity as SitePhotos;
    pub use super::spatial_area::Entity as SpatialAreas;
    pub use super::timeline_event_record::Entity as TimelineEventRecords;
    pub use super::user::Entity as Users;
    pub use super::work_item::Entity as WorkItems;
}
