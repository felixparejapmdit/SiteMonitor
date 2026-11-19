//! Automated background jobs (Phase 2 §3.3), run in the estate's local time:
//!
//! 1. Crew roster reset (default 00:00) + roster prompt push (default 07:30).
//! 2. Dual-warning deadline monitor, 7 days & 48 hours (default 07:00).
//! 3. VIP daily digest (default 17:00).
//!
//! The times live in the estate settings and can be changed by the office.

use chrono::{DateTime, Local, NaiveTime, Timelike, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, Set, TransactionTrait};
use shared::api::{AutomationJob, JobRunResult};
use shared::ws::{DataScope, WsEvent};
use shared::*;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::convert::{self, enc};
use crate::entities::{manpower_ledger_entry, notification, prelude::*, site_photo, timeline_event_record};
use crate::error::AppResult;
use crate::services::{create_notice, load_settings, system_user, NewNotice};
use crate::state::AppState;

/// Starts the scheduler: one tick a minute compares the local time with the
/// times saved in the estate settings, so the office can change them live.
pub async fn start(state: AppState) -> anyhow::Result<JobScheduler> {
    let sched = JobScheduler::new().await?;
    sched
        .add(Job::new_async_tz("0 * * * * *", Local, move |_id, _l| {
            let s = state.clone();
            Box::pin(async move { tick(&s).await })
        })?)
        .await?;
    sched.start().await?;
    tracing::info!("automation scheduler started (job times come from the estate settings)");
    Ok(sched)
}

async fn tick(state: &AppState) {
    let settings = match load_settings(&state.db).await {
        Ok(s) => s,
        Err(e) => return tracing::warn!(error = %e, "couldn't read estate settings; skipping automation tick"),
    };
    let now = Local::now().format("%H:%M").to_string();
    if now == settings.roster_reset_time {
        log_run(run(state, AutomationJob::MidnightRosterReset).await);
    }
    if now == settings.roster_prompt_time {
        if let Err(e) = push_roster_prompt(state).await {
            tracing::warn!(error = %e, "roster prompt push failed");
        }
    }
    if now == settings.deadline_check_time {
        log_run(run(state, AutomationJob::DeadlineMonitor).await);
    }
    if now == settings.digest_time {
        log_run(run(state, AutomationJob::VipDailyDigest).await);
    }
}

/// Parses a saved "HH:MM" into (hour, minute).
fn hm(t: &str) -> (u32, u32) {
    NaiveTime::parse_from_str(t, "%H:%M").map(|t| (t.hour(), t.minute())).unwrap_or((7, 30))
}

fn log_run(result: AppResult<JobRunResult>) {
    match result {
        Ok(r) => tracing::info!(job = ?r.job, summary = %r.summary, "automation job finished"),
        Err(e) => tracing::error!(error = %e, "automation job failed"),
    }
}

/// Runs a job immediately (cron or the admin "run now" button).
pub async fn run(state: &AppState, job: AutomationJob) -> AppResult<JobRunResult> {
    let summary = match job {
        AutomationJob::MidnightRosterReset => midnight_roster_reset(state).await?,
        AutomationJob::DeadlineMonitor => deadline_monitor(state).await?,
        AutomationJob::VipDailyDigest => vip_daily_digest(state).await?,
    };
    Ok(JobRunResult { job, summary })
}

/// Today at a local wall-clock time, as UTC.
fn today_at(h: u32, m: u32) -> DateTime<Utc> {
    let t = NaiveTime::from_hms_opt(h, m, 0).unwrap_or_default();
    Local::now()
        .date_naive()
        .and_time(t)
        .and_local_timezone(Local)
        .single()
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(Utc::now)
}

