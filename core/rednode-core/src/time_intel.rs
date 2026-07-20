// RedNode-OS — Time Intelligence
//
// Makes RedNode aware of time — not just "what time is it?" but:
//   - What should happen now vs. later?
//   - Is this a work day or a weekend?
//   - What recurring tasks are due?
//   - When do certificates expire?
//   - What happened on this day before?
//
// Time Intelligence replaces scattered cron jobs with a unified,
// consciousness-aware scheduler. The consciousness layer reads
// time_intel to know "what should happen next" at any moment.
//
// Built-in schedules (user-configurable via API/dashboard):
//   - Daily 3:00 AM IST  — security scan, SMART check, memory consolidation
//   - Weekly Sunday 2 AM — NixOS garbage-collect, Docker prune
//   - Monthly 1st        — backup verify, storage trend report
//   - Patch Tuesday      — check security updates (2nd Tuesday)
//   - Certificate expiry — 30/14/7/1 day warnings
//
// Timezone: respects REDNODE_TZ env var, defaults to Asia/Kolkata
//
// API:
//   GET    /time                       — current time awareness state
//   GET    /time/events                — all scheduled events
//   POST   /time/events                — create a time event
//   DELETE /time/events/:id            — remove a time event
//   GET    /time/patterns              — recurring patterns
//   POST   /time/patterns              — create a recurring pattern
//   DELETE /time/patterns/:id          — remove a recurring pattern
//   GET    /time/deadlines             — active deadlines
//   POST   /time/deadlines             — create a deadline
//   GET    /time/due                   — events/patterns due right now

use chrono::{DateTime, Datelike, Duration, Local, NaiveTime, TimeZone, Timelike, Utc, Weekday};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static TIME_STATE: once_cell::sync::Lazy<Arc<RwLock<TimeStore>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(TimeStore::default())));

// ─── Core Types ───

/// Everything RedNode knows about time right now
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeAwareness {
    pub current_period: TimePeriod,
    pub is_work_hours: bool,
    pub day_type: DayType,
    pub day_of_week: String,
    pub local_time: String,
    pub utc_time: String,
    pub upcoming_events: Vec<TimeEvent>,
    pub overdue_patterns: Vec<RecurringPattern>,
    pub active_deadlines: Vec<Deadline>,
    pub next_scheduled: Option<ScheduledItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TimePeriod {
    EarlyMorning,
    Morning,
    Afternoon,
    Evening,
    Night,
    LateNight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DayType {
    Weekday,
    Weekend,
    Holiday(String),
}

/// A one-off or calendar event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEvent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub when: DateTime<Utc>,
    pub category: EventCategory,
    pub auto_execute: bool,
    pub action: Option<String>,
    pub executed: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventCategory {
    Maintenance,
    Backup,
    Security,
    Personal,
    Monitoring,
    Cleanup,
    Update,
    Custom(String),
}

/// A recurring task defined by a cron-like schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringPattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub schedule: Schedule,
    pub action: String,
    pub category: EventCategory,
    pub auto_execute: bool,
    pub last_executed: Option<DateTime<Utc>>,
    pub next_execution: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub execution_count: u64,
    pub last_result: Option<String>,
}

/// Schedule specification — simplified, no cron parsing dependency needed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Schedule {
    /// Run every N seconds
    Interval { seconds: u64 },
    /// Run daily at this hour:minute (local time)
    Daily { hour: u32, minute: u32 },
    /// Run weekly on this day at hour:minute
    Weekly { day: Weekday, hour: u32, minute: u32 },
    /// Run monthly on this day at hour:minute
    Monthly { day_of_month: u32, hour: u32, minute: u32 },
    /// Run on the Nth weekday of the month (e.g., 2nd Tuesday = Patch Tuesday)
    NthWeekday { nth: u32, day: Weekday, hour: u32, minute: u32 },
}

