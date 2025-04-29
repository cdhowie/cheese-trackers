//! Server state management.

use std::{
    collections::{HashMap, HashSet},
    future::ready,
    str::FromStr,
    sync::{Arc, LazyLock},
};

use arrayvec::ArrayVec;
use axum::http::HeaderValue;
use chrono::{DateTime, TimeDelta, Utc};
use futures::TryStreamExt;
use jsonwebtoken::Header;
use url::Url;
use uuid::Uuid;

use crate::{
    api::{UiSettings, tracker::UrlEncodedTrackerId},
    auth::{discord::AuthClient, token::TokenProcessor},
    conf::Config,
    db::{
        DataAccess, DataAccessProvider, Transactable, Transaction, create_audit_for,
        model::{
            ApGameIden, ApGameInsertion, ApHintIden, ApHintInsertion, ApTrackerIden,
            ApTrackerInsertion, AvailabilityStatus, CompletionStatus, HintClassification,
            PingPreference, ProgressionStatus, UpdateCompletionStatus,
        },
    },
    logging::log,
    send_hack::{send_future, send_stream},
    stream::try_into_grouping_map_by,
    tracker::{Checks, Game, Hint, ParseTrackerError, parse_tracker_html},
};

#[derive(Debug, thiserror::Error)]
pub enum TrackerUrlParseError {
    #[error("failed to parse URL: {0}")]
    Url(
        #[from]
        #[source]
        url::ParseError,
    ),
    #[error("invalid tracker ID in tracker URL")]
    TrackerId,
}

/// Errors that may occur when fetching the state of an upstream tracker.
#[derive(Debug, thiserror::Error)]
pub enum TrackerUpdateError {
    /// The tracker URL could not be parsed.
    #[error("failed to parse URL: {0}")]
    ParseUrl(
        #[from]
        #[source]
        TrackerUrlParseError,
    ),
    /// The provided upstream URL is not whitelisted in the service
    /// configuration.
    #[error("the upstream URL is not on the upstream whitelist")]
    UpstreamNotWhitelisted,
    /// The HTTP request for the upstream tracker data failed.
    #[error("failed to download tracker data: {0}")]
    Http(
        #[from]
        #[source]
        reqwest::Error,
    ),
    /// The data returned by the upstream tracker could not be parsed.
    #[error("failed to parse tracker response: {0}")]
    Parse(
        #[from]
        #[source]
        ParseTrackerError,
    ),
    /// An unexpected database error occured while synchronizing the state of
    /// the database with the state of the upstream trocker.
    #[error("database error: {0}")]
    Database(
        #[from]
        #[source]
        sqlx::Error,
    ),
    /// The number of slots changed since the last tracker update, which should
    /// not be possible.
    #[error("game count mismatch (tracker has {tracker}, database has {database})")]
    GameCountMismatch { tracker: usize, database: usize },
    /// Immutable information about a specific slot's game since the last
    /// tracker update, which should not be possible.
    #[error("game {0} has mismatching information")]
    GameInformationMismatch(u32),
    /// A numeric type conversion failed between the data type used by the
    /// upstream tracker and the type used by the database.
    #[error("numeric conversion failure processing game {0}")]
    NumericConversion(u32),
    /// A hint exists referencing a slot that does not exist.
    #[error("a hint exists referencing the nonexistent game name {0:?}")]
    HintGameMissing(String),
    /// The upstream tracker does not exist.
    #[error("tracker not found")]
    TrackerNotFound,
}

static GIT_COMMIT_ID: LazyLock<String> = LazyLock::new(|| {
    std::env::var("CT_GIT_COMMIT")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dev".to_owned())
});

/// Global server state.
pub struct AppState<D> {
    /// The server's [data access provider](crate::db::DataAccessProvider).
    pub data_provider: D,
    /// Cached JSON-serialized [UI settings](crate::api::UiSettings) response
    /// header value.
    pub ui_settings_header: HeaderValue,

    /// Discord authentication client.
    pub auth_client: AuthClient,
    /// Authentication token processor.
    pub token_processor: TokenProcessor,

