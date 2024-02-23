use std::{collections::HashMap, sync::Arc};

use arrayvec::ArrayVec;
use auth::token::AuthenticatedUser;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use db::{
    model::{
        ApGame, ApGameIden, ApHint, ApTracker, ApTrackerIden, CtUser, GameStatus, JsError,
        PingPreference, TrackerGameStatus,
    },
    DataAccess, DataAccessProvider, Transactable, Transaction,
};
use futures::{
    future::{BoxFuture, Shared},
    FutureExt, TryFutureExt, TryStreamExt,
};
use jsonwebtoken::Header;
use oauth2::TokenResponse;
use state::{GetDataAccessProvider, GetTokenProcessor};
use tokio::{net::TcpListener, signal::unix::SignalKind, sync::RwLock};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};
use url::Url;

use crate::logging::UnexpectedResultExt;

mod auth;
mod conf;
mod db;
mod logging;
mod signal;
mod state;
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

    auth_client: auth::discord::AuthClient,
    token_processor: auth::token::TokenProcessor,
}

impl<D> GetTokenProcessor for AppState<D> {
    fn get_token_processor(&self) -> &auth::token::TokenProcessor {
        &self.token_processor
    }
}

impl<D: DataAccessProvider> GetDataAccessProvider for AppState<D> {
    type DataProvider = D;

    fn get_data_provider(&self) -> &Self::DataProvider {
        &self.data_provider
    }
}

fn update_game_status(game: &mut ApGame) -> bool {
    if matches!(game.status, GameStatus::Released | GameStatus::Glitched) {
        // Released and glitched are special cases that take priority over all others.
        false
    } else {
        game.status = match (game.checks_done == game.checks_total, game.tracker_status) {
            (true, TrackerGameStatus::GoalCompleted) => GameStatus::Done,
            (true, _) => GameStatus::AllChecks,
            (false, TrackerGameStatus::GoalCompleted) => GameStatus::Goal,
            _ => return false,
        };

        true
    }
}