/// A deadline with urgency tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deadline {
    pub id: String,
    pub name: String,
    pub description: String,
    pub due: DateTime<Utc>,
    pub category: EventCategory,
    pub warning_days: Vec<u32>,
    pub notified_at: Vec<DateTime<Utc>>,
    pub completed: bool,
}

/// Next item that is scheduled to happen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledItem {
    pub name: String,
    pub due_at: DateTime<Utc>,
    pub minutes_until: i64,
    pub auto_execute: bool,
    pub source: String,
}

/// The persistent store for all time-related data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeStore {
    pub events: Vec<TimeEvent>,
    pub patterns: Vec<RecurringPattern>,
    pub deadlines: Vec<Deadline>,
    pub holidays: Vec<Holiday>,
    pub work_hours: WorkHours,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Holiday {
    pub name: String,
    pub month: u32,
    pub day: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkHours {
    pub start_hour: u32,
    pub end_hour: u32,
    pub work_days: Vec<Weekday>,
}

impl Default for TimeStore {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            patterns: default_patterns(),
            deadlines: Vec::new(),
            holidays: default_holidays(),
            work_hours: WorkHours {
                start_hour: 9,
                end_hour: 18,
                work_days: vec![
                    Weekday::Mon,
                    Weekday::Tue,
                    Weekday::Wed,
                    Weekday::Thu,
                    Weekday::Fri,
                ],
            },
        }
    }
}

/// Built-in recurring patterns — the things RedNode should do automatically
fn default_patterns() -> Vec<RecurringPattern> {
    vec![
        RecurringPattern {
            id: "pat_daily_security".into(),
            name: "Daily security scan".into(),
            description: "Run security audit, check for intrusions, verify firewall rules".into(),
            schedule: Schedule::Daily { hour: 3, minute: 0 },
            action: "run security audit and check for intrusions".into(),
            category: EventCategory::Security,
            auto_execute: true,
            last_executed: None,
            next_execution: None,
            enabled: true,
            execution_count: 0,
            last_result: None,
        },
        RecurringPattern {
            id: "pat_daily_smart".into(),
            name: "SMART health check".into(),
            description: "Check all disk SMART attributes for degradation".into(),
            schedule: Schedule::Daily { hour: 3, minute: 15 },
            action: "check disk smart health on all machines".into(),
            category: EventCategory::Maintenance,
            auto_execute: true,
            last_executed: None,
            next_execution: None,
            enabled: true,
            execution_count: 0,
            last_result: None,
        },
        RecurringPattern {
            id: "pat_daily_consolidate".into(),
            name: "Memory consolidation".into(),
            description: "Consolidate learnings, promote patterns, archive old memories".into(),
            schedule: Schedule::Daily { hour: 3, minute: 30 },
            action: "consolidate memory and promote learned patterns".into(),
            category: EventCategory::Maintenance,
            auto_execute: true,
            last_executed: None,
            next_execution: None,
            enabled: true,
            execution_count: 0,
            last_result: None,
        },
        RecurringPattern {
            id: "pat_weekly_gc".into(),
            name: "Weekly cleanup".into(),
            description: "NixOS garbage collect, Docker prune, clear temp files".into(),
            schedule: Schedule::Weekly { day: Weekday::Sun, hour: 2, minute: 0 },
            action: "run nix garbage collection and docker system prune".into(),
            category: EventCategory::Cleanup,
            auto_execute: true,
            last_executed: None,
            next_execution: None,
            enabled: true,
            execution_count: 0,
            last_result: None,
        },
        RecurringPattern {
            id: "pat_monthly_backup".into(),
            name: "Monthly backup verification".into(),
            description: "Verify backup integrity, test restore, report storage trends".into(),
            schedule: Schedule::Monthly { day_of_month: 1, hour: 4, minute: 0 },
            action: "verify backup integrity and generate storage trend report".into(),
            category: EventCategory::Backup,
            auto_execute: false,
            last_executed: None,
            next_execution: None,
            enabled: true,
            execution_count: 0,
            last_result: None,
        },
        RecurringPattern {
            id: "pat_patch_tuesday".into(),
            name: "Patch Tuesday check".into(),
            description: "Check for security updates on the 2nd Tuesday of each month".into(),
            schedule: Schedule::NthWeekday { nth: 2, day: Weekday::Tue, hour: 10, minute: 0 },
            action: "check for security updates and CVEs affecting our systems".into(),
            category: EventCategory::Security,
            auto_execute: false,
            last_executed: None,
            next_execution: None,
            enabled: true,
            execution_count: 0,
            last_result: None,
        },
    ]
}

