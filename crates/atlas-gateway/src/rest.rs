//! REST API routes and handlers.

use crate::db::Database;
use crate::error::GatewayError;
use crate::graphql::AppSchema;
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::{Path, Query, State},
    http::{Method, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

/// Application state.
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub schema: AppSchema,
}

/// Pagination parameters.
#[derive(Debug, Deserialize)]
pub struct Pagination {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    20
}

/// Create the API router.
pub fn create_router(db: Database, schema: AppSchema) -> Router {
    let state = AppState { db, schema };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers(Any);

    Router::new()
        // Health and status
        .route("/health", get(health))
        .route("/status", get(status))
        // GraphQL
        .route("/graphql", post(graphql_handler))
        .route("/graphql/playground", get(graphql_playground))
        // REST API v1
        .nest("/api/v1", api_routes())
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

/// API routes.
fn api_routes() -> Router<AppState> {
    Router::new()
        // Stats
        .route("/stats", get(get_stats))
        // Blocks
        .route("/blocks", get(get_blocks))
        .route("/blocks/latest", get(get_latest_block))
        .route("/blocks/:number", get(get_block))
        .route("/blocks/:number/extrinsics", get(get_block_extrinsics))
        .route("/blocks/:number/events", get(get_block_events))
        // Extrinsics
        .route("/extrinsics", get(get_extrinsics))
        .route("/extrinsics/:hash", get(get_extrinsic))
        // Events
        .route("/events", get(get_events))
        // Comits
        .route("/comits", get(get_comits))
        .route("/comits/:hash", get(get_comit))
        // Accounts
        .route("/accounts/:address", get(get_account))
        .route("/accounts/:address/extrinsics", get(get_account_extrinsics))
        .route("/accounts/:address/comits", get(get_account_comits))
}

// ============================================================================
// Health endpoints
// ============================================================================

async fn health() -> impl IntoResponse {
    StatusCode::OK
}

#[derive(Serialize)]
struct StatusResponse {
    status: String,
    latest_block: Option<i64>,
    total_blocks: i64,
    total_comits: i64,
}

async fn status(State(state): State<AppState>) -> Result<Json<StatusResponse>, GatewayError> {
    let stats = state.db.get_stats().await?;

    Ok(Json(StatusResponse {
        status: "ok".to_string(),
        latest_block: stats.latest_block,
        total_blocks: stats.total_blocks,
        total_comits: stats.total_comits,
    }))
}

// ============================================================================
// GraphQL endpoints
// ============================================================================

async fn graphql_handler(State(state): State<AppState>, req: GraphQLRequest) -> GraphQLResponse {
    state.schema.execute(req.into_inner()).await.into()
}

async fn graphql_playground() -> impl IntoResponse {
    Html(async_graphql::http::playground_source(
        async_graphql::http::GraphQLPlaygroundConfig::new("/graphql"),
    ))
}

// ============================================================================
// Block endpoints
// ============================================================================

async fn get_blocks(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, GatewayError> {
    let blocks = state
        .db
        .get_recent_blocks(pagination.limit.min(100), pagination.offset)
        .await?;
    Ok(Json(blocks))
}

async fn get_latest_block(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, GatewayError> {
    let block = state.db.get_latest_block().await?;
    match block {
        Some(b) => Ok(Json(b)),
        None => Err(GatewayError::NotFound("No blocks indexed yet".to_string())),
    }
}

async fn get_block(
    State(state): State<AppState>,
    Path(number): Path<i64>,
) -> Result<impl IntoResponse, GatewayError> {
    let block = state.db.get_block(number).await?;
    match block {
        Some(b) => Ok(Json(b)),
        None => Err(GatewayError::NotFound(format!(
            "Block {} not found",
            number
        ))),
    }
}

async fn get_block_extrinsics(
    State(state): State<AppState>,
    Path(number): Path<i64>,
) -> Result<impl IntoResponse, GatewayError> {
    let extrinsics = state.db.get_block_extrinsics(number).await?;
    Ok(Json(extrinsics))
}

async fn get_block_events(
    State(state): State<AppState>,
    Path(number): Path<i64>,
) -> Result<impl IntoResponse, GatewayError> {
    let events = state.db.get_block_events(number).await?;
    Ok(Json(events))
}

// ============================================================================
// Extrinsic endpoints
// ============================================================================

async fn get_extrinsics(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, GatewayError> {
    let extrinsics = state
        .db
        .get_recent_extrinsics(pagination.limit.min(100), pagination.offset)
        .await?;
    Ok(Json(extrinsics))
}

async fn get_extrinsic(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<impl IntoResponse, GatewayError> {
    let extrinsic = state.db.get_extrinsic(&hash).await?;
    match extrinsic {
        Some(e) => Ok(Json(e)),
        None => Err(GatewayError::NotFound(format!(
            "Extrinsic {} not found",
            hash
        ))),
    }
}

// ============================================================================
// Event endpoints
// ============================================================================

#[derive(Deserialize)]
struct EventQuery {
    pallet: Option<String>,
    variant: Option<String>,
    #[serde(flatten)]
    pagination: Pagination,
}

async fn get_events(
    State(state): State<AppState>,
    Query(query): Query<EventQuery>,
) -> Result<impl IntoResponse, GatewayError> {
    let limit = query.pagination.limit.min(100);
    let offset = query.pagination.offset;

    let events = match (query.pallet, query.variant) {
        (Some(pallet), Some(variant)) => {
            state
                .db
                .get_events_by_type(&pallet, &variant, limit, offset)
                .await?
        }
        (Some(pallet), None) => {
            state
                .db
                .get_events_by_pallet(&pallet, limit, offset)
                .await?
        }
        _ => {
            // No filter - return error, need at least pallet filter
            return Err(GatewayError::BadRequest(
                "pallet parameter required".to_string(),
            ));
        }
    };

    Ok(Json(events))
}

// ============================================================================
// Comit endpoints
// ============================================================================

async fn get_comits(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, GatewayError> {
    let comits = state
        .db
        .get_recent_comits(pagination.limit.min(100), pagination.offset)
        .await?;
    Ok(Json(comits))
}

async fn get_comit(
    State(state): State<AppState>,
    Path(hash): Path<String>,
) -> Result<impl IntoResponse, GatewayError> {
    let comit = state.db.get_comit(&hash).await?;
    match comit {
        Some(c) => Ok(Json(c)),
        None => Err(GatewayError::NotFound(format!("Comit {} not found", hash))),
    }
}

// ============================================================================
// Account endpoints
// ============================================================================

async fn get_account(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<impl IntoResponse, GatewayError> {
    let account = state.db.get_account(&address).await?;
    match account {
        Some(a) => Ok(Json(a)),
        None => Err(GatewayError::NotFound(format!(
            "Account {} not found",
            address
        ))),
    }
}

async fn get_account_extrinsics(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, GatewayError> {
    let extrinsics = state
        .db
        .get_account_extrinsics(&address, pagination.limit.min(100), pagination.offset)
        .await?;
    Ok(Json(extrinsics))
}

async fn get_account_comits(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(pagination): Query<Pagination>,
) -> Result<impl IntoResponse, GatewayError> {
    let comits = state
        .db
        .get_account_comits(&address, pagination.limit.min(100), pagination.offset)
        .await?;
    Ok(Json(comits))
}

// ============================================================================
// Stats endpoint
// ============================================================================

async fn get_stats(State(state): State<AppState>) -> Result<impl IntoResponse, GatewayError> {
    let stats = state.db.get_stats().await?;
    Ok(Json(stats))
}