impl<D> AppState<D> {
    fn new(config: conf::Config, data_provider: D) -> Self {
        Self {
            reqwest_client: reqwest::Client::builder().build().unwrap(),
            data_provider,
            tracker_base_url: "https://archipelago.gg/tracker/".parse().unwrap(),
            ui_settings: UiSettings {
                is_staging: config.is_staging,
                build_version: option_env!("GIT_COMMIT")
                    .filter(|s| !s.is_empty())
                    .unwrap_or("dev"),
            },
            inflight_tracker_updates: RwLock::default(),
            tracker_update_interval: config.tracker_update_interval,
            auth_client: auth::discord::AuthClient::new(
                config.discord.client_id,
                config.discord.client_secret,
                &config.public_url,
                config.discord.token_cipher,
            ),
            token_processor: auth::token::TokenProcessor::new(
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
                        title: "".to_owned(),
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
                        status: GameStatus::Unknown,
                        last_checked: None,
                        notes: String::new(),
                        claimed_by_ct_user_id: None,
                    };

                    update_game_status(&mut game);

                    let game = db
                        .create_ap_games([game])
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

                    if update_game_status(&mut db_game) {
                        columns.push(ApGameIden::Status);
                    }

                    db.update_ap_game(db_game, &columns).await?;
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
                        // If the receiving game can't be found, it's most
                        // likely an item link check, which means the receiver
                        // would be multiple games.  We record this as null in
                        // the database.
                        receiver_game_id: name_to_id.get(&hint.receiver).copied(),
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
struct UpdateTrackerRequest {
    pub title: String,
}

async fn update_tracker<D>(
    State(state): State<Arc<AppState<D>>>,

    // Not used for now but requires that the user at least be logged in.  Will
    // be used later for an audit trail.
    AuthenticatedUser(_): AuthenticatedUser,

    Path(tracker_id): Path<String>,
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
        .get_tracker_by_ap_tracker_id(&tracker_id)
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

    tracker.title = tracker_update.title;

    tx.update_ap_tracker(tracker, &[ApTrackerIden::Title])
        .await
        .unexpected()?;

    tx.commit().await.unexpected()?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Deserialize)]
struct UpdateGameRequest {
    pub claimed_by_ct_user_id: Option<i32>,
    pub discord_username: Option<String>,
    pub discord_ping: PingPreference,
    pub status: GameStatus,
    pub last_checked: Option<DateTime<Utc>>,
    pub notes: String,
}

async fn update_game<D>(
    State(state): State<Arc<AppState<D>>>,
    AuthenticatedUser(user): AuthenticatedUser,
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

    // Reject new unauthenticated claims.
    if game.discord_username.is_none()
        && game_update.claimed_by_ct_user_id.is_none()
        && game_update.discord_username.is_some()
    {
        return Err(StatusCode::FORBIDDEN);
    }

    // Reject new authenticated claims for a different user ID.
    if game.claimed_by_ct_user_id.is_none()
        && game_update
            .claimed_by_ct_user_id
            .is_some_and(|id| id != user.id)
    {
        return Err(StatusCode::FORBIDDEN);
    }

    // Update the username.
    game.discord_username = match game_update.claimed_by_ct_user_id {
        // If already claimed by the same user, this should be a no-op.
        // Otherwise, sets the display name.
        Some(id) if id == user.id => Some(user.discord_username),
        // No-op.
        Some(_) => game.discord_username,
        // Clear, which should only be a change when disclaiming a slot.
        None => None,
    };

    game.claimed_by_ct_user_id = game_update.claimed_by_ct_user_id;
    game.discord_ping = game_update.discord_ping;
    game.status = game_update.status;
    game.last_checked = game_update.last_checked;
    game.notes = game_update.notes;

    update_game_status(&mut game);

    tx.update_ap_game(
        game.clone(),
        &[
            ApGameIden::ClaimedByCtUserId,
            ApGameIden::DiscordUsername,
            ApGameIden::DiscordPing,
            ApGameIden::Status,
            ApGameIden::LastChecked,
            ApGameIden::Notes,
        ],
    )
    .await
    .unexpected()?;

    tx.commit().await.unexpected()?;

    Ok(Json(game))
}

async fn begin_discord_auth<D>(
    State(state): State<Arc<AppState<D>>>,
) -> Result<impl IntoResponse, StatusCode> {
    state.auth_client.begin().unexpected().map(Json)
}

#[derive(serde::Deserialize)]
struct CompleteAuthRequest {
    pub code: String,
    pub state: String,
    pub continuation_token: String,
}

async fn complete_discord_auth<D>(
    State(state): State<Arc<AppState<D>>>,
    Json(request): Json<CompleteAuthRequest>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    #[derive(Debug, thiserror::Error)]
    #[error("missing refresh token")]
    struct MissingRefreshTokenError;

    #[derive(Debug, thiserror::Error)]
    #[error("failed to insert Discord user {0} but user doesn't exist")]
    struct MissingUserError(u64);

    #[derive(serde::Serialize)]
    struct Response {
        token: String,
        user_id: i32,
        discord_username: String,
    }

    let token = state
        .auth_client
        .complete(request.code, &request.state, &request.continuation_token)
        .await
        .unexpected()?;

    let expires_at = Utc::now()
        + token
            .expires_in()
            .and_then(|d| chrono::Duration::from_std(d).ok())
            .unwrap_or_else(|| chrono::Duration::days(1));

    let user_info = serenity::http::Http::new(&format!("Bearer {}", token.access_token().secret()))
        .get_current_user()
        .await
        .unexpected()?;

    // This may overflow, which is fine.  PostgreSQL doesn't support unsigned
    // types; the alternative is NUMERIC or TEXT.
    //
    // The domains of i64 and u64 are the same size, and the cast is reversible,
    // so we can cast back to u64 later to retrieve the true user ID.
    let discord_user_id = user_info.id.get() as i64;

    let mut db = state
        .data_provider
        .create_data_access()
        .await
        .unexpected()?;

    // Try to insert the user.  If the user already exists, we'll fetch it
    // below.
    let ct_user = match db
        .create_ct_users([CtUser {
            id: 0,
            discord_access_token: token.access_token().secret().to_owned(),
            discord_access_token_expires_at: expires_at,
            discord_refresh_token: token
                .refresh_token()
                .ok_or(MissingRefreshTokenError)
                .unexpected()?
                .secret()
                .to_owned(),
            discord_user_id,
            discord_username: user_info.name.clone(),
        }])
        .try_next()
        .await
    {
        Err(e)
            if e.as_database_error()
                .is_some_and(|dbe| dbe.is_unique_violation()) =>
        {
            Ok(None)
        }
        v => v,
    }
    .unexpected()?;

    let ct_user = match ct_user {
        Some(u) => u,
        None => db
            .get_ct_user_by_discord_user_id(discord_user_id)
            .await
            .unexpected()?
            .ok_or(MissingUserError(user_info.id.get()))
            .unexpected()?,
    };

    Ok(Json(Response {
        token: state.token_processor.encode(ct_user.id).unexpected()?,
        user_id: ct_user.id,
        discord_username: ct_user.discord_username,
    }))
}

#[derive(Debug, Clone, serde::Serialize)]
struct UiSettings {
    pub is_staging: bool,
    pub build_version: &'static str,
}

async fn get_settings<D>(State(state): State<Arc<AppState<D>>>) -> Json<UiSettings> {
    Json(state.ui_settings.clone())
}

#[derive(Debug, Clone, serde::Deserialize)]
struct CreateJsErrorRequest {
    pub ct_user_id: Option<i32>,
    pub error: String,
}

async fn create_js_error<D>(
    State(state): State<Arc<AppState<D>>>,
    Json(request): Json<CreateJsErrorRequest>,
) -> StatusCode
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    // We don't need to inform the client if this fails, so perform the
    // insertion in the background and respond immediately.
    tokio::spawn(async move {
        let mut db = state
            .data_provider
            .create_data_access()
            .await
            .unexpected()?;

        db.create_js_errors([JsError {
            id: 0,
            ct_user_id: request.ct_user_id,
            error: request.error,
        }])
        .try_for_each(|_| std::future::ready(Ok(())))
        .await
        .unexpected()
    });

    StatusCode::ACCEPTED
}

fn create_api_router<D>(state: AppState<D>) -> axum::Router<()>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    use axum::routing::*;

    axum::Router::new()
        .route("/auth/begin", get(begin_discord_auth))
        .route("/auth/complete", post(complete_discord_auth))
        .route("/tracker/:tracker_id", get(get_tracker))
        .route("/tracker/:tracker_id", put(update_tracker))
        .route("/tracker/:tracker_id/game/:game_id", put(update_game))
        .route("/settings", get(get_settings))
        .route("/jserror", post(create_js_error))
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
