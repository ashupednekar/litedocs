use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::internal::app::AppState;
use crate::prelude::{Result, StandardError};

#[derive(Debug, Deserialize)]
pub struct StartRegistrationRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct StartRegistrationResponse {
    pub status: &'static str,
    pub challenge: String,
    pub note: &'static str,
}

pub async fn start_registration(
    State(_state): State<AppState>,
    Json(payload): Json<StartRegistrationRequest>,
) -> Result<Json<StartRegistrationResponse>> {
    if payload.name.trim().is_empty() {
        return Err(StandardError::new("ER-0004").code(StatusCode::BAD_REQUEST));
    }
    Ok(Json(StartRegistrationResponse {
        status: "ok",
        challenge: "placeholder_challenge".to_string(),
        note: "TODO: wire actual WebAuthn registration challenge generation",
    }))
}

#[derive(Debug, Deserialize)]
pub struct FinishRegistrationRequest {
    pub user_id: String,
    pub credential: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct GenericOk {
    pub status: &'static str,
    pub note: &'static str,
}

pub async fn finish_registration(
    State(_state): State<AppState>,
    Json(_payload): Json<FinishRegistrationRequest>,
) -> Result<Json<GenericOk>> {
    Ok(Json(GenericOk {
        status: "ok",
        note: "TODO: verify attestation and persist credential",
    }))
}

#[derive(Debug, Deserialize)]
pub struct StartAuthenticationRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct StartAuthenticationResponse {
    pub status: &'static str,
    pub challenge: String,
    pub note: &'static str,
}

pub async fn start_authentication(
    State(_state): State<AppState>,
    Json(payload): Json<StartAuthenticationRequest>,
) -> Result<Json<StartAuthenticationResponse>> {
    if payload.name.trim().is_empty() {
        return Err(StandardError::new("ER-0004").code(StatusCode::BAD_REQUEST));
    }
    Ok(Json(StartAuthenticationResponse {
        status: "ok",
        challenge: "placeholder_auth_challenge".to_string(),
        note: "TODO: load user credentials and create WebAuthn auth challenge",
    }))
}

#[derive(Debug, Deserialize)]
pub struct FinishAuthenticationRequest {
    pub credential: serde_json::Value,
}

pub async fn finish_authentication(
    State(_state): State<AppState>,
    Json(_payload): Json<FinishAuthenticationRequest>,
) -> Result<Json<GenericOk>> {
    Ok(Json(GenericOk {
        status: "ok",
        note: "TODO: verify assertion and create app session",
    }))
}