/// Default holidays (Indian public holidays — user can customize)
fn default_holidays() -> Vec<Holiday> {
    vec![
        Holiday { name: "Republic Day".into(), month: 1, day: 26 },
        Holiday { name: "Independence Day".into(), month: 8, day: 15 },
        Holiday { name: "Gandhi Jayanti".into(), month: 10, day: 2 },
        Holiday { name: "New Year".into(), month: 1, day: 1 },
    ]
}

fn gen_id(prefix: &str) -> String {
    format!("{}_{}", prefix, chrono::Utc::now().timestamp_millis())
}

// ─── Time Calculation Helpers ───

fn current_period() -> TimePeriod {
    let hour = Local::now().hour();
    match hour {
        0..=4 => TimePeriod::LateNight,
        5..=7 => TimePeriod::EarlyMorning,
        8..=11 => TimePeriod::Morning,
        12..=16 => TimePeriod::Afternoon,
        17..=21 => TimePeriod::Evening,
        _ => TimePeriod::Night,
    }
}

fn current_day_type(store: &TimeStore) -> DayType {
    let now = Local::now();
    let month = now.month();
    let day = now.day();

    // Check holidays
    for h in &store.holidays {
        if h.month == month && h.day == day {
            return DayType::Holiday(h.name.clone());
        }
    }

    match now.weekday() {
        Weekday::Sat | Weekday::Sun => DayType::Weekend,
        _ => DayType::Weekday,
    }
}

fn is_work_hours(store: &TimeStore) -> bool {
    let now = Local::now();
    let hour = now.hour();
    let day = now.weekday();

    store.work_days_contains(day) && hour >= store.work_hours.start_hour && hour < store.work_hours.end_hour
}

impl TimeStore {
    fn work_days_contains(&self, day: Weekday) -> bool {
        self.work_hours.work_days.contains(&day)
    }
}

