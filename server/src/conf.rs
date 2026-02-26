use config::{Config, ConfigError, Environment};
use lazy_static::lazy_static;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
    pub database_url: Option<String>,
    #[serde(default = "default_database_schema")]
    pub database_schema: String,
    #[serde(default = "default_rp_id")]
    pub webauthn_rp_id: String,
    #[serde(default = "default_rp_origin")]
    pub webauthn_rp_origin: String,
}

impl Settings {
    fn new() -> Result<Self, ConfigError> {
        let cfg = Config::builder()
            .add_source(Environment::default())
            .build()?;
        cfg.try_deserialize()
    }
}

fn default_listen_addr() -> String {
    "127.0.0.1:8080".to_string()
}

fn default_database_schema() -> String {
    "public".to_string()
}

fn default_rp_id() -> String {
    "localhost".to_string()
}

fn default_rp_origin() -> String {
    "http://localhost:8080".to_string()
}

lazy_static! {
    // Loaded once from env and reused across command/runtime paths.
    pub static ref SETTINGS: Settings = Settings::new().expect("invalid server settings");
}
