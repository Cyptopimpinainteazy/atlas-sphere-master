/// Media Orchestration System
///
/// Integrates contributor framework + repurposing pipeline + cadence orchestrator + job queue
/// into a unified production system.
///
/// This is the "unified Option 4" mentioned in the strategy:
/// - Contributors are tracked with consent and compensation
/// - Cadence keeps output consistent (hard part)
/// - Repurposing multiplies content (1 recording → 50+ assets)
/// - Job queue distributes work to GPU nodes
/// - Dashboard provides visibility
///
/// Goal: Move founder from "content creator" to "strategic voice amplified at scale"

use crate::contributor::{ContributorManager, Contributor};
use crate::repurposing::{RepurposingEngine, ContentSource, RepurposingRequest};
use crate::cadence::{CadenceOrchestrator, CadenceSchedule};

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashSet;

/// Overall status of media production
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaProductionStatus {
    /// Total recordings scheduled this quarter
    pub recordings_scheduled: usize,

    /// Actually recorded
    pub recordings_completed: usize,

    /// On-time percentage
    pub on_time_percentage: f32,

    /// Total assets created from those recordings
    pub total_assets_created: usize,

    /// Assets ready for publishing
    pub assets_ready: usize,

    /// Assets published
    pub assets_published: usize,

    /// Contributors involved
    pub active_contributors: usize,

    /// Total production time (minutes)
    pub total_production_hours: f64,

    /// Most recent recording
    pub last_recording: Option<DateTime<Utc>>,

    /// Next scheduled recording
    pub next_recording: Option<DateTime<Utc>>,
}

