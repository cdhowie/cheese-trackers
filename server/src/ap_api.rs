use std::{fmt::Display, hash::Hash, str::FromStr};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use serde_cow::CowStr;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Client {
    base: Url,
    client: reqwest::Client,
}

impl Client {
    #[allow(dead_code)]
    pub fn new(base: Url) -> Self {
        Self::new_with_client(base, Default::default())
    }

    pub fn new_with_client(base: Url, client: reqwest::Client) -> Self {
        Self { base, client }
    }

    pub async fn get_room_status(&self, room_id: &str) -> reqwest::Result<RoomStatusResponse> {
        self.client
            .get(self.base.join(&format!("room_status/{room_id}")).unwrap())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoomStatusResponse {
    #[serde(deserialize_with = "deser_last_activity")]
    pub last_activity: DateTime<Utc>,
    pub last_port: u16,
    #[serde(rename = "timeout")]
    pub timeout_sec: u32,
    pub tracker: UrlEncodedUuid,
}

fn deser_last_activity<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error> {
    Ok(
        NaiveDateTime::parse_from_str(&CowStr::deserialize(deserializer)?.0, "%a, %-d %b %Y %T %Z")
            .map_err(serde::de::Error::custom)?
            .and_utc(),
    )
}

const URLSAFE_BASE64_UUID_LEN: usize = 22;

/// URL-safe base64-encoded UUID.
#[derive(Debug, Clone, Copy)]
pub struct UrlEncodedUuid {
    /// The UUID value.
    uuid: Uuid,
    /// Pre-encoded URL-safe base64 string representation of the UUID.  Storing
    /// this inline increases the size of the value but allows easily casting
    /// these values to &str, which means String allocations can be skipped in
    /// some cases.
    string: [u8; URLSAFE_BASE64_UUID_LEN],
}

impl UrlEncodedUuid {
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.string).unwrap()
    }
}

// We can skip the string field because it's derived from the uuid, so this
// winds up being more efficient.
impl PartialEq for UrlEncodedUuid {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl Eq for UrlEncodedUuid {}

impl Hash for UrlEncodedUuid {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

impl From<Uuid> for UrlEncodedUuid {
    fn from(value: Uuid) -> Self {
        let mut string = [0; URLSAFE_BASE64_UUID_LEN];

        URL_SAFE_NO_PAD
            .encode_slice(value.as_bytes(), &mut string)
            .unwrap();

        Self {
            uuid: value,
            string,
        }
    }
}

impl From<UrlEncodedUuid> for Uuid {
    fn from(value: UrlEncodedUuid) -> Self {
        value.uuid
    }
}

impl AsRef<str> for UrlEncodedUuid {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UrlEncodedTrackerIdDecodeError {
    #[error("could not base64-decode tracker ID: {0}")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("could not uuid-decode tracker ID: {0}")]
    UuidDecode(#[from] uuid::Error),
}

impl FromStr for UrlEncodedUuid {
    type Err = UrlEncodedTrackerIdDecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::from_slice(&URL_SAFE_NO_PAD.decode(s)?)?;

        let mut string = [0u8; URLSAFE_BASE64_UUID_LEN];
        string.copy_from_slice(s.as_bytes());

        Ok(Self { uuid, string })
    }
}

impl Display for UrlEncodedUuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for UrlEncodedUuid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        CowStr::deserialize(deserializer)?
            .0
            .parse()
            .map_err(D::Error::custom)
    }
}

impl Serialize for UrlEncodedUuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_str().serialize(serializer)
    }
}
