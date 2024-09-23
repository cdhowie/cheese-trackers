//! Tracker endpoints.

use std::{fmt::Display, str::FromStr, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::token::AuthenticatedUser,
    db::{
        model::{
            ApGame, ApGameIden, ApHint, ApHintIden, ApTracker, ApTrackerIden, AvailabilityStatus,
            CompletionStatus, HintClassification, PingPreference, ProgressionStatus,
        },
        DataAccess, DataAccessProvider, Transactable, Transaction,
    },
    logging::UnexpectedResultExt,
    state::{AppState, TrackerUpdateError},
};

const URLSAFE_BASE64_UUID_LEN: usize = 22;

/// URL-safe base64-encoded UUID.
#[derive(Debug, Clone, Copy)]
pub struct UrlEncodedTrackerId {
    /// The UUID value.
    uuid: Uuid,
    /// Pre-encoded URL-safe base64 string representation of the UUID.  Storing
    /// this inline increases the size of the value but allows easily casting
    /// these values to &str, which means String allocations can be skipped in
    /// some cases.
    string: [u8; URLSAFE_BASE64_UUID_LEN],
}

impl UrlEncodedTrackerId {
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.string).unwrap()
    }
}

// We can skip the string field because it's derived from the uuid, so this
// winds up being more efficient.
impl PartialEq for UrlEncodedTrackerId {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl From<Uuid> for UrlEncodedTrackerId {
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

impl From<UrlEncodedTrackerId> for Uuid {
    fn from(value: UrlEncodedTrackerId) -> Self {
        value.uuid
    }
}

impl AsRef<str> for UrlEncodedTrackerId {
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

impl FromStr for UrlEncodedTrackerId {
    type Err = UrlEncodedTrackerIdDecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::from_slice(&URL_SAFE_NO_PAD.decode(s)?)?;

        let mut string = [0u8; URLSAFE_BASE64_UUID_LEN];
        string.copy_from_slice(s.as_bytes());

        Ok(Self { uuid, string })
    }
}

impl Display for UrlEncodedTrackerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for UrlEncodedTrackerId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        String::deserialize(deserializer)?
            .parse()
            .map_err(D::Error::custom)
    }
}

impl Serialize for UrlEncodedTrackerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_str().serialize(serializer)
    }
}

