/// Cadence Orchestrator
///
/// The hard part: keeping consistent output on schedule.
///
/// A founder records once per week on Tuesday/Friday at 11am Pacific.
/// The system:
/// 1. Generates a week of content calendar (what gets recorded when)
/// 2. Auto-schedules repurposing jobs (1 recording → dozens of assets)
/// 3. Tracks consistency metrics (did we hit the cadence?)
/// 4. Alerts when there are gaps
/// 5. Coordinates with contributors' schedules
///
/// This is what separates "amateur side project" from "production machine."

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Duration, NaiveTime, Utc, Datelike, Weekday, Timelike, NaiveDateTime, TimeZone};
use chrono_tz::{Tz, ParseError as TzParseError};
use std::collections::HashMap;
use std::str::FromStr;

/// When should content be recorded?
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CadenceSchedule {
    /// Name of this schedule
    pub name: String,

    /// Which days of the week?
    pub days_of_week: Vec<Weekday>,

    /// What time?
    pub time_of_day: NaiveTime,

    /// Timezone (e.g., "America/Los_Angeles")
    pub timezone: String,

    /// How long should each recording session be?
    pub session_duration_minutes: u32,

    /// How long until we should have all assets published?
    pub publishing_deadline_days: u32,

    /// What types of content get recorded on each day?
    pub content_themes: Vec<ContentTheme>,

    /// Is this schedule active?
    pub is_active: bool,

    /// Created when?
    pub created_at: DateTime<Utc>,

    /// Updated when?
    pub updated_at: DateTime<Utc>,
}

/// A theme or topic for a recording session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentTheme {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub target_duration_minutes: u32,
    pub required_contributors: Vec<String>,
}

/// A scheduled recording session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordingSession {
    /// Unique ID
    pub id: String,

    /// Which cadence schedule is this part of?
    pub cadence_id: String,

    /// When should this happen?
    pub scheduled_at: DateTime<Utc>,

    /// What's the plan?
    pub theme: ContentTheme,

    /// Who's involved?
    pub contributors: Vec<String>,

    /// Status
    pub status: RecordingStatus,

    /// Actual recording location/file
    pub recording_id: Option<String>,

    /// When was it actually recorded?
    pub recorded_at: Option<DateTime<Utc>>,

    /// Notes
    pub notes: String,

    /// Did it go well?
    pub quality_score: Option<f32>, // 0.0-1.0
}

/// Status of a recording session
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecordingStatus {
    /// Scheduled but not recorded yet
    Scheduled,

    /// Recording in progress
    InProgress,

    /// Recorded, waiting for processing
    RecordedPending,

    /// Processing (repurposing in progress)
    Processing,

    /// All assets published
    Complete,

    /// Skipped (rescheduled or cancelled)
    Skipped,

    /// Something went wrong
    Failed,
}

