//! HTTP API server for job queue bridge between Rust and Python
//!
//! Provides REST endpoints for:
//! - Job submission
//! - Job status queries
//! - Job result retrieval
//! - Job cancellation
//!
//! This allows the Python orchestrator to communicate with the Rust job queue.

use actix_web::{web, App, HttpResponse, HttpServer, Responder, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::job_queue::{JobDispatcher, JobId, JobStatus, JobResult};
use crate::tool_adapter::{ToolType, ToolParams, ToolResult};

/// API server configuration
#[derive(Clone)]
pub struct ApiServer {
    dispatcher: Arc<RwLock<JobDispatcher>>,
}

impl ApiServer {
    pub fn new(dispatcher: JobDispatcher) -> Self {
        Self {
            dispatcher: Arc::new(RwLock::new(dispatcher)),
        }
    }

    pub async fn start(self, bind_addr: &str) -> std::io::Result<()> {
        let dispatcher = self.dispatcher.clone();
        
        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(dispatcher.clone()))
                .route("/jobs/submit", web::post().to(submit_job))
                .route("/jobs/{job_id}/status", web::get().to(get_job_status))
                .route("/jobs/{job_id}/result", web::get().to(get_job_result))
                .route("/jobs/{job_id}/cancel", web::post().to(cancel_job))
                .route("/health", web::get().to(health_check))
        })
        .bind(bind_addr)?
        .run()
        .await
    }
}

/// Request for job submission
#[derive(Deserialize)]
pub struct SubmitJobRequest {
    pub tool_type: String,
    pub params: serde_json::Value,
}

/// Response for job submission
#[derive(Serialize)]
pub struct SubmitJobResponse {
    pub job_id: String,
    pub status: String,
}

/// Response for job status
#[derive(Serialize)]
pub struct JobStatusResponse {
    pub job_id: String,
    pub status: String,
    pub tool_type: String,
    pub created_at: String,
    pub updated_at: Option<String>,
}

/// Response for job result
#[derive(Serialize)]
pub struct JobResultResponse {
    pub job_id: String,
    pub status: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: Option<u32>,
    pub content_hash: Option<String>,
}

/// Submit a new job
async fn submit_job(
    dispatcher: web::Data<Arc<RwLock<JobDispatcher>>>,
    req: web::Json<SubmitJobRequest>,
) -> Result<HttpResponse> {
    let dispatcher = dispatcher.read().await;
    
    // Convert tool type string to enum
    let tool_type = match req.tool_type.as_str() {
        "text_generation" => ToolType::TextGeneration,
        "image_generation" => ToolType::ImageGeneration,
        "video_processing" => ToolType::VideoProcessing,
        "audio_processing" => ToolType::AudioProcessing,
        "general_compute" => ToolType::GeneralCompute,
        _ => return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid tool type"
        }))),
    };

    let params = ToolParams::new(req.params.clone());
    
    match dispatcher.submit_job(tool_type, params).await {
        Ok(job_id) => Ok(HttpResponse::Ok().json(SubmitJobResponse {
            job_id: job_id.to_string(),
            status: "queued".to_string(),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e
        }))),
    }
}

/// Get job status
async fn get_job_status(
    dispatcher: web::Data<Arc<RwLock<JobDispatcher>>>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let job_id = match path.into_inner().parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid job ID format"
        }))),
    };

    let dispatcher = dispatcher.read().await;
    
    match dispatcher.get_job_status(job_id).await {
        Ok(status) => {
            let job_info = dispatcher.get_job_info(job_id).await;
            Ok(HttpResponse::Ok().json(JobStatusResponse {
                job_id: job_id.to_string(),
                status: match status {
                    JobStatus::Queued => "queued".to_string(),
                    JobStatus::Running => "running".to_string(),
                    JobStatus::Completed => "completed".to_string(),
                    JobStatus::Failed => "failed".to_string(),
                    JobStatus::Cancelled => "cancelled".to_string(),
                },
                tool_type: job_info.map(|j| format!("{:?}", j.tool_type)).unwrap_or_default(),
                created_at: job_info.map(|j| j.created_at.to_rfc3339()).unwrap_or_default(),
                updated_at: job_info.and_then(|j| j.updated_at).map(|t| t.to_rfc3339()),
            }))
        },
        Err(e) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": e
        }))),
    }
}

/// Get job result
async fn get_job_result(
    dispatcher: web::Data<Arc<RwLock<JobDispatcher>>>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let job_id = match path.into_inner().parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid job ID format"
        }))),
    };

    let dispatcher = dispatcher.read().await;
    
    match dispatcher.get_job_result(job_id).await {
        Ok(result) => {
            let response = JobResultResponse {
                job_id: job_id.to_string(),
                status: "completed".to_string(),
                result: Some(result.output),
                error: None,
                execution_time_ms: Some(result.execution_time_ms),
                content_hash: result.content_hash,
            };
            Ok(HttpResponse::Ok().json(response))
        },
        Err(e) => {
            // Check if it's a "not found" error or actual failure
            if e.contains("not found") {
                Ok(HttpResponse::NotFound().json(serde_json::json!({
                    "error": e
                })))
            } else {
                Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": e
                })))
            }
        },
    }
}

/// Cancel a job
async fn cancel_job(
    dispatcher: web::Data<Arc<RwLock<JobDispatcher>>>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let job_id = match path.into_inner().parse::<Uuid>() {
        Ok(id) => id,
        Err(_) => return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid job ID format"
        }))),
    };

    let dispatcher = dispatcher.read().await;
    
    match dispatcher.cancel_job(job_id).await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "message": "Job cancelled successfully"
        }))),
        Err(e) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": e
        }))),
    }
}

/// Health check endpoint
async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "service": "swarm-media-api",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use serde_json::json;

    #[actix_web::test]
    async fn test_submit_job() {
        let dispatcher = JobDispatcher::new();
        let api_server = ApiServer::new(dispatcher);
        let data = web::Data::new(api_server.dispatcher);

        let app = test::init_service(
            App::new()
                .app_data(data)
                .route("/jobs/submit", web::post().to(submit_job))
        ).await;

        let req = test::TestRequest::post()
            .uri("/jobs/submit")
            .set_json(&SubmitJobRequest {
                tool_type: "text_generation".to_string(),
                params: json!({"prompt": "Test prompt"}),
            })
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_health_check() {
        let app = test::init_service(
            App::new().route("/health", web::get().to(health_check))
        ).await;

        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;
        
        assert!(resp.status().is_success());
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["status"], "healthy");
    }
}