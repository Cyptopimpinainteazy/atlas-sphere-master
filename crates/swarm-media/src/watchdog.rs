//! Watchdog Scheduler for Fraud Detection
//!
//! Periodically re-runs completed tasks to verify results and trigger slashing
//! when discrepancies are detected.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::tool_adapter::{JobId, JobStatus, ToolAdapter, ToolParams, ToolResult};
use crate::reputation::{ReputationManager, ReputationRepo, ReputationEvent, SlashingEvent, compute_slash_amount};

/// Configuration for watchdog behavior
#[derive(Clone, Debug)]
pub struct WatchdogConfig {
    pub check_interval_seconds: u64,
    pub sample_rate: f64, // Fraction of tasks to check (0.0 to 1.0)
    pub max_concurrent_checks: usize,
    pub tolerance_threshold: f64, // Max allowed difference in results
    pub slash_severity: f64, // Severity factor for slashing (0.0 to 1.0)
    pub base_slash_scale: f64, // Base fraction of bond to slash
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: 300, // 5 minutes
            sample_rate: 0.1, // Check 10% of tasks
            max_concurrent_checks: 5,
            tolerance_threshold: 0.05, // 5% tolerance
            slash_severity: 0.5,
            base_slash_scale: 0.1, // 10% of bond
        }
    }
}

/// Result of a watchdog verification
#[derive(Debug, Clone)]
pub enum VerificationResult {
    Pass {
        task_id: JobId,
        contributor_id: String,
        original_result: serde_json::Value,
        verification_result: serde_json::Value,
        difference_score: f64,
    },
    Fail {
        task_id: JobId,
        contributor_id: String,
        original_result: serde_json::Value,
        verification_result: serde_json::Value,
        difference_score: f64,
        reason: String,
    },
    Error {
        task_id: JobId,
        contributor_id: String,
        error: String,
    },
}

/// Interface for accessing completed tasks
#[async_trait]
pub trait TaskHistory: Send + Sync + 'static {
    async fn get_completed_tasks(&self, since: DateTime<Utc>) -> Result<Vec<TaskRecord>, String>;
    async fn get_task_result(&self, task_id: JobId) -> Result<TaskResult, String>;
}

/// Record of a completed task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task_id: JobId,
    pub contributor_id: String,
    pub workload_type: String,
    pub params: serde_json::Value,
    pub result: serde_json::Value,
    pub completed_at: DateTime<Utc>,
    pub verified: bool,
}

/// Result of a task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: JobId,
    pub contributor_id: String,
    pub workload_type: String,
    pub params: serde_json::Value,
    pub result: serde_json::Value,
    pub execution_time_ms: u32,
    pub success: bool,
}

/// Watchdog scheduler implementation
pub struct WatchdogScheduler<R: ReputationRepo, H: TaskHistory> {
    config: WatchdogConfig,
    reputation_manager: Arc<ReputationManager<R>>,
    task_history: Arc<H>,
    adapters: Arc<dyn ToolAdapter>,
    running: Arc<Mutex<bool>>,
}

impl<R: ReputationRepo, H: TaskHistory> WatchdogScheduler<R, H> {
    pub fn new(
        config: WatchdogConfig,
        reputation_manager: Arc<ReputationManager<R>>,
        task_history: Arc<H>,
        adapters: Arc<dyn ToolAdapter>,
    ) -> Self {
        Self {
            config,
            reputation_manager,
            task_history,
            adapters,
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn start(&self) -> Result<(), String> {
        let mut running = self.running.lock().unwrap();
        if *running {
            return Err("Watchdog is already running".to_string());
        }
        *running = true;
        drop(running);

        // Start the watchdog loop in the background
        let config = self.config.clone();
        let reputation_manager = self.reputation_manager.clone();
        let task_history = self.task_history.clone();
        let adapters = self.adapters.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.check_interval_seconds));
            
            loop {
                interval.tick().await;
                
                if let Err(e) = Self::run_verification_cycle(&config, &reputation_manager, &task_history, &adapters).await {
                    tracing::error!("Watchdog verification cycle failed: {}", e);
                }
            }
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        let mut running = self.running.lock().unwrap();
        *running = false;
        Ok(())
    }