    /// Set of valid upstream tracker prefixes.
    upstream_tracker_prefixes: HashSet<String>,

    /// Client used for upstream tracker updates.
    reqwest_client: reqwest::Client,
    /// Currently-inflight tracker update requests, keyed by the upstream
    /// tracker ID.
    ///
    /// This is used to merge simultaneous update requests for the same tracker
    /// into a single request to the upstream tracker server.
    inflight_tracker_updates: moka::future::Cache<String, Uuid>,
    /// The minimum allowed time between consecutive updates of a single tracker
    /// from the upstream tracker source.
    tracker_update_interval: chrono::Duration,
}

impl<D> AppState<D> {
    /// Create the global state from the given service configuration value and
    /// data access provider.
    pub fn new(config: Config, data_provider: D) -> Self {
        Self {
            reqwest_client: reqwest::Client::builder().build().unwrap(),
            data_provider,
            upstream_tracker_prefixes: config.upstream_trackers,
            ui_settings_header: serde_json::to_string(&UiSettings {
                banners: config.banners,
                hoster: config.hoster,
                build_version: &GIT_COMMIT_ID,
            })
            .unwrap()
            .parse()
            .unwrap(),
            inflight_tracker_updates: moka::future::Cache::builder()
                .time_to_live(config.tracker_update_interval.to_std().unwrap())
                .build(),
            tracker_update_interval: config.tracker_update_interval,
            auth_client: AuthClient::new(
                config.discord.client_id,
                config.discord.client_secret,
                &config.public_url,
                config.discord.token_cipher,
            ),
            token_processor: TokenProcessor::new(
                Header::new(config.token.algorithm),
                &config.token.secret,
                config.token.issuer,
                config.token.validity_duration,
            ),
        }
    }

    fn tracker_is_permitted(&self, url: impl Into<Url>) -> bool {
        let mut url = url.into();
        match url.path_segments_mut() {
            Ok(mut s) => s.pop(),
            Err(_) => return false,
        };

        self.upstream_tracker_prefixes.contains(url.as_str())
    }