/// `GET /tracker/:tracker_id`: Get tracker.
pub async fn get_tracker<D>(
    State(state): State<Arc<AppState<D>>>,
    Path(tracker_id): Path<UrlEncodedTrackerId>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    // Same as ApTracker but with tracker_id encoded.
    #[derive(Debug, Clone, serde::Serialize)]
    pub struct Tracker {
        pub id: i32,
        pub tracker_id: UrlEncodedTrackerId,
        pub updated_at: DateTime<Utc>,
        pub title: String,
        pub description: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub owner_ct_user_id: Option<i32>,
        pub lock_settings: bool,
        pub upstream_url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub global_ping_policy: Option<PingPreference>,
    }

    impl From<ApTracker> for Tracker {
        fn from(value: ApTracker) -> Self {
            Self {
                id: value.id,
                tracker_id: value.tracker_id.into(),
                updated_at: value.updated_at,
                title: value.title,
                description: value.description,
                owner_ct_user_id: value.owner_ct_user_id,
                lock_settings: value.lock_settings,
                upstream_url: value.upstream_url,
                global_ping_policy: value.global_ping_policy,
            }
        }
    }

    #[derive(serde::Serialize)]
    struct GetTrackerResponse {
        #[serde(flatten)]
        pub tracker: Tracker,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub owner_discord_username: Option<String>,
        pub games: Vec<ApGame>,
        pub hints: Vec<ApHint>,
    }

    let upstream_url = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?
        .get_tracker_by_tracker_id(tracker_id.into())
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?
        .upstream_url;

    if let Err(err) = state.upsert_tracker(&upstream_url).await {
        // Log this error but do not fail the overall operation; if we have old
        // data in the database then we can still use it.
        println!("Failed to update tracker {tracker_id}: {err}");

        // ... unless the upstream isn't whitelisted.
        if matches!(&*err, &TrackerUpdateError::UpstreamNotWhitelisted) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let mut tx = db.begin().await.unexpected()?;

    let tracker = tx
        .get_tracker_by_tracker_id(tracker_id.into())
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

    // TODO: Convert this to a join.
    let owner_discord_username = match tracker.owner_ct_user_id {
        None => None,
        Some(uid) => {
            Some(
                tx.get_ct_user_by_id(uid)
                    .await
                    .unexpected()?
                    .ok_or_else(|| {
                        // This should not be possible due to the foreign key
                        // constraint, and we are running in a transaction.
                        eprintln!(
                            "Owner of tracker {} user ID {} doesn't exist",
                            tracker.id, uid
                        );
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?
                    .discord_username,
            )
        }
    };

    let games = tx
        .get_ap_games_by_tracker_id(tracker.id)
        .try_collect()
        .await
        .unexpected()?;

    let hints = tx
        .get_ap_hints_by_tracker_id(tracker.id)
        .try_collect()
        .await
        .unexpected()?;

    tx.rollback().await.unexpected()?;
    drop(db);

    Ok(Json(GetTrackerResponse {
        tracker: tracker.into(),
        owner_discord_username,
        games,
        hints,
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateTrackerRequest {
    pub url: String,
}

/// `POST /tracker`: Create/get tracker by upstream URL.
pub async fn create_tracker<D>(
    State(state): State<Arc<AppState<D>>>,
    Json(body): Json<CreateTrackerRequest>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    #[derive(serde::Serialize)]
    struct CreateTrackerResponse {
        pub tracker_id: UrlEncodedTrackerId,
    }

    let tracker_id = match state.upsert_tracker(&body.url).await {
        Ok(v) => v,
        Err(e) if matches!(&*e, TrackerUpdateError::UpstreamNotWhitelisted) => {
            return Err(StatusCode::FORBIDDEN);
        }
        Err(e) => {
            println!("Failed to fetch tracker from {}: {e}", body.url);

            // We couldn't get/update the tracker but maybe we have data we've
            // fetched before.
            state
                .data_provider
                .create_data_access()
                .await
                .unexpected()?
                .get_tracker_by_upstream_url(&body.url)
                .await
                .unexpected()?
                .ok_or_else(|| {
                    // The database has no record of this URL, so map the
                    // various tracker fetch errors to reasonable HTTP status
                    // codes.
                    use TrackerUpdateError::*;
                    match &*e {
                        ParseUrl(_) => StatusCode::BAD_REQUEST,
                        UpstreamNotWhitelisted => StatusCode::FORBIDDEN,
                        TrackerNotFound => StatusCode::NOT_FOUND,

                        Http(_)
                        | Parse(_)
                        | Database(_)
                        | GameCountMismatch { .. }
                        | GameInformationMismatch(_)
                        | NumericConversion(_)
                        | HintGameMissing(_) => StatusCode::INTERNAL_SERVER_ERROR,
                    }
                })?
                .tracker_id
        }
    };

    Ok(Json(CreateTrackerResponse {
        tracker_id: tracker_id.into(),
    }))
}

/// Request body for [`update_tracker`].
#[derive(Debug, serde::Deserialize)]
pub struct UpdateTrackerRequest {
    pub title: String,
    #[serde(default)] // Backwards-compatibility
    pub description: String,
    pub owner_ct_user_id: Option<i32>,
    #[serde(alias = "lock_title")] // Backwards-compatibility
    pub lock_settings: bool,
    pub global_ping_policy: Option<PingPreference>,
}

/// `PUT /tracker/:tracker_id`: Update tracker.
pub async fn update_tracker<D>(
    State(state): State<Arc<AppState<D>>>,
    Path(tracker_id): Path<UrlEncodedTrackerId>,
    user: Option<AuthenticatedUser>,
    Json(tracker_update): Json<UpdateTrackerRequest>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let mut tx = db.begin().await.unexpected()?;

    let mut tracker = tx
        .get_tracker_by_tracker_id(tracker_id.into())
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Update settings.  Some settings are handled specially:
    //
    // * owner_ct_user_id can only be updated if it's not set and the current
    //   user is setting it to their own user ID (claiming) or if it's set to
    //   the current user's ID and the current user is unsetting it
    //   (disclaiming).  In all other cases, the update request is rejected.
    // * lock_settings can only be changed by the organizer, and if there is no
    //   organizer then there is no point in setting it because anyone could
    //   unset it.
    // * description allows adding arbitrary text and links.  This could allow
    //   CT to become an unwitting accomplice in e.g. phishing schemes.
    //   Therefore, this field can only be edited by the organizer.

    if tracker_update.owner_ct_user_id != tracker.owner_ct_user_id {
        // A change in ownership requires authentication.
        let user = user.as_ref().ok_or(StatusCode::UNAUTHORIZED)?;

        // The only valid changes are claiming or disclaiming ownership, and the
        // user ID must match the authenticated user.
        tracker.owner_ct_user_id = match (tracker.owner_ct_user_id, tracker_update.owner_ct_user_id)
        {
            (None, Some(uid)) | (Some(uid), None) if uid == user.0.id => {
                tracker_update.owner_ct_user_id
            }
            _ => return Err(StatusCode::FORBIDDEN),
        };
    }

    match (tracker.owner_ct_user_id, user, tracker.lock_settings) {
        // The current user is the owner.  They can change all settings.
        (Some(uid), Some(u), _) if uid == u.0.id => {
            tracker.lock_settings = tracker_update.lock_settings;
            tracker.description = tracker_update.description;
        }

        // The current user is not the owner and settings are locked.  They
        // cannot change anything.
        (Some(_), _, true) => return Err(StatusCode::FORBIDDEN),

        // There is no current owner or the current user is not the owner but
        // settings are unlocked.  In both cases, they can change anything
        // except the description, and whether settings are locked.
        (None, _, _) | (_, _, false) => {
            if tracker_update.lock_settings || tracker_update.description != tracker.description {
                // Locking settings makes no sense without an owner, since
                // anyone could unlock them.
                return Err(StatusCode::FORBIDDEN);
            }

            tracker.lock_settings = false;
        }
    };

    tracker.title = tracker_update.title;
    tracker.global_ping_policy = tracker_update.global_ping_policy;

    tx.update_ap_tracker(
        tracker,
        &[
            ApTrackerIden::Title,
            ApTrackerIden::Description,
            ApTrackerIden::OwnerCtUserId,
            ApTrackerIden::LockSettings,
            ApTrackerIden::GlobalPingPolicy,
        ],
    )
    .await
    .unexpected()?;

    tx.commit().await.unexpected()?;

    Ok(StatusCode::NO_CONTENT)
}

/// Request body for [`update_hint`].
#[derive(Debug, serde::Deserialize)]
pub struct UpdateHintRequest {
    pub classification: HintClassification,
}

/// `PUT /tracker/:tracker_id/hint/:hint_id`: Update hint.
pub async fn update_hint<D>(
    State(state): State<Arc<AppState<D>>>,
    Path((tracker_id, hint_id)): Path<(UrlEncodedTrackerId, i32)>,
    Json(hint_update): Json<UpdateHintRequest>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let mut tx = db.begin().await.unexpected()?;

    let tracker = tx
        .get_tracker_by_tracker_id(tracker_id.into())
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut hint = tx
        .get_ap_hint(hint_id)
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

    let game = tx
        .get_ap_game(hint.finder_game_id)
        .await
        .unexpected()?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    if game.tracker_id != tracker.id {
        return Err(StatusCode::NOT_FOUND);
    }

    hint.classification = hint_update.classification;

    let hint = tx
        .update_ap_hint(hint, &[ApHintIden::Classification])
        .await
        .unexpected()?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit().await.unexpected()?;

    Ok(Json(hint))
}

/// Request body for [`update_game`].
#[derive(Debug, serde::Deserialize)]
pub struct UpdateGameRequest {
    pub claimed_by_ct_user_id: Option<i32>,
    pub discord_username: Option<String>,
    pub discord_ping: PingPreference,
    pub availability_status: AvailabilityStatus,
    pub completion_status: CompletionStatus,
    pub progression_status: ProgressionStatus,
    pub last_checked: Option<DateTime<Utc>>,
    pub notes: String,
}

/// `PUT /tracker/:tracker_id/game/:game_id`: Update game.
pub async fn update_game<D>(
    State(state): State<Arc<AppState<D>>>,
    user: Option<AuthenticatedUser>,
    Path((tracker_id, game_id)): Path<(UrlEncodedTrackerId, i32)>,
    Json(game_update): Json<UpdateGameRequest>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let mut tx = db.begin().await.unexpected()?;

    let tracker = tx
        .get_tracker_by_tracker_id(tracker_id.into())
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut game = tx
        .get_ap_game(game_id)
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

    if game.tracker_id != tracker.id {
        return Err(StatusCode::NOT_FOUND);
    }

    // If the claimed user ID is changing to a value other than None, it must
    // match the authenticated user's ID.
    if game_update.claimed_by_ct_user_id != game.claimed_by_ct_user_id
        && game_update
            .claimed_by_ct_user_id
            .is_some_and(|id| user.as_ref().map_or(true, |u| u.0.id != id))
    {
        return Err(match user {
            // A user is trying to claim on behalf of another user.
            Some(_) => StatusCode::FORBIDDEN,
            // A user is trying to claim while unauthenticated; their token
            // probably expired.
            None => StatusCode::UNAUTHORIZED,
        });
    }

    // Update the username.
    game.discord_username = match game_update.claimed_by_ct_user_id {
        // If claimed by an authenticated user, this username is not needed and
        // can be set to NULL.
        Some(_) => None,
        // Otherwise, take the unauthenticated username from the update.
        None => game_update.discord_username,
    };

    game.claimed_by_ct_user_id = game_update.claimed_by_ct_user_id;
    game.discord_ping = game_update.discord_ping;
    game.availability_status = game_update.availability_status;
    game.completion_status = game_update.completion_status;
    game.progression_status = game_update.progression_status;
    game.last_checked = game_update.last_checked;
    game.notes = game_update.notes;

    game.update_completion_status();

    let game_id = game.id;
    let game = tx
        .update_ap_game(
            game,
            &[
                ApGameIden::ClaimedByCtUserId,
                ApGameIden::DiscordUsername,
                ApGameIden::DiscordPing,
                ApGameIden::AvailabilityStatus,
                ApGameIden::CompletionStatus,
                ApGameIden::ProgressionStatus,
                ApGameIden::LastChecked,
                ApGameIden::Notes,
            ],
        )
        .await
        .unexpected()?
        // There should be no way this is None since we're in a transaction and
        // already fetched the record.
        .ok_or_else(|| format!("ApGame {game_id} did not exist on update"))
        .unexpected()?;

    tx.commit().await.unexpected()?;

    Ok(Json(game))
}
