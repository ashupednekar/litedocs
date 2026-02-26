use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webauthn_rs::prelude::{
    PublicKeyCredential, RegisterPublicKeyCredential, RequestChallengeResponse,
};

use crate::prelude::{Result, StandardError, Status};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct StartRegistrationRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct StartRegistrationResponse {
    pub status: &'static str,
    pub challenge: webauthn_rs::prelude::CreationChallengeResponse,
    pub note: &'static str,
}

pub async fn start_registration(
    State(state): State<AppState>,
    Json(payload): Json<StartRegistrationRequest>,
) -> Result<Json<StartRegistrationResponse>> {
    if payload.name.trim().is_empty() {
        return Err(StandardError::new("ER-0004").code(StatusCode::BAD_REQUEST));
    }
    let user_id = {
        let mut ids = state.user_ids.write().await;
        match ids.get(&payload.name) {
            Some(id) => *id,
            None => {
                let id = Uuid::new_v4();
                ids.insert(payload.name.clone(), id);
                id
            }
        }
    };
    let challenge = state
        .webauthn
        .start_passkey_registration(user_id, &payload.name, &payload.name, None)
        .map_err(|_| StandardError::new("ER-0001"))?;
    let (ccr, reg_state) = challenge;
    state
        .pending_registration
        .write()
        .await
        .insert(payload.name.clone(), reg_state);
    Ok(Json(StartRegistrationResponse {
        status: "ok",
        challenge: ccr,
        note: "registration challenge generated",
    }))
}

#[derive(Debug, Deserialize)]
pub struct FinishRegistrationRequest {
    pub name: String,
    pub credential: RegisterPublicKeyCredential,
}

#[derive(Debug, Serialize)]
pub struct GenericOk {
    pub status: &'static str,
    pub note: &'static str,
}

pub async fn finish_registration(
    State(state): State<AppState>,
    Json(payload): Json<FinishRegistrationRequest>,
) -> Result<Json<GenericOk>> {
    let reg_state = state
        .pending_registration
        .write()
        .await
        .remove(&payload.name)
        .ok_or_else(|| StandardError::new("ER-AXUM-BADREQUEST").code(StatusCode::BAD_REQUEST))?;
    let passkey = state
        .webauthn
        .finish_passkey_registration(&payload.credential, &reg_state)
        .map_err(|_| StandardError::new("ER-0001"))?;
    let mut store = state.user_passkeys.write().await;
    store
        .entry(payload.name.clone())
        .or_default()
        .push(passkey);
    Ok(Json(GenericOk {
        status: "ok",
        note: "registration completed",
    }))
}

#[derive(Debug, Deserialize)]
pub struct StartAuthenticationRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct StartAuthenticationResponse {
    pub status: &'static str,
    pub challenge: RequestChallengeResponse,
    pub note: &'static str,
}

pub async fn start_authentication(
    State(state): State<AppState>,
    Json(payload): Json<StartAuthenticationRequest>,
) -> Result<Json<StartAuthenticationResponse>> {
    if payload.name.trim().is_empty() {
        return Err(StandardError::new("ER-0004").code(StatusCode::BAD_REQUEST));
    }
    let passkeys = {
        let store = state.user_passkeys.read().await;
        store.get(&payload.name).cloned().unwrap_or_default()
    };
    if passkeys.is_empty() {
        return Err(StandardError::new("ER-AXUM-NOTFOUND").code(StatusCode::NOT_FOUND));
    }
    let (challenge, auth_state) = state
        .webauthn
        .start_passkey_authentication(&passkeys)
        .map_err(|_| StandardError::new("ER-0001"))?;
    state
        .pending_authentication
        .write()
        .await
        .insert(payload.name.clone(), auth_state);
    Ok(Json(StartAuthenticationResponse {
        status: "ok",
        challenge,
        note: "authentication challenge generated",
    }))
}

#[derive(Debug, Deserialize)]
pub struct FinishAuthenticationRequest {
    pub name: String,
    pub credential: PublicKeyCredential,
}

pub async fn finish_authentication(
    State(state): State<AppState>,
    Json(payload): Json<FinishAuthenticationRequest>,
) -> Result<Json<GenericOk>> {
    let auth_state = state
        .pending_authentication
        .write()
        .await
        .remove(&payload.name)
        .ok_or_else(|| StandardError::new("ER-AXUM-BADREQUEST").code(StatusCode::BAD_REQUEST))?;
    state
        .webauthn
        .finish_passkey_authentication(&payload.credential, &auth_state)
        .map_err(|_| StandardError::new("ER-0001"))?;
    Ok(Json(GenericOk {
        status: "ok",
        note: "authentication completed",
    }))
}
