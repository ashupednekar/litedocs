use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::CorsLayer;

use crate::pkg::server::handlers::{probes, webauthn};
use crate::state::AppState;

pub fn new_router(state: AppState) -> Router {
    Router::new()
        .route("/livez", get(probes::live))
        .route("/healthz", get(probes::health))
        .nest(
            "/api/webauthn",
            Router::new()
                .route("/register/start", post(webauthn::start_registration))
                .route("/register/finish", post(webauthn::finish_registration))
                .route("/auth/start", post(webauthn::start_authentication))
                .route("/auth/finish", post(webauthn::finish_authentication)),
        )
        .layer(CorsLayer::permissive())
        .with_state(state)
}