    async fn run_verification_cycle(
        config: &WatchdogConfig,
        reputation_manager: &ReputationManager<R>,
        task_history: &H,
        adapters: &dyn ToolAdapter,
    ) -> Result<(), String> {
        let since = Utc::now() - chrono::Duration::minutes(30); // Check tasks from last 30 minutes
        let tasks = task_history.get_completed_tasks(since).await?;

        // Sample tasks for verification
        let tasks_to_check = Self::sample_tasks(tasks, config.sample_rate);

        tracing::info!("Watchdog checking {} tasks for fraud detection", tasks_to_check.len());

        // Process checks concurrently with limited concurrency
        let mut handles = Vec::new();
        
        for task in tasks_to_check {
            let config = config.clone();
            let reputation_manager = reputation_manager.clone();
            let task_history = task_history.clone();
            let adapters = adapters.clone();
            
            let handle = tokio::spawn(async move {
                Self::verify_task(&config, &reputation_manager, &task_history, &adapters, task).await
            });

            handles.push(handle);

            // Limit concurrent checks
            if handles.len() >= config.max_concurrent_checks {
                let results = futures::future::join_all(handles).await;
                Self::process_verification_results(results).await?;
                handles.clear();
            }
        }

        // Process remaining checks
        if !handles.is_empty() {
            let results = futures::future::join_all(handles).await;
            Self::process_verification_results(results).await?;
        }

        Ok(())
    }

    fn sample_tasks(tasks: Vec<TaskRecord>, sample_rate: f64) -> Vec<TaskRecord> {
        if sample_rate <= 0.0 || sample_rate >= 1.0 {
            return tasks;
        }

        let mut rng = rand::thread_rng();
        tasks.into_iter()
            .filter(|_| rand::Rng::gen_bool(&mut rng, sample_rate))
            .collect()
    }

    async fn verify_task(
        config: &WatchdogConfig,
        reputation_manager: &ReputationManager<R>,
        task_history: &H,
        adapters: &dyn ToolAdapter,
        task: TaskRecord,
    ) -> Result<VerificationResult, String> {
        // Re-execute the task
        let params = ToolParams::new(task.params.clone());
        let verification_job_id = adapters.invoke(params).await?;

        // Wait for completion
        let mut attempts = 0;
        let max_attempts = 10;
        
        loop {
            sleep(Duration::from_secs(5)).await;
            
            let status = adapters.get_status(verification_job_id).await?;
            if status == JobStatus::Completed {
                break;
            }

            attempts += 1;
            if attempts >= max_attempts {
                return Ok(VerificationResult::Error {
                    task_id: task.task_id,
                    contributor_id: task.contributor_id,
                    error: "Verification task timed out".to_string(),
                });
            }
        }

        let verification_result = adapters.get_result(verification_job_id).await?;

        // Compare results
        let original = task.result;
        let verification = verification_result.output;

        let result = Self::compare_results(&original, &verification, config.tolerance_threshold);

        // Handle verification result
        match &result {
            VerificationResult::Fail { task_id, contributor_id, reason, .. } => {
                // Record slashing event
                let slashing_event = SlashingEvent {
                    id: 0, // Will be set by database
                    wallet_address: contributor_id.clone(),
                    node_id: None, // Could be derived from contributor_id
                    severity: config.slash_severity,
                    slash_amount: 0.0, // Will be calculated
                    recurrence_count: 1, // Could be looked up from database
                    evidence_hash: Some(Self::calculate_evidence_hash(&original, &verification)),
                    occurred_at: Utc::now(),
                    appeal_status: "none".to_string(),
                };

                reputation_manager.record_slashing_event(slashing_event).await?;
                
                tracing::warn!("Fraud detected for task {} by contributor {}: {}", 
                    task_id, contributor_id, reason);
            }
            VerificationResult::Pass { task_id, contributor_id, .. } => {
                tracing::debug!("Task {} verification passed for contributor {}", 
                    task_id, contributor_id);
            }
            VerificationResult::Error { task_id, contributor_id, error } => {
                tracing::warn!("Task {} verification failed for contributor {}: {}", 
                    task_id, contributor_id, error);
            }
        }

        Ok(result)
    }

