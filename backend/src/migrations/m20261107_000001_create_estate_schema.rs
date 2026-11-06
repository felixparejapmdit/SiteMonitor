//! Creates every SiteMonitor table, foreign key and index.

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20261107_000001_create_estate_schema"
    }
}

const UP: &str = r#"
CREATE TABLE users (
    id                UUID PRIMARY KEY,
    name              TEXT NOT NULL,
    role              TEXT NOT NULL CHECK (role IN ('VipClient','OfficeAdmin','SiteMonitor')),
    phone             TEXT NOT NULL,
    email             TEXT NOT NULL UNIQUE,
    assigned_area_ids JSONB NOT NULL DEFAULT '[]'::jsonb
);

CREATE TABLE base_map_versions (
    id                     UUID PRIMARY KEY,
    version_name           TEXT NOT NULL,
    orthographic_image_url TEXT NOT NULL,
    is_active              BOOLEAN NOT NULL DEFAULT FALSE,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX base_map_one_active ON base_map_versions (is_active) WHERE is_active;

CREATE TABLE spatial_areas (
    id             UUID PRIMARY KEY,
    base_map_id    UUID NOT NULL REFERENCES base_map_versions(id),
    parent_id      UUID NULL REFERENCES spatial_areas(id),
    name           TEXT NOT NULL,
    zone_class     TEXT NOT NULL,
    polygon_points JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_archived    BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX spatial_areas_parent ON spatial_areas (parent_id);

CREATE TABLE work_items (
    id                      UUID PRIMARY KEY,
    area_id                 UUID NOT NULL REFERENCES spatial_areas(id),
    stream                  TEXT NOT NULL,
    title                   TEXT NOT NULL,
    description             TEXT NOT NULL,
    status                  TEXT NOT NULL,
    calculated_progress     INTEGER NOT NULL DEFAULT 0 CHECK (calculated_progress BETWEEN 0 AND 100),
    original_deadline       TIMESTAMPTZ NOT NULL,
    current_target_deadline TIMESTAMPTZ NOT NULL,
    group_leader_name       TEXT NOT NULL,
    group_leader_phone      TEXT NOT NULL,
    site_qc_officer_name    TEXT NOT NULL,
    site_qc_officer_phone   TEXT NOT NULL,
    admin_coordinator_name  TEXT NOT NULL,
    admin_coordinator_phone TEXT NOT NULL,
    active_worker_count     INTEGER NOT NULL DEFAULT 0 CHECK (active_worker_count >= 0),
    before_photo_url        TEXT NULL,
    after_photo_url         TEXT NULL
);
CREATE INDEX work_items_area ON work_items (area_id);

CREATE TABLE project_phases (
    id                UUID PRIMARY KEY,
    work_item_id      UUID NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    phase_name        TEXT NOT NULL,
    order_index       INTEGER NOT NULL,
    weight_percentage INTEGER NOT NULL CHECK (weight_percentage BETWEEN 0 AND 100),
    is_completed      BOOLEAN NOT NULL DEFAULT FALSE,
    completed_at      TIMESTAMPTZ NULL,
    proof_photo_url   TEXT NULL
);
CREATE INDEX project_phases_item ON project_phases (work_item_id, order_index);

CREATE TABLE manpower_ledger_entries (
    id                  UUID PRIMARY KEY,
    work_item_id        UUID NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    source_work_item_id UUID NULL REFERENCES work_items(id),
    headcount_delta     INTEGER NOT NULL,
    resulting_headcount INTEGER NOT NULL CHECK (resulting_headcount >= 0),
    reason              TEXT NOT NULL,
    logged_by           UUID NOT NULL REFERENCES users(id),
    "timestamp"         TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX manpower_ledger_item ON manpower_ledger_entries (work_item_id, "timestamp");

CREATE TABLE decision_alerts (
    id                      UUID PRIMARY KEY,
    work_item_id            UUID NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    area_id                 UUID NOT NULL REFERENCES spatial_areas(id),
    urgency                 TEXT NOT NULL,
    category                TEXT NOT NULL,
    title                   TEXT NOT NULL,
    issue_description       TEXT NOT NULL,
    mitigation_action_taken TEXT NOT NULL,
    option_a_label          TEXT NULL,
    option_b_label          TEXT NULL,
    status                  TEXT NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at             TIMESTAMPTZ NULL
);
CREATE INDEX decision_alerts_status ON decision_alerts (status);

CREATE TABLE assets (
    id                  UUID PRIMARY KEY,
    name                TEXT NOT NULL,
    mobility            TEXT NOT NULL,
    current_area_id     UUID NOT NULL REFERENCES spatial_areas(id),
    parent_asset_id     UUID NULL REFERENCES assets(id),
    health              TEXT NOT NULL,
    active_issue_report TEXT NULL,
    impact_level        TEXT NOT NULL,
    repair_eta          TIMESTAMPTZ NULL
);
CREATE INDEX assets_area ON assets (current_area_id);

CREATE TABLE asset_transfer_logs (
    id           UUID PRIMARY KEY,
    asset_id     UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    from_area_id UUID NULL REFERENCES spatial_areas(id),
    to_area_id   UUID NOT NULL REFERENCES spatial_areas(id),
    moved_by     UUID NOT NULL REFERENCES users(id),
    "timestamp"  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE site_photos (
    id                  UUID PRIMARY KEY,
    work_item_id        UUID NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    area_id             UUID NOT NULL REFERENCES spatial_areas(id),
    image_url           TEXT NOT NULL,
    caption             TEXT NOT NULL,
    uploaded_by         UUID NOT NULL REFERENCES users(id),
    uploaded_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    client_acknowledged BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX site_photos_item ON site_photos (work_item_id, uploaded_at);

CREATE TABLE photo_call_outs (
    id             UUID PRIMARY KEY,
    photo_id       UUID NOT NULL REFERENCES site_photos(id) ON DELETE CASCADE,
    x_percent      REAL NOT NULL CHECK (x_percent BETWEEN 0 AND 100),
    y_percent      REAL NOT NULL CHECK (y_percent BETWEEN 0 AND 100),
    client_comment TEXT NOT NULL,
    staff_reply    TEXT NULL,
    status         TEXT NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at    TIMESTAMPTZ NULL
);

CREATE TABLE timeline_event_records (
    id             UUID PRIMARY KEY,
    work_item_id   UUID NOT NULL REFERENCES work_items(id) ON DELETE CASCADE,
    title          TEXT NOT NULL,
    description    TEXT NOT NULL,
    is_milestone   BOOLEAN NOT NULL DEFAULT FALSE,
    delay_category TEXT NOT NULL,
    days_delayed   INTEGER NOT NULL DEFAULT 0 CHECK (days_delayed >= 0),
    "timestamp"    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX timeline_item ON timeline_event_records (work_item_id, "timestamp");

CREATE TABLE estate_agenda_items (
    id                  UUID PRIMARY KEY,
    area_id             UUID NULL REFERENCES spatial_areas(id),
    work_item_id        UUID NULL REFERENCES work_items(id) ON DELETE CASCADE,
    title               TEXT NOT NULL,
    description         TEXT NOT NULL,
    event_type          TEXT NOT NULL,
    intrusion_level     TEXT NOT NULL,
    start_time          TIMESTAMPTZ NOT NULL,
    end_time            TIMESTAMPTZ NOT NULL CHECK (end_time >= start_time),
    is_flagged_conflict BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX agenda_range ON estate_agenda_items (start_time, end_time);

CREATE TABLE executive_handover_reports (
    id                     UUID PRIMARY KEY,
    work_item_id           UUID NOT NULL UNIQUE REFERENCES work_items(id) ON DELETE CASCADE,
    before_photo_url       TEXT NOT NULL,
    after_photo_url        TEXT NOT NULL,
    original_deadline      TIMESTAMPTZ NOT NULL,
    actual_completion_date TIMESTAMPTZ NOT NULL,
    total_delay_days       INTEGER NOT NULL DEFAULT 0,
    delay_audit_summary    JSONB NOT NULL DEFAULT '[]'::jsonb,
    handed_over_assets     JSONB NOT NULL DEFAULT '[]'::jsonb,
    client_signed_off      BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE notifications (
    id           UUID PRIMARY KEY,
    audience     TEXT NOT NULL,
    kind         TEXT NOT NULL,
    work_item_id UUID NULL REFERENCES work_items(id) ON DELETE CASCADE,
    title        TEXT NOT NULL,
    body         TEXT NOT NULL,
    lines        JSONB NOT NULL DEFAULT '[]'::jsonb,
    visible_from TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    dismissed    BOOLEAN NOT NULL DEFAULT FALSE,
    dedupe_key   TEXT NULL UNIQUE
);
CREATE INDEX notifications_audience ON notifications (audience, dismissed);

CREATE TABLE equipment_reservations (
    id             UUID PRIMARY KEY,
    asset_id       UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    agenda_item_id UUID NOT NULL REFERENCES estate_agenda_items(id) ON DELETE CASCADE,
    UNIQUE (asset_id, agenda_item_id)
);
"#;

const DOWN: &str = r#"
DROP TABLE IF EXISTS equipment_reservations;
DROP TABLE IF EXISTS notifications;
DROP TABLE IF EXISTS executive_handover_reports;
DROP TABLE IF EXISTS estate_agenda_items;
DROP TABLE IF EXISTS timeline_event_records;
DROP TABLE IF EXISTS photo_call_outs;
DROP TABLE IF EXISTS site_photos;
DROP TABLE IF EXISTS asset_transfer_logs;
DROP TABLE IF EXISTS assets;
DROP TABLE IF EXISTS decision_alerts;
DROP TABLE IF EXISTS manpower_ledger_entries;
DROP TABLE IF EXISTS project_phases;
DROP TABLE IF EXISTS work_items;
DROP TABLE IF EXISTS spatial_areas;
DROP TABLE IF EXISTS base_map_versions;
DROP TABLE IF EXISTS users;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(DOWN).await?;
        Ok(())
    }
}