/// Calculate the next execution time for a schedule, given a reference "now"
fn next_execution_for(schedule: &Schedule, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let local_now = now.with_timezone(&chrono::FixedOffset::east_opt(5 * 3600 + 30 * 60).unwrap());
    let today = local_now.date_naive();

    match schedule {
        Schedule::Interval { seconds } => {
            Some(now + Duration::seconds(*seconds as i64))
        }
        Schedule::Daily { hour, minute } => {
            let target_time = NaiveTime::from_hms_opt(*hour, *minute, 0)?;
            let target_today = today.and_time(target_time);
            let target_utc = chrono::FixedOffset::east_opt(5 * 3600 + 30 * 60)?
                .from_local_datetime(&target_today)
                .single()?
                .with_timezone(&Utc);

            if target_utc > now {
                Some(target_utc)
            } else {
                let tomorrow = today + Duration::days(1);
                let target_tomorrow = tomorrow.and_time(target_time);
                chrono::FixedOffset::east_opt(5 * 3600 + 30 * 60)?
                    .from_local_datetime(&target_tomorrow)
                    .single()
                    .map(|dt| dt.with_timezone(&Utc))
            }
        }
        Schedule::Weekly { day, hour, minute } => {
            let target_time = NaiveTime::from_hms_opt(*hour, *minute, 0)?;
            let days_ahead = (*day as i64 - local_now.weekday() as i64 + 7) % 7;
            let target_date = today + Duration::days(days_ahead);
            let target_dt = target_date.and_time(target_time);
            let target_utc = chrono::FixedOffset::east_opt(5 * 3600 + 30 * 60)?
                .from_local_datetime(&target_dt)
                .single()?
                .with_timezone(&Utc);

            if target_utc > now {
                Some(target_utc)
            } else {
                // Next week
                let next_date = target_date + Duration::days(7);
                let next_dt = next_date.and_time(target_time);
                chrono::FixedOffset::east_opt(5 * 3600 + 30 * 60)?
                    .from_local_datetime(&next_dt)
                    .single()
                    .map(|dt| dt.with_timezone(&Utc))
            }
        }
        Schedule::Monthly { day_of_month, hour, minute } => {
            let target_time = NaiveTime::from_hms_opt(*hour, *minute, 0)?;
            let this_month = chrono::NaiveDate::from_ymd_opt(
                local_now.year(),
                local_now.month(),
                (*day_of_month).min(28),
            )?;
            let target_dt = this_month.and_time(target_time);
            let target_utc = chrono::FixedOffset::east_opt(5 * 3600 + 30 * 60)?
                .from_local_datetime(&target_dt)
                .single()?
                .with_timezone(&Utc);

            if target_utc > now {
                Some(target_utc)
            } else {
                // Next month
                let (next_year, next_month) = if local_now.month() == 12 {
                    (local_now.year() + 1, 1)
                } else {
                    (local_now.year(), local_now.month() + 1)
                };
                let next_date = chrono::NaiveDate::from_ymd_opt(
                    next_year,
                    next_month,
                    (*day_of_month).min(28),
                )?;
                let next_dt = next_date.and_time(target_time);
                chrono::FixedOffset::east_opt(5 * 3600 + 30 * 60)?
                    .from_local_datetime(&next_dt)
                    .single()
                    .map(|dt| dt.with_timezone(&Utc))
            }
        }
        Schedule::NthWeekday { nth, day, hour, minute } => {
            let target_time = NaiveTime::from_hms_opt(*hour, *minute, 0)?;

            // Find the Nth weekday of the current month
            let first_of_month = chrono::NaiveDate::from_ymd_opt(
                local_now.year(),
                local_now.month(),
                1,
            )?;
            let mut count = 0u32;
            let mut target_date = first_of_month;
            loop {
                if target_date.weekday() == *day {
                    count += 1;
                    if count == *nth {
                        break;
                    }
                }
                target_date += Duration::days(1);
                if target_date.month() != first_of_month.month() {
                    return None; // Nth weekday doesn't exist this month
                }
            }

            let target_dt = target_date.and_time(target_time);
            let target_utc = chrono::FixedOffset::east_opt(5 * 3600 + 30 * 60)?
                .from_local_datetime(&target_dt)
                .single()?
                .with_timezone(&Utc);

            if target_utc > now {
                Some(target_utc)
            } else {
                // Try next month
                let (next_year, next_month) = if local_now.month() == 12 {
                    (local_now.year() + 1, 1)
                } else {
                    (local_now.year(), local_now.month() + 1)
                };
                let first_next = chrono::NaiveDate::from_ymd_opt(next_year, next_month, 1)?;
                let mut count2 = 0u32;
                let mut target2 = first_next;
                loop {
                    if target2.weekday() == *day {
                        count2 += 1;
                        if count2 == *nth {
                            break;
                        }
                    }
                    target2 += Duration::days(1);
                    if target2.month() != first_next.month() {
                        return None;
                    }
                }
                let next_dt = target2.and_time(target_time);
                chrono::FixedOffset::east_opt(5 * 3600 + 30 * 60)?
                    .from_local_datetime(&next_dt)
                    .single()
                    .map(|dt| dt.with_timezone(&Utc))
            }
        }
    }
}

// ─── Public API: Read ───

