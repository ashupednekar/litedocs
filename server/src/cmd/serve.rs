use axum::serve;
use clap::Args;
use standard_error::StandardError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use url::Url;
use uuid::Uuid;
use webauthn_rs::prelude::WebauthnBuilder;

use crate::conf::SETTINGS;
use crate::internal::db::connect_if_configured;
use crate::pkg::server::router::new_router;
use crate::prelude::Result;
use crate::state::AppState;

#[derive(Debug, Default, Args)]
pub struct ServeCmd {}

impl ServeCmd {
    pub async fn run(self) -> Result<()> {
        let pool = connect_if_configured(SETTINGS.database_url.as_deref())
            .await
            .map_err(|_| StandardError::new("ER-0001"))?;
        let rp_origin = Url::parse(&SETTINGS.webauthn_rp_origin)
            .map_err(|_| StandardError::new("ER-0004"))?;
        let webauthn = WebauthnBuilder::new(&SETTINGS.webauthn_rp_id, &rp_origin)
            .map_err(|_| StandardError::new("ER-0001"))?
            .build()
            .map_err(|_| StandardError::new("ER-0001"))?;
        let app = new_router(AppState {
            db_pool: pool,
            database_schema: SETTINGS.database_schema.clone(),
            webauthn: Arc::new(webauthn),
            user_ids: Arc::new(RwLock::new(HashMap::<String, Uuid>::new())),
            user_passkeys: Arc::new(RwLock::new(HashMap::new())),
            pending_registration: Arc::new(RwLock::new(HashMap::new())),
            pending_authentication: Arc::new(RwLock::new(HashMap::new())),
        });
        let listener = TcpListener::bind(&SETTINGS.listen_addr)
            .await
            .map_err(|_| StandardError::new("ER-0001"))?;
        println!("litedocs-server listening on http://{}", SETTINGS.listen_addr);
        serve(listener, app)
            .await
            .map_err(|_| StandardError::new("ER-0001"))?;
        Ok(())
    }
}