    /// Synchronize a tracker in the database with fetched state from
    /// Archipelago.
    ///
    /// Returns the [`tracker_id`](ApTracker::tracker_id) of the tracker in the
    /// database.
    async fn synchronize_tracker(
        db: &mut (impl DataAccess + Send),
        now: DateTime<Utc>,
        upstream_url: &str,
        games: Vec<Game>,
        hints: Vec<Hint>,
    ) -> Result<Uuid, TrackerUpdateError> {
        // This function is quite complicated, but basically it boils down to
        // two parts:
        //
        // * If this is the first time we've seen this tracker ID, put the AP
        //   tracker data into the DB.
        // * If not, make sure the data is consistent and then update any
        //   changed pieces of data.

        match db.get_tracker_by_upstream_url(upstream_url).await? {
            None => {
                let tracker_id = Uuid::new_v4();

                let tracker = {
                    let trackers = send_stream(db.create_ap_trackers([ApTrackerInsertion {
                        tracker_id,
                        upstream_url: upstream_url.to_owned(),
                        updated_at: now,
                        title: "".to_owned(),
                        description: "".to_owned(),
                        owner_ct_user_id: None,
                        lock_settings: false,
                        global_ping_policy: None,
                        room_link: "".to_owned(),
                        last_port: None,
                        next_port_check_at: None,
                        inactivity_threshold_yellow_hours: 24,
                        inactivity_threshold_red_hours: 48,
                        require_authentication_to_claim: false,
                    }]));

                    tokio::pin!(trackers);

                    trackers
                        .try_next()
                        .await?
                        .ok_or(TrackerUpdateError::Database(sqlx::Error::RowNotFound))?
                };

                // Hints only contain the game's name so we need a way to map
                // those to the database IDs.
                let mut name_to_id = HashMap::new();

                for game in games {
                    let checks = game
                        .checks
                        .try_convert()
                        .map_err(|_| TrackerUpdateError::NumericConversion(game.position))?;

                    let mut game = ApGameInsertion {
                        tracker_id: tracker.id,
                        position: game
                            .position
                            .try_into()
                            .map_err(|_| TrackerUpdateError::NumericConversion(game.position))?,
                        name: game.name,
                        game: game.game,
                        tracker_status: game.status,
                        checks_done: checks.completed,
                        checks_total: checks.total,
                        last_activity: game.last_activity.map(|d| now - d),
                        discord_username: None,
                        discord_ping: PingPreference::Never,
                        availability_status: AvailabilityStatus::Unknown,
                        completion_status: CompletionStatus::Incomplete,
                        progression_status: ProgressionStatus::Unknown,
                        last_checked: None,
                        notes: String::new(),
                        claimed_by_ct_user_id: None,
                        effective_discord_username: None,
                        user_is_away: false,
                    };

                    game.update_completion_status();

                    let game = {
                        let games = send_stream(db.create_ap_games([game]));

                        tokio::pin!(games);

                        games
                            .try_next()
                            .await?
                            .ok_or(TrackerUpdateError::Database(sqlx::Error::RowNotFound))?
                    };

                    name_to_id.insert(game.name, game.id);
                }

                // Creating too many hints at once can run into database limits
                // on the number of parameters.  Unfortunately, create hints one
                // at a time to get around this.
                for hint in hints {
                    let receiver_game_id = name_to_id.get(&hint.receiver).copied();

                    let item_link_name = match receiver_game_id {
                        Some(_) => String::new(),
                        None => hint.receiver,
                    };

                    let ap_hint = ApHintInsertion {
                        finder_game_id: *name_to_id
                            .get(&hint.finder)
                            .ok_or_else(|| TrackerUpdateError::HintGameMissing(hint.finder))?,
                        // If the receiving game can't be found, it's most
                        // likely an item link check, which means the receiver
                        // would be multiple games.  We record this as null in
                        // the database.
                        receiver_game_id,
                        item_link_name,
                        item: hint.item,
                        location: hint.location,
                        entrance: hint.entrance,
                        found: hint.found,
                        classification: HintClassification::Unset,
                    };

                    send_stream(db.create_ap_hints([ap_hint]))
                        .try_for_each(|_| std::future::ready(Ok(())))
                        .await?;
                }

                Ok(tracker_id)
            }

            Some(mut tracker) => {
                let old_tracker = tracker.clone();

                let mut db_games: Vec<_> = db
                    .get_ap_games_by_tracker_id(tracker.id)
                    .try_collect()
                    .await?;

                if db_games.len() != games.len() {
                    return Err(TrackerUpdateError::GameCountMismatch {
                        tracker: games.len(),
                        database: db_games.len(),
                    });
                }

                db_games.sort_by_key(|g| g.position);

                let mut name_to_id = HashMap::new();

                for (tracker_game, old_db_game) in games.into_iter().zip(db_games.into_iter()) {
                    let tracker_position: i32 = tracker_game.position.try_into().map_err(|_| {
                        TrackerUpdateError::NumericConversion(tracker_game.position)
                    })?;

                    let tracker_checks: Checks<i32> =
                        tracker_game.checks.try_convert().map_err(|_| {
                            TrackerUpdateError::NumericConversion(tracker_game.position)
                        })?;

                    let mut db_game = old_db_game.clone();

                    // Sanity check that all of the existing information is the
                    // same.  If it's not, something bad probably happened.
                    if tracker_position != db_game.position
                        || tracker_game.game != db_game.game
                        || tracker_checks.total != db_game.checks_total
                    {
                        return Err(TrackerUpdateError::GameInformationMismatch(
                            tracker_game.position,
                        ));
                    }

                    name_to_id.insert(tracker_game.name.clone(), db_game.id);

                    db_game.name = tracker_game.name;
                    db_game.tracker_status = tracker_game.status;
                    db_game.checks_done = tracker_checks.completed;

                    let mut columns: ArrayVec<_, 5> = [
                        ApGameIden::Name,
                        ApGameIden::TrackerStatus,
                        ApGameIden::ChecksDone,
                    ]
                    .into_iter()
                    .collect();

                    // "Last activity" is parsed as a negative duration in
                    // seconds from the last time the AP web tracker information
                    // was updated, and we do not have access to that "epoch."
                    // This means that the time we generate here can vary.  To
                    // prevent spurious updates, we only update it if the time
                    // differs by a minute or more.
                    let new_last_activity = tracker_game.last_activity.map(|d| now - d);

                    if !matches!(
                        (db_game.last_activity, new_last_activity),
                        (Some(a), Some(b)) if (a - b).abs() < chrono::Duration::minutes(1)
                    ) {
                        db_game.last_activity = new_last_activity;
                        columns.push(ApGameIden::LastActivity);
                    }

                    if db_game.update_completion_status() {
                        columns.push(ApGameIden::CompletionStatus);
                    }

                    let audit = create_audit_for(None, None, now, &old_db_game, &db_game);

                    db.update_ap_game(db_game, &columns).await?;

                    send_stream(db.create_audits(audit))
                        .try_for_each(|_| ready(Ok(())))
                        .await?;
                }

                // Reconcile hints.  We need to match up the hints from the
                // tracker with hints in the database, updating hints that have
                // changed their found status, and inserting new hints.

                let mut existing_hints =
                    try_into_grouping_map_by(db.get_ap_hints_by_tracker_id(tracker.id), |hint| {
                        (
                            hint.finder_game_id,
                            hint.receiver_game_id,
                            hint.item_link_name.clone(),
                            hint.item.clone(),
                            hint.location.clone(),
                            hint.entrance.clone(),
                        )
                    })
                    .await?;

                // Reverse each Vec so we can pop() to take the "first" element.
                for v in existing_hints.values_mut() {
                    v.reverse();
                }

                let mut new_hints = vec![];

                for tracker_hint in hints {
                    let finder = name_to_id
                        .get(&tracker_hint.finder)
                        .copied()
                        .ok_or_else(|| TrackerUpdateError::HintGameMissing(tracker_hint.finder))?;

                    let receiver = name_to_id.get(&tracker_hint.receiver).copied();

                    let item_link_name = match receiver {
                        Some(_) => String::new(),
                        None => tracker_hint.receiver,
                    };

                    match existing_hints
                        .get_mut(&(
                            finder,
                            receiver,
                            item_link_name.clone(),
                            tracker_hint.item.clone(),
                            tracker_hint.location.clone(),
                            tracker_hint.entrance.clone(),
                        ))
                        .and_then(|v| v.pop())
                    {
                        Some(mut h) => {
                            // Hint exists.  Update if the found state changed.
                            if h.found != tracker_hint.found {
                                let old_hint = h.clone();
                                h.found = tracker_hint.found;

                                let audit = create_audit_for(None, None, now, &old_hint, &h);

                                db.update_ap_hint(h, &[ApHintIden::Found]).await?;

                                send_stream(db.create_audits(audit))
                                    .try_for_each(|_| ready(Ok(())))
                                    .await?;
                            }
                        }
                        None => {
                            // This is a new hint.
                            new_hints.push(ApHintInsertion {
                                finder_game_id: finder,
                                receiver_game_id: receiver,
                                item_link_name,
                                item: tracker_hint.item,
                                location: tracker_hint.location,
                                entrance: tracker_hint.entrance,
                                found: tracker_hint.found,
                                classification: HintClassification::Unset,
                            });
                        }
                    }
                }

                // Like when creating, we have to create these separately in
                // case there are too many for one statement.
                for hint in new_hints {
                    send_stream(db.create_ap_hints([hint]))
                        .try_for_each(|_| std::future::ready(Ok(())))
                        .await?;
                }

                // Any remaining existing hints don't exist anymore.  This
                // should never happen, but...
                for hint in existing_hints.into_values().flatten() {
                    db.delete_ap_hint_by_id(hint.id).await?;
                }

                let tracker_id = tracker.tracker_id;

                tracker.updated_at = now;

                let audit = create_audit_for(None, None, now, &old_tracker, &tracker);

                db.update_ap_tracker(tracker, &[ApTrackerIden::UpdatedAt])
                    .await?;

                send_stream(db.create_audits(audit))
                    .try_for_each(|_| ready(Ok(())))
                    .await?;

                Ok(tracker_id)
            }
        }
    }

