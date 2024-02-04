use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use db::{
    model::{ApGame, ApGameIden, ApHint, ApTracker, ApTrackerIden, GameStatus},
    DataAccessProvider,
};
use futures::{
    future::{BoxFuture, Shared},
    FutureExt, TryFutureExt, TryStreamExt,
};
use sqlx::PgPool;
use tokio::{net::TcpListener, signal::unix::SignalKind, sync::RwLock};
use url::Url;

use crate::logging::UnexpectedResultExt;

mod conf;
mod db;
mod logging;
mod signal;
mod tracker;

#[derive(Debug, thiserror::Error)]
enum TrackerUpdateError {
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
        tracker::ParseTrackerError,
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

type InflightTrackerUpdateFuture = BoxFuture<'static, Result<(), Arc<TrackerUpdateError>>>;

struct AppState {
    reqwest_client: reqwest::Client,
    data_provider: Box<dyn DataAccessProvider + Send + Sync>,
    tracker_base_url: Url,

    inflight_tracker_updates: RwLock<HashMap<String, Shared<InflightTrackerUpdateFuture>>>,
    tracker_update_interval: chrono::Duration,
}

impl AppState {
    pub async fn new(config: &conf::Config) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            reqwest_client: reqwest::Client::builder().build().unwrap(),
            data_provider: match &config.database {
                conf::Database::Postgres { connection_string } => {
                    Box::new(PgPool::connect(connection_string).await?)
                }
            },
            tracker_base_url: "https://archipelago.gg/tracker/".parse().unwrap(),
            inflight_tracker_updates: RwLock::default(),
            tracker_update_interval: config.tracker_update_interval,
        })
    }
}

impl AppState {
    async fn synchronize_tracker(
        db: &mut (dyn db::DataAccessTransaction<'_> + Send),
        tracker_id: String,
        games: Vec<tracker::Game>,
        hints: Vec<tracker::Hint>,
    ) -> Result<(), TrackerUpdateError> {
        let now = Utc::now();

        let name_to_id = match db.get_tracker_by_ap_tracker_id(&tracker_id).await? {
            None => {
                let tracker = db
                    .create_ap_tracker(db::model::ApTracker {
                        id: 0,
                        tracker_id,
                        updated_at: now.naive_utc(),
                    })
                    .await?;

                // Hints only contain the game's name so we need a way to map
                // those to the database IDs.
                let mut name_to_id = HashMap::new();

                for game in games {
                    let checks = game
                        .checks
                        .try_convert()
                        .map_err(|_| TrackerUpdateError::NumericConversion(game.position))?;

                    let game = db
                        .create_ap_game(ApGame {
                            id: 0,
                            tracker_id: tracker.id,
                            position: game.position.try_into().map_err(|_| {
                                TrackerUpdateError::NumericConversion(game.position)
                            })?,
                            name: game.name,
                            game: game.game,
                            checks_done: checks.completed,
                            checks_total: checks.total,
                            last_activity: game.last_activity.map(|d| (now - d).naive_utc()),
                            discord_username: None,
                            discord_ping: false,
                            status: GameStatus::Unblocked,
                            last_checked: None,
                        })
                        .await?;

                    name_to_id.insert(game.name, game.id);
                }

                name_to_id
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

                    let tracker_checks: tracker::Checks<i32> =
                        tracker_game.checks.try_convert().map_err(|_| {
                            TrackerUpdateError::NumericConversion(tracker_game.position)
                        })?;

                    // Sanity check that all of the existing information is the
                    // same.  If it's not, something bad probably happened.
                    if tracker_position != db_game.position
                        || tracker_game.name != db_game.name
                        || tracker_game.game != db_game.game
                        || tracker_checks.total != db_game.checks_total
                    {
                        return Err(TrackerUpdateError::GameInformationMismatch(
                            tracker_game.position,
                        ));
                    }

                    name_to_id.insert(tracker_game.name, db_game.id);

                    db_game.checks_done = tracker_checks.completed;
                    db_game.last_activity =
                        tracker_game.last_activity.map(|d| (now - d).naive_utc());

                    db.update_ap_game(db_game, &[ApGameIden::ChecksDone, ApGameIden::LastActivity])
                        .await?;
                }

                // Synchronizing hints is a bit tricky because the data we
                // receive has no opaque identifier, and data can be duplicated
                // since there can be multiple items with the exact same finder,
                // receiver, item, and location.
                //
                // Eventually we could possibly make this more optimized, but
                // for now just replace all of the hints in the database.
                db.delete_ap_hints_by_tracker_id(tracker.id).await?;

                tracker.updated_at = now.naive_utc();
                db.update_ap_tracker(tracker, &[ApTrackerIden::UpdatedAt])
                    .await?;

                name_to_id
            }
        };

        for hint in hints {
            db.create_ap_hint(ApHint {
                id: 0,
                finder_game_id: *name_to_id
                    .get(&hint.finder)
                    .ok_or_else(|| TrackerUpdateError::HintGameMissing(hint.finder))?,
                receiver_game_id: *name_to_id
                    .get(&hint.receiver)
                    .ok_or_else(|| TrackerUpdateError::HintGameMissing(hint.receiver))?,
                item: hint.item,
                location: hint.location,
                entrance: hint.entrance,
                found: hint.found,
            })
            .await?;
        }

        Ok(())
    }

