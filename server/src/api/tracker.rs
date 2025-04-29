//! Tracker endpoints.

use std::{fmt::Display, future::ready, str::FromStr, sync::Arc};

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderName, StatusCode},
    response::IntoResponse,
};
use axum_client_ip::ClientIp;
use axum_extra::{TypedHeader, headers::Header};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, TimeDelta, Utc};
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::token::AuthenticatedUser,
    db::{
        DataAccess, DataAccessProvider, Transactable, Transaction, create_audit_for,
        model::{
            ApGame, ApGameIden, ApHint, ApHintIden, ApTracker, ApTrackerDashboardOverride,
            ApTrackerIden, AvailabilityStatus, CompletionStatus, HintClassification,
            PingPreference, ProgressionStatus, UpdateCompletionStatus,
        },
    },
    logging::UnexpectedResultExt,
    send_hack::{send_future, send_stream},
    state::{AppState, GetRoomLinkError, TrackerUpdateError},
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

/// `GET /tracker/{tracker_id}`: Get tracker.
pub async fn get_tracker<D>(
    State(state): State<Arc<AppState<D>>>,
    Path(tracker_id): Path<UrlEncodedTrackerId>,
    user: Option<AuthenticatedUser>,
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
        pub room_link: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_port: Option<i32>,
        pub inactivity_threshold_yellow_hours: i32,
        pub inactivity_threshold_red_hours: i32,
        pub require_authentication_to_claim: bool,
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
                room_link: value.room_link,
                last_port: value.last_port,
                inactivity_threshold_yellow_hours: value.inactivity_threshold_yellow_hours,
                inactivity_threshold_red_hours: value.inactivity_threshold_red_hours,
                require_authentication_to_claim: value.require_authentication_to_claim,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        pub dashboard_override_visibility: Option<bool>,
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

    let dashboard_override_visibility = match user {
        None => None,
        Some(u) => tx
            .get_ap_tracker_dashboard_override(u.user.id, tracker.id)
            .await
            .unexpected()?
            .map(|o| o.visibility),
    };

    send_future(tx.rollback()).await.unexpected()?;
    drop(db);

    Ok(Json(GetTrackerResponse {
        tracker: tracker.into(),
        owner_discord_username,
        games,
        hints,
        dashboard_override_visibility,
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
    pub room_link: String,
    pub inactivity_threshold_yellow_hours: i32,
    pub inactivity_threshold_red_hours: i32,
    pub require_authentication_to_claim: bool,
}

/// `PUT /tracker/{tracker_id}`: Update tracker.
pub async fn update_tracker<D>(
    State(state): State<Arc<AppState<D>>>,
    ClientIp(ip): ClientIp,
    user: Option<AuthenticatedUser>,
    Path(tracker_id): Path<UrlEncodedTrackerId>,
    Json(tracker_update): Json<UpdateTrackerRequest>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    if tracker_update.inactivity_threshold_yellow_hours < 0
        || tracker_update.inactivity_threshold_red_hours < 0
        || tracker_update.inactivity_threshold_yellow_hours
            > tracker_update.inactivity_threshold_red_hours
    {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let mut tx = db.begin().await.unexpected()?;

    let old_tracker = tx
        .get_tracker_by_tracker_id(tracker_id.into())
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut tracker = old_tracker.clone();

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
            (None, Some(uid)) | (Some(uid), None) if uid == user.user.id => {
                tracker_update.owner_ct_user_id
            }
            _ => return Err(StatusCode::FORBIDDEN),
        };
    }

    match (tracker.owner_ct_user_id, &user, tracker.lock_settings) {
        // The current user is the owner.  They can change all settings.
        (Some(uid), Some(u), _) if uid == u.user.id => {
            tracker.lock_settings = tracker_update.lock_settings;
            tracker.description = tracker_update.description;

            // Some settings are not useful if settings aren't locked.
            if !tracker_update.lock_settings
                && (!tracker.require_authentication_to_claim
                    && tracker_update.require_authentication_to_claim)
            {
                return Err(StatusCode::FORBIDDEN);
            }

            tracker.require_authentication_to_claim =
                tracker_update.lock_settings && tracker_update.require_authentication_to_claim;
        }

        // The current user is not the owner and settings are locked.  They
        // cannot change anything.
        (Some(_), _, true) => return Err(StatusCode::FORBIDDEN),

        // There is no current owner or the current user is not the owner but
        // settings are unlocked.  In both cases, they can change almost
        // anything.  Some settings do not make sense to change when settings
        // aren't locked.
        (None, _, _) | (_, _, false) => {
            if tracker_update.lock_settings
                || tracker_update.description != tracker.description
                || tracker_update.require_authentication_to_claim
            {
                return Err(StatusCode::FORBIDDEN);
            }

            tracker.lock_settings = false;
            tracker.require_authentication_to_claim = false;
        }
    };

    tracker.title = tracker_update.title;
    tracker.global_ping_policy = tracker_update.global_ping_policy;
    tracker.inactivity_threshold_yellow_hours = tracker_update.inactivity_threshold_yellow_hours;
    tracker.inactivity_threshold_red_hours = tracker_update.inactivity_threshold_red_hours;

    if tracker.room_link != tracker_update.room_link {
        tracker.room_link = tracker_update.room_link;

        (tracker.last_port, tracker.next_port_check_at) = match tracker.room_link.is_empty() {
            true => (None, None),
            false => {
                match state
                    .get_last_port(&tracker.room_link, &tracker.upstream_url)
                    .await
                {
                    Ok((port, check)) => (Some(port.into()), Some(check)),

                    Err(GetRoomLinkError::UrlParse(_) | GetRoomLinkError::InvalidRoomLink) => {
                        return Err(StatusCode::UNPROCESSABLE_ENTITY);
                    }

                    Err(e) => {
                        eprintln!(
                            "During tracker update request, failed to fetch room info for {:?} for tracker {:?}: {e}",
                            tracker.room_link, tracker.upstream_url
                        );

                        (None, Utc::now().checked_add_signed(TimeDelta::minutes(5)))
                    }
                }
            }
        };
    }

    let audit = create_audit_for(Some(ip), user.as_ref(), Utc::now(), &old_tracker, &tracker);

    if let Some(audit) = audit {
        tx.update_ap_tracker(
            tracker,
            &[
                ApTrackerIden::Title,
                ApTrackerIden::Description,
                ApTrackerIden::OwnerCtUserId,
                ApTrackerIden::LockSettings,
                ApTrackerIden::GlobalPingPolicy,
                ApTrackerIden::RoomLink,
                ApTrackerIden::LastPort,
                ApTrackerIden::NextPortCheckAt,
                ApTrackerIden::InactivityThresholdYellowHours,
                ApTrackerIden::InactivityThresholdRedHours,
                ApTrackerIden::RequireAuthenticationToClaim,
            ],
        )
        .await
        .unexpected()?;

        send_stream(tx.create_audits([audit]))
            .try_for_each(|_| ready(Ok(())))
            .await
            .unexpected()?;
    }

    send_future(tx.commit()).await.unexpected()?;

    get_tracker(State(state), Path(tracker_id), user).await
}

/// Request body for [`update_hint`].
#[derive(Debug, serde::Deserialize)]
pub struct UpdateHintRequest {
    pub classification: HintClassification,
}

/// `PUT /tracker/{tracker_id}/hint/{hint_id}`: Update hint.
pub async fn update_hint<D>(
    State(state): State<Arc<AppState<D>>>,
    ClientIp(ip): ClientIp,
    user: Option<AuthenticatedUser>,
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

    let old_hint = tx
        .get_ap_hint(hint_id)
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

    let game = tx
        .get_ap_game(old_hint.finder_game_id)
        .await
        .unexpected()?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    if game.tracker_id != tracker.id {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut hint = old_hint.clone();

    hint.classification = hint_update.classification;

    let audit = create_audit_for(Some(ip), user.as_ref(), Utc::now(), &old_hint, &hint);

    let hint = tx
        .update_ap_hint(hint, &[ApHintIden::Classification])
        .await
        .unexpected()?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    send_stream(tx.create_audits(audit))
        .try_for_each(|_| ready(Ok(())))
        .await
        .unexpected()?;

    send_future(tx.commit()).await.unexpected()?;

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

pub struct IfOwnerIs {
    pub condition: Option<IfOwnerIsCondition>,
}

#[derive(serde::Deserialize)]
pub struct IfOwnerIsCondition {
    pub claimed_by_ct_user_id: Option<i32>,
    pub discord_username: Option<String>,
}

impl IfOwnerIsCondition {
    pub fn matches(&self, game: &ApGame) -> bool {
        self.claimed_by_ct_user_id == game.claimed_by_ct_user_id
            && self.discord_username == game.discord_username
    }
}

impl Header for IfOwnerIs {
    fn name() -> &'static HeaderName {
        static NAME: HeaderName = HeaderName::from_static("x-if-owner-is");

        &NAME
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum_extra::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'i axum::http::HeaderValue>,
    {
        let condition = values
            .next()
            .map(|v| serde_json::from_slice(v.as_bytes()))
            .transpose()
            .map_err(|_| axum_extra::headers::Error::invalid())?;

        Ok(Self { condition })
    }

    fn encode<E: Extend<axum::http::HeaderValue>>(&self, _values: &mut E) {
        // We don't need to encode this header.
        unimplemented!()
    }
}

/// `PUT /tracker/{tracker_id}/game/{game_id}`: Update game.
pub async fn update_game<D>(
    State(state): State<Arc<AppState<D>>>,
    ClientIp(ip): ClientIp,
    user: Option<AuthenticatedUser>,
    Path((tracker_id, game_id)): Path<(UrlEncodedTrackerId, i32)>,
    TypedHeader(expected_owner): TypedHeader<IfOwnerIs>,
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

    let game = tx
        .get_ap_game(game_id)
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

    if game.tracker_id != tracker.id {
        return Err(StatusCode::NOT_FOUND);
    }

    // Test the owner precondition if it's present.
    let has_owner_precondition = match expected_owner.condition {
        Some(expected) if !expected.matches(&game) => return Err(StatusCode::PRECONDITION_FAILED),

        Some(_) => true,
        None => false,
    };

    // If the claim is changing hands, an owner precondition is required to
    // prevent races where someone else may accidentally clobber an earlier
    // claim because they are viewing old state.
    if (game_update.claimed_by_ct_user_id != game.claimed_by_ct_user_id
        || game_update.discord_username != game.discord_username)
        && !has_owner_precondition
    {
        return Err(StatusCode::PRECONDITION_REQUIRED);
    }

    // If the claimed user ID is changing to a value other than None, it must
    // match the authenticated user's ID.
    if game_update.claimed_by_ct_user_id != game.claimed_by_ct_user_id
        && game_update
            .claimed_by_ct_user_id
            .is_some_and(|id| user.as_ref().is_none_or(|u| u.user.id != id))
    {
        return Err(match user {
            // A user is trying to claim on behalf of another user.
            Some(_) => StatusCode::FORBIDDEN,
            // A user is trying to claim while unauthenticated; their token
            // probably expired.
            None => StatusCode::UNAUTHORIZED,
        });
    }

    let old_game = game;
    let mut game = old_game.clone();

    // Update the username.
    game.discord_username = match game_update.claimed_by_ct_user_id {
        // If claimed by an authenticated user, this username is not needed and
        // can be set to NULL.
        Some(_) => None,
        // Otherwise, take the unauthenticated username from the update.
        None => {
            // But don't allow a new unauthenticated claim if the tracker
            // disallows it.
            if game_update.discord_username != game.discord_username
                && tracker.require_authentication_to_claim
                && game_update.discord_username.is_some()
            {
                return Err(StatusCode::FORBIDDEN);
            }

            game_update.discord_username
        }
    };

    game.claimed_by_ct_user_id = game_update.claimed_by_ct_user_id;
    game.discord_ping = game_update.discord_ping;
    game.availability_status = game_update.availability_status;
    game.completion_status = game_update.completion_status;
    game.progression_status = game_update.progression_status;
    game.last_checked = game_update.last_checked;
    game.notes = game_update.notes;

    game.update_completion_status();

    let audit = create_audit_for(Some(ip), user.as_ref(), Utc::now(), &old_game, &game);

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

    send_stream(tx.create_audits(audit))
        .try_for_each(|_| ready(Ok(())))
        .await
        .unexpected()?;

    send_future(tx.commit()).await.unexpected()?;

    Ok(Json(game))
}

#[derive(Deserialize, Serialize)]
pub struct DashboardOverrideStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub visibility: Option<bool>,
}

/// `GET /tracker/{tracker_id}/dashboard_override`: Get dashboard override
/// status.
pub async fn get_tracker_dashboard_override<D>(
    State(state): State<Arc<AppState<D>>>,
    Path(tracker_id): Path<UrlEncodedTrackerId>,
    user: AuthenticatedUser,
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

    let r = tx
        .get_ap_tracker_dashboard_override(user.user.id, tracker.id)
        .await
        .unexpected()?;

    send_future(tx.commit()).await.unexpected()?;

    Ok(Json(DashboardOverrideStatus {
        visibility: r.map(|o| o.visibility),
    }))
}

/// `PUT /tracker/{tracker_id}/dashboard_override`: Set dashboard override
/// status.
pub async fn put_tracker_dashboard_override<D>(
    State(state): State<Arc<AppState<D>>>,
    Path(tracker_id): Path<UrlEncodedTrackerId>,
    user: AuthenticatedUser,
    Json(status): Json<DashboardOverrideStatus>,
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

    match status.visibility {
        Some(v) => {
            tx.upsert_ap_tracker_dashboard_override(ApTrackerDashboardOverride {
                ct_user_id: user.user.id,
                ap_tracker_id: tracker.id,
                visibility: v,
            })
            .await
            .unexpected()?;
        }

        None => {
            tx.delete_ap_tracker_dashboard_override(user.user.id, tracker.id)
                .await
                .unexpected()?;
        }
    };

    send_future(tx.commit()).await.unexpected()?;

    Ok(Json(status))
}
