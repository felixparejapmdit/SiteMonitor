//! Realistic initial estate data.
//!
//! 3 main residences, 4 guest residences, a parking-conversion hall, gardens,
//! an outdoor pool, the Power House (500 kW generator + nested LiFePO4
//! batteries), the deepwell, water filtration and the contractor yard
//! (office, warehouse, workshop with a 2-ton mini crane, backhoe and dump
//! truck) — plus live projects, decisions, photos and a full agenda.

use chrono::{DateTime, Duration, Local, NaiveTime, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, Set, TransactionTrait};
use shared::*;
use uuid::Uuid;

use crate::convert::{enc, to_json};
use crate::entities::*;
use crate::services::progress;

/// Local day `offset` days from today at `h:m`, as UTC.
fn at(offset: i64, h: u32, m: u32) -> DateTime<Utc> {
    let day = Local::now().date_naive() + Duration::days(offset);
    day.and_time(NaiveTime::from_hms_opt(h, m, 0).unwrap_or_default())
        .and_local_timezone(Local)
        .earliest()
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
}

/// "Earlier today at h:m" — if that moment hasn't happened yet (e.g. the
/// estate is seeded at 1 AM), it is mapped to a recent past time that keeps
/// the same order, so nothing ever appears to have happened in the future.
fn past(h: u32, m: u32) -> DateTime<Utc> {
    let now = Utc::now();
    let target = at(0, h, m);
    if target <= now - Duration::minutes(5) {
        return target;
    }
    let minutes_into_day = (h * 60 + m) as i64;
    now - Duration::minutes(((12 * 60 - minutes_into_day).max(20)) / 4)
}

/// "8:00 AM" in estate time.
fn clock(t: DateTime<Utc>) -> String {
    t.with_timezone(&Local).format("%-I:%M %p").to_string()
}

fn rect(x1: f32, y1: f32, x2: f32, y2: f32) -> Vec<Point2D> {
    vec![Point2D { x: x1, y: y1 }, Point2D { x: x2, y: y1 }, Point2D { x: x2, y: y2 }, Point2D { x: x1, y: y2 }]
}

fn poly(points: &[(f32, f32)]) -> Vec<Point2D> {
    points.iter().map(|(x, y)| Point2D { x: *x, y: *y }).collect()
}

/// Seeds only when the database is empty. Returns `true` if data was added.
pub async fn seed_if_empty(db: &DatabaseConnection) -> anyhow::Result<bool> {
    if user::Entity::find().count(db).await? > 0 {
        return Ok(false);
    }
    seed(db).await?;
    Ok(true)
}

struct Ctx<'a> {
    txn: &'a sea_orm::DatabaseTransaction,
    map: Uuid,
}