    async fn update_tracker(
        self: &Arc<Self>,
        tracker_id: String,
    ) -> Result<(), Arc<TrackerUpdateError>> {
        // This is broken out into a separate statement to ensure the lock guard
        // is dropped as soon as possible.
        let existing = self
            .inflight_tracker_updates
            .read()
            .await
            .get(&tracker_id)
            .cloned();

        if let Some(f) = existing {
            return f.await;
        }

        let mut guard = self.inflight_tracker_updates.write().await;

        // Check again because a concurrent thread could've inserted the task
        // before we acquired the write lock.
        if let Some(f) = guard.get(&tracker_id).cloned() {
            drop(guard);
            return f.await;
        }

        let fut = {
            let this = Arc::clone(self);
            let tracker_id = tracker_id.clone();

            async move {
                let mut db = this.data_provider.create_data_access().await?;
                let mut tx = db.begin().await?;

                if tx
                    .get_tracker_by_ap_tracker_id(&tracker_id)
                    .await?
                    .is_some_and(|r| {
                        Utc::now().naive_utc() < r.updated_at + this.tracker_update_interval
                    })
                {
                    // The tracker was updated within the last
                    // tracker_update_interval, so don't update it now.
                    tx.rollback().await?;
                    return Ok(());
                }

                let url = this.tracker_base_url.join(&tracker_id)?;
                let html = this
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

                let (games, hints) = tracker::parse_tracker_html(&html)?;

                Self::synchronize_tracker(&mut *tx, tracker_id, games, hints).await?;

                tx.commit().await?;

                Ok(())
            }
            .map_err(Arc::new)
            .boxed()
            .shared()
        };

        guard.insert(tracker_id.clone(), fut.clone());
        drop(guard);

        // Spawn a task that will drive the future, and will remove it from
        // the inflight map upon completion.
        tokio::spawn({
            let this = Arc::clone(self);
            let fut = fut.clone();

            async move {
                // We don't care what the result is here, we just want to
                // clean up.
                fut.await.ok();

                this.inflight_tracker_updates
                    .write()
                    .await
                    .remove(&tracker_id);
            }
        });

        fut.await
    }
}

#[derive(Debug, serde::Serialize)]
struct TrackerResponse {
    #[serde(flatten)]
    pub tracker: ApTracker,
    pub games: Vec<ApGame>,
    pub hints: Vec<ApHint>,
}

async fn get_tracker_by_id(
    State(state): State<Arc<AppState>>,
    Path(tracker_id): Path<String>,
) -> Result<Json<TrackerResponse>, StatusCode> {
    {
        let r = state.update_tracker(tracker_id.clone()).await;
        if r.as_ref()
            .is_err_and(|e| matches!(**e, TrackerUpdateError::TrackerNotFound))
        {
            return Err(StatusCode::NOT_FOUND);
        }
        r.unexpected()?;
    }

    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    let mut tx = db.begin().await.unexpected()?;

    let tracker = tx
        .get_tracker_by_ap_tracker_id(&tracker_id)
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

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

    Ok(Json(TrackerResponse {
        tracker,
        games,
        hints,
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = conf::load()?;

    let router = axum::Router::new();

    let state = Arc::new(AppState::new(&config).await?);

    axum::serve(
        TcpListener::bind(config.http_listen).await?,
        router
            .route(
                "/tracker/:tracker_id",
                axum::routing::get(get_tracker_by_id),
            )
            .with_state(state)
            .into_make_service(),
    )
    .with_graceful_shutdown(async {
        match signal::any([SignalKind::interrupt(), SignalKind::terminate()]) {
            Ok(f) => f.await,
            Err(e) => {
                eprintln!("Unable to listen for shutdown signals: {e}");
                std::future::pending().await
            }
        }
    })
    .await?;

    Ok(())
}