    fn compare_results(
        original: &serde_json::Value,
        verification: &serde_json::Value,
        tolerance: f64,
    ) -> VerificationResult {
        // For simplicity, we'll do a basic comparison
        // In production, this would be more sophisticated based on workload type
        
        let diff_score = Self::calculate_difference_score(original, verification);
        
        if diff_score <= tolerance {
            VerificationResult::Pass {
                task_id: Uuid::new_v4(), // Would be from actual task
                contributor_id: "unknown".to_string(), // Would be from actual task
                original_result: original.clone(),
                verification_result: verification.clone(),
                difference_score: diff_score,
            }
        } else {
            VerificationResult::Fail {
                task_id: Uuid::new_v4(),
                contributor_id: "unknown".to_string(),
                original_result: original.clone(),
                verification_result: verification.clone(),
                difference_score: diff_score,
                reason: format!("Result difference {} exceeds tolerance {}", diff_score, tolerance),
            }
        }
    }

    fn calculate_difference_score(original: &serde_json::Value, verification: &serde_json::Value) -> f64 {
        // Simple comparison - in production this would be workload-specific
        if original == verification {
            0.0
        } else {
            // Calculate difference based on type
            match (original, verification) {
                (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
                    let diff = (a.as_f64().unwrap_or(0.0) - b.as_f64().unwrap_or(0.0)).abs();
                    let max_val = a.as_f64().unwrap_or(1.0).max(b.as_f64().unwrap_or(1.0));
                    diff / max_val.max(1.0)
                }
                (serde_json::Value::String(a), serde_json::Value::String(b)) => {
                    if a == b { 0.0 } else { 1.0 }
                }
                _ => 1.0, // Different types or complex objects
            }
        }
    }

    fn calculate_evidence_hash(original: &serde_json::Value, verification: &serde_json::Value) -> String {
        use sha2::{Sha256, Digest};
        
        let data = format!("{}|{}", original, verification);
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    async fn process_verification_results(
        results: Vec<Result<VerificationResult, String>>,
    ) -> Result<(), String> {
        let mut passed = 0;
        let mut failed = 0;
        let mut errors = 0;

        for result in results {
            match result {
                Ok(VerificationResult::Pass { .. }) => passed += 1,
                Ok(VerificationResult::Fail { .. }) => failed += 1,
                Ok(VerificationResult::Error { .. }) => errors += 1,
                Err(e) => {
                    tracing::error!("Verification error: {}", e);
                    errors += 1;
                }
            }
        }

        tracing::info!("Watchdog cycle completed: {} passed, {} failed, {} errors", 
            passed, failed, errors);

        Ok(())
    }
}

// Mock implementations for testing
#[derive(Clone)]
pub struct MockTaskHistory {
    tasks: Arc<Mutex<Vec<TaskRecord>>>,
}

impl MockTaskHistory {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_task(&self, task: TaskRecord) {
        let mut tasks = self.tasks.lock().unwrap();
        tasks.push(task);
    }
}

#[async_trait]
impl TaskHistory for MockTaskHistory {
    async fn get_completed_tasks(&self, since: DateTime<Utc>) -> Result<Vec<TaskRecord>, String> {
        let tasks = self.tasks.lock().unwrap();
        Ok(tasks.iter()
            .filter(|task| task.completed_at >= since)
            .cloned()
            .collect())
    }

    async fn get_task_result(&self, task_id: JobId) -> Result<TaskResult, String> {
        let tasks = self.tasks.lock().unwrap();
        let task = tasks.iter().find(|t| t.task_id == task_id)
            .ok_or_else(|| format!("Task {} not found", task_id))?;

        Ok(TaskResult {
            task_id: task.task_id,
            contributor_id: task.contributor_id.clone(),
            workload_type: task.workload_type.clone(),
            params: task.params.clone(),
            result: task.result.clone(),
            execution_time_ms: 1000,
            success: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reputation::InMemoryRepo;

    #[tokio::test]
    async fn test_watchdog_config_defaults() {
        let config = WatchdogConfig::default();
        assert_eq!(config.check_interval_seconds, 300);
        assert_eq!(config.sample_rate, 0.1);
        assert_eq!(config.max_concurrent_checks, 5);
    }

    #[tokio::test]
    async fn test_difference_calculation() {
        let original = serde_json::json!(100.0);
        let verification = serde_json::json!(105.0);
        
        let diff = WatchdogScheduler::<InMemoryRepo, MockTaskHistory>::calculate_difference_score(&original, &verification);
        assert!(diff > 0.0 && diff < 1.0);
    }

    #[tokio::test]
    async fn test_evidence_hash() {
        let original = serde_json::json!("test");
        let verification = serde_json::json!("test");
        
        let hash = WatchdogScheduler::<InMemoryRepo, MockTaskHistory>::calculate_evidence_hash(&original, &verification);
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA256 hex length
    }
}