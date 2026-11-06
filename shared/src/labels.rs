//! Plain-language wording for every enum, so screens never show raw
//! programmer names like `PendingSignOff` to the people using the app.

use crate::models::*;

/// Colour family of a status pill (maps onto the pastel Tailwind tokens).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Green,
    Amber,
    Red,
    Gray,
    Blue,
}

impl Tone {
    /// CSS class pair defined in `frontend/tailwind.config.js`.
    pub fn pill_class(self) -> &'static str {
        match self {
            Tone::Green => "bg-tag-green-bg text-tag-green-text",
            Tone::Amber => "bg-tag-amber-bg text-tag-amber-text",
            Tone::Red => "bg-tag-red-bg text-tag-red-text",
            Tone::Gray => "bg-tag-gray-bg text-tag-gray-text",
            Tone::Blue => "bg-tag-blue-bg text-tag-blue-text",
        }
    }
}

impl UserRole {
    pub const ALL: [UserRole; 3] = [UserRole::VipClient, UserRole::OfficeAdmin, UserRole::SiteMonitor];
    pub fn label(self) -> &'static str {
        match self {
            UserRole::VipClient => "Estate Owner",
            UserRole::OfficeAdmin => "Office Admin",
            UserRole::SiteMonitor => "Site Monitor",
        }
    }
    pub fn is_staff(self) -> bool {
        !matches!(self, UserRole::VipClient)
    }
}

impl ZoneClass {
    pub const ALL: [ZoneClass; 6] = [
        ZoneClass::MajorResidential,
        ZoneClass::MinorResidential,
        ZoneClass::MultiPurposeFacility,
        ZoneClass::Recreational,
        ZoneClass::UtilityInfrastructure,
        ZoneClass::ContractorOperations,
    ];
    pub fn label(self) -> &'static str {
        match self {
            ZoneClass::MajorResidential => "Main Residence",
            ZoneClass::MinorResidential => "Guest Residence",
            ZoneClass::MultiPurposeFacility => "Multi-Purpose Facility",
            ZoneClass::Recreational => "Gardens & Pools",
            ZoneClass::UtilityInfrastructure => "Utilities",
            ZoneClass::ContractorOperations => "Contractor Yard",
        }
    }
    /// Front-of-House zones are what the owner sees by default.
    pub fn is_front_of_house(self) -> bool {
        matches!(
            self,
            ZoneClass::MajorResidential
                | ZoneClass::MinorResidential
                | ZoneClass::MultiPurposeFacility
                | ZoneClass::Recreational
        )
    }
}

impl AreaStatus {
    pub fn label(self) -> &'static str {
        match self {
            AreaStatus::Optimal => "All good",
            AreaStatus::ActiveWork => "Work in progress",
            AreaStatus::AttentionRequired => "Needs attention",
            AreaStatus::CriticalAlert => "Urgent issue",
            AreaStatus::Archived => "Archived",
        }
    }
    pub fn tone(self) -> Tone {
        match self {
            AreaStatus::Optimal => Tone::Green,
            AreaStatus::ActiveWork => Tone::Blue,
            AreaStatus::AttentionRequired => Tone::Amber,
            AreaStatus::CriticalAlert => Tone::Red,
            AreaStatus::Archived => Tone::Gray,
        }
    }
}

impl WorkStream {
    pub const ALL: [WorkStream; 3] = [WorkStream::CapitalProject, WorkStream::RoutineMaintenance, WorkStream::ReactiveRepair];
    pub fn label(self) -> &'static str {
        match self {
            WorkStream::CapitalProject => "Construction Project",
            WorkStream::RoutineMaintenance => "Routine Upkeep",
            WorkStream::ReactiveRepair => "Repair",
        }
    }
}

impl WorkStatus {
    pub fn label(self) -> &'static str {
        match self {
            WorkStatus::Planning => "Planning",
            WorkStatus::InProgress => "In progress",
            WorkStatus::Halted => "Paused",
            WorkStatus::PendingSignOff => "Ready for your sign-off",
            WorkStatus::CompletedAndArchived => "Completed",
        }
    }
    pub fn staff_label(self) -> &'static str {
        match self {
            WorkStatus::PendingSignOff => "Waiting for owner sign-off",
            other => other.label(),
        }
    }
    pub fn tone(self) -> Tone {
        match self {
            WorkStatus::Planning => Tone::Gray,
            WorkStatus::InProgress => Tone::Green,
            WorkStatus::Halted => Tone::Red,
            WorkStatus::PendingSignOff => Tone::Amber,
            WorkStatus::CompletedAndArchived => Tone::Gray,
        }
    }
    pub fn is_active(self) -> bool {
        matches!(self, WorkStatus::Planning | WorkStatus::InProgress | WorkStatus::Halted | WorkStatus::PendingSignOff)
    }
}