/// Get the complete time awareness state (what consciousness reads)
pub async fn get_awareness() -> TimeAwareness {
    let store = TIME_STATE.read().await;
    let now = Utc::now();
    let local = Local::now();

    // Collect upcoming events (next 24h, not yet executed)
    let upcoming: Vec<_> = store.events.iter()
        .filter(|e| !e.executed && e.when > now && (e.when - now).num_hours() < 24)
        .cloned()
        .collect();

    // Collect overdue patterns
    let overdue: Vec<_> = store.patterns.iter()
        .filter(|p| {
            p.enabled && p.next_execution.map_or(false, |next| next <= now)
        })
        .cloned()
        .collect();

    // Collect active deadlines (not completed, not past)
    let deadlines: Vec<_> = store.deadlines.iter()
        .filter(|d| !d.completed)
        .cloned()
        .collect();

    // Find the very next scheduled item
    let mut next_items: Vec<ScheduledItem> = Vec::new();

    for event in &store.events {
        if !event.executed && event.when > now {
            next_items.push(ScheduledItem {
                name: event.name.clone(),
                due_at: event.when,
                minutes_until: (event.when - now).num_minutes(),
                auto_execute: event.auto_execute,
                source: "event".into(),
            });
        }
    }

    for pattern in &store.patterns {
        if pattern.enabled {
            if let Some(next) = pattern.next_execution {
                if next > now {
                    next_items.push(ScheduledItem {
                        name: pattern.name.clone(),
                        due_at: next,
                        minutes_until: (next - now).num_minutes(),
                        auto_execute: pattern.auto_execute,
                        source: "pattern".into(),
                    });
                }
            }
        }
    }

    next_items.sort_by_key(|i| i.due_at);
    let next_scheduled = next_items.into_iter().next();

    TimeAwareness {
        current_period: current_period(),
        is_work_hours: is_work_hours(&store),
        day_type: current_day_type(&store),
        day_of_week: format!("{:?}", local.weekday()),
        local_time: local.format("%H:%M:%S").to_string(),
        utc_time: now.format("%H:%M:%S").to_string(),
        upcoming_events: upcoming,
        overdue_patterns: overdue,
        active_deadlines: deadlines,
        next_scheduled,
    }
}

/// Get all events
pub async fn get_events() -> Vec<TimeEvent> {
    TIME_STATE.read().await.events.clone()
}

/// Get all patterns
pub async fn get_patterns() -> Vec<RecurringPattern> {
    TIME_STATE.read().await.patterns.clone()
}

/// Get all deadlines
pub async fn get_deadlines() -> Vec<Deadline> {
    TIME_STATE.read().await.deadlines.clone()
}

/// Get items that are due right now (events past their time, overdue patterns)
pub async fn get_due_items() -> Vec<ScheduledItem> {
    let store = TIME_STATE.read().await;
    let now = Utc::now();
    let mut due = Vec::new();

    // Events that are past their scheduled time but not yet executed
    for event in &store.events {
        if !event.executed && event.when <= now {
            due.push(ScheduledItem {
                name: event.name.clone(),
                due_at: event.when,
                minutes_until: (event.when - now).num_minutes(),
                auto_execute: event.auto_execute,
                source: "event".into(),
            });
        }
    }

    // Overdue patterns
    for pattern in &store.patterns {
        if pattern.enabled {
            if let Some(next) = pattern.next_execution {
                if next <= now {
                    due.push(ScheduledItem {
                        name: pattern.name.clone(),
                        due_at: next,
                        minutes_until: (next - now).num_minutes(),
                        auto_execute: pattern.auto_execute,
                        source: "pattern".into(),
                    });
                }
            }
        }
    }

    due
}

// ─── Public API: Write ───