    /// Update the data for the provided upstream tracker URL and return the
    /// local ID of the tracker, creating the tracker if it does not already
    /// exist.
    ///
    /// If the last update was within the [tracker update
    /// interval](Self::tracker_update_interval) then the update is not
    /// performed and the operation succeeds immediately.
    ///
    /// If there is an existing inflight request for an update of the same
    /// tracker, that request will be awaited instead of creating a new request.
    /// This ensures that two simultaneous requests to update the same tracker
    /// will not result in multiple requests to the upstream tracker server.
    pub async fn upsert_tracker(&self, url: &str) -> Result<Uuid, Arc<TrackerUpdateError>>
    where
        D: DataAccessProvider + Send + Sync + 'static,
    {
        let url: Url = url
            .parse()
            .map_err(|e| Arc::new(TrackerUrlParseError::Url(e).into()))?;

        if !self.tracker_is_permitted(url.clone()) {
            return Err(Arc::new(TrackerUpdateError::UpstreamNotWhitelisted));
        }

        // The AP tracker endpoint accepts tracker IDs that have a suffix
        // consisting of invalid url-safe-base64 characters.  This would allow
        // creating multiple CT trackers for the same AP tracker
        // unintentionally, so we validate the ID in the URL to make sure it's
        // entirely valid.
        if url
            .path_segments()
            .and_then(|mut s| s.next_back())
            .is_none_or(|id| UrlEncodedTrackerId::from_str(id).is_err())
        {
            return Err(Arc::new(TrackerUrlParseError::TrackerId.into()));
        }

        let fut = async {
            let now = Utc::now();

            let mut db = self.data_provider.create_data_access().await?;
            let mut tx = db.begin().await?;

            let tracker = tx.get_tracker_by_upstream_url(url.as_str()).await?;

            match tracker {
                Some(t) if now < t.updated_at + self.tracker_update_interval => {
                    // The tracker was updated within the last
                    // tracker_update_interval, so don't update it now.
                    send_future(tx.rollback()).await?;
                    return Ok(t.tracker_id);
                }
                _ => {}
            };

            log!("Requesting AP tracker {url}");

            let sync_tracker_fut = async {
                let html = self
                    .reqwest_client
                    .get(url.clone())
                    .send()
                    .await?
                    .error_for_status()
                    .map_err(|e| match e.status() {
                        Some(reqwest::StatusCode::NOT_FOUND) => TrackerUpdateError::TrackerNotFound,
                        _ => TrackerUpdateError::Http(e),
                    })?
                    .text()
                    .await?;

                let (games, hints) = parse_tracker_html(&html)?;

                Self::synchronize_tracker(&mut tx, now, url.as_str(), games, hints).await
            };

            let last_port_fut = async {
                let tracker = match tracker {
                    None => return Ok(None),
                    Some(t) if t.room_link.is_empty() => return Ok(None),
                    Some(t) => t,
                };

                if tracker.next_port_check_at.is_some_and(|d| d > Utc::now()) {
                    return Ok(None);
                }

                self.get_last_port(&tracker.room_link, &tracker.upstream_url)
                    .await
                    .map(|r| Some((r, tracker)))
            };

            // If the last port check fails we can still accept the results of
            // the data sync.  However, if the data sync fails then we cannot
            // trust the state of the transaction and must roll it back.
            //
            // Therefore, we use try_join to bail early if the tracker sync
            // fails, but this means we need to wrap errors fetching the room
            // port number in success so that a failure there doesn't abort the
            // tracker sync, which may yet succeed.
            let last_port_fut = async { Ok::<_, TrackerUpdateError>(last_port_fut.await) };

            let (tracker_id, last_port) = tokio::try_join!(sync_tracker_fut, last_port_fut)?;

            match last_port {
                // No update at this time.  No room link, not due for update,
                // etc.
                Ok(None) => {}

                Err(e) => {
                    eprintln!(
                        "During tracker refresh request, failed to fetch room info for tracker {url:?}: {e}"
                    );
                }

                Ok(Some(((port, next_check), mut tracker))) => {
                    tracker.last_port = Some(port.into());
                    tracker.next_port_check_at = Some(next_check);

                    tx.update_ap_tracker(
                        tracker,
                        &[ApTrackerIden::LastPort, ApTrackerIden::NextPortCheckAt],
                    )
                    .await?;

                    // No audit for this change since the port fields are not
                    // diffed.
                }
            };

            send_future(tx.commit()).await?;

            Ok(tracker_id)
        };

        self.inflight_tracker_updates
            .try_get_with_by_ref(url.as_str(), fut)
            .await
    }