/// 1. Clears yesterday's crew so they never become "ghost workers" and
///    queues the 07:30 "Allocate Today's Crew" prompt for the Office Admin.
async fn midnight_roster_reset(state: &AppState) -> AppResult<String> {
    let prompt_time = load_settings(&state.db).await?.roster_prompt_time;
    let (ph, pm) = hm(&prompt_time);
    let txn = state.db.begin().await?;
    let logger = system_user(&txn).await?;
    let mut cleared = 0;
    for row in WorkItems::find().all(&txn).await? {
        let count = row.active_worker_count;
        if count > 0 {
            manpower_ledger_entry::ActiveModel {
                id: Set(Uuid::new_v4()),
                work_item_id: Set(row.id),
                source_work_item_id: Set(None),
                headcount_delta: Set(-count),
                resulting_headcount: Set(0),
                reason: Set("Midnight reset — yesterday's crew cleared".into()),
                logged_by: Set(logger),
                timestamp: Set(Utc::now()),
            }
            .insert(&txn)
            .await?;
            cleared += count;
        }
        let mut active = row.into_active_model();
        active.active_worker_count = Set(0);
        active.update(&txn).await?;
    }
    // Older unanswered prompts are superseded by today's.
    for old in Notifications::find()
        .filter(notification::Column::Kind.eq(enc(&NotificationKind::MorningRosterPrompt)))
        .filter(notification::Column::Dismissed.eq(false))
        .all(&txn)
        .await?
    {
        let mut a = old.into_active_model();
        a.dismissed = Set(true);
        a.update(&txn).await?;
    }
    let date = Local::now().date_naive();
    create_notice(
        &txn,
        NewNotice {
            audience: UserRole::OfficeAdmin,
            kind: NotificationKind::MorningRosterPrompt,
            work_item_id: None,
            title: "Morning Roster Reset: Allocate Today's Crew".into(),
            body: "Last night's crew numbers were cleared. Set how many workers are on each job today.".into(),
            lines: vec![],
            visible_from: today_at(ph, pm),
            dedupe_key: Some(format!("roster:{date}")),
        },
    )
    .await?;
    txn.commit().await?;
    state.hub.changed(&[DataScope::WorkItems, DataScope::Notifications, DataScope::Map]).await;
    Ok(format!("Cleared {cleared} workers from yesterday; morning roster prompt queued for {prompt_time}."))
}

/// Roster prompt time — nudge connected admins in real time.
async fn push_roster_prompt(state: &AppState) -> AppResult<()> {
    let prompt = Notifications::find()
        .filter(notification::Column::Kind.eq(enc(&NotificationKind::MorningRosterPrompt)))
        .filter(notification::Column::Dismissed.eq(false))
        .one(&state.db)
        .await?;
    if let Some(p) = prompt {
        state.hub.send_to(&[UserRole::OfficeAdmin], &WsEvent::StaffNotice(convert::notification(p))).await;
        state.hub.changed(&[DataScope::Notifications]).await;
    }
    Ok(())
}