/// Create a one-off time event
pub async fn create_event(
    name: &str,
    description: &str,
    when: DateTime<Utc>,
    category: EventCategory,
    auto_execute: bool,
    action: Option<String>,
) -> TimeEvent {
    let event = TimeEvent {
        id: gen_id("evt"),
        name: name.into(),
        description: description.into(),
        when,
        category,
        auto_execute,
        action,
        executed: false,
        created_at: Utc::now(),
    };

    let mut store = TIME_STATE.write().await;
    store.events.push(event.clone());

    tracing::info!(name = name, when = %when, "Time event created");

    crate::events::emit(serde_json::json!({
        "type": "time_event_created",
        "event_name": name,
        "when": when.to_rfc3339(),
        "ts": Utc::now().to_rfc3339(),
    }));

    event
}

/// Remove a time event by ID
pub async fn remove_event(id: &str) -> bool {
    let mut store = TIME_STATE.write().await;
    let before = store.events.len();
    store.events.retain(|e| e.id != id);
    store.events.len() < before
}

/// Mark an event as executed
pub async fn mark_event_executed(id: &str) {
    let mut store = TIME_STATE.write().await;
    if let Some(event) = store.events.iter_mut().find(|e| e.id == id) {
        event.executed = true;
    }
}

/// Create a recurring pattern
pub async fn create_pattern(
    name: &str,
    description: &str,
    schedule: Schedule,
    action: &str,
    category: EventCategory,
    auto_execute: bool,
) -> RecurringPattern {
    let now = Utc::now();
    let next = next_execution_for(&schedule, now);

    let pattern = RecurringPattern {
        id: gen_id("pat"),
        name: name.into(),
        description: description.into(),
        schedule,
        action: action.into(),
        category,
        auto_execute,
        last_executed: None,
        next_execution: next,
        enabled: true,
        execution_count: 0,
        last_result: None,
    };

    let mut store = TIME_STATE.write().await;
    store.patterns.push(pattern.clone());

    tracing::info!(name = name, next = ?next, "Recurring pattern created");
    pattern
}

/// Remove a recurring pattern by ID
pub async fn remove_pattern(id: &str) -> bool {
    let mut store = TIME_STATE.write().await;
    let before = store.patterns.len();
    store.patterns.retain(|p| p.id != id);
    store.patterns.len() < before
}

/// Record that a pattern was executed, advance its next_execution
pub async fn pattern_executed(id: &str, result: Option<String>) {
    let mut store = TIME_STATE.write().await;
    if let Some(pattern) = store.patterns.iter_mut().find(|p| p.id == id) {
        let now = Utc::now();
        pattern.last_executed = Some(now);
        pattern.execution_count += 1;
        pattern.last_result = result;
        pattern.next_execution = next_execution_for(&pattern.schedule, now);
    }
}

/// Create a deadline
pub async fn create_deadline(
    name: &str,
    description: &str,
    due: DateTime<Utc>,
    category: EventCategory,
    warning_days: Vec<u32>,
) -> Deadline {
    let deadline = Deadline {
        id: gen_id("dl"),
        name: name.into(),
        description: description.into(),
        due,
        category,
        warning_days,
        notified_at: Vec::new(),
        completed: false,
    };

    let mut store = TIME_STATE.write().await;
    store.deadlines.push(deadline.clone());

    tracing::info!(name = name, due = %due, "Deadline created");
    deadline
}

/// Mark a deadline as completed
pub async fn complete_deadline(id: &str) -> bool {
    let mut store = TIME_STATE.write().await;
    if let Some(dl) = store.deadlines.iter_mut().find(|d| d.id == id) {
        dl.completed = true;
        return true;
    }
    false
}

/// Remove a deadline
pub async fn remove_deadline(id: &str) -> bool {
    let mut store = TIME_STATE.write().await;
    let before = store.deadlines.len();
    store.deadlines.retain(|d| d.id != id);
    store.deadlines.len() < before
}

// ─── Background Tick ───

