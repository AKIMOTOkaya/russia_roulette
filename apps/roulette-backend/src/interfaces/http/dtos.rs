//! HTTP request and response DTO schemas and error mappings.

#![forbid(unsafe_code)]

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use roulette_domain::{PlayerCommand, RoomMemberId, TabId};
use roulette_host::ServiceError;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TabQuery {
    pub tab_id: TabId,
}

#[derive(Deserialize)]
pub struct TabRequest {
    pub tab_id: TabId,
}

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    pub tab_id: TabId,
    pub room_name: String,
    pub player_name: String,
    pub password: Option<String>,
}

#[derive(Deserialize)]
pub struct JoinRoomRequest {
    pub tab_id: TabId,
    pub room_id: String,
    pub player_name: String,
    pub password: Option<String>,
}

#[derive(Deserialize)]
pub struct MemberRequest {
    pub tab_id: TabId,
    pub member_id: RoomMemberId,
}

#[derive(Deserialize)]
pub struct StartRoomRequest {
    pub tab_id: TabId,
    pub seed: Option<u64>,
}

#[derive(Deserialize)]
pub struct CommandRequest {
    pub tab_id: TabId,
    pub expected_revision: u64,
    pub command: PlayerCommand,
}

#[derive(Deserialize)]
pub struct StepRequest {
    pub tab_id: TabId,
    pub expected_revision: u64,
}

#[derive(Deserialize)]
pub struct FounderAuthRequest {
    pub tab_id: TabId,
    pub password: String,
}

#[derive(Deserialize)]
pub struct ServerSettingsRequest {
    pub tab_id: TabId,
    pub expose_lan: bool,
}

#[derive(Deserialize)]
pub struct McpCallRequest {
    pub tool: String,
    pub arguments: serde_json::Value,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Serialize)]
pub struct FounderAuthResponse {
    pub authenticated: bool,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    #[must_use]
    pub fn from_service(error: &ServiceError) -> Self {
        let status = match error {
            ServiceError::NotFound(_) => StatusCode::NOT_FOUND,
            ServiceError::Forbidden(_) => StatusCode::FORBIDDEN,
            ServiceError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ServiceError::Conflict(_) => StatusCode::CONFLICT,
            ServiceError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}