    /// Gets the last port the room had (which may be its current port).
    pub async fn get_last_port(
        &self,
        room_link: &str,
        tracker_link: &str,
    ) -> Result<(u16, DateTime<Utc>), GetRoomLinkError> {
        let room_url: Url = room_link.parse()?;
        let mut tracker_url: Url = tracker_link.parse()?;

        let room_id = extract_room_id_from_room_link(&room_url, &tracker_url)
            .ok_or(GetRoomLinkError::InvalidRoomLink)?;

        println!(
            "{} - Requesting port from room {room_url} for tracker {tracker_url}",
            Utc::now()
        );

        // Set the tracker URL's path to the API base and clear out other stuff
        // we don't want.  It doesn't matter whether we use the tracker or the
        // room URL at this point (we verified they have the same protocol,
        // host, and port) -- however, we are borrowing room_id from the room
        // URL, so we can't mutate that.
        tracker_url.set_path("/api/");
        tracker_url.set_query(None);
        tracker_url.set_fragment(None);

        let client =
            crate::ap_api::Client::new_with_client(tracker_url, self.reqwest_client.clone());

        let r = client.get_room_status(room_id).await?;

        // Set the next time to check either when the room times out, or 5
        // minutes from now, whichever is later.
        let next_check = r
            .last_activity
            .checked_add_signed(TimeDelta::seconds(r.timeout_sec.into()))
            .ok_or(GetRoomLinkError::DateTimeOutOfRange)?
            .max(
                Utc::now()
                    .checked_add_signed(TimeDelta::minutes(5))
                    .ok_or(GetRoomLinkError::DateTimeOutOfRange)?,
            );

        Ok((r.last_port, next_check))
    }
}

