//! Server state management.
//!
//! This module provides traits that can be used to obtain specific
//! functionality from the global server state.  This allows better separation
//! of concerns between different server components.

use std::{collections::HashMap, sync::Arc, time::Duration};

use arrayvec::ArrayVec;
use axum::http::HeaderValue;
use chrono::Utc;
use futures::TryStreamExt;
use jsonwebtoken::Header;
use url::Url;

use crate::{
    api::UiSettings,
    auth::{discord::AuthClient, token::TokenProcessor},
    conf::Config,
    db::{
        model::{
            ApGame, ApGameIden, ApHint, ApHintIden, ApTracker, ApTrackerIden, AvailabilityStatus,
            CompletionStatus, HintClassification, PingPreference, ProgressionStatus,
        },
        DataAccess, DataAccessProvider, Transactable, Transaction,
    },
    stream::try_into_grouping_map_by,
    tracker::{parse_tracker_html, Checks, Game, Hint, ParseTrackerError},
};

#[derive(Debug, thiserror::Error)]
pub enum TrackerUpdateError {
    #[error("failed to parse URL: {0}")]
    ParseUrl(
        #[from]
        #[source]
        url::ParseError,
    ),
    #[error("failed to download tracker data: {0}")]
    Http(
        #[from]
        #[source]
        reqwest::Error,
    ),
    #[error("failed to parse tracker response: {0}")]
    Parse(
        #[from]
        #[source]
        ParseTrackerError,
    ),
    #[error("database error: {0}")]
    Database(
        #[from]
        #[source]
        sqlx::Error,
    ),
    #[error("game count mismatch (tracker has {tracker}, database has {database})")]
    GameCountMismatch { tracker: usize, database: usize },
    #[error("game {0} has mismatching information")]
    GameInformationMismatch(u32),
    #[error("numeric conversion failure processing game {0}")]
    NumericConversion(u32),
    #[error("a hint exists referencing the nonexistent game name {0:?}")]
    HintGameMissing(String),
    #[error("tracker not found")]
    TrackerNotFound,
}

pub struct AppState<D> {
    pub data_provider: D,
    pub ui_settings_header: HeaderValue,

    pub auth_client: AuthClient,
    pub token_processor: TokenProcessor,

    reqwest_client: reqwest::Client,
    tracker_base_url: Url,
    inflight_tracker_updates: moka::future::Cache<String, ()>,
    tracker_update_interval: chrono::Duration,
}