/// A piece of work to create from a recording
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionJob {
    /// Unique ID
    pub id: String,

    /// Source recording
    pub recording_session_id: String,

    /// What asset are we making?
    pub asset_type: String,

    /// Target platform/format
    pub target: String,

    /// Title and description (optional, can be AI-generated)
    pub title: Option<String>,
    pub description: Option<String>,

    /// Status
    pub status: ProductionJobStatus,

    /// When should this be done?
    pub deadline: DateTime<Utc>,

    /// Actual work
    pub attempts: Vec<ProductionAttempt>,

    /// Final asset ID (if successful)
    pub asset_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProductionJobStatus {
    Queued,
    InProgress,
    Complete,
    Failed,
    Cancelled,
}

/// An attempt to create an asset
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionAttempt {
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: ProductionJobStatus,
    pub error_message: Option<String>,
    pub metrics: Option<HashMap<String, f64>>,
}

/// Cadence orchestrator - coordinates scheduling and production
pub struct CadenceOrchestrator {
    /// Active schedules
    schedules: HashMap<String, CadenceSchedule>,

    /// All recording sessions
    sessions: HashMap<String, RecordingSession>,

    /// All production jobs
    jobs: HashMap<String, ProductionJob>,

    /// Consistency metrics
    metrics: ConsistencyMetrics,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsistencyMetrics {
    /// Total scheduled sessions
    pub total_scheduled: usize,

    /// Actually recorded
    pub actually_recorded: usize,

    /// On-time percentage (0-100)
    pub on_time_percentage: f32,

    /// Assets published on schedule
    pub on_time_assets: usize,

    /// Average assets per recording
    pub avg_assets_per_recording: f32,

    /// Last recording date
    pub last_recording: Option<DateTime<Utc>>,

    /// Next scheduled recording
    pub next_recording: Option<DateTime<Utc>>,

    /// Current streak (consecutive on-time recordings)
    pub on_time_streak: usize,
}

impl CadenceOrchestrator {
    pub fn new() -> Self {
        Self {
            schedules: HashMap::new(),
            sessions: HashMap::new(),
            jobs: HashMap::new(),
            metrics: ConsistencyMetrics {
                total_scheduled: 0,
                actually_recorded: 0,
                on_time_percentage: 100.0,
                on_time_assets: 0,
                avg_assets_per_recording: 0.0,
                last_recording: None,
                next_recording: None,
                on_time_streak: 0,
            },
        }
    }

    /// Create a new cadence schedule
    pub fn create_schedule(&mut self, schedule: CadenceSchedule) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.schedules.insert(id.clone(), schedule);
        Ok(id)
    }

    /// Generate recording sessions for the next N weeks
    pub fn generate_sessions(
        &mut self,
        schedule_id: &str,
        weeks_ahead: u32,
    ) -> Result<Vec<String>, String> {
        let schedule = self
            .schedules
            .get(schedule_id)
            .ok_or("Schedule not found")?
            .clone();

        if !schedule.is_active {
            return Err("Schedule is not active".to_string());
        }

        let mut session_ids = Vec::new();
        let now = Utc::now();

        // Parse the timezone from schedule
        let tz: Tz = Tz::from_str(&schedule.timezone)
            .map_err(|_| format!("Invalid timezone: {}. Use IANA format like 'America/Los_Angeles'", schedule.timezone))?;

        for week in 0..weeks_ahead {
            for day in &schedule.days_of_week {
                // Calculate the next occurrence of this weekday
                let target_date = self.next_weekday(now, *day, week);

                // Create naive datetime with the scheduled time
                let naive_dt = target_date
                    .and_hms_opt(
                        schedule.time_of_day.hour(),
                        schedule.time_of_day.minute(),
                        0,
                    )
                    .ok_or_else(|| "Invalid time combination".to_string())?;

                // Convert from local timezone to UTC
                // This handles DST transitions correctly
                let local_dt = tz.from_local_datetime(&naive_dt)
                    .single()
                    .or_else(|| {
                        // Handle DST gap/overlap - use earliest time
                        tz.from_local_datetime(&naive_dt).earliest()
                    })
                    .ok_or_else(|| "Could not convert time to timezone".to_string())?;
                
                let scheduled_at: DateTime<Utc> = local_dt.with_timezone(&Utc);

                let session = RecordingSession {
                    id: uuid::Uuid::new_v4().to_string(),
                    cadence_id: schedule_id.to_string(),
                    scheduled_at,
                    theme: schedule.content_themes.first().cloned().unwrap_or(ContentTheme {
                        name: "General".to_string(),
                        description: "General content".to_string(),
                        tags: vec![],
                        target_duration_minutes: schedule.session_duration_minutes,
                        required_contributors: vec![],
                    }),
                    contributors: vec![],
                    status: RecordingStatus::Scheduled,
                    recording_id: None,
                    recorded_at: None,
                    notes: String::new(),
                    quality_score: None,
                };

                session_ids.push(session.id.clone());
                self.sessions.insert(session.id.clone(), session);
            }
        }

        self.metrics.total_scheduled = self.sessions.len();
        self.metrics.next_recording = self.sessions.values().next().map(|s| s.scheduled_at);

        Ok(session_ids)
    }

    /// Mark a session as recorded
    pub fn record_session(
        &mut self,
        session_id: &str,
        recording_id: String,
        quality_score: f32,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or("Session not found")?;

        session.status = RecordingStatus::RecordedPending;
        session.recording_id = Some(recording_id);
        session.recorded_at = Some(Utc::now());
        session.quality_score = Some(quality_score.min(1.0).max(0.0));

        // Check on-time before releasing mutable borrow
        let scheduled_at = session.scheduled_at;
        let recorded_at = session.recorded_at;

        self.metrics.actually_recorded += 1;
        self.metrics.last_recording = Some(Utc::now());

        // Update on-time streak
        let on_time = if let Some(rec_at) = recorded_at {
            rec_at <= scheduled_at + Duration::minutes(30)
        } else {
            false
        };

        if on_time {
            self.metrics.on_time_streak += 1;
        } else {
            self.metrics.on_time_streak = 0;
        }

        // Update percentage
        if self.metrics.total_scheduled > 0 {
            self.metrics.on_time_percentage =
                (self.metrics.actually_recorded as f32 / self.metrics.total_scheduled as f32) * 100.0;
        }

        Ok(())
    }

    /// Create production jobs from a recording
    pub fn create_production_jobs(
        &mut self,
        session_id: &str,
        job_templates: Vec<ProductionJobTemplate>,
    ) -> Result<Vec<String>, String> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or("Session not found")?;

        let schedule = self
            .schedules
            .get(&session.cadence_id)
            .ok_or("Cadence schedule not found")?;

        let deadline = Utc::now() + Duration::days(schedule.publishing_deadline_days as i64);

        let mut job_ids = Vec::new();

        for template in job_templates {
            let job = ProductionJob {
                id: uuid::Uuid::new_v4().to_string(),
                recording_session_id: session_id.to_string(),
                asset_type: template.asset_type,
                target: template.target,
                title: template.title,
                description: template.description,
                status: ProductionJobStatus::Queued,
                deadline,
                attempts: vec![],
                asset_id: None,
            };

            job_ids.push(job.id.clone());
            self.jobs.insert(job.id.clone(), job);
        }

        // Update session status
        self.sessions.get_mut(session_id).unwrap().status = RecordingStatus::Processing;

        Ok(job_ids)
    }

    /// Complete a production job
    pub fn complete_job(&mut self, job_id: &str, asset_id: String) -> Result<(), String> {
        let job = self.jobs.get_mut(job_id).ok_or("Job not found")?;
        job.status = ProductionJobStatus::Complete;
        job.asset_id = Some(asset_id);

        // Check if all jobs for this session are complete
        let session_id = job.recording_session_id.clone();
        let all_complete = self
            .jobs
            .values()
            .filter(|j| j.recording_session_id == session_id)
            .all(|j| j.status == ProductionJobStatus::Complete);

        if all_complete {
            if let Some(session) = self.sessions.get_mut(&session_id) {
                session.status = RecordingStatus::Complete;
            }
        }

        Ok(())
    }

    /// Get consistency dashboard
    pub fn get_metrics(&self) -> &ConsistencyMetrics {
        &self.metrics
    }

    /// Get upcoming sessions
    pub fn get_upcoming_sessions(&self, limit: usize) -> Vec<&RecordingSession> {
        let mut sessions: Vec<_> = self
            .sessions
            .values()
            .filter(|s| s.status == RecordingStatus::Scheduled)
            .collect();
        sessions.sort_by_key(|s| s.scheduled_at);
        sessions.into_iter().take(limit).collect()
    }

    /// Get recorded sessions (useful for metrics)
    pub fn recorded_sessions(&self) -> Vec<&RecordingSession> {
        let mut recorded: Vec<_> = self
            .sessions
            .values()
            .filter(|s| matches!(s.status, RecordingStatus::RecordedPending | RecordingStatus::Processing | RecordingStatus::Complete))
            .collect();
        recorded.sort_by_key(|s| s.recorded_at);
        recorded
    }

    /// Get pending jobs
    pub fn get_pending_jobs(&self) -> Vec<&ProductionJob> {
        self.jobs
            .values()
            .filter(|j| j.status == ProductionJobStatus::Queued || j.status == ProductionJobStatus::InProgress)
            .collect()
    }

    // Helper: next occurrence of weekday
    fn next_weekday(
        &self,
        from: DateTime<Utc>,
        target_weekday: Weekday,
        week_offset: u32,
    ) -> chrono::NaiveDate {
        let mut date = from.naive_utc().date();
        let current_weekday = date.weekday();

        // Days until target weekday (avoid unsigned underflow by using signed math)
        let t = target_weekday.number_from_monday() as i64;
        let c = current_weekday.number_from_monday() as i64;
        let mut diff = t - c;
        if diff < 0 {
            diff += 7;
        }
        let days_ahead = diff as i64;

        date = date + Duration::days(days_ahead + ((week_offset as i64) * 7));
        date
    }

    // Helper: is the session on-time?
    fn is_on_time(&self, session: &RecordingSession) -> bool {
        if let Some(recorded_at) = session.recorded_at {
            recorded_at <= session.scheduled_at + Duration::minutes(30)
        } else {
            false
        }
    }
}