impl Ctx<'_> {
    async fn area(&self, parent: Option<Uuid>, name: &str, class: ZoneClass, pts: Vec<Point2D>) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        spatial_area::ActiveModel {
            id: Set(id),
            base_map_id: Set(self.map),
            parent_id: Set(parent),
            name: Set(name.into()),
            zone_class: Set(enc(&class)),
            polygon_points: Set(to_json(&pts)),
            is_archived: Set(false),
        }
        .insert(self.txn)
        .await?;
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    async fn asset(
        &self,
        name: &str,
        mobility: MobilityType,
        area: Uuid,
        parent: Option<Uuid>,
        health: AssetHealth,
        issue: Option<&str>,
        impact: ImpactLevel,
        eta_days: Option<i64>,
    ) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        asset::ActiveModel {
            id: Set(id),
            name: Set(name.into()),
            mobility: Set(enc(&mobility)),
            current_area_id: Set(area),
            parent_asset_id: Set(parent),
            health: Set(enc(&health)),
            active_issue_report: Set(issue.map(Into::into)),
            impact_level: Set(enc(&impact)),
            repair_eta: Set(eta_days.map(|d| at(d, 16, 0))),
        }
        .insert(self.txn)
        .await?;
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    async fn work(
        &self,
        area: Uuid,
        stream: WorkStream,
        title: &str,
        description: &str,
        status: WorkStatus,
        progress: i32,
        original_deadline: DateTime<Utc>,
        target_deadline: DateTime<Utc>,
        crew: i32,
        before: Option<&str>,
    ) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        work_item::ActiveModel {
            id: Set(id),
            area_id: Set(area),
            stream: Set(enc(&stream)),
            title: Set(title.into()),
            description: Set(description.into()),
            status: Set(enc(&status)),
            calculated_progress: Set(progress),
            original_deadline: Set(original_deadline),
            current_target_deadline: Set(target_deadline),
            group_leader_name: Set("Ramon Dizon".into()),
            group_leader_phone: Set("+63 917 555 0142".into()),
            site_qc_officer_name: Set("Engr. Liza Marquez".into()),
            site_qc_officer_phone: Set("+63 918 555 0177".into()),
            admin_coordinator_name: Set("Marco Villanueva".into()),
            admin_coordinator_phone: Set("+63 917 555 0101".into()),
            active_worker_count: Set(crew),
            before_photo_url: Set(before.map(Into::into)),
            after_photo_url: Set(None),
        }
        .insert(self.txn)
        .await?;
        Ok(id)
    }

    async fn phases(&self, work: Uuid, list: &[(&str, i32, Option<i64>)]) -> anyhow::Result<Vec<Uuid>> {
        let mut ids = Vec::new();
        for (i, (name, weight, done_days_ago)) in list.iter().enumerate() {
            let id = Uuid::new_v4();
            project_phase::ActiveModel {
                id: Set(id),
                work_item_id: Set(work),
                phase_name: Set((*name).into()),
                order_index: Set(i as i32 + 1),
                weight_percentage: Set(*weight),
                is_completed: Set(done_days_ago.is_some()),
                completed_at: Set(done_days_ago.map(|d| at(-d, 15, 30))),
                proof_photo_url: Set(None),
            }
            .insert(self.txn)
            .await?;
            ids.push(id);
        }
        Ok(ids)
    }

    #[allow(clippy::too_many_arguments)]
    async fn event(
        &self,
        work: Uuid,
        title: &str,
        description: &str,
        milestone: bool,
        cat: DelayCategory,
        days: i32,
        when: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        timeline_event_record::ActiveModel {
            id: Set(Uuid::new_v4()),
            work_item_id: Set(work),
            title: Set(title.into()),
            description: Set(description.into()),
            is_milestone: Set(milestone),
            delay_category: Set(enc(&cat)),
            days_delayed: Set(days),
            timestamp: Set(when),
        }
        .insert(self.txn)
        .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn ledger(
        &self,
        work: Uuid,
        source: Option<Uuid>,
        delta: i32,
        result: i32,
        reason: &str,
        by: Uuid,
        when: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        manpower_ledger_entry::ActiveModel {
            id: Set(Uuid::new_v4()),
            work_item_id: Set(work),
            source_work_item_id: Set(source),
            headcount_delta: Set(delta),
            resulting_headcount: Set(result),
            reason: Set(reason.into()),
            logged_by: Set(by),
            timestamp: Set(when),
        }
        .insert(self.txn)
        .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn photo(&self, work: Uuid, area: Uuid, url: &str, caption: &str, by: Uuid, when: DateTime<Utc>, ack: bool) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        site_photo::ActiveModel {
            id: Set(id),
            work_item_id: Set(work),
            area_id: Set(area),
            image_url: Set(url.into()),
            caption: Set(caption.into()),
            uploaded_by: Set(by),
            uploaded_at: Set(when),
            client_acknowledged: Set(ack),
        }
        .insert(self.txn)
        .await?;
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    async fn agenda(
        &self,
        area: Option<Uuid>,
        work: Option<Uuid>,
        title: &str,
        description: &str,
        kind: AgendaEventType,
        level: IntrusionLevel,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();
        estate_agenda_item::ActiveModel {
            id: Set(id),
            area_id: Set(area),
            work_item_id: Set(work),
            title: Set(title.into()),
            description: Set(description.into()),
            event_type: Set(enc(&kind)),
            intrusion_level: Set(enc(&level)),
            start_time: Set(start),
            end_time: Set(end),
            is_flagged_conflict: Set(false),
        }
        .insert(self.txn)
        .await?;
        Ok(id)
    }

    async fn reserve(&self, asset: Uuid, agenda: Uuid) -> anyhow::Result<()> {
        equipment_reservation::ActiveModel { id: Set(Uuid::new_v4()), asset_id: Set(asset), agenda_item_id: Set(agenda) }
            .insert(self.txn)
            .await?;
        Ok(())
    }
}

pub async fn seed(db: &DatabaseConnection) -> anyhow::Result<()> {
    let txn = db.begin().await?;

    // ------------------------------------------------------------ base map
    let map = Uuid::new_v4();
    base_map_version::ActiveModel {
        id: Set(Uuid::new_v4()),
        version_name: Set("2026 Q2 Master Plan (archived)".into()),
        orthographic_image_url: Set("/seed/estate-map-v0.svg".into()),
        is_active: Set(false),
        created_at: Set(Utc::now() - Duration::days(160)),
    }
    .insert(&txn)
    .await?;
    base_map_version::ActiveModel {
        id: Set(map),
        version_name: Set("2026 Q4 Top-Down Render".into()),
        orthographic_image_url: Set("/seed/estate-map-v1.svg".into()),
        is_active: Set(true),
        created_at: Set(Utc::now() - Duration::days(20)),
    }
    .insert(&txn)
    .await?;
    let c = Ctx { txn: &txn, map };

    // ------------------------------------------------------------ zones
    use ZoneClass::*;
    let main1 = c.area(None, "Main Residence 1", MajorResidential, poly(&[(37.0, 6.0), (63.0, 6.0), (63.0, 30.0), (54.0, 30.0), (54.0, 33.0), (46.0, 33.0), (46.0, 30.0), (37.0, 30.0)])).await?;
    let north_wing = c.area(Some(main1), "North Wing", MajorResidential, rect(38.0, 7.0, 62.0, 17.0)).await?;
    let south_wing = c.area(Some(main1), "South Wing", MajorResidential, rect(38.0, 18.0, 53.0, 29.5)).await?;
    let _guest_annex = c.area(Some(main1), "Guest Annex", MajorResidential, rect(54.0, 18.0, 62.0, 29.5)).await?;
    let master_suite = c.area(Some(south_wing), "Master Suite", MajorResidential, rect(38.5, 18.5, 45.5, 29.0)).await?;
    let _library = c.area(Some(south_wing), "Library", MajorResidential, rect(46.0, 18.5, 52.5, 29.0)).await?;
    let main2 = c.area(None, "Main Residence 2", MajorResidential, rect(7.0, 8.0, 30.0, 28.0)).await?;
    let main3 = c.area(None, "Main Residence 3", MajorResidential, rect(70.0, 8.0, 93.0, 28.0)).await?;

    let villa1 = c.area(None, "Guest Villa 1", MinorResidential, rect(5.0, 36.0, 17.0, 48.0)).await?;
    let villa2 = c.area(None, "Guest Villa 2", MinorResidential, rect(19.0, 36.0, 31.0, 48.0)).await?;
    let villa3 = c.area(None, "Guest Villa 3", MinorResidential, rect(69.0, 36.0, 81.0, 48.0)).await?;
    let villa4 = c.area(None, "Guest Villa 4", MinorResidential, rect(83.0, 36.0, 95.0, 48.0)).await?;

    let garden = c.area(None, "Grand Garden", Recreational, poly(&[(35.0, 37.0), (65.0, 37.0), (66.0, 52.0), (63.0, 66.0), (37.0, 66.0), (34.0, 52.0)])).await?;
    let _orchid = c.area(Some(garden), "Orchid Greenhouse", Recreational, rect(37.0, 39.0, 48.0, 50.0)).await?;
    let rose = c.area(Some(garden), "Rose Garden & Gazebo", Recreational, rect(52.0, 39.0, 63.0, 50.0)).await?;
    let retaining = c.area(Some(garden), "Retaining Wall Terrace", Recreational, rect(38.0, 54.0, 62.0, 64.0)).await?;
    let pool = c.area(None, "Outdoor Pool", Recreational, poly(&[(7.0, 54.0), (30.0, 54.0), (30.0, 70.0), (7.0, 70.0)])).await?;
    let hall = c.area(None, "Multi-Purpose Hall (Parking Conversion)", MultiPurposeFacility, rect(70.0, 54.0, 93.0, 70.0)).await?;

    let power = c.area(None, "Power House", UtilityInfrastructure, rect(5.0, 78.0, 20.0, 94.0)).await?;
    let filtration = c.area(None, "Water Filtration Plant", UtilityInfrastructure, rect(23.0, 78.0, 36.0, 94.0)).await?;
    let deepwell = c.area(None, "Active Deepwell", UtilityInfrastructure, poly(&[(40.0, 80.0), (49.0, 80.0), (51.0, 87.0), (49.0, 94.0), (40.0, 94.0), (38.0, 87.0)])).await?;
    let office = c.area(None, "Contractor Office", ContractorOperations, rect(56.0, 78.0, 67.0, 94.0)).await?;
    let warehouse = c.area(None, "Contractor Warehouse", ContractorOperations, rect(70.0, 78.0, 82.0, 94.0)).await?;
    let workshop = c.area(None, "Contractor Workshop", ContractorOperations, rect(84.0, 78.0, 95.0, 94.0)).await?;
    let old_shed = c.area(None, "Old Tool Shed", ContractorOperations, rect(84.0, 30.0, 92.0, 34.0)).await?;
    let _ = (villa1, villa3, villa4, office, main3);

    // ------------------------------------------------------------ users
    let vip = Uuid::new_v4();
    let admin = Uuid::new_v4();
    let monitor = Uuid::new_v4();
    let monitor2 = Uuid::new_v4();
    for (id, name, role, phone, email, areas) in [
        (vip, "Victoria Ashford", UserRole::VipClient, "+63 917 555 0001", "victoria@ashford-estate.example", vec![]),
        (admin, "Marco Villanueva", UserRole::OfficeAdmin, "+63 917 555 0101", "marco@ashford-estate.example", vec![]),
        (monitor, "Jun Dela Cruz", UserRole::SiteMonitor, "+63 917 555 0202", "jun@ashford-estate.example", vec![garden, hall, pool]),
        (monitor2, "Ana Santos", UserRole::SiteMonitor, "+63 917 555 0203", "ana@ashford-estate.example", vec![main1, power, deepwell]),
    ] {
        user::ActiveModel {
            id: Set(id),
            name: Set(name.into()),
            role: Set(enc(&role)),
            phone: Set(phone.into()),
            email: Set(email.into()),
            assigned_area_ids: Set(to_json(&areas)),
        }
        .insert(&txn)
        .await?;
    }

    // ------------------------------------------------------------ assets
    use AssetHealth::*;
    use ImpactLevel::*;
    use MobilityType::*;
    let _genset = c.asset("500 kW Generator", FixedInfrastructure, power, None, Operational, None, NoImpact, None).await?;
    let array = c.asset("Solar/Backup Generator Array", FixedInfrastructure, power, None, Operational, None, NoImpact, None).await?;
    for n in 1..=4 {
        let (h, issue) = if n == 3 {
            (NeedsMaintenance, Some("Cell voltage drifting 0.3V below the others. Scheduled for a balance charge in the workshop."))
        } else {
            (Operational, None)
        };
        c.asset(&format!("12.8V 200Ah LiFePO4 Battery #{n}"), ModularComponent, power, Some(array), h, issue, NoImpact, if n == 3 { Some(3) } else { None }).await?;
    }
    c.asset("Deepwell Pump", FixedInfrastructure, deepwell, None, Operational, None, NoImpact, None).await?;
    c.asset("Water Filtration Skid", FixedInfrastructure, filtration, None, Operational, None, NoImpact, None).await?;
    c.asset("Pool Pump", FixedInfrastructure, pool, None, Operational, None, NoImpact, None).await?;
    c.asset("HVAC Chiller", FixedInfrastructure, main1, None, Operational, None, NoImpact, None).await?;
    let elevator2 = c.asset("Elevator 2", FixedInfrastructure, north_wing, None, NeedsMaintenance, Some("Door sensor fault — car stops between floors. Technician on site replacing the sensor."), MinorDelay, Some(1)).await?;
    c.asset("Elevator 1", FixedInfrastructure, main2, None, Operational, None, NoImpact, None).await?;
    c.asset("Garden Lighting Transformer", FixedInfrastructure, rose, None, Operational, None, NoImpact, None).await?;
    c.asset("Gazebo Ceiling Fans", FixedInfrastructure, rose, None, Operational, None, NoImpact, None).await?;
    let crane = c.asset("2-Ton Mini Crane", MobileMachinery, retaining, None, Operational, None, NoImpact, None).await?;
    let backhoe = c.asset("Backhoe", MobileMachinery, hall, None, Operational, None, NoImpact, None).await?;
    let _truck = c.asset("Dump Truck", MobileMachinery, warehouse, None, Operational, None, NoImpact, None).await?;
    c.asset("Plate Compactor", MobileMachinery, workshop, None, Operational, None, NoImpact, None).await?;
    let _ = elevator2;

    // ------------------------------------------------------------ work items
    use WorkStatus::*;
    use WorkStream::{CapitalProject, ReactiveRepair};
    let parking = c
        .work(
            hall,
            CapitalProject,
            "Parking Conversion to Multi-Purpose Hall",
            "Converting the old 6-car garage into an events hall with natural-stone paving, new lighting and a covered drop-off.",
            InProgress,
            30,
            at(9, 17, 0),
            at(12, 17, 0),
            14,
            Some("/seed/photos/parking-before.svg"),
        )
        .await?;
    c.phases(parking, &[("Demolition", 10, Some(20)), ("Leveling", 20, Some(6)), ("Paving", 60, None), ("Painting", 10, None)]).await?;

    let wall = c
        .work(
            retaining,
            CapitalProject,
            "Retaining Wall Leveling",
            "Rebuilding and leveling the garden terrace retaining wall so the lower lawn stops sliding after heavy rain.",
            InProgress,
            80,
            at(1, 17, 0),
            at(2, 12, 0),
            6,
            Some("/seed/photos/wall-before.svg"),
        )
        .await?;
    c.phases(wall, &[("Excavation", 15, Some(18)), ("Rebar & Formwork", 25, Some(11)), ("Concrete Pour", 40, Some(3)), ("Backfill & Finishing", 20, None)]).await?;

    let gazebo = c
        .work(
            rose,
            CapitalProject,
            "Rose Garden Gazebo Build",
            "A teak and stone gazebo overlooking the rose beds, with ceiling fans and warm garden lighting.",
            InProgress,
            90,
            at(-3, 17, 0),
            at(-1, 17, 0),
            0,
            Some("/seed/photos/gazebo-before.svg"),
        )
        .await?;
    let gazebo_phases = c
        .phases(gazebo, &[("Foundation", 20, Some(30)), ("Timber Frame", 40, Some(16)), ("Roofing", 30, Some(7)), ("Lighting & Finishing", 10, None)])
        .await?;

    let roof = c
        .work(
            villa2,
            CapitalProject,
            "Guest Villa 2 Roof Renovation",
            "Replacing the clay roof tiles and waterproofing membrane before the rainy season.",
            Planning,
            0,
            at(30, 17, 0),
            at(30, 17, 0),
            0,
            None,
        )
        .await?;
    c.phases(roof, &[("Scaffolding", 10, None), ("Tile Removal", 20, None), ("Waterproofing", 40, None), ("New Tiles", 30, None)]).await?;

    let suite = c
        .work(
            master_suite,
            CapitalProject,
            "Master Suite Refit",
            "New walnut joinery, a stone bathroom and a larger picture window facing the garden.",
            InProgress,
            45,
            at(6, 17, 0),
            at(6, 17, 0),
            5,
            Some("/seed/photos/suite-before.svg"),
        )
        .await?;
    c.phases(suite, &[("Strip-Out", 15, Some(25)), ("Window Enlargement", 30, Some(9)), ("Joinery", 35, None), ("Stone & Finishing", 20, None)]).await?;

    let pool_clean = c
        .work(pool, WorkStream::RoutineMaintenance, "Swimming Pool Cleanup", "Skim, vacuum, brush tiles and balance chemicals — every Monday, Wednesday and Friday.", InProgress, 100, at(60, 17, 0), at(60, 17, 0), 2, None)
        .await?;
    let descale = c
        .work(deepwell, WorkStream::RoutineMaintenance, "Deepwell Descaling", "Quarterly descaling of the deepwell pump and pipes to keep water pressure strong.", Planning, 0, at(3, 15, 0), at(3, 15, 0), 0, None)
        .await?;
    let backwash = c
        .work(filtration, WorkStream::RoutineMaintenance, "Water Filtration Backwash", "Weekly backwash of the sand filters for clean, clear water.", InProgress, 100, at(45, 17, 0), at(45, 17, 0), 1, None)
        .await?;
    let lift = c
        .work(north_wing, ReactiveRepair, "Elevator 2 Repair", "The car stops between floors. Technician on site replacing the door sensor.", InProgress, 50, at(1, 17, 0), at(1, 17, 0), 2, None)
        .await?;
    let recal = c
        .work(power, ReactiveRepair, "Generator Recalibration", "Recalibrate the 500 kW generator's voltage regulator after last week's load test.", Planning, 0, at(8, 17, 0), at(8, 17, 0), 0, None)
        .await?;
    let _old = c
        .work(old_shed, WorkStream::RoutineMaintenance, "Old Tool Shed Clear-Out", "Historic record kept after the shed was demolished.", CompletedAndArchived, 100, at(-40, 17, 0), at(-40, 17, 0), 0, None)
        .await?;
    let _ = (roof, recal);

    // ------------------------------------------------------------ timeline
    c.event(parking, "Phase completed: Demolition", "Demolition (10% of the work) finished. Overall progress is now 10%.", true, DelayCategory::None, 0, at(-20, 15, 30)).await?;
    c.event(parking, "Heavy rain stopped crane anchoring", "Torrential rain delayed crane anchoring on the hall's steel canopy.", false, DelayCategory::Weather, 2, at(-12, 9, 0)).await?;
    c.event(parking, "Unstable subsoil found", "Soft ground near the old drainage line needed extra compaction.", false, DelayCategory::SiteInstability, 1, at(-8, 11, 0)).await?;
    c.event(parking, "Phase completed: Leveling", "Leveling (20% of the work) finished. Overall progress is now 30%.", true, DelayCategory::None, 0, at(-6, 15, 30)).await?;
    c.event(wall, "Phase completed: Excavation", "Excavation (15% of the work) finished.", true, DelayCategory::None, 0, at(-18, 15, 30)).await?;
    c.event(wall, "Phase completed: Rebar & Formwork", "Rebar & Formwork (25% of the work) finished.", true, DelayCategory::None, 0, at(-11, 15, 30)).await?;
    c.event(wall, "Concrete truck broke down", "The ready-mix truck broke down on the access road; the pour moved one day.", false, DelayCategory::EquipmentFailure, 1, at(-4, 8, 0)).await?;
    c.event(wall, "Phase completed: Concrete Pour", "Concrete Pour (40% of the work) finished. Overall progress is now 80%.", true, DelayCategory::None, 0, at(-3, 15, 30)).await?;
    c.event(gazebo, "Phase completed: Foundation", "Foundation (20% of the work) finished.", true, DelayCategory::None, 0, at(-30, 15, 30)).await?;
    c.event(gazebo, "Teak delivery late", "The teak beams arrived two days late from the supplier.", false, DelayCategory::MaterialShortage, 2, at(-20, 10, 0)).await?;
    c.event(gazebo, "Phase completed: Timber Frame", "Timber Frame (40% of the work) finished.", true, DelayCategory::None, 0, at(-16, 15, 30)).await?;
    c.event(gazebo, "Phase completed: Roofing", "Roofing (30% of the work) finished.", true, DelayCategory::None, 0, at(-7, 15, 30)).await?;
    c.event(suite, "Phase completed: Strip-Out", "Strip-Out (15% of the work) finished.", true, DelayCategory::None, 0, at(-25, 15, 30)).await?;
    c.event(suite, "Phase completed: Window Enlargement", "Window Enlargement (30% of the work) finished.", true, DelayCategory::None, 0, at(-9, 15, 30)).await?;
    c.event(pool_clean, &format!("Swimming Pool Cleanup — Completed {}", clock(past(8, 0))), "Skimmed, vacuumed and chemicals balanced.", true, DelayCategory::None, 0, past(8, 0)).await?;
    c.event(backwash, &format!("Water Filtration Backwash — Completed {}", clock(past(7, 15))), "Sand filters backwashed; pressure back to normal.", true, DelayCategory::None, 0, past(7, 15)).await?;
    c.event(lift, "Technician on site", "Otis technician arrived and isolated the faulty door sensor.", false, DelayCategory::None, 0, past(9, 30)).await?;

    // ------------------------------------------------------------ manpower
    c.ledger(parking, None, 8, 8, "Morning crew allocation", admin, past(7, 35)).await?;
    c.ledger(roof, None, 6, 6, "Morning crew allocation", admin, past(7, 35)).await?;
    c.ledger(roof, None, -6, 0, "Moved to Parking Conversion to Multi-Purpose Hall: Accelerating leveling before rain", monitor, past(9, 10)).await?;
    c.ledger(parking, Some(roof), 6, 14, "Accelerating leveling before rain", monitor, past(9, 10)).await?;
    c.ledger(wall, None, 6, 6, "Morning crew allocation", admin, past(7, 35)).await?;
    c.ledger(suite, None, 5, 5, "Morning crew allocation", admin, past(7, 35)).await?;
    c.ledger(pool_clean, None, 2, 2, "Morning crew allocation", admin, past(7, 35)).await?;
    c.ledger(backwash, None, 1, 1, "Morning crew allocation", admin, past(7, 35)).await?;
    c.ledger(lift, None, 2, 2, "Morning crew allocation", admin, past(7, 35)).await?;

    // ------------------------------------------------------------ photos & call-outs
    c.photo(parking, hall, "/seed/photos/parking-before.svg", "Day 1 — the old garage before demolition", monitor, at(-24, 10, 0), true).await?;
    let paving = c.photo(parking, hall, "/seed/photos/parking-progress.svg", "Leveled base ready for paving", monitor, at(-1, 16, 0), false).await?;
    c.photo(wall, retaining, "/seed/photos/wall-before.svg", "Day 1 — slipping terrace wall", monitor, at(-22, 10, 0), true).await?;
    let wall_photo = c.photo(wall, retaining, "/seed/photos/wall-progress.svg", "Concrete poured, curing under covers", monitor, at(-3, 16, 30), false).await?;
    c.photo(gazebo, rose, "/seed/photos/gazebo-before.svg", "Day 1 — the empty rose garden corner", monitor, at(-34, 10, 0), true).await?;
    c.photo(suite, master_suite, "/seed/photos/suite-before.svg", "Day 1 — master suite before strip-out", monitor2, at(-28, 10, 0), true).await?;
    c.photo(pool_clean, pool, "/seed/photos/pool-clean.svg", "Pool cleaned and chemicals balanced", monitor, past(8, 5), false).await?;

    photo_call_out::ActiveModel {
        id: Set(Uuid::new_v4()),
        photo_id: Set(paving),
        x_percent: Set(71.5),
        y_percent: Set(64.0),
        client_comment: Set("Is this water pooling near the drop-off? Please check the drainage.".into()),
        staff_reply: Set(None),
        status: Set(enc(&CallOutStatus::Open)),
        created_at: Set(at(-1, 19, 0)),
        resolved_at: Set(None),
    }
    .insert(&txn)
    .await?;
    photo_call_out::ActiveModel {
        id: Set(Uuid::new_v4()),
        photo_id: Set(wall_photo),
        x_percent: Set(28.0),
        y_percent: Set(58.0),
        client_comment: Set("Will these covers stay on through the weekend rain?".into()),
        staff_reply: Set(Some("Yes — the covers are weighted and stay on until Monday so the concrete cures evenly.".into())),
        status: Set(enc(&CallOutStatus::Resolved)),
        created_at: Set(at(-3, 18, 0)),
        resolved_at: Set(Some(at(-2, 8, 30))),
    }
    .insert(&txn)
    .await?;

    // ------------------------------------------------------------ decisions
    decision_alert::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_item_id: Set(parking),
        area_id: Set(hall),
        urgency: Set(enc(&UrgencyLevel::High)),
        category: Set(enc(&AlertCategory::MaterialDelay)),
        title: Set("Imported paving stones delayed by 2 weeks".into()),
        issue_description: Set("The Italian travertine for the hall's drop-off is held at customs. The shipper now estimates a 14-day delay.".into()),
        mitigation_action_taken: Set("We have moved the crew to edge-curbing and drainage so no time is lost this week, and sourced a matching local stone sample for you to compare.".into()),
        option_a_label: Set(Some("Approve Local Paving Supplier - No Delay".into())),
        option_b_label: Set(Some("Wait for Imported Stones - +14 Days Delay".into())),
        status: Set(enc(&DecisionStatus::PublishedPendingClient)),
        created_at: Set(at(-1, 11, 0)),
        resolved_at: Set(None),
    }
    .insert(&txn)
    .await?;
    decision_alert::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_item_id: Set(pool_clean),
        area_id: Set(pool),
        urgency: Set(enc(&UrgencyLevel::Standard)),
        category: Set(enc(&AlertCategory::RoutineUpdate)),
        title: Set("Pool tiles re-grouted at the shallow end".into()),
        issue_description: Set("During this morning's cleaning the team re-grouted six loose tiles at the shallow-end steps.".into()),
        mitigation_action_taken: Set("The grout is fully cured; the pool is ready to use.".into()),
        option_a_label: Set(None),
        option_b_label: Set(None),
        status: Set(enc(&DecisionStatus::PublishedPendingClient)),
        created_at: Set(past(9, 0)),
        resolved_at: Set(None),
    }
    .insert(&txn)
    .await?;
    decision_alert::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_item_id: Set(roof),
        area_id: Set(villa2),
        urgency: Set(enc(&UrgencyLevel::High)),
        category: Set(enc(&AlertCategory::DesignDecision)),
        title: Set("Cracked roof decking found at Guest Villa 2".into()),
        issue_description: Set("While measuring for scaffolding we found cracked plywood decking under the north slope tiles.".into()),
        mitigation_action_taken: Set("Area covered with tarpaulin. No leaks inside yet.".into()),
        option_a_label: Set(None),
        option_b_label: Set(None),
        status: Set(enc(&DecisionStatus::DraftInAdminQueue)),
        created_at: Set(past(10, 15)),
        resolved_at: Set(None),
    }
    .insert(&txn)
    .await?;

    // ------------------------------------------------------------ agenda
    use AgendaEventType::{IntrusiveMaintenance, TargetDeadline};
    use IntrusionLevel::*;
    for (area, work, title, deadline) in [
        (hall, parking, "Parking Conversion to Multi-Purpose Hall", at(12, 17, 0)),
        (retaining, wall, "Retaining Wall Leveling", at(2, 12, 0)),
        (master_suite, suite, "Master Suite Refit", at(6, 17, 0)),
        (villa2, roof, "Guest Villa 2 Roof Renovation", at(30, 17, 0)),
    ] {
        c.agenda(Some(area), Some(work), &format!("{title} — target handover"), "Planned completion and handover.", TargetDeadline, Silent, deadline - Duration::hours(1), deadline).await?;
    }
    c.agenda(Some(deepwell), Some(descale), "Deepwell Descaling", "Water pressure reduced 1:00 PM to 3:00 PM while the pump is descaled.", IntrusiveMaintenance, UtilityCut, at(3, 13, 0), at(3, 15, 0)).await?;
    let compaction = c.agenda(Some(hall), Some(parking), "Paving base compaction", "Backhoe and plate compactor working on the hall's drop-off base.", IntrusiveMaintenance, NoiseHeavy, at(4, 8, 0), at(4, 12, 0)).await?;
    c.reserve(backhoe, compaction).await?;
    let crane_wall = c.agenda(Some(retaining), Some(wall), "Crane lift — wall capstones", "2-Ton Mini Crane setting the stone capstones on the terrace wall.", IntrusiveMaintenance, NoiseHeavy, at(4, 13, 0), at(4, 16, 0)).await?;
    c.reserve(crane, crane_wall).await?;
    let crane_suite = c.agenda(Some(master_suite), Some(suite), "Crane lift — picture window glass", "Lifting the new picture-window glass panel into the master suite.", IntrusiveMaintenance, NoiseHeavy, at(4, 9, 0), at(4, 11, 0)).await?;
    c.reserve(crane, crane_suite).await?;
    c.agenda(Some(main1), Some(lift), "Elevator 2 — sensor replacement", "North Wing elevator out of service while the sensor is replaced.", IntrusiveMaintenance, VisualDisturbance, at(0, 10, 0), at(1, 12, 0)).await?;
    c.agenda(Some(power), Some(recal), "Generator recalibration — power switchover", "Estate switches to the battery array for 30 minutes while the generator is recalibrated.", IntrusiveMaintenance, UtilityCut, at(8, 10, 0), at(8, 10, 30)).await?;
    for offset in 0..14 {
        let day = Local::now().date_naive() + Duration::days(offset);
        use chrono::Datelike;
        let wd = day.weekday().number_from_monday();
        if matches!(wd, 1 | 3 | 5) {
            c.agenda(Some(pool), Some(pool_clean), "Swimming Pool Cleanup", "Routine skim, vacuum and chemical balance.", AgendaEventType::RoutineMaintenance, Silent, at(offset, 7, 0), at(offset, 8, 30)).await?;
        }
        if wd == 2 {
            c.agenda(Some(filtration), Some(backwash), "Water Filtration Backwash", "Weekly sand-filter backwash.", AgendaEventType::RoutineMaintenance, VisualDisturbance, at(offset, 6, 30), at(offset, 7, 30)).await?;
        }
    }
    c.agenda(Some(garden), None, "Garden detailing", "Hand-trimming hedges and deadheading the rose beds.", AgendaEventType::RoutineMaintenance, Silent, at(5, 7, 0), at(5, 10, 0)).await?;

    // Archive the demolished tool shed (history is kept).
    let shed = spatial_area::Entity::find_by_id(old_shed).one(&txn).await?.expect("just inserted");
    let mut shed: spatial_area::ActiveModel = shed.into();
    shed.is_archived = Set(true);
    shed.update(&txn).await?;

    txn.commit().await?;

    // Finish the gazebo's last phase through the real engine so the 99%
    // lock and the auto-generated handover report are produced naturally.
    let last_phase = *gazebo_phases.last().expect("gazebo has phases");
    complete_seed_phase(db, gazebo, last_phase, monitor).await?;
    Ok(())
}

async fn complete_seed_phase(db: &DatabaseConnection, gazebo: Uuid, phase: Uuid, monitor: Uuid) -> anyhow::Result<()> {
    use crate::state::AppState;
    use crate::ws::Hub;
    let state = AppState { db: db.clone(), hub: Hub::default(), upload_dir: std::env::temp_dir() };
    let txn = db.begin().await?;
    site_photo::ActiveModel {
        id: Set(Uuid::new_v4()),
        work_item_id: Set(gazebo),
        area_id: Set(
            work_item::Entity::find_by_id(gazebo).one(&txn).await?.map(|w| w.area_id).unwrap_or_default(),
        ),
        image_url: Set("/seed/photos/gazebo-after.svg".into()),
        caption: Set("Finished gazebo with lighting on".into()),
        uploaded_by: Set(monitor),
        uploaded_at: Set(at(-1, 17, 30)),
        client_acknowledged: Set(false),
    }
    .insert(&txn)
    .await?;
    txn.commit().await?;
    let user = crate::convert::user(user::Entity::find_by_id(monitor).one(db).await?.expect("monitor exists"));
    progress::complete_phase(&state, phase, None, &user).await.map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(())
}