impl<D> AppState<D> {
    pub fn new(config: Config, data_provider: D) -> Self {
        Self {
            reqwest_client: reqwest::Client::builder().build().unwrap(),
            data_provider,
            tracker_base_url: "https://archipelago.gg/tracker/".parse().unwrap(),
            ui_settings_header: serde_json::to_string(&UiSettings {
                banners: config.banners,
                build_version: option_env!("GIT_COMMIT")
                    .filter(|s| !s.is_empty())
                    .unwrap_or("dev"),
            })
            .unwrap()
            .parse()
            .unwrap(),
            inflight_tracker_updates: moka::future::Cache::builder()
                .time_to_live(Duration::from_secs(5))
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

    /// Synchronize a tracker in the database with fetched state from
    /// Archipelago.
    async fn synchronize_tracker(
        db: &mut (impl DataAccess + Send),
        tracker_id: String,
        games: Vec<Game>,
        hints: Vec<Hint>,
    ) -> Result<(), TrackerUpdateError> {
        // This function is quite complicated, but basically it boils down to
        // two parts:
        //
        // * If this is the first time we've seen this tracker ID, put the AP
        //   tracker data into the DB.
        // * If not, make sure the data is consistent and then update any
        //   changed pieces of data.

        let now = Utc::now();

        match db.get_tracker_by_ap_tracker_id(&tracker_id).await? {
            None => {
                let tracker = db
                    .create_ap_trackers([ApTracker {
                        id: 0,
                        tracker_id,
                        updated_at: now,
                        title: "".to_owned(),
                        owner_ct_user_id: None,
                        lock_title: false,
                    }])
                    .try_next()
                    .await?
                    .ok_or(TrackerUpdateError::Database(sqlx::Error::RowNotFound))?;

                // Hints only contain the game's name so we need a way to map
                // those to the database IDs.
                let mut name_to_id = HashMap::new();

                for game in games {
                    let checks = game
                        .checks
                        .try_convert()
                        .map_err(|_| TrackerUpdateError::NumericConversion(game.position))?;

                    let mut game = ApGame {
                        id: 0,
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
                    };

                    game.update_completion_status();

                    let game = db
                        .create_ap_games([game])
                        .try_next()
                        .await?
                        .ok_or(TrackerUpdateError::Database(sqlx::Error::RowNotFound))?;

                    name_to_id.insert(game.name, game.id);
                }

                db.create_ap_hints(
                    hints
                        .into_iter()
                        .map(|hint| {
                            Ok::<_, TrackerUpdateError>(ApHint {
                                id: 0,
                                finder_game_id: *name_to_id.get(&hint.finder).ok_or_else(|| {
                                    TrackerUpdateError::HintGameMissing(hint.finder)
                                })?,
                                // If the receiving game can't be found, it's most
                                // likely an item link check, which means the receiver
                                // would be multiple games.  We record this as null in
                                // the database.
                                receiver_game_id: name_to_id.get(&hint.receiver).copied(),
                                item: hint.item,
                                location: hint.location,
                                entrance: hint.entrance,
                                found: hint.found,
                                classification: HintClassification::Unknown,
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                )
                // This drives the stream to completion.
                .try_for_each(|_| std::future::ready(Ok(())))
                .await?;
            }

            Some(mut tracker) => {
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

                for (tracker_game, mut db_game) in games.into_iter().zip(db_games.into_iter()) {
                    let tracker_position: i32 = tracker_game.position.try_into().map_err(|_| {
                        TrackerUpdateError::NumericConversion(tracker_game.position)
                    })?;

                    let tracker_checks: Checks<i32> =
                        tracker_game.checks.try_convert().map_err(|_| {
                            TrackerUpdateError::NumericConversion(tracker_game.position)
                        })?;

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

                    db.update_ap_game(db_game, &columns).await?;
                }

                // Reconcile hints.  We need to match up the hints from the
                // tracker with hints in the database, updating hints that have
                // changed their found status, and inserting new hints.

                let mut existing_hints =
                    try_into_grouping_map_by(db.get_ap_hints_by_tracker_id(tracker.id), |hint| {
                        (
                            hint.finder_game_id,
                            hint.receiver_game_id,
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

                    match existing_hints
                        .get_mut(&(
                            finder,
                            receiver,
                            tracker_hint.item.clone(),
                            tracker_hint.location.clone(),
                            tracker_hint.entrance.clone(),
                        ))
                        .and_then(|v| v.pop())
                    {
                        Some(mut h) => {
                            // Hint exists.  Update if the found state changed.
                            if h.found != tracker_hint.found {
                                h.found = tracker_hint.found;
                                db.update_ap_hint(h, &[ApHintIden::Found]).await?;
                            }
                        }
                        None => {
                            // This is a new hint.
                            new_hints.push(ApHint {
                                id: 0,
                                finder_game_id: finder,
                                receiver_game_id: receiver,
                                item: tracker_hint.item,
                                location: tracker_hint.location,
                                entrance: tracker_hint.entrance,
                                found: tracker_hint.found,
                                classification: HintClassification::Unknown,
                            });
                        }
                    }
                }

                if !new_hints.is_empty() {
                    db.create_ap_hints(new_hints)
                        .try_for_each(|_| std::future::ready(Ok(())))
                        .await?;
                }

                // Any remaining existing hints don't exist anymore.  This
                // should never happen, but...
                for hint in existing_hints.into_values().flatten() {
                    db.delete_ap_hint_by_id(hint.id).await?;
                }

                tracker.updated_at = now;
                db.update_ap_tracker(tracker, &[ApTrackerIden::UpdatedAt])
                    .await?;
            }
        };

        Ok(())
    }

    pub async fn update_tracker(&self, tracker_id: &str) -> Result<(), Arc<TrackerUpdateError>>
    where
        D: DataAccessProvider + Send + Sync + 'static,
    {
        let fut = async {
            let mut db = self.data_provider.create_data_access().await?;
            let mut tx = db.begin().await?;

            if tx
                .get_tracker_by_ap_tracker_id(tracker_id)
                .await?
                .is_some_and(|r| Utc::now() < r.updated_at + self.tracker_update_interval)
            {
                // The tracker was updated within the last
                // tracker_update_interval, so don't update it now.
                tx.rollback().await?;
                return Ok(());
            }

            let url = self.tracker_base_url.join(tracker_id)?;
            let html = self
                .reqwest_client
                .get(url)
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

            Self::synchronize_tracker(&mut tx, tracker_id.to_owned(), games, hints).await?;

            tx.commit().await?;

            Ok(())
        };

        self.inflight_tracker_updates
            .try_get_with_by_ref(tracker_id, fut)
            .await
    }
}
