use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use futures::TryStreamExt;

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
    state::AppState,
};

pub async fn get_tracker<D>(
    State(state): State<Arc<AppState<D>>>,
    Path(tracker_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode>
where
    D: DataAccessProvider + Send + Sync + 'static,
{
    #[derive(serde::Serialize)]
    struct GetTrackerResponse {
        #[serde(flatten)]
        pub tracker: ApTracker,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub owner_discord_username: Option<String>,
        pub games: Vec<ApGame>,
        pub hints: Vec<ApHint>,
    }

    if let Err(err) = state.update_tracker(&tracker_id).await {
        // Log this error but do not fail the overall operation; if we have old
        // data in the database then we can still use it.
        println!("Failed to update tracker {tracker_id}: {err}");
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
        tracker,
        owner_discord_username,
        games,
        hints,
    }))
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateTrackerRequest {
    pub title: String,
    pub owner_ct_user_id: Option<i32>,
    pub lock_title: bool,
}

pub async fn update_tracker<D>(
    State(state): State<Arc<AppState<D>>>,
    Path(tracker_id): Path<String>,
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
        .get_tracker_by_ap_tracker_id(&tracker_id)
        .await
        .unexpected()?
        .ok_or(StatusCode::NOT_FOUND)?;

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

    match (tracker.owner_ct_user_id, user) {
        // There is no current owner.  Force everything unlocked, and allow
        // updating all settings.
        (None, _) => {
            tracker.title = tracker_update.title;

            if tracker_update.lock_title {
                return Err(StatusCode::FORBIDDEN);
            }
            tracker.lock_title = false;
        }

        // The current user is the owner.  They can change all settings.
        (Some(uid), Some(u)) if uid == u.0.id => {
            tracker.title = tracker_update.title;
            tracker.lock_title = tracker_update.lock_title;
        }

        // The current user is not the owner.  They can only change unlocked
        // settings, and cannot configure any locks.
        _ => {
            if !tracker.lock_title {
                if tracker_update.lock_title {
                    return Err(StatusCode::FORBIDDEN);
                }

                tracker.title = tracker_update.title;
            } else if tracker.title != tracker_update.title {
                return Err(StatusCode::FORBIDDEN);
            }
        }
    };

    tx.update_ap_tracker(
        tracker,
        &[
            ApTrackerIden::Title,
            ApTrackerIden::OwnerCtUserId,
            ApTrackerIden::LockTitle,
        ],
    )
    .await
    .unexpected()?;

    tx.commit().await.unexpected()?;

    Ok(StatusCode::NO_CONTENT)
}
#[derive(Debug, serde::Deserialize)]
pub struct UpdateHintRequest {
    pub classification: HintClassification,
}

pub async fn update_hint<D>(
    State(state): State<Arc<AppState<D>>>,
    Path((tracker_id, hint_id)): Path<(String, i32)>,
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
        .get_tracker_by_ap_tracker_id(&tracker_id)
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

pub async fn update_game<D>(
    State(state): State<Arc<AppState<D>>>,
    user: Option<AuthenticatedUser>,
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

    // If the claimed user ID is changing to a value other than None, it must
    // match the authenticated user's ID.
    if game_update.claimed_by_ct_user_id != game.claimed_by_ct_user_id
        && game_update
            .claimed_by_ct_user_id
            .is_some_and(|id| user.as_ref().map_or(true, |u| u.0.id != id))
    {
        return Err(StatusCode::FORBIDDEN);
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
