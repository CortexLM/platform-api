use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::Deserialize;
use uuid::Uuid;
use sqlx::Row;
use serde_json::Value as JsonValue;

use platform_api_models::{
    CreateChallengeRequest, UpdateChallengeRequest, ChallengeListResponse, 
    ChallengeDetailResponse, ChallengeMetadata, PlatformResult
};
use crate::state::AppState;

/// Create challenges router
pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/challenges", get(list_challenges).post(create_challenge))
        .route("/challenges/active", get(get_active_challenges))
        .route("/challenges/:id", get(get_challenge).put(update_challenge).delete(delete_challenge))
        .route("/challenges/:id/emissions", get(get_challenge_emissions))
        .route("/challenges/:id/jobs", get(get_challenge_jobs))
}

/// List challenges with pagination
pub async fn list_challenges(
    State(state): State<AppState>,
    Query(params): Query<ListChallengesParams>,
) -> Result<Json<ChallengeListResponse>, StatusCode> {
    let challenges = state.storage.list_challenges(
        params.page.unwrap_or(1),
        params.per_page.unwrap_or(20),
        params.status,
        params.visibility,
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(challenges))
}

/// Get challenge details
pub async fn get_challenge(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ChallengeDetailResponse>, StatusCode> {
    let challenge = state.storage.get_challenge(id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(challenge))
}

/// Create new challenge
pub async fn create_challenge(
    State(state): State<AppState>,
    Json(request): Json<CreateChallengeRequest>,
) -> Result<Json<ChallengeMetadata>, StatusCode> {
    let challenge = state.builder.create_challenge(request).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(challenge))
}

/// Update challenge
pub async fn update_challenge(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateChallengeRequest>,
) -> Result<Json<ChallengeMetadata>, StatusCode> {
    let challenge = state.builder.update_challenge(id, request).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(challenge))
}

/// Delete challenge
pub async fn delete_challenge(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    state.builder.delete_challenge(id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Get challenge emissions
pub async fn get_challenge_emissions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<platform_api_models::EmissionsSchedule>, StatusCode> {
    let emissions = state.storage.get_challenge_emissions(id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(emissions))
}

/// Get active challenges only
pub async fn get_active_challenges(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Use challenge_registry instead of storage.list_challenges
    // because storage.list_challenges returns empty list (challenges are in memory registry)
    let challenges = state.list_challenges().await;
    
    // Debug: log registry size
    let registry_size = {
        let registry = state.challenge_registry.read().await;
        registry.len()
    };
    tracing::info!("📋 get_active_challenges: registry size = {}, challenges returned = {}", registry_size, challenges.len());
    
    Ok(Json(serde_json::json!({
        "challenges": challenges.iter().map(|c| serde_json::json!({
            "id": c.id,
            "name": c.name,
            "compose_hash": c.compose_hash,
            "status": "Active", // All challenges in registry are considered active
        })).collect::<Vec<_>>()
    })))
}

/// Query parameters for listing challenges
#[derive(Debug, Deserialize)]
pub struct ListChallengesParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub status: Option<String>,
    pub visibility: Option<String>,
}

/// Get all jobs for a challenge with results
pub async fn get_challenge_jobs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<ChallengeJobsParams>,
) -> Result<Json<JsonValue>, StatusCode> {
    if let Some(pool) = &state.database_pool {
        let page = params.page.unwrap_or(1);
        let per_page = params.per_page.unwrap_or(20);
        let offset = (page - 1) * per_page;
        
        // Get jobs for this challenge
        let jobs = state.scheduler.list_jobs(
            page,
            per_page,
            params.status,
            Some(id),
        ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        // For each job, get test results count
        let jobs_with_results: Vec<JsonValue> = jobs.jobs.iter().map(|job| {
            // Count test results for this job (we'll fetch details separately if needed)
            serde_json::json!({
                "job": job,
                "has_test_results": false, // Will be populated if test_results detail is requested
            })
        }).collect();
        
        Ok(Json(serde_json::json!({
            "challenge_id": id,
            "jobs": jobs_with_results,
            "total": jobs.total,
            "page": jobs.page,
            "per_page": jobs.per_page,
        })))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// Query parameters for challenge jobs
#[derive(Debug, Deserialize)]
pub struct ChallengeJobsParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub status: Option<String>,
    pub include_test_results: Option<bool>,
}