/// 2. Warns staff 7 days and 48 hours before each target deadline and adds a
///    polished anticipation card to the owner's agenda at 48 hours.
async fn deadline_monitor(state: &AppState) -> AppResult<String> {
    let now = Utc::now();
    let mut warnings = 0;
    let mut cards = 0;
    for item in WorkItems::find().all(&state.db).await?.into_iter().map(convert::work_item) {
        if !matches!(item.status, WorkStatus::Planning | WorkStatus::InProgress | WorkStatus::Halted) {
            continue;
        }
        let hours = (item.current_target_deadline - now).num_hours();
        let threshold = if (0..=48).contains(&hours) {
            Some("48h")
        } else if (49..=168).contains(&hours) {
            Some("7d")
        } else {
            None
        };
        let Some(threshold) = threshold else { continue };
        let deadline_key = item.current_target_deadline.format("%Y%m%d");

        for role in [UserRole::OfficeAdmin, UserRole::SiteMonitor] {
            let created = create_notice(
                &state.db,
                NewNotice {
                    audience: role,
                    kind: NotificationKind::DeadlineWarning,
                    work_item_id: Some(item.id),
                    title: format!("{} — deadline approaching", item.title),
                    body: format!(
                        "Target deadline approaching in {hours} hours. Current progress: {}%. Reallocate manpower?",
                        item.calculated_progress
                    ),
                    lines: vec![],
                    visible_from: now,
                    dedupe_key: Some(format!("deadline:{threshold}:{}:{}:{deadline_key}", item.id, enc(&role))),
                },
            )
            .await?;
            if let Some(n) = created {
                warnings += 1;
                state.hub.send_to(&[role], &WsEvent::StaffNotice(n)).await;
            }
        }

        if threshold == "48h" {
            let created = create_notice(
                &state.db,
                NewNotice {
                    audience: UserRole::VipClient,
                    kind: NotificationKind::AnticipationCard,
                    work_item_id: Some(item.id),
                    title: format!("{} — handover in about 2 days", item.title),
                    body: format!(
                        "Your team is in the final stretch ({}% complete). The handover is planned for {}.",
                        item.calculated_progress,
                        item.current_target_deadline.with_timezone(&Local).format("%A, %B %-d")
                    ),
                    lines: vec![],
                    visible_from: now,
                    dedupe_key: Some(format!("anticipation:{}:{deadline_key}", item.id)),
                },
            )
            .await?;
            if created.is_some() {
                cards += 1;
            }
        }
    }
    state.hub.changed(&[DataScope::Notifications, DataScope::Agenda]).await;
    Ok(format!("{warnings} staff deadline warnings sent; {cards} owner anticipation cards added."))
}

/// 3. Batches the day's calm news into one 5:00 PM summary for the owner.
async fn vip_daily_digest(state: &AppState) -> AppResult<String> {
    let start = today_at(0, 0);
    let items: Vec<WorkItem> = WorkItems::find().all(&state.db).await?.into_iter().map(convert::work_item).collect();
    let title_of = |id: Uuid| items.iter().find(|w| w.id == id).map(|w| w.title.clone()).unwrap_or_default();
    let stream_of = |id: Uuid| items.iter().find(|w| w.id == id).map(|w| w.stream);

    let events: Vec<TimelineEventRecord> = TimelineEventRecords::find()
        .filter(timeline_event_record::Column::Timestamp.gte(start))
        .filter(timeline_event_record::Column::IsMilestone.eq(true))
        .all(&state.db)
        .await?
        .into_iter()
        .map(convert::timeline)
        .collect();
    let mut lines = Vec::new();
    for e in &events {
        match stream_of(e.work_item_id) {
            Some(WorkStream::RoutineMaintenance) => lines.push(format!("✓ {}", e.title)),
            _ => lines.push(format!("{} — {}", title_of(e.work_item_id), e.title)),
        }
    }
    let photos = SitePhotos::find()
        .filter(site_photo::Column::UploadedAt.gte(start))
        .all(&state.db)
        .await?
        .len();
    if photos > 0 {
        lines.push(format!("{} uploaded by the site team", super::status::plural(photos, "new photo", "new photos")));
    }
    if lines.is_empty() {
        return Ok("Nothing new today — no digest sent.".into());
    }
    let date = Local::now().date_naive();
    let created = create_notice(
        &state.db,
        NewNotice {
            audience: UserRole::VipClient,
            kind: NotificationKind::DailyDigest,
            work_item_id: None,
            title: format!("Your estate today — {}", Local::now().format("%A, %B %-d")),
            body: format!("{} from today, gathered into one calm summary.", super::status::plural(lines.len(), "update", "updates")),
            lines: lines.clone(),
            visible_from: Utc::now(),
            dedupe_key: Some(format!("digest:{date}")),
        },
    )
    .await?;
    match created {
        Some(n) => {
            state.hub.to_vip(WsEvent::DailyDigest(n)).await;
            state.hub.changed(&[DataScope::Notifications]).await;
            Ok(format!("Daily digest sent with {} updates.", lines.len()))
        }
        None => Ok("Today's digest was already sent.".into()),
    }
}
