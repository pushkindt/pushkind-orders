//! Configuration model loaded from external sources.
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub app: AppConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub domain: String,
    pub database_url: String,
    pub secret: String,
    pub auth_service_url: String,
    pub crm_service_url: String,
}