/// Template for creating production jobs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionJobTemplate {
    pub asset_type: String,
    pub target: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

impl Default for CadenceOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_schedule() {
        let mut orchestrator = CadenceOrchestrator::new();
        let schedule = CadenceSchedule {
            name: "Founder Weekly".to_string(),
            days_of_week: vec![Weekday::Tue, Weekday::Fri],
            time_of_day: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
            timezone: "America/Los_Angeles".to_string(),
            session_duration_minutes: 60,
            publishing_deadline_days: 7,
            content_themes: vec![ContentTheme {
                name: "General Insight".to_string(),
                description: "Founder thoughts on current events".to_string(),
                tags: vec!["blockchain".to_string()],
                target_duration_minutes: 60,
                required_contributors: vec!["founder".to_string()],
            }],
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = orchestrator.create_schedule(schedule);
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_sessions() {
        let mut orchestrator = CadenceOrchestrator::new();
        let schedule = CadenceSchedule {
            name: "Test Schedule".to_string(),
            days_of_week: vec![Weekday::Mon],
            time_of_day: NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
            timezone: "UTC".to_string(),
            session_duration_minutes: 30,
            publishing_deadline_days: 5,
            content_themes: vec![],
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let schedule_id = orchestrator.create_schedule(schedule).unwrap();
        let result = orchestrator.generate_sessions(&schedule_id, 2);

        assert!(result.is_ok());
        assert!(result.unwrap().len() > 0);
    }

    #[test]
    fn test_record_session() {
        let mut orchestrator = CadenceOrchestrator::new();
        let session = RecordingSession {
            id: "sess-001".to_string(),
            cadence_id: "cad-001".to_string(),
            scheduled_at: Utc::now(),
            theme: ContentTheme {
                name: "Test".to_string(),
                description: "Test".to_string(),
                tags: vec![],
                target_duration_minutes: 30,
                required_contributors: vec![],
            },
            contributors: vec![],
            status: RecordingStatus::Scheduled,
            recording_id: None,
            recorded_at: None,
            notes: String::new(),
            quality_score: None,
        };

        orchestrator.sessions.insert(session.id.clone(), session);
        let result = orchestrator.record_session("sess-001", "rec-001".to_string(), 0.95);

        assert!(result.is_ok());
        assert_eq!(orchestrator.metrics.actually_recorded, 1);
    }
}
