use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use db::{
    model::{ApGame, ApGameIden, ApHint, ApTracker, ApTrackerIden, GameStatus},
    DataAccess, DataAccessProvider, Transactable, Transaction,
};
use futures::{
    future::{BoxFuture, Shared},
    FutureExt, TryFutureExt, TryStreamExt,
};
use tokio::{net::TcpListener, signal::unix::SignalKind, sync::RwLock};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};
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

struct AppState<D> {
    reqwest_client: reqwest::Client,
    data_provider: D,
    tracker_base_url: Url,
    ui_settings: UiSettings,

    inflight_tracker_updates: RwLock<HashMap<String, Shared<InflightTrackerUpdateFuture>>>,
    tracker_update_interval: chrono::Duration,
}

impl<D> AppState<D> {
    fn new(config: conf::Config, data_provider: D) -> Self {
        Self {
            reqwest_client: reqwest::Client::builder().build().unwrap(),
            data_provider,
            tracker_base_url: "https://archipelago.gg/tracker/".parse().unwrap(),
            ui_settings: UiSettings {
                is_staging: config.is_staging,
            },
            inflight_tracker_updates: RwLock::default(),
            tracker_update_interval: config.tracker_update_interval,
        }
    }

    /// Synchronize a tracker in the database with fetched state from
    /// Archipelago.
    async fn synchronize_tracker(
        db: &mut (impl db::DataAccess + Send),
        tracker_id: String,
        games: Vec<tracker::Game>,
        hints: Vec<tracker::Hint>,
    ) -> Result<(), TrackerUpdateError> {
        // This function is quite complicated, but basically it boils down to
        // two parts:
        //
        // * If this is the first time we've seen this tracker ID, put the AP
        //   tracker data into the DB.
        // * If not, make sure the data is consistent and then update any
        //   changed pieces of data.

        let now = Utc::now();

        // Both arms return a mapping of game player names to their respective
        // database IDs.  We use this data to populate the hints table.
        let name_to_id = match db.get_tracker_by_ap_tracker_id(&tracker_id).await? {
            None => {
                let tracker = db
                    .create_ap_trackers([db::model::ApTracker {
                        id: 0,
                        tracker_id,
                        updated_at: now,
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

                    let game = db
                        .create_ap_games([ApGame {
                            id: 0,
                            tracker_id: tracker.id,
                            position: game.position.try_into().map_err(|_| {
                                TrackerUpdateError::NumericConversion(game.position)
                            })?,
                            name: game.name,
                            game: game.game,
                            tracker_status: game.status,
                            checks_done: checks.completed,
                            checks_total: checks.total,
                            last_activity: game.last_activity.map(|d| now - d),
                            discord_username: None,
                            discord_ping: false,
                            status: GameStatus::Unblocked,
                            last_checked: None,
                        }])
                        .try_next()
                        .await?
                        .ok_or(TrackerUpdateError::Database(sqlx::Error::RowNotFound))?;

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

                    db_game.tracker_status = tracker_game.status;
                    db_game.checks_done = tracker_checks.completed;
                    db_game.last_activity = tracker_game.last_activity.map(|d| now - d);

                    db.update_ap_game(
                        db_game,
                        &[
                            ApGameIden::TrackerStatus,
                            ApGameIden::ChecksDone,
                            ApGameIden::LastActivity,
                        ],
                    )
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

                tracker.updated_at = now;
                db.update_ap_tracker(tracker, &[ApTrackerIden::UpdatedAt])
                    .await?;

                name_to_id
            }
        };

        db.create_ap_hints(
            hints
                .into_iter()
                .map(|hint| {
                    Ok::<_, TrackerUpdateError>(ApHint {
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
                })
                .collect::<Result<Vec<_>, _>>()?,
        )
        // This drives the stream to completion.
        .try_for_each(|_| std::future::ready(Ok(())))
        .await?;

        Ok(())
    }

    async fn update_tracker(
        self: &Arc<Self>,
        tracker_id: String,
    ) -> Result<(), Arc<TrackerUpdateError>>
    where
        D: DataAccessProvider + Send + Sync + 'static,
    {
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
                    .is_some_and(|r| Utc::now() < r.updated_at + this.tracker_update_interval)
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

                Self::synchronize_tracker(&mut tx, tracker_id, games, hints).await?;

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

async fn get_tracker<D>(
    State(state): State<Arc<AppState<D>>>,
    Path(tracker_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    #[derive(Debug, serde::Serialize)]
    struct GetTrackerResponse {
        #[serde(flatten)]
        pub tracker: ApTracker,
        pub games: Vec<ApGame>,
        pub hints: Vec<ApHint>,
    }

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

    Ok(Json(GetTrackerResponse {
        tracker,
        games,
        hints,
    }))
}

#[derive(Debug, serde::Deserialize)]
struct UpdateGameRequest {
    pub discord_username: Option<String>,
    pub discord_ping: bool,
    pub status: GameStatus,
    pub last_checked: Option<DateTime<Utc>>,
}

async fn update_game<D>(
    State(state): State<Arc<AppState<D>>>,
    Path((tracker_id, game_id)): Path<(String, i32)>,
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
        .get_tracker_by_ap_tracker_id(&tracker_id)
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

    game.discord_username = game_update.discord_username;
    game.discord_ping = game_update.discord_ping;
    game.status = game_update.status;
    game.last_checked = game_update.last_checked;

    tx.update_ap_game(
        game,
        &[
            ApGameIden::DiscordUsername,
            ApGameIden::DiscordPing,
            ApGameIden::Status,
            ApGameIden::LastChecked,
        ],
    )
    .await
    .unexpected()?;

    tx.commit().await.unexpected()?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Clone, serde::Serialize)]
struct UiSettings {
    pub is_staging: bool,
}

async fn get_settings<D>(State(state): State<Arc<AppState<D>>>) -> Json<UiSettings> {
    Json(state.ui_settings.clone())
}

fn create_api_router<D>(state: AppState<D>) -> axum::Router<()>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    axum::Router::new()
        .route("/tracker/:tracker_id", axum::routing::get(get_tracker))
        .route(
            "/tracker/:tracker_id/game/:game_id",
            axum::routing::put(update_game),
        )
        .route("/settings", axum::routing::get(get_settings))
        .with_state(Arc::new(state))
}

async fn create_router_from_config(
    config: conf::Config,
) -> Result<axum::Router<()>, Box<dyn std::error::Error>> {
    Ok(match &config.database {
        conf::Database::Postgres { connection_string } => {
            let data_provider = sqlx::PgPool::connect(connection_string).await?;
            create_api_router(AppState::new(config, data_provider))
        }
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = conf::load()?;
    let listen = config.http_listen;
    let cors = config.cors_permissive;

    let api_router = create_router_from_config(config).await?;
    let api_router = if cors {
        api_router.layer(CorsLayer::permissive())
    } else {
        api_router
    };

    let router = axum::Router::new()
        .nest("/api", api_router)
        .fallback_service(ServeDir::new("dist").fallback(ServeFile::new("dist/index.html")));

    axum::serve(TcpListener::bind(listen).await?, router)
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