impl UrgencyLevel {
    pub const ALL: [UrgencyLevel; 3] = [UrgencyLevel::Standard, UrgencyLevel::High, UrgencyLevel::Urgent];
    pub fn label(self) -> &'static str {
        match self {
            UrgencyLevel::Standard => "Normal",
            UrgencyLevel::High => "Important",
            UrgencyLevel::Urgent => "Urgent",
        }
    }
    pub fn tone(self) -> Tone {
        match self {
            UrgencyLevel::Standard => Tone::Gray,
            UrgencyLevel::High => Tone::Amber,
            UrgencyLevel::Urgent => Tone::Red,
        }
    }
}

impl AlertCategory {
    pub const ALL: [AlertCategory; 5] = [
        AlertCategory::MaterialDelay,
        AlertCategory::DesignDecision,
        AlertCategory::ScheduleChange,
        AlertCategory::EquipmentBreakdown,
        AlertCategory::RoutineUpdate,
    ];
    pub fn label(self) -> &'static str {
        match self {
            AlertCategory::MaterialDelay => "Material delay",
            AlertCategory::DesignDecision => "Design choice",
            AlertCategory::ScheduleChange => "Schedule change",
            AlertCategory::EquipmentBreakdown => "Equipment problem",
            AlertCategory::RoutineUpdate => "Update",
        }
    }
}

impl DecisionStatus {
    pub fn label(self) -> &'static str {
        match self {
            DecisionStatus::DraftInAdminQueue => "Draft — waiting for office review",
            DecisionStatus::PublishedPendingClient => "Waiting for owner",
            DecisionStatus::ApprovedOptionA => "Approved — Option A",
            DecisionStatus::ApprovedOptionB => "Approved — Option B",
            DecisionStatus::AcknowledgedReceipt => "Seen by owner",
        }
    }
    pub fn tone(self) -> Tone {
        match self {
            DecisionStatus::DraftInAdminQueue => Tone::Gray,
            DecisionStatus::PublishedPendingClient => Tone::Amber,
            _ => Tone::Green,
        }
    }
    pub fn is_resolved(self) -> bool {
        matches!(
            self,
            DecisionStatus::ApprovedOptionA | DecisionStatus::ApprovedOptionB | DecisionStatus::AcknowledgedReceipt
        )
    }
}

impl MobilityType {
    pub const ALL: [MobilityType; 3] = [MobilityType::MobileMachinery, MobilityType::FixedInfrastructure, MobilityType::ModularComponent];
    pub fn label(self) -> &'static str {
        match self {
            MobilityType::MobileMachinery => "Heavy machinery",
            MobilityType::FixedInfrastructure => "Built-in equipment",
            MobilityType::ModularComponent => "Swappable part",
        }
    }
}

impl AssetHealth {
    pub fn label(self) -> &'static str {
        match self {
            AssetHealth::Operational => "Working",
            AssetHealth::NeedsMaintenance => "Needs service",
            AssetHealth::BrokenDown => "Broken down",
        }
    }
    pub fn tone(self) -> Tone {
        match self {
            AssetHealth::Operational => Tone::Green,
            AssetHealth::NeedsMaintenance => Tone::Amber,
            AssetHealth::BrokenDown => Tone::Red,
        }
    }
}

impl ImpactLevel {
    pub const ALL: [ImpactLevel; 3] = [ImpactLevel::NoImpact, ImpactLevel::MinorDelay, ImpactLevel::FullWorkStoppage];
    pub fn label(self) -> &'static str {
        match self {
            ImpactLevel::NoImpact => "No effect on work",
            ImpactLevel::MinorDelay => "Slows work down",
            ImpactLevel::FullWorkStoppage => "Work has stopped",
        }
    }
}

