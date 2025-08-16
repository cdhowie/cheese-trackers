//! Service configuration.

use std::net::SocketAddr;

use axum_client_ip::ClientIpSource;
use base64::prelude::*;
use chacha20poly1305::{KeyInit, XChaCha20Poly1305};
use config::ConfigError;
use jsonwebtoken::Algorithm;
use serde::{Deserialize, Deserializer, Serialize, de::Error};
use url::Url;

/// The top-level service configuration.
#[derive(Deserialize)]
pub struct Config {
    /// The public URL of the tracker.
    pub public_url: Url,
    /// The network endpoint the tracker should listen on for requests.
    pub http_listen: SocketAddr,
    /// Where to get the client IP address for incoming requests.
    pub client_ip_source: ClientIpSource,
    /// Whether to add permissive CORS headers.
    #[serde(default)]
    pub cors_permissive: bool,
    /// Name of the entity providing the instance, to properly direct support
    /// requests.
    pub hoster: Option<String>,
    /// Banners to display in the frontend.
    #[serde(default)]
    pub banners: Vec<Banner>,

    /// Permitted list of upstream trackers.
    #[serde(deserialize_with = "deser_upstream_trackers")]
    pub upstream_trackers: Vec<UpstreamTracker>,

    /// The minimum allowed time between consecutive updates of a single tracker
    /// from the upstream tracker source.
    #[serde(rename = "tracker_update_interval_mins")]
    #[serde(deserialize_with = "de_duration_mins")]
    pub tracker_update_interval: chrono::Duration,

    /// JWT configuration.
    pub token: Token,
    /// Database configuration.
    pub database: Database,
    /// Discord authentication configuration.
    pub discord: Discord,
}

fn deser_upstream_trackers<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<UpstreamTracker>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum UpstreamTrackerOrUrl {
        Url(Url),
        UpstreamTracker(UpstreamTracker),
    }

    let items = Vec::<UpstreamTrackerOrUrl>::deserialize(deserializer)?;

    items
        .into_iter()
        .map(|v| match v {
            UpstreamTrackerOrUrl::UpstreamTracker(t) => Ok(t),
            UpstreamTrackerOrUrl::Url(u) => {
                let host = u.host_str().ok_or_else(|| {
                    serde::de::Error::custom(format!("upstream tracker has no host: {u}"))
                })?;

                Ok(UpstreamTracker {
                    ap_host: host.into(),
                    url_prefix: u,
                })
            }
        })
        .collect::<Result<_, _>>()
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpstreamTracker {
    pub url_prefix: Url,
    pub ap_host: String,
}

/// A banner to be displayed in the frontend.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Banner {
    /// The banner's unique ID.
    ///
    /// If an ID is present, the frontend allows the banner to be dismissed by
    /// the user.  The ID will be persisted in local storage and used to filter
    /// out dismissed banners.
    ///
    /// Banners without an ID cannot be dismissed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The message to display.
    ///
    /// This can contain HTML markup.
    pub message: String,
    /// The banner's kind.
    pub kind: BannerKind,
}

/// The kind of a [`Banner`].
///
/// The variants of this enum directly relate to Bootstrap contextual classes
/// that can be applied to
/// [alerts](https://getbootstrap.com/docs/5.3/components/alerts/).
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BannerKind {
    Danger,
    Warning,
    Success,
    Info,
}

/// JWT configuration.
#[derive(Deserialize)]
pub struct Token {
    /// The JWT algorithm to use.
    ///
    /// If omitted, HS256 is used.
    #[serde(default = "default_algorithm")]
    pub algorithm: Algorithm,
    /// The shared secred used to encrypt and decrypt tokens.
    pub secret: String,
    /// The duration for which tokens are valid from the time they are issued.
    #[serde(rename = "validity_duration_days")]
    #[serde(deserialize_with = "de_duration_days")]
    pub validity_duration: chrono::Duration,
    /// The token issuer.
    pub issuer: String,
}

/// Database configuration.
#[derive(Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Database {
    /// Connect to a PostgreSQL database.
    #[cfg(feature = "postgres")]
    Postgres {
        /// The [PostgreSQL connection string](sqlx::postgres::PgConnectOptions)
        /// to use when connecting to the database.
        connection_string: String,
    },
}

/// Discord authentication configuration.
#[derive(Deserialize)]
pub struct Discord {
    /// Discord app client ID.
    pub client_id: String,
    /// Discord app client secret.
    pub client_secret: String,
    /// Cipher used to encrypt and decrypt continuation tokens.  See
    /// [`auth::discord`](crate::auth::discord) for more information.
    #[serde(rename = "token_cipher_key")]
    #[serde(deserialize_with = "de_token_cipher")]
    pub token_cipher: XChaCha20Poly1305,
}

#[doc(hidden)]
fn default_algorithm() -> Algorithm {
    Algorithm::HS256
}

/// Deserializes a duration expressed as a number of minutes.
fn de_duration_mins<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<chrono::Duration, D::Error> {
    Deserialize::deserialize(deserializer).map(chrono::Duration::minutes)
}

/// Deserializes a duration expressed as a number of days.
fn de_duration_days<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<chrono::Duration, D::Error> {
    Deserialize::deserialize(deserializer).map(chrono::Duration::days)
}

/// Deserializes a XChaCha20Poly1305 cipher.
fn de_token_cipher<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<XChaCha20Poly1305, D::Error> {
    let key = BASE64_STANDARD
        .decode(String::deserialize(deserializer)?)
        .map_err(|e| D::Error::custom(format!("cannot decode token cipher key: {e}")))?;

    XChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| D::Error::custom(format!("failed to create cipher: {e}")))
}

/// Loads the configuration from disk.
///
/// Looks in the working directory for a file with the base name `config` and
/// with a supported extension, such as `.json` or `.yaml`.
pub fn load() -> Result<Config, ConfigError> {
    config::Config::builder()
        .add_source(config::File::with_name("config"))
        .build()?
        .try_deserialize()
}