/// Extracts the room ID from a room link.
///
/// This function also verifies that the room link is valid and belongs to the
/// same domain as the tracker link.
fn extract_room_id_from_room_link<'a>(room_link: &'a Url, tracker_link: &Url) -> Option<&'a str> {
    // Make sure the room URL has the same host as the tracker URL.
    if room_link.cannot_be_a_base()
        || tracker_link.cannot_be_a_base()
        || room_link.scheme() != tracker_link.scheme()
        || room_link.host_str() != tracker_link.host_str()
        || room_link.port_or_known_default() != tracker_link.port_or_known_default()
    {
        return None;
    }

    // Get the room ID and verify that it's a single path component.
    room_link
        .path()
        .strip_prefix("/room/")
        .filter(|id| !id.contains('/'))
}

#[derive(Debug, thiserror::Error)]
pub enum GetRoomLinkError {
    #[error("unable to parse URL: {0}")]
    UrlParse(
        #[from]
        #[source]
        url::ParseError,
    ),
    #[error("the room link is invalid")]
    InvalidRoomLink,
    #[error("failed to fetch room information: {0}")]
    ApiRequest(
        #[from]
        #[source]
        reqwest::Error,
    ),
    #[error("a DateTime was out of range")]
    DateTimeOutOfRange,
}