impl CallOutStatus {
    pub fn label(self) -> &'static str {
        match self {
            CallOutStatus::Open => "Open",
            CallOutStatus::InReview => "Team is checking",
            CallOutStatus::Resolved => "Resolved",
        }
    }
    pub fn tone(self) -> Tone {
        match self {
            CallOutStatus::Open => Tone::Amber,
            CallOutStatus::InReview => Tone::Blue,
            CallOutStatus::Resolved => Tone::Green,
        }
    }
}

impl DelayCategory {
    pub const ALL: [DelayCategory; 6] = [
        DelayCategory::None,
        DelayCategory::Weather,
        DelayCategory::EquipmentFailure,
        DelayCategory::MaterialShortage,
        DelayCategory::ClientDecision,
        DelayCategory::SiteInstability,
    ];
    pub fn label(self) -> &'static str {
        match self {
            DelayCategory::None => "No delay",
            DelayCategory::Weather => "Weather",
            DelayCategory::EquipmentFailure => "Equipment Failure",
            DelayCategory::MaterialShortage => "Material Shortage",
            DelayCategory::ClientDecision => "Waiting on Decision",
            DelayCategory::SiteInstability => "Site Conditions",
        }
    }
}

impl IntrusionLevel {
    pub const ALL: [IntrusionLevel; 4] = [
        IntrusionLevel::Silent,
        IntrusionLevel::VisualDisturbance,
        IntrusionLevel::NoiseHeavy,
        IntrusionLevel::UtilityCut,
    ];
    pub fn label(self) -> &'static str {
        match self {
            IntrusionLevel::Silent => "Quiet",
            IntrusionLevel::VisualDisturbance => "Visible work",
            IntrusionLevel::NoiseHeavy => "Noisy",
            IntrusionLevel::UtilityCut => "Water / power interruption",
        }
    }
    /// Noisy or utility-cutting work must stop on a Quiet Day.
    pub fn is_intrusive(self) -> bool {
        matches!(self, IntrusionLevel::NoiseHeavy | IntrusionLevel::UtilityCut)
    }
}

impl AgendaEventType {
    pub const ALL: [AgendaEventType; 4] = [
        AgendaEventType::TargetDeadline,
        AgendaEventType::IntrusiveMaintenance,
        AgendaEventType::RoutineMaintenance,
        AgendaEventType::VipSpecialOccasion,
    ];
    pub fn label(self) -> &'static str {
        match self {
            AgendaEventType::TargetDeadline => "Target finish date",
            AgendaEventType::IntrusiveMaintenance => "Disruptive work",
            AgendaEventType::RoutineMaintenance => "Routine upkeep",
            AgendaEventType::VipSpecialOccasion => "Quiet Day / Special Occasion",
        }
    }
}

/// Weighted progress of a work item from its phases (0 – 100).
pub fn weighted_progress(phases: &[ProjectPhase]) -> u8 {
    let total: u32 = phases
        .iter()
        .filter(|p| p.is_completed)
        .map(|p| p.weight_percentage as u32)
        .sum();
    total.min(100) as u8
}

/// Applies the 99% handover lock: a fully completed project is held at 99
/// until the owner signs off.
pub fn displayed_progress(raw: u8, signed_off: bool) -> u8 {
    if raw >= 100 && !signed_off {
        99
    } else {
        raw.min(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn phase(weight: u8, done: bool) -> ProjectPhase {
        ProjectPhase {
            id: Uuid::nil(),
            work_item_id: Uuid::nil(),
            phase_name: "p".into(),
            order_index: 0,
            weight_percentage: weight,
            is_completed: done,
            completed_at: done.then(Utc::now),
            proof_photo_url: None,
        }
    }

    #[test]
    fn weighted_progress_sums_completed_phases() {
        let phases = vec![phase(10, true), phase(20, true), phase(60, false), phase(10, false)];
        assert_eq!(weighted_progress(&phases), 30);
    }

    #[test]
    fn full_completion_is_held_at_99_until_sign_off() {
        let phases = vec![phase(40, true), phase(60, true)];
        assert_eq!(displayed_progress(weighted_progress(&phases), false), 99);
        assert_eq!(displayed_progress(weighted_progress(&phases), true), 100);
    }

    #[test]
    fn enums_round_trip_through_json() {
        let json = serde_json::to_string(&WorkStatus::PendingSignOff).unwrap();
        assert_eq!(json, "\"PendingSignOff\"");
        let back: WorkStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, WorkStatus::PendingSignOff);
    }
}