/// Called by the consciousness loop every tick.
/// Computes next execution times, checks deadlines, and returns items that are due.
pub async fn tick() -> Vec<DueAction> {
    let mut store = TIME_STATE.write().await;
    let now = Utc::now();
    let mut due_actions = Vec::new();

    // Recalculate next_execution for all patterns that haven't been computed yet
    for pattern in &mut store.patterns {
        if pattern.enabled && pattern.next_execution.is_none() {
            pattern.next_execution = next_execution_for(&pattern.schedule, now);
        }
    }

    // Collect overdue auto-execute patterns
    for pattern in &mut store.patterns {
        if pattern.enabled && pattern.auto_execute {
            if let Some(next) = pattern.next_execution {
                if next <= now {
                    due_actions.push(DueAction {
                        source_id: pattern.id.clone(),
                        source_type: "pattern".into(),
                        name: pattern.name.clone(),
                        action: pattern.action.clone(),
                        auto_execute: true,
                    });
                }
            }
        }
    }

    // Collect overdue auto-execute events
    for event in &mut store.events {
        if !event.executed && event.auto_execute && event.when <= now {
            if let Some(ref action) = event.action {
                due_actions.push(DueAction {
                    source_id: event.id.clone(),
                    source_type: "event".into(),
                    name: event.name.clone(),
                    action: action.clone(),
                    auto_execute: true,
                });
            }
        }
    }

    // Check deadlines for warning notifications
    for deadline in &mut store.deadlines {
        if deadline.completed {
            continue;
        }

        let days_remaining = (deadline.due - now).num_days();
        if days_remaining < 0 {
            // Past due
            due_actions.push(DueAction {
                source_id: deadline.id.clone(),
                source_type: "deadline_overdue".into(),
                name: format!("OVERDUE: {}", deadline.name),
                action: format!("Deadline '{}' is overdue by {} days", deadline.name, -days_remaining),
                auto_execute: false,
            });
        } else {
            // Check warning thresholds
            for threshold in &deadline.warning_days {
                if days_remaining <= *threshold as i64 {
                    // Only notify once per threshold
                    let already_notified = deadline.notified_at.iter().any(|t| {
                        let days_at_notification = (deadline.due - *t).num_days();
                        days_at_notification <= *threshold as i64 && days_at_notification > (*threshold as i64 - 1)
                    });

                    if !already_notified {
                        deadline.notified_at.push(now);
                        due_actions.push(DueAction {
                            source_id: deadline.id.clone(),
                            source_type: "deadline_warning".into(),
                            name: format!("{} — {} days remaining", deadline.name, days_remaining),
                            action: format!(
                                "Deadline '{}' is due in {} days ({})",
                                deadline.name,
                                days_remaining,
                                deadline.due.format("%Y-%m-%d"),
                            ),
                            auto_execute: false,
                        });
                    }
                    break; // Only fire the closest threshold
                }
            }
        }
    }

    // Clean up old executed events (older than 7 days)
    let cutoff = now - Duration::days(7);
    store.events.retain(|e| !e.executed || e.when > cutoff);

    due_actions
}

/// An action that the consciousness layer should consider executing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DueAction {
    pub source_id: String,
    pub source_type: String,
    pub name: String,
    pub action: String,
    pub auto_execute: bool,
}

// ─── Persistence ───

/// Persist time state to PostgreSQL
pub async fn persist() {
    let store = TIME_STATE.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&store) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to serialize time state: {}", e);
                return;
            }
        };

        let _ = sqlx::query(
            "INSERT INTO time_intel_store (id, state, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()"
        )
        .bind(&json)
        .execute(pool)
        .await;
    }
}

/// Restore time state from PostgreSQL (called on boot)
pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT state FROM time_intel_store WHERE id = 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((state_json,)) = row {
            if let Ok(restored) = serde_json::from_value::<TimeStore>(state_json) {
                let event_count = restored.events.len();
                let pattern_count = restored.patterns.len();
                let deadline_count = restored.deadlines.len();

                let mut store = TIME_STATE.write().await;
                *store = restored;

                tracing::info!(
                    events = event_count,
                    patterns = pattern_count,
                    deadlines = deadline_count,
                    "Time intelligence restored from database"
                );
            }
        }
    }
}

