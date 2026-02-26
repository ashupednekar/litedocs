use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::RwLock;
use uuid::Uuid;
use webauthn_rs::prelude::{Passkey, PasskeyAuthentication, PasskeyRegistration, Webauthn};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Option<Arc<PgPool>>,
    pub database_schema: String,
    pub webauthn: Arc<Webauthn>,
    pub user_ids: Arc<RwLock<std::collections::HashMap<String, Uuid>>>,
    pub user_passkeys: Arc<RwLock<std::collections::HashMap<String, Vec<Passkey>>>>,
    pub pending_registration: Arc<RwLock<std::collections::HashMap<String, PasskeyRegistration>>>,
    pub pending_authentication: Arc<RwLock<std::collections::HashMap<String, PasskeyAuthentication>>>,
}