/// Production report for a specific time period
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaProductionReport {
    /// Period covered
    pub period: String,

    /// Status at end of period
    pub status: MediaProductionStatus,

    /// What contributors were used and how
    pub contributor_usage: Vec<ContributorUsageReport>,

    /// Key moments/insights
    pub highlights: Vec<String>,

    /// Issues that occurred
    pub issues: Vec<ProductionIssue>,

    /// Metrics
    pub metrics: ProductionMetrics,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContributorUsageReport {
    pub contributor_id: String,
    pub contributor_name: String,
    pub recordings_featured_in: usize,
    pub assets_created: usize,
    pub compensation_owed: f64,
    pub consent_status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionIssue {
    pub date: DateTime<Utc>,
    pub severity: IssueSeverity,
    pub description: String,
    pub resolution: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionMetrics {
    /// Average time from recording to first asset published (hours)
    pub time_to_first_asset_hours: f64,

    /// Average time from recording to all assets published (hours)
    pub time_to_all_assets_hours: f64,

    /// Average number of assets per recording
    pub assets_per_recording: f64,

    /// Total contributors compensated
    pub total_compensation: f64,

    /// Cost per asset
    pub cost_per_asset: f64,

    /// Quality score (0-1)
    pub quality_score: f32,
}

/// The unified media orchestration system
pub struct MediaOrchestrationSystem {
    /// Who can be featured
    contributors: ContributorManager,

    /// What recordings are scheduled
    cadence: CadenceOrchestrator,

    /// How to repurpose content
    repurposing: RepurposingEngine,

    /// Current production status
    status: MediaProductionStatus,

    /// Production history
    history: Vec<MediaProductionReport>,

    /// Issues that have occurred
    issues: Vec<ProductionIssue>,

    /// Settings
    config: ProductionConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProductionConfig {
    /// How many days do we have to publish after recording?
    pub publishing_deadline_days: u32,

    /// Minimum quality score to auto-publish
    pub min_quality_for_autopublish: f32,

    /// How many assets should we try to create per recording? (baseline)
    pub target_assets_per_recording: u32,

    /// Auto-generate titles and descriptions?
    pub auto_generate_metadata: bool,

    /// What's the default compensation per recording for a contributor?
    pub default_recording_compensation: f64,

    /// How much per derivative asset that uses a contributor?
    pub compensation_per_derivative: f64,
}

impl Default for ProductionConfig {
    fn default() -> Self {
        Self {
            publishing_deadline_days: 7,
            min_quality_for_autopublish: 0.7,
            target_assets_per_recording: 30,
            auto_generate_metadata: true,
            default_recording_compensation: 500.0,
            compensation_per_derivative: 5.0,
        }
    }
}

impl MediaOrchestrationSystem {
    pub fn new(config: ProductionConfig) -> Self {
        Self {
            contributors: ContributorManager::new(),
            cadence: CadenceOrchestrator::new(),
            repurposing: RepurposingEngine::new(),
            status: MediaProductionStatus {
                recordings_scheduled: 0,
                recordings_completed: 0,
                on_time_percentage: 100.0,
                total_assets_created: 0,
                assets_ready: 0,
                assets_published: 0,
                active_contributors: 0,
                total_production_hours: 0.0,
                last_recording: None,
                next_recording: None,
            },
            history: Vec::new(),
            issues: Vec::new(),
            config,
        }
    }

    /// Register a new contributor
    pub fn register_contributor(&mut self, contributor: Contributor) -> Result<String, String> {
        let id = self.contributors.register_contributor(contributor)?;
        // Count active contributors
        self.status.active_contributors = self.contributors.list_contributors().len();
        Ok(id)
    }

    /// Get a contributor
    pub fn get_contributor(&self, id: &str) -> Option<Contributor> {
        self.contributors.get_contributor(id).cloned()
    }

    /// Create a consent agreement
    pub fn create_consent(
        &mut self,
        contributor_id: String,
        _consent: crate::contributor::ContributorConsent,
    ) -> Result<String, String> {
        // Create consent ID
        let consent_id = format!("consent-{}", uuid::Uuid::new_v4());
        // Store consent (implementation would depend on ContributorManager)
        Ok(consent_id)
    }

    /// Set up a production schedule
    pub fn create_cadence(&mut self, schedule: CadenceSchedule) -> Result<String, String> {
        let id = self.cadence.create_schedule(schedule)?;
        Ok(id)
    }

    /// Generate recording sessions for the schedule
    pub fn generate_recording_plan(
        &mut self,
        schedule_id: &str,
        weeks_ahead: u32,
    ) -> Result<Vec<String>, String> {
        let session_ids = self.cadence.generate_sessions(schedule_id, weeks_ahead)?;
        self.status.recordings_scheduled = session_ids.len();
        Ok(session_ids)
    }

    /// Mark that a recording happened
    pub fn record_session(
        &mut self,
        session_id: &str,
        recording_id: String,
        quality_score: f32,
    ) -> Result<(), String> {
        self.cadence.record_session(session_id, recording_id, quality_score)?;
        self.status.recordings_completed += 1;
        self.status.last_recording = Some(Utc::now());

        // Update metrics
        let metrics = self.cadence.get_metrics();
        self.status.on_time_percentage = metrics.on_time_percentage;

        Ok(())
    }

    /// Register source content for repurposing
    pub fn register_content(
        &mut self,
        source: ContentSource,
    ) -> Result<String, String> {
        self.repurposing.register_source(source)
    }

    /// Request content repurposing
    pub fn request_repurposing(
        &mut self,
        request: RepurposingRequest,
    ) -> Result<String, String> {
        self.repurposing.request_repurposing(request)
    }

    /// Mark a repurposing job as complete
    pub fn complete_repurposing(
        &mut self,
        request: RepurposingRequest,
        storage_path: String,
        content_hash: String,
        contributors_featured: Vec<String>,
    ) -> Result<String, String> {
        let asset_id = self.repurposing.complete_asset(
            request.clone(),
            storage_path,
            content_hash,
        )?;

        // Record usage for each contributor
        for contributor_id in &contributors_featured {
            self.contributors.record_usage(
                crate::contributor::ContributorUsageRecord {
                    consent_id: format!("consent-{}", uuid::Uuid::new_v4()),
                    contributor_id: contributor_id.clone(),
                    content_asset_id: asset_id.clone(),
                    usage_scope: "RecordedContent".to_string(),
                    used_at: Utc::now(),
                    context: request.title.clone(),
                    compensable: true,
                },
            );
        }

        self.status.total_assets_created += 1;
        Ok(asset_id)
    }

    /// Get current production status
    pub fn get_status(&self) -> MediaProductionStatus {
        self.status.clone()
    }

    /// Get a production report
    pub fn generate_report(&self, period: String) -> MediaProductionReport {
        MediaProductionReport {
            period,
            status: self.status.clone(),
            contributor_usage: self.get_contributor_usage_reports(),
            highlights: self.extract_highlights(),
            issues: self.issues.clone(),
            metrics: self.calculate_metrics(),
        }
    }

    /// Log an issue
    pub fn log_issue(
        &mut self,
        severity: IssueSeverity,
        description: String,
    ) {
        self.issues.push(ProductionIssue {
            date: Utc::now(),
            severity,
            description,
            resolution: None,
        });
    }

    /// Resolve an issue
    pub fn resolve_issue(&mut self, issue_idx: usize, resolution: String) -> Result<(), String> {
        if issue_idx >= self.issues.len() {
            return Err("Issue not found".to_string());
        }
        self.issues[issue_idx].resolution = Some(resolution);
        Ok(())
    }

    // Internal helpers

    fn get_contributor_usage_reports(&self) -> Vec<ContributorUsageReport> {
        let contributors = self.contributors.list_contributors();
        contributors
            .iter()
            .map(|c| {
                let usage = self.contributors.get_usage_history(&c.id);

                // Count unique recordings featured in (content_asset_id may be the asset id)
                let recordings_featured_in = usage
                    .iter()
                    .filter(|r| r.usage_scope.contains("RecordedContent"))
                    .map(|r| r.content_asset_id.clone())
                    .collect::<HashSet<_>>()
                    .len();

                // Total assets created / usage entries
                let assets_created = usage.len();

                // Compensation: base recording fee once + per-derivative for compensable records
                let compensable_count = usage.iter().filter(|r| r.compensable).count();
                let mut compensation_owed = (compensable_count as f64) * self.config.compensation_per_derivative;
                if usage.iter().any(|r| r.usage_scope.contains("RecordedContent")) {
                    compensation_owed += self.config.default_recording_compensation;
                }

                ContributorUsageReport {
                    contributor_id: c.id.clone(),
                    contributor_name: c.name.clone(),
                    recordings_featured_in,
                    assets_created,
                    compensation_owed,
                    consent_status: if c.status == crate::contributor::ContributorStatus::Active { "Active".to_string() } else { "Inactive".to_string() },
                }
            })
            .collect()
    }

    fn extract_highlights(&self) -> Vec<String> {
        vec![
            format!(
                "Completed {} of {} scheduled recordings",
                self.status.recordings_completed, self.status.recordings_scheduled
            ),
            format!(
                "Created {} derivative assets",
                self.status.total_assets_created
            ),
            format!(
                "{}% on-time consistency",
                self.status.on_time_percentage as u32
            ),
        ]
    }

    fn calculate_metrics(&self) -> ProductionMetrics {
        // Time to first asset: average over sources of (first asset created_at - source.created_at)
        let mut first_deltas: Vec<f64> = Vec::new();
        let mut all_deltas: Vec<f64> = Vec::new();

        for asset in self.repurposing.list_assets() {
            if let Some(source) = self.repurposing.get_source(&asset.source_id) {
                let delta_hours = (asset.created_at.signed_duration_since(source.created_at).num_seconds() as f64) / 3600.0;
                first_deltas.push(delta_hours);
                all_deltas.push(delta_hours);
            }
        }

        // For time_to_all_assets, group by source and compute max per source
        use std::collections::HashMap;
        let mut by_source: HashMap<String, Vec<f64>> = HashMap::new();
        for asset in self.repurposing.list_assets() {
            if let Some(source) = self.repurposing.get_source(&asset.source_id) {
                let delta_hours = (asset.created_at.signed_duration_since(source.created_at).num_seconds() as f64) / 3600.0;
                by_source.entry(source.id.clone()).or_default().push(delta_hours);
            }
        }

        let mut all_max_deltas: Vec<f64> = Vec::new();
        for (_src, deltas) in by_source {
            if let Some(maxv) = deltas.iter().cloned().fold(None, |acc: Option<f64>, v| Some(acc.map_or(v, |a| a.max(v)))) {
                all_max_deltas.push(maxv);
            }
        }

        let time_to_first_asset_hours = if !first_deltas.is_empty() {
            first_deltas.iter().sum::<f64>() / first_deltas.len() as f64
        } else {
            0.0
        };

        let time_to_all_assets_hours = if !all_max_deltas.is_empty() {
            all_max_deltas.iter().sum::<f64>() / all_max_deltas.len() as f64
        } else {
            0.0
        };

        // Compute total compensation from contributor usage reports
        let usage_reports = self.get_contributor_usage_reports();
        let total_compensation: f64 = usage_reports.iter().map(|r| r.compensation_owed).sum();

        let cost_per_asset = if self.status.total_assets_created > 0 {
            total_compensation / (self.status.total_assets_created as f64)
        } else {
            0.0
        };

        // Average quality score from recorded sessions
        let recorded = self.cadence.recorded_sessions();
        let quality_sum: f32 = recorded.iter().filter_map(|s| s.quality_score).sum();
        let quality_count: usize = recorded.iter().filter(|s| s.quality_score.is_some()).count();
        let quality_score = if quality_count > 0 {
            quality_sum / quality_count as f32
        } else {
            0.85
        };

        ProductionMetrics {
            time_to_first_asset_hours,
            time_to_all_assets_hours,
            assets_per_recording: if self.status.recordings_completed > 0 {
                self.status.total_assets_created as f64 / self.status.recordings_completed as f64
            } else {
                0.0
            },
            total_compensation,
            cost_per_asset,
            quality_score,
        }
    }
}

impl Default for MediaOrchestrationSystem {
    fn default() -> Self {
        Self::new(ProductionConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;
    use chrono::Weekday;

    #[test]
    fn test_media_system_creation() {
        let system = MediaOrchestrationSystem::default();
        assert_eq!(system.status.recordings_scheduled, 0);
        assert_eq!(system.status.active_contributors, 0);
    }

    #[test]
    fn test_contributor_registration_integration() {
        let mut system = MediaOrchestrationSystem::default();
        let contributor = Contributor {
            id: "founder-1".to_string(),
            name: "John Doe".to_string(),
            public_name: "john_doe".to_string(),
            email: "john@example.com".to_string(),
            wallet: None,
            role: crate::contributor::ContributorRole::Founder,
            status: crate::contributor::ContributorStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = system.register_contributor(contributor);
        assert!(result.is_ok());
        assert_eq!(system.status.active_contributors, 1);
    }

    #[test]
    fn test_cadence_schedule_integration() {
        let mut system = MediaOrchestrationSystem::default();
        let schedule = CadenceSchedule {
            name: "Weekly".to_string(),
            days_of_week: vec![Weekday::Tue, Weekday::Fri],
            time_of_day: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
            timezone: "UTC".to_string(),
            session_duration_minutes: 60,
            publishing_deadline_days: 7,
            content_themes: vec![],
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let result = system.create_cadence(schedule);
        assert!(result.is_ok());
    }

    #[test]
    fn test_status_reporting() {
        let system = MediaOrchestrationSystem::default();
        let status = system.get_status();

        assert_eq!(status.recordings_scheduled, 0);
        assert_eq!(status.recordings_completed, 0);
        assert_eq!(status.on_time_percentage, 100.0);
    }

    #[test]
    fn test_metrics_and_compensation() {
        let mut system = MediaOrchestrationSystem::default();

        // Register contributor
        let contributor = Contributor {
            id: "founder-1".to_string(),
            name: "John Doe".to_string(),
            public_name: "john_doe".to_string(),
            email: "john@example.com".to_string(),
            wallet: None,
            role: crate::contributor::ContributorRole::Founder,
            status: crate::contributor::ContributorStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        system.register_contributor(contributor).unwrap();

        // Register source
        let source = crate::repurposing::ContentSource {
            id: "talk-001".to_string(),
            name: "Founder Keynote".to_string(),
            content_type: crate::repurposing::ContentType::FounderTalk,
            featured_contributors: vec!["founder-1".to_string()],
            duration_seconds: 1800,
            storage_path: "/vault/talks/keynote-001.mp4".to_string(),
            content_hash: "0xabcd".to_string(),
            created_at: Utc::now() - chrono::Duration::hours(24),
            is_repurposable: true,
            tags: vec!["blockchain".to_string()],
            key_moments: vec![],
        };

        system.register_content(source).unwrap();

        // Request repurposing and complete it (this should record usage and create an asset)
        let request = crate::repurposing::RepurposingRequest {
            source_id: "talk-001".to_string(),
            asset_type: crate::repurposing::AssetType::Clip,
            format: crate::repurposing::OutputFormat::Vertical1080p,
            target: crate::repurposing::DerivationTarget::TikTok,
            title: "Clip".to_string(),
            description: "A clip".to_string(),
            tags: vec![],
            clip_moment_idx: None,
            instructions: None,
            priority: crate::repurposing::RepurposingPriority::Normal,
            auto_publish: true,
        };

        system.request_repurposing(request.clone()).unwrap();

        let asset_id = system.complete_repurposing(request, "/vault/clips/clip-1.mp4".to_string(), "0xhash".to_string(), vec!["founder-1".to_string()]).unwrap();

        assert!(system.status.total_assets_created >= 1);

        let report = system.generate_report("Q1".to_string());
        assert!(report.metrics.assets_per_recording >= 0.0);

        // There should be compensation owed to the contributor (default recording fee + per derivative)
        let usage_reports = report.contributor_usage;
        let founder_report = usage_reports.iter().find(|r| r.contributor_id == "founder-1").expect("founder report missing");
        assert!(founder_report.compensation_owed >= system.config.default_recording_compensation);

        // Cost per asset should be non-negative and consistent
        assert!(report.metrics.cost_per_asset >= 0.0);
    }
}
