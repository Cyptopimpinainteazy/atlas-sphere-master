//! HTTP server for metrics and health checks.

use crate::error::Result;
use crate::metrics::Metrics;
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use prometheus::Encoder;
use serde_json::json;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::info;

/// Application state.
#[derive(Clone)]
struct AppState {
    metrics: Metrics,
}

/// Run the HTTP server.
pub async fn run(port: u16, metrics: Metrics) -> Result<()> {
    let state = AppState { metrics };

    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(prometheus_metrics))
        .route("/status", get(status))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting HTTP server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint.
async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "service": "atlas-indexer"
    }))
}

/// Readiness check endpoint.
async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    let latest_block = state.metrics.latest_block();

    if latest_block > 0 {
        (
            StatusCode::OK,
            Json(json!({
                "ready": true,
                "latest_block": latest_block
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "ready": false,
                "reason": "No blocks indexed yet"
            })),
        )
    }
}

/// Prometheus metrics endpoint.
async fn prometheus_metrics(State(state): State<AppState>) -> impl IntoResponse {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = state.metrics.registry().gather();

    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, prometheus::TEXT_FORMAT)],
        buffer,
    )
}

/// Status endpoint with detailed metrics.
async fn status(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "service": "atlas-indexer",
        "version": env!("CARGO_PKG_VERSION"),
        "metrics": {
            "blocks_indexed": state.metrics.total_blocks(),
            "latest_block": state.metrics.latest_block(),
            "total_errors": state.metrics.total_errors()
        }
    }))
}
