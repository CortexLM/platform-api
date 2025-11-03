use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::Deserialize;
use uuid::Uuid;

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
    let challenges = state.storage.list_challenges(
        1,
        100,
        Some("active".to_string()),
        None,
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "challenges": challenges.challenges.iter().map(|c| serde_json::json!({
            "id": c.id,
            "name": c.name,
            "status": c.status,
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


