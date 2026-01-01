//! RPC server for x3-sidecar

use crate::job::{Job, JobQueue};
use crate::SidecarState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// RPC server state
pub struct RpcState {
    pub job_queue: Arc<RwLock<JobQueue>>,
    pub sidecar_state: Arc<RwLock<SidecarState>>,
}

/// Create the RPC router
pub fn create_router(state: Arc<RpcState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/jobs", post(submit_job))
        .route("/jobs/:id", get(get_job))
        .route("/jobs/:id/cancel", post(cancel_job))
        .route("/queue/stats", get(queue_stats))
        .route("/queue/clear", post(clear_queue))
        .route("/metrics", get(metrics))
        .with_state(state)
}

/// Health check
async fn health() -> &'static str {
    "OK"
}

/// Status response
#[derive(Serialize)]
struct StatusResponse {
    version: &'static str,
    uptime_secs: u64,
    jobs_executed: u64,
    jobs_pending: usize,
    executor_registered: bool,
}

async fn status(State(state): State<Arc<RpcState>>) -> Json<StatusResponse> {
    let queue = state.job_queue.read().await;
    let sidecar = state.sidecar_state.read().await;

    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION"),
        uptime_secs: sidecar.start_time.elapsed().as_secs(),
        jobs_executed: sidecar.jobs_completed,
        jobs_pending: queue.len(),
        executor_registered: sidecar.registered,
    })
}

/// Job submission request
#[derive(Deserialize)]
struct SubmitJobRequest {
    /// Bytecode in hex
    bytecode: String,
    /// Input data in hex
    input: Option<String>,
    /// Gas limit
    gas_limit: Option<u64>,
    /// Priority (1-10)
    priority: Option<u8>,
    /// Callback URL for completion notification
    callback_url: Option<String>,
}

/// Job submission response
#[derive(Serialize)]
struct SubmitJobResponse {
    job_id: String,
    position: usize,
}

async fn submit_job(
    State(state): State<Arc<RpcState>>,
    Json(request): Json<SubmitJobRequest>,
) -> Result<Json<SubmitJobResponse>, (StatusCode, String)> {
    // Parse bytecode
    let bytecode = hex::decode(request.bytecode.trim_start_matches("0x"))
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid bytecode hex: {}", e)))?;

    // Parse input
    let input = match request.input {
        Some(hex) => hex::decode(hex.trim_start_matches("0x"))
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid input hex: {}", e)))?,
        None => vec![],
    };

    // Create job
    let job = Job {
        id: generate_job_id(),
        bytecode,
        input,
        gas_limit: request.gas_limit.unwrap_or(1_000_000),
        priority: request.priority.unwrap_or(5),
        callback_url: request.callback_url,
        submitted_at: std::time::Instant::now(),
        started_at: None,
    };

    let job_id = hex::encode(job.id);

    // Add to queue
    let mut queue = state.job_queue.write().await;
    queue.push(job);
    let position = queue.len();

    Ok(Json(SubmitJobResponse { job_id, position }))
}

/// Job info response
#[derive(Serialize)]
struct JobInfoResponse {
    job_id: String,
    status: String,
    gas_limit: u64,
    priority: u8,
    queued_for_secs: u64,
}

async fn get_job(
    State(state): State<Arc<RpcState>>,
    Path(job_id): Path<String>,
) -> Result<Json<JobInfoResponse>, (StatusCode, String)> {
    let id_bytes: [u8; 32] = hex::decode(job_id.trim_start_matches("0x"))
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid job ID: {}", e)))?
        .try_into()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Job ID must be 32 bytes".to_string()))?;

    let queue = state.job_queue.read().await;

    // Find job in queue
    for job in queue.iter() {
        if job.id == id_bytes {
            return Ok(Json(JobInfoResponse {
                job_id: hex::encode(job.id),
                status: if job.started_at.is_some() {
                    "running"
                } else {
                    "pending"
                }
                .to_string(),
                gas_limit: job.gas_limit,
                priority: job.priority,
                queued_for_secs: job.submitted_at.elapsed().as_secs(),
            }));
        }
    }

    Err((StatusCode::NOT_FOUND, "Job not found".to_string()))
}

async fn cancel_job(
    State(state): State<Arc<RpcState>>,
    Path(job_id): Path<String>,
) -> Result<&'static str, (StatusCode, String)> {
    let id_bytes: [u8; 32] = hex::decode(job_id.trim_start_matches("0x"))
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid job ID: {}", e)))?
        .try_into()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Job ID must be 32 bytes".to_string()))?;

    let mut queue = state.job_queue.write().await;

    if queue.remove(&id_bytes) {
        Ok("Job cancelled")
    } else {
        Err((StatusCode::NOT_FOUND, "Job not found or already running".to_string()))
    }
}

/// Queue stats response
#[derive(Serialize)]
struct QueueStatsResponse {
    pending: usize,
    running: usize,
    completed: u64,
    failed: u64,
    avg_wait_time_ms: u64,
}

async fn queue_stats(State(state): State<Arc<RpcState>>) -> Json<QueueStatsResponse> {
    let queue = state.job_queue.read().await;
    let stats = queue.stats();

    Json(QueueStatsResponse {
        pending: stats.pending,
        running: stats.running,
        completed: stats.completed,
        failed: stats.failed,
        avg_wait_time_ms: stats.avg_wait_time_ms,
    })
}

async fn clear_queue(State(state): State<Arc<RpcState>>) -> &'static str {
    let mut queue = state.job_queue.write().await;
    queue.clear();
    "Queue cleared"
}

/// Prometheus metrics endpoint
async fn metrics(State(state): State<Arc<RpcState>>) -> String {
    let queue = state.job_queue.read().await;
    let sidecar = state.sidecar_state.read().await;
    let stats = queue.stats();

    format!(
        r#"# HELP x3_sidecar_jobs_pending Number of pending jobs
# TYPE x3_sidecar_jobs_pending gauge
x3_sidecar_jobs_pending {}

# HELP x3_sidecar_jobs_running Number of running jobs
# TYPE x3_sidecar_jobs_running gauge
x3_sidecar_jobs_running {}

# HELP x3_sidecar_jobs_completed_total Total completed jobs
# TYPE x3_sidecar_jobs_completed_total counter
x3_sidecar_jobs_completed_total {}

# HELP x3_sidecar_jobs_failed_total Total failed jobs
# TYPE x3_sidecar_jobs_failed_total counter
x3_sidecar_jobs_failed_total {}

# HELP x3_sidecar_uptime_seconds Sidecar uptime in seconds
# TYPE x3_sidecar_uptime_seconds gauge
x3_sidecar_uptime_seconds {}

# HELP x3_sidecar_avg_wait_time_ms Average job wait time in milliseconds
# TYPE x3_sidecar_avg_wait_time_ms gauge
x3_sidecar_avg_wait_time_ms {}
"#,
        stats.pending,
        stats.running,
        stats.completed,
        stats.failed,
        sidecar.start_time.elapsed().as_secs(),
        stats.avg_wait_time_ms
    )
}

/// Generate a unique job ID
fn generate_job_id() -> [u8; 32] {
    use blake2::{Blake2s256, Digest};

    let mut hasher = Blake2s256::new();
    hasher.update(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .to_le_bytes());
    hasher.update(&rand::random::<[u8; 16]>());

    let result = hasher.finalize();
    let mut id = [0u8; 32];
    id.copy_from_slice(&result);
    id
}
