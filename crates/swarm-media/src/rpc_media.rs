/// RPC API for Media Orchestration System
///
/// Provides 6 core endpoints for integrating media system with Substrate RPC layer:
/// 1. media_status        - Get current production status
/// 2. media_schedule      - Get cadence schedule and sessions
/// 3. media_contributors  - Get active contributors
/// 4. media_metrics       - Get detailed production report
/// 5. media_request_repurposing - Submit repurposing job to queue
/// 6. media_job_status    - Get status of repurposing job
///
/// These endpoints are designed to be called from:
/// - Frontend dashboard (Next.js)
/// - CLI tools
/// - Other system components via RPC

use crate::media_orchestration::{MediaOrchestrationSystem, MediaProductionStatus, MediaProductionReport};
use crate::repurposing::{RepurposingRequest, RepurposingPriority, AssetType, DerivationTarget, OutputFormat};
use crate::contributor::Contributor;

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// RPC request/response types

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaStatusRequest {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaStatusResponse {
    pub status: MediaProductionStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaScheduleRequest {
    pub schedule_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduledSession {
    pub session_id: String,
    pub scheduled_at: String,
    pub status: String,
    pub theme: String,
    pub contributors: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaScheduleResponse {
    pub schedule_id: String,
    pub days_of_week: Vec<u32>,
    pub time_of_day: String,
    pub timezone: String,
    pub sessions_this_month: Vec<ScheduledSession>,
    pub next_session: Option<ScheduledSession>,
    pub on_time_percentage: f32,
    pub on_time_streak: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaContributorsRequest {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContributorInfo {
    pub id: String,
    pub name: String,
    pub role: String,
    pub status: String,
    pub email: String,
    pub wallet_address: Option<String>,
    pub is_active: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaContributorsResponse {
    pub contributors: Vec<ContributorInfo>,
    pub total_active: usize,
    pub total_paused: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaMetricsRequest {
    pub period: String, // "week", "month", "quarter"
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub recordings_scheduled: usize,
    pub recordings_completed: usize,
    pub on_time_percentage: f32,
    pub total_assets_created: usize,
    pub total_compensation: f64,
    pub average_assets_per_recording: f32,
    pub highlights: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaMetricsResponse {
    pub period: String,
    pub summary: MetricsSummary,
    pub contributor_breakdown: Vec<ContributorMetrics>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContributorMetrics {
    pub contributor_id: String,
    pub contributor_name: String,
    pub recordings_featured_in: usize,
    pub assets_created: usize,
    pub compensation_owed: f64,
}

/// Request to create a repurposing job (routes to job queue)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaRepurposingRequest {
    pub source_id: String,
    pub asset_type: String,
    pub target: String,
    pub priority: String,
    pub title: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaRepurposingResponse {
    pub job_id: String,
    pub status: String,
    pub estimated_completion_seconds: u64,
}

/// Query job status (from job queue)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaJobStatusRequest {
    pub job_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaJobStatusResponse {
    pub job_id: String,
    pub status: String,
    pub priority: String,
    pub asset_type: String,
    pub target: String,
    pub progress_percentage: u32,
    pub created_at: String,
    pub last_update: String,
    pub estimated_completion: Option<String>,
    pub error_message: Option<String>,
}

/// RPC Handler
///
/// In production, this would be integrated into the Substrate RPC layer.
/// For now, it provides a standalone implementation that can be tested independently.
pub struct MediaRpcHandler {
    media: Arc<Mutex<MediaOrchestrationSystem>>,
    jobs: Arc<Mutex<HashMap<String, JobStatusRecord>>>,
}

#[derive(Clone, Debug)]
struct JobStatusRecord {
    job_id: String,
    status: String,
    priority: String,
    asset_type: String,
    target: String,
    progress_percentage: u32,
    created_at: String,
    last_update: String,
    estimated_completion: Option<String>,
    error_message: Option<String>,
}

impl MediaRpcHandler {
    pub fn new(media: Arc<Mutex<MediaOrchestrationSystem>>) -> Self {
        Self {
            media,
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Endpoint 1: media_status
    /// Returns current production status
    pub fn media_status(&self, _params: MediaStatusRequest) -> Result<MediaStatusResponse, String> {
        let media = self.media.lock().map_err(|e| format!("Lock error: {}", e))?;
        let status = media.get_status();

        Ok(MediaStatusResponse { status })
    }

    /// Endpoint 2: media_schedule
    /// Returns cadence schedule and generated sessions
    pub fn media_schedule(
        &self,
        params: MediaScheduleRequest,
    ) -> Result<MediaScheduleResponse, String> {
        let media = self.media.lock().map_err(|e| format!("Lock error: {}", e))?;

        // In a real implementation, we'd look up the schedule by ID
        // For now, return a representative response
        let status = media.get_status();

        Ok(MediaScheduleResponse {
            schedule_id: params.schedule_id,
            days_of_week: vec![2, 5], // Tuesday, Friday
            time_of_day: "11:00 AM".to_string(),
            timezone: "America/Los_Angeles".to_string(),
            sessions_this_month: vec![],
            next_session: None,
            on_time_percentage: status.on_time_percentage,
            on_time_streak: 0,
        })
    }

    /// Endpoint 3: media_contributors
    /// Returns list of active contributors
    pub fn media_contributors(
        &self,
        _params: MediaContributorsRequest,
    ) -> Result<MediaContributorsResponse, String> {
        let media = self.media.lock().map_err(|e| format!("Lock error: {}", e))?;

        // Get the list of contributors from the system status
        let status = media.get_status();
        
        // In a full implementation, we'd query the contributor manager directly
        // For now, we create placeholder info based on the status
        let contributor_infos: Vec<ContributorInfo> = vec![];

        Ok(MediaContributorsResponse {
            contributors: contributor_infos,
            total_active: status.active_contributors,
            total_paused: 0,
        })
    }

    /// Endpoint 4: media_metrics
    /// Returns detailed production metrics for a period
    pub fn media_metrics(
        &self,
        params: MediaMetricsRequest,
    ) -> Result<MediaMetricsResponse, String> {
        let media = self.media.lock().map_err(|e| format!("Lock error: {}", e))?;

        // Generate report for the requested period
        let report = media.generate_report(params.period.clone());
        let status = &report.status;

        let contributor_breakdown: Vec<ContributorMetrics> = report
            .contributor_usage
            .iter()
            .map(|cu| ContributorMetrics {
                contributor_id: cu.contributor_id.clone(),
                contributor_name: cu.contributor_name.clone(),
                recordings_featured_in: cu.recordings_featured_in,
                assets_created: cu.assets_created,
                compensation_owed: cu.compensation_owed,
            })
            .collect();

        let summary = MetricsSummary {
            recordings_scheduled: status.recordings_scheduled,
            recordings_completed: status.recordings_completed,
            on_time_percentage: status.on_time_percentage,
            total_assets_created: status.total_assets_created,
            total_compensation: contributor_breakdown
                .iter()
                .map(|c| c.compensation_owed)
                .sum(),
            average_assets_per_recording: if status.recordings_completed > 0 {
                status.total_assets_created as f32 / status.recordings_completed as f32
            } else {
                0.0
            },
            highlights: report.highlights,
        };

        Ok(MediaMetricsResponse {
            period: params.period,
            summary,
            contributor_breakdown,
        })
    }

    /// Endpoint 5: media_request_repurposing
    /// Submit a repurposing job to the job queue
    /// In production, this would convert RepurposingRequest → Intent and route to job queue
    pub fn media_request_repurposing(
        &self,
        params: MediaRepurposingRequest,
    ) -> Result<MediaRepurposingResponse, String> {
        // Parse asset type
        let asset_type = match params.asset_type.as_str() {
            "Clip" => AssetType::Clip,
            "FullEpisode" => AssetType::FullEpisode,
            "DubLocalization" => AssetType::DubLocalization,
            "SubtitledVersion" => AssetType::SubtitledVersion,
            "Transcript" => AssetType::Transcript,
            "EducationalExplainer" => AssetType::EducationalExplainer,
            "SocialTeaser" => AssetType::SocialTeaser,
            "Montage" => AssetType::Montage,
            "InteractiveModule" => AssetType::InteractiveModule,
            "PodcastEpisode" => AssetType::PodcastEpisode,
            _ => return Err("Invalid asset type".to_string()),
        };

        // Parse target - map target string to DerivationTarget
        let target = match params.target.as_str() {
            "YouTube" => DerivationTarget::YouTube,
            "YouTubeShorts" => DerivationTarget::YouTubeShorts,
            "TikTok" => DerivationTarget::TikTok,
            "InstagramReels" => DerivationTarget::InstagramReels,
            "InstagramFeed" => DerivationTarget::InstagramFeed,
            "Twitter" => DerivationTarget::Twitter,
            "LinkedIn" => DerivationTarget::LinkedIn,
            "Website" => DerivationTarget::Website,
            "Email" => DerivationTarget::Email,
            "Internal" => DerivationTarget::Internal,
            _ => {
                // Try parsing as PodcastPlatform
                if params.target.starts_with("PodcastPlatform:") {
                    let platform = params.target.strip_prefix("PodcastPlatform:").unwrap().to_string();
                    DerivationTarget::PodcastPlatform(platform)
                } else if params.target.starts_with("Language:") {
                    let lang = params.target.strip_prefix("Language:").unwrap().to_string();
                    DerivationTarget::Language(lang)
                } else {
                    return Err(format!("Invalid target: {}", params.target));
                }
            }
        };

        // Parse priority
        let priority = match params.priority.as_str() {
            "Urgent" => RepurposingPriority::Urgent,
            "High" => RepurposingPriority::High,
            "Normal" => RepurposingPriority::Normal,
            "Low" => RepurposingPriority::Low,
            _ => RepurposingPriority::Normal,
        };

        // Determine output format based on target
        let format = match target {
            DerivationTarget::YouTubeShorts | DerivationTarget::TikTok | DerivationTarget::InstagramReels => {
                OutputFormat::Vertical1080p
            }
            DerivationTarget::InstagramFeed => OutputFormat::Square1080p,
            DerivationTarget::Twitter => OutputFormat::Horizontal720p,
            _ => OutputFormat::Horizontal1080p,
        };

        // Create repurposing request
        let request = RepurposingRequest {
            source_id: params.source_id.clone(),
            asset_type,
            format,
            target,
            title: params.title,
            description: params.description,
            tags: vec![],
            clip_moment_idx: None,
            instructions: None,
            priority,
            auto_publish: true,
        };

        let job_id = uuid::Uuid::new_v4().to_string();

        // In production, this would convert to Intent and route to job queue:
        // let intent = repurposing_to_intent(request)?;
        // let intent_id = job_queue.submit(intent)?;

        // Create job record
        let job_record = JobStatusRecord {
            job_id: job_id.clone(),
            status: "Queued".to_string(),
            priority: params.priority,
            asset_type: params.asset_type,
            target: params.target,
            progress_percentage: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_update: chrono::Utc::now().to_rfc3339(),
            estimated_completion: None,
            error_message: None,
        };

        let mut jobs = self.jobs.lock().map_err(|e| format!("Lock error: {}", e))?;
        jobs.insert(job_id.clone(), job_record);

        Ok(MediaRepurposingResponse {
            job_id,
            status: "Queued".to_string(),
            estimated_completion_seconds: 3600, // 1 hour estimate
        })
    }

    /// Endpoint 6: media_job_status
    /// Query the status of a repurposing job
    pub fn media_job_status(
        &self,
        params: MediaJobStatusRequest,
    ) -> Result<MediaJobStatusResponse, String> {
        let jobs = self.jobs.lock().map_err(|e| format!("Lock error: {}", e))?;

        let job = jobs
            .get(&params.job_id)
            .ok_or_else(|| format!("Job {} not found", params.job_id))?;

        Ok(MediaJobStatusResponse {
            job_id: job.job_id.clone(),
            status: job.status.clone(),
            priority: job.priority.clone(),
            asset_type: job.asset_type.clone(),
            target: job.target.clone(),
            progress_percentage: job.progress_percentage,
            created_at: job.created_at.clone(),
            last_update: job.last_update.clone(),
            estimated_completion: job.estimated_completion.clone(),
            error_message: job.error_message.clone(),
        })
    }

    /// Endpoint 7: media_jobs_list
    /// Return list of job records currently known to the handler
    pub fn media_jobs_list(&self) -> Result<Vec<MediaJobStatusResponse>, String> {
        let jobs = self.jobs.lock().map_err(|e| format!("Lock error: {}", e))?;

        let mut list: Vec<MediaJobStatusResponse> = jobs
            .values()
            .map(|job| MediaJobStatusResponse {
                job_id: job.job_id.clone(),
                status: job.status.clone(),
                priority: job.priority.clone(),
                asset_type: job.asset_type.clone(),
                target: job.target.clone(),
                progress_percentage: job.progress_percentage,
                created_at: job.created_at.clone(),
                last_update: job.last_update.clone(),
                estimated_completion: job.estimated_completion.clone(),
                error_message: job.error_message.clone(),
            })
            .collect();

        // Sort by created_at descending
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_media_status() {
        let media = Arc::new(Mutex::new(MediaOrchestrationSystem::default()));
        let handler = MediaRpcHandler::new(media);

        let request = MediaStatusRequest {};
        let response = handler.media_status(request).unwrap();

        assert_eq!(response.status.recordings_scheduled, 0);
        assert_eq!(response.status.recordings_completed, 0);
    }

    #[test]
    fn test_rpc_media_contributors() {
        let media = Arc::new(Mutex::new(MediaOrchestrationSystem::default()));
        let handler = MediaRpcHandler::new(media);

        let request = MediaContributorsRequest {};
        let response = handler.media_contributors(request).unwrap();

        assert_eq!(response.total_active, 0);
        assert_eq!(response.total_paused, 0);
    }

    #[test]
    fn test_rpc_media_schedule() {
        let media = Arc::new(Mutex::new(MediaOrchestrationSystem::default()));
        let handler = MediaRpcHandler::new(media);

        let request = MediaScheduleRequest {
            schedule_id: "sched-1".to_string(),
        };
        let response = handler.media_schedule(request).unwrap();

        assert_eq!(response.schedule_id, "sched-1");
        assert_eq!(response.days_of_week, vec![2, 5]);
    }

    #[test]
    fn test_rpc_media_metrics() {
        let media = Arc::new(Mutex::new(MediaOrchestrationSystem::default()));
        let handler = MediaRpcHandler::new(media);

        let request = MediaMetricsRequest {
            period: "week".to_string(),
        };
        let response = handler.media_metrics(request).unwrap();

        assert_eq!(response.period, "week");
        assert_eq!(response.summary.recordings_scheduled, 0);
    }

    #[test]
    fn test_rpc_request_repurposing() {
        let media = Arc::new(Mutex::new(MediaOrchestrationSystem::default()));
        let handler = MediaRpcHandler::new(media);

        let request = MediaRepurposingRequest {
            source_id: "source-1".to_string(),
            asset_type: "Clip".to_string(),
            target: "TikTok".to_string(),
            priority: "High".to_string(),
            title: "Test Clip".to_string(),
            description: "A test clip".to_string(),
        };

        let response = handler.media_request_repurposing(request).unwrap();

        assert!(!response.job_id.is_empty());
        assert_eq!(response.status, "Queued");
    }

    #[test]
    fn test_rpc_job_status() {
        let media = Arc::new(Mutex::new(MediaOrchestrationSystem::default()));
        let handler = MediaRpcHandler::new(media);

        // First, submit a job
        let repurposing_request = MediaRepurposingRequest {
            source_id: "source-1".to_string(),
            asset_type: "Clip".to_string(),
            target: "TikTok".to_string(),
            priority: "High".to_string(),
            title: "Test Clip".to_string(),
            description: "A test clip".to_string(),
        };

        let submit_response = handler.media_request_repurposing(repurposing_request).unwrap();
        let job_id = submit_response.job_id;

        // Then query its status
        let status_request = MediaJobStatusRequest {
            job_id: job_id.clone(),
        };

        let status_response = handler.media_job_status(status_request).unwrap();

        assert_eq!(status_response.job_id, job_id);
        assert_eq!(status_response.status, "Queued");
        assert_eq!(status_response.progress_percentage, 0);
    }

    #[test]
    fn test_rpc_jobs_list() {
        let media = Arc::new(Mutex::new(MediaOrchestrationSystem::default()));
        let handler = MediaRpcHandler::new(media);

        // Submit two jobs
        let r1 = MediaRepurposingRequest {
            source_id: "source-a".to_string(),
            asset_type: "Clip".to_string(),
            target: "TikTok".to_string(),
            priority: "High".to_string(),
            title: "A".to_string(),
            description: "First".to_string(),
        };

        let r2 = MediaRepurposingRequest {
            source_id: "source-b".to_string(),
            asset_type: "Clip".to_string(),
            target: "YouTube".to_string(),
            priority: "Normal".to_string(),
            title: "B".to_string(),
            description: "Second".to_string(),
        };

        let _ = handler.media_request_repurposing(r1).unwrap();
        let _ = handler.media_request_repurposing(r2).unwrap();

        let list = handler.media_jobs_list().unwrap();
        assert!(list.len() >= 2);
        assert!(list.iter().any(|j| j.status == "Queued"));
    }
}