/// Create the time_intel_store table if it does not exist
pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS time_intel_store (\
                id INTEGER PRIMARY KEY, \
                state JSONB NOT NULL, \
                updated_at TIMESTAMPTZ DEFAULT NOW()\
            )"
        )
        .execute(pool)
        .await;
    }
}

// ─── API Helpers ───

/// Get time state formatted for API response
pub async fn get_for_api() -> serde_json::Value {
    let awareness = get_awareness().await;
    serde_json::json!({
        "ok": true,
        "awareness": awareness,
    })
}

/// Get a human-readable time summary (for consciousness layer)
pub async fn time_summary() -> String {
    let a = get_awareness().await;
    let mut parts = Vec::new();

    parts.push(format!("{:?} | {} | {:?}", a.current_period, a.local_time, a.day_type));

    if !a.upcoming_events.is_empty() {
        parts.push(format!("{} events in next 24h", a.upcoming_events.len()));
    }
    if !a.overdue_patterns.is_empty() {
        parts.push(format!("{} overdue tasks", a.overdue_patterns.len()));
    }
    if !a.active_deadlines.is_empty() {
        let nearest = a.active_deadlines.iter()
            .map(|d| (d.due - Utc::now()).num_days())
            .min()
            .unwrap_or(999);
        parts.push(format!("{} deadlines (nearest: {}d)", a.active_deadlines.len(), nearest));
    }
    if let Some(ref next) = a.next_scheduled {
        parts.push(format!("Next: {} in {}m", next.name, next.minutes_until));
    }

    parts.join(" | ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_time_store() {
        let store = TimeStore::default();
        assert!(!store.patterns.is_empty());
        assert!(!store.holidays.is_empty());
        assert_eq!(store.work_hours.start_hour, 9);
    }

    #[test]
    fn test_current_period() {
        let period = current_period();
        match period {
            TimePeriod::EarlyMorning | TimePeriod::Morning | TimePeriod::Afternoon |
            TimePeriod::Evening | TimePeriod::Night | TimePeriod::LateNight => {}
        }
    }

    #[test]
    fn test_gen_id() {
        let id1 = gen_id("test");
        assert!(id1.starts_with("test_"));
    }

    #[test]
    fn test_schedule_daily_next() {
        let now = Utc::now();
        let schedule = Schedule::Daily { hour: 3, minute: 0 };
        let next = next_execution_for(&schedule, now);
        assert!(next.is_some());
        let next = next.unwrap();
        assert!(next > now);
    }

    #[test]
    fn test_schedule_interval() {
        let now = Utc::now();
        let schedule = Schedule::Interval { seconds: 3600 };
        let next = next_execution_for(&schedule, now);
        assert!(next.is_some());
        let diff = (next.unwrap() - now).num_seconds();
        assert_eq!(diff, 3600);
    }

    #[test]
    fn test_schedule_weekly_next() {
        let now = Utc::now();
        let schedule = Schedule::Weekly { day: Weekday::Sun, hour: 2, minute: 0 };
        let next = next_execution_for(&schedule, now);
        assert!(next.is_some());
        assert!(next.unwrap() > now);
    }

    #[test]
    fn test_event_category_equality() {
        assert_eq!(EventCategory::Security, EventCategory::Security);
        assert_ne!(EventCategory::Security, EventCategory::Backup);
    }

    #[test]
    fn test_time_event_serialization() {
        let event = TimeEvent {
            id: "test_1".into(),
            name: "Test Event".into(),
            description: "A test".into(),
            when: Utc::now(),
            category: EventCategory::Maintenance,
            auto_execute: false,
            action: None,
            executed: false,
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Test Event"));
        assert!(json.contains("Maintenance"));
    }

    #[test]
    fn test_due_action_serialization() {
        let action = DueAction {
            source_id: "test".into(),
            source_type: "pattern".into(),
            name: "Test".into(),
            action: "do something".into(),
            auto_execute: true,
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("do something"));
    }
}
