use std::net::SocketAddr;

use config::ConfigError;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub http_listen: SocketAddr,
    #[serde(default)]
    pub cors_permissive: bool,
    #[serde(default)]
    pub is_staging: bool,
    #[serde(rename = "tracker_update_interval_mins")]
    #[serde(deserialize_with = "de_duration_mins")]
    pub tracker_update_interval: chrono::Duration,
    pub database: Database,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Database {
    Postgres { connection_string: String },
}

fn de_duration_mins<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<chrono::Duration, D::Error> {
    Deserialize::deserialize(deserializer).map(chrono::Duration::minutes)
}

pub fn load() -> Result<Config, ConfigError> {
    config::Config::builder()
        .add_source(config::File::with_name("config"))
        .build()?
        .try_deserialize()
}
