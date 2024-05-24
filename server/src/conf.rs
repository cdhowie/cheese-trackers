use std::net::SocketAddr;

use base64::prelude::*;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305};
use config::ConfigError;
use jsonwebtoken::Algorithm;
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use url::Url;

#[derive(Deserialize)]
pub struct Config {
    pub public_url: Url,
    pub http_listen: SocketAddr,
    #[serde(default)]
    pub cors_permissive: bool,
    #[serde(default)]
    pub banners: Vec<Banner>,
    #[serde(rename = "tracker_update_interval_mins")]
    #[serde(deserialize_with = "de_duration_mins")]
    pub tracker_update_interval: chrono::Duration,

    pub token: Token,
    pub database: Database,
    pub discord: Discord,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Banner {
    // If an ID is present, the frontend allows the banner to be dismissed by
    // the user.  The ID will be persisted in local storage and used to filter
    // out dismissed banners.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub message: String,
    pub kind: BannerKind,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BannerKind {
    Danger,
    Warning,
    Success,
    Info,
}

#[derive(Deserialize)]
pub struct Token {
    #[serde(default = "default_algorithm")]
    pub algorithm: Algorithm,
    pub secret: String,
    #[serde(rename = "validity_duration_days")]
    #[serde(deserialize_with = "de_duration_days")]
    pub validity_duration: chrono::Duration,
    pub issuer: String,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Database {
    #[cfg(feature = "postgres")]
    Postgres { connection_string: String },
}

#[derive(Deserialize)]
pub struct Discord {
    pub client_id: String,
    pub client_secret: String,
    #[serde(rename = "token_cipher_key")]
    #[serde(deserialize_with = "de_token_cipher")]
    pub token_cipher: XChaCha20Poly1305,
}

fn default_algorithm() -> Algorithm {
    Algorithm::HS256
}

fn de_duration_mins<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<chrono::Duration, D::Error> {
    Deserialize::deserialize(deserializer).map(chrono::Duration::minutes)
}

fn de_duration_days<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<chrono::Duration, D::Error> {
    Deserialize::deserialize(deserializer).map(chrono::Duration::days)
}

fn de_token_cipher<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<XChaCha20Poly1305, D::Error> {
    let key = BASE64_STANDARD
        .decode(String::deserialize(deserializer)?)
        .map_err(|e| D::Error::custom(format!("cannot decode token cipher key: {e}")))?;

    XChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| D::Error::custom(format!("failed to create cipher: {e}")))
}

pub fn load() -> Result<Config, ConfigError> {
    config::Config::builder()
        .add_source(config::File::with_name("config"))
        .build()?
        .try_deserialize()
}
