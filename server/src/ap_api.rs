use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Deserializer};
use url::Url;

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
}

fn deser_last_activity<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error> {
    Ok(
        NaiveDateTime::parse_from_str(&String::deserialize(deserializer)?, "%a, %-d %b %Y %T %Z")
            .map_err(serde::de::Error::custom)?
            .and_utc(),
    )
}
